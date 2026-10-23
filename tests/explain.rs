//! Metadata-only previews, exclusion reasons, budget guidance and resource bounds.

mod support;

use std::fs;
use std::process::{Command, Output};

use clip_context::snapshot::{
    SelectionOptions, SnapshotLimits, collect_project_tree, explain_project,
};
use support::Fixture;

fn run(fixture: &Fixture, mode: &str, options: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg(mode)
        .arg(fixture.path())
        .args(options)
        .output()
        .expect("CLI must run")
}

#[test]
fn reports_metadata_and_reasons_without_reading_or_exporting_source() {
    let fixture = Fixture::new();
    fixture.file("main.rs", "PRIVATE_SOURCE_MARKER");
    fixture.file(".gitignore", "private/\n");
    fixture.file("private/hidden.rs", "private marker");
    fixture.file(".hidden/file.rs", "hidden marker");
    fixture.file("target/generated.rs", "generated marker");
    fixture.file("image.png", "binary marker");
    fixture.file(".env", "credential marker");
    let result = run(&fixture, "--explain", &[]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).expect("UTF-8 report");
    for expected in [
        "21 source bytes",
        "gitignore rule",
        "hidden path",
        "excluded directory",
        "extension selection",
        "protected credential",
        "secrets not checked",
        "Largest selected",
    ] {
        assert!(report.contains(expected), "missing {expected}");
    }
    assert!(!report.contains("PRIVATE_SOURCE_MARKER"));
    assert!(!report.contains("private marker"));
    assert!(!report.contains("private/hidden.rs"));
    assert_eq!(result.stderr, Vec::<u8>::new());
}

#[test]
fn dry_run_is_an_alias_and_source_budget_excess_is_guidance_not_an_export() {
    let fixture = Fixture::new();
    fixture.file("large.rs", &"x".repeat(1024));
    let flags = ["--max-bytes", "1", "--max-estimated-tokens", "1"];
    let explain = run(&fixture, "--explain", &flags);
    let dry_run = run(&fixture, "--dry-run", &flags);
    assert!(explain.status.success());
    assert_eq!(explain.stdout, dry_run.stdout);
    assert!(String::from_utf8_lossy(&explain.stdout).contains("EXCEEDS"));
    assert!(!String::from_utf8_lossy(&explain.stdout).contains(&"x".repeat(1024)));
}

#[test]
fn nested_ignore_negations_and_globs_agree_with_the_export_selection() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "*.rs\n!keep.rs\n");
    fixture.file("src/.gitignore", "!chosen.rs\n");
    fixture.file("src/chosen.rs", "chosen source");
    fixture.file("src/ignored.rs", "ignored source");
    fixture.file("keep.rs", "kept source");
    fixture.file("Dockerfile", "container source");
    let mut options = SelectionOptions::default();
    options.include_pattern("**/*.rs").expect("glob");
    options.include_pattern("Dockerfile").expect("glob");
    options.exclude_pattern("keep.rs").expect("glob");
    let snapshot = collect_project_tree(fixture.path(), &options).expect("snapshot");
    let report =
        explain_project(fixture.path(), &options, &SnapshotLimits::default()).expect("report");
    assert_eq!(snapshot.files().len(), 2);
    for file in snapshot.files() {
        assert!(report.contains(&format!(
                "INCLUDE {} bytes {}",
                fs::metadata(fixture.path().join(file.relative_path()))
                    .expect("metadata")
                    .len(),
                file.relative_path().to_string_lossy().escape_debug()
            )));
    }
    assert!(report.contains("path glob selection"));
}

#[test]
fn binary_and_secret_source_can_be_inspected_without_claiming_secret_validation() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("binary.rs"), [0xff, 0xfe]).expect("binary source");
    let token = format!("ghp_{}", "X".repeat(36));
    fixture.file("secret.rs", &token);
    let result = run(&fixture, "--dry-run", &[]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).expect("report");
    assert!(report.contains("2 source") || report.contains("2 files"));
    assert!(!report.contains(&token));
    assert!(report.contains("secrets not checked"));
}

#[test]
fn empty_project_and_profile_preview_have_deterministic_output() {
    let fixture = Fixture::new();
    let empty = run(&fixture, "--explain", &[]);
    assert!(empty.status.success());
    assert!(String::from_utf8_lossy(&empty.stdout).contains("0 files, 0 source bytes"));
    fixture.file(
        ".cpctx.toml",
        "[profiles.config]\ninclude = ['Dockerfile']\n",
    );
    fixture.file("Dockerfile", "config");
    fixture.file("main.rs", "source");
    let first = run(&fixture, "--explain", &["--profile", "config"]);
    let second = run(&fixture, "--explain", &["--profile", "config"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(String::from_utf8_lossy(&first.stdout).contains("1 files, 6 source bytes"));
}

#[test]
fn unsupported_remote_invalid_root_and_bad_configuration_return_no_report() {
    let fixture = Fixture::new();
    for flags in [vec!["--ssh", "test@host"], vec!["--profile", "missing"]] {
        let result = run(&fixture, "--explain", &flags);
        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
    }
    let result = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("--explain")
        .arg(fixture.path().join("missing"))
        .output()
        .expect("CLI");
    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn excessive_metadata_entries_fail_without_partial_report() {
    let fixture = Fixture::new();
    for index in 0..10_001 {
        fs::create_dir(fixture.path().join(format!("directory{index}"))).expect("directory");
    }
    let result = run(&fixture, "--explain", &[]);
    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&result.stderr).contains("10000 entries"));
}

#[cfg(unix)]
#[test]
fn symlinks_and_control_characters_in_names_do_not_export_or_spoof_contents() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.file("main.rs", "source");
    fixture.file("line\nbreak.rs", "secret marker");
    symlink(
        fixture.path().join("main.rs"),
        fixture.path().join("link.rs"),
    )
    .expect("symlink");
    let result = run(&fixture, "--explain", &[]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).expect("report");
    assert!(report.contains("symbolic link"));
    assert!(report.contains("line\\nbreak.rs"));
    assert!(!report.contains("line\nbreak.rs"));
    assert!(!report.contains("secret marker"));
}
