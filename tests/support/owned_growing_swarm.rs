//! Complete default Growing Swarm owner and finite Sniper receiving channels.
use super::passive_publication;
use poe_optimizer_core::{owned_rules::DefinitionRules, owned_schema::DefinitionDescriptor};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/growing-swarm")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn quantity(value: f64, unit: &Value) -> Value {
    json!({"kind":"quantity","value":{"value":value,"unit":unit}})
}
fn source_records() -> Value {
    json!(["AreaOfEffect","CooldownRecovery"].map(|name| json!({
        "flags":0,"keyword_flags":0,"name":"MinionModifier","source":"Tree:14945","tags":{},"type":"LIST",
        "value":{"mod":{"flags":0,"keywordFlags":0,"name":name,"source":"Tree:14945","type":"INC","value":20}}
    })))
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, want) in [
        ("allocated_definitions", 4),
        ("closed_passive_owners", 1),
        ("closed_empty_declaration_inventories", 7),
        ("registry_last_issued_before", 0x32f3),
        ("registry_last_issued_after", 0x32f7),
    ] {
        assert_eq!(a[field], want);
    }
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
    assert_eq!(m.schema_version, 4);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v19"
    );
    assert_eq!(m.release.as_str(), "pob-3887ae68-growing-swarm-v1");
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (4, 6, 4)
    );
    assert_eq!(b["node"]["key"], "def.0000000000000bc0");
    assert_eq!(b["source_id"], "14945");
    assert_eq!(b["amount"], 20);
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    assert_eq!(
        b["channels"]["cooldown_action"]["key"],
        "def.00000000000032ea"
    );
    for (i, key) in [
        "cooldown_player",
        "cooldown_actor",
        "area_player",
        "area_actor",
    ]
    .into_iter()
    .enumerate()
    {
        let stat = &b["channels"][key];
        assert_eq!(stat["key"], format!("def.{:016x}", 0x32f4 + i));
        assert_eq!(
            json!(m.schema[i]),
            json!({"kind":"definition","value":{"kind":"stat","value":{
                "id":stat,"schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"targets":["actor"]}}
            }}})
        );
        let owner = m
            .owners
            .iter()
            .find(|o| {
                json!(o.owner) == json!({"kind":"definition","value":{"kind":"stat","value":stat}})
            })
            .unwrap();
        let source = if key.ends_with("player") {
            json!({"kind":"contributions","value":{"entity":"current","stat":stat,"contribution":"increase","reduction":"sum","empty":quantity(0.0,&b["percent_unit"])}})
        } else {
            json!({"kind":"stat","value":{"entity":"player","stat":b["channels"][key.replace("_actor","_player")]}})
        };
        assert_eq!(
            json!(owner.programs),
            json!({"closure":{"kind":"complete"},"members":[{
                "id":b["programs"][key],"context":"actor",
                "reads":[{"id":"incoming","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"source":source}],
                "nodes":[{"id":"result","expression":{"kind":"read","input":"incoming"}}],
                "effects":[{"id":"final","when":null,"effect":{"kind":"derive","entity":"current","stat":stat,"value":"result"}}]
            }]})
        );
        let targets = if key.ends_with("player") {
            json!([{"kind":"player"}])
        } else {
            json!([{"kind":"owned_slot","value":{"slot":b["actor_slot"]}}])
        };
        assert_eq!(
            json!(m.receivers[i]),
            json!({"id":b["programs"][key],"stat":stat,"program":b["programs"][key],"targets":targets})
        );
    }
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    let new: Vec<DefinitionDescriptor> = serde_json::from_value(c["definitions"].clone()).unwrap();
    let old_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    assert_eq!(
        (old.len(), new.len(), old_owners.len(), owners.len()),
        (1, 1, 1, 1)
    );
    assert_eq!(c["definitions"][0]["value"]["id"], b["node"]);
    let mut restored = json!(new[0]);
    let declarations = restored["value"]["schema"]["value"]["declarations"]
        .as_object_mut()
        .unwrap();
    assert_eq!(declarations.len(), 7);
    for (name, after) in declarations {
        let before = &d["definitions"][0]["value"]["schema"]["value"]["declarations"][name];
        assert_eq!(before["members"], json!([]));
        assert_eq!(before["closure"]["kind"], "partial");
        assert_eq!(*after, json!({"members":[],"closure":{"kind":"complete"}}));
        *after = before.clone();
    }
    assert_eq!(
        restored,
        json!(old[0]),
        "all adjacency, pool and default definition fields survive"
    );
    assert!(!old_owners[0].programs.is_complete());
    assert!(old_owners[0].programs.members.is_empty());
    assert_eq!(owners[0].owner, old_owners[0].owner);
    assert_eq!(
        json!(owners[0].programs),
        json!({"closure":{"kind":"complete"},"members":[{
            "id":"ordinary-minion-area-and-cooldown","context":"actor","reads":[],
            "nodes":[{"id":"amount","expression":{"kind":"literal","value":quantity(20.0,&b["percent_unit"])}}],
            "effects":[{"id":"area","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["channels"]["area_player"],"contribution":"increase","value":"amount"}},
                {"id":"cooldown","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["channels"]["cooldown_player"],"contribution":"increase","value":"amount"}}]
        }]})
    );
    for (i, action) in ["basic", "gas"].into_iter().enumerate() {
        let old: DefinitionRules = serde_json::from_value(d["action_owners"][i].clone()).unwrap();
        let next = m.owners.iter().find(|o| o.owner == old.owner).unwrap();
        assert_eq!(
            json!(next.owner)["value"]["value"],
            b["actions"][action]["output"]
        );
        assert_eq!(next.programs.closure, old.programs.closure);
        assert!(!next.programs.is_complete());
        assert_eq!(
            &next.programs.members[..old.programs.members.len()],
            old.programs.members
        );
        assert_eq!(next.programs.members.len(), old.programs.members.len() + 1);
        assert_eq!(
            json!(next.programs.members.last().unwrap()),
            json!({
                "id":"unconditional-minion-cooldown","context":"action",
                "reads":[{"id":"incoming","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"source":{"kind":"stat","value":{"entity":"actor","stat":b["channels"]["cooldown_actor"]}}}],
                "nodes":[{"id":"result","expression":{"kind":"read","input":"incoming"}}],
                "effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"current","stat":b["channels"]["cooldown_action"],"contribution":"increase","value":"result"}}]
            })
        );
    }
    for field in [
        "whole_build_parity",
        "cooldown_duration_claimed",
        "area_radius_claimed",
        "effective_transforms_proved_absent",
        "area_receiving_query_observed",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    assert_eq!(a["scope"]["area_receiver_is_actor_input_only"], true);
    for field in ["query_changes", "routing_changes"] {
        assert_eq!(a["scope"][field], 0);
    }
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bindings",
            "closure",
            "dependencies",
            "migration",
            "source-vectors"
        ]
    );
    for (name, digest) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*digest, hash(&bytes));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut pins = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(pins.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| *p == pin)
                .count(),
            1
        );
    }
    check_default_source(&b, &v);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    for (report, receipt) in reports
        .iter()
        .zip(a["source_validation"]["reports"].as_array().unwrap())
    {
        for field in ["path", "bytes", "sha256"] {
            assert_eq!(report[field], receipt[field]);
        }
        let mut found = 0;
        for o in report["observations"].as_array().unwrap() {
            if o["value"].get("modifiers").is_some() {
                let row = &o["value"];
                assert_eq!(row["id"], 14945);
                assert_eq!(row["allocated"], true);
                assert_eq!(
                    row["effective_same_definition"], false,
                    "source records raw tree definition, not spec wrapper identity"
                );
                assert_eq!(row["modifiers"], source_records());
                assert_eq!(row["stats"], v["static_nodes"][0]["catalog_row"]["stats"]);
                found += 1;
            }
        }
        assert_eq!(
            found, 3,
            "original, selected Gas and warm restoration source states"
        );
    }
    assert_eq!(a["source_validation"]["status"], "passed");
}
fn check_default_source(b: &Value, v: &Value) {
    let cat = &v["catalog"];
    let bytes = fs::read(root().join(cat["path"].as_str().unwrap())).unwrap();
    assert_eq!(cat["bytes"], bytes.len());
    assert_eq!(cat["sha256"], hash(&bytes));
    let catalog: Value = serde_json::from_slice(&bytes).unwrap();
    let roots: Vec<_> = catalog["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["root"].as_str().unwrap())
        .collect();
    assert_eq!(json!(roots), cat["class_roots"]);
    assert_eq!(v["static_nodes"].as_array().unwrap().len(), 1);
    let n = &v["static_nodes"][0];
    let node = catalog["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["key"] == "14945")
        .unwrap();
    assert_eq!(*node, n["catalog_row"]);
    assert_eq!(
        node["kind"],
        json!({"kind":"allocation","value":{"pool":"ordinary"}})
    );
    assert_eq!(
        node["stats"],
        json!([
            "Minions have 20% increased Area of Effect",
            "Minions have 20% increased Cooldown Recovery Rate"
        ])
    );
    assert_eq!(node["views"], json!([]));
    assert_eq!(node["unlock"], json!([]));
    assert!(
        catalog["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x["kind"]["value"]["parent"] != "14945")
    );
    assert!(
        catalog["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(
                |x| !((x["left"] == "14945" && roots.contains(&x["right"].as_str().unwrap()))
                    || (x["right"] == "14945" && roots.contains(&x["left"].as_str().unwrap())))
            )
    );
    assert_eq!(cat["candidate_class_edges"], json!([]));
    assert_eq!(cat["attached_choices"], json!([]));
    let fields: BTreeSet<_> = n["text"]
        .as_str()
        .unwrap()
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("\t\t\t")?;
            if rest.starts_with('\t') {
                return None;
            }
            rest.split_once('=').map(|(key, _)| key)
        })
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from([
            "connections",
            "group",
            "icon",
            "isNotable",
            "name",
            "orbit",
            "orbitIndex",
            "recipe",
            "skill",
            "stats",
            "stringId"
        ])
    );
    assert!(
        n["text"]
            .as_str()
            .unwrap()
            .contains("\t\t\tisNotable=true,")
    );
    assert!(n["text"].as_str().unwrap().contains("\t\t\tskill=14945,"));
    assert_eq!(n["mapping_rows"].as_array().unwrap().len(), 1);
    let mapping = &n["mapping_rows"][0];
    assert_eq!(
        mapping["source"]["value"]["value"]["node_id"],
        json!({"kind":"text","value":"14945"})
    );
    assert_eq!(
        mapping["source"]["value"]["value"]["view"],
        json!({"kind":"missing"})
    );
    assert_eq!(
        mapping["outcome"]["value"]["target"]["value"]["value"],
        b["node"]
    );
    let snippets = v["source_excerpts"].as_array().unwrap();
    assert_eq!(snippets.len(), 5);
    assert!(
        snippets[0]["text"]
            .as_str()
            .unwrap()
            .contains("node.type = \"Notable\"")
    );
    assert!(
        snippets[1]["text"]
            .as_str()
            .unwrap()
            .contains("if node.type == \"Normal\" then")
    );
    assert!(
        snippets[1]["text"]
            .as_str()
            .unwrap()
            .contains("Condition:ConnectedTo")
    );
    assert!(
        snippets[2]["text"]
            .as_str()
            .unwrap()
            .contains("}, treeNode)")
    );
    assert!(
        snippets[3]["text"]
            .as_str()
            .unwrap()
            .contains("modList:AddList(node.modList)")
    );
    assert!(
        snippets[4]["text"]
            .as_str()
            .unwrap()
            .contains("minion.modDB:AddMod(value.mod)")
    );
    assert_eq!(
        v["observation_semantics"]["effective_modifiers_recorded"],
        false
    );
    assert_eq!(
        v["observation_semantics"]["area_receiving_query_observed"],
        false
    );
}
fn source_proof() {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    for pin in a["source_files"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(text.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(text.as_bytes()), pin["sha256"]);
    }
    for row in v["static_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .chain(v["source_excerpts"].as_array().unwrap())
    {
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
        let actual = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(actual, row["text"]);
    }
    for evidence in v["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(evidence["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, evidence["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), evidence["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(report["source_revision"], a["source_revision"]);
        assert_eq!(report["business_method_wrappers"], false);
        assert_eq!(report["whole_build_parity"], false);
        assert_eq!(report["case_count"], 3);
        let mut seen = BTreeSet::new();
        for observation in evidence["observations"].as_array().unwrap() {
            let p = observation["pointer"].as_str().unwrap();
            assert!(seen.insert(p));
            assert_eq!(report.pointer(p), Some(&observation["value"]));
        }
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    passive_publication::stage(
        prior,
        &data(),
        "source-bound-growing-swarm",
        "owned-growing-swarm-v1",
    )
}
