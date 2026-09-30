//! Bounded, immutable support preparation definitions. No source runtime or effect delivery.
use crate::owned_rules::OwnedRulePackage;
use poe_optimizer_core::{
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
    owned_supports::*,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const DOMAIN: &str = "owned-support-preparation-v1";

#[derive(Clone, Copy, Debug)]
pub struct SupportStorageLimits {
    pub max_entries: usize,
    pub max_work: usize,
    pub max_predicate_depth: usize,
    pub max_wire_bytes: usize,
}
impl Default for SupportStorageLimits {
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_work: 2_000_000,
            max_predicate_depth: 32,
            max_wire_bytes: 16 * 1024 * 1024,
        }
    }
}
impl SupportStorageLimits {
    pub fn validate(self) -> Result<(), SupportStorageError> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("entries", self.max_entries, hard.max_entries),
            ("work", self.max_work, hard.max_work),
            (
                "predicate depth",
                self.max_predicate_depth,
                hard.max_predicate_depth,
            ),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(SupportStorageError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupportStorageError {
    #[error("invalid support storage limit: {0}")]
    InvalidLimit(&'static str),
    #[error("support preparation package exceeds {0}")]
    Limit(&'static str),
    #[error("unsupported support preparation version {0}")]
    Version(u32),
    #[error("support preparation schema/rule binding mismatch")]
    Binding,
    #[error("invalid support preparation: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SupportStorageUse {
    pub entries: usize,
    pub work: usize,
    pub predicate_depth: usize,
}
impl SupportStorageUse {
    fn check(self, limits: SupportStorageLimits) -> Result<(), SupportStorageError> {
        for (name, used, limit) in [
            ("entries", self.entries, limits.max_entries),
            ("work", self.work, limits.max_work),
            (
                "predicate depth",
                self.predicate_depth,
                limits.max_predicate_depth,
            ),
        ] {
            if used > limit {
                return Err(SupportStorageError::Limit(name));
            }
        }
        Ok(())
    }
    fn charge(
        &mut self,
        count: usize,
        limits: SupportStorageLimits,
    ) -> Result<(), SupportStorageError> {
        self.entries = self
            .entries
            .checked_add(count)
            .ok_or(SupportStorageError::Limit("entries"))?;
        self.work = self
            .work
            .checked_add(count)
            .ok_or(SupportStorageError::Limit("work"))?;
        self.check(limits)
    }
}

fn unique<'a, T: Ord>(
    entries: &'a [T],
    used: &mut SupportStorageUse,
    limits: SupportStorageLimits,
) -> Result<BTreeSet<&'a T>, SupportStorageError> {
    used.charge(entries.len(), limits)?;
    let set: BTreeSet<_> = entries.iter().collect();
    if set.len() != entries.len() {
        return Err(SupportStorageError::Invalid("duplicate membership"));
    }
    Ok(set)
}
fn known<T>(lookup: SchemaLookup<'_, T>) -> Result<&T, SupportStorageError> {
    match lookup {
        SchemaLookup::Known(v) => Ok(v),
        _ => Err(SupportStorageError::Invalid(
            "missing, foreign or unmapped schema reference",
        )),
    }
}
fn member(
    value: &OwnedDefinitionKey,
    set: &BTreeSet<&OwnedDefinitionKey>,
) -> Result<(), SupportStorageError> {
    if !set.contains(value) {
        return Err(SupportStorageError::Invalid(
            "undeclared preparation symbol",
        ));
    }
    Ok(())
}
fn predicate(
    value: &SupportTypePredicate,
    types: &BTreeSet<&OwnedDefinitionKey>,
    depth: usize,
    used: &mut SupportStorageUse,
    limits: SupportStorageLimits,
) -> Result<(), SupportStorageError> {
    used.predicate_depth = used.predicate_depth.max(depth);
    used.charge(1, limits)?;
    match value {
        SupportTypePredicate::Type(v) => member(v, types)?,
        SupportTypePredicate::Any(values) | SupportTypePredicate::All(values) => {
            // Bound width before recursively walking or allocating any additional storage.
            if values.len() > limits.max_entries.saturating_sub(used.entries) {
                return Err(SupportStorageError::Limit("entries"));
            }
            for v in values {
                predicate(v, types, depth + 1, used, limits)?;
            }
        }
        SupportTypePredicate::Not(v) => predicate(v, types, depth + 1, used, limits)?,
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct OwnedSupportPreparation {
    input: SupportPreparationInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: SupportStorageUse,
}
impl OwnedSupportPreparation {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: SupportPreparationInput,
        index: &I,
        rules: &OwnedRulePackage,
        limits: SupportStorageLimits,
    ) -> Result<Self, SupportStorageError> {
        limits.validate()?;
        if input.schema_version != OWNED_SUPPORT_PREPARATION_VERSION {
            return Err(SupportStorageError::Version(input.schema_version));
        }
        if input.definitions != *index.identity()
            || input.namespace != *index.namespace()
            || input.definitions != *rules.definitions()
            || input.rules != *rules.identity()
            || input.namespace != rules.input().namespace
            || input.definitions.validate().is_err()
        {
            return Err(SupportStorageError::Binding);
        }
        let mut used = SupportStorageUse::default();
        used.charge(1, limits)?;
        if known(index.definition(&input.quality_unit))?.dimension
            != UnitDimension::PercentagePoints
        {
            return Err(SupportStorageError::Invalid(
                "support quality requires a percentage-points unit",
            ));
        }
        let types = unique(&input.types, &mut used, limits)?;
        let effects = unique(&input.effects, &mut used, limits)?;
        let families = unique(&input.families, &mut used, limits)?;
        used.charge(input.supports.len(), limits)?;
        let mut gems = BTreeSet::new();
        let mut definition_costs = BTreeMap::new();
        for row in &input.supports {
            known(index.definition(&row.gem))?;
            if !gems.insert(&row.gem) {
                return Err(SupportStorageError::Invalid("duplicate support gem"));
            }
            match &row.preparation {
                SchemaState::Unmapped { gaps } => {
                    used.charge(gaps.len(), limits)?;
                    if gaps.is_empty() {
                        return Err(SupportStorageError::Invalid(
                            "unmapped support requires gaps",
                        ));
                    }
                    let mut codes = BTreeSet::new();
                    for gap in gaps {
                        if gap.subject != SchemaSubject::Definition(row.gem.address())
                            || !codes.insert(&gap.code)
                        {
                            return Err(SupportStorageError::Invalid(
                                "invalid or duplicate support gap",
                            ));
                        }
                    }
                }
                SchemaState::Known(definition) => {
                    let start = used.work;
                    used.charge(1, limits)?;
                    member(&definition.effect, &effects)?;
                    if let Some(v) = &definition.plus_version_of {
                        used.charge(1, limits)?;
                        member(v, &effects)?;
                    }
                    if let Some(values) = &definition.families {
                        for v in unique(values, &mut used, limits)? {
                            member(v, &families)?;
                        }
                    }
                    for v in unique(&definition.added_types, &mut used, limits)? {
                        member(v, &types)?;
                    }
                    for v in [&definition.requires, &definition.excludes]
                        .into_iter()
                        .flatten()
                    {
                        predicate(v, &types, 1, &mut used, limits)?;
                    }
                    definition_costs.insert(row.gem.clone(), used.work - start);
                }
            }
        }
        // Cardinality and recursive depth are checked before serialization. The digest
        // writer enforces the byte limit before allocating the canonical wire buffer.
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        input.types.sort();
        input.effects.sort();
        input.families.sort();
        input.supports.sort_by(|a, b| a.gem.cmp(&b.gem));
        for row in &mut input.supports {
            match &mut row.preparation {
                SchemaState::Known(v) => {
                    if let Some(families) = &mut v.families {
                        families.sort();
                    }
                    v.added_types.sort();
                }
                SchemaState::Unmapped { gaps } => gaps.sort_by(|a, b| a.code.cmp(&b.code)),
            }
        }
        let mut effect_definitions = BTreeMap::new();
        for row in &input.supports {
            if let SchemaState::Known(definition) = &row.preparation
                && let Some(previous) = effect_definitions.insert(&definition.effect, definition)
            {
                used.work = used
                    .work
                    .checked_add(definition_costs[&row.gem])
                    .ok_or(SupportStorageError::Limit("work"))?;
                used.check(limits)?;
                if previous != definition {
                    return Err(SupportStorageError::Invalid(
                        "conflicting facts for the same support effect",
                    ));
                }
            }
        }
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources: used,
        })
    }
    pub fn input(&self) -> &SupportPreparationInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> SupportStorageUse {
        self.resources
    }
    pub fn preparation_for(
        &self,
        gem: &GemDefId,
    ) -> Option<&SchemaState<SupportPreparationDefinition>> {
        self.input
            .supports
            .binary_search_by(|row| row.gem.cmp(gem))
            .ok()
            .map(|position| &self.input.supports[position].preparation)
    }
    pub fn validate_limits(&self, limits: SupportStorageLimits) -> Result<(), SupportStorageError> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(SupportStorageError::Limit("bytes"));
        }
        Ok(())
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        rules: &OwnedRulePackage,
    ) -> Result<(), SupportStorageError> {
        if self.input.definitions != *index.identity()
            || self.input.namespace != *index.namespace()
            || self.input.rules != *rules.identity()
            || self.input.definitions != *rules.definitions()
            || self.input.namespace != rules.input().namespace
        {
            return Err(SupportStorageError::Binding);
        }
        Ok(())
    }
}
pub fn decode_support_preparation<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    rules: &OwnedRulePackage,
    limits: SupportStorageLimits,
) -> Result<OwnedSupportPreparation, SupportStorageError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(SupportStorageError::Limit("bytes"));
    }
    OwnedSupportPreparation::new(serde_json::from_slice(bytes)?, index, rules, limits)
}
pub fn encode_support_preparation(
    package: &OwnedSupportPreparation,
    limits: SupportStorageLimits,
) -> Result<Vec<u8>, SupportStorageError> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
