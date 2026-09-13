//! Offline diagnostic for actual typed-program rejections, never runtime admission.
//! Usage: inspect_parser_programs <new-output.json> <callback-id>...
use poe_optimizer_data::{
    game_data::bundled_snapshot, item_loading::ItemLoadingSource, modifier_parser::*,
};
use poe_optimizer_pob::parser_programs::extract_pinned;
use serde::Serialize;
use std::{
    collections::{BTreeSet, VecDeque},
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
};

const MAX_REQUESTED: usize = 16;
const MAX_CALLBACKS: usize = 64;
const MAX_DEPTH: usize = 8;
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

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
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().ok_or("provide a new JSON output path")?);
    if destination.exists() {
        return Err("output path already exists".into());
    }
    let mut requested = Vec::new();
    let mut seen = BTreeSet::new();
    for arg in args {
        if requested.len() == MAX_REQUESTED {
            return Err("at most 16 callback IDs may be requested".into());
        }
        let id = ParserCallbackId(arg.to_str().ok_or("callback ID is not UTF-8")?.parse()?);
        if id.0 == 0 || !seen.insert(id) {
            return Err("callback IDs must be positive and distinct".into());
        }
        requested.push(id);
    }
    if requested.is_empty() {
        return Err("provide at least one callback ID".into());
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
