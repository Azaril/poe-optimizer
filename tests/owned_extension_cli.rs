//! Remaining historical family laws awaiting direct DSL-release harness migration.
#[path = "support/owned_active_gem_inputs.rs"]
mod active_gem_inputs;
#[path = "support/owned_actor_ability_supply.rs"]
mod actor_ability_supply;
#[path = "support/owned_actor_attributes.rs"]
mod actor_attributes;
#[path = "support/owned_actor_baselines.rs"]
mod actor_baselines;
#[path = "support/owned_canonical_admission.rs"]
mod canonical_admission;
#[path = "support/owned_cleric_ability_supply.rs"]
mod cleric_ability_supply;
#[path = "support/owned_cleric_native.rs"]
mod cleric_native;
#[path = "support/owned_cold_family.rs"]
mod cold_family;
#[path = "support/owned_defence_profiles.rs"]
mod defence_profiles;
#[path = "support/owned_elemental_resistance.rs"]
mod elemental_resistance;
#[path = "support/owned_elemental_weapons.rs"]
mod elemental_weapons;
#[path = "support/owned_gem_inputs.rs"]
mod gem_inputs;
#[path = "support/owned_gem_numeric_aliases.rs"]
mod gem_numeric_aliases;
#[path = "support/owned_gem_schemas.rs"]
mod gem_schemas;
#[path = "support/owned_guarded_gem_inputs.rs"]
mod guarded_gem_inputs;
#[path = "support/owned_item_headers.rs"]
mod headers;
#[path = "support/owned_item_attributes.rs"]
mod item_attributes;
#[path = "support/owned_item_defence_inputs.rs"]
mod item_defence_inputs;
#[path = "support/owned_item_layouts.rs"]
mod item_layouts;
#[path = "support/owned_item_metadata.rs"]
mod item_metadata;
#[path = "support/owned_item_observations.rs"]
mod item_observations;
#[path = "support/owned_item_quality_inputs.rs"]
mod item_quality_inputs;
#[path = "support/owned_local_modifiers.rs"]
mod local_modifiers;
#[path = "support/owned_local_scaling.rs"]
mod local_scaling;
#[path = "support/owned_modifier_transforms.rs"]
mod modifier_transforms;
#[path = "support/owned_modifier_values.rs"]
mod modifier_values;
#[path = "support/owned_multieffect_gem_inputs.rs"]
mod multieffect_gem_inputs;
#[path = "support/owned_passive_attributes.rs"]
mod passive_attributes;
#[path = "support/owned_intrinsic_predecessor.rs"]
mod predecessor;
#[path = "support/owned_rarity_chaos.rs"]
mod rarity_chaos;
#[path = "support/owned_release_inputs.rs"]
mod release_inputs;
#[path = "support/owned_scalar_families.rs"]
mod scalar_families;
#[path = "support/owned_selected_actions.rs"]
mod selected_actions;
#[path = "support/owned_skill_scopes.rs"]
mod skill_scopes;
#[path = "support/owned_source_conditions.rs"]
mod source_conditions;
#[path = "support/owned_source_role_migration.rs"]
mod source_role_migration;
#[allow(dead_code)]
#[path = "support/owned_bundle_cli.rs"]
mod support;
#[path = "support/owned_support_gem_inputs.rs"]
mod support_gem_inputs;
#[path = "support/owned_weapon_catalyst.rs"]
mod weapon_catalysts;
use poe_optimizer_import::owned_recipe::{OwnedRecipeInput, assemble_owned_recipe};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{data, json, success};
fn command(cwd: &Path, name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(cwd).arg(name);
    c
}
fn profiles(cwd: &Path) -> PathBuf {
    let mut prior = predecessor::intrinsic_predecessor(cwd);
    for (name, folder) in [
        ("compile-owned-item-bases", "item-bases"),
        ("compile-owned-weapon-profiles", "weapon-profiles"),
    ] {
        let output = cwd.join(folder);
        let mut c = command(cwd, name);
        c.arg(prior)
            .arg("--catalog")
            .arg(data().join(folder).join("catalog.json"))
            .arg("--policy")
            .arg(data().join(folder).join("policy.json"))
            .arg("--definitions")
            .arg(data().join(folder).join("definitions.json"))
            .arg("--output")
            .arg(&output);
        if folder == "weapon-profiles" {
            c.arg("--base-catalog")
                .arg(data().join("item-bases/catalog.json"));
        }
        success(c.output().unwrap());
        prior = output;
    }
    prior
}
fn run(cwd: &Path, prior: &Path, extension: &Path, output: &Path) -> Output {
    command(cwd, "extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(extension)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
#[test]
fn remaining_historical_family_laws() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path();
    let prior = profiles(cwd);
    let extension = data().join("local-weapon-inputs/extension.json");
    let output = cwd.join("local");
    success(run(cwd, &prior, &extension, &output));
    let recipe = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: 1,
            registry: serde_json::from_value(json(output.join("registry.json"))).unwrap(),
            schema: serde_json::from_value(json(output.join("schema.json"))).unwrap(),
            rules: serde_json::from_value(json(output.join("rules.json"))).unwrap(),
            routing: serde_json::from_value(json(output.join("routing.json"))).unwrap(),
        },
        Default::default(),
    )
    .unwrap();
    let headers = headers::check_headers(cwd, &output, recipe.schema());
    let local = local_modifiers::check_local_modifiers(cwd, &headers);
    let actor = actor_baselines::check_actor_baselines(cwd, &local);
    let transforms = modifier_transforms::check_modifier_transforms(cwd, &actor);
    let values = modifier_values::check_modifier_values(cwd, &transforms);
    let canonical = canonical_admission::check_canonical_admission(cwd, &values);
    let catalysts = weapon_catalysts::check_weapon_catalysts(cwd, &canonical);
    let local_scaled = local_scaling::check_local_scaling(cwd, &catalysts);
    let elemental = elemental_weapons::check_elemental_weapons(cwd, &local_scaled);
    let layouts = item_layouts::check_item_layouts(cwd, &elemental);
    let metadata = item_metadata::check_item_metadata(cwd, &layouts);
    let cold = cold_family::check_cold_family(cwd, &metadata);
    let conditions = source_conditions::check_source_conditions(cwd, &cold);
    let source_roles = source_role_migration::check_source_role_migration(cwd, &conditions);
    let observations = item_observations::check_item_observations(cwd, &source_roles);
    let resistance = elemental_resistance::check_elemental_resistance(cwd, &observations);
    let attributes = item_attributes::check_item_attributes(cwd, &resistance);
    let rarity = rarity_chaos::check_rarity_chaos(cwd, &attributes);
    gem_inputs::check_gem_inputs(cwd, &rarity);
    let scopes = skill_scopes::check_skill_scopes(cwd, &rarity);
    let actor = actor_attributes::check_actor_attributes(cwd, &scopes);
    let passive = passive_attributes::check_passive_attributes(cwd, &actor);
    let defences = passive_attributes::check_passive_defences(cwd, &passive);
    let profiles = defence_profiles::check_defence_profiles(cwd, &defences);
    let inputs = item_defence_inputs::check_item_defence_inputs(cwd, &profiles);
    let quality = item_quality_inputs::check_item_quality_inputs(cwd, &inputs);
    let guarded = guarded_gem_inputs::check_guarded_gem_inputs(cwd, &quality);
    let supports = support_gem_inputs::check_support_gem_inputs(cwd, &guarded);
    let aliases = gem_numeric_aliases::check_gem_numeric_aliases(cwd, &supports);
    let multieffect = multieffect_gem_inputs::check_multieffect_gem_inputs(cwd, &aliases);
    gem_schemas::check_gem_schemas(cwd, &multieffect);
    let release = release_inputs::check_release_inputs(cwd, &multieffect);
    let active = active_gem_inputs::check_active_gem_inputs(cwd, &release);
    selected_actions::check_selected_actions(cwd, &active);
}
