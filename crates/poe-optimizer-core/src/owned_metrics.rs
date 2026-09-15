//! Injected semantic relationships from requested metrics to final computed stats.
//! No occurrence selection, activation, contribution closure or numerical value.
use crate::{data::DataIdentity, owned_definitions::*};
use serde::{Deserialize, Serialize};

pub const OWNED_METRIC_MAPPING_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricMappingInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    /// A missing key has no declared mapping. This vector makes no catalog
    /// completeness claim, including when empty.
    pub bindings: Vec<MetricStatBinding>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricBindingRole {
    PlayerActor,
    OwnedActor,
    /// Player/owned actor and concrete provider compatibility bind with the
    /// actual action selector; this is not an unrestricted action proof.
    Action,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricStatBinding {
    pub metric: MetricDefId,
    pub role: MetricBindingRole,
    /// Only an exact Quantity(unit) stat can supply the metric's declared unit.
    pub stat: StatDefId,
}
