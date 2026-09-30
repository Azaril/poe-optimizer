//! Injected support selection and type-preparation data, independent of any source format.
//!
//! This package describes preparation only. It does not authorize receiving effects,
//! prove active contributors complete, or supply effective gem values. Symbols in its
//! type/effect/family vocabularies are local to the exact package identity.
use crate::{
    data::DataIdentity, owned_content::OwnedContentDigest, owned_definitions::*,
    owned_schema::SchemaState,
};
use serde::{Deserialize, Deserializer, Serialize};

pub const OWNED_SUPPORT_PREPARATION_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportPreparationPolicy {
    /// Ordered replacement, retained duplicate positions, and bounded retry passes
    /// stopping at the first position accepted on an earlier retry pass.
    OrderedReplacementRetryFrontierV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportPreparationInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub policy: SupportPreparationPolicy,
    pub quality_unit: UnitDefId,
    pub types: Vec<OwnedDefinitionKey>,
    pub effects: Vec<OwnedDefinitionKey>,
    pub families: Vec<OwnedDefinitionKey>,
    pub supports: Vec<SupportPreparationEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportPreparationEntry {
    pub gem: GemDefId,
    pub preparation: SchemaState<SupportPreparationDefinition>,
}

// Null is a declared absence. Omission is not conversion evidence.
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportPreparationDefinition {
    pub effect: OwnedDefinitionKey,
    /// None and Some(empty) have different selection semantics.
    #[serde(deserialize_with = "required_option")]
    pub families: Option<Vec<OwnedDefinitionKey>>,
    #[serde(deserialize_with = "required_option")]
    pub plus_version_of: Option<OwnedDefinitionKey>,
    #[serde(deserialize_with = "required_option")]
    pub requires: Option<SupportTypePredicate>,
    #[serde(deserialize_with = "required_option")]
    pub excludes: Option<SupportTypePredicate>,
    pub added_types: Vec<OwnedDefinitionKey>,
    pub gems_only: bool,
    pub from_item: bool,
    pub is_support: bool,
    pub is_trigger: bool,
    pub ignore_minion_types: bool,
}

/// Typed Boolean expressions. A source adapter must translate its own expression
/// format (including residual-stack semantics) before constructing this data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SupportTypePredicate {
    Type(OwnedDefinitionKey),
    Any(Vec<Self>),
    All(Vec<Self>),
    Not(Box<Self>),
}
