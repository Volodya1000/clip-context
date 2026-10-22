#!/usr/bin/env bash
# Gate publishing on verified packages and a tag reachable from main.
set -euo pipefail
[[ "$GITHUB_REF_NAME" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]
git fetch origin main
git merge-base --is-ancestor "$GITHUB_SHA" origin/main
cat dist/release-linux/SHA256SUMS.linux.txt \
    dist/release-windows/SHA256SUMS.windows.txt > dist/SHA256SUMS.txt
cp dist/release-linux/*.{deb,gz} dist/
cp dist/release-windows/*.zip dist/
(cd dist && sha256sum -c SHA256SUMS.txt)
test "$(find dist -maxdepth 1 -type f \( -name '*.zip' -o -name '*.tar.gz' -o -name '*.deb' \) | wc -l)" -eq 3

set -euo pipefail
gh release create "$GITHUB_REF_NAME" dist/*.zip dist/*.tar.gz \
    dist/*.deb dist/SHA256SUMS.txt \
    --verify-tag --generate-notes --title "$GITHUB_REF_NAME"
