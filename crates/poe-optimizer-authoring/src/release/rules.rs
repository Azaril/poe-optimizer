//! Connect declared source packets to the sole release compiler. No evaluator,
//! include resolution, alias allocation, or identity repair happens here.
use super::{Decoder, ReleaseBuildError, ReleaseBuildLimits, ReleaseManifest, Result};
use crate::{NamesLimits, PacketLimits, RuleSource, load_names, lower_packet, parse_packet};
use poe_optimizer_core::owned_rules::RulePackageInput;
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use std::collections::{BTreeMap, BTreeSet};

fn charge(total: &mut usize, amount: usize, maximum: usize, label: &'static str) -> Result<()> {
    *total = total
        .checked_add(amount)
        .filter(|value| *value <= maximum)
        .ok_or(ReleaseBuildError::Limit(label))?;
    Ok(())
}

pub(super) fn compose(
    manifest: &ReleaseManifest,
    decoder: &Decoder<'_>,
    schema: &SchemaPackageInput,
    rules: &mut RulePackageInput,
    limits: ReleaseBuildLimits,
) -> Result<()> {
    // Temporary R1 bootstrap admission, removed with the tracked source cutover.
    // A source-based release never merges with or falls back to embedded programs.
    if manifest.packets.is_empty() {
        return Ok(());
    }
    if rules
        .owners
        .iter()
        .any(|owner| !owner.programs.members.is_empty())
    {
        return Err(ReleaseBuildError::Invalid(
            "rule packets require a program-free rules constituent",
        ));
    }
    let index = OwnedDefinitionSchemaPackage::new(schema.clone(), limits.release.recipe.schema)?;
    let declared: BTreeMap<_, _> = manifest
        .rules
        .iter()
        .map(|&position| (manifest.sources[position].path.as_str(), position))
        .collect();
    let mut used = BTreeSet::new();
    let mut expanded_bytes = 0;
    let mut programs = 0;
    let mut reads = 0;
    let mut nodes = 0;
    let mut effects = 0;
    let compile = limits.release.recipe.compile;
    for spec in &manifest.packets {
        let names_path = &manifest.sources[spec.names].path;
        let packet_path = &manifest.sources[spec.packet].path;
        let symbols = load_names(
            decoder.text(spec.names)?,
            &index,
            NamesLimits {
                manifest: limits.manifest,
                ..NamesLimits::default()
            },
        )
        .map_err(|source| ReleaseBuildError::SourceManifest {
            path: names_path.clone(),
            source,
        })?;
        let packet = parse_packet(decoder.text(spec.packet)?, &symbols, limits.manifest).map_err(
            |source| ReleaseBuildError::SourceManifest {
                path: packet_path.clone(),
                source,
            },
        )?;
        let mut referenced = BTreeMap::new();
        for append in &packet.appends {
            charge(
                &mut programs,
                append.programs.len(),
                compile.max_programs,
                "expanded rule programs",
            )?;
            for member in &append.programs {
                let &position =
                    declared
                        .get(member.source.as_str())
                        .ok_or(ReleaseBuildError::Invalid(
                            "packet reference must name a declared rule-source",
                        ))?;
                charge(
                    &mut expanded_bytes,
                    decoder.decoded[position].len(),
                    limits.max_expanded_rule_bytes,
                    "expanded rule bytes",
                )?;
                used.insert(position);
                referenced.insert(member.source.as_str(), position);
            }
        }
        let sources = referenced
            .iter()
            .map(|(&name, &position)| {
                Ok(RuleSource {
                    name,
                    text: decoder.text(position)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let lowered = lower_packet(
            &packet,
            &sources,
            &index,
            &symbols,
            &rules.tables,
            PacketLimits::default(),
        )
        .map_err(|source| ReleaseBuildError::Packet {
            path: packet_path.clone(),
            source,
        })?;
        // Count occurrences, not unique files or local program IDs. Reusing one
        // source at multiple owners still expands work in the compiled package.
        for append in lowered.appends() {
            for &position in &append.programs {
                let program = &lowered.programs()[position].lowered.program;
                charge(
                    &mut reads,
                    program.reads.len(),
                    compile.max_reads,
                    "expanded rule reads",
                )?;
                charge(
                    &mut nodes,
                    program.nodes.len(),
                    compile.max_nodes,
                    "expanded rule nodes",
                )?;
                charge(
                    &mut effects,
                    program.effects.len(),
                    compile.max_effects,
                    "expanded rule effects",
                )?;
            }
        }
        lowered
            .apply_to(rules)
            .map_err(|source| ReleaseBuildError::Packet {
                path: packet_path.clone(),
                source,
            })?;
    }
    if used != manifest.rules {
        return Err(ReleaseBuildError::Invalid("unused declared rule source"));
    }
    Ok(())
}
