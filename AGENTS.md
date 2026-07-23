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

## Releasing

Follow `RELEASE.md` whenever the work involves any of:
- Bumping the version in `Cargo.toml`.
- Creating or pushing a `vX.Y.Z` git tag.
- The user asking to "release", "publish", "cut a release", "ship", or similar.

Do not skip straight to `git tag` / `git push --tags` just because a version bump was requested — pushing a tag triggers the real `cargo-dist` CI workflow, which builds binaries for every target and publishes a public GitHub Release with an installer script that users may `curl | sh`. Treat it as a production deploy, not a routine commit.

In particular, always work through RELEASE.md's "Before you begin" checklist yourself before tagging - don't assume `main`'s CI is green or that `cargo test` passing means clippy also passes. They are checked separately in CI and can diverge (e.g. a clippy-only failure can sit unnoticed on `main` for multiple commits since nothing local re-runs it until release time).

## Skills

The `tnote-agent` and `tnote-manager` skills in `skills/` depend on each other's note format. If you change the tnote note structure (sections, `---` separators, header fields) in the agent skill, update the manager skill to match — especially §5 (write an agent note) and §7 (spawn), which parse agent notes with `awk`.
