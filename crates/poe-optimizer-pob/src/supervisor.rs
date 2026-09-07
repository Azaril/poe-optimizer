//! Supervision of one fresh evaluator worker, including process startup in its deadline.

use std::{
    fmt,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, absolute},
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use poe_optimizer_core::{
    EvaluationSnapshot, MAX_WIRE_BYTES, PROTOCOL_VERSION, WorkerHello, WorkerRequest,
    WorkerResponse,
};
use tempfile::TempDir;
use thiserror::Error;

use crate::import::MAX_XML_BYTES;

const REQUEST_ID: u64 = 1;
const MAX_HELLO_BYTES: usize = 64 * 1024;
pub const MAX_STDERR_BYTES: usize = 64 * 1024;
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// A typed failure with bounded diagnostic output from the worker.
#[derive(Debug)]
pub struct SupervisorError {
    pub kind: SupervisorErrorKind,
    pub stderr: String,
    pub stderr_truncated: bool,
    /// A cleanup error must not hide the failure that first stopped the evaluation.
    pub cleanup_error: Option<String>,
}

impl fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if !self.stderr.is_empty() {
            let qualifier = if self.stderr_truncated {
                " (truncated)"
            } else {
                ""
            };
            write!(f, "\nworker stderr{qualifier}:\n{}", self.stderr)?;
        }
        if let Some(error) = &self.cleanup_error {
            write!(f, "\nworker cleanup: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SupervisorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

impl From<SupervisorErrorKind> for SupervisorError {
    fn from(kind: SupervisorErrorKind) -> Self {
        Self {
            kind,
            stderr: String::new(),
            stderr_truncated: false,
            cleanup_error: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SupervisorErrorKind {
    #[error("the evaluator timeout must be positive and fit the monotonic clock")]
    InvalidTimeout,
    #[error("build XML exceeds the {MAX_XML_BYTES}-byte input limit")]
    InputTooLarge,
    #[error("cannot resolve {name} path: {source}")]
    Path {
        name: &'static str,
        source: io::Error,
    },
    #[error("cannot serialize the worker request: {0}")]
    Serialize(serde_json::Error),
    #[error("worker {stage} exceeds its {limit}-byte protocol limit")]
    OutputTooLarge { stage: &'static str, limit: usize },
    #[error("cannot start evaluator worker: {0}")]
    Spawn(io::Error),
    #[error("cannot start {name} I/O thread: {source}")]
    ThreadSpawn {
        name: &'static str,
        source: io::Error,
    },
    #[error("evaluator {stream} failed: {source}")]
    Io {
        stream: &'static str,
        source: io::Error,
    },
    #[error("evaluator worker {stage} is not a complete JSON line")]
    IncompleteLine { stage: &'static str },
    #[error("invalid evaluator {stage} JSON: {source}")]
    InvalidJson {
        stage: &'static str,
        source: serde_json::Error,
    },
    #[error("worker {stage} uses protocol {actual}; expected {PROTOCOL_VERSION}")]
    ProtocolVersion { stage: &'static str, actual: u32 },
    #[error("worker response has request ID {actual}; expected {REQUEST_ID}")]
    RequestId { actual: u64 },
    #[error("worker wrote additional stdout after its response")]
    TrailingOutput,
    #[error("worker protocol messages arrived out of order")]
    MessageOrder,
    #[error("evaluator timed out after {timeout:?}, including startup")]
    Timeout { timeout: Duration },
    #[error("evaluator worker exited unsuccessfully: {status}")]
    Exit { status: ExitStatus },
    #[error("evaluator worker failed ({code}): {message}")]
    Worker { code: String, message: String },
    #[error("an evaluator I/O thread panicked")]
    ThreadPanicked,
}

/// Evaluate one build in a new, isolated CLI worker process.
///
/// The executable must implement `__worker --pob <root> --scratch <path>` and JSONL
/// protocol. The worker receives an absolute PoB root and runs from its `src`
/// directory. The supervisor owns scratch storage until process and pipe cleanup
/// finish, including when the worker is killed. No live Lua state is shared.
///
/// Separate I/O threads keep a full stdin pipe or verbose stderr from preventing
/// timeout enforcement. Workers must not launch descendants that inherit these
/// pipes; the supervised process is the lifetime boundary for this adapter.
pub fn evaluate(
    executable: &Path,
    pob_root: &Path,
    xml: &str,
    timeout: Duration,
) -> Result<EvaluationSnapshot, SupervisorError> {
    evaluate_with_options(
        executable,
        pob_root,
        xml,
        &poe_optimizer_core::options::EvaluationOptions::default(),
        timeout,
    )
}

pub fn evaluate_with_options(
    executable: &Path,
    pob_root: &Path,
    xml: &str,
    options: &poe_optimizer_core::options::EvaluationOptions,
    timeout: Duration,
) -> Result<EvaluationSnapshot, SupervisorError> {
    let deadline = deadline(timeout)?;
    if xml.len() > MAX_XML_BYTES {
        return Err(SupervisorErrorKind::InputTooLarge.into());
    }
    let mut request = serde_json::to_vec(&WorkerRequest {
        protocol_version: PROTOCOL_VERSION,
        request_id: REQUEST_ID,
        xml: xml.to_owned(),
        options: options.clone(),
    })
    .map_err(SupervisorErrorKind::Serialize)?;
    if request.len() >= MAX_WIRE_BYTES {
        return Err(SupervisorErrorKind::OutputTooLarge {
            stage: "request",
            limit: MAX_WIRE_BYTES,
        }
        .into());
    }
    request.push(b'\n');
    let executable = absolute(executable).map_err(|source| SupervisorErrorKind::Path {
        name: "executable",
        source,
    })?;
    let pob_root = absolute(pob_root).map_err(|source| SupervisorErrorKind::Path {
        name: "PoB root",
        source,
    })?;
    let mut command = Command::new(executable);
    command
        .arg("__worker")
        .arg("--pob")
        .arg(&pob_root)
        .current_dir(pob_root.join("src"));
    let scratch = tempfile::Builder::new()
        .prefix("poe-optimizer-worker-")
        .tempdir()
        .map_err(|source| SupervisorErrorKind::Io {
            stream: "scratch creation",
            source,
        })?;
    with_scratch(scratch, |scratch| {
        command.arg("--scratch").arg(scratch);
        supervise(command, request, deadline, timeout)
    })
}

/// The operation must finish reaping its child and joining its I/O before returning.
fn with_scratch<T>(
    scratch: TempDir,
    operation: impl FnOnce(&Path) -> Result<T, SupervisorError>,
) -> Result<T, SupervisorError> {
    let path = absolute(scratch.path()).map_err(|source| SupervisorErrorKind::Path {
        name: "scratch",
        source,
    })?;
    let result = operation(&path);
    match scratch.close() {
        Ok(()) => result,
        Err(source) => {
            let cleanup = format!(
                "cannot remove scratch directory {}: {source}",
                path.display()
            );
            match result {
                Ok(_) => {
                    let mut error: SupervisorError = SupervisorErrorKind::Io {
                        stream: "scratch cleanup",
                        source,
                    }
                    .into();
                    error.cleanup_error = Some(cleanup);
                    Err(error)
                }
                Err(mut error) => {
                    if let Some(previous) = &mut error.cleanup_error {
                        previous.push('\n');
                        previous.push_str(&cleanup);
                    } else {
                        error.cleanup_error = Some(cleanup);
                    }
                    Err(error)
                }
            }
        }
    }
}

fn deadline(timeout: Duration) -> Result<Instant, SupervisorErrorKind> {
    if timeout.is_zero() {
        return Err(SupervisorErrorKind::InvalidTimeout);
    }
    Instant::now()
        .checked_add(timeout)
        .ok_or(SupervisorErrorKind::InvalidTimeout)
}

fn supervise(
    mut command: Command,
    request: Vec<u8>,
    deadline: Instant,
    timeout: Duration,
) -> Result<EvaluationSnapshot, SupervisorError> {
    if Instant::now() >= deadline {
        return Err(SupervisorErrorKind::Timeout { timeout }.into());
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let child = command.spawn().map_err(SupervisorErrorKind::Spawn)?;
    let mut child = ReapedChild {
        child,
        reaped: false,
    };
    // These handles are guaranteed by the Stdio::piped configuration above.
    let mut stdin = child.child.stdin.take();
    let stdout = child.child.stdout.take().expect("piped stdout");
    let stderr = child.child.stderr.take().expect("piped stderr");
    let diagnostics = Arc::new(Mutex::new(Diagnostics::default()));
    let (sender, receiver) = mpsc::channel();
    let mut threads = Vec::new();

    let result = (|| {
        let output_sender = sender.clone();
        spawn_io_thread("stdout", &mut threads, move || {
            let result = read_stdout(stdout, &output_sender);
            send_completion(&output_sender, Stream::Stdout, result);
        })?;
        let error_sender = sender.clone();
        let error_diagnostics = Arc::clone(&diagnostics);
        spawn_io_thread("stderr", &mut threads, move || {
            let result = drain_stderr(stderr, &error_diagnostics);
            send_completion(&error_sender, Stream::Stderr, result);
        })?;
        exchange(
            &mut child,
            &mut stdin,
            request,
            &sender,
            &receiver,
            &mut threads,
            deadline,
            timeout,
        )
    })();

    // Closing stdin and killing the worker unblocks all owned pipe I/O before join.
    drop(stdin);
    let cleanup_error = if result.is_err() {
        child.stop().err().map(|error| error.to_string())
    } else {
        None
    };
    let mut thread_panicked = false;
    for thread in threads {
        thread_panicked |= thread.join().is_err();
    }
    let result = result.and_then(|snapshot| {
        if thread_panicked {
            Err(SupervisorErrorKind::ThreadPanicked)
        } else {
            Ok(snapshot)
        }
    });
    let diagnostics = diagnostics
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let stderr = String::from_utf8_lossy(&diagnostics.bytes).into_owned();
    match result {
        Ok(mut snapshot) => {
            snapshot.diagnostics = stderr;
            snapshot.diagnostics_truncated = diagnostics.truncated;
            Ok(snapshot)
        }
        Err(kind) => Err(SupervisorError {
            kind,
            stderr,
            stderr_truncated: diagnostics.truncated,
            cleanup_error,
        }),
    }
}

pub(crate) struct ReapedChild {
    child: Child,
    reaped: bool,
}

impl ReapedChild {
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child,
            reaped: false,
        }
    }
    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        self.reaped |= status.is_some();
        Ok(status)
    }

    pub(crate) fn stop(&mut self) -> io::Result<()> {
        if self.reaped || self.try_wait()?.is_some() {
            return Ok(());
        }
        if let Err(error) = self.child.kill() {
            // The child may have exited between try_wait and kill.
            if self.try_wait()?.is_none() {
                return Err(error);
            }
        }
        self.child.wait()?;
        self.reaped = true;
        Ok(())
    }
}

impl Drop for ReapedChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[derive(Clone, Copy)]
enum Stream {
    Stdin,
    Stdout,
    Stderr,
}

enum Event {
    Hello(Vec<u8>),
    Response(Vec<u8>),
    Finished(Stream),
    Failed(SupervisorErrorKind),
}

fn spawn_io_thread(
    name: &'static str,
    threads: &mut Vec<JoinHandle<()>>,
    task: impl FnOnce() + Send + 'static,
) -> Result<(), SupervisorErrorKind> {
    let thread = thread::Builder::new()
        .name(format!("pob-{name}"))
        .spawn(task)
        .map_err(|source| SupervisorErrorKind::ThreadSpawn { name, source })?;
    threads.push(thread);
    Ok(())
}

fn send_completion(
    sender: &mpsc::Sender<Event>,
    stream: Stream,
    result: Result<(), SupervisorErrorKind>,
) {
    let event = match result {
        Ok(()) => Event::Finished(stream),
        Err(error) => Event::Failed(error),
    };
    let _ = sender.send(event);
}

#[allow(clippy::too_many_arguments)]
fn exchange(
    child: &mut ReapedChild,
    stdin: &mut Option<ChildStdin>,
    request: Vec<u8>,
    sender: &mpsc::Sender<Event>,
    receiver: &mpsc::Receiver<Event>,
    threads: &mut Vec<JoinHandle<()>>,
    deadline: Instant,
    timeout: Duration,
) -> Result<EvaluationSnapshot, SupervisorErrorKind> {
    let mut request = Some(request);
    let mut hello_received = false;
    let mut response = None;
    let mut stdin_finished = false;
    let mut stdout_finished = false;
    let mut stderr_finished = false;
    let mut exit_status = None;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or(SupervisorErrorKind::Timeout { timeout })?;
        if exit_status.is_none() {
            exit_status = child.try_wait().map_err(|source| SupervisorErrorKind::Io {
                stream: "process status",
                source,
            })?;
        }
        if let Some(status) = exit_status {
            if !status.success() {
                return Err(SupervisorErrorKind::Exit { status });
            }
            if stdin_finished && stdout_finished && stderr_finished {
                return response.ok_or(SupervisorErrorKind::MessageOrder);
            }
        }
        let event = match receiver.recv_timeout(remaining.min(EXIT_POLL_INTERVAL)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(SupervisorErrorKind::MessageOrder);
            }
        };
        match event {
            Event::Hello(bytes) => {
                if hello_received {
                    return Err(SupervisorErrorKind::MessageOrder);
                }
                validate_hello(&bytes)?;
                hello_received = true;
                let mut stdin = stdin.take().ok_or(SupervisorErrorKind::MessageOrder)?;
                let request = request.take().ok_or(SupervisorErrorKind::MessageOrder)?;
                let write_sender = sender.clone();
                spawn_io_thread("stdin", threads, move || {
                    let result = stdin
                        .write_all(&request)
                        .and_then(|()| stdin.flush())
                        .map_err(|source| SupervisorErrorKind::Io {
                            stream: "stdin",
                            source,
                        });
                    drop(stdin);
                    send_completion(&write_sender, Stream::Stdin, result);
                })?;
            }
            Event::Response(bytes) => {
                if !hello_received || response.is_some() {
                    return Err(SupervisorErrorKind::MessageOrder);
                }
                response = Some(validate_response(&bytes)?);
            }
            Event::Finished(Stream::Stdin) => stdin_finished = true,
            Event::Finished(Stream::Stdout) => stdout_finished = true,
            Event::Finished(Stream::Stderr) => stderr_finished = true,
            Event::Failed(error) => return Err(error),
        }
    }
}

fn validate_hello(bytes: &[u8]) -> Result<(), SupervisorErrorKind> {
    let hello: WorkerHello =
        serde_json::from_slice(bytes).map_err(|source| SupervisorErrorKind::InvalidJson {
            stage: "hello",
            source,
        })?;
    if hello.protocol_version != PROTOCOL_VERSION {
        return Err(SupervisorErrorKind::ProtocolVersion {
            stage: "hello",
            actual: hello.protocol_version,
        });
    }
    Ok(())
}

fn validate_response(bytes: &[u8]) -> Result<EvaluationSnapshot, SupervisorErrorKind> {
    let response: WorkerResponse =
        serde_json::from_slice(bytes).map_err(|source| SupervisorErrorKind::InvalidJson {
            stage: "response",
            source,
        })?;
    if response.protocol_version != PROTOCOL_VERSION {
        return Err(SupervisorErrorKind::ProtocolVersion {
            stage: "response",
            actual: response.protocol_version,
        });
    }
    if response.request_id != REQUEST_ID {
        return Err(SupervisorErrorKind::RequestId {
            actual: response.request_id,
        });
    }
    response
        .result
        .map_err(|failure| SupervisorErrorKind::Worker {
            code: failure.code,
            message: failure.message,
        })
}

fn read_stdout(stdout: impl Read, sender: &mpsc::Sender<Event>) -> Result<(), SupervisorErrorKind> {
    let mut reader = BufReader::new(stdout);
    let mut remaining = MAX_WIRE_BYTES;
    let hello = read_bounded_line(&mut reader, "hello", MAX_HELLO_BYTES, &mut remaining)?;
    if sender.send(Event::Hello(hello)).is_err() {
        return Ok(());
    }
    let response_limit = remaining;
    let response = read_bounded_line(&mut reader, "response", response_limit, &mut remaining)?;
    if sender.send(Event::Response(response)).is_err() {
        return Ok(());
    }
    let mut tail = [0u8; 1];
    if reader
        .read(&mut tail)
        .map_err(|source| SupervisorErrorKind::Io {
            stream: "stdout",
            source,
        })?
        != 0
    {
        return Err(SupervisorErrorKind::TrailingOutput);
    }
    Ok(())
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    stage: &'static str,
    limit: usize,
    remaining: &mut usize,
) -> Result<Vec<u8>, SupervisorErrorKind> {
    let limit = limit.min(*remaining);
    let mut line = Vec::new();
    (&mut *reader)
        .take((limit + 1) as u64)
        .read_until(b'\n', &mut line)
        .map_err(|source| SupervisorErrorKind::Io {
            stream: "stdout",
            source,
        })?;
    if line.len() > limit {
        return Err(SupervisorErrorKind::OutputTooLarge { stage, limit });
    }
    if line.last() != Some(&b'\n') {
        return Err(SupervisorErrorKind::IncompleteLine { stage });
    }
    *remaining -= line.len();
    line.pop();
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    Ok(line)
}

#[derive(Default)]
struct Diagnostics {
    bytes: Vec<u8>,
    truncated: bool,
}

impl Diagnostics {
    fn append(&mut self, bytes: &[u8]) {
        let keep = bytes.len().min(MAX_STDERR_BYTES - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..keep]);
        self.truncated |= keep != bytes.len();
    }
}

fn drain_stderr(
    mut stderr: impl Read,
    diagnostics: &Mutex<Diagnostics>,
) -> Result<(), SupervisorErrorKind> {
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let count = stderr
            .read(&mut buffer)
            .map_err(|source| SupervisorErrorKind::Io {
                stream: "stderr",
                source,
            })?;
        if count == 0 {
            return Ok(());
        }
        diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .append(&buffer[..count]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn failure_response(version: u32, id: u64) -> Vec<u8> {
        serde_json::to_vec(&WorkerResponse {
            protocol_version: version,
            request_id: id,
            result: Err(poe_optimizer_core::WorkerFailure {
                code: "fixture_error".to_owned(),
                message: "specific failure".to_owned(),
            }),
        })
        .unwrap()
    }

    #[test]
    fn validates_protocol_version_request_identity_and_worker_failure() {
        assert!(validate_hello(br#"{"protocol_version":2,"backend":"mlua"}"#).is_ok());
        assert!(matches!(
            validate_hello(br#"{"protocol_version":999,"backend":"mlua"}"#),
            Err(SupervisorErrorKind::ProtocolVersion {
                stage: "hello",
                actual: 999
            })
        ));
        assert!(matches!(
            validate_response(&failure_response(999, 1)),
            Err(SupervisorErrorKind::ProtocolVersion {
                stage: "response",
                actual: 999
            })
        ));
        assert!(matches!(
            validate_response(&failure_response(PROTOCOL_VERSION, 2)),
            Err(SupervisorErrorKind::RequestId { actual: 2 })
        ));
        assert!(
            matches!(validate_response(&failure_response(PROTOCOL_VERSION, 1)),
            Err(SupervisorErrorKind::Worker { code, message }) if code == "fixture_error" && message == "specific failure")
        );
    }

    #[test]
    fn rejects_malformed_or_duplicate_protocol_fields() {
        for bytes in [
            b"not JSON".as_slice(),
            br#"{"protocol_version":2,"protocol_version":2,"backend":"mlua"}"#,
        ] {
            assert!(matches!(
                validate_hello(bytes),
                Err(SupervisorErrorKind::InvalidJson { stage: "hello", .. })
            ));
        }
        assert!(matches!(
            validate_response(br#"{"protocol_version":2,"request_id":1}"#),
            Err(SupervisorErrorKind::InvalidJson {
                stage: "response",
                ..
            })
        ));
    }

    #[test]
    fn reads_exactly_two_lines_and_accepts_crlf() {
        let (sender, receiver) = mpsc::channel();
        read_stdout(Cursor::new(b"hello\r\nresponse\n"), &sender).unwrap();
        assert!(matches!(receiver.recv().unwrap(), Event::Hello(bytes) if bytes == b"hello"));
        assert!(matches!(receiver.recv().unwrap(), Event::Response(bytes) if bytes == b"response"));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn rejects_missing_lines_and_trailing_stdout() {
        // Keep the receiver alive: a disconnected consumer stops the reader early.
        let (sender, _receiver) = mpsc::channel();
        for bytes in [b"".as_slice(), b"hello", b"hello\n", b"hello\nresponse"] {
            assert!(matches!(
                read_stdout(Cursor::new(bytes), &sender),
                Err(SupervisorErrorKind::IncompleteLine { .. })
            ));
        }
        assert!(matches!(
            read_stdout(Cursor::new(b"hello\nresponse\nextra"), &sender),
            Err(SupervisorErrorKind::TrailingOutput)
        ));
    }

    #[test]
    fn bounds_lines_including_terminators_and_cumulative_bytes() {
        let mut remaining = 8;
        let mut reader = Cursor::new(b"abc\ndef\n");
        assert_eq!(
            read_bounded_line(&mut reader, "hello", 4, &mut remaining).unwrap(),
            b"abc"
        );
        assert_eq!(remaining, 4);
        assert_eq!(
            read_bounded_line(&mut reader, "response", 4, &mut remaining).unwrap(),
            b"def"
        );
        assert_eq!(remaining, 0);
        let mut remaining = 4;
        assert!(matches!(
            read_bounded_line(&mut Cursor::new(b"abcd\n"), "hello", 4, &mut remaining),
            Err(SupervisorErrorKind::OutputTooLarge { limit: 4, .. })
        ));
        let mut remaining = 3;
        assert!(matches!(
            read_bounded_line(&mut Cursor::new(b"abc\n"), "response", 10, &mut remaining),
            Err(SupervisorErrorKind::OutputTooLarge { limit: 3, .. })
        ));
    }

    #[test]
    fn bounds_hello_before_receiving_a_response() {
        let (sender, _receiver) = mpsc::channel();
        let bytes = vec![b'x'; MAX_HELLO_BYTES + 1];
        assert!(matches!(
            read_stdout(Cursor::new(bytes), &sender),
            Err(SupervisorErrorKind::OutputTooLarge {
                stage: "hello",
                limit: MAX_HELLO_BYTES
            })
        ));
    }

    #[test]
    fn drains_stderr_after_retention_limit() {
        let bytes = vec![b'x'; MAX_STDERR_BYTES * 3];
        let mut reader = Cursor::new(bytes);
        let diagnostics = Mutex::new(Diagnostics::default());
        drain_stderr(&mut reader, &diagnostics).unwrap();
        assert_eq!(reader.position() as usize, MAX_STDERR_BYTES * 3);
        let diagnostics = diagnostics.lock().unwrap();
        assert_eq!(diagnostics.bytes.len(), MAX_STDERR_BYTES);
        assert!(diagnostics.truncated);
    }

    #[test]
    fn zero_timeout_and_oversized_xml_fail_before_spawn() {
        let missing = Path::new("this-evaluator-does-not-exist");
        assert!(matches!(
            evaluate(missing, missing, "", Duration::ZERO)
                .unwrap_err()
                .kind,
            SupervisorErrorKind::InvalidTimeout
        ));
        assert!(matches!(
            evaluate(
                missing,
                missing,
                &"x".repeat(MAX_XML_BYTES + 1),
                Duration::from_secs(1)
            )
            .unwrap_err()
            .kind,
            SupervisorErrorKind::InputTooLarge
        ));
    }

    #[test]
    fn deadline_stops_and_reaps_a_process_with_blocked_stdin() {
        let scratch = tempfile::tempdir().unwrap();
        let scratch_path = scratch.path().to_owned();
        let result: Result<(), SupervisorError> = with_scratch(scratch, |scratch| {
            std::fs::write(scratch.join("settings.xml"), "worker-owned state").unwrap();
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "supervisor::tests::blocking_process_fixture",
                    "--ignored",
                    "--nocapture",
                ])
                .env("POE_OPTIMIZER_SUPERVISOR_BLOCKING_FIXTURE", "1")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000);
            }
            let mut child = ReapedChild {
                child: command.spawn().unwrap(),
                reaped: false,
            };
            let mut stdin = child.child.stdin.take();
            let (sender, receiver) = mpsc::channel();
            // The helper intentionally never reads stdin. Inject a validated startup
            // event to exercise the production writer and deadline loop independently
            // of the Rust test harness's own stdout preamble.
            sender
                .send(Event::Hello(
                    br#"{"protocol_version":2,"backend":"fixture"}"#.to_vec(),
                ))
                .unwrap();
            let timeout = Duration::from_millis(250);
            let mut threads = Vec::new();
            let result = exchange(
                &mut child,
                &mut stdin,
                vec![b'x'; MAX_XML_BYTES],
                &sender,
                &receiver,
                &mut threads,
                deadline(timeout).unwrap(),
                timeout,
            );
            child.stop().unwrap();
            drop(stdin);
            for thread in threads {
                thread.join().unwrap();
            }
            assert!(matches!(result, Err(SupervisorErrorKind::Timeout { .. })));
            assert!(child.reaped);
            assert!(child.child.try_wait().unwrap().is_some());
            assert!(scratch.join("settings.xml").is_file());
            result.map(|_| ()).map_err(SupervisorError::from)
        });
        assert!(matches!(
            result.unwrap_err().kind,
            SupervisorErrorKind::Timeout { .. }
        ));
        assert!(!scratch_path.exists());
    }

    #[test]
    fn removes_scratch_after_success_and_spawn_failure() {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().to_owned();
        with_scratch(scratch, |scratch| {
            std::fs::create_dir(scratch.join("cache")).unwrap();
            std::fs::write(scratch.join("cache/state"), "scratch state").unwrap();
            Ok(())
        })
        .unwrap();
        assert!(!path.exists());

        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().to_owned();
        let error = with_scratch(scratch, |scratch| {
            let command = Command::new(scratch.join("missing-worker"));
            let timeout = Duration::from_secs(1);
            supervise(
                command,
                b"{}\n".to_vec(),
                deadline(timeout).unwrap(),
                timeout,
            )
        })
        .unwrap_err();
        assert!(matches!(error.kind, SupervisorErrorKind::Spawn(_)));
        assert!(!path.exists());
    }

    #[test]
    #[ignore = "subprocess fixture; invoked only by the supervisor deadline test"]
    fn blocking_process_fixture() {
        if std::env::var_os("POE_OPTIMIZER_SUPERVISOR_BLOCKING_FIXTURE").is_some() {
            // Bounded fallback if this helper is invoked manually without its parent.
            thread::sleep(Duration::from_secs(30));
        }
    }
    #[test]
    fn rejects_a_real_process_that_does_not_implement_the_protocol() {
        // The Rust test executable is portable and needs no external shell or helper.
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.arg("--help");
        let timeout = Duration::from_secs(10);
        let error = supervise(
            command,
            b"{}\n".to_vec(),
            deadline(timeout).unwrap(),
            timeout,
        )
        .unwrap_err();
        assert!(matches!(
            error.kind,
            SupervisorErrorKind::InvalidJson { stage: "hello", .. }
                | SupervisorErrorKind::IncompleteLine { .. }
                | SupervisorErrorKind::Exit { .. }
        ));
    }
}
