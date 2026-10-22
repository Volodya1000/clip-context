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
    let result = collect_project_with_options(fixture.path(), &options)
        .expect("selection should work");

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
    let result = collect_project_with_options(fixture.path(), &options)
        .expect("selection should work");

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.files()[0].content(), "keep-source");
}
