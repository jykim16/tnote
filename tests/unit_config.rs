use std::sync::Mutex;
use tempfile::TempDir;
use tnote::config::{read_config_file, parse_u16, parse_str, Config};

// Serialize tests that mutate env vars.
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ── read_config_file ─────────────────────────────────────────────────────────

#[test]
fn test_read_config_file_parses_key_value() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "editor=nvim\nkey=t\nwidth=80\n").unwrap();
    let map = read_config_file(&path);
    assert_eq!(map.get("editor").map(String::as_str), Some("nvim"));
    assert_eq!(map.get("key").map(String::as_str), Some("t"));
    assert_eq!(map.get("width").map(String::as_str), Some("80"));
}

#[test]
fn test_read_config_file_missing_returns_empty() {
    let dir = TempDir::new().unwrap();
    let map = read_config_file(&dir.path().join("nonexistent"));
    assert!(map.is_empty());
}

#[test]
fn test_read_config_file_ignores_invalid_lines() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "not_a_key_value\neditor=vim\n").unwrap();
    let map = read_config_file(&path);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("editor").map(String::as_str), Some("vim"));
}

#[test]
fn test_read_config_file_trims_whitespace() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, " editor = nvim \n").unwrap();
    let map = read_config_file(&path);
    assert_eq!(map.get("editor").map(String::as_str), Some("nvim"));
}

// ── parse_u16 ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_u16_env_takes_precedence() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::set_var("_TNOTE_TEST_U16", "99");
    let v = parse_u16("_TNOTE_TEST_U16", Some("50"), 10);
    std::env::remove_var("_TNOTE_TEST_U16");
    assert_eq!(v, 99);
}

#[test]
fn test_parse_u16_file_fallback() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::remove_var("_TNOTE_TEST_U16B");
    let v = parse_u16("_TNOTE_TEST_U16B", Some("42"), 10);
    assert_eq!(v, 42);
}

#[test]
fn test_parse_u16_default() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::remove_var("_TNOTE_TEST_U16C");
    let v = parse_u16("_TNOTE_TEST_U16C", None, 7);
    assert_eq!(v, 7);
}

#[test]
fn test_parse_u16_invalid_env_uses_file() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::set_var("_TNOTE_TEST_U16D", "not_a_number");
    let v = parse_u16("_TNOTE_TEST_U16D", Some("55"), 1);
    std::env::remove_var("_TNOTE_TEST_U16D");
    assert_eq!(v, 55);
}

// ── parse_str ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_str_env_takes_precedence() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::set_var("_TNOTE_TEST_STR", "helix");
    let v = parse_str("_TNOTE_TEST_STR", Some("vim"), "nano");
    std::env::remove_var("_TNOTE_TEST_STR");
    assert_eq!(v, "helix");
}

#[test]
fn test_parse_str_file_fallback() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::remove_var("_TNOTE_TEST_STRB");
    let v = parse_str("_TNOTE_TEST_STRB", Some("micro"), "nano");
    assert_eq!(v, "micro");
}

#[test]
fn test_parse_str_default() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::remove_var("_TNOTE_TEST_STRC");
    let v = parse_str("_TNOTE_TEST_STRC", None, "fallback");
    assert_eq!(v, "fallback");
}

// ── Config::save / from_env ───────────────────────────────────────────────────

#[test]
fn test_save_writes_correct_format() {
    let dir = TempDir::new().unwrap();
    let cfg = Config {
        dir:    dir.path().to_path_buf(),
        editor: "nvim".into(),
        key:    "n".into(),
        width:  100,
        height: 30,
    };
    cfg.save().unwrap();
    let content = std::fs::read_to_string(dir.path().join("meta").join("config")).unwrap();
    assert!(content.contains("editor=nvim"));
    assert!(content.contains("key=n"));
    assert!(content.contains("width=100"));
    assert!(content.contains("height=30"));
}

#[test]
fn test_save_round_trip() {
    let _lock = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    let cfg = Config {
        dir:    dir.path().to_path_buf(),
        editor: "nano".into(),
        key:    "u".into(),
        width:  50,
        height: 15,
    };
    cfg.save().unwrap();

    std::env::set_var("TNOTE_DIR", dir.path().to_str().unwrap());
    std::env::remove_var("TNOTE_WIDTH");
    std::env::remove_var("TNOTE_HEIGHT");
    std::env::remove_var("TNOTE_KEY");
    std::env::remove_var("EDITOR");
    let loaded = Config::from_env();
    std::env::remove_var("TNOTE_DIR");

    assert_eq!(loaded.editor, "nano");
    assert_eq!(loaded.key,    "u");
    assert_eq!(loaded.width,  50);
    assert_eq!(loaded.height, 15);
}

#[test]
fn test_from_env_defaults_when_no_config_file() {
    let _lock = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    std::env::set_var("TNOTE_DIR", dir.path().to_str().unwrap());
    std::env::remove_var("TNOTE_WIDTH");
    std::env::remove_var("TNOTE_HEIGHT");
    std::env::remove_var("TNOTE_KEY");
    std::env::remove_var("EDITOR");
    let cfg = Config::from_env();
    std::env::remove_var("TNOTE_DIR");
    assert_eq!(cfg.width,  62);
    assert_eq!(cfg.height, 22);
    assert_eq!(cfg.key,    "t");
}

#[test]
fn test_from_env_migration_tnotes_to_tnote() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = TempDir::new().unwrap();
    let old_dir = home.path().join(".tnotes");
    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::write(old_dir.join("note.md"), "hello").unwrap();

    std::env::remove_var("TNOTE_DIR");
    std::env::set_var("HOME", home.path().to_str().unwrap());
    let cfg = Config::from_env();
    std::env::remove_var("HOME");

    assert!(cfg.dir.ends_with(".tnote"));
    assert!(cfg.dir.exists());
    assert!(!old_dir.exists());
}

#[test]
fn test_from_env_migration_termnotes_to_tnote() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = TempDir::new().unwrap();
    let old_dir = home.path().join(".termnotes");
    std::fs::create_dir_all(&old_dir).unwrap();

    std::env::remove_var("TNOTE_DIR");
    std::env::set_var("HOME", home.path().to_str().unwrap());
    let cfg = Config::from_env();
    std::env::remove_var("HOME");

    assert!(cfg.dir.ends_with(".tnote"));
    assert!(!old_dir.exists());
}

#[test]
fn test_from_env_no_migration_when_tnote_exists() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = TempDir::new().unwrap();
    let tnote_dir  = home.path().join(".tnote");
    let tnotes_dir = home.path().join(".tnotes");
    std::fs::create_dir_all(&tnote_dir).unwrap();
    std::fs::create_dir_all(&tnotes_dir).unwrap();

    std::env::remove_var("TNOTE_DIR");
    std::env::set_var("HOME", home.path().to_str().unwrap());
    Config::from_env();
    std::env::remove_var("HOME");

    assert!(tnotes_dir.exists());
}
