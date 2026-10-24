//! Named query metadata composes after Stat allocation into the existing DTOs.
//! The ordinary storage/compiler validates the complete final contribution graph.
use super::{Decoder, ReleaseBuildError, ReleaseBuildLimits, ReleaseManifest, Result};
use crate::{NamesLimits, StagedStatDeclarations, load_names, parse_contribution_queries};
use poe_optimizer_core::{
    owned_rules::{ContributionOrigin, ContributionProducer, ContributionQuery, RulePackageInput},
    owned_schema::{DeclaredSet, SchemaClosure},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use std::collections::BTreeSet;

#[derive(Default)]
struct Usage {
    queries: usize,
    groups: usize,
    members: usize,
    slots: usize,
    gaps: usize,
}

fn charge(used: &mut usize, count: usize, maximum: usize, label: &'static str) -> Result<()> {
    *used = used
        .checked_add(count)
        .filter(|value| *value <= maximum)
        .ok_or(ReleaseBuildError::Limit(label))?;
    Ok(())
}

impl Usage {
    // Charge aggregate retained payload before appending any packet. The same
    // final validators still own graph work, exact permissions and reduction laws.
    fn charge(
        &mut self,
        packet: &DeclaredSet<ContributionQuery>,
        limits: ReleaseBuildLimits,
    ) -> Result<()> {
        let storage = limits.release.recipe.rules;
        let compile = limits.release.recipe.compile;
        charge(
            &mut self.queries,
            packet.members.len(),
            storage.max_ordered_queries.min(compile.max_programs),
            "contribution queries",
        )?;
        self.closure(&packet.closure, limits)?;
        for query in &packet.members {
            charge(
                &mut self.groups,
                query.groups.len(),
                storage.max_ordered_groups.min(compile.max_reads),
                "contribution groups",
            )?;
            for group in &query.groups {
                charge(
                    &mut self.members,
                    group.members.members.len(),
                    storage.max_ordered_members.min(compile.max_effects),
                    "contribution members",
                )?;
                self.closure(&group.members.closure, limits)?;
                for member in &group.members.members {
                    let origin_slots = match &member.producer {
                        ContributionProducer::ProgramEffect(producer) => match &producer.origin {
                            ContributionOrigin::EquipmentUse { slots }
                            | ContributionOrigin::ItemModifier { slots } => slots.len(),
                            ContributionOrigin::SuppliedActor { slots } => slots.len(),
                            ContributionOrigin::Skill { supplies, .. }
                            | ContributionOrigin::Action { supplies, .. } => supplies.len(),
                            ContributionOrigin::Character
                            | ContributionOrigin::Encounter
                            | ContributionOrigin::Allocation
                            | ContributionOrigin::ExistingActor { .. }
                            | ContributionOrigin::Reward => 0,
                        },
                        ContributionProducer::ApplicationGroup(producer) => {
                            charge(
                                &mut self.members,
                                producer.declarations.len(),
                                storage.max_ordered_members.min(compile.max_effects),
                                "contribution members",
                            )?;
                            0
                        }
                    };
                    charge(
                        &mut self.slots,
                        origin_slots,
                        storage.max_ordered_slots.min(compile.max_receiver_targets),
                        "contribution source bindings",
                    )?;
                    // Slot ranks consume ordered work, not source bindings;
                    // final storage/compiler checks own that separate budget.
                }
            }
        }
        Ok(())
    }

    fn closure(&mut self, closure: &SchemaClosure, limits: ReleaseBuildLimits) -> Result<()> {
        if let SchemaClosure::Partial { gaps } = closure {
            charge(
                &mut self.gaps,
                gaps.len(),
                limits
                    .release
                    .recipe
                    .rules
                    .max_gaps
                    .min(limits.release.recipe.compile.max_gaps),
                "contribution gaps",
            )?;
        }
        Ok(())
    }
}

pub(super) fn compose(
    manifest: &ReleaseManifest,
    decoder: &Decoder<'_>,
    schema: &OwnedDefinitionSchemaPackage,
    rules: &mut RulePackageInput,
    declarations: &StagedStatDeclarations,
    limits: ReleaseBuildLimits,
) -> Result<()> {
    if rules
        .contribution_queries
        .as_ref()
        .is_some_and(|queries| !queries.members.is_empty())
    {
        return Err(ReleaseBuildError::Invalid(
            "release requires query-free rules constituent",
        ));
    }
    if manifest.contribution_packets.is_empty() {
        return Ok(());
    }
    let mut output = rules
        .contribution_queries
        .take()
        .unwrap_or_else(|| DeclaredSet::complete(vec![]));
    let mut usage = Usage::default();
    usage.charge(&output, limits)?;
    let mut ids = BTreeSet::new();
    for spec in &manifest.contribution_packets {
        let path = &manifest.sources[spec.packet].path;
        let names = &manifest.sources[spec.names].path;
        let aliases = load_names(
            decoder.text(spec.names)?,
            schema,
            NamesLimits {
                manifest: limits.manifest,
                ..Default::default()
            },
        )
        .map_err(|source| ReleaseBuildError::SourceManifest {
            path: names.clone(),
            source,
        })?;
        let symbols = declarations
            .symbols_for_packet(&aliases, limits.declarations.symbols)
            .map_err(|source| ReleaseBuildError::PacketDeclarations {
                path: path.clone(),
                names: names.clone(),
                source,
            })?;
        let packet = parse_contribution_queries(
            decoder.text(spec.packet)?,
            schema,
            &symbols,
            limits.manifest,
        )
        .map_err(|source| ReleaseBuildError::SourceManifest {
            path: path.clone(),
            source,
        })?;
        usage.charge(&packet, limits)?;
        for query in &packet.members {
            if !ids.insert(query.id.clone()) {
                return Err(ReleaseBuildError::Invalid(
                    "duplicate contribution query across sources",
                ));
            }
        }
        output.members.extend(packet.members);
        if let SchemaClosure::Partial { gaps } = packet.closure {
            match &mut output.closure {
                SchemaClosure::Complete => output.closure = SchemaClosure::Partial { gaps },
                SchemaClosure::Partial { gaps: existing } => existing.extend(gaps),
            }
        }
    }
    rules.contribution_queries = Some(output);
    Ok(())
}
