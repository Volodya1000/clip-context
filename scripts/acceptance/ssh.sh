#!/usr/bin/env bash
# Start a disposable real SSH server; generated keys never leave the temp dir.
set -euo pipefail
binary="$(realpath "${1:-target/release/cpctx}")"
temporary="$(mktemp -d)"
server_pid=''
sshd=(/usr/sbin/sshd)
if [ "$(id -u)" -ne 0 ] && [ "${CPCTX_SSHD_UNPRIVILEGED:-0}" != 1 ]; then
  sshd=(sudo -n /usr/sbin/sshd)
fi
cleanup() {
  if [ -n "$server_pid" ]; then
    if [ "${CPCTX_SSHD_UNPRIVILEGED:-0}" = 1 ] || [ "$(id -u)" -eq 0 ]; then
      kill "$server_pid" 2>/dev/null || true
    else
      sudo -n kill "$server_pid" 2>/dev/null || true
    fi
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$temporary"
}
trap cleanup EXIT
mkdir -p "$temporary/project with spaces/src" "$temporary/tools" "$temporary/posix"
project="$temporary/project with spaces"
printf 'remote Привет 🌍\n' > "$project/src/main.rs"
printf '[profiles.rust]\nextensions = ["rs"]\n' > "$project/.cpctx.toml"
printf 'private/\n' > "$project/.gitignore"
mkdir "$project/private"
printf 'not exported\n' > "$project/private/secret.rs"
ssh-keygen -q -t ed25519 -N '' -f "$temporary/client"
ssh-keygen -q -t ed25519 -N '' -f "$temporary/host"
port=$((22000 + RANDOM % 20000))
printf '[127.0.0.1]:%s %s\n' "$port" "$(cat "$temporary/host.pub")" > "$temporary/known_hosts"
cat > "$temporary/client.config" <<EOF
Host cpctx-acceptance
  HostName 127.0.0.1
  Port $port
  User $(id -un)
  IdentityFile "$temporary/client"
  IdentitiesOnly yes
  UserKnownHostsFile "$temporary/known_hosts"
  StrictHostKeyChecking yes
EOF
cat > "$temporary/tools/ssh" <<EOF
#!/bin/sh
exec /usr/bin/ssh -F '$temporary/client.config' "\$@"
EOF
chmod 755 "$temporary/tools/ssh"
cat > "$temporary/server.config" <<EOF
ListenAddress 127.0.0.1
Port $port
HostKey $temporary/host
PidFile $temporary/server.pid
AuthorizedKeysFile $temporary/client.pub
StrictModes no
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
AllowUsers $(id -un)
LogLevel ERROR
EOF
"${sshd[@]}" -D -e -f "$temporary/server.config" > "$temporary/sshd.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if /usr/bin/ssh -F "$temporary/client.config" -o BatchMode=yes cpctx-acceptance true 2>/dev/null; then break; fi
  if ! kill -0 "$server_pid" 2>/dev/null; then cat "$temporary/sshd.log" >&2; exit 1; fi
  sleep 0.1
done
"$binary" -s "$project" --profile rust --format compact > "$temporary/local" 2>/dev/null
PATH="$temporary/tools:$PATH" "$binary" -s "$project" --ssh cpctx-acceptance \
  --profile rust --format compact > "$temporary/remote" 2> "$temporary/error"
cmp "$temporary/local" "$temporary/remote"
printf 'PASS real SSH key authentication/host verification/Unicode/spaces/profiles/gitignore\n'
mkdir -p "$project/node_modules/package"
dd if=/dev/zero of="$project/node_modules/package/large.js" bs=1048576 count=17 status=none
PATH="$temporary/tools:$PATH" "$binary" -s "$project" --ssh cpctx-acceptance \
  --profile rust --format compact > "$temporary/remote" 2> "$temporary/error"
cmp "$temporary/local" "$temporary/remote"
printf 'PASS real SSH generated-directory prefilter with >16 MiB dependency\n'
if PATH="$temporary/tools:$PATH" "$binary" -s "$project/missing" --ssh cpctx-acceptance \
  > "$temporary/rejected" 2>/dev/null; then
  echo 'Missing remote directory unexpectedly succeeded' >&2
  exit 1
fi
test ! -s "$temporary/rejected"
printf 'PASS real SSH failure has empty stdout\n'
