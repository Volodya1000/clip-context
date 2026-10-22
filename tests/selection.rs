//! Explicit path selection, precedence and immutable safety boundaries.

mod support;

use std::process::{Command, Output};

use clip_context::snapshot::{SelectionOptions, collect_project_with_options};
use support::Fixture;

fn run(fixture: &Fixture, options: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-s")
        .arg(fixture.path())
        .args(options)
        .output()
        .expect("CLI must run")
}

#[test]
fn explicit_globs_select_extensionless_files_and_named_hidden_configuration() {
    let fixture = Fixture::new();
    for path in [
        "Dockerfile",
        "Makefile",
        ".editorconfig",
        ".github/workflows/ci.yml",
        "src/main.rs",
    ] {
        fixture.file(path, &format!("content of {path}"));
    }
    let output = run(
        &fixture,
        &[
            "--include",
            "Dockerfile",
            "--include",
            "Makefile",
            "--include",
            ".editorconfig",
            "--include",
            ".github/workflows/*.yml",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("UTF-8 output");
    for path in [
        "Dockerfile",
        "Makefile",
        ".editorconfig",
        ".github/workflows/ci.yml",
    ] {
        assert!(text.contains(&format!("content of {path}")));
    }
    assert!(!text.contains("content of src/main.rs"));
}

#[test]
fn broad_globs_do_not_implicitly_open_hidden_paths() {
    let fixture = Fixture::new();
    fixture.file("main.rs", "public");
    fixture.file(".github/config.yml", "hidden marker");
    fixture.file("config/.nested/settings.toml", "nested marker");
    let output = run(&fixture, &["--include", "**/*"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(text.contains("public"));
    assert!(!text.contains("hidden marker"));
    assert!(!text.contains("nested marker"));
    let explicit = run(&fixture, &["--include", "config/.nested/*.toml"]);
    assert!(explicit.status.success());
    assert!(String::from_utf8_lossy(&explicit.stdout).contains("nested marker"));
}

#[test]
fn include_cannot_override_credential_paths_git_metadata_or_generated_directories() {
    let fixture = Fixture::new();
    for path in [
        ".env",
        ".env.example",
        ".envrc",
        ".git/config",
        ".ssh/config",
        ".aws/config",
        ".kube/config",
        "id_rsa",
        "id_dsa",
        "id_ecdsa",
        "id_ed25519",
        "target/main.rs",
        ".venv/main.py",
        ".mypy_cache/main.py",
        ".pytest_cache/main.py",
        ".tox/main.py",
    ] {
        fixture.file(path, "must remain excluded");
    }
    let mut options = SelectionOptions::default();
    for path in [
        "**/*",
        ".env",
        ".env.example",
        ".envrc",
        ".git/**",
        ".ssh/**",
        ".aws/**",
        ".kube/**",
        ".venv/**",
        ".mypy_cache/**",
        ".pytest_cache/**",
        ".tox/**",
    ] {
        options.include_pattern(path).expect("valid glob");
    }
    let snapshot = collect_project_with_options(fixture.path(), &options).expect("safe snapshot");
    assert_eq!(snapshot.files(), []);
}

#[test]
fn excludes_win_over_includes_and_profile_extensions_narrow_path_matches() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "included Rust");
    fixture.file("src/generated.rs", "excluded Rust");
    fixture.file("src/main.py", "excluded Python");
    fixture.file("Dockerfile", "excluded extensionless");
    let output = run(
        &fixture,
        &[
            "--exclude",
            "**/generated.*",
            "--include",
            "**/*",
            "--ext",
            "rs",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(text.contains("included Rust"));
    assert!(!text.contains("excluded"));
}

#[test]
fn nested_gitignore_still_wins_over_explicit_includes() {
    let fixture = Fixture::new();
    fixture.file(".gitignore", "private/\n");
    fixture.file("src/.gitignore", "generated.rs\n");
    fixture.file("private/secret.rs", "private marker");
    fixture.file("src/generated.rs", "generated marker");
    fixture.file("src/main.rs", "public marker");
    let output = run(&fixture, &["--include", "**/*.rs"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(text.contains("public marker"));
    assert!(!text.contains("private marker"));
    assert!(!text.contains("generated marker"));
}

#[test]
fn profile_globs_and_cli_globs_combine_with_exclusions() {
    let fixture = Fixture::new();
    fixture.file(
        ".cpctx.toml",
        "[profiles.app]\ninclude = ['Dockerfile', 'src/**']\nexclude = ['**/generated.rs']\n",
    );
    fixture.file("Dockerfile", "selected container");
    fixture.file("Makefile", "selected make");
    fixture.file("src/main.rs", "excluded by CLI");
    fixture.file("src/generated.rs", "excluded by profile");
    let output = run(
        &fixture,
        &[
            "--profile",
            "app",
            "--include",
            "Makefile",
            "--exclude",
            "src/main.rs",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(text.contains("selected container"));
    assert!(text.contains("selected make"));
    assert!(!text.contains("excluded by"));
}

#[test]
fn invalid_globs_fail_before_stdout_or_clipboard() {
    let fixture = Fixture::new();
    fixture.file("main.rs", "not partially exported");
    for pattern in [
        "",
        "../*",
        "src/../*.rs",
        "/etc/*",
        "!*.rs",
        "#comment",
        "C:/code",
        "src\\*.rs",
        "[abc",
        " spaced",
        "a\nb",
        "a\rb",
    ] {
        for flag in ["--include", "--exclude"] {
            let result = run(&fixture, &[flag, pattern]);
            assert!(!result.status.success(), "{pattern:?}");
            assert_eq!(result.stdout, Vec::<u8>::new());
        }
    }
    for field in ["include", "exclude"] {
        fixture.file(
            ".cpctx.toml",
            &format!("[profiles.invalid]\n{field} = ['[abc']\n"),
        );
        let result = run(&fixture, &["--profile", "invalid"]);
        assert!(!result.status.success());
        assert_eq!(result.stdout, Vec::<u8>::new());
    }
}

#[test]
fn path_selected_extensionless_files_still_receive_credential_checks() {
    let fixture = Fixture::new();
    let token = format!("ghp_{}", "X".repeat(36));
    fixture.file("Dockerfile", &format!("ENV access_token=\"{token}\""));
    let output = run(&fixture, &["--include", "Dockerfile"]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(&token));
}
