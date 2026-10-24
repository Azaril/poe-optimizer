//! Owned Import integration contracts; each original test root is registered once.
//! Shared fixtures contain no tests and retain their original source authority.

#[allow(dead_code, unused_imports)]
#[path = "../../poe-optimizer-engine/tests/support/owned_fixtures.rs"]
mod engine_fixtures;

#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
mod actions;
#[allow(dead_code)]
#[path = "support/owned_character_reward_policy.rs"]
mod character_reward_policy;
#[allow(dead_code)]
#[path = "support/owned_compact_fixture.rs"]
mod compact_fixture;
#[allow(dead_code)]
#[path = "support/owned_configuration_disposition_policy.rs"]
mod configuration_disposition_policy;
#[allow(dead_code)]
#[path = "support/owned_configuration_reward_policy.rs"]
mod configuration_reward_policy;
#[allow(dead_code)]
#[path = "support/direct_dispositions_fixture.rs"]
mod direct_dispositions_fixture;
#[allow(dead_code)]
#[path = "support/direct_minion_actions_fixture.rs"]
mod direct_minions;
#[allow(dead_code)]
#[path = "support/owned_empty_character_rune_policy.rs"]
mod empty_character_rune_policy;
#[allow(dead_code)]
#[path = "support/owned_enemy_level_policy.rs"]
mod enemy_level_policy;
#[allow(dead_code)]
#[path = "../../../tests/support/owned_evaluation_release_fixture.rs"]
mod evaluation_release_fixture;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
mod inventory;
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod item_normalization_fixture;
#[allow(dead_code)]
#[path = "support/minion_actions_fixture.rs"]
mod minions;
#[allow(dead_code)]
#[path = "support/owned_passive_socket_policy.rs"]
mod passive_socket_policy;
#[allow(dead_code)]
#[path = "support/production_owned_artifacts.rs"]
mod production;
#[allow(dead_code)]
#[path = "support/owned_recipe_fixture.rs"]
mod recipe_fixture;
#[allow(dead_code)]
#[path = "support/owned_reference_fixture.rs"]
mod reference_fixture;

// The usage inventory helper historically resolves the Gem fixture at crate::fixture.
use inventory as fixture;

mod owned_actor_baseline_plan;
mod owned_actor_baseline_recipe;
mod owned_actor_baselines;
mod owned_actor_supply_real;
mod owned_allocation_source;
mod owned_attribute_recipe;
mod owned_augment_reconstruction;
mod owned_augments;
mod owned_catalog_recipe;
mod owned_class_bases;
mod owned_compact_successor;
mod owned_declaration_refinement;
mod owned_defence_profiles;
mod owned_direct_dispositions;
mod owned_direct_source_actions;
mod owned_effective_gem_recipe;
mod owned_gem_catalog;
mod owned_gem_dispositions;
mod owned_gem_schema;
mod owned_intrinsic_attack;
mod owned_item_bases;
mod owned_item_context_options;
mod owned_item_default_breadth;
mod owned_item_defaults;
mod owned_item_layouts;
mod owned_item_lines;
mod owned_item_modifier_publication;
mod owned_item_normalization;
mod owned_item_observation_policy;
mod owned_item_observations;
mod owned_item_properties;
mod owned_item_rounding;
mod owned_item_source;
mod owned_item_source_bounds;
mod owned_item_source_categories;
mod owned_item_source_conditions;
mod owned_item_source_controls;
mod owned_item_source_flags;
mod owned_item_source_metadata;
mod owned_item_source_observations;
mod owned_item_source_properties;
mod owned_item_template_parameters;
mod owned_mapping;
mod owned_membership_refinement;
mod owned_minion_source_actions;
mod owned_modifier_value_recipe;
mod owned_normalization_transition;
mod owned_normalize_breadth;
mod owned_normalize_tree;
mod owned_passive_refinement;
mod owned_passive_views;
mod owned_query_targets;
mod owned_recipe;
mod owned_recipe_extension;
mod owned_recipe_membership_patch;
mod owned_recipe_real;
mod owned_reference_projection;
mod owned_release;
mod owned_release_evaluation;
mod owned_release_migration;
mod owned_release_revision;
mod owned_resistance_recipe;
mod owned_reward_policy;
mod owned_skill_catalog;
mod owned_skill_catalog_extension;
mod owned_skill_inventory;
mod owned_source;
mod owned_source_actions;
mod owned_successor;
mod owned_tree_catalog;
mod owned_tree_policy;
mod owned_usage_inventory;
mod owned_value;
mod owned_value_policy;
mod owned_weapon_profiles;
