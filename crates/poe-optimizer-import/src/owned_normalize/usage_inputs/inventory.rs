//! Completion authority for saved preset usage, separate from value projection.
//! Receipts are private, attempt-local, and checked against the current draft.
use super::*;
use poe_optimizer_core::owned_preset_intent::PresetApplicability;
mod group_selectors;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageInventoryPolicy {
    pub evidence: OwnedContentDigest,
    pub rows: Vec<UsageInventoryRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageInventoryRow {
    pub source: UsageInventorySource,
    /// Every destination is required, including consumers of a losing lane.
    pub fields: Vec<UsageInventoryField>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInventorySource {
    Physical { gem: GemDefId },
    Occurrence { target: Box<OccurrenceUsageTarget> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageInventoryField {
    pub field: UsageInventoryFieldSelector,
    pub destinations: Vec<UsageInventoryDestination>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInventoryFieldSelector {
    OccurrenceAttribute { name: String },
    GroupAttribute { name: String },
    ReferenceDescendants,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInventoryDestination {
    /// Successful existing physical, Direct, support or generated-input disposition.
    /// Authored/Direct receipts include checked literal primary/secondary-true
    /// source gates; generated receipts include only primary-true. Neither
    /// enables an effect. Exact physical
    /// singleton construction can additionally account for the unread secondary
    /// source switch; Direct/generated dispatch is not covered. A checked physical
    /// SupportAssignment with complete singleton construction may account count=1
    /// and enableGlobal1=enableGlobal2=true only when raw and decoded text agree.
    /// Final whole-group accounting may additionally prove
    /// nil/1 selectors for one physical singleton SkillUse with only checked
    /// singleton support companions. Other deferred usage remains excluded.
    InputDisposition,
    UsageParameter {
        policy: UsagePolicyDefId,
        parameter: DeclaredSlot<ParameterSlotDefId>,
    },
    Deferred {
        reason: OwnedDefinitionKey,
    },
}

pub(super) struct CompiledInventory<'p> {
    rows: &'p [UsageInventoryRow],
}

pub(super) fn compile<'p>(
    input: Option<&'p UsageInventoryPolicy>,
    compiled: &CompiledUsageInputs<'_>,
    limits: NormalizationLimits,
    work: &mut usize,
) -> Result<Option<CompiledInventory<'p>>> {
    let Some(input) = input else { return Ok(None) };
    if input.rows.is_empty() || input.rows.len() > limits.mapping.max_entries {
        return invalid("usage inventory rows");
    }
    for (index, row) in input.rows.iter().enumerate() {
        charge(work, index.saturating_add(1), limits)?;
        if input.rows[..index]
            .iter()
            .any(|other| other.source == row.source)
            || row.fields.is_empty()
            || row.fields.len() > 128
        {
            return invalid("usage inventory source or fields");
        }
        for (field_index, field) in row.fields.iter().enumerate() {
            charge(
                work,
                field_index.saturating_add(field.destinations.len().saturating_mul(64)),
                limits,
            )?;
            if row.fields[..field_index]
                .iter()
                .any(|other| other.field == field.field)
                || field.destinations.is_empty()
                || field.destinations.len() > 64
                || field
                    .destinations
                    .iter()
                    .enumerate()
                    .any(|(i, value)| field.destinations[..i].contains(value))
            {
                return invalid("usage inventory field destinations");
            }
            let allowed = match &field.field {
                UsageInventoryFieldSelector::OccurrenceAttribute { name } => {
                    skill_source_census::GEM_ATTRIBUTES.contains(&name.as_str())
                }
                UsageInventoryFieldSelector::GroupAttribute { name } => {
                    skill_source_census::GROUP_ATTRIBUTES.contains(&name.as_str())
                }
                UsageInventoryFieldSelector::ReferenceDescendants => true,
            };
            if !allowed {
                return invalid("usage inventory field");
            }
            for destination in &field.destinations {
                if let UsageInventoryDestination::UsageParameter { policy, parameter } = destination
                {
                    charge(
                        work,
                        compiled
                            .occurrences
                            .as_ref()
                            .map_or(0, |rows| rows.inventory_row_count())
                            .saturating_add(1),
                        limits,
                    )?;
                    let policies = match &row.source {
                        UsageInventorySource::Physical { gem } => {
                            compiled.rules.get(gem).map(|row| row.policies.as_slice())
                        }
                        UsageInventorySource::Occurrence { target } => compiled
                            .occurrences
                            .as_ref()
                            .and_then(|rows| rows.inventory_policies(target)),
                    };
                    if let Some(policies) = policies {
                        charge(work, policies.len(), limits)?;
                        for policy in policies {
                            charge(work, policy.parameters.len().saturating_mul(3), limits)?;
                        }
                    }
                    if !policies.is_some_and(|policies| {
                        policies.iter().any(|row| {
                            row.policy == policy
                                && row.parameters.iter().any(|p| {
                                    &p.slot == parameter
                                        && parameter_fields(p).contains(&field.field)
                                })
                        })
                    }) {
                        return invalid("usage inventory parameter authority");
                    }
                }
            }
        }
    }
    Ok(Some(CompiledInventory { rows: &input.rows }))
}

#[derive(Default)]
pub(in crate::owned_normalize) struct InventoryReceipts {
    inputs: Vec<InputReceipt>,
    usage: Vec<UsageReceipt>,
}
struct InputReceipt {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    preset: SkillPresetId,
    identity: UsageInventorySource,
    output: InputOutput,
    fields: BTreeSet<UsageInventoryFieldSelector>,
    descendants: BTreeSet<SourceOccurrenceId>,
}
enum InputOutput {
    Authored {
        skill: Box<SkillDraft>,
        gem: Option<Box<GemDraft>>,
        scope: AuthoredInputScope,
    },
    Support {
        support: Box<SupportDraft>,
        gem: Box<GemDraft>,
    },
    Generated(Box<generated_skill_inputs::CheckedInputDisposition>),
}
#[derive(Clone)]
enum AuthoredInputScope {
    ReferenceDisposition,
    PrimaryUsage {
        binding: Box<PresetUsageBindingDraft>,
    },
}

/// Existing producers retain different authority. A flat intrinsic proof never
/// borrows the complete reference correspondence of a disposition.
pub(in crate::owned_normalize) enum AuthoredInputProof<'a, 'p> {
    ReferenceDisposition(&'a skill_input_disposition::CheckedSourceReference),
    PrimaryUsage {
        proof: &'a gem_inventory::PendingPrimaryInventory,
        usage: &'a AttachedPrimaryUsage<'p>,
    },
}

struct UsageReceipt {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    // Minted by the adapter's checked frame, not inferred from a later link.
    group_contributes: bool,
    preset: SkillPresetId,
    identity: UsageInventorySource,
    binding: PresetUsageBindingDraft,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    fields: BTreeSet<UsageInventoryFieldSelector>,
}

fn occurrence_field(name: &str) -> UsageInventoryFieldSelector {
    UsageInventoryFieldSelector::OccurrenceAttribute { name: name.into() }
}
fn group_field(name: &str) -> UsageInventoryFieldSelector {
    UsageInventoryFieldSelector::GroupAttribute { name: name.into() }
}
fn recipe_field(recipe: &ValueRecipe, group: bool) -> UsageInventoryFieldSelector {
    let name = &recipe.input().tiers[0].selectors[0].name;
    if group {
        group_field(name)
    } else {
        occurrence_field(name)
    }
}
fn parameter_fields(parameter: &BoundParameter<'_>) -> BTreeSet<UsageInventoryFieldSelector> {
    match &parameter.source {
        BoundUsageValue::Occurrence(recipe) => [recipe_field(recipe, false)].into(),
        BoundUsageValue::ContainingGroup(recipe) => [recipe_field(recipe, true)].into(),
        BoundUsageValue::ContainingGroupOverride {
            group, occurrence, ..
        } => [recipe_field(group, true), recipe_field(occurrence, false)].into(),
    }
}

fn descendants(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
) -> Result<BTreeSet<SourceOccurrenceId>> {
    let mut pending = row.children().to_vec();
    let mut found = BTreeSet::new();
    b.charge(pending.len())?;
    while let Some(source) = pending.pop() {
        b.charge(1)?;
        if !found.insert(source) {
            return invalid("usage inventory repeated descendant");
        }
        let children = b.evidence.rows()[source.ordinal() as usize].children();
        b.charge(children.len())?;
        pending.extend_from_slice(children);
    }
    Ok(found)
}

/// Called only after the existing source proof has a real authored destination.
/// Its private scope preserves reference versus flat-usage authority. No token
/// is deserializable; scalar completeness alone cannot create one.
pub(in crate::owned_normalize) fn record_input_disposition(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    preset: &SkillPresetDraft,
    skill: &SkillDraft,
    gem: Option<&GemDraft>,
    proof: AuthoredInputProof<'_, '_>,
    receipts: &mut InventoryReceipts,
) -> Result<()> {
    let row = &b.evidence.rows()[source.ordinal() as usize];
    let Some(group_id) = row.occurrence().parent() else {
        return Ok(());
    };
    let group = &b.evidence.rows()[group_id.ordinal() as usize];
    let identity = match (&skill.source, gem) {
        (DraftAuthoredSkillSource::Gem(DraftField::Known { value }), Some(gem))
            if *value == gem.id =>
        {
            if gem.to_resolved().is_none() {
                return Ok(());
            }
            let Some(definition) = gem.definition.to_resolved() else {
                return Ok(());
            };
            UsageInventorySource::Physical { gem: definition }
        }
        (DraftAuthoredSkillSource::Direct(DraftField::Known { value }), None) => {
            let Some((game_id, variant_id)) =
                component(b, row, "gemId").zip(component(b, row, "variantId"))
            else {
                return Ok(());
            };
            let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id,
                variant_id,
            });
            let Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(catalog)),
                basis: MappingBasis::Exact,
            }) = b.roles.lookup(&selector)
            else {
                return Ok(());
            };
            let text = |name| source_shape::value(row, name).map(str::to_owned);
            let (Some(game_id), Some(variant_id), Some(skill_id), Some(name_spec)) = (
                text("gemId"),
                text("variantId"),
                text("skillId"),
                text("nameSpec"),
            ) else {
                return Ok(());
            };
            UsageInventorySource::Occurrence {
                target: Box::new(OccurrenceUsageTarget::AuthoredDirect {
                    gem: catalog.clone(),
                    game_id,
                    variant_id,
                    skill_id,
                    name_spec,
                    skill: Box::new(value.clone()),
                }),
            }
        }
        _ => return Ok(()),
    };
    let mut reference = None;
    let scope = match proof {
        AuthoredInputProof::ReferenceDisposition(proof) => {
            if proof.source() != source || proof.group() != Some(group_id) {
                return Ok(());
            }
            reference = Some(proof);
            AuthoredInputScope::ReferenceDisposition
        }
        AuthoredInputProof::PrimaryUsage { proof, usage } => {
            let UsageInventorySource::Physical { gem } = &identity else {
                return Ok(());
            };
            if proof.source() != source
                || proof.group() != group_id
                || proof.gem() != gem
                || !row.children().is_empty()
            {
                return Ok(());
            }
            // Policies are bounded by compilation; only the actual designated
            // binding can keep this source proof live.
            b.charge(usage.bindings.len().saturating_mul(3))?;
            let Some(binding) = proof.receipt_binding(usage, preset.id, skill.id) else {
                return Ok(());
            };
            b.charge(binding.selection.parameters.members.len().saturating_add(1))?;
            AuthoredInputScope::PrimaryUsage {
                binding: Box::new(binding.clone()),
            }
        }
    };
    b.charge(
        row.attributes()
            .len()
            .saturating_add(group.attributes().len()),
    )?;
    // This function is called only after the actual intrinsic/reference
    // disposition succeeds. Restrict the new source fact to the physical branch;
    // Direct and generated occurrences have different loader dispatch.
    let physical_secondary = if let UsageInventorySource::Physical { gem } = &identity {
        let roles = b.roles;
        let role = skill_input_disposition::charged_source_role(b, roles, row, gem)?;
        role.is_some_and(|role| {
            skill_input_disposition::physical_secondary_global_is_unconsumed(row, gem, role)
        })
    } else {
        false
    };
    let mut fields: BTreeSet<_> = row
        .attributes()
        .iter()
        .filter(|attribute| {
            let name = attribute.origin().name.as_str();
            let intrinsic = match &scope {
                AuthoredInputScope::ReferenceDisposition => true,
                AuthoredInputScope::PrimaryUsage { .. } => matches!(
                    name,
                    "gemId"
                        | "variantId"
                        | "skillId"
                        | "nameSpec"
                        | "level"
                        | "quality"
                        | "corrupted"
                        | "corruptLevel"
                        | "enabled"
                        | "enableGlobal1"
                        | "enableGlobal2"
                        | "count"
                ),
            };
            intrinsic
                && (!matches!(
                    attribute.origin().name.as_str(),
                    "count" | "enableGlobal1" | "enableGlobal2"
                ) || skill_input_disposition::exact_global_gate_is_neutral(
                    attribute,
                    "enableGlobal1",
                ) || skill_input_disposition::exact_global_gate_is_neutral(
                    attribute,
                    "enableGlobal2",
                ) || physical_secondary && attribute.origin().name == "enableGlobal2")
        })
        .map(|attribute| occurrence_field(&attribute.origin().name))
        .collect();
    fields.extend(
        group
            .attributes()
            .iter()
            .map(|a| a.origin().name.as_str())
            .filter(|name| {
                let intrinsic = match &scope {
                    AuthoredInputScope::ReferenceDisposition => true,
                    AuthoredInputScope::PrimaryUsage { .. } => {
                        matches!(*name, "enabled" | "source" | "label")
                    }
                };
                intrinsic
                    && !matches!(
                        *name,
                        "groupCount"
                            | "includeInFullDPS"
                            | "mainActiveSkill"
                            | "mainActiveSkillCalcs"
                    )
            })
            .map(group_field),
    );
    if skill.enabled.to_resolved().is_none() {
        fields.remove(&occurrence_field("enabled"));
        fields.remove(&group_field("enabled"));
    }
    if skill.scope.to_resolved().is_none() {
        fields.remove(&group_field("source"));
        fields.remove(&group_field("slot"));
    }
    let descendants = descendants(b, row)?;
    if let Some(reference) = reference {
        b.charge(reference.children().len().saturating_add(descendants.len()))?;
        if !reference
            .children()
            .iter()
            .copied()
            .eq(descendants.iter().copied())
        {
            return Ok(());
        }
        for name in reference.absent_fields() {
            // Bind each finite fact to the current immutable exact source;
            // charge lookup/string allocation before testing or inserting it.
            b.charge(
                row.attributes()
                    .len()
                    .saturating_add(1)
                    .saturating_mul(name.len().saturating_add(1)),
            )?;
            if row.attribute(name).is_some() {
                return Ok(());
            }
            fields.insert(occurrence_field(name));
        }
        if matches!(&identity, UsageInventorySource::Occurrence { target }
            if matches!(target.as_ref(), OccurrenceUsageTarget::AuthoredDirect { .. }))
        {
            // The successful exact manual Direct reference proof already
            // validated this source frame and its manual-source policy. Retain
            // only genuine absence, never source="" or a generated marker.
            // This records the authored origin, not a native scope default.
            b.charge(group.attributes().len().saturating_add(1).saturating_mul(7))?;
            if group.attribute("source").is_none() {
                fields.insert(group_field("source"));
            }
        }
        if matches!(identity, UsageInventorySource::Physical { .. }) {
            // Separate physical presentation fact: the successful full source
            // disposition admits this group grammar; an absent label chooses
            // source display fallback, never native identity or gameplay input.
            // Direct/generated and flat PrimaryUsage do not borrow this fact.
            b.charge(group.attributes().len().saturating_add(1).saturating_mul(6))?;
            if group.attribute("label").is_none() {
                fields.insert(group_field("label"));
            }
        }
    }
    fields.insert(UsageInventoryFieldSelector::ReferenceDescendants);
    b.charge(1)?;
    receipts.inputs.push(InputReceipt {
        source,
        group: group_id,
        preset: preset.id,
        identity,
        output: InputOutput::Authored {
            skill: Box::new(skill.clone()),
            gem: gem.cloned().map(Box::new),
            scope,
        },
        fields,
        descendants,
    });
    Ok(())
}

/// A resolved Gem alone has no support-input authority: retain the successful
/// source proof through construction of its exact SupportAssignment.
pub(in crate::owned_normalize) fn record_support_input_disposition(
    b: &mut Builder<'_, '_>,
    proof: gem_inventory::PhysicalSupportInputProof,
    preset: &SkillPresetDraft,
    support: &SupportDraft,
    gem: &GemDraft,
    receipts: &mut InventoryReceipts,
) -> Result<()> {
    b.charge(gem.parameters.members.len().saturating_add(1))?;
    if gem.to_resolved().is_none()
        || support.to_resolved().is_none()
        || support.support.to_resolved() != Some(gem.id)
        || gem.definition.to_resolved().as_ref() != Some(proof.gem())
    {
        return Ok(());
    }
    let row = &b.evidence.rows()[proof.source().ordinal() as usize];
    let group = &b.evidence.rows()[proof.group().ordinal() as usize];
    if row.occurrence().parent() != Some(proof.group())
        || row.children() != proof.reference_descendants()
    {
        return Ok(());
    }
    product(
        b,
        &[
            row.attributes()
                .len()
                .saturating_add(group.attributes().len()),
            proof
                .occurrence_attributes()
                .len()
                .saturating_add(proof.group_attributes().len())
                .saturating_add(1),
        ],
    )?;
    // A physical support's neutral saved usage fields have no admitted active
    // receiver only within the complete singleton source-construction domain.
    // Additional/missing effects may have their own count and indexed gates.
    // Keep this separate from SkillUse secondary-switch authority.
    let roles = b.roles;
    let role = skill_input_disposition::charged_source_role(b, roles, row, proof.gem())?;
    b.charge(row.attributes().len().saturating_mul(65).saturating_add(8))?;
    let neutral_support_usage = role.is_some_and(|role| {
        role.role == OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)
            && skill_input_disposition::physical_source_is_singleton(row, proof.gem(), role)
            && [
                ("count", "1"),
                ("enableGlobal1", "true"),
                ("enableGlobal2", "true"),
            ]
            .into_iter()
            .all(|(name, value)| {
                row.attribute(name).is_some_and(|attribute| {
                    attribute.raw() == value && attribute.decoded().ok() == Some(value)
                })
            })
    });
    let mut fields: BTreeSet<_> = row
        .attributes()
        .iter()
        .map(|attribute| attribute.origin().name.as_str())
        .filter(|name| {
            proof.occurrence_attributes().contains(name)
                || neutral_support_usage
                    && matches!(*name, "count" | "enableGlobal1" | "enableGlobal2")
        })
        .map(occurrence_field)
        .collect();
    fields.extend(
        group
            .attributes()
            .iter()
            .map(|attribute| attribute.origin().name.as_str())
            .filter(|name| proof.group_attributes().contains(name))
            .map(group_field),
    );
    fields.insert(UsageInventoryFieldSelector::ReferenceDescendants);
    receipts.inputs.push(InputReceipt {
        source: proof.source(),
        group: proof.group(),
        preset: preset.id,
        identity: UsageInventorySource::Physical {
            gem: proof.gem().clone(),
        },
        output: InputOutput::Support {
            support: Box::new(support.clone()),
            gem: Box::new(gem.clone()),
        },
        fields,
        descendants: proof.reference_descendants().iter().copied().collect(),
    });
    Ok(())
}

/// The generated-input adapter owns the proof. This layer consumes only the
/// fields it actually accounts for, never the rest of its admitted syntax.
pub(in crate::owned_normalize) fn record_generated_input_disposition(
    b: &mut Builder<'_, '_>,
    proof: generated_skill_inputs::CheckedInputDisposition,
    receipts: &mut InventoryReceipts,
) -> Result<()> {
    b.charge(
        proof
            .occurrence_attributes()
            .len()
            .saturating_add(proof.group_attributes().len())
            .saturating_add(proof.descendants().len())
            .saturating_add(1),
    )?;
    let mut fields: BTreeSet<_> = proof
        .occurrence_attributes()
        .iter()
        .map(|name| occurrence_field(name))
        .collect();
    fields.extend(
        proof
            .group_attributes()
            .iter()
            .map(|name| group_field(name)),
    );
    fields.insert(UsageInventoryFieldSelector::ReferenceDescendants);
    receipts.inputs.push(InputReceipt {
        source: proof.source(),
        group: proof.group(),
        preset: proof.preset(),
        identity: UsageInventorySource::Occurrence {
            target: Box::new(OccurrenceUsageTarget::Generated {
                correspondence: Box::new(proof.correspondence().clone()),
            }),
        },
        fields,
        descendants: proof.descendants().iter().copied().collect(),
        output: InputOutput::Generated(Box::new(proof)),
    });
    Ok(())
}

/// Projection may ignore a malformed losing lane. Completion cannot: independently
/// validate every present lane, without changing projection precedence or outputs.
#[allow(clippy::too_many_arguments)]
pub(super) fn record_usage(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    group: &SourceEvidenceRow<'_>,
    preset: SkillPresetId,
    identity: UsageInventorySource,
    group_contributes: bool,
    binding: &PresetUsageBindingDraft,
    parameters: &[BoundParameter<'_>],
    receipts: &mut InventoryReceipts,
) -> Result<()> {
    for parameter in parameters {
        b.charge(1)?;
        let lanes: Vec<_> = match &parameter.source {
            BoundUsageValue::Occurrence(recipe) => vec![(row, recipe)],
            BoundUsageValue::ContainingGroup(recipe) => vec![(group, recipe)],
            BoundUsageValue::ContainingGroupOverride {
                group: recipe,
                occurrence,
                ..
            } => vec![(group, recipe.as_ref()), (row, occurrence)],
        };
        let mut valid = true;
        for (source, recipe) in lanes {
            let name = &recipe.input().tiers[0].selectors[0].name;
            b.charge(source.attributes().len())?;
            if let Some(value) = source.attribute(name) {
                b.charge(value.raw().len())?;
                match b.scalar_value(source, recipe) {
                    Ok(ScalarValue::Selected(value))
                        if gem_inputs::value_valid(&value, &parameter.schema) => {}
                    Ok(_)
                    | Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {
                        valid = false
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        if valid {
            receipts.usage.push(UsageReceipt {
                source: row.occurrence().id(),
                group: group.occurrence().id(),
                group_contributes,
                preset,
                identity: identity.clone(),
                binding: binding.clone(),
                parameter: parameter.slot.clone(),
                fields: parameter_fields(parameter),
            });
        }
    }
    Ok(())
}

fn usage_present(preset: &SkillPresetDraft, binding: &PresetUsageBindingDraft) -> bool {
    if let Some(intent) = &preset.intent {
        intent
            .usage
            .members
            .iter()
            .filter(|row| *row == binding)
            .count()
            == 1
    } else {
        binding.applicability == PresetApplicability::Required
            && preset.usage_preferences.as_ref().is_some_and(|rows| {
                rows.members
                    .iter()
                    .filter(|row| **row == binding.selection)
                    .count()
                    == 1
            })
    }
}

fn sole_link(
    b: &Builder<'_, '_>,
    source: SourceOccurrenceId,
    expected: &OwnedOriginTarget,
    same_kind: impl Fn(&OwnedOriginTarget) -> bool,
) -> bool {
    let links = &b.origins[source.ordinal() as usize].links;
    links.contains(expected) && links.iter().filter(|link| same_kind(link)).count() == 1
}

fn preset_links(
    b: &Builder<'_, '_>,
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    preset: SkillPresetId,
) -> bool {
    [source, group].into_iter().all(|origin| {
        sole_link(b, origin, &OwnedOriginTarget::SkillPreset(preset), |link| {
            matches!(link, OwnedOriginTarget::SkillPreset(_))
        })
    })
}

/// Both producers retain exact occurrence/set ownership. A group frame is a
/// required origin only when that adapter actually checked it as contributing.
/// Callers precharge source/group links; this helper charges the parent lookup
/// and additional exact set-origin scans.
fn usage_origins_present(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    group_id: SourceOccurrenceId,
    preset: SkillPresetId,
    group_contributes: bool,
) -> Result<bool> {
    b.charge(3)?;
    let row = &b.evidence.rows()[source.ordinal() as usize];
    let group = &b.evidence.rows()[group_id.ordinal() as usize];
    if row.occurrence().parent() != Some(group_id)
        || row.occurrence().name() != "Gem"
        || group.occurrence().name() != "Skill"
    {
        return Ok(false);
    }
    let Some(set) = group.occurrence().parent() else {
        return Ok(false);
    };
    if b.evidence.rows()[set.ordinal() as usize]
        .occurrence()
        .name()
        != "SkillSet"
    {
        return Ok(false);
    }
    b.charge(
        b.origins[set.ordinal() as usize]
            .links
            .len()
            .saturating_mul(2),
    )?;
    Ok(
        sole_link(b, source, &OwnedOriginTarget::SkillPreset(preset), |link| {
            matches!(link, OwnedOriginTarget::SkillPreset(_))
        }) && sole_link(b, set, &OwnedOriginTarget::SkillPreset(preset), |link| {
            matches!(link, OwnedOriginTarget::SkillPreset(_))
        }) && if group_contributes {
            sole_link(
                b,
                group_id,
                &OwnedOriginTarget::SkillPreset(preset),
                |link| matches!(link, OwnedOriginTarget::SkillPreset(_)),
            )
        } else {
            b.origins[group_id.ordinal() as usize]
                .links
                .iter()
                .all(|link| !matches!(link, OwnedOriginTarget::SkillPreset(id) if *id != preset))
        },
    )
}

/// Flat primary attachment need not have contributed any group value; its
/// independent usage receipt retains any required contributing-group origin.
fn primary_usage_present(
    b: &mut Builder<'_, '_>,
    preset: &SkillPresetDraft,
    receipt: &InputReceipt,
    binding: &PresetUsageBindingDraft,
) -> Result<bool> {
    let row = &b.evidence.rows()[receipt.source.ordinal() as usize];
    if !row.children().is_empty()
        || !receipt.descendants.is_empty()
        || !usage_origins_present(b, receipt.source, receipt.group, preset.id, false)?
    {
        return Ok(false);
    }
    let usages = preset.intent.as_ref().map_or_else(
        || {
            preset
                .usage_preferences
                .as_ref()
                .map_or(0, |rows| rows.members.len())
        },
        |intent| intent.usage.members.len(),
    );
    product(
        b,
        &[
            usages,
            binding
                .selection
                .parameters
                .members
                .len()
                .saturating_add(65),
        ],
    )?;
    Ok(usage_present(preset, binding))
}

fn input_present(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    preset: &SkillPresetDraft,
    receipt: &InputReceipt,
) -> Result<bool> {
    Ok(match &receipt.output {
        InputOutput::Authored { skill, gem, scope } => {
            let origin_present = match scope {
                AuthoredInputScope::ReferenceDisposition => {
                    preset_links(b, receipt.source, receipt.group, receipt.preset)
                }
                AuthoredInputScope::PrimaryUsage { binding } => {
                    primary_usage_present(b, preset, receipt, binding)?
                }
            };
            draft
                .skills
                .members
                .iter()
                .filter(|row| *row == skill.as_ref())
                .count()
                == 1
                && preset
                    .skills
                    .members
                    .iter()
                    .filter(|id| **id == skill.id)
                    .count()
                    == 1
                && origin_present
                && sole_link(
                    b,
                    receipt.source,
                    &OwnedOriginTarget::Skill(skill.id),
                    |link| matches!(link, OwnedOriginTarget::Skill(_)),
                )
                && gem.as_deref().is_none_or(|gem| {
                    sole_link(b, receipt.source, &OwnedOriginTarget::Gem(gem.id), |link| {
                        matches!(link, OwnedOriginTarget::Gem(_))
                    }) && draft.gems.members.iter().filter(|row| *row == gem).count() == 1
                })
                && receipt.descendants.iter().all(|source| {
                    sole_link(b, *source, &OwnedOriginTarget::Skill(skill.id), |link| {
                        matches!(link, OwnedOriginTarget::Skill(_))
                    })
                })
        }
        InputOutput::Support { support, gem } => {
            let DraftSkillTarget::Authored(DraftField::Known { value: target }) = &support.target
            else {
                return Ok(false);
            };
            b.charge(
                draft
                    .skills
                    .members
                    .len()
                    .saturating_add(preset.skills.members.len()),
            )?;
            if draft
                .skills
                .members
                .iter()
                .filter(|skill| skill.id == *target)
                .count()
                != 1
                || preset
                    .skills
                    .members
                    .iter()
                    .filter(|id| **id == *target)
                    .count()
                    != 1
            {
                return Ok(false);
            }
            let Some(set) = b.evidence.rows()[receipt.group.ordinal() as usize]
                .occurrence()
                .parent()
            else {
                return Ok(false);
            };
            b.charge(
                b.origins[set.ordinal() as usize]
                    .links
                    .len()
                    .saturating_mul(2),
            )?;
            sole_link(
                b,
                set,
                &OwnedOriginTarget::SkillPreset(preset.id),
                |link| matches!(link, OwnedOriginTarget::SkillPreset(_)),
            )
                // Supports already have exact Gem/Support attribution and
                // containing-set membership; no redundant preset link is
                // manufactured to make the receipt pass.
                && [receipt.source, receipt.group].into_iter().all(|source| {
                    b.origins[source.ordinal() as usize].links.iter().all(|link| {
                        !matches!(link, OwnedOriginTarget::SkillPreset(id) if *id != preset.id)
                    })
                })
                && sole_link(b, receipt.source, &OwnedOriginTarget::Gem(gem.id), |link| {
                    matches!(link, OwnedOriginTarget::Gem(_))
                })
                && sole_link(
                    b,
                    receipt.source,
                    &OwnedOriginTarget::Support(support.id),
                    |link| matches!(link, OwnedOriginTarget::Support(_)),
                )
                && draft.gems.members.iter().filter(|row| *row == gem.as_ref()).count() == 1
                && draft.supports.members.iter().filter(|row| *row == support.as_ref()).count() == 1
                && preset.supports.members.iter().filter(|id| **id == support.id).count() == 1
        }
        InputOutput::Generated(proof) => proof.matches_current(b, draft, preset)?,
    })
}

/// Charge the existing live-output and origin scans only after the cheap exact
/// source/group/preset match. Unrelated saved presets cannot witness this row.
fn matching_input_present(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    preset: &SkillPresetDraft,
    receipt: &InputReceipt,
) -> Result<bool> {
    b.charge(
        b.origins[receipt.source.ordinal() as usize]
            .links
            .len()
            .saturating_mul(6),
    )?;
    b.charge(
        b.origins[receipt.group.ordinal() as usize]
            .links
            .len()
            .saturating_mul(2),
    )?;
    for descendant in &receipt.descendants {
        b.charge(
            b.origins[descendant.ordinal() as usize]
                .links
                .len()
                .saturating_mul(2),
        )?;
    }
    b.charge(
        draft
            .skills
            .members
            .len()
            .saturating_add(draft.gems.members.len())
            .saturating_add(draft.supports.members.len())
            .saturating_add(preset.skills.members.len())
            .saturating_add(preset.supports.members.len())
            .saturating_add(1),
    )?;
    input_present(b, draft, preset, receipt)
}

fn product(b: &mut Builder<'_, '_>, factors: &[usize]) -> Result<()> {
    let cost = factors
        .iter()
        .try_fold(1usize, |value, factor| value.checked_mul(*factor))
        .ok_or(NormalizationError::Limit("usage inventory work"))?;
    b.charge(cost)
}

fn field_proved(
    field: &UsageInventoryField,
    inputs: &[&InputReceipt],
    usages: &[&UsageReceipt],
    selectors: Option<&group_selectors::GroupSelectors>,
) -> bool {
    field
        .destinations
        .iter()
        .all(|destination| match destination {
            UsageInventoryDestination::Deferred { .. } => false,
            UsageInventoryDestination::InputDisposition => inputs.iter().any(|receipt| {
                receipt.fields.contains(&field.field)
                    || selectors.is_some_and(|proof| proof.proves(receipt, &field.field))
            }),
            UsageInventoryDestination::UsageParameter { policy, parameter } => {
                usages.iter().any(|receipt| {
                    receipt.binding.selection.policy.to_resolved().as_ref() == Some(policy)
                        && &receipt.parameter == parameter
                        && receipt.fields.contains(&field.field)
                })
            }
        })
}

fn completion(preset: &SkillPresetDraft) -> Option<&DraftListCompletion> {
    preset
        .intent
        .as_ref()
        .map(|value| &value.usage.completion)
        .or_else(|| {
            preset
                .usage_preferences
                .as_ref()
                .map(|value| &value.completion)
        })
}

pub(super) fn complete(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    presets: &BTreeMap<SourceOccurrenceId, usize>,
    compiled: Option<&CompiledInventory<'_>>,
    receipts: &InventoryReceipts,
) -> Result<()> {
    let Some(compiled) = compiled else {
        return Ok(());
    };
    let Some(sets) = skill_source_census::sets(b)? else {
        return Ok(());
    };
    // Prepare every retirement before making the first mutation. A late work
    // limit failure cannot leave an earlier preset or sidecar partly retired.
    let mut retirements = Vec::new();
    // A missing required consumer already prevents this preset's retirement.
    // Stop its proof work, while retaining the whole source census above and
    // independent preset checks. The caller still validates draft/origin integrity.
    'presets: for set_id in sets {
        let Some(preset_id) = skill_input_disposition::unique_link(b, set_id, |link| {
            if let OwnedOriginTarget::SkillPreset(id) = link {
                Some(*id)
            } else {
                None
            }
        })?
        else {
            continue;
        };
        b.charge(draft.skill_presets.members.len())?;
        let Some(index) = draft
            .skill_presets
            .members
            .iter()
            .position(|row| row.id == preset_id)
        else {
            continue;
        };
        if presets.get(&set_id) != Some(&index) {
            continue;
        }
        let preset = &draft.skill_presets.members[index];
        let Some(DraftListCompletion::Pending { id: issue, code }) = completion(preset) else {
            continue;
        };
        if code.as_str() != "usage-preferences-not-converted" {
            continue;
        }
        let issue = *issue;
        let set = &b.evidence.rows()[set_id.ordinal() as usize];
        let mut accounted: BTreeSet<_> = [set_id].into();
        let mut accounted_parameters: Vec<&UsageReceipt> = Vec::new();
        if set.children().is_empty() {
            continue 'presets;
        }
        b.charge(set.children().len())?;
        for group_id in set.children() {
            let group = &b.evidence.rows()[group_id.ordinal() as usize];
            if group.children().is_empty() {
                continue 'presets;
            }
            accounted.insert(*group_id);
            let selectors = group_selectors::prove(b, draft, preset, group, receipts)?;
            let mut group_fields = BTreeSet::new();
            b.charge(group.children().len())?;
            for source in group.children() {
                let row = &b.evidence.rows()[source.ordinal() as usize];
                let binding_count = preset.intent.as_ref().map_or_else(
                    || {
                        preset
                            .usage_preferences
                            .as_ref()
                            .map_or(0, |rows| rows.members.len())
                    },
                    |intent| intent.usage.members.len(),
                );
                // Identity filtering still visits every receipt. Deep scans
                // are relevant only to receipts for this exact occurrence.
                b.charge(receipts.inputs.len())?;
                let mut inputs = Vec::new();
                for receipt in &receipts.inputs {
                    if receipt.source == *source
                        && receipt.group == *group_id
                        && receipt.preset == preset_id
                        && matching_input_present(b, draft, preset, receipt)?
                    {
                        inputs.push(receipt);
                    }
                }
                b.charge(receipts.usage.len())?;
                let mut usages = Vec::new();
                for receipt in &receipts.usage {
                    if receipt.source != *source
                        || receipt.group != *group_id
                        || receipt.preset != preset_id
                    {
                        continue;
                    }
                    for origin in [receipt.source, receipt.group] {
                        b.charge(
                            b.origins[origin.ordinal() as usize]
                                .links
                                .len()
                                .saturating_mul(2),
                        )?;
                    }
                    // A compiled usage policy has at most 64 parameters.
                    // Preserve a bound for the complete binding comparison,
                    // not just the number of rows in the receiving preset.
                    product(b, &[binding_count.saturating_add(1), 65])?;
                    if usage_origins_present(
                        b,
                        receipt.source,
                        receipt.group,
                        receipt.preset,
                        receipt.group_contributes,
                    )? && usage_present(preset, &receipt.binding)
                    {
                        usages.push(receipt);
                    }
                }
                product(
                    b,
                    &[
                        compiled.rows.len(),
                        inputs.len().saturating_add(usages.len()).saturating_add(1),
                    ],
                )?;
                let matching: Vec<_> = compiled
                    .rows
                    .iter()
                    .filter(|candidate| {
                        inputs.iter().any(|r| r.identity == candidate.source)
                            || usages.iter().any(|r| r.identity == candidate.source)
                    })
                    .collect();
                if matching.len() != 1 {
                    continue 'presets;
                }
                let policy = matching[0];
                b.charge(inputs.len().saturating_add(usages.len()))?;
                let inputs: Vec<_> = inputs
                    .into_iter()
                    .filter(|r| r.identity == policy.source)
                    .collect();
                let usages: Vec<_> = usages
                    .into_iter()
                    .filter(|r| r.identity == policy.source)
                    .collect();
                let mut actual: BTreeSet<_> = row
                    .attributes()
                    .iter()
                    .map(|a| occurrence_field(&a.origin().name))
                    .collect();
                actual.insert(UsageInventoryFieldSelector::ReferenceDescendants);
                product(b, &[actual.len(), policy.fields.len().saturating_add(1)])?;
                if !actual
                    .iter()
                    .all(|field| policy.fields.iter().any(|row| &row.field == field))
                {
                    continue 'presets;
                }
                // Check declared fields as well: an absent field is not a default
                // unless its real decoder/disposition supplied the receipt.
                for field in &policy.fields {
                    product(
                        b,
                        &[
                            field.destinations.len(),
                            inputs.len().saturating_add(usages.len()).saturating_add(1),
                        ],
                    )?;
                    if !field_proved(field, &inputs, &usages, selectors.as_ref()) {
                        continue 'presets;
                    }
                    if matches!(
                        field.field,
                        UsageInventoryFieldSelector::GroupAttribute { .. }
                    ) {
                        group_fields.insert(field.field.clone());
                    }
                }
                // Extra emitted parameters are also consumers. The authored
                // inventory cannot omit an inconvenient successful projection.
                for receipt in &usages {
                    product(b, &[receipt.fields.len(), policy.fields.len(), 65])?;
                    let declared = receipt.fields.iter().all(|source_field| {
                        policy.fields.iter().any(|field| {
                            source_field == &field.field && field.destinations.iter().any(|destination| {
                                matches!(destination, UsageInventoryDestination::UsageParameter { policy, parameter }
                                    if receipt.binding.selection.policy.to_resolved().as_ref() == Some(policy)
                                        && &receipt.parameter == parameter)
                            })
                        })
                    });
                    if !declared {
                        continue 'presets;
                    }
                    accounted_parameters.push(receipt);
                }
                let descendants = descendants(b, row)?;
                product(b, &[inputs.len(), descendants.len().saturating_add(1)])?;
                if !inputs
                    .iter()
                    .any(|receipt| receipt.descendants == descendants)
                {
                    continue 'presets;
                }
                accounted.extend(descendants);
                accounted.insert(*source);
            }
            b.charge(group.attributes().len())?;
            if !group
                .attributes()
                .iter()
                .all(|attribute| group_fields.contains(&group_field(&attribute.origin().name)))
            {
                continue 'presets;
            }
        }
        let binding_count = preset.intent.as_ref().map_or_else(
            || {
                preset
                    .usage_preferences
                    .as_ref()
                    .map_or(0, |rows| rows.members.len())
            },
            |intent| intent.usage.members.len(),
        );
        product(b, &[binding_count, 65])?;
        let bindings: Vec<_> = if let Some(intent) = &preset.intent {
            intent.usage.members.clone()
        } else {
            preset
                .usage_preferences
                .as_ref()
                .expect("checked usage")
                .members
                .iter()
                .cloned()
                .map(|selection| PresetUsageBindingDraft {
                    selection,
                    applicability: PresetApplicability::Required,
                })
                .collect()
        };
        product(
            b,
            &[
                bindings.len(),
                65,
                accounted_parameters.len().saturating_add(1),
            ],
        )?;
        if !bindings.iter().all(|binding| {
            matches!(
                binding.selection.parameters.completion,
                DraftListCompletion::Complete
            ) && binding
                .selection
                .parameters
                .members
                .iter()
                .all(|parameter| {
                    accounted_parameters.iter().any(|receipt| {
                        &receipt.binding == binding
                            && parameter.slot.to_resolved().as_ref() == Some(&receipt.parameter)
                    })
                })
        }) {
            continue 'presets;
        }
        let mut linked = Vec::new();
        b.charge(b.origins.len())?;
        for origin_index in 0..b.origins.len() {
            b.charge(b.origins[origin_index].links.len())?;
            let origin = &b.origins[origin_index];
            if origin.links.contains(&OwnedOriginTarget::Issue(issue)) {
                if !accounted.contains(&origin.source) || origin.links.len() < 2 {
                    continue 'presets;
                }
                linked.push(origin_index);
            }
        }
        if linked.is_empty() {
            continue 'presets;
        }
        b.charge(linked.len().saturating_add(1))?;
        retirements.push((index, issue, linked));
    }
    for (index, issue, linked) in retirements {
        for origin in linked {
            b.origins[origin]
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(issue));
        }
        let preset = &mut draft.skill_presets.members[index];
        if let Some(intent) = &mut preset.intent {
            intent.usage.completion = DraftListCompletion::Complete;
        } else {
            preset
                .usage_preferences
                .as_mut()
                .expect("checked usage")
                .completion = DraftListCompletion::Complete;
        }
    }
    Ok(())
}
