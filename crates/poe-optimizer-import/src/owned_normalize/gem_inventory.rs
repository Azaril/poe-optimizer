//! Finite saved physical-Gem input inventories. This proves only the actual
//! assignment list; partial definition, effect and calculation coverage survives.
use super::*;
mod dispositions;
pub(super) use dispositions::PendingDisposition;
#[cfg(test)]
pub(super) use dispositions::tests::with_usage_inventory_destination;
pub use dispositions::{DeferredGemUsageField, DeferredGemUsageInput, PrimaryGemInputDisposition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GemInventoryPolicy {
    PobFreshPhysicalV3 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        usage_inputs: OwnedContentDigest,
        supports: Vec<PhysicalGemInputInventory>,
        primary_skills: Vec<PrimarySkillGemInventory>,
        primary_dispositions: Vec<PrimaryGemInputDisposition>,
    },
}

/// Physical identity and intrinsic slots, reviewed against the authenticated
/// catalog. The selected domain supplies its effect/reference disposition proof;
/// primary dispositions may retain reviewed alternative actions. The role
/// package binds the exact catalog/source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalGemInputInventory {
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub corrupted: DeclaredSlot<ParameterSlotDefId>,
    pub corruption_level: DeclaredSlot<ParameterSlotDefId>,
}

/// Intrinsic physical inputs are complete only after the exact primary usage
/// preference has been attached to its real, still-Pending containing preset.
/// Count/reporting remain outside this physical parameter inventory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimarySkillGemInventory {
    pub physical: PhysicalGemInputInventory,
    pub usage_policy: UsagePolicyDefId,
}

#[derive(Serialize)]
struct ScalarInputs<'a> {
    gem_inputs: &'a Option<GemInputPolicy>,
    gem_level: &'a ValueRecipeInput,
    gem_quality: &'a GemQualityPolicy,
    gem_enabled: &'a ValueRecipeInput,
    group_enabled: &'a ValueRecipeInput,
    manual_skill_sources: &'a [SourceComponent],
    generated_support_prefixes: &'a [String],
}

/// Exact acyclic commitment to the existing converters and physical admission.
/// No new inventory policy field participates in its own commitment.
pub fn gem_inventory_scalar_inputs_identity(
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-gem-inventory-scalar-inputs-v1",
        &ScalarInputs {
            gem_inputs: &policy.gem_inputs,
            gem_level: &policy.gem_level,
            gem_quality: &policy.gem_quality,
            gem_enabled: &policy.gem_enabled,
            group_enabled: &policy.group_enabled,
            manual_skill_sources: &policy.manual_skill_sources,
            generated_support_prefixes: &policy.generated_support_prefixes,
        },
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?)
}

/// Only checked, inherited policies may be rebound. The full successor is
/// validated again; explicitly supplied policies never call this helper.
pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    definitions: &DataIdentity,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<()> {
    if policy.gem_inventory.is_none() {
        return Ok(());
    }
    let scalar = gem_inventory_scalar_inputs_identity(policy, limits)?;
    let usage = usage_inputs_identity(policy, limits)?;
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        definitions: bound_definitions,
        roles: bound_roles,
        scalar_inputs,
        usage_inputs,
        primary_dispositions,
        ..
    } = policy.gem_inventory.as_mut().unwrap();
    *usage_inputs = usage;
    for row in primary_dispositions {
        crate::owned_source_actions::rebind_input(&mut row.reference_action, definitions, roles);
    }
    *bound_definitions = definitions.clone();
    *bound_roles = *roles.identity();
    *scalar_inputs = scalar;
    Ok(())
}

pub(super) struct CompiledGemInventory<'p> {
    gems: BTreeMap<&'p GemDefId, BoundGem<'p>>,
    level: ValueRecipe,
    enabled: ValueRecipe,
    group_enabled: ValueRecipe,
    manual_sources: &'p [SourceComponent],
    generated_prefixes: &'p [String],
    pub work: usize,
}
struct BoundGem<'p> {
    input: &'p PhysicalGemInputInventory,
    level: IntegerRange,
    corruption_level: QuantityRange,
    quality: QualityDefId,
    quality_amount: QuantityRange,
    usage: Option<BoundPrimaryUsage<'p>>,
    disposition: Option<dispositions::CompiledDisposition<'p>>,
}

impl CompiledGemInventory<'_> {
    /// Borrow only the immutable selector/deferred-field proof. Generated
    /// accounting never receives physical parameter-assignment authority.
    pub(super) fn generated_proof(
        &self,
        row: &generated_skill_inputs::GeneratedSkillInputRule,
    ) -> Option<(
        &crate::owned_source_actions::SourceActionCorrespondence,
        &skill_input_disposition::CompiledDeferredUsage,
    )> {
        self.gems
            .get(&row.gem)?
            .disposition
            .as_ref()?
            .generated_proof(row)
    }
}

/// The independently reviewed physical inventory selects one usage policy.
/// Other projections on the same primary never inherit this authority.
#[derive(Clone, Copy)]
struct BoundPrimaryUsage<'p> {
    row: &'p PrimarySkillUsageInput,
    policy: &'p UsagePolicyDefId,
}

pub(super) struct GemInventoryContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub group: &'a SourceEvidenceRow<'s>,
    pub gem: &'a DraftField<GemDefId>,
    pub quality: &'a DraftQuality,
    pub parameters: &'a [ParameterDraft],
}
pub(super) enum GemInventoryProof {
    PhysicalSupport(PhysicalSupportInputProof),
    AwaitUsage(PendingPrimaryInventory),
    AwaitDisposition(PendingDisposition),
}

/// Attempt-local evidence from the explicit physical SupportAssignment proof.
/// This is not a usage-completeness certificate. The caller must still bind the
/// actual Gem/Support outputs, preset membership and current source links.
pub(super) struct PhysicalSupportInputProof {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    gem: GemDefId,
}

impl PhysicalSupportInputProof {
    pub(super) fn source(&self) -> SourceOccurrenceId {
        self.source
    }

    pub(super) fn group(&self) -> SourceOccurrenceId {
        self.group
    }

    pub(super) fn gem(&self) -> &GemDefId {
        &self.gem
    }

    pub(super) fn occurrence_attributes(&self) -> &'static [&'static str] {
        &[
            "gemId",
            "variantId",
            "skillId",
            "nameSpec",
            "level",
            "quality",
            "corrupted",
            "corruptLevel",
            "enabled",
            "statSetIndex",
            "statSetIndexCalcs",
        ]
    }

    pub(super) fn group_attributes(&self) -> &'static [&'static str] {
        &["enabled", "source", "label"]
    }

    /// This support proof admits only flat source rows and absent/nil reference
    /// selectors. It does not inherit the primary skill's reference accounting.
    pub(super) fn reference_descendants(&self) -> &[SourceOccurrenceId] {
        &[]
    }
}

pub(super) struct PendingPrimaryInventory {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    gem: GemDefId,
    policy: UsagePolicyDefId,
}
impl PendingPrimaryInventory {
    pub(super) fn completed_by(
        &self,
        usage: Option<&usage_inputs::AttachedPrimaryUsage<'_>>,
    ) -> bool {
        usage.is_some_and(|usage| usage.proves(self.source, &self.gem, &self.policy))
    }

    pub(super) fn source(&self) -> SourceOccurrenceId {
        self.source
    }
    pub(super) fn group(&self) -> SourceOccurrenceId {
        self.group
    }
    pub(super) fn gem(&self) -> &GemDefId {
        &self.gem
    }

    pub(super) fn receipt_binding<'a>(
        &self,
        usage: &'a usage_inputs::AttachedPrimaryUsage<'_>,
        preset: SkillPresetId,
        skill: SkillUseId,
    ) -> Option<&'a PresetUsageBindingDraft> {
        self.completed_by(Some(usage))
            .then(|| usage.binding(self.source, self.group, preset, skill, &self.policy))
            .flatten()
    }
}

fn invalid<T>(reason: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(reason))
}
fn charge(work: &mut usize, n: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(n)
        .filter(|v| *v <= limits.max_work)
        .ok_or(NormalizationError::Limit("gem inventory work"))?;
    Ok(())
}
fn direct(recipe: &ValueRecipeInput, attribute: &str) -> bool {
    recipe.tiers.len() == 1
        && recipe.tiers[0].selectors.len() == 1
        && recipe.tiers[0].duplicates == DuplicatePolicy::Reject
        && recipe.tiers[0].selectors[0].lane == ValueLane::Attribute
        && recipe.tiers[0].selectors[0].name == attribute
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledGemInventory<'p>>> {
    let Some(inventory) = &policy.gem_inventory else {
        return Ok(None);
    };
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        definitions: identity,
        roles: role_identity,
        catalog,
        scalar_inputs,
        usage_inputs: usage_binding,
        supports,
        primary_skills,
        primary_dispositions,
    } = inventory;
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || roles.input().definitions != *definitions.identity()
        || catalog != &roles.input().compilation.catalog_digest
        || scalar_inputs != &gem_inventory_scalar_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    if (!primary_skills.is_empty() && policy.usage_inputs.is_none())
        || usage_binding != &usage_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    digest_owned(
        "owned-gem-inventory-policy-v1",
        &policy.gem_inventory,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?;
    if supports
        .len()
        .saturating_add(primary_skills.len())
        .saturating_add(primary_dispositions.len())
        > 4096
    {
        return Err(NormalizationError::Limit("gem inventory rows"));
    }
    let mut compiled = CompiledGemInventory {
        gems: BTreeMap::new(),
        level: ValueRecipe::new(policy.gem_level.clone(), limits.value)?,
        enabled: ValueRecipe::new(policy.gem_enabled.clone(), limits.value)?,
        group_enabled: ValueRecipe::new(policy.group_enabled.clone(), limits.value)?,
        manual_sources: &policy.manual_skill_sources,
        generated_prefixes: &policy.generated_support_prefixes,
        work: 0,
    };
    // An empty domain is inert, but all commitments above are still mandatory.
    if supports.is_empty() && primary_skills.is_empty() && primary_dispositions.is_empty() {
        return Ok(Some(compiled));
    }
    let Some(inputs) = &policy.gem_inputs else {
        return invalid("gem inventory requires scalar inputs");
    };
    let GemQualityPolicy::Attributes(quality) = &policy.gem_quality else {
        return invalid("gem inventory requires explicit quality");
    };
    if !direct(&policy.gem_level, "level")
        || !matches!(policy.gem_level.missing, MissingValuePolicy::Pending)
        || !matches!(policy.gem_level.codec.codec, ValueCodecKind::Integer { .. })
        || !direct(&quality.amount, "quality")
        || !matches!(quality.amount.missing, MissingValuePolicy::Pending)
        || !direct(&policy.gem_enabled, "enabled")
        || !direct(&policy.group_enabled, "enabled")
    {
        return invalid("gem inventory intrinsic source recipes");
    }
    charge(&mut compiled.work, inputs.gems.len(), limits)?;
    let input_rows: BTreeMap<_, _> = inputs.gems.iter().map(|r| (&r.gem, r)).collect();
    if input_rows.len() != inputs.gems.len() || inputs.definitions != *identity {
        return invalid("gem inventory scalar owner binding");
    }
    charge(&mut compiled.work, quality.kinds.len(), limits)?;
    let quality_kinds: Vec<_> = quality
        .kinds
        .iter()
        .filter(|r| r.source == SourceComponent::Missing)
        .collect();
    if quality_kinds.len() != 1 || quality.definitions != *identity {
        return invalid("gem inventory quality convention");
    }
    let kind = &quality_kinds[0].kind;
    let SchemaLookup::Known(kind_schema) = definitions.definition(kind) else {
        return invalid("gem inventory quality schema");
    };
    let mut usage_rows = BTreeMap::new();
    if !primary_skills.is_empty() {
        let Some(usage) = &policy.usage_inputs else {
            return invalid("primary inventory requires usage inputs");
        };
        let gems = usage.physical_rows();
        if gems.len() > 4096 {
            return Err(NormalizationError::Limit("primary inventory usage rows"));
        }
        charge(&mut compiled.work, gems.len(), limits)?;
        for row in gems {
            if usage_rows.insert(&row.gem, row).is_some() {
                return invalid("duplicate primary inventory usage row");
            }
        }
    }
    let mut selectors = BTreeSet::new();
    for (input, primary_inventory, disposition) in supports
        .iter()
        .map(|row| (row, None, None))
        .chain(
            primary_skills
                .iter()
                .map(|row| (&row.physical, Some(row), None)),
        )
        .chain(
            primary_dispositions
                .iter()
                .map(|row| (&row.physical, None, Some(row))),
        )
    {
        let usage = if let Some(primary_inventory) = primary_inventory {
            let Some(usage) = usage_rows.get(&input.gem).copied() else {
                return invalid("primary inventory usage absent");
            };
            if usage.attributes.len() > 64 || usage.guards.len() > 64 || usage.policies.len() > 64 {
                return invalid("primary inventory usage disposition limits");
            }
            charge(&mut compiled.work, usage.policies.len(), limits)?;
            let selected: Vec<_> = usage
                .policies
                .iter()
                .filter(|row| row.policy == primary_inventory.usage_policy)
                .collect();
            let [selected] = selected.as_slice() else {
                return invalid("primary inventory designated usage policy");
            };
            let [parameter] = selected.parameters.as_slice() else {
                return invalid("primary inventory designated usage parameters");
            };
            let UsageValueSource::Occurrence { value } = &parameter.source else {
                return invalid("primary inventory designated usage source");
            };
            charge(
                &mut compiled.work,
                usage
                    .attributes
                    .iter()
                    .fold(usage.guards.len().saturating_add(12), |n, name| {
                        n.saturating_add(name.len())
                    })
                    .saturating_add(
                        usage
                            .guards
                            .iter()
                            .fold(0usize, |n, guard| n.saturating_add(guard.attribute.len())),
                    )
                    .saturating_add(usage.game_id.len())
                    .saturating_add(usage.variant_id.len())
                    .saturating_add(usage.skill_id.len())
                    .saturating_add(usage.name_spec.len()),
                limits,
            )?;
            if usage.game_id != input.game_id
                || usage.variant_id != input.variant_id
                || usage.skill_id != input.skill_id
                || usage.name_spec != input.name_spec
                || usage
                    .attributes
                    .iter()
                    .any(|name| !GEM_ATTRIBUTES.contains(&name.as_str()))
                || GEM_ATTRIBUTES[..12]
                    .iter()
                    .any(|name| !usage.attributes.iter().any(|value| value == name))
                || !["count", "enableGlobal2"]
                    .iter()
                    .all(|name| usage.guards.iter().any(|guard| guard.attribute == *name))
                || !direct(value, "enableGlobal1")
                || !matches!(value.codec.codec, ValueCodecKind::Boolean { .. })
            {
                return invalid("primary inventory usage disposition");
            }
            Some(BoundPrimaryUsage {
                row: usage,
                policy: &selected.policy,
            })
        } else {
            None
        };
        let strings = [
            &input.game_id,
            &input.variant_id,
            &input.skill_id,
            &input.name_spec,
        ];
        charge(
            &mut compiled.work,
            strings
                .iter()
                .fold(1usize, |n, v| n.saturating_add(v.len())),
            limits,
        )?;
        if strings
            .iter()
            .any(|s| s.is_empty() || s.len() > limits.mapping.max_string_bytes)
            || !selectors.insert((&input.game_id, &input.variant_id))
            || compiled.gems.contains_key(&input.gem)
        {
            return invalid("duplicate or oversized gem inventory identity");
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(input.game_id.clone()),
            variant_id: SourceComponent::Text(input.variant_id.clone()),
        });
        if !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            basis: MappingBasis::Exact,
        }) if gem == &input.gem)
        {
            return invalid("gem inventory exact source identity");
        }
        let Some(role) = roles.role(&input.gem) else {
            return invalid("gem inventory role");
        };
        let OwnedPrimarySkill::Known(primary) = &role.primary else {
            return invalid("gem inventory primary effect");
        };
        let expected_role = if usage.is_some() || disposition.is_some() {
            AuthoredGemRole::SkillUse
        } else {
            AuthoredGemRole::SupportAssignment
        };
        if role.materialization != OwnedGemMaterialization::Physical
            || role.role != OwnedGemRole::Known(expected_role)
            || usage.is_some_and(|usage| usage.row.primary != *primary)
        {
            return invalid("gem inventory physical support role");
        }
        let SchemaLookup::Known(schema) = definitions.definition(&input.gem) else {
            return invalid("gem inventory owner schema");
        };
        charge(
            &mut compiled.work,
            schema.declarations.parameters.members.len()
                + schema.skills.members.len()
                + schema.roles.len()
                + schema.quality.allowed_kinds.members.len(),
            limits,
        )?;
        // Partial skills/parameters remain Partial. The separate catalog-bound
        // source construction claim supplies only this physical inventory proof.
        // A support's source primary is catalogue identity, not an executable
        // skill supply. The current producer emits no supply for this
        // single-support domain. Additional potential
        // skills still require their own reviewed inventory proof.
        let skills_match = match expected_role {
            AuthoredGemRole::SkillUse => schema.skills.members == [primary.clone()],
            AuthoredGemRole::SupportAssignment => schema.skills.members.is_empty(),
        };
        if schema.roles != [expected_role]
            || !skills_match
            || schema.declarations.parameters.members.len() != 2
            || !schema
                .declarations
                .parameters
                .members
                .contains(&input.corrupted)
            || !schema
                .declarations
                .parameters
                .members
                .contains(&input.corruption_level)
            || input.corrupted == input.corruption_level
            || schema.quality.presence == QualityPresence::Forbidden
            || !schema.quality.allowed_kinds.members.contains(kind)
        {
            return invalid("gem inventory declared input domain");
        }
        let Some(rule) = input_rows.get(&input.gem) else {
            return invalid("gem inventory scalar recipe absent");
        };
        charge(&mut compiled.work, rule.parameters.len(), limits)?;
        if rule.parameters.len() != 2 {
            return invalid("gem inventory scalar recipe membership");
        }
        let mut range = None;
        for (slot, attribute) in [
            (&input.corrupted, "corrupted"),
            (&input.corruption_level, "corruptLevel"),
        ] {
            let Some(recipe) = rule.parameters.iter().find(|r| &r.slot == slot) else {
                return invalid("gem inventory scalar slot");
            };
            let SchemaLookup::Known(slot_schema) = definitions.slot(slot) else {
                return invalid("gem inventory slot schema");
            };
            charge(&mut compiled.work, slot_schema.sites.len(), limits)?;
            if slot.declaration != SlotOwnerDefId::Gem(input.gem.clone())
                || slot_schema.presence != SlotPresence::RequiredOnce
                || !slot_schema.sites.contains(&ParameterSite::GemParameter)
                || !direct(&recipe.value, attribute)
                || !matches!(recipe.value.missing, MissingValuePolicy::Pending)
            {
                return invalid("gem inventory required source slot");
            }
            if attribute == "corrupted" {
                if !matches!(slot_schema.value, ValueSchema::Boolean)
                    || !matches!(recipe.value.codec.codec, ValueCodecKind::Boolean { .. })
                {
                    return invalid("gem inventory corruption flag type");
                }
            } else {
                let ValueSchema::Quantity(r) = &slot_schema.value else {
                    return invalid("gem inventory corruption delta type");
                };
                let SchemaLookup::Known(unit) = definitions.definition(r.minimum.unit()) else {
                    return invalid("gem inventory corruption delta unit");
                };
                if unit.dimension != UnitDimension::Count {
                    return invalid("gem inventory corruption delta dimension");
                }
                range = Some(r.clone());
            }
        }
        let disposition = disposition
            .map(|row| dispositions::compile(row, definitions, roles, mappings, limits))
            .transpose()?;
        if let Some(disposition) = &disposition {
            charge(&mut compiled.work, disposition.work, limits)?;
        }
        compiled.gems.insert(
            &input.gem,
            BoundGem {
                input,
                level: schema.level.clone(),
                corruption_level: range.expect("checked corruption-level slot"),
                quality: kind.clone(),
                quality_amount: kind_schema.amount.clone(),
                usage,
                disposition,
            },
        );
    }
    Ok(Some(compiled))
}

const GEM_ATTRIBUTES: &[&str] = &[
    "gemId",
    "variantId",
    "skillId",
    "nameSpec",
    "level",
    "quality",
    "corrupted",
    "corruptLevel",
    "enabled",
    "count",
    "enableGlobal1",
    "enableGlobal2",
    "statSetIndex",
    "statSetIndexCalcs",
];
const MINION_GEM_ATTRIBUTES: &[&str] = &[
    "gemId",
    "variantId",
    "skillId",
    "nameSpec",
    "level",
    "quality",
    "corrupted",
    "corruptLevel",
    "enabled",
    "count",
    "enableGlobal1",
    "enableGlobal2",
    "statSetIndex",
    "statSetIndexCalcs",
    "skillMinion",
    "skillMinionCalcs",
    "skillMinionSkill",
    "skillMinionSkillCalcs",
];
const GROUP_ATTRIBUTES: &[&str] = &[
    "enabled",
    "source",
    "label",
    "mainActiveSkill",
    "mainActiveSkillCalcs",
    "includeInFullDPS",
];

impl CompiledGemInventory<'_> {
    pub(super) fn prove(
        &self,
        b: &mut Builder<'_, '_>,
        context: GemInventoryContext<'_, '_>,
    ) -> Result<Option<GemInventoryProof>> {
        let GemInventoryContext {
            row,
            group,
            gem,
            quality,
            parameters,
        } = context;
        b.charge(1)?;
        let DraftField::Known { value: gem } = gem else {
            return Ok(None);
        };
        let Some(bound) = self.gems.get(gem) else {
            return Ok(None);
        };
        source_shape::charge_row(b, row)?;
        source_shape::charge_row(b, group)?;
        let attributes = if bound
            .disposition
            .as_ref()
            .is_some_and(|row| row.is_minion())
        {
            MINION_GEM_ATTRIBUTES
        } else {
            GEM_ATTRIBUTES
        };
        if row.occurrence().name() != "Gem"
            || group.occurrence().name() != "Skill"
            || row.occurrence().parent() != Some(group.occurrence().id())
            || !source_shape::plain_row(row, attributes, bound.disposition.is_none())
            || !source_shape::container_text(row)
            || !source_shape::plain_row(
                group,
                if bound.disposition.is_some() {
                    dispositions::GROUP_ATTRIBUTES
                } else {
                    GROUP_ATTRIBUTES
                },
                false,
            )
            || !source_shape::container_text(group)
            || GEM_ATTRIBUTES[..12]
                .iter()
                .any(|name| row.attribute(name).is_none())
        {
            return Ok(None);
        }
        for child in group.children() {
            b.charge(1)?;
            let sibling = &b.evidence.rows()[child.ordinal() as usize];
            if sibling.occurrence().name() != "Gem" || sibling.occurrence().has_namespace_context()
            {
                return Ok(None);
            }
        }
        for (name, expected) in [
            ("gemId", bound.input.game_id.as_str()),
            ("variantId", bound.input.variant_id.as_str()),
            ("skillId", bound.input.skill_id.as_str()),
            ("nameSpec", bound.input.name_spec.as_str()),
        ] {
            if source_shape::value(row, name) != Some(expected) {
                return Ok(None);
            }
        }
        if let Some(usage) = bound.usage {
            // Known physical values do not resolve these use/reporting fields.
            // Admit only the injected finite usage domain and ordinary group
            // selection frame; the real Pending destination is checked later.
            if !b.gem_guards_match(row, &usage.row.guards)?
                || ["mainActiveSkill", "mainActiveSkillCalcs"]
                    .iter()
                    .any(|name| {
                        !matches!(source_shape::value(group, name), None | Some("nil" | "1"))
                    })
                || !matches!(
                    source_shape::value(group, "includeInFullDPS"),
                    None | Some("nil" | "false" | "true")
                )
            {
                return Ok(None);
            }
        } else if bound.disposition.is_none() {
            for (name, expected) in [
                ("count", "1"),
                ("enableGlobal1", "true"),
                ("enableGlobal2", "true"),
            ] {
                if source_shape::value(row, name) != Some(expected) {
                    return Ok(None);
                }
            }
        }
        for name in ["statSetIndex", "statSetIndexCalcs"] {
            if bound.disposition.is_none()
                && !matches!(source_shape::value(row, name), None | Some("nil"))
            {
                return Ok(None);
            }
        }
        b.charge(
            self.manual_sources
                .len()
                .saturating_add(self.generated_prefixes.len()),
        )?;
        let source = component(b, group, "source");
        let admitted = source
            .as_ref()
            .is_some_and(|v| self.manual_sources.contains(v))
            || (bound.usage.is_none()
                && bound.disposition.is_none()
                && source.as_ref().is_some_and(|v| {
                    matches!(v, SourceComponent::Text(text)
                if self.generated_prefixes.iter().any(|prefix| text.starts_with(prefix)))
                }));
        if !admitted {
            return Ok(None);
        }
        // Both contexts are checked without allocating another enabled issue.
        for (row, recipe) in [(row, &self.enabled), (group, &self.group_enabled)] {
            if !matches!(
                b.scalar_value(row, recipe)?,
                ScalarValue::Selected(ParameterValue::Boolean(_))
            ) {
                return Ok(None);
            }
        }
        let ScalarValue::Selected(ParameterValue::Integer(level)) =
            b.scalar_value(row, &self.level)?
        else {
            return Ok(None);
        };
        if u16::try_from(level.get()).is_err()
            || level < bound.level.minimum
            || level > bound.level.maximum
        {
            return Ok(None);
        }
        let Some(Some(quality)) = quality.to_resolved() else {
            return Ok(None);
        };
        if quality.kind != bound.quality || !quantity_fits(&quality.amount, &bound.quality_amount) {
            return Ok(None);
        }
        b.charge(parameters.len())?;
        if parameters.len() != 2 {
            return Ok(None);
        }
        let mut seen = BTreeSet::new();
        for parameter in parameters {
            let Some(parameter) = parameter.to_resolved() else {
                return Ok(None);
            };
            if !seen.insert(parameter.slot.clone()) {
                return Ok(None);
            }
            if parameter.slot == bound.input.corrupted {
                if !matches!(parameter.value, ParameterValue::Boolean(_)) {
                    return Ok(None);
                }
            } else if parameter.slot == bound.input.corruption_level {
                if !matches!(&parameter.value, ParameterValue::Quantity(value) if quantity_fits(value, &bound.corruption_level))
                {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }
        if let Some(disposition) = &bound.disposition {
            return Ok(disposition
                .prove(b, row, group)?
                .map(GemInventoryProof::AwaitDisposition));
        }
        Ok(Some(if let Some(usage) = bound.usage {
            b.charge(2)?;
            GemInventoryProof::AwaitUsage(PendingPrimaryInventory {
                source: row.occurrence().id(),
                group: group.occurrence().id(),
                gem: bound.input.gem.clone(),
                policy: usage.policy.clone(),
            })
        } else {
            // Compilation admitted this branch only for a physical
            // SupportAssignment. Primary proofs always take one of the two
            // deferred branches above, independently of policy format.
            GemInventoryProof::PhysicalSupport(PhysicalSupportInputProof {
                source: row.occurrence().id(),
                group: group.occurrence().id(),
                gem: bound.input.gem.clone(),
            })
        }))
    }
}

fn quantity_fits(value: &FiniteQuantity, range: &QuantityRange) -> bool {
    value.unit() == range.minimum.unit()
        && value.unit() == range.maximum.unit()
        && value.value() >= range.minimum.value()
        && value.value() <= range.maximum.value()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owned_value::{BooleanToken, DecimalSyntax, ValueCodecInput, WhitespacePolicy};

    const GEM: &str = r#"<Gem gemId="support" variantId="variant" skillId="support-effect" nameSpec="Fixture support" level="17" quality="0" corrupted="false" corruptLevel="0" enabled="true" count="1" enableGlobal1="true" enableGlobal2="true"/>"#;

    fn xml(gem: &str) -> String {
        format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true" label="Support group">{gem}</Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#
        )
    }

    fn recipe(ns: &GameVersionNamespace, name: &str, boolean: bool) -> ValueRecipe {
        ValueRecipe::new(
            ValueRecipeInput {
                id: key(name),
                codec: ValueCodecInput {
                    namespace: ns.clone(),
                    whitespace: WhitespacePolicy::Exact,
                    codec: if boolean {
                        ValueCodecKind::Boolean {
                            tokens: vec![
                                BooleanToken {
                                    token: "true".into(),
                                    value: true,
                                },
                                BooleanToken {
                                    token: "false".into(),
                                    value: false,
                                },
                            ],
                        }
                    } else {
                        ValueCodecKind::Integer {
                            syntax: DecimalSyntax::Integer,
                        }
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
            },
            Default::default(),
        )
        .unwrap()
    }

    // Exercise the real private runtime proof using the existing Builder fixture.
    // The bound domain models compile's explicit SupportAssignment branch; this
    // does not establish any production definition or source-schema authority.
    fn with_support_domain(
        source_xml: &str,
        test: impl FnOnce(
            &mut Builder<'_, '_>,
            &CompiledGemInventory<'_>,
            &DraftField<GemDefId>,
            &DraftQuality,
            &[ParameterDraft],
            SourceOccurrenceId,
        ),
    ) {
        with_usage_inventory_destination(source_xml, |b, _, _, gem, source| {
            b.attributes = b
                .evidence
                .rows()
                .iter()
                .map(|row| {
                    row.attributes()
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| a.origin().namespace.is_none())
                        .map(|(index, a)| (a.origin().name.as_str(), (index as u32, a)))
                        .collect()
                })
                .collect();
            let ns = gem.namespace();
            let count = UnitDefId::new(ns.clone(), key("count"));
            let percent = UnitDefId::new(ns.clone(), key("percent"));
            let quality_kind = QualityDefId::new(ns.clone(), key("quality"));
            let input = PhysicalGemInputInventory {
                gem: gem.clone(),
                game_id: "support".into(),
                variant_id: "variant".into(),
                skill_id: "support-effect".into(),
                name_spec: "Fixture support".into(),
                corrupted: DeclaredSlot {
                    declaration: SlotOwnerDefId::Gem(gem.clone()),
                    slot: ParameterSlotDefId::new(ns.clone(), key("corrupted")),
                },
                corruption_level: DeclaredSlot {
                    declaration: SlotOwnerDefId::Gem(gem.clone()),
                    slot: ParameterSlotDefId::new(ns.clone(), key("corruption-level")),
                },
            };
            let parameters: Vec<ParameterDraft> = vec![
                ParameterAssignment {
                    slot: input.corrupted.clone(),
                    value: ParameterValue::Boolean(false),
                }
                .into(),
                ParameterAssignment {
                    slot: input.corruption_level.clone(),
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(0.0, count.clone()).unwrap(),
                    ),
                }
                .into(),
            ];
            let quality = DraftQuality::Known {
                value: Some(
                    QualitySelection {
                        kind: quality_kind.clone(),
                        amount: FiniteQuantity::new(0.0, percent.clone()).unwrap(),
                    }
                    .into(),
                ),
            };
            let bound = BoundGem {
                input: &input,
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(40).unwrap(),
                },
                corruption_level: QuantityRange {
                    minimum: FiniteQuantity::new(-10.0, count.clone()).unwrap(),
                    maximum: FiniteQuantity::new(10.0, count).unwrap(),
                },
                quality: quality_kind,
                quality_amount: QuantityRange {
                    minimum: FiniteQuantity::new(0.0, percent.clone()).unwrap(),
                    maximum: FiniteQuantity::new(100.0, percent).unwrap(),
                },
                usage: None,
                disposition: None,
            };
            let manual = [SourceComponent::Missing];
            let domain = CompiledGemInventory {
                gems: [(&input.gem, bound)].into(),
                level: recipe(ns, "level", false),
                enabled: recipe(ns, "enabled", true),
                group_enabled: recipe(ns, "enabled", true),
                manual_sources: &manual,
                generated_prefixes: &[],
                work: 0,
            };
            test(
                b,
                &domain,
                &gem.clone().into(),
                &quality,
                &parameters,
                source,
            );
        });
    }

    fn prove(
        b: &mut Builder<'_, '_>,
        domain: &CompiledGemInventory<'_>,
        gem: &DraftField<GemDefId>,
        quality: &DraftQuality,
        parameters: &[ParameterDraft],
        source: SourceOccurrenceId,
        wrong_group: bool,
    ) -> Option<GemInventoryProof> {
        let evidence = b.evidence;
        let row = &evidence.rows()[source.ordinal() as usize];
        let group = if wrong_group {
            evidence
                .rows()
                .iter()
                .rev()
                .find(|row| row.occurrence().name() == "Skill")
                .unwrap()
        } else {
            &evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize]
        };
        domain
            .prove(
                b,
                GemInventoryContext {
                    row,
                    group,
                    gem,
                    quality,
                    parameters,
                },
            )
            .unwrap()
    }

    #[test]
    fn support_token_preserves_exact_intrinsic_scope_without_usage_meaning() {
        with_support_domain(&xml(GEM), |b, domain, gem, quality, parameters, source| {
            let Some(GemInventoryProof::PhysicalSupport(proof)) =
                prove(b, domain, gem, quality, parameters, source, false)
            else {
                panic!("explicit support proof")
            };
            assert_eq!(proof.source(), source);
            assert_eq!(
                proof.group(),
                b.evidence.rows()[source.ordinal() as usize]
                    .occurrence()
                    .parent()
                    .unwrap()
            );
            assert_eq!(Some(proof.gem()), gem.to_resolved().as_ref());
            assert_eq!(
                proof.occurrence_attributes(),
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
                    "statSetIndex",
                    "statSetIndexCalcs",
                ]
            );
            assert_eq!(proof.group_attributes(), ["enabled", "source", "label"]);
            assert!(proof.reference_descendants().is_empty());
            let before = (b.work, b.links, b.issues, b.allocator.state());
            let origins = b.origins.clone();
            let expected = complete(parameters.to_vec());
            let value = gem_inputs::GemParameters::with_proof(
                expected.clone(),
                GemInventoryProof::PhysicalSupport(proof),
            );
            let (parameters, retained, reservation) = value.finish(b, source, None).unwrap();
            assert!(reservation.is_none());
            let Some(GemInventoryProof::PhysicalSupport(retained)) = retained else {
                panic!("finishing retains exactly the support proof")
            };
            assert_eq!(retained.source(), source);
            assert_eq!(parameters, expected);
            assert_eq!(before, (b.work, b.links, b.issues, b.allocator.state()));
            assert_eq!(b.origins, origins);
        });
    }

    #[test]
    fn support_token_refuses_unknown_fields_references_wrong_owners_and_invalid_inputs() {
        let invalid = [
            GEM.replace("variantId=\"variant\"", "variantId=\"other\""),
            GEM.replace("level=\"17\"", "level=\"41\""),
            GEM.replace("count=\"1\"", "count=\"2\""),
            GEM.replace("enableGlobal2=\"true\"", "enableGlobal2=\"false\""),
            GEM.replace("/>", " statSetIndex=\"1\"/>"),
            GEM.replace("/>", " future=\"true\"/>"),
            GEM.replace(
                "/>",
                "><StatSetIndex grantedEffect=\"support-effect\" index=\"1\"/></Gem>",
            ),
        ];
        // Duplicate XML attributes fail before a source proof can be attempted.
        assert!(
            crate::decode_build(xml(&GEM.replace("/>", " enabled=\"false\"/>")).as_bytes())
                .is_err()
        );
        for raw in invalid {
            with_support_domain(&xml(&raw), |b, domain, gem, quality, parameters, source| {
                assert!(
                    prove(b, domain, gem, quality, parameters, source, false).is_none(),
                    "{raw}"
                );
            });
        }
        with_support_domain(&xml(GEM), |b, domain, gem, quality, parameters, source| {
            assert!(prove(b, domain, gem, quality, parameters, source, true).is_none());
            assert!(prove(b, domain, gem, quality, &parameters[..1], source, false).is_none());
            let foreign: DraftField<_> = GemDefId::new(
                domain.gems.keys().next().unwrap().namespace().clone(),
                key("other"),
            )
            .into();
            assert!(prove(b, domain, &foreign, quality, parameters, source, false).is_none());
        });
    }

    #[test]
    fn unproved_parameters_and_primary_usage_cannot_mint_support_tokens() {
        with_support_domain(&xml(GEM), |b, domain, gem, quality, parameters, source| {
            let expected = complete(parameters.to_vec());
            let (resolved, proof, reservation) =
                gem_inputs::GemParameters::resolved(expected.clone())
                    .finish(b, source, None)
                    .unwrap();
            assert!(proof.is_none());
            assert!(reservation.is_none());
            assert_eq!(resolved, expected);
            let value = gem_inputs::GemParameters::await_usage(
                parameters.to_vec(),
                PendingPrimaryInventory {
                    source,
                    group: b.evidence.rows()[source.ordinal() as usize]
                        .occurrence()
                        .parent()
                        .unwrap(),
                    gem: gem.to_resolved().unwrap(),
                    policy: UsagePolicyDefId::new(
                        domain.gems.keys().next().unwrap().namespace().clone(),
                        key("primary-usage"),
                    ),
                },
            );
            assert!(value.awaits_usage());
            let (unresolved, proof, reservation) = value.finish(b, source, None).unwrap();
            assert!(
                reservation.is_none(),
                "a usage failure is not a reserved reference proof"
            );
            assert!(
                proof.is_none(),
                "missing designated usage cannot mint any receipt"
            );
            assert!(matches!(
                unresolved.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert_eq!(unresolved.members, parameters);

            // The normal finishing path, including imports with no inventory
            // consumer, produces exactly the same values without extra work.
            let Some(GemInventoryProof::PhysicalSupport(proof)) =
                prove(b, domain, gem, quality, parameters, source, false)
            else {
                panic!("explicit support proof")
            };
            let before = (b.work, b.links, b.issues, b.allocator.state());
            let value = gem_inputs::GemParameters::with_proof(
                expected.clone(),
                GemInventoryProof::PhysicalSupport(proof),
            );
            let (parameters, proof, reservation) = value.finish(b, source, None).unwrap();
            assert!(reservation.is_none());
            assert_eq!(parameters, expected);
            assert!(matches!(proof, Some(GemInventoryProof::PhysicalSupport(_))));
            assert_eq!(before, (b.work, b.links, b.issues, b.allocator.state()));
        });
    }
}
