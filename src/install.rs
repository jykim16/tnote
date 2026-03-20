use crate::config::Config;
use std::fs;
use std::process::Command;

pub fn run(config: &Config) {
    if let Err(e) = fs::create_dir_all(&config.dir) {
        eprintln!("tnote: failed to create {}: {}", config.dir.display(), e);
        std::process::exit(1);
    }

    // Write ~/.tnote/meta/tmux.conf
    let meta_dir = config.dir.join("meta");
    if let Err(e) = fs::create_dir_all(&meta_dir) {
        eprintln!("tnote: failed to create {}: {}", meta_dir.display(), e);
        std::process::exit(1);
    }
    let tmux_conf_path = meta_dir.join("tmux.conf");
    let tmux_conf = format!(
        "# tnote key bindings — managed by 'tnote install' / 'tnote uninstall'\n\
         unbind-key -n C-n\n\
         unbind-key C-n\n\
         bind-key -n C-n display-popup -x R -y T -w 64 -h 24 -b rounded -E 'tnote popup'\n"
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

    // Bind C-n in the popup key table (tmux 3.4+) so pressing C-n inside
    // the popup closes it. Ignore failures on older tmux without this table.
    let _ = Command::new("tmux")
        .args(["bind-key", "-T", "popup", "C-n", "display-popup", "-C"])
        .status();

    println!("tnote: install complete. Binding: C-n opens/closes tnote popup");
    println!("tnote: to persist across tmux restarts, add this one line to ~/.tmux.conf:");
    println!("         source-file {}", tmux_conf_path.display());
}

pub fn uninstall(config: &Config) {
    let tmux_conf_path = config.dir.join("meta").join("tmux.conf");
    let cleared = "# tnote key bindings — cleared by 'tnote uninstall'\nunbind-key -n C-n\n";

    match fs::write(&tmux_conf_path, cleared) {
        Ok(_) => {
            let _ = Command::new("tmux")
                .args(["source-file", &tmux_conf_path.to_string_lossy()])
                .status();
            println!("tnote: cleared bindings from live tmux session");
        }
        Err(_) => {
            let _ = Command::new("tmux").args(["unbind-key", "C-n"]).status();
        }
    }

    println!("tnote: uninstall complete.");
    println!("tnote: if you added source-file to ~/.tmux.conf, you can remove that line now.");
}
