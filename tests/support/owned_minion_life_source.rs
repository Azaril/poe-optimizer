//! Injected intrinsic allied Life and one exact historical channel correction.
#[allow(dead_code)]
#[path = "owned_sniper_activation_readiness.rs"]
pub mod activation_family;
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{DefinitionRules, RuleProgram},
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-minion-life";
const DOMAIN: &str = "owned-minion-life-source-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/minion-life-source")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|x| x.is_empty()) {
        &[]
    } else {
        v.as_array().unwrap()
    }
}
fn expected_program(b: &Value) -> Value {
    json!({"id":b["program"],"context":"actor","reads":[{"id":"level","value_type":{"kind":"integer"},"source":{"kind":"stat","value":{"entity":"current","stat":b["channels"]["actor_level"]}}}],
      "nodes":[{"id":"level","expression":{"kind":"read","input":"level"}},
      {"id":"base","expression":{"kind":"lookup_integer_table","table":b["table"],"key":"level"}},
      {"id":"scale","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0.55,"unit":b["units"]["factor"]}}}},
      {"id":"scaled","expression":{"kind":"scale","value":"base","factor":"scale"}},
      {"id":"base-life","expression":{"kind":"round","value":"scaled","quantum":{"value":1.0,"unit":b["units"]["life"]},"mode":"floor"}}],
      "effects":[{"id":"intrinsic-life","when":null,"effect":{"kind":"contribute","entity":"current","stat":b["channels"]["life"],"contribution":"add","value":"base-life"}}]})
}
fn digest() -> poe_optimizer_core::owned_content::OwnedContentDigest {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let c: Value = read("replacement.json");
    digest_owned(DOMAIN, &(a, b, d, v, m, c), 4 * 1024 * 1024).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let c: Value = read("replacement.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (key, n) in [
        ("allocated_definitions", 0),
        ("new_tables", 1),
        ("new_programs", 1),
        ("replaced_programs", 1),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x330b),
        ("registry_last_issued_after", 0x330b),
    ] {
        assert_eq!(a[key], n);
    }
    for key in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[key], b[key]);
        assert_eq!(
            a[key],
            d["source"][if key == "before" { "input" } else { key }]
        );
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-minion-life-source-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.schema.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!((m.owners.len(), m.tables.len()), (1, 1));
    for (key, id) in [
        ("actor_level", 0x1c),
        ("life", 0x311a),
        ("retired_life_more", 0x330a),
        ("damage_more", 0x330b),
    ] {
        assert_eq!(b["channels"][key]["key"], format!("def.{id:016x}"));
    }
    assert_eq!(b["units"]["life"]["key"], "def.0000000000003119");
    assert_eq!(b["units"]["factor"]["key"], "def.0000000000000001");
    assert_eq!(b["program"], "intrinsic-allied-minion-life");
    assert_eq!(b["scale"], 0.55);
    assert_eq!(b["table"], "actor.allied-life-by-level");
    assert_eq!(b["profile"], "RaisedSkeletonSniper");
    assert_eq!(b["actor_definition"]["key"], "def.0000000000003091");
    assert_eq!(b["actor_slot"]["slot"]["key"], "def.000000000000001f");
    assert_eq!(
        b["actor_slot"]["declaration"]["definition"]["key"],
        "def.0000000000000012"
    );
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].programs.members.len(), 4);
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
    let before: RuleProgram = serde_json::from_value(c["before"].clone()).unwrap();
    assert_eq!(
        old[0]
            .programs
            .members
            .iter()
            .filter(|p| **p == before)
            .count(),
        1
    );
    assert_eq!(c["schema_version"], 1);
    assert_eq!(c["owner"], json!(old[0].owner));
    assert_eq!(c["retained_definition"], b["channels"]["retired_life_more"]);
    assert_eq!(c["before"]["id"], "gigantic-life-and-damage");
    let mut expected = c["before"].clone();
    assert_eq!(expected["effects"][0]["id"], "life_more");
    assert_eq!(
        expected["effects"][0]["effect"]["stat"],
        b["channels"]["retired_life_more"]
    );
    expected["effects"][0]["effect"]["stat"] = b["channels"]["life"].clone();
    assert_eq!(
        expected, c["after"],
        "only the redundant Life output address changes"
    );
    assert_eq!(
        c["after"]["effects"][1]["effect"]["stat"],
        b["channels"]["damage_more"]
    );
    assert_eq!(
        c["after"]["nodes"][1]["expression"]["value"]["value"]["value"],
        1.2
    );
    let table = json!(m.tables[0]);
    let definitions = &v["definitions"];
    assert_eq!(definitions["allied_life"].as_array().unwrap().len(), 100);
    assert_eq!(table["id"], b["table"]);
    assert_eq!(table["minimum"], 1);
    assert_eq!(table["maximum"], 100);
    assert_eq!(
        table["value_type"],
        json!({"kind":"quantity","value":{"unit":b["units"]["life"]}})
    );
    let expected:Vec<_>=definitions["allied_life"].as_array().unwrap().iter().map(|n|json!({"kind":"quantity","value":{"value":n.as_f64().unwrap(),"unit":b["units"]["life"]}})).collect();
    assert_eq!(table["rows"], json!(expected));
    assert_eq!(definitions["profile_id"], b["profile"]);
    assert_eq!(definitions["profile"]["life"], b["scale"]);
    assert_eq!(definitions["profile_hostile"], json!({"present":false}));
    assert!(definitions["profile"].get("hostile").is_none());
    assert_eq!(definitions["global_table_identity"], true);
    assert_eq!(definitions["global_profile_identity"], true);
    let population = &d["prerequisite_owners"][0];
    assert_eq!(d["prerequisite_owners"].as_array().unwrap().len(), 1);
    assert_eq!(
        population["owner"]["value"]["value"],
        b["actor_slot"]["declaration"]["definition"]
    );
    let programs = population["programs"]["members"].as_array().unwrap();
    let program = programs
        .iter()
        .find(|p| p["id"] == b["population_program"])
        .unwrap();
    assert_eq!(b["population_program"], "ordinary-population-inputs");
    assert_eq!(b["population_table"], "sniper.actor-level");
    assert_eq!(
        program["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["expression"]["kind"] == "lookup_integer_table"
                && n["expression"]["table"] == b["population_table"])
            .count(),
        1
    );
    assert_eq!(
        program["effects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["effect"]["kind"] == "project_actor_stat"
                && e["effect"]["actor"] == b["actor_slot"]
                && e["effect"]["stat"] == b["channels"]["actor_level"])
            .count(),
        1
    );
    assert_eq!(d["prerequisite_tables"].as_array().unwrap().len(), 1);
    let level_table = &d["prerequisite_tables"][0];
    assert_eq!(level_table["id"], b["population_table"]);
    assert_eq!(level_table["minimum"], 1);
    assert_eq!(level_table["maximum"], 40);
    assert_eq!(
        level_table["rows"],
        json!(
            definitions["minion_levels"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| json!({"kind":"integer","value":n}))
                .collect::<Vec<_>>()
        )
    );
    let slot = d["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["value"]["id"] == b["actor_slot"])
        .unwrap();
    assert_eq!(
        slot["value"]["schema"]["value"]["provider_definition"],
        b["actor_definition"]
    );
    for key in [
        "whole_build_parity",
        "whole_life_result",
        "incoming_contributor_inventory_closed",
        "actor_owner_closed",
        "additional_actor_profiles_admitted",
    ] {
        assert_eq!(a["scope"][key], false);
    }
    for key in [
        "new_definitions",
        "new_receivers",
        "query_changes",
        "routing_changes",
        "retired_channel_executable_references",
    ] {
        assert_eq!(a["scope"][key], 0);
    }
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"], v["scope"]);
    assert_eq!(
        a["scope"]["receiver_profiles"],
        json!(["RaisedSkeletonSniper"])
    );
    assert_eq!(
        a["scope"]["profile_admission"],
        "exact-allied-provider-definition"
    );
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bindings",
            "dependencies",
            "migration",
            "replacement",
            "source-vectors"
        ]
    );
    for (name, expected) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*expected, hash(&bytes));
    }
    check_vectors(&a, &v, false);
}
fn check_vectors(a: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
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
            assert_eq!(json!(text.len()), pin["bytes"]);
            assert_eq!(hash(text.as_bytes()), pin["sha256"]);
        }
    }
    for excerpt in v["source_excerpts"].as_array().unwrap() {
        assert!(paths.contains(excerpt["path"].as_str().unwrap()));
        if full {
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(excerpt["path"].as_str().unwrap()),
            )
            .unwrap();
            let first = excerpt["first_line"].as_u64().unwrap() as usize;
            let last = excerpt["last_line"].as_u64().unwrap() as usize;
            assert_eq!(
                text.lines()
                    .skip(first - 1)
                    .take(last - first + 1)
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n",
                excerpt["text"]
            );
        }
    }
    check_runtime(v);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for key in ["bytes", "sha256"] {
        assert_eq!(reports[0][key], reports[1][key]);
    }
    let meta = &v["report_metadata"];
    assert_eq!(meta["case_count"], 9);
    assert_eq!(meta["complete_load_attempts_per_jit"], 20);
    assert_eq!(meta["business_method_wrappers"], false);
    assert_eq!(meta["source_revision"], a["source_revision"]);
    assert_eq!(meta["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(
        meta["original_xml_sha256"],
        hash(
            &fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap()
        )
    );
    for key in [
        "original_table_selection",
        "original_base_initializer",
        "original_life_consumer",
        "unhooked_controls",
    ] {
        assert_eq!(meta["capture"][key], true);
    }
    assert_eq!(meta["capture"]["copied_formula_as_evidence"], false);
    for key in [
        "whole_build_parity",
        "native_coverage",
        "final_life_formula_parity",
        "hostile_profile_admitted",
        "physical_level_40_obtainable_claimed",
        "actor_level_mutation",
    ] {
        assert_eq!(meta["scope"][key], false);
    }
    for file in meta["files"].as_array().unwrap() {
        assert_eq!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["path"] == file["path"] && x["sha256"] == file["sha256"])
                .count(),
            1
        );
    }
    if full {
        for report in reports {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(json!(bytes.len()), report["bytes"]);
            assert_eq!(hash(&bytes), report["sha256"]);
            let actual: Value = serde_json::from_slice(&bytes).unwrap();
            let mut metadata = actual.clone();
            metadata.as_object_mut().unwrap().remove("cases");
            metadata.as_object_mut().unwrap().remove("definitions");
            assert_eq!(metadata, *meta);
            assert_eq!(actual["definitions"], v["definitions"]);
            for observation in v["observations"].as_array().unwrap() {
                assert_eq!(
                    actual.pointer(observation["pointer"].as_str().unwrap()),
                    Some(&observation["value"])
                );
            }
            for case in actual["cases"].as_array().unwrap() {
                assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
                for key in [
                    "original_functions_preserved",
                    "loaded_state_preserved",
                    "cached_outputs_preserved",
                    "saved_specs_preserved",
                    "fresh_actor_construction",
                    "query_state_preserved",
                    "intrinsic_life_methods_preserved",
                ] {
                    assert_eq!(case["state"][key], true);
                }
                assert_eq!(case["state"]["source_actor_level_mutated"], false);
            }
            for repeat in [1, 2] {
                assert_eq!(
                    actual["cases"][0]["state"],
                    actual["cases"][repeat]["state"]
                );
            }
        }
    }
}
fn check_runtime(v: &Value) {
    let native = v["native_cases"].as_array().unwrap();
    assert_eq!(native.len(), 9);
    let observations = v["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 27);
    let mut pointers = BTreeSet::new();
    for o in observations {
        assert!(pointers.insert(o["pointer"].as_str().unwrap()));
    }
    let definitions = &v["definitions"];
    let curve = definitions["allied_life"].as_array().unwrap();
    for (index, n) in native.iter().enumerate() {
        let name = [
            "original-05",
            "repeat-original-05",
            "warm-level-one-to-original",
            "sniper-calcs-selected",
            "saved-level-1",
            "saved-level-2",
            "saved-level-19",
            "saved-level-20",
            "saved-level-40",
        ][index];
        assert_eq!(n["case_index"], index);
        assert_eq!(n["name"], name);
        let label = &observations[index * 3];
        assert_eq!(label["pointer"], format!("/cases/{index}/name"));
        assert_eq!(label["value"], name);
        for (offset, mode) in [(1, "main"), (2, "calcs")] {
            let o = &observations[index * 3 + offset];
            let actor_index = o["actor_index"].as_u64().unwrap();
            assert_eq!(
                o["pointer"],
                format!("/cases/{index}/state/{mode}/actors/{actor_index}")
            );
            let actor = &o["value"];
            assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
            assert_eq!(actor["summon_effect_id"], "SummonSkeletalSnipersPlayer");
            let selected = mode == "main" || index >= 3;
            assert_eq!(actor["is_environment_minion"], selected);
            assert_eq!(actor["hostile"], false);
            assert_eq!(actor["effective_level"], n["parent_level"]);
            assert_eq!(actor["actor_level"], n["actor_level"]);
            let level = n["actor_level"].as_u64().unwrap() as usize;
            assert!((1..=100).contains(&level));
            let intrinsic = &actor["intrinsic_life"];
            let selections = rows(&intrinsic["table_selections"]);
            assert_eq!(selections.len(), 1);
            assert_eq!(selections[0]["observed_at"], 963);
            assert_eq!(selections[0]["ally_branch_executed"], true);
            for facts in [&selections[0], &intrinsic["facts"]] {
                for key in [
                    "profile_is_loaded",
                    "life_table_is_allied",
                    "exact_source_actor",
                    "exact_parent",
                ] {
                    assert_eq!(facts[key], true);
                }
                assert_eq!(facts["hostile"], false);
                assert_eq!(facts["profile_hostile"], json!({"present":false}));
                assert_eq!(facts["life_table_is_hostile"], false);
                assert_eq!(facts["profile_life"], definitions["profile"]["life"]);
                assert_eq!(facts["actor_level"], n["actor_level"]);
                assert_eq!(facts["effective_level"], n["parent_level"]);
                assert_eq!(facts["table_value"], curve[level - 1]);
                assert_eq!(facts["source"], actor["source_occurrence"]);
            }
            assert_eq!(selections[0]["level_table_value"], n["actor_level"]);
            let parent = n["parent_level"].as_u64().unwrap() as usize;
            assert!((1..=40).contains(&parent));
            assert_eq!(definitions["minion_levels"][parent - 1], n["actor_level"]);
            let inits = rows(&intrinsic["initializers"]);
            let benefits = &actor["gigantic_benefits"];
            let calls = rows(&benefits["original_life_calls"]);
            assert_eq!(inits.len(), usize::from(selected));
            if !selected {
                assert!(calls.is_empty());
                assert!(benefits["actor_output_life"].is_null());
                continue;
            }
            let init = &inits[0];
            assert_eq!(init["input"], intrinsic["facts"]);
            assert_eq!(init["source"], actor["source_occurrence"]);
            assert_eq!(init["unrounded_observed_at"], 1061);
            assert_eq!(init["stored_observed_at"], 1065);
            for key in ["selected", "exact_actor_store", "original_record_preserved"] {
                assert_eq!(init[key], true);
            }
            assert_eq!(init["unrounded_base_life"], init["base_life_at_store"]);
            assert_eq!(
                init["stored_base"],
                json!({"name":"Life","type":"BASE","value":n["base_life"],"source":"Base","flags":0,"keyword_flags":0,"tags":{}})
            );
            assert!(!calls.is_empty());
            for call in calls {
                for key in [
                    "exact_actor_store",
                    "exact_actor_output",
                    "summoner_owns_actor",
                    "summoner_actor_is_parent",
                ] {
                    assert_eq!(call[key], true);
                }
                assert_eq!(call["source"], actor["source_occurrence"]);
                let c = &call["computation"];
                assert_eq!(c["intrinsic_base"]["original_record_is_eligible"], true);
                assert_eq!(c["intrinsic_base"]["record"], init["stored_base"]);
                assert_eq!(c["base"], n["base_life"]);
                assert_eq!(call["return_life"], call["post_return_life"]);
                assert_eq!(c["life_after_assignment"], call["return_life"]);
            }
        }
    }
}
fn retired_references(value: &Value, retired: &Value) -> usize {
    usize::from(value == retired)
        + match value {
            Value::Object(o) => o.values().map(|v| retired_references(v, retired)).sum(),
            Value::Array(a) => a.iter().map(|v| retired_references(v, retired)).sum(),
            _ => 0,
        }
}
/// Authenticate this component in a current successor without loading an old
/// release or requiring unrelated Actor programs to remain frozen.
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    activation_family::assert_component(endpoint);
    activation_family::population_partition::check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("replacement.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proofs: Vec<_> = endpoint
        .input()
        .provenance
        .iter()
        .filter(|p| p.kind.as_str() == KIND)
        .collect();
    assert_eq!(proofs.len(), 1);
    assert_eq!(json!(proofs[0].prior_input), a["before"]);
    assert_eq!(proofs[0].authoring_input, digest());
    let recipe = &endpoint.input().recipe;
    let owners: Vec<_> = recipe
        .rules
        .owners
        .iter()
        .filter(|o| o.owner == m.owners[0].owner)
        .collect();
    assert_eq!(owners.len(), 1);
    assert!(!owners[0].programs.is_complete());
    let benefit: RuleProgram = serde_json::from_value(c["after"].clone()).unwrap();
    for program in m.owners[0].programs.members.iter().chain([&benefit]) {
        assert_eq!(
            owners[0]
                .programs
                .members
                .iter()
                .filter(|p| p.id == program.id)
                .collect::<Vec<_>>(),
            [program]
        );
    }
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["supporting_definitions"].clone())
            .unwrap()
    {
        assert_eq!(
            recipe
                .schema
                .definitions
                .iter()
                .filter(|actual| actual.address() == row.address())
                .collect::<Vec<_>>(),
            [&row]
        );
    }
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            recipe
                .schema
                .slots
                .iter()
                .filter(|actual| actual.address() == row.address())
                .collect::<Vec<_>>(),
            [&row]
        );
    }
    for expected in
        serde_json::from_value::<Vec<DefinitionRules>>(d["prerequisite_owners"].clone()).unwrap()
    {
        let restored = activation_family::restored_owner(endpoint, &expected.owner)
            .expect("the current population partition is required");
        assert_eq!(restored.programs.closure, expected.programs.closure);
        for program in &expected.programs.members {
            assert_eq!(
                restored
                    .programs
                    .members
                    .iter()
                    .filter(|actual| actual.id == program.id)
                    .collect::<Vec<_>>(),
                [program]
            );
        }
    }
    for expected in m
        .tables
        .iter()
        .map(|t| json!(t))
        .chain(d["prerequisite_tables"].as_array().unwrap().iter().cloned())
    {
        assert_eq!(
            recipe
                .rules
                .tables
                .iter()
                .filter(|t| json!(t.id) == expected["id"])
                .map(|t| json!(t))
                .collect::<Vec<_>>(),
            [expected]
        );
    }
    assert_eq!(
        retired_references(&json!(recipe.rules), &b["channels"]["retired_life_more"]),
        0,
        "the retained historical identity is not an executable Life alias"
    );
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("replacement.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    let mut expected: DefinitionRules = serde_json::from_value(d["owners"][0].clone()).unwrap();
    let after: RuleProgram = serde_json::from_value(c["after"].clone()).unwrap();
    let target = expected
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == after.id)
        .unwrap();
    *target = after;
    expected
        .programs
        .members
        .extend(m.owners[0].programs.members.clone());
    assert!(!expected.programs.is_complete());
    assert_eq!(expected.programs.members.len(), 5);
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
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .tables
            .iter()
            .filter(|t| **t == m.tables[0])
            .count(),
        1
    );
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
    for row in
        serde_json::from_value::<Vec<DefinitionRules>>(d["prerequisite_owners"].clone()).unwrap()
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
    for row in d["prerequisite_tables"].as_array().unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .tables
                .iter()
                .filter(|x| json!(x) == *row)
                .count(),
            1
        );
    }
    assert_eq!(
        retired_references(
            &json!(endpoint.input().recipe.rules),
            &b["channels"]["retired_life_more"]
        ),
        0,
        "retained definition has no executable reader, writer or receiver"
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let c: Value = read("replacement.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for k in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[k], a[k]);
    }
    assert!(prior.evaluation().is_none());
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["supporting_definitions"].clone())
            .unwrap()
    {
        assert_eq!(
            prior
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
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let prerequisites: Vec<DefinitionRules> =
        serde_json::from_value(d["prerequisite_owners"].clone()).unwrap();
    for row in old.iter().chain(&prerequisites) {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            prior
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
    for row in d["prerequisite_tables"].as_array().unwrap() {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .tables
                .iter()
                .filter(|x| json!(x) == *row)
                .count(),
            1
        );
    }
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|x| x.owner == old[0].owner)
        .unwrap();
    let before: RuleProgram = serde_json::from_value(c["before"].clone()).unwrap();
    let after: RuleProgram = serde_json::from_value(c["after"].clone()).unwrap();
    let program = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == before.id)
        .unwrap();
    assert_eq!(*program, before);
    *program = after;
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    *inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|x| x.owner == old[0].owner)
        .unwrap() = migrated
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|x| x.owner == old[0].owner)
        .unwrap()
        .clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert_eq!(
        inverse,
        *migrated.input(),
        "only the reviewed owner and provenance change after checked rebinding"
    );
    let mut restored = next.input().recipe.clone();
    assert_eq!(restored.registry, prior.input().recipe.registry);
    assert_eq!(
        restored.schema.definitions,
        prior.input().recipe.schema.definitions
    );
    assert_eq!(
        restored.rules.tables.len(),
        prior.input().recipe.rules.tables.len() + 1
    );
    restored.rules.tables.retain(|t| t.id != m.tables[0].id);
    for row in old {
        let target = restored
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == row.owner)
            .unwrap();
        *target = row;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "one table, one append and one exact channel correction"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
