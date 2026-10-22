//! The offline executable formats syntax without changing source files or
//! bringing schema/evaluator authority into a formatting operation.
use std::{
    ffi::OsStr,
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

const SOURCE: &str = "// a mechanic\nprogram \"sample\" context actor{let amount=-0.0;}";
const FORMATTED: &str =
    "// a mechanic\nprogram \"sample\" context actor {\n    let amount = -0.0;\n}\n";

fn run(args: &[&OsStr], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_poe-author"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input).unwrap();
    drop(stdin);
    child.wait_with_output().unwrap()
}

fn text_args<'a>(args: &'a [&'a str]) -> Vec<&'a OsStr> {
    args.iter().map(OsStr::new).collect()
}

#[test]
fn stdin_format_and_check_have_distinct_statuses_and_bounded_output() {
    let output = run(&text_args(&["rule", "fmt", "-"]), SOURCE.as_bytes());
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, FORMATTED.as_bytes());
    assert!(output.stderr.is_empty());

    for (input, expected) in [(SOURCE, 1), (FORMATTED, 0)] {
        let output = run(
            &text_args(&["rule", "fmt", "--check", "-"]),
            input.as_bytes(),
        );
        assert_eq!(output.status.code(), Some(expected));
        assert!(output.stdout.is_empty());
        if expected == 1 {
            assert_eq!(output.stderr, b"input needs formatting\n");
        } else {
            assert!(output.stderr.is_empty());
        }
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "poe-author-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // An existing path is an error; never reuse a possibly foreign directory.
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        for name in ["build input λ.rule", "-dash.rule"] {
            let _ = fs::remove_file(self.0.join(name));
        }
        let _ = fs::remove_dir(&self.0);
    }
}

#[test]
fn file_formatting_keeps_source_bytes_and_accepts_os_paths_and_option_terminator() {
    let directory = TestDirectory::new();
    let input = directory.0.join("build input λ.rule");
    fs::write(&input, SOURCE).unwrap();
    for check in [false, true] {
        let mut args = text_args(&["rule", "fmt"]);
        if check {
            args.push(OsStr::new("--check"));
        }
        args.extend([OsStr::new("--"), input.as_os_str()]);
        let output = run(&args, b"");
        assert_eq!(output.status.code(), Some(if check { 1 } else { 0 }));
        assert_eq!(fs::read(&input).unwrap(), SOURCE.as_bytes());
    }
    fs::write(directory.0.join("-dash.rule"), SOURCE).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-author"))
        .current_dir(&directory.0)
        .args(["rule", "fmt", "--", "-dash.rule"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, FORMATTED.as_bytes());
}

#[test]
fn invalid_encoding_syntax_and_oversized_inputs_produce_no_partial_source() {
    let oversized = vec![b' '; 256 * 1024 + 1];
    let compact = "\nprogram \"at-limit\" context actor{}";
    let grows_past_source_cap =
        format!("//{}{compact}", "x".repeat(256 * 1024 - compact.len() - 2));
    for (input, reason) in [
        (&[0xff][..], "UTF-8"),
        (
            &b"program \"bad\" context actor { let x = ; }"[..],
            "expected expression",
        ),
        (oversized.as_slice(), "source limit"),
        (grows_past_source_cap.as_bytes(), "formatted source"),
    ] {
        let output = run(&text_args(&["rule", "fmt", "-"]), input);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains(reason));
    }
}

#[test]
fn help_argument_errors_and_missing_files_do_not_claim_semantic_validation() {
    let help = run(&text_args(&["--help"]), b"");
    assert_eq!(help.status.code(), Some(0));
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("syntax only")
    );
    for args in [
        vec![],
        vec!["rule", "fmt"],
        vec!["rule", "check", "-"],
        vec!["rule", "fmt", "--unknown", "-"],
        vec!["rule", "fmt", "--check", "--check", "-"],
        vec!["rule", "fmt", "-", "extra.rule"],
        vec!["release"],
        vec!["release", "check"],
        vec!["release", "build"],
        vec!["release", "build", "input.kdl"],
        vec!["release", "build", "input.kdl", "--output"],
        vec![
            "release",
            "build",
            "input.kdl",
            "extra.kdl",
            "--output",
            "out",
        ],
        vec![
            "release",
            "build",
            "input.kdl",
            "--output",
            "out",
            "--output",
            "twice",
        ],
        vec!["release", "build", "--unknown"],
    ] {
        let output = run(&text_args(&args), b"");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let directory = TestDirectory::new();
    let missing = directory.0.join("absent.rule");
    let output = run(
        &[OsStr::new("rule"), OsStr::new("fmt"), missing.as_os_str()],
        b"",
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("cannot open")
    );
}
