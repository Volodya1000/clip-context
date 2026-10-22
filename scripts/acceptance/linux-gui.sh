#!/usr/bin/env bash
# Real headless X11 and Wayland clipboard servers; no clipboard mocks.
set -euo pipefail
binary="$(realpath "${1:-target/release/cpctx}")"
temporary="$(mktemp -d)"
x11_pid=''
wayland_pid=''
cleanup() {
  if [ -n "$x11_pid" ]; then kill "$x11_pid" 2>/dev/null || true; fi
  if [ -n "$wayland_pid" ]; then kill "$wayland_pid" 2>/dev/null || true; fi
  rm -rf "$temporary"
}
trap cleanup EXIT
mkdir "$temporary/project" "$temporary/runtime"
chmod 700 "$temporary/runtime"
printf 'Clipboard: Привет 🌍\n' > "$temporary/project/main.rs"
"$binary" -s "$temporary/project" > "$temporary/expected" 2>/dev/null
Xvfb -displayfd 3 -screen 0 800x600x24 3> "$temporary/display" > "$temporary/x11.log" 2>&1 &
x11_pid=$!
for _ in {1..100}; do
  if [ -s "$temporary/display" ]; then break; fi
  sleep 0.1
done
test -s "$temporary/display"
export DISPLAY=":$(cat "$temporary/display")"
unset WAYLAND_DISPLAY
"$binary" -c "$temporary/project" > "$temporary/stdout" 2>/dev/null
test ! -s "$temporary/stdout"
xclip -selection clipboard -o > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS real X11 clipboard persists after cpctx exit\n'
export XDG_RUNTIME_DIR="$temporary/runtime"
export WAYLAND_DISPLAY=cpctx-acceptance
weston --backend=headless-backend.so --socket="$WAYLAND_DISPLAY" --idle-time=0 \
  > "$temporary/wayland.log" 2>&1 &
wayland_pid=$!
for _ in {1..100}; do
  if [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then break; fi
  sleep 0.1
done
test -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY"
"$binary" -c "$temporary/project" > "$temporary/stdout" 2>/dev/null
test ! -s "$temporary/stdout"
wl-paste --no-newline > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS real Wayland clipboard persists after cpctx exit\n'
# A validation failure must leave the previous clipboard untouched.
if "$binary" -c "$temporary/project" --max-bytes 1 > "$temporary/stdout" 2>/dev/null; then
  echo 'Clipboard budget failure unexpectedly succeeded' >&2
  exit 1
fi
test ! -s "$temporary/stdout"
wl-paste --no-newline > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS clipboard unchanged after validation failure\n'
