//! Read-only selection of current tracked paths changed since a Git revision.

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MAX_GIT_OUTPUT: u64 = 8 * 1024 * 1024;

/// Return existing tracked paths changed between a revision and the worktree.
///
/// Includes committed, staged and unstaged differences within `root`. Deleted
/// and untracked paths are omitted. Names are parsed from NUL-delimited output,
/// never from line-oriented output or a shell expression. This does not read or
/// export file contents and does not change Git configuration or index state.
///
/// # Errors
/// Returns an error for an invalid revision, unavailable Git, a non-repository
/// root, failed Git operations or excessive filename output.
pub fn changed_paths(root: &Path, revision: &str) -> io::Result<Vec<PathBuf>> {
    if revision.is_empty() || revision.starts_with('-') || revision.contains(['\n', '\r', '\0']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--changed-since requires a Git commit or revision, not options",
        ));
    }
    let expression = format!("{revision}^{{commit}}");
    let resolved = run_git(
        root,
        &["rev-parse", "--verify", "--end-of-options", &expression],
    )?;
    let commit = std::str::from_utf8(&resolved)
        .map_err(io::Error::other)?
        .trim();
    let names = run_git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--relative",
            "--name-only",
            "--diff-filter=ACMRT",
            "-z",
            commit,
            "--",
            ".",
        ],
    )?;
    names
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(path_from_bytes)
        .collect()
}

fn run_git(root: &Path, arguments: &[&str]) -> io::Result<Vec<u8>> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("could not start Git: {error}")))?;
    let mut bytes = Vec::new();
    let result = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Git stdout unavailable"))?
        .take(MAX_GIT_OUTPUT + 1)
        .read_to_end(&mut bytes);
    if result.is_err() || bytes.len() as u64 > MAX_GIT_OUTPUT {
        let _ = child.kill();
        let _ = child.wait();
        result?;
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "Git filename output exceeds 8 MiB",
        ));
    }
    if !child.wait()?.success() {
        return Err(io::Error::other(
            "Git selection failed; verify the repository, revision and available history",
        ));
    }
    Ok(bytes)
}

fn path_from_bytes(bytes: &[u8]) -> io::Result<PathBuf> {
    let path = {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            PathBuf::from(OsString::from_vec(bytes.to_vec()))
        }
        #[cfg(not(unix))]
        {
            let text = std::str::from_utf8(bytes).map_err(io::Error::other)?;
            PathBuf::from(OsString::from(text))
        }
    };
    if path
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Git returned a non-relative file path",
        ));
    }
    Ok(path)
}
