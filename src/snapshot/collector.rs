//! Safe, deterministic collection of source files from a project directory.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use super::filter::{self, SelectionOptions};
use super::{Snapshot, SourceFile};

/// Read eligible project files, rejecting unreadable and non-UTF-8 source files.
///
/// Hidden paths, common generated directories and symbolic links are excluded.
/// File selection uses a fixed list of familiar source-code extensions. No
/// secret-content scanner or `.gitignore` interpreter is included in this stage.
///
/// # Errors
///
/// Fails if the path is not a directory or an included file cannot be read.
/// No partial result is returned on failure.
pub fn collect_project(root: &Path) -> io::Result<Snapshot> {
    collect_project_with_options(root, &SelectionOptions::default())
}

/// Collect project files with additional selection rules.
///
/// Built-in exclusions still apply even when custom extensions or exclusions
/// are supplied.
///
/// # Errors
///
/// Fails if the root is not a directory or a matching file cannot be read.
pub fn collect_project_with_options(
    root: &Path,
    options: &SelectionOptions,
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

    let mut files = Vec::new();
    visit_directory(root, root, options, &mut files)?;
    files.sort_unstable_by(|left: &SourceFile, right: &SourceFile| {
        left.relative_path.cmp(&right.relative_path)
    });
    Ok(Snapshot {
        project_name,
        files,
    })
}

fn visit_directory(
    root: &Path,
    directory: &Path,
    options: &SelectionOptions,
    files: &mut Vec<SourceFile>,
) -> io::Result<()> {
    for result in fs::read_dir(directory)? {
        let entry = result?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }

        let name = entry.file_name();
        if filter::is_hidden(&name) {
            continue;
        }

        if kind.is_dir() {
            if filter::includes_directory(&name, options) {
                visit_directory(root, &entry.path(), options, files)?;
            }
        } else if kind.is_file() && filter::has_supported_extension(&entry.path(), options) {
            let path = entry.path();
            let content = fs::read_to_string(&path).map_err(|error| {
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
    }

    Ok(())
}
