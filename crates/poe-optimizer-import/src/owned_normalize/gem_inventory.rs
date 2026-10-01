//! Finite saved physical-support input inventories. This proves only the actual
//! assignment list; partial definition, effect and calculation coverage survives.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GemInventoryPolicy {
    PobFreshSingleSupportV1 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        gems: Vec<SingleSupportGemInventory>,
    },
}

/// Reviewed against the authenticated catalog, including absent declared,
/// constructed and resolved additional effects/stat sets and physical origin.
/// The role package binds that catalog and its exact source/mapping commitments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SingleSupportGemInventory {
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub corrupted: DeclaredSlot<ParameterSlotDefId>,
    pub corruption_level: DeclaredSlot<ParameterSlotDefId>,
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
    input: &'p SingleSupportGemInventory,
    level: IntegerRange,
    corruption_level: QuantityRange,
    quality: QualityDefId,
    quality_amount: QuantityRange,
}

pub(super) struct GemInventoryContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub group: &'a SourceEvidenceRow<'s>,
    pub gem: &'a DraftField<GemDefId>,
    pub quality: &'a DraftQuality,
    pub parameters: &'a [ParameterDraft],
}
pub(super) struct GemInventoryProof(());

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
    limits: NormalizationLimits,
) -> Result<Option<CompiledGemInventory<'p>>> {
    let Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
        definitions: identity,
        roles: role_identity,
        catalog,
        scalar_inputs,
        gems,
    }) = &policy.gem_inventory
    else {
        return Ok(None);
    };
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || roles.input().definitions != *definitions.identity()
        || catalog != &roles.input().compilation.catalog_digest
        || scalar_inputs != &gem_inventory_scalar_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    digest_owned(
        "owned-gem-inventory-policy-v1",
        &policy.gem_inventory,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?;
    if gems.len() > 4096 {
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
    if gems.is_empty() {
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
    let mut selectors = BTreeSet::new();
    for input in gems {
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
        if role.materialization != OwnedGemMaterialization::Physical
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)
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
        if schema.roles != [AuthoredGemRole::SupportAssignment]
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
        compiled.gems.insert(
            &input.gem,
            BoundGem {
                input,
                level: schema.level.clone(),
                corruption_level: range.expect("checked corruption-level slot"),
                quality: kind.clone(),
                quality_amount: kind_schema.amount.clone(),
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
        if row.occurrence().name() != "Gem"
            || group.occurrence().name() != "Skill"
            || row.occurrence().parent() != Some(group.occurrence().id())
            || !source_shape::plain_row(row, GEM_ATTRIBUTES, true)
            || !source_shape::plain_row(group, GROUP_ATTRIBUTES, false)
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
            ("count", "1"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
        ] {
            if source_shape::value(row, name) != Some(expected) {
                return Ok(None);
            }
        }
        for name in ["statSetIndex", "statSetIndexCalcs"] {
            if !matches!(source_shape::value(row, name), None | Some("nil")) {
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
            || source.as_ref().is_some_and(|v| {
                matches!(v, SourceComponent::Text(text)
                if self.generated_prefixes.iter().any(|prefix| text.starts_with(prefix)))
            });
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
        Ok(Some(GemInventoryProof(())))
    }
}

fn quantity_fits(value: &FiniteQuantity, range: &QuantityRange) -> bool {
    value.unit() == range.minimum.unit()
        && value.unit() == range.maximum.unit()
        && value.value() >= range.minimum.value()
        && value.value() <= range.maximum.value()
}
