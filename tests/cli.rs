//! End-to-end smoke tests of the command-line interface.

mod support;

use std::process::{Command, Output};

use support::Fixture;

fn execute(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .args(arguments)
        .output()
        .expect("the compiled CLI executable must be runnable")
}

fn execute_on_project(mode: &str, fixture: &Fixture) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg(mode)
        .arg(fixture.path())
        .output()
        .expect("the CLI must be runnable")
}

#[test]
fn help_is_available() {
    let output = execute(&["--help"]);
    assert!(output.status.success());
    assert_ne!(output.stdout, Vec::<u8>::new());
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn no_arguments_displays_usage() {
    let output = execute(&[]);
    assert!(output.status.success());
    assert_ne!(output.stdout, Vec::<u8>::new());
}

#[test]
fn short_help_is_available() {
    assert!(execute(&["-h"]).status.success());
}

#[test]
fn version_is_available() {
    let output = execute(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("version must be UTF-8")
            .trim(),
        format!("clip-context {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn short_version_is_available() {
    assert!(execute(&["-V"]).status.success());
}

#[test]
fn unexpected_arguments_fail() {
    let output = execute(&["--unknown"]);
    assert!(!output.status.success());
    assert_ne!(output.stderr, Vec::<u8>::new());
}

#[test]
fn conflicting_arguments_fail() {
    assert!(!execute(&["--help", "--version"]).status.success());
}

#[test]
fn stdout_command_prints_project_context() {
    let fixture = Fixture::new();
    fixture.file("code.py", "print('hello')");
    fixture.file("ignored.png", "binary");

    let output = execute_on_project("--stdout", &fixture);
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    let body = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    assert!(body.contains("code.py"));
    assert!(body.contains("print('hello')"));
    assert!(!body.contains("ignored.png"));
}

#[test]
fn tree_command_prints_only_filenames() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "super_secret_test_content");
    let output = execute_on_project("--tree", &fixture);
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    assert!(body.contains("main.rs"));
    assert!(!body.contains("super_secret_test_content"));
}

#[test]
fn short_stdout_option_matches_long_option() {
    let fixture = Fixture::new();
    fixture.file("src/lib.rs", "pub fn result() -> bool { true }");

    let short = execute_on_project("-s", &fixture);
    let long = execute_on_project("--stdout", &fixture);
    assert!(short.status.success());
    assert_eq!(short.stdout, long.stdout);
    assert_eq!(short.stderr, long.stderr);
}

#[test]
fn short_tree_option_matches_long_option() {
    let fixture = Fixture::new();
    fixture.file("src/lib.rs", "pub fn result() -> bool { true }");

    let short = execute_on_project("-t", &fixture);
    let long = execute_on_project("--tree", &fixture);
    assert!(short.status.success());
    assert_eq!(short.stdout, long.stdout);
    assert_eq!(short.stderr, long.stderr);
}

#[test]
fn missing_project_path_does_not_produce_partial_stdout() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("--stdout")
        .arg(fixture.path().join("missing"))
        .output()
        .expect("the CLI must be runnable");
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_ne!(output.stderr, Vec::<u8>::new());
}

#[test]
fn invalid_project_path_fails_for_tree_mode_too() {
    let fixture = Fixture::new();
    fixture.file("single.py", "content");
    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("--tree")
        .arg(fixture.path().join("single.py"))
        .output()
        .expect("the CLI must be runnable");
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn unknown_extra_arguments_fail() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("--stdout")
        .arg(fixture.path())
        .arg("--bad")
        .output()
        .expect("the CLI must be runnable");
    assert!(!output.status.success());
}

#[test]
fn a_file_with_invalid_utf8_produces_no_output() {
    let fixture = Fixture::new();
    fixture.file("a.py", "valid");
    std::fs::write(fixture.path().join("b.rs"), [0xff]).expect("fixture file must be writable");

    let output = execute_on_project("--stdout", &fixture);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn repeated_extensions_select_only_requested_types() {
    let fixture = Fixture::new();
    fixture.file("lib.rs", "rust-source");
    fixture.file("script.py", "python-source");
    fixture.file("config.toml", "toml-source");

    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-s")
        .arg(fixture.path())
        .args(["--ext", "rs", "--ext", "TOML"])
        .output()
        .expect("CLI executable must be runnable");

    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("Markdown is UTF-8");
    assert!(markdown.contains("rust-source"));
    assert!(markdown.contains("toml-source"));
    assert!(!markdown.contains("python-source"));
}

#[test]
fn additional_exclusions_apply_at_any_depth() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "keep-this");
    fixture.file("generated/first.rs", "not-included");
    fixture.file("src/generated/second.rs", "also-excluded");

    let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-s")
        .arg(fixture.path())
        .args(["--exclude-dir", "generated"])
        .output()
        .expect("CLI executable must be runnable");

    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("Markdown is UTF-8");
    assert!(markdown.contains("keep-this"));
    assert!(!markdown.contains("not-included"));
    assert!(!markdown.contains("also-excluded"));
}

#[test]
fn invalid_selection_flags_produce_no_snapshot_output() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "readable-source");

    for flags in [
        vec!["--ext"],
        vec!["--ext", "../rs"],
        vec!["--exclude-dir", "../outside"],
        vec!["--exclude-dir", ""],
        vec!["--unknown", "value"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cpctx"))
            .arg("--stdout")
            .arg(fixture.path())
            .args(flags)
            .output()
            .expect("CLI executable must be runnable");
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_ne!(output.stderr, Vec::<u8>::new());
    }
}

#[test]
fn named_profile_selects_matching_files_and_excludes_directories() {
    let fixture = Fixture::new();
    fixture.file(
        ".cpctx.toml",
        "[profiles.backend]\nextensions = ['rs']\nexclude_directories = ['generated']\n",
    );
    fixture.file("src/main.rs", "included-rust");
    fixture.file("src/helper.py", "excluded-python");
    fixture.file("generated/model.rs", "excluded-generated");

    let output = execute_with_flags(&fixture, &["--profile", "backend"]);
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("snapshot must be UTF-8");
    assert!(body.contains("included-rust"));
    assert!(!body.contains("excluded-python"));
    assert!(!body.contains("excluded-generated"));
}

#[test]
fn cli_extensions_override_profile_while_directory_exclusions_accumulate() {
    let fixture = Fixture::new();
    fixture.file(
        ".cpctx.toml",
        "[profiles.backend]\nextensions = ['rs']\nexclude_directories = ['generated']\n",
    );
    fixture.file(".gitignore", "private/\n");
    fixture.file("src/main.rs", "excluded-rust");
    fixture.file("src/lib.py", "included-python");
    fixture.file("generated/lib.py", "excluded-profile");
    fixture.file("drafts/lib.py", "excluded-cli");
    fixture.file("private/lib.py", "excluded-gitignore");

    let output = execute_with_flags(
        &fixture,
        &[
            "--profile",
            "backend",
            "--ext",
            "py",
            "--exclude-dir",
            "drafts",
        ],
    );
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("snapshot must be UTF-8");
    assert!(body.contains("included-python"));
    for excluded in [
        "excluded-rust",
        "excluded-profile",
        "excluded-cli",
        "excluded-gitignore",
    ] {
        assert!(!body.contains(excluded));
    }
}

#[test]
fn explicit_config_is_resolved_against_the_project_directory() {
    let fixture = Fixture::new();
    fixture.file(
        "config/profiles.toml",
        "[profiles.docs]\nextensions = ['md']\n",
    );
    fixture.file("README.md", "documentation");
    fixture.file("src/lib.rs", "implementation");

    let output = execute_with_flags(
        &fixture,
        &["--profile", "docs", "--config", "config/profiles.toml"],
    );
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("snapshot must be UTF-8");
    assert!(body.contains("documentation"));
    assert!(!body.contains("implementation"));
}

#[test]
fn invalid_or_missing_profile_is_rejected_before_output() {
    let fixture = Fixture::new();
    fixture.file(".cpctx.toml", "[profiles.docs]\nextensions = ['md']\n");
    fixture.file("README.md", "should-not-be-emitted");

    let invalid_sets: &[&[&str]] = &[
        &["--profile", "missing"],
        &["--profile", "docs", "--profile", "docs"],
        &["--profile", "docs", "--config", "missing.toml"],
        &["--config", ".cpctx.toml"],
        &["--profile", "../outside"],
        &["--profile"],
    ];
    for arguments in invalid_sets {
        let output = execute_with_flags(&fixture, arguments);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_ne!(output.stderr, Vec::<u8>::new());
    }
}

#[test]
fn malformed_and_unsafe_profile_documents_are_rejected_without_output() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "not-for-output");
    let bad_documents = [
        "[profiles.backend]\nextensions = 'rs'\n",
        "[profiles.backend]\nextensions = ['rs']\nunknown = true\n",
        "[profiles.backend]\nextensions = ['../rs']\n",
        "[profiles.backend]\nexclude_directories = ['../outside']\n",
        "[profiles.backend]\nexclude_directories = ['.']\n",
        "[profiles.backend\n",
    ];

    for document in bad_documents {
        fixture.file(".cpctx.toml", document);
        let output = execute_with_flags(&fixture, &["--profile", "backend"]);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_ne!(output.stderr, Vec::<u8>::new());
    }
}

#[test]
fn profile_configuration_is_opt_in() {
    let fixture = Fixture::new();
    fixture.file(".cpctx.toml", "this is not TOML");
    fixture.file("src/main.rs", "included-default");

    let output = execute_on_project("-s", &fixture);
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("snapshot must be UTF-8");
    assert!(body.contains("included-default"));
}

fn execute_with_flags(fixture: &Fixture, flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-s")
        .arg(fixture.path())
        .args(flags)
        .output()
        .expect("CLI executable must be runnable")
}
