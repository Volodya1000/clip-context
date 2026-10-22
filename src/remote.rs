//! SSH-based collection without requiring cpctx on the remote host.
//!
//! The remote POSIX system only needs SSH and tar. The bounded archive is
//! unpacked into a temporary local directory before the existing project
//! selection, secret detection, and resource limits are applied.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use crate::snapshot::{
    SelectionOptions, Snapshot, SnapshotLimits, collect_project_tree,
    collect_project_with_options_and_limits,
};

/// Maximum bytes accepted from remote tar, including tar metadata (16 MiB).
pub const MAX_REMOTE_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
const MAX_REMOTE_ARCHIVE_ENTRIES: usize = 10_000;

/// Collect a POSIX project over SSH, without installing cpctx remotely.
///
/// The remote account requires a POSIX-compatible shell and `tar`. SSH uses
/// configured host keys and credentials; no host-key verification is bypassed.
/// The remote command only reads the project and never creates remote files.
///
/// The complete tar stream is bounded independently of the later source-byte
/// and estimated-token budgets. Selected content is scanned locally before
/// any snapshot is returned. Entries cannot write outside the temporary root.
///
/// # Errors
///
/// Returns an error for an invalid destination, failed SSH/tar command, an
/// oversized or malformed archive, unsafe archive entries, or failed local
/// collection. Error messages never include source-file contents.
pub fn collect_remote_project(
    destination: &str,
    remote_root: &Path,
    options: &SelectionOptions,
    limits: &SnapshotLimits,
) -> io::Result<Snapshot> {
    collect_remote_project_with_selection(destination, remote_root, limits, |_root| {
        Ok(options.clone())
    })
}

/// Collect a remote project and resolve its selection rules after extraction.
///
/// This supports remote TOML profiles while retaining the exact local
/// configuration validation and precedence rules.
///
/// # Errors
///
/// Returns any SSH, extraction, selection, or collection error.
pub fn collect_remote_project_with_selection<F>(
    destination: &str,
    remote_root: &Path,
    limits: &SnapshotLimits,
    select: F,
) -> io::Result<Snapshot>
where
    F: FnOnce(&Path) -> io::Result<SelectionOptions>,
{
    collect_remote_with_selection(destination, remote_root, limits, select, true)
}

/// Collect a remote project's tree without downloading code contents into
/// the final snapshot. Archive transfer bounds still apply.
///
/// # Errors
///
/// Returns any SSH, extraction, configuration or traversal error.
pub fn collect_remote_tree_with_selection<F>(
    destination: &str,
    remote_root: &Path,
    limits: &SnapshotLimits,
    select: F,
) -> io::Result<Snapshot>
where
    F: FnOnce(&Path) -> io::Result<SelectionOptions>,
{
    collect_remote_with_selection(destination, remote_root, limits, select, false)
}

fn collect_remote_with_selection<F>(
    destination: &str,
    remote_root: &Path,
    limits: &SnapshotLimits,
    select: F,
    include_contents: bool,
) -> io::Result<Snapshot>
where
    F: FnOnce(&Path) -> io::Result<SelectionOptions>,
{
    validate_destination(destination)?;
    let remote_path = remote_root.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "remote path must be valid UTF-8",
        )
    })?;
    if remote_path.is_empty() || remote_path.contains(['\n', '\r', '\0']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "remote path must be nonempty and contain no control line breaks",
        ));
    }

    let command = format!("tar -cf - -C {} .", quote_posix(remote_path));
    let mut child = Command::new("ssh")
        .args([
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "ClearAllForwardings=yes",
            "-o",
            "ConnectTimeout=10",
        ])
        .arg(destination)
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("could not start SSH: {error}")))?;

    let mut bytes = Vec::new();
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("SSH stdout pipe was not available"))?;
    let read_result = stdout
        .by_ref()
        .take(MAX_REMOTE_ARCHIVE_BYTES as u64 + 1)
        .read_to_end(&mut bytes);
    drop(stdout);

    if bytes.len() > MAX_REMOTE_ARCHIVE_BYTES || read_result.is_err() {
        child.kill()?;
        child.wait()?;
        if let Err(error) = read_result {
            return Err(io::Error::new(
                error.kind(),
                format!("could not read SSH archive: {error}"),
            ));
        }
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "remote archive exceeds the 16 MiB transfer limit",
        ));
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::other(
            "remote SSH or tar command failed; verify access, directory and tar availability",
        ));
    }

    let directory = tempfile::tempdir()?;
    unpack_archive(&bytes, directory.path())?;
    let options = select(directory.path())?;
    let mut snapshot = if include_contents {
        collect_project_with_options_and_limits(directory.path(), &options, limits)?
    } else {
        collect_project_tree(directory.path(), &options)?
    };
    snapshot.set_project_name(
        remote_root
            .file_name()
            .unwrap_or_else(|| OsStr::new("remote-project"))
            .to_string_lossy()
            .into_owned(),
    );
    Ok(snapshot)
}

fn validate_destination(destination: &str) -> io::Result<()> {
    if destination.is_empty()
        || destination.starts_with('-')
        || destination.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'@' | b'.' | b'-' | b'_' | b':' | b'[' | b']')
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SSH destination must be a host or user@host without whitespace or options",
        ));
    }
    Ok(())
}

fn quote_posix(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', "'\\''"))
}

fn safe_relative_path(path: &Path) -> io::Result<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                let name = part.to_string_lossy();
                if name.contains(['\\', ':', '\0']) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "remote archive contains an unsafe file name",
                    ));
                }
                result.push(part);
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "remote archive contains a path outside its root",
                ));
            }
        }
    }
    Ok(result)
}

fn unpack_archive(bytes: &[u8], root: &Path) -> io::Result<()> {
    let mut archive = tar::Archive::new(Cursor::new(bytes));
    let mut count = 0_usize;
    for entry in archive.entries()? {
        count += 1;
        if count > MAX_REMOTE_ARCHIVE_ENTRIES {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "remote archive has too many entries",
            ));
        }
        let mut entry = entry?;
        let relative = safe_relative_path(&entry.path()?)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = root.join(relative);
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.header().entry_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)?;
            io::copy(&mut entry, &mut file)?;
        }
        // Symlinks, hardlinks and device entries are ignored. No links are
        // ever created inside the local extraction directory.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};
    use std::path::Path;

    use super::{quote_posix, safe_relative_path, unpack_archive, validate_destination};

    fn archive_file(path: &str, contents: &str) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, path, Cursor::new(contents))
            .expect("test tar entry must be valid");
        builder.into_inner().expect("test archive must finish")
    }

    #[test]
    fn destination_accepts_ssh_alias_and_user_at_host() {
        assert!(validate_destination("production").is_ok());
        assert!(validate_destination("operator@host.example").is_ok());
        assert!(validate_destination("[::1]").is_ok());
    }

    #[test]
    fn destination_rejects_ssh_flags_and_shell_syntax() {
        for invalid in [
            "",
            "-F",
            "user@host;sh",
            "host name",
            "host\nextra",
            "x'bad",
        ] {
            assert!(validate_destination(invalid).is_err());
        }
    }

    #[test]
    fn paths_are_quoted_without_interpreting_shell_metacharacters() {
        assert_eq!(quote_posix("/srv/a b"), "'/srv/a b'");
        assert_eq!(quote_posix("/srv/a'b"), "'/srv/a'\\''b'");
    }

    #[test]
    fn archive_rejects_paths_outside_the_temporary_root() {
        assert!(safe_relative_path(Path::new("../secrets")).is_err());
        assert!(safe_relative_path(Path::new("/etc/passwd")).is_err());
        assert!(safe_relative_path(Path::new("C:\\Windows")).is_err());
        assert!(safe_relative_path(Path::new("./src/main.rs")).is_ok());
    }

    #[test]
    fn safe_archive_files_are_extracted_into_the_requested_root() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let bytes = archive_file("./src/main.rs", "fn main() {}");
        unpack_archive(&bytes, directory.path()).expect("valid archive");
        assert_eq!(
            std::fs::read_to_string(directory.path().join("src/main.rs")).expect("extracted file"),
            "fn main() {}"
        );
    }

    #[test]
    fn malformed_tar_archive_fails_without_extraction() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let result = unpack_archive(b"bad archive bytes", directory.path());
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_archive_file_entries_are_rejected() {
        let mut builder = tar::Builder::new(Vec::new());
        for value in ["first", "second"] {
            let mut header = tar::Header::new_gnu();
            header.set_size(value.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "same.rs", Cursor::new(value))
                .expect("test archive creation");
        }
        let bytes = builder.into_inner().expect("test archive bytes");
        let directory = tempfile::tempdir().expect("temporary directory");
        let err = unpack_archive(&bytes, directory.path())
            .expect_err("a duplicate must not overwrite an earlier entry");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn archive_does_not_materialize_symlinks() {
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(0o777);
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_link_name("outside").expect("link name");
        header.set_cksum();
        builder
            .append_data(&mut header, "src/link.rs", io::empty())
            .expect("synthetic symlink");
        let bytes = builder.into_inner().expect("test archive bytes");
        let directory = tempfile::tempdir().expect("temporary directory");
        unpack_archive(&bytes, directory.path()).expect("links should be skipped");
        assert!(!directory.path().join("src/link.rs").exists());
    }
}
