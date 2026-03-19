use std::path::PathBuf;

pub struct Config {
    pub dir: PathBuf,
    pub editor: String,
    pub width: String,
    pub height: String,
}

impl Config {
    pub fn from_env() -> Self {
        let dir = std::env::var("TNOTE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                let home = PathBuf::from(home);
                let new_dir = home.join(".tnotes");
                let old_dir = home.join(".termnotes");
                // Auto-migrate from old default dir if new one doesn't exist yet
                if !new_dir.exists() && old_dir.exists() {
                    if let Err(e) = std::fs::rename(&old_dir, &new_dir) {
                        eprintln!("tnote: could not migrate {} → {}: {}", old_dir.display(), new_dir.display(), e);
                    } else {
                        eprintln!("tnote: migrated {} → {}", old_dir.display(), new_dir.display());
                    }
                }
                new_dir
            });

        Config {
            dir,
            editor: std::env::var("TNOTE_EDITOR").unwrap_or_else(|_| "vim".to_string()),
            width: std::env::var("TNOTE_WIDTH").unwrap_or_else(|_| "62".to_string()),
            height: std::env::var("TNOTE_HEIGHT").unwrap_or_else(|_| "22".to_string()),
        }
    }
}
