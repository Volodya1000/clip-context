//! End-to-end clipboard mode tests without requiring a real desktop session.

mod support;

use std::process::Command;

use support::Fixture;

#[test]
fn clipboard_mode_rejects_invalid_projects_without_emitting_content() {
    let fixture = Fixture::new();
    let result = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .args(["-c"])
        .arg(fixture.path().join("missing"))
        .output()
        .expect("binary must run");

    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn clipboard_mode_rejects_suspected_credentials_before_backend_invocation() {
    let fixture = Fixture::new();
    let synthetic_token = format!("ghp_{}", "Q".repeat(36));
    fixture.file("main.rs", &format!("let token = \"{synthetic_token}\";"));
    let result = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("--clipboard")
        .arg(fixture.path())
        .output()
        .expect("binary must run");

    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert!(!String::from_utf8_lossy(&result.stderr).contains(&synthetic_token));
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::{Command, Output};

    use super::Fixture;

    fn executable(fixture: &Fixture, program: &str, code: &str) {
        let path = format!("tools/{program}");
        fixture.file(&path, code);
        let full_path = fixture.path().join(path);
        let mut permissions = fs::metadata(&full_path)
            .expect("test command must exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(full_path, permissions).expect("test script must be executable");
    }

    fn execute_in_session(
        fixture: &Fixture,
        mode: &str,
        wayland: bool,
        x11: bool,
        arguments: &[&str],
    ) -> Output {
        let tool_dir: PathBuf = fixture.path().join("tools");
        fs::create_dir_all(&tool_dir).expect("test tool directory must exist");
        let mut command = Command::new(env!("CARGO_BIN_EXE_cpctx"));
        command
            .arg(mode)
            .arg(fixture.path())
            .args(arguments)
            .env("PATH", tool_dir)
            .env("CPCTX_CAPTURE", fixture.path().join("captured.md"));
        if wayland {
            command.env("WAYLAND_DISPLAY", "test-wayland");
        } else {
            command.env_remove("WAYLAND_DISPLAY");
        }
        if x11 {
            command.env("DISPLAY", ":99");
        } else {
            command.env_remove("DISPLAY");
        }
        command.output().expect("binary must run")
    }

    fn captured(fixture: &Fixture) -> String {
        fs::read_to_string(fixture.path().join("captured.md"))
            .expect("mock clipboard captured text")
    }

    const CAPTURE: &str = "#!/bin/sh\n/bin/cat > \"$CPCTX_CAPTURE\"\n";

    #[test]
    fn wayland_clipboard_matches_stdout_exactly_without_printing_source() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "fn main() {}");
        executable(&fixture, "wl-copy", CAPTURE);

        let expected = Command::new(env!("CARGO_BIN_EXE_cpctx"))
            .arg("-s")
            .arg(fixture.path())
            .output()
            .expect("stdout mode must execute");
        assert!(expected.status.success());

        let result = execute_in_session(&fixture, "-c", true, false, &[]);
        assert!(result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        assert_eq!(captured(&fixture).as_bytes(), expected.stdout);
    }

    #[test]
    fn compact_clipboard_matches_stdout_and_reports_payload_size() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "minimal context");
        executable(&fixture, "wl-copy", CAPTURE);

        let expected = Command::new(env!("CARGO_BIN_EXE_cpctx"))
            .arg("-s")
            .arg(fixture.path())
            .args(["--format", "compact"])
            .output()
            .expect("CLI must execute");
        assert!(expected.status.success());

        let copied = execute_in_session(&fixture, "-c", true, false, &["--format", "compact"]);
        assert!(copied.status.success());
        assert_eq!(copied.stdout, Vec::<u8>::new());
        assert_eq!(captured(&fixture).as_bytes(), expected.stdout);
        let stderr = String::from_utf8_lossy(&copied.stderr);
        assert!(stderr.contains(&format!("({} bytes)", expected.stdout.len())));
        assert!(stderr.contains("tokens (estimated)"));
        assert!(!stderr.contains("minimal context"));
    }

    #[test]
    fn long_clipboard_mode_respects_file_selection() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "selected Rust text");
        fixture.file("src/other.py", "excluded Python text");
        executable(&fixture, "wl-copy", CAPTURE);

        let result = execute_in_session(&fixture, "--clipboard", true, false, &["--ext", "rs"]);
        assert!(result.status.success());
        let content = captured(&fixture);
        assert!(content.contains("selected Rust text"));
        assert!(!content.contains("excluded Python text"));
    }

    #[test]
    fn x11_uses_clipboard_selection() {
        let fixture = Fixture::new();
        fixture.file("README.md", "X11 captured context");
        executable(&fixture, "xclip", CAPTURE);

        let result = execute_in_session(&fixture, "-c", false, true, &[]);
        assert!(result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        assert!(captured(&fixture).contains("X11 captured context"));
    }

    #[test]
    fn x11_uses_xsel_when_xclip_is_missing() {
        let fixture = Fixture::new();
        fixture.file("README.md", "xsel captured context");
        executable(&fixture, "xsel", CAPTURE);

        let result = execute_in_session(&fixture, "-c", false, true, &[]);
        assert!(result.status.success());
        assert!(captured(&fixture).contains("xsel captured context"));
    }

    #[test]
    fn wayland_can_use_x11_when_wl_copy_is_unavailable() {
        let fixture = Fixture::new();
        fixture.file("README.md", "XWayland content");
        executable(&fixture, "xclip", CAPTURE);

        let result = execute_in_session(&fixture, "-c", true, true, &[]);
        assert!(result.status.success());
        assert!(captured(&fixture).contains("XWayland content"));
    }

    #[test]
    fn fails_closed_if_clipboard_backend_is_unavailable() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "confidential snapshot contents");

        let result = execute_in_session(&fixture, "-c", true, false, &[]);
        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        assert!(
            !String::from_utf8_lossy(&result.stderr).contains("confidential snapshot contents")
        );
        assert!(!fixture.path().join("captured.md").exists());
    }

    #[test]
    fn headless_environment_refuses_clipboard_without_stdout() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "safely not printed");
        let result = execute_in_session(&fixture, "-c", false, false, &[]);

        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        assert!(!fixture.path().join("captured.md").exists());
    }

    #[test]
    fn backend_failure_is_not_disguised_by_fallback() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "safe code");
        executable(&fixture, "wl-copy", "#!/bin/sh\nexit 7\n");
        executable(&fixture, "xclip", CAPTURE);

        let result = execute_in_session(&fixture, "-c", true, true, &[]);
        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        assert!(!fixture.path().join("captured.md").exists());
    }

    #[test]
    fn source_validation_precedes_clipboard_invocation() {
        let fixture = Fixture::new();
        fixture.file("src/main.rs", "some valid data");
        executable(&fixture, "wl-copy", CAPTURE);
        let result = execute_in_session(&fixture, "-c", true, false, &["--max-bytes", "1"]);

        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        assert!(!fixture.path().join("captured.md").exists());
    }
}
