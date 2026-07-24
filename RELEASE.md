# Release process

## Before you begin

- All changes are on the main branch. Check the *actual* latest CI run for `main` (e.g. `gh run list --branch main --limit 1`) - don't assume it's green just because nobody mentioned otherwise. A tag pushed on top of an already-red main will just fail the same way.
- `cargo test` passes locally.
- `cargo clippy -- -D warnings` passes locally - this is the exact command CI runs. `cargo test` passing is not enough; clippy warnings-as-errors is a separate, easy-to-forget check that CI enforces on every push.
- **Both are also verified on Linux, not just your local OS.** CI's `test` job is a matrix over `ubuntu-latest` and `macos-latest`; running the two checks above only on your own machine leaves the other platform unverified - and the Docker integration suite below does *not* cover this gap, since its Dockerfile only runs `cargo build --release`, never `cargo test`/`cargo clippy`. If developing on macOS, additionally run:
  ```sh
  docker run --rm -v "$PWD":/app -w /app rust:latest bash -c \
    "rustup component add clippy && cargo test && cargo clippy -- -D warnings"
  ```
  (swap the base image/flags to mirror whatever the other matrix leg actually is if `ci.yml` changes).
- The Docker integration suite passes locally: `docker build -f tests/integration/Dockerfile -t tnote-test . && docker run --rm tnote-test`.
- `cargo install --path .` produces a working binary (`tnote --version` prints the expected version).

## 1. Decide the new version

Follow [Semantic Versioning](https://semver.org/):

| Change type | Example | Version bump |
|---|---|---|
| Bug fix, docs, internals | Fix popup sizing | Patch — `0.1.0 → 0.1.1` |
| New command or flag, new config key | Add `tnote export` | Minor — `0.1.0 → 0.2.0` |
| Breaking CLI change, file-format change | Rename `--all` flag, change note key scheme | Major — `0.1.0 → 1.0.0` |

**When in doubt, bump minor.** It is always safe to do so.

## 2. Check backwards compatibility

Before bumping the version, answer these questions:

- **CLI flags** — are any existing flags renamed or removed? If yes, add a deprecation note in the help text for at least one minor release before removing.
- **Note file layout** — does `~/.tnote/` still work without migration? File names, `.link` format, and `meta/config` keys must remain readable by the old binary during any transition.
- **Config keys** — new keys should have defaults; never make an existing key required.
- **tmux.conf integration** — the line added to `~/.tmux.conf` by `tnote setup` should continue to work. If you change the popup invocation, ensure the old sourced `meta/tmux.conf` still behaves gracefully.

If any of these require a migration, document the migration steps in the changelog before releasing.

## 3. Bump the version

Edit `Cargo.toml`:

```toml
version = "X.Y.Z"
```

Commit:

```sh
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to X.Y.Z"
```

## 4. Tag the release

The tag must match the version in `Cargo.toml` exactly (with a `v` prefix):

```sh
git tag vX.Y.Z
git push origin main --tags
```

Pushing the tag triggers the cargo-dist CI workflow, which builds binaries for all targets and publishes a GitHub Release with a shell installer.

## 5. Verify the release

Once CI finishes:

1. Open the GitHub Releases page and confirm the release notes and attached binaries look correct.
2. The `Verify Release Install` workflow (`.github/workflows/verify-release.yml`) runs automatically when the release is published - it curls the real shell installer in a clean container and confirms `tnote --version` reports the new version and a basic smoke command works. Check that it's green (`gh run list --workflow=verify-release.yml --limit 1`) rather than re-doing this by hand; only fall back to running the installer yourself if that workflow is red or didn't fire.
3. Run `tnote setup` on a test machine to confirm the tmux integration still works end-to-end - this part is still manual, since it needs a live tmux session.

## 6. If something goes wrong

**Bad binary / wrong version printed** — delete the tag locally and remotely, fix the issue, and re-tag:

```sh
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
# fix, then re-tag and push
```

**Breaking change shipped by mistake** — issue a patch release immediately that either reverts the change or restores the old behaviour under the old flag/format. Do not leave users on a broken version.

## Release targets

cargo-dist builds for the following targets (see `Cargo.toml`):

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
