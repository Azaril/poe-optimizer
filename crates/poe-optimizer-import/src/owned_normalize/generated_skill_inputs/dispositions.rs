//! Complete source-field accounting after independent raw/count materialization.
//! Materialized receipts prove output correspondence. Archived syntax retains
//! separate Pending raw-input, usage and authored-support owners and proves no
//! generated target or output. No draft values or IDs are changed.
use super::*;
use crate::owned_normalize::skill_input_disposition::{
    account_reference, link_once, pending_generated_responsibilities, pending_intent_usage,
};
use source_shape::{container_text, plain_row, value};

const GEM_FIELDS: &[&str] = &[
    "gemId",
    "variantId",
    "skillId",
    "nameSpec",
    "level",
    "quality",
    "count",
    "enabled",
    "enableGlobal1",
    "enableGlobal2",
    "corrupted",
    "corruptLevel",
    "statSetIndex",
    "statSetIndexCalcs",
    "skillMinion",
    "skillMinionCalcs",
    "skillMinionSkill",
    "skillMinionSkillCalcs",
];
const GROUP_FIELDS: &[&str] = &[
    "source",
    "slot",
    "enabled",
    "label",
    "groupCount",
    "includeInFullDPS",
    "mainActiveSkill",
    "mainActiveSkillCalcs",
];
const SELECTOR_FIELDS: &[&str] = &[
    "statSetIndex",
    "statSetIndexCalcs",
    "skillMinion",
    "skillMinionCalcs",
    "skillMinionSkill",
    "skillMinionSkillCalcs",
];

type SelectorProof<'a> = (
    &'a crate::owned_source_actions::SourceActionCorrespondence,
    &'a skill_input_disposition::CompiledDeferredUsage,
);
fn selector_proof<'a>(
    row: &GeneratedSkillInputRule,
    direct: Option<&'a direct_skill_inputs::CompiledDirectInputs<'_>>,
    physical: Option<&'a gem_inventory::CompiledGemInventory<'_>>,
) -> Result<Option<SelectorProof<'a>>> {
    let direct = direct.and_then(|d| {
        d.generated_selector_adapter(row)
            .zip(d.generated_deferred_usage(row))
    });
    let physical = physical.and_then(|p| p.generated_proof(row));
    unique_selector_proof(direct, physical)
}
fn unique_selector_proof<T>(direct: Option<T>, physical: Option<T>) -> Result<Option<T>> {
    match (direct, physical) {
        (Some(_), Some(_)) => invalid("ambiguous generated selector proof"),
        (Some(proof), None) | (None, Some(proof)) => Ok(Some(proof)),
        (None, None) => Ok(None),
    }
}
fn saved_switch(row: &SourceEvidenceRow<'_>, name: &str, nil: bool) -> bool {
    matches!(value(row, name), None | Some("true" | "false"))
        || (nil && value(row, name) == Some("nil"))
}
fn frame(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
) -> Result<bool> {
    let row = &b.evidence.rows()[source.ordinal() as usize];
    let parent = &b.evidence.rows()[group.ordinal() as usize];
    source_shape::charge_row(b, row)?;
    source_shape::charge_row(b, parent)?;
    Ok(
        row.occurrence().name() == "Gem" && row.occurrence().parent() == Some(group)
        && parent.occurrence().name() == "Skill" && parent.children() == [source]
        && plain_row(row, GEM_FIELDS, false) && container_text(row)
        && plain_row(parent, GROUP_FIELDS, false) && container_text(parent)
        && saved_switch(row, "enabled", false) && saved_switch(parent, "enabled", false)
        && saved_switch(row, "enableGlobal1", true) && saved_switch(row, "enableGlobal2", true)
        && saved_switch(parent, "includeInFullDPS", true)
        && matches!(value(parent, "label"), None | Some(""))
        // These are saved generated-source sentinels, not corruption defaults.
        && matches!(value(row, "corrupted"), None | Some("nil"))
        && matches!(value(row, "corruptLevel"), None | Some("nil"))
        && ["mainActiveSkill", "mainActiveSkillCalcs"].iter()
            .all(|name| matches!(value(parent, name), None | Some("nil" | "1"))),
    )
}
fn outputs_match(
    preset: &SkillPresetDraft,
    receipt: &MaterializedInput,
    usages: &[usage_inputs::MaterializedUsage],
) -> bool {
    let Some(intent) = &preset.intent else {
        return false;
    };
    let Some(target) = receipt.binding.target.to_resolved() else {
        return false;
    };
    if receipt.binding.applicability != PresetApplicability::WhenExactSourceSelected
        || intent
            .generated_inputs
            .members
            .iter()
            .filter(|row| **row == receipt.binding)
            .count()
            != 1
    {
        return false;
    }
    let target = UsageTarget::Skill(SkillTarget::Generated(Box::new(target)));
    let mut count = false;
    for row in usages.iter().filter(|row| {
        row.source == receipt.source
            && row.group == receipt.group
            && row.preset == preset.id
            && row.binding.applicability == PresetApplicability::WhenExactSourceSelected
            && row.binding.selection.target.to_resolved().as_ref() == Some(&target)
    }) {
        if intent
            .usage
            .members
            .iter()
            .filter(|actual| **actual == row.binding)
            .count()
            != 1
        {
            return false;
        }
        count |= row.counts;
    }
    count
}
/// Run only after both independent materializers. Failed accounting does not
/// remove their values or manufacture an obligation to make this proof succeed.
pub(in crate::owned_normalize) fn account(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    compiled: Option<&CompiledGeneratedInputs<'_>>,
    direct: Option<&direct_skill_inputs::CompiledDirectInputs<'_>>,
    physical: Option<&gem_inventory::CompiledGemInventory<'_>>,
    inputs: &InputAccounting,
    usages: &[usage_inputs::MaterializedUsage],
) -> Result<()> {
    let Some(compiled) = compiled else {
        return Ok(());
    };
    b.charge(draft.choice_presets.members.len())?;
    let configuration: BTreeSet<_> = draft
        .choice_presets
        .members
        .iter()
        .filter_map(|p| match &p.choices.completion {
            DraftListCompletion::Pending { id, code }
                if code.as_str() == "configuration-roles-not-converted" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    for receipt in &inputs.materialized {
        b.charge(1)?;
        let preset = &draft.skill_presets.members[receipt.preset];
        let Some(intent) = &preset.intent else {
            continue;
        };
        let comparison_work = intent.generated_inputs.members.len().saturating_add(
            usages.len().saturating_mul(
                intent
                    .usage
                    .members
                    .iter()
                    .map(|row| row.selection.parameters.members.len().saturating_add(1))
                    .sum::<usize>()
                    .saturating_add(1),
            ),
        );
        b.charge(comparison_work)?;
        if !outputs_match(preset, receipt, usages)
            || b.evidence.rows()[receipt.group.ordinal() as usize]
                .occurrence()
                .parent()
                != Some(receipt.set)
            || !frame(b, receipt.source, receipt.group)?
        {
            continue;
        }
        let Some(issue) = pending_intent_usage(b, receipt.set, preset)? else {
            continue;
        };
        let bound = &compiled.rows[receipt.row];
        let row = &b.evidence.rows()[receipt.source.ordinal() as usize];
        let children = if let Some((adapter, _)) = selector_proof(bound.row, direct, physical)? {
            let evidence = b.evidence;
            let Some(children) = account_reference(
                b,
                row,
                |context| {
                    Ok(adapter.inspect_generated_selectors(evidence, receipt.source, context)?)
                },
                adapter.construction_work(),
            )?
            else {
                continue;
            };
            children
        } else {
            // With no checked topology adapter only actual selector absence is
            // provable. No default Action, Actor or stat-set is invented.
            if !row.children().is_empty()
                || SELECTOR_FIELDS
                    .iter()
                    .any(|name| row.attribute(name).is_some())
            {
                continue;
            }
            vec![]
        };
        for source in [receipt.source, receipt.group].into_iter().chain(children) {
            link_once(b, source, OwnedOriginTarget::SkillPreset(preset.id))?;
            link_once(b, source, OwnedOriginTarget::Issue(issue))?;
            b.charge(b.origins[source.ordinal() as usize].links.len())?;
            b.origins[source.ordinal() as usize].links.retain(
                |link| !matches!(link, OwnedOriginTarget::Issue(id) if configuration.contains(id)),
            );
        }
    }
    account_archived(
        b,
        draft,
        compiled,
        direct,
        physical,
        &inputs.archived,
        &configuration,
    )?;
    Ok(())
}

fn account_archived(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    compiled: &CompiledGeneratedInputs<'_>,
    direct: Option<&direct_skill_inputs::CompiledDirectInputs<'_>>,
    physical: Option<&gem_inventory::CompiledGemInventory<'_>>,
    inputs: &[ArchivedInput],
    configuration: &BTreeSet<DraftIssueId>,
) -> Result<()> {
    for receipt in inputs {
        b.charge(1)?;
        let preset = &draft.skill_presets.members[receipt.preset];
        if b.evidence.rows()[receipt.group.ordinal() as usize]
            .occurrence()
            .parent()
            != Some(receipt.set)
            || !frame(b, receipt.source, receipt.group)?
        {
            continue;
        }
        let Some(issues) = pending_generated_responsibilities(b, receipt.set, preset)? else {
            continue;
        };
        let bound = &compiled.rows[receipt.row];
        let evidence = b.evidence;
        let row = &evidence.rows()[receipt.source.ordinal() as usize];
        let group = &evidence.rows()[receipt.group.ordinal() as usize];
        if !generated_skill_sources::scalar(b, row, &bound.quality)?
            .is_some_and(|value| gem_inputs::value_valid(&value, &bound.schema.value))
        {
            continue;
        }
        let Some((adapter, usage)) = selector_proof(bound.row, direct, physical)? else {
            continue;
        };
        if !usage.prove_generated(b, row, group)? {
            continue;
        }
        let Some(children) = account_reference(
            b,
            row,
            |context| Ok(adapter.inspect_generated_selectors(evidence, receipt.source, context)?),
            adapter.construction_work(),
        )?
        else {
            continue;
        };
        // The receipt recognizes external syntax only. No live physical or
        // generated output may be relabelled as an unresolved archived row.
        b.charge(children.len().saturating_add(2))?;
        let sources: Vec<_> = [receipt.source, receipt.group]
            .into_iter()
            .chain(children)
            .collect();
        let mut unbound = true;
        for source in &sources {
            let links = &b.origins[source.ordinal() as usize].links;
            b.charge(links.len())?;
            if b.origins[source.ordinal() as usize]
                .links
                .iter()
                .any(|link| {
                    matches!(
                        link,
                        OwnedOriginTarget::Gem(_)
                            | OwnedOriginTarget::Skill(_)
                            | OwnedOriginTarget::Support(_)
                            | OwnedOriginTarget::GeneratedSkillInput { .. }
                    )
                })
            {
                unbound = false;
            }
        }
        if !unbound {
            continue;
        }
        for source in sources {
            link_once(b, source, OwnedOriginTarget::SkillPreset(preset.id))?;
            for issue in issues {
                link_once(b, source, OwnedOriginTarget::Issue(issue))?;
            }
            b.charge(b.origins[source.ordinal() as usize].links.len())?;
            b.origins[source.ordinal() as usize].links.retain(
                |link| !matches!(link, OwnedOriginTarget::Issue(id) if configuration.contains(id)),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambiguous_selector_proofs_never_have_precedence_even_when_equal() {
        assert!(unique_selector_proof(Some("same"), Some("same")).is_err());
        assert!(unique_selector_proof(Some("direct"), Some("physical")).is_err());
        assert_eq!(
            unique_selector_proof(Some("direct"), None).unwrap(),
            Some("direct")
        );
        assert_eq!(
            unique_selector_proof(None, Some("physical")).unwrap(),
            Some("physical")
        );
        assert_eq!(unique_selector_proof::<()>(None, None).unwrap(), None);
    }
    #[test]
    fn materialized_receipts_require_exact_live_values_targets_and_single_binding() {
        let namespace = GameVersionNamespace::new("receipt-test", "one").unwrap();
        let lineage = BuildLineage::from_bytes([9; 16]);
        let instance = |n| InstanceId::from_parts(lineage, n).unwrap();
        let node = PassiveNodeDefId::new(namespace.clone(), key("source"));
        let target = GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::Allocation(AllocationId::from_instance_id(instance(1))),
                grant_path: vec![],
            },
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::PassiveNode(node),
                slot: SkillGrantSlotDefId::new(namespace.clone(), key("supply")),
            },
        };
        let skill = SkillDefId::new(namespace.clone(), key("skill"));
        let raw = GeneratedSkillInputBindingDraft {
            target: target.clone().into(),
            applicability: PresetApplicability::WhenExactSourceSelected,
            parameters: complete(vec![ParameterDraft {
                slot: DeclaredSlot {
                    declaration: SlotOwnerDefId::Skill(skill),
                    slot: ParameterSlotDefId::new(namespace.clone(), key("quality")),
                }
                .into(),
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(12.0, UnitDefId::new(namespace.clone(), key("percent")))
                        .unwrap(),
                )
                .into(),
            }]),
        };
        let policy = UsagePolicyDefId::new(namespace.clone(), key("count"));
        let usage = PresetUsageBindingDraft {
            applicability: PresetApplicability::WhenExactSourceSelected,
            selection: UsagePolicyDraft {
                target: UsageTarget::Skill(SkillTarget::Generated(Box::new(target))).into(),
                policy: policy.clone().into(),
                parameters: complete(vec![ParameterDraft {
                    slot: DeclaredSlot {
                        declaration: SlotOwnerDefId::UsagePolicy(policy),
                        slot: ParameterSlotDefId::new(namespace, key("count-slot")),
                    }
                    .into(),
                    value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()).into(),
                }]),
            },
        };
        let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
            crate::decode_build(b"<PathOfBuilding2><Skills><SkillSet><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>").unwrap(),
            lineage, Default::default()).unwrap();
        let rows = imported.occurrences();
        let preset = SkillPresetDraft {
            id: SkillPresetId::from_instance_id(instance(2)),
            skills: complete(vec![]),
            supports: complete(vec![]),
            authored_support_order: None,
            payload_links: complete(vec![]),
            usage_preferences: None,
            intent: Some(SkillPresetIntentDraftV1 {
                schema_version: 1,
                usage: complete(vec![usage.clone()]),
                generated_inputs: complete(vec![raw.clone()]),
            }),
        };
        let receipt = MaterializedInput {
            source: rows[4].id(),
            group: rows[3].id(),
            set: rows[2].id(),
            preset: 0,
            row: 0,
            binding: raw,
        };
        let usage = usage_inputs::MaterializedUsage {
            source: rows[4].id(),
            group: rows[3].id(),
            preset: preset.id,
            binding: usage,
            counts: true,
        };
        assert!(outputs_match(
            &preset,
            &receipt,
            std::slice::from_ref(&usage)
        ));
        for case in 0..7 {
            let mut changed = preset.clone();
            let intent = changed.intent.as_mut().unwrap();
            match case {
                0 => {
                    intent.generated_inputs.members[0].parameters.members[0].value =
                        ParameterValue::Integer(BoundedInteger::new(99).unwrap()).into()
                }
                1 => {
                    intent.usage.members[0].selection.parameters.members[0].value =
                        ParameterValue::Integer(BoundedInteger::new(0).unwrap()).into()
                }
                2 => intent
                    .generated_inputs
                    .members
                    .push(receipt.binding.clone()),
                3 => intent.usage.members.push(usage.binding.clone()),
                4 => changed.id = SkillPresetId::from_instance_id(instance(3)),
                5 => intent.usage.members[0].applicability = PresetApplicability::Required,
                6 => {
                    intent.generated_inputs.members[0].applicability = PresetApplicability::Required
                }
                _ => unreachable!(),
            }
            assert!(
                !outputs_match(&changed, &receipt, std::slice::from_ref(&usage)),
                "case {case}"
            );
        }
        assert!(!outputs_match(&preset, &receipt, &[]));
    }
}
