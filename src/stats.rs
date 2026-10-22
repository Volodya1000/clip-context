//! Fast approximate output metrics for terminal diagnostics.

use std::fmt;

/// Size of the exact UTF-8 output and a lightweight token estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutputStats {
    bytes: usize,
    tokens_estimate: usize,
}

impl OutputStats {
    /// Calculate O(1) metrics from already-rendered output, without tokenizing.
    pub(crate) const fn from_utf8(text: &str) -> Self {
        let bytes = text.len();
        Self {
            bytes,
            tokens_estimate: bytes.div_ceil(4),
        }
    }
}

impl fmt::Display for OutputStats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Context: {:.1} KiB ({} bytes) | ~{} tokens (estimated)",
            self.bytes as f64 / 1024.0,
            self.bytes,
            self.tokens_estimate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::OutputStats;

    #[test]
    fn estimates_tokens_using_rounded_up_utf8_byte_count() {
        for (input, bytes, tokens) in [
            ("", 0, 0),
            ("a", 1, 1),
            ("abcd", 4, 1),
            ("abcde", 5, 2),
            ("123456789", 9, 3),
        ] {
            let stats = OutputStats::from_utf8(input);
            assert_eq!(stats.bytes, bytes);
            assert_eq!(stats.tokens_estimate, tokens);
        }
    }

    #[test]
    fn byte_count_includes_unicode_encoding() {
        let text = "Привет 🌍";
        let stats = OutputStats::from_utf8(text);
        assert_eq!(stats.bytes, text.len());
        assert_eq!(stats.tokens_estimate, text.len().div_ceil(4));
    }

    #[test]
    fn diagnostic_line_is_english_and_contains_only_numbers() {
        let stats = OutputStats::from_utf8("abcd");
        assert_eq!(
            stats.to_string(),
            "Context: 0.0 KiB (4 bytes) | ~1 tokens (estimated)"
        );
    }
}
