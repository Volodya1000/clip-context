//! Real Git repository selection across history, index and worktree changes.

mod support;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use clip_context::snapshot::SelectionOptions;
use support::Fixture;

fn git(fixture: &Fixture, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(fixture.path())
        .args(arguments)
        .output()
        .expect("Git must be installed");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repository() -> Fixture {
    let fixture = Fixture::new();
    git(&fixture, &["init", "-q"]);
    git(&fixture, &["config", "user.name", "cpctx test"]);
    git(&fixture, &["config", "user.email", "test@example.invalid"]);
    fixture.file("changed.rs", "original");
    fixture.file("unchanged.rs", "unchanged marker");
    fixture.file("deleted.rs", "deleted marker");
    fixture.file("old.rs", "rename marker");
    fixture.file("nested/пример с пробелами.rs", "original nested");
    git(&fixture, &["add", "."]);
    git(&fixture, &["commit", "-qm", "test: initial snapshot"]);
    fixture
}

fn run(fixture: &Fixture, mode: &str, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg(mode)
        .arg(fixture.path())
        .args(arguments)
        .output()
        .expect("CLI must run")
}

#[test]
fn current_changed_context_contains_tracked_worktree_and_index_changes_only() {
    let fixture = repository();
    fixture.file("changed.rs", "modified marker");
    fixture.file("added.rs", "added marker");
    fixture.file("untracked.rs", "untracked marker");
    fixture.file("nested/пример с пробелами.rs", "modified nested marker");
    git(&fixture, &["add", "added.rs"]);
    git(&fixture, &["mv", "old.rs", "renamed.rs"]);
    fs::remove_file(fixture.path().join("deleted.rs")).expect("remove file");
    let output = run(&fixture, "-s", &["--changed-since", "HEAD"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let context = String::from_utf8(output.stdout).expect("UTF-8 output");
    for marker in [
        "modified marker",
        "added marker",
        "modified nested marker",
        "rename marker",
    ] {
        assert!(context.contains(marker), "missing {marker}");
    }
    for marker in ["unchanged marker", "deleted marker", "untracked marker"] {
        assert!(!context.contains(marker));
    }
    assert!(context.contains("renamed.rs"));
}

#[test]
fn history_revision_subdirectories_globs_and_preview_share_selection() {
    let fixture = repository();
    git(&fixture, &["tag", "baseline"]);
    fixture.file("nested/пример с пробелами.rs", "committed marker");
    fixture.file("changed.rs", "committed outside marker");
    git(&fixture, &["add", "."]);
    git(&fixture, &["commit", "-qm", "test: update sources"]);
    let paths = clip_context::git::changed_paths(&fixture.path().join("nested"), "baseline")
        .expect("changed paths");
    assert_eq!(paths, [PathBuf::from("пример с пробелами.rs")]);
    let output = run(
        &fixture,
        "-s",
        &["--changed-since", "baseline", "--include", "nested/**"],
    );
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("committed marker"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("outside marker"));
    let preview = run(&fixture, "--explain", &["--changed-since", "baseline"]);
    assert!(preview.status.success());
    assert!(
        String::from_utf8_lossy(&preview.stdout)
            .contains("unchanged tracked path or untracked file")
    );
}

#[test]
fn no_changes_is_empty_and_does_not_scan_unchanged_sources() {
    let fixture = repository();
    let token = format!("ghp_{}", "X".repeat(36));
    fixture.file("unchanged.rs", &token);
    git(&fixture, &["add", "."]);
    git(
        &fixture,
        &["commit", "-qm", "test: synthetic sensitive fixture"],
    );
    let output = run(&fixture, "-s", &["--changed-since", "HEAD"]);
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(&token));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("unchanged.rs"));
    fixture.file("changed.rs", &token);
    let rejected = run(&fixture, "-s", &["--changed-since", "HEAD"]);
    assert!(!rejected.status.success());
    assert_eq!(rejected.stdout, Vec::<u8>::new());
    assert!(!String::from_utf8_lossy(&rejected.stderr).contains(&token));
}

#[test]
fn invalid_revisions_duplicates_remote_mode_and_nonrepositories_fail_atomically() {
    let fixture = repository();
    for arguments in [
        vec!["--changed-since", "--help"],
        vec!["--changed-since", ""],
        vec!["--changed-since", "missing-revision"],
        vec!["--changed-since", "HEAD", "--changed-since", "HEAD"],
        vec!["--changed-since", "HEAD", "--ssh", "test@host"],
        vec!["--changed-since", "a\nb"],
    ] {
        let result = run(&fixture, "-c", &arguments);
        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
    }
    let plain = Fixture::new();
    let result = run(&plain, "-s", &["--changed-since", "HEAD"]);
    assert!(!result.status.success());
    assert_eq!(result.stdout, Vec::<u8>::new());
    let mut options = SelectionOptions::default();
    for path in ["", "../escape.rs", "/absolute.rs"] {
        assert!(options.restrict_to_paths([PathBuf::from(path)]).is_err());
    }
}

#[cfg(unix)]
#[test]
fn nul_delimited_git_names_preserve_newlines_without_splitting_paths() {
    let fixture = repository();
    fixture.file("line\nbreak.rs", "source");
    git(&fixture, &["add", "."]);
    let paths = clip_context::git::changed_paths(fixture.path(), "HEAD").expect("Git paths");
    assert_eq!(paths, [PathBuf::from("line\nbreak.rs")]);
}
