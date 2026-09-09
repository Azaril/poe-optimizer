//! Supervised offline extraction of a complete game-data package and its evidence.
//! This is an optional reference process, never part of native candidate evaluation.
use crate::{
    game_data::{ExtractedGameData, extract_pinned_game_data},
    supervisor::ReapedChild,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Private canonical package/evidence envelope. The contained portable package
/// retains its independent 32 MiB loader limit; 1 MiB allows bounded source evidence.
pub const MAX_GAME_DATA_ARTIFACT_BYTES: u64 = 33 * 1024 * 1024;
const MAX_ERROR_BYTES: u64 = 64 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("isolated game-data extraction failed: {0}")]
pub struct GameDataWorkerError(pub String);
fn error(value: impl std::fmt::Display) -> GameDataWorkerError {
    GameDataWorkerError(value.to_string())
}

/// Includes startup, extraction, artifact decoding and evidence validation in one
/// deadline. The executable must implement our hidden `__game-data-worker` CLI.
pub fn extract_game_data(
    executable: &Path,
    pob: &Path,
    timeout: Duration,
) -> Result<ExtractedGameData, GameDataWorkerError> {
    if timeout.is_zero() {
        return Err(error("game-data extraction timeout must be positive"));
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| error("timeout is too large"))?;
    let scratch = tempfile::Builder::new()
        .prefix("poe-game-data-")
        .tempdir()
        .map_err(error)?;
    let artifact = scratch.path().join("extracted.json");
    let errors = scratch.path().join("error.txt");
    let mut command = Command::new(executable);
    command
        .arg("__game-data-worker")
        .arg("--pob")
        .arg(pob)
        .arg("--artifact")
        .arg(&artifact)
        .arg("--error-file")
        .arg(&errors);
    supervise(
        command,
        &artifact,
        &errors,
        deadline,
        MAX_GAME_DATA_ARTIFACT_BYTES,
    )?;
    check_deadline(deadline)?;
    let bytes = read_bounded(&artifact, MAX_GAME_DATA_ARTIFACT_BYTES)?;
    decode_envelope(&bytes, deadline)
}

/// Hidden worker entry point. Existing/colliding destinations reject before any
/// Lua work. Successful artifacts use the strict private canonical JSON encoding.
pub fn worker(pob: &Path, artifact: &Path, error_file: &Path) -> Result<(), GameDataWorkerError> {
    validate_destinations(artifact, error_file)?;
    let result = (|| {
        let extracted = extract_pinned_game_data(pob).map_err(error)?;
        extracted.validate().map_err(error)?;
        let bytes = serde_json::to_vec(&extracted).map_err(error)?;
        if bytes.len() as u64 > MAX_GAME_DATA_ARTIFACT_BYTES {
            return Err(error("worker artifact exceeds byte limit"));
        }
        let mut output = File::create_new(artifact).map_err(error)?;
        output.write_all(&bytes).map_err(error)?;
        output.flush().map_err(error)?;
        Ok(())
    })();
    if let Err(failure) = &result {
        let message = failure.to_string();
        if let Ok(mut output) = File::create_new(error_file) {
            let mut end = message.len().min(MAX_ERROR_BYTES as usize);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            let _ = output.write_all(&message.as_bytes()[..end]);
        }
    }
    result
}
fn check_deadline(deadline: Instant) -> Result<(), GameDataWorkerError> {
    if Instant::now() >= deadline {
        Err(error("game-data worker deadline exceeded"))
    } else {
        Ok(())
    }
}
fn decode_envelope(
    bytes: &[u8],
    deadline: Instant,
) -> Result<ExtractedGameData, GameDataWorkerError> {
    check_deadline(deadline)?;
    if bytes.len() as u64 > MAX_GAME_DATA_ARTIFACT_BYTES {
        return Err(error("worker artifact exceeds byte limit"));
    }
    let extracted: ExtractedGameData = decode_canonical(bytes)?;
    extracted.validate().map_err(error)?;
    check_deadline(deadline)?;
    Ok(extracted)
}
/// Worker bytes are generated with this exact serializer, rather than a public
/// authoring format. Requiring a byte-for-byte round trip rejects duplicate map
/// keys, aliases and fields discarded by nested source enums during decoding.
fn decode_canonical<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> Result<T, GameDataWorkerError> {
    let value: T = serde_json::from_slice(bytes).map_err(error)?;
    if serde_json::to_vec(&value).map_err(error)? != bytes {
        return Err(error(
            "worker envelope is not canonical or contains discarded/ambiguous fields",
        ));
    }
    Ok(value)
}
fn destination(path: &Path) -> Result<PathBuf, GameDataWorkerError> {
    match fs::symlink_metadata(path) {
        Ok(_) => return Err(error("worker output already exists")),
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {}
        Err(failure) => return Err(error(failure)),
    }
    let name = path
        .file_name()
        .ok_or_else(|| error("worker output must name a new file"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let resolved = parent.canonicalize().map_err(error)?.join(name);
    #[cfg(windows)]
    let resolved = PathBuf::from(resolved.to_string_lossy().to_lowercase());
    Ok(resolved)
}
fn validate_destinations(artifact: &Path, errors: &Path) -> Result<(), GameDataWorkerError> {
    if destination(artifact)? == destination(errors)? {
        return Err(error(
            "worker artifact and error output must use different paths",
        ));
    }
    Ok(())
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, GameDataWorkerError> {
    within_limit(path, limit)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(error)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 > limit {
        return Err(error("worker artifact exceeds byte limit"));
    }
    within_limit(path, limit)?;
    Ok(bytes)
}
fn within_limit(path: &Path, limit: u64) -> Result<(), GameDataWorkerError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit => {
            Err(error("worker artifact is not a bounded ordinary file"))
        }
        Ok(_) => Ok(()),
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(failure) => Err(error(failure)),
    }
}
fn supervise(
    mut command: Command,
    artifact: &Path,
    errors: &Path,
    deadline: Instant,
    limit: u64,
) -> Result<(), GameDataWorkerError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    check_deadline(deadline)?;
    let mut child = ReapedChild::new(command.spawn().map_err(error)?);
    let outcome = (|| {
        loop {
            within_limit(artifact, limit)?;
            within_limit(errors, MAX_ERROR_BYTES)?;
            check_deadline(deadline)?;
            if let Some(status) = child.try_wait().map_err(error)? {
                // A final write must not race the running-process admission check.
                within_limit(artifact, limit)?;
                within_limit(errors, MAX_ERROR_BYTES)?;
                check_deadline(deadline)?;
                if status.success() {
                    return Ok(());
                }
                let detail = read_bounded(errors, MAX_ERROR_BYTES)
                    .ok()
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .filter(|message| !message.is_empty())
                    .unwrap_or_else(|| format!("worker exited with {status}"));
                return Err(error(detail));
            }
            std::thread::sleep(
                Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    })();
    let cleanup = child.stop();
    match (outcome, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(failure), Ok(())) => Err(failure),
        (outcome, Err(cleanup)) => Err(error(format!(
            "{}; child cleanup failed: {cleanup}",
            outcome
                .err()
                .map(|failure| failure.to_string())
                .unwrap_or_else(|| "extraction finished".into())
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    #[ignore = "child fixture invoked by game-data isolation tests"]
    fn game_data_worker_fixture() {
        let mode = std::env::var("POE_GAME_DATA_TEST_MODE").unwrap();
        let artifact = PathBuf::from(std::env::var_os("POE_GAME_DATA_TEST_ARTIFACT").unwrap());
        let errors = PathBuf::from(std::env::var_os("POE_GAME_DATA_TEST_ERRORS").unwrap());
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(30)),
            "oversize" => fs::write(artifact, vec![b'x'; 1025]).unwrap(),
            "oversize_error" => {
                fs::write(errors, vec![b'x'; MAX_ERROR_BYTES as usize + 1]).unwrap()
            }
            "directory" => fs::create_dir(artifact).unwrap(),
            "success" => fs::write(artifact, b"{}").unwrap(),
            "failure" => {
                fs::write(errors, b"explicit fixture failure").unwrap();
                std::process::exit(7);
            }
            _ => panic!("unknown fixture mode"),
        }
    }
    fn run(mode: &str, timeout: Duration) -> Result<(), GameDataWorkerError> {
        let scratch = tempfile::tempdir().unwrap();
        let artifact = scratch.path().join("artifact");
        let errors = scratch.path().join("errors");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "game_data_worker::tests::game_data_worker_fixture",
                "--nocapture",
            ])
            .env("POE_GAME_DATA_TEST_MODE", mode)
            .env("POE_GAME_DATA_TEST_ARTIFACT", &artifact)
            .env("POE_GAME_DATA_TEST_ERRORS", &errors);
        supervise(command, &artifact, &errors, Instant::now() + timeout, 1024)
    }
    #[test]
    fn deadline_stops_and_reaps_worker_and_rejects_starting_late() {
        let started = Instant::now();
        assert!(
            run("sleep", Duration::from_millis(200))
                .unwrap_err()
                .to_string()
                .contains("deadline")
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(
            run("success", Duration::ZERO)
                .unwrap_err()
                .to_string()
                .contains("deadline")
        );
        assert!(
            extract_game_data(Path::new("missing"), Path::new("missing"), Duration::ZERO)
                .unwrap_err()
                .to_string()
                .contains("positive")
        );
    }
    #[test]
    fn final_artifacts_and_errors_remain_bounded_and_ordinary() {
        for mode in ["oversize", "oversize_error", "directory"] {
            assert!(
                run(mode, Duration::from_secs(10))
                    .unwrap_err()
                    .to_string()
                    .contains("bounded"),
                "{mode}"
            );
        }
        assert!(run("success", Duration::from_secs(10)).is_ok());
        assert!(
            run("failure", Duration::from_secs(10))
                .unwrap_err()
                .to_string()
                .contains("explicit fixture failure")
        );
    }
    #[test]
    fn canonical_protocol_rejects_ambiguous_discarded_and_noncanonical_json() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Envelope {
            fields: BTreeMap<u32, f64>,
        }
        for bytes in [
            br#"{"fields":{"1":2.0,"1":3.0}}"#.as_slice(),
            br#"{"fields":{"01":2.0}}"#,
            br#"{"fields":{"1":2.0},"ignored":true}"#,
            br#"{ "fields":{"1":2.0}}"#,
        ] {
            assert!(decode_canonical::<Envelope>(bytes).is_err());
        }
        let bytes = br#"{"fields":{"1":2.0}}"#;
        assert_eq!(decode_canonical::<Envelope>(bytes).unwrap().fields[&1], 2.0);
    }
    #[test]
    fn malformed_envelopes_and_expired_read_deadlines_reject() {
        let deadline = Instant::now() + Duration::from_secs(5);
        assert!(decode_envelope(b"{}", deadline).is_err());
        assert!(decode_envelope(b"[]", deadline).is_err());
        assert!(
            decode_envelope(b"{}", Instant::now())
                .unwrap_err()
                .to_string()
                .contains("deadline")
        );
        let too_large = vec![b' '; MAX_GAME_DATA_ARTIFACT_BYTES as usize + 1];
        assert!(
            decode_envelope(&too_large, deadline)
                .unwrap_err()
                .to_string()
                .contains("byte limit")
        );
    }
    #[test]
    fn existing_and_colliding_outputs_reject_before_source_work() {
        let scratch = tempfile::tempdir().unwrap();
        let artifact = scratch.path().join("artifact");
        let errors = scratch.path().join("errors");
        let missing_source = scratch.path().join("no-source-checkout");
        assert!(
            worker(&missing_source, &artifact, &artifact)
                .unwrap_err()
                .to_string()
                .contains("different paths")
        );
        assert!(!artifact.exists());
        fs::write(&artifact, b"preserve artifact").unwrap();
        assert!(
            worker(&missing_source, &artifact, &errors)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
        assert_eq!(fs::read(&artifact).unwrap(), b"preserve artifact");
        assert!(!errors.exists());
        let other = scratch.path().join("other");
        fs::write(&errors, b"preserve error").unwrap();
        assert!(
            worker(&missing_source, &other, &errors)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
        assert!(!other.exists());
        assert_eq!(fs::read(&errors).unwrap(), b"preserve error");
    }
}
