//! Finite Gigantic Actor contributions; neither final pools nor contributor closure.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-gigantic-benefits";
const DOMAIN: &str = "owned-gigantic-benefits-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/gigantic-benefits")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn expected_program(b: &Value) -> Value {
    json!({"id":b["program"],"context":"actor","reads":[
      {"id":"active","value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["channels"]["active"]}}}],
      "nodes":[{"id":"active","expression":{"kind":"read","input":"active"}},
      {"id":"factor","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":1.2,"unit":b["factor_unit"]}}}}],
      "effects":[{"id":"life_more","when":"active","effect":{"kind":"contribute","entity":"current","stat":b["channels"]["life_more"],"contribution":"multiply","value":"factor"}},
      {"id":"damage_more","when":"active","effect":{"kind":"contribute","entity":"current","stat":b["channels"]["damage_more"],"contribution":"multiply","value":"factor"}}]})
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, n) in [
        ("allocated_definitions", 2),
        ("new_programs", 1),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x3309),
        ("registry_last_issued_after", 0x330b),
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
    assert_eq!(m.release.as_str(), "pob-3887ae68-gigantic-benefits-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!((m.schema.len(), m.owners.len()), (2, 1));
    assert_eq!(b["factor_unit"]["key"], "def.0000000000000001");
    assert_eq!(b["channels"]["active"]["key"], "def.0000000000003308");
    assert_eq!(b["factor"], 1.2);
    assert_eq!(b["program"], "gigantic-life-and-damage");
    for (i, k) in ["life_more", "damage_more"].into_iter().enumerate() {
        assert_eq!(b["channels"][k]["key"], format!("def.{:016x}", 0x330a + i));
        assert_eq!(
            json!(m.schema[i]),
            json!({"kind":"definition","value":{"kind":"stat","value":{
            "id":b["channels"][k],"schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":b["factor_unit"]}},"targets":["actor"]}}}}})
        );
    }
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].programs.members.len(), 3);
    assert!(!old[0].programs.is_complete());
    assert_eq!(
        json!(old[0].owner),
        json!({"kind":"slot","value":{"kind":"actor","value":b["actor_slot"]}})
    );
    assert_eq!(b["actor_slot"]["slot"]["key"], "def.000000000000001f");
    assert_eq!(
        b["actor_slot"]["declaration"]["definition"]["key"],
        "def.0000000000000012"
    );
    assert_eq!(m.owners[0].owner, old[0].owner);
    assert_eq!(m.owners[0].programs.closure, old[0].programs.closure);
    assert_eq!(
        json!(m.owners[0].programs.members),
        json!([expected_program(&b)])
    );
    assert!(
        old[0]
            .programs
            .members
            .iter()
            .all(|p| json!(p.id) != b["program"])
    );
    assert_eq!(d["slots"].as_array().unwrap().len(), 1);
    assert_eq!(d["slots"][0]["value"]["id"], b["actor_slot"]);
    assert_eq!(d["status_owners"].as_array().unwrap().len(), 1);
    assert_eq!(d["status_receivers"].as_array().unwrap().len(), 1);
    assert_eq!(
        d["status_owners"][0]["owner"]["value"]["value"],
        b["channels"]["active"]
    );
    assert_eq!(d["status_receivers"][0]["stat"], b["channels"]["active"]);
    assert_eq!(
        d["status_receivers"][0]["targets"],
        json!([{"kind":"owned_slot","value":{"slot":b["actor_slot"]}}])
    );
    for field in [
        "whole_build_parity",
        "whole_life_result",
        "whole_damage_result",
        "reservation_action_delivery_published",
        "incoming_contributor_inventory_closed",
        "actor_owner_closed",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    for field in [
        "new_receivers",
        "published_scalar_reducers",
        "query_changes",
        "routing_changes",
    ] {
        assert_eq!(a["scope"][field], 0);
    }
    assert_eq!(
        a["scope"]["receiver_profiles"],
        json!(["RaisedSkeletonSniper"])
    );
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"], v["scope"]);
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, digest) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*digest, hash(&bytes));
    }
    check_vectors(&a, &v, false);
}

fn check_vectors(a: &Value, v: &Value, full: bool) {
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
    assert!(v["source_excerpts"].as_array().unwrap().iter().any(|e| {
        let text = e["text"].as_str().unwrap();
        text.contains("modDB:Flag(nil, \"Gigantic\")")
            && text.contains("modDB:NewMod(\"Life\", \"MORE\", 20, \"Gigantic\")")
            && text.contains("modDB:NewMod(\"Damage\", \"MORE\", 20, \"Gigantic\")")
    }));
    check_runtime_evidence(a, v, full);
}

// Completed by exact projections of the existing opt-in source observer. These
// source inventories are evidence; native channels remain owned typed values.
fn check_runtime_evidence(a: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    assert_eq!(v["individual_factor"], 1.2);
    assert_eq!(v["composed_factor_parity"], false);
    let metadata = &v["report_metadata"];
    assert_eq!(metadata["case_count"], 9);
    assert_eq!(metadata["complete_load_attempts_per_jit"], 20);
    assert_eq!(metadata["business_method_wrappers"], false);
    assert_eq!(metadata["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(metadata["source_revision"], a["source_revision"]);
    assert_eq!(
        metadata["original_xml_sha256"],
        hash(
            &fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap()
        )
    );
    assert_eq!(
        metadata["capture"]["consumer"],
        "original calcs.doActorLifeManaSpirit"
    );
    assert_eq!(metadata["capture"]["after_assignment_line"], 97);
    for key in [
        "original_more_local",
        "original_actor_output_after_return",
        "unhooked_controls",
    ] {
        assert_eq!(metadata["capture"][key], true);
    }
    assert_eq!(metadata["capture"]["copied_formula_as_evidence"], false);
    for key in [
        "whole_build_parity",
        "native_coverage",
        "full_life_formula_parity",
        "composed_more_factor_parity",
        "obtainable_second_grant_claimed",
    ] {
        assert_eq!(metadata["scope"][key], false);
    }
    for file in metadata["files"].as_array().unwrap() {
        assert_eq!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == file["path"] && p["sha256"] == file["sha256"])
                .count(),
            1
        );
    }
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    for report in reports {
        let observations = report["observations"].as_array().unwrap();
        assert_eq!(observations.len(), 18);
        let mut pointers = BTreeSet::new();
        for observation in observations {
            let projections = observation["projections"].as_array().unwrap();
            let index = observation["case_index"].as_u64().unwrap() as usize;
            let case = [
                "original-05",
                "repeat-original-05",
                "without-gigantic",
                "warm-removal-to-original",
                "duplicate-gigantic-grant",
                "sniper-calcs-effective",
                "sniper-calcs-combat",
                "sniper-calcs-buffed",
                "sniper-calcs-unbuffed",
            ][index];
            let mode = observation["mode"].as_str().unwrap();
            assert!(mode == "main" || mode == "calcs");
            let actor_index = observation["actor_index"].as_u64().unwrap();
            let frame = format!("/cases/{index}/state/{mode}");
            let actor = format!("{frame}/actors/{actor_index}");
            let selected = projections[6]["value"].as_bool().unwrap();
            assert_eq!(selected, mode == "main" || index >= 5);
            assert_eq!(projections[1]["value"], mode == "main" || index <= 6);
            assert_eq!(projections[2]["value"], mode == "main" || index <= 7);
            assert_eq!(projections[3]["value"], mode == "main" || index <= 5);
            let child = format!(
                "{actor}/children/{}",
                observation["child_index"].as_u64().unwrap()
            );
            let pointers_expected = [
                format!("/cases/{index}/name"),
                format!("{frame}/combat"),
                format!("{frame}/buffs_enabled"),
                format!("{frame}/effective"),
                format!("{actor}/actor_profile"),
                format!("{actor}/source_occurrence"),
                format!("{actor}/is_environment_minion"),
                format!("{actor}/gigantic_benefits"),
                if selected {
                    format!("{child}/damage_calls")
                } else {
                    child.clone()
                },
                format!(
                    "{actor}/children/{}/effect_id",
                    observation["child_index"].as_u64().unwrap()
                ),
            ];
            assert_eq!(projections.len(), pointers_expected.len());
            for (p, expected) in projections.iter().zip(pointers_expected) {
                assert_eq!(p["pointer"], expected);
                // The case label is shared by its two frame observations.
                if !expected.ends_with("/name") {
                    assert!(pointers.insert(p["pointer"].as_str().unwrap()));
                }
            }
            let values: Vec<_> = projections.iter().map(|p| &p["value"]).collect();
            assert_eq!(values[0].as_str(), Some(case));
            assert_eq!(values[4].as_str(), Some("RaisedSkeletonSniper"));
            assert_eq!(values[9].as_str(), Some("MinionMeleeBow"));
            assert_eq!(values[5]["source_present"], true);
            assert_eq!(values[5]["matches"].as_array().unwrap().len(), 1);
            assert_eq!(
                values[5]["matches"][0]["gem_id"],
                "Metadata/Items/Gems/SkillGemSkeletalSniper"
            );
            let benefits = values[7];
            assert_eq!(benefits["exact_parent"], true);
            assert_eq!(benefits["exact_summoner"], true);
            let calls = rows(&benefits["original_life_calls"]);
            if values[6].as_bool() == Some(true) {
                assert!(!calls.is_empty(), "actual selected Life execution");
            }
            let removed = case == "without-gigantic";
            let applies = !removed && values[1].as_bool() == Some(true);
            for call in calls {
                for key in [
                    "exact_actor_store",
                    "exact_actor_output",
                    "summoner_owns_actor",
                    "summoner_actor_is_parent",
                ] {
                    assert_eq!(call[key], true);
                }
                assert_eq!(call["source"], *values[5]);
                assert_eq!(call["return_life"], call["post_return_life"]);
                assert!(
                    call["post_return_observed_at"].as_u64().unwrap()
                        > call["caller_line"].as_u64().unwrap()
                );
                let c = &call["computation"];
                assert_eq!(c["observed_at"], 97);
                assert_eq!(c["override_present"], false);
                assert_eq!(c["life_precision"]["present"], false);
                assert_eq!(c["life_after_assignment"], call["return_life"]);
                assert_eq!(c["gigantic"], !removed);
                assert_eq!(
                    rows(&c["gigantic_records"]).len(),
                    if removed {
                        0
                    } else if case == "duplicate-gigantic-grant" {
                        2
                    } else {
                        1
                    }
                );
                assert_eq!(c["more"].as_f64().unwrap(), if applies { 1.2 } else { 1.0 });
                let life = rows(&c["eligible_life_more"]);
                assert_eq!(life.len(), usize::from(applies));
                for row in life {
                    assert_eq!(row["value"], 20);
                    assert_eq!(row["mod"], source_mod("Life"));
                }
                for stat in ["Life", "Damage"] {
                    let generated: Vec<_> = rows(&c["raw_modifiers"])
                        .iter()
                        .filter(|r| r["mod"]["source"] == "Gigantic" && r["mod"]["name"] == stat)
                        .collect();
                    assert_eq!(generated.len(), usize::from(applies));
                    for row in generated {
                        assert_eq!(row["ancestor_depth"], 0);
                        assert_eq!(row["mod"], source_mod(stat));
                    }
                }
            }
            if let Some(last) = calls.last() {
                assert_eq!(last["post_return_life"], benefits["actor_output_life"]);
            }
            if values[6].as_bool() == Some(true) {
                let damage_calls: Vec<_> = rows(values[8])
                    .iter()
                    .filter(|r| r["damage_type"] == "Physical" && r["critical"] == false)
                    .collect();
                assert_eq!(damage_calls.len(), 1);
                let damage = damage_calls[0];
                assert_eq!(
                    damage["more_factor"].as_f64().unwrap(),
                    if applies { 1.2 } else { 1.0 }
                );
                let generated: Vec<_> = rows(&damage["more_records"])
                    .iter()
                    .filter(|r| r["mod"]["source"] == "Gigantic")
                    .collect();
                assert_eq!(generated.len(), usize::from(applies));
                for row in generated {
                    assert_eq!(row["mod"], source_mod("Damage"));
                }
            } else {
                assert_eq!(values[8]["effect_id"], *values[9]);
                assert!(
                    values[8].get("damage_calls").is_none(),
                    "unselected source frame does not invent calls"
                );
            }
            for p in projections {
                assert!(
                    p["pointer"]
                        .as_str()
                        .unwrap()
                        .starts_with(&format!("/cases/{index}/"))
                );
            }
        }
        for repeat in [1, 3] {
            for mode in ["main", "calcs"] {
                let values = |case: usize| {
                    observations
                        .iter()
                        .find(|o| o["case_index"] == case && o["mode"] == mode)
                        .unwrap()["projections"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .skip(1)
                        .map(|p| p["value"].clone())
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    values(0),
                    values(repeat),
                    "fresh/repeat and warm restoration preserve actual evidence"
                );
            }
        }
        if full {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(json!(bytes.len()), report["bytes"]);
            assert_eq!(hash(&bytes), report["sha256"]);
            let actual: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(actual["source_revision"], a["source_revision"]);
            assert_eq!(actual["manifest_sha256"], a["source_manifest_sha256"]);
            let mut metadata = actual.clone();
            metadata.as_object_mut().unwrap().remove("cases");
            assert_eq!(metadata, v["report_metadata"]);
            for case in actual["cases"].as_array().unwrap() {
                assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
                for field in [
                    "original_functions_preserved",
                    "loaded_state_preserved",
                    "cached_outputs_preserved",
                    "saved_specs_preserved",
                    "fresh_actor_construction",
                    "query_state_preserved",
                ] {
                    assert_eq!(case["state"][field], true);
                }
            }
            for observation in observations {
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
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|x| x.is_empty()) {
        &[]
    } else {
        v.as_array().unwrap()
    }
}
fn source_mod(stat: &str) -> Value {
    json!({"name":stat,"type":"MORE","value":20,"source":"Gigantic","flags":0,"keyword_flags":0,"tags":{}})
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(
        proof.authoring_input,
        digest_owned(DOMAIN, &(a, b, d.clone(), v, m.clone()), 4 * 1024 * 1024).unwrap()
    );
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    for row in m.schema {
        let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(row) =
            row
        else {
            panic!("two Stat descriptors only")
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
    let mut expected: DefinitionRules = serde_json::from_value(d["owners"][0].clone()).unwrap();
    expected
        .programs
        .members
        .extend(m.owners[0].programs.members.clone());
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
    for row in serde_json::from_value::<Vec<DefinitionRules>>(d["status_owners"].clone()).unwrap() {
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
    for row in d["status_receivers"].as_array().unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|x| json!(x) == *row)
                .count(),
            1
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &v, true);
    let receipt = json!(prior.receipt());
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
    assert_eq!(receipt["input"], a["before"]);
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
    let statuses: Vec<DefinitionRules> =
        serde_json::from_value(d["status_owners"].clone()).unwrap();
    for row in old.iter().chain(&statuses) {
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
    for row in d["status_receivers"].as_array().unwrap() {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .receivers
                .members
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
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(DOMAIN, &(a, b, d, v, m), 4 * 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added = [
        registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address(),
        registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address(),
    ];
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 2
    );
    restored
        .schema
        .definitions
        .retain(|row| !added.contains(&row.address()));
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    for row in old {
        let target = restored
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == row.owner)
            .unwrap();
        *target = row;
    }
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only two generic Actor Stats and one exact program append"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
