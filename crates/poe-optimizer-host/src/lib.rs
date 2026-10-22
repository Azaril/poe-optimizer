//! Filesystem publication for command-line hosts.
//!
//! Callers validate and bound their artifacts before publication. This crate
//! owns no game model, authoring format, evaluator or source backend.
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

/// Resolve an existing parent directory while retaining the requested final name.
/// This does not create parents or follow an existing final destination.
pub fn destination(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "output needs a file name"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize()?.join(name))
}

/// Write and sync a complete file, then publish it without replacing an existing path.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let path = destination(path)?;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().expect("absolute output parent"))?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|error| error.error)?;
    Ok(())
}

/// Publish an already validated artifact set as one new directory.
///
/// Artifact names are single basenames: separators, parent components, drive or
/// stream syntax and NUL are rejected on every platform. Files are written and
/// synced in a private sibling directory before one no-replace rename. Failure
/// leaves any existing destination untouched and removes unpublished staging.
/// Directory publication is supported on Windows and Linux; other hosts return
/// `Unsupported` before staging any files. This is atomic visibility, not a
/// guarantee that a directory rename survives sudden power loss.
pub fn publish_artifacts<'a>(
    output: &Path,
    artifacts: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> io::Result<()> {
    if !cfg!(any(windows, target_os = "linux")) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic directory publication is supported on Windows and Linux",
        ));
    }
    let output = destination(output)?;
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "artifact output already exists",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let staging = tempfile::Builder::new()
        .prefix(".poe-publish-")
        .tempdir_in(output.parent().expect("absolute output parent"))?;
    for (name, bytes) in artifacts {
        if name.contains(['/', '\\', ':', '\0'])
            || Path::new(name).components().count() != 1
            || !matches!(
                Path::new(name).components().next(),
                Some(Component::Normal(_))
            )
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "artifact name is not a basename",
            ));
        }
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(staging.path().join(name))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    publish_noclobber(staging.path(), &output)?;
    // The old path no longer exists. Disarm cleanup so another process creating
    // that old path cannot have its unrelated directory removed by TempDir Drop.
    let _ = staging.keep();
    Ok(())
}

#[cfg(target_os = "linux")]
fn publish_noclobber(from: &Path, to: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(windows)]
fn publish_noclobber(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    fn wide(path: &Path) -> io::Result<Vec<u16>> {
        let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
        if value.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "NUL in publication path",
            ));
        }
        value.push(0);
        Ok(value)
    }
    let from = wide(from)?;
    let to = wide(to)?;
    // SAFETY: both buffers are live, initialized, NUL-terminated UTF-16 paths
    // with no interior NUL. flags=0 forbids replacement and cross-volume copying.
    // std::fs::rename is deliberately unsuitable: its Windows fallback can replace
    // an empty directory on filesystems supporting FileRenameInfoEx.
    let result = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(from.as_ptr(), to.as_ptr(), 0)
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn publish_noclobber(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic directory publication unavailable",
    ))
}

#[cfg(test)]
mod tests;
