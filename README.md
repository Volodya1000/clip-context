# clip-context

A local-first Rust CLI for collecting source files and a filtered directory tree into a reproducible text snapshot.

**Current stage:** project snapshots, file selection, nested `.gitignore`, TOML profiles and stdout. Native clipboard integration, exhaustive secret detection, model-specific tokenization and SSH are **not yet implemented**.

## Example

Requires Rust 1.85 or newer.

```sh
cargo install --locked --path .
cpctx -t .
cpctx -s .
cpctx -s . --ext rs --ext toml
cpctx -s . --exclude-dir generated
cpctx -s . --profile backend
cpctx -s . --profile backend --ext py
cpctx -s . --max-bytes 50000 --max-estimated-tokens 10000
cargo test --locked --all-targets --all-features
```

- `-s` / `--stdout` prints Markdown with selected source files and an ordered tree.
- `-t` / `--tree` prints only the tree.
- `--ext EXT` can be repeated to restrict selection to exactly those extensions, replacing the built-in extension allowlist.
- `--exclude-dir NAME` can be repeated to exclude directories by name at every depth, in addition to built-in exclusions.

Root and nested `.gitignore` files determine which files are included. Negated patterns (`!file`) work, including outside Git repositories. Ignore files from parent directories, global Git settings and `.git/info/exclude` are deliberately not read, so results only depend on the chosen root. CLI exclusions, hidden-file rules, and built-in generated-directory exclusions always take precedence over an unignore rule.

## Named selection profiles

Profiles are loaded **only** when `--profile NAME` is provided. Create `.cpctx.toml` in the project directory:

```toml
[profiles.backend]
extensions = ["rs", "toml"]
exclude_directories = ["generated"]

[profiles.docs]
extensions = ["md"]
exclude_directories = ["drafts"]
```

```sh
cpctx -s . --profile backend
cpctx -t . --profile docs
cpctx -s . --profile backend --ext py --exclude-dir temporary
cpctx -s . --profile docs --config settings/profiles.toml
```

The profile defines initial extensions and directory exclusions. Explicit `--ext` parameters **replace** profile extensions, while `--exclude-dir` **adds** excluded directories. Hidden-file rules, built-in exclusions, and all `.gitignore` rules always apply. Unknown profile keys and invalid values cause an error without printing any file content.

An explicit `--config FILE` path is resolved relative to the chosen project directory unless absolute; it requires `--profile`. The hidden default `.cpctx.toml` is never included in snapshots.

Both commands require a project directory. They do not modify any files or clipboard contents. All source files are read before output is printed, so a read or UTF-8 error does not produce a partial snapshot.

By default, only common source-file extensions are collected. Hidden files/directories, symbolic links, and known generated directories such as `.git`, `.venv`, `target`, `build` and `node_modules` are excluded. The built-in directory exclusions remain active when options are supplied.

**Security limitation:** neither `.gitignore` nor extension filtering finds secrets embedded in source files. Do not send snapshots of confidential or untrusted repositories to external services.

## Safety defaults and output budgets

Before producing **any stdout**, the collector rejects selected files with common private-key headers, selected credential token signatures (including GitHub, AWS, OpenAI, GitLab and Stripe) and obvious long inline credential assignments. The tool never includes detected secret values in error messages. Hidden paths and ignored files remain excluded before scanning.

The defaults are **262,144 source bytes** and **32,000 estimated tokens**, whichever limit is reached first. Token estimates use `ceil(source_bytes / 4)` and are **not** exact LLM token counts. Markdown delimiters, paths and the directory tree are outside these source budgets. Use `--max-bytes N` and `--max-estimated-tokens N` with strictly positive integers to tune them. Every included file is read with a bounded reader to avoid loading an unexpectedly large file. If the budget is exceeded, nothing is printed.

**Security limitation:** these are targeted, high-confidence detections, not a full substitute for specialized scanners (for example, Gitleaks), code review, or a model-specific tokenizer. An unknown credential format can still be leaked by a snapshot. **Do not send confidential code to external services** based only on this screening.

## Quality gates

CI runs rustfmt, strict Clippy (`all`, `pedantic`, `nursery`, `cargo`), tests on Windows/Linux, rustdoc, MSRV compilation, **90% minimum line coverage**, cargo-deny, TOML/YAML hygiene, workflow linters, Conventional Commits verification through Cocogitto, changelog checks, and secret scanning.

Every PR must follow Conventional Commits naming and change `CHANGELOG.md`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

1. Deterministic project snapshots and plain output (implemented).
2. Additional directory and extension selection via CLI (implemented).
3. Root and nested `.gitignore` rules (this PR).
4. Named TOML selection profiles (this PR).
5. Selected credential signatures and bounded source budgets (this PR).
6. Broader secret rules and exact-output limits, followed by native Windows/Linux clipboard.
7. SSH remote collection and packaging.

## License

MIT.
