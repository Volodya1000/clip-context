# Changelog

All notable changes to this project are documented here. Every pull request must update this file.

## [Unreleased]

### Added

- Add metadata-only --explain/--dry-run modes with source sizes, largest files, exclusion reasons and budget guidance; bound diagnostic traversal/output independently and reject SSH rather than downloading source for a dry run.

- Add project-relative include/exclude globs in CLI and profiles, extensionless file selection and explicit safe hidden-configuration prefixes; retain Git ignores, credential checks and protected-path exclusions.

- Add live Linux X11/Wayland, Windows PowerShell clipboard and real SSH acceptance scripts and CI evidence artifacts, with an explicit table of completed and pending release gates.
- Cover every credential-prefix length boundary, embedded private-key markers, typed credential literals and safe environment references; document the detector threat model and an ordinary-export performance measurement.


- Document the exact main-branch ruleset, required checks, no-bypass policy and a reproducible negative enforcement test; record that administration access is still required to apply it.

- Prefilter known generated and hidden cache directories on SSH servers using supported tar exclude patterns, with automatic fallback for POSIX tar implementations without exclusions.

- Build and smoke-test Windows/Linux release archives and an installable Debian package; verify SHA-256 assets, allow opt-in Windows user PATH installation, and publish only on verified version tags.

- Expand `cpctx --help`/`-h` into a structured, English CLI reference with examples, defaults, safety and SSH notes; add precise invalid-option diagnostics and integration coverage.

- Print exact rendered UTF-8 context bytes and fast approximate token counts to stderr after successful stdout/tree/clipboard output.
- Add `--format compact` to remove the duplicate file tree and shorten Markdown file headers while preserving source files; default full format remains unchanged.

- Support SSH collection from POSIX servers without installing cpctx remotely, using bounded tar archives and local filtering, safety checks and clipboard output.

- Add `cpctx -c PATH` / `--clipboard PATH` to copy a validated Markdown snapshot using Windows PowerShell or native Linux clipboard utilities, without printing source text.

- Block snapshots containing recognized credentials, and enforce bounded source-byte and estimated-token budgets before output.

- Add strict named TOML selection profiles with `--profile` / `--config`, reusable validation, and CLI override precedence.

- Respect project-root and nested `.gitignore` patterns, including negations, without reading ignore files outside the chosen root.

- Allow repeated `--ext EXT` and `--exclude-dir NAME` flags for project selection, with library support and integration tests.

- First deterministic UTF-8 source-file collector with a filtered directory tree and Markdown formatter.
- `--stdout PATH` and `--tree PATH` commands, including diagnostics for invalid project paths.
- Integration tests for file selection, generated-directory exclusion, Markdown formatting, CLI outputs and errors.

### Fixed

- Detect private-key headers embedded in quoted source and credentials in typed Rust/Python assignments; avoid treating typed assignments of identifiers as literal passwords.

### Changed

- Extract release build, packaging, smoke checks and publication into focused Windows/Linux scripts, keeping GitHub Actions as a small, guarded orchestration workflow.

- Replace outdated README with concise English and Russian user guides, language links, release installation links, and current CLI examples.

- Enforce `--max-bytes` and `--max-estimated-tokens` on fully rendered context, including Markdown, paths and optional tree; tree-only output no longer reads source content.

- Make `cpctx --help`/`-h` concise and introduce `--help-full` for the existing detailed reference; cover both with integration tests.

- Update GitHub Actions security and commit-checking actions: Zizmor v0.7.1, Cocogitto action v4.2.0 and Gitleaks action v3.0.0.

- Increase the mandatory CI line coverage threshold from 75% to 90%.
- Expose the CLI as `cpctx`, retaining the `clip-context` package name and `-t` / `-s` short options.
- Split project collection, selection, tree rendering, and Markdown output into focused Rust modules with unit tests.

## [0.1.0]

### Added

- Bootstrap Rust binary with help, version, and integration tests.
- Establish strict cross-platform CI, dependency auditing, project hygiene, and contribution policies.
- Replace custom Python commit and PR validation with standard Cocogitto checks.
