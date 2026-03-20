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
