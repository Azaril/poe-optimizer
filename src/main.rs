use std::{env, process::ExitCode};

const HELP: &str = "poe-optimizer — experimental Path of Exile 2 build optimizer

Usage: poe-optimizer [--help | --version]

This repository currently contains the Rust scaffold and proposed design.
Build evaluation and optimization are not implemented yet.

Design: docs/design.md
Upstream integration notes: docs/pob-integration.md";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => println!("{HELP}"),
        [arg] if arg == "--help" || arg == "-h" => println!("{HELP}"),
        [arg] if arg == "--version" || arg == "-V" => {
            println!("poe-optimizer {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("Unsupported arguments. Run poe-optimizer --help for current capabilities.");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
