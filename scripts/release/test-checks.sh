#!/usr/bin/env bash
# Negative cases exercise release gating without mutating remote status checks.
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
sha=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
jq --null-input --arg sha "$sha" --slurpfile required "$script_dir/required-checks.json" '
  {check_runs: [$required[0] | to_entries[] | {
    id: (.key + 1), name: .value, head_sha: $sha,
    app: {id: 15368}, check_suite: {id: 123}, status: "completed", conclusion: "success"
  }]}' > "$temporary/success.json"
evaluate() {
  jq --slurp --slurpfile required "$script_dir/required-checks.json" --arg sha "$sha" \
    --argjson suites "${2:-[123]}" \
    --from-file "$script_dir/checks.jq" "$1" |
    jq --exit-status 'length == 0' >/dev/null
}
evaluate "$temporary/success.json"
for mutation in \
  '.check_runs[0].conclusion = "failure"' \
  'del(.check_runs[0])' \
  '.check_runs[0].status = "in_progress" | .check_runs[0].conclusion = null' \
  '.check_runs[0].app.id = 1' \
  '.check_runs[0].head_sha = "another-commit"' \
  '.check_runs[0].check_suite.id = 999' \
  '.check_runs += [(.check_runs[0] | .id = 1000 | .conclusion = "failure")]'; do
  jq "$mutation" "$temporary/success.json" > "$temporary/rejected.json"
  if evaluate "$temporary/rejected.json"; then
    printf 'Release gate accepted invalid checks: %s\n' "$mutation" >&2
    exit 1
  fi
done
jq '.check_runs += [(.check_runs[0] | .id = 0 | .conclusion = "failure")]' \
  "$temporary/success.json" > "$temporary/old-failure.json"
evaluate "$temporary/old-failure.json"
jq '{check_runs: .check_runs[:5]}, {check_runs: .check_runs[5:]}' \
  "$temporary/success.json" > "$temporary/pages.json"
evaluate "$temporary/pages.json"
if evaluate "$temporary/success.json" '[]'; then
  printf 'Release gate accepted PR-only checks without an exact-SHA push suite\n' >&2
  exit 1
fi
printf 'PASS release gates: trusted exact SHA, latest run, all checks, failure/pending/missing rejection, pagination\n'
