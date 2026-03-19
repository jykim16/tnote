use std::fs;
use std::path::PathBuf;

pub struct Notes {
    pub dir: PathBuf,
}

impl Notes {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)
    }

    /// Resolve the note file for a window key, following `.link` files for named notes.
    pub fn file_for_key(&self, key: &str) -> PathBuf {
        let link = self.dir.join(format!("{}.link", key));
        if link.exists() {
            if let Ok(name) = fs::read_to_string(&link) {
                let name = name.trim();
                return self.dir.join(format!("named-{}.md", name));
            }
        }
        self.dir.join(format!("{}.md", key))
    }

    /// Return the display label for a key (the assigned name, or the key itself).
    pub fn label_for_key(&self, key: &str) -> String {
        let link = self.dir.join(format!("{}.link", key));
        if link.exists() {
            if let Ok(name) = fs::read_to_string(&link) {
                return name.trim().to_string();
            }
        }
        key.to_string()
    }

    /// Assign a name to the current window's note. Migrates existing unnamed content.
    /// Returns true if content was migrated.
    pub fn name_window(&self, key: &str, name: &str) -> std::io::Result<bool> {
        let link = self.dir.join(format!("{}.link", key));
        let old_file = self.dir.join(format!("{}.md", key));
        let new_file = self.dir.join(format!("named-{}.md", name));

        let migrated =
            if old_file.exists() && old_file.metadata()?.len() > 0 && !new_file.exists() {
                fs::rename(&old_file, &new_file)?;
                true
            } else {
                false
            };

        fs::write(&link, name)?;
        Ok(migrated)
    }

    /// List all `.md` files in the notes dir. Returns (display_name, line_count, path).
    pub fn list_notes(&self) -> std::io::Result<Vec<(String, usize, PathBuf)>> {
        let mut notes = Vec::new();

        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // Strip "named-" prefix for display
            let display = stem.trim_start_matches("named-").to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            let lines = content.lines().count();

            notes.push((display, lines, path));
        }

        notes.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(notes)
    }
}
