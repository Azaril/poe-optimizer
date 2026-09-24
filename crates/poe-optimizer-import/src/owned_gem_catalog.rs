//! Offline compilation of explicitly reviewed physical-Gem schema/input facts.
//! Source catalog identities are joined once; no source data enters evaluation.
use crate::{
    owned_gem_schema::{
        GemSchemaMigrationInput, GemSchemaMigrationLimits, StagedGemSchemaMigration,
        stage_owned_gem_schema,
    },
    owned_mapping::*,
    owned_normalize::{
        GemInputGuard, GemInputPolicy, GemInputRule, GemParameterInput, NormalizationError,
        NormalizationLimits, validate_gem_input_policy,
    },
    owned_recipe::{OwnedRecipeError, StagedOwnedRecipe, assemble_owned_recipe},
    owned_skill_catalog::{
        AbsentFromTreePolicy, AbsentSupportPolicy, OwnedGemMaterialization, OwnedGemRole,
        OwnedPrimarySkill, OwnedSkillRoleIndex,
    },
    owned_successor::SuccessorBundleError,
    owned_value_policy::{MissingValuePolicy, ValueRecipeInput},
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalGemParameterPolicy {
    pub schema: ParameterSlotSchema,
    pub value: ValueRecipeInput,
}
/// Finite reviewed scope and shared schema/lexical policy. Source keys select
/// existing identities; names, level domains, units and tokens are never inferred.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalGemSchemaPolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub catalog_digest: OwnedContentDigest,
    pub source_gems: Vec<String>,
    pub level: IntegerRange,
    pub quality_presence: QualityPresence,
    pub quality_kinds: Vec<QualityDefId>,
    pub guards: Vec<GemInputGuard>,
    pub parameters: Vec<PhysicalGemParameterPolicy>,
}
#[derive(Clone, Copy, Debug)]
pub struct PhysicalGemCatalogLimits {
    pub max_wire_bytes: usize,
    pub max_source_gems: usize,
    pub max_parameters: usize,
    pub migration: GemSchemaMigrationLimits,
}
impl Default for PhysicalGemCatalogLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 16 * 1024 * 1024,
            max_source_gems: 4096,
            max_parameters: 64,
            migration: Default::default(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PhysicalGemCatalogReceipt {
    pub policy: OwnedContentDigest,
    pub policy_version: OwnedDefinitionKey,
    pub catalog: OwnedContentDigest,
    pub source: SourcePin,
    pub before: DataIdentity,
    pub after: DataIdentity,
    pub promoted_gems: usize,
    pub allocated_parameters: usize,
}
pub struct CompiledOwnedGemCatalog {
    pub migration: GemSchemaMigrationInput,
    pub staged: StagedGemSchemaMigration,
    /// Rules for the successor schema, to merge with explicitly rebound prior rules.
    pub inputs: Vec<GemInputRule>,
    pub receipt: PhysicalGemCatalogReceipt,
}
#[derive(Debug, thiserror::Error)]
pub enum PhysicalGemCatalogError {
    #[error("physical Gem catalog: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Migration(#[from] SuccessorBundleError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Normalization(#[from] NormalizationError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, PhysicalGemCatalogError>;
fn invalid(message: &'static str) -> PhysicalGemCatalogError {
    PhysicalGemCatalogError::Invalid(message)
}
fn partial<T>(gem: &GemDefId, members: Vec<T>) -> DeclaredSet<T> {
    DeclaredSet::partial(
        members,
        vec![SchemaGap {
            subject: SchemaSubject::Definition(gem.address()),
            facet: SchemaFacet::InputSchema,
            code: OwnedDefinitionKey::new("physical-gem-input-review-incomplete")
                .expect("compiler symbol"),
        }],
    )
}
fn gem_selector(game: &str, variant: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(game.into()),
        variant_id: SourceComponent::Text(variant.into()),
    })
}

/// Prepare one checked V4 migration and its typed input rules. This function
/// does not publish, close input coverage, change prior rules or claim mechanics.
/// Hosts must bound wire bytes before deserializing the catalog and policy.
pub fn compile_owned_gem_catalog(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    catalog: &SkillIdentityCatalog,
    policy: &PhysicalGemSchemaPolicy,
    limits: PhysicalGemCatalogLimits,
) -> Result<CompiledOwnedGemCatalog> {
    let hard = PhysicalGemCatalogLimits::default();
    if limits.max_wire_bytes == 0
        || limits.max_wire_bytes > hard.max_wire_bytes
        || limits.max_source_gems == 0
        || limits.max_source_gems > hard.max_source_gems
        || limits.max_parameters == 0
        || limits.max_parameters > hard.max_parameters
        || policy.schema_version != 1
        || policy.source_gems.is_empty()
        || policy.source_gems.len() > limits.max_source_gems
        || policy.parameters.is_empty()
        || policy.parameters.len() > limits.max_parameters
        || policy.guards.len() > 64
        || policy.quality_kinds.len() > 64
    {
        return Err(invalid("invalid limits, version or policy cardinality"));
    }
    let entries = policy
        .source_gems
        .len()
        .checked_mul(policy.parameters.len() + 1)
        .ok_or_else(|| invalid("migration expansion overflow"))?;
    if entries > limits.migration.max_entries {
        return Err(invalid("migration expansion exceeds entry limit"));
    }
    let policy_digest = digest_owned(
        "owned-physical-gem-schema-policy-v1",
        policy,
        limits.max_wire_bytes,
    )?;
    // Bound template fan-out before cloning it for each owner. The input digest
    // already bounded serialization, so this one template allocation is bounded.
    // The allowance covers owned IDs, slot references and per-owner Partial gaps;
    // stage_owned_gem_schema also enforces the exact final migration wire bound.
    let template_bytes = serde_json::to_vec(&(
        &policy.level,
        &policy.quality_kinds,
        &policy.guards,
        &policy.parameters,
    ))
    .map_err(|_| invalid("policy template serialization"))?
    .len();
    let per_owner_bytes = policy
        .parameters
        .len()
        .checked_mul(2048)
        .and_then(|bytes| bytes.checked_add(8192))
        .and_then(|bytes| bytes.checked_add(template_bytes))
        .ok_or_else(|| invalid("template expansion overflow"))?;
    let expanded_bytes = per_owner_bytes
        .checked_mul(policy.source_gems.len())
        .ok_or_else(|| invalid("template expansion overflow"))?;
    if expanded_bytes > limits.max_wire_bytes.min(limits.migration.max_wire_bytes) {
        return Err(invalid("template expansion exceeds byte budget"));
    }
    let data = catalog.data();
    let catalog_digest =
        digest_owned("owned-skill-source-catalog-v1", data, limits.max_wire_bytes)?;
    if catalog_digest != policy.catalog_digest
        || catalog_digest != roles.input().compilation.catalog_digest
        || mapping.input().source.system != ExternalSourceSystem::PathOfBuilding2
        || mapping.input().source.revision != data.source.upstream_revision
        || mapping.input().definitions != *base.schema().identity()
        || mapping.input().registry != base.registry().identity()?
        || roles.input().definitions != *base.schema().identity()
        || roles.input().mapping != *mapping.identity()
    {
        return Err(invalid("stale catalog or prior artifact binding"));
    }
    let pins: BTreeMap<_, _> = mapping
        .input()
        .source
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect();
    if data
        .source
        .files
        .iter()
        .any(|(path, hash)| pins.get(path.as_str()).copied() != Some(hash.as_str()))
    {
        return Err(invalid(
            "catalog source is not an immutable matching subset",
        ));
    }
    if policy
        .parameters
        .iter()
        .any(|parameter| parameter.value.missing != MissingValuePolicy::Pending)
    {
        return Err(invalid(
            "physical Gem recipes must retain missing input as Pending",
        ));
    }
    if policy.quality_kinds.iter().collect::<BTreeSet<_>>().len() != policy.quality_kinds.len()
        || policy
            .quality_kinds
            .iter()
            .any(|id| !matches!(base.schema().definition(id), SchemaLookup::Known(_)))
    {
        return Err(invalid(
            "quality membership must contain distinct Known definitions",
        ));
    }
    // Index every source selector before selected joins, so a duplicate elsewhere
    // in the catalog cannot lend its identity to a selected row.
    let mut selectors = BTreeMap::new();
    for gem in &data.gems {
        let count = selectors
            .entry((gem.game_id.as_str(), gem.variant_id.as_str()))
            .or_insert(0usize);
        *count += 1;
    }
    let mut source_keys = BTreeSet::new();
    let mut selected = BTreeMap::new();
    for key in &policy.source_gems {
        if key.is_empty() || key.len() > 16 * 1024 || !source_keys.insert(key.as_str()) {
            return Err(invalid("empty, oversized or duplicate source key"));
        }
        let source = catalog
            .gem_by_key(key)
            .ok_or_else(|| invalid("selected source Gem is absent"))?;
        if selectors.get(&(source.game_id.as_str(), source.variant_id.as_str())) != Some(&1)
            || !source.declared_additional_effects.is_empty()
            || !source.constructed_additional_effects.is_empty()
            || !source.additional_effects.is_empty()
            || source.effect_list.len() != 1
            || source.effect_list.first() != Some(&source.primary_effect_id)
        {
            return Err(invalid(
                "source Gem has ambiguous identity or unreviewed potential effects",
            ));
        }
        let selector = gem_selector(&source.game_id, &source.variant_id);
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            basis: MappingBasis::Exact,
        }) = mapping.lookup(&selector)
        else {
            return Err(invalid("selected Gem has no exact unique mapping"));
        };
        if !matches!(base.schema().definition(gem), SchemaLookup::Unmapped(_)) {
            return Err(invalid("selected Gem must have an Unmapped schema"));
        }
        let row = roles
            .role(gem)
            .ok_or_else(|| invalid("selected Gem has no role"))?;
        let (
            OwnedGemMaterialization::Physical,
            OwnedGemRole::Known(role),
            OwnedPrimarySkill::Known(primary),
        ) = (&row.materialization, &row.role, &row.primary)
        else {
            return Err(invalid("selected Gem lacks physical/role/primary evidence"));
        };
        let skill = catalog
            .skill_by_id(&source.primary_effect_id)
            .ok_or_else(|| invalid("primary source effect is absent"))?;
        let expected_role = match (
            skill.support,
            roles.input().compilation.policy.absent_support,
        ) {
            (Some(true), _) => AuthoredGemRole::SupportAssignment,
            (Some(false), _) | (None, AbsentSupportPolicy::NonSupport) => AuthoredGemRole::SkillUse,
            _ => return Err(invalid("source primary role is unresolved")),
        };
        if *role != expected_role
            || !matches!(
                (
                    skill.from_tree,
                    roles.input().compilation.policy.absent_from_tree
                ),
                (Some(false), _) | (None, AbsentFromTreePolicy::Physical)
            )
        {
            return Err(invalid("source primary contradicts physical role evidence"));
        }
        let skill_selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(source.primary_effect_id.clone()),
        });
        if !matches!(mapping.lookup(&skill_selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(id)), basis: MappingBasis::Exact,
        }) if id == primary)
        {
            return Err(invalid("source primary does not match owned role mapping"));
        }
        if selected
            .insert(gem.clone(), (*role, primary.clone()))
            .is_some()
        {
            return Err(invalid("selected source Gems collide on an owned identity"));
        }
    }
    let mut registry = base.registry().clone();
    let mut gems = Vec::with_capacity(selected.len());
    let mut parameters = Vec::with_capacity(entries - selected.len());
    let mut inputs = Vec::with_capacity(selected.len());
    for (gem, (role, primary)) in selected {
        let mut slots = Vec::with_capacity(policy.parameters.len());
        let mut values = Vec::with_capacity(policy.parameters.len());
        for parameter in &policy.parameters {
            let slot = registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))?;
            slots.push(slot.clone());
            parameters.push(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(parameter.schema.clone()),
            });
            values.push(GemParameterInput {
                slot,
                value: parameter.value.clone(),
            });
        }
        gems.push(DefinitionEntry {
            id: gem.clone(),
            schema: SchemaState::Known(GemSchema {
                level: policy.level.clone(),
                roles: vec![role],
                skills: partial(&gem, vec![primary]),
                quality: QualityUseSchema {
                    presence: policy.quality_presence,
                    allowed_kinds: partial(&gem, policy.quality_kinds.clone()),
                },
                declarations: DeclaredSlots {
                    parameters: partial(&gem, slots),
                    choices: partial(&gem, vec![]),
                    grants: partial(&gem, vec![]),
                    actors: partial(&gem, vec![]),
                    skill_grants: partial(&gem, vec![]),
                    outputs: partial(&gem, vec![]),
                    sockets: partial(&gem, vec![]),
                },
            }),
        });
        inputs.push(GemInputRule {
            gem,
            guards: policy.guards.clone(),
            parameters: values,
        });
    }
    let migration = GemSchemaMigrationInput {
        schema_version: 1,
        before: base.schema().identity().clone(),
        mapping: *mapping.identity(),
        roles: *roles.identity(),
        source: mapping.input().source.clone(),
        gems,
        parameters,
    };
    let staged = stage_owned_gem_schema(base, mapping, roles, &migration, limits.migration)?;
    let after = assemble_owned_recipe(staged.successor.clone(), limits.migration.recipe)?;
    validate_gem_input_policy(
        &GemInputPolicy {
            definitions: after.schema().identity().clone(),
            gems: inputs.clone(),
        },
        after.schema(),
        NormalizationLimits::default(),
    )?;
    let receipt = PhysicalGemCatalogReceipt {
        policy: policy_digest,
        policy_version: policy.version.clone(),
        catalog: catalog_digest,
        source: mapping.input().source.clone(),
        before: staged.receipt.before.clone(),
        after: staged.receipt.after.clone(),
        promoted_gems: staged.receipt.promoted_gems,
        allocated_parameters: staged.receipt.allocated_parameters,
    };
    Ok(CompiledOwnedGemCatalog {
        migration,
        staged,
        inputs,
        receipt,
    })
}
