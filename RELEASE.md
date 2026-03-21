# Release process

## Before you begin

- All changes are on the main branch and CI is green.
- `cargo test` passes locally.
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
2. Run the shell installer on a clean machine (or a fresh shell session) and confirm `tnote --version` reports the new version.
3. Run `tnote setup` on the test machine to confirm the tmux integration still works end-to-end.

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
