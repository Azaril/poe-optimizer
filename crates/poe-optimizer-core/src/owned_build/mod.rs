//! Owned semantic inputs shared by hosts, future native resolution and search.
//!
//! These contracts neither import source documents nor load a definition package.
//! Structural validity is not game legality, definition coverage or computability.
//! Persisted selectors are symbolic addresses, not private prepared-plan authority.
mod codec;
mod records;
mod structure;

pub use codec::{CodecError, OwnedDocument, decode_owned, encode_owned};
pub use records::*;
use serde::Serialize;
pub use structure::{OccurrenceKind, OwnedInputLimits, StructuralError, StructuralErrorKind};
pub(crate) use structure::{
    RecordTables, RecordTablesMut, StructuralCheck, build_occurrences, canonical_choice_owner,
    canonicalize_choices, canonicalize_item_records, canonicalize_record_tables,
    validate_item_records, validate_record_tables,
};

pub const OWNED_INPUT_SCHEMA_VERSION: u32 = 3;

/// A self-contained, structurally validated build. Construct via `new` or the codec.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BuildSpec(BuildInput);
impl BuildSpec {
    pub fn new(mut input: BuildInput, limits: OwnedInputLimits) -> Result<Self, StructuralError> {
        structure::validate_build(&input, limits)?;
        structure::canonicalize_build(&mut input);
        Ok(Self(input))
    }
    pub fn input(&self) -> &BuildInput {
        &self.0
    }
    pub fn into_input(self) -> BuildInput {
        self.0
    }
}

/// External assumptions and usage policies; no derived character-stat overrides.
/// Definition binding must verify external-input permission, units and scope.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ScenarioSpec(ScenarioInput);
impl ScenarioSpec {
    pub fn new(
        mut input: ScenarioInput,
        limits: OwnedInputLimits,
    ) -> Result<Self, StructuralError> {
        structure::validate_scenario(&input, limits, None)?;
        structure::canonicalize_scenario(&mut input);
        Ok(Self(input))
    }
    pub fn input(&self) -> &ScenarioInput {
        &self.0
    }
    pub fn into_input(self) -> ScenarioInput {
        self.0
    }
}

/// Ordered measurements. A missing/disabled supplying occurrence stays addressable
/// here; definition/resolution binding must report it unavailable, never retarget it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct QuerySpec(QueryInput);
impl QuerySpec {
    pub fn new(input: QueryInput, limits: OwnedInputLimits) -> Result<Self, StructuralError> {
        structure::validate_queries(&input, limits, None)?;
        Ok(Self(input))
    }
    pub fn input(&self) -> &QueryInput {
        &self.0
    }
    pub fn into_input(self) -> QueryInput {
        self.0
    }
}

/// Coherent owned input boundary for future resolution. This does not yet evaluate.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedEvaluationRequest {
    build: BuildSpec,
    scenario: ScenarioSpec,
    queries: QuerySpec,
}
impl OwnedEvaluationRequest {
    pub fn new(
        build: BuildSpec,
        scenario: ScenarioSpec,
        queries: QuerySpec,
        limits: OwnedInputLimits,
    ) -> Result<Self, StructuralError> {
        structure::validate_request(build.input(), scenario.input(), queries.input(), limits)?;
        Ok(Self {
            build,
            scenario,
            queries,
        })
    }
    pub fn validate_limits(&self, limits: OwnedInputLimits) -> Result<(), StructuralError> {
        structure::validate_request(
            self.build.input(),
            self.scenario.input(),
            self.queries.input(),
            limits,
        )
    }
    pub fn build(&self) -> &BuildSpec {
        &self.build
    }
    pub fn scenario(&self) -> &ScenarioSpec {
        &self.scenario
    }
    pub fn queries(&self) -> &QuerySpec {
        &self.queries
    }
    pub fn into_input(self) -> OwnedRequestInput {
        OwnedRequestInput {
            build: self.build.into_input(),
            scenario: self.scenario.into_input(),
            queries: self.queries.into_input(),
        }
    }
}
