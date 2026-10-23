#!/usr/bin/env bash
# Real headless X11 and Wayland clipboard servers; no clipboard mocks.
set -euo pipefail
binary="$(realpath "${1:-target/release/cpctx}")"
deadline="$(command -v timeout)"
temporary="$(mktemp -d)"
x11_pid=''
wayland_pid=''
cleanup() {
  status=$?
  if [ "$status" -ne 0 ]; then
    cat "$temporary/wayland.log" "$temporary/error" 2>/dev/null || true
  fi
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
printf 'Checking X11/xclip export\n'
"$deadline" --kill-after=5 15 "$binary" -c "$temporary/project" > "$temporary/stdout" 2> "$temporary/error"
test ! -s "$temporary/stdout"
"$deadline" --kill-after=5 15 xclip -selection clipboard -o > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS real X11 clipboard persists after cpctx exit\n'
# Keep only xsel discoverable to exercise the real fallback backend.
mkdir "$temporary/xsel-only"
ln -s "$(command -v xsel)" "$temporary/xsel-only/xsel"
printf 'Checking X11/xsel export\n'
PATH="$temporary/xsel-only" "$deadline" --kill-after=5 15 "$binary" -c "$temporary/project" > "$temporary/stdout" 2> "$temporary/error"
test ! -s "$temporary/stdout"
"$deadline" --kill-after=5 15 xsel --clipboard --output > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS real xsel fallback clipboard\n'
export XDG_RUNTIME_DIR="$temporary/runtime"
printf 'seat seat0 fallback true\noutput HEADLESS-1 resolution 800x600\n' > "$temporary/sway.config"
WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 \
  sway --config "$temporary/sway.config" \
  > "$temporary/wayland.log" 2>&1 &
wayland_pid=$!
for _ in {1..100}; do
  for socket in "$XDG_RUNTIME_DIR"/wayland-*; do
    if [ -S "$socket" ]; then export WAYLAND_DISPLAY="${socket##*/}"; break; fi
  done
  if [ -n "${WAYLAND_DISPLAY:-}" ]; then break; fi
  sleep 0.1
done
test -n "${WAYLAND_DISPLAY:-}"
test -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY"
printf 'Checking Wayland export\n'
"$deadline" --kill-after=5 15 "$binary" -c "$temporary/project" > "$temporary/stdout" 2> "$temporary/error"
test ! -s "$temporary/stdout"
"$deadline" --kill-after=5 15 wl-paste --no-newline > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS real Wayland clipboard persists after cpctx exit\n'
# A validation failure must leave the previous clipboard untouched.
if "$binary" -c "$temporary/project" --max-bytes 1 > "$temporary/stdout" 2>/dev/null; then
  echo 'Clipboard budget failure unexpectedly succeeded' >&2
  exit 1
fi
test ! -s "$temporary/stdout"
"$deadline" --kill-after=5 15 wl-paste --no-newline > "$temporary/actual"
cmp "$temporary/expected" "$temporary/actual"
printf 'PASS clipboard unchanged after validation failure\n'
