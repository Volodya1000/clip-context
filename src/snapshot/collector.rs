//! Safe, deterministic collection of source files from a project directory.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use ignore::{DirEntry, WalkBuilder};

use super::filter::{self, SelectionOptions};
use super::safety::SnapshotLimits;
use super::secrets::detect_secret;
use super::{Snapshot, SourceFile};

/// Read eligible project files, respecting nested `.gitignore` rules.
///
/// Hidden paths, common generated directories and symbolic links are excluded.
/// Git ignore rules, including negated patterns, are applied inside the root
/// even when the directory is not itself a Git checkout. Parent and global
/// ignore files are deliberately not read.
///
/// # Errors
///
/// Fails if the root is not a directory or an included file cannot be read,
/// or when traversing the directory fails. No partial result is returned.
pub fn collect_project(root: &Path) -> io::Result<Snapshot> {
    collect_project_with_options(root, &SelectionOptions::default())
}

/// Read project files respecting nested Git ignore rules and selection options.
///
/// Custom exclusions only remove more files; explicit extensions replace the
/// default extension list but do not override ignored or hidden paths.
///
/// # Errors
///
/// Returns an error for invalid roots, traversal failures or unreadable source
/// files. No partial snapshot is returned.
pub fn collect_project_with_options(
    root: &Path,
    options: &SelectionOptions,
) -> io::Result<Snapshot> {
    collect_project_with_options_and_limits(root, options, &SnapshotLimits::default())
}

/// Collect selected project files with a bounded byte/token-estimate budget
/// and high-confidence detection of credentials in each included source file.
///
/// Exceeding either budget or finding a detected credential aborts collection.
/// No partial snapshot is returned, and errors never contain secret contents.
/// The byte budget counts selected source contents rather than Markdown framing;
/// the token budget is a heuristic, not a model tokenizer.
///
/// # Errors
///
/// Returns an error for invalid roots, traversal/read failures, invalid UTF-8,
/// exceeded budgets, or a suspected secret.
pub fn collect_project_with_options_and_limits(
    root: &Path,
    options: &SelectionOptions,
    limits: &SnapshotLimits,
) -> io::Result<Snapshot> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not a project directory: {}", root.display()),
        ));
    }

    let project_name = root
        .file_name()
        .unwrap_or_else(|| OsStr::new("project"))
        .to_string_lossy()
        .into_owned();

    let mut walker = WalkBuilder::new(root);
    walker
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .ignore(false)
        .parents(false)
        .require_git(false)
        .hidden(false)
        .follow_links(false)
        .filter_entry({
            let options = options.clone();
            move |entry| is_included_entry(entry, &options)
        });

    let mut files = Vec::new();
    let mut total_source_bytes = 0_usize;
    for item in walker.build() {
        let entry = item.map_err(io::Error::other)?;
        let Some(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() || !filter::has_supported_extension(entry.path(), options) {
            continue;
        }

        let path = entry.path();
        let remaining = limits
            .permitted_source_bytes()
            .saturating_sub(total_source_bytes);
        let content = read_bounded_source(path, remaining)?;
        if let Some(kind) = detect_secret(&content) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} detected in {}", kind.description(), path.display()),
            ));
        }
        total_source_bytes += content.len();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
            .to_path_buf();
        files.push(SourceFile {
            relative_path,
            content,
        });
    }

    files.sort_unstable_by(|left: &SourceFile, right: &SourceFile| {
        left.relative_path.cmp(&right.relative_path)
    });
    Ok(Snapshot {
        project_name,
        files,
    })
}

fn is_included_entry(entry: &DirEntry, options: &SelectionOptions) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    let Some(kind) = entry.file_type() else {
        return false;
    };
    if kind.is_symlink() || filter::is_hidden(entry.file_name()) {
        return false;
    }

    !kind.is_dir() || filter::includes_directory(entry.file_name(), options)
}

/// Read at most `remaining` UTF-8 source bytes, testing one extra byte to
/// reject oversized input before allocating or printing a partial snapshot.
fn read_bounded_source(path: &Path, remaining: usize) -> io::Result<String> {
    let file = File::open(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    let limit = u64::try_from(remaining)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    file.take(limit).read_to_end(&mut bytes).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    if bytes.len() > remaining {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!(
                "source-byte or estimated-token limit exceeded at {}",
                path.display()
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|_error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("non-UTF-8 source file: {}", path.display()),
        )
    })
}
