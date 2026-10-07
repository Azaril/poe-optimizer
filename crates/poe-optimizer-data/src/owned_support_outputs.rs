//! Bounded final type channel bindings. Storage does not establish runtime
//! preparation-context uniqueness, activation, or complete numeric coverage.
use crate::{
    owned_rules::OwnedRulePackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_core::{
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::{PlayerEquipmentSlotRead, RuleEffectKind, RuleEntity, RuleReadSource},
    owned_schema::*,
    owned_stages::StageChannel,
    owned_support_outputs::*,
};
use serde::Serialize;

const DOMAIN: &str = "owned-support-output-bindings-v1";

/// Exact immutable packages used to validate an output-binding artifact.
pub struct SupportOutputDependencies<'a, I> {
    pub definitions: &'a I,
    pub rules: &'a OwnedRulePackage,
    pub preparation: &'a OwnedSupportPreparation,
    pub inputs: &'a OwnedSupportInputBindings,
    pub receiving: &'a OwnedSupportReceiving,
    pub stages: &'a OwnedEvaluationStages,
}

#[derive(Clone, Copy, Debug)]
pub struct SupportOutputStorageLimits {
    pub max_entries: usize,
    pub max_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for SupportOutputStorageLimits {
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_work: 2_000_000,
            max_wire_bytes: 16 * 1024 * 1024,
        }
    }
}
impl SupportOutputStorageLimits {
    pub fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("entries", self.max_entries, hard.max_entries),
            ("work", self.max_work, hard.max_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(SupportOutputStorageError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupportOutputStorageError {
    #[error("invalid support output storage limit: {0}")]
    InvalidLimit(&'static str),
    #[error("support output bindings exceed {0}")]
    Limit(&'static str),
    #[error("unsupported support output bindings version {0}")]
    Version(u32),
    #[error("support output schema/rule/preparation/input/receiving/stage binding mismatch")]
    Binding,
    #[error("invalid support output bindings: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, SupportOutputStorageError>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SupportOutputStorageUse {
    pub entries: usize,
    pub work: usize,
}
impl SupportOutputStorageUse {
    fn check(self, limits: SupportOutputStorageLimits) -> Result<()> {
        if self.entries > limits.max_entries {
            return Err(SupportOutputStorageError::Limit("entries"));
        }
        if self.work > limits.max_work {
            return Err(SupportOutputStorageError::Limit("work"));
        }
        Ok(())
    }
    fn work(&mut self, n: usize, limits: SupportOutputStorageLimits) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or(SupportOutputStorageError::Limit("work"))?;
        self.check(limits)
    }
    fn entries(&mut self, n: usize, limits: SupportOutputStorageLimits) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(n)
            .ok_or(SupportOutputStorageError::Limit("entries"))?;
        self.work(n, limits)
    }
    fn sort(&mut self, n: usize, limits: SupportOutputStorageLimits) -> Result<()> {
        self.work(
            n.checked_mul(search_work(n))
                .ok_or(SupportOutputStorageError::Limit("work"))?,
            limits,
        )
    }
}
fn search_work(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as usize + 1
}

fn bindings<I: DefinitionSchemaIndex>(
    input: &SupportOutputBindingsInput,
    dependencies: &SupportOutputDependencies<'_, I>,
) -> Result<()> {
    let d = dependencies;
    if input.definitions != *d.definitions.identity()
        || input.namespace != *d.definitions.namespace()
        || input.rules != *d.rules.identity()
        || input.preparation != *d.preparation.identity()
        || input.inputs != *d.inputs.identity()
        || input.receiving != *d.receiving.identity()
        || input.stages != *d.stages.identity()
        || d.receiving
            .verify_bindings(d.definitions, d.rules, d.preparation, d.inputs, d.stages)
            .is_err()
        || input.definitions.validate().is_err()
    {
        return Err(SupportOutputStorageError::Binding);
    }
    Ok(())
}

struct Check<'a> {
    outputs: Vec<&'a StatDefId>,
    stage: &'a OwnedDefinitionKey,
    stages: &'a OwnedEvaluationStages,
    limits: SupportOutputStorageLimits,
    used: SupportOutputStorageUse,
}
impl Check<'_> {
    fn is_output(&mut self, stat: &StatDefId) -> Result<bool> {
        self.used
            .work(search_work(self.outputs.len()), self.limits)?;
        Ok(self.outputs.binary_search(&stat).is_ok())
    }
    fn forbid(&mut self, stat: &StatDefId, reason: &'static str) -> Result<()> {
        if self.is_output(stat)? {
            return Err(SupportOutputStorageError::Invalid(reason));
        }
        Ok(())
    }
    fn consumer(&mut self, owner: &SchemaSubject, program: &OwnedDefinitionKey) -> Result<()> {
        self.used.work(
            search_work(self.stages.input().programs.members.len()),
            self.limits,
        )?;
        let Some(stage) = self.stages.stage_for(owner, program) else {
            return Err(SupportOutputStorageError::Invalid(
                "output consumer has no declared stage",
            ));
        };
        if !self.stages.precedes(self.stage, stage) {
            return Err(SupportOutputStorageError::Invalid(
                "output consumer must follow output stage",
            ));
        }
        Ok(())
    }
    fn rules(&mut self, rules: &OwnedRulePackage) -> Result<()> {
        // Known potential accesses are checked even in Partial owners, under
        // false guards, or in presently unselected programs. Unknown membership
        // remains a separate plan coverage gap; it is not certified here.
        self.used.work(rules.input().owners.len(), self.limits)?;
        for owner in &rules.input().owners {
            self.used.work(owner.programs.members.len(), self.limits)?;
            for program in &owner.programs.members {
                self.used.work(program.reads.len(), self.limits)?;
                self.used.work(program.effects.len(), self.limits)?;
                for read in &program.reads {
                    match &read.source {
                        RuleReadSource::PlayerEquipmentSlot {
                            read: PlayerEquipmentSlotRead::Stat { stat },
                            ..
                        } => self.forbid(stat, "final type output cannot be an equipment stat")?,
                        RuleReadSource::Stat { entity, stat } if self.is_output(stat)? => {
                            let skill =
                                matches!(entity, RuleEntity::Skill | RuleEntity::AssignedSkill)
                                    || (*entity == RuleEntity::Current
                                        && program.context == RuleEntityKind::Skill);
                            if !skill || read.value_type != ComputedValueType::Boolean {
                                return Err(SupportOutputStorageError::Invalid(
                                    "output consumer must read a Boolean Skill stat",
                                ));
                            }
                            self.consumer(&owner.owner, &program.id)?;
                        }
                        RuleReadSource::Contributions { stat, .. } => {
                            self.forbid(stat, "final type output cannot be a contribution channel")?
                        }
                        RuleReadSource::ContributionQuery { query, .. } => {
                            let registry = rules.input().contribution_queries.as_ref().ok_or(
                                SupportOutputStorageError::Invalid(
                                    "missing ordered contribution inventory",
                                ),
                            )?;
                            self.used.work(registry.members.len(), self.limits)?;
                            let query = registry.members.iter().find(|q| &q.id == query).ok_or(
                                SupportOutputStorageError::Invalid(
                                    "unknown ordered contribution query",
                                ),
                            )?;
                            self.forbid(
                                &query.stat,
                                "final type output cannot be a contribution channel",
                            )?;
                        }
                        RuleReadSource::ModifierTransforms { stat, initial } => {
                            self.forbid(stat, "final type output cannot be a transform channel")?;
                            self.forbid(initial, "final type output cannot be a modifier stat")?;
                        }
                        _ => {}
                    }
                }
                for effect in &program.effects {
                    match &effect.effect {
                        RuleEffectKind::Derive { stat, .. }
                        | RuleEffectKind::ProjectActorStat { stat, .. }
                        | RuleEffectKind::Contribute { stat, .. } => {
                            self.forbid(stat, "output stat has an ordinary potential writer")?;
                        }
                        RuleEffectKind::ProjectModifierTransform { stat, targets, .. } => {
                            self.forbid(stat, "output stat has an ordinary potential writer")?;
                            self.used.work(targets.len(), self.limits)?;
                            for target in targets {
                                if let Some(stat) = &target.when {
                                    self.forbid(
                                        stat,
                                        "final type output cannot be a modifier predicate",
                                    )?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        self.used
            .work(rules.input().receivers.members.len(), self.limits)?;
        for receiver in &rules.input().receivers.members {
            self.forbid(
                &receiver.stat,
                "final type output cannot be an actor receiver stat",
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct OwnedSupportOutputBindings {
    input: SupportOutputBindingsInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: SupportOutputStorageUse,
}
impl OwnedSupportOutputBindings {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: SupportOutputBindingsInput,
        dependencies: &SupportOutputDependencies<'_, I>,
        limits: SupportOutputStorageLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_SUPPORT_OUTPUT_BINDINGS_VERSION {
            return Err(SupportOutputStorageError::Version(input.schema_version));
        }
        bindings(&input, dependencies)?;
        // Bound caller wire size before allocating indexes or canonical copies.
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let d = dependencies;
        let mut used = SupportOutputStorageUse::default();
        used.entries(input.final_skill_types.len(), limits)?;
        used.work(d.stages.input().stages.len(), limits)?;
        if !d
            .stages
            .precedes(&d.inputs.input().preparation_stage, &input.output_stage)
        {
            return Err(SupportOutputStorageError::Invalid(
                "output stage must strictly follow preparation stage",
            ));
        }
        let vocabulary = &d.preparation.input().types;
        if input.final_skill_types.len() != vocabulary.len() {
            return Err(SupportOutputStorageError::Invalid(
                "final type outputs must cover exact preparation vocabulary",
            ));
        }
        used.sort(input.final_skill_types.len(), limits)?;
        input
            .final_skill_types
            .sort_unstable_by(|a, b| a.support_type.cmp(&b.support_type));
        for (row, expected) in input.final_skill_types.iter().zip(vocabulary) {
            if &row.support_type != expected {
                return Err(SupportOutputStorageError::Invalid(
                    "final type outputs must cover exact preparation vocabulary",
                ));
            }
            used.work(1, limits)?;
            let SchemaLookup::Known(schema) = d.definitions.definition(&row.stat) else {
                return Err(SupportOutputStorageError::Invalid(
                    "missing, foreign or unmapped output stat",
                ));
            };
            used.work(schema.targets.len(), limits)?;
            if schema.value != ComputedValueType::Boolean
                || schema.targets.as_slice() != [RuleEntityKind::Skill]
            {
                return Err(SupportOutputStorageError::Invalid(
                    "output stat must be Boolean and exclusively Skill-scoped",
                ));
            }
            used.work(search_work(d.stages.input().frozen_channels.len()), limits)?;
            if d.stages.frozen_at(&StageChannel::Stat {
                scope: RuleEntityKind::Skill,
                stat: row.stat.clone(),
            }) != Some(&input.output_stage)
            {
                return Err(SupportOutputStorageError::Invalid(
                    "output stat must freeze exactly at output stage",
                ));
            }
        }
        used.sort(input.final_skill_types.len(), limits)?;
        let mut outputs: Vec<_> = input
            .final_skill_types
            .iter()
            .map(|row| &row.stat)
            .collect();
        outputs.sort_unstable();
        if outputs.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SupportOutputStorageError::Invalid(
                "final type output stats must be distinct",
            ));
        }
        let mut check = Check {
            outputs,
            stage: &input.output_stage,
            stages: d.stages,
            limits,
            used,
        };
        let initial = d.inputs.input();
        let target = &initial.target;
        for stat in [
            &initial.effective_level,
            &initial.effective_quality,
            &target.cannot_be_supported,
            &target.has_gem,
            &target.from_item,
            &target.is_player_actor,
            &target.minion_types.present,
            &target.summoner.present,
            &target.summoner.minion_types.present,
        ] {
            check.forbid(stat, "final type output aliases an initial input channel")?;
        }
        for rows in [
            &target.skill_types,
            &target.minion_types.members,
            &target.summoner.skill_types,
            &target.summoner.minion_types.members,
        ] {
            check.used.work(rows.len(), limits)?;
            for row in rows {
                check.forbid(
                    &row.stat,
                    "final type output aliases an initial input channel",
                )?;
            }
        }
        check.rules(d.rules)?;
        let resources = check.used;
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources,
        })
    }
    pub fn input(&self) -> &SupportOutputBindingsInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> SupportOutputStorageUse {
        self.resources
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        dependencies: &SupportOutputDependencies<'_, I>,
    ) -> Result<()> {
        bindings(&self.input, dependencies)
    }
    pub fn validate_limits(&self, limits: SupportOutputStorageLimits) -> Result<()> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(SupportOutputStorageError::Limit("bytes"));
        }
        Ok(())
    }
}

pub fn decode_support_output_bindings<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    dependencies: &SupportOutputDependencies<'_, I>,
    limits: SupportOutputStorageLimits,
) -> Result<OwnedSupportOutputBindings> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(SupportOutputStorageError::Limit("bytes"));
    }
    OwnedSupportOutputBindings::new(serde_json::from_slice(bytes)?, dependencies, limits)
}
pub fn encode_support_output_bindings(
    package: &OwnedSupportOutputBindings,
    limits: SupportOutputStorageLimits,
) -> Result<Vec<u8>> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
