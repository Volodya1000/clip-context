# Contributing

Rust 1.85+ is required. Format, lint and test before opening a PR:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo deny check
```

CI additionally checks coverage, minimum supported Rust version, dependency security, workflows, files and metadata.

## Commit and PR policy

Commit subjects and pull-request titles use Conventional Commits, e.g. `feat(cli): support directory arguments` or `ci: add linting`. Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Commit subjects must be at most 72 characters; PR titles at most 100 characters. Merge commits are excluded from subject validation.

**Every PR must change `CHANGELOG.md`.** CI enforces this, even for documentation and dependency changes. Document changes in `[Unreleased]`.

No new lint suppressions should be introduced to make CI green. Fix underlying issues. Changes to lint policy or the security configuration require clear justification and review.

## Security

Never commit secrets, credentials, private keys or actual project code from third parties. Use minimal GitHub Actions permissions. Treat code packaged for LLMs as potentially confidential.
