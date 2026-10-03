//! Finite saved physical-Gem input inventories. This proves only the actual
//! assignment list; partial definition, effect and calculation coverage survives.
use super::*;
mod dispositions;
pub use dispositions::{DeferredGemUsageField, DeferredGemUsageInput, PrimaryGemInputDisposition};
pub(super) use dispositions::{PendingDisposition, ProvenDisposition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GemInventoryPolicy {
    PobFreshSingleSupportV1 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        gems: Vec<PhysicalGemInputInventory>,
    },
    PobFreshPhysicalV2 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        usage_inputs: OwnedContentDigest,
        supports: Vec<PhysicalGemInputInventory>,
        primary_skills: Vec<PrimarySkillGemInventory>,
    },
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
/// catalog. Each inventory version supplies its own effect/reference disposition
/// proof. V1/V2 still require absent additional effects/stat sets; V3 may retain
/// reviewed alternative actions. The role package binds the exact catalog/source.
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
    let usage = matches!(
        policy.gem_inventory,
        Some(
            GemInventoryPolicy::PobFreshPhysicalV2 { .. }
                | GemInventoryPolicy::PobFreshPhysicalV3 { .. }
        )
    )
    .then(|| usage_inputs_identity(policy, limits))
    .transpose()?;
    let (bound_definitions, bound_roles, scalar_inputs) =
        match policy.gem_inventory.as_mut().unwrap() {
            GemInventoryPolicy::PobFreshSingleSupportV1 {
                definitions,
                roles,
                scalar_inputs,
                ..
            } => (definitions, roles, scalar_inputs),
            GemInventoryPolicy::PobFreshPhysicalV2 {
                definitions,
                roles,
                scalar_inputs,
                usage_inputs,
                ..
            } => {
                *usage_inputs = usage.expect("V2 usage commitment");
                (definitions, roles, scalar_inputs)
            }
            GemInventoryPolicy::PobFreshPhysicalV3 {
                definitions: bound,
                roles: bound_roles,
                scalar_inputs,
                usage_inputs,
                primary_dispositions,
                ..
            } => {
                *usage_inputs = usage.expect("V3 inherited usage commitment");
                for row in primary_dispositions {
                    crate::owned_source_actions::rebind_input(
                        &mut row.reference_action,
                        definitions,
                        roles,
                    );
                }
                (bound, bound_roles, scalar_inputs)
            }
        };
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
    usage: Option<&'p PrimarySkillUsageInput>,
    disposition: Option<dispositions::CompiledDisposition<'p>>,
}

pub(super) struct GemInventoryContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub group: &'a SourceEvidenceRow<'s>,
    pub gem: &'a DraftField<GemDefId>,
    pub quality: &'a DraftQuality,
    pub parameters: &'a [ParameterDraft],
}
pub(super) enum GemInventoryProof {
    Physical,
    AwaitUsage(PendingPrimaryInventory),
    AwaitDisposition(PendingDisposition),
}
pub(super) struct PendingPrimaryInventory {
    source: SourceOccurrenceId,
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
    let (
        identity,
        role_identity,
        catalog,
        scalar_inputs,
        supports,
        primary_skills,
        primary_dispositions,
        usage_binding,
    ) = match inventory {
        GemInventoryPolicy::PobFreshSingleSupportV1 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
        } => (
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems.as_slice(),
            &[][..],
            &[][..],
            None,
        ),
        GemInventoryPolicy::PobFreshPhysicalV2 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            usage_inputs,
            supports,
            primary_skills,
        } => (
            definitions,
            roles,
            catalog,
            scalar_inputs,
            supports.as_slice(),
            primary_skills.as_slice(),
            &[][..],
            Some(usage_inputs),
        ),
        GemInventoryPolicy::PobFreshPhysicalV3 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            usage_inputs,
            supports,
            primary_skills,
            primary_dispositions,
        } => (
            definitions,
            roles,
            catalog,
            scalar_inputs,
            supports.as_slice(),
            primary_skills.as_slice(),
            primary_dispositions.as_slice(),
            Some(usage_inputs),
        ),
    };
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || roles.input().definitions != *definitions.identity()
        || catalog != &roles.input().compilation.catalog_digest
        || scalar_inputs != &gem_inventory_scalar_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    let usage_required = matches!(inventory, GemInventoryPolicy::PobFreshPhysicalV2 { .. })
        || !primary_skills.is_empty();
    if let Some(binding) = usage_binding
        && ((usage_required && policy.usage_inputs.is_none())
            || binding != &usage_inputs_identity(policy, limits)?)
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
        let gems = usage.boolean_rows();
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
            if usage.attributes.len() > 64 || usage.guards.len() > 64 || usage.parameters.len() != 1
            {
                return invalid("primary inventory usage disposition limits");
            }
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
            if usage.policy != primary_inventory.usage_policy
                || usage.game_id != input.game_id
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
                || usage.parameters.len() != 1
                || !direct(&usage.parameters[0].value, "enableGlobal1")
            {
                return invalid("primary inventory usage disposition");
            }
            Some(usage)
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
            || usage.is_some_and(|usage| usage.primary != *primary)
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
        if schema.roles != [expected_role]
            || schema.skills.members != [primary.clone()]
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
            if !b.gem_guards_match(row, &usage.guards)?
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
                gem: bound.input.gem.clone(),
                policy: usage.policy.clone(),
            })
        } else {
            GemInventoryProof::Physical
        }))
    }
}

fn quantity_fits(value: &FiniteQuantity, range: &QuantityRange) -> bool {
    value.unit() == range.minimum.unit()
        && value.unit() == range.maximum.unit()
        && value.value() >= range.minimum.value()
        && value.value() <= range.maximum.value()
}
