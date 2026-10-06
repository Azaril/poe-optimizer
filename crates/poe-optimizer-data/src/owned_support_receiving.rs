//! Bounded finite receiving roles. Storage supplies no activation or coverage authority.
use crate::{
    owned_rules::OwnedRulePackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
    owned_support_receiving::*,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

mod preparation;
mod source_properties;

const DOMAIN: &str = "owned-support-receiving-v1";

#[derive(Clone, Copy, Debug)]
pub struct SupportReceivingStorageLimits {
    pub max_entries: usize,
    pub max_path_depth: usize,
    pub max_expanded_endpoints: usize,
    pub max_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for SupportReceivingStorageLimits {
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_path_depth: 64,
            max_expanded_endpoints: 200_000,
            max_work: 2_000_000,
            max_wire_bytes: 16 * 1024 * 1024,
        }
    }
}
impl SupportReceivingStorageLimits {
    pub fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("entries", self.max_entries, hard.max_entries),
            ("path depth", self.max_path_depth, hard.max_path_depth),
            (
                "expanded endpoints",
                self.max_expanded_endpoints,
                hard.max_expanded_endpoints,
            ),
            ("work", self.max_work, hard.max_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(SupportReceivingStorageError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SupportReceivingStorageError {
    #[error("invalid support receiving limit: {0}")]
    InvalidLimit(&'static str),
    #[error("support receiving exceeds {0}")]
    Limit(&'static str),
    #[error("unsupported support receiving version {0}")]
    Version(u32),
    #[error("support receiving package binding mismatch")]
    Binding,
    #[error("invalid support receiving: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, SupportReceivingStorageError>;
fn invalid(message: &'static str) -> SupportReceivingStorageError {
    SupportReceivingStorageError::Invalid(message)
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SupportReceivingStorageUse {
    pub entries: usize,
    pub path_depth: usize,
    pub expanded_endpoints: usize,
    pub work: usize,
}
impl SupportReceivingStorageUse {
    fn check(self, l: SupportReceivingStorageLimits) -> Result<()> {
        for (name, used, maximum) in [
            ("entries", self.entries, l.max_entries),
            ("path depth", self.path_depth, l.max_path_depth),
            (
                "expanded endpoints",
                self.expanded_endpoints,
                l.max_expanded_endpoints,
            ),
            ("work", self.work, l.max_work),
        ] {
            if used > maximum {
                return Err(SupportReceivingStorageError::Limit(name));
            }
        }
        Ok(())
    }
    fn work(&mut self, n: usize, l: SupportReceivingStorageLimits) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or(SupportReceivingStorageError::Limit("work"))?;
        self.check(l)
    }
    fn entries(&mut self, n: usize, l: SupportReceivingStorageLimits) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(n)
            .ok_or(SupportReceivingStorageError::Limit("entries"))?;
        self.work(n, l)
    }
    fn expanded(&mut self, n: usize, l: SupportReceivingStorageLimits) -> Result<()> {
        self.expanded_endpoints = self
            .expanded_endpoints
            .checked_add(n)
            .ok_or(SupportReceivingStorageError::Limit("expanded endpoints"))?;
        self.work(n, l)
    }
}
fn known<T>(v: SchemaLookup<'_, T>) -> Result<&T> {
    match v {
        SchemaLookup::Known(v) => Ok(v),
        _ => Err(invalid("missing, foreign or unmapped schema")),
    }
}
fn bindings<I: DefinitionSchemaIndex>(
    input: &SupportReceivingInput,
    index: &I,
    rules: &OwnedRulePackage,
    preparation: &OwnedSupportPreparation,
    inputs: &OwnedSupportInputBindings,
    stages: &OwnedEvaluationStages,
) -> Result<()> {
    if input.namespace != *index.namespace()
        || input.definitions != *index.identity()
        || input.rules != *rules.identity()
        || input.preparation != *preparation.identity()
        || input.inputs != *inputs.identity()
        || input.stages != *stages.identity()
        || inputs
            .verify_bindings(index, rules, preparation, stages)
            .is_err()
        || input.definitions.validate().is_err()
    {
        return Err(SupportReceivingStorageError::Binding);
    }
    Ok(())
}

/// Actor normalization excludes child skill suffixes: one actor is not one action.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RelativeActor {
    Assigned,
    Owned {
        parent: Vec<DeclaredSlot<GrantSlotDefId>>,
        slot: DeclaredSlot<ActorSlotDefId>,
    },
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum NormalizedEndpoint {
    Actor(Box<RelativeActor>),
    Action(Box<NormalizedAction>),
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NormalizedAction {
    path: Vec<DeclaredSlot<GrantSlotDefId>>,
    output: DeclaredSlot<ActionOutputDefId>,
    part: ActionPartDefId,
    mode: ActionModeDefId,
    stat_set: ActionStatSetDefId,
}
struct PathContext<'a> {
    owner: Option<SlotOwnerDefId>,
    declarations: Option<&'a DeclaredSlots>,
    outputs: &'a DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
    actor_skills: Option<&'a DeclaredSet<SkillDefId>>,
    potential_skills: Option<&'a DeclaredSet<SkillDefId>>,
    actor: RelativeActor,
    exact_skill_supply: bool,
}
fn target_owner(target: &SupportTargetDefinition) -> SlotOwnerDefId {
    match target {
        SupportTargetDefinition::Gem(id) => SlotOwnerDefId::Gem(id.clone()),
        SupportTargetDefinition::Skill(id) => SlotOwnerDefId::Skill(id.clone()),
    }
}
fn target_subject(target: &SupportTargetDefinition) -> SchemaSubject {
    SchemaSubject::Definition(match target {
        SupportTargetDefinition::Gem(id) => id.address(),
        SupportTargetDefinition::Skill(id) => id.address(),
    })
}
struct Check<'a, I> {
    index: &'a I,
    stages: &'a OwnedEvaluationStages,
    preparation_stage: &'a OwnedDefinitionKey,
    l: SupportReceivingStorageLimits,
    used: SupportReceivingStorageUse,
    declarations_complete: bool,
}
impl<'a, I: DefinitionSchemaIndex> Check<'a, I> {
    fn receiving_reads(&self, program: &RuleProgram) -> Result<()> {
        if program.context == RuleEntityKind::Actor
            && program.reads.iter().any(|read| {
                matches!(
                    read.source,
                    RuleReadSource::Stat {
                        entity: RuleEntity::Skill,
                        ..
                    } | RuleReadSource::Capability {
                        entity: RuleEntity::Skill,
                        ..
                    } | RuleReadSource::External {
                        entity: RuleEntity::Skill,
                        ..
                    } | RuleReadSource::Contributions {
                        entity: RuleEntity::Skill,
                        ..
                    }
                )
            })
        {
            return Err(invalid(
                "actor receiving role has no unique receiving skill",
            ));
        }
        Ok(())
    }
    fn membership<T: PartialEq>(&mut self, set: &DeclaredSet<T>, member: &T) -> Result<()> {
        self.used.work(set.members.len() + 1, self.l)?;
        self.declarations_complete &= set.is_complete();
        if !set.members.contains(member) && set.is_complete() {
            return Err(invalid("receiver path or selection is not declared"));
        }
        Ok(())
    }
    fn closure(&mut self, closure: &mut SchemaClosure, subject: &SchemaSubject) -> Result<()> {
        if let SchemaClosure::Partial { gaps } = closure {
            self.declarations_complete = false;
            self.used.entries(gaps.len(), self.l)?;
            if gaps.is_empty() {
                return Err(invalid("partial receiving inventory requires gaps"));
            }
            gaps.sort_by(|a, b| a.code.cmp(&b.code));
            let mut previous = None;
            for gap in gaps {
                if &gap.subject != subject
                    || gap.facet != SchemaFacet::GameRules
                    || previous == Some(&gap.code)
                {
                    return Err(invalid(
                        "receiving gap must name its exact owner and GameRules facet",
                    ));
                }
                previous = Some(&gap.code);
            }
        }
        Ok(())
    }
    fn declarations(&mut self, owner: &SlotOwnerDefId) -> Result<&'a DeclaredSlots> {
        self.used.work(1, self.l)?;
        Ok(match owner {
            SlotOwnerDefId::Skill(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Actor(id) => &known(self.index.definition(id))?.declarations,
            // Legacy actor outputs can be declared by their summoning Gem.
            SlotOwnerDefId::Gem(id) => &known(self.index.definition(id))?.declarations,
            _ => return Err(invalid("receiver declaration is not a skill, actor or gem")),
        })
    }
    fn path(
        &mut self,
        target: &SupportTargetDefinition,
        path: &[DeclaredSlot<GrantSlotDefId>],
    ) -> Result<PathContext<'a>> {
        self.used.work(path.len() + 1, self.l)?;
        let initial_owner = target_owner(target);
        let initial = self.declarations(&initial_owner)?;
        let mut context = PathContext {
            owner: Some(initial_owner),
            declarations: Some(initial),
            outputs: &initial.outputs,
            actor_skills: None,
            potential_skills: match target {
                SupportTargetDefinition::Gem(gem) => {
                    Some(&known(self.index.definition(gem))?.skills)
                }
                SupportTargetDefinition::Skill(_) => None,
            },
            actor: RelativeActor::Assigned,
            exact_skill_supply: false,
        };
        for (i, step) in path.iter().enumerate() {
            self.used.work(2, self.l)?;
            let declarations = if context.owner.as_ref() == Some(&step.declaration) {
                context
                    .declarations
                    .ok_or_else(|| invalid("receiver path has no provider declarations"))?
            } else if let (Some(skills), SlotOwnerDefId::Skill(skill)) =
                (context.potential_skills, &step.declaration)
            {
                self.membership(skills, skill)?;
                self.declarations(&step.declaration)?
            } else {
                return Err(invalid("receiver path crosses an unrelated declaration"));
            };
            self.membership(&declarations.grants, step)?;
            let grant = known(self.index.slot(step))?;
            context.potential_skills = None;
            // Root provider roles and actual activation are occurrence facts.
            match &grant.target {
                GrantTarget::Actor(slot) => {
                    let declarations = self.declarations(&slot.declaration)?;
                    self.membership(&declarations.actors, slot)?;
                    let schema = known(self.index.slot(slot))?;
                    self.used.work(i + 1, self.l)?;
                    context.actor = RelativeActor::Owned {
                        parent: path[..i].to_vec(),
                        slot: slot.clone(),
                    };
                    context.exact_skill_supply = false;
                    context.actor_skills = Some(&schema.skills);
                    context.outputs = &schema.outputs;
                    match &schema.provider_definition {
                        Some(actor) => {
                            context.owner = Some(SlotOwnerDefId::Actor(actor.clone()));
                            context.declarations =
                                Some(&known(self.index.definition(actor))?.declarations);
                        }
                        None => {
                            context.owner = None;
                            context.declarations = None;
                        }
                    }
                }
                GrantTarget::Skill(slot) => {
                    let declarations = self.declarations(&slot.declaration)?;
                    self.membership(&declarations.skill_grants, slot)?;
                    let schema = known(self.index.slot(slot))?;
                    if let Some(skills) = context.actor_skills {
                        self.membership(skills, &schema.skill)?;
                    }
                    context.owner = Some(SlotOwnerDefId::Skill(schema.skill.clone()));
                    context.declarations =
                        Some(&known(self.index.definition(&schema.skill))?.declarations);
                    context.outputs = &schema.outputs;
                    context.exact_skill_supply = true;
                }
                GrantTarget::AllocationAccess { .. } => {
                    return Err(invalid("allocation access is not a receiving provider"));
                }
            }
        }
        Ok(context)
    }
    fn endpoint(
        &mut self,
        target: &SupportTargetDefinition,
        endpoint: &SupportReceiverEndpoint,
        normalized: &mut BTreeSet<NormalizedEndpoint>,
    ) -> Result<()> {
        let context = self.path(target, endpoint.path())?;
        if let SupportAdmissionContext::ReceivingSkill { summoner_path } = endpoint.admission() {
            if endpoint.kind() != SupportReceiverKind::Action || !context.exact_skill_supply {
                return Err(invalid(
                    "receiving admission requires an exact generated skill endpoint",
                ));
            }
            if let Some(path) = summoner_path {
                if path.starts_with(endpoint.path()) {
                    return Err(invalid(
                        "summoner admission cannot depend on receiving skill or descendant",
                    ));
                }
                if !path.is_empty() && !self.path(target, path)?.exact_skill_supply {
                    return Err(invalid("summoner path must reach an exact generated skill"));
                }
            }
        }
        match endpoint {
            SupportReceiverEndpoint::Actor { .. } => {
                self.used.expanded(1, self.l)?;
                if !normalized.insert(NormalizedEndpoint::Actor(Box::new(context.actor))) {
                    return Err(invalid("duplicate normalized receiving endpoint"));
                }
            }
            SupportReceiverEndpoint::Action {
                path,
                output,
                selection,
                ..
            } => {
                // An actor provider exposes its own direct declarations as well as
                // its finite legacy output list. Skill providers expose slot outputs.
                if context.owner.as_ref() == Some(&output.declaration)
                    && matches!(context.owner, Some(SlotOwnerDefId::Actor(_)))
                {
                    self.membership(
                        &context.declarations.expect("actor declarations").outputs,
                        output,
                    )?;
                } else if let (Some(skills), SlotOwnerDefId::Skill(skill)) =
                    (context.potential_skills, &output.declaration)
                {
                    self.membership(skills, skill)?;
                } else {
                    self.membership(context.outputs, output)?;
                }
                let declarations = self.declarations(&output.declaration)?;
                self.membership(&declarations.outputs, output)?;
                let schema = known(self.index.slot(output))?;
                let mut insert = |part: &ActionPartDefId,
                                  mode: &ActionModeDefId,
                                  stat_set: &ActionStatSetDefId|
                 -> Result<()> {
                    if !normalized.insert(NormalizedEndpoint::Action(Box::new(NormalizedAction {
                        path: path.clone(),
                        output: output.clone(),
                        part: part.clone(),
                        mode: mode.clone(),
                        stat_set: stat_set.clone(),
                    }))) {
                        return Err(invalid("duplicate normalized receiving endpoint"));
                    }
                    Ok(())
                };
                match selection {
                    SupportActionSelection::Exact(variant) => {
                        let SupportActionVariant {
                            part,
                            mode,
                            stat_set,
                        } = variant.as_ref();
                        self.membership(&schema.parts, part)?;
                        self.membership(&schema.modes, mode)?;
                        self.membership(&schema.stat_sets, stat_set)?;
                        known(self.index.definition(part))?;
                        known(self.index.definition(mode))?;
                        known(self.index.definition(stat_set))?;
                        self.used.expanded(1, self.l)?;
                        self.used.work(path.len() + 4, self.l)?;
                        insert(part, mode, stat_set)?;
                    }
                    SupportActionSelection::AllDeclared => {
                        // Known members still establish overlap/size evidence under
                        // Partial membership, but never authorize runtime expansion.
                        self.declarations_complete &= schema.parts.is_complete()
                            && schema.modes.is_complete()
                            && schema.stat_sets.is_complete();
                        self.used.work(
                            schema.parts.members.len()
                                + schema.modes.members.len()
                                + schema.stat_sets.members.len(),
                            self.l,
                        )?;
                        for part in &schema.parts.members {
                            known(self.index.definition(part))?;
                        }
                        for mode in &schema.modes.members {
                            known(self.index.definition(mode))?;
                        }
                        for stat_set in &schema.stat_sets.members {
                            known(self.index.definition(stat_set))?;
                        }
                        let n = schema
                            .parts
                            .members
                            .len()
                            .checked_mul(schema.modes.members.len())
                            .and_then(|n| n.checked_mul(schema.stat_sets.members.len()))
                            .ok_or(SupportReceivingStorageError::Limit("expanded endpoints"))?;
                        self.used.expanded(n, self.l)?;
                        self.used.work(
                            n.checked_mul(path.len() + 4)
                                .ok_or(SupportReceivingStorageError::Limit("work"))?,
                            self.l,
                        )?;
                        // Empty Cartesian dimensions must not leave nonempty loop
                        // frontiers doing uncharged work.
                        if n == 0 {
                            return Ok(());
                        }
                        for part in &schema.parts.members {
                            for mode in &schema.modes.members {
                                for stat_set in &schema.stat_sets.members {
                                    insert(part, mode, stat_set)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn programs(
        &mut self,
        owner: &SchemaSubject,
        row: &SupportRolePrograms,
        kind: SupportReceiverKind,
        programs: &BTreeMap<&OwnedDefinitionKey, &RuleProgram>,
    ) -> Result<()> {
        let context = match kind {
            SupportReceiverKind::Actor => RuleEntityKind::Actor,
            SupportReceiverKind::Action => RuleEntityKind::Action,
        };
        if self.stages.readiness().is_some() {
            self.readiness_role(
                owner,
                &row.applicability,
                poe_optimizer_core::owned_readiness::ReadinessProgramRole::Execution,
            )?;
            for key in &row.delivery {
                self.readiness_role(
                    owner,
                    key,
                    poe_optimizer_core::owned_readiness::ReadinessProgramRole::Execution,
                )?;
            }
        }
        self.preparation_programs(owner, row, kind, programs)?;
        let applicability = programs
            .get(&row.applicability)
            .ok_or_else(|| invalid("unknown applicability program"))?;
        self.used.work(
            applicability.effects.len() + applicability.reads.len() + 1,
            self.l,
        )?;
        self.receiving_reads(applicability)?;
        if applicability.context != context
            || applicability.effects.len() != 1
            || applicability.effects[0].when.is_some()
            || !matches!(
                applicability.effects[0].effect,
                RuleEffectKind::SupportApplicability { .. }
            )
        {
            return Err(invalid(
                "applicability requires one unguarded final producer in receiving context",
            ));
        }
        let app_stage = self
            .stages
            .stage_for(owner, &row.applicability)
            .ok_or_else(|| invalid("applicability program has no stage"))?;
        if !self.stages.precedes(self.preparation_stage, app_stage) {
            return Err(invalid("applicability must follow preparation"));
        }
        for key in &row.delivery {
            if key == &row.applicability {
                return Err(invalid("applicability program cannot deliver"));
            }
            let program = programs
                .get(key)
                .ok_or_else(|| invalid("unknown delivery program"))?;
            self.used
                .work(program.effects.len() + program.reads.len() + 1, self.l)?;
            self.receiving_reads(program)?;
            if program.context != context {
                return Err(invalid(
                    "delivery program context differs from receiving role",
                ));
            }
            for effect in &program.effects {
                match &effect.effect {
                    RuleEffectKind::Contribute { entity, .. }
                    | RuleEffectKind::Derive { entity, .. }
                    | RuleEffectKind::Capability { entity, .. }
                        if *entity == RuleEntity::Current
                            || (context == RuleEntityKind::Actor
                                && *entity == RuleEntity::Actor) => {}
                    RuleEffectKind::Requirement { .. } => {}
                    _ => {
                        return Err(invalid(
                            "delivery may write only its receiving scope and requirements",
                        ));
                    }
                }
            }
            let delivery_stage = self
                .stages
                .stage_for(owner, key)
                .ok_or_else(|| invalid("delivery program has no stage"))?;
            if !self.stages.precedes(app_stage, delivery_stage) {
                return Err(invalid("delivery must follow applicability"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct OwnedSupportReceiving {
    input: SupportReceivingInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: SupportReceivingStorageUse,
    declarations_complete: bool,
}
impl OwnedSupportReceiving {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: SupportReceivingInput,
        index: &I,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
        inputs: &OwnedSupportInputBindings,
        stages: &OwnedEvaluationStages,
        limits: SupportReceivingStorageLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if !matches!(
            input.schema_version,
            OWNED_SUPPORT_RECEIVING_VERSION
                | OWNED_SUPPORT_RECEIVING_V2
                | OWNED_SUPPORT_RECEIVING_V3
        ) {
            return Err(SupportReceivingStorageError::Version(input.schema_version));
        }
        bindings(&input, index, rules, preparation, inputs, stages)?;
        let new_version = input.schema_version >= OWNED_SUPPORT_RECEIVING_V2;
        let source_version = input.schema_version == OWNED_SUPPORT_RECEIVING_V3;
        if source_version != input.source_properties.is_some()
            || (source_version
                && (!RuleOperationsVersion::parse(rules.input().operations_version.as_str())
                    .is_some_and(RuleOperationsVersion::supports_source_properties)
                    || !matches!(
                        stages.input().schema_version,
                        poe_optimizer_core::owned_stages::OWNED_EVALUATION_STAGES_V3
                            | poe_optimizer_core::owned_stages::OWNED_EVALUATION_STAGES_V4
                    )))
        {
            return Err(invalid(
                "source properties require receiving V3, operations V18 and stages V3",
            ));
        }
        if new_version
            && (!RuleOperationsVersion::parse(rules.input().operations_version.as_str())
                .is_some_and(RuleOperationsVersion::supports_readiness)
                || stages.readiness().is_none())
        {
            return Err(invalid("receiving V2 requires explicit V16 readiness"));
        }
        let domain = if source_version {
            "owned-support-receiving-v3"
        } else if new_version {
            "owned-support-receiving-v2"
        } else {
            DOMAIN
        };
        if !RuleOperationsVersion::parse(rules.input().operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_actor_support_applicability)
        {
            return Err(invalid(
                "support receiving requires owned-domain-operations-v13",
            ));
        }
        digest_owned(domain, &input, limits.max_wire_bytes)?;
        let mut used = SupportReceivingStorageUse::default();
        used.entries(input.roles.len(), limits)?;
        used.entries(input.targets.len(), limits)?;
        used.entries(input.supports.len(), limits)?;
        if let Some(source) = &input.source_properties {
            source_properties::charge(source, &mut used, limits)?;
        }
        for target in &input.targets {
            used.entries(target.roles.members.len(), limits)?;
            for binding in &target.roles.members {
                used.entries(binding.endpoints.members.len(), limits)?;
                for endpoint in &binding.endpoints.members {
                    used.path_depth = used.path_depth.max(endpoint.path().len());
                    used.entries(endpoint.path().len(), limits)?;
                    if let SupportAdmissionContext::ReceivingSkill {
                        summoner_path: Some(path),
                    } = endpoint.admission()
                    {
                        used.path_depth = used.path_depth.max(path.len());
                        used.entries(path.len(), limits)?;
                    }
                }
            }
        }
        for support in &input.supports {
            used.entries(support.receivers.members.len(), limits)?;
            for row in &support.receivers.members {
                used.entries(row.delivery.len(), limits)?;
                if let Some(preparation) = &row.preparation {
                    if !new_version {
                        return Err(invalid("preparation programs require receiving V2"));
                    }
                    used.entries(preparation.properties.len() + 1, limits)?;
                }
            }
        }
        // Index only after accounting for the complete stored owner/program inventory.
        used.work(rules.input().owners.len(), limits)?;
        for owner in &rules.input().owners {
            used.work(owner.programs.members.len(), limits)?;
        }
        let owners: BTreeMap<_, _> = rules
            .input()
            .owners
            .iter()
            .filter_map(|owner| {
                if let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) = &owner.owner {
                    Some((gem, owner))
                } else {
                    None
                }
            })
            .collect();
        input.roles.sort_by(|a, b| a.id.cmp(&b.id));
        let roles: BTreeMap<_, _> = input
            .roles
            .iter()
            .map(|role| (&role.id, role.kind))
            .collect();
        if roles.len() != input.roles.len() {
            return Err(invalid("duplicate receiving role"));
        }
        let mut check = Check {
            index,
            stages,
            preparation_stage: &inputs.input().preparation_stage,
            l: limits,
            used,
            declarations_complete: true,
        };
        input.targets.sort_by(|a, b| a.owner.cmp(&b.owner));
        if input.targets.windows(2).any(|v| v[0].owner == v[1].owner) {
            return Err(invalid("duplicate target receiving inventory"));
        }
        for target in &mut input.targets {
            check.used.work(1, limits)?;
            match &target.owner {
                SupportTargetDefinition::Skill(skill) => {
                    known(index.definition(skill))?;
                }
                SupportTargetDefinition::Gem(gem) => {
                    let schema = known(index.definition(gem))?;
                    check.used.work(schema.roles.len(), limits)?;
                    if !schema.roles.contains(&AuthoredGemRole::SkillUse) {
                        return Err(invalid("receiving target Gem does not declare SkillUse"));
                    }
                }
            }
            let subject = target_subject(&target.owner);
            check.closure(&mut target.roles.closure, &subject)?;
            target.roles.members.sort_by(|a, b| a.role.cmp(&b.role));
            if target
                .roles
                .members
                .windows(2)
                .any(|v| v[0].role == v[1].role)
            {
                return Err(invalid("duplicate target receiving role"));
            }
            for binding in &mut target.roles.members {
                let kind = roles
                    .get(&binding.role)
                    .ok_or_else(|| invalid("unknown target receiving role"))?;
                check.closure(&mut binding.endpoints.closure, &subject)?;
                binding.endpoints.members.sort();
                let mut normalized = BTreeSet::new();
                for endpoint in &binding.endpoints.members {
                    if endpoint.kind() != *kind {
                        return Err(invalid("endpoint kind differs from receiving role"));
                    }
                    check.endpoint(&target.owner, endpoint, &mut normalized)?;
                }
            }
        }
        input.supports.sort_by(|a, b| a.gem.cmp(&b.gem));
        if input.supports.windows(2).any(|v| v[0].gem == v[1].gem) {
            return Err(invalid("duplicate support receiving inventory"));
        }
        for support in &mut input.supports {
            let schema = known(index.definition(&support.gem))?;
            check.used.work(schema.roles.len() + 1, limits)?;
            if !schema.roles.contains(&AuthoredGemRole::SupportAssignment) {
                return Err(invalid("receiving owner is not a support Gem"));
            }
            let subject = SchemaSubject::Definition(support.gem.address());
            check.closure(&mut support.receivers.closure, &subject)?;
            let owner = owners
                .get(&support.gem)
                .ok_or_else(|| invalid("support receiving owner has no rule inventory"))?;
            check.declarations_complete &= owner.programs.is_complete();
            check.used.work(owner.programs.members.len(), limits)?;
            let programs: BTreeMap<_, _> =
                owner.programs.members.iter().map(|p| (&p.id, p)).collect();
            let mut used_programs = BTreeSet::new();
            let receiving_complete = support.receivers.is_complete();
            support
                .receivers
                .members
                .sort_by(|a, b| a.role.cmp(&b.role));
            if support
                .receivers
                .members
                .windows(2)
                .any(|v| v[0].role == v[1].role)
            {
                return Err(invalid("duplicate support receiving role"));
            }
            for row in &mut support.receivers.members {
                let kind = roles
                    .get(&row.role)
                    .ok_or_else(|| invalid("unknown support receiving role"))?;
                row.delivery.sort();
                if row.delivery.windows(2).any(|v| v[0] == v[1]) {
                    return Err(invalid("duplicate delivery program"));
                }
                if let Some(preparation) = &mut row.preparation {
                    preparation.properties.sort();
                }
                check.programs(&subject, row, *kind, &programs)?;
                if new_version {
                    let keys = std::iter::once(&row.applicability)
                        .chain(row.delivery.iter())
                        .chain(row.preparation.iter().flat_map(|p| {
                            std::iter::once(&p.applicability).chain(p.properties.iter())
                        }));
                    for key in keys {
                        if !used_programs.insert(key) {
                            return Err(invalid("program overlaps receiving roles"));
                        }
                    }
                }
                used_programs.insert(&row.applicability);
                used_programs.extend(&row.delivery);
                if let Some(preparation) = &row.preparation {
                    used_programs.insert(&preparation.applicability);
                    used_programs.extend(&preparation.properties);
                }
            }
            if receiving_complete
                && owner.programs.members.iter().any(|p| {
                    matches!(p.context, RuleEntityKind::Actor | RuleEntityKind::Action)
                        && !used_programs.contains(&p.id)
                })
            {
                return Err(invalid(
                    "complete support receiving inventory omits known delivery/applicability programs",
                ));
            }
        }
        if let Some(source) = &mut input.source_properties {
            check.source_properties(source, rules, preparation)?;
        } else if stages.readiness().is_some_and(|r| {
            r.programs.members.iter().any(|p| {
                matches!(p.role,
            poe_optimizer_core::owned_readiness::ReadinessProgramRole::SourceSupportedProperty
            | poe_optimizer_core::owned_readiness::ReadinessProgramRole::SourceExternalProperty
            | poe_optimizer_core::owned_readiness::ReadinessProgramRole::SourceFinalInputAssembly)
            })
        }) {
            return Err(invalid(
                "source readiness requires receiving source relations",
            ));
        }
        let identity = digest_owned(domain, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources: check.used,
            declarations_complete: check.declarations_complete,
        })
    }
    pub fn input(&self) -> &SupportReceivingInput {
        &self.input
    }
    pub fn source_properties(
        &self,
    ) -> Option<&poe_optimizer_core::owned_source_properties::SourcePropertyPreparationInput> {
        self.input.source_properties.as_ref()
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> SupportReceivingStorageUse {
        self.resources
    }
    /// Local inventory/schema-path evidence only. Missing rows are still unknown;
    /// this does not prove rule contributors, activation or build coverage.
    pub fn declarations_complete(&self) -> bool {
        self.declarations_complete
    }
    pub fn role(&self, id: &OwnedDefinitionKey) -> Option<&SupportReceivingRole> {
        self.input
            .roles
            .binary_search_by(|v| v.id.cmp(id))
            .ok()
            .map(|i| &self.input.roles[i])
    }
    pub fn target_for(
        &self,
        owner: &SupportTargetDefinition,
    ) -> Option<&SupportTargetReceivingRoles> {
        self.input
            .targets
            .binary_search_by(|v| v.owner.cmp(owner))
            .ok()
            .map(|i| &self.input.targets[i])
    }
    pub fn support_for(&self, id: &GemDefId) -> Option<&SupportReceivingEntry> {
        self.input
            .supports
            .binary_search_by(|v| v.gem.cmp(id))
            .ok()
            .map(|i| &self.input.supports[i])
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
        inputs: &OwnedSupportInputBindings,
        stages: &OwnedEvaluationStages,
    ) -> Result<()> {
        bindings(&self.input, index, rules, preparation, inputs, stages)
    }
    pub fn validate_limits(&self, limits: SupportReceivingStorageLimits) -> Result<()> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(SupportReceivingStorageError::Limit("bytes"));
        }
        Ok(())
    }
}
pub fn decode_support_receiving<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    rules: &OwnedRulePackage,
    preparation: &OwnedSupportPreparation,
    inputs: &OwnedSupportInputBindings,
    stages: &OwnedEvaluationStages,
    limits: SupportReceivingStorageLimits,
) -> Result<OwnedSupportReceiving> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(SupportReceivingStorageError::Limit("bytes"));
    }
    OwnedSupportReceiving::new(
        serde_json::from_slice(bytes)?,
        index,
        rules,
        preparation,
        inputs,
        stages,
        limits,
    )
}
pub fn encode_support_receiving(
    package: &OwnedSupportReceiving,
    limits: SupportReceivingStorageLimits,
) -> Result<Vec<u8>> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
