# clip-context

**One command to collect the right project files for an AI assistant.**

A lightweight Rust CLI that builds a readable, deterministic context from local projects or remote servers over SSH. Print it, inspect the file tree, or copy it straight to your clipboard.

**English** · [Русский](README.ru.md)

## Installation

Choose a prebuilt [GitHub Release](https://github.com/Volodya1000/clip-context/releases) once a version is published, or build from source.

- **Windows:** x86_64 ZIP and optional PowerShell user-PATH installer.
- **Linux:** x86_64 `.deb` or `.tar.gz`, with checksum verification.

See [installation instructions](docs/installation.md) ([на русском](docs/installation.ru.md)). Release workflows build and test packages on PRs; downloading assets requires a published version tag. Building from source requires **Rust 1.85+**:

```sh
git clone https://github.com/Volodya1000/clip-context.git
cd clip-context
cargo install --locked --path .
```

```sh
cpctx -s .                         # Print full context
cpctx -c . --format compact        # Copy compact context to clipboard
cpctx -t .                         # Show just the project tree
```

`-s` prints context to stdout; `-c` copies it **locally** without printing source text; `-t` prints file paths without reading file contents.

## Choose what to include

```sh
cpctx -s . --ext rs --ext toml      # Only Rust and TOML
cpctx -s . --exclude-dir generated # Exclude a directory name at any depth
cpctx -c . --profile backend       # Use a named configuration
```

Create `.cpctx.toml` in the project's root:

```toml
[profiles.backend]
extensions = ["rs", "toml"]
exclude_directories = ["generated"]

[profiles.docs]
extensions = ["md"]
```

Run `cpctx -s . --profile docs`, or supply another file with `--config settings/profiles.toml --profile docs`. A CLI `--ext` **replaces** profile extensions; `--exclude-dir` **adds** exclusions. Root and nested `.gitignore` rules, hidden paths and built-in exclusions still apply.

## Output size and format

Use `--format compact` to keep all selected files but omit the repeated directory tree. The default `--format full` includes it. `-t` always produces only a tree.

After successful output, `cpctx` writes a short diagnostic to **stderr**:

```text
Context: 12.4 KiB (12700 bytes) | ~3175 tokens (estimated)
```

The size is the **complete rendered UTF-8 context**, not RAM usage. Token count is a fast approximation, `ceil(bytes / 4)`, not a model-specific tokenizer. Neither the diagnostic nor errors are included in stdout or the clipboard.

Use `--max-bytes N` (default **262144**) and `--max-estimated-tokens N` (default **32000**) to cap the **final output**, including file paths, code fences and the tree if present. The stricter limit wins; exceeding it fails before printing or copying.

```sh
cpctx -s . --format compact --max-estimated-tokens 8000 > context.md
```

## Remote projects via SSH

Only the **client computer** needs `cpctx`. A remote POSIX server needs SSH access, a shell and `tar`:

```sh
cpctx -s /srv/project --ssh dev@server
cpctx -c /srv/project --ssh dev@server --format compact
```

The path is remote, but clipboard output stays on your local computer. Host-key verification uses your SSH configuration. The transferred archive is limited to **16 MiB**; supported tar implementations prefilter known generated directories before transfer. Profiles and .gitignore filtering still run locally, so other large ignored files can hit this limit.

## Clipboard support

- **Windows:** Windows PowerShell with `Set-Clipboard`.
- **Linux / Wayland:** `wl-copy` from `wl-clipboard`.
- **Linux / X11:** `xclip` or `xsel`.

A graphical session is required to copy. Headless systems can use `cpctx -s` and redirect stdout to a file.

## Safety and limitations

Known private-key headers and several common credential patterns block source exports. This is **not exhaustive secret detection**: inspect context before sharing sensitive projects. Tree-only mode reports paths without reading source file contents. SSH archives are handled in a temporary directory, with unsafe paths and link materialization blocked. The full-context output budget and the SSH archive transfer limit are separate safeguards.

See [acceptance evidence](docs/acceptance.md) and the [detector threat model](docs/security.md).

## Reference

Run `cpctx --help` for a short summary or `cpctx --help-full` for the complete English reference.

```text
cpctx -s PATH [OPTIONS]    Print source context (default: full)
cpctx -t PATH [OPTIONS]    Print file tree only
cpctx -c PATH [OPTIONS]    Copy source context to local clipboard

--format full|compact      Markdown format
--ext EXT                  Include extension (repeatable)
--exclude-dir NAME         Exclude directory name (repeatable)
--profile NAME             Named profile from .cpctx.toml
--config FILE              Alternative profile file (requires --profile)
--ssh USER@HOST            Read project via SSH
--max-bytes N              Maximum rendered UTF-8 bytes
--max-estimated-tokens N   Maximum approximate tokens of rendered output
-h, --help                 Short CLI help
--help-full                Detailed CLI help
-V, --version              Version
```

For development, run `cargo test --locked --all-targets --all-features`. See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution and CI policies.

Licensed under [MIT](LICENSE).
