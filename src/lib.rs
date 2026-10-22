//! Project snapshot collection and shared CLI metadata.
//!
//! The collector intentionally works without a clipboard or network connection.
//! Both transports can use this library in later development stages.
//!
//! ```
//! assert_eq!(clip_context::APPLICATION_NAME, "clip-context");
//! ```

pub mod profile;
pub mod snapshot;

/// Stable executable and package name.
pub const APPLICATION_NAME: &str = env!("CARGO_PKG_NAME");
