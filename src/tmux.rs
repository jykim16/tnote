use std::process::Command;

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

/// Open a tmux popup anchored top-right that runs the tnote viewer.
/// Writes $TMUX_PANE to state_file on open and removes it on exit so toggle detection works.
pub fn open_popup(label: &str, width: &str, height: &str, state_file: &str) {
    let quoted_state = shell_quote(state_file);

    // Resolve the path to the running tnote binary so the popup runs the same build.
    let tnote_bin = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "tnote".into());
    let quoted_tnote = shell_quote(&tnote_bin);

    // The popup shell:
    //   1. Records its own pane ID so tnote can target it for toggle detection
    //   2. Sets the pane title via OSC 2 (belt-and-suspenders detection)
    //   3. Runs the viewer (interactive; exits on q/Esc)
    //   4. Cleans up the state file on exit
    let popup_cmd = format!(
        "printf '%s' \"$TMUX_PANE\" > {state}; \
         printf '\\033]2;tnote-popup\\033\\\\'; \
         {tnote} view; \
         rm -f {state}",
        state = quoted_state,
        tnote = quoted_tnote,
    );

    let title = format!(" ≡ tnote  {} ", label);

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
