use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tnote"))
}

/// Build a Command with TNOTE_DIR set and TMUX removed (simulates outside-tmux).
fn tnote(dir: &Path) -> Command {
    let mut cmd = Command::new(binary());
    cmd.env("TNOTE_DIR", dir);
    cmd.env_remove("TMUX");
    cmd.env("EDITOR", "true"); // prevents editor from blocking
    cmd
}

/// Returns path to the tests/bin directory containing the fake tmux binary.
fn fake_tmux_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("bin")
}

/// Build a Command that simulates running inside tmux (fake tmux binary in PATH).
fn tnote_in_tmux(dir: &Path, home: &Path) -> Command {
    let mut cmd = Command::new(binary());
    cmd.env("TNOTE_DIR", dir);
    cmd.env("HOME", home);
    cmd.env("TMUX", "/tmp/fake-tmux,0,0"); // any non-empty value enables tmux mode
    cmd.env("EDITOR", "true");
    let old_path = std::env::var("PATH").unwrap_or_default();
    cmd.env("PATH", format!("{}:{}", fake_tmux_bin_dir().display(), old_path));
    cmd
}

fn stdout(cmd: &mut Command) -> String {
    String::from_utf8_lossy(&cmd.output().unwrap().stdout).into_owned()
}

fn stderr(cmd: &mut Command) -> String {
    String::from_utf8_lossy(&cmd.output().unwrap().stderr).into_owned()
}

fn exit_ok(cmd: &mut Command) -> bool {
    cmd.output().unwrap().status.success()
}

// ── tnote help ────────────────────────────────────────────────────────────────

#[test]
fn help_shows_usage() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).arg("help"));
    assert!(out.contains("USAGE"));
    assert!(out.contains("tnote clean"));
}

// ── tnote path ────────────────────────────────────────────────────────────────

#[test]
fn path_prints_a_file_path() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).arg("path"));
    let p = Path::new(out.trim());
    assert!(p.to_str().unwrap().ends_with(".md"));
}

// ── tnote show ────────────────────────────────────────────────────────────────

#[test]
fn show_empty_when_no_note() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).arg("show"));
    assert!(out.contains("empty"));
}

#[test]
fn show_displays_note_content() {
    let dir = TempDir::new().unwrap();
    // Get the path for the current shell context
    let path_out = stdout(tnote(dir.path()).arg("path"));
    let note_path = Path::new(path_out.trim()).to_path_buf();
    fs::create_dir_all(note_path.parent().unwrap()).unwrap();
    fs::write(&note_path, "## hello\n- item one\n").unwrap();

    let out = stdout(tnote(dir.path()).arg("show"));
    assert!(out.contains("hello"));
    assert!(out.contains("item one"));
}

// ── tnote list ────────────────────────────────────────────────────────────────

#[test]
fn list_empty_when_no_notes() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    let out = stdout(tnote(dir.path()).arg("list"));
    assert!(out.contains("no notes yet"));
}

#[test]
fn list_shows_named_notes() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-backend.md"), "content\n").unwrap();
    let out = stdout(tnote(dir.path()).arg("list"));
    assert!(out.contains("backend"));
    assert!(out.contains("named"));
}

#[test]
fn list_shows_shell_notes() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("shell-99991.md"), "line1\nline2\n").unwrap();
    let out = stdout(tnote(dir.path()).arg("list"));
    assert!(out.contains("shell"));
    assert!(out.contains("2 lines"));
}

#[test]
fn list_marks_current_note() {
    let dir = TempDir::new().unwrap();
    let path_out = stdout(tnote(dir.path()).arg("path"));
    let note_path = Path::new(path_out.trim()).to_path_buf();
    fs::create_dir_all(note_path.parent().unwrap()).unwrap();
    fs::write(&note_path, "content\n").unwrap();
    let out = stdout(tnote(dir.path()).arg("list"));
    assert!(out.contains("◀"));
}

// ── tnote name ────────────────────────────────────────────────────────────────

#[test]
fn name_creates_link_and_renames() {
    let dir = TempDir::new().unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["name", "myproject"])));
    let out = stdout(tnote(dir.path()).arg("path"));
    assert!(out.trim().ends_with("named-myproject.md"));
}

#[test]
fn name_migrates_existing_content() {
    let dir = TempDir::new().unwrap();
    // Write content to the shell note first
    let path_out = stdout(tnote(dir.path()).arg("path"));
    let note_path = Path::new(path_out.trim()).to_path_buf();
    fs::create_dir_all(note_path.parent().unwrap()).unwrap();
    fs::write(&note_path, "existing content\n").unwrap();

    assert!(exit_ok(tnote(dir.path()).args(["name", "migrated"])));
    let named = dir.path().join("named-migrated.md");
    assert!(named.exists());
    assert_eq!(fs::read_to_string(&named).unwrap(), "existing content\n");
}

#[test]
fn name_output_says_named() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).args(["name", "testname"]));
    assert!(out.contains("testname"));
}

#[test]
fn name_bind_accepts_tmux_window_key() {
    let dir = TempDir::new().unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["name", "boundproj", "--bind", "$9+@17"])));
    let named = dir.path().join("named-boundproj.md");
    assert!(named.exists());
    let link = dir.path().join("meta").join("tmux-$9+@17.link");
    assert_eq!(fs::read_to_string(link).unwrap(), "boundproj");
}

#[test]
fn name_bind_boolean_uses_current_shell_key() {
    let dir = TempDir::new().unwrap();
    let path_out = stdout(tnote(dir.path()).arg("path"));
    let key = Path::new(path_out.trim())
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();

    assert!(exit_ok(tnote(dir.path()).args(["name", "boundproj", "--bind"])));
    let link = dir.path().join("meta").join(format!("{}.link", key));
    assert_eq!(fs::read_to_string(link).unwrap(), "boundproj");
}

#[test]
fn name_bind_accepts_prefixed_tmux_window_key() {
    let dir = TempDir::new().unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["name", "boundproj", "--bind", "tmux-$9+@17"])));
    let link = dir.path().join("meta").join("tmux-$9+@17.link");
    assert_eq!(fs::read_to_string(link).unwrap(), "boundproj");
}

#[test]
fn name_bind_accepts_shell_pid() {
    let dir = TempDir::new().unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["name", "shellproj", "--bind", "4242"])));
    let named = dir.path().join("named-shellproj.md");
    assert!(named.exists());
    let link = dir.path().join("meta").join("shell-4242.link");
    assert_eq!(fs::read_to_string(link).unwrap(), "shellproj");
}

#[test]
fn name_bind_accepts_prefixed_shell_pid() {
    let dir = TempDir::new().unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["name", "shellproj", "--bind", "shell-4242"])));
    let link = dir.path().join("meta").join("shell-4242.link");
    assert_eq!(fs::read_to_string(link).unwrap(), "shellproj");
}

#[test]
fn name_bind_rejects_invalid_target() {
    let dir = TempDir::new().unwrap();
    let output = tnote(dir.path())
        .args(["name", "broken", "--bind", "not-a-key"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("invalid bind target"));
}

#[test]
fn name_bind_rejects_invalid_prefixed_targets() {
    let dir = TempDir::new().unwrap();

    let bad_tmux = tnote(dir.path())
        .args(["name", "broken", "--bind", "tmux-$9+17"])
        .output()
        .unwrap();
    assert!(!bad_tmux.status.success());

    let bad_shell = tnote(dir.path())
        .args(["name", "broken", "--bind", "shell-notapid"])
        .output()
        .unwrap();
    assert!(!bad_shell.status.success());
}

#[test]
fn name_unbind_removes_specific_tmux_binding() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-boundproj.md"), "data\n").unwrap();
    fs::write(dir.path().join("meta").join("tmux-$9+@17.link"), "boundproj").unwrap();
    fs::write(dir.path().join("meta").join("shell-4242.link"), "boundproj").unwrap();

    assert!(exit_ok(tnote(dir.path()).args(["name", "boundproj", "--unbind", "$9+@17"])));
    assert!(!dir.path().join("meta").join("tmux-$9+@17.link").exists());
    assert!(dir.path().join("meta").join("shell-4242.link").exists());
}

#[test]
fn name_unbind_boolean_removes_all_bindings_for_note() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-boundproj.md"), "data\n").unwrap();
    fs::write(dir.path().join("meta").join("shell-4242.link"), "boundproj").unwrap();
    fs::write(dir.path().join("meta").join("tmux-$9+@17.link"), "boundproj").unwrap();
    fs::write(dir.path().join("meta").join("shell-9999.link"), "otherproj").unwrap();

    assert!(exit_ok(tnote(dir.path()).args(["name", "boundproj", "--unbind"])));
    assert!(!dir.path().join("meta").join("shell-4242.link").exists());
    assert!(!dir.path().join("meta").join("tmux-$9+@17.link").exists());
    assert!(dir.path().join("meta").join("shell-9999.link").exists());
}

#[test]
fn name_unbind_rejects_key_bound_to_other_note() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-boundproj.md"), "data\n").unwrap();
    fs::write(dir.path().join("meta").join("tmux-$9+@17.link"), "otherproj").unwrap();

    let output = tnote(dir.path())
        .args(["name", "boundproj", "--unbind", "$9+@17"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("is not bound to 'boundproj'"));
}

#[test]
fn name_rejects_bind_and_unbind_together() {
    let dir = TempDir::new().unwrap();
    let output = tnote(dir.path())
        .args(["name", "boundproj", "--bind", "$9+@17", "--unbind", "4242"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

// ── tnote clean ───────────────────────────────────────────────────────────────

#[test]
fn clean_nothing_to_clean() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    let out = stdout(tnote(dir.path()).arg("clean"));
    assert!(out.contains("nothing to clean"));
}

#[test]
fn clean_removes_dead_shell_note() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();

    // Spawn a process, let it die, use its PID
    let mut child = Command::new("true").spawn().unwrap();
    let dead = child.id();
    child.wait().unwrap();

    fs::write(dir.path().join(format!("shell-{}.md", dead)), "stale\n").unwrap();
    assert!(exit_ok(tnote(dir.path()).arg("clean")));
    assert!(!dir.path().join(format!("shell-{}.md", dead)).exists());
}

#[test]
fn clean_dryrun_does_not_delete() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();

    let mut child = Command::new("true").spawn().unwrap();
    let dead = child.id();
    child.wait().unwrap();
    let note = dir.path().join(format!("shell-{}.md", dead));
    fs::write(&note, "stale\n").unwrap();

    let out = stdout(tnote(dir.path()).args(["clean", "--dryrun"]));
    assert!(out.contains("would remove"));
    assert!(note.exists()); // still there
}

#[test]
fn clean_named_removes_named_note() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-deleteme.md"), "bye\n").unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["clean", "--name", "deleteme"])));
    assert!(!dir.path().join("named-deleteme.md").exists());
}

#[test]
fn clean_named_not_found_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    let status = tnote(dir.path()).args(["clean", "--name", "ghost"]).output().unwrap().status;
    assert!(!status.success());
    let err = stderr(tnote(dir.path()).args(["clean", "--name", "ghost"]));
    assert!(err.contains("not found"));
}

#[test]
fn clean_all_removes_named_notes() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-work.md"), "content\n").unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["clean", "--all", "named"])));
    assert!(!dir.path().join("named-work.md").exists());
}

#[test]
fn clean_all_scope_removes_all() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-work.md"), "content\n").unwrap();
    fs::write(dir.path().join("orphan.md"), "content\n").unwrap();
    assert!(exit_ok(tnote(dir.path()).args(["clean", "--all", "all"])));
    assert!(!dir.path().join("named-work.md").exists());
}

// ── tnote popup (hidden) ──────────────────────────────────────────────────────

#[test]
fn popup_runs_editor_on_note_file() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    // EDITOR=true just exits 0 immediately — popup should exit cleanly
    let status = tnote(dir.path())
        .args(["popup", "shell-11111"])
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    assert!(dir.path().join("shell-11111.md").exists());
}

// ── tmux-aware paths (fake tmux binary) ──────────────────────────────────────

#[test]
fn open_in_tmux_uses_popup_session() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    // tnote with no args + TMUX set → calls open_popup_session → fake tmux exits 0
    let status = tnote_in_tmux(dir.path(), home.path())
        .output()
        .unwrap()
        .status;
    assert!(status.success());
}

#[test]
fn show_in_tmux_falls_back_to_shell_note() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    // Create a shell note; tmux note doesn't exist (fake tmux returns empty IDs)
    let path_out = stdout(tnote(dir.path()).arg("path"));
    let note_path = Path::new(path_out.trim()).to_path_buf();
    fs::create_dir_all(note_path.parent().unwrap()).unwrap();
    fs::write(&note_path, "shell content\n").unwrap();

    let out = stdout(tnote_in_tmux(dir.path(), home.path()).arg("show"));
    assert!(out.contains("shell content"));
}

#[test]
fn list_in_tmux_shows_notes() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-proj.md"), "content\n").unwrap();
    let out = stdout(tnote_in_tmux(dir.path(), home.path()).arg("list"));
    assert!(out.contains("proj"));
}

#[test]
fn clean_in_tmux_kills_orphaned_popup_session() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    // Fake tmux returns "tnote-popup-orphan:0" from list-sessions;
    // no matching note file exists → session is killed (fake tmux exits 0)
    let out = stdout(tnote_in_tmux(dir.path(), home.path()).arg("clean"));
    assert!(out.contains("tnote-popup-orphan") || out.contains("nothing to clean"));
}

#[test]
fn name_in_tmux_renames_window() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    // rename-window is called → fake tmux exits 0
    let status = tnote_in_tmux(dir.path(), home.path())
        .args(["name", "tmuxwin"])
        .output()
        .unwrap()
        .status;
    assert!(status.success());
}

#[test]
fn name_without_argument_in_tmux_opens_name_menu() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let log = dir.path().join("fake-tmux.log");

    fs::write(dir.path().join("named-alpha.md"), "").unwrap();
    fs::write(dir.path().join("named-beta project.md"), "").unwrap();

    let status = Command::new(binary())
        .env("TNOTE_DIR", dir.path())
        .env("HOME", home.path())
        .env("TMUX", "/tmp/fake-tmux,0,0")
        .env("EDITOR", "true")
        .env("FAKE_TMUX_LOG", &log)
        .env("PATH", format!("{}:{}", fake_tmux_bin_dir().display(),
                             std::env::var("PATH").unwrap_or_default()))
        .arg("name")
        .output()
        .unwrap()
        .status;

    assert!(status.success());
    let recorded = fs::read_to_string(&log).unwrap();
    assert!(recorded.contains("display-menu"));
    assert!(recorded.contains("New name..."));
    assert!(recorded.contains("alpha"));
    assert!(recorded.contains("beta project"));
}

// ── tnote setup / uninstall ───────────────────────────────────────────────────

#[test]
fn setup_writes_tmux_conf_and_updates_tmux_conf_file() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    // Pipe newlines to accept all defaults in the interactive prompts
    let mut child = Command::new(binary())
        .env("TNOTE_DIR", dir.path())
        .env("HOME", home.path())
        .env("TMUX", "/tmp/fake-tmux,0,0")
        .env("EDITOR", "true")
        .env("PATH", format!("{}:{}", fake_tmux_bin_dir().display(),
                             std::env::var("PATH").unwrap_or_default()))
        .arg("setup")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.take().unwrap().write_all(b"\n\n\n\n").unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(dir.path().join("meta").join("tmux.conf").exists());
    let user_conf = home.path().join(".tmux.conf");
    assert!(user_conf.exists());
    let content = fs::read_to_string(&user_conf).unwrap();
    assert!(content.contains("source-file"));
}

#[test]
fn uninstall_removes_source_line_from_tmux_conf() {
    let dir  = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    // Pre-create the tmux.conf with a binding and a ~/.tmux.conf with source line
    let meta = dir.path().join("meta");
    fs::create_dir_all(&meta).unwrap();
    let tnote_conf = meta.join("tmux.conf");
    fs::write(&tnote_conf, "bind-key t run-shell 'tnote'\n").unwrap();
    let user_conf = home.path().join(".tmux.conf");
    fs::write(&user_conf, format!("source-file {}\n", tnote_conf.display())).unwrap();

    let status = Command::new(binary())
        .env("TNOTE_DIR", dir.path())
        .env("HOME", home.path())
        .env("TMUX", "/tmp/fake-tmux,0,0")
        .env("PATH", format!("{}:{}", fake_tmux_bin_dir().display(),
                             std::env::var("PATH").unwrap_or_default()))
        .arg("uninstall")
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    let content = fs::read_to_string(&user_conf).unwrap();
    assert!(!content.contains("source-file"));
}

#[test]
fn complete_named_notes_prints_existing_named_notes() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-beta.md"), "").unwrap();
    fs::write(dir.path().join("named-alpha.md"), "").unwrap();
    fs::write(dir.path().join("shell-123.md"), "").unwrap();

    let out = stdout(tnote(dir.path()).args(["__complete-named-notes"]));
    assert_eq!(out, "alpha\nbeta\n");
}

#[test]
fn completions_bash_include_dynamic_named_note_helper() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).args(["completions", "bash"]));
    assert!(out.contains("__complete-named-notes"));
    assert!(out.contains("complete -F _tnote tnote"));
}

#[test]
fn completions_zsh_include_dynamic_named_note_helper() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).args(["completions", "zsh"]));
    assert!(out.contains("__complete-named-notes"));
    assert!(out.contains("compdef _tnote tnote"));
}

#[test]
fn completions_fish_include_dynamic_named_note_helper() {
    let dir = TempDir::new().unwrap();
    let out = stdout(tnote(dir.path()).args(["completions", "fish"]));
    assert!(out.contains("__complete-named-notes"));
    assert!(out.contains("complete -c tnote"));
}

// ── --name flag ───────────────────────────────────────────────────────────────

#[test]
fn show_name_not_found_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let status = tnote(dir.path()).args(["show", "--name", "ghost"]).output().unwrap().status;
    assert!(!status.success());
    let err = stderr(&mut tnote(dir.path()).args(["show", "--name", "ghost"]));
    assert!(err.contains("named note 'ghost' not found"));
}

#[test]
fn path_name_not_found_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let status = tnote(dir.path()).args(["path", "--name", "ghost"]).output().unwrap().status;
    assert!(!status.success());
    let err = stderr(&mut tnote(dir.path()).args(["path", "--name", "ghost"]));
    assert!(err.contains("named note 'ghost' not found"));
}

#[test]
fn show_name_prints_named_note() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-todo.md"), "buy milk\n").unwrap();
    let output = tnote(dir.path()).args(["show", "--name", "todo"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("buy milk"));
}

#[test]
fn path_name_prints_named_path() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-todo.md"), "").unwrap();
    let output = tnote(dir.path()).args(["path", "--name", "todo"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("named-todo.md"));
}

#[test]
fn clean_name_removes_named_note() {
    let dir = TempDir::new().unwrap();
    let note = dir.path().join("named-temp.md");
    fs::write(&note, "data").unwrap();
    assert!(exit_ok(&mut tnote(dir.path()).args(["clean", "--name", "temp"])));
    assert!(!note.exists());
}

#[test]
fn clean_name_not_found_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let status = tnote(dir.path()).args(["clean", "--name", "ghost"]).output().unwrap().status;
    assert!(!status.success());
}

#[test]
fn show_name_shorthand_works() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-x.md"), "hello\n").unwrap();
    let output = tnote(dir.path()).args(["show", "-n", "x"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"));
}

#[test]
fn show_name_glob_matches_multiple_notes() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-proj-auth-login.md"),   "login content\n").unwrap();
    fs::write(dir.path().join("named-proj-auth-session.md"), "session content\n").unwrap();
    fs::write(dir.path().join("named-other-api-search.md"),  "search content\n").unwrap();
    let output = tnote(dir.path()).args(["show", "--name", "proj-*"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("login content"));
    assert!(stdout.contains("session content"));
    assert!(!stdout.contains("search content"));
}

#[test]
fn show_name_glob_no_matches_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let status = tnote(dir.path()).args(["show", "--name", "ghost-*"]).output().unwrap().status;
    assert!(!status.success());
}

#[test]
fn show_name_glob_mid_pattern() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-proj-auth-login.md"),  "auth login\n").unwrap();
    fs::write(dir.path().join("named-proj-api-login.md"),   "api login\n").unwrap();
    fs::write(dir.path().join("named-proj-auth-logout.md"), "auth logout\n").unwrap();
    let output = tnote(dir.path()).args(["show", "--name", "proj-*-login"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("auth login"));
    assert!(stdout.contains("api login"));
    assert!(!stdout.contains("auth logout"));
}

#[test]
fn ls_is_alias_for_list() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-foo.md"), "content\n").unwrap();
    let out_list = tnote(dir.path()).args(["list"]).output().unwrap();
    let out_ls   = tnote(dir.path()).args(["ls"]).output().unwrap();
    assert!(out_ls.status.success());
    assert_eq!(out_list.stdout, out_ls.stdout);
}

#[test]
fn name_unbind_boolean_removes_named_note_link_files() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("meta")).unwrap();
    fs::write(dir.path().join("named-myproj.md"), "data\n").unwrap();
    fs::write(dir.path().join("meta").join("tmux-work+0.link"), "myproj").unwrap();
    fs::write(dir.path().join("meta").join("tmux-work+1.link"), "myproj").unwrap();
    assert!(exit_ok(&mut tnote(dir.path()).args(["name", "myproj", "--unbind"])));
    assert!(!dir.path().join("meta").join("tmux-work+0.link").exists());
    assert!(!dir.path().join("meta").join("tmux-work+1.link").exists());
    assert!(dir.path().join("named-myproj.md").exists());
}

#[test]
fn name_unbind_boolean_not_bound_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("named-myproj.md"), "data\n").unwrap();
    let status = tnote(dir.path()).args(["name", "myproj", "--unbind"]).output().unwrap().status;
    assert!(!status.success());
}
