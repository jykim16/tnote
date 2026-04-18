use crate::config::Config;
use crate::tmux::shell_escape;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn which(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null 2>&1", cmd)])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn user_tmux_conf() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".tmux.conf"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn executable_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| "tnote".to_string())
}

fn tmux_run_shell(cmd: &str) -> String {
    format!("run-shell {}", shell_escape(cmd))
}

/// Detect the user's shell from $SHELL.
fn detect_shell() -> Option<String> {
    std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|n| n.to_string()))
}

/// Return the rc file path for the given shell.
fn shell_rc(shell: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    match shell {
        "zsh" => Some(home.join(".zshrc")),
        "bash" => {
            // Keybindings belong in .bashrc (interactive shells), not .bash_profile (login shells).
            Some(home.join(".bashrc"))
        }
        "fish" => {
            let dir = home.join(".config/fish");
            let _ = std::fs::create_dir_all(&dir);
            Some(dir.join("config.fish"))
        }
        _ => None,
    }
}

/// Generate the shell keybinding snippet for the given shell and key.
fn shell_binding(shell: &str, key: &str) -> Option<String> {
    let ctrl_key = format!("\\C-{}", key);
    match shell {
        "zsh" => Some(format!(
            "{marker}\n\
             _tnote_toggle() {{\n\
               local j p\n\
               j=$(jobs -l 2>/dev/null | awk 'tolower($0)~/tnote/ && tolower($0)~/stop|suspend/ {{gsub(/[\\[\\]+-]/,\"\",$1);print $1;exit}}')\n\
               p=$(jobs -l 2>/dev/null | awk '!/tnote/ && tolower($0)~/stop|suspend/ {{for(i=1;i<=NF;i++) if($i~/^[0-9]+$/ && $i+0>100){{print $i;exit}}}}')\n\
               if [ -n \"$p\" ]; then echo \"$p\" > \"/tmp/tnote-swap-target-$$\"\n\
               else rm -f \"/tmp/tnote-swap-target-$$\"; fi\n\
               if [ -n \"$j\" ]; then fg %\"$j\"\n\
               else tnote; fi\n\
               zle reset-prompt\n\
             }}\n\
             _tnote_precmd() {{\n\
               local f=\"/tmp/tnote-swap-$$\"\n\
               [ -f \"$f\" ] || return\n\
               local spid; spid=$(cat \"$f\"); rm -f \"$f\"\n\
               [ -n \"$spid\" ] || return\n\
               local j; j=$(jobs -l 2>/dev/null | awk -v p=\"$spid\" '$0~p && tolower($0)~/stop|suspend/ {{gsub(/[\\[\\]+-]/,\"\",$1);print $1;exit}}')\n\
               [ -n \"$j\" ] && fg %\"$j\" 2>/dev/null\n\
             }}\n\
             zle -N _tnote_toggle\n\
             precmd_functions+=(_tnote_precmd)\n\
             if [ -z \"$TMUX\" ]; then bindkey '{ctrl_key}' _tnote_toggle; fi\n\
             {end}",
            marker = SHELL_MARKER,
            ctrl_key = ctrl_key,
            end = SHELL_END_MARKER,
        )),
        "bash" => Some(format!(
            "{marker}\n\
             _tnote_toggle() {{\n\
               local j p\n\
               j=$(jobs -l 2>/dev/null | awk 'tolower($0)~/tnote/ && tolower($0)~/stop|suspend/ {{gsub(/[\\[\\]+-]/,\"\",$1);print $1;exit}}')\n\
               p=$(jobs -l 2>/dev/null | awk '!/tnote/ && tolower($0)~/stop|suspend/ {{for(i=1;i<=NF;i++) if($i~/^[0-9]+$/ && $i+0>100){{print $i;exit}}}}')\n\
               if [ -n \"$p\" ]; then echo \"$p\" > \"/tmp/tnote-swap-target-$$\"\n\
               else rm -f \"/tmp/tnote-swap-target-$$\"; fi\n\
               if [ -n \"$j\" ]; then fg %\"$j\"\n\
               else tnote; fi\n\
             }}\n\
             _tnote_precmd() {{\n\
               local f=\"/tmp/tnote-swap-$$\"\n\
               [ -f \"$f\" ] || return\n\
               local spid; spid=$(cat \"$f\"); rm -f \"$f\"\n\
               [ -n \"$spid\" ] || return\n\
               local j; j=$(jobs -l 2>/dev/null | awk -v p=\"$spid\" '$0~p && tolower($0)~/stop|suspend/ {{gsub(/[\\[\\]+-]/,\"\",$1);print $1;exit}}')\n\
               [ -n \"$j\" ] && fg %\"$j\" 2>/dev/null\n\
             }}\n\
             if [ -z \"$PROMPT_COMMAND\" ]; then PROMPT_COMMAND=\"_tnote_precmd\"\n\
             else PROMPT_COMMAND=\"_tnote_precmd; $PROMPT_COMMAND\"; fi\n\
             if [ -z \"$TMUX\" ]; then bind -x '\"{ctrl_key}\":\"_tnote_toggle\"'; fi\n\
             {end}",
            marker = SHELL_MARKER,
            ctrl_key = ctrl_key,
            end = SHELL_END_MARKER,
        )),
        "fish" => Some(format!(
            "{marker}\n\
             function _tnote_toggle\n\
               set -l j (jobs 2>/dev/null | awk 'tolower($0)~/tnote/ && tolower($0)~/stop|suspend/ {{print NR;exit}}')\n\
               set -l p (jobs 2>/dev/null | awk '!/tnote/ && tolower($0)~/stop|suspend/ {{print NR;exit}}')\n\
               if test -n \"$p\"; echo \"$p\" > \"/tmp/tnote-swap-target-$fish_pid\"\n\
               else rm -f \"/tmp/tnote-swap-target-$fish_pid\"; end\n\
               if test -n \"$j\"; fg $j\n\
               else tnote; end\n\
               commandline -f repaint\n\
             end\n\
             function _tnote_precmd --on-event fish_prompt\n\
               set -l f \"/tmp/tnote-swap-$fish_pid\"\n\
               test -f \"$f\" || return\n\
               set -l spid (cat \"$f\"); rm -f \"$f\"\n\
               test -n \"$spid\" || return\n\
               set -l j (jobs 2>/dev/null | awk -v p=\"$spid\" 'NR>1 && $0~p {{print NR-1;exit}}')\n\
               test -n \"$j\"; and fg $j 2>/dev/null\n\
             end\n\
             if not set -q TMUX; bind \\c{key} '_tnote_toggle'; end\n\
             {end}",
            marker = SHELL_MARKER,
            key = key,
            end = SHELL_END_MARKER,
        )),
        _ => None,
    }
}

/// Marker line used to identify the tnote block in shell rc files.
const SHELL_MARKER: &str = "# tnote keybinding — managed by 'tnote setup' / 'tnote uninstall'";
const SHELL_END_MARKER: &str = "# end tnote keybinding";

fn add_shell_binding(rc_path: &Path, binding: &str) -> std::io::Result<()> {
    let content = fs::read_to_string(rc_path).unwrap_or_default();
    // Remove any existing tnote block first
    let cleaned = remove_shell_block(&content);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(rc_path)?;
    f.write_all(cleaned.as_bytes())?;
    if !cleaned.is_empty() && !cleaned.ends_with('\n') {
        writeln!(f)?;
    }
    writeln!(f, "{}", binding)?;
    Ok(())
}

fn remove_shell_binding(rc_path: &Path) -> std::io::Result<bool> {
    let Ok(content) = fs::read_to_string(rc_path) else {
        return Ok(false);
    };
    let cleaned = remove_shell_block(&content);
    if cleaned == content {
        return Ok(false);
    }
    fs::write(rc_path, &cleaned)?;
    Ok(true)
}

/// Remove the tnote keybinding block from shell rc content.
///
/// Supports two formats:
/// - New (end-marker): removes all lines from SHELL_MARKER through SHELL_END_MARKER inclusive.
/// - Legacy (no end-marker): removes SHELL_MARKER and exactly the one line after it.
fn remove_shell_block(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<&str> = Vec::new();

    // Detect whether new end-marker format is present.
    let has_end_marker = {
        let mut found_start = false;
        let mut found_end = false;
        for line in &lines {
            let t = line.trim();
            if t == SHELL_MARKER {
                found_start = true;
            } else if found_start && t == SHELL_END_MARKER {
                found_end = true;
                break;
            }
        }
        found_start && found_end
    };

    if has_end_marker {
        let mut in_block = false;
        for line in &lines {
            let t = line.trim();
            if !in_block && t == SHELL_MARKER {
                in_block = true;
                continue;
            }
            if in_block {
                if t == SHELL_END_MARKER {
                    in_block = false;
                }
                continue;
            }
            result.push(line);
        }
    } else {
        let mut skip_next = false;
        for line in &lines {
            if skip_next {
                skip_next = false;
                continue;
            }
            if line.trim() == SHELL_MARKER {
                skip_next = true;
                continue;
            }
            result.push(line);
        }
    }

    let mut out = result.join("\n");
    if content.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out
}

pub fn add_source_line(user_conf: &Path, source_path: &Path) -> std::io::Result<()> {
    let line = format!("source-file {}", source_path.display());
    let content = fs::read_to_string(user_conf).unwrap_or_default();
    if content.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(user_conf)?;
    if !content.is_empty() && !content.ends_with('\n') {
        writeln!(f)?;
    }
    writeln!(f, "{}", line)?;
    Ok(())
}

pub fn remove_source_line(user_conf: &Path, source_path: &Path) -> std::io::Result<()> {
    let Ok(content) = fs::read_to_string(user_conf) else {
        return Ok(());
    };
    let line = format!("source-file {}", source_path.display());
    let filtered: Vec<&str> = content.lines().filter(|l| l.trim() != line).collect();
    let new_content = if content.ends_with('\n') {
        format!("{}\n", filtered.join("\n"))
    } else {
        filtered.join("\n")
    };
    if new_content != content {
        fs::write(user_conf, new_content)?;
    }
    Ok(())
}

pub fn run(config: &Config) {
    if let Err(e) = fs::create_dir_all(&config.dir) {
        eprintln!(
            "tnote setup: failed to create {}: {}",
            config.dir.display(),
            e
        );
        std::process::exit(1);
    }

    let meta_dir = config.dir.join("meta");
    if let Err(e) = fs::create_dir_all(&meta_dir) {
        eprintln!(
            "tnote setup: failed to create {}: {}",
            meta_dir.display(),
            e
        );
        std::process::exit(1);
    }

    // ── tmux bindings ─────────────────────────────────────────────────────
    if which("tmux") || user_tmux_conf().map(|p| p.exists()).unwrap_or(false) {
        let tmux_conf_path = meta_dir.join("tmux.conf");

        if let Ok(old) = fs::read_to_string(&tmux_conf_path) {
            if let Some(old_key) = old.lines().find_map(|l| {
                l.strip_prefix("bind-key ")
                    .and_then(|r| r.split_whitespace().next())
            }) {
                if old_key != config.key {
                    let _ = Command::new("tmux").args(["unbind-key", old_key]).status();
                }
            }
        }

        let key = &config.key;
        let tnote_bin = executable_path();
        let tnote_cmd = tmux_run_shell(&tnote_bin);
        let tnote_show_cmd = tmux_run_shell(&format!("{} show", shell_escape(&tnote_bin)));
        let tnote_list_cmd = tmux_run_shell(&format!("{} list", shell_escape(&tnote_bin)));
        let tnote_path_cmd = tmux_run_shell(&format!("{} path", shell_escape(&tnote_bin)));
        let tnote_clean_cmd = tmux_run_shell(&format!("{} clean", shell_escape(&tnote_bin)));
        let tnote_help_cmd = tmux_run_shell(&format!("{} help", shell_escape(&tnote_bin)));
        let tnote_name_cmd = tmux_run_shell(&format!("{} name", shell_escape(&tnote_bin)));
        let tnote_ls_cmd = tmux_run_shell(&format!("{} ls", shell_escape(&tnote_bin)));
        let tmux_conf = format!(
            "# tnote key bindings — managed by 'tnote setup' / 'tnote uninstall'\n\
             unbind-key {key}\n\
             bind-key {key} {tnote_cmd}\n\
             set -s command-alias[100] \"tnote={tnote_cmd}\"\n\
             set -s command-alias[101] \"tnote-show={tnote_show_cmd}\"\n\
             set -s command-alias[102] \"tnote-list={tnote_list_cmd}\"\n\
             set -s command-alias[103] \"tnote-path={tnote_path_cmd}\"\n\
             set -s command-alias[104] \"tnote-clean={tnote_clean_cmd}\"\n\
             set -s command-alias[105] \"tnote-help={tnote_help_cmd}\"\n\
             set -s command-alias[106] \"tnote-name={tnote_name_cmd}\"\n\
             set -s command-alias[107] \"tnote-ls={tnote_ls_cmd}\"\n",
            key = key,
            tnote_cmd = tnote_cmd,
            tnote_show_cmd = tnote_show_cmd,
            tnote_list_cmd = tnote_list_cmd,
            tnote_path_cmd = tnote_path_cmd,
            tnote_clean_cmd = tnote_clean_cmd,
            tnote_help_cmd = tnote_help_cmd,
            tnote_name_cmd = tnote_name_cmd,
            tnote_ls_cmd = tnote_ls_cmd,
        );

        if let Err(e) = fs::write(&tmux_conf_path, &tmux_conf) {
            eprintln!(
                "tnote setup: failed to write {}: {}",
                tmux_conf_path.display(),
                e
            );
            std::process::exit(1);
        }
        println!("tnote setup: wrote {}", tmux_conf_path.display());

        let status = Command::new("tmux")
            .args(["source-file", &tmux_conf_path.to_string_lossy()])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("tnote setup: sourced bindings into live tmux session")
            }
            _ => eprintln!("tnote setup: tmux source-file failed"),
        }

        if let Some(user_conf) = user_tmux_conf() {
            match add_source_line(&user_conf, &tmux_conf_path) {
                Ok(_) => println!(
                    "tnote setup: added source-file line to {}",
                    user_conf.display()
                ),
                Err(e) => eprintln!(
                    "tnote setup: could not update {}: {}",
                    user_conf.display(),
                    e
                ),
            }
        }

        println!(
            "tnote setup: tmux binding: prefix+{} opens/closes tnote popup",
            config.key
        );
    }

    // ── shell keybinding ──────────────────────────────────────────────────
    if let Some(shell) = detect_shell() {
        if let (Some(rc), Some(binding)) = (shell_rc(&shell), shell_binding(&shell, &config.key)) {
            match add_shell_binding(&rc, &binding) {
                Ok(_) => {
                    println!(
                        "tnote setup: added Ctrl-{} binding to {}",
                        config.key,
                        rc.display()
                    );
                    println!(
                        "tnote setup: restart your shell or run: source {}",
                        rc.display()
                    );
                }
                Err(e) => eprintln!("tnote setup: could not update {}: {}", rc.display(), e),
            }
        } else {
            println!(
                "tnote setup: unsupported shell '{}', skipping shell keybinding",
                shell
            );
        }
    }

    println!("tnote setup: setup complete.");
}

pub fn uninstall(config: &Config) {
    let tmux_conf_path = config.dir.join("meta").join("tmux.conf");
    let key = &config.key;

    // ── tmux ──────────────────────────────────────────────────────────────
    let cleared = format!(
        "# tnote key bindings — cleared by 'tnote uninstall'\n\
         unbind-key {key}\n\
         set -su command-alias[100]\n\
         set -su command-alias[101]\n\
         set -su command-alias[102]\n\
         set -su command-alias[103]\n\
         set -su command-alias[104]\n\
         set -su command-alias[105]\n\
         set -su command-alias[106]\n\
         set -su command-alias[107]\n\
         set -su command-alias[108]\n",
        key = key
    );

    match fs::write(&tmux_conf_path, &cleared) {
        Ok(_) => {
            let _ = Command::new("tmux")
                .args(["source-file", &tmux_conf_path.to_string_lossy()])
                .status();
            println!("tnote uninstall: cleared tmux bindings");
        }
        Err(_) => {
            let _ = Command::new("tmux").args(["unbind-key", key]).status();
        }
    }

    if let Some(user_conf) = user_tmux_conf() {
        match remove_source_line(&user_conf, &tmux_conf_path) {
            Ok(_) => println!(
                "tnote uninstall: removed source-file line from {}",
                user_conf.display()
            ),
            Err(e) => eprintln!(
                "tnote uninstall: could not update {}: {}",
                user_conf.display(),
                e
            ),
        }
    }

    // ── shell keybinding ──────────────────────────────────────────────────
    if let Some(shell) = detect_shell() {
        if let Some(rc) = shell_rc(&shell) {
            match remove_shell_binding(&rc) {
                Ok(true) => println!("tnote uninstall: removed keybinding from {}", rc.display()),
                Ok(false) => {}
                Err(e) => eprintln!("tnote uninstall: could not update {}: {}", rc.display(), e),
            }
        }
    }

    println!("tnote uninstall: complete.");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── add_source_line ───────────────────────────────────────────────────────

    #[test]
    fn add_source_line_creates_file_if_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        add_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.contains(&format!("source-file {}", src.display())));
    }

    #[test]
    fn add_source_line_appends_to_existing_content() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, "set -g mouse on\n").unwrap();
        add_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.starts_with("set -g mouse on\n"));
        assert!(content.contains(&format!("source-file {}", src.display())));
    }

    #[test]
    fn add_source_line_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        add_source_line(&conf, &src).unwrap();
        add_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        let count = content
            .lines()
            .filter(|l| l.trim() == format!("source-file {}", src.display()))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn add_source_line_adds_newline_before_if_file_lacks_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, "set -g mouse on").unwrap(); // no trailing newline
        add_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        // The source line should be on its own line
        assert!(content.contains(&format!("\nsource-file {}", src.display())));
    }

    // ── remove_source_line ────────────────────────────────────────────────────

    #[test]
    fn remove_source_line_removes_the_line() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        fs::write(
            &conf,
            format!("set -g mouse on\nsource-file {}\n", src.display()),
        )
        .unwrap();
        remove_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(!content.contains(&format!("source-file {}", src.display())));
        assert!(content.contains("set -g mouse on"));
    }

    #[test]
    fn remove_source_line_noop_when_line_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, "set -g mouse on\n").unwrap();
        remove_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert_eq!(content, "set -g mouse on\n");
    }

    #[test]
    fn remove_source_line_ok_when_file_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        // conf does not exist — should not error
        remove_source_line(&conf, &src).unwrap();
    }

    #[test]
    fn remove_source_line_preserves_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, format!("a\nsource-file {}\nb\n", src.display())).unwrap();
        remove_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.ends_with('\n'));
        assert_eq!(content, "a\nb\n");
    }

    // ── which ─────────────────────────────────────────────────────────────────

    #[test]
    fn which_finds_sh() {
        assert!(which("sh"));
    }

    #[test]
    fn which_returns_false_for_nonexistent() {
        assert!(!which("tnote_nonexistent_cmd_xyz"));
    }

    // ── detect_shell ──────────────────────────────────────────────────────────

    #[test]
    fn detect_shell_returns_some() {
        // $SHELL is set in most test environments
        if std::env::var("SHELL").is_ok() {
            assert!(detect_shell().is_some());
        }
    }

    // ── shell_rc ──────────────────────────────────────────────────────────────

    #[test]
    fn shell_rc_zsh_returns_zshrc() {
        if let Some(home) = home_dir() {
            assert_eq!(shell_rc("zsh"), Some(home.join(".zshrc")));
        }
    }

    #[test]
    fn shell_rc_bash_returns_bashrc() {
        if let Some(home) = home_dir() {
            assert_eq!(shell_rc("bash"), Some(home.join(".bashrc")));
        }
    }

    #[test]
    fn shell_rc_unknown_returns_none() {
        assert_eq!(shell_rc("powershell"), None);
    }

    // ── shell_binding ─────────────────────────────────────────────────────────

    #[test]
    fn shell_binding_zsh() {
        let b = shell_binding("zsh", "t").unwrap();
        assert!(b.contains("bindkey"));
        assert!(b.contains("\\C-t"));
        assert!(b.contains("TMUX"));
        assert!(b.contains("_tnote_toggle"));
        assert!(b.contains("_tnote_precmd"));
        assert!(b.contains(SHELL_END_MARKER));
    }

    #[test]
    fn shell_binding_bash() {
        let b = shell_binding("bash", "t").unwrap();
        assert!(b.contains("bind"));
        assert!(b.contains("\\C-t"));
        assert!(b.contains("TMUX"));
        assert!(b.contains("_tnote_toggle"));
        assert!(b.contains("_tnote_precmd"));
        assert!(b.contains(SHELL_END_MARKER));
    }

    #[test]
    fn shell_binding_fish() {
        let b = shell_binding("fish", "t").unwrap();
        assert!(b.contains("bind \\ct"));
        assert!(b.contains("TMUX"));
        assert!(b.contains("_tnote_toggle"));
        assert!(b.contains("_tnote_precmd"));
        assert!(b.contains(SHELL_END_MARKER));
    }

    #[test]
    fn remove_shell_block_removes_end_marked_block() {
        let content = format!(
            "before\n{}\n_tnote_toggle() {{}}\nbindkey stuff\n{}\nafter\n",
            SHELL_MARKER, SHELL_END_MARKER
        );
        let result = remove_shell_block(&content);
        assert_eq!(result, "before\nafter\n");
        assert!(!result.contains("_tnote_toggle"));
        assert!(!result.contains(SHELL_END_MARKER));
    }

    #[test]
    fn shell_binding_unknown_returns_none() {
        assert!(shell_binding("powershell", "t").is_none());
    }

    // ── remove_shell_block ────────────────────────────────────────────────────

    #[test]
    fn remove_shell_block_removes_marker_and_next_line() {
        let content = format!(
            "alias ls='ls -la'\n{}\nbindkey -s '\\C-t' 'tnote\\n'\nexport FOO=1\n",
            SHELL_MARKER
        );
        let result = remove_shell_block(&content);
        assert_eq!(result, "alias ls='ls -la'\nexport FOO=1\n");
    }

    #[test]
    fn remove_shell_block_noop_when_no_marker() {
        let content = "alias ls='ls -la'\nexport FOO=1\n";
        assert_eq!(remove_shell_block(content), content);
    }

    #[test]
    fn remove_shell_block_handles_marker_at_end() {
        let content = format!("alias ls='ls -la'\n{}\nbindkey stuff\n", SHELL_MARKER);
        let result = remove_shell_block(&content);
        assert_eq!(result, "alias ls='ls -la'\n");
    }

    #[test]
    fn remove_shell_block_empty_content() {
        assert_eq!(remove_shell_block(""), "");
    }

    // ── add_shell_binding / remove_shell_binding ──────────────────────────────

    #[test]
    fn add_shell_binding_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".zshrc");
        add_shell_binding(
            &rc,
            "# tnote keybinding — managed by 'tnote setup' / 'tnote uninstall'\nbindkey stuff",
        )
        .unwrap();
        let content = fs::read_to_string(&rc).unwrap();
        assert!(content.contains(SHELL_MARKER));
        assert!(content.contains("bindkey stuff"));
    }

    #[test]
    fn add_shell_binding_appends_to_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".zshrc");
        fs::write(&rc, "export PATH=/usr/bin\n").unwrap();
        add_shell_binding(
            &rc,
            "# tnote keybinding — managed by 'tnote setup' / 'tnote uninstall'\nbindkey stuff",
        )
        .unwrap();
        let content = fs::read_to_string(&rc).unwrap();
        assert!(content.starts_with("export PATH=/usr/bin\n"));
        assert!(content.contains("bindkey stuff"));
    }

    #[test]
    fn add_shell_binding_replaces_existing_block() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".zshrc");
        fs::write(
            &rc,
            format!("before\n{}\nold binding\nafter\n", SHELL_MARKER),
        )
        .unwrap();
        add_shell_binding(
            &rc,
            "# tnote keybinding — managed by 'tnote setup' / 'tnote uninstall'\nnew binding",
        )
        .unwrap();
        let content = fs::read_to_string(&rc).unwrap();
        assert!(!content.contains("old binding"));
        assert!(content.contains("new binding"));
        assert!(content.contains("before"));
        assert!(content.contains("after"));
    }

    #[test]
    fn remove_shell_binding_removes_block() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".zshrc");
        fs::write(
            &rc,
            format!("before\n{}\nbindkey stuff\nafter\n", SHELL_MARKER),
        )
        .unwrap();
        assert!(remove_shell_binding(&rc).unwrap());
        let content = fs::read_to_string(&rc).unwrap();
        assert!(!content.contains(SHELL_MARKER));
        assert!(!content.contains("bindkey stuff"));
        assert!(content.contains("before"));
        assert!(content.contains("after"));
    }

    #[test]
    fn remove_shell_binding_returns_false_when_no_block() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".zshrc");
        fs::write(&rc, "export FOO=1\n").unwrap();
        assert!(!remove_shell_binding(&rc).unwrap());
    }

    #[test]
    fn remove_shell_binding_returns_false_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let rc = tmp.path().join(".zshrc");
        assert!(!remove_shell_binding(&rc).unwrap());
    }
}
