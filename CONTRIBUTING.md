# Contributing to tnote

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)
- [Docker](https://docs.docker.com/get-docker/) (for integration tests)
- tmux 3.2+ (optional, for manual testing)

## Setup

```sh
git clone https://github.com/jykim16/tnote.git
cd tnote
cargo build
```

## Development commands

```sh
cargo build              # Debug build
cargo build --release    # Release build
cargo run -- help        # Run locally
cargo install --path .   # Install to ~/.cargo/bin
```

## Testing

### Unit tests

```sh
cargo test
```

Runs 196 unit tests across all modules. No external dependencies required.

### Clippy

```sh
cargo clippy --all-targets -- -D warnings
```

CI enforces zero warnings. Run this before pushing.

### Integration tests

```sh
make integration-test
```

Builds a Docker container with real tmux and runs 30 end-to-end tests covering:
- All subcommands outside tmux
- `tnote setup` and `tnote uninstall`
- All subcommands inside tmux via `tmux run-shell`
- tmux command aliases (`:tnote-show`, `:tnote-list`, etc.)

### Coverage (optional)

```sh
cargo install cargo-tarpaulin
cargo tarpaulin --lib --skip-clean
```

## Project structure

```
src/
  main.rs      CLI entry point, subcommand dispatch
  config.rs    Config file parsing, env var handling
  editor.rs    Built-in terminal editor (PTY + ratatui)
  install.rs   tnote setup / uninstall (tmux + shell bindings)
  notes.rs     Note file management, cleanup, naming
  tmux.rs      tmux command wrappers
  lib.rs       Module exports

tests/
  cli.rs              CLI integration tests (fake tmux)
  unit_config.rs      Config unit tests
  unit_editor.rs      Editor key mapping tests
  unit_install.rs     Install/uninstall unit tests
  unit_notes.rs       Notes unit tests
  unit_tmux.rs        Tmux helper unit tests
  bin/tmux            Fake tmux binary for tests
  integration/
    Dockerfile        Docker image with real tmux
    run.sh            Integration test script
```

## Release process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit: `git commit -am "v0.X.0: description"`
4. Tag: `git tag v0.X.0`
5. Push: `git push origin main --tags`

The release workflow builds binaries for all targets, creates a GitHub release, and publishes the Homebrew formula.
