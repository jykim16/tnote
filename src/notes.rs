use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use crate::ClearScope;

pub struct Notes {
    pub dir: PathBuf,
}

impl Notes {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// `~/.tnote/meta/` — stores .link, .pid, and tmux.conf (non-note files).
    pub fn meta_dir(&self) -> PathBuf {
        self.dir.join("meta")
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        fs::create_dir_all(self.meta_dir())?;
        self.migrate_to_meta()
    }

    /// Move any .link / .pid files that exist in the root into meta/ (one-time migration).
    fn migrate_to_meta(&self) -> std::io::Result<()> {
        let meta = self.meta_dir();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.ends_with(".link") || name.ends_with(".pid") {
                let dest = meta.join(&name);
                if !dest.exists() {
                    let _ = fs::rename(&path, &dest);
                }
            }
        }
        Ok(())
    }

    /// Resolve the note file for a window key, following .link files for named notes.
    pub fn file_for_key(&self, key: &str) -> PathBuf {
        let link = self.meta_dir().join(format!("{}.link", key));
        if link.exists() {
            if let Ok(name) = fs::read_to_string(&link) {
                return self.dir.join(format!("named-{}.md", name.trim()));
            }
        }
        self.dir.join(format!("{}.md", key))
    }

    /// Return the display label for a key (the assigned name, or the key itself).
    pub fn label_for_key(&self, key: &str) -> String {
        let link = self.meta_dir().join(format!("{}.link", key));
        if link.exists() {
            if let Ok(name) = fs::read_to_string(&link) {
                return name.trim().to_string();
            }
        }
        if key.starts_with("tmux-") {
            if let Some(label) = crate::tmux::window_display_label(key) {
                return label;
            }
        }
        key.to_string()
    }

    /// Assign a name to the current window's note. Migrates existing unnamed content.
    /// Returns true if content was migrated.
    pub fn name_window(&self, key: &str, name: &str) -> std::io::Result<bool> {
        let link = self.meta_dir().join(format!("{}.link", key));
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

    /// Remove all notes whose associated process or tmux window is no longer alive.
    ///
    /// Rules:
    ///   `shell-<pid>.*`   — PID is embedded in the key; checked with ps.
    ///   `tmux-*.*`        — tmux window key; checked against live tmux windows.
    ///   `named-<name>.*`  — Never removed unless scope is Named or All.
    ///   unprefixed        — Never removed unless scope is Unprefixed or All.
    pub fn cleanup_orphaned(&self, scope: Option<&ClearScope>, dry_run: bool) -> std::io::Result<Vec<String>> {
        let meta = self.meta_dir();

        let include_named = matches!(scope, Some(ClearScope::Named) | Some(ClearScope::All));

        // Collect unique stems from .md files in root and .link files in meta/.
        let mut stems: HashSet<String> = HashSet::new();
        for dir in [&self.dir, &meta] {
            for entry in fs::read_dir(dir)? {
                let name = entry?.file_name();
                let name = name.to_string_lossy();
                let stem = if let Some(s) = name.strip_suffix(".md")   { s }
                      else if let Some(s) = name.strip_suffix(".link") { s }
                      else { continue };
                if !stem.starts_with("named-") || include_named {
                    stems.insert(stem.to_string());
                }
            }
        }

        let live_windows = crate::tmux::live_window_keys();

        let mut removed = Vec::new();
        for stem in stems {
            let dead = if let Some(s) = stem.strip_prefix("shell-") {
                matches!(scope, Some(ClearScope::All))
                    || s.parse::<u32>().map_or(false, |pid| !is_pid_alive(pid))
            } else if stem.starts_with("tmux-") {
                // Superseded: a .link exists for this key, so the raw .md is redundant
                let has_link = meta.join(format!("{}.link", &stem)).exists();
                let md_exists = self.dir.join(format!("{}.md", &stem)).exists();
                (has_link && md_exists)
                    || matches!(scope, Some(ClearScope::Tmux) | Some(ClearScope::All))
                    || !live_windows.contains(&stem)
            } else if stem.starts_with("named-") {
                true // only collected when include_named is set
            } else {
                matches!(scope, Some(ClearScope::Unprefixed) | Some(ClearScope::All))
            };

            if dead {
                if !dry_run {
                    let _ = fs::remove_file(self.dir.join(format!("{}.md", &stem)));
                    // Only remove the .link if the window is truly gone (not just superseded)
                    let superseded_only = stem.starts_with("tmux-")
                        && meta.join(format!("{}.link", &stem)).exists()
                        && live_windows.contains(&stem);
                    if !superseded_only {
                        let _ = fs::remove_file(meta.join(format!("{}.link", &stem)));
                        let _ = fs::remove_file(meta.join(format!("{}.pid",  &stem)));
                    }
                }
                removed.push(stem);
            }
        }

        Ok(removed)
    }

    /// Remove a named note and any .link files pointing to it.
    /// Returns Ok(true) if the note existed, Ok(false) if not found.
    pub fn remove_named(&self, name: &str, dry_run: bool) -> std::io::Result<bool> {
        let note_file = self.dir.join(format!("named-{}.md", name));
        if !note_file.exists() {
            return Ok(false);
        }
        if !dry_run {
            fs::remove_file(&note_file)?;
            // Remove any .link files pointing to this name
            let meta = self.meta_dir();
            if let Ok(entries) = fs::read_dir(&meta) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("link") {
                        if let Ok(target) = fs::read_to_string(entry.path()) {
                            if target.trim() == name {
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
        Ok(true)
    }

    /// Builds a reverse map: note name → list of keys that link to it.
    /// e.g. "api-server" → ["tmux-work+0", "shell-12345"]
    fn link_sources(&self) -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let meta = self.meta_dir();
        if let Ok(entries) = fs::read_dir(&meta) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(key) = name.strip_suffix(".link") {
                    if let Ok(target) = fs::read_to_string(entry.path()) {
                        map.entry(target.trim().to_string())
                            .or_default()
                            .push(key.to_string());
                    }
                }
            }
        }
        map
    }

    /// List all `.md` files in the notes dir.
    /// Returns (category, display_name, sources, line_count, path).
    /// Category is one of: "tmux", "named", "shell", "other".
    /// Sources is non-empty only for named notes — the keys that link to this note.
    pub fn list_notes(&self) -> std::io::Result<Vec<(String, String, Vec<String>, usize, PathBuf)>> {
        let sources   = self.link_sources();
        let label_map = crate::tmux::window_label_map();
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

            let (category, display) = if stem.starts_with("tmux-") {
                let label = label_map.get(stem.as_str())
                    .cloned()
                    .unwrap_or_else(|| stem.strip_prefix("tmux-").unwrap_or(&stem).to_string());
                ("tmux".to_string(), label)
            } else if let Some(s) = stem.strip_prefix("named-") {
                ("named".to_string(), s.to_string())
            } else if stem.starts_with("shell-") {
                ("shell".to_string(), stem.clone())
            } else {
                ("other".to_string(), stem.clone())
            };

            let note_sources = if category == "named" {
                sources.get(&display).cloned().unwrap_or_default()
            } else {
                vec![]
            };

            let content = fs::read_to_string(&path).unwrap_or_default();
            let lines = content.lines().count();

            notes.push((category, display, note_sources, lines, path));
        }

        notes.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        Ok(notes)
    }

}

fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("ps")
        .args(["-p", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(true)
}
