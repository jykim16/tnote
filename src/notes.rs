use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Clone)]
pub enum ClearScope {
    /// Remove files without a recognized prefix (tmux-, shell-, named-)
    Unprefixed,
    /// Remove all named notes
    Named,
    /// Remove all tmux-keyed notes regardless of window liveness
    Tmux,
    /// Remove all notes
    All,
}

/// Returns the PID of the parent shell process (tnote's direct parent).
pub fn get_shell_pid() -> Option<u32> {
    // /proc/self/stat is not available on macOS, so we use getppid() via libc.
    let ppid = unsafe { libc::getppid() };
    if ppid > 0 { Some(ppid as u32) } else { None }
}

/// Returns a stable key for the current shell session using the parent process PID.
pub fn shell_session_key() -> String {
    get_shell_pid()
        .map(|ppid| format!("shell-{}", ppid))
        .unwrap_or_else(|| format!("pid-{}", std::process::id()))
}

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

        // Ensure the named note file exists so it appears in `tnote list`.
        if !new_file.exists() {
            fs::write(&new_file, "")?;
        }

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
                    || s.parse::<u32>().is_ok_and(|pid| !is_pid_alive(pid))
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

    /// Remove the .link binding for a single window key.
    /// Returns Ok(Some(name)) if a binding was removed, Ok(None) if none existed.
    pub fn unbind_key(&self, key: &str) -> std::io::Result<Option<String>> {
        let link = self.meta_dir().join(format!("{}.link", key));
        if !link.exists() {
            return Ok(None);
        }
        let name = fs::read_to_string(&link).unwrap_or_default().trim().to_string();
        fs::remove_file(&link)?;
        Ok(Some(name))
    }

    /// Remove all .link bindings pointing to a named note.
    /// Returns the list of keys that were unbound.
    pub fn unbind_named(&self, name: &str) -> std::io::Result<Vec<String>> {
        let meta = self.meta_dir();
        let mut unbound = Vec::new();
        if let Ok(entries) = fs::read_dir(&meta) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("link") {
                    if let Ok(target) = fs::read_to_string(entry.path()) {
                        if target.trim() == name {
                            let key = entry.file_name().to_string_lossy()
                                .strip_suffix(".link").unwrap_or("").to_string();
                            let _ = fs::remove_file(entry.path());
                            unbound.push(key);
                        }
                    }
                }
            }
        }
        Ok(unbound)
    }

    /// Builds a reverse map: note name → list of keys that link to it.
    /// e.g. "api-server" → ["tmux-work+0", "shell-12345"]
    pub fn link_sources(&self) -> HashMap<String, Vec<String>> {
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
    #[allow(clippy::type_complexity)]
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

pub fn is_pid_alive(pid: u32) -> bool {
    // kill(pid, 0) checks process existence without sending a signal.
    // Returns 0 if process exists. On error, ESRCH means not found;
    // EPERM means it exists but we lack permission — still alive.
    unsafe { libc::kill(pid as i32, 0) == 0 || io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup(tmp: &tempfile::TempDir) -> Notes {
        let notes = Notes::new(tmp.path().to_path_buf());
        notes.ensure_dir().unwrap();
        notes
    }

    // ── ensure_dir ────────────────────────────────────────────────────────────

    #[test]
    fn ensure_dir_creates_note_and_meta_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = Notes::new(tmp.path().join("notes"));
        notes.ensure_dir().unwrap();
        assert!(tmp.path().join("notes").is_dir());
        assert!(tmp.path().join("notes/meta").is_dir());
    }

    // ── migrate_to_meta ───────────────────────────────────────────────────────

    #[test]
    fn migrate_to_meta_moves_link_and_pid_files() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = Notes::new(tmp.path().to_path_buf());
        fs::create_dir_all(notes.meta_dir()).unwrap();

        // Place .link and .pid files in root (old layout)
        fs::write(tmp.path().join("tmux-$1+@2.link"), "myname").unwrap();
        fs::write(tmp.path().join("tmux-$1+@2.pid"), "1234").unwrap();
        // .md files should NOT be migrated
        fs::write(tmp.path().join("tmux-$1+@2.md"), "content").unwrap();

        notes.ensure_dir().unwrap();

        assert!(notes.meta_dir().join("tmux-$1+@2.link").exists());
        assert!(notes.meta_dir().join("tmux-$1+@2.pid").exists());
        assert!(!tmp.path().join("tmux-$1+@2.link").exists());
        assert!(!tmp.path().join("tmux-$1+@2.pid").exists());
        // .md stays in root
        assert!(tmp.path().join("tmux-$1+@2.md").exists());
    }

    // ── file_for_key ──────────────────────────────────────────────────────────

    #[test]
    fn file_for_key_no_link_returns_key_md() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let path = notes.file_for_key("tmux-$1+@3");
        assert_eq!(path, tmp.path().join("tmux-$1+@3.md"));
    }

    #[test]
    fn file_for_key_follows_link_file() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(notes.meta_dir().join("tmux-$1+@3.link"), "myproject").unwrap();
        let path = notes.file_for_key("tmux-$1+@3");
        assert_eq!(path, tmp.path().join("named-myproject.md"));
    }

    #[test]
    fn file_for_key_link_trims_whitespace() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(notes.meta_dir().join("tmux-$1+@3.link"), "  myproject\n").unwrap();
        let path = notes.file_for_key("tmux-$1+@3");
        assert_eq!(path, tmp.path().join("named-myproject.md"));
    }

    // ── label_for_key ─────────────────────────────────────────────────────────

    #[test]
    fn label_for_key_no_link_returns_key() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        assert_eq!(notes.label_for_key("shell-1234"), "shell-1234");
    }

    #[test]
    fn label_for_key_with_link_returns_name() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(notes.meta_dir().join("tmux-$1+@3.link"), "api-server").unwrap();
        assert_eq!(notes.label_for_key("tmux-$1+@3"), "api-server");
    }

    // ── name_window ───────────────────────────────────────────────────────────

    #[test]
    fn name_window_creates_link_no_content() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let migrated = notes.name_window("tmux-$1+@3", "work").unwrap();
        assert!(!migrated);
        let link = fs::read_to_string(notes.meta_dir().join("tmux-$1+@3.link")).unwrap();
        assert_eq!(link, "work");
    }

    #[test]
    fn name_window_migrates_non_empty_unnamed_note() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("tmux-$1+@3.md"), "some notes").unwrap();
        let migrated = notes.name_window("tmux-$1+@3", "work").unwrap();
        assert!(migrated);
        assert!(!tmp.path().join("tmux-$1+@3.md").exists());
        assert!(tmp.path().join("named-work.md").exists());
        let content = fs::read_to_string(tmp.path().join("named-work.md")).unwrap();
        assert_eq!(content, "some notes");
    }

    #[test]
    fn name_window_skips_migration_if_named_file_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("tmux-$1+@3.md"), "old content").unwrap();
        fs::write(tmp.path().join("named-work.md"), "existing").unwrap();
        let migrated = notes.name_window("tmux-$1+@3", "work").unwrap();
        assert!(!migrated);
        // Both files remain unchanged
        assert!(tmp.path().join("tmux-$1+@3.md").exists());
        assert_eq!(
            fs::read_to_string(tmp.path().join("named-work.md")).unwrap(),
            "existing"
        );
    }

    #[test]
    fn name_window_skips_migration_for_empty_unnamed_note() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("tmux-$1+@3.md"), "").unwrap();
        let migrated = notes.name_window("tmux-$1+@3", "work").unwrap();
        assert!(!migrated);
    }

    // ── remove_named ──────────────────────────────────────────────────────────

    #[test]
    fn remove_named_returns_false_when_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        assert!(!notes.remove_named("ghost", false).unwrap());
    }

    #[test]
    fn remove_named_deletes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "content").unwrap();
        assert!(notes.remove_named("work", false).unwrap());
        assert!(!tmp.path().join("named-work.md").exists());
    }

    #[test]
    fn remove_named_dry_run_leaves_file() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "content").unwrap();
        assert!(notes.remove_named("work", true).unwrap());
        assert!(tmp.path().join("named-work.md").exists());
    }

    #[test]
    fn remove_named_also_removes_pointing_links() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "content").unwrap();
        fs::write(notes.meta_dir().join("tmux-$1+@3.link"), "work").unwrap();
        fs::write(notes.meta_dir().join("tmux-$1+@5.link"), "other").unwrap();
        notes.remove_named("work", false).unwrap();
        assert!(!notes.meta_dir().join("tmux-$1+@3.link").exists());
        // Unrelated link untouched
        assert!(notes.meta_dir().join("tmux-$1+@5.link").exists());
    }

    // ── cleanup_orphaned ──────────────────────────────────────────────────────

    /// Returns a PID that is guaranteed to not be running.
    fn dead_pid() -> u32 {
        // Try a few high PIDs; pick the first one ps says is gone.
        for pid in [9_999_997u32, 9_999_998, 9_999_999] {
            if !is_pid_alive(pid) {
                return pid;
            }
        }
        panic!("could not find a dead PID for test");
    }

    #[test]
    fn cleanup_orphaned_removes_dead_shell_note() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let pid = dead_pid();
        let key = format!("shell-{}", pid);
        fs::write(tmp.path().join(format!("{}.md", key)), "stale").unwrap();
        let removed = notes.cleanup_orphaned(None, false).unwrap();
        assert!(removed.contains(&key));
        assert!(!tmp.path().join(format!("{}.md", key)).exists());
    }

    #[test]
    fn cleanup_orphaned_keeps_alive_shell_note() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let pid = std::process::id(); // current process is definitely alive
        let key = format!("shell-{}", pid);
        fs::write(tmp.path().join(format!("{}.md", key)), "active").unwrap();
        let removed = notes.cleanup_orphaned(None, false).unwrap();
        assert!(!removed.contains(&key));
        assert!(tmp.path().join(format!("{}.md", key)).exists());
    }

    #[test]
    fn cleanup_orphaned_dry_run_leaves_files() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let pid = dead_pid();
        let key = format!("shell-{}", pid);
        fs::write(tmp.path().join(format!("{}.md", key)), "stale").unwrap();
        let removed = notes.cleanup_orphaned(None, true).unwrap();
        assert!(removed.contains(&key));
        // File must still exist after dry run
        assert!(tmp.path().join(format!("{}.md", key)).exists());
    }

    #[test]
    fn cleanup_orphaned_named_not_removed_without_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "important").unwrap();
        let removed = notes.cleanup_orphaned(None, false).unwrap();
        assert!(!removed.iter().any(|k| k.starts_with("named-")));
        assert!(tmp.path().join("named-work.md").exists());
    }

    #[test]
    fn cleanup_orphaned_named_removed_with_named_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "content").unwrap();
        let removed = notes.cleanup_orphaned(Some(&ClearScope::Named), false).unwrap();
        assert!(removed.contains(&"named-work".to_string()));
        assert!(!tmp.path().join("named-work.md").exists());
    }

    #[test]
    fn cleanup_orphaned_tmux_dead_window_removed() {
        // Without a live tmux server, all tmux keys look dead
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let key = "tmux-$99+@99";
        fs::write(tmp.path().join(format!("{}.md", key)), "old").unwrap();
        let removed = notes.cleanup_orphaned(None, false).unwrap();
        assert!(removed.contains(&key.to_string()));
    }

    #[test]
    fn cleanup_orphaned_tmux_superseded_removes_raw_md() {
        // A tmux key that has a .link should have its raw .md cleaned up
        // even if the window is still live (because the named file is canonical)
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let key = "tmux-$1+@3";
        fs::write(tmp.path().join(format!("{}.md", key)), "old raw").unwrap();
        fs::write(notes.meta_dir().join(format!("{}.link", key)), "work").unwrap();
        let removed = notes.cleanup_orphaned(None, false).unwrap();
        // The stem is removed (superseded or dead either way)
        assert!(removed.contains(&key.to_string()));
    }

    #[test]
    fn cleanup_orphaned_all_scope_removes_everything() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "x").unwrap();
        fs::write(tmp.path().join("tmux-$1+@1.md"), "y").unwrap();
        fs::write(tmp.path().join("custom.md"), "z").unwrap();
        let removed = notes.cleanup_orphaned(Some(&ClearScope::All), false).unwrap();
        assert!(removed.contains(&"named-work".to_string()));
        assert!(removed.contains(&"tmux-$1+@1".to_string()));
        assert!(removed.contains(&"custom".to_string()));
    }

    // ── list_notes ────────────────────────────────────────────────────────────

    #[test]
    fn list_notes_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        let list = notes.list_notes().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_notes_categorizes_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "a\nb").unwrap();
        fs::write(tmp.path().join("tmux-$1+@3.md"), "c").unwrap();
        fs::write(tmp.path().join("shell-1234.md"), "d\ne\nf").unwrap();
        fs::write(tmp.path().join("custom.md"), "g").unwrap();

        let list = notes.list_notes().unwrap();
        // sorted by (category, display)
        let cats: Vec<&str> = list.iter().map(|(c, _, _, _, _)| c.as_str()).collect();
        assert!(cats.contains(&"named"));
        assert!(cats.contains(&"tmux"));
        assert!(cats.contains(&"shell"));
        assert!(cats.contains(&"other"));

        let work = list.iter().find(|(c, d, _, _, _)| c == "named" && d == "work").unwrap();
        assert_eq!(work.3, 2); // line count

        let shell = list.iter().find(|(c, _, _, _, _)| c == "shell").unwrap();
        assert_eq!(shell.3, 3);
    }

    #[test]
    fn list_notes_ignores_non_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("notes.txt"), "ignored").unwrap();
        fs::write(tmp.path().join("readme.md"), "counted").unwrap();
        let list = notes.list_notes().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn list_notes_named_includes_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = setup(&tmp);
        fs::write(tmp.path().join("named-work.md"), "content").unwrap();
        fs::write(notes.meta_dir().join("tmux-$1+@3.link"), "work").unwrap();
        let list = notes.list_notes().unwrap();
        let work = list.iter().find(|(c, d, _, _, _)| c == "named" && d == "work").unwrap();
        assert_eq!(work.2, vec!["tmux-$1+@3"]);
    }
}
