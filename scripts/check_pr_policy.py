"""Validate pull-request titles, commit subjects, and changelog changes."""

from __future__ import annotations

import os
import re
import subprocess
import sys

SUBJECT = re.compile(
    r"(?:feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)"
    r"(?:\([a-z][a-z0-9-]*\))?!?: [^\s].*"
)
HASH = re.compile(r"[0-9a-f]{40}")


def valid_subject(subject: str, limit: int) -> bool:
    """Check the observable Conventional Commits subject contract."""
    return len(subject) <= limit and SUBJECT.fullmatch(subject) is not None


def git(*arguments: str) -> list[str]:
    """Call git without a shell and return non-empty output lines."""
    process = subprocess.run(
        ["git", *arguments],
        check=True,
        text=True,
        capture_output=True,
    )
    return [line for line in process.stdout.splitlines() if line]


def check_pr(title: str, base: str, head: str) -> list[str]:
    """Return policy violations without altering the repository."""
    errors: list[str] = []

    if not valid_subject(title, 100):
        errors.append("PR title must follow Conventional Commits (max 100 chars).")

    if HASH.fullmatch(base) is None or HASH.fullmatch(head) is None:
        return [*errors, "BASE_SHA and HEAD_SHA must be full Git commit hashes."]

    commit_subjects = git("log", "--no-merges", "--format=%s", f"{base}..{head}")
    if not commit_subjects:
        errors.append("No non-merge commits found in the pull request.")
    for subject in commit_subjects:
        if not valid_subject(subject, 72):
            errors.append(f"Invalid commit subject: {subject}")

    changed_paths = git("diff", "--name-only", base, head)
    if "CHANGELOG.md" not in changed_paths:
        errors.append("Every PR must change CHANGELOG.md.")

    return errors


def main() -> int:
    """Validate PR data safely supplied through GitHub Actions environment."""
    title = os.environ.get("PR_TITLE", "")
    base = os.environ.get("BASE_SHA", "")
    head = os.environ.get("HEAD_SHA", "")
    errors = check_pr(title, base, head)

    for error in errors:
        print(f"::error::{error}", file=sys.stderr)

    if errors:
        return 1
    print("PR title, commit subjects and changelog: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
