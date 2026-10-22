//! Offline diagnostic for actual typed-program rejections, never runtime admission.
use clap::{CommandFactory, Parser, error::ErrorKind};
use poe_optimizer_data::{
    game_data::bundled_snapshot, item_loading::ItemLoadingSource, modifier_parser::*,
};
use poe_optimizer_pob::parser_programs::extract_pinned;
use serde::Serialize;
use std::{
    collections::{BTreeSet, VecDeque},
    fs::OpenOptions,
    io::{self, Write},
    num::NonZeroU32,
    path::PathBuf,
};

const MAX_REQUESTED: usize = 16;
const MAX_CALLBACKS: usize = 64;
const MAX_DEPTH: usize = 8;
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

/// Inspect typed-program rejections for distinct callbacks from the pinned source.
#[derive(Debug, Parser)]
#[command(name = "inspect_parser_programs")]
struct Args {
    /// New diagnostic JSON file; an existing file is never overwritten.
    #[arg(value_name = "NEW_OUTPUT.json")]
    destination: PathBuf,
    /// Positive, distinct callback IDs (at most 16).
    #[arg(value_name = "CALLBACK_ID", required = true, num_args = 1..=MAX_REQUESTED)]
    callbacks: Vec<NonZeroU32>,
}

impl Args {
    fn validate(&self) -> Result<(), clap::Error> {
        for (index, id) in self.callbacks.iter().enumerate() {
            if self.callbacks[..index].contains(id) {
                return Err(Self::command()
                    .error(ErrorKind::ValueValidation, "callback IDs must be distinct"));
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct CallbackReport<'a> {
    callback: ParserCallbackId,
    requested: bool,
    captured_depth: usize,
    descriptor: &'a ParserCallback,
    legacy_factory: Option<&'a ParserFactoryDisposition>,
    extraction_status: &'static str,
    raw_rejection: Option<&'a str>,
    lowered_program_sha256: Option<String>,
    lowered_bindings: Option<&'a [ParserProgramBinding]>,
    package_admission: Option<&'a ParserProgramAdmission>,
}
#[derive(Serialize)]
struct Report<'a> {
    scope: &'static str,
    owner_definition_sha256: String,
    owner_source: &'a ItemLoadingSource,
    extractor_sha256: &'a str,
    requested: &'a [ParserCallbackId],
    callbacks: Vec<CallbackReport<'a>>,
}
#[derive(Default)]
struct Output(Vec<u8>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_OUTPUT_BYTES)
        {
            return Err(io::Error::other("diagnostic JSON exceeds 2 MiB"));
        }
        self.0.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    args.validate().unwrap_or_else(|error| error.exit());
    let destination = args.destination;
    let requested: Vec<_> = args
        .callbacks
        .into_iter()
        .map(|id| ParserCallbackId(id.get()))
        .collect();
    let mut seen: BTreeSet<_> = requested.iter().copied().collect();
    if destination.exists() {
        return Err("output path already exists".into());
    }
    let snapshot = bundled_snapshot()?;
    let owner = snapshot.modifier_parser();
    let data = owner.data();
    if requested
        .iter()
        .any(|id| id.0 as usize > data.callbacks.len())
    {
        return Err("requested callback is absent from this owner".into());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    // Preserve extraction errors verbatim on stderr; no JSON file is created on failure.
    let extracted = extract_pinned(&root, owner)?;
    if !extracted.catalog().is_bound_to(owner) {
        return Err("extracted programs lost their exact owner binding".into());
    }
    let mut queue: VecDeque<_> = requested.iter().map(|id| (*id, 0usize)).collect();
    let mut callbacks = Vec::new();
    while let Some((id, depth)) = queue.pop_front() {
        let callback = data
            .callbacks
            .get(id.0 as usize - 1)
            .ok_or("captured callback is absent")?;
        let legacy = data.factories.get(&id);
        let program = extracted.catalog().for_callback(id);
        let rejection = extracted.unsupported().get(&id).map(String::as_str);
        let status = match (
            matches!(legacy, Some(ParserFactoryDisposition::Pure(_))),
            program.is_some(),
            rejection.is_some(),
        ) {
            (true, false, false) => "legacy_pure_not_attempted",
            (false, true, false) => "complete_lowered_program",
            (false, false, true) => "unsupported",
            _ => return Err("callback extraction partition is inconsistent".into()),
        };
        callbacks.push(CallbackReport {
            callback: id,
            requested: requested.contains(&id),
            captured_depth: depth,
            descriptor: callback,
            legacy_factory: legacy,
            extraction_status: status,
            raw_rejection: rejection,
            lowered_program_sha256: program.map(ParserProgram::sha256).transpose()?,
            lowered_bindings: program.map(|p| p.bindings.as_slice()),
            package_admission: data.programs.admissions.get(&id),
        });
        for capture in &callback.upvalues {
            let ParserValue::Callback(child) = &capture.value else {
                continue;
            };
            if seen.contains(child) {
                continue;
            }
            if seen.len() == MAX_CALLBACKS || depth == MAX_DEPTH {
                return Err("captured-callback traversal exceeds 64 callbacks or depth 8".into());
            }
            seen.insert(*child);
            queue.push_back((*child, depth + 1));
        }
    }
    let report = Report {
        scope: "Offline complete extraction with raw final rejection reasons. Traversal follows direct captured callbacks only, not tables or globals. Lowering, legacy recipes and package admission are separate; no native capability or source-parity claim.",
        owner_definition_sha256: data.definition_sha256()?,
        owner_source: &data.source,
        extractor_sha256: extracted.implementation_sha256(),
        requested: &requested,
        callbacks,
    };
    let mut output = Output::default();
    serde_json::to_writer_pretty(&mut output, &report)?;
    output.write_all(b"\n")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)?;
    file.write_all(&output.0)?;
    println!("Wrote program diagnostic: {}", destination.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ids(ids: &[&str]) -> Result<Args, clap::Error> {
        let args = Args::try_parse_from(
            ["inspect_parser_programs", "new-output.json"]
                .into_iter()
                .chain(ids.iter().copied()),
        )?;
        args.validate()?;
        Ok(args)
    }

    #[test]
    fn callback_request_bounds_and_order_are_preserved() {
        let args = parse_ids(&["4294967295"]).unwrap();
        assert_eq!(args.callbacks[0].get(), u32::MAX);

        let ids: Vec<_> = (1..=MAX_REQUESTED).rev().map(|id| id.to_string()).collect();
        let values: Vec<_> = ids.iter().map(String::as_str).collect();
        let args = parse_ids(&values).unwrap();
        assert_eq!(args.callbacks.len(), MAX_REQUESTED);
        assert_eq!(args.callbacks[0].get(), 16);
        assert_eq!(args.callbacks[15].get(), 1);

        assert!(parse_ids(&[]).is_err());
        let mut too_many = values;
        too_many.push("17");
        assert!(parse_ids(&too_many).is_err());
    }

    #[test]
    fn callback_ids_must_be_positive_u32_values_and_unique() {
        for invalid in ["0", "-1", "4294967296", "callback", "1.5"] {
            assert!(parse_ids(&[invalid]).is_err(), "accepted {invalid}");
        }
        for duplicates in [["1", "1"], ["1", "01"]] {
            assert_eq!(
                parse_ids(&duplicates).unwrap_err().kind(),
                ErrorKind::ValueValidation,
            );
        }
    }

    #[test]
    fn separator_allows_option_like_output_paths() {
        let args =
            Args::try_parse_from(["inspect_parser_programs", "--", "--output.json", "1"]).unwrap();
        args.validate().unwrap();
        assert_eq!(args.destination, PathBuf::from("--output.json"));
    }
}
