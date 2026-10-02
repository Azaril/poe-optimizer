//! CLI boundaries keep the owned evaluator distinct from optional PoB reference tools.
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}

fn help(args: &[&str]) -> String {
    let output = cli().args(args).arg("--help").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn advertises(help: &str, command: &str) -> bool {
    help.lines()
        .any(|line| line.split_whitespace().next() == Some(command))
}

#[test]
fn retired_commands_are_not_advertised_or_redirected() {
    let help = help(&[]);
    let temp = tempfile::tempdir().unwrap();
    for command in [
        "search-experimental",
        "prepare-build",
        "search-build",
        "benchmark-native",
    ] {
        assert!(!advertises(&help, command), "{command}");
        let output = cli()
            .current_dir(temp.path())
            .args([command, "not-read.json", "--output", "not-written.json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{command}");
        assert!(output.stdout.is_empty(), "{command}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!temp.path().join("not-written.json").exists());
    }
}

#[test]
fn owned_evaluation_is_available_without_the_retired_backend_selector() {
    assert!(advertises(&help(&[]), "evaluate-owned"));
    let owned_help = help(&["evaluate-owned"]);
    assert!(owned_help.contains("--release"));
    assert!(!owned_help.contains("--backend"));
}

#[cfg(not(feature = "pob"))]
#[test]
fn default_cli_has_no_pob_reference_commands_or_worker() {
    let help = help(&[]);
    for command in ["evaluate", "metrics", "extract-game-data", "__worker"] {
        assert!(!advertises(&help, command), "{command}");
        let output = cli().args([command, "--help"]).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{command}");
        assert!(output.stdout.is_empty(), "{command}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"),
            "{command}"
        );
    }
}

#[cfg(feature = "pob")]
#[test]
fn optional_pob_commands_are_explicit_reference_tools() {
    let top_help = help(&[]);
    for command in ["evaluate", "metrics"] {
        assert!(advertises(&top_help, command), "{command}");
        let command_help = help(&[command]);
        assert!(
            command_help.contains("Path of Building (PoB) reference"),
            "{command_help}"
        );
        for removed in ["--backend", "--data", "--data-sha256"] {
            assert!(!command_help.contains(removed), "{command}: {removed}");
        }
    }
}

#[cfg(feature = "pob")]
#[test]
fn reference_commands_reject_retired_native_arguments_before_reading_inputs() {
    let temp = tempfile::tempdir().unwrap();
    for (flag, value) in [
        ("--backend", "native"),
        ("--backend", "pob"),
        ("--data", "not-read.json"),
        ("--data-sha256", "not-a-digest"),
    ] {
        for command in ["evaluate", "metrics"] {
            let mut invocation = cli();
            invocation.current_dir(temp.path()).arg(command);
            if command == "evaluate" {
                invocation.args(["not-read.xml", "--output", "not-written.json"]);
            }
            let output = invocation.args([flag, value]).output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{command} {flag}");
            assert!(output.stdout.is_empty());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("unexpected argument"), "{error}");
            assert!(error.contains(flag), "{error}");
            assert!(!temp.path().join("not-written.json").exists());
        }
    }
}
