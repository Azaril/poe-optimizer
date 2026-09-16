//! Portable, injected action/stat transport declarations. No occurrence selection or effects.
use crate::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_definitions::*, owned_schema::DeclaredSet,
};
use serde::{Deserialize, Deserializer, Serialize, de};

pub const OWNED_ACTION_ROUTING_VERSION: u32 = 2;
pub const OWNED_ACTION_ROUTING_V1: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRoutingInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub outputs: Vec<ActionOutputRoutes>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOutputRoutes {
    pub output: DeclaredSlot<ActionOutputDefId>,
    /// Missing output entries are unknown, not equivalent to a complete empty set.
    pub routes: DeclaredSet<ActionStatRoute>,
    /// Absent only in v1; v2 declares even an explicitly empty selector set.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "non_null_selectors"
    )]
    pub source_selectors: Option<DeclaredSet<ActionSourceSelector>>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionStatRoute {
    /// Local to this exact output declaration. Never an occurrence identity.
    pub id: OwnedDefinitionKey,
    pub selection: ActionRouteSelection,
    pub source: ActionStatRouteSource,
    pub target: StatDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActionRouteSelection {
    /// Every schema-valid selection of this output; no default part/mode/stat-set.
    All,
    Exact(Box<ActionRouteSelector>),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRouteSelector {
    pub part: ActionPartDefId,
    pub mode: ActionModeDefId,
    pub stat_set: ActionStatSetDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActionStatRouteSource {
    /// The resolver must establish an active player equipment use in this slot.
    PlayerEquipment {
        slot: EquipmentSlotDefId,
        stat: StatDefId,
    },
    /// The exact selected action's actor, which is not necessarily the player.
    ActionActor { stat: StatDefId },
    Selected {
        selector: OwnedDefinitionKey,
        stats: Vec<ActionSourceStat>,
    },
}

fn non_null_selectors<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<DeclaredSet<ActionSourceSelector>>, D::Error> {
    Option::<DeclaredSet<ActionSourceSelector>>::deserialize(d)?
        .map(Some)
        .ok_or_else(|| {
            de::Error::custom("source_selectors must be omitted or a non-null declaration")
        })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSourceSelector {
    pub id: OwnedDefinitionKey,
    pub selection: ActionRouteSelection,
    pub sources: Vec<NamedActionSource>,
    pub policy: ActionSourcePolicy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedActionSource {
    pub id: OwnedDefinitionKey,
    pub origin: ActionSourceOrigin,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActionSourceOrigin {
    ActionActor,
    CurrentAction,
    PlayerEquipment { slot: EquipmentSlotDefId },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActionSourcePolicy {
    Fixed {
        source: OwnedDefinitionKey,
    },
    EquipmentEligibility {
        source: OwnedDefinitionKey,
        capability: CapabilityDefId,
        when_empty: ActionSourceOutcome,
        when_ineligible: ActionSourceOutcome,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActionSourceOutcome {
    Use { source: OwnedDefinitionKey },
    Unavailable,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSourceStat {
    pub source: OwnedDefinitionKey,
    pub stat: StatDefId,
}
