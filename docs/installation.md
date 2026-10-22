# Install clip-context without Rust

Windows x86_64 and Linux x86_64 packages are produced by the
[release workflow](../.github/workflows/release.yml). A PR only tests packages:
**download links work only after the first tagged release has been published**.
See the [Releases page](https://github.com/Volodya1000/clip-context/releases).

## Windows (PowerShell)

Download and inspect the [install script](../scripts/install.ps1), then run:

```powershell
powershell -NoProfile -File .\scripts\install.ps1
cpctx --version
cpctx --help
```

The script downloads the latest stable Windows x86_64 release and its checksums,
validates SHA-256, checks the embedded executable version, and installs to
`%LOCALAPPDATA%\Programs\clip-context`. It adds this directory to your
**user PATH**, updates the current shell and broadcasts an environment-change
notification. You do not need administrator privileges or Rust. Open a new
terminal if an existing terminal still has an old environment.

To pin a release, pass `-Version vX.Y.Z`. Local/offline installation is supported
through `-Version`, `-ArchivePath` and `-ChecksumPath`. To uninstall:

```powershell
powershell -NoProfile -File .\scripts\install.ps1 -Uninstall
```

Do not run downloaded PowerShell files you have not inspected.

## Debian / Ubuntu / Linux Mint (x86_64)

Download the `clip-context_X.Y.Z_amd64.deb` asset from the published release.
From the directory containing it, run:

```sh
sudo dpkg -i clip-context_X.Y.Z_amd64.deb
cpctx --version
cpctx --help
```

The Debian package installs `/usr/bin/cpctx`, already on PATH. Remove it
using `sudo dpkg -r clip-context`. It is compiled for Ubuntu 22.04
(glibc 2.35) and newer compatible x86_64 Linux distributions.

## Other x86_64 Linux distributions

Download the `cpctx-vX.Y.Z-linux-x86_64.tar.gz` archive and verify it against
`SHA256SUMS.txt`, both published with the release:

```sh
grep 'linux-x86_64.tar.gz
tar -xzf cpctx-vX.Y.Z-linux-x86_64.tar.gz
sudo install -m 755 cpctx /usr/local/bin/cpctx
cpctx --version
```

For a single-archive check, select the applicable checksum line first.
`/usr/local/bin` should already be on PATH on standard Linux distributions.
Alternatively, inspect [the Linux installer](../scripts/install.sh), then execute
`sudo sh scripts/install.sh vX.Y.Z`. It verifies SHA-256 and installs
`/usr/local/bin/cpctx` (normally on PATH). For test environments, set
`PREFIX`, `CPCTX_ARCHIVE` and `CPCTX_CHECKSUMS` to use checked local files.

This release does not claim macOS, ARM64 or Alpine/musl compatibility.

## Publishing a real release (maintainers)

1. Merge and verify the PR checks and manual platform acceptance.
2. Update `Cargo.toml`/lockfile and `CHANGELOG.md` if the release version changes.
3. Create/push an existing tag such as `v0.1.0` pointing to a commit on `main`.
4. The tag workflow checks exact version equality, builds both platforms,
   smoke-tests the ZIP / user PATH install / Debian package, checks SHA-256,
   and only then publishes GitHub Release assets.
5. A manual `workflow_dispatch` run **never publishes** a release.

Checksum files validate transfers but do not by themselves prove artifact
authenticity. Review GitHub Actions provenance and signing before treating
artifacts as production supply-chain attested.
 SHA256SUMS.txt | sha256sum -c -
tar -xzf cpctx-vX.Y.Z-linux-x86_64.tar.gz
sudo install -m 755 cpctx /usr/local/bin/cpctx
cpctx --version
```

For a single-archive check, select the applicable checksum line first.
`/usr/local/bin` should already be on PATH on standard Linux distributions.
Alternatively, inspect [the Linux installer](../scripts/install.sh), then execute
`sudo sh scripts/install.sh vX.Y.Z`. It verifies SHA-256 and installs
`/usr/local/bin/cpctx` (normally on PATH). For test environments, set
`PREFIX`, `CPCTX_ARCHIVE` and `CPCTX_CHECKSUMS` to use checked local files.

This release does not claim macOS, ARM64 or Alpine/musl compatibility.

## Publishing a real release (maintainers)

1. Merge and verify the PR checks and manual platform acceptance.
2. Update `Cargo.toml`/lockfile and `CHANGELOG.md` if the release version changes.
3. Create/push an existing tag such as `v0.1.0` pointing to a commit on `main`.
4. The tag workflow checks exact version equality, builds both platforms,
   smoke-tests the ZIP / user PATH install / Debian package, checks SHA-256,
   and only then publishes GitHub Release assets.
5. A manual `workflow_dispatch` run **never publishes** a release.

Checksum files validate transfers but do not by themselves prove artifact
authenticity. Review GitHub Actions provenance and signing before treating
artifacts as production supply-chain attested.
