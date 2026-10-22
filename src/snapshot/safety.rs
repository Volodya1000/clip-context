//! Budgets based on the **fully rendered** context, including any tree and headers.

use std::io;
use std::num::NonZeroUsize;

/// Default limit for the complete UTF-8 report: 256 KiB.
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 256 * 1024;

/// Default approximate token limit for the complete UTF-8 report.
pub const DEFAULT_MAX_ESTIMATED_TOKENS: usize = 32_000;

/// Limits for a fully rendered context; no separate source-only budget exists.
///
/// Estimated tokens use `ceil(rendered_utf8_bytes / 4)`; this is a fast
/// heuristic, not a model tokenizer. The same limits apply to full Markdown,
/// compact Markdown, and tree-only output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotLimits {
    max_bytes: NonZeroUsize,
    max_estimated_tokens: NonZeroUsize,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_bytes: NonZeroUsize::new(DEFAULT_MAX_SOURCE_BYTES)
                .expect("default output byte limit is positive"),
            max_estimated_tokens: NonZeroUsize::new(DEFAULT_MAX_ESTIMATED_TOKENS)
                .expect("default estimated token limit is positive"),
        }
    }
}

impl SnapshotLimits {
    /// Maximum bytes of the final rendered context, including formatting.
    #[must_use]
    pub const fn max_source_bytes(self) -> usize {
        self.max_bytes.get()
    }

    /// Maximum estimated tokens of the final rendered context.
    #[must_use]
    pub const fn max_estimated_tokens(self) -> usize {
        self.max_estimated_tokens.get()
    }

    /// Configure the maximum complete UTF-8 output size.
    pub const fn set_max_source_bytes(&mut self, value: NonZeroUsize) {
        self.max_bytes = value;
    }

    /// Configure the estimated token ceiling for complete output.
    pub const fn set_max_estimated_tokens(&mut self, value: NonZeroUsize) {
        self.max_estimated_tokens = value;
    }

    /// A necessary upper bound on selected source bytes when source is included
    /// in output. This is only an early allocation guard, not a second budget.
    pub(crate) fn permitted_source_bytes(self) -> usize {
        self.max_bytes
            .get()
            .min(self.max_estimated_tokens.get().saturating_mul(4))
    }

    /// Verify the exact final UTF-8 report *before* writing stdout or clipboard.
    ///
    /// # Errors
    ///
    /// Returns `FileTooLarge` if the fully formatted report exceeds either
    /// limit. Never includes report contents in diagnostics.
    pub fn validate_rendered(self, report: &str) -> io::Result<()> {
        let bytes = report.len();
        let estimated_tokens = bytes.div_ceil(4);
        if bytes > self.max_bytes.get() || estimated_tokens > self.max_estimated_tokens.get() {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                format!(
                    "rendered context exceeds limit: {bytes} bytes / ~{estimated_tokens} tokens; maximum {} bytes / ~{} tokens",
                    self.max_bytes, self.max_estimated_tokens
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::num::NonZeroUsize;

    use super::SnapshotLimits;

    #[test]
    fn defaults_limit_complete_utf8_output() {
        let limits = SnapshotLimits::default();
        assert_eq!(limits.permitted_source_bytes(), 128_000);
        assert!(limits.validate_rendered("ordinary context").is_ok());
    }

    #[test]
    fn exact_output_limit_is_allowed_and_extra_markup_is_not() {
        let mut limits = SnapshotLimits::default();
        limits.set_max_source_bytes(NonZeroUsize::new(9).expect("positive"));
        assert!(limits.validate_rendered("123456789").is_ok());
        let error = limits
            .validate_rendered("1234567890")
            .expect_err("too many bytes");
        assert_eq!(error.kind(), ErrorKind::FileTooLarge);
    }

    #[test]
    fn rendered_token_limit_is_checked_independently() {
        let mut limits = SnapshotLimits::default();
        limits.set_max_estimated_tokens(NonZeroUsize::new(2).expect("positive"));
        assert!(limits.validate_rendered("12345678").is_ok());
        let error = limits
            .validate_rendered("123456789")
            .expect_err("too many tokens");
        assert_eq!(error.kind(), ErrorKind::FileTooLarge);
    }

    #[test]
    fn source_bytes_are_only_a_necessary_early_bound() {
        let mut limits = SnapshotLimits::default();
        limits.set_max_source_bytes(NonZeroUsize::new(9).expect("positive"));
        limits.set_max_estimated_tokens(NonZeroUsize::new(1).expect("positive"));
        assert_eq!(limits.permitted_source_bytes(), 4);
    }

    #[test]
    fn overlarge_configured_budget_does_not_overflow() {
        let mut limits = SnapshotLimits::default();
        limits.set_max_estimated_tokens(NonZeroUsize::new(usize::MAX).expect("positive"));
        assert_eq!(limits.permitted_source_bytes(), limits.max_source_bytes());
    }
}
