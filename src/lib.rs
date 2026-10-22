//! Shared application metadata for the command-line frontend.
//!
//! The same metadata is available to future clipboard and SSH adapters.
//!
//! ```
//! assert_eq!(clip_context::APPLICATION_NAME, "clip-context");
//! ```

/// Stable executable and package name.
pub const APPLICATION_NAME: &str = env!("CARGO_PKG_NAME");
