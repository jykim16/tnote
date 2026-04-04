use tnote::{config, editor, install, name_picker, notes, tmux};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use config::Config;
use std::io::{self, Write};
use notes::Notes;
use owo_colors::{OwoColorize, Style, Stream::Stdout, Stream::Stderr};
use std::fs;
use std::path::PathBuf;

const CURRENT_TARGET_SENTINEL: &str = "__current__";

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
#[command(name = "tnote", about = "Terminal Notepad", disable_help_subcommand = true, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,

    /// Open a specific named note
    #[arg(short = 'n', long)]
    name: Option<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Name or rebind this window's note to a named note (also renames the tmux window)
    Name {
        name: Option<String>,
        /// Bind the current tmux window / shell session, or a specific tmux window key / shell pid if provided
        #[arg(long, value_name = "TARGET", num_args = 0..=1, default_missing_value = CURRENT_TARGET_SENTINEL, conflicts_with = "unbind")]
        bind: Option<String>,
        /// Unbind all links for this named note, or a specific tmux window key / shell pid if provided
        #[arg(long, value_name = "TARGET", num_args = 0..=1, default_missing_value = CURRENT_TARGET_SENTINEL, conflicts_with = "bind")]
        unbind: Option<String>,
    },
    /// Print note contents inline
    Show {
        /// Show a specific named note
        #[arg(short = 'n', long)]
        name: Option<String>,
    },
    /// Remove notes not tied to a running process or window
    Clean {
        /// Also remove notes in the given category: unprefixed, named, tmux, all
        #[arg(long, value_name = "CATEGORY")]
        all: Option<ClearScope>,
        /// Remove a specific named note by name
        #[arg(short = 'n', long, value_name = "NAME")]
        name: Option<String>,
        /// Move to archive instead of deleting
        #[arg(long)]
        archive: bool,
        /// Restore from archive
        #[arg(long)]
        unarchive: bool,
        /// Print what would be removed without removing anything
        #[arg(long)]
        dryrun: bool,
    },
    /// List all notes with line counts
    #[command(alias = "ls")]
    List {
        /// List archived notes instead
        #[arg(long)]
        archive: bool,
    },
    /// Print the note file path
    Path {
        /// Show path for a specific named note
        #[arg(short = 'n', long)]
        name: Option<String>,
    },
    /// Configure editor, key binding, and dimensions, then install keybindings
    Setup,
    /// Remove tmux and shell keybindings
    Uninstall,
    /// Show usage
    Help,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Internal: print existing named notes for shell completion
    #[command(name = "__complete-named-notes", hide = true)]
    CompleteNamedNotes,
    /// Internal: interactive tmux popup picker for note naming
    #[command(name = "__name-picker", hide = true)]
    NamePicker {
        window_key: Option<String>,
    },
    /// Internal: name a specific tmux target from tmux command-prompt
    #[command(name = "__name-target", hide = true)]
    NameTarget {
        window_key: String,
        name: String,
    },
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
        None => cmd_open(&config, &notes, cli.name.as_deref()),
        Some(Cmd::Name { name, bind, unbind }) => cmd_name(&config, &notes, name.as_deref(), bind.as_deref(), unbind.as_deref()),
        Some(Cmd::Show { name }) => cmd_show(&notes, name.as_deref()),
        Some(Cmd::Clean { all, name, archive, unarchive, dryrun }) => cmd_clean(&notes, all.clone(), name.as_deref(), *archive, *unarchive, *dryrun),
        Some(Cmd::List { archive }) => {
            if *archive {
                cmd_list_archive(&notes);
            } else {
                cmd_list(&notes, &config);
            }
        }
        Some(Cmd::Path { name }) => cmd_path(&notes, name.as_deref()),
        Some(Cmd::Setup) => cmd_setup(&config),
        Some(Cmd::Uninstall) => install::uninstall(&config),
        Some(Cmd::Help) => print_help(),
        Some(Cmd::Completions { shell }) => cmd_completions(*shell),
        Some(Cmd::CompleteNamedNotes) => cmd_complete_named_notes(&notes),
        Some(Cmd::NamePicker { window_key }) => {
            let key = window_key.as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| std::env::var("TNOTE_WINDOW_KEY").ok());
            cmd_name_picker(&notes, key.as_deref());
        }
        Some(Cmd::NameTarget { window_key, name }) => cmd_name_target(&notes, window_key, name),
        Some(Cmd::Popup { window_key }) => {
            let key = window_key.as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| std::env::var("TNOTE_WINDOW_KEY").ok())
                .or_else(tmux::window_key)
                .unwrap_or_else(notes::shell_session_key);
            cmd_popup_inline(&config, &notes, &key);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn current_note(notes: &Notes) -> (String, PathBuf) {
    let key = if tmux::is_in_tmux() {
        tmux::window_key().unwrap_or_else(notes::shell_session_key)
    } else {
        notes::shell_session_key()
    };
    let file = notes.file_for_key(&key);
    (key, file)
}

fn named_or_current(notes: &Notes, name: Option<&str>) -> (String, PathBuf) {
    match name {
        Some(n) => {
            let key = format!("named-{}", n);
            let file = notes.dir.join(format!("{}.md", key));
            (key, file)
        }
        None => current_note(notes),
    }
}

// ── Subcommands ───────────────────────────────────────────────────────────────

fn cmd_open(config: &Config, notes: &Notes, name: Option<&str>) {
    // First-run hint
    if !config.dir.join("meta").join("config").exists() {
        eprintln!("tnote: tip — run 'tnote setup' to configure keybindings and editor");
    }

    if tmux::is_in_tmux() && tmux::is_popup_session() {
        let _ = std::process::Command::new("tmux").args(["detach-client"]).status();
        return;
    }

    let (key, file) = named_or_current(notes, name);
    let label = match name {
        Some(n) => n.to_string(),
        None => notes.label_for_key(&key),
    };

    if !file.exists() {
        let _ = fs::write(&file, "");
    }

    let result = if tmux::is_in_tmux() {
        tmux::open_popup_session(&file, &key, config)
    } else {
        editor::run(&file, &label, &config.width, &config.height)
    };

    if let Err(e) = result {
        if e.kind() == std::io::ErrorKind::NotFound {
            eprintln!("tnote: editor '{}' not found — run 'tnote setup' to configure", config.editor);
        } else {
            eprintln!("tnote: {}", e);
        }
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
        if e.kind() == std::io::ErrorKind::NotFound {
            eprintln!("tnote: editor '{}' not found — run 'tnote setup' to configure", editor);
        } else {
            eprintln!("tnote: failed to run {}: {}", editor, e);
        }
        std::process::exit(1);
    }
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn is_tmux_window_id(s: &str) -> bool {
    let Some((session, window)) = s.split_once("+@") else {
        return false;
    };

    let Some(session) = session.strip_prefix('$') else {
        return false;
    };

    is_digits(session) && is_digits(window)
}

fn normalize_bind_target(target: &str) -> Result<String, String> {
    if let Some(pid) = target.strip_prefix("shell-") {
        if is_digits(pid) {
            return Ok(target.to_string());
        }
    }

    if let Some(tmux_id) = target.strip_prefix("tmux-") {
        if is_tmux_window_id(tmux_id) {
            return Ok(target.to_string());
        }
    }

    if is_digits(target) {
        return Ok(format!("shell-{}", target));
    }

    if is_tmux_window_id(target) {
        return Ok(format!("tmux-{}", target));
    }

    Err(format!(
        "invalid bind target '{}'; expected shell-<pid>, <pid>, tmux-$SESSION+@WINDOW, or $SESSION+@WINDOW",
        target
    ))
}

fn resolve_bind_target(notes: &Notes, target: Option<&str>) -> Result<String, String> {
    match target {
        Some(CURRENT_TARGET_SENTINEL) | None => Ok(current_note(notes).0),
        Some(target) => normalize_bind_target(target),
    }
}

fn cmd_name(_config: &Config, notes: &Notes, name: Option<&str>, bind: Option<&str>, unbind: Option<&str>) {
    let Some(name) = name else {
        if tmux::is_in_tmux() {
            let window_key = current_note(notes).0;
            tmux::prompt_name(_config, &window_key);
        } else {
            eprintln!("tnote name: provide a name, e.g.: tnote name <name>");
            std::process::exit(1);
        }
        return;
    };

    if unbind.is_some() {
        if unbind == Some(CURRENT_TARGET_SENTINEL) {
            match notes.unbind_named(name) {
                Ok(keys) if keys.is_empty() => {
                    eprintln!("tnote name: no windows bound to '{}'", name);
                    std::process::exit(1);
                }
                Ok(keys) => {
                    for key in &keys {
                        println!("tnote name: unbound {} from '{}'", key, name);
                    }
                }
                Err(e) => {
                    eprintln!("tnote name: {}", e);
                    std::process::exit(1);
                }
            }
            return;
        }

        let key = match resolve_bind_target(notes, unbind) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("tnote name: {}", e);
                std::process::exit(1);
            }
        };

        match notes.unbind_named_key(name, &key) {
            Ok(true) => println!("tnote name: unbound {} from '{}'", key, name),
            Ok(false) => {
                eprintln!("tnote name: {} is not bound to '{}'", key, name);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("tnote name: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    let key = match resolve_bind_target(notes, bind) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("tnote name: {}", e);
            std::process::exit(1);
        }
    };

    match notes.name_window(&key, name) {
        Ok(migrated) => {
            if migrated {
                println!("tnote name: migrated existing notes → {}", name);
            }
            if tmux::is_in_tmux() && bind.is_none() {
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

fn cmd_name_picker(notes: &Notes, window_key: Option<&str>) {
    let note_names = notes.named_note_names().unwrap_or_default();
    let Some(window_key) = window_key else {
        eprintln!("tnote name: missing tmux window key for popup picker");
        std::process::exit(1);
    };

    match name_picker::run(&note_names) {
        Ok(Some(name_picker::Selection::Existing(name))) => {
            cmd_name_target(notes, window_key, &name);
        }
        Ok(Some(name_picker::Selection::PromptNew)) => {
            tmux::prompt_name_for_target(window_key);
        }
        Ok(None) => (),
        Err(e) => {
            eprintln!("tnote name: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_name_target(notes: &Notes, window_key: &str, name: &str) {
    match notes.name_window(window_key, name) {
        Ok(migrated) => {
            if migrated {
                println!("tnote name: migrated existing notes → {}", name);
            }
            if window_key.starts_with("tmux-") {
                tmux::rename_window_target(window_key, name);
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

fn show_named_note(notes: &Notes, n: &str) {
    let file = notes.dir.join(format!("named-{}.md", n));
    if !file.exists() {
        eprintln!("tnote show: named note '{}' not found", n);
        std::process::exit(1);
    }
    if file.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        println!("{}", format!("── {}: {} ──", n, file.display()).if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
        match fs::read_to_string(&file) {
            Ok(content) => print!("{}", content),
            Err(e) => eprintln!("tnote show: {}", e.if_supports_color(Stderr, |t| t.red())),
        }
        println!("{}", "──────────────────────".if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
    } else {
        println!("tnote show: (empty) [{}]", n.if_supports_color(Stdout, |t| t.dimmed()));
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    if !text.starts_with(parts[0]) {
        return false;
    }
    let mut pos = parts[0].len();
    for part in &parts[1..parts.len() - 1] {
        match text[pos..].find(part) {
            Some(i) => pos += i + part.len(),
            None => return false,
        }
    }
    let last = parts[parts.len() - 1];
    if last.is_empty() { true } else { text[pos..].contains(last) && text.ends_with(last) }
}

fn cmd_show(notes: &Notes, name: Option<&str>) {
    if let Some(n) = name {
        if n.contains('*') {
            let mut matches: Vec<String> = fs::read_dir(&notes.dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let fname = e.file_name();
                    let s = fname.to_string_lossy();
                    let stem = s.strip_prefix("named-")?.strip_suffix(".md")?;
                    if glob_match(n, stem) { Some(stem.to_string()) } else { None }
                })
                .collect();
            matches.sort();
            if matches.is_empty() {
                eprintln!("tnote show: no notes matching '{}'", n);
                std::process::exit(1);
            }
            for note_name in &matches {
                show_named_note(notes, note_name);
            }
            return;
        }
        show_named_note(notes, n);
        return;
    }

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
            println!("{}", format!("── {}: {} ──", label, file.display()).if_supports_color(Stdout, |t| t.style(Style::new().cyan().bold())));
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

fn cmd_clean(notes: &Notes, scope: Option<ClearScope>, named: Option<&str>, archive: bool, unarchive: bool, dry_run: bool) {
    if let Some(name) = named {
        let result = if unarchive {
            notes.unarchive_named(name, dry_run)
        } else if archive {
            notes.archive_named(name, dry_run)
        } else {
            notes.remove_named(name, dry_run)
        };
        match result {
            Ok(true) => {
                let verb = if dry_run {
                    if unarchive { "would unarchive" } else if archive { "would archive" } else { "would remove" }
                } else if unarchive { "unarchived" } else if archive { "archived" } else { "removed" };
                println!("tnote clean: {} named note '{}'", verb, name.if_supports_color(Stdout, |t| t.yellow()));
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

        return;
    }

    let lib_scope: Option<notes::ClearScope> = scope.map(Into::into);
    let mut any = false;
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

fn run_annotation_cmd(cmd_template: &str, path: &std::path::Path) -> String {
    let cmd = cmd_template.replace("{}", &path.display().to_string());
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim_end_matches('\n').to_string()
        }
        _ => String::new(),
    }
}

fn cmd_list(notes: &Notes, config: &Config) {
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
                annotation:    String,
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
                let annotation = if let Some(ref cmd) = config.ls_annotation {
                    run_annotation_cmd(cmd, path)
                } else {
                    format!("{} lines", lines)
                };
                Row {
                    category:      cat.clone(),
                    plain_label:   shown,
                    colored_label: colored_name,
                    annotation,
                    marker,
                    sources,
                }
            }).collect();

            let max_width        = rows.iter().map(|r| r.plain_label.len()).max().unwrap_or(0);
            let annotation_width = rows.iter().map(|r| r.annotation.len()).max().unwrap_or(0);

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
                    println!("  {}{}  {:width$}{}",
                        row.colored_label,
                        " ".repeat(padding),
                        row.annotation,
                        row.marker,
                        width = annotation_width,
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

fn cmd_list_archive(notes: &Notes) {
    let archive = notes.archive_dir();
    let entries = match std::fs::read_dir(&archive) {
        Ok(e) => e,
        Err(_) => {
            println!("tnote list: (no archived notes)");
            return;
        }
    };
    let mut items: Vec<(String, usize)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let name = name.strip_prefix("named-")?.strip_suffix(".md")?.to_string();
            let lines = std::fs::read_to_string(e.path()).unwrap_or_default().lines().count();
            Some((name, lines))
        })
        .collect();
    if items.is_empty() {
        println!("tnote list: (no archived notes)");
        return;
    }
    items.sort();
    let max_w = items.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    println!("{}", "archived:".if_supports_color(Stdout, |t| t.style(owo_colors::Style::new().cyan().bold())));
    for (name, lines) in &items {
        println!("  {}{}  {} lines", name, " ".repeat(max_w - name.len()), lines);
    }
}

fn cmd_path(notes: &Notes, name: Option<&str>) {
    if let Some(n) = name {
        let file = notes.dir.join(format!("named-{}.md", n));
        if !file.exists() {
            eprintln!("tnote path: named note '{}' not found", n);
            std::process::exit(1);
        }
        println!("{}", file.display());
        return;
    }
    let (_, file) = current_note(notes);
    println!("{}", file.display());
}

fn cmd_complete_named_notes(notes: &Notes) {
    if let Ok(note_names) = notes.named_note_names() {
        for name in note_names {
            println!("{}", name);
        }
    }
}

fn cmd_completions(shell: clap_complete::Shell) {
    let script = match shell {
        clap_complete::Shell::Bash => bash_completions(),
        clap_complete::Shell::Zsh => zsh_completions(),
        clap_complete::Shell::Fish => fish_completions(),
        _ => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "tnote", &mut std::io::stdout());
            return;
        }
    };

    print!("{}", script);
}

fn bash_completions() -> &'static str {
    r#"_tnote_named_notes() {
    tnote __complete-named-notes 2>/dev/null
}

_tnote() {
    local cur prev cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev=""
    if (( COMP_CWORD > 0 )); then
        prev="${COMP_WORDS[COMP_CWORD-1]}"
    fi
    cmd=""
    if (( ${#COMP_WORDS[@]} > 1 )); then
        cmd="${COMP_WORDS[1]}"
    fi

    case "$prev" in
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish elvish powershell" -- "$cur") )
            return
            ;;
        --all)
            COMPREPLY=( $(compgen -W "unprefixed named tmux all" -- "$cur") )
            return
            ;;
        -n|--name)
            if [[ "$cmd" == "show" || "$cmd" == "clean" || "$cmd" == "path" ]]; then
                COMPREPLY=( $(compgen -W "$(_tnote_named_notes)" -- "$cur") )
                return
            fi
            ;;
    esac

    if (( COMP_CWORD == 1 )); then
        COMPREPLY=( $(compgen -W "name show clean list ls path setup uninstall help completions" -- "$cur") )
        return
    fi

    if [[ "$cmd" == "name" && $COMP_CWORD -eq 2 && "$cur" != -* ]]; then
        COMPREPLY=( $(compgen -W "$(_tnote_named_notes)" -- "$cur") )
        return
    fi
}

complete -F _tnote tnote
"#
}

fn zsh_completions() -> &'static str {
    r#"#compdef tnote

_tnote_named_notes() {
  local -a notes
  notes=("${(@f)$(tnote __complete-named-notes 2>/dev/null)}")
  (( ${#notes[@]} )) && compadd -- $notes
}

_tnote() {
  local cur prev cmd
  cur="${words[CURRENT]}"
  prev=""
  (( CURRENT > 1 )) && prev="${words[CURRENT-1]}"
  cmd=""
  (( ${#words[@]} > 1 )) && cmd="${words[2]}"

  if (( CURRENT == 2 )); then
    compadd -- name show clean list ls path setup uninstall help completions
    return
  fi

  case "$prev" in
    completions)
      compadd -- bash zsh fish elvish powershell
      return
      ;;
    --all)
      compadd -- unprefixed named tmux all
      return
      ;;
    -n|--name)
      if [[ "$cmd" == "show" || "$cmd" == "clean" || "$cmd" == "path" ]]; then
        _tnote_named_notes
        return
      fi
      ;;
  esac

  if [[ "$cmd" == "name" && CURRENT -eq 3 && "$cur" != -* ]]; then
    _tnote_named_notes
    return
  fi
}

compdef _tnote tnote
"#
}

fn fish_completions() -> &'static str {
    r#"function __tnote_named_notes
    tnote __complete-named-notes 2>/dev/null
end

complete -c tnote -f
complete -c tnote -n '__fish_use_subcommand' -a 'name show clean list ls path setup uninstall help completions'
complete -c tnote -n '__fish_seen_subcommand_from completions' -a 'bash zsh fish elvish powershell'
complete -c tnote -n '__fish_seen_subcommand_from clean' -l all -a 'unprefixed named tmux all'
complete -c tnote -n '__fish_seen_subcommand_from show clean path' -s n -l name -a '(__tnote_named_notes)'
complete -c tnote -n '__fish_seen_subcommand_from name' -l bind
complete -c tnote -n '__fish_seen_subcommand_from name' -l unbind
complete -c tnote -n '__fish_seen_subcommand_from name; and not __fish_seen_subcommand_from --bind --unbind' -a '(__tnote_named_notes)'
"#
}

fn cmd_setup(config: &Config) {
    println!("tnote setup\n");

    let editor        = prompt_editor(&config.editor);
    let key           = prompt("Key (tmux: prefix+?, shell: Ctrl+?)", &config.key);
    let width         = prompt("Popup width  (e.g. 100%, 80)", &config.width);
    let height        = prompt("Popup height (e.g. 50%, 22)",  &config.height);
    let annotation_default = config.ls_annotation.as_deref().unwrap_or("");
    let annotation_raw = prompt("ls annotation (e.g. head -1 {})", annotation_default);
    let ls_annotation = if annotation_raw.is_empty() { None } else { Some(annotation_raw) };

    let new_config = Config {
        dir:    config.dir.clone(),
        editor,
        key,
        width,
        height,
        ls_annotation,
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

fn print_help() {
    println!(
        "tnote — Terminal Notepad

USAGE:
  tnote                            Open editor for the current window
  tnote name [name]                Name or rebind this window's note (also renames the tmux window)
  tnote name <name> --bind [key]   Bind the current session, or one specific tmux/shell binding, to a named note
  tnote name <name> --unbind [key] Remove all bindings for a named note, or one specific binding
  tnote show                       Print note contents inline
  tnote show --name 'proj-*'       Print all notes matching a pattern (quote the glob)
  tnote clean [--dryrun]           Remove orphaned notes and popup sessions
  tnote list / ls                  List all notes with line counts
  tnote path                       Print the note file path
  tnote setup                      Configure and install keybindings
  tnote uninstall                  Remove tmux and shell keybindings
  tnote help                       Show this help

TMUX COMMAND LINE (works while a process is running):
  Press ':' in any tmux window, then type:
    tnote              Open note popup
    tnote-name         Name menu with existing notes plus a new-name prompt
    tnote-show         Print note contents
    tnote-clean        Remove orphaned notes
    tnote-list         List all notes
    tnote-ls           List all notes (alias)
    tnote-path         Print the note file path
    tnote-help         Show this help
  Requires 'tnote setup' to install the ':tnote' command aliases.

NOTE TYPES:
  tmux    One note per tmux window, keyed to <session>+<window> (e.g. work+0).
          Persists until the window is closed. Cleared by 'tnote clean'.

  named   A preserved note that persists even after closing a terminal session.
          Created with 'tnote name <name>'. Multiple sessions can share a note
          by using the same name.

  shell   One note per shell session (parent PID), used outside tmux.
          Cleared by 'tnote clean' once the shell process exits.

ENVIRONMENT (configurable via 'tnote setup'):
  TNOTE_DIR              Note storage directory  (default: ~/.tnote)
  TNOTE_KEY              Key binding             (default: t, tmux: prefix+t, shell: Ctrl+t)
  TNOTE_WIDTH            Popup width  (default: 100%, e.g. 80 or 75%)
  TNOTE_HEIGHT           Popup height (default: 50%,  e.g. 22 or 40%)
  TNOTE_LS_ANNOTATION    Shell command shown next to each note in 'tnote ls'.
                         Use {{}} as the note file path placeholder.
                         Default: line count.
                         Example: ls_annotation=head -1 {{}} (show first line)"
    );
}

#[cfg(test)]
mod tests {
    use super::{CURRENT_TARGET_SENTINEL, glob_match, is_digits, is_tmux_window_id, normalize_bind_target, resolve_bind_target};
    use crate::notes::Notes;

    #[test]
    fn glob_exact_match() {
        assert!(glob_match("foo", "foo"));
        assert!(!glob_match("foo", "bar"));
    }

    #[test]
    fn glob_trailing_wildcard() {
        assert!(glob_match("proj-*", "proj-auth-login"));
        assert!(glob_match("proj-*", "proj-"));
        assert!(!glob_match("proj-*", "other-auth-login"));
    }

    #[test]
    fn glob_leading_wildcard() {
        assert!(glob_match("*-login", "proj-auth-login"));
        assert!(!glob_match("*-login", "proj-auth-logout"));
    }

    #[test]
    fn glob_mid_wildcard() {
        assert!(glob_match("proj-*-login", "proj-auth-login"));
        assert!(glob_match("proj-*-login", "proj-api-login"));
        assert!(!glob_match("proj-*-login", "proj-auth-logout"));
    }

    #[test]
    fn glob_star_only() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
    }

    #[test]
    fn bind_digits_validation() {
        assert!(is_digits("4242"));
        assert!(!is_digits(""));
        assert!(!is_digits("42a"));
    }

    #[test]
    fn bind_tmux_window_id_validation() {
        assert!(is_tmux_window_id("$255+@278"));
        assert!(!is_tmux_window_id("255+@278"));
        assert!(!is_tmux_window_id("$255+278"));
        assert!(!is_tmux_window_id("$abc+@278"));
    }

    #[test]
    fn bind_target_normalization_accepts_supported_forms() {
        assert_eq!(normalize_bind_target("4242").unwrap(), "shell-4242");
        assert_eq!(normalize_bind_target("shell-4242").unwrap(), "shell-4242");
        assert_eq!(normalize_bind_target("$255+@278").unwrap(), "tmux-$255+@278");
        assert_eq!(normalize_bind_target("tmux-$255+@278").unwrap(), "tmux-$255+@278");
    }

    #[test]
    fn bind_target_normalization_rejects_invalid_forms() {
        assert!(normalize_bind_target("not-a-key").is_err());
        assert!(normalize_bind_target("shell-notapid").is_err());
        assert!(normalize_bind_target("tmux-$255+278").is_err());
    }

    #[test]
    fn resolve_bind_target_uses_current_key_for_boolean_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = Notes::new(tmp.path().to_path_buf());
        let expected = super::current_note(&notes).0;
        assert_eq!(resolve_bind_target(&notes, Some(CURRENT_TARGET_SENTINEL)).unwrap(), expected);
    }
}
