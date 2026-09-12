//! Map actual bytecode evidence to exact complete lowered source expressions.
use super::*;
mod lexical;
use capture::ObservedSourceConstructors;
use poe_optimizer_data::source_program::{
    SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES, SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES,
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
    let lowered = lower_from_sources(sources, owner)?;
    attach(sources, owner, observations, lowered)
}
pub(super) fn attach(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
    observations: &ObservedSourceConstructors,
    mut lowered: SourceProgramExtraction,
) -> Result<SourceProgramExtraction> {
    observations.validate_owner(owner)?;
    let mut sites = Vec::new();
    let mut retained = RetainedSiteBudget {
        sites: 0,
        text_bytes: observations.profile.source_revision.len(),
    };
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
        let body = capture::closures::source_slice(&sources[&span.path], span.line, span.end_line)?;
        let start = program.provenance.function_start as usize;
        let end = program.provenance.function_end as usize;
        let function = body
            .get(start..end)
            .ok_or_else(|| error("constructor function byte range"))?;
        let tokens = lowering::lex(function, &mut budget).map_err(error)?;
        let positions = lexical::inventory(&tokens).map_err(error)?;
        if positions.is_empty() && observation.constructors.is_empty() {
            continue;
        }
        let inventory_error = observation.unsupported.clone().or_else(|| {
            (positions.len() != observation.constructors.len())
                .then(|| "complete constructor source/bytecode occurrence inventory differs".into())
        });
        if let Some(reason) = inventory_error {
            lowered
                .constructor_unsupported
                .insert(program.callback, reason);
            continue;
        }
        let mut failures = 0usize;
        let mut first_failure = None;
        // Lexical allocation order has monotonically increasing byte offsets.
        // Scan each source byte once instead of recounting every site's prefix.
        let mut line_cursor = 0;
        let mut line = span.line;
        for (position, instruction) in positions.iter().zip(&observation.constructors) {
            let token_end = start + position.previous_token_end;
            line += body.as_bytes()[line_cursor..token_end]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count() as u32;
            line_cursor = token_end;
            let result = (|| -> std::result::Result<SourceTableAllocation, String> {
                if instruction.line != line {
                    return Err("allocation instruction/previous-token source line differs".into());
                }
                let fields = position
                    .list_fields
                    .ok_or("keyed constructors have no admitted source layout")?;
                if instruction.word & 255 != 52 {
                    return Err("TDUP/template constructor has no admitted source layout".into());
                }
                let allocation = SourceTableAllocation::from_tnew_instruction(instruction.word)
                    .map_err(|e| e.to_string())?;
                let expected_hint = if fields == 0 {
                    0
                } else {
                    (fields + 1).clamp(3, 2047)
                };
                let expected_array = if expected_hint == 2047 {
                    2049
                } else {
                    expected_hint as u32
                };
                if allocation
                    != (SourceTableAllocation::New {
                        array_slots: expected_array,
                        hash_bits: 0,
                    })
                {
                    return Err(
                        "actual TNEW capacity differs from canonical source list-field count"
                            .into(),
                    );
                }
                Ok(allocation)
            })();
            match result {
                Ok(allocation) => {
                    // Bound retained metadata amplification before cloning any
                    // per-site provenance, including long repeated source paths.
                    retained.charge([
                        program.provenance.source.path.len(),
                        program.provenance.source.sha256.len(),
                        program.provenance.function_sha256.len(),
                        observation.sha256.len(),
                    ])?;
                    sites.push(SourceProgramConstructor {
                        callback: program.callback,
                        provenance: program.provenance.clone(),
                        expression: ParserProgramLocation {
                            start: position.start as u32,
                            end: position.end as u32,
                        },
                        bytecode_sha256: observation.sha256.clone(),
                        bytecode_pc: instruction.pc,
                        instruction: instruction.word,
                        allocation,
                    });
                }
                Err(reason) => {
                    failures += 1;
                    if first_failure.is_none() {
                        first_failure = Some(format!(
                            "constructor {}..{}: {reason}",
                            position.start, position.end
                        ));
                    }
                }
            }
        }
        if let Some(reason) = first_failure {
            lowered.constructor_unsupported.insert(
                program.callback,
                format!("{failures} unsupported constructor site(s); {reason}"),
            );
        }
    }
    let constructors = SourceProgramConstructors {
        schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
        profile: observations.profile.clone(),
        sites,
    };
    lowered.catalog = if let Some(creations) = lowered.catalog.closure_creations() {
        if creations.profile != constructors.profile {
            return Err(error("closure/table constructor profile mismatch"));
        }
        SourceProgramCatalog::new_with_closure_creations(
            lowered.catalog.data().clone(),
            owner.clone(),
            Some(constructors),
            creations.clone(),
        )
    } else {
        SourceProgramCatalog::new_with_constructors(
            lowered.catalog.data().clone(),
            owner.clone(),
            constructors,
        )
    }
    .map_err(error)?;
    Ok(lowered)
}

struct RetainedSiteBudget {
    sites: usize,
    text_bytes: usize,
}
impl RetainedSiteBudget {
    fn charge(&mut self, lengths: [usize; 4]) -> Result<()> {
        let sites = self
            .sites
            .checked_add(1)
            .ok_or_else(|| error("source constructor retained metadata bound"))?;
        let bytes = lengths
            .into_iter()
            .try_fold(self.text_bytes, |total, length| {
                if length > 4096 {
                    return None;
                }
                total.checked_add(length)
            })
            .ok_or_else(|| error("source constructor retained metadata bound"))?;
        if sites > SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES
            || bytes > SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES
        {
            return Err(error("source constructor retained metadata bound"));
        }
        self.sites = sites;
        self.text_bytes = bytes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cumulative_metadata_is_reserved_before_site_copy_and_refusal_is_atomic() {
        let lengths = [4096, 64, 64, 64];
        let per_site = lengths.iter().sum::<usize>();
        let mut budget = RetainedSiteBudget {
            sites: 0,
            text_bytes: SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES - per_site,
        };
        budget.charge(lengths).unwrap();
        assert_eq!(
            budget.text_bytes,
            SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES
        );
        let mut retained = 0;
        let result = (|| -> Result<()> {
            budget.charge(lengths)?;
            retained += 1;
            Ok(())
        })();
        assert!(result.is_err());
        assert_eq!(
            retained, 0,
            "no per-site copy can follow a refused reservation"
        );
        assert_eq!(budget.sites, 1);
        assert_eq!(
            budget.text_bytes,
            SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES
        );
        budget.sites = SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES;
        budget.text_bytes = 0;
        assert!(budget.charge(lengths).is_err());
        assert_eq!(budget.sites, SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES);
        assert_eq!(budget.text_bytes, 0);
        budget.sites = 0;
        assert!(budget.charge([4097, 0, 0, 0]).is_err());
        assert_eq!(budget.sites, 0);
        budget.text_bytes = usize::MAX;
        assert!(budget.charge(lengths).is_err());
        assert_eq!(budget.text_bytes, usize::MAX);
    }
}
