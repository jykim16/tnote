use std::path::{Path, PathBuf};
use std::process::Command;

const REPO: &str = "jykim16/tnote";

/// The real, symlink-resolved path of the binary currently running.
fn current_exe_canonical() -> Option<PathBuf> {
    std::env::current_exe().ok()?.canonicalize().ok()
}

/// Resolve the `releases/latest` redirect to read off the current release tag,
/// without needing a JSON parser or hitting the rate-limited API host.
fn latest_release_tag() -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "-sSL",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            &format!("https://github.com/{REPO}/releases/latest"),
        ])
        .output()
        .map_err(|e| format!("failed to run curl: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "curl exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    url.rsplit('/')
        .next()
        .filter(|s| s.starts_with('v'))
        .map(|s| s.to_string())
        .ok_or_else(|| format!("unexpected response resolving latest release: {url}"))
}

fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Whether the binary actually running is the Homebrew-managed one, per
/// `Cargo.toml`'s `workspace.metadata.dist` tap config. `brew --prefix tnote`
/// returns Homebrew's opt-symlink for the formula regardless of whether it's
/// installed; canonicalizing it and requiring the running exe to live under
/// it confirms this specific binary came from Homebrew, rather than just
/// "Homebrew has a tnote installed somewhere on this machine" (a real bug
/// class other self-updaters have hit when a stray install sits elsewhere
/// on PATH).
fn installed_via_homebrew() -> bool {
    let Some(current_exe) = current_exe_canonical() else {
        return false;
    };
    let Ok(output) = Command::new("brew").args(["--prefix", "tnote"]).output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let Ok(prefix) = Path::new(&prefix).canonicalize() else {
        return false;
    };
    current_exe.starts_with(&prefix)
}

/// `$CARGO_HOME/bin` (or `~/.cargo/bin`), canonicalized.
fn cargo_bin_dir() -> Option<PathBuf> {
    let home = match std::env::var("CARGO_HOME") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(std::env::var("HOME").ok()?).join(".cargo"),
    };
    home.join("bin").canonicalize().ok()
}

/// Detail string (e.g. `v0.3.8 (/path/to/checkout)`) if the binary actually
/// running looks like a `cargo install` build rather than a packaged
/// release — i.e. it lives in cargo's own bin dir *and* `cargo install
/// --list` confirms cargo put it there, mirroring the same "is this specific
/// binary theirs" check used for Homebrew above. `tnote` isn't published to
/// crates.io (see `Cargo.toml`'s `publish-jobs`), and the packaged
/// installers don't touch `~/.cargo/bin`, so neither upgrade path can
/// safely take over a cargo-installed binary — surface it instead of
/// silently upgrading a binary that PATH won't actually pick up.
fn installed_via_cargo() -> Option<String> {
    let current_exe = current_exe_canonical()?;
    let bin_dir = cargo_bin_dir()?;
    if current_exe.parent() != Some(bin_dir.as_path()) {
        return None;
    }

    let output = Command::new("cargo").args(["install", "--list"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("tnote "))
        .map(|rest| rest.trim_end_matches(':').trim().to_string())
}

fn run_upgrade_command(mut cmd: Command, label: &str, tag: &str) {
    match cmd.status() {
        Ok(s) if s.success() => println!("tnote upgrade: upgraded to {tag}"),
        Ok(s) => {
            eprintln!("tnote upgrade: {label} exited with {s}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("tnote upgrade: failed to run {label}: {e}");
            std::process::exit(1);
        }
    }
}

pub fn run() {
    let current = env!("CARGO_PKG_VERSION");
    let current_v = parse_version(current).unwrap_or((0, 0, 0));

    let tag = match latest_release_tag() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("tnote upgrade: failed to check for updates: {e}");
            std::process::exit(1);
        }
    };

    let latest_v = match parse_version(&tag) {
        Some(v) => v,
        None => {
            eprintln!("tnote upgrade: failed to parse latest version from tag '{tag}'");
            std::process::exit(1);
        }
    };

    if latest_v <= current_v {
        println!("tnote upgrade: already up to date (v{current})");
        return;
    }

    if installed_via_homebrew() {
        println!("tnote upgrade: upgrading v{current} -> {tag} (via Homebrew)");
        let mut cmd = Command::new("brew");
        cmd.args(["upgrade", "tnote"]);
        run_upgrade_command(cmd, "brew upgrade", &tag);
    } else if let Some(detail) = installed_via_cargo() {
        eprintln!(
            "tnote upgrade: this appears to be a `cargo install` build ({detail}), not a packaged release."
        );
        eprintln!(
            "tnote upgrade: pull the latest changes in that checkout and run `cargo install --path .` to update — `tnote upgrade` only knows how to fetch packaged GitHub releases (curl installer / Homebrew), and tnote isn't published to crates.io."
        );
        std::process::exit(1);
    } else {
        println!("tnote upgrade: upgrading v{current} -> {tag}");
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(format!(
            "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/{REPO}/releases/latest/download/tnote-installer.sh | sh"
        ));
        run_upgrade_command(cmd, "installer", &tag);
    }
}
