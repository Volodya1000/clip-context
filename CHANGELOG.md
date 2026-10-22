# Changelog

All notable changes to this project are documented here. Every pull request must update this file.

## [Unreleased]

### Added

- Respect nested `.gitignore` patterns and negated rules when collecting project context, even for directories without Git metadata; exclude rules outside the selected root.
- First deterministic UTF-8 source-file collector with a filtered directory tree and Markdown formatter.
- `--stdout PATH` and `--tree PATH` commands, including diagnostics for invalid project paths.
- Integration tests for file selection, generated-directory exclusion, Markdown formatting, CLI outputs and errors.

### Changed

- Increase the mandatory CI line coverage threshold from 75% to 90%.
- Expose the CLI as `cpctx`, retaining the `clip-context` package name and `-t` / `-s` short options.
- Split project collection, selection, tree rendering, and Markdown output into focused Rust modules with unit tests.

## [0.1.0]

### Added

- Bootstrap Rust binary with help, version, and integration tests.
- Establish strict cross-platform CI, dependency auditing, project hygiene, and contribution policies.
- Replace custom Python commit and PR validation with standard Cocogitto checks.
