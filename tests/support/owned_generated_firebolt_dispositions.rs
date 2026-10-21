//! Exact generated field destinations using the maintained release path.
use crate::{family, release, selected};
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, configuration_disposition_dependencies, usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const BEFORE: &str = "6f58eba00e71984264123419f51114f375bdce23440eaa4ff3ffa4999e2beb11";
const BINDINGS: &str = "bfbbd8e70a5d4a777700322624323a6f0809923a53b55671d118ef0862a9ecfe";
const REVIEW: &str = "fe21b428e73a8b285d7d08b7c2ad51dc46739e1e48726126de4308dbe9ef5f38";
fn data() -> PathBuf {
    family::root().join("data/owned/poe2/3887ae68/generated-firebolt-dispositions")
}
fn sealed(name: &str, hash: &str) -> Value {
    let bytes = fs::read(data().join(name)).unwrap();
    assert_eq!(family::sha(&bytes), hash);
    serde_json::from_slice(&bytes).unwrap()
}
pub fn check() -> (Value, Value) {
    let bindings = sealed("bindings.json", BINDINGS);
    let review = sealed("review.json", REVIEW);
    assert_eq!(bindings["before"], BEFORE);
    assert_eq!(bindings["evidence"], REVIEW);
    assert_eq!(
        bindings["prior_evidence"],
        review["predecessor"]["inventory_evidence"]
    );
    assert_eq!(bindings["selectors"], review["selectors"]);
    assert_eq!(bindings["selectors"].as_array().unwrap().len(), 15);
    for pin in review["producer_consumer_files"].as_array().unwrap() {
        let bytes = fs::read(family::root().join(pin["path"].as_str().unwrap())).unwrap();
        let normalized = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
        assert_eq!(family::sha(normalized.as_bytes()), pin["sha256"]);
        assert_eq!(normalized.len(), pin["bytes"]);
    }
    let source = &review["original05"]["source"];
    let bytes = fs::read(family::root().join(source["path"].as_str().unwrap())).unwrap();
    assert_eq!(family::sha(&bytes), source["sha256"]);
    assert_eq!(bytes.len(), source["bytes"]);
    for pin in review["reporting"]["packet_pins"]
        .as_array()
        .unwrap()
        .iter()
        .chain(std::iter::once(&review["reporting"]["source_manifest"]))
    {
        let bytes = fs::read(family::root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(family::sha(&bytes), pin["sha256"]);
        assert_eq!(bytes.len(), pin["bytes"]);
    }
    let manifest = family::read(
        family::root().join(
            review["reporting"]["source_manifest"]["path"]
                .as_str()
                .unwrap(),
        ),
    );
    assert_eq!(
        manifest["upstream_revision"],
        review["reporting"]["source_revision"]
    );
    for pin in review["reporting"]["source_bodies"].as_array().unwrap() {
        let matches: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["path"] == pin["path"])
            .collect();
        assert_eq!(matches, vec![pin]);
    }
    check_enabled_evidence(&review["enabled_gate"], &manifest);
    let source_evidence = family::read(
        family::root().join("data/owned/poe2/3887ae68/reporting-inclusion/source-evidence.json"),
    );
    assert_eq!(
        source_evidence["attribution"]["cases"]
            .as_array()
            .unwrap()
            .len(),
        44
    );
    assert_eq!(
        source_evidence["attribution"]["raw_sha256"],
        review["reporting"]["expanded_attribution"]["sha256"]
    );
    assert_eq!(
        source_evidence["attribution"]["raw_bytes"],
        review["reporting"]["expanded_attribution"]["bytes"]
    );
    check_rows(&bindings);
    (bindings, review)
}
fn check_enabled_evidence(proof: &Value, manifest: &Value) {
    assert_eq!(proof["schema_version"], 1);
    assert_eq!(proof["scope"], "literal-enabled-true-nonrestricting");
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    // This is an authenticated authoring record, not the execution result.
    // The separate source-test gate records actual validation at publication.
    assert_eq!(proof["status"], "tests-pending");
    assert_eq!(proof["law"]["raw_token"], "true");
    assert_eq!(proof["law"]["decoded_token"], "true");
    assert_eq!(
        proof["law"]["retained_usage_destinations"],
        json!([
            {"field":"Skill.enabled","policy":"def.000000000000332b","parameter":"def.000000000000332c"},
            {"field":"Gem.enabled","policy":"def.000000000000332b","parameter":"def.000000000000332d"}
        ])
    );
    assert_eq!(proof["component_tests"], 5);
    for pin in proof["files"].as_array().unwrap() {
        let bytes = fs::read(family::root().join(pin["path"].as_str().unwrap())).unwrap();
        let bytes = match pin["encoding"].as_str().unwrap() {
            "raw" => bytes,
            "utf8-lf" => String::from_utf8(bytes)
                .unwrap()
                .replace("\r\n", "\n")
                .into_bytes(),
            _ => panic!("unrecognized evidence encoding"),
        };
        assert_eq!(family::sha(&bytes), pin["sha256"]);
        assert_eq!(bytes.len(), pin["bytes"]);
    }
    // Ordinary native CLI checks use the bundled manifest; no vendor checkout
    // or Lua host is introduced into this test target.
    for pin in proof["source_files"].as_array().unwrap() {
        assert_eq!(pin["encoding"], "utf8-lf");
        let matches: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["path"] == pin["path"])
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["sha256"], pin["sha256"]);
        assert_eq!(matches[0]["bytes"], pin["bytes"]);
    }
}
fn check_rows(packet: &Value) {
    let selectors = packet["selectors"].as_array().unwrap();
    let rows = packet["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let mut intrinsic = 0;
    let mut reporting = 0;
    for (i, pair) in rows.iter().enumerate() {
        let before = &pair["before"];
        let mut inverse = pair["after"].clone();
        assert_eq!(before["source"], inverse["source"]);
        let before_fields = before["fields"].as_array().unwrap();
        let after_fields = inverse["fields"].as_array_mut().unwrap();
        assert_eq!(before_fields.len(), 21);
        assert_eq!(after_fields.len(), before_fields.len());
        for (old, new) in before_fields.iter().zip(after_fields) {
            assert_eq!(old["field"], new["field"]);
            if i == 0 && selectors.contains(&new["field"]) {
                let mut destinations = old["destinations"].as_array().unwrap().clone();
                let deferred: Vec<_> = destinations
                    .iter()
                    .enumerate()
                    .filter(|(_, destination)| destination["kind"] == "deferred")
                    .map(|(index, _)| index)
                    .collect();
                assert_eq!(deferred.len(), 1);
                destinations[deferred[0]] = json!({"kind":"input_disposition"});
                if new["field"]["name"] == "enabled" {
                    assert_eq!(destinations.len(), 2);
                    let usage = destinations
                        .iter()
                        .find(|destination| destination["kind"] == "usage_parameter")
                        .unwrap();
                    assert_eq!(usage["policy"]["key"], "def.000000000000332b");
                    assert_eq!(
                        usage["parameter"]["slot"]["key"],
                        if new["field"]["kind"] == "group_attribute" {
                            "def.000000000000332c"
                        } else {
                            "def.000000000000332d"
                        }
                    );
                } else {
                    assert_eq!(destinations.len(), 1);
                }
                assert_eq!(new["destinations"], json!(destinations));
                intrinsic += 1;
            } else if new["field"] == json!({"kind":"group_attribute","name":"includeInFullDPS"}) {
                let original = old["destinations"].as_array().unwrap();
                assert_eq!(original.len(), 2);
                let retained: Vec<_> = original
                    .iter()
                    .filter(|d| d["kind"] != "deferred")
                    .cloned()
                    .collect();
                assert_eq!(retained.len(), 1);
                assert_eq!(retained[0]["kind"], "usage_parameter");
                assert_eq!(retained[0]["policy"]["key"], "def.000000000000338e");
                assert_eq!(
                    retained[0]["parameter"]["slot"]["key"],
                    "def.000000000000338f"
                );
                assert_eq!(new["destinations"], json!(retained));
                reporting += 1;
            } else {
                assert_eq!(new, old);
            }
            new["destinations"] = old["destinations"].clone();
        }
        assert_eq!(
            &inverse, before,
            "whole row inverse: no source or other consumer changes"
        );
        assert_eq!(
            pair["after"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["destinations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["kind"] == "deferred"))
                .count(),
            [4, 19][i]
        );
    }
    assert_eq!((intrinsic, reporting), (15, 2));
}
pub fn mutation_controls() {
    let (packet, _) = check();
    for selector in packet["selectors"].as_array().unwrap() {
        let mut changed = packet.clone();
        let row = changed["rows"][0]["after"]["fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|field| &field["field"] == selector)
            .unwrap();
        row["destinations"] = json!([{"kind":"deferred","reason":"missing-checked-proof"}]);
        assert!(std::panic::catch_unwind(|| check_rows(&changed)).is_err());
    }
    for kind in ["group_attribute", "occurrence_attribute"] {
        for omitted in ["input_disposition", "usage_parameter"] {
            let mut changed = packet.clone();
            let row = changed["rows"][0]["after"]["fields"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|field| field["field"] == json!({"kind":kind,"name":"enabled"}))
                .unwrap();
            row["destinations"]
                .as_array_mut()
                .unwrap()
                .retain(|destination| destination["kind"] != omitted);
            assert!(
                std::panic::catch_unwind(|| check_rows(&changed)).is_err(),
                "the reviewed policy retains both independent enabled consumers"
            );
        }
    }
    for index in 0..2 {
        let mut changed = packet.clone();
        let row = changed["rows"][index]["after"]["fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|field| {
                field["field"] == json!({"kind":"group_attribute","name":"includeInFullDPS"})
            })
            .unwrap();
        row["destinations"] = json!([{"kind":"input_disposition"}]);
        assert!(std::panic::catch_unwind(|| check_rows(&changed)).is_err());
    }
}
fn stage(prior: &StagedOwnedRelease, packet: &Value) -> StagedOwnedRelease {
    assert_eq!(prior.receipt().input.to_string(), BEFORE);
    let old = prior.input();
    assert_eq!(old.query_sets.len(), 5);
    assert_eq!(
        old.query_sets
            .iter()
            .map(|set| set.queries.len())
            .sum::<usize>(),
        110
    );
    let mut policy = serde_json::to_value(&old.normalization).unwrap();
    let inventory = &mut policy["usage_inputs"]["inventory"];
    assert_eq!(inventory["evidence"], packet["prior_evidence"]);
    for pair in packet["rows"].as_array().unwrap() {
        let matches: Vec<_> = inventory["rows"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, row)| row["source"] == pair["before"]["source"])
            .map(|(i, _)| i)
            .collect();
        assert_eq!(matches.len(), 1);
        let row = &mut inventory["rows"][matches[0]];
        assert_eq!(row, &pair["before"]);
        *row = pair["after"].clone();
    }
    inventory["evidence"] = packet["evidence"].clone();
    let mut normalization = serde_json::from_value(policy).unwrap();
    let identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("current physical inventory")
    };
    *usage_inputs = identity;
    let dependencies =
        configuration_disposition_dependencies(&normalization, prior.rewards(), Default::default())
            .unwrap();
    let disposition = normalization.configuration_dispositions.as_mut().unwrap();
    let mut dependency_inverse = dependencies.clone();
    dependency_inverse.usage_inputs = disposition.dependencies.usage_inputs;
    assert_eq!(dependency_inverse, disposition.dependencies);
    disposition.dependencies = dependencies;
    let mut inverse = serde_json::to_value(&normalization).unwrap();
    let original = serde_json::to_value(&old.normalization).unwrap();
    inverse["usage_inputs"]["inventory"] = original["usage_inputs"]["inventory"].clone();
    inverse["gem_inventory"]["usage_inputs"] = original["gem_inventory"]["usage_inputs"].clone();
    inverse["configuration_dispositions"]["dependencies"] =
        original["configuration_dispositions"]["dependencies"].clone();
    assert_eq!(inverse, original);
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: old.recipe.clone(),
            successor: old.recipe.clone(),
            mapping: old.mapping.clone(),
            roles: old.roles.clone(),
            normalization: old.normalization.clone(),
            rewards: old.rewards.clone(),
            query_sets: old.query_sets.clone(),
            items: old.items.clone(),
            item_source: old.item_source.clone(),
        },
        old.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = old.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|t| t.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("checked-generated-intrinsic-and-reporting-destinations")
            .unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: REVIEW.parse().unwrap(),
    });
    let mut inverse = full.clone();
    inverse.normalization = old.normalization.clone();
    inverse.tree = old.tree.clone();
    inverse.provenance.pop();
    assert_eq!(&inverse, old);
    assemble_owned_release(full, Default::default()).unwrap()
}
fn actual_bindings(output: &family::Output, expected: &Value) {
    let origins = output.sidecar["origins"].as_array().unwrap();
    let origin = |n| {
        origins
            .iter()
            .find(|r| r["source"]["ordinal"] == n)
            .unwrap()
            .clone()
    };
    let snapshot = &expected["original05"]["observations"];
    let draft = &output.draft["draft"];
    let preset = draft["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"]["local"] == "00000000000001dd")
        .unwrap();
    assert_eq!(
        preset["intent"]["usage"]["completion"],
        snapshot["completion"]
    );
    let expected_binding = &snapshot["bindings"][0];
    let binding_matches: Vec<_> = preset["intent"]["generated_inputs"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["target"] == expected_binding["target"])
        .cloned()
        .collect();
    assert_eq!(json!(binding_matches), snapshot["bindings"]);
    let expected_target = &snapshot["usages"][0]["selection"]["target"];
    let usages: Vec<_> = preset["intent"]["usage"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| &b["selection"]["target"] == expected_target)
        .cloned()
        .collect();
    assert_eq!(json!(usages), snapshot["usages"]);
    let reporting: Vec<_> = usages
        .iter()
        .filter(|usage| usage["selection"]["policy"]["value"]["key"] == "def.000000000000338e")
        .collect();
    assert_eq!(reporting.len(), 1);
    let parameters = &reporting[0]["selection"]["parameters"]["members"];
    assert_eq!(parameters.as_array().unwrap().len(), 1);
    assert_eq!(
        parameters[0]["slot"]["value"]["slot"]["key"],
        "def.000000000000338f"
    );
    assert_eq!(
        parameters[0]["value"],
        json!({"kind":"known","value":{"kind":"boolean","value":false}})
    );
    let participation: Vec<_> = usages
        .iter()
        .filter(|usage| usage["selection"]["policy"]["value"]["key"] == "def.000000000000332b")
        .collect();
    assert_eq!(participation.len(), 1);
    let parameters = participation[0]["selection"]["parameters"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(parameters.len(), 2);
    for slot in ["def.000000000000332c", "def.000000000000332d"] {
        let matching: Vec<_> = parameters
            .iter()
            .filter(|parameter| parameter["slot"]["value"]["slot"]["key"] == slot)
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(
            matching[0]["value"],
            json!({"kind":"known","value":{"kind":"boolean","value":true}})
        );
    }
    for (collection, key) in [("equipment", "equipment"), ("items", "item")] {
        let matches: Vec<_> = draft[collection]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == snapshot[key]["id"])
            .collect();
        assert_eq!(matches, vec![&snapshot[key]]);
    }
    assert_eq!(json!([origin(243), origin(244)]), snapshot["origins"]);
    assert_eq!(json!([origin(174), origin(175)]), snapshot["archived"]);
}
pub fn run() {
    let (packet, review) = check();
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_DISPOSITIONS_PRIOR").unwrap());
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_DISPOSITIONS_OUTPUT").unwrap(),
    );
    assert!(!out.exists());
    for name in ["release", "normalization"] {
        let bytes = fs::read(prior_path.join(format!("{name}.json"))).unwrap();
        assert_eq!(family::sha(&bytes), review["predecessor"][name]["sha256"]);
        assert_eq!(bytes.len(), review["predecessor"][name]["bytes"]);
    }
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = stage(&prior, &packet);
    fs::create_dir_all(&out).unwrap();
    family::write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    assert_eq!(
        family::publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        family::publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let files = release::inventory(&package);
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    let changed: Vec<_> = files
        .keys()
        .filter(|key| prior_files.get(*key) != files.get(*key))
        .map(String::as_str)
        .collect();
    assert_eq!(
        changed,
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    assert_eq!(files.len(), prior_files.len());
    let mut cases = Vec::new();
    for case in 1..=5 {
        let source = family::root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&source).unwrap();
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        let repeat = out.join(format!("repeated-original-{case:02}"));
        release::normalize(&prior_path, &source, case, &old);
        release::normalize(&package, &source, case, &new);
        release::normalize(&package, &source, case, &repeat);
        let before = family::checked_output(&old, &prior_path, case, &prior, &xml);
        let after = family::checked_output(&new, &package, case, &next, &xml);
        assert_eq!(
            before, after,
            "whole draft, allocator, bindings, origins and report"
        );
        assert_eq!(
            after,
            family::checked_output(&repeat, &package, case, &next, &xml)
        );
        if case == 5 {
            actual_bindings(&before, &review);
            actual_bindings(&after, &review);
        }
        let check = |dir: &Path, schema: &Path, file: &Path| {
            let report = selected::finalize_with_definitions(&xml, dir, file, schema);
            let mut issues = report["finalization"]["issues"].clone();
            selected::canonical(&mut issues);
            issues
        };
        let old_issues = check(
            &old,
            &prior_path.join("schema.json"),
            &out.join(format!("prior-selection-{case:02}.json")),
        );
        let new_issues = check(
            &new,
            &package.join("schema.json"),
            &out.join(format!("selection-{case:02}.json")),
        );
        assert_eq!(old_issues, new_issues);
        assert_eq!(
            new_issues.as_array().unwrap().len(),
            [123, 137, 127, 144, 5][case - 1]
        );
        assert_eq!(fs::read(&source).unwrap(), xml);
        cases.push(json!({"original":case,"selected_pending_issues":new_issues.as_array().unwrap().len(),"whole_checked_output_unchanged":true,"fresh_replay_identical":true}));
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    family::write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":cases,"intrinsic_destinations":15,"enabled_nonrestricting_destinations":2,"retained_enabled_participation_destinations":2,"reporting_deferred_destinations_removed":2,"retired_issues":0,"complete_original_builds":0,"queries_unchanged":110,"rebuild_byte_identical":true,"actual_persisted_bindings_and_origins_preserved":true,"private_actual_receipts_observed":false,"calculation":"not_run"}),
    );
}
