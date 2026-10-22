//! Output metrics, compact format and stdout/stderr separation.

mod support;

use std::process::{Command, Output};

use support::Fixture;

fn invoke(fixture: &Fixture, mode: &str, options: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg(mode)
        .arg(fixture.path())
        .args(options)
        .output()
        .expect("CLI must execute")
}

fn assert_exact_stats(output: &Output, bytes: usize) {
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains(&format!("({bytes} bytes)")));
    assert!(diagnostic.contains(&format!("~{} tokens (estimated)", bytes.div_ceil(4))));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Context: "));
}

#[test]
fn stats_describe_exact_utf8_stdout_including_markdown_framing() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "fn hello() { println!(\"Привет 🌍\"); }");
    let output = invoke(&fixture, "-s", &[]);
    assert!(output.status.success());
    assert_exact_stats(&output, output.stdout.len());
}

#[test]
fn compact_format_omits_duplicate_tree_without_dropping_source_code() {
    let fixture = Fixture::new();
    for i in 0..8 {
        fixture.file(
            &format!("src/component/nested/mod{i}.rs"),
            &format!("unique_source_{i}"),
        );
    }

    let full = invoke(&fixture, "-s", &[]);
    let compact = invoke(&fixture, "-s", &["--format", "compact"]);
    assert!(full.status.success());
    assert!(compact.status.success());
    assert_exact_stats(&full, full.stdout.len());
    assert_exact_stats(&compact, compact.stdout.len());
    let full_text = String::from_utf8(full.stdout).expect("UTF-8 report");
    let compact_text = String::from_utf8(compact.stdout).expect("UTF-8 report");

    assert!(full_text.contains("## File tree"));
    assert!(!compact_text.contains("## File tree"));
    assert!(compact_text.len() < full_text.len());
    for i in 0..8 {
        assert!(compact_text.contains(&format!("## src/component/nested/mod{i}.rs")));
        assert_eq!(compact_text.matches(&format!("unique_source_{i}")).count(), 1);
    }
}

#[test]
fn explicit_full_format_matches_default_byte_for_byte() {
    let fixture = Fixture::new();
    fixture.file("docs/main.md", "content");
    let implicit = invoke(&fixture, "-s", &[]);
    let explicit = invoke(&fixture, "--stdout", &["--format", "full"]);
    assert!(implicit.status.success());
    assert_eq!(implicit.stdout, explicit.stdout);
    assert_eq!(implicit.stderr, explicit.stderr);
}

#[test]
fn compact_output_preserves_backticks_and_newlines() {
    let fixture = Fixture::new();
    fixture.file("docs/code.md", "header\n`````\nsuffix");
    let output = invoke(&fixture, "-s", &["--format", "compact"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("Markdown");
    assert!(text.contains("``````\nheader"));
    assert!(text.contains("suffix\n``````"));
}

#[test]
fn tree_only_mode_reports_only_emitted_tree_size() {
    let fixture = Fixture::new();
    fixture.file("src/code.rs", "very long irrelevant content repeated repeatedly");
    let tree = invoke(&fixture, "-t", &["--format", "compact"]);
    let plain_tree = invoke(&fixture, "-t", &[]);
    assert!(tree.status.success());
    assert_eq!(tree.stdout, plain_tree.stdout);
    assert_exact_stats(&tree, tree.stdout.len());
    assert!(!String::from_utf8_lossy(&tree.stdout).contains("very long"));
}

#[test]
fn invalid_format_fails_without_data_or_stats() {
    let fixture = Fixture::new();
    fixture.file("src/main.rs", "must-not-be-exposed");
    for values in [vec!["--format", "minimal"], vec!["--format"], vec!["--format", ""]] {
        let output = invoke(&fixture, "-s", &values);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Context: "));
    }
}

#[test]
fn failed_collection_has_no_success_metrics() {
    let fixture = Fixture::new();
    let secret = format!("ghp_{}", "X".repeat(36));
    fixture.file("src/config.py", &format!("api_key = '{secret}'"));
    let output = invoke(&fixture, "-s", &["--format", "compact"]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(!err.contains(&secret));
    assert!(!err.contains("Context: "));
}
