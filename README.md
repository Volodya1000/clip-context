# clip-context

A local-first Rust CLI for collecting source files and a filtered directory tree into a reproducible text snapshot.

**Current stage:** project snapshots and stdout. Native clipboard integration, `.gitignore` semantics, configurable profiles, secret detection and SSH are **not yet implemented**.

## Example

Requires Rust 1.85 or newer.

```sh
cargo install --locked --path .
cpctx -t .
cpctx -s .
cpctx -s . --ext rs --ext toml
cpctx -s . --exclude-dir generated
cargo test --locked --all-targets --all-features
```

- `-s` / `--stdout` prints Markdown with selected source files and an ordered tree.
- `-t` / `--tree` prints only the tree.
- `--ext EXT` can be repeated to restrict selection to exactly those extensions, replacing the built-in extension allowlist.
- `--exclude-dir NAME` can be repeated to exclude directories by name at every depth, in addition to built-in exclusions.

Both commands require a project directory. They do not modify any files or clipboard contents. All source files are read before output is printed, so a read or UTF-8 error does not produce a partial snapshot.

By default, only common source-file extensions are collected. Hidden files/directories, symbolic links, and known generated directories such as `.git`, `.venv`, `target`, `build` and `node_modules` are excluded. The built-in directory exclusions remain active when options are supplied.

**Security limitation:** extension filtering does not find secrets embedded inside source files and does not yet interpret `.gitignore`. Do not send snapshots of confidential or untrusted repositories to external services.

## Quality gates

CI runs rustfmt, strict Clippy (`all`, `pedantic`, `nursery`, `cargo`), tests on Windows/Linux, rustdoc, MSRV compilation, **90% minimum line coverage**, cargo-deny, TOML/YAML hygiene, workflow linters, Conventional Commits verification through Cocogitto, changelog checks, and secret scanning.

Every PR must follow Conventional Commits naming and change `CHANGELOG.md`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

1. Deterministic project snapshots and plain output (implemented).
2. Additional directory and extension selection via CLI (this PR).
3. Full `.gitignore` and user TOML profiles.
4. Content-based secret detection and safe output limits.
5. Native Windows/Linux clipboard.
6. SSH remote collection and packaging.

## License

MIT.
