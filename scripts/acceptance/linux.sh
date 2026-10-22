#!/usr/bin/env bash
# Exercise the built CLI without relying on Rust test doubles.
set -euo pipefail
binary="$(realpath "${1:-target/release/cpctx}")"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
project="$temporary/project with spaces"
mkdir -p "$project/src" "$project/private" "$project/node_modules"
printf 'Unicode: Привет 🌍\n' > "$project/src/пример.rs"
printf 'private/\n' > "$project/.gitignore"
printf '[profiles.rust]\nextensions = ["rs"]\n' > "$project/.cpctx.toml"
printf 'not exported\n' > "$project/private/hidden.rs"
printf 'not exported\n' > "$project/node_modules/generated.js"
"$binary" -s "$project" --profile rust > "$temporary/full" 2> "$temporary/stats"
"$binary" -s "$project" --profile rust --format compact > "$temporary/compact" 2>/dev/null
"$binary" -t "$project" > "$temporary/tree" 2>/dev/null
grep -Fq 'Привет 🌍' "$temporary/full"
grep -Fq 'пример.rs' "$temporary/tree"
! grep -Fq 'not exported' "$temporary/full"
test "$(wc -c < "$temporary/compact")" -lt "$(wc -c < "$temporary/full")"
grep -Fq "($(wc -c < "$temporary/full") bytes)" "$temporary/stats"
if "$binary" -s "$project" --max-bytes 1 > "$temporary/rejected" 2>/dev/null; then
  echo 'Output limit unexpectedly succeeded' >&2
  exit 1
fi
test ! -s "$temporary/rejected"
if env -u DISPLAY -u WAYLAND_DISPLAY "$binary" -c "$project" > "$temporary/rejected" 2>/dev/null; then
  echo 'Headless clipboard unexpectedly succeeded' >&2
  exit 1
fi
test ! -s "$temporary/rejected"
printf 'PASS Linux stdout/tree/Unicode/profiles/ignores/limits/headless clipboard\n'
