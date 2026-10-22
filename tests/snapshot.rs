//! Integration tests for real filesystem project snapshots.

mod support;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use clip_context::snapshot::{SelectionOptions, collect_project, collect_project_with_options};
use support::Fixture;

#[test]
fn collects_known_source_extensions_and_preserves_content() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "fn main() {}\n");
    fixture.file("docs/guide.md", "# Guide");
    fixture.file("assets/logo.png", "not source code");
    fixture.file("LICENSE", "license text");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    let paths: Vec<_> = snapshot
        .files()
        .iter()
        .map(|file| file.relative_path().to_path_buf())
        .collect();

    assert_eq!(
        paths,
        [
            Path::new("docs").join("guide.md"),
            Path::new("src").join("main.rs"),
        ]
    );
    assert_eq!(snapshot.files()[1].content(), "fn main() {}\n");
}

#[test]
fn excludes_hidden_directories_and_files() {
    let fixture = Fixture::new();
    fixture.file(".env", "TOKEN=secret");
    fixture.file(".github/workflows/ci.yml", "secret");
    fixture.file("src/.internal.py", "secret");
    fixture.file("src/app.py", "print('safe')");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "print('safe')");
}

#[test]
fn omits_common_generated_directories_without_losing_underscored_sources() {
    let fixture = Fixture::new();
    for directory in [
        "target",
        "dist",
        "build",
        "venv",
        "node_modules",
        "__pycache__",
    ] {
        fixture.file(&format!("{directory}/unwanted.py"), "not included");
    }
    fixture.file("_domain/model.py", "model");
    fixture.file("TARGET/ignored.py", "excluded too");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "model");
}

#[test]
fn sorts_files_consistently_and_builds_a_nested_tree() {
    let fixture = Fixture::new();
    fixture.file("src/z.py", "z");
    fixture.file("src/a.py", "a");
    fixture.file("readme.md", "readme");
    fixture.file("src/deep/x.rs", "x");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    let names: Vec<_> = snapshot
        .files()
        .iter()
        .map(|file| file.relative_path().to_path_buf())
        .collect();
    assert_eq!(
        names,
        [
            Path::new("readme.md").to_path_buf(),
            Path::new("src").join("a.py"),
            Path::new("src").join("deep").join("x.rs"),
            Path::new("src").join("z.py"),
        ]
    );

    let tree = snapshot.tree();
    assert!(tree.contains("src/"));
    assert!(tree.contains("deep/"));
    assert!(tree.contains("x.rs"));
    assert!(tree.contains("├── "));
    assert!(tree.contains("└── "));
}

#[test]
fn markdown_includes_every_selected_file_once() {
    let fixture = Fixture::new();
    fixture.file("a.py", "alpha");
    fixture.file("b.rs", "beta");

    let text = collect_project(fixture.path())
        .expect("the project should be readable")
        .to_markdown();

    assert_eq!(text.matches("### File: a.py").count(), 1);
    assert_eq!(text.matches("### File: b.rs").count(), 1);
    assert!(text.contains("alpha"));
    assert!(text.contains("beta"));
    assert!(text.contains("## File tree"));
}

#[test]
fn markdown_uses_safe_fences_for_source_containing_backticks() {
    let fixture = Fixture::new();
    fixture.file("example.md", "Before\n`````\nAfter\n");

    let text = collect_project(fixture.path())
        .expect("the project should be readable")
        .to_markdown();

    assert!(text.contains("``````\nBefore"));
    assert!(text.contains("After\n``````"));
}

#[test]
fn markdown_adds_separator_for_non_newline_terminated_file() {
    let fixture = Fixture::new();
    fixture.file("main.py", "print(1)");

    let text = collect_project(fixture.path())
        .expect("the project should be readable")
        .to_markdown();

    assert!(text.contains("print(1)\n```"));
}

#[test]
fn empty_project_produces_a_valid_empty_snapshot() {
    let fixture = Fixture::new();
    fixture.file("image.png", "binary placeholder");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    assert_eq!(snapshot.files(), &[]);
    assert!(snapshot.tree().ends_with("/\n"));
    assert!(snapshot.to_markdown().contains("## File tree"));
}

#[test]
fn extension_matching_is_case_insensitive() {
    let fixture = Fixture::new();
    fixture.file("SOURCE.PY", "python");
    fixture.file("component.RS", "rust");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    assert_eq!(snapshot.files().len(), 2);
}

#[test]
fn nonexistent_root_is_rejected() {
    let fixture = Fixture::new();
    let missing = fixture.path().join("missing");

    let error = collect_project(&missing).expect_err("missing project must fail");
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[test]
fn file_instead_of_directory_is_rejected() {
    let fixture = Fixture::new();
    fixture.file("source.py", "python");

    let error = collect_project(&fixture.path().join("source.py"))
        .expect_err("single file is not a project root");
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[test]
fn invalid_utf8_in_included_file_aborts_the_entire_snapshot() {
    let fixture = Fixture::new();
    fixture.file("good.py", "fine");
    fs::write(fixture.path().join("bad.rs"), [0xff, 0xfe])
        .expect("fixture must support writing bytes");

    let error = collect_project(fixture.path()).expect_err("malformed text must fail");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error.to_string().contains("bad.rs"));
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_not_followed() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    fixture.file("source.py", "safe");
    symlink(
        fixture.path().join("source.py"),
        fixture.path().join("shortcut.py"),
    )
    .expect("test symlink must be created");
    symlink(fixture.path(), fixture.path().join("loop")).expect("test symlink must be created");

    let snapshot = collect_project(fixture.path()).expect("the project should be readable");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "safe");
}

#[test]
fn selecting_specific_extensions_does_not_disable_builtin_exclusions() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "keep-source");
    fixture.file("src/helper.py", "not-selected");
    fixture.file("target/generated.rs", "generated-source");
    fixture.file(".hidden.rs", "hidden-source");

    let mut options = SelectionOptions::default();
    options.include_extension("rs");
    let result =
        collect_project_with_options(fixture.path(), &options).expect("selection should work");

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.files()[0].content(), "keep-source");
}

#[test]
fn extra_directory_exclusions_preserve_nested_source_files_elsewhere() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "keep-source");
    fixture.file("generated/unwanted.rs", "skip-one");
    fixture.file("src/generated/another.rs", "skip-two");

    let mut options = SelectionOptions::default();
    options.exclude_directory("generated");
    let result =
        collect_project_with_options(fixture.path(), &options).expect("selection should work");

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.files()[0].content(), "keep-source");
}

#[test]
fn respects_project_root_gitignore_patterns_and_negations() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "*.py\n!keep.py\n");
    fixture.file("keep.py", "included");
    fixture.file("drop.py", "excluded");
    fixture.file("source.rs", "source");

    let snapshot = collect_project(fixture.path()).expect("project snapshot succeeds");
    let names: Vec<_> = snapshot
        .files()
        .iter()
        .map(|file| file.relative_path().to_path_buf())
        .collect();
    assert_eq!(names, [Path::new("keep.py"), Path::new("source.rs")]);
}

#[test]
fn nested_gitignore_overrides_parent_patterns() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "*.py\n");
    fixture.file("nested/.gitignore", "!keep.py\n");
    fixture.file("nested/keep.py", "included");
    fixture.file("nested/drop.py", "excluded");
    fixture.file("root.py", "excluded");

    let snapshot = collect_project(fixture.path()).expect("project snapshot succeeds");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "included");
}

#[test]
fn ignored_directories_are_not_traversed() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "private/\n");
    fixture.file("private/secret.rs", "not included");
    fixture.file("public/code.rs", "included");

    let snapshot = collect_project(fixture.path()).expect("project snapshot succeeds");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "included");
}

#[test]
fn gitignore_is_respected_without_git_metadata_directory() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "ignored.rs\n");
    fixture.file("ignored.rs", "excluded");
    fixture.file("kept.rs", "kept");

    let snapshot = collect_project(fixture.path()).expect("standalone project succeeds");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "kept");
}

#[test]
fn hidden_files_remain_excluded_even_when_unignored_by_gitignore() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "!.secret.py\n");
    fixture.file(".secret.py", "hidden");
    fixture.file("visible.py", "included");

    let snapshot = collect_project(fixture.path()).expect("project snapshot succeeds");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "included");
}
#[test]
fn gitignore_and_explicit_extension_filters_apply_together() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "*.rs\n!src/keep.rs\n");
    fixture.file("src/keep.rs", "included");
    fixture.file("src/drop.rs", "excluded");
    fixture.file("src/script.py", "not-selected");

    let mut options = SelectionOptions::default();
    options.include_extension("rs");
    let snapshot = collect_project_with_options(fixture.path(), &options)
        .expect("combined selection should succeed");

    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "included");
}

#[test]
fn custom_directory_exclusions_cannot_be_undone_by_gitignore_negation() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "!generated/keep.rs\n");
    fixture.file("generated/keep.rs", "excluded");
    fixture.file("src/keep.rs", "included");

    let mut options = SelectionOptions::default();
    options.exclude_directory("generated");
    let snapshot =
        collect_project_with_options(fixture.path(), &options).expect("exclusions should apply");

    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "included");
}

#[test]
fn named_profile_can_be_used_through_library_api() {
    use clip_context::profile::load_profile;

    let fixture = Fixture::new();
    fixture.file(
        "selection.toml",
        "[profiles.source]\nextensions = ['RS']\nexclude_directories = ['generated']\n",
    );
    fixture.file("src/lib.rs", "included");
    fixture.file("src/other.py", "other");
    fixture.file("generated/lib.rs", "generated");

    let options = load_profile(&fixture.path().join("selection.toml"), "source")
        .expect("named profile should deserialize");
    let snapshot = collect_project_with_options(fixture.path(), &options)
        .expect("profile should collect files");

    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "included");
}

#[test]
fn unknown_profile_name_has_explicit_invalid_input_error() {
    use clip_context::profile::load_profile;

    let fixture = Fixture::new();
    fixture.file("selection.toml", "[profiles.source]\nextensions = ['rs']\n");
    let error = load_profile(&fixture.path().join("selection.toml"), "missing")
        .expect_err("unknown names must be rejected");
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[test]
fn configuration_errors_are_not_silently_recovered() {
    use clip_context::profile::load_profile;

    let fixture = Fixture::new();
    fixture.file(
        "selection.toml",
        "[profiles.source]\nexclude_directories = [4]\n",
    );
    let error = load_profile(&fixture.path().join("selection.toml"), "source")
        .expect_err("invalid type must fail");
    assert_eq!(error.kind(), ErrorKind::InvalidData);

    let missing = load_profile(&fixture.path().join("missing.toml"), "source")
        .expect_err("missing file must fail");
    assert_eq!(missing.kind(), ErrorKind::NotFound);
}

#[test]
fn suspected_github_token_aborts_snapshot_without_leaking_credentials() {
    use clip_context::snapshot::collect_project;

    let fixture = Fixture::new();
    fixture.file("a.rs", "public code");
    let synthetic_secret = format!("ghp_{}", "Q".repeat(36));
    fixture.file("b.rs", &format!("token = \"{synthetic_secret}\""));

    let error = collect_project(fixture.path()).expect_err("suspected token must stop export");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    assert!(!error.to_string().contains(&synthetic_secret));
}

#[test]
fn private_key_material_is_rejected() {
    let fixture = Fixture::new();
    fixture.file(
        "src/key.txt",
        &format!("-----BEGIN {}-----\nplaceholder\n", "PRIVATE KEY"),
    );
    let error = collect_project(fixture.path()).expect_err("private key must block collection");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
}

#[test]
fn ignored_credentials_do_not_block_safe_selected_files() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "private/\n");
    fixture.file(
        "private/key.txt",
        &format!("-----BEGIN {}-----\nprivate\n", "PRIVATE KEY"),
    );
    fixture.file("src/main.rs", "safe");

    let snapshot = collect_project(fixture.path()).expect("ignored files are not selected");
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].content(), "safe");
}

#[test]
fn cumulative_source_byte_budget_is_enforced() {
    use clip_context::snapshot::{SnapshotLimits, collect_project_with_options_and_limits};
    use std::num::NonZeroUsize;

    let fixture = Fixture::new();
    fixture.file("first.rs", "123456");
    fixture.file("second.rs", "abcdef");
    let mut limits = SnapshotLimits::default();
    limits.set_max_source_bytes(NonZeroUsize::new(10).expect("positive"));

    let error = collect_project_with_options_and_limits(
        fixture.path(),
        &SelectionOptions::default(),
        &limits,
    )
    .expect_err("combined files must exceed byte budget");
    assert_eq!(error.kind(), ErrorKind::FileTooLarge);
}

#[test]
fn exact_byte_budget_succeeds_and_larger_file_fails() {
    use clip_context::snapshot::{SnapshotLimits, collect_project_with_options_and_limits};
    use std::num::NonZeroUsize;

    let fixture = Fixture::new();
    fixture.file("main.rs", "123456789");
    let mut limits = SnapshotLimits::default();
    limits.set_max_source_bytes(NonZeroUsize::new(9).expect("positive"));

    let snapshot = collect_project_with_options_and_limits(
        fixture.path(),
        &SelectionOptions::default(),
        &limits,
    )
    .expect("exact byte boundary is valid");
    assert_eq!(snapshot.files()[0].content(), "123456789");

    limits.set_max_source_bytes(NonZeroUsize::new(8).expect("positive"));
    let error = collect_project_with_options_and_limits(
        fixture.path(),
        &SelectionOptions::default(),
        &limits,
    )
    .expect_err("one byte past budget must fail");
    assert_eq!(error.kind(), ErrorKind::FileTooLarge);
}

#[test]
fn estimated_tokens_enforce_an_independent_byte_based_budget() {
    use clip_context::snapshot::{SnapshotLimits, collect_project_with_options_and_limits};
    use std::num::NonZeroUsize;

    let fixture = Fixture::new();
    fixture.file("main.py", "123456789");
    let mut limits = SnapshotLimits::default();
    limits.set_max_source_bytes(NonZeroUsize::new(100).expect("positive"));
    limits.set_max_estimated_tokens(NonZeroUsize::new(2).expect("positive"));

    let error = collect_project_with_options_and_limits(
        fixture.path(),
        &SelectionOptions::default(),
        &limits,
    )
    .expect_err("estimate should disallow nine bytes at two tokens");
    assert_eq!(error.kind(), ErrorKind::FileTooLarge);

    limits.set_max_estimated_tokens(NonZeroUsize::new(3).expect("positive"));
    let snapshot = collect_project_with_options_and_limits(
        fixture.path(),
        &SelectionOptions::default(),
        &limits,
    )
    .expect("three estimated tokens should accommodate nine bytes");
    assert_eq!(snapshot.files().len(), 1);
}

#[test]
fn empty_source_remains_valid_when_byte_budget_is_exhausted() {
    use clip_context::snapshot::{SnapshotLimits, collect_project_with_options_and_limits};
    use std::num::NonZeroUsize;

    let fixture = Fixture::new();
    fixture.file("a.py", "x");
    fixture.file("b.py", "");
    let mut limits = SnapshotLimits::default();
    limits.set_max_source_bytes(NonZeroUsize::new(1).expect("positive"));

    let snapshot = collect_project_with_options_and_limits(
        fixture.path(),
        &SelectionOptions::default(),
        &limits,
    )
    .expect("an empty second file consumes no extra bytes");
    assert_eq!(snapshot.files().len(), 2);
}
