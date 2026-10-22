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

## Release workflow

`.github/workflows/release.yml` orchestrates three jobs: Linux, Windows and
tag-only publication. Build and packaging logic lives in `scripts/release/`
and is called by the corresponding job. The Windows and Linux installers
remain in `scripts/install.ps1` and `scripts/install.sh`.

The release workflow runs in dry-run mode for pull requests and manual
dispatches. A real GitHub Release requires a `vX.Y.Z` tag matching
`Cargo.toml`, a tag commit reachable from `main`, both tested packages and
matching SHA-256 checksums. Only the publish job receives `contents: write`.
Keep these gates intact when changing release scripts; never bypass them to
make an individual job green.

## Commit and PR policy

Commit subjects and pull-request titles use Conventional Commits, e.g. `feat(cli): support directory arguments` or `ci: add linting`. Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Commits and PR titles are both validated with Cocogitto during the regular `pull_request` workflow, without elevated token permissions. Git hooks can optionally be installed with Cocogitto. Do not add Python-only project dependencies for repository policy.

**Every PR must change `CHANGELOG.md`.** CI enforces this, even for documentation and dependency changes. Document changes in `[Unreleased]`.

No new lint suppressions should be introduced to make CI green. Fix underlying issues. Changes to lint policy or the security configuration require clear justification and review.

## Security

Never commit secrets, credentials, private keys or actual project code from third parties. Use minimal GitHub Actions permissions. Treat code packaged for LLMs as potentially confidential.
