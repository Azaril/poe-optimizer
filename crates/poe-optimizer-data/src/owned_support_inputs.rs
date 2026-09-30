//! Bounded preparation input bindings. Scheduling and schemas do not prove values complete.
use crate::{
    owned_rules::OwnedRulePackage, owned_stages::OwnedEvaluationStages,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_core::{
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::RuleOperationsVersion,
    owned_schema::*,
    owned_stages::StageChannel,
    owned_support_inputs::*,
};
use serde::Serialize;

const DOMAIN: &str = "owned-support-input-bindings-v1";

#[derive(Clone, Copy, Debug)]
pub struct SupportInputStorageLimits {
    pub max_entries: usize,
    pub max_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for SupportInputStorageLimits {
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_work: 2_000_000,
            max_wire_bytes: 16 * 1024 * 1024,
        }
    }
}
impl SupportInputStorageLimits {
    pub fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("entries", self.max_entries, hard.max_entries),
            ("work", self.max_work, hard.max_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(SupportInputStorageError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupportInputStorageError {
    #[error("invalid support input storage limit: {0}")]
    InvalidLimit(&'static str),
    #[error("support input bindings exceed {0}")]
    Limit(&'static str),
    #[error("unsupported support input bindings version {0}")]
    Version(u32),
    #[error("support input schema/rule/preparation/stage binding mismatch")]
    Binding,
    #[error("invalid support input bindings: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, SupportInputStorageError>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SupportInputStorageUse {
    pub entries: usize,
    pub work: usize,
}
impl SupportInputStorageUse {
    fn check(self, limits: SupportInputStorageLimits) -> Result<()> {
        if self.entries > limits.max_entries {
            return Err(SupportInputStorageError::Limit("entries"));
        }
        if self.work > limits.max_work {
            return Err(SupportInputStorageError::Limit("work"));
        }
        Ok(())
    }
    fn work(&mut self, n: usize, limits: SupportInputStorageLimits) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or(SupportInputStorageError::Limit("work"))?;
        self.check(limits)
    }
    fn entries(&mut self, n: usize, limits: SupportInputStorageLimits) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(n)
            .ok_or(SupportInputStorageError::Limit("entries"))?;
        self.work(n, limits)
    }
}

fn bindings<I: DefinitionSchemaIndex>(
    input: &SupportInputBindingsInput,
    index: &I,
    rules: &OwnedRulePackage,
    preparation: &OwnedSupportPreparation,
    stages: &OwnedEvaluationStages,
) -> Result<()> {
    if input.definitions != *index.identity()
        || input.namespace != *index.namespace()
        || input.rules != *rules.identity()
        || input.preparation != *preparation.identity()
        || input.stages != *stages.identity()
        || input.definitions != *rules.definitions()
        || input.namespace != rules.input().namespace
        || input.definitions != preparation.input().definitions
        || input.namespace != preparation.input().namespace
        || input.rules != preparation.input().rules
        || input.definitions != stages.input().definitions
        || input.namespace != stages.input().namespace
        || input.rules != stages.input().rules
        || input.definitions.validate().is_err()
    {
        return Err(SupportInputStorageError::Binding);
    }
    Ok(())
}

struct Check<'a, I> {
    index: &'a I,
    stages: &'a OwnedEvaluationStages,
    preparation_stage: &'a OwnedDefinitionKey,
    limits: SupportInputStorageLimits,
    used: SupportInputStorageUse,
}
impl<I: DefinitionSchemaIndex> Check<'_, I> {
    fn stat(
        &mut self,
        stat: &StatDefId,
        scope: RuleEntityKind,
        value: &ComputedValueType,
    ) -> Result<()> {
        self.used.work(1, self.limits)?;
        let SchemaLookup::Known(schema) = self.index.definition(stat) else {
            return Err(SupportInputStorageError::Invalid(
                "missing, foreign or unmapped stat",
            ));
        };
        self.used.work(schema.targets.len(), self.limits)?;
        if !schema.targets.contains(&scope) || &schema.value != value {
            return Err(SupportInputStorageError::Invalid(
                "stat scope or value type/unit mismatch",
            ));
        }
        let channel = StageChannel::Stat {
            scope,
            stat: stat.clone(),
        };
        let Some(frozen) = self.stages.frozen_at(&channel) else {
            return Err(SupportInputStorageError::Invalid(
                "input stat channel has no freeze stage",
            ));
        };
        if frozen != self.preparation_stage && !self.stages.precedes(frozen, self.preparation_stage)
        {
            return Err(SupportInputStorageError::Invalid(
                "input stat freezes after or outside preparation stage",
            ));
        }
        Ok(())
    }
    fn boolean(&mut self, stat: &StatDefId) -> Result<()> {
        self.stat(stat, RuleEntityKind::Skill, &ComputedValueType::Boolean)
    }
    fn types(
        &mut self,
        rows: &mut [SupportTypeStat],
        vocabulary: &[OwnedDefinitionKey],
    ) -> Result<()> {
        if rows.len() != vocabulary.len() {
            return Err(SupportInputStorageError::Invalid(
                "type inputs must cover exact preparation vocabulary",
            ));
        }
        // These rows are a finite symbol-to-stat map; only set order canonicalizes.
        self.used.work(rows.len(), self.limits)?;
        rows.sort_unstable_by(|a, b| a.support_type.cmp(&b.support_type));
        let mut previous = None;
        for (row, expected) in rows.iter().zip(vocabulary) {
            if previous == Some(&row.support_type) {
                return Err(SupportInputStorageError::Invalid(
                    "duplicate support type key",
                ));
            }
            previous = Some(&row.support_type);
            if &row.support_type != expected {
                return Err(SupportInputStorageError::Invalid(
                    "type inputs must cover exact preparation vocabulary",
                ));
            }
            self.boolean(&row.stat)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct OwnedSupportInputBindings {
    input: SupportInputBindingsInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: SupportInputStorageUse,
}
impl OwnedSupportInputBindings {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: SupportInputBindingsInput,
        index: &I,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
        stages: &OwnedEvaluationStages,
        limits: SupportInputStorageLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_SUPPORT_INPUT_BINDINGS_VERSION {
            return Err(SupportInputStorageError::Version(input.schema_version));
        }
        bindings(&input, index, rules, preparation, stages)?;
        if !RuleOperationsVersion::parse(rules.input().operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_preparation_scopes)
        {
            return Err(SupportInputStorageError::Invalid(
                "support input bindings require owned-domain-operations-v12",
            ));
        }
        // Streaming byte and aggregate bounds precede copies, sorting and lookups.
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let mut used = SupportInputStorageUse::default();
        used.entries(9, limits)?;
        for rows in [
            &input.target.skill_types,
            &input.target.minion_types.members,
            &input.target.summoner.skill_types,
            &input.target.summoner.minion_types.members,
        ] {
            used.entries(rows.len(), limits)?;
        }
        used.work(stages.input().stages.len(), limits)?;
        if !stages
            .input()
            .stages
            .iter()
            .any(|stage| stage.id == input.preparation_stage)
        {
            return Err(SupportInputStorageError::Invalid(
                "unknown preparation stage",
            ));
        }
        let mut check = Check {
            index,
            stages,
            preparation_stage: &input.preparation_stage,
            limits,
            used,
        };
        check.stat(
            &input.effective_level,
            RuleEntityKind::SupportOrigin,
            &ComputedValueType::Integer,
        )?;
        check.stat(
            &input.effective_quality,
            RuleEntityKind::SupportOrigin,
            &ComputedValueType::Quantity {
                unit: preparation.input().quality_unit.clone(),
            },
        )?;
        for stat in [
            &input.target.cannot_be_supported,
            &input.target.has_gem,
            &input.target.from_item,
            &input.target.is_player_actor,
            &input.target.minion_types.present,
            &input.target.summoner.present,
            &input.target.summoner.minion_types.present,
        ] {
            check.boolean(stat)?;
        }
        let vocabulary = &preparation.input().types;
        for rows in [
            &mut input.target.skill_types,
            &mut input.target.minion_types.members,
            &mut input.target.summoner.skill_types,
            &mut input.target.summoner.minion_types.members,
        ] {
            check.types(rows, vocabulary)?;
        }
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
    pub fn input(&self) -> &SupportInputBindingsInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> SupportInputStorageUse {
        self.resources
    }
    pub fn validate_limits(&self, limits: SupportInputStorageLimits) -> Result<()> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(SupportInputStorageError::Limit("bytes"));
        }
        Ok(())
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
        stages: &OwnedEvaluationStages,
    ) -> Result<()> {
        bindings(&self.input, index, rules, preparation, stages)
    }
}

pub fn decode_support_input_bindings<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    rules: &OwnedRulePackage,
    preparation: &OwnedSupportPreparation,
    stages: &OwnedEvaluationStages,
    limits: SupportInputStorageLimits,
) -> Result<OwnedSupportInputBindings> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(SupportInputStorageError::Limit("bytes"));
    }
    OwnedSupportInputBindings::new(
        serde_json::from_slice(bytes)?,
        index,
        rules,
        preparation,
        stages,
        limits,
    )
}
pub fn encode_support_input_bindings(
    package: &OwnedSupportInputBindings,
    limits: SupportInputStorageLimits,
) -> Result<Vec<u8>> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
