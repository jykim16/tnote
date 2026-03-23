use tnote::{config, editor, install, notes, tmux};

use clap::{Parser, Subcommand, ValueEnum};
use config::Config;
use std::io::{self, Write};
use notes::Notes;
use owo_colors::{OwoColorize, Style, Stream::Stdout, Stream::Stderr};
use std::fs;
use std::path::PathBuf;

#[derive(ValueEnum, Clone)]
enum ClearScope {
    /// Remove files without a recognized prefix (tmux-, shell-, named-)
    Unprefixed,
    /// Remove all named notes
    Named,
    /// Remove all tmux-keyed notes regardless of window liveness
    Tmux,
    /// Remove all notes
    All,

}

impl From<ClearScope> for notes::ClearScope {
    fn from(s: ClearScope) -> Self {
        match s {
            ClearScope::Unprefixed => notes::ClearScope::Unprefixed,
            ClearScope::Named      => notes::ClearScope::Named,
            ClearScope::Tmux       => notes::ClearScope::Tmux,
            ClearScope::All        => notes::ClearScope::All,
        }
    }
}

#[derive(Parser)]
#[command(name = "tnote", about = "Per-tmux-window notepad", disable_help_subcommand = true, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Name this window's note (also renames the tmux window)
    Name { name: Option<String> },
    /// Print note contents inline
    Show,
    /// Remove notes not tied to a running process or window
    Clean {
        /// Also remove notes in the given category: unprefixed, named, tmux, all
        #[arg(long, value_name = "CATEGORY")]
        all: Option<ClearScope>,
        /// Remove a specific named note by name
        #[arg(long, value_name = "NAME")]
        named: Option<String>,
        /// Print what would be removed without removing anything
        #[arg(long)]
        dryrun: bool,
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
        Some(Cmd::Name { name }) => cmd_name(&notes, name.as_deref()),
        Some(Cmd::Show) => cmd_show(&notes),
        Some(Cmd::Clean { all, named, dryrun }) => cmd_clean(&notes, all.clone(), named.as_deref(), *dryrun),
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
                .unwrap_or_else(|| notes::shell_session_key());
            cmd_popup_inline(&config, &notes, &key);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn current_note(notes: &Notes) -> (String, PathBuf) {
    let key = if tmux::is_in_tmux() {
        tmux::window_key().unwrap_or_else(|| notes::shell_session_key())
    } else {
        notes::shell_session_key()
    };
    let file = notes.file_for_key(&key);
    (key, file)
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

fn cmd_name(notes: &Notes, name: Option<&str>) {
    let Some(name) = name else {
        if tmux::is_in_tmux() {
            tmux::prompt_name();
        } else {
            eprintln!("tnote name: provide a name, e.g.: tnote name <name>");
            std::process::exit(1);
        }
        return;
    };

    let (key, _) = current_note(notes);

    match notes.name_window(&key, name) {
        Ok(migrated) => {
            if migrated {
                println!("tnote name: migrated existing notes → {}", name);
            }
            if tmux::is_in_tmux() {
                tmux::rename_window(name);
                tmux::display_message(&format!("tnote: note named '{}'", name));
            }
            println!("tnote name: window note named '{}'", name);
        }
        Err(e) => {
            eprintln!("tnote name: error naming note: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_show(notes: &Notes) {
    // Resolve: named (via .link) → tmux → shell, show first non-empty.
    let mut candidates: Vec<String> = Vec::new();
    if tmux::is_in_tmux() {
        if let Some(k) = tmux::window_key() {
            candidates.push(k);
        }
    }
    candidates.push(notes::shell_session_key());

    // Expand any key that has a .link into the named key first.
    let resolved: Vec<(String, String, PathBuf)> = candidates.into_iter().map(|k| {
        let label = notes.label_for_key(&k);
        let file  = notes.file_for_key(&k);
        (k, label, file)
    }).collect();

    for (_, label, file) in &resolved {
        let non_empty = file.exists() && file.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if non_empty {
            println!("{}", format!("── tnote: {} ──", label).if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
            match fs::read_to_string(file) {
                Ok(content) => print!("{}", content),
                Err(e) => eprintln!("tnote show: {}", e.if_supports_color(Stderr, |t| t.red())),
            }
            println!("{}", "──────────────────────".if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
            return;
        }
    }

    let label = resolved.first().map(|(_, l, _)| l.as_str()).unwrap_or("unknown");
    println!("tnote show: (empty) [{}]", label.if_supports_color(Stdout, |t| t.dimmed()));
}

fn cmd_clean(notes: &Notes, scope: Option<ClearScope>, named: Option<&str>, dry_run: bool) {
    let mut any = false;

    if let Some(name) = named {
        match notes.remove_named(name, dry_run) {
            Ok(true) => {
                let verb = if dry_run { "would remove" } else { "removed" };
                println!("tnote clean: {} named note '{}'", verb, name.if_supports_color(Stdout, |t| t.yellow()));
                any = true;
            }
            Ok(false) => {
                eprintln!("tnote clean: named note '{}' not found", name);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("tnote clean: {}", e.if_supports_color(Stderr, |t| t.red()));
                std::process::exit(1);
            }
        }
    }

    let lib_scope: Option<notes::ClearScope> = scope.map(Into::into);
    match notes.cleanup_orphaned(lib_scope.as_ref(), dry_run) {
        Ok(removed) if !removed.is_empty() => {
            let verb = if dry_run { "would remove" } else { "removed" };
            for key in &removed {
                println!("tnote clean: {} note {}", verb, key.if_supports_color(Stdout, |t| t.yellow()));
            }
            any = true;
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("tnote clean: {}", e.if_supports_color(Stderr, |t| t.red()));
            std::process::exit(1);
        }
    }

    let sessions = tmux::cleanup_popup_sessions(&notes.dir, dry_run);
    for s in &sessions {
        let verb = if dry_run { "would kill" } else { "killed" };
        println!("tnote clean: {} popup session {}", verb, s.if_supports_color(Stdout, |t| t.yellow()));
        any = true;
    }

    if !any {
        println!("tnote clean: {}", "nothing to clean up".if_supports_color(Stdout, |t| t.green()));
    }
}

fn cmd_list(notes: &Notes) {
    let (_, current_file) = current_note(notes);
    match notes.list_notes() {
        Ok(list) => {
            if list.is_empty() {
                println!("tnote list: (no notes yet)");
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
                    if k.starts_with("tmux-") {
                        let label = tmux::window_display_label(k)
                            .unwrap_or_else(|| k.strip_prefix("tmux-").unwrap_or(k).to_string());
                        format!("tmux - {}", label)
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
            eprintln!("tnote list: {}", e);
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
        eprintln!("tnote setup: failed to save config: {}", e);
        std::process::exit(1);
    }

    install::run(&new_config);
}

fn prompt_editor(current: &str) -> String {
    let candidates = ["nvim", "vim", "vi", "nano", "emacs", "hx", "micro", "kak", "helix"];
    let available: Vec<&str> = candidates.iter()
        .copied()
        .filter(|e| install::which(e))
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
  tnote name             Interactive name prompt (tmux only)
  tnote show             Print note contents inline
  tnote clean            Remove orphaned notes and popup sessions
  tnote clean --dryrun   Show what would be removed without removing anything
  tnote list             List all notes with line counts
  tnote path             Print the note file path
  tnote setup            Configure and install tmux key binding
  tnote uninstall        Remove the tmux keybinding
  tnote help             Show this help

TMUX COMMAND LINE (works while a process is running):
  Press ':' in any tmux window, then type:
    tnote              Open note popup
    tnote-name         Interactive name prompt
    tnote-show         Print note contents
    tnote-list         List all notes
    tnote-path         Print the note file path
    tnote-clean        Remove orphaned notes
    tnote-help         Show this help
  Requires 'tnote setup' to install the ':tnote' command aliases.

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
