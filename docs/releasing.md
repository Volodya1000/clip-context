# Releasing cpctx

Release publication is reserved for version tags after acceptance. PRs and manual
workflow dispatches build packages without publishing. No token, tag or release
is created by the commands in the validation section below.

## Validate a candidate

1. Complete [acceptance](acceptance.md), including the remaining manual platform
   rows. Keep the actual commit and evidence URLs; do not turn pending checks
   into passing claims.
2. Apply and negatively test [main protection](main-protection.md). The connected
   integration could not administer rulesets (HTTP 403); the JSON is a policy
   template until an authorized owner applies it.
3. Merge the candidate PRs in their documented dependency order. Wait for strict
   CI and Live acceptance on the final `main` commit. The acceptance workflow
   runs on pushes to `main` so release validation can use the exact commit.
4. Finalize the changelog: move candidate changes out of `[Unreleased]`, combine
   them with the existing initial `0.1.0` notes rather than adding a duplicate
   version section, and use the actual release date. Keep `[Unreleased]` for
   subsequent work. If this changes the commit, wait for its checks again.
5. Confirm `Cargo.toml`, `Cargo.lock` and the desired `vX.Y.Z` tag agree. Review
   both Windows/Linux package smoke tests and their SHA-256 artifacts.

Local policy tests:

```sh
bash -n scripts/release/*.sh
bash scripts/release/test-checks.sh
```

The test suite rejects missing/failed/pending checks, a newer failed run after an
older success, a different commit or app, and handles paginated check results.
It uses synthetic JSON, not fabricated successful checks on GitHub.

## Publish after the gates are complete

An authorized maintainer can tag the verified commit after the checklist above.
The tag must be exactly `vX.Y.Z`, match Cargo's version, and be reachable from
`main`. Pushing it invokes the release workflow. That workflow builds/tests both
platform packages and verifies checksums before calling the publish script.

Immediately before `gh release create`, `verify-checks.sh` queries GitHub for the
exact tagged SHA. Every name in `scripts/release/required-checks.json` must have
a **latest completed successful check from GitHub Actions app 15368** for that
SHA. A stale success cannot override a newer failure. The gate includes both
native acceptance jobs, security, coverage, compilers and package checks.
Only the publish job receives `contents: write` and `checks: read`.

Conventional Commits and changelog checks apply to PRs, not post-merge commits;
they belong in the branch ruleset and are not accepted as skipped substitutes
for the runtime checks. Keep both checked-in check lists in sync with workflow
job renames and with the live ruleset.

The gate fails closed if GitHub is unavailable or any required check is missing,
pending or failed. Finish/fix those checks and rerun the failed publish job;
never bypass the gate or push a different commit under an existing version tag.

After publication, download the assets, verify `SHA256SUMS.txt`, install using
the documented platform path, and check `cpctx --version` and a small export.
Record the published release and verification results in acceptance evidence.
