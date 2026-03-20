use crate::config::Config;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn user_tmux_conf() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".tmux.conf"))
}

fn add_source_line(user_conf: &Path, source_path: &Path) -> std::io::Result<()> {
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

fn remove_source_line(user_conf: &Path, source_path: &Path) -> std::io::Result<()> {
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
        eprintln!("tnote: failed to create {}: {}", config.dir.display(), e);
        std::process::exit(1);
    }

    let meta_dir = config.dir.join("meta");
    if let Err(e) = fs::create_dir_all(&meta_dir) {
        eprintln!("tnote: failed to create {}: {}", meta_dir.display(), e);
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
         bind-key {key} run-shell 'tnote'\n",
        key = key,
    );

    if let Err(e) = fs::write(&tmux_conf_path, &tmux_conf) {
        eprintln!("tnote: failed to write {}: {}", tmux_conf_path.display(), e);
        std::process::exit(1);
    }
    println!("tnote: wrote {}", tmux_conf_path.display());

    // Source it into the live tmux session
    let status = Command::new("tmux")
        .args(["source-file", &tmux_conf_path.to_string_lossy()])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("tnote: sourced bindings into live tmux session");
        }
        Ok(_) => {
            eprintln!("tnote: tmux source-file failed (is tmux running?)");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("tnote: could not run tmux: {}", e);
            std::process::exit(1);
        }
    }

    // Persist across tmux restarts by adding source-file line to ~/.tmux.conf
    if let Some(user_conf) = user_tmux_conf() {
        match add_source_line(&user_conf, &tmux_conf_path) {
            Ok(_) => println!("tnote: added source-file line to {}", user_conf.display()),
            Err(e) => eprintln!("tnote: could not update {}: {}", user_conf.display(), e),
        }
    }

    println!("tnote: setup complete. Binding: prefix+{} opens/closes tnote popup", key);
}

pub fn uninstall(config: &Config) {
    let tmux_conf_path = config.dir.join("meta").join("tmux.conf");
    let key = &config.key;
    let cleared = format!(
        "# tnote key bindings — cleared by 'tnote uninstall'\nunbind-key {}\n",
        key
    );

    match fs::write(&tmux_conf_path, &cleared) {
        Ok(_) => {
            let _ = Command::new("tmux")
                .args(["source-file", &tmux_conf_path.to_string_lossy()])
                .status();
            println!("tnote: cleared bindings from live tmux session");
        }
        Err(_) => {
            let _ = Command::new("tmux").args(["unbind-key", key]).status();
        }
    }

    // Remove source-file line from ~/.tmux.conf
    if let Some(user_conf) = user_tmux_conf() {
        match remove_source_line(&user_conf, &tmux_conf_path) {
            Ok(_) => println!("tnote: removed source-file line from {}", user_conf.display()),
            Err(e) => eprintln!("tnote: could not update {}: {}", user_conf.display(), e),
        }
    }

    println!("tnote: uninstall complete.");
}
