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

pub const OWNED_RULE_PACKAGE_VERSION: u32 = 1;
/// Version of the closed operations below, independent of game coefficients.
pub const OWNED_RULE_OPERATIONS_VERSION: &str = "owned-domain-operations-v3";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulePackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub semantics_version: OwnedDefinitionKey,
    pub operations_version: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub owners: Vec<DefinitionRules>,
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
/// Relative semantic targets. Current actor comes from the bound actor/action,
/// never a source-selected minion or an index into a UI list.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleEntity {
    Current,
    Actor,
    Player,
    Enemy,
    Environment,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    Add,
    Increase,
    Multiply,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionReduction {
    Sum,
    Product,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuleReadSource {
    Parameter {
        slot: DeclaredSlot<ParameterSlotDefId>,
    },
    Choice {
        slot: DeclaredSlot<ChoiceSlotDefId>,
    },
    CharacterLevel,
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
    Capability {
        entity: RuleEntity,
        capability: CapabilityDefId,
    },
    External {
        entity: RuleEntity,
        input: ExternalInputDefId,
    },
    /// The resolver proves complete incoming membership before applying a
    /// reduction. The explicit empty identity is never a missing-stat default.
    Contributions {
        entity: RuleEntity,
        stat: StatDefId,
        contribution: ContributionKind,
        reduction: ContributionReduction,
        empty: ParameterValue,
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleExpression {
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
