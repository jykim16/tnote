use tempfile::TempDir;
use tnote::notes::{Notes, ClearScope, get_shell_pid, shell_session_key};

fn setup() -> (Notes, TempDir) {
    let dir = TempDir::new().unwrap();
    let notes = Notes::new(dir.path().to_path_buf());
    notes.ensure_dir().unwrap();
    (notes, dir)
}

fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

fn write_note(notes: &Notes, key: &str, content: &str) {
    std::fs::write(notes.dir.join(format!("{}.md", key)), content).unwrap();
}

fn write_link(notes: &Notes, key: &str, name: &str) {
    std::fs::write(notes.meta_dir().join(format!("{}.link", key)), name).unwrap();
}

// ── get_shell_pid / shell_session_key ─────────────────────────────────────────

#[test]
fn test_get_shell_pid_returns_some() {
    assert!(get_shell_pid().is_some());
}

#[test]
fn test_shell_session_key_format() {
    let key = shell_session_key();
    assert!(key.starts_with("shell-") || key.starts_with("pid-"));
}

// ── ensure_dir / migrate_to_meta ─────────────────────────────────────────────

#[test]
fn test_ensure_dir_creates_dirs() {
    let dir = TempDir::new().unwrap();
    let notes = Notes::new(dir.path().join("notes"));
    notes.ensure_dir().unwrap();
    assert!(dir.path().join("notes").exists());
    assert!(dir.path().join("notes").join("meta").exists());
}

#[test]
fn test_migrate_to_meta_moves_link_and_pid_files() {
    let (notes, _dir) = setup();
    std::fs::write(notes.dir.join("tmux-$1+@3.link"), "project").unwrap();
    std::fs::write(notes.dir.join("tmux-$1+@3.pid"),  "12345").unwrap();
    notes.ensure_dir().unwrap();
    assert!(notes.meta_dir().join("tmux-$1+@3.link").exists());
    assert!(notes.meta_dir().join("tmux-$1+@3.pid").exists());
    assert!(!notes.dir.join("tmux-$1+@3.link").exists());
}

// ── file_for_key ─────────────────────────────────────────────────────────────

#[test]
fn test_file_for_key_no_link() {
    let (notes, _dir) = setup();
    let file = notes.file_for_key("tmux-$1+@3");
    assert_eq!(file, notes.dir.join("tmux-$1+@3.md"));
}

#[test]
fn test_file_for_key_follows_link() {
    let (notes, _dir) = setup();
    write_link(&notes, "tmux-$1+@3", "myproject");
    let file = notes.file_for_key("tmux-$1+@3");
    assert_eq!(file, notes.dir.join("named-myproject.md"));
}

// ── label_for_key ─────────────────────────────────────────────────────────────

#[test]
fn test_label_for_key_no_link_returns_key() {
    let (notes, _dir) = setup();
    assert_eq!(notes.label_for_key("shell-12345"), "shell-12345");
}

#[test]
fn test_label_for_key_with_link_returns_name() {
    let (notes, _dir) = setup();
    write_link(&notes, "tmux-$1+@3", "api-server");
    assert_eq!(notes.label_for_key("tmux-$1+@3"), "api-server");
}

// ── name_window ───────────────────────────────────────────────────────────────

#[test]
fn test_name_window_creates_link() {
    let (notes, _dir) = setup();
    let migrated = notes.name_window("tmux-$1+@3", "backend").unwrap();
    assert!(!migrated);
    assert!(notes.meta_dir().join("tmux-$1+@3.link").exists());
    let link_content = std::fs::read_to_string(notes.meta_dir().join("tmux-$1+@3.link")).unwrap();
    assert_eq!(link_content.trim(), "backend");
}

#[test]
fn test_name_window_migrates_existing_content() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "existing content");
    let migrated = notes.name_window("tmux-$1+@3", "backend").unwrap();
    assert!(migrated);
    assert!(!notes.dir.join("tmux-$1+@3.md").exists());
    let content = std::fs::read_to_string(notes.dir.join("named-backend.md")).unwrap();
    assert_eq!(content, "existing content");
}

#[test]
fn test_name_window_no_migrate_when_target_exists() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "old content");
    write_note(&notes, "named-backend", "existing named");
    let migrated = notes.name_window("tmux-$1+@3", "backend").unwrap();
    assert!(!migrated);
    assert!(notes.dir.join("tmux-$1+@3.md").exists());
}

#[test]
fn test_name_window_no_migrate_when_note_empty() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "");
    let migrated = notes.name_window("tmux-$1+@3", "backend").unwrap();
    assert!(!migrated);
    assert!(notes.dir.join("tmux-$1+@3.md").exists());
}

// ── remove_named ─────────────────────────────────────────────────────────────

#[test]
fn test_remove_named_returns_false_when_not_found() {
    let (notes, _dir) = setup();
    assert!(!notes.remove_named("ghost", false).unwrap());
}

#[test]
fn test_remove_named_deletes_file() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-project", "content");
    assert!(notes.remove_named("project", false).unwrap());
    assert!(!notes.dir.join("named-project.md").exists());
}

#[test]
fn test_remove_named_also_removes_links() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-project", "content");
    write_link(&notes, "tmux-$1+@3", "project");
    write_link(&notes, "tmux-$2+@5", "project");
    notes.remove_named("project", false).unwrap();
    assert!(!notes.meta_dir().join("tmux-$1+@3.link").exists());
    assert!(!notes.meta_dir().join("tmux-$2+@5.link").exists());
}

#[test]
fn test_remove_named_dryrun_does_not_delete() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-project", "content");
    write_link(&notes, "tmux-$1+@3", "project");
    assert!(notes.remove_named("project", true).unwrap());
    assert!(notes.dir.join("named-project.md").exists());
    assert!(notes.meta_dir().join("tmux-$1+@3.link").exists());
}

// ── cleanup_orphaned ─────────────────────────────────────────────────────────

#[test]
fn test_cleanup_removes_dead_shell_note() {
    let (notes, _dir) = setup();
    let pid = dead_pid();
    write_note(&notes, &format!("shell-{}", pid), "stale");
    let removed = notes.cleanup_orphaned(None, false).unwrap();
    assert!(removed.contains(&format!("shell-{}", pid)));
    assert!(!notes.dir.join(format!("shell-{}.md", pid)).exists());
}

#[test]
fn test_cleanup_keeps_live_shell_note() {
    let (notes, _dir) = setup();
    let pid = std::process::id();
    write_note(&notes, &format!("shell-{}", pid), "active");
    let removed = notes.cleanup_orphaned(None, false).unwrap();
    assert!(!removed.contains(&format!("shell-{}", pid)));
    assert!(notes.dir.join(format!("shell-{}.md", pid)).exists());
}

#[test]
fn test_cleanup_removes_tmux_note_outside_tmux() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "orphaned");
    let removed = notes.cleanup_orphaned(None, false).unwrap();
    assert!(removed.contains(&"tmux-$1+@3".to_string()));
    assert!(!notes.dir.join("tmux-$1+@3.md").exists());
}

#[test]
fn test_cleanup_keeps_named_note_by_default() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-docs", "important");
    let removed = notes.cleanup_orphaned(None, false).unwrap();
    assert!(!removed.contains(&"named-docs".to_string()));
    assert!(notes.dir.join("named-docs.md").exists());
}

#[test]
fn test_cleanup_scope_named_removes_named() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-docs", "content");
    let removed = notes.cleanup_orphaned(Some(&ClearScope::Named), false).unwrap();
    assert!(removed.contains(&"named-docs".to_string()));
}

#[test]
fn test_cleanup_scope_all_removes_everything() {
    let (notes, _dir) = setup();
    let pid = dead_pid();
    write_note(&notes, &format!("shell-{}", pid), "s");
    write_note(&notes, "tmux-$1+@3", "t");
    write_note(&notes, "named-docs", "n");
    let removed = notes.cleanup_orphaned(Some(&ClearScope::All), false).unwrap();
    assert!(removed.contains(&format!("shell-{}", pid)));
    assert!(removed.contains(&"tmux-$1+@3".to_string()));
    assert!(removed.contains(&"named-docs".to_string()));
}

#[test]
fn test_cleanup_scope_tmux_forces_tmux_removal() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "t");
    let removed = notes.cleanup_orphaned(Some(&ClearScope::Tmux), false).unwrap();
    assert!(removed.contains(&"tmux-$1+@3".to_string()));
}

#[test]
fn test_cleanup_scope_unprefixed() {
    let (notes, _dir) = setup();
    write_note(&notes, "weird-file", "content");
    let removed = notes.cleanup_orphaned(Some(&ClearScope::Unprefixed), false).unwrap();
    assert!(removed.contains(&"weird-file".to_string()));
}

#[test]
fn test_cleanup_dryrun_does_not_delete() {
    let (notes, _dir) = setup();
    let pid = dead_pid();
    write_note(&notes, &format!("shell-{}", pid), "stale");
    let removed = notes.cleanup_orphaned(None, true).unwrap();
    assert!(removed.contains(&format!("shell-{}", pid)));
    assert!(notes.dir.join(format!("shell-{}.md", pid)).exists());
}

#[test]
fn test_cleanup_superseded_tmux_removes_md_keeps_link() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "leftover");
    write_link(&notes, "tmux-$1+@3", "myproject");
    write_note(&notes, "named-myproject", "real content");
    let removed = notes.cleanup_orphaned(None, false).unwrap();
    assert!(removed.contains(&"tmux-$1+@3".to_string()));
    assert!(!notes.dir.join("tmux-$1+@3.md").exists());
    assert!(notes.dir.join("named-myproject.md").exists());
}

// ── list_notes ───────────────────────────────────────────────────────────────

#[test]
fn test_list_notes_empty() {
    let (notes, _dir) = setup();
    let list = notes.list_notes().unwrap();
    assert!(list.is_empty());
}

#[test]
fn test_list_notes_returns_all_types() {
    let (notes, _dir) = setup();
    write_note(&notes, "tmux-$1+@3", "tmux note");
    write_note(&notes, "named-project", "named note");
    write_note(&notes, "shell-12345", "shell note");
    let list = notes.list_notes().unwrap();
    let categories: Vec<&str> = list.iter().map(|(c, _, _, _, _)| c.as_str()).collect();
    assert!(categories.contains(&"tmux"));
    assert!(categories.contains(&"named"));
    assert!(categories.contains(&"shell"));
}

#[test]
fn test_list_notes_line_count() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-work", "line1\nline2\nline3\n");
    let list = notes.list_notes().unwrap();
    let (_, _, _, lines, _) = list.iter().find(|(c, _, _, _, _)| c == "named").unwrap();
    assert_eq!(*lines, 3);
}

#[test]
fn test_list_notes_sources_for_named() {
    let (notes, _dir) = setup();
    write_note(&notes, "named-project", "content");
    write_link(&notes, "tmux-$1+@3", "project");
    let list = notes.list_notes().unwrap();
    let (_, _, sources, _, _) = list.iter().find(|(c, _, _, _, _)| c == "named").unwrap();
    assert!(!sources.is_empty());
}

// ── link_sources ─────────────────────────────────────────────────────────────

#[test]
fn test_link_sources_builds_reverse_map() {
    let (notes, _dir) = setup();
    write_link(&notes, "tmux-$1+@3", "api");
    write_link(&notes, "tmux-$2+@5", "api");
    write_link(&notes, "tmux-$1+@7", "ui");
    let map = notes.link_sources();
    let mut api_sources = map["api"].clone();
    api_sources.sort();
    assert_eq!(api_sources.len(), 2);
    assert_eq!(map["ui"].len(), 1);
}
