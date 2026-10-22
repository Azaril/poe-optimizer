//! Filesystem boundary for offline final-release construction.
use poe_optimizer_authoring::{
    ReleaseBuildLimits, ReleaseSource, build_release, parse_release_manifest,
};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

pub(super) struct Command {
    manifest: PathBuf,
    output: PathBuf,
}

pub(super) fn arguments(mut args: impl Iterator<Item = OsString>) -> Result<Command, &'static str> {
    if args.next().as_deref() != Some(std::ffi::OsStr::new("build")) {
        return Err("expected 'release build'; use --help");
    }
    let mut manifest = None;
    let mut output = None;
    let mut positional_only = false;
    while let Some(arg) = args.next() {
        if !positional_only && arg == "--" {
            positional_only = true;
        } else if !positional_only && arg == "--output" {
            if output.is_some() {
                return Err("--output may only be specified once");
            }
            output = Some(PathBuf::from(
                args.next().ok_or("--output requires a directory")?,
            ));
        } else if !positional_only && arg.as_encoded_bytes().starts_with(b"-") {
            return Err("unknown release option; use '--' before a path beginning with '-'");
        } else if manifest.replace(PathBuf::from(arg)).is_some() {
            return Err("expected one release manifest");
        }
    }
    Ok(Command {
        manifest: manifest.ok_or("expected one release manifest")?,
        output: output.ok_or("release build requires --output")?,
    })
}

fn read(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect '{}': {e}", path.display()))?
        .file_type()
        .is_file()
    {
        return Err(format!(
            "source is not a regular file: '{}'",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(maximum as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("cannot read '{}': {e}", path.display()))?;
    if bytes.len() > maximum {
        return Err(format!(
            "source byte budget exceeded at '{}'",
            path.display()
        ));
    }
    Ok(bytes)
}

pub(super) fn run(command: Command) -> Result<ExitCode, String> {
    let limits = ReleaseBuildLimits::default();
    let manifest_path = command
        .manifest
        .canonicalize()
        .map_err(|e| format!("cannot resolve manifest: {e}"))?;
    let root = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?;
    let manifest_bytes = read(&manifest_path, limits.manifest.max_source_bytes)?;
    let text = std::str::from_utf8(&manifest_bytes).map_err(|_| "manifest is not UTF-8")?;
    let manifest = parse_release_manifest(text, limits).map_err(|e| e.to_string())?;
    let mut left = limits.max_encoded_bytes;
    let mut buffers = Vec::new();
    for source in manifest.sources() {
        let path = root
            .join(source.path())
            .canonicalize()
            .map_err(|e| format!("cannot resolve source '{}': {e}", source.path()))?;
        if !path.starts_with(root) {
            return Err("source resolves outside the manifest directory".into());
        }
        let bytes = read(&path, left)?;
        left -= bytes.len();
        buffers.push(bytes);
    }
    let sources: Vec<_> = manifest
        .sources()
        .iter()
        .zip(&buffers)
        .map(|(spec, bytes)| ReleaseSource {
            path: spec.path(),
            bytes,
        })
        .collect();
    let built = build_release(&manifest, &sources, limits).map_err(|e| e.to_string())?;
    if built.decoded_bytes() * 5 >= limits.max_decoded_bytes * 4 {
        eprintln!(
            "warning: decoded release sources use {} of {} bytes; compact-format migration is required before expanding content",
            built.decoded_bytes(),
            limits.max_decoded_bytes
        );
    }
    poe_optimizer_host::publish_artifacts(&command.output, built.release().artifacts())
        .map_err(|e| format!("cannot publish release: {e}"))?;
    let receipt = built
        .release()
        .artifacts()
        .find(|(name, _)| *name == "release.json")
        .expect("release assembly emits its receipt")
        .1;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(receipt)
        .and_then(|()| stdout.write_all(b"\n"))
        .map_err(|e| format!("cannot write release receipt: {e}"))?;
    Ok(ExitCode::SUCCESS)
}
