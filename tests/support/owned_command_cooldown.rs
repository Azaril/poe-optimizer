//! Exact conditional passive inputs and Sniper receiving; no cooldown duration.
use super::passive_publication;

use poe_optimizer_core::{owned_rules::DefinitionRules, owned_schema::DefinitionDescriptor};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/command-cooldown")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(a["allocated_definitions"], 7);
    assert_eq!(a["closed_passive_owners"], 6);
    assert_eq!(a["closed_empty_declaration_inventories"], 42);
    assert_eq!(a["registry_last_issued_before"], 0x32e7);
    assert_eq!(a["registry_last_issued_after"], 0x32ee);
    assert_eq!(a["scope"]["whole_build_parity"], false);
    assert_eq!(a["scope"]["cooldown_duration_claimed"], false);
    assert_eq!(a["scope"]["effective_transforms_proved_absent"], false);
    for field in [
        "before",
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
    ] {
        assert_eq!(b[field], a[field]);
        assert_eq!(
            d["source"][if field == "before" { "input" } else { field }],
            a[field]
        );
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v18"
    );
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (8, 4, 2)
    );
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    for (name, suffix) in [
        ("player", "32e8"),
        ("actor", "32e9"),
        ("action", "32ea"),
        ("commandable", "32eb"),
    ] {
        assert_eq!(
            b["channels"][name]["key"],
            format!("def.000000000000{suffix}")
        );
    }
    for (index, row) in b["gas_stat_sets"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["id"]["key"], format!("def.{:016x}", 0x32ec + index));
        assert_eq!(row["source_index"], index + 1);
        assert_eq!(row["scope"], format!("sniper_gas_shot_statset_{index}"));
        assert_eq!(row["label"], ["Impact", "Poison Cloud", "Explosion"][index]);
    }
    let SchemaExtensionEntry::Slot(next_gas) = &m.schema[0] else {
        panic!("Gas output replacement")
    };
    let mut restored_gas = json!(next_gas);
    assert_eq!(
        restored_gas["value"]["schema"]["value"]["stat_sets"],
        json!({
            "closure":{"kind":"complete"},
            "members": b["gas_stat_sets"].as_array().unwrap().iter().map(|v| v["id"].clone()).collect::<Vec<_>>()
        })
    );
    restored_gas["value"]["schema"]["value"]["stat_sets"] =
        d["gas_output"]["value"]["schema"]["value"]["stat_sets"].clone();
    assert_eq!(restored_gas, d["gas_output"]);
    for (index, entry) in m.schema[1..].iter().enumerate() {
        let SchemaExtensionEntry::Definition(descriptor) = entry else {
            panic!("seven new definitions")
        };
        assert_eq!(
            descriptor.address().key().as_str(),
            format!("def.{:016x}", 0x32e8 + index)
        );
        let expected = if index < 3 {
            json!({"value":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"targets":[if index < 2 {"actor"} else {"action"}]})
        } else if index == 3 {
            json!({"value":{"kind":"boolean"},"targets":["action"]})
        } else {
            json!({})
        };
        assert_eq!(
            json!(descriptor)["value"]["schema"],
            json!({"kind":"known","value":expected})
        );
    }
    let prior_definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    let next_definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let prior_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let next_owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    assert_eq!(
        (
            prior_definitions.len(),
            next_definitions.len(),
            prior_owners.len(),
            next_owners.len()
        ),
        (6, 6, 6, 6)
    );
    let expected = [
        ("0bac", "14598", 8),
        ("11ca", "35645", 20),
        ("1434", "4345", 8),
        ("1462", "43979", 8),
        ("167e", "50837", 8),
        ("1974", "6077", 20),
    ];
    for (i, (suffix, source, amount)) in expected.into_iter().enumerate() {
        let binding = &b["nodes"][i];
        assert_eq!(
            binding["definition"]["key"],
            format!("def.000000000000{suffix}")
        );
        assert_eq!(binding["source_id"], source);
        assert_eq!(binding["amount"], amount);
        let mut restored = json!(&next_definitions[i]);
        let old = json!(&prior_definitions[i]);
        assert_eq!(restored["value"]["id"], binding["definition"]);
        for (name, before) in old["value"]["schema"]["value"]["declarations"]
            .as_object()
            .unwrap()
        {
            let after = &mut restored["value"]["schema"]["value"]["declarations"][name];
            assert_eq!(before["members"], json!([]));
            assert_eq!(before["closure"]["kind"], "partial");
            assert_eq!(*after, json!({"members":[],"closure":{"kind":"complete"}}));
            *after = before.clone();
        }
        assert_eq!(
            restored, old,
            "only empty default declaration closure changes"
        );
        let (old, next) = (&prior_owners[i], &next_owners[i]);
        assert_eq!(old.owner, next.owner);
        assert!(!old.programs.is_complete() && next.programs.is_complete());
        assert_eq!(old.programs.members.len(), usize::from(amount == 8));
        assert_eq!(next.programs.members.len(), old.programs.members.len() + 1);
        assert_eq!(
            &next.programs.members[..old.programs.members.len()],
            old.programs.members
        );
        let appended = json!(next.programs.members.last().unwrap());
        assert_eq!(
            appended,
            json!({"id":"ordinary-command-cooldown","context":"actor","reads":[],
                "nodes":[{"id":"amount","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":amount as f64,"unit":b["percent_unit"]}}}}],
                "effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["channels"]["player"],"contribution":"increase","value":"amount"}}]
            })
        );
    }
    for (i, action) in ["basic", "gas"].into_iter().enumerate() {
        let owner = m
            .owners
            .iter()
            .find(|o| json!(o.owner)["value"]["value"] == b["actions"][action]["output"])
            .unwrap();
        let old: DefinitionRules = serde_json::from_value(d["action_owners"][i].clone()).unwrap();
        assert_eq!(owner.owner, old.owner);
        assert_eq!(owner.programs.closure, old.programs.closure);
        assert!(!owner.programs.is_complete());
        assert_eq!(
            &owner.programs.members[..old.programs.members.len()],
            old.programs.members
        );
        assert_eq!(owner.programs.members.len(), old.programs.members.len() + 2);
        let fact = json!(&owner.programs.members[old.programs.members.len()]);
        assert_eq!(fact["id"], b["programs"]["eligibility"]);
        assert_eq!(
            fact["nodes"][0]["expression"],
            json!({"kind":"literal","value":{"kind":"boolean","value":action == "gas"}})
        );
        assert_eq!(
            fact["effects"][0]["effect"],
            json!({"kind":"derive","entity":"current","stat":b["channels"]["commandable"],"value":"eligible"})
        );
        let contribution = json!(owner.programs.members.last().unwrap());
        assert_eq!(contribution["id"], b["programs"]["action"]);
        assert_eq!(
            contribution["reads"][0]["source"],
            json!({"kind":"stat","value":{"entity":"current","stat":b["channels"]["commandable"]}})
        );
        assert_eq!(
            contribution["reads"][1]["source"],
            json!({"kind":"stat","value":{"entity":"actor","stat":b["channels"]["actor"]}})
        );
        assert_eq!(
            contribution["nodes"][3]["expression"],
            json!({"kind":"select","condition":"eligible","when_true":"received","when_false":"zero"})
        );
        assert_eq!(
            contribution["effects"][0]["effect"],
            json!({"kind":"contribute","entity":"current","stat":b["channels"]["action"],"contribution":"increase","value":"conditional"})
        );
    }
    assert_eq!(json!(m.receivers[0].targets), json!([{"kind":"player"}]));
    assert_eq!(
        json!(m.receivers[1].targets),
        json!([{"kind":"owned_slot","value":{"slot":b["actor_slot"]}}])
    );
    for name in ["player", "actor"] {
        let stat = &b["channels"][name];
        let source = if name == "player" {
            json!({"kind":"contributions","value":{"entity":"current","stat":stat,"contribution":"increase","reduction":"sum","empty":{"kind":"quantity","value":{"value":0.0,"unit":b["percent_unit"]}}}})
        } else {
            json!({"kind":"stat","value":{"entity":"player","stat":b["channels"]["player"]}})
        };
        let owner = m
            .owners
            .iter()
            .find(|o| {
                json!(o.owner) == json!({"kind":"definition","value":{"kind":"stat","value":stat}})
            })
            .unwrap();
        assert_eq!(
            json!(owner.programs),
            json!({"closure":{"kind":"complete"},"members":[{
                "id":b["programs"][name],"context":"actor",
                "reads":[{"id":"incoming","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"source":source}],
                "nodes":[{"id":"result","expression":{"kind":"read","input":"incoming"}}],
                "effects":[{"id":"final","when":null,"effect":{"kind":"derive","entity":"current","stat":stat,"value":"result"}}]
            }]})
        );
    }
    let vectors: Value = read("source-vectors.json");
    assert_eq!(vectors["reports"].as_array().unwrap().len(), 2);
    let receiving = &vectors["reports"][0];
    assert_eq!(
        receiving["sha256"],
        "cfd7a36757dc59171b44ec9f6e8bba8960467d26cf02e5cd0d58e8926b075652"
    );
    assert_eq!(receiving["observations"].as_array().unwrap().len(), 7);
    for observation in receiving["observations"].as_array().unwrap() {
        let row = &observation["value"];
        if let Some(family) = row.as_array() {
            assert_eq!(family.len(), 7);
            for binding in b["nodes"].as_array().unwrap() {
                let id: u64 = binding["source_id"].as_str().unwrap().parse().unwrap();
                let source: Vec<_> = family.iter().filter(|r| r["id"] == id).collect();
                assert_eq!(source.len(), 1);
                assert_eq!(source[0]["allocated"], true);
                let records = source[0]["modifiers"].as_array().unwrap();
                assert_eq!(records.len(), if binding["amount"] == 8 { 2 } else { 1 });
                let cooldown = &records.last().unwrap()["value"]["mod"];
                assert_eq!(cooldown["name"], "CooldownRecovery");
                assert_eq!(cooldown["type"], "INC");
                assert_eq!(cooldown["value"], binding["amount"]);
                assert_eq!(cooldown["source"], format!("Tree:{id}"));
                assert_eq!(
                    cooldown["_positions"],
                    json!([{"index":1,"value":{"type":"Condition","var":"CommandableSkill"}}])
                );
            }
        } else {
            let gas = row["effect_id"] == "GasShotSkeletonSniperMinion";
            assert!(gas || row["effect_id"] == "MinionMeleeBow");
            assert_eq!(row["commandable"], gas);
            assert_eq!(row["received"]["value"], if gas { 92 } else { 20 });
            assert_eq!(row["actor_is_actual_minion"], true);
            assert_eq!(row["actor_parent_is_player"], true);
            assert_eq!(row["source_query_state_preserved"], true);
            let conditional: f64 = rows(&row["received"]["records"])
                .iter()
                .filter(|r| r["mod"]["source"] != "Tree:14945")
                .map(|r| r["value"].as_f64().unwrap())
                .sum();
            assert_eq!(conditional, if gas { 72.0 } else { 0.0 });
            assert_eq!(
                rows(&row["original_cooldown_calls"]).len(),
                usize::from(gas)
            );
            for call in rows(&row["original_cooldown_calls"]) {
                assert_eq!(call["exact_cfg"], true);
                assert_eq!(call["exact_skill_store"], true);
            }
        }
    }
    let topology = &vectors["reports"][1];
    assert_eq!(
        topology["sha256"],
        "c854302d2d3516d20b09a4da02934bdc9eab854b6f71b53d6d3f7e2c9d67e005"
    );
    for row in topology["observations"].as_array().unwrap() {
        if row["pointer"].as_str().unwrap().ends_with("/stat_sets") {
            let sets = row["value"].as_array().unwrap();
            assert_eq!(sets.len(), 3);
            for (actual, authored) in sets.iter().zip(b["gas_stat_sets"].as_array().unwrap()) {
                assert_eq!(actual["index"], authored["source_index"]);
                assert_eq!(actual["label"], authored["label"]);
                assert_eq!(actual["stat_description_scope"], authored["scope"]);
            }
        }
    }
    let artifacts = a["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 5);
    let mut names = BTreeSet::new();
    for artifact in artifacts {
        let name = artifact["file"].as_str().unwrap();
        assert!(names.insert(name));
        assert!(
            [
                "bindings.json",
                "closure.json",
                "dependencies.json",
                "migration.json",
                "source-vectors.json"
            ]
            .contains(&name)
        );
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(bytes.len() as u64, artifact["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), artifact["sha256"]);
        assert!(!bytes.contains(&b'\r'), "authored JSON uses LF");
    }
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let vectors: Value = read("source-vectors.json");
    for pin in a["source_files"].as_array().unwrap() {
        let bytes = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(bytes.as_bytes()), pin["sha256"]);
    }
    for row in vectors["source_excerpts"].as_array().unwrap() {
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pin| pin["path"] == row["path"])
        );
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(row["path"].as_str().unwrap()),
        )
        .unwrap();
        let first = row["first_line"].as_u64().unwrap() as usize;
        let last = row["last_line"].as_u64().unwrap() as usize;
        assert_eq!(
            text.lines()
                .skip(first - 1)
                .take(last - first + 1)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
            row["text"]
        );
    }
    for evidence in vectors["reports"].as_array().unwrap() {
        let mut previous = None;
        for path in evidence["paths"].as_array().unwrap() {
            let bytes = fs::read(root().join(path.as_str().unwrap())).unwrap();
            assert_eq!(bytes.len() as u64, evidence["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), evidence["sha256"]);
            if let Some(old) = &previous {
                assert_eq!(old, &bytes, "JIT reports are exact");
            }
            let report: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(report["source_revision"], a["source_revision"]);
            assert_eq!(
                report[if evidence["family"] == "receiving" {
                    "source_hash"
                } else {
                    "manifest_sha256"
                }],
                a["source_manifest_sha256"]
            );
            assert_eq!(
                report["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| &v["name"])
                    .collect::<Vec<_>>(),
                evidence["case_names"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .collect::<Vec<_>>()
            );
            if evidence["family"] == "receiving" {
                assert_eq!(report["case_count"], 3);
                assert_eq!(report["complete_load_attempts_per_jit"], 4);
                assert_eq!(report["business_method_wrappers"], false);
                assert_eq!(report["native_coverage"], false);
                assert_eq!(report["whole_build_parity"], false);
            } else {
                assert_eq!(report["cases"].as_array().unwrap().len(), 34);
                assert_eq!(report["native_build_parity"], false);
                assert_eq!(report["native_inventory_authority"], false);
            }
            for pin in report["files"].as_array().unwrap() {
                assert!(
                    a["source_files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                );
            }
            let mut pointers = BTreeSet::new();
            for observation in evidence["observations"].as_array().unwrap() {
                let pointer = observation["pointer"].as_str().unwrap();
                assert!(pointers.insert(pointer));
                assert_eq!(report.pointer(pointer), Some(&observation["value"]));
            }
            previous = Some(bytes);
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    passive_publication::stage(
        prior,
        &data(),
        "source-bound-command-cooldown",
        "owned-command-cooldown-v1",
    )
}
