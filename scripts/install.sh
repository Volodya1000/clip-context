#!/bin/sh
# Install a verified Linux x86_64 release to a directory already on PATH.
set -eu

REPO='Volodya1000/clip-context'
PREFIX="${PREFIX:-/usr/local}"
case "$PREFIX" in
    /*) ;;
    *) echo "PREFIX must be an absolute path" >&2; exit 2 ;;
esac
if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
    echo "Only Linux x86_64 is supported by this installer." >&2
    exit 2
fi
for command in curl tar sha256sum; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "Required command not found: $command" >&2
        exit 2
    }
done
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
        sed -n 's/.*"tag_name": *"\(v[0-9][0-9.]*\)".*/\1/p' | head -n 1)
fi
case "$VERSION" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "Supply a version such as v0.1.0" >&2; exit 2 ;;
esac
ASSET="cpctx-$VERSION-linux-x86_64.tar.gz"
WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT HUP INT TERM
BASE="https://github.com/$REPO/releases/download/$VERSION"
curl -fLsS "$BASE/$ASSET" -o "$WORKDIR/$ASSET"
curl -fLsS "$BASE/SHA256SUMS.txt" -o "$WORKDIR/SHA256SUMS.txt"
(
    cd "$WORKDIR"
    grep "  $ASSET\$" SHA256SUMS.txt > selected.sha256
    test "$(wc -l < selected.sha256)" -eq 1 || {
        echo "Missing or duplicated release checksum" >&2
        exit 1
    }
    sha256sum -c selected.sha256
    tar -xzf "$ASSET" cpctx LICENSE
    test "$(./cpctx --version)" = "clip-context ${VERSION#v}"
)
install -m 755 "$WORKDIR/cpctx" "$PREFIX/bin/cpctx"
printf 'Installed cpctx %s to %s/bin/cpctx\n' "${VERSION#v}" "$PREFIX"
