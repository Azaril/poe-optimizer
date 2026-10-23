//! Filesystem publication for command-line hosts.
//!
//! Callers validate and bound their artifacts before publication. This crate
//! owns no game model, authoring format, evaluator or source backend.
use std::{
    collections::BTreeSet,
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
    publish_directory(output, |staging| {
        for (name, bytes) in artifacts {
            validate_basename(name)?;
            write_artifact(&staging.join(name), bytes)?;
        }
        Ok(())
    })
}

/// One borrowed artifact. The publisher never clones its payload.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactFile<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}

/// An immediate child directory containing files only. Arbitrary nested paths
/// cannot be represented by this API.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactDirectory<'a> {
    pub name: &'a str,
    pub files: &'a [ArtifactFile<'a>],
}

/// Aggregate publication bounds, checked before staging or writing any files.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactBundleLimits {
    /// Root files, child directories and child files all consume one entry.
    pub max_entries: usize,
    /// Sum of all file payload lengths, including root and child files.
    pub max_bytes: usize,
}

/// Publish a bounded two-level bundle with one atomic no-replace rename.
///
/// Root and child names are portable ASCII basenames of at most 255 bytes.
/// Windows device names, controls, reserved punctuation and trailing dots/spaces
/// are rejected on every host. Names must be unique ignoring ASCII case within
/// each directory, including file/directory collisions at the root. Both bounds
/// must be nonzero. No user-supplied name can introduce another path component.
///
/// Files are synced before publication. As with [`publish_artifacts`], this
/// guarantees atomic visibility, not persistence after sudden power loss.
pub fn publish_artifact_bundle(
    output: &Path,
    root_files: &[ArtifactFile<'_>],
    directories: &[ArtifactDirectory<'_>],
    limits: ArtifactBundleLimits,
) -> io::Result<()> {
    validate_bundle(root_files, directories, limits)?;
    publish_directory(output, |staging| {
        for file in root_files {
            write_artifact(&staging.join(file.name), file.bytes)?;
        }
        for directory in directories {
            let path = staging.join(directory.name);
            fs::create_dir(&path)?;
            for file in directory.files {
                write_artifact(&path.join(file.name), file.bytes)?;
            }
        }
        Ok(())
    })
}

fn validate_bundle(
    root_files: &[ArtifactFile<'_>],
    directories: &[ArtifactDirectory<'_>],
    limits: ArtifactBundleLimits,
) -> io::Result<()> {
    if limits.max_entries == 0 || limits.max_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid artifact bundle limits",
        ));
    }
    let mut entries = limits.max_entries;
    let mut bytes = limits.max_bytes;
    // Charge lengths before allocating name indexes or examining payloads.
    for count in [root_files.len(), directories.len()]
        .into_iter()
        .chain(directories.iter().map(|directory| directory.files.len()))
    {
        entries = entries.checked_sub(count).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "artifact bundle entry limit")
        })?;
    }
    for file in root_files
        .iter()
        .chain(directories.iter().flat_map(|directory| directory.files))
    {
        bytes = bytes.checked_sub(file.bytes.len()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "artifact bundle byte limit")
        })?;
    }
    let mut root_names = BTreeSet::new();
    for file in root_files {
        insert_portable_name(&mut root_names, file.name)?;
    }
    for directory in directories {
        insert_portable_name(&mut root_names, directory.name)?;
        let mut names = BTreeSet::new();
        for file in directory.files {
            insert_portable_name(&mut names, file.name)?;
        }
    }
    Ok(())
}

fn insert_portable_name(names: &mut BTreeSet<String>, name: &str) -> io::Result<()> {
    if name.len() > 255
        || !name.bytes().all(|byte| (b' '..=b'~').contains(&byte))
        || name.contains(['<', '>', '"', '|', '?', '*'])
        || name.ends_with(['.', ' '])
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "artifact name is not portable",
        ));
    }
    validate_basename(name)?;
    let folded = name.to_ascii_lowercase();
    let stem = folded
        .split('.')
        .next()
        .expect("split always yields a stem")
        .trim_end_matches(' ');
    if matches!(
        stem,
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$" | "clock$"
    ) || (stem.len() == 4
        && (stem.starts_with("com") || stem.starts_with("lpt"))
        && (b'1'..=b'9').contains(&stem.as_bytes()[3]))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "artifact name is a reserved device",
        ));
    }
    if !names.insert(folded) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "artifact names collide",
        ));
    }
    Ok(())
}

fn validate_basename(name: &str) -> io::Result<()> {
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
    Ok(())
}

fn write_artifact(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::options().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn publish_directory(
    output: &Path,
    populate: impl FnOnce(&Path) -> io::Result<()>,
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
    populate(staging.path())?;
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
