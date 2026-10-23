//! Reviewed manual occurrences with typed authored Skill inputs. Catalog role
//! correspondence alone never proves a Direct occurrence or a complete inventory.
use super::*;
use crate::owned_source_actions::SourceActionCorrespondenceContent;
mod dispositions;
pub use dispositions::DirectSkillInputDisposition;
pub(super) use dispositions::PendingDirectDisposition;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectSkillInputPolicy {
    /// Raw inputs and explicitly reviewed complete field dispositions.
    /// An explicit empty disposition list grants only unresolved raw inputs.
    /// Nested source maps are admitted only through their exact correspondence.
    PobManualDirectSkillV2 {
        definitions: DataIdentity,
        source: SourcePin,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        manual_sources: Vec<SourceComponent>,
        group_attributes: Vec<String>,
        skills: Vec<DirectSkillInputRule>,
        dispositions: Vec<DirectSkillInputDisposition>,
    },
}

/// Manual source interpretation without current schema or role bindings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectSkillInputContent {
    PobManualDirectSkillV2 {
        source: SourcePin,
        catalog: OwnedContentDigest,
        manual_sources: Vec<SourceComponent>,
        group_attributes: Vec<String>,
        skills: Vec<DirectSkillInputRule>,
        dispositions: Vec<DirectSkillInputDispositionContent>,
    },
}

/// The saved-field disposition keeps its exact action content and usage obligations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectSkillInputDispositionContent {
    pub skill: SkillDefId,
    pub reference_action: SourceActionCorrespondenceContent,
    pub deferred_usage: Vec<DeferredSourceUsageInput>,
    pub inert_fields: Vec<GemInputGuard>,
    pub group_guards: Vec<GemInputGuard>,
}

impl DirectSkillInputDispositionContent {
    fn from_checked(input: DirectSkillInputDisposition) -> Self {
        let DirectSkillInputDisposition {
            skill,
            reference_action,
            deferred_usage,
            inert_fields,
            group_guards,
        } = input;
        Self {
            skill,
            reference_action: SourceActionCorrespondenceContent::from_checked(reference_action),
            deferred_usage,
            inert_fields,
            group_guards,
        }
    }

    fn into_input<I: DefinitionSchemaIndex>(
        self,
        definitions: &I,
        roles: &OwnedSkillRoleIndex,
    ) -> DirectSkillInputDisposition {
        DirectSkillInputDisposition {
            skill: self.skill,
            reference_action: self.reference_action.into_input(definitions, roles),
            deferred_usage: self.deferred_usage,
            inert_fields: self.inert_fields,
            group_guards: self.group_guards,
        }
    }
}

impl DirectSkillInputContent {
    /// Only the enclosing checked normalization owner may extract inherited content.
    pub(crate) fn from_checked(input: DirectSkillInputPolicy) -> Self {
        match input {
            DirectSkillInputPolicy::PobManualDirectSkillV2 {
                definitions: _,
                source,
                roles: _,
                catalog,
                manual_sources,
                group_attributes,
                skills,
                dispositions,
            } => Self::PobManualDirectSkillV2 {
                source,
                catalog,
                manual_sources,
                group_attributes,
                skills,
                dispositions: dispositions
                    .into_iter()
                    .map(DirectSkillInputDispositionContent::from_checked)
                    .collect(),
            },
        }
    }

    /// Construct a raw policy; ordinary normalization validation remains required.
    pub(crate) fn into_input<I: DefinitionSchemaIndex>(
        self,
        definitions: &I,
        roles: &OwnedSkillRoleIndex,
    ) -> DirectSkillInputPolicy {
        match self {
            Self::PobManualDirectSkillV2 {
                source,
                catalog,
                manual_sources,
                group_attributes,
                skills,
                dispositions,
            } => DirectSkillInputPolicy::PobManualDirectSkillV2 {
                definitions: definitions.identity().clone(),
                source,
                roles: *roles.identity(),
                catalog,
                manual_sources,
                group_attributes,
                skills,
                dispositions: dispositions
                    .into_iter()
                    .map(|row| row.into_input(definitions, roles))
                    .collect(),
            },
        }
    }
}

/// Exact assignment-to-source correspondence, independent of support admission,
/// effect receiving, activation and numerical completeness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectSupportTargetPolicy {
    PobManualSingleDirectRootV1 { direct_inputs: OwnedContentDigest },
}

/// Explicit target authority whose dependency is constructed from final Direct inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectSupportTargetContent {
    PobManualSingleDirectRootV1 {},
}

impl DirectSupportTargetContent {
    /// Only the enclosing checked normalization owner may extract inherited content.
    pub(crate) fn from_checked(input: DirectSupportTargetPolicy) -> Self {
        match input {
            DirectSupportTargetPolicy::PobManualSingleDirectRootV1 { direct_inputs: _ } => {
                Self::PobManualSingleDirectRootV1 {}
            }
        }
    }

    /// Construct a raw policy; this does not establish the Direct source's validity.
    pub(crate) fn into_input(
        self,
        direct: Option<&DirectSkillInputPolicy>,
        limits: NormalizationLimits,
    ) -> Result<DirectSupportTargetPolicy> {
        match self {
            Self::PobManualSingleDirectRootV1 {} => {
                let Some(direct) = direct else {
                    return invalid("Direct support targets require source authority");
                };
                Ok(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 {
                    direct_inputs: direct_skill_inputs_identity(direct, limits)?,
                })
            }
        }
    }
}

pub(super) fn present_support_targets<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<DirectSupportTargetPolicy>, D::Error> {
    DirectSupportTargetPolicy::deserialize(deserializer).map(Some)
}

fn support_target_authority(
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<bool> {
    let Some(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 { direct_inputs }) =
        &policy.direct_support_targets
    else {
        return Ok(false);
    };
    let Some(input) = &policy.direct_skill_inputs else {
        return invalid("Direct support targets require source authority");
    };
    if direct_skill_inputs_identity(input, limits)? != *direct_inputs {
        return Err(NormalizationError::Binding);
    }
    Ok(true)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectSkillInputRule {
    /// Import catalog correspondence only; never materialized as a physical Gem.
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub skill: SkillDefId,
    pub attributes: Vec<String>,
    pub guards: Vec<GemInputGuard>,
    pub parameters: Vec<DirectSkillParameterInput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectSkillParameterInput {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub value: ValueRecipeInput,
}

pub(super) struct CompiledDirectInputs<'p> {
    rules: BTreeMap<ExternalSelector, BoundDirect<'p>>,
    manual_sources: &'p [SourceComponent],
    group_attributes: Vec<&'p str>,
    support_target: bool,
    pub work: usize,
}
struct BoundDirect<'p> {
    input: &'p DirectSkillInputRule,
    attributes: Vec<&'p str>,
    parameters: Vec<BoundParameter>,
    disposition: Option<dispositions::CompiledDirectDisposition<'p>>,
}
struct BoundParameter {
    slot: DeclaredSlot<ParameterSlotDefId>,
    schema: ParameterSlotSchema,
    recipe: ValueRecipe,
}
pub(super) struct DirectInputs {
    pub skill: SkillDefId,
    pub parameters: DraftList<ParameterDraft>,
    pub pending: Option<PendingDirectDisposition>,
    pub support_target: bool,
}

pub(super) fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<DirectSkillInputPolicy>, D::Error> {
    DirectSkillInputPolicy::deserialize(deserializer).map(Some)
}

/// Rebind an already validated inherited policy; callers check the full new
/// package afterward. Source/catalog commitments deliberately remain unchanged.
pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    definitions: &DataIdentity,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<()> {
    // Never turn a stale inherited commitment into a valid new one.
    let support_targets = support_target_authority(policy, limits)?;
    let Some(input) = &mut policy.direct_skill_inputs else {
        return Ok(());
    };
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions: bound_definitions,
        roles: bound_roles,
        dispositions,
        ..
    } = input;
    *bound_definitions = definitions.clone();
    *bound_roles = *roles.identity();
    for row in dispositions {
        crate::owned_source_actions::rebind_input(&mut row.reference_action, definitions, roles);
    }
    if support_targets {
        let identity = direct_skill_inputs_identity(input, limits)?;
        let Some(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 { direct_inputs }) =
            &mut policy.direct_support_targets
        else {
            unreachable!("checked inherited Direct target authority")
        };
        *direct_inputs = identity;
    }
    Ok(())
}

pub(super) fn validate_source(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
) -> Result<()> {
    if let Some(input) = &policy.direct_skill_inputs {
        let DirectSkillInputPolicy::PobManualDirectSkillV2 { source, .. } = input;
        if !provenance_is_subset(source, &mappings.input().source) {
            return Err(NormalizationError::Binding);
        }
    }
    Ok(())
}

fn invalid<T>(reason: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(reason))
}
fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|value| *value <= limits.max_work)
        .ok_or(NormalizationError::Limit("direct skill input work"))?;
    Ok(())
}
fn attributes(names: &[String]) -> Result<Vec<&str>> {
    if names.is_empty()
        || names.len() > 64
        || names.iter().any(|name| name.is_empty() || name.len() > 128)
        || names.iter().collect::<BTreeSet<_>>().len() != names.len()
    {
        return invalid("direct skill input attribute frame");
    }
    Ok(names.iter().map(String::as_str).collect())
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledDirectInputs<'p>>> {
    let support_target = support_target_authority(policy, limits)?;
    let Some(input) = &policy.direct_skill_inputs else {
        return Ok(None);
    };
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions: identity,
        source,
        roles: role_identity,
        catalog,
        manual_sources,
        group_attributes,
        skills,
        dispositions: disposition_rows,
    } = input;
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || identity != &roles.input().definitions
        || source != &roles.input().compilation.source
        || source.system != ExternalSourceSystem::PathOfBuilding2
        || catalog != &roles.input().compilation.catalog_digest
    {
        return Err(NormalizationError::Binding);
    }
    let bytes = serde_json::to_vec(&policy.direct_skill_inputs)
        .map_err(|_| NormalizationError::Policy("direct skill input encoding"))?;
    if bytes.len() > limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES) {
        return Err(NormalizationError::Limit("direct skill input bytes"));
    }
    let mut work = 0;
    charge(&mut work, bytes.len(), limits)?;
    if skills.is_empty() || skills.len() > 4096
        || manual_sources.is_empty() || manual_sources.len() > 64
        || manual_sources.iter().collect::<BTreeSet<_>>().len() != manual_sources.len()
        || manual_sources.iter().any(|source| !policy.manual_skill_sources.contains(source)
            || matches!(source, SourceComponent::Text(text) if text.len() > limits.mapping.max_string_bytes))
    {
        return invalid("direct skill input rows or manual sources");
    }
    charge(
        &mut work,
        manual_sources
            .len()
            .saturating_mul(policy.manual_skill_sources.len()),
        limits,
    )?;
    let group_attributes = attributes(group_attributes)?;
    if !group_attributes.contains(&"source") {
        return invalid("direct skill group source frame");
    }
    let mut compiled = CompiledDirectInputs {
        rules: BTreeMap::new(),
        manual_sources,
        group_attributes,
        support_target,
        work,
    };
    if disposition_rows.len() > skills.len() {
        return invalid("direct skill disposition rows");
    }
    let mut dispositions = BTreeMap::new();
    for row in disposition_rows {
        charge(&mut compiled.work, 1, limits)?;
        if dispositions.insert(&row.skill, row).is_some() {
            return invalid("duplicate direct skill disposition");
        }
    }
    let mut gems = BTreeSet::new();
    for input in skills {
        charge(&mut compiled.work, 1, limits)?;
        if input.guards.len() > 64
            || input.parameters.is_empty()
            || input.parameters.len() > 64
            || !gems.insert(&input.gem)
            || [
                &input.game_id,
                &input.variant_id,
                &input.skill_id,
                &input.name_spec,
            ]
            .iter()
            .any(|value| value.is_empty() || value.len() > limits.mapping.max_string_bytes)
        {
            return invalid("duplicate or oversized direct skill row");
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(input.game_id.clone()),
            variant_id: SourceComponent::Text(input.variant_id.clone()),
        });
        if compiled.rules.contains_key(&selector)
            || !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
                basis: MappingBasis::Exact,
            }) if gem == &input.gem)
        {
            return invalid("direct skill exact source mapping");
        }
        let Some(role) = roles.role(&input.gem) else {
            return invalid("direct skill catalog role");
        };
        if role.materialization != OwnedGemMaterialization::ProviderOnly
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
            || role.primary != OwnedPrimarySkill::Known(input.skill.clone())
        {
            return invalid("direct skill provider primary role");
        }
        let SchemaLookup::Known(skill) = definitions.definition(&input.skill) else {
            return invalid("direct skill owner schema");
        };
        if !skill.directly_selectable {
            return invalid("direct skill owner is not directly selectable");
        }
        let attributes = attributes(&input.attributes)?;
        if ["gemId", "variantId", "skillId", "nameSpec"]
            .iter()
            .any(|name| !attributes.contains(name))
        {
            return invalid("direct skill source identity frame");
        }
        let mut guards = BTreeSet::new();
        for guard in &input.guards {
            if !attributes.contains(&guard.attribute.as_str()) || !guards.insert(&guard.attribute)
                || guard.allowed.is_empty() || guard.allowed.len() > 64
                || guard.allowed.iter().collect::<BTreeSet<_>>().len() != guard.allowed.len()
                || guard.allowed.iter().any(|value| matches!(value, SourceComponent::Text(text) if text.len() > limits.mapping.max_string_bytes))
            {
                return invalid("direct skill source guards");
            }
        }
        charge(
            &mut compiled.work,
            skill.declarations.parameters.members.len(),
            limits,
        )?;
        let declared: BTreeSet<_> = skill.declarations.parameters.members.iter().collect();
        let mut slots = BTreeSet::new();
        let mut parameters = Vec::new();
        for input_parameter in &input.parameters {
            let slot = &input_parameter.slot;
            let value = &input_parameter.value;
            if slot.declaration != SlotOwnerDefId::Skill(input.skill.clone())
                || !declared.contains(slot)
                || !slots.insert(slot)
            {
                return invalid("direct skill input slot declaration");
            }
            let SchemaLookup::Known(schema) = definitions.slot(slot) else {
                return invalid("direct skill input slot schema");
            };
            charge(&mut compiled.work, schema.sites.len(), limits)?;
            if let ValueSchema::Option { allowed } = &schema.value {
                charge(&mut compiled.work, allowed.members.len(), limits)?;
            }
            if !schema.permits_authored_skill_input()
                || schema.sites != [ParameterSite::SkillParameter]
                || value.codec.namespace != *definitions.namespace()
                || value.codec.whitespace != crate::owned_value::WhitespacePolicy::Exact
                || !matches!(value.missing, MissingValuePolicy::Pending)
                || !value.numeric_aliases.is_empty()
                || value.tiers.len() != 1
                || value.tiers[0].duplicates != DuplicatePolicy::Reject
                || value.tiers[0].selectors.len() != 1
                || value.tiers[0].selectors.iter().any(|selector| {
                    selector.lane != ValueLane::Attribute
                        || !attributes.contains(&selector.name.as_str())
                })
            {
                return invalid("direct skill input authority or recipe");
            }
            let compatible = match (&value.codec.codec, &schema.value) {
                (ValueCodecKind::Boolean { .. }, ValueSchema::Boolean)
                | (ValueCodecKind::Integer { .. }, ValueSchema::Integer(_)) => true,
                (ValueCodecKind::Quantity { unit, .. }, ValueSchema::Quantity(range)) => {
                    unit == range.minimum.unit()
                        && unit == range.maximum.unit()
                        && matches!(definitions.definition(unit), SchemaLookup::Known(_))
                }
                (ValueCodecKind::Option { tokens }, ValueSchema::Option { allowed }) => {
                    charge(
                        &mut compiled.work,
                        tokens.len().saturating_mul(allowed.members.len()),
                        limits,
                    )?;
                    tokens.iter().all(|token| {
                        allowed.members.contains(&token.value)
                            && matches!(
                                definitions.definition(&token.value),
                                SchemaLookup::Known(_)
                            )
                    })
                }
                _ => false,
            };
            if !compatible {
                return invalid("direct skill input codec schema");
            }
            parameters.push(BoundParameter {
                slot: slot.clone(),
                schema: schema.clone(),
                recipe: ValueRecipe::new(value.clone(), limits.value)?,
            });
        }
        let disposition = if let Some(row) = dispositions.remove(&input.skill) {
            Some(dispositions::compile(
                row,
                input,
                manual_sources,
                &compiled.group_attributes,
                definitions,
                roles,
                mappings,
                limits,
                &mut compiled.work,
            )?)
        } else {
            None
        };
        compiled.rules.insert(
            selector,
            BoundDirect {
                input,
                attributes,
                parameters,
                disposition,
            },
        );
    }
    if !dispositions.is_empty() {
        return invalid("unmatched direct skill disposition");
    }
    Ok(Some(compiled))
}

impl CompiledDirectInputs<'_> {
    /// Only the immutable, already-compiled selector correspondence is shared.
    /// Raw Direct fields and manual-root authority are not applied to generated uses.
    pub(super) fn generated_selector_adapter(
        &self,
        row: &generated_skill_inputs::GeneratedSkillInputRule,
    ) -> Option<&crate::owned_source_actions::SourceActionCorrespondence> {
        self.generated_disposition(row)
            .map(|d| d.selector_adapter())
    }
    /// Share the existing typed deferred-field proof without assigning the
    /// archived generated row any Direct skill or raw-input ownership.
    pub(super) fn generated_deferred_usage(
        &self,
        row: &generated_skill_inputs::GeneratedSkillInputRule,
    ) -> Option<&skill_input_disposition::CompiledDeferredUsage> {
        self.generated_disposition(row).map(|d| d.deferred_usage())
    }
    fn generated_disposition(
        &self,
        row: &generated_skill_inputs::GeneratedSkillInputRule,
    ) -> Option<&dispositions::CompiledDirectDisposition<'_>> {
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(row.game_id.clone()),
            variant_id: SourceComponent::Text(row.variant_id.clone()),
        });
        let bound = self.rules.get(&selector)?;
        let input = bound.input;
        (input.gem == row.gem
            && input.game_id == row.game_id
            && input.variant_id == row.variant_id
            && input.skill_id == row.skill_id
            && input.name_spec == row.name_spec
            && input.skill == row.skill)
            .then_some(bound.disposition.as_ref())
            .flatten()
    }
    pub(super) fn sets(&self, b: &mut Builder<'_, '_>) -> Result<Option<Vec<SourceOccurrenceId>>> {
        skill_source_census::container_sets(b)
    }

    /// A cheap exact catalog candidate check, not source-frame authority. Callers
    /// must inspect the complete immutable saved-set frame before attachment.
    pub(super) fn matches_selector(
        &self,
        b: &mut Builder<'_, '_>,
        selector: Option<&ExternalSelector>,
    ) -> Result<bool> {
        b.charge(1)?;
        Ok(selector.is_some_and(|selector| self.rules.contains_key(selector)))
    }

    /// Successful admission proves this source occurrence and scalar destinations,
    /// never the complete input, use, support, or generated-effect inventories.
    pub(super) fn inputs(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
        selector: Option<&ExternalSelector>,
    ) -> Result<Option<DirectInputs>> {
        b.charge(1)?;
        let Some(input) = selector.and_then(|selector| self.rules.get(selector)) else {
            return Ok(None);
        };
        source_shape::charge_row(b, row)?;
        source_shape::charge_row(b, group)?;
        b.charge(
            row.attributes()
                .len()
                .saturating_mul(input.attributes.len())
                .saturating_add(
                    group
                        .attributes()
                        .len()
                        .saturating_mul(self.group_attributes.len()),
                ),
        )?;
        if row.occurrence().name() != "Gem"
            || group.occurrence().name() != "Skill"
            || row.occurrence().parent() != Some(group.occurrence().id())
            || !source_shape::plain_row(row, &input.attributes, input.disposition.is_none())
            || !source_shape::container_text(row)
            || !source_shape::plain_row(group, &self.group_attributes, false)
            || !source_shape::container_text(group)
            || source_shape::value(row, "gemId") != Some(input.input.game_id.as_str())
            || source_shape::value(row, "variantId") != Some(input.input.variant_id.as_str())
            || source_shape::value(row, "skillId") != Some(input.input.skill_id.as_str())
            || source_shape::value(row, "nameSpec") != Some(input.input.name_spec.as_str())
        {
            return Ok(None);
        }
        let source = source_shape::value(group, "source")
            .map_or(SourceComponent::Missing, |value| {
                SourceComponent::Text(value.into())
            });
        b.charge(self.manual_sources.len().saturating_mul(match &source {
            SourceComponent::Missing => 1,
            SourceComponent::Text(text) => text.len().saturating_add(1),
        }))?;
        if !self.manual_sources.contains(&source)
            || !b.gem_guards_match(row, &input.input.guards)?
        {
            return Ok(None);
        }
        let reference_children = if let Some(disposition) = &input.disposition {
            disposition.reference(b, row, group)?
        } else {
            None
        };
        if !row.children().is_empty() && reference_children.is_none() {
            return Ok(None);
        }
        let source = row.occurrence().id();
        let mut members = Vec::new();
        for parameter in &input.parameters {
            let selector = &parameter.recipe.input().tiers[0].selectors[0];
            b.charge(row.attributes().len())?;
            if let Some(attribute) = row.attribute(&selector.name) {
                b.charge(attribute.raw().len())?;
            }
            if let ValueSchema::Option { allowed } = &parameter.schema.value {
                b.charge(allowed.members.len())?;
            }
            let value = match b.scalar_value(row, &parameter.recipe) {
                Ok(ScalarValue::Selected(value))
                    if gem_inputs::value_valid(&value, &parameter.schema.value) =>
                {
                    value.into()
                }
                Ok(_) | Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {
                    b.pending(source, "direct-skill-input-not-converted")?
                }
                Err(error) => return Err(error),
            };
            members.push(ParameterDraft {
                slot: parameter.slot.clone().into(),
                value,
            });
        }
        // Exact target identity requires the reviewed disposition and its
        // successful source/reference proof, independently of raw input
        // readiness. An explicit empty source string is generated/source-owned.
        let support_target = self.support_target
            && input.disposition.is_some()
            && reference_children.is_some()
            && group.attribute("source").is_none();
        let pending = match (&input.disposition, reference_children) {
            (Some(disposition), Some(children)) => {
                disposition.capture(b, row, group, &members, children)?
            }
            _ => None,
        };
        Ok(Some(DirectInputs {
            skill: input.input.skill.clone(),
            parameters: b.closure(source, "direct-skill-parameters-not-converted", members)?,
            pending,
            support_target,
        }))
    }
}
