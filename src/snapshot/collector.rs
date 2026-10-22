//! Safe, deterministic collection of source files from a project directory.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use ignore::{DirEntry, WalkBuilder};

use super::filter;
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
        .filter_entry(is_included_entry);

    let mut files = Vec::new();
    for item in walker.build() {
        let entry = item.map_err(io::Error::other)?;
        let Some(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() || !filter::has_supported_extension(entry.path()) {
            continue;
        }

        let path = entry.path();
        let content = fs::read_to_string(path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to read {}: {error}", path.display()),
            )
        })?;
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

fn is_included_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    let Some(kind) = entry.file_type() else {
        return false;
    };
    if kind.is_symlink() || filter::is_hidden(entry.file_name()) {
        return false;
    }

    !kind.is_dir() || filter::includes_directory(entry.file_name())
}
