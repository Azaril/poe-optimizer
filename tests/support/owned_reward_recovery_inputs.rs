//! Three exact Reward inventories; no final recovery, threshold or charm-use formulas.
use super::reward_publication::{self, expected_owner};
use poe_optimizer_core::{owned_rules::DefinitionRules, owned_schema::DefinitionDescriptor};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/reward-recovery-inputs-v1")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    assert_eq!(a["registry_last_issued_before"], 0x32ef);
    assert_eq!(a["registry_last_issued_after"], 0x32f3);
    for (field, value) in [
        ("allocated_definitions", 4),
        ("new_complete_program_owners", 3),
        ("appended_numerical_programs", 3),
        ("appended_numerical_effects", 4),
        ("changed_schema_descriptors", 0),
    ] {
        assert_eq!(a[field], value);
    }
    assert_eq!(b["release"], "pob-3887ae68-reward-recovery-inputs-v1");
    for field in [
        "before",
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(a[field], d["source"][field]);
    }
    assert_eq!(a["rewards"], b["reward_policy"]);
    assert_eq!(a["rewards"], d["source"]["rewards"]);
    assert_eq!(a["definitions"]["schema_version"], 6);
    assert_eq!(a["scope"]["three_reward_producers_complete"], true);
    assert_eq!(a["scope"]["charm_effects_preserved_together"], true);
    for field in [
        "selection_policy_changed",
        "final_ailment_flask_and_charm_results",
        "selected_build_complete",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    for field in ["published_scalar_reducers", "published_receivers"] {
        assert_eq!(a["scope"][field], 0);
    }
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    assert_eq!(b["count_unit"]["key"], "def.000000000000295a");
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    assert_eq!(
        (definitions.len(), owners.len(), dependencies.len()),
        (4, 3, 5)
    );
    assert_eq!(d["owners"], json!([]));
    assert_eq!(d["absent_owners"].as_array().unwrap().len(), 3);
    for (definition, (suffix, channel)) in c["definitions"].as_array().unwrap().iter().zip([
        ("32f0", "ailment_threshold_increase"),
        ("32f1", "flask_life_recovery_increase"),
        ("32f2", "charm_charges_gained_increase"),
        ("32f3", "charm_capacity"),
    ]) {
        assert_eq!(definition["kind"], "stat");
        assert_eq!(definition["value"]["id"], b["channels"][channel]);
        assert_eq!(
            definition["value"]["id"]["key"],
            format!("def.000000000000{suffix}")
        );
        let unit = if suffix == "32f3" {
            &b["count_unit"]
        } else {
            &b["percent_unit"]
        };
        assert_eq!(
            definition["value"]["schema"],
            json!({"kind":"known","value":{
            "value":{"kind":"quantity","value":{"unit":unit}},"targets":["actor"]}})
        );
    }
    let bindings = b["rewards"].as_array().unwrap();
    assert_eq!(bindings.len(), 3);
    // Independent finite record expectations, not values derived from rule bodies.
    let expected = [
        (
            "002d",
            vec![
                (
                    "CharmChargesGained",
                    "charm_charges_gained_increase",
                    "INC",
                    "increase",
                    30,
                    "percent_unit",
                ),
                (
                    "CharmLimit",
                    "charm_capacity",
                    "BASE",
                    "add",
                    1,
                    "count_unit",
                ),
            ],
        ),
        (
            "0036",
            vec![(
                "AilmentThreshold",
                "ailment_threshold_increase",
                "INC",
                "increase",
                30,
                "percent_unit",
            )],
        ),
        (
            "003b",
            vec![(
                "FlaskLifeRecovery",
                "flask_life_recovery_increase",
                "INC",
                "increase",
                30,
                "percent_unit",
            )],
        ),
    ];
    for (index, (binding, (reward, effects))) in bindings.iter().zip(expected).enumerate() {
        assert_eq!(
            binding["reward"]["key"],
            format!("def.000000000000{reward}")
        );
        assert_eq!(binding["program"], "reward-recovery-contributions");
        assert_eq!(binding["effects"].as_array().unwrap().len(), effects.len());
        for (effect, (name, channel, source_type, kind, amount, unit)) in
            binding["effects"].as_array().unwrap().iter().zip(effects)
        {
            assert_eq!(
                *effect,
                json!({"source_stat":name,"source_type":source_type,"source_tags":{},
                "stat":b["channels"][channel],"unit":b[unit],"contribution":kind,"amount":amount})
            );
        }
        assert_eq!(json!(owners[index]), expected_owner(binding));
        assert_eq!(d["absent_owners"][index], json!(owners[index].owner));
        let descriptor = &d["definitions"][index];
        assert_eq!(descriptor["kind"], "reward");
        assert_eq!(descriptor["value"]["id"], binding["reward"]);
        assert_eq!(descriptor["value"]["schema"]["kind"], "known");
        let declarations = descriptor["value"]["schema"]["value"]["declarations"]
            .as_object()
            .unwrap();
        assert_eq!(declarations.len(), 7);
        for declaration in declarations.values() {
            assert_eq!(
                *declaration,
                json!({"members":[],"closure":{"kind":"complete"}})
            );
        }
    }
    for (index, unit) in [(3, "percent_unit"), (4, "count_unit")] {
        assert_eq!(d["definitions"][index]["value"]["id"], b[unit]);
    }
    reward_publication::check_artifacts(&data(), &a);
    reward_publication::check_source_vectors(&a, &b, &v);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    reward_publication::stage(
        prior,
        &data(),
        "source-bound-reward-recovery-inputs",
        "owned-reward-recovery-inputs-v1",
    )
}
