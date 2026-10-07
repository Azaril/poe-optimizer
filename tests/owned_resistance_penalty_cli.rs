//! Real-package import/default publication, preserving all five unresolved builds.
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_resistance_penalty_publication.rs"]
mod packet;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
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
    let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
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
fn expected(value: f64) -> Value {
    let input: Value = packet::read("input.json");
    let mut v = input["constructor_default"].clone();
    v["value"]["value"] = json!(value);
    json!({"input":{"kind":"known","value":input["value_input"]},"target":{"kind":"actor","value":{"kind":"player"}},"value":{"kind":"known","value":v}})
}
#[test]
fn penalty_configuration_has_player_authority_and_explicit_typed_default() {
    let input: poe_optimizer_import::owned_normalize::ConfigurationDefaultInput =
        packet::read("input.json");
    assert_eq!(json!(input.target), "player");
    assert_eq!(input.source_name, "resistancePenalty");
    assert_eq!(json!(input.constructor_default)["value"]["value"], -60.);
    assert_eq!(input.value_input.key().as_str(), "def.000000000000334d");
}
#[test]
#[ignore = "requires checked current package and fresh publication directory"]
fn publish_penalty_input_and_native_consumer_preserving_originals() {
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PENALTY_PRIOR").expect("prior package"));
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PENALTY_OUTPUT").expect("fresh output"));
    assert!(!out.exists());
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = packet::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    fs::write(
        out.join("endpoint.json"),
        serde_json::to_vec(next.input()).unwrap(),
    )
    .unwrap();
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
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
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let a = read(old.join("draft.json"));
        let mut b = read(new.join("draft.json"));
        let values = assumptions(&mut b);
        assert_eq!(values.pop().unwrap(), expected(-60.));
        let oldlineage = a["draft"]["allocator"]["lineage"].clone();
        let newlineage = b["draft"]["allocator"]["lineage"].clone();
        assert_eq!(
            a["draft"]["allocator"]["last_issued"],
            b["draft"]["allocator"]["last_issued"]
        );
        b["draft"]["allocator"] = a["draft"]["allocator"].clone();
        let mut ids = BTreeMap::new();
        identity::correspond(&a, &mut b, &mut ids, "whole draft inverse");
        let sa = read(old.join("sidecar.json"));
        let mut sb = read(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, &prior_path, case, &prior);
        preservation::authenticate(&sb, &new, &package, case, &next);
        assert_eq!(sa["allocator_after"], a["draft"]["allocator"]);
        assert_eq!(
            sb["allocator_after"],
            read(new.join("draft.json"))["draft"]["allocator"]
        );
        let old_items = sa["item_texts"].as_array().unwrap();
        let new_items = sb["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_items.len(), new_items.len());
        for (old, new) in old_items.iter().zip(new_items) {
            for field in ["policy", "item_lines"] {
                new["attribution"][field] = old["attribution"][field].clone();
            }
        }
        // Only authenticated dependency/derived commitments can change.
        for field in [
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
            sb[field] = sa[field].clone();
        }
        fn lineages(v: &mut Value, old: &Value, new: &Value) {
            if v.get("local").is_some() && v.get("lineage").is_some() {
                return;
            }
            match v {
                Value::Object(o) => {
                    if let Some(l) = o.get_mut("lineage") {
                        if l == new {
                            *l = old.clone()
                        } else {
                            assert_eq!(l, old)
                        }
                    }
                    for x in o.values_mut() {
                        lineages(x, old, new)
                    }
                }
                Value::Array(a) => {
                    for x in a {
                        lineages(x, old, new)
                    }
                }
                _ => {}
            }
        }
        lineages(&mut sb, &oldlineage, &newlineage);
        identity::correspond(&sa, &mut sb, &mut ids, "whole sidecar inverse");
        let bytes = fs::read(&xml).unwrap();
        let mut s = selected::selection(&bytes, &new);
        identity::relocate(&mut s, &ids);
        assert_eq!(s, selected::selection(&bytes, &old));
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
        results.push(json!({"original":case,"added_assumptions":1,"whole_import_inverse":true,"selected_issues":issues.as_array().unwrap().len()}));
    }
    let original = fs::read_to_string(
        packet::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    assert_eq!(original.matches("</ConfigSet>").count(), 1);
    assert!(!original.contains("name=\"resistancePenalty\""));
    let cases = [
        (
            "zero",
            r#"<Input name="resistancePenalty" number="0"/>"#,
            Some(0.),
        ),
        (
            "act",
            r#"<Input name="resistancePenalty" number="-30"/>"#,
            Some(-30.),
        ),
        (
            "fraction",
            r#"<Input name="resistancePenalty" number="-12.5"/>"#,
            Some(-12.5),
        ),
        (
            "positive",
            r#"<Input name="resistancePenalty" number="10"/>"#,
            Some(10.),
        ),
        (
            "malformed",
            r#"<Input name="resistancePenalty" number="bad"/>"#,
            None,
        ),
        (
            "wrong-lane",
            r#"<Input name="resistancePenalty" string="-30"/>"#,
            None,
        ),
        (
            "placeholder",
            r#"<Placeholder name="resistancePenalty" number="20"/>"#,
            Some(-60.),
        ),
        (
            "zero-with-placeholder",
            r#"<Input name="resistancePenalty" number="0"/><Placeholder name="resistancePenalty" number="20"/>"#,
            Some(0.),
        ),
        (
            "malformed-placeholder",
            r#"<Placeholder name="resistancePenalty" number="bad"/>"#,
            None,
        ),
        (
            "string-placeholder",
            r#"<Placeholder name="resistancePenalty" string="20"/>"#,
            None,
        ),
        (
            "duplicate",
            r#"<Input name="resistancePenalty" number="0"/><Input name="resistancePenalty" number="-30"/>"#,
            None,
        ),
    ];
    let mut controls = vec![];
    for (name, addition, value) in cases {
        let xml = out.join(format!("{name}.xml"));
        fs::write(
            &xml,
            original.replace("</ConfigSet>", &format!("{addition}</ConfigSet>")),
        )
        .unwrap();
        let dest = out.join(format!("control-{name}"));
        release::normalize(&package, &xml, 5, &dest);
        let mut draft = read(dest.join("draft.json"));
        let values = assumptions(&mut draft);
        let rows: Vec<_> = values
            .iter()
            .filter(|v| v["input"]["value"]["key"] == "def.000000000000334d")
            .collect();
        if let Some(value) = value {
            assert_eq!(rows, vec![&expected(value)]);
        } else {
            assert!(rows.is_empty(), "{name}");
        }
        controls.push(json!({"name":name,"value":value}));
    }
    fs::write(out.join("validation.json"),serde_json::to_vec_pretty(&json!({"originals":results,"controls":controls,"queries":110,"complete_native_builds":0,"calculation":"not_run"})).unwrap()).unwrap();
    assert_eq!(prior_files, release::inventory(&prior_path));
}
