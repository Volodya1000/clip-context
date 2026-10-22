#!/usr/bin/env bash
# Package and exercise both Linux install paths before upload.
set -euo pipefail
version="$PACKAGE_VERSION"
test "$(./target/release/cpctx --version)" = "clip-context $version"
mkdir -p dist archive package/DEBIAN package/usr/bin
install -m 755 target/release/cpctx archive/cpctx
install -m 755 target/release/cpctx package/usr/bin/cpctx
cp LICENSE archive/LICENSE
tar -czf "dist/cpctx-v$version-linux-x86_64.tar.gz" -C archive cpctx LICENSE
cat > package/DEBIAN/control <<EOF
Package: clip-context
Version: $version
Section: utils
Priority: optional
Architecture: amd64
Maintainer: clip-context contributors
Depends: libc6 (>= 2.35)
Description: Project source context packaging CLI
 Collect files and directory trees for sharing as Markdown.
EOF
dpkg-deb --root-owner-group --build package "dist/clip-context_${version}_amd64.deb"
mkdir -p "$RUNNER_TEMP/archive-test"
tar -xzf "dist/cpctx-v$version-linux-x86_64.tar.gz" -C "$RUNNER_TEMP/archive-test"
test "$("$RUNNER_TEMP/archive-test/cpctx" --version)" = "clip-context $version"
sudo dpkg -i "dist/clip-context_${version}_amd64.deb"
command -v cpctx
test "$(cpctx --version)" = "clip-context $version"
sudo dpkg -r clip-context
(cd dist && sha256sum ./*.tar.gz ./*.deb > SHA256SUMS.linux.txt)
PREFIX="$RUNNER_TEMP/prefix" \
  CPCTX_ARCHIVE="$PWD/dist/cpctx-v$version-linux-x86_64.tar.gz" \
  CPCTX_CHECKSUMS="$PWD/dist/SHA256SUMS.linux.txt" \
  sh scripts/install.sh "v$version"
test "$("$RUNNER_TEMP/prefix/bin/cpctx" --version)" = "clip-context $version"
(cd dist && sha256sum --check SHA256SUMS.linux.txt)
