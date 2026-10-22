//! Shared stateless native fixtures for Engine and their existing external consumers.
//! Every fixture module is compiled once; no test cases or reference VMs live here.

#[path = "owned_computed_support_fixture.rs"]
pub mod computed_support;
#[path = "configuration_resistance_fixture.rs"]
pub mod configuration_resistance;
#[path = "owned_support_delivery_fixture.rs"]
pub mod delivery;
#[path = "owned_empty_support_contribution_fixture.rs"]
pub mod empty_support;
#[path = "owned_metric_fixture.rs"]
pub mod metrics;
#[path = "minion_attack_source_fixture.rs"]
pub mod minion_attack;
#[path = "pain_offering_fixture.rs"]
pub mod pain_offering;
#[path = "owned_skill_participation_fixture.rs"]
pub mod participation;
#[path = "plain_minion_damage_fixture.rs"]
pub mod plain_minion;
#[path = "owned_plan_fixture.rs"]
pub mod plan;
#[path = "owned_preparation_readiness_fixture.rs"]
pub mod readiness;
#[path = "owned_rule_fixture.rs"]
pub mod rules;
#[path = "owned_source_properties_fixture.rs"]
pub mod source_properties;
#[path = "owned_support_metric_fixture.rs"]
pub mod support_metrics;
#[path = "owned_support_output_fixture.rs"]
pub mod support_outputs;
// The empty-contribution fixture retains its existing sibling support contract.
pub use plan as support;
