//! Offline developer commands. This executable is separate from the runtime CLI.
#![forbid(unsafe_code)]

mod release_cli;

use poe_optimizer_authoring::{FormatLimits, format_program};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

const HELP: &str = "Usage: poe-author rule fmt [--check] [--] <path|->\n\
       poe-author release build <release.kdl> --output <new-directory>\n\
\n\
Format one .rule program to stdout; '-' reads stdin.\n\
--check emits no source and exits 1 if formatting differs.\n\
Exit 0 means success; exit 2 means an argument, I/O or syntax error.\n\
Formatting validates syntax only, not rule types, ownership or coverage.\n\
Release build checks all declared constituents before atomic publication.\n\
Input files and existing output directories are never overwritten.\n";

struct FormatCommand {
    input: OsString,
    check: bool,
}

enum Command {
    Help,
    Format(FormatCommand),
    Release(release_cli::Command),
}

fn arguments(mut args: impl Iterator<Item = OsString>) -> Result<Command, &'static str> {
    let Some(first) = args.next() else {
        return Err("expected a command; use --help");
    };
    if first == "--help" || first == "-h" {
        return if args.next().is_none() {
            Ok(Command::Help)
        } else {
            Err("unexpected arguments after --help")
        };
    }
    if first == "release" {
        return release_cli::arguments(args).map(Command::Release);
    }
    if first != "rule" || args.next().as_deref() != Some(OsStr::new("fmt")) {
        return Err("expected 'rule fmt' or 'release build'; use --help");
    }
    let mut check = false;
    let mut positional_only = false;
    let mut input = None;
    for arg in args {
        if !positional_only && arg == "--" {
            positional_only = true;
        } else if !positional_only && arg == "--check" {
            if check {
                return Err("--check may only be specified once");
            }
            check = true;
        } else if !positional_only && (arg == "--help" || arg == "-h") {
            return Err("use 'poe-author --help' for usage");
        } else if !positional_only && arg != "-" && arg.as_encoded_bytes().starts_with(b"-") {
            return Err("unknown option; use '--' before a path beginning with '-'");
        } else if input.replace(arg).is_some() {
            return Err("expected exactly one input path or '-'");
        }
    }
    Ok(Command::Format(FormatCommand {
        input: input.ok_or("expected one input path or '-'")?,
        check,
    }))
}

fn read_source(reader: impl Read, maximum: usize) -> Result<String, String> {
    // Read one sentinel byte beyond the parser cap, without trusting file size
    // metadata or reading an arbitrarily large pipe into memory.
    let limit = u64::try_from(maximum)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or("source byte limit is invalid")?;
    let mut bytes = Vec::new();
    reader
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read input: {error}"))?;
    if bytes.len() > maximum {
        return Err(format!("input exceeds the {maximum}-byte source limit"));
    }
    String::from_utf8(bytes).map_err(|_| "input is not valid UTF-8".to_owned())
}

fn run(command: Command) -> Result<ExitCode, String> {
    let command = match command {
        Command::Help => {
            io::stdout()
                .lock()
                .write_all(HELP.as_bytes())
                .map_err(|error| format!("cannot write help: {error}"))?;
            return Ok(ExitCode::SUCCESS);
        }
        Command::Format(command) => command,
        Command::Release(command) => return release_cli::run(command),
    };
    let limits = FormatLimits::default();
    let source = if command.input == "-" {
        read_source(
            io::stdin().lock(),
            limits.parse.expressions.max_source_bytes,
        )?
    } else {
        let path = PathBuf::from(&command.input);
        let file = File::open(&path)
            .map_err(|error| format!("cannot open '{}': {error}", path.display()))?;
        read_source(file, limits.parse.expressions.max_source_bytes)?
    };
    let formatted = format_program(&source, limits).map_err(|error| error.to_string())?;
    if command.check {
        if source == formatted {
            Ok(ExitCode::SUCCESS)
        } else {
            eprintln!("input needs formatting");
            Ok(ExitCode::from(1))
        }
    } else {
        io::stdout()
            .lock()
            .write_all(formatted.as_bytes())
            .map_err(|error| format!("cannot write formatted source: {error}"))?;
        Ok(ExitCode::SUCCESS)
    }
}

fn main() -> ExitCode {
    match arguments(env::args_os().skip(1))
        .map_err(str::to_owned)
        .and_then(run)
    {
        Ok(code) => code,
        Err(error) => {
            eprintln!("poe-author: {error}");
            ExitCode::from(2)
        }
    }
}
