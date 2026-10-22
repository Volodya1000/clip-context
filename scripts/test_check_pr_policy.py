"""Unit tests for repository contribution rules."""

import unittest

from check_pr_policy import valid_subject


class SubjectPolicyTests(unittest.TestCase):
    """Check semantic policy rather than GitHub-specific output wording."""

    def test_valid_conventional_subjects(self) -> None:
        for subject in (
            "ci: add strict checks",
            "feat(cli): support help",
            "fix!: change command behavior",
            "docs(readme): clarify installation",
        ):
            with self.subTest(subject=subject):
                self.assertTrue(valid_subject(subject, 72))

    def test_reject_invalid_subjects(self) -> None:
        for subject in (
            "Update README",
            "feature: add help",
            "feat: ",
            "feat(Scope): unsupported scope",
            "feat: " + "x" * 80,
        ):
            with self.subTest(subject=subject):
                self.assertFalse(valid_subject(subject, 72))

    def test_pr_title_accepts_longer_subject(self) -> None:
        subject = "ci: " + "x" * 75
        self.assertTrue(valid_subject(subject, 100))
        self.assertFalse(valid_subject(subject, 72))


if __name__ == "__main__":
    unittest.main()
