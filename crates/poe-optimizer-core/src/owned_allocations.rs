//! Injected allocation costs and point-budget semantics, independent of source/UI data.
//! These DTOs are not validated packages, progression evidence or access certificates.
use crate::{
    data::DataIdentity,
    owned_definitions::*,
    owned_schema::{DeclaredSet, SchemaState},
};
use serde::{Deserialize, Serialize};

pub const OWNED_ALLOCATION_RULES_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationRulesInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    /// Exact node/pool key. Missing rows mean unknown costs, never free allocation.
    pub costs: Vec<AllocationCost>,
    /// A partial registry cannot certify that every relevant budget was checked.
    pub budgets: DeclaredSet<AllocationBudget>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationCost {
    pub node: PassiveNodeDefId,
    pub pool: PointPoolDefId,
    /// Nonnegative exact points. Zero must be explicitly supplied by game data.
    pub points: BoundedInteger,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationBudget {
    pub id: OwnedDefinitionKey,
    pub pools: Vec<PointPoolDefId>,
    pub usage: AllocationBudgetUsage,
    /// Missing progression conversion does not authorize a maximum or zero.
    pub capacity: SchemaState<AllocationCapacity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AllocationBudgetUsage {
    /// Sum each authored occurrence once, independent of its number of scopes.
    Total,
    SharedPlusMaximumScoped,
    /// Each known alternative is checked, including inactive/zero-use alternatives.
    EachScope {
        include_shared: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AllocationCapacity {
    /// A final Integer PlayerActor stat. Every authored loadout must produce the
    /// same known nonnegative value before it supplies one shared capacity.
    Uniform { stat: StatDefId },
    /// Only EachScope usage admits a separate final capacity in each loadout.
    /// Reading the active loadout once does not establish inactive capacities.
    PerScope { stat: StatDefId },
}
impl AllocationCapacity {
    pub fn stat(&self) -> &StatDefId {
        match self {
            Self::Uniform { stat } | Self::PerScope { stat } => stat,
        }
    }
}
