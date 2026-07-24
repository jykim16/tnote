#!/bin/bash
# Verifies that tnote's latest published GitHub Release is actually
# installable end-to-end via the real cargo-dist shell installer, on a
# clean machine with no pre-existing tnote or Rust toolchain.
#
# Run locally against a fresh container, e.g.:
#   docker run --rm -v "$PWD":/repo -w /repo ubuntu:22.04 bash -c \
#     "apt-get update && apt-get install -y curl ca-certificates xz-utils && bash tests/integration/verify-install.sh"
set -euo pipefail

REPO="jykim16/tnote"
INSTALLER_URL="https://github.com/${REPO}/releases/latest/download/tnote-installer.sh"

echo "=== tnote install verification ==="
echo ""

echo "Fetching latest release metadata..."
# Capture the full response before grepping - piping straight into `grep -m1`
# lets grep close the pipe after its first match, which sends SIGPIPE to
# curl; `pipefail` (set above) would then treat that normal termination as
# a pipeline failure.
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")
LATEST_TAG=$(echo "$RELEASE_JSON" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"v?([^"]+)".*/\1/')
if [ -z "$LATEST_TAG" ]; then
    echo "FAIL: could not determine latest release tag from GitHub API"
    exit 1
fi
echo "Latest published version: $LATEST_TAG"
echo ""

echo "Running installer script: $INSTALLER_URL"
curl --proto '=https' --tlsv1.2 -LsSf "$INSTALLER_URL" | sh
echo ""

export PATH="$HOME/.local/bin:$PATH"

echo "Checking tnote landed on PATH..."
if ! command -v tnote >/dev/null; then
    echo "FAIL: tnote not found on PATH (\$HOME/.local/bin) after install"
    exit 1
fi
echo "  ok: $(command -v tnote)"

echo "Checking tnote --version..."
ACTUAL_VERSION=$(tnote --version | awk '{print $2}')
if [ "$ACTUAL_VERSION" != "$LATEST_TAG" ]; then
    echo "FAIL: expected version $LATEST_TAG, got $ACTUAL_VERSION"
    exit 1
fi
echo "  ok: $ACTUAL_VERSION"

echo "Running smoke command 'tnote list'..."
if ! tnote list >/dev/null; then
    echo "FAIL: 'tnote list' exited non-zero"
    exit 1
fi
echo "  ok"

echo ""
echo "=== Install verification passed ($ACTUAL_VERSION) ==="
