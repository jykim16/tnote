# tnote

Per-tmux-window notepad. Each window gets its own persistent markdown note, opened in a floating popup anchored to the top-right corner. Press the same key to close it.

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

- tmux 3.2+
- Rust toolchain (to build)

## Install

```sh
git clone https://github.com/jkim/tnote
cd tnote
cargo install --path .
tnote setup
```

`tnote setup` runs an interactive prompt to choose your editor, key binding, and popup dimensions, then wires up the tmux binding. The binding persists across tmux restarts via a `source-file` line added to `~/.tmux.conf`.

## Usage

```
tnote                       Open/toggle popup for the current window
tnote name <name>           Name this window's note (also renames the tmux window)
tnote show                  Print note contents inline
tnote list                  List all notes with line counts
tnote path                  Print the note file path
tnote clean                 Remove orphaned notes and popup sessions
tnote clean --named <name>  Remove a specific named note
tnote clean --dryrun        Preview what would be removed
tnote clean --all <scope>   Remove notes by category: unprefixed, named, tmux, all
tnote setup                 Interactive config: editor, key binding, popup size
tnote uninstall             Remove the tmux key binding
tnote help                  Show help
```

## Note types

**tmux** — one note per tmux window, keyed by stable session and window IDs. Unaffected by session or window renames. Cleaned by `tnote clean` once the window is closed.

**named** — created with `tnote name <name>`. The current window gets a `.link` pointing to `named-<name>.md`. Two windows can share a note by using the same name. Never auto-cleaned.

**shell** — one note per shell session (parent PID), used when running outside tmux. Cleaned by `tnote clean` once the shell process exits.

## How it works

**Toggle** — pressing `prefix+t` (default) runs `tnote`. If you're inside a tnote popup, it detaches the client (closing the popup). Otherwise it opens a `tmux display-popup` backed by a persistent tmux session named `tnote-popup-<stem>`. Reopening the same note reattaches to the existing session — editor state is preserved.

**Window keys** — tmux notes use `#{session_id}+#{window_id}` (e.g. `$1+@3`). These IDs are stable across renames, so renaming a session or window never breaks the note association. Display labels (e.g. `work+0`) are resolved from the live tmux state.

**Outside tmux** — if `$TMUX` is not set, `tnote` opens the editor inline in the current terminal instead of spawning a popup.

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

| Variable      | Default    | Description                          |
|---------------|------------|--------------------------------------|
| `TNOTE_DIR`   | `~/.tnote` | Directory where notes are stored     |
| `EDITOR`      | `vim`      | Editor to open inside the popup      |
| `TNOTE_KEY`   | `t`        | Tmux key binding (used as prefix+t)  |
| `TNOTE_WIDTH` | `62`       | Popup width in columns               |
| `TNOTE_HEIGHT`| `22`       | Popup height in lines                |

The config file can also be edited directly:

```
# ~/.tnote/meta/config
editor=nvim
key=t
width=80
height=24
```
