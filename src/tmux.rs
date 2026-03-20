use std::io;
use std::path::Path;
use std::process::Command;

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// True when running inside a tmux session.
pub fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Returns the current tmux window's note key, e.g. "tmux-work+0".
pub fn window_key() -> Option<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}+#{window_index}"])
        .output()
        .ok()?;
    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if key.is_empty() { None } else { Some(format!("tmux-{}", key)) }
}

/// Returns the set of all live tmux window keys (e.g. "tmux-work+0") across all sessions.
pub fn live_window_keys() -> std::collections::HashSet<String> {
    let Ok(output) = Command::new("tmux")
        .args(["list-windows", "-a", "-F", "#{session_name}+#{window_index}"])
        .output()
    else {
        return std::collections::HashSet::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| format!("tmux-{}", l.trim()))
        .collect()
}

/// Open the note file in a tmux display-popup anchored to the top-right of
/// the tmux window, which spans the full physical terminal regardless of pane layout.
pub fn open_popup(file: &Path, label: &str, width: u16, height: u16) -> io::Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let shell_cmd = format!("{} {}", editor, shell_escape(&file.to_string_lossy()));
    let status = Command::new("tmux")
        .args([
            "display-popup",
            "-x", "R",
            "-y", "T",
            "-w", &width.to_string(),
            "-h", &height.to_string(),
            "-b", "rounded",
            "-T", &format!(" tnote: {} ", label),
            "-E",
            &shell_cmd,
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "tmux display-popup failed"));
    }
    Ok(())
}

/// Rename the current tmux window.
pub fn rename_window(name: &str) {
    let _ = Command::new("tmux")
        .args(["rename-window", name])
        .output();
}
