use crate::config::Config;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn which(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn user_tmux_conf() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".tmux.conf"))
}

pub fn add_source_line(user_conf: &Path, source_path: &Path) -> std::io::Result<()> {
    let line = format!("source-file {}", source_path.display());
    let content = fs::read_to_string(user_conf).unwrap_or_default();
    if content.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(user_conf)?;
    if !content.is_empty() && !content.ends_with('\n') {
        writeln!(f)?;
    }
    writeln!(f, "{}", line)?;
    Ok(())
}

pub fn remove_source_line(user_conf: &Path, source_path: &Path) -> std::io::Result<()> {
    let Ok(content) = fs::read_to_string(user_conf) else { return Ok(()); };
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
        eprintln!("tnote setup: failed to create {}: {}", config.dir.display(), e);
        std::process::exit(1);
    }

    let meta_dir = config.dir.join("meta");
    if let Err(e) = fs::create_dir_all(&meta_dir) {
        eprintln!("tnote setup: failed to create {}: {}", meta_dir.display(), e);
        std::process::exit(1);
    }

    // Unbind the old key if it differs from the new one
    let tmux_conf_path = meta_dir.join("tmux.conf");
    if let Ok(old) = fs::read_to_string(&tmux_conf_path) {
        if let Some(old_key) = old.lines()
            .find_map(|l| l.strip_prefix("bind-key ").and_then(|r| r.split_whitespace().next()))
        {
            if old_key != config.key {
                let _ = Command::new("tmux").args(["unbind-key", old_key]).status();
            }
        }
    }

    let key = &config.key;
    let tmux_conf = format!(
        "# tnote key bindings — managed by 'tnote setup' / 'tnote uninstall'\n\
         unbind-key {key}\n\
         bind-key {key} run-shell 'tnote'\n\
         set -s command-alias[100] 'tnote=run-shell tnote %*'\n",
        key = key,
    );

    if let Err(e) = fs::write(&tmux_conf_path, &tmux_conf) {
        eprintln!("tnote setup: failed to write {}: {}", tmux_conf_path.display(), e);
        std::process::exit(1);
    }
    println!("tnote setup: wrote {}", tmux_conf_path.display());

    // Source it into the live tmux session
    let status = Command::new("tmux")
        .args(["source-file", &tmux_conf_path.to_string_lossy()])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("tnote setup: sourced bindings into live tmux session");
        }
        Ok(_) => {
            eprintln!("tnote setup: tmux source-file failed (is tmux running?)");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("tnote setup: could not run tmux: {}", e);
            std::process::exit(1);
        }
    }

    // Persist across tmux restarts by adding source-file line to ~/.tmux.conf
    if let Some(user_conf) = user_tmux_conf() {
        match add_source_line(&user_conf, &tmux_conf_path) {
            Ok(_) => println!("tnote setup: added source-file line to {}", user_conf.display()),
            Err(e) => eprintln!("tnote setup: could not update {}: {}", user_conf.display(), e),
        }
    }

    println!("tnote setup: setup complete. Binding: prefix+{} opens/closes tnote popup", key);
}

pub fn uninstall(config: &Config) {
    let tmux_conf_path = config.dir.join("meta").join("tmux.conf");
    let key = &config.key;
    let cleared = format!(
        "# tnote key bindings — cleared by 'tnote uninstall'\nunbind-key {}\nset -su command-alias[100]\n",
        key
    );

    match fs::write(&tmux_conf_path, &cleared) {
        Ok(_) => {
            let _ = Command::new("tmux")
                .args(["source-file", &tmux_conf_path.to_string_lossy()])
                .status();
            println!("tnote uninstall: cleared bindings from live tmux session");
        }
        Err(_) => {
            let _ = Command::new("tmux").args(["unbind-key", key]).status();
        }
    }

    // Remove source-file line from ~/.tmux.conf
    if let Some(user_conf) = user_tmux_conf() {
        match remove_source_line(&user_conf, &tmux_conf_path) {
            Ok(_) => println!("tnote uninstall: removed source-file line from {}", user_conf.display()),
            Err(e) => eprintln!("tnote uninstall: could not update {}: {}", user_conf.display(), e),
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
        let src  = tmp.path().join("meta/tmux.conf");
        add_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.contains(&format!("source-file {}", src.display())));
    }

    #[test]
    fn add_source_line_appends_to_existing_content() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src  = tmp.path().join("meta/tmux.conf");
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
        let src  = tmp.path().join("meta/tmux.conf");
        add_source_line(&conf, &src).unwrap();
        add_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        let count = content.lines()
            .filter(|l| l.trim() == format!("source-file {}", src.display()))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn add_source_line_adds_newline_before_if_file_lacks_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src  = tmp.path().join("meta/tmux.conf");
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
        let src  = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, format!("set -g mouse on\nsource-file {}\n", src.display())).unwrap();
        remove_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(!content.contains(&format!("source-file {}", src.display())));
        assert!(content.contains("set -g mouse on"));
    }

    #[test]
    fn remove_source_line_noop_when_line_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src  = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, "set -g mouse on\n").unwrap();
        remove_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert_eq!(content, "set -g mouse on\n");
    }

    #[test]
    fn remove_source_line_ok_when_file_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src  = tmp.path().join("meta/tmux.conf");
        // conf does not exist — should not error
        remove_source_line(&conf, &src).unwrap();
    }

    #[test]
    fn remove_source_line_preserves_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        let conf = tmp.path().join(".tmux.conf");
        let src  = tmp.path().join("meta/tmux.conf");
        fs::write(&conf, format!("a\nsource-file {}\nb\n", src.display())).unwrap();
        remove_source_line(&conf, &src).unwrap();
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.ends_with('\n'));
        assert_eq!(content, "a\nb\n");
    }
}
