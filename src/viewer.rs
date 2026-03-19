use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

use crate::render;

/// Run the interactive note viewer for the given file.
///
/// Keys:
///   e / E  → open $EDITOR on the note, then redraw on return
///   q / Q / Esc → quit
pub fn run(file: &Path, editor: &str) {
    set_cbreak();

    // Restore the terminal unconditionally when this scope exits (panic-safe).
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            restore_terminal();
        }
    }
    let _guard = Guard;

    loop {
        redraw(file);

        match read_key() {
            Some(b'q') | Some(b'Q') | Some(27 /* Esc */) => break,
            Some(b'e') | Some(b'E') => {
                restore_terminal();
                let _ = Command::new(editor).arg(file).status();
                set_cbreak();
            }
            _ => {}
        }
    }

    // Clear screen so the popup shell exits cleanly.
    print!("\x1b[2J\x1b[H");
    let _ = std::io::stdout().flush();
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn redraw(file: &Path) {
    // Move cursor to top-left and clear screen.
    print!("\x1b[2J\x1b[H");

    let content = std::fs::read_to_string(file).unwrap_or_default();
    if content.trim().is_empty() {
        println!("\x1b[2m(empty — press e to start editing)\x1b[0m");
    } else {
        print!("{}", render::render_note(&content));
    }

    println!("\n\x1b[2m  e  edit   q  quit\x1b[0m");
    let _ = std::io::stdout().flush();
}

/// Read one raw byte from /dev/tty (works even when stdin is redirected).
fn read_key() -> Option<u8> {
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .open("/dev/tty")
        .ok()?;
    let mut buf = [0u8; 1];
    tty.read_exact(&mut buf).ok()?;
    Some(buf[0])
}

/// Put the terminal in cbreak mode (single-char reads, no echo).
fn set_cbreak() {
    let _ = Command::new("stty").args(["cbreak", "-echo"]).status();
}

/// Restore normal terminal settings.
fn restore_terminal() {
    let _ = Command::new("stty").arg("sane").status();
}
