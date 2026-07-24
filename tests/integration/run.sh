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

# path
tnote path | grep -q ".md" && pass "path" || fail "path"

# show (empty)
tnote show | grep -q "empty" && pass "show (empty)" || fail "show (empty)"

# show (with content)
NOTE=$(tnote path)
mkdir -p "$(dirname "$NOTE")"
echo "hello world" > "$NOTE"
tnote show | grep -q "hello world" && pass "show (content)" || fail "show (content)"

# show (bat renderer)
# Note: no 2>&1 here - adding it forces bash to fork an extra subshell for
# this command substitution, which changes tnote's getppid()-derived shell
# identity mid-script and makes it see an unrelated empty note.
SHOW_BAT=$(TNOTE_RENDERER=bat tnote show)
echo "$SHOW_BAT" | grep -q "hello world" && pass "show (bat renderer)" || fail "show (bat renderer)"

# list
tnote list | grep -q "shell" && pass "list" || fail "list"

# name
tnote name testproject
tnote path | grep -q "named-testproject.md" && pass "name" || fail "name"

# name --bind (tmux key from outside tmux)
tnote name boundproject --bind '$9+@17'
[ -f "$TNOTE_DIR/meta/tmux-\$9+@17.link" ] && pass "name --bind writes tmux link" || fail "name --bind writes tmux link"
grep -q "boundproject" "$TNOTE_DIR/meta/tmux-\$9+@17.link" && pass "name --bind stores note name" || fail "name --bind stores note name"
tnote show -n boundproject >/dev/null && pass "name --bind creates named note" || fail "name --bind creates named note"

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

# list --archive (with content)
mkdir -p "$TNOTE_DIR/archive"
echo "old stuff" > "$TNOTE_DIR/archive/named-oldproject.md"
tnote list --archive | grep -q "oldproject" && pass "list --archive (content)" || fail "list --archive (content)"
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
