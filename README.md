# clip-context

A local-first Rust CLI for collecting source files and a filtered directory tree into a reproducible text snapshot.

**Current stage:** project snapshots and stdout. Native clipboard integration, custom file selection, `.gitignore` semantics, configurable profiles, secret detection and SSH are **not yet implemented**.

## Example

Requires Rust 1.85 or newer.

```sh
cargo run --bin cc -- --help
cargo run --bin cc -- --tree .
cargo run --bin cc -- --stdout .
cargo test --locked --all-targets --all-features
```

After adding the executable to your PATH, the commands are `cc --tree .` and `cc --stdout .`. The shorthand forms `cc -t .` and `cc -s .` also work.\n\n`--stdout` prints Markdown with selected source files and an ordered tree. `--tree` prints only the tree. Both commands require a directory; they do not modify files or clipboard contents. You can redirect stdout to a file if needed.

By default, only common source-file extensions are collected. Hidden files/directories, symbolic links, and known generated directories such as `.git`, `.venv`, `target`, `build` and `node_modules` are excluded. Failing to read any selected file aborts the operation instead of producing a partial result.

**Linux naming caveat:** `cc` is also the standard C-compiler command. Installing a Rust binary named `cc` earlier in the PATH can break compiling C and Rust projects. For now, prefer `cargo run --bin cc -- ...` instead of globally installing `cc`; the distribution workflow must resolve this collision safely.\n\n**Security limitation:** filenames and extensions alone do not detect secrets embedded inside source files. Until content scanning is implemented, do not pass untrusted or confidential repositories to `--stdout` or paste the output into an external service.

## Quality gates

The CI runs rustfmt, strict Clippy (`all`, `pedantic`, `nursery`, `cargo`), tests on Windows/Linux, rustdoc, MSRV compilation, **90% minimum line coverage**, cargo-deny, TOML/YAML hygiene, workflow linters, Conventional Commits verification of both commit history and PR titles through Cocogitto, changelog checks, and secret scanning.

Every PR must follow Conventional Commits naming and change `CHANGELOG.md`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

1. Deterministic project snapshots and plain output (this PR).
2. Configuration-driven selection and `.gitignore` support.
3. Content-based secret detection and safe output limits.
4. Native Windows/Linux clipboard.
5. SSH remote collection and packaging.

## License

MIT.
