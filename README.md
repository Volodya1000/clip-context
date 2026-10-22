# clip-context

A local-first Rust CLI for collecting source files and a filtered directory tree into a reproducible text snapshot.

**Current stage:** project snapshots, file selection, nested `.gitignore`, TOML profiles, stdout and clipboard output. Exhaustive secret detection, model-specific tokenization and exact output budgets are **not yet implemented**.

## Installation without Rust

The forthcoming tagged GitHub Releases include a Windows x86_64 ZIP and verified
PowerShell installer, and Linux x86_64 `.deb` and `.tar.gz` packages.
See [installation (English)](docs/installation.md) or
[установка (Русский)](docs/installation.ru.md). No release binaries have been
published by this PR: build/test workflows run on pull requests; publishing
requires a matching version tag on `main`.

## Example

Requires Rust 1.85 or newer.

```sh
cargo install --locked --path .
cpctx -t .
cpctx -s .
cpctx -s . --ext rs --ext toml
cpctx -c . --format compact
cpctx -s . --exclude-dir generated
cpctx -s . --profile backend
cpctx -s . --profile backend --ext py
cpctx -s . --max-bytes 50000 --max-estimated-tokens 10000
cargo test --locked --all-targets --all-features
```

Run `cpctx --help` (or `cpctx -h`) for a concise option reference and
`cpctx --help-full` for the complete English manual. The references are maintained
in [`src/help.txt`](src/help.txt) and [`src/help_full.txt`](src/help_full.txt).

Root and nested `.gitignore` files determine which files are included. Negated patterns (`!file`) work, including outside Git repositories. Ignore files from parent directories, global Git settings and `.git/info/exclude` are deliberately not read, so results only depend on the chosen root. CLI exclusions, hidden-file rules, and built-in generated-directory exclusions always take precedence over an unignore rule.

## Context output formats and diagnostics

By default, `-s` and `-c` retain the full Markdown report, which includes the
directory tree followed by every selected file. The tree repeats paths that
already appear in the file headings, especially for projects with many tiny
files. Use `--format compact` to omit the duplicate tree and shorten headings
without dropping source contents or path attribution:

```sh
cpctx -s . --format compact
cpctx -c . --format compact
cpctx -s /srv/project --ssh dev@server --format compact
```

`--format full` explicitly restores the default. `-t` is already tree-only,
so `--format` does not change tree output. Code fences in compact Markdown
still adapt to backticks in the original file content.

Successful collection prints a short English diagnostic to **stderr**:

```text
Context: 12.4 KiB (12700 bytes) | ~3175 tokens (estimated)
```

The byte count measures the **exact rendered UTF-8 output** (`-s` text, `-t`
tree or `-c` clipboard payload), not process RAM usage. The rough token
estimate is `ceil(bytes / 4)`, computed in constant time once the output is
rendered. Actual tokenizer results vary with model, programming language and
Unicode, so this is for quick planning, not a guaranteed context-window count.
No stats are printed on failed collection or clipboard operations.

The diagnostic is **never embedded** in the report or clipboard. Redirect
stdout to save clean context while still seeing stats in the terminal:

```sh
cpctx -s . --format compact > project-context.md
```

Compact Markdown usually saves the most on repositories with many small files;
source code itself remains unchanged, so savings vary by project. Neither
format compresses, summarizes or silently drops source code.

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

All modes require a project directory. Only `-c` modifies the clipboard; `-s` and `-t` leave it unchanged. All source files are read before output is printed, so a read or UTF-8 error does not produce a partial snapshot.

By default, only common source-file extensions are collected. Hidden files/directories, symbolic links, and known generated directories such as `.git`, `.venv`, `target`, `build` and `node_modules` are excluded. The built-in directory exclusions remain active when options are supplied.

**Security limitation:** neither `.gitignore` nor extension filtering finds secrets embedded in source files. Do not send snapshots of confidential or untrusted repositories to external services.

## Safety defaults and output budgets

Before producing **any stdout**, the collector rejects selected files with common private-key headers, selected credential token signatures (including GitHub, AWS, OpenAI, GitLab and Stripe) and obvious long inline credential assignments. The tool never includes detected secret values in error messages. Hidden paths and ignored files remain excluded before scanning.

The defaults are **262,144 source bytes** and **32,000 estimated tokens**, whichever limit is reached first. Token estimates use `ceil(source_bytes / 4)` and are **not** exact LLM token counts. Markdown delimiters, paths and the directory tree are outside these source budgets. Use `--max-bytes N` and `--max-estimated-tokens N` with strictly positive integers to tune them. Every included file is read with a bounded reader to avoid loading an unexpectedly large file. If the budget is exceeded, nothing is printed.

**Security limitation:** these are targeted, high-confidence detections, not a full substitute for specialized scanners (for example, Gitleaks), code review, or a model-specific tokenizer. An unknown credential format can still be leaked by a snapshot. **Do not send confidential code to external services** based only on this screening.

## Clipboard support

```sh
cpctx -c .
cpctx --clipboard . --profile backend
```

**Windows:** Requires built-in Windows PowerShell with `Set-Clipboard`. Input is passed as UTF-8 through the child's stdin, never as a shell argument.

**Linux:** In Wayland, install `wl-clipboard` (which provides `wl-copy`). In X11, install `xclip` or `xsel`. The CLI selects the backend using `WAYLAND_DISPLAY` and `DISPLAY`; on XWayland it can fall back from an unavailable Wayland executable to X11. These Linux clipboard tools keep the copied content available after `cpctx` exits. A headless/SSH-only environment without graphical session fails with an actionable error, never silently falls back to printing or writing a file.

For all modes, selected files must first pass `.gitignore`, configuration, credential screening and source limits. A failure during collection does **not** alter the clipboard; failure to invoke a clipboard backend produces nonzero exit status and **no source content on stdout**. A clipboard service might modify its state before reporting failure, so this guarantee applies to pre-copy validation only.

## Remote projects over SSH

Install `cpctx` **only on the local computer**. The remote machine needs a POSIX-compatible shell, `tar`, and an SSH server (plus read access to the requested directory).

```sh
cpctx -s /srv/myapp --ssh admin@dev-host
cpctx -t /srv/myapp --ssh admin@dev-host
cpctx -c /srv/myapp --ssh admin@dev-host
cpctx -c '/home/dev/my project' --ssh dev-host --profile backend
```

The path passed after `-s`, `-t` or `-c` is **remote** when `--ssh` is present. `--ssh` accepts an SSH config alias or `user@host`; SSH authentication and host-key checking use your existing `ssh` configuration. Clipboard output is always local.

Remote tar travels over the encrypted SSH connection; the CLI restricts the **raw transfer to 16 MiB** and extracts it in a temporary directory. Archive paths are validated: entries cannot escape the directory, archive symlinks and devices are never materialized, and duplicate file entries cannot overwrite. After extraction, existing `.gitignore` rules, selection profiles, byte/token budgets, and secret checks run locally before any output or clipboard change.

**Limitations:** `tar` initially packages the remote directory before local filtering, so large directories (especially containing `node_modules` or binary media) can hit the 16 MiB transfer limit even if most of their files would be ignored later. Remote files are temporarily materialized locally and removed when the operation ends. The server must have the `tar` command and a POSIX-compatible shell; Windows SSH servers and projects exceeding the raw archive cap are not yet supported.

### Security considerations

Only connect to SSH servers you trust and retain normal SSH host-key verification. Remote archive contents are treated as untrusted input; dangerous paths are rejected. Local credential detection covers known signatures but cannot guarantee that no secret is present.

## Quality gates

CI runs rustfmt, strict Clippy (`all`, `pedantic`, `nursery`, `cargo`), tests on Windows/Linux, rustdoc, MSRV compilation, **90% minimum line coverage**, cargo-deny, TOML/YAML hygiene, workflow linters, Conventional Commits verification through Cocogitto, changelog checks, and secret scanning.

Every PR must follow Conventional Commits naming and change `CHANGELOG.md`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Roadmap

1. Deterministic project snapshots and plain output (implemented).
2. Additional directory and extension selection via CLI (implemented).
3. Root and nested `.gitignore` rules (this PR).
4. Named TOML selection profiles (this PR).
5. Selected credential signatures and bounded source budgets (this PR).
6. OS clipboard output on Windows/Linux (this PR); improved detection and final-output limits remain future work.
7. SSH remote collection without installing cpctx on servers (this PR).
8. Context output metrics and compact Markdown (implemented).
9. Improved server-side prefiltering and packaging for larger repositories (planned).
10. Expanded `cpctx --help` reference (implemented).

## License

MIT.
