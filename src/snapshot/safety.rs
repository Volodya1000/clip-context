//! Resource limits for project snapshot collection.

use std::num::NonZeroUsize;

/// Default maximum number of selected source bytes: 256 KiB.
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 256 * 1024;

/// Default upper bound for the approximate token count of selected source.
pub const DEFAULT_MAX_ESTIMATED_TOKENS: usize = 32_000;

/// Resource budgets applied before any selected source content is printed.
///
/// Token counts are estimated from source bytes using a fixed four-byte-per-token
/// heuristic. They are NOT exact model token counts; use a model-specific
/// tokenizer before relying on an exact context-window budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotLimits {
    max_source_bytes: NonZeroUsize,
    max_estimated_tokens: NonZeroUsize,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: NonZeroUsize::new(DEFAULT_MAX_SOURCE_BYTES)
                .expect("default source-byte limit is positive"),
            max_estimated_tokens: NonZeroUsize::new(DEFAULT_MAX_ESTIMATED_TOKENS)
                .expect("default estimated-token limit is positive"),
        }
    }
}

impl SnapshotLimits {
    /// Cumulative maximum source bytes across all selected files.
    #[must_use]
    pub fn max_source_bytes(self) -> usize {
        self.max_source_bytes.get()
    }

    /// Cumulative estimated tokens across all selected files.
    #[must_use]
    pub fn max_estimated_tokens(self) -> usize {
        self.max_estimated_tokens.get()
    }

    /// Change the maximum source byte budget.
    pub fn set_max_source_bytes(&mut self, value: NonZeroUsize) {
        self.max_source_bytes = value;
    }

    /// Change the maximum estimated token budget.
    pub fn set_max_estimated_tokens(&mut self, value: NonZeroUsize) {
        self.max_estimated_tokens = value;
    }

    pub(crate) fn permitted_source_bytes(self) -> usize {
        self.max_source_bytes()
            .min(self.max_estimated_tokens().saturating_mul(4))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::SnapshotLimits;

    #[test]
    fn default_budgets_bound_the_source_bytes() {
        let limits = SnapshotLimits::default();
        assert_eq!(limits.permitted_source_bytes(), 128_000);
    }

    #[test]
    fn smallest_budget_wins() {
        let mut limits = SnapshotLimits::default();
        limits.set_max_source_bytes(NonZeroUsize::new(9).expect("positive"));
        limits.set_max_estimated_tokens(NonZeroUsize::new(1).expect("positive"));
        assert_eq!(limits.permitted_source_bytes(), 4);
        assert_eq!(limits.max_source_bytes(), 9);
        assert_eq!(limits.max_estimated_tokens(), 1);
    }

    #[test]
    fn oversized_estimated_budget_does_not_overflow() {
        let mut limits = SnapshotLimits::default();
        limits.set_max_estimated_tokens(NonZeroUsize::new(usize::MAX).expect("positive"));
        assert_eq!(limits.permitted_source_bytes(), limits.max_source_bytes());
    }
}
