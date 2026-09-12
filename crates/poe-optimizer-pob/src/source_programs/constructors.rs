//! Map actual bytecode evidence to exact complete lowered source expressions.
use super::*;
use capture::ObservedSourceConstructors;
use poe_optimizer_data::source_program::{
    SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION, SourceProgramConstructor,
    SourceProgramConstructors, SourceTableAllocation,
};

/// Add layout evidence from the exact observer owner after ordinary complete
/// source lowering. Missing/ambiguous constructor proofs remain diagnostics and
/// never grant evidence to a logically equivalent authored table expression.
pub fn lower_observed_from_sources(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
    observations: &ObservedSourceConstructors,
) -> Result<SourceProgramExtraction> {
    observations.validate_owner(owner)?;
    let mut lowered = lower_from_sources(sources, owner)?;
    let mut sites = Vec::new();
    let mut budget = Budget::default();
    for program in &lowered.catalog.data().programs {
        let Some(observation) = observations.callbacks.get(&program.callback) else {
            return Err(error(
                "lowered callback lacks its actual constructor observation",
            ));
        };
        if observation.source != program.provenance.source {
            return Err(error("constructor callback source provenance changed"));
        }
        let span = &program.provenance.source;
        let body = sources[&span.path]
            .split_inclusive('\n')
            .skip(span.line as usize - 1)
            .take((span.end_line - span.line + 1) as usize)
            .collect::<String>();
        let start = program.provenance.function_start as usize;
        let end = program.provenance.function_end as usize;
        let function = body
            .get(start..end)
            .ok_or_else(|| error("constructor function byte range"))?;
        let tokens = lowering::lex(function, &mut budget).map_err(error)?;
        let positions: Vec<_> = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| !token.quoted && token.text == "{")
            .map(|(index, _)| index)
            .collect();
        if positions.is_empty() && observation.constructors.is_empty() {
            continue;
        }
        let reason = if let Some(reason) = &observation.unsupported {
            Some(reason.clone())
        } else if positions.len() != 1 || observation.constructors.len() != 1 {
            Some("constructor source/bytecode occurrence is not uniquely matched".into())
        } else {
            let token = positions[0];
            let instruction = &observation.constructors[0];
            let line = span.line
                + body.as_bytes()[..start + tokens[token].start]
                    .iter()
                    .filter(|byte| **byte == b'\n')
                    .count() as u32;
            if tokens
                .get(token + 1)
                .is_none_or(|next| next.text != "}" || next.quoted)
            {
                Some("only a syntactically empty constructor has allocation proof".into())
            } else if instruction.word & 255 != 52 || instruction.word >> 16 != 0 {
                Some("constructor is not an observed zero-allocation TNEW".into())
            } else if instruction.line != line {
                Some("constructor instruction/source line is not uniquely matched".into())
            } else {
                sites.push(SourceProgramConstructor {
                    callback: program.callback,
                    provenance: program.provenance.clone(),
                    expression: ParserProgramLocation {
                        start: tokens[token].start as u32,
                        end: tokens[token + 1].end as u32,
                    },
                    bytecode_sha256: observation.sha256.clone(),
                    bytecode_pc: instruction.pc,
                    instruction: instruction.word,
                    allocation: SourceTableAllocation::New {
                        array_slots: 0,
                        hash_bits: 0,
                    },
                });
                None
            }
        };
        if let Some(reason) = reason {
            lowered
                .constructor_unsupported
                .insert(program.callback, reason);
        }
    }
    lowered.catalog = SourceProgramCatalog::new_with_constructors(
        lowered.catalog.data().clone(),
        owner.clone(),
        SourceProgramConstructors {
            schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
            profile: observations.profile.clone(),
            sites,
        },
    )
    .map_err(error)?;
    Ok(lowered)
}
