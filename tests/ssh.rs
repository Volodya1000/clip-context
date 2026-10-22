//! SSH collection smoke tests without requiring a real remote host.

#[cfg(unix)]
mod support;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
use support::Fixture;

#[cfg(unix)]
fn prepare_mock_ssh(fixture: &Fixture) {
    use std::os::unix::fs::PermissionsExt;

    fixture.file(
        "tools/ssh",
        "#!/bin/sh\nexec /bin/tar -cf - -C \"$CPCTX_TEST_REMOTE\" .\n",
    );
    let command = fixture.path().join("tools/ssh");
    let mut permissions = fs::metadata(&command)
        .expect("mock SSH should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(command, permissions).expect("mock SSH should be executable");
}

#[cfg(unix)]
fn run_mock_ssh(fixture: &Fixture, flags: &[&str]) -> std::process::Output {
    let remote = fixture.path().join("remote");
    let mut command = Command::new(env!("CARGO_BIN_EXE_cpctx"));
    command
        .arg("-s")
        .arg("/srv/project")
        .args(["--ssh", "test@host"])
        .args(flags)
        .env("PATH", fixture.path().join("tools"))
        .env("CPCTX_TEST_REMOTE", remote);
    command.output().expect("cpctx should run")
}

#[cfg(unix)]
#[test]
fn ssh_mode_collects_only_allowed_files_and_respects_nested_gitignore() {
    let fixture = Fixture::new();
    fixture.file("remote/src/main.rs", "expected remote Rust code");
    fixture.file("remote/src/extra.py", "unselected Python code");
    fixture.file("remote/.gitignore", "private/\n");
    fixture.file("remote/private/password.rs", "blocked by gitignore");
    prepare_mock_ssh(&fixture);

    let output = run_mock_ssh(&fixture, &["--ext", "rs"]);
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("Markdown should be UTF-8");
    assert!(markdown.contains("expected remote Rust code"));
    assert!(!markdown.contains("unselected Python code"));
    assert!(!markdown.contains("blocked by gitignore"));
    assert!(!markdown.contains("clip-context-fixture"));
    let stats = String::from_utf8_lossy(&output.stderr);
    assert!(stats.contains(&format!("({} bytes)", markdown.len())));
    assert!(stats.contains("tokens (estimated)"));
}

#[cfg(unix)]
#[test]
fn ssh_mode_rejects_remote_secrets_before_printing_any_content() {
    let fixture = Fixture::new();
    fixture.file("remote/src/main.rs", "public source");
    let synthetic_token = format!("ghp_{}", "Q".repeat(36));
    fixture.file(
        "remote/src/secret.rs",
        &format!("token = \"{synthetic_token}\""),
    );
    prepare_mock_ssh(&fixture);

    let output = run_mock_ssh(&fixture, &[]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(&synthetic_token));
}

#[cfg(unix)]
#[test]
fn ssh_mode_rejects_invalid_hosts_before_spawning_ssh() {
    let fixture = Fixture::new();
    fixture.file("remote/src/lib.rs", "public source");
    prepare_mock_ssh(&fixture);

    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .args(["-s", "/srv/project", "--ssh", "-oBadOption"])
        .env("PATH", fixture.path().join("tools"))
        .env("CPCTX_TEST_REMOTE", fixture.path().join("remote"))
        .output()
        .expect("cpctx should run");

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[cfg(unix)]
#[test]
fn ssh_mode_respects_source_size_limit_after_transfer() {
    let fixture = Fixture::new();
    fixture.file("remote/src/main.rs", "longer than a tiny budget");
    prepare_mock_ssh(&fixture);

    let output = run_mock_ssh(&fixture, &["--max-bytes", "2"]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[cfg(unix)]
#[test]
fn ssh_mode_rejects_oversized_raw_tar_without_printing_content() {
    let fixture = Fixture::new();
    let oversized = vec![b'x'; clip_context::remote::MAX_REMOTE_ARCHIVE_BYTES + 1];
    let remote_dir = fixture.path().join("remote");
    fs::create_dir_all(&remote_dir).expect("remote directory");
    fs::write(remote_dir.join("huge.txt"), oversized).expect("remote fixture");
    prepare_mock_ssh(&fixture);

    let output = run_mock_ssh(&fixture, &[]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[cfg(unix)]
#[test]
fn ssh_mode_preserves_profile_rules_using_local_config_files() {
    let fixture = Fixture::new();
    fixture.file("remote/src/main.rs", "backend Rust source");
    fixture.file("remote/src/app.py", "frontend Python source");
    fixture.file(
        "remote/.cpctx.toml",
        "[profiles.rust]\nextensions = ['rs']\n",
    );
    prepare_mock_ssh(&fixture);

    let output = run_mock_ssh(&fixture, &["--profile", "rust"]);
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("Markdown is UTF-8");
    assert!(markdown.contains("backend Rust source"));
    assert!(!markdown.contains("frontend Python source"));
}

#[cfg(unix)]
#[test]
fn server_prefilter_omits_large_generated_cache_before_archive_transfer() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    fixture.file("remote/src/main.rs", "small useful source");
    fixture.file("remote/.gitignore", "private/\n");
    fixture.file("remote/private/ignored.rs", "private content");
    fixture.file(
        "remote/.cpctx.toml",
        "[profiles.rust]\nextensions = ['rs']\n",
    );
    let big_directory = fixture.path().join("remote/node_modules/package");
    fs::create_dir_all(&big_directory).expect("generated dependencies");
    fs::write(
        big_directory.join("huge.js"),
        vec![b'x'; clip_context::remote::MAX_REMOTE_ARCHIVE_BYTES + 1024],
    )
    .expect("oversized fixture");

    // The mock SSH command executes the exact POSIX tar command emitted by
    // cpctx against a local directory; no real SSH server is needed.
    fixture.file(
        "tools/ssh",
        "#!/bin/sh\nfor command do :; done\nexec /bin/sh -c \"$command\"\n",
    );
    let executable = fixture.path().join("tools/ssh");
    let permissions = fs::Permissions::from_mode(0o755);
    fs::set_permissions(executable, permissions).expect("mock SSH should be executable");

    let existing_path = std::env::var_os("PATH").expect("test PATH must exist");
    let path = std::env::join_paths(
        std::iter::once(fixture.path().join("tools")).chain(std::env::split_paths(&existing_path)),
    )
    .expect("test PATH is valid");
    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-s")
        .arg(fixture.path().join("remote"))
        .args(["--ssh", "test@server", "--profile", "rust"])
        .env("PATH", path)
        .output()
        .expect("CLI should run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = String::from_utf8(output.stdout).expect("UTF-8 context");
    assert!(body.contains("small useful source"));
    assert!(!body.contains("private content"));
    assert!(!body.contains("huge.js"));
}

#[cfg(unix)]
#[test]
fn ssh_deadline_covers_a_stalled_archive_stream() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    let fixture = Fixture::new();
    fixture.file("tools/ssh", "#!/bin/sh\nexec /bin/sleep 20\n");
    fs::set_permissions(
        fixture.path().join("tools/ssh"),
        fs::Permissions::from_mode(0o755),
    )
    .expect("executable");
    let started = Instant::now();
    let result = run_mock_ssh(&fixture, &["--ssh-timeout", "1"]);
    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&result.stderr).contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn ssh_deadline_also_covers_process_completion_after_stdout_closes() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    let fixture = Fixture::new();
    fixture.file("tools/ssh", "#!/bin/sh\nexec 1>&-\nexec /bin/sleep 20\n");
    fs::set_permissions(
        fixture.path().join("tools/ssh"),
        fs::Permissions::from_mode(0o755),
    )
    .expect("executable");
    let started = Instant::now();
    let result = run_mock_ssh(&fixture, &["--ssh-timeout", "1"]);
    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&result.stderr).contains("process completion timed out"));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn ssh_failure_categories_do_not_echo_remote_stderr() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    for (status, expected) in [
        (255, "host keys, authentication and network"),
        (2, "directory permissions and tar"),
    ] {
        fixture.file(
            "tools/ssh",
            &format!("#!/bin/sh\nprintf 'REMOTE_PRIVATE_MARKER' >&2\nexit {status}\n"),
        );
        fs::set_permissions(
            fixture.path().join("tools/ssh"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("executable");
        let result = run_mock_ssh(&fixture, &[]);
        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
        let diagnostic = String::from_utf8_lossy(&result.stderr);
        assert!(diagnostic.contains(expected));
        assert!(!diagnostic.contains("REMOTE_PRIVATE_MARKER"));
    }
}

#[test]
fn invalid_transport_deadlines_are_rejected_before_starting_ssh() {
    use clip_context::remote::{RemoteOptions, collect_remote_with_options};
    use clip_context::snapshot::SnapshotLimits;
    use std::path::Path;
    use std::time::Duration;
    for timeout in [Duration::ZERO, Duration::MAX] {
        let result = collect_remote_with_options(
            "test@host",
            Path::new("/project"),
            &SnapshotLimits::default(),
            &RemoteOptions {
                timeout,
                ..RemoteOptions::default()
            },
            |_| panic!("must not resolve selection"),
        );
        assert_eq!(
            result.expect_err("invalid timeout").kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}

#[test]
fn timeout_cli_requires_positive_seconds_and_an_ssh_destination() {
    use std::process::Command;
    for flags in [
        vec!["--ssh-timeout", "1"],
        vec!["--ssh", "host", "--ssh-timeout", "0"],
        vec!["--ssh", "host", "--ssh-timeout", "invalid"],
        vec!["--ssh", "host", "--ssh-timeout", "1", "--ssh-timeout", "2"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
            .args(["-c", "/project"])
            .args(flags)
            .output()
            .expect("CLI");
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--ssh-timeout"));
    }
}
