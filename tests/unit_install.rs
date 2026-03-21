use tempfile::TempDir;
use tnote::install::{add_source_line, remove_source_line, which};

// ── which ────────────────────────────────────────────────────────────────────

#[test]
fn test_which_known_command() {
    assert!(which("sh"));
}

#[test]
fn test_which_nonexistent_command() {
    assert!(!which("__tnote_nonexistent_cmd_xyz__"));
}

// ── add_source_line ──────────────────────────────────────────────────────────

#[test]
fn test_add_source_line_creates_new_file() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    add_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert!(content.contains(&format!("source-file {}", source.display())));
}

#[test]
fn test_add_source_line_appends_to_existing() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    std::fs::write(&conf, "# existing config\n").unwrap();
    add_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert!(content.starts_with("# existing config\n"));
    assert!(content.contains(&format!("source-file {}", source.display())));
}

#[test]
fn test_add_source_line_idempotent() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    add_source_line(&conf, &source).unwrap();
    add_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert_eq!(content.matches("source-file").count(), 1);
}

#[test]
fn test_add_source_line_adds_newline_before_if_missing() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    std::fs::write(&conf, "# no newline").unwrap();
    add_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert!(content.contains("# no newline\n"));
}

// ── remove_source_line ───────────────────────────────────────────────────────

#[test]
fn test_remove_source_line_missing_file_is_ok() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    assert!(remove_source_line(&conf, &source).is_ok());
}

#[test]
fn test_remove_source_line_removes_the_line() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    std::fs::write(&conf, format!("# header\nsource-file {}\n# footer\n", source.display())).unwrap();
    remove_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert!(!content.contains("source-file"));
    assert!(content.contains("# header"));
    assert!(content.contains("# footer"));
}

#[test]
fn test_remove_source_line_preserves_trailing_newline() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    std::fs::write(&conf, format!("source-file {}\n", source.display())).unwrap();
    remove_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert!(content.ends_with('\n'));
}

#[test]
fn test_remove_source_line_no_change_when_not_present() {
    let dir    = TempDir::new().unwrap();
    let conf   = dir.path().join("tmux.conf");
    let source = dir.path().join("tnote.conf");
    std::fs::write(&conf, "# unrelated config\n").unwrap();
    remove_source_line(&conf, &source).unwrap();
    let content = std::fs::read_to_string(&conf).unwrap();
    assert_eq!(content, "# unrelated config\n");
}
