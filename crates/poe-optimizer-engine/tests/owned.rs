//! Native owned integration contracts. Each original test root is registered once.
//! Shared fixtures contain no tests or mutable process state.

#[allow(dead_code, unused_imports)]
#[path = "support/owned_fixtures.rs"]
mod engine_fixtures;

#[allow(dead_code)]
use crate::engine_fixtures::plan as support;

#[allow(dead_code)]
use crate::engine_fixtures::empty_support;

// Original Lua oracle helpers remain Engine-only.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/owned_item_reference.rs"]
mod item_reference_support;

mod ordinary_resistance;
mod owned_action_contribution_queries;
mod owned_action_selection;
mod owned_actor_projection;
mod owned_actor_reward_contributions;
mod owned_actor_supply;
mod owned_attribute_contributions;
mod owned_boolean_support_binding;
mod owned_character_identity;
mod owned_computed_supports;
mod owned_configuration_ratings;
mod owned_configuration_resistance;
mod owned_effect_applications;
mod owned_enemy_level;
mod owned_equipment_receivers;
mod owned_existing_actor_rules;
mod owned_frost_bomb_inputs;
mod owned_gem_activation;
mod owned_generated_skill_inputs;
mod owned_generated_source_properties;
mod owned_generic_actor_rules;
mod owned_ice_nova_actions;
mod owned_implicit_passives;
mod owned_incoming_damage;
mod owned_item_properties;
mod owned_item_range_reference;
mod owned_item_reference;
mod owned_item_scaling;
mod owned_lookup_plan;
mod owned_metrics;
mod owned_minion_accuracy_rules;
mod owned_minion_attack_source;
mod owned_modifier_transform_contract;
mod owned_modifier_transforms;
mod owned_modifier_values;
mod owned_numeric_domain_plan;
mod owned_numeric_selection;
mod owned_numeric_selection_support;
mod owned_ordered_contributions;
mod owned_pain_offering;
mod owned_plain_minion_damage;
mod owned_plain_minion_life;
mod owned_plain_minion_owner_closure;
mod owned_plan;
mod owned_plan_binding;
mod owned_plan_guards;
mod owned_plan_required;
mod owned_player_equipment_slots;
mod owned_preparation_readiness;
mod owned_preparation_scopes;
mod owned_progression_reward_authority;
mod owned_quantize_integer;
mod owned_readiness_reuse;
mod owned_receivers;
mod owned_requirement_assessment;
mod owned_rule_receivers;
mod owned_rules;
mod owned_skill_contribution_queries;
mod owned_skill_occurrence_inputs;
mod owned_skill_participation;
mod owned_skill_projection;
mod owned_skill_usage_inputs;
mod owned_sniper_replay;
mod owned_sniper_reservation;
mod owned_source_properties;
mod owned_source_selection;
mod owned_support_build_index;
mod owned_support_delivery;
mod owned_support_discovery;
mod owned_support_generated;
mod owned_support_inheritance;
mod owned_support_inherited_delivery;
mod owned_support_metrics;
mod owned_support_outputs;
mod owned_support_preparation;
mod owned_support_requirement_assessment;
mod owned_tables;
mod owned_timing;
mod owned_usage_policy;
