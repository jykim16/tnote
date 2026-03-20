mod config;
mod editor;
mod install;
mod notes;
mod tmux;

use clap::{Parser, Subcommand, ValueEnum};
use config::Config;
use std::io::{self, Write};
use notes::Notes;
use owo_colors::{OwoColorize, Style, Stream::Stdout, Stream::Stderr};
use std::fs;
use std::path::PathBuf;

#[derive(ValueEnum, Clone)]
pub enum ClearScope {
    /// Remove files without a recognized prefix (tmux-, shell-, named-)
    Unprefixed,
    /// Remove all named notes
    Named,
    /// Remove all tmux-keyed notes regardless of window liveness
    Tmux,
    /// Remove all notes
    All,
}

#[derive(Parser)]
#[command(name = "tnote", about = "Per-tmux-window notepad", disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Name this window's note (also renames the tmux window)
    Name { name: String },
    /// Print note contents inline
    Show,
    /// Remove notes not tied to a running process or window
    Clean {
        /// Also remove notes in the given category: unprefixed, named, tmux, all
        #[arg(long, value_name = "CATEGORY")]
        all: Option<ClearScope>,
        /// Print what would be removed without removing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// List all notes with line counts
    List,
    /// Print the note file path
    Path,
    /// Configure editor, key binding, and dimensions, then install
    Setup,
    /// Remove the tmux keybinding
    Uninstall,
    /// Show usage
    Help,
    /// Internal: run editor inline when already inside a tmux display-popup
    #[command(hide = true)]
    Popup { window_key: Option<String> },
}

fn main() {
    let cli = Cli::parse();
    let config = Config::from_env();
    let notes = Notes::new(config.dir.clone());

    if let Err(e) = notes.ensure_dir() {
        eprintln!("tnote: failed to create notes dir {}: {}", config.dir.display(), e);
        std::process::exit(1);
    }

    match &cli.command {
        None => cmd_open(&config, &notes),
        Some(Cmd::Name { name }) => cmd_name(&notes, name),
        Some(Cmd::Show) => cmd_show(&notes),
        Some(Cmd::Clean { all, dry_run }) => cmd_clean(&notes, all.as_ref(), *dry_run),
        Some(Cmd::List) => cmd_list(&notes),
        Some(Cmd::Path) => cmd_path(&notes),
        Some(Cmd::Setup) => cmd_setup(&config),
        Some(Cmd::Uninstall) => install::uninstall(&config),
        Some(Cmd::Help) => print_help(),
        Some(Cmd::Popup { window_key }) => {
            let key = window_key.as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| std::env::var("TNOTE_WINDOW_KEY").ok())
                .or_else(|| tmux::window_key())
                .unwrap_or_else(|| shell_session_key());
            cmd_popup_inline(&config, &notes, &key);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn current_note(notes: &Notes) -> (String, PathBuf) {
    let key = if tmux::is_in_tmux() {
        tmux::window_key().unwrap_or_else(|| shell_session_key())
    } else {
        shell_session_key()
    };
    let file = notes.file_for_key(&key);
    (key, file)
}

/// Returns the PID of the parent shell process (tnote's direct parent).
fn get_shell_pid() -> Option<u32> {
    let pid = std::process::id().to_string();
    std::process::Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid])
        .output()
        .ok()
        .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse::<u32>().ok())
}

/// Returns a stable key for the current terminal session by using the parent
/// process PID (the shell). Unlike our own PID, the shell's PID is constant
/// for the lifetime of the terminal session across multiple tnote invocations.
fn shell_session_key() -> String {
    get_shell_pid()
        .map(|ppid| format!("shell-{}", ppid))
        .unwrap_or_else(|| format!("pid-{}", std::process::id()))
}

// ── Subcommands ───────────────────────────────────────────────────────────────

fn cmd_open(config: &Config, notes: &Notes) {
    if tmux::is_in_tmux() && tmux::is_popup_session() {
        let _ = std::process::Command::new("tmux").args(["detach-client"]).status();
        return;
    }

    let (key, file) = current_note(notes);
    let label = notes.label_for_key(&key);

    if !file.exists() {
        let _ = fs::write(&file, "");
    }

    let result = if tmux::is_in_tmux() {
        tmux::open_popup_session(&file, &key, config)
    } else {
        editor::run(&file, &label, config.width, config.height)
    };

    if let Err(e) = result {
        eprintln!("tnote: {}", e);
        std::process::exit(1);
    }
}

fn cmd_popup_inline(_config: &Config, notes: &Notes, key: &str) {
    let file = notes.file_for_key(key);

    if !file.exists() {
        let _ = fs::write(&file, "");
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&file)
        .status();

    if let Err(e) = status {
        eprintln!("tnote: failed to run {}: {}", editor, e);
        std::process::exit(1);
    }
}

fn cmd_name(notes: &Notes, name: &str) {
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
}

fn cmd_show(notes: &Notes) {
    let (key, file) = current_note(notes);
    let label = notes.label_for_key(&key);
    let non_empty = file.exists() && file.metadata().map(|m| m.len() > 0).unwrap_or(false);

    if non_empty {
        println!("{}", format!("── tnote: {} ──", label).if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
        match fs::read_to_string(&file) {
            Ok(content) => print!("{}", content),
            Err(e) => eprintln!("tnote: {}", e.if_supports_color(Stderr, |t| t.red())),
        }
        println!("{}", "──────────────────────".if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
    } else {
        println!("tnote: (empty) [{}]", label.if_supports_color(Stdout, |t| t.dimmed()));
    }
}

fn cmd_clean(notes: &Notes, scope: Option<&ClearScope>, dry_run: bool) {
    let mut any = false;

    match notes.cleanup_orphaned(scope, dry_run) {
        Ok(removed) if !removed.is_empty() => {
            let verb = if dry_run { "would remove" } else { "removed" };
            for key in &removed {
                println!("tnote: {} note {}", verb, key.if_supports_color(Stdout, |t| t.yellow()));
            }
            println!("tnote: {} {} orphaned note(s)", verb, removed.len().if_supports_color(Stdout, |t| t.style(Style::new().yellow().bold())));
            any = true;
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("tnote: {}", e.if_supports_color(Stderr, |t| t.red()));
            std::process::exit(1);
        }
    }

    let sessions = tmux::cleanup_popup_sessions(&notes.dir, dry_run);
    for s in &sessions {
        let verb = if dry_run { "would kill" } else { "killed" };
        println!("tnote: {} popup session {}", verb, s.if_supports_color(Stdout, |t| t.yellow()));
        any = true;
    }

    if !any {
        println!("tnote: {}", "nothing to clean up".if_supports_color(Stdout, |t| t.green()));
    }
}

fn cmd_list(notes: &Notes) {
    let (_, current_file) = current_note(notes);
    match notes.list_notes() {
        Ok(list) => {
            if list.is_empty() {
                println!("tnote: (no notes yet)");
                return;
            }

            struct Row {
                category:      String,
                plain_label:   String,
                colored_label: String,
                lines:         usize,
                marker:        String,
                sources:       Vec<String>,
            }

            // Build all rows up front so we can compute global alignment widths.
            let rows: Vec<Row> = list.iter().map(|(cat, display, note_sources, lines, path)| {
                let shown = if cat == "shell" {
                    display.trim_start_matches("shell-").to_string()
                } else {
                    display.clone()
                };
                let sources: Vec<String> = note_sources.iter().map(|k| {
                    if let Some(s) = k.strip_prefix("tmux-") {
                        format!("tmux - {}", s)
                    } else {
                        format!("shell - {}", k.strip_prefix("shell-").unwrap_or(k))
                    }
                }).collect();
                let is_current = *path == current_file;
                let colored_name = if is_current {
                    format!("{}", shown.if_supports_color(Stdout, |t| t.style(Style::new().bold())))
                } else {
                    shown.clone()
                };
                let marker = if is_current {
                    format!(" {}", "◀".if_supports_color(Stdout, |t| t.style(Style::new().green().bold())))
                } else {
                    String::new()
                };
                Row {
                    category:      cat.clone(),
                    plain_label:   shown,
                    colored_label: colored_name,
                    lines:         *lines,
                    marker,
                    sources,
                }
            }).collect();

            let max_width   = rows.iter().map(|r| r.plain_label.len()).max().unwrap_or(0);
            let lines_width = rows.iter().map(|r| r.lines).max().unwrap_or(0).to_string().len();

            let categories = [
                ("named", "named"),
                ("tmux",  "tmux (session+window)"),
                ("shell", "shell (pid)"),
                ("other", "other"),
            ];
            for (cat_key, cat_label) in categories {
                let cat_rows: Vec<_> = rows.iter().filter(|r| r.category == cat_key).collect();
                if cat_rows.is_empty() { continue; }
                println!("{}", format!("{}:", cat_label).if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
                for row in cat_rows {
                    let padding = max_width - row.plain_label.len();
                    println!("  {}{}  {:>width$} lines{}",
                        row.colored_label,
                        " ".repeat(padding),
                        row.lines,
                        row.marker,
                        width = lines_width,
                    );
                    for source in &row.sources {
                        println!("    {} {}", "↳".if_supports_color(Stdout, |t| t.dimmed()), source.if_supports_color(Stdout, |t| t.dimmed()));
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("tnote: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_path(notes: &Notes) {
    let (_, file) = current_note(notes);
    println!("{}", file.display());
}

fn cmd_setup(config: &Config) {
    println!("tnote setup\n");

    let editor = prompt_editor(&config.editor);
    let key    = prompt("Key (prefix+?)", &config.key);
    let width  = prompt_u16("Popup width",  config.width);
    let height = prompt_u16("Popup height", config.height);

    let new_config = Config {
        dir:    config.dir.clone(),
        editor,
        key,
        width,
        height,
    };

    if let Err(e) = new_config.save() {
        eprintln!("tnote: failed to save config: {}", e);
        std::process::exit(1);
    }

    install::run(&new_config);
}

fn prompt_editor(current: &str) -> String {
    let candidates = ["nvim", "vim", "vi", "nano", "emacs", "hx", "micro", "kak", "helix"];
    let available: Vec<&str> = candidates.iter()
        .copied()
        .filter(|e| which(e))
        .collect();

    if available.is_empty() {
        return prompt("Editor", current);
    }

    println!("  Available editors:");
    for (i, e) in available.iter().enumerate() {
        let marker = if *e == current { " ◀" } else { "" };
        println!("    {}. {}{}", i + 1, e, marker);
    }
    println!("  Enter a number or type a path.");

    loop {
        print!("  {:<18} [{}]: ", "Editor", current);
        let _ = io::stdout().flush();
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return current.to_string();
        }
        if let Ok(n) = trimmed.parse::<usize>() {
            if n >= 1 && n <= available.len() {
                return available[n - 1].to_string();
            }
            eprintln!("  please enter a number between 1 and {}", available.len());
            continue;
        }
        return trimmed.to_string();
    }
}

fn which(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn prompt(label: &str, default: &str) -> String {
    print!("  {:<18} [{}]: ", label, default);
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let trimmed = input.trim();
    if trimmed.is_empty() { default.to_string() } else { trimmed.to_string() }
}

fn prompt_u16(label: &str, default: u16) -> u16 {
    loop {
        let s = prompt(label, &default.to_string());
        match s.parse() {
            Ok(v) => return v,
            Err(_) => eprintln!("  please enter a number"),
        }
    }
}

fn print_help() {
    println!(
        "tnote — per-tmux-window notepad

USAGE:
  tnote                  Open editor for the current window
  tnote name <name>      Name this window's note (also renames the tmux window)
  tnote show             Print note contents inline
  tnote clean            Remove orphaned notes and popup sessions
  tnote list             List all notes with line counts
  tnote path             Print the note file path
  tnote setup            Configure and install tmux key binding
  tnote uninstall        Remove the tmux keybinding
  tnote help             Show this help

NOTE TYPES:
  tmux    One note per tmux window, keyed to <session>+<window> (e.g. work+0).
          Persists until the window is closed. Cleared by 'tnote clean'.

  named   Created with 'tnote name <name>'. Shared across any window that
          points to the same name. Never auto-cleared.

  shell   One note per shell session (parent PID), used outside tmux.
          Cleared by 'tnote clean' once the shell process exits.

ENVIRONMENT:
  TNOTE_DIR              Note storage directory  (default: ~/.tnote)
  TNOTE_KEY              Tmux key binding        (default: t, used as prefix+t)
  TNOTE_WIDTH            Popup width in columns  (default: 62)
  TNOTE_HEIGHT           Popup height in lines   (default: 22)"
    );
}
