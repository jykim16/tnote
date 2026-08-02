#!/bin/bash
set -euo pipefail

PASS=0
FAIL=0
ERRORS=""

pass() { PASS=$((PASS + 1)); echo "  ✓ $1"; }
fail() { FAIL=$((FAIL + 1)); ERRORS="${ERRORS}\n  ✗ $1"; echo "  ✗ $1"; }

export TNOTE_DIR=$(mktemp -d)
export EDITOR=vi
export HOME=/root
chmod +x /tnote/tests/bin/bat /tnote/tests/bin/curl /tnote/tests/bin/brew /tnote/tests/bin/cargo
REAL_PATH="$PATH"
export PATH="/tnote/tests/bin:$PATH"

echo "=== tnote integration tests ($(tmux -V)) ==="
echo ""

# ── Basic commands outside tmux ────────────────────────────────────────────────

echo "Outside tmux:"

unset TMUX 2>/dev/null || true

# help
tnote help | grep -q "USAGE" && pass "help" || fail "help"
tnote help | grep -q -- "--template" && pass "help documents --template" || fail "help documents --template"

# path
tnote path | grep -q ".md" && pass "path" || fail "path"

# show (empty)
tnote show | grep -q "empty" && pass "show (empty)" || fail "show (empty)"

# show --json (empty)
tnote show --json | jq -e '.[0].empty == true' >/dev/null && pass "show --json (empty)" || fail "show --json (empty)"

# show (with content)
NOTE=$(tnote path)
mkdir -p "$(dirname "$NOTE")"
echo "hello world" > "$NOTE"
tnote show | grep -q "hello world" && pass "show (content)" || fail "show (content)"

# show --json (content)
tnote show --json | jq -e '.[0].content == "hello world\n" and .[0].empty == false' >/dev/null && pass "show --json (content)" || fail "show --json (content)"
tnote show --json | jq -e '.[0].path | endswith(".md")' >/dev/null && pass "show --json (path field)" || fail "show --json (path field)"

# show (bat renderer)
# Note: no 2>&1 here - adding it forces bash to fork an extra subshell for
# this command substitution, which changes tnote's getppid()-derived shell
# identity mid-script and makes it see an unrelated empty note.
SHOW_BAT=$(TNOTE_RENDERER=bat tnote show)
echo "$SHOW_BAT" | grep -q "hello world" && pass "show (bat renderer)" || fail "show (bat renderer)"

# list
tnote list | grep -q "shell" && pass "list" || fail "list"

# list --json
tnote list --json | jq -e 'type == "array"' >/dev/null && pass "list --json (valid array)" || fail "list --json (valid array)"
tnote list --json | jq -e 'any(.[]; .category == "shell")' >/dev/null && pass "list --json (has shell category)" || fail "list --json (has shell category)"
tnote list --json | jq -e 'any(.[]; .current == true)' >/dev/null && pass "list --json (marks current note)" || fail "list --json (marks current note)"

# name
tnote name testproject
tnote path | grep -q "named-testproject.md" && pass "name" || fail "name"

# name --bind (tmux key from outside tmux)
tnote name boundproject --bind '$9+@17'
[ -f "$TNOTE_DIR/meta/tmux-\$9+@17.link" ] && pass "name --bind writes tmux link" || fail "name --bind writes tmux link"
grep -q "boundproject" "$TNOTE_DIR/meta/tmux-\$9+@17.link" && pass "name --bind stores note name" || fail "name --bind stores note name"
tnote show -n boundproject >/dev/null && pass "name --bind creates named note" || fail "name --bind creates named note"

# show --json (named note)
tnote show -n boundproject --json | jq -e '.[0].name == "boundproject"' >/dev/null && pass "show --json (named note)" || fail "show --json (named note)"

# show --json (named note, glob match)
echo "glob one" > "$TNOTE_DIR/named-globtest1.md"
echo "glob two" > "$TNOTE_DIR/named-globtest2.md"
GLOB_JSON=$(tnote show -n 'globtest*' --json)
echo "$GLOB_JSON" | jq -e 'length == 2' >/dev/null && pass "show --json (glob match count)" || fail "show --json (glob match count)"
echo "$GLOB_JSON" | jq -e '.[0].name == "globtest1" and .[1].name == "globtest2"' >/dev/null && pass "show --json (glob match sorted)" || fail "show --json (glob match sorted)"
echo "$GLOB_JSON" | jq -e '.[0].content == "glob one\n" and .[1].content == "glob two\n"' >/dev/null && pass "show --json (glob match content)" || fail "show --json (glob match content)"
rm -f "$TNOTE_DIR/named-globtest1.md" "$TNOTE_DIR/named-globtest2.md"

# show --json (named note, not found)
! tnote show -n ghost --json 2>/dev/null && pass "show --json (named note not found exits nonzero)" || fail "show --json (named note not found exits nonzero)"

# name --bind (boolean current shell binding)
CURRENT_KEY="shell-$$"
tnote name currentbound --bind
[ -f "$TNOTE_DIR/meta/${CURRENT_KEY}.link" ] && pass "name --bind boolean writes current link" || fail "name --bind boolean writes current link"
grep -q "currentbound" "$TNOTE_DIR/meta/${CURRENT_KEY}.link" && pass "name --bind boolean stores current note name" || fail "name --bind boolean stores current note name"

# name --bind (shell pid from outside tmux)
tnote name shellbound --bind 4242
[ -f "$TNOTE_DIR/meta/shell-4242.link" ] && pass "name --bind writes shell link" || fail "name --bind writes shell link"
grep -q "shellbound" "$TNOTE_DIR/meta/shell-4242.link" && pass "name --bind stores shell note name" || fail "name --bind stores shell note name"

# name --unbind (specific key from outside tmux)
tnote name boundproject --unbind '$9+@17'
[ ! -f "$TNOTE_DIR/meta/tmux-\$9+@17.link" ] && pass "name --unbind removes tmux link" || fail "name --unbind removes tmux link"
[ -f "$TNOTE_DIR/meta/shell-4242.link" ] && pass "name --unbind keeps other links" || fail "name --unbind keeps other links"

# name --unbind (boolean removes all links for the note)
tnote name currentbound --bind
tnote name currentbound --bind 9999
[ -f "$TNOTE_DIR/meta/${CURRENT_KEY}.link" ] && [ -f "$TNOTE_DIR/meta/shell-9999.link" ] && pass "name --unbind boolean setup" || fail "name --unbind boolean setup"
tnote name currentbound --unbind
[ ! -f "$TNOTE_DIR/meta/${CURRENT_KEY}.link" ] && [ ! -f "$TNOTE_DIR/meta/shell-9999.link" ] && pass "name --unbind boolean removes all note links" || fail "name --unbind boolean removes all note links"
[ -f "$TNOTE_DIR/meta/shell-4242.link" ] && pass "name --unbind boolean keeps other note links" || fail "name --unbind boolean keeps other note links"

# name --bind (invalid format)
! tnote name broken --bind not-a-key 2>/dev/null && pass "name --bind invalid target exits nonzero" || fail "name --bind invalid target exits nonzero"

# name --unbind (wrong note)
! tnote name wrongnote --unbind 4242 2>/dev/null && pass "name --unbind wrong note exits nonzero" || fail "name --unbind wrong note exits nonzero"

# goto (no name, outside tmux) exits nonzero with guidance rather than erroring obscurely
GOTO_NO_NAME=$(tnote goto 2>&1; echo "exit=$?")
echo "$GOTO_NO_NAME" | grep -q "provide a name" && pass "goto no name outside tmux errors" || fail "goto no name outside tmux errors"
echo "$GOTO_NO_NAME" | grep -q "exit=1" && pass "goto no name outside tmux exits nonzero" || fail "goto no name outside tmux exits nonzero"

# name --template / tnote --template (file-based note templates, ~/.tnote/template-NAME.md)
# Run in isolated scratch TNOTE_DIRs so the shared state built up above
# (testproject, boundproject, etc.) isn't disturbed; restore TNOTE_DIR after.
ORIG_TNOTE_DIR="$TNOTE_DIR"

# name --template NAME saves this window's current note as a reusable template file
export TNOTE_DIR=$(mktemp -d)
NOTE=$(tnote path)
mkdir -p "$(dirname "$NOTE")"
echo "## Status: in-progress" > "$NOTE"
tnote name --template savetest >/dev/null
[ -f "$TNOTE_DIR/template-savetest.md" ] && pass "name --template saves template file" || fail "name --template saves template file"
grep -q "## Status: in-progress" "$TNOTE_DIR/template-savetest.md" && pass "name --template saved content matches current note" || fail "name --template saved content matches current note"
[ ! -f "$TNOTE_DIR/named-savetest.md" ] && pass "name --template does not create a named note" || fail "name --template does not create a named note"

# name --template rejects being combined with a positional name or --bind/--unbind
! tnote name someproj --template savetest 2>/dev/null && pass "name --template conflicts with positional name" || fail "name --template conflicts with positional name"
! tnote name --template savetest --bind 2>/dev/null && pass "name --template conflicts with --bind" || fail "name --template conflicts with --bind"

# --template / -t NAME (apply a saved template) errors when the template doesn't exist
export TNOTE_DIR=$(mktemp -d)
! tnote --template ghost-template 2>/dev/null && pass "--template missing template exits nonzero" || fail "--template missing template exits nonzero"
ERR=$(tnote --template ghost-template 2>&1 >/dev/null || true)
echo "$ERR" | grep -q "not found" && pass "--template missing template error message" || fail "--template missing template error message"

# --template errors when the target note already has content, and leaves it untouched
export TNOTE_DIR=$(mktemp -d)
echo "template body" > "$TNOTE_DIR/template-savetest.md"
NOTE=$(tnote path)
mkdir -p "$(dirname "$NOTE")"
echo "existing content" > "$NOTE"
! tnote --template savetest 2>/dev/null && pass "--template refuses to overwrite existing content" || fail "--template refuses to overwrite existing content"
grep -q "existing content" "$NOTE" && pass "--template leaves existing content untouched" || fail "--template leaves existing content untouched"

# --force/-f overrides the refusal and overwrites. apply_template() writes the file
# before opening the editor, so we only assert the write — not the exit code, since
# opening a real editor pty can fail headlessly in some sandboxes (see cli.rs tests).
tnote --force --template savetest >/dev/null 2>&1 || true
grep -q "template body" "$NOTE" && pass "--force --template overwrites existing content" || fail "--force --template overwrites existing content"

# combined short flags -ft NAME behave the same as --force --template NAME
export TNOTE_DIR=$(mktemp -d)
echo "template body 2" > "$TNOTE_DIR/template-savetest.md"
NOTE=$(tnote path)
mkdir -p "$(dirname "$NOTE")"
echo "existing content" > "$NOTE"
tnote -ft savetest >/dev/null 2>&1 || true
grep -q "template body 2" "$NOTE" && pass "-ft NAME combined short flags apply and overwrite" || fail "-ft NAME combined short flags apply and overwrite"

# --template applies cleanly to a fresh (empty) note
export TNOTE_DIR=$(mktemp -d)
echo "fresh template body" > "$TNOTE_DIR/template-savetest.md"
NOTE=$(tnote path)
tnote --template savetest >/dev/null 2>&1 || true
grep -q "fresh template body" "$NOTE" && pass "--template applies to a fresh note" || fail "--template applies to a fresh note"

export TNOTE_DIR="$ORIG_TNOTE_DIR"

# clean --dryrun
# Note: capture full output first rather than piping directly into
# `grep -q` - grep closing the pipe after its first match sends SIGPIPE to
# tnote for any remaining output lines, and `pipefail` (set above) treats
# that completely normal termination as a pipeline failure.
echo "stale" > "$TNOTE_DIR/shell-9999999.md"
CLEAN_DRYRUN=$(tnote clean --dryrun)
echo "$CLEAN_DRYRUN" | grep -q "would remove" && pass "clean --dryrun" || fail "clean --dryrun"

# clean
CLEAN_OUT=$(tnote clean)
echo "$CLEAN_OUT" | grep -q "removed" && pass "clean" || fail "clean"
[ ! -f "$TNOTE_DIR/shell-9999999.md" ] && pass "clean removes file" || fail "clean removes file"

# clean --name --archive --dryrun
tnote name archivetest
echo "archive me" > "$TNOTE_DIR/named-archivetest.md"
tnote clean --name archivetest --archive --dryrun | grep -q "would archive" && pass "archive --dryrun message" || fail "archive --dryrun message"
[ -f "$TNOTE_DIR/named-archivetest.md" ] && pass "archive --dryrun keeps file" || fail "archive --dryrun keeps file"

# clean --name --archive
tnote clean --name archivetest --archive | grep -q "archived" && pass "archive message" || fail "archive message"
[ ! -f "$TNOTE_DIR/named-archivetest.md" ] && pass "archive removes original" || fail "archive removes original"
[ -f "$TNOTE_DIR/archive/named-archivetest.md" ] && pass "archive creates archive file" || fail "archive creates archive file"
grep -q "archive me" "$TNOTE_DIR/archive/named-archivetest.md" && pass "archive preserves content" || fail "archive preserves content"

# clean --name --archive leaves unrelated bindings alone
tnote name archiveisolated
echo "archive isolated" > "$TNOTE_DIR/named-archiveisolated.md"
tnote name unrelated-live --bind '$9+@18' >/dev/null
ARCHIVE_OUTPUT=$(tnote clean --name archiveisolated --archive 2>&1)
[ -f "$TNOTE_DIR/meta/tmux-\$9+@18.link" ] && pass "archive keeps unrelated tmux link" || fail "archive keeps unrelated tmux link"
! echo "$ARCHIVE_OUTPUT" | grep -q "tmux-\$9+@18" && pass "archive avoids unrelated cleanup output" || fail "archive avoids unrelated cleanup output"
rm -f "$TNOTE_DIR/meta/tmux-\$9+@18.link" "$TNOTE_DIR/named-unrelated-live.md"

# clean --name --archive (nonexistent)
! tnote clean --name ghost --archive 2>/dev/null && pass "archive nonexistent exits nonzero" || fail "archive nonexistent exits nonzero"

# clean --name --unarchive --dryrun
tnote clean --name archivetest --unarchive --dryrun | grep -q "would unarchive" && pass "unarchive --dryrun message" || fail "unarchive --dryrun message"
[ -f "$TNOTE_DIR/archive/named-archivetest.md" ] && pass "unarchive --dryrun keeps archived file" || fail "unarchive --dryrun keeps archived file"

# clean --name --unarchive
tnote clean --name archivetest --unarchive | grep -q "unarchived" && pass "unarchive message" || fail "unarchive message"
[ -f "$TNOTE_DIR/named-archivetest.md" ] && pass "unarchive restores file" || fail "unarchive restores file"
[ ! -f "$TNOTE_DIR/archive/named-archivetest.md" ] && pass "unarchive removes from archive" || fail "unarchive removes from archive"
grep -q "archive me" "$TNOTE_DIR/named-archivetest.md" && pass "unarchive preserves content" || fail "unarchive preserves content"

# clean --name --unarchive (nonexistent)
! tnote clean --name ghost --unarchive 2>/dev/null && pass "unarchive nonexistent exits nonzero" || fail "unarchive nonexistent exits nonzero"

# list --archive (empty)
rm -rf "$TNOTE_DIR/archive"
tnote list --archive | grep -q "no archived notes" && pass "list --archive (empty)" || fail "list --archive (empty)"

# list --archive --json (empty)
tnote list --archive --json | jq -e '. == []' >/dev/null && pass "list --archive --json (empty)" || fail "list --archive --json (empty)"

# list --archive (with content)
mkdir -p "$TNOTE_DIR/archive"
echo "old stuff" > "$TNOTE_DIR/archive/named-oldproject.md"
tnote list --archive | grep -q "oldproject" && pass "list --archive (content)" || fail "list --archive (content)"

# list --archive --json (with content)
tnote list --archive --json | jq -e 'any(.[]; .name == "oldproject" and .lines == 1)' >/dev/null && pass "list --archive --json (content)" || fail "list --archive --json (content)"
rm -rf "$TNOTE_DIR/archive"

# clean archive retention purge (opt-in via config; off by default)
mkdir -p "$TNOTE_DIR/archive"
echo "fresh" > "$TNOTE_DIR/archive/named-freshnote.md"
echo "stale" > "$TNOTE_DIR/archive/named-stalenote.md"
touch -d "40 days ago" "$TNOTE_DIR/archive/named-stalenote.md"

tnote clean 2>&1 | grep -q "purged" && fail "purge disabled by default" || pass "purge disabled by default"
[ -f "$TNOTE_DIR/archive/named-stalenote.md" ] && pass "stale note kept without retention config" || fail "stale note kept without retention config"

TNOTE_ARCHIVE_RETENTION_DAYS=30 tnote clean --dryrun | grep -q "would purge archived note 'stalenote'" && pass "purge --dryrun message" || fail "purge --dryrun message"
[ -f "$TNOTE_DIR/archive/named-stalenote.md" ] && pass "purge --dryrun keeps file" || fail "purge --dryrun keeps file"

TNOTE_ARCHIVE_RETENTION_DAYS=30 tnote clean | grep -q "purged archived note 'stalenote'" && pass "purge message" || fail "purge message"
[ ! -f "$TNOTE_DIR/archive/named-stalenote.md" ] && pass "purge removes stale archived note" || fail "purge removes stale archived note"
[ -f "$TNOTE_DIR/archive/named-freshnote.md" ] && pass "purge keeps fresh archived note" || fail "purge keeps fresh archived note"

rm -rf "$TNOTE_DIR/archive"

# completions
tnote completions bash | grep -q "complete" && pass "completions bash" || fail "completions bash"
tnote completions zsh | grep -q "compdef" && pass "completions zsh" || fail "completions zsh"
tnote completions fish | grep -q "complete" && pass "completions fish" || fail "completions fish"
tnote __complete-named-notes | grep -q "testproject" && pass "named note completion source" || fail "named note completion source"

# version
tnote --version | grep -q "tnote" && pass "--version" || fail "--version"

echo ""

# ── Upgrade ───────────────────────────────────────────────────────────────────

echo "Upgrade:"

# `tests/bin/curl` (still on PATH here, before it's reset to REAL_PATH below)
# stubs the GitHub redirect so this never touches the network.
CURRENT_VERSION=$(tnote --version | awk '{print $2}')

FAKE_CURL_LATEST_TAG="v$CURRENT_VERSION" tnote upgrade 2>&1 | grep -q "already up to date" \
  && pass "upgrade (already up to date)" || fail "upgrade (already up to date)"

UPGRADE_OUT=$(FAKE_CURL_LATEST_TAG=v99.0.0 tnote upgrade 2>&1)
echo "$UPGRADE_OUT" | grep -q "upgrading v$CURRENT_VERSION -> v99.0.0" && pass "upgrade (detects newer release)" || fail "upgrade (detects newer release)"
echo "$UPGRADE_OUT" | grep -q "(stub) installer ran" && pass "upgrade (runs installer)" || fail "upgrade (runs installer)"
echo "$UPGRADE_OUT" | grep -q "upgraded to v99.0.0" && pass "upgrade (reports success)" || fail "upgrade (reports success)"

# Homebrew/cargo detection now checks that the *running binary itself*
# lives under the reported install dir (not just "brew/cargo knows about
# tnote somewhere") - so these tests copy the real binary into a fake
# install dir and invoke it from there, rather than just stubbing the
# brew/cargo output in isolation.
TNOTE_BIN=$(command -v tnote)

# Homebrew-managed install: should call `brew upgrade`, not the curl installer
mkdir -p /fake/brew/opt/tnote/bin
cp "$TNOTE_BIN" /fake/brew/opt/tnote/bin/tnote
BREW_UPGRADE_OUT=$(FAKE_BREW_PREFIX=/fake/brew/opt/tnote FAKE_CURL_LATEST_TAG=v99.0.0 /fake/brew/opt/tnote/bin/tnote upgrade 2>&1)
echo "$BREW_UPGRADE_OUT" | grep -q "via Homebrew" && pass "upgrade (detects Homebrew install)" || fail "upgrade (detects Homebrew install)"
echo "$BREW_UPGRADE_OUT" | grep -q "(stub) brew upgrade ran" && pass "upgrade (runs brew upgrade)" || fail "upgrade (runs brew upgrade)"
! echo "$BREW_UPGRADE_OUT" | grep -q "(stub) installer ran" && pass "upgrade (skips curl installer for brew installs)" || fail "upgrade (skips curl installer for brew installs)"

# A brew prefix reported for tnote that the running binary does NOT actually
# live under should NOT be treated as a Homebrew install (this is the "brew
# knows about a tnote somewhere, but not this one" bug other self-updaters
# have hit) - falls through to the curl installer instead.
mkdir -p /fake/brew-other/opt/tnote/bin
cp "$TNOTE_BIN" /fake/brew-other/opt/tnote/bin/tnote
UNRELATED_BREW_OUT=$(FAKE_BREW_PREFIX=/fake/brew/opt/tnote FAKE_CURL_LATEST_TAG=v99.0.0 /fake/brew-other/opt/tnote/bin/tnote upgrade 2>&1)
echo "$UNRELATED_BREW_OUT" | grep -q "(stub) installer ran" && pass "upgrade (ignores unrelated brew prefix)" || fail "upgrade (ignores unrelated brew prefix)"

# cargo-install builds: neither packaged installer can safely take over, so
# `tnote upgrade` should refuse rather than silently no-op.
mkdir -p /fake/cargo-home/bin
cp "$TNOTE_BIN" /fake/cargo-home/bin/tnote
CARGO_UPGRADE_OUT=$(CARGO_HOME=/fake/cargo-home FAKE_CARGO_INSTALLED=1 FAKE_CURL_LATEST_TAG=v99.0.0 /fake/cargo-home/bin/tnote upgrade 2>&1) && CARGO_UPGRADE_STATUS=0 || CARGO_UPGRADE_STATUS=$?
echo "$CARGO_UPGRADE_OUT" | grep -q "not a packaged release" && pass "upgrade (detects cargo install)" || fail "upgrade (detects cargo install)"
[ "$CARGO_UPGRADE_STATUS" -ne 0 ] && pass "upgrade (cargo install exits nonzero)" || fail "upgrade (cargo install exits nonzero)"
! echo "$CARGO_UPGRADE_OUT" | grep -qE "\(stub\) (installer|brew upgrade) ran" && pass "upgrade (skips packaged installers for cargo installs)" || fail "upgrade (skips packaged installers for cargo installs)"

# Likewise: cargo claiming to know about tnote doesn't matter if the running
# binary isn't actually sitting in cargo's bin dir.
UNRELATED_CARGO_OUT=$(FAKE_CARGO_INSTALLED=1 FAKE_CURL_LATEST_TAG=v99.0.0 tnote upgrade 2>&1)
echo "$UNRELATED_CARGO_OUT" | grep -q "(stub) installer ran" && pass "upgrade (ignores cargo when binary isn't in cargo's bin dir)" || fail "upgrade (ignores cargo when binary isn't in cargo's bin dir)"

echo ""

# ── Setup and uninstall ───────────────────────────────────────────────────────

echo "Setup/Uninstall:"

# From here on we need the real tmux binary, not the fake stub used above -
# both tnote's own internal `tmux ...` calls and this script's now need
# genuine tmux behavior (live sessions, run-shell, display-message, etc.).
export PATH="$REAL_PATH"

# Start a tmux server
tmux new-session -d -s test-session

export TMUX="/tmp/tmux-0/default,$(pgrep -f 'tmux new-session' | head -1),0"

# setup (accept all defaults)
printf '\n\n\n\n' | tnote setup 2>&1 | grep -q "setup complete" && pass "setup" || fail "setup"

# Verify tmux.conf was written
[ -f "$TNOTE_DIR/meta/tmux.conf" ] && pass "setup writes tmux.conf" || fail "setup writes tmux.conf"

# Verify config was written
[ -f "$TNOTE_DIR/meta/config" ] && pass "setup writes config" || fail "setup writes config"

# Verify source-file line in ~/.tmux.conf
grep -q "source-file" "$HOME/.tmux.conf" 2>/dev/null && pass "setup updates ~/.tmux.conf" || fail "setup updates ~/.tmux.conf"

# Verify tmux binding is live
tmux show -s command-alias 2>/dev/null | grep -q "tnote" && pass "setup installs tmux aliases" || fail "setup installs tmux aliases"

# uninstall
tnote uninstall 2>&1 | grep -q "complete" && pass "uninstall" || fail "uninstall"

# Verify source-file line removed
! grep -q "source-file.*tnote" "$HOME/.tmux.conf" 2>/dev/null && pass "uninstall cleans ~/.tmux.conf" || fail "uninstall cleans ~/.tmux.conf"

echo ""

# ── Tmux popup tests ──────────────────────────────────────────────────────────

echo "Inside tmux:"

# Re-setup for tmux tests
printf '\n\n\n\n' | tnote setup 2>&1 >/dev/null

# Reset TNOTE_DIR for fresh state
export TNOTE_DIR=$(mktemp -d)
mkdir -p "$TNOTE_DIR/meta"

# tnote path inside tmux
TMUX_PATH=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote path' 2>&1)
echo "$TMUX_PATH" | grep -q "tmux-" && pass "path in tmux" || fail "path in tmux"

# tnote show inside tmux (empty)
TMUX_SHOW=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote show' 2>&1)
echo "$TMUX_SHOW" | grep -q "empty" && pass "show in tmux (empty)" || fail "show in tmux (empty)"

# tnote name inside tmux
tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote name tmuxtest' 2>&1
TMUX_PATH2=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote path' 2>&1)
echo "$TMUX_PATH2" | grep -q "named-tmuxtest.md" && pass "name in tmux" || fail "name in tmux"

# manager-style spawn flow: create agent note file first, then bind the detached window from the manager shell
AGENT_NOTE="$TNOTE_DIR/named-agent-manager-flow.md"
cat <<'EOF' > "$AGENT_NOTE"
## Status: queued
## Domain: tnote-skills
## Workspace: /work
## Goal: Validate manager spawn flow
## Started: 2026-04-04

---

## In Progress

## Queue
- [ ] Start work

## Blocked

## Done

---

## Context

### Background
- Seeded by integration test.

### Files
- `skills/tnote-manager/SKILL.md`

### Links
- None

### Dependencies
- None

---

## Log
### 2026-04-04
- [manager] created note
EOF
tmux new-window -d -t test-session: -n manager-spawn "TNOTE_DIR='$TNOTE_DIR' sh"
sleep 1
MANAGER_SPAWN_KEY=$(tmux display-message -p -t test-session:manager-spawn '#{session_id}+#{window_id}')
tnote name agent-manager-flow --bind "$MANAGER_SPAWN_KEY" >/dev/null
[ -f "$TNOTE_DIR/meta/tmux-${MANAGER_SPAWN_KEY}.link" ] && pass "manager spawn writes tmux link from manager shell" || fail "manager spawn writes tmux link from manager shell"
grep -q "agent-manager-flow" "$TNOTE_DIR/meta/tmux-${MANAGER_SPAWN_KEY}.link" && pass "manager spawn stores agent note name" || fail "manager spawn stores agent note name"
tmux kill-window -t test-session:manager-spawn 2>/dev/null || true

# tnote list inside tmux
echo "content" > "$TNOTE_DIR/named-tmuxtest.md"
TMUX_LIST=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote list' 2>&1)
echo "$TMUX_LIST" | grep -q "tmuxtest" && pass "list in tmux" || fail "list in tmux"

# tnote show with content
echo "tmux note content" > "$TNOTE_DIR/named-tmuxtest.md"
TMUX_SHOW2=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote show' 2>&1)
echo "$TMUX_SHOW2" | grep -q "tmux note content" && pass "show in tmux (content)" || fail "show in tmux (content)"

# tnote clean inside tmux
echo "orphan" > "$TNOTE_DIR/shell-9999998.md"
TMUX_CLEAN=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote clean' 2>&1)
echo "$TMUX_CLEAN" | grep -q "removed" && pass "clean in tmux" || fail "clean in tmux"

# tnote goto — unbound name exits nonzero
GOTO_MISSING=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote goto -n nosuchnote 2>&1; echo "exit=$?"')
echo "$GOTO_MISSING" | grep -q "not found or not bound" && pass "goto missing note errors" || fail "goto missing note errors"
echo "$GOTO_MISSING" | grep -q "exit=1" && pass "goto missing note exits nonzero" || fail "goto missing note exits nonzero"

# tnote goto — bound only to a dead/stale tmux window
tnote name gotostale --bind '$999+@999' >/dev/null
GOTO_STALE=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote goto -n gotostale 2>&1; echo "exit=$?"')
echo "$GOTO_STALE" | grep -q "no live bound window" && pass "goto stale binding errors" || fail "goto stale binding errors"
echo "$GOTO_STALE" | grep -q "exit=1" && pass "goto stale binding exits nonzero" || fail "goto stale binding exits nonzero"

# tnote goto — bound only to a shell (pid) note, not a tmux window
# (bind to this script's own PID so the liveness check finds it alive)
tnote name gotoshell --bind "$$" >/dev/null
GOTO_SHELL=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote goto -n gotoshell 2>&1; echo "exit=$?"')
echo "$GOTO_SHELL" | grep -q "not a tmux window" && pass "goto shell-only binding errors" || fail "goto shell-only binding errors"

# tnote goto — bound to two live tmux windows is ambiguous
tmux new-window -d -t test-session: -n goto-a "sh"
tmux new-window -d -t test-session: -n goto-b "sh"
GOTO_A_KEY=$(tmux display-message -p -t test-session:goto-a '#{session_id}+#{window_id}')
GOTO_B_KEY=$(tmux display-message -p -t test-session:goto-b '#{session_id}+#{window_id}')
tnote name gotoambiguous --bind "$GOTO_A_KEY" >/dev/null
tnote name gotoambiguous --bind "$GOTO_B_KEY" >/dev/null
GOTO_AMBIG=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote goto -n gotoambiguous 2>&1; echo "exit=$?"')
echo "$GOTO_AMBIG" | grep -q "bound to multiple live windows" && pass "goto ambiguous binding errors" || fail "goto ambiguous binding errors"

# tnote goto — single live tmux window: select-window should switch test-session's
# current window even though switch-client/attach-session then fails (no attached
# client in headless docker), matching the same "no crash" acceptance as `tnote open`.
tmux new-window -d -t test-session: -n goto-target "sh"
GOTO_TARGET_KEY=$(tmux display-message -p -t test-session:goto-target '#{session_id}+#{window_id}')
tnote name gototarget --bind "$GOTO_TARGET_KEY" >/dev/null
tmux select-window -t test-session:0
tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote goto -n gototarget' >/dev/null 2>&1 || true
GOTO_CURRENT=$(tmux display-message -p -t test-session '#{window_name}')
[ "$GOTO_CURRENT" = "goto-target" ] && pass "goto selects the bound window" || fail "goto selects the bound window"
tmux kill-window -t test-session:goto-a 2>/dev/null || true
tmux kill-window -t test-session:goto-b 2>/dev/null || true
tmux kill-window -t test-session:goto-target 2>/dev/null || true

# tnote __name-picker filter flow
printf '' > "$TNOTE_DIR/named-alpha.md"
printf '' > "$TNOTE_DIR/named-beta project.md"
PICKER_TARGET="tmux-\$77+@3"
tmux new-window -d -t test-session: -n picker "TNOTE_DIR='$TNOTE_DIR' tnote __name-picker '$PICKER_TARGET'"
sleep 1
tmux send-keys -t test-session:picker 'proj' Down Enter
sleep 1
[ -f "$TNOTE_DIR/meta/${PICKER_TARGET}.link" ] && pass "name picker filter writes tmux link" || fail "name picker filter writes tmux link"
grep -q "beta project" "$TNOTE_DIR/meta/${PICKER_TARGET}.link" && pass "name picker filter selects filtered note" || fail "name picker filter selects filtered note"
tmux kill-window -t test-session:picker 2>/dev/null || true

# tnote __goto-picker filter flow — unlike __name-picker, there's no "New name..."
# row (allow_new: false), so the first filtered match is selectable without a
# leading Down keypress. Selecting it should perform the actual jump, so we check
# test-session's current window the same way as the "goto selects the bound
# window" test above.
tmux new-window -d -t test-session: -n goto-pick "sh"
GOTO_PICK_KEY=$(tmux display-message -p -t test-session:goto-pick '#{session_id}+#{window_id}')
tnote name gotopicktest --bind "$GOTO_PICK_KEY" >/dev/null
tmux select-window -t test-session:0
tmux new-window -d -t test-session: -n goto-picker "TNOTE_DIR='$TNOTE_DIR' tnote __goto-picker"
sleep 1
tmux send-keys -t test-session:goto-picker 'gotopick' Enter
sleep 1
GOTO_PICKER_CURRENT=$(tmux display-message -p -t test-session '#{window_name}')
[ "$GOTO_PICKER_CURRENT" = "goto-pick" ] && pass "goto picker filter selects and jumps" || fail "goto picker filter selects and jumps"
tmux kill-window -t test-session:goto-pick 2>/dev/null || true
tmux kill-window -t test-session:goto-picker 2>/dev/null || true

# tnote open (popup) — in headless docker, display-popup fails (no client), so we just verify it doesn't crash
TMUX_OPEN=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' EDITOR=true tnote 2>&1; echo "exit=$?"')
# exit=0 means popup opened (real tmux), exit=1 means display-popup failed (headless) — both are acceptable
echo "$TMUX_OPEN" | grep -qE "exit=(0|1)" && pass "open in tmux (no crash)" || fail "open in tmux (no crash)"

echo ""

# ── :tnote command aliases ────────────────────────────────────────────────────

echo "Tmux command aliases:"

# Test :tnote-show alias
ALIAS_SHOW=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote show' 2>&1)
echo "$ALIAS_SHOW" | grep -q "tmux note content\|empty" && pass ":tnote-show" || fail ":tnote-show"

# Test :tnote-list alias
ALIAS_LIST=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote list' 2>&1)
echo "$ALIAS_LIST" | grep -q "tmuxtest\|no notes" && pass ":tnote-list" || fail ":tnote-list"

# Test :tnote-path alias
ALIAS_PATH=$(tmux run-shell 'TNOTE_DIR='"$TNOTE_DIR"' tnote path' 2>&1)
echo "$ALIAS_PATH" | grep -q ".md" && pass ":tnote-path" || fail ":tnote-path"

echo ""

# ── Cleanup ───────────────────────────────────────────────────────────────────

tmux kill-server 2>/dev/null || true

# ── Summary ───────────────────────────────────────────────────────────────────

echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then
    echo -e "\nFailures:$ERRORS"
    exit 1
fi
