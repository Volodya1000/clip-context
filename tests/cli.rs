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
