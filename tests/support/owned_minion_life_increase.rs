//! One finite Actor delivery of ordinary minion-Life Increase contributions.
//! Producer coverage and final Life reduction remain separate inventories.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub const KIND: &str = "source-bound-minion-life-increase";
const DOMAIN: &str = "owned-minion-life-increase-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/minion-life-increase")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    digest_owned(DOMAIN, &(a, b, d, v, m), 4 * 1024 * 1024).unwrap()
}
fn expected_program(b: &Value) -> Value {
    json!({"id":b["program"],"context":"actor",
      "reads":[{"id":"increase","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},
        "source":{"kind":"contributions","value":{"entity":"player","stat":b["carrier"],"contribution":"increase","reduction":"sum","empty":{"kind":"quantity","value":{"value":0.0,"unit":b["percent_unit"]}}}}}],
      "nodes":[{"id":"increase","expression":{"kind":"read","input":"increase"}}],
      "effects":[{"id":"life-increase","when":null,"effect":{"kind":"contribute","entity":"current","stat":b["life"],"contribution":"increase","value":"increase"}}]})
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, value) in [
        ("allocated_definitions", 0),
        ("new_tables", 0),
        ("new_programs", 1),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x330b),
        ("registry_last_issued_after", 0x330b),
    ] {
        assert_eq!(a[field], value);
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
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-minion-life-increase-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.owners.len(), 1);
    assert_eq!(b["program"], "received-minion-life-increase");
    assert_eq!(b["carrier"]["key"], "def.00000000000032e5");
    assert_eq!(b["life"]["key"], "def.000000000000311a");
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    assert_eq!(b["actor_definition"]["key"], "def.0000000000003091");
    assert_eq!(b["actor_slot"]["slot"]["key"], "def.000000000000001f");
    assert_eq!(
        b["actor_slot"]["declaration"]["definition"]["key"],
        "def.0000000000000012"
    );
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].programs.members.len(), 5);
    assert!(!old[0].programs.is_complete());
    assert_eq!(
        json!(old[0].owner),
        json!({"kind":"slot","value":{"kind":"actor","value":b["actor_slot"]}})
    );
    assert_eq!(m.owners[0].owner, old[0].owner);
    assert_eq!(m.owners[0].programs.closure, old[0].programs.closure);
    assert_eq!(
        json!(m.owners[0].programs.members),
        json!([expected_program(&b)])
    );
    assert!(
        !old[0]
            .programs
            .members
            .iter()
            .any(|p| p.id.as_str() == b["program"].as_str().unwrap())
    );
    assert_eq!(a["scope"], b["scope"]);
    for field in [
        "whole_build_parity",
        "whole_life_result",
        "incoming_contributor_inventory_closed",
        "actor_owner_closed",
        "additional_actor_profiles_admitted",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    for field in [
        "new_definitions",
        "new_receivers",
        "query_changes",
        "routing_changes",
    ] {
        assert_eq!(a["scope"][field], 0);
    }
    assert_eq!(
        a["scope"]["receiver_profiles"],
        json!(["RaisedSkeletonSniper"])
    );
    let owners: Vec<DefinitionRules> = serde_json::from_value(d["passive_owners"].clone()).unwrap();
    assert_eq!(owners.len(), 6);
    assert_eq!(b["nodes"].as_array().unwrap().len(), 6);
    for (i, (source, id, amount)) in [
        ("19006", 0xcf8, 6),
        ("229", 0xdfb, 6),
        ("39461", 0x1311, 6),
        ("54453", 0x1791, 6),
        ("1218", 0xaef, 10),
        ("40894", 0x1372, 10),
    ]
    .into_iter()
    .enumerate()
    {
        let binding = &b["nodes"][i];
        assert_eq!(binding["source_id"], source);
        assert_eq!(binding["definition"]["key"], format!("def.{id:016x}"));
        assert_eq!(binding["amount"], amount);
        assert_eq!(
            json!(owners[i].owner),
            json!({"kind":"definition","value":{"kind":"passive_node","value":binding["definition"]}})
        );
        assert!(owners[i].programs.is_complete());
        let programs: Vec<_> = owners[i]
            .programs
            .members
            .iter()
            .filter(|p| p.id.as_str() == "ordinary-minion-life")
            .collect();
        assert_eq!(programs.len(), 1);
        assert_eq!(
            json!(programs[0]),
            json!({"id":"ordinary-minion-life","context":"actor","reads":[],"nodes":[{"id":"amount","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":amount as f64,"unit":b["percent_unit"]}}}}],"effects":[{"id":"grant","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["carrier"],"contribution":"increase","value":"amount"}}]})
        );
    }
    for (packet, key, indices) in [
        ("plain-minion-life-passives", "owners", 0..4),
        ("minion-life-passives", "owners", 4..6),
    ] {
        let closure: Value = serde_json::from_slice(
            &fs::read(root().join(format!("data/owned/poe2/3887ae68/{packet}/closure.json")))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            json!(&owners[indices]),
            closure[key],
            "published producer bodies remain exact"
        );
    }
    assert_eq!(d["slots"].as_array().unwrap().len(), 1);
    assert_eq!(d["slots"][0]["value"]["id"], b["actor_slot"]);
    assert_eq!(
        d["slots"][0]["value"]["schema"]["value"]["provider_definition"],
        b["actor_definition"]
    );
    let assets = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, digest) in assets {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*digest, hash(&bytes));
    }
    check_vectors(&a, &b, &v, false);
}
fn check_dependencies(endpoint: &StagedOwnedRelease, d: &Value) {
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["supporting_definitions"].clone())
            .unwrap()
    {
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
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<DefinitionRules>>(d["passive_owners"].clone()).unwrap()
    {
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
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    let mut expected: DefinitionRules = serde_json::from_value(d["owners"][0].clone()).unwrap();
    expected
        .programs
        .members
        .extend(m.owners[0].programs.members.clone());
    assert_eq!(expected.programs.members.len(), 6);
    assert!(!expected.programs.is_complete());
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|x| **x == expected)
            .count(),
        1
    );
    check_dependencies(endpoint, &d);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &b, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert!(prior.evaluation().is_none());
    check_dependencies(prior, &d);
    let old: DefinitionRules = serde_json::from_value(d["owners"][0].clone()).unwrap();
    assert_eq!(
        prior
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|x| **x == old)
            .count(),
        1
    );
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert_eq!(
        inverse,
        *migrated.input(),
        "only authored provenance changes after the checked compiler"
    );
    let mut restored = next.input().recipe.clone();
    let target = restored
        .rules
        .owners
        .iter_mut()
        .find(|x| x.owner == old.owner)
        .unwrap();
    *target = old;
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only the exact Actor contribution program is appended"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

fn rows(value: &Value) -> &[Value] {
    if value.as_object().is_some_and(|v| v.is_empty()) {
        &[]
    } else {
        value.as_array().unwrap()
    }
}
// Keep exact source values and absence, excluding only unrelated Damage record
// inventories already present in the immutable full witness report.
fn project_actor(actor: &Value) -> Value {
    let mut projected = serde_json::Map::new();
    for key in [
        "actor_profile",
        "actor_level",
        "effective_level",
        "is_environment_minion",
        "source_occurrence",
        "summon_effect_id",
        "life_delivery",
    ] {
        if let Some(value) = actor.get(key) {
            projected.insert(key.to_owned(), value.clone());
        }
    }
    let mut benefits = actor["gigantic_benefits"].clone();
    if let Some(calls) = benefits["original_life_calls"].as_array_mut() {
        for call in calls {
            call["computation"]
                .as_object_mut()
                .unwrap()
                .remove("raw_modifiers");
        }
    }
    projected.insert("gigantic_benefits".to_owned(), benefits);
    Value::Object(projected)
}
fn check_vectors(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["scope"],
        json!({"original_actor_delivery":true,"original_life_consumer":true,"whole_life_result":false,"whole_build_parity":false})
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
                .filter(|x| *x == pin)
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
    let names = [
        "original-05",
        "repeat-original-05",
        "warm-empty-to-original",
        "without-life-229",
        "without-life-1218",
        "without-six-life-nodes",
        "sniper-calcs-selected",
    ];
    assert_eq!(v["observations"].as_array().unwrap().len(), 14);
    assert_eq!(v["native_cases"].as_array().unwrap().len(), 7);
    for (i, name) in names.into_iter().enumerate() {
        let native = &v["native_cases"][i];
        assert_eq!(native["case_index"], i);
        assert_eq!(native["name"], name);
        for (j, mode) in ["main", "calcs"].into_iter().enumerate() {
            let obs = &v["observations"][i * 2 + j];
            assert_eq!(obs["case_index"], i);
            assert_eq!(obs["name"], name);
            assert_eq!(obs["mode"], mode);
            assert_eq!(
                obs["pointer"],
                format!(
                    "/cases/{i}/state/{mode}/actors/{}",
                    obs["actor_index"].as_u64().unwrap()
                )
            );
            check_delivery(b, native, &obs["value"], j == 0 || i == 6);
        }
    }
    assert_eq!(v["reports"].as_array().unwrap().len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    assert_eq!(v["reports"][0]["bytes"], v["reports"][1]["bytes"]);
    assert_eq!(
        v["report_metadata"]["source_revision"],
        a["source_revision"]
    );
    assert_eq!(
        v["report_metadata"]["manifest_sha256"],
        a["source_manifest_sha256"]
    );
    assert_eq!(v["report_metadata"]["case_count"], 7);
    assert_eq!(v["report_metadata"]["complete_load_attempts_per_jit"], 16);
    assert_eq!(v["report_metadata"]["business_method_wrappers"], false);
    for field in [
        "original_table_selection",
        "original_base_initializer",
        "original_life_consumer",
        "unhooked_controls",
        "original_minion_modifier_list",
        "original_modifier_insertion",
    ] {
        assert_eq!(v["report_metadata"]["capture"][field], true);
    }
    assert_eq!(
        v["report_metadata"]["capture"]["copied_formula_as_evidence"],
        false
    );
    for field in [
        "whole_build_parity",
        "native_coverage",
        "final_life_formula_parity",
        "hostile_profile_admitted",
        "actor_level_mutation",
    ] {
        assert_eq!(v["report_metadata"]["scope"][field], false);
    }
    assert_eq!(
        v["report_metadata"]["scope"]["passive_life_delivery_only"],
        true
    );
    assert_eq!(
        v["report_metadata"]["scope"]["removal_may_prune_other_nodes"],
        true
    );
    for pin in v["report_metadata"]["files"].as_array().unwrap() {
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
    }
    if !full {
        return;
    }
    let observer = fs::read(
        root()
            .join("crates/poe-optimizer-pob/tests/support/owned_minion_physical_damage_source.lua"),
    )
    .unwrap();
    assert_eq!(hash(&observer), v["report_metadata"]["observer_sha256"]);
    let mut first = None;
    for (index, pin) in v["reports"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "runs/owned-minion-life-delivery-source-01/source-jit-{}.json",
                if index == 0 { "off" } else { "on" }
            )
        );
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        if let Some(previous) = &first {
            assert_eq!(&bytes, previous);
        } else {
            first = Some(bytes.clone());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut metadata = report.clone();
        metadata.as_object_mut().unwrap().remove("cases");
        assert_eq!(metadata, v["report_metadata"]);
        assert_eq!(report["cases"].as_array().unwrap().len(), 7);
        for case in report["cases"].as_array().unwrap() {
            for field in [
                "original_functions_preserved",
                "loaded_state_preserved",
                "cached_outputs_preserved",
                "saved_specs_preserved",
                "fresh_actor_construction",
                "query_state_preserved",
                "intrinsic_life_methods_preserved",
                "life_delivery_methods_preserved",
            ] {
                assert_eq!(case["state"][field], true);
            }
            assert_eq!(case["state"]["source_actor_level_mutated"], false);
            assert_eq!(case["state"]["business_method_wrappers"], false);
            assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
        }
        for (i, name) in names.into_iter().enumerate() {
            assert_eq!(report["cases"][i]["name"], name);
        }
        for obs in v["observations"].as_array().unwrap() {
            assert_eq!(
                project_actor(report.pointer(obs["pointer"].as_str().unwrap()).unwrap()),
                obs["value"]
            );
        }
        assert_eq!(report["cases"][0]["state"], report["cases"][1]["state"]);
        assert_eq!(report["cases"][0]["state"], report["cases"][2]["state"]);
    }
}
fn check_delivery(b: &Value, native: &Value, actor: &Value, selected: bool) {
    assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
    assert_eq!(actor["summon_effect_id"], "SummonSkeletalSnipersPlayer");
    assert_eq!(actor["is_environment_minion"], selected);
    assert_eq!(actor["actor_level"], 44);
    assert_eq!(actor["effective_level"], 22);
    let index = native["case_index"].as_u64().unwrap();
    let mut expected: BTreeMap<String, i64> = b["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["source_id"].as_str().unwrap().to_owned(),
                n["amount"].as_i64().unwrap(),
            )
        })
        .collect();
    match index {
        3 => {
            expected.remove("229");
        }
        4 => {
            expected.remove("1218");
        }
        5 => expected.clear(),
        _ => {}
    }
    assert_eq!(
        native["eligible_sources"],
        json!(expected.keys().collect::<Vec<_>>())
    );
    assert_eq!(native["received_increase"], expected.values().sum::<i64>());
    let delivery = &actor["life_delivery"];
    let transfers = rows(&delivery["transfers"]);
    let benefits = &actor["gigantic_benefits"];
    let calls = rows(&benefits["original_life_calls"]);
    assert_eq!(benefits["exact_parent"], true);
    assert_eq!(benefits["exact_summoner"], true);
    if !selected {
        assert!(transfers.is_empty() && calls.is_empty());
        assert!(benefits.get("actor_output_life").is_none());
        return;
    }
    assert!(!transfers.is_empty());
    assert_eq!(calls.len(), 3);
    let mut delivered = BTreeMap::new();
    let mut parent_calls = 0;
    for transfer in transfers {
        assert_eq!(transfer["actor_profile"], "RaisedSkeletonSniper");
        for field in [
            "selected",
            "exact_parent",
            "exact_summoner",
            "exact_parent_cfg",
            "list_return_observed",
        ] {
            assert_eq!(transfer[field], true);
        }
        assert_eq!(transfer["source"], actor["source_occurrence"]);
        assert_eq!(transfer["list_caller_line"], 1162);
        let listed = rows(&transfer["listed_life"]);
        let inserted = rows(&transfer["inserted_life"]);
        if transfer["parent_skill_store"] == true {
            parent_calls += 1;
            assert_eq!(transfer["caller_line"], 1854);
            assert_eq!(listed.len(), expected.len());
            assert_eq!(inserted.len(), expected.len());
        } else {
            assert!(listed.is_empty() && inserted.is_empty());
        }
        for row in listed {
            assert_eq!(row["recipient_type"], json!({"present":false}));
            let record = &row["record"];
            let source = record["source"].as_str().unwrap();
            let id = source.strip_prefix("Tree:").unwrap();
            assert_eq!(record["name"], "Life");
            assert_eq!(record["type"], "INC");
            assert_eq!(record["value"], expected[id]);
            assert_eq!(record["flags"], 0);
            assert_eq!(record["keyword_flags"], 0);
            assert!(rows(&record["tags"]).is_empty());
            let provider = &row["provider"];
            assert_eq!(provider["tree_source"], true);
            assert_eq!(provider["allocated"], true);
            let id_number = id.parse::<u64>().unwrap();
            assert_eq!(provider["node_id"], id_number);
            assert!(rows(&delivery["allocated_node_ids"]).contains(&json!(id_number)));
            let matches = rows(&provider["matches"]);
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0]["record"], *record);
            let targets: Vec<_> = inserted
                .iter()
                .filter(|i| i["payload_index"] == row["payload_index"])
                .collect();
            assert_eq!(targets.len(), 1);
            let target = targets[0];
            assert_eq!(target["record"], *record);
            assert_eq!(target["provider"], *provider);
            assert_eq!(target["addmod_caller_line"], 1164);
            assert_eq!(target["exact_list_payload"], true);
            assert_eq!(target["exact_actor_store"], true);
            assert_eq!(target["stored_identity_count"], 1);
            assert!(delivered.insert(id, record).is_none());
        }
    }
    assert_eq!(parent_calls, 1);
    assert_eq!(delivered.len(), expected.len());
    for call in calls {
        assert_eq!(call["source"], actor["source_occurrence"]);
        assert_eq!(call["actor_profile"], "RaisedSkeletonSniper");
        for field in [
            "exact_actor_store",
            "exact_actor_output",
            "summoner_owns_actor",
            "summoner_actor_is_parent",
        ] {
            assert_eq!(call[field], true);
        }
        let c = &call["computation"];
        assert_eq!(c["increased"], native["received_increase"]);
        assert_eq!(c["base"], 1615);
        assert_eq!(c["observed_at"], 97);
        assert!(c.get("raw_modifiers").is_none());
        assert_eq!(c["life_after_assignment"], call["return_life"]);
        assert_eq!(call["return_life"], call["post_return_life"]);
        let eligible = rows(&c["life_increase_delivery"]["eligible"]);
        assert_eq!(eligible.len(), expected.len());
        let mut seen = BTreeSet::new();
        for row in eligible {
            let id = row["record"]["source"]
                .as_str()
                .unwrap()
                .strip_prefix("Tree:")
                .unwrap();
            assert!(seen.insert(id));
            assert_eq!(row["actual_transfer_count"], 1);
            assert_eq!(row["record"], *delivered[id]);
            assert_eq!(row["value"], expected[id]);
        }
    }
    assert_eq!(
        calls.last().unwrap()["post_return_life"],
        benefits["actor_output_life"]
    );
    for removed in match index {
        3 => vec![229],
        4 => vec![1218],
        5 => vec![19006, 229, 39461, 54453, 1218, 40894],
        _ => vec![],
    } {
        assert!(!rows(&delivery["allocated_node_ids"]).contains(&json!(removed)));
    }
}
