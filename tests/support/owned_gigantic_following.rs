//! Exact Gigantic Following defaults and finite Sniper status delivery.
//! Reference graphs are test evidence only; native rules contain no Lua values.
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

pub const KIND: &str = "source-bound-gigantic-following";
const DOMAIN: &str = "owned-gigantic-following-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/gigantic-following")
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
fn passive_program(b: &Value, grants: bool) -> Value {
    let key = if grants {
        "grants"
    } else {
        "reservation_efficiency"
    };
    let amount = if grants {
        json!({"kind":"integer","value":1})
    } else {
        quantity(-25.0, &b["percent_unit"])
    };
    json!({"id":b["programs"][key],"context":"actor","reads":[],
      "nodes":[{"id":"amount","expression":{"kind":"literal","value":amount}}],
      "effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"player",
      "stat":b["channels"][key],"contribution":if grants {"add"} else {"increase"},"value":"amount"}}]})
}
fn receiver_program(b: &Value) -> Value {
    json!({"id":b["programs"]["active"],"context":"actor","reads":[
      {"id":"grants","value_type":{"kind":"integer"},"source":{"kind":"contributions","value":{
      "entity":"player","stat":b["channels"]["grants"],"contribution":"add","reduction":"sum","empty":{"kind":"integer","value":0}}}}],
      "nodes":[{"id":"grants","expression":{"kind":"read","input":"grants"}},
      {"id":"zero","expression":{"kind":"literal","value":{"kind":"integer","value":0}}},
      {"id":"active","expression":{"kind":"compare","operation":"greater","left":"grants","right":"zero"}}],
      "effects":[{"id":"active","when":null,"effect":{"kind":"derive","entity":"current","stat":b["channels"]["active"],"value":"active"}}]})
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
        ("new_programs", 3),
        ("closed_passive_owners", 1),
        ("closed_empty_declaration_inventories", 7),
        ("registry_last_issued_before", 0x3306),
        ("registry_last_issued_after", 0x3309),
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
    assert_eq!(m.release.as_str(), "pob-3887ae68-gigantic-following-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (3, 1, 1)
    );
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    for (i, (key, value_type)) in [
        ("grants", json!({"kind":"integer"})),
        ("active", json!({"kind":"boolean"})),
        (
            "reservation_efficiency",
            json!({"kind":"quantity","value":{"unit":b["percent_unit"]}}),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            b["channels"][key]["key"],
            format!("def.{:016x}", 0x3307 + i)
        );
        assert_eq!(
            json!(m.schema[i]),
            json!({"kind":"definition","value":{"kind":"stat","value":{
            "id":b["channels"][key],"schema":{"kind":"known","value":{"value":value_type,"targets":["actor"]}}}}})
        );
    }
    assert_eq!(
        json!(m.owners[0]),
        json!({"owner":{"kind":"definition","value":{"kind":"stat","value":b["channels"]["active"]}},
        "programs":{"members":[receiver_program(&b)],"closure":{"kind":"complete"}}})
    );
    assert_eq!(json!(m.receivers[0]), b["receiver"]);
    assert_eq!(b["receiver"]["id"], "sniper-received-minion-gigantic");
    assert_eq!(b["receiver"]["program"], b["programs"]["active"]);
    assert_eq!(b["receiver"]["stat"], b["channels"]["active"]);
    assert_eq!(b["receiver"]["targets"].as_array().unwrap().len(), 1);
    assert_eq!(d["slots"].as_array().unwrap().len(), 1);
    assert_eq!(
        b["receiver"]["targets"][0],
        json!({"kind":"owned_slot","value":{"slot":d["slots"][0]["value"]["id"]}})
    );
    assert_eq!(
        d["slots"][0]["value"]["id"]["slot"]["key"],
        "def.000000000000001f"
    );
    assert_eq!(
        d["slots"][0]["value"]["id"]["declaration"]["definition"]["key"],
        "def.0000000000000012"
    );
    assert_eq!(d["action_owners"], json!([]));
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    let new: Vec<DefinitionDescriptor> = serde_json::from_value(c["definitions"].clone()).unwrap();
    let old_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    assert_eq!(
        (old.len(), new.len(), old_owners.len(), owners.len()),
        (1, 1, 1, 1)
    );
    assert_eq!(b["node"]["source_id"], "46365");
    assert_eq!(b["node"]["definition"]["key"], "def.0000000000001532");
    assert_eq!(json!(new[0])["value"]["id"], b["node"]["definition"]);
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
        "only the seven exact declaration inventories change"
    );
    assert!(!old_owners[0].programs.is_complete() && old_owners[0].programs.members.is_empty());
    assert_eq!(owners[0].owner, old_owners[0].owner);
    assert!(owners[0].programs.is_complete());
    assert_eq!(
        json!(owners[0].programs.members),
        json!([passive_program(&b, true), passive_program(&b, false)])
    );
    for field in [
        "whole_build_parity",
        "life_damage_factors_published",
        "reservation_action_delivery_published",
        "effective_transforms_proved_absent",
        "incoming_contributor_inventory_closed",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    assert_eq!(
        a["scope"]["receiver_profiles"],
        json!(["RaisedSkeletonSniper"])
    );
    for field in ["query_changes", "routing_changes"] {
        assert_eq!(a["scope"][field], 0);
    }
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"], v["scope"]);
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
    check_vectors(&a, &b, &d, &v, false);
}

// A bounded evidence assertion, not a Lua interpreter or a native data format.
// Typed keys preserve mixed array/string fields; cyclic graphs are rejected here.
fn graph_value(graph: &Value, atom: &Value, depth: usize) -> Value {
    assert!(depth < 24);
    match atom["kind"].as_str().unwrap() {
        "nil" => Value::Null,
        "boolean" => atom["value"].clone(),
        "number" => {
            let n = f64::from_bits(atom["value"].as_u64().unwrap());
            assert!(n.is_finite());
            json!(n)
        }
        "bytes" => json!(
            String::from_utf8(serde_json::from_value(atom["value"].clone()).unwrap()).unwrap()
        ),
        "table" => {
            let mut object = serde_json::Map::new();
            for pair in graph["tables"][atom["value"].as_u64().unwrap() as usize]
                .as_array()
                .unwrap()
            {
                let key = match pair[0]["kind"].as_str().unwrap() {
                    "bytes" => format!(
                        "s:{}",
                        graph_value(graph, &pair[0], depth + 1).as_str().unwrap()
                    ),
                    "number" => format!("n:{}", f64::from_bits(pair[0]["value"].as_u64().unwrap())),
                    other => panic!("unexpected default inventory key {other}"),
                };
                assert!(
                    object
                        .insert(key, graph_value(graph, &pair[1], depth + 1))
                        .is_none()
                );
            }
            Value::Object(object)
        }
        other => panic!("unexpected default inventory atom {other}"),
    }
}
fn default_records(with_source: bool) -> [Value; 2] {
    let mut gigantic = json!({"s:flags":0.0,"s:keywordFlags":0.0,"s:name":"MinionModifier","s:type":"LIST",
      "s:value":{"s:mod":{"s:flags":0.0,"s:keywordFlags":0.0,"s:name":"Gigantic","s:type":"FLAG","s:value":true}}});
    let mut reservation = json!({"n:1":{"s:skillType":6.0,"s:type":"SkillType"},"s:flags":0.0,"s:keywordFlags":0.0,
      "s:name":"ReservationEfficiency","s:type":"INC","s:value":-25.0});
    if with_source {
        gigantic["s:source"] = json!("Tree:46365");
        gigantic["s:value"]["s:mod"]["s:source"] = json!("Tree:46365");
        reservation["s:source"] = json!("Tree:46365");
    }
    [gigantic, reservation]
}
fn check_default_graph(v: &Value) {
    let r = &v["default_report"];
    assert_eq!(
        r["determinism"],
        json!({"fresh_jit_off":2,"fresh_jit_on":1,"warm_parser_and_process_stats":true})
    );
    let default = &r["default"]["value"];
    assert_eq!(default["source_id"], 46365);
    assert_eq!(default["name"], "Gigantic Following");
    assert_eq!(default["accepted_count"], 2);
    assert_eq!(default["unknown"], false);
    assert_eq!(default["extra"], false);
    assert_eq!(
        default["lines"],
        json!([
      {"combined":false,"extra":null,"line":"Your Minions are Gigantic","list_present":true},
      {"combined":false,"extra":null,"line":"25% reduced Reservation Efficiency of Minion Skills","list_present":true}])
    );
    let graph = &default["full_default_graph"];
    assert_eq!(graph["callbacks"], json!([]));
    assert_eq!(graph["roots"].as_array().unwrap().len(), 7);
    let [gigantic, reservation] = default_records(true);
    assert_eq!(
        graph_value(graph, &graph["roots"][2], 0),
        json!({"n:1":gigantic,"n:2":reservation})
    );
    assert_eq!(graph_value(graph, &graph["roots"][3], 0), false);
    assert_eq!(graph_value(graph, &graph["roots"][4], 0), false);
    let parser = r["parser"].as_array().unwrap();
    assert_eq!(parser.len(), 2);
    for (entry, (line, index)) in parser.iter().zip([
        ("25% reduced Reservation Efficiency of Minion Skills", 1),
        ("Your Minions are Gigantic", 0),
    ]) {
        let row = &entry["value"];
        assert_eq!(row["line"], line);
        assert_eq!(row["disk_entry_present"], true);
        assert_eq!(row["disk_return_matches_live"], true);
        let graph = &row["fresh_and_warm"];
        assert_eq!(graph["callbacks"], json!([]));
        assert_eq!(graph["roots"].as_array().unwrap().len(), 1);
        assert_eq!(
            graph_value(graph, &graph["roots"][0], 0),
            json!({"n:1":default_records(false)[index]})
        );
        assert_eq!(row["disk_return"], *graph);
    }
}
fn check_vectors(a: &Value, b: &Value, d: &Value, v: &Value, full: bool) {
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&manifest_bytes));
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    assert_eq!(v["status"], "passed");
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
            assert_eq!(json!(text.len()), pin["bytes"]);
            assert_eq!(hash(text.as_bytes()), pin["sha256"]);
        }
    }
    let catalog_bytes = fs::read(root().join(v["catalog"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(v["catalog"]["bytes"], catalog_bytes.len());
    assert_eq!(v["catalog"]["sha256"], hash(&catalog_bytes));
    let catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    let roots: BTreeSet<_> = catalog["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["root"].as_str().unwrap())
        .collect();
    assert_eq!(json!(roots), v["catalog"]["class_roots"]);
    let n = &v["static_node"];
    let source = b["node"]["source_id"].as_str().unwrap();
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
        json!([
            "Your Minions are Gigantic",
            "25% reduced Reservation Efficiency of Minion Skills"
        ])
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
            "activeEffectImage",
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
    assert_eq!(n["mapping_rows"], d["mapping_rows"]);
    assert_eq!(n["mapping_rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        n["mapping_rows"][0]["source"]["value"]["value"]["node_id"],
        json!({"kind":"text","value":source})
    );
    assert_eq!(
        n["mapping_rows"][0]["source"]["value"]["value"]["view"],
        json!({"kind":"missing"})
    );
    assert_eq!(
        n["mapping_rows"][0]["outcome"]["value"]["target"]["value"]["value"],
        b["node"]["definition"]
    );
    let excerpts = v["source_excerpts"].as_array().unwrap();
    assert_eq!(excerpts.len(), 9);
    for row in std::iter::once(n).chain(excerpts) {
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
    check_default_graph(v);
    if full {
        let report = &v["default_report"];
        let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(bytes.len()), report["bytes"]);
        assert_eq!(hash(&bytes), report["sha256"]);
        let actual: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(actual["source_revision"], a["source_revision"]);
        assert_eq!(actual["determinism"], report["determinism"]);
        assert_eq!(actual["sources"], report["sources"]);
        for projection in
            std::iter::once(&report["default"]).chain(report["parser"].as_array().unwrap())
        {
            assert_eq!(
                actual.pointer(projection["pointer"].as_str().unwrap()),
                Some(&projection["value"])
            );
        }
        let selected = actual["selected_partial_owners"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["source_id"] == 46365)
            .unwrap();
        assert_eq!(selected["definition"], d["definitions"][0]);
        assert_eq!(selected["owner"], d["owners"][0]);
        assert_eq!(selected["mapping"], d["mapping_rows"][0]);
    }
    check_build_reports(a, v, full);
}
fn source_rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|x| x.is_empty()) {
        &[]
    } else {
        v.as_array().unwrap()
    }
}
fn check_build_reports(a: &Value, v: &Value, full: bool) {
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
        assert_eq!(observations.len(), 7);
        let mut pointers = BTreeSet::new();
        for (i, (obs, name)) in observations
            .iter()
            .zip([
                "original-05",
                "repeat-original-05",
                "warm-calcs-to-original",
                "without-gigantic",
                "sniper-calcs-combat",
                "sniper-calcs-buffed",
                "sniper-calcs-unbuffed",
            ])
            .enumerate()
        {
            assert_eq!(obs["case"], name);
            assert_eq!(obs["mode"], if i < 4 { "main" } else { "calcs" });
            let p = obs["projections"].as_array().unwrap();
            assert_eq!(p.len(), 10);
            let case = [4, 6, 7, 12, 14, 15, 16][i];
            let mode = obs["mode"].as_str().unwrap();
            for (projection, suffix) in p.iter().zip([
                "combat",
                "buffs_enabled",
                "effective",
                "actors/2/actor_profile",
                "actors/2/summon_effect_id",
                "actors/2/source_occurrence",
                "actors/2/children/0/passes/0/inputs/actor_gigantic",
                "actors/2/children/0/passes/0/inputs/gigantic_records",
                "actors/2/children/0/passes/0/inputs/raw_player_minion_modifiers",
                "actors/2/children/0/passes/0/inputs/raw_skill_modifiers",
            ]) {
                assert_eq!(
                    projection["pointer"],
                    format!("/cases/{case}/state/{mode}/{suffix}")
                );
                assert!(pointers.insert(projection["pointer"].as_str().unwrap()));
            }
            assert_eq!(p[0]["value"], i < 5);
            assert_eq!(p[1]["value"], i < 6);
            assert_eq!(p[2]["value"], i < 4);
            assert_eq!(p[3]["value"], "RaisedSkeletonSniper");
            assert_eq!(p[4]["value"], "SummonSkeletalSnipersPlayer");
            assert_eq!(p[5]["value"]["source_present"], true);
            assert_eq!(p[5]["value"]["matches"].as_array().unwrap().len(), 1);
            assert_eq!(
                p[5]["value"]["matches"][0]["gem_id"],
                "Metadata/Items/Gems/SkillGemSkeletalSniper"
            );
            assert_eq!(p[6]["value"], i != 3);
            let raw: Vec<_> = source_rows(&p[8]["value"])
                .iter()
                .filter(|x| x["mod"]["source"] == "Tree:46365")
                .collect();
            let flag = source_rows(&p[7]["value"]);
            let damage: Vec<_> = source_rows(&p[9]["value"])
                .iter()
                .filter(|x| x["mod"]["source"] == "Gigantic")
                .collect();
            if i == 3 {
                assert!(raw.is_empty() && flag.is_empty());
            } else {
                assert_eq!(
                    json!(raw),
                    json!([{"ancestor_depth":0,"mod":{"flags":0,"keyword_flags":0,"name":"MinionModifier","source":"Tree:46365","tags":{},"type":"LIST","value":{"mod":{"flags":0,"keywordFlags":0,"name":"Gigantic","source":"Tree:46365","type":"FLAG","value":true}}}}])
                );
                assert_eq!(
                    json!(flag),
                    json!([{"mod":{"flags":0,"keyword_flags":0,"name":"Gigantic","source":"Tree:46365","tags":{},"type":"FLAG","value":true},"value":true}])
                );
            }
            if i == 3 || i >= 5 {
                assert!(damage.is_empty());
            } else {
                assert_eq!(
                    json!(damage),
                    json!([{"ancestor_depth":1,"mod":{"flags":0,"keyword_flags":0,"name":"Damage","source":"Gigantic","tags":{},"type":"MORE","value":20}}])
                );
            }
        }
        for repeat in [1, 2] {
            let values = |index: usize| {
                observations[index]["projections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["value"].clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(values(0), values(repeat));
        }
        if full {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(json!(bytes.len()), report["bytes"]);
            assert_eq!(hash(&bytes), report["sha256"]);
            let actual: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(actual["source_revision"], a["source_revision"]);
            assert_eq!(actual["source_hash"], a["source_manifest_sha256"]);
            assert_eq!(actual["evidence"], v["evidence"]);
            for (observation, case) in observations.iter().zip([4, 6, 7, 12, 14, 15, 16]) {
                assert_eq!(actual["cases"][case]["name"], observation["case"]);
                for p in observation["projections"].as_array().unwrap() {
                    assert_eq!(
                        actual.pointer(p["pointer"].as_str().unwrap()),
                        Some(&p["value"])
                    );
                }
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
    check_vectors(
        &read("authoring.json"),
        &read("bindings.json"),
        &read("dependencies.json"),
        &read("source-vectors.json"),
        true,
    );
    let next = passive_publication::stage(prior, &data(), KIND, DOMAIN);
    assert_endpoint(&next);
    next
}
