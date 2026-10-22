//! High-confidence patterns for dangerous credentials in source files.
//!
//! This is defense in depth, not a complete credentials scanner. Patterns
//! intentionally identify a focused set of secrets without external services.

/// A non-sensitive description of suspicious material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecretKind {
    PrivateKey,
    AccessToken,
    CredentialAssignment,
}

impl SecretKind {
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::PrivateKey => "private key",
            Self::AccessToken => "access token",
            Self::CredentialAssignment => "credential assignment",
        }
    }
}

/// Detect selected well-known secrets without returning their contents.
pub(crate) fn detect_secret(content: &str) -> Option<SecretKind> {
    if content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("-----BEGIN ") && trimmed.ends_with("PRIVATE KEY-----")
    }) {
        return Some(SecretKind::PrivateKey);
    }

    const PREFIXES: &[(&str, usize)] = &[
        ("github_pat_", 20),
        ("ghp_", 30),
        ("gho_", 30),
        ("ghu_", 30),
        ("ghs_", 30),
        ("ghr_", 30),
        ("glpat-", 16),
        ("sk_live_", 16),
        ("xoxb-", 24),
        ("xoxp-", 24),
        ("sk-", 24),
    ];

    if PREFIXES
        .iter()
        .any(|(prefix, minimum)| has_long_token(content, prefix, *minimum))
        || contains_aws_access_key(content)
    {
        return Some(SecretKind::AccessToken);
    }

    if content.lines().any(has_inline_credential) {
        return Some(SecretKind::CredentialAssignment);
    }

    None
}

fn has_long_token(content: &str, prefix: &str, minimum: usize) -> bool {
    content.match_indices(prefix).any(|(index, _)| {
        let remainder = &content[index + prefix.len()..];
        let count = remainder
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            .count();
        count >= minimum
    })
}

fn contains_aws_access_key(content: &str) -> bool {
    ["AKIA", "ASIA"].iter().any(|prefix| {
        content.match_indices(prefix).any(|(index, _)| {
            let remainder = &content[index + prefix.len()..];
            let suffix: Vec<u8> = remainder.bytes().take(17).collect();
            suffix.len() >= 16
                && suffix[..16]
                    .iter()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
                && (suffix.len() == 16 || !suffix[16].is_ascii_alphanumeric())
        })
    })
}

fn has_inline_credential(line: &str) -> bool {
    let Some((left, right)) = line.split_once(['=', ':']) else {
        return false;
    };
    let name = left
        .trim()
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .rsplit(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .next()
        .unwrap_or_default();
    if ![
        "password",
        "passwd",
        "api_key",
        "access_token",
        "auth_token",
        "client_secret",
        "secret_key",
    ]
    .iter()
    .any(|expected| name.eq_ignore_ascii_case(expected))
    {
        return false;
    }
    let value = right
        .trim()
        .trim_start_matches(['\'', '"', '`'])
        .bytes()
        .take_while(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'/' | b'+' | b'=')
        })
        .count();
    value >= 12
}

#[cfg(test)]
mod tests {
    use super::{SecretKind, detect_secret};

    #[test]
    fn detects_pem_private_keys_without_revealing_the_body() {
        let rsa_header = format!("-----BEGIN {}-----\nsecret\n", "RSA PRIVATE KEY");
        assert_eq!(detect_secret(&rsa_header), Some(SecretKind::PrivateKey));
        let ssh_header = format!("-----BEGIN {}-----\n", "OPENSSH PRIVATE KEY");
        assert_eq!(detect_secret(&ssh_header), Some(SecretKind::PrivateKey));
    }

    #[test]
    fn detects_synthetic_service_tokens() {
        let github = format!("ghp_{}", "Q".repeat(36));
        let openai = format!("sk-{}", "x".repeat(35));
        let aws = format!("AKIA{}", "A".repeat(16));
        let gitlab = format!("glpat-{}", "z".repeat(25));
        for synthetic_token in [github, openai, aws, gitlab] {
            assert_eq!(
                detect_secret(&synthetic_token),
                Some(SecretKind::AccessToken)
            );
        }
    }

    #[test]
    fn detects_literal_credentials_in_multiple_formats() {
        assert_eq!(
            detect_secret("password = \"sufficiently_long_value\""),
            Some(SecretKind::CredentialAssignment)
        );
        assert_eq!(
            detect_secret("client_secret: \"long_secret_literal_value\""),
            Some(SecretKind::CredentialAssignment)
        );
    }

    #[test]
    fn does_not_block_regular_source_or_short_examples() {
        for example in [
            "const PASSWORD_KEY: &str = \"password\";",
            "token = \"short\"",
            "fn main() { println!(\"hello\"); }",
            "ghp_short",
            "AKIAABC",
            "sk-brief",
        ] {
            assert_eq!(detect_secret(example), None);
        }
    }

    #[test]
    fn only_detects_aws_tokens_with_full_expected_length() {
        let too_long = format!("AKIA{}extra", "1234567890123456");
        let matching = format!("ASIA{}", "1234567890123456");
        assert_eq!(detect_secret(&too_long), None);
        assert_eq!(detect_secret(&matching), Some(SecretKind::AccessToken));
    }

    #[test]
    fn safe_error_labels_never_contain_a_credential() {
        assert_eq!(SecretKind::PrivateKey.description(), "private key");
        assert_eq!(SecretKind::AccessToken.description(), "access token");
        assert_eq!(
            SecretKind::CredentialAssignment.description(),
            "credential assignment"
        );
    }
}
