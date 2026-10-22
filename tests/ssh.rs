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
