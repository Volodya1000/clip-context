//! Keep the two user-facing README variants discoverable and in sync.

const ENGLISH: &str = include_str!("../README.md");
const RUSSIAN: &str = include_str!("../README.ru.md");

#[test]
fn both_languages_link_to_each_other() {
    assert!(ENGLISH.contains("[Русский](README.ru.md)"));
    assert!(RUSSIAN.contains("[English](README.md)"));
}

#[test]
fn both_languages_document_every_primary_cli_mode_and_limit() {
    for guide in [ENGLISH, RUSSIAN] {
        for argument in [
            "cpctx -s",
            "cpctx -t",
            "cpctx -c",
            "--ext",
            "--exclude-dir",
            "--profile",
            "--config",
            "--format",
            "--ssh",
            "--max-bytes",
            "--max-estimated-tokens",
        ] {
            assert!(guide.contains(argument), "README missing {argument}");
        }
        assert!(guide.contains("262144"));
        assert!(guide.contains("32000"));
    }
}
