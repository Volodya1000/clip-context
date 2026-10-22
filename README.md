# clip-context

Fast, local-first project context copier (Rust CLI). The project is **bootstrapping**; copying and SSH are not implemented yet.

## Build and run

Requires Rust 1.85 or newer.

```sh
cargo run -- --help
cargo run -- --version
cargo test --locked --all-targets --all-features
```

## Quality gates

The CI runs rustfmt, strict Clippy (`all`, `pedantic`, `nursery`, `cargo`), tests on Windows/Linux, rustdoc, MSRV compilation, coverage, cargo-deny, TOML/YAML hygiene, workflow linters, Conventional Commits verification via Cocogitto, PR titles via a dedicated GitHub Action, changelog checks, and secret scanning.

Every PR must follow Conventional Commits naming and change `CHANGELOG.md`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

Project-file discovery, filtering, secrets protection, clipboard integration on Windows/Linux, profiles, and SSH will be implemented in follow-up pull requests. No clipboard functionality is claimed for this bootstrap release.

## License

MIT.
