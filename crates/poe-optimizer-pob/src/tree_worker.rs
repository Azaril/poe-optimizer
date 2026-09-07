//! Process isolation for offline data extraction, separate from calculation protocol.
use crate::{
    supervisor::ReapedChild,
    tree_data::{SUPPORTED_TREE_VERSION, TreeDataSnapshot, extract_pinned_tree},
};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const MAX_TREE_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ERROR_BYTES: u64 = 64 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("isolated tree extraction failed: {0}")]
pub struct TreeWorkerError(pub String);
fn error(value: impl std::fmt::Display) -> TreeWorkerError {
    TreeWorkerError(value.to_string())
}

/// A startup-inclusive deadline and bounded private artifact; every child is reaped.
/// The executable must implement the hidden `__tree-worker` command in our CLI.
pub fn extract_tree(
    executable: &Path,
    pob: &Path,
    version: &str,
    timeout: Duration,
) -> Result<TreeDataSnapshot, TreeWorkerError> {
    if timeout.is_zero() || version != SUPPORTED_TREE_VERSION {
        return Err(error("unsupported version or zero timeout"));
    }
    let started = Instant::now();
    let deadline = started
        .checked_add(timeout)
        .ok_or_else(|| error("timeout is too large"))?;
    let scratch = tempfile::Builder::new()
        .prefix("poe-tree-")
        .tempdir()
        .map_err(error)?;
    let artifact = scratch.path().join("tree.json");
    let errors = scratch.path().join("error.txt");
    let mut command = Command::new(executable);
    command
        .arg("__tree-worker")
        .arg("--pob")
        .arg(pob)
        .arg("--tree-version")
        .arg(version)
        .arg("--artifact")
        .arg(&artifact)
        .arg("--error-file")
        .arg(&errors);
    supervise(
        command,
        &artifact,
        &errors,
        deadline,
        MAX_TREE_ARTIFACT_BYTES,
    )?;
    let bytes = read_bounded(&artifact, MAX_TREE_ARTIFACT_BYTES)?;
    let snapshot: TreeDataSnapshot = serde_json::from_slice(&bytes).map_err(error)?;
    snapshot.validate_source_identity().map_err(error)?;
    if Instant::now() >= deadline {
        return Err(error("deadline exceeded while reading extracted data"));
    }
    Ok(snapshot)
}

/// Worker entrypoint. It is intentionally separate from the application's calculation VM.
pub fn worker(
    pob: &Path,
    version: &str,
    artifact: &Path,
    error_file: &Path,
) -> Result<(), TreeWorkerError> {
    let result = (|| {
        let snapshot = extract_pinned_tree(pob, version).map_err(error)?;
        let mut output = File::create_new(artifact).map_err(error)?;
        serde_json::to_writer(&mut output, &snapshot).map_err(error)?;
        output.flush().map_err(error)?;
        Ok(())
    })();
    if let Err(error) = &result {
        let message = format!("{error}");
        if let Ok(mut output) = File::create_new(error_file) {
            let _ = output
                .write_all(&message.as_bytes()[..message.len().min(MAX_ERROR_BYTES as usize)]);
        }
    }
    result
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, TreeWorkerError> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(error)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 > limit {
        return Err(error("worker artifact exceeds byte limit"));
    }
    Ok(bytes)
}
fn within_limit(path: &Path, limit: u64) -> Result<(), TreeWorkerError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit => {
            Err(error("worker artifact is not a bounded ordinary file"))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(error(err)),
    }
}
fn supervise(
    mut command: Command,
    artifact: &Path,
    errors: &Path,
    deadline: Instant,
    limit: u64,
) -> Result<(), TreeWorkerError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    if Instant::now() >= deadline {
        return Err(error("deadline exceeded before worker start"));
    }
    let mut child = ReapedChild::new(command.spawn().map_err(error)?);
    let outcome = (|| {
        loop {
            within_limit(artifact, limit)?;
            within_limit(errors, MAX_ERROR_BYTES)?;
            if Instant::now() >= deadline {
                return Err(error("worker deadline exceeded"));
            }
            if let Some(status) = child.try_wait().map_err(error)? {
                // Check again after exit so a final write cannot race the admission check.
                within_limit(artifact, limit)?;
                within_limit(errors, MAX_ERROR_BYTES)?;
                return if status.success() {
                    Ok(())
                } else {
                    let detail = read_bounded(errors, MAX_ERROR_BYTES)
                        .ok()
                        .map(|b| String::from_utf8_lossy(&b).into_owned())
                        .unwrap_or_else(|| format!("worker exited with {status}"));
                    Err(error(detail))
                };
            }
            std::thread::sleep(
                Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    })();
    let cleanup = child.stop();
    match (outcome, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (result, Err(cleanup)) => Err(error(format!(
            "{}; child cleanup failed: {cleanup}",
            result
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "extraction finished".into())
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "child fixture invoked by isolation tests"]
    fn extraction_worker_fixture() {
        let mode = std::env::var("POE_TREE_TEST_MODE").unwrap();
        let artifact = std::env::var("POE_TREE_TEST_ARTIFACT").unwrap();
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(30)),
            "oversize" => fs::write(artifact, vec![b'x'; 1025]).unwrap(),
            "success" => fs::write(artifact, b"{}").unwrap(),
            _ => panic!("unknown fixture mode"),
        }
    }
    fn run(mode: &str, timeout: Duration) -> Result<(), TreeWorkerError> {
        let scratch = tempfile::tempdir().unwrap();
        let artifact = scratch.path().join("artifact");
        let errors = scratch.path().join("errors");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "tree_worker::tests::extraction_worker_fixture",
                "--nocapture",
            ])
            .env("POE_TREE_TEST_MODE", mode)
            .env("POE_TREE_TEST_ARTIFACT", &artifact);
        supervise(command, &artifact, &errors, Instant::now() + timeout, 1024)
    }
    #[test]
    fn worker_timeout_stops_and_reaps_child() {
        let start = Instant::now();
        let result = run("sleep", Duration::from_millis(200));
        assert!(result.unwrap_err().to_string().contains("deadline"));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn worker_exit_still_checks_artifact_size_and_success() {
        assert!(
            run("oversize", Duration::from_secs(10))
                .unwrap_err()
                .to_string()
                .contains("bounded")
        );
        assert!(run("success", Duration::from_secs(10)).is_ok());
    }
}
