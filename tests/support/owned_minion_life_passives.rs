//! Two default Life producers. Incoming contributors, transformations and the
//! final minion Life pool are independent unresolved inventories.
use super::passive_publication;
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

pub const KIND: &str = "source-bound-minion-life-passives";
const DOMAIN: &str = "owned-minion-life-passives-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/minion-life-passives")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, expected) in [
        ("allocated_definitions", 0),
        ("new_programs", 2),
        ("closed_passive_owners", 2),
        ("closed_empty_declaration_inventories", 14),
    ] {
        assert_eq!(a[field], expected);
    }
    assert_eq!(
        a["registry_last_issued_before"],
        a["registry_last_issued_after"]
    );
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
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert_eq!(m.release.as_str(), "pob-3887ae68-minion-life-passives-v1");
    assert_eq!(json!(m.release), b["release"]);
    assert!(
        m.schema.is_empty()
            && m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(b["life_increase"]["key"], "def.00000000000032e5");
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    for field in [
        "whole_build_parity",
        "whole_life_result",
        "effective_transforms_proved_absent",
        "incoming_contributor_inventory_closed",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    for field in [
        "published_scalar_reducers",
        "published_receivers",
        "query_changes",
        "routing_changes",
    ] {
        assert_eq!(a["scope"][field], 0);
    }
    assert_eq!(b["scope"], a["scope"]);
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    let new: Vec<DefinitionDescriptor> = serde_json::from_value(c["definitions"].clone()).unwrap();
    let previous: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    assert_eq!(
        (old.len(), new.len(), previous.len(), owners.len()),
        (2, 2, 2, 2)
    );
    let bindings = b["nodes"].as_array().unwrap();
    assert_eq!(bindings.len(), 2);
    for (i, (source, id)) in [("1218", 0xaef), ("40894", 0x1372)].into_iter().enumerate() {
        assert_eq!(bindings[i]["source_id"], source);
        assert_eq!(bindings[i]["definition"]["key"], format!("def.{id:016x}"));
        assert_eq!(bindings[i]["amount"], 10);
        assert_eq!(json!(new[i])["value"]["id"], bindings[i]["definition"]);
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
            "only the seven proved declaration closures"
        );
        assert!(previous[i].programs.members.is_empty() && !previous[i].programs.is_complete());
        assert_eq!(owners[i].owner, previous[i].owner);
        assert!(owners[i].programs.is_complete());
        assert_eq!(
            json!(owners[i].programs.members),
            json!([{
                "id":"ordinary-minion-life","context":"actor","reads":[],
                "nodes":[{"id":"amount","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":10.0,"unit":b["percent_unit"]}}}}],
                "effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["life_increase"],"contribution":"increase","value":"amount"}}]
            }])
        );
    }
    assert_eq!(d["preserved_life_owners"].as_array().unwrap().len(), 4);
    let old_packet: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/plain-minion-life-passives/closure.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        d["preserved_life_owners"], old_packet["owners"],
        "four established Life/Damage bodies remain exact"
    );
    let assets = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bindings",
            "closure",
            "dependencies",
            "migration",
            "source-vectors"
        ]
    );
    for (name, digest) in assets {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*digest, hash(&bytes));
    }
    check_vectors(&a, &b, &v, false);
}

pub fn check_vectors(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["scope"],
        json!({"default_node_bodies":true,"records_are_runtime_player_store":true,"effective_transforms_proved_absent":false,"whole_life_result":false,"whole_build_parity":false})
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest_bytes), a["source_manifest_sha256"]);
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
        if full {
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
    }
    for pin in v["evidence"]["files"].as_array().unwrap() {
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
    }
    let cat = &v["catalog"];
    let bytes = fs::read(root().join(cat["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len() as u64, cat["bytes"].as_u64().unwrap());
    assert_eq!(hash(&bytes), cat["sha256"]);
    let catalog: Value = serde_json::from_slice(&bytes).unwrap();
    let roots: BTreeSet<_> = catalog["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["root"].as_str().unwrap())
        .collect();
    assert_eq!(json!(roots), cat["class_roots"]);
    let nodes = v["static_nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    for (n, binding) in nodes.iter().zip(b["nodes"].as_array().unwrap()) {
        let source = binding["source_id"].as_str().unwrap();
        assert_eq!(n["source_id"], source);
        let node = catalog["nodes"]
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
            json!(["Minions have 10% increased maximum Life"])
        );
        assert_eq!(node["views"], json!([]));
        assert_eq!(node["unlock"], json!([]));
        assert!(
            catalog["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x["kind"]["value"]["parent"] != source)
        );
        assert!(
            catalog["edges"]
                .as_array()
                .unwrap()
                .iter()
                .all(
                    |x| !((x["left"] == source && roots.contains(x["right"].as_str().unwrap()))
                        || (x["right"] == source && roots.contains(x["left"].as_str().unwrap())))
                )
        );
        let fields: BTreeSet<_> = n["text"]
            .as_str()
            .unwrap()
            .lines()
            .filter_map(|l| {
                let rest = l.strip_prefix("\t\t\t")?;
                if rest.starts_with('\t') {
                    return None;
                }
                rest.split_once('=').map(|(k, _)| k)
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
        assert!(
            n["text"]
                .as_str()
                .unwrap()
                .contains(&format!("\t\t\tskill={source},\n"))
        );
        assert_eq!(n["mapping_rows"].as_array().unwrap().len(), 1);
        let map = &n["mapping_rows"][0];
        assert_eq!(
            map["source"]["value"]["value"]["node_id"],
            json!({"kind":"text","value":source})
        );
        assert_eq!(
            map["source"]["value"]["value"]["view"],
            json!({"kind":"missing"})
        );
        assert_eq!(
            map["outcome"]["value"]["target"]["value"]["value"],
            binding["definition"]
        );
    }
    let excerpts = v["source_excerpts"].as_array().unwrap();
    assert_eq!(excerpts.len(), 5);
    for (i, needle) in [
        "node.__index = node",
        "self.nodes[treeNode.id] = setmetatable({",
        "modList:AddList(node.modList)",
        "ConnectedTo",
        "modLib.parseMod(line)",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(excerpts[i]["text"].as_str().unwrap().contains(needle));
    }
    for row in nodes.iter().chain(excerpts) {
        assert!(paths.contains(row["path"].as_str().unwrap()));
        if full {
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
    }
    assert_eq!(v["evidence"]["case_count"], 37);
    assert_eq!(v["evidence"]["complete_load_attempts_per_jit"], 38);
    assert_eq!(v["evidence"]["business_method_wrappers"], false);
    assert_eq!(v["evidence"]["whole_build_parity"], false);
    assert_eq!(v["evidence"]["originals"].as_array().unwrap().len(), 5);
    for pin in v["evidence"]["originals"].as_array().unwrap() {
        assert_eq!(
            hash(
                &fs::read(
                    root()
                        .join("tests/fixtures/builds/breadth-20260908")
                        .join(pin["name"].as_str().unwrap())
                )
                .unwrap()
            ),
            pin["sha256"]
        );
    }
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    assert_eq!(reports[0]["bytes"], 19_612_949);
    assert_eq!(
        reports[0]["sha256"],
        "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935"
    );
    for report in reports {
        let observations = report["observations"].as_array().unwrap();
        assert_eq!(observations.len(), 4);
        let mut pointers = BTreeSet::new();
        for (obs, name) in observations.iter().zip([
            "original-05",
            "repeat-original-05",
            "warm-calcs-to-original",
            "without-plain-node",
        ]) {
            assert_eq!(obs["case"], name);
            assert!(pointers.insert(obs["pointer"].as_str().unwrap()));
            for source in ["1218", "40894"] {
                let rows: Vec<_> = obs["value"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x["mod"]["source"] == format!("Tree:{source}"))
                    .collect();
                assert_eq!(
                    rows.len(),
                    1,
                    "complete captured store cannot hide another same-source payload"
                );
                assert_eq!(
                    rows[0],
                    &json!({"ancestor_depth":0,"mod":{"flags":0,"keyword_flags":0,"name":"MinionModifier","source":format!("Tree:{source}"),"tags":{},"type":"LIST","value":{"mod":{"flags":0,"keywordFlags":0,"name":"Life","source":format!("Tree:{source}"),"type":"INC","value":10}}}})
                );
            }
        }
        if full {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len() as u64, report["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), report["sha256"]);
            let actual: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(actual["source_revision"], a["source_revision"]);
            assert_eq!(actual["source_hash"], a["source_manifest_sha256"]);
            assert_eq!(actual["evidence"], v["evidence"]);
            for obs in observations {
                assert!(
                    actual.pointer(obs["pointer"].as_str().unwrap()) == Some(&obs["value"]),
                    "exact committed source projection"
                );
            }
        }
    }
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
        digest_owned(
            DOMAIN,
            &(a, b.clone(), d.clone(), c.clone(), v),
            4 * 1024 * 1024
        )
        .unwrap()
    );
    assert_eq!(json!(endpoint.input().recipe.schema.release), b["release"]);
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let preserved: Vec<DefinitionRules> =
        serde_json::from_value(d["preserved_life_owners"].clone()).unwrap();
    for row in definitions {
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
    for row in owners.into_iter().chain(preserved) {
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
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_vectors(
        &read("authoring.json"),
        &read("bindings.json"),
        &read("source-vectors.json"),
        true,
    );
    let next = passive_publication::stage_refinement(prior, &data(), KIND, DOMAIN);
    assert_endpoint(&next);
    next
}
