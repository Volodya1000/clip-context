//! End-to-end limits on the **rendered output**, including Markdown markup.

mod support;

use std::process::{Command, Output};

use support::Fixture;

fn execute(fixture: &Fixture, mode: &str, flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg(mode)
        .arg(fixture.path())
        .args(flags)
        .output()
        .expect("CLI must run")
}

fn size_argument(size: usize) -> String {
    size.to_string()
}

#[test]
fn final_budget_includes_tree_filenames_and_markdown_overhead() {
    let fixture = Fixture::new();
    fixture.file("src/nested/module.py", "x");
    let full = execute(&fixture, "-s", &[]);
    assert!(full.status.success());
    let limit = full.stdout.len();
    let too_small = size_argument(limit - 1);
    let denied = execute(&fixture, "-s", &["--max-bytes", &too_small]);
    assert!(!denied.status.success());
    assert_eq!(denied.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("rendered context exceeds limit"));
    let exact = size_argument(limit);
    let accepted = execute(&fixture, "-s", &["--max-bytes", &exact]);
    assert!(accepted.status.success());
    assert_eq!(accepted.stdout, full.stdout);
}

#[test]
fn compact_output_can_fit_even_when_full_tree_exceeds_budget() {
    let fixture = Fixture::new();
    for number in 0..8 {
        fixture.file(&format!("src/very/deep/package/mod{number}.rs"), "x");
    }
    let full = execute(&fixture, "-s", &[]);
    let compact = execute(&fixture, "-s", &["--format", "compact"]);
    assert!(full.status.success());
    assert!(compact.status.success());
    assert!(compact.stdout.len() < full.stdout.len());
    let cap = size_argument(compact.stdout.len());
    let fits = execute(
        &fixture,
        "-s",
        &["--format", "compact", "--max-bytes", &cap],
    );
    assert!(fits.status.success());
    let rejects = execute(&fixture, "-s", &["--format", "full", "--max-bytes", &cap]);
    assert!(!rejects.status.success());
    assert_eq!(rejects.stdout, Vec::<u8>::new());
}

#[test]
fn token_budget_applies_to_final_output_including_markers() {
    let fixture = Fixture::new();
    fixture.file("readme.md", "abc");
    let plain = execute(&fixture, "-s", &[]);
    assert!(plain.status.success());
    let estimate = plain.stdout.len().div_ceil(4);
    let low = size_argument(estimate - 1);
    let result = execute(&fixture, "-s", &["--max-estimated-tokens", &low]);
    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
    let exact = size_argument(estimate);
    assert!(
        execute(&fixture, "-s", &["--max-estimated-tokens", &exact])
            .status
            .success()
    );
}

#[test]
fn tree_only_can_report_a_file_larger_than_a_source_budget() {
    let fixture = Fixture::new();
    fixture.file("src/large.rs", &"x".repeat(140_000));
    let output = execute(&fixture, "-t", &["--max-bytes", "250"]);
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("tree is UTF-8");
    assert!(body.contains("large.rs"));
    assert!(!body.contains("xxxx"));
}

#[test]
fn tree_only_budget_counts_just_the_rendered_tree() {
    let fixture = Fixture::new();
    fixture.file("src/readme.rs", "code");
    let plain = execute(&fixture, "-t", &[]);
    assert!(plain.status.success());
    let cap = size_argument(plain.stdout.len() - 1);
    let rejected = execute(&fixture, "-t", &["--max-bytes", &cap]);
    assert!(!rejected.status.success());
    assert_eq!(rejected.stdout, Vec::<u8>::new());
    let exact = size_argument(plain.stdout.len());
    assert!(
        execute(&fixture, "-t", &["--max-bytes", &exact])
            .status
            .success()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn clipboard_budget_failure_never_invokes_backend() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    fixture.file("src/file.rs", "x");
    fixture.file(
        "bin/wl-copy",
        "#!/bin/sh\n/bin/touch \"$CPCTX_CLIPBOARD_MARKER\"\n",
    );
    let bin = fixture.path().join("bin");
    fs::set_permissions(bin.join("wl-copy"), fs::Permissions::from_mode(0o755))
        .expect("mock backend is executable");
    let marker = fixture.path().join("invoked");
    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-c")
        .arg(fixture.path())
        .args(["--max-bytes", "1"])
        .env("PATH", &bin)
        .env("WAYLAND_DISPLAY", "test-wayland")
        .env_remove("DISPLAY")
        .env("CPCTX_CLIPBOARD_MARKER", &marker)
        .output()
        .expect("CLI must run");
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(!marker.exists());
}
