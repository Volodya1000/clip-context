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
fn help_is_a_complete_bounded_terminal_reference() {
    let long = execute(&["--help"]);
    let short = execute(&["-h"]);
    assert!(long.status.success());
    assert_eq!(long.stdout, short.stdout);
    assert_eq!(long.stderr, Vec::<u8>::new());
    assert_eq!(short.stderr, Vec::<u8>::new());

    let reference = String::from_utf8(long.stdout).expect("help must be UTF-8");
    for heading in [
        "Usage:",
        "Output modes:",
        "Output formats:",
        "Selection:",
        "Profiles:",
        "Safety limits:",
        "Remote SSH:",
        "Clipboard:",
        "Diagnostics:",
        "Examples:",
    ] {
        assert!(reference.contains(heading), "missing heading: {heading}");
    }
    for option in [
        "-s",
        "--stdout",
        "-t",
        "--tree",
        "-c",
        "--clipboard",
        "-h",
        "--help",
        "-V",
        "--version",
        "--format",
        "--ext",
        "--exclude-dir",
        "--profile",
        "--config",
        "--ssh",
        "--max-bytes",
        "--max-estimated-tokens",
    ] {
        assert!(reference.contains(option), "missing option: {option}");
    }
    for detail in [
        "262144",
        "32000",
        ".gitignore",
        "stderr",
        "stdout",
        "16 MiB",
        "extensions = ",
        "exclude_directories = ",
        "repeatable",
        "remote",
        "cpctx -c",
        "cpctx -s",
        "cpctx -t",
        "replaces",
        "adds exclusions",
    ] {
        assert!(reference.contains(detail), "missing help detail: {detail}");
    }
    for line in reference.lines() {
        assert!(line.chars().count() <= 100, "overwide help line: {line}");
    }
}

#[test]
fn invalid_cli_arguments_have_actionable_diagnostics_and_no_stdout() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "never print this");

    let cases: &[(&[&str], &str)] = &[
        (&["--ext", "../rs"], "--ext requires"),
        (&["--exclude-dir", "../src"], "--exclude-dir requires"),
        (&["--profile", "../bad"], "--profile requires"),
        (&["--profile", "ok", "--profile", "ok"], "--profile may"),
        (&["--config", ""], "--config requires"),
        (
            &["--profile", "ok", "--config", "a", "--config", "b"],
            "--config may",
        ),
        (&["--ssh", ""], "--ssh requires"),
        (&["--ssh", "host", "--ssh", "other"], "--ssh may"),
        (&["--format", "unknown"], "--format must"),
        (&["--max-bytes", "0"], "--max-bytes must"),
        (
            &["--max-estimated-tokens", "0"],
            "--max-estimated-tokens must",
        ),
        (&["--unknown", "value"], "Unknown option"),
        (&["--config", "file.toml"], "--config requires --profile"),
        (&["--ext"], "Missing value for --ext"),
    ];
    for (arguments, diagnostic) in cases {
        let output = execute_with_flags(&fixture, arguments);
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert_eq!(output.stdout, Vec::<u8>::new(), "arguments: {arguments:?}");
        let error = String::from_utf8(output.stderr).expect("diagnostic must be UTF-8");
        assert!(error.contains(diagnostic), "unexpected diagnostic: {error}");
        assert!(
            error.contains("Run cpctx --help."),
            "missing help hint: {error}"
        );
        assert!(!error.contains("never print this"));
    }
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
    let stats = String::from_utf8_lossy(&output.stderr);
    assert!(stats.contains("Context: "));
    assert!(stats.contains("tokens (estimated)"));
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

#[test]
fn recognized_credentials_do_not_appear_in_stdout_or_stderr() {
    let fixture = Fixture::new();
    fixture.file("public.rs", "safe");
    let token = format!("ghp_{}", "Q".repeat(36));
    fixture.file("private.rs", &format!("token = \"{token}\""));

    let output = execute_with_flags(&fixture, &[]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let errors = String::from_utf8(output.stderr).expect("error must be UTF-8");
    assert!(!errors.contains(&token));
    assert!(errors.contains("access token"));
}

#[test]
fn tree_output_also_refuses_projects_containing_private_keys() {
    let fixture = Fixture::new();
    let private_key_label = ["PRIVATE", "KEY"].join(" ");
    fixture.file(
        "private.txt",
        &format!("-----BEGIN {private_key_label}-----\n"),
    );
    let output = execute_on_project("-t", &fixture);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn explicit_byte_budget_prevents_output_when_project_is_oversized() {
    let fixture = Fixture::new();
    fixture.file("main.py", "more than three bytes");
    let output = execute_with_flags(&fixture, &["--max-bytes", "3"]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn estimated_token_budget_prevents_output_when_exceeded() {
    let fixture = Fixture::new();
    fixture.file("main.py", "123456789");
    let output = execute_with_flags(&fixture, &["--max-estimated-tokens", "2"]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn explicitly_raised_budgets_allow_an_ordinary_project() {
    let fixture = Fixture::new();
    fixture.file("main.py", "print('ready')");
    let output = execute_with_flags(
        &fixture,
        &["--max-bytes", "200", "--max-estimated-tokens", "100"],
    );
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout is UTF-8")
            .contains("print('ready')")
    );
}

#[test]
fn invalid_budget_arguments_fail_before_emitting_any_content() {
    let fixture = Fixture::new();
    fixture.file("main.py", "safe");
    for flags in [
        vec!["--max-bytes", "0"],
        vec!["--max-bytes", "not_a_number"],
        vec!["--max-bytes"],
        vec!["--max-estimated-tokens", "0"],
        vec!["--max-estimated-tokens", "-1"],
    ] {
        let output = execute_with_flags(&fixture, &flags);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
}

#[test]
fn secret_in_excluded_directory_does_not_block_export() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "safe");
    let synthetic_header = ["-----", "BEGIN ", "PRIVATE ", "KEY-----", "\n"].concat();
    fixture.file("secrets/keys.rs", &synthetic_header);
    let output = execute_with_flags(&fixture, &["--exclude-dir", "secrets"]);
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(body.contains("safe"));
    assert!(!body.contains("keys.rs"));
}
