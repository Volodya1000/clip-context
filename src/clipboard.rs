//! Safe clipboard output through OS-native clipboard commands.
//!
//! Backend processes receive the completed snapshot through a piped standard
//! input. Neither the snapshot nor its contents are inserted into command
//! arguments, shell expressions, or diagnostic messages.

use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Place prepared UTF-8 text on the system clipboard.
///
/// On Windows, PowerShell's `Set-Clipboard` handles Unicode text. On Linux,
/// `wl-copy` is used for Wayland, with `xclip` or `xsel` for X11. Linux
/// clipboard utilities preserve the selection after this CLI exits.
///
/// # Errors
///
/// Fails if no supported clipboard backend is available or a backend process
/// cannot accept the complete text. Errors never contain clipboard contents.
pub fn copy_text(text: &str) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // The command is constant; the snapshot is passed solely via stdin.
        return write_command(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false); Set-Clipboard -Value ([Console]::In.ReadToEnd())",
            ],
            text,
        );
    }

    #[cfg(target_os = "linux")]
    {
        return copy_linux(text);
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = text;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "clipboard output currently supports only Windows and Linux",
        ))
    }
}

#[cfg(target_os = "linux")]
fn copy_linux(text: &str) -> io::Result<()> {
    use std::env;

    let wayland = env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty());
    let x11 = env::var_os("DISPLAY").is_some_and(|value| !value.is_empty());

    if wayland {
        match write_command("wl-copy", &[], text) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound && x11 => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Wayland clipboard requires wl-copy (install wl-clipboard)",
                ));
            }
            Err(error) => return Err(error),
        }
    }

    if x11 {
        return match write_command("xclip", &["-selection", "clipboard"], text) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match write_command("xsel", &["--clipboard", "--input"], text) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            "X11 clipboard requires xclip or xsel",
                        ))
                    }
                    result => result,
                }
            }
            result => result,
        };
    }

    Err(io::Error::new(
        io::ErrorKind::NotConnected,
        "no graphical clipboard session (WAYLAND_DISPLAY or DISPLAY is missing)",
    ))
}

fn write_command(program: &str, arguments: &[&str], text: &str) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot launch clipboard backend {program}: {error}"),
            )
        })?;
    let mut stdin = child.stdin.take().expect("clipboard stdin must be piped");
    let write_result = stdin.write_all(text.as_bytes());
    drop(stdin);
    let wait_result = child.wait();
    write_result?;
    let status = wait_result?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "clipboard backend {program} returned an unsuccessful status"
        )));
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::write_command;

    #[test]
    fn passes_text_through_standard_input_without_shell_interpolation() {
        let source = "UTF-8: Привет 🌍 and \"quoted\"; $(exit 7)";
        let result = write_command("sh", &["-c", "cat >/dev/null"], source);
        assert!(result.is_ok());
    }

    #[test]
    fn nonzero_backend_status_is_an_error_without_data_in_diagnostics() {
        let secret_like_text = "a value that must not appear in diagnostic output";
        let error = write_command("sh", &["-c", "cat >/dev/null; exit 3"], secret_like_text)
            .expect_err("failed backend must be reported");
        assert!(!error.to_string().contains(secret_like_text));
    }

    #[test]
    fn a_missing_backend_returns_not_found() {
        let error = write_command(
            "cpctx-nonexistent-clipboard-backend",
            &[],
            "text",
        )
        .expect_err("nonexistent program must fail");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}
