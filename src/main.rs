mod config;
mod install;
mod notes;
mod render;
mod tmux;
mod viewer;

use clap::{Parser, Subcommand};
use config::Config;
use notes::Notes;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "tnote",
    about = "Per-tmux-window notepad",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Name this window's note (also renames the tmux window)
    Name {
        /// Name to assign
        name: String,
    },
    /// Print note contents inline
    Show,
    /// Clear this window's note
    Clear,
    /// List all notes with line counts
    List,
    /// Print the note file path
    Path,
    /// Install toggle.sh and bind Ctrl+N in tmux
    Install,
    /// Remove toggle.sh and unbind Ctrl+N in tmux
    Uninstall,
    /// Open a rendered, interactive view of the current note
    View,
    /// Show usage
    Help,
}

fn main() {
    // Require tmux 3.2+ on every invocation
    if let Err(e) = tmux::check_version() {
        eprintln!("tnote: {}", e);
        std::process::exit(1);
    }

    let cli = Cli::parse();
    let config = Config::from_env();
    let notes = Notes::new(config.dir.clone());

    if let Err(e) = notes.ensure_dir() {
        eprintln!("tnote: failed to create notes dir {}: {}", config.dir.display(), e);
        std::process::exit(1);
    }

    match &cli.command {
        None => cmd_open(&config, &notes),
        Some(Cmd::Name { name }) => cmd_name(&config, &notes, name),
        Some(Cmd::Show) => cmd_show(&notes),
        Some(Cmd::View) => cmd_view(&config, &notes),
        Some(Cmd::Clear) => cmd_clear(&notes),
        Some(Cmd::List) => cmd_list(&notes),
        Some(Cmd::Path) => cmd_path(&notes),
        Some(Cmd::Install) => install::run(&config),
        Some(Cmd::Uninstall) => install::uninstall(&config),
        Some(Cmd::Help) => print_help(),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns (window_key, note_file_path) for the current window.
fn current_note(notes: &Notes) -> (String, PathBuf) {
    let key = if tmux::is_in_tmux() {
        tmux::window_key().unwrap_or_else(|| format!("pid-{}", std::process::id()))
    } else {
        format!("pid-{}", std::process::id())
    };
    let file = notes.file_for_key(&key);
    (key, file)
}

// ── Subcommand implementations ────────────────────────────────────────────────

fn cmd_open(config: &Config, notes: &Notes) {
    let (key, file) = current_note(notes);
    let label = notes.label_for_key(&key);

    // Ensure the note file exists so the editor doesn't complain
    if !file.exists() {
        let _ = fs::write(&file, "");
    }

    if !tmux::is_in_tmux() {
        eprintln!("tnote: not inside a tmux session — opening inline instead");
        let _ = std::process::Command::new(&config.editor).arg(&file).status();
        return;
    }

    // Toggle: if a tnote popup is open, save & close it.
    // Primary: read the state file written by the popup shell ($TMUX_PANE).
    // Fallback: scan list-panes by title (works when called from outside the popup).
    let state_file = config.dir.join(".popup");
    let popup_pane = tmux::read_popup_state(&state_file)
        .or_else(|| tmux::find_popup_pane_id());

    if popup_pane.is_some() {
        // The popup is a viewer — kill it directly (no unsaved editor state).
        tmux::kill_popup();
        let _ = std::fs::remove_file(&state_file);
        return;
    }

    // Open a new popup anchored top-right running the viewer.
    let state_str = state_file.to_string_lossy().into_owned();
    tmux::open_popup(&label, &config.width, &config.height, &state_str);
}

fn cmd_name(config: &Config, notes: &Notes, name: &str) {
    let (key, _) = current_note(notes);

    match notes.name_window(&key, name) {
        Ok(migrated) => {
            if migrated {
                println!("tnote: migrated existing notes → {}", name);
            }
            if tmux::is_in_tmux() {
                tmux::rename_window(name);
            }
            println!("tnote: window note named '{}'", name);
        }
        Err(e) => {
            eprintln!("tnote: error naming note: {}", e);
            std::process::exit(1);
        }
    }

    let _ = config; // suppress unused warning
}

fn cmd_show(notes: &Notes) {
    let (key, file) = current_note(notes);
    let label = notes.label_for_key(&key);

    let non_empty = file.exists() && file.metadata().map(|m| m.len() > 0).unwrap_or(false);

    if non_empty {
        println!("── tnote: {} ──", label);
        match fs::read_to_string(&file) {
            Ok(content) => print!("{}", content),
            Err(e) => eprintln!("tnote: error reading file: {}", e),
        }
        println!("──────────────────────");
    } else {
        println!("tnote: (empty) [{}]", label);
    }
}

fn cmd_view(config: &Config, notes: &Notes) {
    let (_, file) = current_note(notes);

    if !file.exists() {
        let _ = fs::write(&file, "");
    }

    viewer::run(&file, &config.editor);
}

fn cmd_clear(notes: &Notes) {
    let (key, file) = current_note(notes);
    let label = notes.label_for_key(&key);

    match fs::write(&file, "") {
        Ok(_) => println!("tnote: cleared '{}'", label),
        Err(e) => {
            eprintln!("tnote: error clearing note: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_list(notes: &Notes) {
    let (_, current_file) = current_note(notes);

    println!("tnote notes:");
    match notes.list_notes() {
        Ok(list) => {
            if list.is_empty() {
                println!("  (no notes yet)");
                return;
            }
            for (display, lines, path) in list {
                let marker = if path == current_file { " ◀" } else { "" };
                println!("  {:<24}  {} lines{}", display, lines, marker);
            }
        }
        Err(e) => {
            eprintln!("tnote: error listing notes: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_path(notes: &Notes) {
    let (_, file) = current_note(notes);
    println!("{}", file.display());
}

fn print_help() {
    println!(
        "tnote — per-tmux-window notepad

USAGE:
  tnote                  Open/toggle viewer popup for the current window
  tnote view             Open the interactive viewer inline (e=edit, q=quit)
  tnote name <name>      Name this window's note (also renames the tmux window)
  tnote show             Print note contents inline (plain text)
  tnote clear            Clear this window's note
  tnote list             List all notes with line counts
  tnote path             Print the note file path
  tnote install          Write toggle.sh and bind Ctrl+N in tmux
  tnote uninstall        Remove toggle.sh and unbind Ctrl+N in tmux
  tnote help             Show this help

ENVIRONMENT:
  TNOTE_DIR              Note storage directory  (default: ~/.tnotes)
  TNOTE_EDITOR           Editor to use           (default: vim)
  TNOTE_WIDTH            Popup width in columns  (default: 62)
  TNOTE_HEIGHT           Popup height in lines   (default: 22)

Each tmux window gets its own note, keyed to <session>-<window> (e.g. work-0).
Named notes are shared across windows that point to the same name.

Files:
  ~/.tnotes/<session>-<index>.md   Unnamed window notes
  ~/.tnotes/named-<name>.md        Named notes
  ~/.tnotes/<session>-<index>.link Pointer from window key to name"
    );
}
