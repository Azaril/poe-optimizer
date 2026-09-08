//! Git-independent integrity checks for the pinned PoB evaluator source.
//!
//! The committed manifest covers all tracked Lua files below `src/` and
//! `runtime/lua/`, plus the root application manifest. It intentionally excludes
//! graphics and development/export inputs. All tree.lua files must be present;
//! the upstream fallback that converts tree JSON is therefore never needed.
//!
//! Only CRLF is normalized to LF. Other bytes, including a BOM or lone CR, remain
//! significant. This permits normal Git checkouts on Windows and Linux to share
//! the same fingerprint without accepting semantic source modifications.

use std::{
    borrow::Cow,
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const UPSTREAM_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const MANIFEST: &str = include_str!("../data/pob-source-manifest.json");
const MAX_MANIFEST_FILES: usize = 4_096;
const MAX_NORMALIZED_FILE_BYTES: usize = 16 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 20_000;
const LUA_ROOTS: [&str; 2] = ["src", "runtime/lua"];
const ABSENT_PATHS: [&str; 3] = ["src/first.run", "src/installed.cfg", "src/manifest.xml"];

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("invalid embedded PoB source manifest: {0}")]
    Manifest(String),
    #[error("cannot inspect PoB source {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("PoB source path escapes its runtime root: {0}")]
    OutsideRoot(PathBuf),
    #[error("PoB source entry must be an ordinary file or directory, without symbolic links: {0}")]
    UnsupportedEntry(PathBuf),
    #[error("PoB source {path} exceeds its {limit}-byte read limit")]
    TooLarge { path: String, limit: usize },
    #[error("PoB source {path} is not UTF-8: {source}")]
    Utf8 {
        path: String,
        #[source]
        source: std::str::Utf8Error,
    },
    #[error("PoB source mismatch in {path}: {detail}")]
    Mismatch { path: String, detail: String },
    #[error("unmanifested Lua source can shadow a pinned module: {0}")]
    UnexpectedLua(PathBuf),
    #[error("PoB startup override must be absent: {0}")]
    StartupOverride(String),
    #[error("PoB source inventory exceeds its {MAX_DIRECTORY_ENTRIES}-entry limit")]
    InventoryTooLarge,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    upstream_revision: String,
    normalization: String,
    lua_roots: Vec<String>,
    absent_paths: Vec<String>,
    files: Vec<SourceFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    bytes: usize,
    sha256: String,
}

/// Verify every pinned source file and return the source manifest's SHA-256.
///
/// No Git executable or repository metadata is needed. The manifest fingerprint
/// normalizes its own CRLF line endings too, so checkout conventions do not alter
/// runtime identity. Hash changes represent changes to the committed manifest.
///
/// Additional Lua files and startup overrides are rejected because they can
/// change module lookup or startup behavior despite all pinned files matching.
/// Non-Lua assets and repository documentation do not affect this check.
pub fn verify(root: &Path) -> Result<String, SourceError> {
    let manifest: Manifest =
        serde_json::from_str(MANIFEST).map_err(|error| SourceError::Manifest(error.to_string()))?;
    verify_manifest(root, &manifest)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(normalize(MANIFEST).as_bytes())
    ))
}

/// Hash of the compiled normalized manifest, without filesystem access.
pub fn manifest_sha256() -> String {
    format!("{:x}", Sha256::digest(normalize(MANIFEST).as_bytes()))
}
/// Expected normalized hash for a path in the compiled, validated manifest.
pub fn expected_file_sha256(path: &str) -> Result<String, SourceError> {
    let manifest: Manifest =
        serde_json::from_str(MANIFEST).map_err(|e| SourceError::Manifest(e.to_string()))?;
    validate_manifest(&manifest)?;
    manifest
        .files
        .into_iter()
        .find(|file| file.path == path)
        .map(|file| file.sha256)
        .ok_or_else(|| SourceError::Manifest(format!("source path is not manifested: {path}")))
}
/// Read and authenticate the exact normalized bytes an offline extractor will use.
/// This checks one file; callers separately call `verify` for the full inventory.
pub fn read_verified_text(root: &Path, path: &str) -> Result<String, SourceError> {
    let manifest: Manifest =
        serde_json::from_str(MANIFEST).map_err(|e| SourceError::Manifest(e.to_string()))?;
    validate_manifest(&manifest)?;
    let file = manifest
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| SourceError::Manifest(format!("source path is not manifested: {path}")))?;
    let root = fs::canonicalize(root).map_err(|e| io_error(root, e))?;
    verify_file(&root, file)
}
fn verify_manifest(root: &Path, manifest: &Manifest) -> Result<(), SourceError> {
    validate_manifest(manifest)?;
    let root = fs::canonicalize(root).map_err(|source| io_error(root, source))?;
    for source in &manifest.files {
        verify_file(&root, source)?;
    }
    for relative in &manifest.absent_paths {
        let path = root.join(relative);
        match fs::symlink_metadata(&path) {
            Ok(_) => return Err(SourceError::StartupOverride(relative.clone())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&path, error)),
        }
    }
    reject_additional_lua(&root, manifest)
}

fn validate_manifest(manifest: &Manifest) -> Result<(), SourceError> {
    let invalid = |message: &str| SourceError::Manifest(message.into());
    if manifest.schema_version != 1
        || manifest.upstream_revision != UPSTREAM_REVISION
        || manifest.normalization != "utf8_crlf_to_lf"
        || manifest.lua_roots != LUA_ROOTS
        || manifest.absent_paths != ABSENT_PATHS
    {
        return Err(invalid(
            "unsupported schema, revision, normalization or source scope",
        ));
    }
    if manifest.files.is_empty() || manifest.files.len() > MAX_MANIFEST_FILES {
        return Err(invalid("unexpected number of pinned files"));
    }
    let mut paths = BTreeSet::new();
    for source in &manifest.files {
        let relative = Path::new(&source.path);
        if source.path.contains(['\\', ':'])
            || source
                .path
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(invalid(
                "source paths must be portable, normalized relative paths",
            ));
        }
        let in_scope = source.path == "manifest.xml"
            || (source.path.ends_with(".lua")
                && LUA_ROOTS
                    .iter()
                    .any(|prefix| source.path.starts_with(&format!("{prefix}/"))));
        if !in_scope || !paths.insert(source.path.as_str()) {
            return Err(invalid("duplicate or out-of-scope source path"));
        }
        if source.bytes > MAX_NORMALIZED_FILE_BYTES
            || source.sha256.len() != 64
            || !source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid("invalid source length or SHA-256"));
        }
    }
    if !paths.contains("manifest.xml") {
        return Err(invalid("root application manifest is missing"));
    }
    Ok(())
}

fn verify_file(root: &Path, source: &SourceFile) -> Result<String, SourceError> {
    let path = root.join(&source.path);
    let resolved = fs::canonicalize(&path).map_err(|error| io_error(&path, error))?;
    if !resolved.starts_with(root) {
        return Err(SourceError::OutsideRoot(path));
    }
    // Check file kind before opening, so a FIFO cannot block the verifier here.
    let metadata = fs::metadata(&resolved).map_err(|error| io_error(&path, error))?;
    if !metadata.is_file() {
        return Err(SourceError::UnsupportedEntry(path));
    }
    let file = File::open(&resolved).map_err(|error| io_error(&path, error))?;
    let metadata = file.metadata().map_err(|error| io_error(&path, error))?;
    if !metadata.is_file() {
        return Err(SourceError::UnsupportedEntry(path));
    }
    // A CRLF checkout can be at most twice the normalized byte length.
    let limit = source.bytes * 2;
    if metadata.len() > limit as u64 {
        return Err(SourceError::TooLarge {
            path: source.path.clone(),
            limit,
        });
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(&path, error))?;
    if bytes.len() > limit {
        return Err(SourceError::TooLarge {
            path: source.path.clone(),
            limit,
        });
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| SourceError::Utf8 {
        path: source.path.clone(),
        source: error,
    })?;
    let normalized = normalize(text);
    if normalized.len() != source.bytes {
        return Err(SourceError::Mismatch {
            path: source.path.clone(),
            detail: format!(
                "expected {} normalized bytes, found {}",
                source.bytes,
                normalized.len()
            ),
        });
    }
    let actual = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    if actual != source.sha256 {
        return Err(SourceError::Mismatch {
            path: source.path.clone(),
            detail: format!("expected SHA-256 {}, found {actual}", source.sha256),
        });
    }
    Ok(normalized.into_owned())
}

fn reject_additional_lua(root: &Path, manifest: &Manifest) -> Result<(), SourceError> {
    let expected: BTreeSet<&str> = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let mut directories: Vec<PathBuf> = LUA_ROOTS.iter().map(|path| root.join(path)).collect();
    let mut entries_seen = 0;
    while let Some(directory) = directories.pop() {
        let metadata =
            fs::symlink_metadata(&directory).map_err(|error| io_error(&directory, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(SourceError::UnsupportedEntry(directory));
        }
        for entry in fs::read_dir(&directory).map_err(|error| io_error(&directory, error))? {
            entries_seen += 1;
            if entries_seen > MAX_DIRECTORY_ENTRIES {
                return Err(SourceError::InventoryTooLarge);
            }
            let entry = entry.map_err(|error| io_error(&directory, error))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|error| io_error(&path, error))?;
            if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
                return Err(SourceError::UnsupportedEntry(path));
            }
            if kind.is_dir() {
                directories.push(path);
            } else if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("lua"))
            {
                let relative = path
                    .strip_prefix(root)
                    .expect("directory traversal stays beneath the root")
                    .to_str()
                    .ok_or_else(|| SourceError::UnexpectedLua(path.clone()))?
                    .replace('\\', "/");
                if !expected.contains(relative.as_str()) {
                    return Err(SourceError::UnexpectedLua(path));
                }
            }
        }
    }
    Ok(())
}

fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

fn io_error(path: &Path, source: io::Error) -> SourceError {
    SourceError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Manifest) {
        let directory = tempfile::tempdir().unwrap();
        let content = [
            ("manifest.xml", "<PoBVersion/>\n"),
            ("src/module.lua", "return 42\n"),
            ("runtime/lua/base64.lua", "return {}\n"),
        ];
        let mut files = Vec::new();
        for (path, text) in content {
            let destination = directory.path().join(path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::write(destination, text).unwrap();
            files.push(SourceFile {
                path: path.into(),
                bytes: text.len(),
                sha256: format!("{:x}", Sha256::digest(text)),
            });
        }
        let manifest = Manifest {
            schema_version: 1,
            upstream_revision: UPSTREAM_REVISION.into(),
            normalization: "utf8_crlf_to_lf".into(),
            lua_roots: LUA_ROOTS.map(str::to_owned).into(),
            absent_paths: ABSENT_PATHS.map(str::to_owned).into(),
            files,
        };
        (directory, manifest)
    }

    #[test]
    fn accepts_lf_and_crlf_packaged_sources_without_git() {
        let (directory, manifest) = fixture();
        verify_manifest(directory.path(), &manifest).unwrap();
        for file in &manifest.files {
            let path = directory.path().join(&file.path);
            let text = fs::read_to_string(&path).unwrap().replace('\n', "\r\n");
            fs::write(path, text).unwrap();
        }
        verify_manifest(directory.path(), &manifest).unwrap();
        assert!(!directory.path().join(".git").exists());
    }

    #[test]
    fn detects_changed_and_missing_source() {
        let (directory, manifest) = fixture();
        let path = directory.path().join("src/module.lua");
        fs::write(&path, "return 43\n").unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::Mismatch { .. })
        ));
        fs::remove_file(path).unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::Io { .. })
        ));
    }

    #[test]
    fn rejects_unmanifested_lua_that_could_shadow_a_runtime_module() {
        let (directory, manifest) = fixture();
        fs::write(directory.path().join("src/base64.lua"), "return {}\n").unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::UnexpectedLua(_))
        ));
    }

    #[test]
    fn rejects_ignored_startup_overrides() {
        let (directory, manifest) = fixture();
        fs::write(directory.path().join("src/installed.cfg"), "").unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::StartupOverride(_))
        ));
    }

    #[test]
    fn rejects_path_escape_in_manifest_before_reading_source() {
        let (directory, mut manifest) = fixture();
        manifest.files[1].path = "src/../../outside.lua".into();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::Manifest(_))
        ));
    }

    #[test]
    fn bounds_reads_and_rejects_invalid_utf8() {
        let (directory, manifest) = fixture();
        let path = directory.path().join("src/module.lua");
        fs::write(&path, vec![b'x'; manifest.files[1].bytes * 2 + 1]).unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::TooLarge { .. })
        ));
        fs::write(path, b"return\xff42\n").unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::Utf8 { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_source_symlink_outside_the_runtime_root() {
        let (directory, manifest) = fixture();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("module.lua");
        fs::write(&target, "return 42\n").unwrap();
        let source = directory.path().join("src/module.lua");
        fs::remove_file(&source).unwrap();
        std::os::unix::fs::symlink(target, source).unwrap();
        assert!(matches!(
            verify_manifest(directory.path(), &manifest),
            Err(SourceError::OutsideRoot(_))
        ));
    }
    #[test]
    fn verified_read_returns_exact_normalized_bytes_and_rejects_later_edits() {
        let (directory, manifest) = fixture();
        verify_manifest(directory.path(), &manifest).unwrap();
        let canonical_root = fs::canonicalize(directory.path()).unwrap();
        let record = &manifest.files[1];
        let path = directory.path().join(&record.path);
        fs::write(&path, "return 42\r\n").unwrap();
        assert_eq!(verify_file(&canonical_root, record).unwrap(), "return 42\n");
        fs::write(&path, "return 43\n").unwrap();
        assert!(matches!(
            verify_file(&canonical_root, record),
            Err(SourceError::Mismatch { .. })
        ));
        assert_eq!(
            manifest_sha256(),
            format!("{:x}", Sha256::digest(normalize(MANIFEST).as_bytes()))
        );
        assert!(expected_file_sha256("../outside.lua").is_err());
    }
}
