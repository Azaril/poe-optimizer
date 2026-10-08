//! Owned game-rule authoring contracts. No source language or UI state belongs here.
//!
//! Definitions declare bounded expression DAGs and domain effects. Data storage
//! is separate from semantic compilation and concrete provider/actor resolution.
use crate::{
    data::DataIdentity,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_schema::{ComputedValueType, DeclaredSet, RuleEntityKind, SchemaSubject},
};
use serde::{Deserialize, Serialize};

pub const OWNED_RULE_PACKAGE_VERSION: u32 = 3;
/// Baseline operation subset used when authors do not opt into newer capabilities.
/// Package schema and content identity domains always use the current format.
pub const OWNED_RULE_OPERATIONS_VERSION: &str = OWNED_RULE_OPERATIONS_V14;
/// Checked query reads and self-contributions on exact authored/supplied Skills.
pub const OWNED_RULE_OPERATIONS_V24: &str = "owned-domain-operations-v24";
/// Exact shared/supplied Actor and selected reward contribution membership.
pub const OWNED_RULE_OPERATIONS_V23: &str = "owned-domain-operations-v23";
/// Typed Boolean contribution channels with unordered, complete Any reductions.
pub const OWNED_RULE_OPERATIONS_V22: &str = "owned-domain-operations-v22";
/// Candidate-bound ordered contribution groups. Earlier reductions stay unchanged.
pub const OWNED_RULE_OPERATIONS_V21: &str = "owned-domain-operations-v21";
/// Read-only predicates on an exact bound Action selection.
pub const OWNED_RULE_OPERATIONS_V20: &str = "owned-domain-operations-v20";
/// Explicit preset input producers for exact provider-generated Skills.
pub const OWNED_RULE_OPERATIONS_V19: &str = "owned-domain-operations-v19";
/// Explicit source-property invocation authority over existing Skill targets.
pub const OWNED_RULE_OPERATIONS_V18: &str = "owned-domain-operations-v18";
/// Explicit authored/projected skill input authority. Defaults stay unchanged.
pub const OWNED_RULE_OPERATIONS_V17: &str = "owned-domain-operations-v17";
/// Explicit readiness on one occurrence graph requires stages V2.
pub const OWNED_RULE_OPERATIONS_V16: &str = "owned-domain-operations-v16";
/// Explicit opt-in to source/recipient effect applications. The default remains V14.
pub const OWNED_RULE_OPERATIONS_V15: &str = "owned-domain-operations-v15";
/// Supported operation subsets with frozen semantic capabilities. All use the
/// current package schema; this does not promise old artifact compatibility.
/// Scenario enemy level requires v14, actor support applicability requires v13,
/// assignment/skill preparation scopes
/// require v12, actor-owned ability supply
/// requires v11, ordered modifier transforms v10,
/// equipment receivers v9,
/// QuantizeInteger v8, and character identity v7.
pub const OWNED_RULE_OPERATIONS_V14: &str = "owned-domain-operations-v14";
pub const OWNED_RULE_OPERATIONS_V13: &str = "owned-domain-operations-v13";
pub const OWNED_RULE_OPERATIONS_V12: &str = "owned-domain-operations-v12";
pub const OWNED_RULE_OPERATIONS_V11: &str = "owned-domain-operations-v11";
pub const OWNED_RULE_OPERATIONS_V10: &str = "owned-domain-operations-v10";
pub const OWNED_RULE_OPERATIONS_V9: &str = "owned-domain-operations-v9";
pub const OWNED_RULE_OPERATIONS_V8: &str = "owned-domain-operations-v8";
pub const OWNED_RULE_OPERATIONS_V7: &str = "owned-domain-operations-v7";
pub const OWNED_RULE_OPERATIONS_V6: &str = "owned-domain-operations-v6";

/// Closed native semantic versions. Storage may preserve older opaque version
/// strings, but executable semantics and plan identity require one of these.
/// Capabilities refer to frozen revisions, never to the moving latest alias.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleOperationsVersion {
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    V16,
    V17,
    V18,
    V19,
    V20,
    V21,
    V22,
    V23,
    V24,
}
impl RuleOperationsVersion {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            OWNED_RULE_OPERATIONS_V6 => Self::V6,
            OWNED_RULE_OPERATIONS_V7 => Self::V7,
            OWNED_RULE_OPERATIONS_V8 => Self::V8,
            OWNED_RULE_OPERATIONS_V9 => Self::V9,
            OWNED_RULE_OPERATIONS_V10 => Self::V10,
            OWNED_RULE_OPERATIONS_V11 => Self::V11,
            OWNED_RULE_OPERATIONS_V12 => Self::V12,
            OWNED_RULE_OPERATIONS_V13 => Self::V13,
            OWNED_RULE_OPERATIONS_V14 => Self::V14,
            OWNED_RULE_OPERATIONS_V15 => Self::V15,
            OWNED_RULE_OPERATIONS_V16 => Self::V16,
            OWNED_RULE_OPERATIONS_V17 => Self::V17,
            OWNED_RULE_OPERATIONS_V18 => Self::V18,
            OWNED_RULE_OPERATIONS_V19 => Self::V19,
            OWNED_RULE_OPERATIONS_V20 => Self::V20,
            OWNED_RULE_OPERATIONS_V21 => Self::V21,
            OWNED_RULE_OPERATIONS_V22 => Self::V22,
            OWNED_RULE_OPERATIONS_V23 => Self::V23,
            OWNED_RULE_OPERATIONS_V24 => Self::V24,
            _ => return None,
        })
    }
    pub const fn revision(self) -> u32 {
        match self {
            Self::V6 => 6,
            Self::V7 => 7,
            Self::V8 => 8,
            Self::V9 => 9,
            Self::V10 => 10,
            Self::V11 => 11,
            Self::V12 => 12,
            Self::V13 => 13,
            Self::V14 => 14,
            Self::V15 => 15,
            Self::V16 => 16,
            Self::V17 => 17,
            Self::V18 => 18,
            Self::V19 => 19,
            Self::V20 => 20,
            Self::V21 => 21,
            Self::V22 => 22,
            Self::V23 => 23,
            Self::V24 => 24,
        }
    }
    pub const fn supports_character_identity(self) -> bool {
        self.revision() >= 7
    }
    pub const fn supports_quantize_integer(self) -> bool {
        self.revision() >= 8
    }
    pub const fn supports_equipment_receivers(self) -> bool {
        self.revision() >= 9
    }
    pub const fn supports_modifier_transforms(self) -> bool {
        self.revision() >= 10
    }
    pub const fn supports_actor_supply(self) -> bool {
        self.revision() >= 11
    }
    pub const fn supports_preparation_scopes(self) -> bool {
        self.revision() >= 12
    }
    pub const fn supports_actor_support_applicability(self) -> bool {
        self.revision() >= 13
    }
    pub const fn supports_enemy_level(self) -> bool {
        self.revision() >= 14
    }
    pub const fn supports_effect_applications(self) -> bool {
        self.revision() >= 15
    }
    pub const fn supports_readiness(self) -> bool {
        self.revision() >= 16
    }
    pub const fn supports_skill_inputs(self) -> bool {
        self.revision() >= 17
    }
    pub const fn supports_source_properties(self) -> bool {
        self.revision() >= 18
    }
    pub const fn supports_preset_skill_inputs(self) -> bool {
        self.revision() >= 19
    }
    pub const fn supports_action_selection(self) -> bool {
        self.revision() >= 20
    }
    pub const fn supports_contribution_queries(self) -> bool {
        self.revision() >= 21
    }
    pub const fn supports_boolean_contributions(self) -> bool {
        self.revision() >= 22
    }
    pub const fn supports_actor_reward_contributions(self) -> bool {
        self.revision() >= 23
    }
    pub const fn supports_skill_contribution_queries(self) -> bool {
        self.revision() >= 24
    }
    /// Current explicit Skill participation in V4 readiness metadata.
    pub const fn supports_skill_participation(self) -> bool {
        self.revision() >= 21
    }
    /// Read-only slot state in an explicit existing-Player Actor application.
    pub const fn supports_player_equipment_slots(self) -> bool {
        self.revision() >= 21
    }
    /// Artifact domains are frozen explicitly, even where capabilities overlap.
    pub const fn effect_plan_domain(self) -> &'static str {
        match self {
            Self::V6 | Self::V7 | Self::V8 | Self::V9 => "owned-effect-plan-v6",
            Self::V10 => "owned-effect-plan-v7",
            Self::V11 => "owned-effect-plan-v8",
            Self::V12 => "owned-effect-plan-v9",
            Self::V13 => "owned-effect-plan-v10",
            Self::V14 => "owned-effect-plan-v11",
            Self::V15 => "owned-effect-plan-v12",
            Self::V16 => "owned-effect-plan-v13",
            Self::V17 => "owned-effect-plan-v14",
            Self::V18 => "owned-effect-plan-v15",
            Self::V19 => "owned-effect-plan-v16",
            Self::V20 => "owned-effect-plan-v17",
            Self::V21 => "owned-effect-plan-v18",
            Self::V22 => "owned-effect-plan-v19",
            Self::V23 => "owned-effect-plan-v20",
            Self::V24 => "owned-effect-plan-v21",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulePackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub semantics_version: OwnedDefinitionKey,
    pub operations_version: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    /// Immutable finite data shared by programs in this package.
    pub tables: Vec<IntegerRuleTable>,
    pub owners: Vec<DefinitionRules>,
    /// Explicit applicability; this registry never creates actor/equipment occurrences.
    /// Partial membership remains a coverage gap even if all known rows run.
    pub receivers: DeclaredSet<StatReceiver>,
    /// Explicit inventory required from operations V15. Omission is permitted
    /// only by older operation subsets; Partial never proves an absent effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_applications: Option<DeclaredSet<EffectApplicationRule>>,
    /// Explicit contribution membership inventory, mandatory from operations V21.
    /// This does not close the numerical coverage of any contributing owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution_queries: Option<DeclaredSet<ContributionQuery>>,
    /// Rules applied once to actors already present in the request. Omission is
    /// a known empty applicability inventory, never an inferred actor default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::owned_build::non_null_extension"
    )]
    pub existing_actor_rules: Option<DeclaredSet<ExistingActorRuleApplication>>,
    /// Reviewed support-source domains, checked against the composed provider
    /// graph. Omission is unknown coverage, never an empty origin inventory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_discovery: Option<SupportDiscoveryInput>,
}

/// Definition-side evidence only. Engine binds each selected occurrence before
/// treating authored assignment sequences as complete runtime support origins.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportDiscoveryInput {
    pub providers: Vec<SupportSourceDomainDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportSourceDomainDeclaration {
    pub owner: SchemaSubject,
    pub domain: crate::owned_schema::SchemaState<SupportSourceDomain>,
}

/// Supported source domains, independent of activation and support admission.
/// A positive additional/linked source needs its own future origin authority;
/// today its declaration must remain Unmapped rather than be silently dropped.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportSourceDomain {
    /// This owner introduces no support origins beyond authored assignments
    /// throughout its admitted input domain. It does not permit every recipient.
    AuthoredAssignmentsOnly,
}

/// Explicit applicability of one Actor definition's complete program inventory.
/// This relation creates no actor, authored input, or provider grant.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExistingActorRuleApplication {
    pub id: OwnedDefinitionKey,
    pub owner: ActorDefId,
    pub targets: Vec<ExistingActorRuleTarget>,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExistingActorRuleTarget {
    Player,
}

/// A finite partition of one recipient's contribution channel. Every discovered
/// matching effect must bind exactly once across the entire query, including
/// groups that a particular consumer does not read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionQuery {
    pub id: OwnedDefinitionKey,
    pub stat: StatDefId,
    pub contribution: ContributionKind,
    pub groups: Vec<ContributionGroup>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionGroup {
    pub id: OwnedDefinitionKey,
    pub reduction: ContributionReduction,
    /// Numeric folds require explicit semantic order; Boolean Any forbids ranks.
    pub ordering: ContributionOrdering,
    pub empty: ParameterValue,
    pub members: DeclaredSet<ContributionMember>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionOrdering {
    Ordered,
    Unordered,
}
/// Definition-qualified selection, expanded against actual candidate occurrences.
/// Equal values or definitions never merge distinct provider occurrences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionMember {
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
    pub effect: OwnedDefinitionKey,
    pub origin: ContributionOrigin,
    pub order: Option<ContributionOrder>,
}
/// Compare (source_rank, slot_rank, modifier_position, program_rank, effect_rank).
/// Inapplicable slot/position components are zero. No instance ID or discovery
/// index breaks a tie. Equipment ranks are shared by the group's origin lane;
/// modifier_position comes only from the rolled item's semantic modifier_order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionOrder {
    pub source_rank: u32,
    pub program_rank: u32,
    pub effect_rank: u32,
    pub slot_ranks: Vec<ContributionSlotRank>,
}
/// Explicit source authority. Direct origins require empty grant paths. Supplied
/// Actors and Skills use the already-validated exact supply relation. Skill
/// origins admit only self-contributions, not inherited or support delivery.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContributionOrigin {
    Character,
    Allocation,
    EquipmentUse {
        slots: Vec<EquipmentSlotDefId>,
    },
    ItemModifier {
        slots: Vec<EquipmentSlotDefId>,
    },
    /// The exact applicability declaration of an already-existing Actor owner.
    ExistingActor {
        application: OwnedDefinitionKey,
    },
    /// A selected reward occurrence, authenticated against its Reward owner.
    Reward,
    /// One of these declared Actor slots, reached through its validated grant.
    /// This admits Actor-slot or matching provider-Actor definition owners only.
    SuppliedActor {
        slots: Vec<DeclaredSlot<ActorSlotDefId>>,
    },
    /// Exact Skill self-contributions. One definition/program/effect may serve
    /// both direct authored uses and these explicitly admitted generated supplies.
    /// At least one source permission is required; this grants no inheritance.
    Skill {
        authored: bool,
        supplies: Vec<DeclaredSlot<SkillGrantSlotDefId>>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionSlotRank {
    pub slot: EquipmentSlotDefId,
    pub rank: u32,
}

/// A declaration, not a runtime occurrence or an instruction to create actors.
/// Each discovered source/recipient pair gets independent intermediate state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectApplicationRule {
    pub id: OwnedDefinitionKey,
    pub source: EffectApplicationSource,
    pub targets: Vec<EffectApplicationTarget>,
    /// Boolean program node. False must skip strength reads; unknown is not false.
    pub activation: OwnedDefinitionKey,
    pub program: RuleProgram,
    /// Every contribution effect has exactly one stacking declaration.
    pub stacking: Vec<EffectStackingRule>,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EffectApplicationSource {
    /// Exact already-discovered authored or generated Skill occurrences.
    Skill {
        skill: SkillDefId,
    },
    OwnedSlot {
        slot: DeclaredSlot<ActorSlotDefId>,
    },
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EffectApplicationTarget {
    Player,
    Enemy,
    OwnedSlot { slot: DeclaredSlot<ActorSlotDefId> },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectStackingRule {
    pub effect: OwnedDefinitionKey,
    pub family: OwnedDefinitionKey,
    pub modifier: OwnedDefinitionKey,
    pub reduction: EffectStackingReduction,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStackingReduction {
    /// Fold scaled, rounded, active candidates for an exact recipient/channel.
    /// No candidates emits no contribution; it does not seed a zero maximum.
    /// Unknown candidates block the result. Retain every equal winning origin.
    Maximum,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatReceiver {
    pub id: OwnedDefinitionKey,
    pub stat: StatDefId,
    pub program: OwnedDefinitionKey,
    pub targets: Vec<StatReceiverTarget>,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum StatReceiverTarget {
    Player,
    OwnedSlot {
        slot: DeclaredSlot<ActorSlotDefId>,
    },
    /// Exact template applicability to already-discovered active equipment uses.
    /// No implicit wildcard, source-presence test or item-quality read authority.
    EquipmentTemplate {
        template: ItemTemplateDefId,
    },
}
/// Compatibility names retain existing source and serialized actor representations.
pub type ActorStatReceiver = StatReceiver;
pub type ActorReceiverTarget = StatReceiverTarget;
/// Dense, explicitly bounded integer-keyed scalar data. Each row corresponds to
/// `minimum + row_index`; no interpolation, sparse fallback or endpoint clamping.
/// IDs are local to a rule package, not game-definition or runtime instance IDs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerRuleTable {
    pub id: OwnedDefinitionKey,
    pub minimum: BoundedInteger,
    pub maximum: BoundedInteger,
    pub value_type: ComputedValueType,
    pub rows: Vec<ParameterValue>,
}
impl IntegerRuleTable {
    /// Checked before allocation, including on 32-bit/WASM hosts.
    pub fn domain_size(&self) -> Option<usize> {
        self.maximum
            .get()
            .checked_sub(self.minimum.get())
            .filter(|n| *n >= 0)?
            .checked_add(1)
            .and_then(|n| usize::try_from(n).ok())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionRules {
    pub owner: SchemaSubject,
    /// Partial membership remains unresolved even when every known program runs.
    pub programs: DeclaredSet<RuleProgram>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleProgram {
    /// Stable within the owned declaration, never a runtime provider selector.
    pub id: OwnedDefinitionKey,
    pub context: RuleEntityKind,
    pub reads: Vec<RuleRead>,
    pub nodes: Vec<RuleNode>,
    pub effects: Vec<RuleEffect>,
}
impl RuleProgram {
    pub fn uses_source_property_scopes(&self) -> bool {
        self.reads.iter().any(|read| {
            matches!(
                &read.source,
                RuleReadSource::Stat {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } | RuleReadSource::Capability {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } | RuleReadSource::External {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } | RuleReadSource::Contributions {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } | RuleReadSource::ContributionQuery {
                    entity: RuleEntity::PropertyOwner,
                    ..
                }
            )
        }) || self.effects.iter().any(|effect| {
            matches!(
                &effect.effect,
                RuleEffectKind::Contribute {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } | RuleEffectKind::Derive {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } | RuleEffectKind::Capability {
                    entity: RuleEntity::PropertyOwner,
                    ..
                }
            )
        })
    }
    /// These scopes have authority only inside a declared effect application.
    pub fn uses_effect_application_scopes(&self) -> bool {
        self.reads.iter().any(|read| match &read.source {
            RuleReadSource::EffectSourceParameter { .. }
            | RuleReadSource::EffectSourceChoice { .. } => true,
            RuleReadSource::Stat { entity, .. }
            | RuleReadSource::Capability { entity, .. }
            | RuleReadSource::External { entity, .. }
            | RuleReadSource::Contributions { entity, .. }
            | RuleReadSource::ContributionQuery { entity, .. } => {
                *entity == RuleEntity::EffectSource
            }
            _ => false,
        }) || self.effects.iter().any(|effect| match &effect.effect {
            RuleEffectKind::Contribute { entity, .. }
            | RuleEffectKind::Derive { entity, .. }
            | RuleEffectKind::Capability { entity, .. } => *entity == RuleEntity::EffectSource,
            _ => false,
        })
    }
    /// Feature detection shared by bounded storage and semantic compilation.
    /// It grants no authority to bind the scopes to concrete occurrences.
    pub fn uses_preparation_scopes(&self) -> bool {
        let preparation_entity = |entity: &RuleEntity| {
            matches!(
                entity,
                RuleEntity::SupportOrigin | RuleEntity::Skill | RuleEntity::AssignedSkill
            )
        };
        matches!(
            self.context,
            RuleEntityKind::SupportOrigin | RuleEntityKind::Skill
        ) || self.reads.iter().any(|read| match &read.source {
            RuleReadSource::Stat { entity, .. }
            | RuleReadSource::Capability { entity, .. }
            | RuleReadSource::External { entity, .. }
            | RuleReadSource::Contributions { entity, .. }
            | RuleReadSource::ContributionQuery { entity, .. } => preparation_entity(entity),
            _ => false,
        }) || self.effects.iter().any(|effect| match &effect.effect {
            RuleEffectKind::Contribute { entity, .. }
            | RuleEffectKind::Derive { entity, .. }
            | RuleEffectKind::Capability { entity, .. } => preparation_entity(entity),
            _ => false,
        })
    }
}
/// Relative semantic targets. Current actor comes from the bound actor/action,
/// never a source-selected minion or an index into a UI list.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleEntity {
    Current,
    /// This program's exact modifier occurrence. Semantic compilation permits
    /// this only for a Modifier owner in EquipmentUse context; Current remains
    /// the owning equipment use, not the modifier.
    Modifier,
    Actor,
    Player,
    Enemy,
    Environment,
    /// This support Gem program's exact assignment. Requires SupportOrigin
    /// context, or an explicitly assignment-bound Actor/Action invocation.
    SupportOrigin,
    /// The exact current SkillTarget in Skill/Action context.
    Skill,
    /// This support assignment's target, which can differ from a receiving
    /// action's skill (for example a summoned actor's action).
    AssignedSkill,
    /// Read-only exact source occurrence of a declared effect application.
    /// Current and Actor retain their receiving-entity meaning.
    EffectSource,
    /// Exact input Skill owner of a checked source-property invocation. Producer
    /// Current/Parameter/Gem/Modifier contexts retain their original authority.
    PropertyOwner,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    /// Idempotent Boolean fact; never a numeric stand-in.
    Flag,
    Add,
    Increase,
    Multiply,
}
/// Ordered scalar operations. Both operands use the channel's exact factor unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierTransformOperation {
    Add,
    Multiply,
}
/// Finite recipient definition and optional eligibility value on that recipient.
/// This never grants access to another modifier's raw parameter slots.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifierTransformTarget {
    pub definition: ModifierDefId,
    pub when: Option<StatDefId>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionReduction {
    /// Complete unordered Boolean domain. Every unresolved active member blocks.
    Any,
    Sum,
    Product,
}
/// A selected occurrence's computed outputs, never its raw item parameters.
/// Empty slots have occupancy false and no Stat/Capability value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PlayerEquipmentSlotRead {
    Occupied,
    Stat { stat: StatDefId },
    Capability { capability: CapabilityDefId },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuleReadSource {
    /// Only an already-admitted/projected parameter of the exact source Skill.
    /// This neither reads a physical Gem implicitly nor supplies missing inputs.
    EffectSourceParameter {
        slot: DeclaredSlot<ParameterSlotDefId>,
    },
    EffectSourceChoice {
        slot: DeclaredSlot<ChoiceSlotDefId>,
    },
    Parameter {
        slot: DeclaredSlot<ParameterSlotDefId>,
    },
    Choice {
        slot: DeclaredSlot<ChoiceSlotDefId>,
    },
    CharacterLevel,
    /// Integer level of the validated request's scenario Enemy, independent of
    /// the current actor or invocation context. Requires operations v14.
    EnemyLevel,
    /// The validated request's player character, independent of invocation actor.
    CharacterClassIs {
        class: ClassDefId,
    },
    /// Exact player ascendancy membership; an explicitly absent ascendancy is false.
    CharacterAscendancyIs {
        ascendancy: AscendancyDefId,
    },
    /// Resolve an exact slot in the active Player equipment relation. Requires
    /// operations V21 and an explicit existing-Player Actor rule application.
    /// This does not establish equipment legality or supply a missing value.
    PlayerEquipmentSlot {
        slot: EquipmentSlotDefId,
        read: PlayerEquipmentSlotRead,
    },
    /// Exact selected part of the current Action, not its source Skill or a
    /// reference UI selection. Requires Action context and operations v20.
    /// Matching a known definition never grants availability or membership.
    ActionPartIs {
        part: ActionPartDefId,
    },
    /// Exact selected mode of the current Action. Same authority as ActionPartIs.
    ActionModeIs {
        mode: ActionModeDefId,
    },
    /// Exact selected stat set of the current Action. Selection is already
    /// validated by the owned request; there is no source ordinal or fallback.
    ActionStatSetIs {
        stat_set: ActionStatSetDefId,
    },
    GemLevel,
    ItemLevel,
    /// Amount is available only for this explicitly selected quality kind.
    /// HasQuality guards let the package declare its own absent-quality outcome.
    ItemQualityAmount {
        quality: QualityDefId,
    },
    HasItemQuality {
        quality: QualityDefId,
    },
    GemQualityAmount {
        quality: QualityDefId,
    },
    HasGemQuality {
        quality: QualityDefId,
    },
    Stat {
        entity: RuleEntity,
        stat: StatDefId,
    },
    /// Fold the complete ordered channel for this exact modifier occurrence,
    /// starting from its own initial factor stat. An empty sequence needs proven
    /// completeness; this is never an implicit identity for missing input.
    ModifierTransforms {
        stat: StatDefId,
        initial: StatDefId,
    },
    Capability {
        entity: RuleEntity,
        capability: CapabilityDefId,
    },
    External {
        entity: RuleEntity,
        input: ExternalInputDefId,
    },
    /// Numeric direct reduction. Boolean channels require ContributionQuery for
    /// explicit complete membership. Empty identity is never a missing-stat default.
    Contributions {
        entity: RuleEntity,
        stat: StatDefId,
        contribution: ContributionKind,
        reduction: ContributionReduction,
        empty: ParameterValue,
    },
    /// Read one named group of an explicitly declared candidate-bound query.
    /// Group arithmetic after the fold remains ordinary typed rule expressions.
    ContributionQuery {
        entity: RuleEntity,
        query: OwnedDefinitionKey,
        group: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleRead {
    pub id: OwnedDefinitionKey,
    pub value_type: ComputedValueType,
    pub source: RuleReadSource,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleComparison {
    Equal,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleRounding {
    Floor,
    Ceiling,
    Truncate,
    NearestTiesPositive,
}
/// Outputs of the bounded ordinary direct-action timing algorithm. Each channel
/// has independent finite availability; an uncapped infinity does not erase a
/// finite capped rate or time. The recipe selects this algorithm, not other
/// trigger/channel/cooldown/reload branches.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdinaryTimingChannel {
    SpeedMultiplier,
    PreCapRate,
    ActionRate,
    ActionTime,
}

/// Explicit assertion that rate is measured per one of the selected time units.
/// Equal dimensions alone never select this pair or authorize unit conversion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReciprocalTimingUnits {
    pub time: UnitDefId,
    pub rate: UnitDefId,
}

/// All inputs are expression references and must be available for this atomic
/// algorithm. Rounding and reciprocal/cap order are versioned native semantics;
/// game values and selection of the timing branch remain injected rule data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryTimingRecipe {
    pub base_time: OwnedDefinitionKey,
    pub increased_percent: OwnedDefinitionKey,
    pub more_multiplier: OwnedDefinitionKey,
    pub additional_attack_time: OwnedDefinitionKey,
    pub additional_cast_time: OwnedDefinitionKey,
    pub action_speed_multiplier: OwnedDefinitionKey,
    pub repeats: OwnedDefinitionKey,
    pub server_tick_rate: OwnedDefinitionKey,
    pub speed_multiplier_rounding_precision: u32,
    pub units: ReciprocalTimingUnits,
    pub output: OrdinaryTimingChannel,
}
impl OrdinaryTimingRecipe {
    pub fn inputs(&self) -> [&OwnedDefinitionKey; 8] {
        [
            &self.base_time,
            &self.increased_percent,
            &self.more_multiplier,
            &self.additional_attack_time,
            &self.additional_cast_time,
            &self.action_speed_multiplier,
            &self.repeats,
            &self.server_tick_rate,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleExpression {
    /// The key is an integer expression. A known out-of-domain key is unsupported,
    /// distinct from a missing key or an arithmetic failure.
    LookupIntegerTable {
        table: OwnedDefinitionKey,
        key: OwnedDefinitionKey,
    },
    OrdinaryTiming {
        recipe: Box<OrdinaryTimingRecipe>,
    },
    Literal {
        value: ParameterValue,
    },
    Read {
        input: OwnedDefinitionKey,
    },
    Add {
        left: OwnedDefinitionKey,
        right: OwnedDefinitionKey,
    },
    Subtract {
        left: OwnedDefinitionKey,
        right: OwnedDefinitionKey,
    },
    Minimum {
        left: OwnedDefinitionKey,
        right: OwnedDefinitionKey,
    },
    Maximum {
        left: OwnedDefinitionKey,
        right: OwnedDefinitionKey,
    },
    /// Multiplication by a declared dimensionless quantity, not arbitrary units.
    Scale {
        value: OwnedDefinitionKey,
        factor: OwnedDefinitionKey,
    },
    ScaleInteger {
        value: OwnedDefinitionKey,
        count: OwnedDefinitionKey,
    },
    DivideFactor {
        value: OwnedDefinitionKey,
        divisor: OwnedDefinitionKey,
    },
    /// Exact same-unit ratio with a declared dimensionless result unit.
    Ratio {
        numerator: OwnedDefinitionKey,
        denominator: OwnedDefinitionKey,
        unit: UnitDefId,
    },
    /// Percentage points divided by 100, with an explicit dimensionless unit.
    PercentAsFactor {
        percent: OwnedDefinitionKey,
        unit: UnitDefId,
    },
    Round {
        value: OwnedDefinitionKey,
        quantum: FiniteQuantity,
        mode: RuleRounding,
    },
    /// Count explicit positive, exact-unit quanta using the selected rounding
    /// mode. Returns a bounded Integer; overflow is a numerical failure, never
    /// saturation. ScaleInteger with the same quantum converts a count back to
    /// a quantity without an implicit unit conversion.
    QuantizeInteger {
        value: OwnedDefinitionKey,
        quantum: FiniteQuantity,
        mode: RuleRounding,
    },
    Compare {
        operation: RuleComparison,
        left: OwnedDefinitionKey,
        right: OwnedDefinitionKey,
    },
    Not {
        value: OwnedDefinitionKey,
    },
    All {
        values: Vec<OwnedDefinitionKey>,
    },
    Any {
        values: Vec<OwnedDefinitionKey>,
    },
    /// Lazy branch selection; an inactive branch never requests missing inputs.
    Select {
        condition: OwnedDefinitionKey,
        when_true: OwnedDefinitionKey,
        when_false: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleNode {
    pub id: OwnedDefinitionKey,
    pub expression: RuleExpression,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleEffectKind {
    Contribute {
        entity: RuleEntity,
        stat: StatDefId,
        contribution: ContributionKind,
        value: OwnedDefinitionKey,
    },
    /// Project an ordered scalar operation to finite modifier definitions on
    /// the same item and receiving equipment use. The item modifier order comes
    /// first, then this explicit nonnegative step within a producer occurrence.
    /// Eligibility is read on each exact recipient before demanding the value.
    ProjectModifierTransform {
        stat: StatDefId,
        targets: Vec<ModifierTransformTarget>,
        order: BoundedInteger,
        operation: ModifierTransformOperation,
        value: OwnedDefinitionKey,
    },
    /// A final stat producer. The resolver rejects competing producers and
    /// unsupported inter-program dependency cycles instead of choosing one.
    Derive {
        entity: RuleEntity,
        stat: StatDefId,
        value: OwnedDefinitionKey,
    },
    Capability {
        entity: RuleEntity,
        capability: CapabilityDefId,
        enabled: OwnedDefinitionKey,
    },
    SupportApplicability {
        applicable: OwnedDefinitionKey,
    },
    ActivateGrant {
        slot: DeclaredSlot<GrantSlotDefId>,
        enabled: OwnedDefinitionKey,
    },
    /// Project a computed input into this owner's declared generated skill.
    /// The target parameter belongs to that exact Skill definition. This emits
    /// a value only; concrete occurrence binding and grant activation belong to
    /// the resolver, which also checks complete required-input/producer coverage.
    ProjectSkillParameter {
        skill: DeclaredSlot<SkillGrantSlotDefId>,
        parameter: DeclaredSlot<ParameterSlotDefId>,
        value: OwnedDefinitionKey,
    },
    /// Project a computed stat into this owner's exact declared child actor.
    /// The stat must support Actor targets with the same value type/unit. This
    /// emits a value only: the resolver binds the parent provider occurrence,
    /// proves producer coverage and resolves activation separately.
    ProjectActorStat {
        actor: DeclaredSlot<ActorSlotDefId>,
        stat: StatDefId,
        value: OwnedDefinitionKey,
    },
    Requirement {
        satisfied: OwnedDefinitionKey,
        code: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleEffect {
    pub id: OwnedDefinitionKey,
    pub when: Option<OwnedDefinitionKey>,
    pub effect: RuleEffectKind,
}
