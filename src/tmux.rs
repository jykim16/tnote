use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::process::Command;

pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// True when running inside a tmux session.
pub fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// True when running inside a tnote popup session.
pub fn is_popup_session() -> bool {
    let Ok(output) = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .starts_with("tnote-popup-")
}

/// Returns the current tmux window's note key, e.g. "tmux-$1+@3".
/// Uses stable session/window IDs so renames don't break the key.
pub fn window_key() -> Option<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{session_id}+#{window_id}"])
        .output()
        .ok()?;
    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if key.is_empty() { None } else { Some(format!("tmux-{}", key)) }
}

/// Returns the set of all live tmux window keys (e.g. "tmux-$1+@3") across all sessions.
pub fn live_window_keys() -> std::collections::HashSet<String> {
    let Ok(output) = Command::new("tmux")
        .args(["list-windows", "-a", "-F", "#{session_id}+#{window_id}"])
        .output()
    else {
        return std::collections::HashSet::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| format!("tmux-{}", l.trim()))
        .collect()
}

/// Returns a map of "tmux-$1+@3" → "session_name+window_index" for all live windows.
pub fn window_label_map() -> HashMap<String, String> {
    let Ok(output) = Command::new("tmux")
        .args(["list-windows", "-a", "-F",
               "#{session_id} #{window_id} #{session_name}+#{window_index}"])
        .output()
    else {
        return HashMap::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, ' ');
            let s_id  = parts.next()?;
            let w_id  = parts.next()?;
            let label = parts.next()?;
            Some((format!("tmux-{}+{}", s_id, w_id), label.trim().to_string()))
        })
        .collect()
}

/// Returns the human-readable "session_name+window_index" label for an ID-based key.
pub fn window_display_label(key: &str) -> Option<String> {
    window_label_map().get(key).cloned()
}


/// Parse a tmux version string like "3.2a" or "2.9" into (major, minor).
fn parse_version_str(s: &str) -> Option<(u32, u32)> {
    let mut parts = s.trim().splitn(2, '.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor_str = parts.next().unwrap_or("0");
    // Strip any trailing non-numeric chars (e.g. "2a" -> 2)
    let minor: u32 = minor_str
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    Some((major, minor))
}

/// Returns the tmux server version as a (major, minor) tuple, or None if it can't be determined.
fn tmux_server_version() -> Option<(u32, u32)> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{version}"])
        .output()
        .ok()?;
    parse_version_str(&String::from_utf8_lossy(&output.stdout))
}

/// Open (or reattach to) a persistent popup session for the given note file.
pub fn open_popup_session(file: &Path, key: &str, config: &crate::config::Config) -> io::Result<()> {
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("note");
    let stem_safe = stem.replace(['+', '/', '$', '@'], "_");
    let popup_session = format!("tnote-popup-{}", stem_safe);

    let (note_type, note_label) = if let Some(s) = stem.strip_prefix("named-") {
        ("named".to_string(), s.to_string())
    } else if stem.starts_with("tmux-") {
        let label = window_display_label(stem)
            .unwrap_or_else(|| stem.strip_prefix("tmux-").unwrap_or(stem).to_string());
        ("tmux".to_string(), label)
    } else if let Some(s) = stem.strip_prefix("shell-") {
        ("shell".to_string(), s.to_string())
    } else {
        ("".to_string(), stem.to_string())
    };

    let popup_title = if note_type.is_empty() {
        format!(" tnote - {} ", note_label)
    } else {
        format!(" {} tnote - {} ", note_type, note_label)
    };

    let attach_cmd = format!(
        "tmux attach-session -t {sess} 2>/dev/null || \
         tmux new-session -s {sess} -e TNOTE_WINDOW_KEY={key} -e EDITOR={editor} tnote popup",
        sess   = shell_escape(&popup_session),
        key    = shell_escape(key),
        editor = shell_escape(&config.editor),
    );

    let output = Command::new("tmux")
        .args([
            "display-popup",
            "-x", "R", "-y", "T",
            "-w", &config.width,
            "-h", &config.height,
            "-b", "rounded",
            "-T", &popup_title,
            "-E", &attach_cmd,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("protocol version mismatch") {
            return Err(io::Error::other(
                "tmux popup: protocol version mismatch between tmux client and server \
                 (tmux was likely upgraded while a server was running) — \
                 kill the old server with `pkill tmux`, then start a fresh session",
            ));
        }
        match tmux_server_version() {
            Some((major, minor)) if major < 3 || (major == 3 && minor < 2) => {
                return Err(io::Error::other(format!(
                    "tmux popup: display-popup requires tmux 3.2+ (server is {}.{}); \
                     please upgrade tmux",
                    major, minor
                )));
            }
            _ => {
                return Err(io::Error::other("tmux popup: display-popup failed"));
            }
        }
    }
    Ok(())
}

/// Kill any tnote popup sessions whose note file no longer exists.
/// Returns the list of session names that were removed.
pub fn cleanup_popup_sessions(note_dir: &std::path::Path, dry_run: bool) -> Vec<String> {
    // Build the set of session names that have a matching note file.
    let valid: std::collections::HashSet<String> =
        std::fs::read_dir(note_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                if path.extension()?.to_str()? != "md" { return None; }
                let stem = path.file_stem()?.to_str()?;
                Some(format!("tnote-popup-{}", stem.replace(['+', '/'], "_")))
            })
            .collect();

    let Ok(output) = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}:#{session_attached}"])
        .output()
    else {
        return vec![];
    };

    let to_kill: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (name, attached) = line.split_once(':')?;
            let attached: u32 = attached.trim().parse().ok()?;
            if name.starts_with("tnote-popup-") && attached == 0 && !valid.contains(name) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();

    if !dry_run {
        for session in &to_kill {
            let _ = Command::new("tmux").args(["kill-session", "-t", session]).status();
        }
    }

    to_kill
}

/// Rename the current tmux window.
pub fn rename_window(name: &str) {
    let _ = Command::new("tmux")
        .args(["rename-window", name])
        .output();
}

/// Show a message in the tmux status bar.
pub fn display_message(msg: &str) {
    let _ = Command::new("tmux")
        .args(["display-message", msg])
        .status();
}

fn new_name_prompt_command() -> String {
    "command-prompt -p 'Note name:' \"run-shell 'tnote name %%'\"".to_string()
}

fn name_menu_args(note_names: &[String]) -> Vec<String> {
    let mut args = vec![
        "display-menu".to_string(),
        "-T".to_string(),
        " tnote name ".to_string(),
        "New name...".to_string(),
        "N".to_string(),
        new_name_prompt_command(),
    ];

    for name in note_names {
        let shell_cmd = format!("tnote name {}", shell_escape(name));
        args.push(name.clone());
        args.push(String::new());
        args.push(format!("run-shell {}", shell_escape(&shell_cmd)));
    }

    args
}

/// Show tmux's note-naming menu, with a fallback prompt if the menu fails.
pub fn prompt_name(note_names: &[String]) {
    let status = Command::new("tmux")
        .args(name_menu_args(note_names))
        .status();

    if status.as_ref().is_ok_and(|s| s.success()) {
        return;
    }

    let _ = Command::new("tmux")
        .args(["command-prompt", "-p", "Note name:", "run-shell 'tnote name %%'"])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── shell_escape ──────────────────────────────────────────────────────────

    #[test]
    fn shell_escape_simple_string() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn shell_escape_empty_string() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn shell_escape_single_quote() {
        // it's → 'it'\''s'
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn shell_escape_multiple_quotes() {
        assert_eq!(shell_escape("a'b'c"), "'a'\\''b'\\''c'");
    }

    #[test]
    fn shell_escape_only_quote() {
        assert_eq!(shell_escape("'"), "''\\'''");
    }

    #[test]
    fn shell_escape_special_chars_unchanged() {
        // Spaces, $, @, etc. are safe inside single quotes
        assert_eq!(shell_escape("foo bar $HOME"), "'foo bar $HOME'");
    }

    // ── parse_version_str ─────────────────────────────────────────────────────

    #[test]
    fn parse_version_simple() {
        assert_eq!(parse_version_str("3.2"), Some((3, 2)));
    }

    #[test]
    fn parse_version_with_suffix() {
        // tmux sometimes releases versions like "3.2a"
        assert_eq!(parse_version_str("3.2a"), Some((3, 2)));
    }

    #[test]
    fn parse_version_major_only_treated_as_minor_zero() {
        assert_eq!(parse_version_str("3"), Some((3, 0)));
    }

    #[test]
    fn parse_version_old_version() {
        assert_eq!(parse_version_str("2.9"), Some((2, 9)));
    }

    #[test]
    fn parse_version_trims_whitespace() {
        assert_eq!(parse_version_str("  3.3\n"), Some((3, 3)));
    }

    #[test]
    fn parse_version_empty_string_returns_none() {
        assert_eq!(parse_version_str(""), None);
    }

    #[test]
    fn parse_version_non_numeric_returns_none() {
        assert_eq!(parse_version_str("invalid"), None);
    }

    #[test]
    fn parse_version_satisfies_popup_requirement() {
        // display-popup requires >= 3.2
        let requires = |(maj, min): (u32, u32)| maj > 3 || (maj == 3 && min >= 2);
        assert!(requires(parse_version_str("3.2").unwrap()));
        assert!(requires(parse_version_str("3.2a").unwrap()));
        assert!(requires(parse_version_str("3.3").unwrap()));
        assert!(requires(parse_version_str("4.0").unwrap()));
        assert!(!requires(parse_version_str("3.1").unwrap()));
        assert!(!requires(parse_version_str("2.9").unwrap()));
        assert!(!requires(parse_version_str("3.0").unwrap()));
    }

    #[test]
    fn name_menu_args_include_new_name_entry() {
        let args = name_menu_args(&[]);
        assert_eq!(args[0], "display-menu");
        assert!(args.iter().any(|arg| arg == "New name..."));
        assert!(args.iter().any(|arg| arg.contains("command-prompt")));
    }

    #[test]
    fn name_menu_args_include_existing_names() {
        let args = name_menu_args(&["alpha".to_string(), "beta project".to_string()]);
        assert!(args.iter().any(|arg| arg == "alpha"));
        assert!(args.iter().any(|arg| arg == "beta project"));
        assert!(args.iter().any(|arg| arg.contains("run-shell")));
    }
}
