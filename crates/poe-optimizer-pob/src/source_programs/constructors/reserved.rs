//! Explicit acquisition of the closed, source-bound reserved string-key family.
use super::*;
use capture::{ConstructorDiagnosticWitness, ConstructorTemplateValue};
use poe_optimizer_data::source_program::{
    SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES, SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE,
    SOURCE_PROGRAM_CONSTRUCTORS_MAX_RESERVED_KEYS, SourceTableRuntimeProfile,
};
use std::{collections::BTreeSet, ptr};

/// Attach self-marker-only TDUP facts obtained from exact live original witnesses.
/// Normal lowering remains unchanged, and serialized diagnostic reports cannot
/// supply this proof. Every witness must reference this exact input catalog.
/// A fresh catalog preserves the existing TNEW and closure-creation facets.
///
/// Keep the witnesses/original functions rooted throughout source comparison,
/// including GC tests: nil-valued instance keys do not root their strings, while
/// the retained original prototype/template does. The resulting native catalog
/// owns its key bytes independently and contains no Lua handles. This does not
/// prove behavior of source instances after their original template roots die,
/// nor does observation itself establish warm parity, capacity or tail length.
pub fn attach_reserved_string_templates(
    sources: &BTreeMap<String, String>,
    mut lowered: SourceProgramExtraction,
    witnesses: &[&ConstructorDiagnosticWitness],
) -> Result<SourceProgramExtraction> {
    if witnesses.is_empty() {
        return Ok(lowered);
    }
    if witnesses.len() > SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES {
        return Err(bound());
    }
    let catalog = lowered.catalog();
    validate_sources(sources, catalog.owner())?;
    let profile = catalog
        .constructors()
        .map(|value| &value.profile)
        .unwrap_or_else(|| witnesses[0].source_profile());
    if !profile.is_supported_array_profile() {
        return Err(error(
            "reserved template source runtime profile is not admitted",
        ));
    }
    let mut budget = RetainedBudget::new(profile);
    let mut identities = BTreeSet::new();
    if let Some(previous) = catalog.constructors() {
        for site in &previous.sites {
            budget.site(site)?;
            identities.insert((site.callback, site.expression.start, site.expression.end));
        }
    }
    // All bounds and borrowed witness checks precede retained metadata cloning.
    for witness in witnesses {
        if !same_catalog(catalog, witness.catalog()) {
            return Err(error(
                "reserved template witness belongs to a different catalog",
            ));
        }
        if witness.source_profile() != profile {
            return Err(error("reserved template source runtime profile mismatch"));
        }
        witness.verify_unchanged()?;
        let report = witness.report();
        if report.instruction.word & 255 != 53
            || report.template_constant_index != Some(-1 - (report.instruction.word >> 16) as i32)
            || witness.template().is_none()
        {
            return Err(error(
                "reserved template requires an original TDUP table constant",
            ));
        }
        if !identities.insert((
            report.callback,
            report.expression.start,
            report.expression.end,
        )) {
            return Err(error(
                "duplicate or already admitted reserved constructor site",
            ));
        }
        budget.header(&report.provenance, report.bytecode_sha256.len())?;
        budget.keys(
            report.template_rows.len(),
            report
                .template_rows
                .iter()
                .map(|row| match (&row.key, &row.value) {
                    (
                        ConstructorTemplateValue::Bytes(bytes),
                        ConstructorTemplateValue::SelfMarker,
                    ) => Ok(bytes.as_slice()),
                    _ => Err(error(
                        "reserved template requires only string keys and exact self markers",
                    )),
                }),
        )?;
    }
    let mut constructors =
        catalog
            .constructors()
            .cloned()
            .unwrap_or_else(|| SourceProgramConstructors {
                schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
                profile: profile.clone(),
                sites: Vec::new(),
            });
    let changed: BTreeSet<_> = witnesses
        .iter()
        .map(|witness| witness.report().callback)
        .collect();
    for witness in witnesses {
        let report = witness.report();
        let keys = report
            .template_rows
            .iter()
            .map(|row| {
                let ConstructorTemplateValue::Bytes(bytes) = &row.key else {
                    unreachable!("validated string key")
                };
                bytes.clone()
            })
            .collect();
        constructors.sites.push(SourceProgramConstructor {
            callback: report.callback,
            provenance: report.provenance.clone(),
            expression: report.expression,
            bytecode_sha256: report.bytecode_sha256.clone(),
            bytecode_pc: report.instruction.pc,
            instruction: report.instruction.word,
            allocation: SourceTableAllocation::DuplicateReservedStrings { keys },
        });
    }
    let next = if let Some(creations) = catalog.closure_creations() {
        if creations.profile != constructors.profile {
            return Err(error("closure/reserved constructor profile mismatch"));
        }
        SourceProgramCatalog::new_with_closure_creations(
            catalog.data().clone(),
            catalog.owner().clone(),
            Some(constructors),
            creations.clone(),
        )
    } else {
        SourceProgramCatalog::new_with_constructors(
            catalog.data().clone(),
            catalog.owner().clone(),
            constructors,
        )
    }
    .map_err(error)?;
    // Do not remove a callback's other constructor frontiers when one site gains
    // evidence. Recompute uncovered source occurrences with the complete lexer.
    for callback in changed {
        let program = next
            .for_callback(callback)
            .expect("validated constructor callback");
        let span = &program.provenance.source;
        let body = capture::closures::source_slice(&sources[&span.path], span.line, span.end_line)?;
        let function = body
            .get(
                program.provenance.function_start as usize
                    ..program.provenance.function_end as usize,
            )
            .ok_or_else(|| error("reserved constructor function byte range"))?;
        let mut lexical_budget = Budget::default();
        let tokens = lowering::lex(function, &mut lexical_budget).map_err(error)?;
        let positions = lexical::inventory(&tokens).map_err(error)?;
        let mut missing = positions.iter().filter(|position| {
            !identities.contains(&(callback, position.start as u32, position.end as u32))
        });
        if let Some(first) = missing.next() {
            lowered.constructor_unsupported.insert(callback, format!(
                "{} unsupported constructor site(s); constructor {}..{}: no admitted source constructor evidence",
                1 + missing.count(), first.start, first.end,
            ));
        } else {
            lowered.constructor_unsupported.remove(&callback);
        }
    }
    lowered.catalog = next;
    Ok(lowered)
}

fn same_optional<T>(left: Option<&T>, right: Option<&T>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => ptr::eq(left, right),
        _ => false,
    }
}
fn same_catalog(left: &SourceProgramCatalog, right: &SourceProgramCatalog) -> bool {
    left.owner().is_same_owner(right.owner())
        && ptr::eq(left.data(), right.data())
        && same_optional(left.constructors(), right.constructors())
        && same_optional(left.closure_creations(), right.closure_creations())
}
fn bound() -> GameDataExtractionError {
    error("reserved constructor retained metadata bound")
}
struct RetainedBudget {
    sites: usize,
    keys: usize,
    bytes: usize,
}
impl RetainedBudget {
    fn new(profile: &SourceTableRuntimeProfile) -> Self {
        Self {
            sites: 0,
            keys: 0,
            bytes: profile.source_revision.len(),
        }
    }
    fn header(
        &mut self,
        provenance: &poe_optimizer_data::source_program::SourceProgramProvenance,
        digest_bytes: usize,
    ) -> Result<()> {
        self.sites = self
            .sites
            .checked_add(1)
            .filter(|n| *n <= SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES)
            .ok_or_else(bound)?;
        for length in [
            provenance.source.path.len(),
            provenance.source.sha256.len(),
            provenance.function_sha256.len(),
            digest_bytes,
        ] {
            if length > 4096 {
                return Err(bound());
            }
            self.bytes = self
                .bytes
                .checked_add(length)
                .filter(|n| *n <= SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES)
                .ok_or_else(bound)?;
        }
        Ok(())
    }
    fn keys<'a>(
        &mut self,
        count: usize,
        keys: impl Iterator<Item = Result<&'a [u8]>>,
    ) -> Result<()> {
        if count == 0 || count > SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE {
            return Err(bound());
        }
        self.keys = self
            .keys
            .checked_add(count)
            .filter(|n| *n <= SOURCE_PROGRAM_CONSTRUCTORS_MAX_RESERVED_KEYS)
            .ok_or_else(bound)?;
        let mut unique = BTreeSet::new();
        for key in keys {
            let key = key?;
            if key.len() > SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES || !unique.insert(key) {
                return Err(bound());
            }
            self.bytes = self
                .bytes
                .checked_add(key.len())
                .filter(|n| *n <= SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES)
                .ok_or_else(bound)?;
        }
        Ok(())
    }
    fn site(&mut self, site: &SourceProgramConstructor) -> Result<()> {
        self.header(&site.provenance, site.bytecode_sha256.len())?;
        if let SourceTableAllocation::DuplicateReservedStrings { keys } = &site.allocation {
            self.keys(keys.len(), keys.iter().map(|key| Ok(key.as_slice())))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_count_and_text_reservations_fail_before_retained_clones() {
        let mut budget = RetainedBudget::new(&SourceTableRuntimeProfile::luajit21_x64_single());
        budget.keys = SOURCE_PROGRAM_CONSTRUCTORS_MAX_RESERVED_KEYS;
        assert!(
            budget
                .keys(
                    1,
                    std::iter::once_with(|| -> Result<&[u8]> {
                        panic!("aggregate key count must reject before inspecting/copying keys")
                    })
                )
                .is_err()
        );
        budget.keys = 0;
        assert!(
            budget
                .keys(
                    SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE + 1,
                    std::iter::once_with(|| -> Result<&[u8]> {
                        panic!("per-template count must reject first")
                    })
                )
                .is_err()
        );
        budget.bytes = SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES - 1;
        budget
            .keys(1, std::iter::once(Ok(b"a".as_slice())))
            .unwrap();
        let mut retained = 0;
        let refused = (|| -> Result<()> {
            budget.keys(1, std::iter::once(Ok(b"b".as_slice())))?;
            retained += 1;
            Ok(())
        })();
        assert!(refused.is_err());
        assert_eq!(retained, 0);
        let mut budget = RetainedBudget::new(&SourceTableRuntimeProfile::luajit21_x64_single());
        assert!(
            budget
                .keys(
                    2,
                    [Ok(b"same".as_slice()), Ok(b"same".as_slice())].into_iter()
                )
                .is_err()
        );
        assert!(budget.keys(0, std::iter::empty()).is_err());
        let too_long = [0; SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES + 1];
        assert!(
            budget
                .keys(1, std::iter::once(Ok(too_long.as_slice())))
                .is_err()
        );
    }
}
