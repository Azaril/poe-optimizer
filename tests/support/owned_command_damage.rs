//! Exact default passive bodies and conditional Sniper contribution transport.
//! Source evidence and occurrence transforms are separate closure authorities.
use super::passive_publication;
#[path = "owned_command_damage_evidence.rs"]
mod evidence;
use poe_optimizer_core::{
    owned_content::digest_owned, owned_rules::DefinitionRules, owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-command-damage";
const DOMAIN: &str = "owned-command-damage-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/command-damage")
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
fn expected_passive(node: &Value, b: &Value) -> Value {
    json!({"id":b["programs"]["passive"],"context":"actor","reads":[],
      "nodes":[{"id":"amount","expression":{"kind":"literal","value":quantity(node["amount"].as_f64().unwrap(),&b["percent_unit"])}}],
      "effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["channels"]["player"],"contribution":"increase","value":"amount"}}]})
}
fn expected_action(b: &Value) -> Value {
    json!({"id":b["programs"]["action"],"context":"action","reads":[
      {"id":"commandable","value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["channels"]["commandable"]}}},
      {"id":"received","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"source":{"kind":"stat","value":{"entity":"actor","stat":b["channels"]["actor"]}}}],
      "nodes":[{"id":"eligible","expression":{"kind":"read","input":"commandable"}},
      {"id":"received","expression":{"kind":"read","input":"received"}},
      {"id":"zero","expression":{"kind":"literal","value":quantity(0.0,&b["percent_unit"])}},
      {"id":"conditional","expression":{"kind":"select","condition":"eligible","when_true":"received","when_false":"zero"}}],
      "effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"current","stat":b["channels"]["action"],"contribution":"increase","value":"conditional"}}]})
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, n) in [
        ("allocated_definitions", 3),
        ("new_programs", 7),
        ("closed_passive_owners", 3),
        ("closed_empty_declaration_inventories", 21),
        ("registry_last_issued_before", 0x3301),
        ("registry_last_issued_after", 0x3304),
    ] {
        assert_eq!(a[field], n);
    }
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-command-damage-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (3, 4, 2)
    );
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    assert_eq!(b["channels"]["commandable"]["key"], "def.00000000000032eb");
    for (i, k) in ["player", "actor", "action"].into_iter().enumerate() {
        assert_eq!(b["channels"][k]["key"], format!("def.{:016x}", 0x3302 + i));
        assert_eq!(
            json!(m.schema[i]),
            json!({"kind":"definition","value":{"kind":"stat","value":{"id":b["channels"][k],
          "schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"targets":[if k=="action" {"action"} else {"actor"}]}}}}})
        );
    }
    for (i, k) in ["player", "actor"].into_iter().enumerate() {
        let stat = &b["channels"][k];
        let subject = json!({"kind":"definition","value":{"kind":"stat","value":stat}});
        let owner = m.owners.iter().find(|o| json!(o.owner) == subject).unwrap();
        let source = if k == "player" {
            json!({"kind":"contributions","value":{"entity":"current","stat":stat,"contribution":"increase","reduction":"sum","empty":quantity(0.0,&b["percent_unit"])}})
        } else {
            json!({"kind":"stat","value":{"entity":"player","stat":b["channels"]["player"]}})
        };
        assert_eq!(
            json!(owner.programs),
            json!({"members":[{"id":b["programs"][k],"context":"actor",
          "reads":[{"id":"incoming","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"source":source}],
          "nodes":[{"id":"result","expression":{"kind":"read","input":"incoming"}}],
          "effects":[{"id":"final","when":null,"effect":{"kind":"derive","entity":"current","stat":stat,"value":"result"}}]}],"closure":{"kind":"complete"}})
        );
        let targets = if k == "player" {
            json!([{"kind":"player"}])
        } else {
            json!([{"kind":"owned_slot","value":{"slot":b["actor_slot"]}}])
        };
        assert_eq!(
            json!(m.receivers[i]),
            json!({"id":b["programs"][k],"stat":stat,"program":b["programs"][k],"targets":targets})
        );
    }
    let nodes = b["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    let new: Vec<DefinitionDescriptor> = serde_json::from_value(c["definitions"].clone()).unwrap();
    let old_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    assert_eq!(
        (old.len(), new.len(), old_owners.len(), owners.len()),
        (3, 3, 3, 3)
    );
    for (i, (id, source, amount)) in [
        (0xedd, "25927", 20),
        (0x10e4, "32847", 20),
        (0x139d, "41511", 15),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(nodes[i]["definition"]["key"], format!("def.{id:016x}"));
        assert_eq!(nodes[i]["source_id"], source);
        assert_eq!(nodes[i]["amount"], amount);
        assert_eq!(json!(new[i])["value"]["id"], nodes[i]["definition"]);
        let mut restored = json!(new[i]);
        let declarations = restored["value"]["schema"]["value"]["declarations"]
            .as_object_mut()
            .unwrap();
        assert_eq!(declarations.len(), 7);
        for (name, after) in declarations {
            let before = &d["definitions"][i]["value"]["schema"]["value"]["declarations"][name];
            assert_eq!(before["members"], json!([]));
            assert_eq!(before["closure"]["kind"], "partial");
            assert_eq!(*after, json!({"members":[],"closure":{"kind":"complete"}}));
            *after = before.clone();
        }
        assert_eq!(
            restored,
            json!(old[i]),
            "only exact declaration inventories refined"
        );
        assert!(!old_owners[i].programs.is_complete() && old_owners[i].programs.members.is_empty());
        assert_eq!(owners[i].owner, old_owners[i].owner);
        assert!(owners[i].programs.is_complete());
        assert_eq!(
            json!(owners[i].programs.members),
            json!([expected_passive(&nodes[i], &b)])
        );
    }
    let old_actions: Vec<DefinitionRules> =
        serde_json::from_value(d["action_owners"].clone()).unwrap();
    assert_eq!(old_actions.len(), 2);
    for (i, old) in old_actions.iter().enumerate() {
        assert_eq!(
            json!(old.owner)["value"]["value"],
            b["actions"][["basic", "gas"][i]]["output"]
        );
        let next = m.owners.iter().find(|o| o.owner == old.owner).unwrap();
        assert!(!next.programs.is_complete());
        assert_eq!(next.programs.closure, old.programs.closure);
        assert_eq!(
            &next.programs.members[..old.programs.members.len()],
            old.programs.members
        );
        assert_eq!(next.programs.members.len(), old.programs.members.len() + 1);
        assert_eq!(
            json!(next.programs.members.last().unwrap()),
            expected_action(&b)
        );
    }
    for field in [
        "whole_build_parity",
        "final_damage_claimed",
        "effective_transforms_proved_absent",
        "incoming_contributor_inventory_closed",
    ] {
        assert_eq!(a["scope"][field], false);
        assert_eq!(b["scope"][field], false);
    }
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
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&manifest_bytes));
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut paths = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
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
    check_default_nodes(&b, &v);
    evidence::check(&v, false);
}
fn check_default_nodes(b: &Value, v: &Value) {
    let bytes = fs::read(root().join(v["catalog"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(v["catalog"]["bytes"], bytes.len());
    assert_eq!(v["catalog"]["sha256"], hash(&bytes));
    let cat: Value = serde_json::from_slice(&bytes).unwrap();
    let roots: Vec<_> = cat["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["root"].as_str().unwrap())
        .collect();
    assert_eq!(json!(roots), v["catalog"]["class_roots"]);
    let rows = v["static_nodes"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for (n, binding) in rows.iter().zip(b["nodes"].as_array().unwrap()) {
        assert_eq!(n["source_id"], binding["source_id"]);
        let source = n["source_id"].as_str().unwrap();
        let node = cat["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["key"] == source)
            .unwrap();
        assert_eq!(*node, n["catalog_row"]);
        assert_eq!(
            node["kind"],
            json!({"kind":"allocation","value":{"pool":"ordinary"}})
        );
        assert_eq!(
            node["stats"],
            json!([format!(
                "Minions deal {}% increased Damage with Command Skills",
                binding["amount"].as_u64().unwrap()
            )])
        );
        assert_eq!(node["views"], json!([]));
        assert_eq!(node["unlock"], json!([]));
        assert!(
            cat["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x["kind"]["value"]["parent"] != source)
        );
        assert!(
            cat["edges"]
                .as_array()
                .unwrap()
                .iter()
                .all(
                    |x| !((x["left"] == source && roots.contains(&x["right"].as_str().unwrap()))
                        || (x["right"] == source && roots.contains(&x["left"].as_str().unwrap())))
                )
        );
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
                "name",
                "orbit",
                "orbitIndex",
                "skill",
                "stats",
                "stringId"
            ])
        );
        let maps = n["mapping_rows"].as_array().unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(
            maps[0]["source"]["value"]["value"]["view"],
            json!({"kind":"missing"})
        );
        assert_eq!(
            maps[0]["outcome"]["value"]["target"]["value"]["value"],
            binding["definition"]
        );
    }
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
    evidence::check(&v, true);
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(
        proof.authoring_input,
        digest_owned(DOMAIN, &(a, b, d, c.clone(), v), 4 * 1024 * 1024).unwrap()
    );
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    let refined: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    for row in refined {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in m.schema {
        let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(row) =
            row
        else {
            panic!("only three new Stat definitions")
        };
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let closed: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    for row in m.owners.into_iter().chain(closed) {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in m.receivers {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let next = passive_publication::stage(prior, &data(), KIND, DOMAIN);
    assert_endpoint(&next);
    next
}
