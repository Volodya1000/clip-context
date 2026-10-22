#!/usr/bin/env bash
# Validate the release tag and compile the matching Linux binary.
set -euo pipefail
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
test -n "$version"
if [[ "$GITHUB_REF" == refs/tags/* ]]; then
  [[ "$GITHUB_REF_NAME" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]
  test "$GITHUB_REF_NAME" = "v$version"
fi
echo "PACKAGE_VERSION=$version" >> "$GITHUB_ENV"

cargo build --locked --release
