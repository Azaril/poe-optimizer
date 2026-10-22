//! Offline developer commands. This executable is separate from the runtime CLI.
#![forbid(unsafe_code)]

mod release_cli;

use clap::{Args, Parser, Subcommand};
use poe_optimizer_authoring::{FormatLimits, format_program};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "poe-author",
    about = "Offline release construction and syntax only rule formatting",
    after_help = "Input files and existing output directories are never overwritten."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
#[command(
    after_help = "Formatting validates syntax only, not rule types, ownership or coverage.\nExit 0 means success; exit 1 means formatting differs; exit 2 means an argument, I/O or syntax error."
)]
struct FormatCommand {
    /// Input .rule path; '-' reads stdin
    #[arg(value_name = "PATH|-")]
    input: PathBuf,
    /// Emit no source and exit 1 if formatting differs
    #[arg(long)]
    check: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Work with rule source programs
    #[command(subcommand)]
    Rule(RuleCommand),
    /// Construct releases from checked final constituents
    #[command(subcommand)]
    Release(release_cli::Command),
}

#[derive(Subcommand)]
enum RuleCommand {
    /// Format one .rule program to stdout without changing its source file
    Fmt(FormatCommand),
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
        Command::Rule(RuleCommand::Fmt(command)) => command,
        Command::Release(command) => return release_cli::run(command),
    };
    let limits = FormatLimits::default();
    let source = if command.input.as_os_str() == "-" {
        read_source(
            io::stdin().lock(),
            limits.parse.expressions.max_source_bytes,
        )?
    } else {
        let file = File::open(&command.input)
            .map_err(|error| format!("cannot open '{}': {error}", command.input.display()))?;
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
    match run(Cli::parse().command) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("poe-author: {error}");
            ExitCode::from(2)
        }
    }
}
