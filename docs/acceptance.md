# Acceptance evidence and remaining release gates

Only mark a row passed after running its commands on the named platform. Unit
tests and mock backends do not prove a real desktop clipboard or SSH connection.

| Environment | Scenario | Status and evidence |
| --- | --- | --- |
| Managed Linux x86_64, Rust 1.85 | Baseline full test suite | Passed locally, 2026-10-22 |
| Managed Linux x86_64 | Unicode, paths with spaces, profiles, ignores, full/compact/tree, budgets and headless clipboard errors | Passed locally via `bash scripts/acceptance/linux.sh`, 2026-10-22 |
| Managed Linux x86_64, actual OpenSSH server and GNU tar | Key authentication, strict host-key verification, Unicode, profiles, gitignore and >16 MiB generated dependencies | Passed locally via `CPCTX_SSHD_UNPRIVILEGED=1 bash scripts/acceptance/ssh.sh`, 2026-10-22 |
| Ubuntu 22.04, real Xvfb/X11 and Sway/Wayland | Clipboard survives CLI exit, real xsel fallback; validation failure preserves clipboard | Passed in [CI run 39838322954](https://github.com/Volodya1000/clip-context/actions/runs/39838322954), 2026-10-22 |
| Windows GitHub runner | Unicode stdout/tree and native PowerShell clipboard, persistence, budget atomicity | Passed in [CI run 39838322954](https://github.com/Volodya1000/clip-context/actions/runs/39838322954), 2026-10-22 |
| Linux Mint desktop, Wayland and X11 | Desktop clipboard, including xsel fallback | Pending manual desktop run |
| Real SSH with a tar shim exposing POSIX-only options | No-exclude fallback, oversized rejection, normal project | Passed locally, 2026-10-22; a different native POSIX tar implementation remains a manual portability check |
| Packaged Windows/Linux installation | ZIP/PATH install/uninstall, tar/deb/checksum checks | Release packages workflow; attach run URL for the exact release commit |
| Active main protection | A failing check actually blocks merge | Blocked: administration API returned HTTP 403; see [setup](main-protection.md) |

## Reproduce

```sh
cargo build --release --locked
bash scripts/acceptance/linux.sh
bash scripts/acceptance/linux-gui.sh
bash scripts/acceptance/ssh.sh
```

The GUI script needs Xvfb, xclip, xsel, Sway and wl-clipboard. The SSH script needs
OpenSSH client/server, keygen and GNU timeout; it creates disposable keys, a localhost-only
server and a verified known-host entry. It uses `sudo -n` to run sshd on a CI
runner. `CPCTX_SSHD_UNPRIVILEGED=1` supports environments where the current user
can run sshd directly. Privileged Linux runners use PAM account checks so
password-locked runner accounts can authenticate by key; password and keyboard
interactive authentication remain disabled. The actual daemon PID is used for
cleanup, independently of sudo monitor processes. No permanent SSH configuration is changed, authentication
is key-only, and keys/server/temporary projects are removed on exit.

On Windows, run `scripts/acceptance/windows.ps1` after the release build. For a
Mint desktop, additionally compare `cpctx -s PROJECT` with the clipboard after
`cpctx -c PROJECT`, wait until cpctx exits, and paste into another application.
Exercise Wayland/wl-copy, X11/xclip and X11 with only xsel installed. Preserve a
clipboard value, trigger a budget/credential error, and confirm it is unchanged.

For each manual run record the commit, OS/version, backend versions, command,
exit status, byte comparison, and sanitized logs. The Live acceptance workflow
retains Linux/Windows logs as artifacts, including failures. Do not upload
temporary directories containing SSH keys or confidential source.

## Release checklist

- [ ] Apply and negatively test main protection (issue #23).
- [ ] All strict CI, secret scanning, package and live acceptance jobs succeed
  on the exact final commit.
- [ ] Complete the pending desktop/native POSIX-tar rows above (issue #20).
- [ ] Review [credential detection limitations](security.md) (issue #22).
- [ ] Finalize the versioned changelog; confirm Cargo version and `vX.Y.Z` match.
- [ ] Publish the tag only after acceptance, then verify downloaded checksums,
  installation and `cpctx --version` from the published assets.

No release has been published by these acceptance changes.
