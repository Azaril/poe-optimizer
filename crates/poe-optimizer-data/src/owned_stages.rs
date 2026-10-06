//! Bounded stage scheduling and frozen-channel validation, without coverage authority.
use crate::{owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage};
use poe_optimizer_core::{
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_readiness::*,
    owned_routing::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

mod applications;
mod readiness;

const DOMAIN: &str = "owned-evaluation-stages-v1";
#[derive(Clone, Copy, Debug)]
pub struct StageStorageLimits {
    pub max_stages: usize,
    pub max_entries: usize,
    pub max_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for StageStorageLimits {
    fn default() -> Self {
        Self {
            max_stages: 64,
            max_entries: 1_000_000,
            max_work: 8_000_000,
            max_wire_bytes: 32 * 1024 * 1024,
        }
    }
}
impl StageStorageLimits {
    pub fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("stages", self.max_stages, hard.max_stages),
            ("entries", self.max_entries, hard.max_entries),
            ("work", self.max_work, hard.max_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(StageStorageError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum StageStorageError {
    #[error("invalid stage storage limit: {0}")]
    InvalidLimit(&'static str),
    #[error("stage package exceeds {0}")]
    Limit(&'static str),
    #[error("unsupported evaluation stages version {0}")]
    Version(u32),
    #[error("evaluation stages schema/rule/routing binding mismatch")]
    Binding,
    #[error("invalid evaluation stages: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, StageStorageError>;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct StageStorageUse {
    pub stages: usize,
    pub entries: usize,
    pub work: usize,
}
impl StageStorageUse {
    fn check(self, limits: StageStorageLimits) -> Result<()> {
        for (name, used, max) in [
            ("stages", self.stages, limits.max_stages),
            ("entries", self.entries, limits.max_entries),
            ("work", self.work, limits.max_work),
        ] {
            if used > max {
                return Err(StageStorageError::Limit(name));
            }
        }
        Ok(())
    }
    fn work(&mut self, n: usize, l: StageStorageLimits) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or(StageStorageError::Limit("work"))?;
        self.check(l)
    }
    fn entries(&mut self, n: usize, l: StageStorageLimits) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(n)
            .ok_or(StageStorageError::Limit("entries"))?;
        self.work(n, l)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum OwnerKey {
    Definition(DefinitionAddress),
    Slot(SlotAddress),
}
fn owner_key(owner: &SchemaSubject) -> OwnerKey {
    match owner {
        SchemaSubject::Definition(v) => OwnerKey::Definition(v.clone()),
        SchemaSubject::Slot(v) => OwnerKey::Slot(v.clone()),
    }
}
fn known<T>(lookup: SchemaLookup<'_, T>) -> Result<&T> {
    match lookup {
        SchemaLookup::Known(v) => Ok(v),
        _ => Err(StageStorageError::Invalid(
            "missing, foreign or unmapped schema reference",
        )),
    }
}
fn scope(
    entity: RuleEntity,
    context: RuleEntityKind,
    source: Option<RuleEntityKind>,
) -> Result<RuleEntityKind> {
    Ok(match entity {
        RuleEntity::Current => context,
        RuleEntity::Modifier => RuleEntityKind::Modifier,
        RuleEntity::Actor | RuleEntity::Player => RuleEntityKind::Actor,
        RuleEntity::Enemy => RuleEntityKind::Enemy,
        RuleEntity::Environment => RuleEntityKind::Environment,
        RuleEntity::SupportOrigin => RuleEntityKind::SupportOrigin,
        RuleEntity::Skill | RuleEntity::AssignedSkill | RuleEntity::PropertyOwner => {
            RuleEntityKind::Skill
        }
        RuleEntity::EffectSource => {
            return source.ok_or(StageStorageError::Invalid(
                "effect source scope outside an application",
            ));
        }
    })
}
fn check_channel<I: DefinitionSchemaIndex>(
    channel: &StageChannel,
    index: &I,
    used: &mut StageStorageUse,
    l: StageStorageLimits,
) -> Result<()> {
    used.work(1, l)?;
    match channel {
        StageChannel::Stat { scope, stat } | StageChannel::Contributions { scope, stat, .. } => {
            let schema = known(index.definition(stat))?;
            used.work(schema.targets.len(), l)?;
            if !schema.targets.contains(scope) {
                return Err(StageStorageError::Invalid("stat channel scope mismatch"));
            }
            if let ComputedValueType::Quantity { unit } = &schema.value {
                known(index.definition(unit))?;
            }
            if matches!(channel, StageChannel::Contributions { .. })
                && !matches!(
                    schema.value,
                    ComputedValueType::Integer | ComputedValueType::Quantity { .. }
                )
            {
                return Err(StageStorageError::Invalid(
                    "contribution channel must be numeric",
                ));
            }
        }
        StageChannel::Capability { scope, capability } => {
            let schema = known(index.definition(capability))?;
            used.work(schema.targets.len(), l)?;
            if !schema.targets.contains(scope) {
                return Err(StageStorageError::Invalid(
                    "capability channel scope mismatch",
                ));
            }
        }
        StageChannel::ModifierTransforms { stat } => {
            let schema = known(index.definition(stat))?;
            used.work(schema.targets.len() + 1, l)?;
            let ComputedValueType::Quantity { unit } = &schema.value else {
                return Err(StageStorageError::Invalid(
                    "transform channel must be a factor",
                ));
            };
            if !schema.targets.contains(&RuleEntityKind::Modifier)
                || known(index.definition(unit))?.dimension != UnitDimension::DimensionlessFactor
            {
                return Err(StageStorageError::Invalid(
                    "transform channel scope/unit mismatch",
                ));
            }
        }
        StageChannel::Grant { slot } => {
            known(index.slot(slot))?;
        }
        StageChannel::SkillParameter { parameter } => {
            known(index.slot(parameter))?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct OwnedEvaluationStages {
    input: EvaluationStagesInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: StageStorageUse,
    stages: BTreeMap<OwnedDefinitionKey, usize>,
    ancestors: Vec<u64>,
    programs: BTreeMap<(OwnerKey, OwnedDefinitionKey), usize>,
    effect_applications: BTreeMap<OwnedDefinitionKey, usize>,
    frozen: BTreeMap<StageChannel, usize>,
    readiness: readiness::ReadinessIndex,
}
impl OwnedEvaluationStages {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: EvaluationStagesInput,
        index: &I,
        rules: &OwnedRulePackage,
        routing: &OwnedActionRouting,
        limits: StageStorageLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if !matches!(
            input.schema_version,
            OWNED_EVALUATION_STAGES_VERSION
                | OWNED_EVALUATION_STAGES_V2
                | OWNED_EVALUATION_STAGES_V3
                | OWNED_EVALUATION_STAGES_V4
        ) {
            return Err(StageStorageError::Version(input.schema_version));
        }
        bindings(&input, index, rules, routing)?;
        let readiness_version =
            RuleOperationsVersion::parse(rules.input().operations_version.as_str())
                .is_some_and(RuleOperationsVersion::supports_readiness);
        let source_version =
            RuleOperationsVersion::parse(rules.input().operations_version.as_str())
                .is_some_and(RuleOperationsVersion::supports_source_properties);
        if (input.schema_version >= OWNED_EVALUATION_STAGES_V2) != input.readiness.is_some()
            || (input.schema_version >= OWNED_EVALUATION_STAGES_V2 && !readiness_version)
            || (readiness_version && input.schema_version < OWNED_EVALUATION_STAGES_V2)
            || (source_version
                != matches!(
                    input.schema_version,
                    OWNED_EVALUATION_STAGES_V3 | OWNED_EVALUATION_STAGES_V4
                ))
        {
            return Err(StageStorageError::Invalid(
                "readiness requires operations V16 and stages V2 with explicit metadata",
            ));
        }
        let domain = if input.schema_version == OWNED_EVALUATION_STAGES_V4 {
            "owned-evaluation-stages-v4"
        } else if input.schema_version == OWNED_EVALUATION_STAGES_V3 {
            "owned-evaluation-stages-v3"
        } else if input.schema_version == OWNED_EVALUATION_STAGES_V2 {
            "owned-evaluation-stages-v2"
        } else {
            DOMAIN
        };
        // Bound the complete caller-owned DTO before any secondary indexes.
        digest_owned(domain, &input, limits.max_wire_bytes)?;
        let mut used = StageStorageUse {
            stages: input.stages.len(),
            ..Default::default()
        };
        used.check(limits)?;
        if input.stages.is_empty() {
            return Err(StageStorageError::Invalid("stages cannot be empty"));
        }
        used.entries(input.stages.len(), limits)?;
        used.entries(input.programs.members.len(), limits)?;
        used.entries(input.frozen_channels.len(), limits)?;
        readiness::charge(&input, &mut used, limits)?;
        for stage in &input.stages {
            used.entries(stage.predecessors.len(), limits)?;
        }
        if let SchemaClosure::Partial { gaps } = &input.programs.closure {
            used.entries(gaps.len(), limits)?;
            if gaps.is_empty() {
                return Err(StageStorageError::Invalid(
                    "partial program partition requires gaps",
                ));
            }
            let mut seen = BTreeSet::new();
            for gap in gaps {
                let owner = owner_key(&gap.subject);
                if gap.facet != SchemaFacet::GameRules || !seen.insert((owner, gap.code.clone())) {
                    return Err(StageStorageError::Invalid("invalid partition gap"));
                }
                match &gap.subject {
                    SchemaSubject::Definition(v) => {
                        if index.lookup_definition(v).is_none() {
                            return Err(StageStorageError::Invalid("foreign partition gap"));
                        }
                    }
                    SchemaSubject::Slot(v) => {
                        if index.lookup_slot(v).is_none() {
                            return Err(StageStorageError::Invalid("foreign partition gap"));
                        }
                    }
                }
            }
        }
        input.stages.sort_by(|a, b| a.id.cmp(&b.id));
        let stages: BTreeMap<_, _> = input
            .stages
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();
        if stages.len() != input.stages.len() {
            return Err(StageStorageError::Invalid("duplicate stage"));
        }
        if !stages.contains_key(&input.routing_stage) {
            return Err(StageStorageError::Invalid("unknown routing stage"));
        }
        let mut ancestors = vec![0u64; stages.len()];
        for (i, stage) in input.stages.iter_mut().enumerate() {
            stage.predecessors.sort();
            let mut previous = None;
            for predecessor in &stage.predecessors {
                if previous == Some(predecessor) {
                    return Err(StageStorageError::Invalid("duplicate predecessor"));
                }
                previous = Some(predecessor);
                let Some(&j) = stages.get(predecessor) else {
                    return Err(StageStorageError::Invalid("unknown predecessor"));
                };
                ancestors[i] |= 1u64 << j;
            }
        }
        used.work(
            stages
                .len()
                .checked_mul(stages.len())
                .ok_or(StageStorageError::Limit("work"))?,
            limits,
        )?;
        for k in 0..stages.len() {
            for i in 0..stages.len() {
                if ancestors[i] & (1u64 << k) != 0 {
                    ancestors[i] |= ancestors[k];
                }
            }
        }
        if ancestors
            .iter()
            .enumerate()
            .any(|(i, a)| a & (1u64 << i) != 0)
        {
            return Err(StageStorageError::Invalid("cyclic stage predecessors"));
        }
        let mut known_programs = BTreeMap::new();
        used.entries(rules.input().owners.len(), limits)?;
        for owner in &rules.input().owners {
            used.entries(owner.programs.members.len(), limits)?;
            for p in &owner.programs.members {
                known_programs.insert((owner_key(&owner.owner), p.id.clone()), p);
            }
        }
        let mut programs = BTreeMap::new();
        for row in &input.programs.members {
            let key = (owner_key(&row.owner), row.program.clone());
            if !known_programs.contains_key(&key) {
                return Err(StageStorageError::Invalid(
                    "unknown owner-qualified program",
                ));
            }
            let Some(&stage) = stages.get(&row.stage) else {
                return Err(StageStorageError::Invalid("unknown program stage"));
            };
            if programs.insert(key, stage).is_some() {
                return Err(StageStorageError::Invalid(
                    "duplicate program classification",
                ));
            }
        }
        if input.programs.is_complete() && programs.len() != known_programs.len() {
            return Err(StageStorageError::Invalid(
                "complete partition omits known programs",
            ));
        }
        let effect_applications =
            applications::validate(&input, index, rules, &stages, &mut used, limits)?;
        let readiness = readiness::validate(&mut input, index, rules, &mut used, limits)?;
        let mut frozen = BTreeMap::new();
        for row in &input.frozen_channels {
            check_channel(&row.channel, index, &mut used, limits)?;
            let Some(&stage) = stages.get(&row.stage) else {
                return Err(StageStorageError::Invalid("unknown frozen stage"));
            };
            if frozen.insert(row.channel.clone(), stage).is_some() {
                return Err(StageStorageError::Invalid("duplicate frozen channel"));
            }
        }
        let mut ordered = BTreeMap::new();
        if let Some(registry) = &rules.input().ordered_contributions {
            used.entries(registry.members.len(), limits)?;
            used.work(registry.members.len(), limits)?;
            for query in &registry.members {
                ordered.insert(&query.id, (&query.stat, query.contribution));
            }
        }
        let mut access = Access {
            ordered: &ordered,
            frozen: &frozen,
            ancestors: &ancestors,
            used: &mut used,
            limits,
        };
        for (key, program) in known_programs {
            if let Some(&stage) = programs.get(&key) {
                access.program(program, stage, None)?;
            }
        }
        if let Some(applications) = &rules.input().effect_applications {
            for application in &applications.members {
                if let Some(&stage) = effect_applications.get(&application.id) {
                    let source = match application.source {
                        EffectApplicationSource::Skill { .. } => RuleEntityKind::Skill,
                        EffectApplicationSource::OwnedSlot { .. } => RuleEntityKind::Actor,
                    };
                    access.program(&application.program, stage, Some(source))?;
                }
            }
        }
        access.routing(
            routing,
            *stages.get(&input.routing_stage).expect("validated stage"),
        )?;
        input.programs.members.sort_by(|a, b| {
            (owner_key(&a.owner), &a.program).cmp(&(owner_key(&b.owner), &b.program))
        });
        if let SchemaClosure::Partial { gaps } = &mut input.programs.closure {
            gaps.sort_by(|a, b| {
                (owner_key(&a.subject), &a.code).cmp(&(owner_key(&b.subject), &b.code))
            });
        }
        if let Some(applications) = &mut input.effect_applications {
            applications
                .members
                .sort_by(|a, b| a.application.cmp(&b.application));
            if let SchemaClosure::Partial { gaps } = &mut applications.closure {
                gaps.sort_by(|a, b| {
                    (owner_key(&a.subject), &a.code).cmp(&(owner_key(&b.subject), &b.code))
                });
            }
        }
        input
            .frozen_channels
            .sort_by(|a, b| a.channel.cmp(&b.channel));
        let identity = digest_owned(domain, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources: used,
            stages,
            ancestors,
            programs,
            effect_applications,
            frozen,
            readiness,
        })
    }
    pub fn readiness(&self) -> Option<&ReadinessInput> {
        self.input.readiness.as_ref()
    }
    pub fn program_readiness(
        &self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
    ) -> Option<&ReadinessProgram> {
        self.readiness
            .programs
            .get(&(owner_key(owner), program.clone()))
            .and_then(|i| {
                self.input
                    .readiness
                    .as_ref()
                    .map(|v| &v.programs.members[*i])
            })
    }
    pub fn parameter_phase(
        &self,
        skill: &SkillDefId,
        parameter: &poe_optimizer_core::owned_build::DeclaredSlot<ParameterSlotDefId>,
    ) -> Option<ReadinessPhase> {
        self.readiness
            .parameters
            .get(&(skill.clone(), parameter.clone()))
            .copied()
    }
    pub fn skill_readiness(&self, skill: &SkillDefId) -> Option<&GeneratedSkillReadiness> {
        self.readiness
            .skills
            .get(skill)
            .and_then(|i| self.input.readiness.as_ref().map(|v| &v.skills[*i]))
    }
    pub fn input(&self) -> &EvaluationStagesInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> StageStorageUse {
        self.resources
    }
    /// Classification completeness only. All ordinary schema/rule/receiver gates remain required.
    pub fn is_complete(&self) -> bool {
        self.input.programs.is_complete()
            && self
                .input
                .effect_applications
                .as_ref()
                .is_none_or(DeclaredSet::is_complete)
    }
    pub fn stage_for(
        &self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
    ) -> Option<&OwnedDefinitionKey> {
        self.programs
            .get(&(owner_key(owner), program.clone()))
            .map(|i| &self.input.stages[*i].id)
    }
    pub fn stage_for_application(
        &self,
        application: &OwnedDefinitionKey,
    ) -> Option<&OwnedDefinitionKey> {
        self.effect_applications
            .get(application)
            .map(|i| &self.input.stages[*i].id)
    }
    /// Strict transitive precedence. Unknown stages and equality return false.
    pub fn precedes(&self, before: &OwnedDefinitionKey, after: &OwnedDefinitionKey) -> bool {
        match (self.stages.get(before), self.stages.get(after)) {
            (Some(a), Some(b)) => self.ancestors[*b] & (1u64 << a) != 0,
            _ => false,
        }
    }
    pub fn frozen_at(&self, channel: &StageChannel) -> Option<&OwnedDefinitionKey> {
        self.frozen.get(channel).map(|i| &self.input.stages[*i].id)
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        rules: &OwnedRulePackage,
        routing: &OwnedActionRouting,
    ) -> Result<()> {
        bindings(&self.input, index, rules, routing)
    }
    pub fn validate_limits(&self, limits: StageStorageLimits) -> Result<()> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(StageStorageError::Limit("bytes"));
        }
        Ok(())
    }
}
fn bindings<I: DefinitionSchemaIndex>(
    input: &EvaluationStagesInput,
    index: &I,
    rules: &OwnedRulePackage,
    routing: &OwnedActionRouting,
) -> Result<()> {
    if input.definitions != *index.identity()
        || input.namespace != *index.namespace()
        || input.rules != *rules.identity()
        || input.routing != *routing.identity()
        || input.definitions != *rules.definitions()
        || input.namespace != rules.input().namespace
        || input.definitions != routing.input().definitions
        || input.namespace != routing.input().namespace
        || input.definitions.validate().is_err()
    {
        return Err(StageStorageError::Binding);
    }
    Ok(())
}

struct Access<'a> {
    ordered: &'a BTreeMap<&'a OwnedDefinitionKey, (&'a StatDefId, ContributionKind)>,
    frozen: &'a BTreeMap<StageChannel, usize>,
    ancestors: &'a [u64],
    used: &'a mut StageStorageUse,
    limits: StageStorageLimits,
}
impl Access<'_> {
    fn channel(&mut self, channel: StageChannel, stage: usize, write: bool) -> Result<()> {
        self.used.work(1, self.limits)?;
        if let Some(&freeze) = self.frozen.get(&channel) {
            let (before, after) = if write {
                (stage, freeze)
            } else {
                (freeze, stage)
            };
            if before != after && self.ancestors[after] & (1u64 << before) == 0 {
                return Err(StageStorageError::Invalid(if write {
                    "potential writer occurs after or outside frozen stage"
                } else {
                    "frozen channel read occurs before or outside frozen stage"
                }));
            }
        }
        Ok(())
    }
    fn stat(
        &mut self,
        scope: RuleEntityKind,
        stat: &StatDefId,
        stage: usize,
        write: bool,
    ) -> Result<()> {
        self.channel(
            StageChannel::Stat {
                scope,
                stat: stat.clone(),
            },
            stage,
            write,
        )
    }
    fn program(
        &mut self,
        p: &RuleProgram,
        stage: usize,
        source: Option<RuleEntityKind>,
    ) -> Result<()> {
        self.used.entries(p.reads.len(), self.limits)?;
        self.used.entries(p.effects.len(), self.limits)?;
        for read in &p.reads {
            match &read.source {
                RuleReadSource::Stat { entity, stat } => {
                    self.stat(scope(*entity, p.context, source)?, stat, stage, false)?
                }
                RuleReadSource::Capability { entity, capability } => self.channel(
                    StageChannel::Capability {
                        scope: scope(*entity, p.context, source)?,
                        capability: capability.clone(),
                    },
                    stage,
                    false,
                )?,
                RuleReadSource::Contributions {
                    entity,
                    stat,
                    contribution,
                    ..
                } => self.channel(
                    StageChannel::Contributions {
                        scope: scope(*entity, p.context, source)?,
                        stat: stat.clone(),
                        contribution: *contribution,
                    },
                    stage,
                    false,
                )?,
                RuleReadSource::OrderedContributions { entity, query, .. } => {
                    let Some((stat, contribution)) = self.ordered.get(query) else {
                        return Err(StageStorageError::Invalid(
                            "unknown ordered contribution query",
                        ));
                    };
                    self.channel(
                        StageChannel::Contributions {
                            scope: scope(*entity, p.context, source)?,
                            stat: (*stat).clone(),
                            contribution: *contribution,
                        },
                        stage,
                        false,
                    )?;
                }
                RuleReadSource::ModifierTransforms { stat, initial } => {
                    self.channel(
                        StageChannel::ModifierTransforms { stat: stat.clone() },
                        stage,
                        false,
                    )?;
                    self.stat(RuleEntityKind::Modifier, initial, stage, false)?;
                }
                RuleReadSource::Parameter { slot }
                | RuleReadSource::EffectSourceParameter { slot } => self.channel(
                    StageChannel::SkillParameter {
                        parameter: slot.clone(),
                    },
                    stage,
                    false,
                )?,
                _ => {}
            }
        }
        for effect in &p.effects {
            match &effect.effect {
                RuleEffectKind::Derive { entity, stat, .. } => {
                    self.stat(scope(*entity, p.context, source)?, stat, stage, true)?
                }
                RuleEffectKind::Capability {
                    entity, capability, ..
                } => self.channel(
                    StageChannel::Capability {
                        scope: scope(*entity, p.context, source)?,
                        capability: capability.clone(),
                    },
                    stage,
                    true,
                )?,
                RuleEffectKind::Contribute {
                    entity,
                    stat,
                    contribution,
                    ..
                } => self.channel(
                    StageChannel::Contributions {
                        scope: scope(*entity, p.context, source)?,
                        stat: stat.clone(),
                        contribution: *contribution,
                    },
                    stage,
                    true,
                )?,
                RuleEffectKind::ProjectModifierTransform { stat, targets, .. } => {
                    self.channel(
                        StageChannel::ModifierTransforms { stat: stat.clone() },
                        stage,
                        true,
                    )?;
                    self.used.entries(targets.len(), self.limits)?;
                    for target in targets {
                        if let Some(stat) = &target.when {
                            self.stat(RuleEntityKind::Modifier, stat, stage, false)?;
                        }
                    }
                }
                RuleEffectKind::ActivateGrant { slot, .. } => {
                    self.channel(StageChannel::Grant { slot: slot.clone() }, stage, true)?
                }
                RuleEffectKind::ProjectSkillParameter { parameter, .. } => self.channel(
                    StageChannel::SkillParameter {
                        parameter: parameter.clone(),
                    },
                    stage,
                    true,
                )?,
                RuleEffectKind::ProjectActorStat { stat, .. } => {
                    self.stat(RuleEntityKind::Actor, stat, stage, true)?
                }
                RuleEffectKind::SupportApplicability { .. }
                | RuleEffectKind::Requirement { .. } => {}
            }
        }
        Ok(())
    }
    fn routing(&mut self, routing: &OwnedActionRouting, stage: usize) -> Result<()> {
        self.used
            .entries(routing.input().outputs.len(), self.limits)?;
        for output in &routing.input().outputs {
            self.used
                .entries(output.routes.members.len(), self.limits)?;
            for route in &output.routes.members {
                self.stat(RuleEntityKind::Action, &route.target, stage, true)?;
                match &route.source {
                    ActionStatRouteSource::PlayerEquipment { stat, .. } => {
                        self.stat(RuleEntityKind::EquipmentUse, stat, stage, false)?
                    }
                    ActionStatRouteSource::ActionActor { stat } => {
                        self.stat(RuleEntityKind::Actor, stat, stage, false)?
                    }
                    ActionStatRouteSource::Selected { selector, stats } => {
                        self.used.entries(stats.len(), self.limits)?;
                        let selectors =
                            output
                                .source_selectors
                                .as_ref()
                                .ok_or(StageStorageError::Invalid(
                                    "selected routing without selectors",
                                ))?;
                        self.used.work(selectors.members.len(), self.limits)?;
                        let selector = selectors
                            .members
                            .iter()
                            .find(|v| &v.id == selector)
                            .ok_or(StageStorageError::Invalid("missing routing selector"))?;
                        for stat in stats {
                            self.used.work(selector.sources.len(), self.limits)?;
                            let source = selector
                                .sources
                                .iter()
                                .find(|s| s.id == stat.source)
                                .ok_or(StageStorageError::Invalid("missing named route source"))?;
                            let scope = match source.origin {
                                ActionSourceOrigin::ActionActor => RuleEntityKind::Actor,
                                ActionSourceOrigin::CurrentAction => RuleEntityKind::Action,
                                ActionSourceOrigin::PlayerEquipment { .. } => {
                                    RuleEntityKind::EquipmentUse
                                }
                            };
                            self.stat(scope, &stat.stat, stage, false)?;
                        }
                    }
                }
            }
            if let Some(selectors) = &output.source_selectors {
                self.used.entries(selectors.members.len(), self.limits)?;
                for selector in &selectors.members {
                    if let ActionSourcePolicy::EquipmentEligibility { capability, .. } =
                        &selector.policy
                    {
                        self.channel(
                            StageChannel::Capability {
                                scope: RuleEntityKind::EquipmentUse,
                                capability: capability.clone(),
                            },
                            stage,
                            false,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}
pub fn decode_evaluation_stages<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    rules: &OwnedRulePackage,
    routing: &OwnedActionRouting,
    limits: StageStorageLimits,
) -> Result<OwnedEvaluationStages> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(StageStorageError::Limit("bytes"));
    }
    OwnedEvaluationStages::new(
        serde_json::from_slice(bytes)?,
        index,
        rules,
        routing,
        limits,
    )
}
pub fn encode_evaluation_stages(
    package: &OwnedEvaluationStages,
    limits: StageStorageLimits,
) -> Result<Vec<u8>> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
