# Main branch protection

The repository policy is stored in
[`scripts/maintenance/main-ruleset.json`](../scripts/maintenance/main-ruleset.json).
It requires pull requests and the exact current GitHub Actions check names,
including Windows/Linux release packaging. No user, administrator or bot has a
bypass; no review approval is required for this single-maintainer project.
Force pushes and deletion of `main` are forbidden. The branch must be up to date
before merging.

## Apply with repository administration permission

The connected integration returned HTTP 403 (`Resource not accessible by
integration`) on 2026-10-22. **The policy has not been applied.** A repository
permission reported as `admin: true` does not grant the integration the required
API permission.

An owner can open **Settings → Rules → Rulesets → New branch ruleset** and use
the checked-in JSON as the exact configuration. Select `main`, active
enforcement, an empty bypass list, required pull requests with zero approvals,
required up-to-date status checks, and block force pushes and deletion.
Select every check in the JSON, sourced from the GitHub Actions app (ID 15368).

Alternatively, with an independently authorized GitHub CLI administration token:

```sh
gh api --method POST repos/Volodya1000/clip-context/rulesets \
  --input scripts/maintenance/main-ruleset.json
gh api repos/Volodya1000/clip-context/rulesets \
  --jq '.[] | {id, name, enforcement}'
gh api repos/Volodya1000/clip-context/branches/main \
  --jq '{protected, protection}'
```

If `main-required-checks` already exists, inspect it and use `PUT` on
`repos/Volodya1000/clip-context/rulesets/ID` instead of creating a duplicate.
Retrieve that ruleset afterward to verify its rules and empty bypass list.
Do not record the token or its value in files, logs or issues.

## Verify enforcement

1. Open a temporary PR with a deliberate failing change, for example a
   documentation change without its required changelog entry. Wait for the
   `CHANGELOG.md is updated` check to fail.
2. Verify GitHub blocks merging, including for the repository owner and bots.
   Record the PR URL, failing check URL and merge rejection. Pending checks
   alone are not evidence of rejection for a failed check.
3. Add the changelog entry. Wait for **all** required checks on the final PR
   revision to succeed, including secret scanning and both package jobs.
4. Verify the PR is mergeable without a reviewer approval. Close a test-only PR;
   merge only if its final change is useful. Preserve the evidence in issue #23.

Dependency bots receive no exemption. A maintainer must add the changelog entry
to their PR branch before merging, and wait for the resulting revision's checks.
When renaming workflow jobs, update the checked-in policy and the live ruleset
together; otherwise a renamed required check will remain pending indefinitely.

Issue #23 remains open until live enforcement and the negative test are proven.
