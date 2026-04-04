# tnote

Terminal Notepad. Each tmux window or shell session gets its own persistent markdown note. In tmux, notes open in a floating popup anchored to the top-right corner. Press the same key to close it.

## Why tnote?

Running more than a dozen Claude Code sessions in parallel made it easy to get a lot done, but constant context switching became a real problem. Returning to a session meant trying to remember what the next job was before getting back into flow.

tnote was built to solve that. It's lightweight and stays out of the way: a quick popup to check what you were doing, drop in a task list, jot down commands to run later, or record what you've already finished. A note pinned to your tmux window means your context lives exactly where you left it.

Since tnotes are just markdown files, my agents use tnote too. I tell it to log its progress in tnote or complete all the tasks I've listed in my tnote. It's as simple as that!

```
┌─────────────────────────────────────────────────┐
│  work: api-server                               │
│                  ╭ tmux tnote - work+0 ────────╮│
│  $ cargo run     │ ## TODO                     ││
│  Compiling...    │ - add auth header           ││
│                  │ - check rate limit          ││
│                  │                             ││
│                  ╰─────────────────────────────╯│
└─────────────────────────────────────────────────┘
```

## Requirements

- macOS or Linux (Unix-only)
- tmux 3.2+ (optional — tnote works without tmux using shell keybindings)

## Install

### From a release (macOS and Linux)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jykim16/tnote/releases/latest/download/tnote-installer.sh | sh
```

### Homebrew (macOS)

```sh
brew install jykim16/tap/tnote
```

### From source

```sh
cargo install --git https://github.com/jykim16/tnote
```

---

After installing, run:

```sh
tnote setup
```

`tnote setup` runs an interactive prompt to choose your editor, key binding, and popup dimensions. It installs:
- **tmux bindings** (if tmux is available): `prefix+t` keybinding and `:tnote` command aliases
- **shell keybinding**: `Ctrl+t` in your shell (zsh, bash, or fish), automatically disabled inside tmux to avoid conflicts

## Usage

```
tnote                       Open/toggle popup for the current window
tnote name [name]           Name or rebind this window's note (also renames the tmux window)
tnote show                  Print note contents inline
tnote list                  List all notes with line counts
tnote path                  Print the note file path
tnote clean [--dryrun]      Remove orphaned notes and popup sessions
tnote clean --named <name>  Remove a specific named note
tnote clean --all <scope>   Remove notes by category: unprefixed, named, tmux, all
tnote setup                 Configure and install keybindings
tnote uninstall             Remove tmux and shell keybindings
tnote help                  Show help
```

## Note types

**tmux** — one note per tmux window, keyed by stable session and window IDs. Unaffected by session or window renames. Cleaned by `tnote clean` once the window is closed.

**named** — a preserved note that persists even after closing a terminal session. Created with `tnote name <name>`. Multiple sessions can share a note by using the same name.

**shell** — one note per shell session (parent PID), used when running outside tmux. Cleaned by `tnote clean` once the shell process exits.

## How it works

**In tmux** — pressing `prefix+t` (default) runs `tnote`. If you're inside a tnote popup, it detaches the client (closing the popup). Otherwise it opens a `tmux display-popup` backed by a persistent tmux session named `tnote-popup-<stem>`. Reopening the same note reattaches to the existing session — editor state is preserved.

**Outside tmux** — pressing `Ctrl+t` (default) runs `tnote`, which opens the editor inline in the current terminal. The shell keybinding is automatically disabled inside tmux to avoid conflicts with the tmux binding.

**Window keys** — tmux notes use `#{session_id}+#{window_id}` (e.g. `$1+@3`). These IDs are stable across renames, so renaming a session or window never breaks the note association. Display labels (e.g. `work+0`) are resolved from the live tmux state.

**tmux command line** — you can also type `:tnote` in the tmux command prompt (press `:` first). Other commands: `:tnote-show`, `:tnote-list`, `:tnote-name`, `:tnote-path`, `:tnote-clean`, `:tnote-help`. Running `:tnote-name` opens a tmux-native menu of existing named notes plus a `New name...` prompt.

**Shell completions** — `tnote completions bash|zsh|fish` emits completions that suggest existing named notes for `tnote name` and named-note flags like `tnote show -n`.

## File layout

```
~/.tnote/
  tmux-$1+@3.md          note for tmux window @3 in session $1
  named-api-server.md    a named note
  shell-12345.md         shell note for PID 12345
  meta/
    tmux-$1+@3.link      contains "api-server" — links window to named note
    tmux.conf            tmux key binding (sourced by ~/.tmux.conf)
    config               editor, key, width, height settings
```

Notes are plain markdown files. You can read, edit, grep, or back them up with any standard tool.

## Configuration

Settings are read from `~/.tnote/meta/config` (written by `tnote setup`), with environment variables taking precedence.

| Variable      | Default    | Description                                          |
|---------------|------------|------------------------------------------------------|
| `TNOTE_DIR`   | `~/.tnote` | Directory where notes are stored                     |
| `EDITOR`      | `vim`      | Editor to open inside the popup                      |
| `TNOTE_KEY`   | `t`        | Key binding (tmux: prefix+t, shell: Ctrl+t)          |
| `TNOTE_WIDTH` | `62`       | Popup width in columns                               |
| `TNOTE_HEIGHT`| `22`       | Popup height in lines                                |

The config file can also be edited directly:

```
# ~/.tnote/meta/config
editor=nvim
key=t
width=80
height=24
```
