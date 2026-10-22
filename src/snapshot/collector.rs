//! Safe, deterministic collection of source files from a project directory.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use ignore::{DirEntry, WalkBuilder};

use super::filter::{SelectionOptions, SelectionRules};
use super::safety::SnapshotLimits;
use super::secrets::detect_secret;
use super::{Snapshot, SourceFile};

/// Read eligible project files, respecting nested `.gitignore` rules.
///
/// Hidden paths, common generated directories and symbolic links are excluded
/// by default; explicit include prefixes can select safe hidden configuration.
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
/// Custom exclusions only remove more files. Include globs can select safe
/// extensionless/hidden configuration; `.gitignore` and protected paths win.
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

/// Collect source files with a necessary early bound derived from final output
/// budgets, plus credential checks.
///
/// Source text larger than the eventual output ceiling is rejected early.
/// The definitive check for output bytes, filenames and Markdown framing runs
/// after rendering through `SnapshotLimits::validate_rendered`.
///
/// # Errors
///
/// Returns an error for invalid roots, traversal/read failures, invalid UTF-8,
/// exceeded source lower bounds, or a suspected secret.
pub fn collect_project_with_options_and_limits(
    root: &Path,
    options: &SelectionOptions,
    limits: &SnapshotLimits,
) -> io::Result<Snapshot> {
    collect_with_contents(root, options, limits, true)
}

/// Collect selected paths without reading source contents, for tree-only output.
///
/// This allows a small tree report from a project containing very large source
/// files. Content-based credential detection is unnecessary when no source
/// bytes are exported.
///
/// # Errors
///
/// Returns an error on invalid directories or failed traversal.
pub fn collect_project_tree(root: &Path, options: &SelectionOptions) -> io::Result<Snapshot> {
    collect_with_contents(root, options, &SnapshotLimits::default(), false)
}

fn collect_with_contents(
    root: &Path,
    options: &SelectionOptions,
    limits: &SnapshotLimits,
    include_contents: bool,
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
            let rules = SelectionRules::new(root, options)?;
            let root = root.to_path_buf();
            move |entry| is_included_entry(entry, &root, &rules)
        });

    let mut files = Vec::new();
    let mut total_source_bytes = 0_usize;
    for item in walker.build() {
        let entry = item.map_err(io::Error::other)?;
        let Some(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() {
            continue;
        }

        let path = entry.path();
        let content = if include_contents {
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
            content
        } else {
            String::new()
        };
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

fn is_included_entry(entry: &DirEntry, root: &Path, rules: &SelectionRules) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    let Some(kind) = entry.file_type() else {
        return false;
    };
    if kind.is_symlink() {
        return false;
    }
    entry.path().strip_prefix(root).is_ok_and(|relative| {
        rules
            .exclusion(entry.path(), relative, kind.is_dir())
            .is_none()
    })
}

/// Read at most `remaining` bytes to avoid allocations when the eventual
/// rendered context could never fit the configured ceiling.
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
                "rendered context cannot fit output limit: selected source at {}",
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
