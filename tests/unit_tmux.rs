use tnote::tmux::{shell_escape, is_in_tmux, window_label_map, window_display_label, live_window_keys};

// ── shell_escape ─────────────────────────────────────────────────────────────

#[test]
fn test_shell_escape_simple_string() {
    assert_eq!(shell_escape("hello"), "'hello'");
}

#[test]
fn test_shell_escape_empty_string() {
    assert_eq!(shell_escape(""), "''");
}

#[test]
fn test_shell_escape_with_spaces() {
    assert_eq!(shell_escape("hello world"), "'hello world'");
}

#[test]
fn test_shell_escape_single_quote() {
    assert_eq!(shell_escape("it's"), "'it'\\''s'");
}

#[test]
fn test_shell_escape_multiple_single_quotes() {
    assert_eq!(shell_escape("a'b'c"), "'a'\\''b'\\''c'");
}

#[test]
fn test_shell_escape_special_chars() {
    assert_eq!(shell_escape("$VAR"), "'$VAR'");
    assert_eq!(shell_escape("~/path"), "'~/path'");
}

// ── is_in_tmux ───────────────────────────────────────────────────────────────

#[test]
fn test_is_in_tmux_false_without_tmux_env() {
    if std::env::var("TMUX").is_err() {
        assert!(!is_in_tmux());
    }
}

// ── window_label_map / window_display_label ───────────────────────────────────

#[test]
fn test_window_label_map_returns_empty_outside_tmux() {
    if std::env::var("TMUX").is_err() {
        let map = window_label_map();
        assert!(map.is_empty());
    }
}

#[test]
fn test_window_display_label_returns_none_outside_tmux() {
    if std::env::var("TMUX").is_err() {
        assert!(window_display_label("tmux-$1+@3").is_none());
    }
}

// ── live_window_keys ─────────────────────────────────────────────────────────

#[test]
fn test_live_window_keys_empty_outside_tmux() {
    if std::env::var("TMUX").is_err() {
        let keys = live_window_keys();
        assert!(keys.is_empty());
    }
}
