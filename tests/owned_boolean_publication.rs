//! One explicit development-format cutover, with source evidence and full import preservation.
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{owned_content::digest_owned, owned_rules::*};
use poe_optimizer_import::owned_release::{OwnedReleaseInput, OwnedReleaseProvenance};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/inherent-attribute-flags")
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn authored_passive_flags_are_source_bound_and_do_not_complete_missing_domains() {
    let rules = read(data().join("rules.json"));
    let evidence = read(data().join("source.json"));
    let manifest = read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"));
    assert_eq!(evidence["upstream_revision"], manifest["upstream_revision"]);
    for pin in evidence["pins"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
        let path = root()
            .join("vendor/path-of-building-poe2")
            .join(pin["path"].as_str().unwrap());
        // Checkout validation is additional when the optional submodule is present.
        if path.exists() {
            let text = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
            assert_eq!(pin["bytes"], text.len());
            assert_eq!(pin["sha256"], hash(text.as_bytes()));
        }
    }
    assert_eq!(rules["producers"].as_array().unwrap().len(), 2);
    assert_eq!(evidence["producers"].as_array().unwrap().len(), 2);
    let queries: Vec<ContributionQuery> = serde_json::from_value(rules["queries"].clone()).unwrap();
    assert_eq!(queries.len(), 5);
    assert_eq!(
        queries
            .iter()
            .map(|q| q.groups[0].members.members.len())
            .collect::<Vec<_>>(),
        [0, 0, 0, 1, 1]
    );
    for q in &queries {
        assert_eq!(q.contribution, ContributionKind::Flag);
        let g = &q.groups[0];
        assert_eq!(g.ordering, ContributionOrdering::Unordered);
        assert_eq!(g.reduction, ContributionReduction::Any);
        assert_eq!(
            g.empty,
            Some(poe_optimizer_core::owned_build::ParameterValue::Boolean(
                false
            ))
        );
        assert!(!g.members.is_complete());
        assert!(g.members.members.iter().all(|m| m.order.is_none()
            && matches!(
                &m.producer,
                ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    origin: ContributionOrigin::Allocation,
                    ..
                })
            )));
    }
    for (producer, source) in rules["producers"]
        .as_array()
        .unwrap()
        .iter()
        .zip(evidence["producers"].as_array().unwrap())
    {
        assert_eq!(producer["owner"], source["owner"]);
        assert_eq!(producer["program"]["id"], source["program"]);
        assert_eq!(
            producer["program"]["effects"][0]["effect"]["stat"],
            source["stat"]
        );
        assert!(!source["remaining_mechanics"].as_array().unwrap().is_empty());
        let _: RuleProgram = serde_json::from_value(producer["program"].clone()).unwrap();
    }
    assert!(
        evidence["scope"]
            .as_object()
            .unwrap()
            .values()
            .all(|v| v == false)
    );
}

/// Test-only inverse of this single authored cutover. Never used by a loader.
fn current_contract(value: &mut Value) {
    match value {
        Value::Array(rows) => rows.iter_mut().for_each(current_contract),
        Value::Object(row) => {
            if row.get("kind") == Some(&json!("ordered_contributions")) {
                row.insert("kind".into(), json!("contribution_query"));
            }
            if let Some(old) = row.remove("ordered_contributions") {
                row.insert("contribution_queries".into(), old);
            }
            if ["reduction", "empty", "members"]
                .iter()
                .all(|k| row.contains_key(*k))
            {
                row.insert("ordering".into(), json!("ordered"));
            }
            if ["owner", "program", "effect", "order"]
                .iter()
                .all(|k| row.contains_key(*k))
            {
                let order = row.get_mut("order").unwrap().as_object_mut().unwrap();
                let mut origin = order.remove("origin").unwrap();
                let ranks = origin
                    .as_object_mut()
                    .unwrap()
                    .remove("slots")
                    .unwrap_or(json!([]));
                if matches!(
                    origin["kind"].as_str(),
                    Some("equipment_use" | "item_modifier")
                ) {
                    origin["slots"] = json!(
                        ranks
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|s| &s["slot"])
                            .collect::<Vec<_>>()
                    );
                }
                order.insert("slot_ranks".into(), ranks);
                row.insert("origin".into(), origin);
            }
            row.values_mut().for_each(current_contract);
        }
        _ => {}
    }
}

#[test]
#[ignore = "requires authenticated BOOLEAN_INPUT, prior package/imports and a fresh BOOLEAN_OUTPUT"]
fn publish_flags_and_reimport_all_five_through_the_public_cli() {
    let env = |key: &str| PathBuf::from(std::env::var_os(key).expect(key));
    let input_path = env("POE_OPTIMIZER_TEST_BOOLEAN_INPUT");
    let prior = env("POE_OPTIMIZER_TEST_BOOLEAN_PRIOR");
    let imports = env("POE_OPTIMIZER_TEST_BOOLEAN_IMPORTS");
    let out = env("POE_OPTIMIZER_TEST_BOOLEAN_OUTPUT");
    assert!(!out.exists());
    let old_inventory = release::inventory(&prior);
    let receipt = read(prior.join("release.json"));
    assert_eq!(
        receipt["input"],
        "b19134496c85e208a735568b5e5191f3b60f68154be336ab4afb6fd953b6fa01"
    );
    for pin in receipt["artifacts"].as_array().unwrap() {
        let b = fs::read(prior.join(pin["file"].as_str().unwrap())).unwrap();
        assert_eq!(pin["sha256"], hash(&b));
        assert_eq!(pin["bytes"], b.len());
    }
    let authored = read(data().join("rules.json"));
    let sources = read(data().join("source.json"));
    let mut input: OwnedReleaseInput =
        serde_json::from_slice(&fs::read(input_path).unwrap()).unwrap();
    assert_eq!(json!(input.provenance), receipt["provenance"]);
    assert_eq!(
        input.recipe.rules.schema_version,
        OWNED_RULE_PACKAGE_VERSION
    );
    assert_eq!(
        input.recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V22
    );
    let mut inverse = json!(input.recipe.rules);
    let mut old_rules = read(prior.join("rules.json"));
    assert_eq!(old_rules["schema_version"], 2);
    assert_eq!(old_rules["operations_version"], OWNED_RULE_OPERATIONS_V21);
    current_contract(&mut old_rules);
    for p in authored["producers"].as_array().unwrap() {
        let owner = inverse["owners"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["owner"] == p["owner"])
            .unwrap();
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
        assert_eq!(owner["programs"]["members"], json!([p["program"].clone()]));
        owner["programs"]["members"] = json!([]);
        let mapped = read(prior.join("mapping.json"));
        let source = sources["producers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["owner"] == p["owner"])
            .unwrap();
        let source_id = source["node_id"].as_u64().unwrap().to_string();
        assert!(
            mapped["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["outcome"]["value"]["target"] == p["owner"]
                    && r["source"]["value"]["value"]["node_id"]["value"] == source_id)
        );
    }
    for field in ["owners", "receivers", "queries"] {
        let rows = match field {
            "owners" => inverse["owners"].as_array_mut().unwrap(),
            "receivers" => inverse["receivers"]["members"].as_array_mut().unwrap(),
            _ => inverse["contribution_queries"]["members"]
                .as_array_mut()
                .unwrap(),
        };
        for row in authored[field].as_array().unwrap().iter().rev() {
            assert_eq!(rows.pop().as_ref(), Some(row));
        }
    }
    for field in ["schema_version", "operations_version", "release"] {
        inverse[field] = old_rules[field].clone();
    }
    assert_eq!(
        inverse, old_rules,
        "only reviewed producers, reducers and current DTO may change"
    );
    let authoring = digest_owned(
        "owned-inherent-attribute-flags-v1",
        &(&authored, &sources),
        1024 * 1024,
    )
    .unwrap();
    input.provenance.push(OwnedReleaseProvenance {
        kind: "inherent-attribute-flags-v1".parse().unwrap(),
        prior_input: serde_json::from_value(receipt["input"].clone()).unwrap(),
        authoring_input: authoring,
    });
    fs::create_dir_all(&out).unwrap();
    let publish_input = out.join("input.json");
    fs::write(&publish_input, serde_json::to_vec(&input).unwrap()).unwrap();
    let package = out.join("package");
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(&publish_input)
        .arg("--output")
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let endpoint = release::load(&package);
    assert_eq!(endpoint.receipt().provenance, input.provenance);
    assert_eq!(endpoint.receipt().query_rows, 110);
    assert_ne!(json!(endpoint.receipt().input), receipt["input"]);
    for (name, bytes) in endpoint.artifacts() {
        if !["rules.json", "manifest.json", "release.json"].contains(&name) {
            assert_eq!(
                bytes,
                fs::read(prior.join(name)).unwrap(),
                "unchanged {name}"
            );
        }
    }
    let mut results = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let source = fs::read(&xml).unwrap();
        let before = imports.join(format!("original-{case:02}"));
        let old = release::inventory(&before);
        let after = out.join(format!("original-{case:02}"));
        release::normalize(&package, &xml, case, &after);
        for directory in [&before, &after] {
            let sidecar = read(directory.join("sidecar.json"));
            preservation::authenticate(&sidecar, directory, &package, case, &endpoint);
            assert_eq!(sidecar["source_sha256"], hash(&source));
            assert_eq!(sidecar["source_bytes"], source.len());
        }
        for name in ["draft.json", "sidecar.json"] {
            let mut a = read(before.join(name));
            let mut b = read(after.join(name));
            selected::canonical(&mut a);
            selected::canonical(&mut b);
            if name == "sidecar.json" {
                a.as_object_mut().unwrap().remove("draft");
                b.as_object_mut().unwrap().remove("draft");
            }
            assert_eq!(
                a, b,
                "every input and disposition survives case {case} {name}"
            );
        }
        let report = selected::finalize_with_definitions(
            &source,
            &after,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        assert_eq!(
            report["finalization"]["issues"].as_array().unwrap().len(),
            [107, 117, 109, 123, 5][case - 1]
        );
        let prior_report = selected::finalize_with_definitions(
            &source,
            &before,
            &out.join(format!("prior-selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        let mut a = prior_report["finalization"].clone();
        let mut b = report["finalization"].clone();
        for result in [&mut a, &mut b] {
            result.as_object_mut().unwrap().remove("draft_digest");
            selected::canonical(result);
        }
        assert_eq!(a, b, "entire selected finalization survives");
        let mut a = selected::selection(&source, &before);
        let mut b = selected::selection(&source, &after);
        selected::canonical(&mut a);
        selected::canonical(&mut b);
        assert_eq!(a, b, "saved selections survive");
        let replay = out.join(format!("replay-{case:02}"));
        release::normalize(&package, &xml, case, &replay);
        preservation::authenticate(
            &read(replay.join("sidecar.json")),
            &replay,
            &package,
            case,
            &endpoint,
        );
        for name in ["draft.json", "sidecar.json"] {
            let mut a = read(after.join(name));
            let mut b = read(replay.join(name));
            selected::canonical(&mut a);
            selected::canonical(&mut b);
            if name == "sidecar.json" {
                a.as_object_mut().unwrap().remove("draft");
                b.as_object_mut().unwrap().remove("draft");
            }
            assert_eq!(a, b);
        }
        assert_eq!(old, release::inventory(&before));
        results.push(json!({"original":case,"inputs_and_dispositions_unchanged":true,"independent_replay":true,"selected_issues":report["finalization"]["issues"].as_array().unwrap().len()}));
    }
    assert_eq!(old_inventory, release::inventory(&prior));
    fs::write(out.join("validation.json"),serde_json::to_vec_pretty(&json!({"input":endpoint.receipt().input,"rules":endpoint.receipt().rules,"cases":results,"query_rows":110,"complete_builds":0,"flag_inventory_complete":false})).unwrap()).unwrap();
}
