//! Portable owned-data contracts, with shared fixtures compiled once.
#[allow(dead_code)]
#[path = "support/owned_effect_application_fixture.rs"]
mod effect_application_fixture;
#[allow(dead_code)]
#[path = "support/owned_rule_receiver_fixture.rs"]
mod rule_receiver_fixture;

mod owned_action_selection_reads;
mod owned_allocations;
mod owned_effect_application_stages;
mod owned_effect_applications;
mod owned_evaluation_stages;
mod owned_existing_actor_rules;
mod owned_implicit_passives;
mod owned_metrics;
mod owned_ordered_contributions;
mod owned_routing;
mod owned_rule_operation_versions;
mod owned_rule_receivers;
mod owned_rules;
mod owned_scenario_usage_applicability;
mod owned_schema;
mod owned_skill_input_authority;
mod owned_support_inputs;
mod owned_support_outputs;
mod owned_support_preparation;
mod owned_support_receiving;
