//! Saved generated quality joins an exact existing provider in the independently
//! selected source context. It never creates a provider, supplies a level, or
//! closes gameplay usage. Archived cross-preset correspondence remains Pending.
use super::*;
use poe_optimizer_core::owned_preset_intent::PresetApplicability;
mod source;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratedSkillInputPolicy {
    PobSavedGeneratedInputsV1 {
        definitions: DataIdentity,
        source: SourcePin,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        rows: Vec<GeneratedSkillInputRule>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSkillInputRule {
    /// Source catalog correspondence only, never an authored physical Gem.
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub skill: SkillDefId,
    pub provider: GeneratedSkillInputProvider,
    /// V1 proves exact raw-level equality only. Source-normalized-only matches
    /// require further correspondence and remain Pending, never a copied clamp.
    pub saved_level: ValueRecipeInput,
    pub provider_level: GeneratedSkillProviderLevel,
    pub parameters: Vec<GeneratedSkillParameterInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratedSkillInputProvider {
    TreeAllocation {
        source_node_id: String,
        passive: PassiveNodeDefId,
        skill_supply: DeclaredSlot<SkillGrantSlotDefId>,
    },
    ItemModifier {
        modifier: ModifierDefId,
        skill_supply: DeclaredSlot<SkillGrantSlotDefId>,
        template: ItemTemplateDefId,
        source_name: String,
        /// Exact already-attributed name evidence, not another Item parser.
        name_lines: Vec<GeneratedItemNameLine>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedItemNameLine {
    /// Zero-based position among nonblank attributed semantic lines after the
    /// existing Item preamble's ASCII whitespace normalization. This is not the
    /// one-based raw source line number retained by ItemAttributedLine.
    pub index: usize,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratedSkillProviderLevel {
    Fixed {
        value: BoundedInteger,
    },
    ModifierRoll {
        slot: DeclaredSlot<ParameterSlotDefId>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedSkillInputField {
    Quality,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSkillParameterInput {
    pub field: GeneratedSkillInputField,
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub value: ValueRecipeInput,
}

impl GeneratedSkillInputProvider {
    fn supply(&self) -> &DeclaredSlot<SkillGrantSlotDefId> {
        match self {
            Self::TreeAllocation { skill_supply, .. } | Self::ItemModifier { skill_supply, .. } => {
                skill_supply
            }
        }
    }
}
pub(super) fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<GeneratedSkillInputPolicy>, D::Error> {
    GeneratedSkillInputPolicy::deserialize(deserializer).map(Some)
}
pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    definitions: &DataIdentity,
    roles: &OwnedSkillRoleIndex,
) {
    if let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
        definitions: bound,
        roles: bound_roles,
        ..
    }) = &mut policy.generated_skill_inputs
    {
        *bound = definitions.clone();
        *bound_roles = *roles.identity();
    }
}

struct BoundRow<'p> {
    row: &'p GeneratedSkillInputRule,
    level: ValueRecipe,
    quality: ValueRecipe,
    schema: ParameterSlotSchema,
}
pub(super) struct CompiledGeneratedInputs<'p> {
    rows: BTreeMap<(&'p str, &'p str), Vec<BoundRow<'p>>>,
    pub work: usize,
}
fn invalid<T>(why: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(why))
}
fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|v| *v <= limits.max_work)
        .ok_or(NormalizationError::Limit("generated skill input work"))?;
    Ok(())
}
fn text_valid(text: &str, limits: NormalizationLimits) -> bool {
    !text.is_empty()
        && text.trim() == text
        && !text.chars().any(char::is_control)
        && text.len() <= limits.mapping.max_string_bytes
}

/// Count the bounded policy wire once without retaining a second serialization
/// or computing an unused sub-policy digest. The enclosing normalization policy
/// supplies the public commitment; this pass supplies its own byte/work bound.
fn policy_wire_work<T: Serialize>(input: &T, maximum: usize) -> Result<usize> {
    if maximum == 0 || maximum > poe_optimizer_core::owned_content::MAX_OWNED_CONTENT_BYTES {
        return Err(ContentDigestError::InvalidLimit.into());
    }
    struct Counter {
        written: usize,
        maximum: usize,
        exceeded: bool,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.maximum - self.written {
                self.exceeded = true;
                return Err(std::io::Error::other("generated policy byte limit"));
            }
            self.written += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter {
        written: 0,
        maximum,
        exceeded: false,
    };
    let encoded = serde_json::to_writer(&mut counter, input);
    if counter.exceeded {
        return Err(ContentDigestError::TooLarge { maximum }.into());
    }
    encoded.map_err(ContentDigestError::Json)?;
    Ok(counter.written)
}
fn recipe(
    value: &ValueRecipeInput,
    name: &str,
    namespace: &GameVersionNamespace,
    limits: NormalizationLimits,
) -> Result<ValueRecipe> {
    if &value.codec.namespace != namespace
        || value.codec.whitespace != crate::owned_value::WhitespacePolicy::Exact
        || value.missing != MissingValuePolicy::Pending
        || !value.numeric_aliases.is_empty()
        || value.tiers.len() != 1
        || value.tiers[0].duplicates != DuplicatePolicy::Reject
        || value.tiers[0].selectors.len() != 1
        || value.tiers[0].selectors[0].lane != ValueLane::Attribute
        || value.tiers[0].selectors[0].name != name
    {
        return invalid("generated skill exact scalar recipe");
    }
    Ok(ValueRecipe::new(value.clone(), limits.value)?)
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledGeneratedInputs<'p>>> {
    let Some(
        input @ GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
            definitions: identity,
            source,
            roles: binding,
            catalog,
            rows,
        },
    ) = &policy.generated_skill_inputs
    else {
        return Ok(None);
    };
    if identity != definitions.identity()
        || binding != roles.identity()
        || source != &roles.input().compilation.source
        || catalog != &roles.input().compilation.catalog_digest
        || roles.input().mapping != *mappings.identity()
        || !provenance_is_subset(source, &mappings.input().source)
    {
        return Err(NormalizationError::Binding);
    }
    let wire_work = policy_wire_work(input, limits.max_policy_bytes)?;
    let mut compiled = CompiledGeneratedInputs {
        rows: BTreeMap::new(),
        work: 0,
    };
    charge(&mut compiled.work, wire_work, limits)?;
    if rows.is_empty() || rows.len() > 256 || rows.len() > limits.mapping.max_entries {
        return invalid("generated skill input row inventory");
    }
    let mut authorities = BTreeSet::new();
    for row in rows {
        charge(&mut compiled.work, 1, limits)?;
        if [&row.game_id, &row.variant_id, &row.skill_id, &row.name_spec]
            .iter()
            .any(|v| !text_valid(v, limits))
            || row.parameters.len() != 1
        {
            return invalid("generated skill input source identity or fields");
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(row.game_id.clone()),
            variant_id: SourceComponent::Text(row.variant_id.clone()),
        });
        let effect = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(row.skill_id.clone()),
        });
        if !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)), basis: MappingBasis::Exact,
        }) if gem == &row.gem)
            || !matches!(mappings.lookup(&effect), Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)), basis: MappingBasis::Exact,
            }) if skill == &row.skill)
            || !roles.role(&row.gem).is_some_and(|role| {
                role.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse)
                    && role.primary == OwnedPrimarySkill::Known(row.skill.clone())
                    && !matches!(
                        role.materialization,
                        OwnedGemMaterialization::Unmapped { .. }
                    )
            })
        {
            return invalid("generated skill exact provider catalog role");
        }
        let supply = row.provider.supply();
        let frame = match &row.provider {
            GeneratedSkillInputProvider::TreeAllocation { source_node_id, .. } => {
                source_node_id.as_str()
            }
            GeneratedSkillInputProvider::ItemModifier { source_name, .. } => source_name.as_str(),
        };
        if !authorities.insert((supply, frame)) {
            return invalid("duplicate generated skill supply authority");
        }
        let declared = match &row.provider {
            GeneratedSkillInputProvider::TreeAllocation {
                passive,
                source_node_id,
                ..
            } => {
                if !source::positive(source_node_id)
                    || supply.declaration != SlotOwnerDefId::PassiveNode(passive.clone())
                    || !matches!(
                        row.provider_level,
                        GeneratedSkillProviderLevel::Fixed { .. }
                    )
                {
                    return invalid("generated tree provider declaration");
                }
                let SchemaLookup::Known(schema) = definitions.definition(passive) else {
                    return invalid("generated tree provider schema");
                };
                &schema.declarations
            }
            GeneratedSkillInputProvider::ItemModifier {
                modifier,
                template,
                source_name,
                name_lines,
                ..
            } => {
                if supply.declaration != SlotOwnerDefId::Modifier(modifier.clone())
                    || !text_valid(source_name, limits)
                    || name_lines.is_empty()
                    || name_lines.len() > 4
                    || name_lines
                        .iter()
                        .any(|line| line.index >= 8 || !text_valid(&line.text, limits))
                    || name_lines
                        .iter()
                        .map(|l| l.index)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != name_lines.len()
                {
                    return invalid("generated item source name proof");
                }
                let SchemaLookup::Known(item) = definitions.definition(template) else {
                    return invalid("generated item template schema");
                };
                charge(&mut compiled.work, item.modifiers.members.len(), limits)?;
                if !item.modifiers.members.contains(modifier) {
                    return invalid("generated item modifier membership");
                }
                let SchemaLookup::Known(schema) = definitions.definition(modifier) else {
                    return invalid("generated item modifier schema");
                };
                let GeneratedSkillProviderLevel::ModifierRoll { slot } = &row.provider_level else {
                    return invalid("generated item level authority");
                };
                charge(
                    &mut compiled.work,
                    schema.declarations.parameters.members.len(),
                    limits,
                )?;
                if slot.declaration != SlotOwnerDefId::Modifier(modifier.clone())
                    || !schema.declarations.parameters.members.contains(slot)
                    || !matches!(definitions.slot(slot), SchemaLookup::Known(s)
                        if matches!(s.value, ValueSchema::Integer(_)) && s.sites == [ParameterSite::ModifierRoll])
                {
                    return invalid("generated item level roll");
                }
                &schema.declarations
            }
        };
        charge(
            &mut compiled.work,
            declared.skill_grants.members.len(),
            limits,
        )?;
        if !declared.skill_grants.members.contains(supply) {
            return invalid("generated supply declaration");
        }
        let SchemaLookup::Known(grant) = definitions.slot(supply) else {
            return invalid("generated supply schema");
        };
        let Some(permission) = &grant.preset_inputs else {
            return invalid("generated supply lacks preset permission");
        };
        if grant.skill != row.skill
            || permission.schema_version != PRESET_SKILL_INPUT_PERMISSION_V1
            || permission.parameters.closure != SchemaClosure::Complete
        {
            return invalid("generated supply preset permission");
        }
        let parameter = &row.parameters[0];
        let SchemaLookup::Known(skill) = definitions.definition(&row.skill) else {
            return invalid("generated skill schema");
        };
        charge(
            &mut compiled.work,
            skill.declarations.parameters.members.len() + permission.parameters.members.len(),
            limits,
        )?;
        if parameter.slot.declaration != SlotOwnerDefId::Skill(row.skill.clone())
            || !skill
                .declarations
                .parameters
                .members
                .contains(&parameter.slot)
            || !permission.parameters.members.contains(&parameter.slot)
        {
            return invalid("generated skill parameter declaration");
        }
        let SchemaLookup::Known(schema) = definitions.slot(&parameter.slot) else {
            return invalid("generated skill parameter schema");
        };
        if !matches!(
            schema.skill_input,
            Some(SkillInputAuthority::Projected | SkillInputAuthority::AuthoredOrProjected)
        ) {
            return invalid("generated skill explicit projected authority");
        }
        for slot in &permission.parameters.members {
            let SchemaLookup::Known(required) = definitions.slot(slot) else {
                return invalid("generated permission parameter schema");
            };
            if required.presence == SlotPresence::RequiredOnce && slot != &parameter.slot {
                return invalid("generated input required permission coverage");
            }
        }
        match (&parameter.value.codec.codec, &schema.value) {
            (ValueCodecKind::Quantity { unit, scale, .. }, ValueSchema::Quantity(range))
                if unit == range.minimum.unit()
                    && unit == range.maximum.unit()
                    && scale.numerator.get() == 1
                    && scale.denominator.get() == 1
                    && matches!(definitions.definition(unit), SchemaLookup::Known(_)) => {}
            _ => return invalid("generated quality codec and unit"),
        }
        if !matches!(row.saved_level.codec.codec, ValueCodecKind::Integer { .. }) {
            return invalid("generated saved level codec");
        }
        let level = recipe(&row.saved_level, "level", definitions.namespace(), limits)?;
        let quality = recipe(&parameter.value, "quality", definitions.namespace(), limits)?;
        compiled
            .rows
            .entry((&row.game_id, &row.variant_id))
            .or_default()
            .push(BoundRow {
                row,
                level,
                quality,
                schema: schema.clone(),
            });
    }
    Ok(Some(compiled))
}

pub(super) use source::{Context, materialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_policy_single_pass_counts_exact_wire_and_rejects_limit_plus_one() {
        let input =
            serde_json::json!({"name": "quoted \"name\"", "unicode": "é", "rows": [1, 2, 3]});
        let length = serde_json::to_vec(&input).unwrap().len();
        assert_eq!(policy_wire_work(&input, length).unwrap(), length);
        assert!(matches!(policy_wire_work(&input, length - 1),
            Err(NormalizationError::Digest(ContentDigestError::TooLarge { maximum })) if maximum == length - 1));
        assert_eq!(policy_wire_work(&input, length).unwrap(), length);
    }
}
