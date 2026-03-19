use std::process::Command;
use std::thread;
use std::time::Duration;

/// Check that tmux is installed and >= 3.2. Returns Err with a user-friendly message.
pub fn check_version() -> Result<(), String> {
    let output = Command::new("tmux")
        .arg("-V")
        .output()
        .map_err(|_| "tmux not found. tnote requires tmux 3.2+".to_string())?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let raw = raw.trim();

    // Format: "tmux 3.3a", "tmux next-3.4", "tmux 3.2"
    let ver_part = raw
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("Could not parse tmux version from: {}", raw))?;

    // Strip leading "next-" or similar prefixes
    let ver_digits: String = ver_part
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    let mut parts = ver_digits.splitn(2, '.');
    let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    if major > 3 || (major == 3 && minor >= 2) {
        Ok(())
    } else {
        Err(format!(
            "tnote requires tmux 3.2+, but found {}. Please upgrade tmux.",
            raw
        ))
    }
}

/// True when running inside a tmux session.
pub fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Returns the current window key as "<session>-<window_index>", e.g. "work-0".
pub fn window_key() -> Option<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#S-#I"])
        .output()
        .ok()?;
    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// Search all panes for one titled "tnote-popup" and return its pane ID (e.g. "%42").
pub fn find_popup_pane_id() -> Option<String> {
    let output = Command::new("tmux")
        .args(["list-panes", "-a", "-F", "#{pane_id} #{pane_title}"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some((id, title)) = line.split_once(' ') {
            if title == "tnote-popup" {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Send a single tmux key/sequence to the given pane.
fn send_key(pane_id: &str, key: &str) {
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", pane_id, key])
        .output();
    thread::sleep(Duration::from_millis(60));
}

/// Send save+quit keystrokes appropriate for the configured editor, then kill the popup.
pub fn save_and_close(pane_id: &str, editor: &str) {
    // Match on the basename of the editor path
    let base = std::path::Path::new(editor)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(editor);

    match base {
        e if e.ends_with("nvim") || e.ends_with("vim") || e == "vi" => {
            send_key(pane_id, "Escape");
            send_key(pane_id, ":wq");
            send_key(pane_id, "Enter");
        }
        e if e.ends_with("nano") => {
            send_key(pane_id, "C-o");
            send_key(pane_id, "Enter");
            send_key(pane_id, "C-x");
        }
        e if e.ends_with("micro") => {
            send_key(pane_id, "C-s");
            send_key(pane_id, "C-q");
        }
        e if e.ends_with("hx") || e.ends_with("helix") => {
            send_key(pane_id, "Escape");
            send_key(pane_id, ":wq");
            send_key(pane_id, "Enter");
        }
        _ => {
            // Unknown editor — kill popup immediately
        }
    }

    thread::sleep(Duration::from_millis(100));
    kill_popup();
}

/// Kill the current tmux popup.
pub fn kill_popup() {
    let _ = Command::new("tmux").arg("kill-popup").output();
}

/// Rename the current tmux window.
pub fn rename_window(name: &str) {
    let _ = Command::new("tmux")
        .args(["rename-window", name])
        .output();
}

/// Read the popup state file to get the active popup's pane ID.
/// Returns None if no popup is recorded or the pane no longer exists.
pub fn read_popup_state(state_file: &std::path::Path) -> Option<String> {
    let pane_id = std::fs::read_to_string(state_file).ok()?;
    let pane_id = pane_id.trim().to_string();
    if pane_id.is_empty() {
        return None;
    }
    // Verify the pane still exists (guards against stale state files)
    let output = Command::new("tmux")
        .args(["list-panes", "-a", "-F", "#{pane_id}"])
        .output()
        .ok()?;
    let panes = String::from_utf8_lossy(&output.stdout);
    if panes.lines().any(|l| l.trim() == pane_id) {
        Some(pane_id)
    } else {
        let _ = std::fs::remove_file(state_file);
        None
    }
}

/// Open a tmux popup anchored top-right that sets its pane title and launches the editor.
/// Writes $TMUX_PANE to state_file on open and removes it on exit so toggle detection works.
pub fn open_popup(file: &str, label: &str, width: &str, height: &str, editor: &str, state_file: &str) {
    let quoted_file = shell_quote(file);
    let quoted_state = shell_quote(state_file);

    // The popup shell:
    //   1. Records its own pane ID so tnote can target it for save+quit
    //   2. Sets the pane title via OSC 2 (belt-and-suspenders detection)
    //   3. Opens the editor
    //   4. Cleans up the state file on exit
    let popup_cmd = format!(
        "printf '%s' \"$TMUX_PANE\" > {state}; \
         printf '\\033]2;tnote-popup\\033\\\\'; \
         {editor} {file}; \
         rm -f {state}",
        state = quoted_state,
        editor = editor,
        file = quoted_file,
    );

    let title = format!(" tnote: {} ", label);

    let _ = Command::new("tmux")
        .args([
            "popup", "-E",
            "-x", "100%",
            "-y", "0",
            "-w", width,
            "-h", height,
            "-T", &title,
            &popup_cmd,
        ])
        .status();

    // Clean up state file in case the popup was killed without running the shell's rm
    let _ = std::fs::remove_file(state_file);
}

/// Wrap a string in single quotes, escaping any embedded single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
