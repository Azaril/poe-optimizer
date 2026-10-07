//! Saved generated quality joins an exact existing provider in the independently
//! selected source context. It never creates a provider, supplies a level, or
//! closes gameplay usage. Archived cross-preset correspondence remains Pending.
use super::*;
mod dispositions;
pub(super) use dispositions::account;
use generated_skill_sources::{charge, invalid, recipe};
use poe_optimizer_core::owned_preset_intent::PresetApplicability;

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
    pub(super) fn supply(&self) -> &DeclaredSlot<SkillGrantSlotDefId> {
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
    quality: ValueRecipe,
    schema: ParameterSlotSchema,
}
pub(super) struct CompiledGeneratedInputs<'p> {
    rows: Vec<BoundRow<'p>>,
    sources: generated_skill_sources::CompiledSources<'p>,
    pub work: usize,
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
        rows: Vec::new(),
        sources: generated_skill_sources::CompiledSources::new(),
        work: 0,
    };
    charge(&mut compiled.work, wire_work, limits)?;
    if rows.is_empty() || rows.len() > 256 || rows.len() > limits.mapping.max_entries {
        return invalid("generated skill input row inventory");
    }
    let source_context = generated_skill_sources::CompilationContext {
        definitions,
        roles,
        mappings,
        limits,
    };
    for row in rows {
        charge(&mut compiled.work, 1, limits)?;
        if row.parameters.len() != 1 {
            return invalid("generated skill input source identity or fields");
        }
        compiled.sources.insert(
            row.into(),
            compiled.rows.len(),
            &source_context,
            &mut compiled.work,
        )?;
        let SchemaLookup::Known(grant) = definitions.slot(row.provider.supply()) else {
            unreachable!()
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
        let quality = recipe(&parameter.value, "quality", definitions.namespace(), limits)?;
        compiled.rows.push(BoundRow {
            row,
            quality,
            schema: schema.clone(),
        });
    }
    Ok(Some(compiled))
}

/// Private evidence of an actually emitted, fully known raw-input binding.
/// It is never an admission token for unrelated sources or a public cache format.
pub(super) struct MaterializedInput {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    set: SourceOccurrenceId,
    preset: usize,
    row: usize,
    binding: GeneratedSkillInputBindingDraft,
}
#[derive(Default)]
pub(super) struct InputAccounting {
    materialized: Vec<MaterializedInput>,
    archived: Vec<ArchivedInput>,
}
struct ArchivedInput {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    set: SourceOccurrenceId,
    preset: usize,
    row: usize,
}
pub(super) use generated_skill_sources::Context;
pub(super) fn materialize(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    compiled: Option<&CompiledGeneratedInputs<'_>>,
    context: Context<'_>,
) -> Result<InputAccounting> {
    let Some(compiled) = compiled else {
        return Ok(InputAccounting::default());
    };
    let mut receipts = InputAccounting::default();
    let plans =
        generated_skill_sources::resolve_with_archived(b, draft, &compiled.sources, context)?;
    for generated_skill_sources::ResolvedPreset {
        source: set,
        preset_index: index,
        complete: proven,
        sources: rows,
        archived,
    } in plans
    {
        let preset = &mut draft.skill_presets.members[index];
        if preset.intent.is_some() {
            return invalid("normalization generated intent already exists");
        }
        b.charge(
            preset
                .usage_preferences
                .as_ref()
                .map_or(0, |usage| usage.members.len()),
        )?;
        let usage = match preset.usage_preferences.take() {
            Some(legacy) => DraftList {
                completion: legacy.completion,
                members: legacy
                    .members
                    .into_iter()
                    .map(|selection| PresetUsageBindingDraft {
                        selection,
                        applicability: PresetApplicability::Required,
                    })
                    .collect(),
            },
            // This explicit opt-in conversion preserves legacy absence. It does
            // not retire any existing source/usage/configuration obligation.
            None => complete(vec![]),
        };
        let mut members = Vec::new();
        for row in rows {
            let bound = &compiled.rows[row.index];
            let gem = &b.evidence.rows()[row.source.ordinal() as usize];
            let quality = generated_skill_sources::scalar(b, gem, &bound.quality)?
                .filter(|v| gem_inputs::value_valid(v, &bound.schema.value));
            let value = match quality {
                Some(v) => v.into(),
                None => b.pending(row.source, "generated-skill-input-value-unresolved")?,
            };
            let binding = GeneratedSkillInputBindingDraft {
                target: row.target.clone().into(),
                parameters: complete(vec![ParameterDraft {
                    slot: bound.row.parameters[0].slot.clone().into(),
                    value,
                }]),
                applicability: PresetApplicability::WhenExactSourceSelected,
            };
            if binding
                .parameters
                .members
                .iter()
                .all(|p| matches!(p.value, DraftField::Known { .. }))
            {
                receipts.materialized.push(MaterializedInput {
                    source: row.source,
                    group: row.group,
                    set,
                    preset: index,
                    row: row.index,
                    binding: binding.clone(),
                });
            }
            members.push(binding);
            let link = OwnedOriginTarget::GeneratedSkillInput {
                skill_preset: preset.id,
                target: row.target,
            };
            let sources: BTreeSet<_> = row
                .provider_sources
                .into_iter()
                .chain([set, row.group, row.source])
                .collect();
            b.charge(sources.len())?;
            for source in sources {
                b.link(source, link.clone())?;
            }
        }
        let generated_inputs = if proven {
            complete(members)
        } else {
            b.closure(set, "generated-skill-inputs-not-converted", members)?
        };
        preset.intent = Some(SkillPresetIntentDraftV1 {
            schema_version: 1,
            usage,
            generated_inputs,
        });
        b.charge(archived.len())?;
        receipts
            .archived
            .extend(archived.into_iter().map(|row| ArchivedInput {
                source: row.source,
                group: row.group,
                set,
                preset: index,
                row: row.index,
            }));
    }
    Ok(receipts)
}

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
