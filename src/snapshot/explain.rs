//! Metadata-only selection diagnostics without exporting source contents.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use super::filter::SelectionRules;
use super::{SelectionOptions, SnapshotLimits};

const MAX_ENTRIES: usize = 10_000;
const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;

struct Entry {
    path: PathBuf,
    bytes: Option<u64>,
    reason: &'static str,
}

/// Explain selected paths, sizes, exclusions and a source-byte budget lower bound.
///
/// Reads metadata and project-local `.gitignore` files, but no source contents.
/// It does not run credential detection, validate UTF-8 source, or promise that
/// the final rendered context fits. Export budgets are shown for comparison;
/// the diagnostic itself has independent 10,000-entry / 4 MiB safety bounds.
///
/// # Errors
/// Returns an error for invalid roots, traversal/ignore failures or a report
/// exceeding the diagnostic safety bounds. No partial report is returned.
pub fn explain_project(
    root: &Path,
    options: &SelectionOptions,
    limits: &SnapshotLimits,
) -> io::Result<String> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a project directory",
        ));
    }
    let rules = SelectionRules::new(root, options)?;
    let mut pending = vec![(root.to_path_buf(), Vec::new())];
    let mut entries = Vec::new();
    let mut visited = 0_usize;
    while let Some((directory, mut ignores)) = pending.pop() {
        let file = directory.join(".gitignore");
        if file.try_exists()? {
            let mut builder = GitignoreBuilder::new(&directory);
            if let Some(error) = builder.add(file) {
                return Err(io::Error::other(error));
            }
            ignores.push(builder.build().map_err(io::Error::other)?);
        }
        inspect_directory(
            root,
            &directory,
            &ignores,
            &rules,
            &mut pending,
            &mut entries,
            &mut visited,
        )?;
    }
    entries.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    render_report(&entries, limits)
}

fn inspect_directory(
    root: &Path,
    directory: &Path,
    ignores: &[Gitignore],
    rules: &SelectionRules,
    pending: &mut Vec<(PathBuf, Vec<Gitignore>)>,
    entries: &mut Vec<Entry>,
    visited: &mut usize,
) -> io::Result<()> {
    for item in fs::read_dir(directory)? {
        *visited += 1;
        if *visited > MAX_ENTRIES {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "explanation exceeds 10000 entries",
            ));
        }
        let item = item?;
        let path = item.path();
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        let kind = item.file_type()?;
        let matches_gitignore = ignores
            .iter()
            .rev()
            .map(|ignore| ignore.matched(&path, kind.is_dir()))
            .find(|matched| !matched.is_none())
            .is_some_and(|matched| matched.is_ignore());
        let reason = if kind.is_symlink() {
            Some("symbolic link")
        } else if !kind.is_file() && !kind.is_dir() {
            Some("special file")
        } else if matches_gitignore {
            Some("gitignore rule")
        } else {
            rules.exclusion(&path, relative, kind.is_dir())
        };
        if kind.is_dir() && reason.is_none() {
            pending.push((path, ignores.to_vec()));
            continue;
        }
        let bytes = if reason.is_none() {
            Some(item.metadata()?.len())
        } else {
            None
        };
        entries.push(Entry {
            path: relative.to_path_buf(),
            bytes,
            reason: reason.unwrap_or("selected"),
        });
        if entries.len() + pending.len() > MAX_ENTRIES {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "explanation exceeds 10000 entries",
            ));
        }
    }
    Ok(())
}

fn render_report(entries: &[Entry], limits: &SnapshotLimits) -> io::Result<String> {
    let mut selected: Vec<_> = entries
        .iter()
        .filter(|entry| entry.bytes.is_some())
        .collect();
    let total = selected.iter().fold(0_u64, |sum, entry| {
        sum.saturating_add(entry.bytes.unwrap_or_default())
    });
    let mut output = format!(
        "Selection preview (metadata only; source contents and secrets not checked)\nSelected: {} files, {total} source bytes, ~{} source tokens (estimated)\nExport limits: {} rendered bytes / ~{} rendered tokens\n",
        selected.len(),
        total.div_ceil(4),
        limits.max_source_bytes(),
        limits.max_estimated_tokens()
    );
    if total > u64::try_from(limits.permitted_source_bytes()).unwrap_or(u64::MAX) {
        output.push_str("Budget: EXCEEDS the export limit even before Markdown overhead. Narrow --include or add --exclude.\n");
    } else {
        output.push_str("Budget: source lower bound fits; Markdown/tree overhead and credential checks still apply at export.\n");
    }
    output.push_str("\nLargest selected files (source bytes):\n");
    selected.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then(left.path.cmp(&right.path))
    });
    for entry in selected.into_iter().take(5) {
        writeln!(
            output,
            "  {} {}",
            entry.bytes.unwrap_or_default(),
            entry.path.to_string_lossy().escape_debug()
        )
        .expect("String write");
    }
    output.push_str("\nDecisions (excluded directory descendants are not traversed):\n");
    for entry in entries {
        if let Some(bytes) = entry.bytes {
            writeln!(
                output,
                "INCLUDE {bytes} bytes {}",
                entry.path.to_string_lossy().escape_debug()
            )
            .expect("String write");
        } else {
            writeln!(
                output,
                "EXCLUDE {}: {}",
                entry.path.to_string_lossy().escape_debug(),
                entry.reason
            )
            .expect("String write");
        }
        if output.len() > MAX_REPORT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "explanation exceeds 4 MiB",
            ));
        }
    }
    Ok(output)
}
