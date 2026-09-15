//! Immutable, request-bound topology queries for cold plan compilation.
//! These results certify neither activation nor complete incoming contributions.
use super::*;

/// A directly exposed owner and its authored declaration registry.
#[derive(Clone, Debug)]
pub struct ProviderOwner<'a> {
    definition: SlotOwnerDefId,
    declarations: &'a DeclaredSlots,
}
impl<'a> ProviderOwner<'a> {
    pub(super) fn new(definition: SlotOwnerDefId, declarations: &'a DeclaredSlots) -> Self {
        Self {
            definition,
            declarations,
        }
    }
    pub fn definition(&self) -> &SlotOwnerDefId {
        &self.definition
    }
    pub fn subject(&self) -> SchemaSubject {
        owner_subject(&self.definition)
    }
    pub fn declarations(&self) -> &'a DeclaredSlots {
        self.declarations
    }
}

/// One selected class/ascendancy declaration and its exact borrowed root schemas.
/// Missing/unmapped nodes and partial root/pool sets remain explicit evidence;
/// they do not erase independently known root owners or establish completeness.
#[derive(Clone, Debug)]
pub struct ImplicitPassiveRoots<'a> {
    owner: SlotOwnerDefId,
    declaration: &'a DeclaredSet<PassiveNodeDefId>,
    nodes: Vec<(&'a PassiveNodeDefId, SchemaLookup<'a, PassiveNodeSchema>)>,
}
impl<'a> ImplicitPassiveRoots<'a> {
    pub(super) fn new(
        owner: SlotOwnerDefId,
        declaration: &'a DeclaredSet<PassiveNodeDefId>,
        nodes: Vec<(&'a PassiveNodeDefId, SchemaLookup<'a, PassiveNodeSchema>)>,
    ) -> Self {
        Self {
            owner,
            declaration,
            nodes,
        }
    }
    pub fn owner(&self) -> &SlotOwnerDefId {
        &self.owner
    }
    pub fn declaration(&self) -> &'a DeclaredSet<PassiveNodeDefId> {
        self.declaration
    }
    pub fn nodes(&self) -> &[(&'a PassiveNodeDefId, SchemaLookup<'a, PassiveNodeSchema>)] {
        &self.nodes
    }
}

/// Only explicitly reachable declarations are exposed. Partial sets remain partial.
/// An actor context exposes its listed outputs, never its registry owner's siblings.
#[derive(Clone, Debug)]
pub enum ProviderExposure<'a> {
    Root {
        owners: Vec<ProviderOwner<'a>>,
        /// Potential definitions supplied by a physical gem; no one skill is selected here.
        skills: Option<&'a DeclaredSet<SkillDefId>>,
        /// Character roots only; static membership is separate from grant activation.
        implicit_passives: Vec<ImplicitPassiveRoots<'a>>,
    },
    Skill {
        /// The supplying parent prefix plus the skill slot, not the traversal address.
        key: GeneratedSkillKey,
        owner: SlotOwnerDefId,
        declarations: &'a DeclaredSlots,
        outputs: &'a DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
    },
    Actor {
        key: OwnedActorKey,
        parent_actor: ActorKey,
        schema: &'a ActorSlotSchema,
    },
    AllocationAccess {
        pools: &'a DeclaredSet<PointPoolDefId>,
    },
}

#[derive(Clone, Debug)]
pub struct ProviderOccurrence<'a> {
    key: ProviderKey,
    actor: ActorKey,
    role: ProviderRole,
    exposure: ProviderExposure<'a>,
}
impl<'a> ProviderOccurrence<'a> {
    pub(super) fn new(
        key: ProviderKey,
        actor: ActorKey,
        role: ProviderRole,
        exposure: ProviderExposure<'a>,
    ) -> Self {
        Self {
            key,
            actor,
            role,
            exposure,
        }
    }
    pub fn key(&self) -> &ProviderKey {
        &self.key
    }
    pub fn actor(&self) -> &ActorKey {
        &self.actor
    }
    pub fn role(&self) -> ProviderRole {
        self.role
    }
    pub fn exposure(&self) -> &ProviderExposure<'a> {
        &self.exposure
    }
}

#[derive(Clone, Debug)]
pub struct SkillOccurrence<'a> {
    target: SkillTarget,
    provider: ProviderOccurrence<'a>,
}
impl<'a> SkillOccurrence<'a> {
    pub fn target(&self) -> &SkillTarget {
        &self.target
    }
    pub fn provider(&self) -> &ProviderOccurrence<'a> {
        &self.provider
    }
    /// None for a physical gem use exposing multiple/potential skill definitions.
    pub fn definition(&self) -> Option<&SkillDefId> {
        match self.provider.exposure() {
            ProviderExposure::Skill {
                owner: SlotOwnerDefId::Skill(id),
                ..
            } => Some(id),
            ProviderExposure::Root {
                owners,
                skills: None,
                ..
            } => owners.iter().find_map(|owner| {
                if let SlotOwnerDefId::Skill(id) = owner.definition() {
                    Some(id)
                } else {
                    None
                }
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ActorOccurrence<'a> {
    Player,
    Owned {
        key: Box<OwnedActorKey>,
        parent_actor: ActorKey,
        schema: &'a ActorSlotSchema,
    },
}
impl ActorOccurrence<'_> {
    pub fn key(&self) -> ActorKey {
        match self {
            Self::Player => ActorKey::Player,
            Self::Owned { key, .. } => ActorKey::Owned(key.clone()),
        }
    }
    pub fn subject(&self) -> Option<SchemaSubject> {
        match self {
            Self::Player => None,
            Self::Owned { key, .. } => {
                Some(SchemaSubject::Slot(ActorSlotDefId::address(&key.slot)))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActionOccurrence<'a> {
    selection: ActionSelection,
    provider: ProviderOccurrence<'a>,
    expected_actor: ActorKey,
    schema: &'a ActionOutputSchema,
}
impl<'a> ActionOccurrence<'a> {
    pub(super) fn new(
        selection: ActionSelection,
        provider: ProviderOccurrence<'a>,
        expected_actor: ActorKey,
        schema: &'a ActionOutputSchema,
    ) -> Self {
        Self {
            selection,
            provider,
            expected_actor,
            schema,
        }
    }
    pub fn selection(&self) -> &ActionSelection {
        &self.selection
    }
    pub fn provider(&self) -> &ProviderOccurrence<'a> {
        &self.provider
    }
    pub fn expected_actor(&self) -> &ActorKey {
        &self.expected_actor
    }
    pub fn schema(&self) -> &'a ActionOutputSchema {
        self.schema
    }
    pub fn subject(&self) -> SchemaSubject {
        SchemaSubject::Slot(ActionOutputDefId::address(&self.selection.action.output))
    }
}

/// Topology evidence only. A schema-valid PendingResolution may expose a value;
/// game activation, legality, incoming contribution closure and numerical coverage remain unproved.
#[derive(Debug)]
pub struct OccurrenceResolution<T> {
    request_digest: OwnedContentDigest,
    data_identity: DataIdentity,
    schema: SchemaBindingStatus,
    status: SelectorBindingStatus,
    issues: Vec<BindingIssue>,
    work_used: usize,
    value: Option<T>,
}
impl<T> OccurrenceResolution<T> {
    pub fn request_digest(&self) -> OwnedContentDigest {
        self.request_digest
    }
    pub fn data_identity(&self) -> &DataIdentity {
        &self.data_identity
    }
    pub fn schema(&self) -> SchemaBindingStatus {
        self.schema
    }
    pub fn status(&self) -> SelectorBindingStatus {
        self.status
    }
    pub fn issues(&self) -> &[BindingIssue] {
        &self.issues
    }
    /// Includes selector registry setup and all charged shared traversal work.
    pub fn work_used(&self) -> usize {
        self.work_used
    }
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }
    pub fn into_value(self) -> Option<T> {
        self.value
    }
}

/// Reuses the binder's exact traversal, including explicit root availability.
/// The constructor validates structure/index identity and hashes the canonical request once.
/// It does not perform whole-request schema binding; plans must separately consume that report.
/// Calls use independent bounded work budgets and report usage for aggregate planner accounting.
pub struct OwnedOccurrenceResolver<'a, I> {
    index: &'a I,
    request: &'a OwnedEvaluationRequest,
    limits: BindingLimits,
    request_digest: OwnedContentDigest,
    data_identity: DataIdentity,
    members: Vec<(InstanceId, OccurrenceKind)>,
}
impl<'a, I: DefinitionSchemaIndex> OwnedOccurrenceResolver<'a, I> {
    pub fn new(
        index: &'a I,
        request: &'a OwnedEvaluationRequest,
        limits: BindingLimits,
    ) -> Result<Self> {
        let request_digest = validated_request_digest(index, request, limits)?;
        let members = build_occurrences(request.build(), limits.input)?;
        Ok(Self {
            index,
            request,
            limits,
            request_digest,
            data_identity: index.identity().clone(),
            members,
        })
    }
    pub fn request_digest(&self) -> OwnedContentDigest {
        self.request_digest
    }
    pub fn data_identity(&self) -> &DataIdentity {
        &self.data_identity
    }

    fn start(&self) -> Result<(Checker<'a, I>, StructuralCheck<'a>)> {
        if self.index.identity() != &self.data_identity
            || self.index.namespace() != &self.request.build().input().game_version
        {
            return Err(BindingError::Index {
                subject: None,
                fault: IndexFault::InconsistentLookup,
            });
        }
        let build = self.request.build().input();
        let mut checker = Checker {
            index: self.index,
            request: self.request,
            limits: self.limits,
            work: self.limits.max_work,
            issues: vec![],
        };
        let mut structure = StructuralCheck::new(
            &build.game_version,
            self.limits.input,
            Some(build.allocator),
        )?;
        structure.begin_membership();
        checker.charge(self.members.len() + build.items.len() + build.equipment.len() + 1)?;
        for (id, kind) in &self.members {
            structure.register("selector", *id, *kind)?;
        }
        for item in &build.items {
            checker.charge(item.modifiers.len())?;
            for modifier in &item.modifiers {
                structure.seed_modifier_item("selector", modifier.id, item.id)?;
            }
        }
        for equipment in &build.equipment {
            structure.seed_equipment_item("selector", equipment.id, equipment.item)?;
        }
        Ok((checker, structure))
    }
    fn finish<T>(
        &self,
        checker: Checker<'a, I>,
        value: Option<T>,
        default_status: SelectorBindingStatus,
    ) -> OccurrenceResolution<T> {
        let schema = schema_status(&checker.issues);
        let status = if checker
            .issues
            .iter()
            .any(|i| i.class == IssueClass::Unavailable)
        {
            SelectorBindingStatus::Unavailable
        } else if checker
            .issues
            .iter()
            .any(|i| i.class == IssueClass::Unresolved)
            || value.is_none()
        {
            SelectorBindingStatus::Unresolved
        } else {
            default_status
        };
        let value = if schema == SchemaBindingStatus::Valid
            && !matches!(
                status,
                SelectorBindingStatus::Unavailable | SelectorBindingStatus::Unresolved
            ) {
            value
        } else {
            None
        };
        OccurrenceResolution {
            request_digest: self.request_digest,
            data_identity: self.data_identity.clone(),
            schema,
            status,
            issues: checker.issues,
            work_used: self.limits.max_work - checker.work,
            value,
        }
    }
    pub fn provider(
        &self,
        key: &ProviderKey,
    ) -> Result<OccurrenceResolution<ProviderOccurrence<'a>>> {
        let (mut checker, mut structure) = self.start()?;
        structure.with_query_references(|s| s.provider("selector", key))?;
        let value = checker
            .bind_provider(
                key,
                &BindingSite::new(BindingLocation::Occurrence, BindingFacet::Provider),
                Purpose::Query,
            )?
            .map(|c| c.occurrence(key.clone()));
        Ok(self.finish(checker, value, SelectorBindingStatus::PendingResolution))
    }
    pub fn skill(&self, target: &SkillTarget) -> Result<OccurrenceResolution<SkillOccurrence<'a>>> {
        let (mut checker, mut structure) = self.start()?;
        structure.with_query_references(|s| s.skill("selector", target))?;
        let key = match target {
            SkillTarget::Authored(id) => ProviderKey {
                root: ProviderRoot::SkillUse(*id),
                grant_path: vec![],
            },
            SkillTarget::Generated(key) => key.provider.clone(),
        };
        let value = checker
            .skill_context(
                target,
                &BindingSite::new(BindingLocation::Occurrence, BindingFacet::Target),
                Purpose::Query,
            )?
            .map(|c| SkillOccurrence {
                target: target.clone(),
                provider: c.occurrence(key),
            });
        Ok(self.finish(checker, value, SelectorBindingStatus::PendingResolution))
    }
    pub fn actor(&self, key: &ActorKey) -> Result<OccurrenceResolution<ActorOccurrence<'a>>> {
        let (mut checker, mut structure) = self.start()?;
        structure.with_query_references(|s| s.actor("selector", key))?;
        let value = checker.actor_occurrence(
            key,
            &BindingSite::new(BindingLocation::Occurrence, BindingFacet::Target),
            Purpose::Query,
        )?;
        let status = if *key == ActorKey::Player {
            SelectorBindingStatus::SchemaBound
        } else {
            SelectorBindingStatus::PendingResolution
        };
        Ok(self.finish(checker, value, status))
    }
    pub fn action(
        &self,
        selection: &ActionSelection,
    ) -> Result<OccurrenceResolution<ActionOccurrence<'a>>> {
        let (mut checker, mut structure) = self.start()?;
        structure.with_query_references(|s| s.action("selector", selection))?;
        let value = checker.action_occurrence(
            selection,
            &BindingSite::new(BindingLocation::Occurrence, BindingFacet::Output),
            Purpose::Query,
        )?;
        Ok(self.finish(checker, value, SelectorBindingStatus::PendingResolution))
    }
}
