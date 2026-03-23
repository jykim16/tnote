# Changelog

## 0.3.0

- Shell keybindings: `tnote setup` now installs `Ctrl+t` in zsh, bash, and fish (auto-disabled inside tmux)
- Portable: replaced `which` with `command -v`, replaced `ps` with `libc` calls for PID checks
- Added `aarch64-unknown-linux-gnu` and `aarch64-unknown-linux-musl` build targets
- Installer now places binary in `~/.local/bin` instead of `~/.cargo/bin`
- First-run hint: shows "run `tnote setup`" when config doesn't exist
- Friendly error message when configured editor is not found
- Shell completions for zsh, bash, and fish via `tnote completions <shell>`
- Updated branding to "Terminal Notepad"
- Updated README with shell keybinding docs and platform requirements

## 0.2.0

- Fixed tmux command aliases to use single-quoted `run-shell 'tnote'`

## 0.1.0

- Initial release
- Per-tmux-window notes with floating popup
- Named notes shared across windows
- Shell notes (outside tmux) keyed by parent PID
- Built-in terminal editor with PTY support
- `tnote setup` for interactive configuration
- tmux command aliases (`:tnote`, `:tnote-show`, etc.)
