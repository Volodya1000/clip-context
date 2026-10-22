# clip-context

A local-first Rust CLI for collecting source files and a filtered directory tree into a reproducible text snapshot.

**Current stage:** project snapshots and stdout. Native clipboard integration, custom file selection, configurable profiles, secret detection and SSH are **not yet implemented**.

## Example

Requires Rust 1.85 or newer.

```sh
cargo run --bin cpctx -- --help
cargo run --bin cpctx -- --tree .
cargo run --bin cpctx -- --stdout .
cargo test --locked --all-targets --all-features
```

Install the application with `cargo install --locked --path .`. The commands are `cpctx --tree .` and `cpctx --stdout .`. The shorthand forms `cpctx -t .` and `cpctx -s .` also work.

`--stdout` prints Markdown with selected source files and an ordered tree. `--tree` prints only the tree. Both commands require a directory; they do not modify files or clipboard contents. You can redirect stdout to a file if needed.

By default, only common source-file extensions are collected. Files are selected according to root and nested `.gitignore` files, including negation rules (`!`). Git ignore files are used even without a `.git` directory. Global and parent ignore files are not consulted; the hidden-file and generated-directory exclusions remain mandatory. Hidden files/directories, symbolic links, and known generated directories such as `.git`, `.venv`, `target`, `build` and `node_modules` are excluded. Failing to read any selected file aborts the operation instead of producing a partial result.

**Security limitation:** filenames and extensions alone do not detect secrets embedded inside source files. Until content scanning is implemented, do not pass untrusted or confidential repositories to `--stdout` or paste the output into an external service.

## Quality gates

The CI runs rustfmt, strict Clippy (`all`, `pedantic`, `nursery`, `cargo`), tests on Windows/Linux, rustdoc, MSRV compilation, **90% minimum line coverage**, cargo-deny, TOML/YAML hygiene, workflow linters, Conventional Commits verification of both commit history and PR titles through Cocogitto, changelog checks, and secret scanning.

Every PR must follow Conventional Commits naming and change `CHANGELOG.md`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

1. Deterministic project snapshots and plain output (implemented).
2. `.gitignore` support (this PR).
3. Configuration-driven file selection and profiles.
4. Content-based secret detection and safe output limits.
5. Native Windows/Linux clipboard.
6. SSH remote collection and packaging.

## License

MIT.
