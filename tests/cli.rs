//! End-to-end smoke tests of the command-line interface.

use std::process::{Command, Output};

fn execute(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clip-context"))
        .args(arguments)
        .output()
        .expect("the compiled CLI executable must be runnable")
}

#[test]
fn help_is_available() {
    let output = execute(&["--help"]);
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
    assert!(output.stderr.is_empty());
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
fn unexpected_arguments_fail() {
    let output = execute(&["--unknown"]);
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn conflicting_arguments_fail() {
    let output = execute(&["--help", "--version"]);
    assert!(!output.status.success());
}
