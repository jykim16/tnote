# tnote — development notes

## Validation

Always run `cargo install --path .` before validating any behavior. Do not test with `cargo run` or assume `cargo build` is sufficient.

`tnote` in PATH resolves to the Homebrew release binary. To test dev changes, always use the full path: `/Users/jonkimj/.cargo/bin/tnote`.

After making any code changes, always run `cargo install --path .` so the user can immediately test the changes locally with the dev binary.

## Logging

Log messages should be prefixed with `tnote <command>:` where `<command>` is the subcommand being run. For example:
- `tnote name: window note named 'foo'`
- `tnote clean: removed note tmux-$1+@3`
- `tnote setup: wrote ~/.tnote/meta/tmux.conf`

Use `tnote:` as the prefix only for startup-level errors (before any subcommand runs).

## Integration Tests

Any feature that is validated end-to-end (e.g. manually tested via CLI commands) should have a corresponding test in `tests/integration/run.sh`. Integration tests run in Docker with a real tmux server — build and run with:

```
docker build -f tests/integration/Dockerfile -t tnote-test . && docker run --rm tnote-test
```
