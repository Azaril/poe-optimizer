//! Eight exact Reward producer inventories with explicit ordinary global channels.
use super::reward_publication::{self, expected_owner};
use poe_optimizer_core::{owned_rules::DefinitionRules, owned_schema::DefinitionDescriptor};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/permanent-reward-effects")
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
    assert_eq!(a["allocated_definitions"], 2);
    assert_eq!(a["registry_last_issued_before"], 0x32e5);
    assert_eq!(a["registry_last_issued_after"], 0x32e7);
    assert_eq!(a["new_complete_program_owners"], 8);
    assert_eq!(a["appended_numerical_programs"], 8);
    assert_eq!(a["appended_numerical_effects"], 12);
    assert_eq!(a["changed_schema_descriptors"], 0);
    assert_eq!(b["release"], "pob-3887ae68-permanent-reward-effects-v1");
    for field in [
        "before",
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
    ] {
        assert_eq!(b[field], a[field]);
        assert_eq!(d["source"][field], a[field]);
    }
    assert_eq!(b["reward_policy"], a["rewards"]);
    assert_eq!(d["source"]["rewards"], a["rewards"]);
    assert_eq!(d["owners"], json!([]));
    for field in [
        "selection_policy_changed",
        "full_resource_defense_and_resistance_results",
        "selected_build_complete",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    assert_eq!(a["scope"]["published_scalar_reducers"], 0);
    assert_eq!(a["scope"]["published_receivers"], 0);
    let bindings = b["rewards"].as_array().unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    assert_eq!(
        (
            bindings.len(),
            owners.len(),
            definitions.len(),
            dependencies.len()
        ),
        (8, 8, 2, 18)
    );
    assert_eq!(d["absent_owners"].as_array().unwrap().len(), 8);
    for (definition, suffix) in c["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(["32e6", "32e7"])
    {
        assert_eq!(
            definition["value"]["id"]["key"],
            format!("def.000000000000{suffix}")
        );
        assert_eq!(definition["kind"], "stat");
        assert_eq!(
            definition["value"]["schema"],
            json!({"kind":"known","value":{
                "value":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"targets":["actor"]
            }})
        );
    }
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    let expected = [
        ("002a", 10, 1),
        ("0031", 10, 1),
        ("003e", 5, 1),
        ("0041", 30, 3),
        ("0048", 5, 1),
        ("0052", 5, 1),
        ("0053", 5, 1),
        ("005a", 15, 3),
    ];
    let mut total = 0;
    for (index, (binding, (reward, amount, count))) in bindings.iter().zip(expected).enumerate() {
        assert_eq!(
            binding["reward"]["key"],
            format!("def.000000000000{reward}")
        );
        assert_eq!(binding["program"], "permanent-reward-contributions");
        assert_eq!(binding["effects"].as_array().unwrap().len(), count);
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
        let expected_names: Vec<&str> = match reward {
            "002a" | "0052" => vec!["LightningResist"],
            "0031" | "0048" => vec!["FireResist"],
            "003e" => vec!["Mana"],
            "0053" => vec!["Life"],
            _ => vec!["Armour", "Evasion", "EnergyShield"],
        };
        for (effect, source_name) in binding["effects"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected_names)
        {
            total += 1;
            assert_eq!(effect["source_stat"], source_name);
            assert_eq!(effect["amount"], amount);
            assert_eq!(effect["unit"], b["percent_unit"]);
            let (channel, suffix, kind, source_type, global) = match source_name {
                "FireResist" => ("fire_resistance", "32e6", "add", "BASE", false),
                "LightningResist" => ("lightning_resistance", "32e7", "add", "BASE", false),
                "Mana" => ("mana", "29f9", "increase", "INC", false),
                "Life" => ("life", "311a", "increase", "INC", false),
                "Armour" => ("armour", "29f2", "increase", "INC", true),
                "Evasion" => ("evasion", "29f1", "increase", "INC", true),
                "EnergyShield" => ("energy_shield", "29f0", "increase", "INC", true),
                _ => unreachable!(),
            };
            assert_eq!(effect["stat"], b["channels"][channel]);
            assert_eq!(effect["stat"]["key"], format!("def.000000000000{suffix}"));
            assert_eq!(effect["contribution"], kind);
            assert_eq!(effect["source_type"], source_type);
            assert_eq!(
                effect["source_tags"],
                if global {
                    json!([{"type":"Global"}])
                } else {
                    json!({})
                }
            );
        }
    }
    assert_eq!(total, 12);
    reward_publication::check_artifacts(&data(), &a);
    reward_publication::check_source_vectors(&a, &b, &v);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    reward_publication::stage(
        prior,
        &data(),
        "source-bound-permanent-reward-effects",
        "owned-permanent-reward-effects-v1",
    )
}
