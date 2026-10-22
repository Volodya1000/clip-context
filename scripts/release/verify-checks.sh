#!/usr/bin/env bash
# Refuse publication without trusted successful checks on the exact tagged SHA.
set -euo pipefail
sha="${1:?release commit SHA required}"
[[ "$sha" =~ ^[0-9a-f]{40}$ ]]
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
blocked="$(
  gh api --paginate "repos/${GITHUB_REPOSITORY:?}/commits/$sha/check-runs?per_page=100" |
    jq --slurp --slurpfile required "$script_dir/required-checks.json" \
      --arg sha "$sha" --from-file "$script_dir/checks.jq"
)"
if ! jq --exit-status 'length == 0' <<< "$blocked" >/dev/null; then
  printf 'Refusing release: required checks for %s are missing, pending or failed.\n' "$sha" >&2
  jq --raw-output '.[] | "  \(.name): \(.status) / \(.conclusion)"' <<< "$blocked" >&2
  printf 'Wait for successful checks on this commit, then rerun the publish job.\n' >&2
  exit 1
fi
printf 'Verified all required checks for release commit %s\n' "$sha"
