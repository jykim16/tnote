use std::collections::HashMap;
use std::path::PathBuf;

pub struct Config {
    pub dir: PathBuf,
    pub width: u16,
    pub height: u16,
    pub key: String,
    pub editor: String,
}

impl Config {
    pub fn from_env() -> Self {
        let dir = std::env::var("TNOTE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = PathBuf::from(
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
                );
                let target    = home.join(".tnote");
                let from_old  = home.join(".tnotes");
                let from_old2 = home.join(".termnotes");

                if !target.exists() {
                    if from_old.exists() {
                        if let Err(e) = std::fs::rename(&from_old, &target) {
                            eprintln!("tnote: could not migrate {} → {}: {}", from_old.display(), target.display(), e);
                        } else {
                            eprintln!("tnote: migrated {} → {}", from_old.display(), target.display());
                        }
                    } else if from_old2.exists() {
                        if let Err(e) = std::fs::rename(&from_old2, &target) {
                            eprintln!("tnote: could not migrate {} → {}: {}", from_old2.display(), target.display(), e);
                        } else {
                            eprintln!("tnote: migrated {} → {}", from_old2.display(), target.display());
                        }
                    }
                }
                target
            });

        let file = read_config_file(&dir.join("meta").join("config"));

        Config {
            width:  parse_u16("TNOTE_WIDTH",  file.get("width").map(String::as_str),  62),
            height: parse_u16("TNOTE_HEIGHT", file.get("height").map(String::as_str), 22),
            key:    parse_str("TNOTE_KEY",    file.get("key").map(String::as_str),    "t"),
            editor: parse_str("EDITOR",       file.get("editor").map(String::as_str), "vim"),
            dir,
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.dir.join("meta"))?;
        std::fs::write(
            self.dir.join("meta").join("config"),
            format!(
                "editor={}\nkey={}\nwidth={}\nheight={}\n",
                self.editor, self.key, self.width, self.height
            ),
        )
    }
}

fn read_config_file(path: &std::path::Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

fn parse_u16(env_key: &str, file_val: Option<&str>, default: u16) -> u16 {
    std::env::var(env_key).ok().and_then(|v| v.parse().ok())
        .or_else(|| file_val.and_then(|v| v.parse().ok()))
        .unwrap_or(default)
}

fn parse_str(env_key: &str, file_val: Option<&str>, default: &str) -> String {
    std::env::var(env_key).ok()
        .or_else(|| file_val.map(str::to_string))
        .unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Env key that is very unlikely to be set in any real environment.
    const ABSENT: &str = "TNOTE_TEST_ABSENT_KEY_XYZ_99999";

    // ── read_config_file ──────────────────────────────────────────────────────

    #[test]
    fn read_config_file_returns_empty_for_missing_file() {
        let map = read_config_file(Path::new("/nonexistent/path/config"));
        assert!(map.is_empty());
    }

    #[test]
    fn read_config_file_parses_key_value_pairs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config");
        std::fs::write(&path, "editor=nvim\nwidth=80\n").unwrap();
        let map = read_config_file(&path);
        assert_eq!(map.get("editor").map(String::as_str), Some("nvim"));
        assert_eq!(map.get("width").map(String::as_str), Some("80"));
    }

    #[test]
    fn read_config_file_trims_whitespace() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config");
        std::fs::write(&path, "  key  =  t  \n").unwrap();
        let map = read_config_file(&path);
        assert_eq!(map.get("key").map(String::as_str), Some("t"));
    }

    #[test]
    fn read_config_file_ignores_lines_without_equals() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config");
        std::fs::write(&path, "# comment\neditor=vim\n").unwrap();
        let map = read_config_file(&path);
        assert!(!map.contains_key("# comment"));
        assert_eq!(map.get("editor").map(String::as_str), Some("vim"));
    }

    // ── parse_u16 ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_u16_uses_file_val_when_env_absent() {
        assert_eq!(parse_u16(ABSENT, Some("42"), 0), 42);
    }

    #[test]
    fn parse_u16_uses_default_when_both_absent() {
        assert_eq!(parse_u16(ABSENT, None, 99), 99);
    }

    #[test]
    fn parse_u16_falls_back_to_default_on_invalid_file_val() {
        assert_eq!(parse_u16(ABSENT, Some("bad"), 7), 7);
    }

    #[test]
    fn parse_u16_falls_back_to_default_on_overflow_file_val() {
        assert_eq!(parse_u16(ABSENT, Some("99999"), 7), 7);
    }

    // ── parse_str ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_str_uses_file_val_when_env_absent() {
        assert_eq!(parse_str(ABSENT, Some("nvim"), "vim"), "nvim");
    }

    #[test]
    fn parse_str_uses_default_when_both_absent() {
        assert_eq!(parse_str(ABSENT, None, "vim"), "vim");
    }

    // ── Config::save ─────────────────────────────────────────────────────────

    #[test]
    fn config_save_writes_all_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config {
            dir:    tmp.path().to_path_buf(),
            editor: "nano".to_string(),
            key:    "n".to_string(),
            width:  100,
            height: 30,
        };
        cfg.save().unwrap();
        let content = std::fs::read_to_string(tmp.path().join("meta/config")).unwrap();
        assert!(content.contains("editor=nano"));
        assert!(content.contains("key=n"));
        assert!(content.contains("width=100"));
        assert!(content.contains("height=30"));
    }

    #[test]
    fn config_save_round_trips_via_read_config_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config {
            dir:    tmp.path().to_path_buf(),
            editor: "hx".to_string(),
            key:    "g".to_string(),
            width:  70,
            height: 25,
        };
        cfg.save().unwrap();
        let map = read_config_file(&tmp.path().join("meta/config"));
        assert_eq!(map.get("editor").map(String::as_str), Some("hx"));
        assert_eq!(map.get("key").map(String::as_str), Some("g"));
        assert_eq!(map.get("width").map(String::as_str), Some("70"));
        assert_eq!(map.get("height").map(String::as_str), Some("25"));
    }
}
