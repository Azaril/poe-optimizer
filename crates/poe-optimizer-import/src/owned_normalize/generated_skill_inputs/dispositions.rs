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

/// Attempt-local proof, produced only by successful live accounting below.
/// No constructor, deserializer or archived-syntax path can create this token.
pub(in crate::owned_normalize) struct CheckedInputDisposition {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    set: SourceOccurrenceId,
    preset: SkillPresetId,
    correspondence: generated_skill_sources::GeneratedSkillSourceRule,
    binding: GeneratedSkillInputBindingDraft,
    usages: Vec<PresetUsageBindingDraft>,
    occurrence_attributes: BTreeSet<String>,
    group_attributes: BTreeSet<String>,
    descendants: Vec<SourceOccurrenceId>,
    provider: ProviderSnapshot,
}

enum ProviderSnapshot {
    Allocation {
        source: SourceOccurrenceId,
        preset: AllocationPresetId,
        allocation: Box<AllocationDraft>,
    },
    ItemModifier {
        sources: [SourceOccurrenceId; 3],
        preset: EquipmentPresetId,
        equipment: Box<EquipmentDraft>,
        item: Box<ItemDraft>,
    },
}

fn unique<T>(mut rows: impl Iterator<Item = T>) -> Option<T> {
    let first = rows.next()?;
    rows.next().is_none().then_some(first)
}

fn allocation_work(row: &AllocationDraft) -> usize {
    row.choices.members.len().saturating_add(1)
}

fn item_work(row: &ItemDraft) -> usize {
    let orders = [
        &row.modifier_orders.contributions,
        &row.modifier_orders.numeric_transforms,
    ]
    .into_iter()
    .fold(0usize, |work, order| {
        work.saturating_add(match order {
            DraftField::Known { value } => value.len(),
            DraftField::Pending(_) => 1,
        })
    });
    row.modifiers.members.iter().fold(
        row.parameters
            .members
            .len()
            .saturating_add(orders.saturating_add(1)),
        |work, modifier| work.saturating_add(modifier.rolls.members.len().saturating_add(1)),
    )
}

fn checked_link(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    expected: &OwnedOriginTarget,
    unique_kind: bool,
) -> Result<bool> {
    let Some(origin) = b.origins.get(source.ordinal() as usize) else {
        return Ok(false);
    };
    let count = origin.links.len();
    b.charge(count.saturating_mul(if unique_kind { 2 } else { 1 }))?;
    let links = &b.origins[source.ordinal() as usize].links;
    Ok(links.iter().filter(|link| *link == expected).count() == 1
        && (!unique_kind
            || links
                .iter()
                .filter(|link| std::mem::discriminant(*link) == std::mem::discriminant(expected))
                .count()
                == 1))
}

impl CheckedInputDisposition {
    pub(in crate::owned_normalize) fn source(&self) -> SourceOccurrenceId {
        self.source
    }
    pub(in crate::owned_normalize) fn group(&self) -> SourceOccurrenceId {
        self.group
    }
    pub(in crate::owned_normalize) fn set(&self) -> SourceOccurrenceId {
        self.set
    }
    pub(in crate::owned_normalize) fn preset(&self) -> SkillPresetId {
        self.preset
    }
    pub(in crate::owned_normalize) fn correspondence(
        &self,
    ) -> &generated_skill_sources::GeneratedSkillSourceRule {
        &self.correspondence
    }
    pub(in crate::owned_normalize) fn binding(&self) -> &GeneratedSkillInputBindingDraft {
        &self.binding
    }
    pub(in crate::owned_normalize) fn usage_bindings(&self) -> &[PresetUsageBindingDraft] {
        &self.usages
    }
    pub(in crate::owned_normalize) fn occurrence_attributes(&self) -> &BTreeSet<String> {
        &self.occurrence_attributes
    }
    pub(in crate::owned_normalize) fn group_attributes(&self) -> &BTreeSet<String> {
        &self.group_attributes
    }
    pub(in crate::owned_normalize) fn descendants(&self) -> &[SourceOccurrenceId] {
        &self.descendants
    }
    pub(in crate::owned_normalize) fn matches_outputs(&self, preset: &SkillPresetDraft) -> bool {
        let Some(intent) = &preset.intent else {
            return false;
        };
        preset.id == self.preset()
            && unique(
                intent
                    .generated_inputs
                    .members
                    .iter()
                    .filter(|binding| binding.target == self.binding().target),
            ) == Some(self.binding())
            && self.usage_bindings().iter().all(|binding| {
                unique(intent.usage.members.iter().filter(|actual| {
                    actual.selection.target == binding.selection.target
                        && actual.selection.policy == binding.selection.policy
                })) == Some(binding)
            })
    }

    /// Revalidate output snapshots and the actual supplying records. This checks
    /// correspondence, not selection of a future composed build or its legality.
    pub(in crate::owned_normalize) fn matches_current(
        &self,
        b: &mut Builder<'_, '_>,
        draft: &DraftSessionInput,
        preset: &SkillPresetDraft,
    ) -> Result<bool> {
        if preset.id != self.preset() {
            return Ok(false);
        }
        b.charge(draft.skill_presets.members.len())?;
        let Some(preset) = unique(
            draft
                .skill_presets
                .members
                .iter()
                .filter(|row| row.id == self.preset()),
        ) else {
            return Ok(false);
        };
        let Some(intent) = &preset.intent else {
            return Ok(false);
        };
        b.charge(
            intent
                .generated_inputs
                .members
                .len()
                .saturating_add(intent.usage.members.len()),
        )?;
        b.charge(
            intent
                .generated_inputs
                .members
                .iter()
                .fold(0usize, |work, row| {
                    work.saturating_add(row.parameters.members.len().saturating_add(1))
                }),
        )?;
        for _ in self.usage_bindings() {
            b.charge(intent.usage.members.len())?;
            b.charge(intent.usage.members.iter().fold(0usize, |work, row| {
                work.saturating_add(row.selection.parameters.members.len().saturating_add(1))
            }))?;
        }
        if !self.matches_outputs(preset)
            || b.evidence.rows()[self.source().ordinal() as usize]
                .occurrence()
                .parent()
                != Some(self.group())
            || b.evidence.rows()[self.group().ordinal() as usize]
                .occurrence()
                .parent()
                != Some(self.set())
        {
            return Ok(false);
        }
        for source in [self.set(), self.group(), self.source()]
            .into_iter()
            .chain(self.descendants().iter().copied())
        {
            if !checked_link(
                b,
                source,
                &OwnedOriginTarget::SkillPreset(self.preset()),
                true,
            )? {
                return Ok(false);
            }
        }
        let Some(target) = self.binding().target.to_resolved() else {
            return Ok(false);
        };
        let raw = OwnedOriginTarget::GeneratedSkillInput {
            skill_preset: self.preset(),
            target,
        };
        for source in [self.source(), self.group()] {
            if !checked_link(b, source, &raw, true)? {
                return Ok(false);
            }
        }
        if !checked_link(b, self.set(), &raw, false)? {
            return Ok(false);
        }
        self.provider.matches_current(b, draft, &raw)
    }
}

impl ProviderSnapshot {
    fn matches_current(
        &self,
        b: &mut Builder<'_, '_>,
        draft: &DraftSessionInput,
        raw: &OwnedOriginTarget,
    ) -> Result<bool> {
        match self {
            Self::Allocation {
                source,
                preset,
                allocation,
            } => {
                b.charge(
                    draft
                        .allocations
                        .members
                        .len()
                        .saturating_add(draft.allocation_presets.members.len()),
                )?;
                let Some(actual) = unique(
                    draft
                        .allocations
                        .members
                        .iter()
                        .filter(|row| row.id == allocation.id),
                ) else {
                    return Ok(false);
                };
                let Some(selection) = unique(
                    draft
                        .allocation_presets
                        .members
                        .iter()
                        .filter(|row| row.id == *preset),
                ) else {
                    return Ok(false);
                };
                b.charge(
                    allocation_work(actual).saturating_add(selection.allocations.members.len()),
                )?;
                Ok(actual == allocation.as_ref()
                    && selection
                        .allocations
                        .members
                        .iter()
                        .filter(|id| **id == allocation.id)
                        .count()
                        == 1
                    && checked_link(
                        b,
                        *source,
                        &OwnedOriginTarget::AllocationPreset(*preset),
                        true,
                    )?
                    && checked_link(
                        b,
                        *source,
                        &OwnedOriginTarget::Allocation(allocation.id),
                        false,
                    )?
                    && checked_link(b, *source, raw, false)?)
            }
            Self::ItemModifier {
                sources: [set, slot, source],
                preset,
                equipment,
                item,
            } => {
                b.charge(
                    draft
                        .equipment
                        .members
                        .len()
                        .saturating_add(draft.items.members.len())
                        .saturating_add(draft.equipment_presets.members.len()),
                )?;
                let Some(actual_equipment) = unique(
                    draft
                        .equipment
                        .members
                        .iter()
                        .filter(|row| row.id == equipment.id),
                ) else {
                    return Ok(false);
                };
                let Some(actual_item) =
                    unique(draft.items.members.iter().filter(|row| row.id == item.id))
                else {
                    return Ok(false);
                };
                let Some(selection) = unique(
                    draft
                        .equipment_presets
                        .members
                        .iter()
                        .filter(|row| row.id == *preset),
                ) else {
                    return Ok(false);
                };
                b.charge(actual_item.modifiers.members.len())?;
                b.charge(item_work(actual_item).saturating_add(selection.equipment.members.len()))?;
                if actual_equipment != equipment.as_ref()
                    || actual_item != item.as_ref()
                    || selection
                        .equipment
                        .members
                        .iter()
                        .filter(|id| **id == equipment.id)
                        .count()
                        != 1
                    || !checked_link(b, *set, &OwnedOriginTarget::EquipmentPreset(*preset), true)?
                    || !checked_link(b, *slot, &OwnedOriginTarget::Equipment(equipment.id), true)?
                    || !checked_link(b, *source, &OwnedOriginTarget::Item(item.id), true)?
                {
                    return Ok(false);
                }
                for origin in [*set, *slot, *source] {
                    if !checked_link(b, origin, raw, false)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}
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

fn matching_usage<'a>(
    receipt: &MaterializedInput,
    preset: SkillPresetId,
    usages: &'a [usage_inputs::MaterializedUsage],
) -> impl Iterator<Item = &'a PresetUsageBindingDraft> {
    let target = receipt
        .binding
        .target
        .to_resolved()
        .map(|target| UsageTarget::Skill(SkillTarget::Generated(Box::new(target))));
    let source = receipt.source;
    let group = receipt.group;
    usages
        .iter()
        .filter(move |row| {
            row.source == source
                && row.group == group
                && row.preset == preset
                && row.binding.applicability == PresetApplicability::WhenExactSourceSelected
                && row.binding.selection.target.to_resolved() == target
        })
        .map(|row| &row.binding)
}

fn capture_provider(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    inputs: &InputAccounting,
    receipt: &MaterializedInput,
) -> Result<Option<ProviderSnapshot>> {
    let Some(target) = receipt.binding.target.to_resolved() else {
        return Ok(None);
    };
    b.charge(inputs.resolved.len())?;
    for plan in &inputs.resolved {
        b.charge(plan.sources.len())?;
    }
    let Some(source) = unique(
        inputs
            .resolved
            .iter()
            .filter(|plan| plan.source == receipt.set && plan.preset_index == receipt.preset)
            .flat_map(|plan| &plan.sources)
            .filter(|source| {
                source.source == receipt.source
                    && source.group == receipt.group
                    && source.index == receipt.row
                    && source.target == target
            }),
    ) else {
        return Ok(None);
    };
    match (&target.provider.root, source.provider_sources.as_slice()) {
        (ProviderRoot::Allocation(id), [source]) => {
            let Some(preset) = skill_input_disposition::unique_link(b, *source, |link| {
                if let OwnedOriginTarget::AllocationPreset(id) = link {
                    Some(*id)
                } else {
                    None
                }
            })?
            else {
                return Ok(None);
            };
            b.charge(draft.allocations.members.len())?;
            let Some(allocation) =
                unique(draft.allocations.members.iter().filter(|row| row.id == *id))
            else {
                return Ok(None);
            };
            b.charge(allocation_work(allocation))?;
            Ok(Some(ProviderSnapshot::Allocation {
                source: *source,
                preset,
                allocation: Box::new(allocation.clone()),
            }))
        }
        (ProviderRoot::ItemModifier { equipment_use, .. }, [set, slot, source]) => {
            let Some(preset) = skill_input_disposition::unique_link(b, *set, |link| {
                if let OwnedOriginTarget::EquipmentPreset(id) = link {
                    Some(*id)
                } else {
                    None
                }
            })?
            else {
                return Ok(None);
            };
            b.charge(
                draft
                    .equipment
                    .members
                    .len()
                    .saturating_add(draft.items.members.len()),
            )?;
            let Some(equipment) = unique(
                draft
                    .equipment
                    .members
                    .iter()
                    .filter(|row| row.id == *equipment_use),
            ) else {
                return Ok(None);
            };
            let Some(item_id) = equipment.item.to_resolved() else {
                return Ok(None);
            };
            let Some(item) = unique(draft.items.members.iter().filter(|row| row.id == item_id))
            else {
                return Ok(None);
            };
            b.charge(item.modifiers.members.len())?;
            b.charge(item_work(item))?;
            Ok(Some(ProviderSnapshot::ItemModifier {
                sources: [*set, *slot, *source],
                preset,
                equipment: Box::new(equipment.clone()),
                item: Box::new(item.clone()),
            }))
        }
        _ => Ok(None),
    }
}

fn intrinsic_attributes(
    row: &SourceEvidenceRow<'_>,
    group: &SourceEvidenceRow<'_>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    // Literal enabled/primary-true gates add no restriction; they do not prove
    // participation or effects. Other global/count fields and group Action
    // selectors have only lexical or deferred checks here.
    let occurrence = row
        .attributes()
        .iter()
        .filter(|attribute| {
            !matches!(
                attribute.origin().name.as_str(),
                "enabled" | "enableGlobal1" | "enableGlobal2" | "count"
            ) || skill_input_disposition::exact_global_gate_is_neutral(attribute, "enableGlobal1")
                || skill_input_disposition::exact_enabled_gate_is_neutral(attribute)
        })
        .map(|attribute| attribute.origin().name.clone())
        .collect();
    let group = group
        .attributes()
        .iter()
        .filter(|attribute| {
            matches!(
                attribute.origin().name.as_str(),
                "source" | "slot" | "label"
            ) || skill_input_disposition::exact_enabled_gate_is_neutral(attribute)
        })
        .map(|attribute| attribute.origin().name.clone())
        .collect();
    (occurrence, group)
}

/// Only the successful live branch calls this after all reference/link checks.
/// Optional inventory accounting pays for its own snapshots and revalidation.
fn capture_checked(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    inputs: &InputAccounting,
    receipt: &MaterializedInput,
    row: &GeneratedSkillInputRule,
    usages: &[usage_inputs::MaterializedUsage],
    reference: Option<&skill_input_disposition::CheckedSourceReference>,
) -> Result<Option<CheckedInputDisposition>> {
    let children = reference.map_or(&[][..], |proof| proof.children());
    let Some(provider) = capture_provider(b, draft, inputs, receipt)? else {
        return Ok(None);
    };
    let preset = &draft.skill_presets.members[receipt.preset];
    b.charge(usages.len())?;
    for usage in matching_usage(receipt, preset.id, usages) {
        b.charge(usage.selection.parameters.members.len().saturating_add(1))?;
    }
    let source = &b.evidence.rows()[receipt.source.ordinal() as usize];
    let group = &b.evidence.rows()[receipt.group.ordinal() as usize];
    b.charge(
        source
            .attributes()
            .len()
            .saturating_add(group.attributes().len())
            .saturating_add(children.len())
            .saturating_add(receipt.binding.parameters.members.len())
            .saturating_add(reference.map_or(0, |_| 6))
            .saturating_add(1),
    )?;
    let source = &b.evidence.rows()[receipt.source.ordinal() as usize];
    let group = &b.evidence.rows()[receipt.group.ordinal() as usize];
    let (mut occurrence_attributes, group_attributes) = intrinsic_attributes(source, group);
    if let Some(reference) = reference {
        if reference.source() != receipt.source || reference.group() != Some(receipt.group) {
            return Ok(None);
        }
        // Both generated MAIN/CALCS inspections have already accounted for
        // every present selector and descendant. Retain only their proven
        // absence facts; no missing value becomes a native default.
        occurrence_attributes.extend(reference.absent_fields().map(str::to_owned));
    }
    let proof = CheckedInputDisposition {
        source: receipt.source,
        group: receipt.group,
        set: receipt.set,
        preset: preset.id,
        correspondence: generated_skill_sources::GeneratedSkillSourceRule {
            gem: row.gem.clone(),
            game_id: row.game_id.clone(),
            variant_id: row.variant_id.clone(),
            skill_id: row.skill_id.clone(),
            name_spec: row.name_spec.clone(),
            skill: row.skill.clone(),
            provider: row.provider.clone(),
            saved_level: row.saved_level.clone(),
            provider_level: row.provider_level.clone(),
        },
        binding: receipt.binding.clone(),
        usages: matching_usage(receipt, preset.id, usages)
            .cloned()
            .collect(),
        occurrence_attributes,
        group_attributes,
        descendants: children.to_vec(),
        provider,
    };
    Ok(proof.matches_current(b, draft, preset)?.then_some(proof))
}

/// Run only after both independent materializers. Failed accounting does not
/// remove their values or manufacture an obligation to make this proof succeed.
#[allow(clippy::too_many_arguments)]
pub(in crate::owned_normalize) fn account(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    compiled: Option<&CompiledGeneratedInputs<'_>>,
    direct: Option<&direct_skill_inputs::CompiledDirectInputs<'_>>,
    physical: Option<&gem_inventory::CompiledGemInventory<'_>>,
    inputs: &InputAccounting,
    usages: &[usage_inputs::MaterializedUsage],
    mut checked: Option<&mut Vec<CheckedInputDisposition>>,
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
        let reference = if let Some((adapter, _)) = selector_proof(bound.row, direct, physical)? {
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
            Some(children)
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
            None
        };
        let children = reference.as_ref().map_or(&[][..], |proof| proof.children());
        for source in [receipt.source, receipt.group]
            .into_iter()
            .chain(children.iter().copied())
        {
            link_once(b, source, OwnedOriginTarget::SkillPreset(preset.id))?;
            link_once(b, source, OwnedOriginTarget::Issue(issue))?;
            b.charge(b.origins[source.ordinal() as usize].links.len())?;
            b.origins[source.ordinal() as usize].links.retain(
                |link| !matches!(link, OwnedOriginTarget::Issue(id) if configuration.contains(id)),
            );
        }
        if let Some(checked) = checked.as_deref_mut()
            && let Some(proof) = capture_checked(
                b,
                draft,
                inputs,
                receipt,
                bound.row,
                usages,
                reference.as_ref(),
            )?
        {
            checked.push(proof);
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
        let children = children.into_children();
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

    fn blank(b: &Builder<'_, '_>) -> DraftSessionInput {
        DraftSessionInput {
            allocator: b.allocator.state(),
            revision: b.evidence.identity().revision,
            game_version: b.roles.input().namespace.clone(),
            weapon_loadouts: complete(vec![]),
            items: complete(vec![]),
            gems: complete(vec![]),
            rewards: complete(vec![]),
            equipment: complete(vec![]),
            allocations: complete(vec![]),
            skills: complete(vec![]),
            supports: complete(vec![]),
            payload_links: complete(vec![]),
            character_presets: complete(vec![]),
            equipment_presets: complete(vec![]),
            allocation_presets: complete(vec![]),
            skill_presets: complete(vec![]),
            choice_presets: complete(vec![]),
            scenario_presets: complete(vec![]),
            query_presets: complete(vec![]),
            saved_variants: complete(vec![]),
        }
    }

    fn with_provider(
        item: bool,
        test: impl FnOnce(&mut Builder<'_, '_>, DraftSessionInput, ProviderSnapshot, OwnedOriginTarget),
    ) {
        let xml = r#"<PathOfBuilding2><Tree><Spec/></Tree><Items><Item/><ItemSet><Slot/></ItemSet></Items><Skills><SkillSet><Skill enabled="true"><Gem enabled="true" enableGlobal1="true"/></Skill></SkillSet><SkillSet><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;
        gem_inventory::with_usage_inventory_destination(xml, |b, _, preset, _, _| {
            let ns = b.roles.input().namespace.clone();
            let mut draft = blank(b);
            let source = |name| {
                b.evidence
                    .rows()
                    .iter()
                    .find(|row| row.occurrence().name() == name)
                    .unwrap()
                    .occurrence()
                    .id()
            };
            let spec = source("Spec");
            let item_set = source("ItemSet");
            let slot = source("Slot");
            let item_source = source("Item");
            let (snapshot, root, owner, links) = if item {
                let modifier = ModifierDefId::new(ns.clone(), key("modifier"));
                let rolled = ModifierDraft {
                    id: b.id().unwrap(),
                    definition: modifier.clone().into(),
                    rolls: complete(vec![ParameterDraft {
                        slot: DeclaredSlot {
                            declaration: SlotOwnerDefId::Modifier(modifier.clone()),
                            slot: ParameterSlotDefId::new(ns.clone(), key("level")),
                        }
                        .into(),
                        value: ParameterValue::Integer(BoundedInteger::new(20).unwrap()).into(),
                    }]),
                };
                let item = ItemDraft {
                    id: b.id().unwrap(),
                    template: ItemTemplateDefId::new(ns.clone(), key("template")).into(),
                    parameters: complete(vec![]),
                    item_level: None.into(),
                    quality: DraftQuality::Known { value: None },
                    modifiers: complete(vec![rolled.clone()]),
                    modifier_orders: ItemModifierOrdersDraft {
                        contributions: vec![rolled.id].into(),
                        numeric_transforms: vec![rolled.id].into(),
                    },
                };
                let equipment = EquipmentDraft {
                    id: b.id().unwrap(),
                    item: item.id.into(),
                    destination: DraftEquipmentDestination::CharacterSlot(
                        EquipmentSlotDefId::new(ns.clone(), key("slot")).into(),
                    ),
                    scope: LoadoutScope::Shared.into(),
                };
                let selection = EquipmentPresetDraft {
                    id: b.id().unwrap(),
                    equipment: complete(vec![equipment.id]),
                };
                let links = vec![
                    (item_set, OwnedOriginTarget::EquipmentPreset(selection.id)),
                    (slot, OwnedOriginTarget::Equipment(equipment.id)),
                    (item_source, OwnedOriginTarget::Item(item.id)),
                ];
                let root = ProviderRoot::ItemModifier {
                    equipment_use: equipment.id,
                    modifier: rolled.id,
                };
                let snapshot = ProviderSnapshot::ItemModifier {
                    sources: [item_set, slot, item_source],
                    preset: selection.id,
                    equipment: Box::new(equipment.clone()),
                    item: Box::new(item.clone()),
                };
                draft.items.members.push(item);
                draft.equipment.members.push(equipment);
                draft.equipment_presets.members.push(selection);
                (snapshot, root, SlotOwnerDefId::Modifier(modifier), links)
            } else {
                let node = PassiveNodeDefId::new(ns.clone(), key("node"));
                let allocation = AllocationDraft {
                    id: b.id().unwrap(),
                    node: node.clone().into(),
                    pool: PointPoolDefId::new(ns.clone(), key("pool")).into(),
                    scope: LoadoutScope::Shared.into(),
                    access: DraftAllocationAccess::Ordinary,
                    choices: complete(vec![]),
                };
                let selection = AllocationPresetDraft {
                    id: b.id().unwrap(),
                    allocations: complete(vec![allocation.id]),
                    equipment: complete(vec![]),
                };
                let links = vec![
                    (spec, OwnedOriginTarget::AllocationPreset(selection.id)),
                    (spec, OwnedOriginTarget::Allocation(allocation.id)),
                ];
                let root = ProviderRoot::Allocation(allocation.id);
                let snapshot = ProviderSnapshot::Allocation {
                    source: spec,
                    preset: selection.id,
                    allocation: Box::new(allocation.clone()),
                };
                draft.allocations.members.push(allocation);
                draft.allocation_presets.members.push(selection);
                (snapshot, root, SlotOwnerDefId::PassiveNode(node), links)
            };
            let raw = OwnedOriginTarget::GeneratedSkillInput {
                skill_preset: preset.id,
                target: GeneratedSkillKey {
                    provider: ProviderKey {
                        root,
                        grant_path: vec![],
                    },
                    slot: DeclaredSlot {
                        declaration: owner,
                        slot: SkillGrantSlotDefId::new(ns, key("supply")),
                    },
                },
            };
            for (source, link) in links {
                b.link(source, link).unwrap();
                if !b.origins[source.ordinal() as usize].links.contains(&raw) {
                    b.link(source, raw.clone()).unwrap();
                }
            }
            draft.skill_presets.members.push(preset);
            test(b, draft, snapshot, raw);
        });
    }

    #[test]
    fn generated_provider_receipts_reject_stale_records_memberships_and_origins() {
        for item in [false, true] {
            with_provider(item, |b, draft, snapshot, raw| {
                assert!(snapshot.matches_current(b, &draft, &raw).unwrap());
                let baseline_origins = b.origins.clone();
                for mutation in 0..8 {
                    let mut changed = draft.clone();
                    b.origins = baseline_origins.clone();
                    if item {
                        match mutation {
                            0 => {
                                changed.items.members[0].template =
                                    ItemTemplateDefId::new(draft.game_version.clone(), key("other"))
                                        .into()
                            }
                            1 => {
                                changed.items.members[0].modifiers.members[0].rolls.members[0]
                                    .value =
                                    ParameterValue::Integer(BoundedInteger::new(21).unwrap()).into()
                            }
                            2 => {
                                let other: ItemRecordId = b.id().unwrap();
                                changed.equipment.members[0].item = other.into();
                            }
                            3 => changed.equipment_presets.members[0]
                                .equipment
                                .members
                                .clear(),
                            4 => changed.equipment_presets.members[0]
                                .equipment
                                .members
                                .push(changed.equipment.members[0].id),
                            5 => changed.items.members.push(changed.items.members[0].clone()),
                            6 => {
                                for origin in &mut b.origins {
                                    origin.links.retain(|link| {
                                        !matches!(link, OwnedOriginTarget::Equipment(_))
                                    });
                                }
                            }
                            _ => {
                                for origin in &mut b.origins {
                                    origin.links.retain(|link| link != &raw);
                                }
                            }
                        }
                    } else {
                        match mutation {
                            0 => {
                                changed.allocations.members[0].node =
                                    PassiveNodeDefId::new(draft.game_version.clone(), key("other"))
                                        .into()
                            }
                            1 => {
                                changed.allocations.members[0].pool =
                                    PointPoolDefId::new(draft.game_version.clone(), key("other"))
                                        .into()
                            }
                            2 => changed.allocation_presets.members[0]
                                .allocations
                                .members
                                .clear(),
                            3 => changed.allocation_presets.members[0]
                                .allocations
                                .members
                                .push(changed.allocations.members[0].id),
                            4 => changed
                                .allocations
                                .members
                                .push(changed.allocations.members[0].clone()),
                            5 => changed
                                .allocation_presets
                                .members
                                .push(changed.allocation_presets.members[0].clone()),
                            6 => {
                                for origin in &mut b.origins {
                                    origin.links.retain(|link| {
                                        !matches!(link, OwnedOriginTarget::Allocation(_))
                                    });
                                }
                            }
                            _ => {
                                for origin in &mut b.origins {
                                    origin.links.retain(|link| link != &raw);
                                }
                            }
                        }
                    }
                    assert!(
                        !snapshot.matches_current(b, &changed, &raw).unwrap(),
                        "item={item}, mutation={mutation}"
                    );
                }
                b.origins = baseline_origins;
                let before_origins = b.origins.clone();
                let before_allocator = b.allocator.state();
                b.limits.max_work = b.work;
                assert!(matches!(
                    snapshot.matches_current(b, &draft, &raw),
                    Err(NormalizationError::Limit(_))
                ));
                assert_eq!(b.origins, before_origins);
                assert_eq!(b.allocator.state(), before_allocator);
            });
        }
    }

    #[test]
    fn generated_intrinsic_receipts_only_add_exact_nonrestricting_gates() {
        let xml = r#"<PathOfBuilding2><Skills><SkillSet><Skill source="Tree:1" label="" enabled="false" groupCount="3" includeInFullDPS="true" mainActiveSkill="1" mainActiveSkillCalcs="1"><Gem gemId="g" variantId="v" skillId="s" nameSpec="n" level="1" quality="2" corrupted="nil" corruptLevel="nil" count="1" enabled="true" enableGlobal1="true" enableGlobal2="false"/></Skill></SkillSet><SkillSet><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;
        gem_inventory::with_usage_inventory_destination(xml, |b, _, _, _, source| {
            let row = &b.evidence.rows()[source.ordinal() as usize];
            let group = &b.evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
            let (occurrence, group) = intrinsic_attributes(row, group);
            assert_eq!(
                occurrence,
                [
                    "gemId",
                    "variantId",
                    "skillId",
                    "nameSpec",
                    "level",
                    "quality",
                    "corrupted",
                    "corruptLevel",
                    "enabled",
                    "enableGlobal1"
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()
            );
            assert_eq!(
                group,
                ["source", "label"].into_iter().map(str::to_owned).collect()
            );
        });
    }

    #[test]
    fn generated_enabled_receipts_require_exact_raw_and_decoded_true_per_field() {
        for token in [
            None,
            Some("true"),
            Some("false"),
            Some("nil"),
            Some(""),
            Some(" true"),
            Some("true "),
            Some("TRUE"),
            Some("1"),
            Some("unknown"),
            Some("tr&#117;e"),
        ] {
            for group_changed in [false, true] {
                let changed =
                    token.map_or_else(String::new, |token| format!(r#" enabled="{token}""#));
                let (group, gem) = if group_changed {
                    (changed.as_str(), r#" enabled="true""#)
                } else {
                    (r#" enabled="true""#, changed.as_str())
                };
                let xml = format!(
                    r#"<PathOfBuilding2><Skills><SkillSet><Skill{group}><Gem{gem}/></Skill></SkillSet><SkillSet><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#
                );
                gem_inventory::with_usage_inventory_destination(&xml, |b, _, _, _, source| {
                    let row = &b.evidence.rows()[source.ordinal() as usize];
                    let parent =
                        &b.evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
                    let (occurrence, group) = intrinsic_attributes(row, parent);
                    assert_eq!(
                        occurrence.contains("enabled"),
                        group_changed || token == Some("true"),
                        "Gem {token:?}, group_changed={group_changed}"
                    );
                    assert_eq!(
                        group.contains("enabled"),
                        !group_changed || token == Some("true"),
                        "group {token:?}, group_changed={group_changed}"
                    );
                });
            }
        }
    }

    #[test]
    fn live_generated_account_emits_only_after_existing_checks_and_preserves_absent_sink() {
        use crate::owned_value::{DecimalSyntax, ValueCodecInput, WhitespacePolicy};
        with_provider(false, |b, mut draft, _, raw| {
            let ns = draft.game_version.clone();
            let source = b
                .evidence
                .rows()
                .iter()
                .find(|row| row.occurrence().name() == "Gem")
                .unwrap()
                .occurrence()
                .id();
            let group = b.evidence.rows()[source.ordinal() as usize]
                .occurrence()
                .parent()
                .unwrap();
            let set = b.evidence.rows()[group.ordinal() as usize]
                .occurrence()
                .parent()
                .unwrap();
            let spec = b
                .evidence
                .rows()
                .iter()
                .find(|row| row.occurrence().name() == "Spec")
                .unwrap()
                .occurrence()
                .id();
            let OwnedOriginTarget::GeneratedSkillInput {
                target,
                skill_preset,
            } = &raw
            else {
                unreachable!()
            };
            let skill = SkillDefId::new(ns.clone(), key("generated"));
            let quality_slot = DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(skill.clone()),
                slot: ParameterSlotDefId::new(ns.clone(), key("quality")),
            };
            let recipe = |name| ValueRecipeInput {
                id: key(name),
                codec: ValueCodecInput {
                    namespace: ns.clone(),
                    whitespace: WhitespacePolicy::Exact,
                    codec: ValueCodecKind::Integer {
                        syntax: DecimalSyntax::Integer,
                    },
                },
                tiers: vec![ValueTier {
                    selectors: vec![ValueSelector {
                        lane: ValueLane::Attribute,
                        name: name.into(),
                    }],
                    duplicates: DuplicatePolicy::Reject,
                }],
                missing: MissingValuePolicy::Pending,
                numeric_aliases: vec![],
            };
            let quality = recipe("quality");
            let row = GeneratedSkillInputRule {
                gem: GemDefId::new(ns.clone(), key("catalog-only")),
                game_id: "game".into(),
                variant_id: "variant".into(),
                skill_id: "effect".into(),
                name_spec: "source".into(),
                skill,
                provider: GeneratedSkillInputProvider::TreeAllocation {
                    source_node_id: "1".into(),
                    passive: draft.allocations.members[0].node.to_resolved().unwrap(),
                    skill_supply: target.slot.clone(),
                },
                saved_level: recipe("level"),
                provider_level: GeneratedSkillProviderLevel::Fixed {
                    value: BoundedInteger::new(1).unwrap(),
                },
                parameters: vec![GeneratedSkillParameterInput {
                    field: GeneratedSkillInputField::Quality,
                    slot: quality_slot.clone(),
                    value: quality.clone(),
                }],
            };
            let compiled = CompiledGeneratedInputs {
                rows: vec![BoundRow {
                    row: &row,
                    quality: ValueRecipe::new(quality, Default::default()).unwrap(),
                    schema: ParameterSlotSchema {
                        value: ValueSchema::Integer(IntegerRange {
                            minimum: BoundedInteger::new(0).unwrap(),
                            maximum: BoundedInteger::new(100).unwrap(),
                        }),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![],
                        skill_input: None,
                    },
                }],
                sources: generated_skill_sources::CompiledSources::new(),
                work: 0,
            };
            let binding = GeneratedSkillInputBindingDraft {
                target: target.clone().into(),
                applicability: PresetApplicability::WhenExactSourceSelected,
                parameters: complete(vec![ParameterDraft {
                    slot: quality_slot.into(),
                    value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()).into(),
                }]),
            };
            let policy = UsagePolicyDefId::new(ns.clone(), key("count"));
            let usage = usage_inputs::MaterializedUsage {
                source,
                group,
                preset: *skill_preset,
                counts: true,
                binding: PresetUsageBindingDraft {
                    applicability: PresetApplicability::WhenExactSourceSelected,
                    selection: UsagePolicyDraft {
                        target: UsageTarget::Skill(SkillTarget::Generated(Box::new(
                            target.clone(),
                        )))
                        .into(),
                        policy: policy.clone().into(),
                        parameters: complete(vec![ParameterDraft {
                            slot: DeclaredSlot {
                                declaration: SlotOwnerDefId::UsagePolicy(policy),
                                slot: ParameterSlotDefId::new(ns.clone(), key("count-value")),
                            }
                            .into(),
                            value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()).into(),
                        }]),
                    },
                },
            };
            let participation = UsagePolicyDefId::new(ns.clone(), key("participation"));
            let participation_usage = usage_inputs::MaterializedUsage {
                source,
                group,
                preset: *skill_preset,
                counts: false,
                binding: PresetUsageBindingDraft {
                    applicability: PresetApplicability::WhenExactSourceSelected,
                    selection: UsagePolicyDraft {
                        target: usage.binding.selection.target.clone(),
                        policy: participation.clone().into(),
                        parameters: complete(
                            ["group-enabled", "occurrence-enabled"]
                                .into_iter()
                                .map(|name| ParameterDraft {
                                    slot: DeclaredSlot {
                                        declaration: SlotOwnerDefId::UsagePolicy(
                                            participation.clone(),
                                        ),
                                        slot: ParameterSlotDefId::new(ns.clone(), key(name)),
                                    }
                                    .into(),
                                    value: ParameterValue::Boolean(true).into(),
                                })
                                .collect(),
                        ),
                    },
                },
            };
            let usages = [usage, participation_usage];
            let prior_usage = draft.skill_presets.members[0]
                .usage_preferences
                .take()
                .unwrap();
            draft.skill_presets.members[0].intent = Some(SkillPresetIntentDraftV1 {
                schema_version: 1,
                usage: DraftList {
                    completion: prior_usage.completion,
                    members: usages.iter().map(|row| row.binding.clone()).collect(),
                },
                generated_inputs: complete(vec![binding.clone()]),
            });
            let inputs = InputAccounting {
                materialized: vec![MaterializedInput {
                    source,
                    group,
                    set,
                    preset: 0,
                    row: 0,
                    binding,
                }],
                archived: vec![],
                resolved: vec![generated_skill_sources::ResolvedPreset {
                    source: set,
                    preset_index: 0,
                    complete: true,
                    sources: vec![generated_skill_sources::ProvenSource {
                        source,
                        group,
                        provider_sources: vec![spec],
                        target: target.clone(),
                        index: 0,
                    }],
                    archived: vec![],
                }],
            };
            for origin in [source, group, set] {
                b.link(origin, raw.clone()).unwrap();
            }
            // This fixture starts after independent materializers. It tests the
            // real account gate, not source-rule compilation or provider discovery.
            let prior_origins = b.origins.clone();
            let before_work = b.work;
            let mut emitted = vec![];
            account(
                b,
                &draft,
                Some(&compiled),
                None,
                None,
                &inputs,
                &usages,
                Some(&mut emitted),
            )
            .unwrap();
            assert_eq!(emitted.len(), 1);
            assert!(emitted[0].occurrence_attributes().contains("enableGlobal1"));
            assert!(emitted[0].occurrence_attributes().contains("enabled"));
            assert!(emitted[0].group_attributes().contains("enabled"));
            assert_eq!(emitted[0].usage_bindings().len(), 2);
            assert!(!emitted[0].occurrence_attributes().contains("enableGlobal2"));
            assert!(
                emitted[0]
                    .matches_current(b, &draft, &draft.skill_presets.members[0])
                    .unwrap()
            );
            let checked_work = b.work - before_work;
            let checked_origins = b.origins.clone();
            b.origins = prior_origins.clone();
            b.work = before_work;
            account(
                b,
                &draft,
                Some(&compiled),
                None,
                None,
                &inputs,
                &usages,
                None,
            )
            .unwrap();
            assert_eq!(b.origins, checked_origins);
            assert!(b.work - before_work < checked_work);
            let absent_work = b.work - before_work;
            b.origins = prior_origins.clone();
            b.work = before_work;
            account(
                b,
                &draft,
                Some(&compiled),
                None,
                None,
                &inputs,
                &usages,
                None,
            )
            .unwrap();
            assert_eq!(b.work - before_work, absent_work);
            for mutation in 0..8 {
                let mut changed = draft.clone();
                let intent = changed.skill_presets.members[0].intent.as_mut().unwrap();
                match mutation {
                    0 => {
                        intent.generated_inputs.members[0].parameters.members[0].value =
                            ParameterValue::Integer(BoundedInteger::new(3).unwrap()).into()
                    }
                    1 => intent.usage.members.clear(),
                    2 => {
                        intent.usage.members[0].selection.parameters.members[0].value =
                            ParameterValue::Integer(BoundedInteger::new(3).unwrap()).into()
                    }
                    3 => {
                        let mut duplicate = intent.generated_inputs.members[0].clone();
                        duplicate.parameters.members[0].value =
                            ParameterValue::Integer(BoundedInteger::new(3).unwrap()).into();
                        intent.generated_inputs.members.push(duplicate);
                    }
                    4 => intent.usage.members.push(intent.usage.members[0].clone()),
                    5 | 6 => {
                        intent.usage.members[1].selection.parameters.members[mutation - 5].value =
                            ParameterValue::Boolean(false).into();
                    }
                    _ => {
                        let mut changed_target = target.clone();
                        changed_target.provider.root =
                            ProviderRoot::Allocation(AllocationId::from_instance_id(
                                InstanceId::from_parts(draft.allocator.lineage(), 999).unwrap(),
                            ));
                        intent.usage.members[1].selection.target =
                            UsageTarget::Skill(SkillTarget::Generated(Box::new(changed_target)))
                                .into();
                    }
                }
                b.origins = prior_origins.clone();
                let mut rejected = vec![];
                account(
                    b,
                    &changed,
                    Some(&compiled),
                    None,
                    None,
                    &inputs,
                    &usages,
                    Some(&mut rejected),
                )
                .unwrap();
                assert!(rejected.is_empty(), "mutation {mutation}");
                assert!(
                    !emitted[0]
                        .matches_current(b, &changed, &changed.skill_presets.members[0])
                        .unwrap()
                );
            }
            // Reuse the actual shared reference accountant. Its resolved
            // reports prove overwritten scalar-header absence, but without a
            // minion report they prove no minion-selector default or absence.
            b.origins = checked_origins;
            let evidence = b.evidence;
            let reference = account_reference(
                b,
                &evidence.rows()[source.ordinal() as usize],
                |_| {
                    Ok(crate::owned_source_actions::SourceActionInspection {
                        resolved: true,
                        selection: crate::owned_source_actions::SourceActionSelection::Absent,
                        ignored_legacy_attributes: vec![],
                        minion: None,
                        work: 0,
                    })
                },
                0,
            )
            .unwrap()
            .unwrap();
            let retained = capture_checked(
                b,
                &draft,
                &inputs,
                &inputs.materialized[0],
                &row,
                &usages,
                Some(&reference),
            )
            .unwrap()
            .unwrap();
            assert!(retained.occurrence_attributes().contains("statSetIndex"));
            assert!(
                retained
                    .occurrence_attributes()
                    .contains("statSetIndexCalcs")
            );
            for field in [
                "skillMinion",
                "skillMinionCalcs",
                "skillMinionSkill",
                "skillMinionSkillCalcs",
            ] {
                assert!(!retained.occurrence_attributes().contains(field));
            }
            assert_eq!(retained.binding(), emitted[0].binding());
            assert_eq!(retained.usage_bindings(), emitted[0].usage_bindings());
            assert!(!emitted[0].occurrence_attributes().contains("statSetIndex"));
            let foreign = skill_input_disposition::CheckedSourceReference::consumer_fixture(
                set,
                Some(group),
                vec![],
            );
            assert!(
                capture_checked(
                    b,
                    &draft,
                    &inputs,
                    &inputs.materialized[0],
                    &row,
                    &usages,
                    Some(&foreign),
                )
                .unwrap()
                .is_none()
            );
        });
    }

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
