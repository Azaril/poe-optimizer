//! Current-package publication, import controls and exact five-build preservation.
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_incoming_critical_publication.rs"]
mod packet;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::ConfigurationNumericInput,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn publish(input: &Path, out: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
fn assumptions(draft: &mut Value) -> &mut Vec<Value> {
    assert_eq!(
        draft["draft"]["scenario_presets"]["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    draft["draft"]["scenario_presets"]["members"][0]["scenario"]["assumptions"]["members"]
        .as_array_mut()
        .unwrap()
}
fn expected(input: &ConfigurationNumericInput, value: Option<f64>) -> Vec<Value> {
    let mut result = vec![
        json!({"input":{"kind":"known","value":input.presence_input},"target":{"kind":"enemy"},"value":{"kind":"known","value":{"kind":"boolean","value":value.is_some()}}}),
    ];
    if let Some(value) = value {
        let unit = json!(input.recipe)["codec"]["codec"]["value"]["unit"].clone();
        result.push(json!({"input":{"kind":"known","value":input.value_input},"target":{"kind":"enemy"},"value":{"kind":"known","value":{"kind":"quantity","value":{"value":value,"unit":unit}}}}));
    }
    result
}
fn owns(row: &Value, input: &ConfigurationNumericInput) -> bool {
    row["input"]["value"] == json!(input.presence_input)
        || row["input"]["value"] == json!(input.value_input)
}
fn source_fields(xml: &[u8], names: &[&str]) -> Vec<(usize, String)> {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([71; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    source
        .occurrences()
        .iter()
        .filter(|r| r.name() == "Placeholder")
        .filter_map(|r| {
            let attribute = source.attribute(r.id(), "name").unwrap()?;
            let name = attribute.decoded();
            names
                .contains(&name)
                .then(|| (r.id().ordinal() as usize, name.to_owned()))
        })
        .collect()
}
fn lineages(value: &mut Value, old: &Value, new: &Value) {
    if value.get("local").is_some() && value.get("lineage").is_some() {
        return;
    }
    match value {
        Value::Object(o) => {
            if let Some(lineage) = o.get_mut("lineage") {
                if lineage == new {
                    *lineage = old.clone();
                } else {
                    assert_eq!(lineage, old);
                }
            }
            for v in o.values_mut() {
                lineages(v, old, new);
            }
        }
        Value::Array(a) => {
            for v in a {
                lineages(v, old, new);
            }
        }
        _ => {}
    }
}
#[test]
fn critical_authoring_keeps_raw_inputs_and_partial_membership_explicit() {
    packet::check_authored_source();
    let inputs: Vec<ConfigurationNumericInput> = packet::read("policy.json");
    assert_eq!(
        inputs
            .iter()
            .map(|i| i.source_name.as_str())
            .collect::<Vec<_>>(),
        ["enemyCritChance", "enemyCritDamage"]
    );
    for input in inputs {
        let recipe = json!(input.recipe);
        assert_eq!(recipe["tiers"].as_array().unwrap().len(), 1);
        assert_eq!(
            recipe["tiers"][0]["selectors"],
            json!([{"lane":"input_number","name":input.source_name}])
        );
        assert_eq!(recipe["tiers"][0]["duplicates"], "reject");
        assert_eq!(recipe["missing"]["kind"], "pending");
    }
    let queries: Value = packet::read("queries.json");
    assert_eq!(queries.as_array().unwrap().len(), 3);
    for query in queries.as_array().unwrap() {
        assert_eq!(query["contribution"], "flag");
        assert_eq!(query["groups"][0]["reduction"], "any");
        assert_eq!(query["groups"][0]["ordering"], "unordered");
    }
}

#[test]
#[ignore = "requires the two retained full incoming-critical source reports"]
fn compact_critical_vectors_authenticate_against_full_source_reports() {
    packet::authenticate_source();
}

#[test]
#[ignore = "requires current CRITICAL_PRIOR, source witness and fresh CRITICAL_OUTPUT"]
fn publish_critical_consumer_and_reimport_all_five() {
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_CRITICAL_PRIOR").expect("prior"));
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_CRITICAL_OUTPUT").expect("output"));
    assert!(!out.exists());
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = packet::stage(&prior);
    let inputs: Vec<ConfigurationNumericInput> = packet::read("policy.json");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        out.join("input.json"),
        serde_json::to_vec(next.input()).unwrap(),
    )
    .unwrap();
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("input.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    let mut results = vec![];
    for case in 1..=5 {
        let xml = packet::root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let a = read(old.join("draft.json"));
        let mut b = read(new.join("draft.json"));
        for input in &inputs {
            let rows = assumptions(&mut b);
            assert_eq!(
                rows.iter()
                    .filter(|r| owns(r, input))
                    .cloned()
                    .collect::<Vec<_>>(),
                expected(input, None)
            );
            rows.retain(|r| !owns(r, input));
        }
        let oldlineage = a["draft"]["allocator"]["lineage"].clone();
        let newlineage = b["draft"]["allocator"]["lineage"].clone();
        assert_eq!(
            a["draft"]["allocator"]["last_issued"],
            b["draft"]["allocator"]["last_issued"]
        );
        b["draft"]["allocator"] = a["draft"]["allocator"].clone();
        let mut ids = BTreeMap::new();
        identity::correspond(&a, &mut b, &mut ids, "entire draft inverse");
        let sa = read(old.join("sidecar.json"));
        let mut sb = read(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, &prior_path, case, &prior);
        preservation::authenticate(&sb, &new, &package, case, &next);
        let scenario =
            read(new.join("draft.json"))["draft"]["scenario_presets"]["members"][0]["id"].clone();
        let placeholders = source_fields(&bytes, &["enemyCritChance", "enemyCritDamage"]);
        assert_eq!(placeholders.len(), 2);
        for (ordinal, _) in &placeholders {
            let before = &sa["origins"][*ordinal];
            let after = &mut sb["origins"][*ordinal];
            assert_eq!(before["source"], after["source"]);
            assert_eq!(before["disposition"], json!({"kind":"contributes"}));
            assert_eq!(before["links"].as_array().unwrap().len(), 1);
            assert_eq!(before["links"][0]["kind"], "issue");
            assert_eq!(
                after["disposition"],
                json!({"kind":"source_only","value":"overwritten-config-placeholder"})
            );
            assert_eq!(
                after["links"],
                json!([{"kind":"scenario_preset","value":scenario}])
            );
            // Only the two source-authenticated overwritten values change disposition.
            *after = before.clone();
        }
        for (before, after) in sa["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .zip(sb["item_texts"].as_array_mut().unwrap())
        {
            for key in ["policy", "item_lines"] {
                after["attribution"][key] = before["attribution"][key].clone();
            }
        }
        for key in [
            "policy",
            "mapping",
            "registry",
            "definitions",
            "skill_roles",
            "reward_policy",
            "item_policy",
            "item_source_policy",
            "tree_policy",
            "draft",
            "allocator_after",
        ] {
            sb[key] = sa[key].clone();
        }
        lineages(&mut sb, &oldlineage, &newlineage);
        identity::correspond(&sa, &mut sb, &mut ids, "entire sidecar inverse");
        let mut selection = selected::selection(&bytes, &new);
        identity::relocate(&mut selection, &ids);
        assert_eq!(selection, selected::selection(&bytes, &old));
        let before = selected::finalize_with_definitions(
            &bytes,
            &old,
            &out.join(format!("prior-selected-{case:02}.json")),
            &prior_path.join("schema.json"),
        );
        let after = selected::finalize_with_definitions(
            &bytes,
            &new,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        assert_eq!(after["intent_validation"]["schema_issues"], json!([]));
        let mut issues = after["finalization"]["issues"].clone();
        identity::relocate(&mut issues, &ids);
        assert_eq!(issues, before["finalization"]["issues"]);
        assert_eq!(
            issues.as_array().unwrap().len(),
            [107, 117, 109, 123, 5][case - 1]
        );
        let replay = out.join(format!("replay-{case:02}"));
        release::normalize(&package, &xml, case, &replay);
        preservation::authenticate(
            &read(replay.join("sidecar.json")),
            &replay,
            &package,
            case,
            &next,
        );
        for file in ["draft.json", "sidecar.json"] {
            let mut a = read(new.join(file));
            let mut b = read(replay.join(file));
            selected::canonical(&mut a);
            selected::canonical(&mut b);
            if file == "sidecar.json" {
                a.as_object_mut().unwrap().remove("draft");
                b.as_object_mut().unwrap().remove("draft");
            }
            assert_eq!(a, b, "fresh repeat {case} {file}");
        }
        results.push(json!({"original":case,"new_absence_inputs":2,"accounted_placeholders":placeholders,"selected_issues":issues.as_array().unwrap().len(),"whole_import_inverse":true,"independent_replay":true}));
    }
    let xml = fs::read_to_string(
        packet::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    assert_eq!(xml.matches("</ConfigSet>").count(), 1);
    let mut controls = vec![];
    for input in &inputs {
        for (label, attribute, value) in [
            ("zero", "number=\"0\"", Some(0.)),
            ("negative", "number=\"-7.5\"", Some(-7.5)),
            ("fraction", "number=\"12.25\"", Some(12.25)),
            ("malformed", "number=\"bad\"", None),
            ("wrong-lane", "string=\"12\"", None),
        ] {
            let name = format!("{}-{label}", input.source_name);
            let path = out.join(format!("{name}.xml"));
            fs::write(
                &path,
                xml.replace(
                    "</ConfigSet>",
                    &format!(
                        "<Input name=\"{}\" {attribute}/></ConfigSet>",
                        input.source_name
                    ),
                ),
            )
            .unwrap();
            let dir = out.join(&name);
            release::normalize(&package, &path, 5, &dir);
            let mut draft = read(dir.join("draft.json"));
            let rows: Vec<_> = assumptions(&mut draft)
                .iter()
                .filter(|r| owns(r, input))
                .cloned()
                .collect();
            if let Some(value) = value {
                assert_eq!(rows, expected(input, Some(value)));
            } else {
                assert!(rows.is_empty());
            }
            controls.push(json!({"case":name,"projected":value}));
        }
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    fs::write(out.join("validation.json"),serde_json::to_vec_pretty(&json!({"input":next.receipt().input,"rules":next.receipt().rules,"originals":results,"controls":controls,"queries":110,"complete_native_builds":0,"upstream_membership_complete":false})).unwrap()).unwrap();
}
