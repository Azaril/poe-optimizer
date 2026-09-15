//! Offline host publication. Semantic assembly remains in Import.
use poe_optimizer_import::owned_recipe::{OwnedRecipeLimits, decode_owned_recipe};
use std::{
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Persisted registry, schema, rule and routing recipe JSON.
    input: PathBuf,
    /// New directory for registry/schema/rules/routing/manifest JSON artifacts.
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if !cfg!(any(windows, target_os = "linux")) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic recipe directory publication is supported on Windows and Linux",
        )
        .into());
    }
    let limits = OwnedRecipeLimits::default();
    let mut input = Vec::new();
    File::open(&args.input)?
        .take(limits.max_wire_bytes as u64 + 1)
        .read_to_end(&mut input)?;
    let staged = decode_owned_recipe(&input, limits)?;
    let output = super::destination(&args.output)?;
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "recipe output already exists",
            )
            .into());
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    // All semantic validation and bounded encoding finishes before staging I/O.
    let staging = tempfile::Builder::new()
        .prefix(".owned-recipe-")
        .tempdir_in(output.parent().expect("absolute output"))?;
    for artifact in staged.artifacts() {
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(staging.path().join(artifact.name()))?;
        file.write_all(artifact.bytes())?;
        file.sync_all()?;
    }
    publish_noclobber(staging.path(), &output)?;
    // The old path no longer exists. Disarm cleanup so another process creating
    // that old path cannot have its unrelated directory removed by TempDir Drop.
    let _ = staging.keep();
    let manifest = staged
        .artifacts()
        .iter()
        .find(|a| a.name() == "manifest.json")
        .expect("fixed artifact set");
    let mut stdout = io::stdout().lock();
    stdout.write_all(manifest.bytes())?;
    stdout.write_all(b"\n")?;
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
        "atomic recipe publication unavailable",
    ))
}

#[cfg(all(test, any(windows, target_os = "linux")))]
mod tests {
    use super::*;
    #[test]
    fn publication_race_never_replaces_an_existing_empty_directory() {
        let parent = tempfile::tempdir().unwrap();
        let from = parent.path().join("staging");
        let to = parent.path().join("published");
        fs::create_dir(&from).unwrap();
        fs::write(from.join("manifest.json"), b"complete").unwrap();
        assert!(!to.exists());
        // Simulate a concurrent publisher after the caller's absence check.
        fs::create_dir(&to).unwrap();
        assert!(publish_noclobber(&from, &to).is_err());
        assert_eq!(fs::read_dir(&to).unwrap().count(), 0);
        assert_eq!(fs::read(from.join("manifest.json")).unwrap(), b"complete");
    }
    #[test]
    fn publication_has_one_winner_and_keeps_the_losing_staged_bundle() {
        let parent = tempfile::tempdir().unwrap();
        let a = parent.path().join("a");
        let b = parent.path().join("b");
        let output = parent.path().join("output");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        fs::write(a.join("manifest.json"), b"first").unwrap();
        fs::write(b.join("manifest.json"), b"second").unwrap();
        publish_noclobber(&a, &output).unwrap();
        assert!(publish_noclobber(&b, &output).is_err());
        assert_eq!(fs::read(output.join("manifest.json")).unwrap(), b"first");
        assert_eq!(fs::read(b.join("manifest.json")).unwrap(), b"second");
    }
}
