//! Whole-export rejection for literal credentials missed by the old detector.

mod support;

use std::process::Command;

use support::Fixture;

#[test]
fn typed_and_embedded_credentials_fail_before_any_export() {
    let header = format!("-----BEGIN {}-----", "EC PRIVATE KEY");
    for source in [
        "const API_KEY: &str = \"long_literal_credential\";".to_owned(),
        format!("const KEY: &str = \"{header}\\n\";"),
    ] {
        let fixture = Fixture::new();
        fixture.file(
            "public.rs",
            "safe content that must not be partially exported",
        );
        fixture.file("secret.rs", &source);
        for mode in ["-s", "-c"] {
            let result = Command::new(env!("CARGO_BIN_EXE_cpctx"))
                .arg(mode)
                .arg(fixture.path())
                .output()
                .expect("CLI must run");
            assert!(!result.status.success());
            assert_eq!(result.stdout, Vec::<u8>::new());
            let diagnostic = String::from_utf8_lossy(&result.stderr);
            assert!(diagnostic.contains("detected"));
            assert!(!diagnostic.contains("long_literal_credential"));
            assert!(!diagnostic.contains(&header));
            assert!(!diagnostic.contains("Clipboard error"));
        }
    }
}

#[test]
fn environment_identifiers_are_exported_and_tree_does_not_scan_contents() {
    let fixture = Fixture::new();
    fixture.file("main.rs", "let password = password_from_environment;");
    let safe = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-s")
        .arg(fixture.path())
        .output()
        .expect("CLI must run");
    assert!(safe.status.success());
    fixture.file(
        "main.rs",
        "const API_KEY: &str = \"long_literal_credential\";",
    );
    let tree = Command::new(env!("CARGO_BIN_EXE_cpctx"))
        .arg("-t")
        .arg(fixture.path())
        .output()
        .expect("CLI must run");
    assert!(tree.status.success());
    assert!(!String::from_utf8_lossy(&tree.stdout).contains("long_literal_credential"));
}
