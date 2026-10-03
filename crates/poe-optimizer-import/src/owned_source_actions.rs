//! Source reference settings resolved into the existing owned query vocabulary.
//!
//! This adapter neither materializes a SkillUse nor proves activity, gameplay
//! usage, numerical coverage or a complete input inventory. Its returned target
//! still passes through ordinary normalization and Core binding. MAIN and CALCS
//! exist only here, as names of independent saved source reference settings.
mod minion;
mod resolve;

use crate::{
    owned_mapping::*,
    owned_normalize::{
        ImportActionTarget, ImportActorTarget, ImportProviderTarget, ImportQueryTarget,
        ImportSkillUseLocator,
    },
    owned_skill_catalog::*,
    owned_source::{SourceAttributeRef, SourceProjectEvidence},
    owned_value::{ValueCodecKind, WhitespacePolicy},
    owned_value_policy::*,
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceActionCorrespondenceInput {
    PobPhysicalPrimaryStatSetsV1 {
        definitions: DataIdentity,
        source: SourcePin,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        gem: GemDefId,
        game_id: String,
        variant_id: String,
        skill_id: String,
        name_spec: String,
        primary: SkillDefId,
        primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
        entering_grant: DeclaredSlot<GrantSlotDefId>,
        output: DeclaredSlot<ActionOutputDefId>,
        part: ActionPartDefId,
        mode: ActionModeDefId,
        stat_sets: Vec<SourceStatSetMapping>,
        /// Explicit source-bound absence behavior, never a malformed-token default.
        absent_stat_set: Option<ActionStatSetDefId>,
        index: ValueRecipeInput,
    },
    PobPhysicalSingletonMinionActionsV1 {
        definitions: DataIdentity,
        source: SourcePin,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        gem: GemDefId,
        game_id: String,
        variant_id: String,
        skill_id: String,
        name_spec: String,
        primary: SkillDefId,
        primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
        entering_grant: DeclaredSlot<GrantSlotDefId>,
        minion: SourceSingletonMinion,
        actions: Vec<SourceMinionActionMapping>,
        absent_action: Option<u32>,
        main_action_index: Box<ValueRecipeInput>,
        calcs_action_index: Box<ValueRecipeInput>,
        map_skill_index: Box<ValueRecipeInput>,
        map_stat_set_index: ValueRecipeInput,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSingletonMinion {
    pub source_id: String,
    pub allow_absent: bool,
    pub actor: ActorDefId,
    pub population: DeclaredSlot<ActorSlotDefId>,
    pub entering_grant: DeclaredSlot<GrantSlotDefId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMinionActionMapping {
    pub source_index: u32,
    pub skill_id: String,
    pub skill: SkillDefId,
    pub supply: DeclaredSlot<SkillGrantSlotDefId>,
    pub entering_grant: DeclaredSlot<GrantSlotDefId>,
    pub output: DeclaredSlot<ActionOutputDefId>,
    pub part: ActionPartDefId,
    pub mode: ActionModeDefId,
    pub stat_sets: Vec<SourceStatSetMapping>,
    pub absent_stat_set: Option<ActionStatSetDefId>,
}

struct PhysicalFields<'a> {
    definitions: &'a DataIdentity,
    source: &'a SourcePin,
    roles: &'a OwnedContentDigest,
    catalog: &'a OwnedContentDigest,
    gem: &'a GemDefId,
    game_id: &'a str,
    variant_id: &'a str,
    skill_id: &'a str,
    name_spec: &'a str,
    primary: &'a SkillDefId,
    primary_supply: &'a DeclaredSlot<SkillGrantSlotDefId>,
    entering_grant: &'a DeclaredSlot<GrantSlotDefId>,
}
impl SourceActionCorrespondenceInput {
    fn physical(&self) -> PhysicalFields<'_> {
        match self {
            Self::PobPhysicalPrimaryStatSetsV1 {
                definitions,
                source,
                roles,
                catalog,
                gem,
                game_id,
                variant_id,
                skill_id,
                name_spec,
                primary,
                primary_supply,
                entering_grant,
                ..
            }
            | Self::PobPhysicalSingletonMinionActionsV1 {
                definitions,
                source,
                roles,
                catalog,
                gem,
                game_id,
                variant_id,
                skill_id,
                name_spec,
                primary,
                primary_supply,
                entering_grant,
                ..
            } => PhysicalFields {
                definitions,
                source,
                roles,
                catalog,
                gem,
                game_id,
                variant_id,
                skill_id,
                name_spec,
                primary,
                primary_supply,
                entering_grant,
            },
        }
    }
    pub(crate) fn matches_physical(
        &self,
        gem: &GemDefId,
        game_id: &str,
        variant_id: &str,
        skill_id: &str,
        name_spec: &str,
    ) -> bool {
        let fields = self.physical();
        fields.gem == gem
            && fields.game_id == game_id
            && fields.variant_id == variant_id
            && fields.skill_id == skill_id
            && fields.name_spec == name_spec
    }
    pub(crate) fn is_minion(&self) -> bool {
        matches!(self, Self::PobPhysicalSingletonMinionActionsV1 { .. })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceStatSetMapping {
    pub source_index: u32,
    pub stat_set: ActionStatSetDefId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportReferenceContext {
    Main,
    Calcs,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceActionRequest {
    pub skill_use: ImportSkillUseLocator,
    pub context: ImportReferenceContext,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceActionSelection {
    Explicit {
        source_index: u32,
        attribute: SourceAttributeRef,
    },
    Absent,
    Pending {
        code: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceActionReport {
    pub schema_version: u32,
    pub definitions: DataIdentity,
    pub correspondence: OwnedContentDigest,
    pub request: SourceActionRequest,
    pub source_sha256: String,
    pub selection: SourceActionSelection,
    pub target: ImportQueryTarget,
    /// The original loader overwrites these scalar headers with fresh maps.
    pub ignored_legacy_attributes: Vec<SourceAttributeRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minion: Option<SourceMinionActionReport>,
    /// Checked construction plus this resolution's bounded work, without refunds.
    pub work: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceMinionActionReport {
    pub actor_attributes: Vec<SourceAttributeRef>,
    pub action_selection: SourceActionSelection,
    /// All checked nested map containers and entries, never unrelated fields.
    pub accounted_occurrences: Vec<crate::build_instance::SourceOccurrenceId>,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceActionLimits {
    pub max_wire_bytes: usize,
    pub max_output_bytes: usize,
    pub max_work: usize,
    pub max_map_rows: usize,
    pub max_stat_sets: usize,
    pub value: ValuePolicyLimits,
}
impl Default for SourceActionLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 2 * 1024 * 1024,
            max_output_bytes: 2 * 1024 * 1024,
            max_work: 1_000_000,
            max_map_rows: 4096,
            max_stat_sets: 64,
            value: ValuePolicyLimits::default(),
        }
    }
}
impl SourceActionLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("wire bytes", self.max_wire_bytes, hard.max_wire_bytes),
            ("output bytes", self.max_output_bytes, hard.max_output_bytes),
            ("work", self.max_work, hard.max_work),
            ("map rows", self.max_map_rows, hard.max_map_rows),
            ("stat sets", self.max_stat_sets, hard.max_stat_sets),
        ] {
            if value == 0 || value > maximum {
                return Err(SourceActionError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SourceActionError {
    #[error("invalid source action {0} limit")]
    InvalidLimit(&'static str),
    #[error("source action exceeds {0} limit")]
    Limit(&'static str),
    #[error("source action correspondence dependency binding mismatch")]
    Binding,
    #[error("invalid source action correspondence: {0}")]
    Policy(&'static str),
    #[error(transparent)]
    Value(#[from] ValuePolicyError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, SourceActionError>;

/// Immutable checked correspondence. The owned data package retains no source
/// strings, MAIN/CALCS mode or executable source dependency from this adapter.
pub struct SourceActionCorrespondence {
    input: SourceActionCorrespondenceInput,
    identity: OwnedContentDigest,
    compiled: CompiledSourceActions,
    limits: SourceActionLimits,
    work: usize,
}
enum CompiledSourceActions {
    Primary {
        recipe: Box<ValueRecipe>,
        stat_sets: BTreeMap<u32, ActionStatSetDefId>,
    },
    Minion(Box<minion::CompiledMinion>),
}
struct Budget {
    used: usize,
    maximum: usize,
}
impl Budget {
    fn charge(&mut self, amount: usize) -> Result<()> {
        self.used = self
            .used
            .checked_add(amount)
            .filter(|used| *used <= self.maximum)
            .ok_or(SourceActionError::Limit("work"))?;
        Ok(())
    }
    fn members<T: PartialEq>(&mut self, members: &[T], value: &T) -> Result<bool> {
        self.charge(members.len())?;
        Ok(members.contains(value))
    }
}
fn require(condition: bool, reason: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(SourceActionError::Policy(reason))
    }
}
fn known<'a, T>(lookup: SchemaLookup<'a, T>, reason: &'static str) -> Result<&'a T> {
    match lookup {
        SchemaLookup::Known(value) => Ok(value),
        _ => Err(SourceActionError::Policy(reason)),
    }
}

impl SourceActionCorrespondence {
    pub fn new<I: DefinitionSchemaIndex>(
        input: SourceActionCorrespondenceInput,
        definitions: &I,
        roles: &OwnedSkillRoleIndex,
        mappings: &OwnedMappingIndex,
        limits: SourceActionLimits,
    ) -> Result<Self> {
        limits.validate()?;
        // Bounded serialization precedes any input cloning or full-buffer encoding.
        let identity = digest_owned(
            "owned-source-action-correspondence-v1",
            &input,
            limits.max_wire_bytes,
        )?;
        let bytes =
            serde_json::to_vec(&input).map_err(|_| SourceActionError::Policy("encoding"))?;
        let mut budget = Budget {
            used: 0,
            maximum: limits.max_work,
        };
        budget.charge(bytes.len())?;
        let PhysicalFields {
            definitions: bound,
            source,
            roles: role_digest,
            catalog,
            gem,
            game_id,
            variant_id,
            skill_id,
            name_spec,
            primary,
            primary_supply,
            entering_grant,
        } = input.physical();
        // The shared immutable-pin subset check compares each required pin with
        // the combined mapping manifest; account for that bounded search too.
        budget.charge(
            source
                .files
                .len()
                .saturating_mul(mappings.input().source.files.len()),
        )?;
        if bound != definitions.identity()
            || bound != &roles.input().definitions
            || bound != &mappings.input().definitions
            || roles.input().mapping != *mappings.identity()
            || role_digest != roles.identity()
            || catalog != &roles.input().compilation.catalog_digest
            || source != &roles.input().compilation.source
            || !provenance_is_subset(source, &mappings.input().source)
            || source.system != ExternalSourceSystem::PathOfBuilding2
        {
            return Err(SourceActionError::Binding);
        }
        require(
            [game_id, variant_id, skill_id, name_spec]
                .iter()
                .all(|s| !s.is_empty() && s.len() <= 16 * 1024),
            "source identity strings",
        )?;
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(game_id.into()),
            variant_id: SourceComponent::Text(variant_id.into()),
        });
        require(
            matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(id)), basis: MappingBasis::Exact,
        }) if id == gem),
            "exact Gem mapping",
        )?;
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(skill_id.into()),
        });
        require(
            matches!(mappings.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(id)), basis: MappingBasis::Exact,
        }) if id == primary),
            "exact primary effect mapping",
        )?;
        require(
            roles.role(gem).is_some_and(|row| {
                row.materialization == OwnedGemMaterialization::Physical
                    && row.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse)
                    && row.primary == OwnedPrimarySkill::Known(primary.clone())
            }),
            "physical primary role",
        )?;
        let gem_schema = known(definitions.definition(gem), "Gem schema")?;
        let skill_schema = known(definitions.definition(primary), "Skill schema")?;
        if input.is_minion() {
            let compiled = minion::compile(&input, definitions, mappings, limits, &mut budget)?;
            return Ok(Self {
                input,
                identity,
                compiled: CompiledSourceActions::Minion(Box::new(compiled)),
                limits,
                work: budget.used,
            });
        }
        let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            output,
            part,
            mode,
            stat_sets,
            absent_stat_set,
            index,
            ..
        } = &input
        else {
            unreachable!()
        };
        require(
            budget.members(&gem_schema.skills.members, primary)?
                && budget.members(&gem_schema.roles, &AuthoredGemRole::SkillUse)?
                && primary_supply.declaration == SlotOwnerDefId::Gem(gem.clone())
                && entering_grant.declaration == SlotOwnerDefId::Gem(gem.clone())
                && output.declaration == SlotOwnerDefId::Skill(primary.clone())
                && budget.members(
                    &gem_schema.declarations.skill_grants.members,
                    primary_supply,
                )?
                && budget.members(&gem_schema.declarations.grants.members, entering_grant)?
                && budget.members(&skill_schema.declarations.outputs.members, output)?,
            "declared primary topology",
        )?;
        let supply = known(definitions.slot(primary_supply), "primary supply schema")?;
        let grant = known(definitions.slot(entering_grant), "entering grant schema")?;
        let action = known(definitions.slot(output), "action output schema")?;
        require(
            supply.skill == *primary
                && budget.members(&supply.outputs.members, output)?
                && grant.target == GrantTarget::Skill(primary_supply.clone())
                && budget.members(&grant.provider_roles, &ProviderRole::SkillUse)?
                && action.actor_role == DeclaredActorRole::Player
                && budget.members(&action.parts.members, part)?
                && budget.members(&action.modes.members, mode)?,
            "primary action correspondence",
        )?;
        known(definitions.definition(part), "action part schema")?;
        known(definitions.definition(mode), "action mode schema")?;
        require(
            !stat_sets.is_empty() && stat_sets.len() <= limits.max_stat_sets,
            "stat set rows",
        )?;
        let mut mapped = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for row in stat_sets {
            budget.charge(1)?;
            require(
                row.source_index > 0
                    && mapped
                        .insert(row.source_index, row.stat_set.clone())
                        .is_none()
                    && ids.insert(&row.stat_set)
                    && budget.members(&action.stat_sets.members, &row.stat_set)?,
                "stat set correspondence",
            )?;
            known(
                definitions.definition(&row.stat_set),
                "action stat set schema",
            )?;
        }
        require(
            absent_stat_set.as_ref().is_none_or(|id| ids.contains(id)),
            "absence stat set correspondence",
        )?;
        require(
            index.codec.namespace == *definitions.namespace()
                && index.codec.whitespace == WhitespacePolicy::Exact
                && matches!(index.codec.codec, ValueCodecKind::Integer { .. })
                && index.missing == MissingValuePolicy::Pending
                && index.numeric_aliases.is_empty()
                && index.tiers.len() == 1
                && index.tiers[0].duplicates == DuplicatePolicy::Reject
                && index.tiers[0].selectors
                    == [ValueSelector {
                        lane: ValueLane::Attribute,
                        name: "index".into(),
                    }],
            "index recipe",
        )?;
        let recipe = ValueRecipe::new(index.clone(), limits.value)?;
        Ok(Self {
            input,
            identity,
            compiled: CompiledSourceActions::Primary {
                recipe: Box::new(recipe),
                stat_sets: mapped,
            },
            limits,
            work: budget.used,
        })
    }
    pub fn input(&self) -> &SourceActionCorrespondenceInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub(crate) fn construction_work(&self) -> usize {
        self.work
    }
    pub fn resolve(
        &self,
        evidence: &SourceProjectEvidence<'_>,
        request: &SourceActionRequest,
    ) -> Result<SourceActionReport> {
        resolve::resolve(self, evidence, request)
    }
}

/// Rebind only an already checked inherited adapter. Source/catalog facts remain
/// committed; changed acquisition evidence requires explicit new authoring.
pub(crate) fn rebind_input(
    input: &mut SourceActionCorrespondenceInput,
    definitions: &DataIdentity,
    roles: &OwnedSkillRoleIndex,
) {
    let (bound, role_digest) = match input {
        SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            definitions: bound,
            roles: role_digest,
            ..
        }
        | SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
            definitions: bound,
            roles: role_digest,
            ..
        } => (bound, role_digest),
    };
    *bound = definitions.clone();
    *role_digest = *roles.identity();
}
