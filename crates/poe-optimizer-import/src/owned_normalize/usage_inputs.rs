//! Finite source-to-usage projection. This does not by itself prove a usage or
//! physical Gem inventory; the private attachment token participates in V2 proof.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInputPolicy {
    PobPhysicalPrimarySkillV1 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        gems: Vec<PrimarySkillUsageInput>,
    },
}

/// A catalog-bound physical row and its exact primary supply. V1 transports
/// required Boolean parameters only; it invents neither source defaults nor
/// count/population semantics. `attributes` admits a source frame, not values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimarySkillUsageInput {
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub primary: SkillDefId,
    pub supply: DeclaredSlot<SkillGrantSlotDefId>,
    pub grant: DeclaredSlot<GrantSlotDefId>,
    pub policy: UsagePolicyDefId,
    pub attributes: Vec<String>,
    pub guards: Vec<GemInputGuard>,
    pub parameters: Vec<GemParameterInput>,
}

pub(super) struct CompiledUsageInputs<'p> {
    rules: BTreeMap<&'p GemDefId, BoundUsage<'p>>,
    pub work: usize,
}
struct BoundUsage<'p> {
    input: &'p PrimarySkillUsageInput,
    attributes: Vec<&'p str>,
    parameters: Vec<(DeclaredSlot<ParameterSlotDefId>, ValueRecipe)>,
}

/// Acyclic commitment to the complete existing source-to-usage projection.
pub fn usage_inputs_identity(
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-usage-inputs-v1",
        &policy.usage_inputs,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?)
}

/// Constructible only after attaching the real source-linked preference. Its
/// presence proves neither complete usage inventory nor numerical consumers.
pub(super) struct AttachedPrimaryUsage<'p> {
    input: &'p PrimarySkillUsageInput,
    source: SourceOccurrenceId,
}
pub(super) struct UsageInputContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub group: &'a SourceEvidenceRow<'s>,
    pub gem: &'a DraftField<GemDefId>,
    pub skill: SkillUseId,
    pub preset: &'a mut SkillPresetDraft,
    pub inventory_proof: bool,
}
impl AttachedPrimaryUsage<'_> {
    pub(super) fn proves(
        &self,
        source: SourceOccurrenceId,
        gem: &GemDefId,
        policy: &UsagePolicyDefId,
    ) -> bool {
        self.source == source && self.input.gem == *gem && self.input.policy == *policy
    }
}

fn invalid<T>(reason: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(reason))
}
fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|value| *value <= limits.max_work)
        .ok_or(NormalizationError::Limit("usage input work"))?;
    Ok(())
}

/// Only checked, inherited policies may call this while constructing a new
/// release. The caller validates the full resulting policy again afterward.
pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    definitions: &DataIdentity,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<()> {
    if policy.usage_inputs.is_none() {
        return Ok(());
    }
    let scalar = gem_inventory_scalar_inputs_identity(policy, limits)?;
    let Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions: bound_definitions,
        roles: bound_roles,
        scalar_inputs,
        ..
    }) = &mut policy.usage_inputs
    else {
        unreachable!("checked optional usage policy");
    };
    *bound_definitions = definitions.clone();
    *bound_roles = *roles.identity();
    *scalar_inputs = scalar;
    Ok(())
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledUsageInputs<'p>>> {
    let Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions: identity,
        roles: role_identity,
        catalog,
        scalar_inputs,
        gems,
    }) = &policy.usage_inputs
    else {
        return Ok(None);
    };
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || roles.input().definitions != *identity
        || catalog != &roles.input().compilation.catalog_digest
        || scalar_inputs != &gem_inventory_scalar_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    usage_inputs_identity(policy, limits)?;
    let bytes = serde_json::to_vec(&policy.usage_inputs)
        .map_err(|_| NormalizationError::Policy("usage input encoding"))?;
    if bytes.len() > limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES) {
        return Err(NormalizationError::Limit("usage input bytes"));
    }
    let mut compiled = CompiledUsageInputs {
        rules: BTreeMap::new(),
        work: 0,
    };
    charge(&mut compiled.work, bytes.len(), limits)?;
    if gems.len() > 4096 {
        return Err(NormalizationError::Limit("usage input rows"));
    }
    let mut selectors = BTreeSet::new();
    for input in gems {
        if input.attributes.len() > 64
            || input.guards.len() > 64
            || input.parameters.is_empty()
            || input.parameters.len() > 64
            || compiled.rules.contains_key(&input.gem)
            || !selectors.insert((&input.game_id, &input.variant_id))
        {
            return invalid("duplicate or oversized usage input row");
        }
        if [
            &input.game_id,
            &input.variant_id,
            &input.skill_id,
            &input.name_spec,
        ]
        .iter()
        .any(|value| value.is_empty() || value.len() > limits.mapping.max_string_bytes)
        {
            return invalid("usage input source identity");
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
            return invalid("usage input exact source mapping");
        }
        let Some(role) = roles.role(&input.gem) else {
            return invalid("usage input physical role");
        };
        if role.materialization != OwnedGemMaterialization::Physical
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
            || role.primary != OwnedPrimarySkill::Known(input.primary.clone())
        {
            return invalid("usage input physical primary role");
        }
        let SchemaLookup::Known(gem) = definitions.definition(&input.gem) else {
            return invalid("usage input gem schema");
        };
        charge(
            &mut compiled.work,
            gem.roles.len()
                + gem.skills.members.len()
                + gem.declarations.skill_grants.members.len()
                + gem.declarations.grants.members.len(),
            limits,
        )?;
        if !gem.roles.contains(&AuthoredGemRole::SkillUse)
            || !gem.skills.members.contains(&input.primary)
            || !matches!(
                definitions.definition(&input.primary),
                SchemaLookup::Known(_)
            )
            || input.supply.declaration != SlotOwnerDefId::Gem(input.gem.clone())
            || input.grant.declaration != SlotOwnerDefId::Gem(input.gem.clone())
            || !gem
                .declarations
                .skill_grants
                .members
                .contains(&input.supply)
            || !gem.declarations.grants.members.contains(&input.grant)
        {
            return invalid("usage input declared primary supply");
        }
        let SchemaLookup::Known(supply) = definitions.slot(&input.supply) else {
            return invalid("usage input supply schema");
        };
        let SchemaLookup::Known(grant) = definitions.slot(&input.grant) else {
            return invalid("usage input grant schema");
        };
        charge(&mut compiled.work, grant.provider_roles.len(), limits)?;
        if supply.skill != input.primary
            || grant.target != GrantTarget::Skill(input.supply.clone())
            || !grant.provider_roles.contains(&ProviderRole::SkillUse)
        {
            return invalid("usage input primary grant target");
        }
        let SchemaLookup::Known(usage) = definitions.definition(&input.policy) else {
            return invalid("usage input policy schema");
        };
        charge(
            &mut compiled.work,
            usage.targets.len() + usage.declarations.parameters.members.len(),
            limits,
        )?;
        if !usage.targets.contains(&UsageTargetKind::Skill)
            || !usage.declarations.parameters.is_complete()
            || usage.declarations.parameters.members.len() != input.parameters.len()
        {
            return invalid("usage input policy target or parameters");
        }
        let declared: BTreeSet<_> = usage
            .declarations
            .parameters
            .members
            .iter()
            .cloned()
            .collect();
        let attributes: BTreeSet<_> = input.attributes.iter().map(String::as_str).collect();
        if attributes.len() != input.attributes.len()
            || attributes
                .iter()
                .any(|value| value.is_empty() || value.len() > 128)
            || ["gemId", "variantId", "skillId", "nameSpec"]
                .iter()
                .any(|value| !attributes.contains(value))
        {
            return invalid("usage input attribute frame");
        }
        let mut guards = BTreeSet::new();
        for guard in &input.guards {
            if !attributes.contains(guard.attribute.as_str()) || !guards.insert(&guard.attribute)
                || guard.allowed.is_empty() || guard.allowed.len() > 64
                || guard.allowed.iter().collect::<BTreeSet<_>>().len() != guard.allowed.len()
                || guard.allowed.iter().any(|value| matches!(value, SourceComponent::Text(text) if text.len() > limits.mapping.max_string_bytes))
            {
                return invalid("usage input guards");
            }
        }
        let mut slots = BTreeSet::new();
        let mut parameters = Vec::new();
        for parameter in &input.parameters {
            if parameter.slot.declaration != SlotOwnerDefId::UsagePolicy(input.policy.clone())
                || !declared.contains(&parameter.slot)
                || !slots.insert(parameter.slot.clone())
            {
                return invalid("usage input parameter owner");
            }
            let SchemaLookup::Known(schema) = definitions.slot(&parameter.slot) else {
                return invalid("usage input parameter schema");
            };
            charge(&mut compiled.work, schema.sites.len(), limits)?;
            if schema.value != ValueSchema::Boolean
                || schema.presence != SlotPresence::RequiredOnce
                || !schema.sites.contains(&ParameterSite::UsagePolicyParameter)
                || parameter.value.codec.namespace != *definitions.namespace()
                || !matches!(parameter.value.codec.codec, ValueCodecKind::Boolean { .. })
                || !matches!(parameter.value.missing, MissingValuePolicy::Pending)
                || !parameter.value.numeric_aliases.is_empty()
                || parameter
                    .value
                    .tiers
                    .iter()
                    .flat_map(|tier| &tier.selectors)
                    .any(|selector| {
                        selector.lane != ValueLane::Attribute
                            || !attributes.contains(selector.name.as_str())
                    })
            {
                return invalid("usage input Boolean source recipe");
            }
            parameters.push((
                parameter.slot.clone(),
                ValueRecipe::new(parameter.value.clone(), limits.value)?,
            ));
        }
        if slots != declared {
            return invalid("usage input parameter coverage");
        }
        compiled.rules.insert(
            &input.gem,
            BoundUsage {
                input,
                attributes: input.attributes.iter().map(String::as_str).collect(),
                parameters,
            },
        );
    }
    Ok(Some(compiled))
}

impl<'p> CompiledUsageInputs<'p> {
    pub(super) fn attach(
        &self,
        b: &mut Builder<'_, '_>,
        context: UsageInputContext<'_, '_>,
    ) -> Result<Option<AttachedPrimaryUsage<'p>>> {
        let UsageInputContext {
            row,
            group,
            gem,
            skill,
            preset,
            inventory_proof,
        } = context;
        b.charge(1)?;
        let DraftField::Known { value: gem } = gem else {
            return Ok(None);
        };
        let Some(bound) = self.rules.get(gem) else {
            return Ok(None);
        };
        let source = row.occurrence().id();
        // A participating row establishes an unresolved preset inventory even
        // when its finite value proof fails. Never confuse no record with none.
        if preset.usage_preferences.is_none() {
            preset.usage_preferences =
                Some(b.closure(source, "usage-preferences-not-converted", vec![])?);
        }
        source_shape::charge_row(b, row)?;
        if row.occurrence().name() != "Gem"
            || group.occurrence().name() != "Skill"
            || row.occurrence().parent() != Some(group.occurrence().id())
            || !source_shape::plain_row(row, &bound.attributes, true)
            || !b.gem_guards_match(row, &bound.input.guards)?
        {
            return Ok(None);
        }
        for (name, expected) in [
            ("gemId", &bound.input.game_id),
            ("variantId", &bound.input.variant_id),
            ("skillId", &bound.input.skill_id),
            ("nameSpec", &bound.input.name_spec),
        ] {
            if source_shape::value(row, name) != Some(expected.as_str()) {
                return Ok(None);
            }
        }
        let mut parameters = Vec::new();
        let mut converted = true;
        for (slot, recipe) in &bound.parameters {
            b.charge(1)?;
            for selector in recipe.input().tiers.iter().flat_map(|tier| &tier.selectors) {
                b.charge(row.attributes().len())?;
                if let Some(value) = row.attribute(&selector.name) {
                    b.charge(value.raw().len())?;
                }
            }
            match b.scalar_value(row, recipe) {
                Ok(ScalarValue::Selected(value @ ParameterValue::Boolean(_))) => {
                    parameters.push(
                        ParameterAssignment {
                            slot: slot.clone(),
                            value,
                        }
                        .into(),
                    );
                }
                Ok(_) | Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {
                    converted = false
                }
                Err(error) => return Err(error),
            }
        }
        let parameters = if converted {
            complete(parameters)
        } else {
            b.closure(source, "usage-parameters-not-converted", parameters)?
        };
        preset
            .usage_preferences
            .as_mut()
            .expect("initialized above")
            .members
            .push(UsagePolicyDraft {
                policy: bound.input.policy.clone().into(),
                target: UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(skill),
                        grant_path: vec![],
                    },
                    slot: bound.input.supply.clone(),
                })))
                .into(),
                parameters,
            });
        // The preset already has its source-set origin. This exact Gem row also
        // contributed its preference, independently of the Gem/Skill links.
        b.link(source, OwnedOriginTarget::SkillPreset(preset.id))?;
        if !inventory_proof || !converted {
            return Ok(None);
        }
        b.charge(preset.skills.members.len())?;
        if !preset.skills.members.contains(&skill)
            || !matches!(&preset.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Pending { code, .. } if code.as_str() == "usage-preferences-not-converted")
        {
            return Ok(None);
        }
        let Some(set) = b.ancestor(source, "SkillSet")? else {
            return Ok(None);
        };
        b.charge(b.origins[set.ordinal() as usize].links.len())?;
        if !b.origins[set.ordinal() as usize]
            .links
            .contains(&OwnedOriginTarget::SkillPreset(preset.id))
        {
            return Ok(None);
        }
        Ok(Some(AttachedPrimaryUsage {
            input: bound.input,
            source,
        }))
    }
}
