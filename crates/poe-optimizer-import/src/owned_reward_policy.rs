//! Injected finite reward selection over caller-collected configuration facts.
//!
//! Rules always describe a partial reward-catalog contribution, including when
//! the list is empty. The caller proves candidate scope/presence, preserves any
//! unconverted membership, and allocates owned occurrences. This module neither
//! inspects source trees nor evaluates game rules or source programs.
use crate::{owned_mapping::*, owned_value::ValueCodecKind, owned_value_policy::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::{ParameterAssignment, ParameterValue},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_REWARD_POLICY_VERSION: u32 = 1;
const DIGEST_DOMAIN: &str = "owned-reward-policy-v1";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RewardValue {
    Boolean(bool),
    Option(OptionDefId),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RewardTemplate {
    None,
    Reward {
        selector: ExternalSelector,
        parameters: Vec<ParameterAssignment>,
    },
    Unmapped {
        issue: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardOutcomeCase {
    pub when: RewardValue,
    pub outcome: RewardTemplate,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardRuleInput {
    pub recipe: ValueRecipeInput,
    pub outcomes: Vec<RewardOutcomeCase>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardPolicyInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub version: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub mapping: OwnedContentDigest,
    pub rules: Vec<RewardRuleInput>,
}

#[derive(Clone, Copy, Debug)]
pub struct RewardPolicyLimits {
    pub value: ValuePolicyLimits,
    pub max_rules: usize,
    /// Aggregate across all rules.
    pub max_outcomes: usize,
    /// Aggregate across all outcome templates.
    pub max_parameters: usize,
    pub max_schema_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for RewardPolicyLimits {
    fn default() -> Self {
        Self {
            value: ValuePolicyLimits::default(),
            max_rules: 256,
            max_outcomes: 4_096,
            max_parameters: 16_384,
            max_schema_work: 1_000_000,
            max_wire_bytes: 8 * 1024 * 1024,
        }
    }
}
impl RewardPolicyLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("rules", self.max_rules, hard.max_rules),
            ("outcomes", self.max_outcomes, hard.max_outcomes),
            ("parameters", self.max_parameters, hard.max_parameters),
            ("schema work", self.max_schema_work, hard.max_schema_work),
            ("wire bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(RewardPolicyError::InvalidLimit(name));
            }
        }
        self.value.validate()?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardPolicyViolation {
    UnsupportedRecipe,
    UnsupportedLane,
    WrongWhenKind,
    DuplicateRule,
    DuplicateOutcome,
    DuplicateParameter,
    ForeignNamespace,
    UnsupportedRewardSelector,
    WrongMappingDomain,
    WrongParameterOwner,
    ParameterNotDeclared,
    WrongParameterSite,
    WrongValueKind,
    WrongUnit,
    OutOfRange,
    OptionNotAllowed,
    RequiredParameterMissing,
}
#[derive(Debug, thiserror::Error)]
pub enum RewardPolicyError {
    #[error("unsupported reward policy version {0}")]
    UnsupportedVersion(u32),
    #[error("invalid reward policy limit: {0}")]
    InvalidLimit(&'static str),
    #[error("reward policy exceeds {0}")]
    Limit(&'static str),
    #[error("reward policy artifact bindings disagree")]
    Binding,
    #[error("unknown reward rule: {0:?}")]
    UnknownRule(OwnedDefinitionKey),
    #[error("{path}: {violation:?}")]
    Invalid {
        path: String,
        violation: RewardPolicyViolation,
    },
    #[error("inconsistent schema index at {subject:?}")]
    IndexFault { subject: Box<SchemaSubject> },
    #[error(transparent)]
    Value(#[from] ValuePolicyError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, RewardPolicyError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RewardUnmappedReason {
    ValuePending,
    ValueAbsent,
    NoMatchingOutcome,
    Explicit { issue: OwnedDefinitionKey },
    MappingMissing,
    MappingUnmapped { issue: OwnedDefinitionKey },
    MappingAmbiguous,
    DefinitionMissing { subject: SchemaSubject },
    SchemaUnmapped { subject: SchemaSubject },
    SchemaPartial { subject: SchemaSubject },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RewardOutcome {
    None,
    Reward {
        definition: RewardDefId,
        parameters: Vec<ParameterAssignment>,
    },
    Unmapped {
        reason: RewardUnmappedReason,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RewardDecision<'a> {
    pub value: ValueDecision<'a>,
    pub outcome: RewardOutcome,
}
#[derive(Clone, Debug)]
struct Rule {
    recipe: ValueRecipe,
    outcomes: BTreeMap<RewardValue, RewardOutcome>,
}
#[derive(Clone, Debug)]
pub struct OwnedRewardPolicy {
    input: RewardPolicyInput,
    identity: OwnedContentDigest,
    rules: BTreeMap<OwnedDefinitionKey, Rule>,
}

fn invalid<T>(path: &str, violation: RewardPolicyViolation) -> Result<T> {
    Err(RewardPolicyError::Invalid {
        path: path.into(),
        violation,
    })
}
fn pending(reason: RewardUnmappedReason) -> RewardOutcome {
    RewardOutcome::Unmapped { reason }
}
fn note(first: &mut Option<RewardUnmappedReason>, reason: RewardUnmappedReason) {
    if first.is_none() {
        *first = Some(reason);
    }
}
fn charge(remaining: &mut usize, amount: usize, name: &'static str) -> Result<()> {
    *remaining = remaining
        .checked_sub(amount)
        .ok_or(RewardPolicyError::Limit(name))?;
    Ok(())
}
fn bindings<I: DefinitionSchemaIndex>(
    input: &RewardPolicyInput,
    mappings: &OwnedMappingIndex,
    schema: &I,
) -> Result<()> {
    if input.mapping != *mappings.identity()
        || input.definitions != *schema.identity()
        || input.namespace != *schema.namespace()
        || input.namespace != mappings.input().namespace
        || input.definitions != mappings.input().definitions
        || input.definitions.validate().is_err()
    {
        return Err(RewardPolicyError::Binding);
    }
    Ok(())
}
impl OwnedRewardPolicy {
    pub fn new<I: DefinitionSchemaIndex>(
        input: RewardPolicyInput,
        mappings: &OwnedMappingIndex,
        schema: &I,
        limits: RewardPolicyLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_REWARD_POLICY_VERSION {
            return Err(RewardPolicyError::UnsupportedVersion(input.schema_version));
        }
        bindings(&input, mappings, schema)?;
        if input.rules.len() > limits.max_rules {
            return Err(RewardPolicyError::Limit("rules"));
        }
        let identity = digest_owned(DIGEST_DOMAIN, &input, limits.max_wire_bytes)?;
        let mut outcomes_left = limits.max_outcomes;
        let mut parameters_left = limits.max_parameters;
        let mut checker = SchemaChecker {
            schema,
            namespace: &input.namespace,
            work: limits.max_schema_work,
        };
        let mut rules = BTreeMap::new();
        for (index, rule) in input.rules.iter().enumerate() {
            let path = format!("rules[{index}]");
            if rules.contains_key(&rule.recipe.id) {
                return invalid(&path, RewardPolicyViolation::DuplicateRule);
            }
            if rule.recipe.codec.namespace != input.namespace {
                return invalid(&path, RewardPolicyViolation::ForeignNamespace);
            }
            let boolean = match &rule.recipe.codec.codec {
                ValueCodecKind::Boolean { .. } => true,
                ValueCodecKind::Option { .. } => false,
                _ => return invalid(&path, RewardPolicyViolation::UnsupportedRecipe),
            };
            if rule
                .recipe
                .tiers
                .iter()
                .flat_map(|tier| &tier.selectors)
                .any(|selector| {
                    !matches!(
                        selector.lane,
                        ValueLane::InputBoolean | ValueLane::InputString
                    )
                })
            {
                return invalid(&path, RewardPolicyViolation::UnsupportedLane);
            }
            let recipe = ValueRecipe::new(rule.recipe.clone(), limits.value)?;
            charge(&mut outcomes_left, rule.outcomes.len(), "outcomes")?;
            let mut outcomes = BTreeMap::new();
            for (case_index, case) in rule.outcomes.iter().enumerate() {
                let path = format!("{path}.outcomes[{case_index}]");
                if outcomes.contains_key(&case.when) {
                    return invalid(&path, RewardPolicyViolation::DuplicateOutcome);
                }
                if matches!(case.when, RewardValue::Boolean(_)) != boolean {
                    return invalid(&path, RewardPolicyViolation::WrongWhenKind);
                }
                let mut unresolved = None;
                if let RewardValue::Option(id) = &case.when {
                    checker.namespace(id.namespace(), &path)?;
                    checker.definition(id, &mut unresolved)?;
                }
                let outcome = match &case.outcome {
                    RewardTemplate::None => RewardOutcome::None,
                    RewardTemplate::Unmapped { issue } => pending(RewardUnmappedReason::Explicit {
                        issue: issue.clone(),
                    }),
                    RewardTemplate::Reward {
                        selector,
                        parameters,
                    } => {
                        charge(&mut parameters_left, parameters.len(), "parameters")?;
                        checker.reward(selector, parameters, mappings, &path, &mut unresolved)?
                    }
                };
                outcomes.insert(case.when.clone(), unresolved.map_or(outcome, pending));
            }
            rules.insert(rule.recipe.id.clone(), Rule { recipe, outcomes });
        }
        Ok(Self {
            input,
            identity,
            rules,
        })
    }
    pub fn input(&self) -> &RewardPolicyInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    /// Ordered partial catalog contribution; an empty iterator proves no absence.
    pub fn rules(&self) -> std::slice::Iter<'_, RewardRuleInput> {
        self.input.rules.iter()
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        mappings: &OwnedMappingIndex,
        schema: &I,
    ) -> Result<()> {
        bindings(&self.input, mappings, schema)
    }
    /// The caller must block this call/default inference when source key or
    /// relevant shape uncertainty prevents proving the supplied scope's presence.
    pub fn decide<'a>(
        &self,
        rule_id: &OwnedDefinitionKey,
        candidates: &[ValueCandidate<'a>],
    ) -> Result<RewardDecision<'a>> {
        let rule = self
            .rules
            .get(rule_id)
            .ok_or_else(|| RewardPolicyError::UnknownRule(rule_id.clone()))?;
        let value = rule.recipe.decide(candidates)?;
        let outcome = match &value.outcome {
            ValueOutcome::Selected { value, .. } | ValueOutcome::Defaulted { value } => {
                let key = match value {
                    ParameterValue::Boolean(value) => RewardValue::Boolean(*value),
                    ParameterValue::Option(value) => RewardValue::Option(value.clone()),
                    _ => return invalid("decision", RewardPolicyViolation::UnsupportedRecipe),
                };
                rule.outcomes
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| pending(RewardUnmappedReason::NoMatchingOutcome))
            }
            ValueOutcome::Absent => pending(RewardUnmappedReason::ValueAbsent),
            ValueOutcome::Pending { .. } => pending(RewardUnmappedReason::ValuePending),
        };
        Ok(RewardDecision { value, outcome })
    }
}

struct SchemaChecker<'a, I> {
    schema: &'a I,
    namespace: &'a GameVersionNamespace,
    work: usize,
}
impl<'s, I: DefinitionSchemaIndex> SchemaChecker<'s, I> {
    fn namespace(&self, namespace: &GameVersionNamespace, path: &str) -> Result<()> {
        if namespace != self.namespace {
            return invalid(path, RewardPolicyViolation::ForeignNamespace);
        }
        Ok(())
    }
    fn lookup<'a, T>(
        &mut self,
        result: SchemaLookup<'a, T>,
        subject: SchemaSubject,
        unresolved: &mut Option<RewardUnmappedReason>,
    ) -> Result<Option<&'a T>> {
        charge(&mut self.work, 1, "schema work")?;
        match result {
            SchemaLookup::Known(value) => Ok(Some(value)),
            SchemaLookup::Missing => {
                note(
                    unresolved,
                    RewardUnmappedReason::DefinitionMissing { subject },
                );
                Ok(None)
            }
            SchemaLookup::Unmapped(_) => {
                note(unresolved, RewardUnmappedReason::SchemaUnmapped { subject });
                Ok(None)
            }
            SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                Err(RewardPolicyError::IndexFault {
                    subject: Box::new(subject),
                })
            }
        }
    }
    fn definition<D: SchemaDefinitionId>(
        &mut self,
        id: &D,
        unresolved: &mut Option<RewardUnmappedReason>,
    ) -> Result<Option<&'s D::Descriptor>> {
        let schema = self.schema;
        self.lookup(
            schema.definition(id),
            SchemaSubject::Definition(id.address()),
            unresolved,
        )
    }
    fn value(
        &mut self,
        value: &ParameterValue,
        schema: &ValueSchema,
        subject: &SchemaSubject,
        path: &str,
        unresolved: &mut Option<RewardUnmappedReason>,
    ) -> Result<()> {
        match (value, schema) {
            (ParameterValue::Boolean(_), ValueSchema::Boolean) => {}
            (ParameterValue::Integer(value), ValueSchema::Integer(range)) => {
                if range.minimum > range.maximum {
                    return Err(RewardPolicyError::IndexFault {
                        subject: Box::new(subject.clone()),
                    });
                }
                if value < &range.minimum || value > &range.maximum {
                    return invalid(path, RewardPolicyViolation::OutOfRange);
                }
            }
            (ParameterValue::Quantity(value), ValueSchema::Quantity(range)) => {
                if range.minimum.unit() != range.maximum.unit()
                    || range.minimum.value() > range.maximum.value()
                {
                    return Err(RewardPolicyError::IndexFault {
                        subject: Box::new(subject.clone()),
                    });
                }
                self.namespace(value.unit().namespace(), path)?;
                self.definition(value.unit(), unresolved)?;
                self.definition(range.minimum.unit(), unresolved)?;
                if value.unit() != range.minimum.unit() {
                    return invalid(path, RewardPolicyViolation::WrongUnit);
                }
                if value.value() < range.minimum.value() || value.value() > range.maximum.value() {
                    return invalid(path, RewardPolicyViolation::OutOfRange);
                }
            }
            (ParameterValue::Option(value), ValueSchema::Option { allowed }) => {
                self.namespace(value.namespace(), path)?;
                self.definition(value, unresolved)?;
                charge(&mut self.work, allowed.members.len(), "schema work")?;
                if !allowed.is_complete() {
                    note(
                        unresolved,
                        RewardUnmappedReason::SchemaPartial {
                            subject: subject.clone(),
                        },
                    );
                }
                if !allowed.members.contains(value) && allowed.is_complete() {
                    return invalid(path, RewardPolicyViolation::OptionNotAllowed);
                }
            }
            _ => return invalid(path, RewardPolicyViolation::WrongValueKind),
        }
        Ok(())
    }
    fn reward(
        &mut self,
        selector: &ExternalSelector,
        parameters: &[ParameterAssignment],
        mappings: &OwnedMappingIndex,
        path: &str,
        unresolved: &mut Option<RewardUnmappedReason>,
    ) -> Result<RewardOutcome> {
        if !matches!(
            selector,
            ExternalSelector::Definition(ExternalOwnerSelector::Reward { .. })
                | ExternalSelector::Configuration {
                    role: ConfigMappingRole::Reward,
                    ..
                }
        ) {
            return invalid(path, RewardPolicyViolation::UnsupportedRewardSelector);
        }
        let reward = match mappings.lookup(selector) {
            Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Reward(id)),
                ..
            }) => Some(id),
            Some(MappingOutcome::Mapped { .. }) => {
                return invalid(path, RewardPolicyViolation::WrongMappingDomain);
            }
            Some(MappingOutcome::Ambiguous { .. }) => {
                note(unresolved, RewardUnmappedReason::MappingAmbiguous);
                None
            }
            Some(MappingOutcome::Unmapped { issue }) => {
                note(
                    unresolved,
                    RewardUnmappedReason::MappingUnmapped {
                        issue: issue.clone(),
                    },
                );
                None
            }
            None => {
                note(unresolved, RewardUnmappedReason::MappingMissing);
                None
            }
        };
        let mut supplied = BTreeSet::new();
        for row in parameters {
            self.namespace(row.slot.declaration.namespace(), path)?;
            self.namespace(row.slot.slot.namespace(), path)?;
            if !matches!(&row.slot.declaration, SlotOwnerDefId::Reward(id) if reward.is_none_or(|expected| id == expected))
            {
                return invalid(path, RewardPolicyViolation::WrongParameterOwner);
            }
            if !supplied.insert(&row.slot) {
                return invalid(path, RewardPolicyViolation::DuplicateParameter);
            }
            match &row.value {
                ParameterValue::Option(id) => self.namespace(id.namespace(), path)?,
                ParameterValue::Quantity(value) => {
                    self.namespace(value.unit().namespace(), path)?
                }
                _ => {}
            }
            let subject = SchemaSubject::Slot(ParameterSlotDefId::address(&row.slot));
            if let Some(slot) =
                self.lookup(self.schema.slot(&row.slot), subject.clone(), unresolved)?
            {
                charge(&mut self.work, slot.sites.len(), "schema work")?;
                if !slot.sites.contains(&ParameterSite::RewardParameter) {
                    return invalid(path, RewardPolicyViolation::WrongParameterSite);
                }
                self.value(&row.value, &slot.value, &subject, path, unresolved)?;
            }
        }
        let Some(reward) = reward else {
            return Ok(pending(RewardUnmappedReason::MappingMissing));
        };
        self.namespace(reward.namespace(), path)?;
        let subject = SchemaSubject::Definition(reward.address());
        if let Some(schema) = self.definition(reward, unresolved)? {
            let declarations = &schema.declarations.parameters;
            charge(
                &mut self.work,
                declarations.members.len() + parameters.len(),
                "schema work",
            )?;
            let mut declared = BTreeSet::new();
            if !declarations.is_complete() {
                note(
                    unresolved,
                    RewardUnmappedReason::SchemaPartial {
                        subject: subject.clone(),
                    },
                );
            }
            for slot in &declarations.members {
                if slot.declaration != SlotOwnerDefId::Reward(reward.clone())
                    || !declared.insert(slot)
                {
                    return Err(RewardPolicyError::IndexFault {
                        subject: Box::new(subject.clone()),
                    });
                }
                let slot_subject = SchemaSubject::Slot(ParameterSlotDefId::address(slot));
                if let Some(schema) =
                    self.lookup(self.schema.slot(slot), slot_subject, unresolved)?
                {
                    charge(&mut self.work, schema.sites.len(), "schema work")?;
                    if schema.presence == SlotPresence::RequiredOnce
                        && schema.sites.contains(&ParameterSite::RewardParameter)
                        && !supplied.contains(slot)
                    {
                        return invalid(path, RewardPolicyViolation::RequiredParameterMissing);
                    }
                }
            }
            if declarations.is_complete()
                && parameters.iter().any(|row| !declared.contains(&row.slot))
            {
                return invalid(path, RewardPolicyViolation::ParameterNotDeclared);
            }
        }
        Ok(RewardOutcome::Reward {
            definition: reward.clone(),
            parameters: parameters.to_vec(),
        })
    }
}

pub fn decode_reward_policy<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    mappings: &OwnedMappingIndex,
    schema: &I,
    limits: RewardPolicyLimits,
) -> Result<OwnedRewardPolicy> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(RewardPolicyError::Limit("wire bytes"));
    }
    OwnedRewardPolicy::new(serde_json::from_slice(bytes)?, mappings, schema, limits)
}
pub fn encode_reward_policy(
    policy: &OwnedRewardPolicy,
    limits: RewardPolicyLimits,
) -> Result<Vec<u8>> {
    limits.validate()?;
    if policy.input.rules.len() > limits.max_rules {
        return Err(RewardPolicyError::Limit("rules"));
    }
    let mut outcomes = limits.max_outcomes;
    let mut parameters = limits.max_parameters;
    for rule in policy.rules() {
        charge(&mut outcomes, rule.outcomes.len(), "outcomes")?;
        ValueRecipe::new(rule.recipe.clone(), limits.value)?;
        for case in &rule.outcomes {
            if let RewardTemplate::Reward {
                parameters: rows, ..
            } = &case.outcome
            {
                charge(&mut parameters, rows.len(), "parameters")?;
            }
        }
    }
    digest_owned(DIGEST_DOMAIN, &policy.input, limits.max_wire_bytes)?;
    Ok(serde_json::to_vec(&policy.input)?)
}
