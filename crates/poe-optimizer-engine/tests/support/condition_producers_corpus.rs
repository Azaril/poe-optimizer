//! Frozen post-MAIN dependency closures, not a native full-build admission test.
//! Paths under runs/ in the immutable fixture are historical provenance only;
//! this target never opens them. Related numeric/LIST consumers remain evidence.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap()
}
fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().unwrap()
}

pub fn authenticated_capture() -> Value {
    let raw = include_str!("condition_producer_corpus.json").replace("\r\n", "\n");
    // Original captured audit bytes: e33377aabdac38aeb4327a42236bb2540deff54f16587a54ca9fb6f1a8755614.
    // Only fixture checkout line endings may vary; all embedded source XML hashes are exact.
    assert_eq!(
        digest(raw.as_bytes()),
        "e6f77d681d2b78dcec57afa8eb4ee8661692bb59e9e3c8abe032ac0661ca1d67"
    );
    let capture: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        capture["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let manifest = include_str!("../../../poe-optimizer-pob/data/pob-source-manifest.json")
        .replace("\r\n", "\n");
    assert_eq!(
        digest(manifest.as_bytes()),
        text(&capture, "source_manifest_sha256")
    );
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for build in array(&capture, "evidence") {
        let source = &build["source"];
        let xml = std::fs::read(repo.join(text(source, "path").replace('\\', "/"))).unwrap();
        assert_eq!(xml.len() as u64, source["bytes"].as_u64().unwrap());
        assert_eq!(digest(&xml), text(source, "sha256"));
        assert_eq!(
            build["runtime"]["upstream_revision"],
            capture["source_revision"]
        );
        assert_eq!(
            build["runtime"]["source_hash"],
            capture["source_manifest_sha256"]
        );
        for key in ["dependencies", "snapshot_envelope"] {
            assert_eq!(text(&build[key], "sha256").len(), 64);
            assert!(build[key]["bytes"].as_u64().unwrap() > 0);
        }
    }
    for anchor in capture["source_anchors"].as_object().unwrap().values() {
        let path = text(anchor, "path");
        // Historical instrumentation hashes stay recorded, without an ignored-file dependency.
        if let Some(path) = path.strip_prefix("vendor/path-of-building-poe2/") {
            let source = super::oracle::source(path);
            assert_eq!(
                digest(source.as_bytes()),
                text(anchor, "normalized_file_sha256")
            );
            let first = anchor["start_line"].as_u64().unwrap() as usize;
            let last = anchor["end_line"].as_u64().unwrap() as usize;
            let span: String = source
                .split_inclusive('\n')
                .skip(first - 1)
                .take(last - first + 1)
                .collect();
            assert_eq!(digest(span.as_bytes()), text(anchor, "span_sha256"));
        }
    }
    // The actual action layer constructors are separately authenticated. Neither
    // allocation order nor an actor's ModDB is inferred from a skill display name.
    let active = super::oracle::source("src/Modules/CalcActiveSkill.lua");
    assert!(active.contains("local skillModList = new(\"ModList\"):ModList(activeSkill.actor.modDB)\n\tactiveSkill.skillModList = skillModList\n\tactiveSkill.baseSkillModList = skillModList"));
    let perform = super::oracle::source("src/Modules/CalcPerform.lua");
    assert!(perform.contains(
        "activeSkill.skillModList = new(\"ModList\"):ModList(activeSkill.baseSkillModList)"
    ));
    assert_eq!(array(&capture, "cases").len(), 19);
    capture
}

fn tags(record: &Value) -> Vec<Value> {
    let mut indexed = BTreeMap::new();
    for (key, value) in record.as_object().unwrap() {
        if let Ok(index) = key.parse::<usize>() {
            indexed.insert(index, value.clone());
        }
    }
    for (expected, actual) in (1..).zip(indexed.keys()) {
        assert_eq!(expected, *actual);
    }
    indexed.into_values().collect()
}

/// A test-only closure adapter. It rejects incomplete captures before constructing
/// either runtime. In particular it never supplies fabricated weapon/skill data.
pub fn replay_input(case: &Value) -> Result<Value, String> {
    assert_eq!(case["complete_build_native_admission"], false);
    assert_eq!(case["original_query_executed"], false); // provenance, not this test's fresh query
    assert!(case["expected_result"].is_null());
    if case["closure_status"] == "unresolved" {
        assert!(!array(case, "unresolved").is_empty());
        return Err(case["unresolved"].to_string());
    }
    assert_eq!(
        case["closure_status"],
        "represented_query_shape_with_frozen_context"
    );
    assert!(array(case, "unresolved").is_empty());
    let contexts = case["contexts"].as_object().unwrap();
    let nodes = array(case, "dependency_nodes");
    let cfg = case["query"]["cfg"].clone();
    // ModList and ModDB differ on bypass semantics. These captured queries use
    // their common unfiltered branch; do not widen this test to arbitrary lists.
    assert!(cfg.get("source").is_none());
    assert!(cfg.get("ignoreSourceInCheckConditions").is_none());
    let mut actor_ids = BTreeMap::new();
    for context in contexts.values() {
        let name = text(context, "actor");
        assert!(matches!(name, "player" | "enemy"));
        let next = actor_ids.len() + 1;
        actor_ids.entry(name).or_insert(next);
    }
    let mut stores = Vec::new();
    let mut starts = BTreeMap::new();
    let mut actor_stores = BTreeMap::new();
    for (owner, context) in contexts {
        let start = stores.len() + 1;
        starts.insert(owner.as_str(), start);
        let layers = array(context, "layers");
        let is_action = owner.contains("/action/");
        assert_eq!(layers.len(), if is_action { 3 } else { 1 });
        if is_action {
            assert_eq!(case["source_action"]["id"], *owner);
            assert_eq!(case["source_action"]["actor"], context["actor"]);
        }
        let actor = text(context, "actor");
        assert!(
            actor_stores
                .insert(actor, start + layers.len() - 1)
                .is_none()
        );
        for (index, layer) in layers.iter().enumerate() {
            assert_eq!(layer["index"], index);
            stores.push(json!({"actor":actor_ids[actor],
                "parent":(index + 1 < layers.len()).then_some(start + index + 1),
                "store_type":if is_action && index < 2 {"ModList"} else {"ModDB"},
                "conditions":layer["conditions"], "mods":[]}));
        }
    }
    let mut actors = vec![Value::Null; actor_ids.len()];
    for (name, index) in &actor_ids {
        let mut links = serde_json::Map::new();
        // Original MAIN CalcSetup actor links; only captured target actors exist.
        if let Some(target) = actor_ids.get("player") {
            links.insert("player".into(), json!(target));
        }
        let opposing = if *name == "player" { "enemy" } else { "player" };
        if let Some(target) = actor_ids.get(opposing) {
            links.insert("enemy".into(), json!(target));
        }
        actors[index - 1] = json!({"store":actor_stores[name],"links":links});
    }
    let mut ordered = BTreeMap::new();
    for node in nodes {
        let owner = text(node, "owner");
        let context = &contexts[owner];
        let start = starts[owner];
        let layers = array(context, "layers");
        let name = text(node, "name");
        if node["operation"] == "GetCondition" {
            assert_eq!(
                array(node, "explicit_condition_by_layer").len(),
                layers.len()
            );
            for (layer, explicit) in layers
                .iter()
                .zip(array(node, "explicit_condition_by_layer"))
            {
                match layer["conditions"].get(name) {
                    Some(value) => assert_eq!(value, explicit),
                    None => assert_eq!(*explicit, json!({"absent":true})),
                }
            }
        }
        for row in array(node, "all_same_name_rows") {
            let record = &row["record"];
            assert_eq!(
                digest(&serde_json::to_vec(record).unwrap()),
                text(row, "record_sha256")
            );
            assert_eq!(record["type"], "FLAG");
            assert_eq!(row["flag_kind_matches"], true);
            let wanted = if node["operation"] == "GetCondition" {
                format!("Condition:{name}")
            } else {
                name.into()
            };
            assert_eq!(record["name"], wanted);
            let layer = row["layer"].as_u64().unwrap() as usize;
            let order = row["source_order_zero_based"].as_u64().unwrap() as usize;
            assert!(order < layers[layer]["retained_record_count"].as_u64().unwrap() as usize);
            assert!(text(row, "json_pointer").starts_with(text(context, "pointer")));
            let mut normalized = record.clone();
            let record_tags = tags(record);
            for (index, tag) in record_tags.iter().enumerate() {
                let dependency = array(node, "dependencies")
                    .iter()
                    .find(|d| {
                        d["record_sha256"] == row["record_sha256"] && d["tag_index"] == index + 1
                    })
                    .unwrap();
                assert_eq!(dependency["tag"], *tag);
                match text(tag, "type") {
                    "Condition" | "ActorCondition" => {
                        // Negated Condition can inspect uncaptured weapon fields.
                        assert_ne!(tag["neg"], true);
                        let target = text(dependency, "target_owner");
                        assert!(contexts.contains_key(target));
                        for variable in array(dependency, "condition_names") {
                            assert!(nodes.iter().any(|n| n["owner"] == target
                                && n["operation"] == "GetCondition"
                                && n["name"] == *variable));
                        }
                    }
                    "StatThreshold" => {
                        let bindings = array(dependency, "stat_bindings");
                        assert_eq!(bindings.len(), 1);
                        assert_eq!(bindings[0]["name"], tag["stat"]);
                        assert!(tag["threshold"].is_number());
                        let actor = actor_ids[text(context, "actor")] - 1;
                        let output = actors[actor]
                            .as_object_mut()
                            .unwrap()
                            .entry("output")
                            .or_insert_with(|| json!({}));
                        output[text(&bindings[0], "name")] = bindings[0]["value"].clone();
                    }
                    kind => panic!("unrepresented captured dependency {kind}"),
                }
                normalized
                    .as_object_mut()
                    .unwrap()
                    .remove(&(index + 1).to_string());
            }
            normalized["tags"] = json!(record_tags);
            // Keep duplicate occurrences. Collapse only repeated references to
            // the same original layer/order, never rows with equal payloads.
            if let Some(previous) = ordered.insert((start + layer - 1, order), normalized.clone()) {
                assert_eq!(previous, normalized);
            }
        }
    }
    for ((store, _), record) in ordered {
        stores[store]["mods"].as_array_mut().unwrap().push(record);
    }
    let query = match text(&case["query"], "operation") {
        "GetCondition" => {
            json!({"kind":"condition","variable":case["query"]["name"],"no_mod":false})
        }
        "Flag" => json!({"kind":"flag","names":[case["query"]["name"]]}),
        value => panic!("unrepresented operation {value}"),
    };
    Ok(
        json!({"root":starts[text(case,"owner")],"stores":stores,"actors":actors,"cfg":cfg,"query":query}),
    )
}

pub fn unresolved_names(capture: &Value) -> BTreeSet<&str> {
    array(capture, "cases")
        .iter()
        .filter(|c| c["closure_status"] == "unresolved")
        .map(|c| text(&c["query"], "name"))
        .collect()
}
