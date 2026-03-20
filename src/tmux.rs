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

    let status = Command::new("tmux")
        .args([
            "display-popup",
            "-x", "R", "-y", "T",
            "-w", &config.width.to_string(),
            "-h", &config.height.to_string(),
            "-b", "rounded",
            "-T", &popup_title,
            "-E", &attach_cmd,
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "tmux display-popup failed"));
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

    for session in &to_kill {
        let _ = Command::new("tmux").args(["kill-session", "-t", session]).status();
    }

    to_kill
}

/// Rename the current tmux window.
pub fn rename_window(name: &str) {
    let _ = Command::new("tmux")
        .args(["rename-window", name])
        .output();
}
