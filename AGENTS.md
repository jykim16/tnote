# tnote — development notes

## Validation

Always run `cargo install --path .` before validating any behavior. Do not test with `cargo run` or assume `cargo build` is sufficient — the installed binary in `~/.cargo/bin/tnote` is what matters.

After making any code changes, always run `cargo install --path .` so the user can immediately test the changes locally with the installed binary.

## Logging

Log messages should be prefixed with `tnote <command>:` where `<command>` is the subcommand being run. For example:
- `tnote name: window note named 'foo'`
- `tnote clean: removed note tmux-$1+@3`
- `tnote setup: wrote ~/.tnote/meta/tmux.conf`

Use `tnote:` as the prefix only for startup-level errors (before any subcommand runs).

