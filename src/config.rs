use std::path::PathBuf;

pub struct Config {
    pub dir: PathBuf,
    pub width: u16,
    pub height: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let dir = std::env::var("TNOTE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = PathBuf::from(
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
                );
                let target   = home.join(".tnote");
                let from_old = home.join(".tnotes");
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

        Config {
            dir,
            width: parse_env("TNOTE_WIDTH", 62),
            height: parse_env("TNOTE_HEIGHT", 22),
        }
    }
}

fn parse_env(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
