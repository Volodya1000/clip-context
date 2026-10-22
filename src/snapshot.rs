//! Project snapshot data types and the public collection API.
//!
//! The collector reads common UTF-8 files, respects nested `.gitignore`, and
//! supports named selection profiles. High-confidence credential detection and source-byte
//! budgets are applied before producing a snapshot.

mod collector;
mod filter;
mod markdown;
mod safety;
mod secrets;
mod tree;

use std::path::{Path, PathBuf};

pub use collector::{
    collect_project, collect_project_with_options, collect_project_with_options_and_limits,
};
pub use filter::SelectionOptions;
pub use safety::{DEFAULT_MAX_ESTIMATED_TOKENS, DEFAULT_MAX_SOURCE_BYTES, SnapshotLimits};

/// Source text together with its path relative to the project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    relative_path: PathBuf,
    content: String,
}

impl SourceFile {
    /// The source file path relative to the requested project directory.
    #[must_use]
    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    /// The original UTF-8 text, including trailing newlines.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// A reproducibly ordered, complete snapshot of matching project files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    project_name: String,
    files: Vec<SourceFile>,
}

impl Snapshot {
    /// Files in lexicographic relative-path order.
    #[must_use]
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }
}
