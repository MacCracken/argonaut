#!/usr/bin/env bash
# ci-install-cyrius.sh — install the pinned Cyrius toolchain from its GitHub
# release, verified, into the version-pinned layout cyrius resolves:
#   ~/.cyrius/versions/<V>/{bin,lib}, with ~/.cyrius/{bin,lib} symlinked to it.
# Used by .github/workflows/ci.yml and release.yml; runs the same locally.
#
# The tarball matches the host: x86_64-linux on an x86_64 runner, aarch64-linux
# (a native cycc and cyrius CLI) on an arm64 runner.
#
# Verification, fail-closed, before anything is unpacked:
#   1. SHA256SUMS.sig must be a valid Ed25519 signature over SHA256SUMS under
#      the release key pinned below, not the key the release ships (anyone able
#      to replace the tarball could replace that too). Checked with stock
#      `openssl pkeyutl -rawin` (OpenSSL 3.0+).
#   2. The tarball's SHA-256 must match its one line in that SHA256SUMS.
#
# Usage: scripts/ci-install-cyrius.sh [--arch x86_64|aarch64] [VERSION]
#   VERSION defaults to $CYRIUS_VERSION, else the `cyrius = "X.Y.Z"` pin in
#   cyrius.cyml (the only pin source; never hardcode it in YAML).
#   --arch overrides `uname -m`.
# Environment: CYRIUS_HOME (default ~/.cyrius). Under GitHub Actions the bin
# dir is appended to $GITHUB_PATH and CYRIUS_HOME to $GITHUB_ENV.

set -euo pipefail

ARCH="$(uname -m)"
if [ "${1:-}" = "--arch" ]; then ARCH="${2:-}"; shift 2; fi
case "$ARCH" in
    x86_64|aarch64) ;;
    arm64) ARCH=aarch64 ;;
    *) echo "ci-install-cyrius: no cyrius linux tarball for arch '$ARCH'" >&2; exit 2 ;;
esac

# The cyrius release-signing key (Ed25519, raw 32 bytes as hex). It is the
# release asset cyrius-release.ed25519.pub, keys/cyrius-release.ed25519.pub in
# the cyrius repo, and CYRIUS_RELEASE_PUBKEY in cyrius's scripts/install.sh.
# If cyrius rotates its key, this check fails closed: confirm the new key out
# of band in those places, then update it here.
CYRIUS_RELEASE_PUBKEY="adbde6b11ccf8d86dc760387fa7f4dfbe3942fa318e459fb6e62d1536e254008"
REPO="MacCracken/cyrius"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-${CYRIUS_VERSION:-$(grep -oP '(?<=^cyrius = ")[^"]+' "$REPO_ROOT/cyrius.cyml")}}"
echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' || {
    echo "ci-install-cyrius: '$VERSION' is not a X.Y.Z cyrius version" >&2; exit 2; }
CYRIUS_HOME="${CYRIUS_HOME:-$HOME/.cyrius}"
STAGE="cyrius-$VERSION-$ARCH-linux"
TARBALL="$STAGE.tar.gz"
URL="https://github.com/$REPO/releases/download/$VERSION"

# No xxd: the Ubuntu 26.04 runner images do not ship it. The key bytes come
# from bash's printf (step 1).
for tool in curl openssl sha256sum tar sed awk; do
    command -v "$tool" > /dev/null || { echo "ci-install-cyrius: needs $tool" >&2; exit 2; }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

echo "Installing Cyrius $VERSION ($ARCH)"
for f in "$TARBALL" SHA256SUMS SHA256SUMS.sig; do
    curl -fsSL --retry 3 -o "$f" "$URL/$f" || {
        echo "ci-install-cyrius: could not fetch $URL/$f" >&2; exit 1; }
done

# 1. The signature over SHA256SUMS, against the pinned key. The key is wrapped
#    in the fixed SubjectPublicKeyInfo DER prefix for Ed25519 (RFC 8410), and
#    the hex turned into bytes by printf's \xHH escapes: 12 + 32 = 44 bytes.
[[ "$CYRIUS_RELEASE_PUBKEY" =~ ^[0-9a-f]{64}$ ]] || {
    echo "ci-install-cyrius: the pinned key is not 64 hex digits" >&2; exit 2; }
printf '%b' "$(printf '%s' "302a300506032b6570032100$CYRIUS_RELEASE_PUBKEY" | sed 's/../\\x&/g')" > release.pub.der
[ "$(wc -c < release.pub.der)" -eq 44 ] || {
    echo "ci-install-cyrius: the public key DER is not 44 bytes" >&2; exit 1; }
openssl pkey -pubin -inform DER -in release.pub.der -out release.pub.pem
if ! openssl pkeyutl -verify -pubin -inkey release.pub.pem -rawin \
        -in SHA256SUMS -sigfile SHA256SUMS.sig > /dev/null 2>&1; then
    echo "ci-install-cyrius: SHA256SUMS.sig does not verify against the pinned cyrius release key." >&2
    echo "  Either the release was tampered with, or cyrius rotated its key (see this script's header)." >&2
    exit 1
fi
echo "signature: SHA256SUMS verified against the pinned release key"

# 2. The tarball against its line in the now-trusted SHA256SUMS. Exactly one
#    line must name it; `sha256sum -c` then checks the bytes.
lines="$(awk -v f="$TARBALL" '$2 == f || $2 == "*" f' SHA256SUMS)"
if [ "$(printf '%s\n' "$lines" | grep -c .)" -ne 1 ]; then
    echo "ci-install-cyrius: SHA256SUMS has no single entry for $TARBALL" >&2; exit 1
fi
printf '%s\n' "$lines" | sha256sum -c --strict - || {
    echo "ci-install-cyrius: $TARBALL does not match the signed SHA256SUMS" >&2; exit 1; }

# Unpack and lay out the version-pinned toolchain.
tar xzf "$TARBALL"
SRC="$WORK/$STAGE"
[ -d "$SRC/bin" ] && [ -d "$SRC/lib" ] || {
    echo "ci-install-cyrius: $TARBALL has no bin/ and lib/" >&2; exit 1; }
VERSIONED="$CYRIUS_HOME/versions/$VERSION"
mkdir -p "$VERSIONED/bin" "$VERSIONED/lib"
cp "$SRC/bin/"* "$VERSIONED/bin/"
cp -r "$SRC/lib/"* "$VERSIONED/lib/"
chmod +x "$VERSIONED/bin/"*
ln -sfn "$VERSIONED/bin" "$CYRIUS_HOME/bin"
ln -sfn "$VERSIONED/lib" "$CYRIUS_HOME/lib"
echo "$VERSION" > "$CYRIUS_HOME/current"

if [ -n "${GITHUB_PATH:-}" ]; then echo "$CYRIUS_HOME/bin" >> "$GITHUB_PATH"; fi
if [ -n "${GITHUB_ENV:-}" ]; then echo "CYRIUS_HOME=$CYRIUS_HOME" >> "$GITHUB_ENV"; fi
echo "installed: $VERSIONED ($(ls "$VERSIONED/bin" | wc -l) tools)"
