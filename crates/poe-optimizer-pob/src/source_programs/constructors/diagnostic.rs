//! Complete lexical mapping for diagnostics; does not attach allocation facets.
use super::*;
use capture::constructors::{CallbackObservation, Instruction};

pub(in crate::source_programs) fn diagnostic_site(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
    program: &ParserProgram,
    observation: &CallbackObservation,
    expression: ParserProgramLocation,
) -> Result<std::result::Result<Instruction, String>> {
    validate_sources(sources, owner)?;
    if observation.source != program.provenance.source {
        return Err(error("constructor diagnostic source provenance differs"));
    }
    let span = &program.provenance.source;
    let body = capture::closures::source_slice(&sources[&span.path], span.line, span.end_line)?;
    let start = program.provenance.function_start as usize;
    let function = body
        .get(start..program.provenance.function_end as usize)
        .ok_or_else(|| error("constructor diagnostic function byte range"))?;
    if hash(function.as_bytes()) != program.provenance.function_sha256 {
        return Err(error(
            "constructor diagnostic complete function digest differs",
        ));
    }
    let mut budget = Budget::default();
    let tokens = lowering::lex(function, &mut budget).map_err(error)?;
    let positions = match lexical::inventory(&tokens) {
        Ok(positions) => positions,
        Err(reason) => return Ok(Err(reason)),
    };
    if let Some(reason) = &observation.unsupported {
        return Ok(Err(reason.clone()));
    }
    if positions.len() != observation.constructors.len() {
        return Ok(Err(
            "complete constructor source/bytecode occurrence inventory differs".into(),
        ));
    }
    let mut cursor = 0;
    let mut line = span.line;
    let mut selected = None;
    for (position, instruction) in positions.iter().zip(&observation.constructors) {
        let end = start + position.previous_token_end;
        line += body.as_bytes()[cursor..end]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count() as u32;
        cursor = end;
        if line != instruction.line {
            return Ok(Err(
                "complete allocation instruction/source line inventory differs".into(),
            ));
        }
        if position.start == expression.start as usize && position.end == expression.end as usize {
            if selected.is_some() {
                return Ok(Err("ambiguous constructor expression range".into()));
            }
            selected = Some(instruction.clone());
        }
    }
    Ok(selected.ok_or_else(|| "requested range is not an immediate constructor expression".into()))
}
