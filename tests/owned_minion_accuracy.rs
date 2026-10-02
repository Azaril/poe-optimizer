//! Publish source-bound block configuration and actual native minion hit chance.
#[path = "support/owned_minion_accuracy.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::{FiniteQuantity, UnitDefId},
    owned_draft::{DraftLimits, decode_draft},
};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationInputsPolicy, NormalizationLimits},
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
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
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
}
fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
#[test]
fn raw_block_inputs_and_native_hit_chance_keep_coverage_explicit() {
    family::check_authored();
}
fn check_bindings(p: &StagedOwnedRelease, case: usize, directory: &Path, side: &Value) {
    let receipt = json!(p.receipt());
    for (s, r) in [
        ("mapping", "mapping"),
        ("mapping_source", "mapping_source"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        let expected = if r == "mapping_source" {
            json!(p.mapping().source_identity())
        } else {
            receipt[r].clone()
        };
        assert_eq!(side[s], expected, "authenticated {s}");
    }
    for item in side["item_texts"].as_array().unwrap() {
        assert_eq!(item["attribution"]["policy"], receipt["item_source"]);
        assert_eq!(item["attribution"]["item_lines"], receipt["items"]);
    }
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(
        side["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let queries = &p
        .input()
        .query_sets
        .iter()
        .find(|q| q.name.as_str() == format!("original-{case:02}"))
        .unwrap()
        .queries;
    assert_eq!(
        side["policy"],
        json!(
            digest_owned(
                "owned-normalization-policy-v3",
                &(p.normalization(), queries),
                NormalizationLimits::default().max_policy_bytes
            )
            .unwrap()
        )
    );
}
fn check_measured_configuration_controls(package: &Path, output: &Path) {
    let source: Value =
        read(root().join("runs/owned-minion-accuracy-source-01/source-jit-off.json"));
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    assert!(!original.contains("name=\"enemyBlockChance\""));
    assert_eq!(original.matches("</ConfigSet>").count(), 1);
    let policy = json!(family::policy());
    let block = &policy["placeholder_fallback_inputs"][0];
    let unit: UnitDefId =
        serde_json::from_value(block["recipe"]["codec"]["codec"]["value"]["unit"].clone()).unwrap();
    for (name, input, placeholder, raw) in [
        ("block-placeholder-37", None, Some("37"), 37.),
        ("block-input-zero-placeholder-37", Some("0"), Some("37"), 0.),
        ("block-input-25-placeholder-37", Some("25"), Some("37"), 25.),
        ("block-input-negative", Some("-10"), None, -10.),
        ("block-input-fractional", Some("12.5"), None, 12.5),
        ("block-input-100", Some("100"), None, 100.),
        ("block-input-125", Some("125"), None, 125.),
    ] {
        let extra = [("Input", input), ("Placeholder", placeholder)]
            .into_iter()
            .filter_map(|(kind, value)| {
                value.map(|v| format!("<{kind} name=\"enemyBlockChance\" number=\"{v}\"/>"))
            })
            .collect::<String>();
        let xml = original.replacen("</ConfigSet>", &format!("{extra}</ConfigSet>"), 1);
        let reference = source["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap();
        assert_eq!(reference["available"], true);
        assert_eq!(
            reference["xml_sha256"],
            format!("{:x}", Sha256::digest(xml.as_bytes()))
        );
        let path = output.join(format!("{name}.xml"));
        fs::write(&path, &xml).unwrap();
        let normalized = output.join(name);
        release::normalize(package, &path, 5, &normalized);
        let draft: Value = read(normalized.join("draft.json"));
        let presets = draft["draft"]["scenario_presets"]["members"]
            .as_array()
            .unwrap();
        assert_eq!(presets.len(), 1);
        let assumptions = &presets[0]["scenario"]["assumptions"];
        assert_eq!(assumptions["completion"]["kind"], "pending");
        for (id, value) in [
            (&block["presence_input"], ParameterValue::Boolean(true)),
            (
                &block["value_input"],
                ParameterValue::Quantity(FiniteQuantity::new(raw, unit.clone()).unwrap()),
            ),
        ] {
            let facts: Vec<_> = assumptions["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["input"]["value"] == *id)
                .collect();
            assert_eq!(facts.len(), 1);
            assert_eq!(
                *facts[0],
                json!({"input":{"kind":"known","value":id},"target":{"kind":"enemy"},"value":{"kind":"known","value":value}})
            );
        }
    }
}
#[test]
#[ignore = "requires checked intrinsic-source predecessor and fresh hit-chance source witness"]
fn block_configuration_and_minion_hit_chance_preserve_original_requests() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_ACCURACY_PRIOR").expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_ACCURACY_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(publish(&package, &rebuilt), json!(next.receipt()));
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    assert_eq!(prior.input().query_sets, next.input().query_sets);
    let mut reports = Vec::new();
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let mut a: Value = read(old.join("draft.json"));
        let mut b: Value = read(new.join("draft.json"));
        let mut sa: Value = read(old.join("sidecar.json"));
        let mut sb: Value = read(new.join("sidecar.json"));
        check_bindings(&prior, case, &old, &sa);
        check_bindings(&next, case, &new, &sb);
        for v in [&mut a, &mut b, &mut sa, &mut sb] {
            selected::canonical(v);
        }
        let ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
            placeholder_fallback_inputs,
            ..
        } = family::policy()
        else {
            panic!("explicit source fallback policy");
        };
        let block = &placeholder_fallback_inputs[0];
        let expected = json!({
            "input":{"kind":"known","value":block.presence_input},
            "target":{"kind":"enemy"},
            "value":{"kind":"known","value":{"kind":"boolean","value":false}}
        });
        let old_sets = a["draft"]["scenario_presets"]["members"]
            .as_array_mut()
            .unwrap();
        let new_sets = b["draft"]["scenario_presets"]["members"]
            .as_array()
            .unwrap();
        assert_eq!(old_sets.len(), new_sets.len());
        for (old, new) in old_sets.iter_mut().zip(new_sets) {
            let old_inputs = &mut old["scenario"]["assumptions"];
            let new_inputs = &new["scenario"]["assumptions"];
            assert_eq!(old_inputs["completion"], new_inputs["completion"]);
            assert_eq!(new_inputs["completion"]["kind"], "pending");
            let old_members = old_inputs["members"].as_array_mut().unwrap();
            assert_eq!(
                new_inputs["members"].as_array().unwrap().len(),
                old_members.len() + 1
            );
            old_members.push(expected.clone());
            assert_eq!(
                old_inputs, new_inputs,
                "exact raw absence; no manufactured block value"
            );
        }
        assert!(a == b, "every source fact/ID/issue and allocator preserved");
        for f in [
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
        ] {
            sb[f] = sa[f].clone();
        }
        let old_items = sa["item_texts"].as_array().unwrap();
        let new_items = sb["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_items.len(), new_items.len());
        for (old, new) in old_items.iter().zip(new_items) {
            for field in ["policy", "item_lines"] {
                new["attribution"][field] = old["attribution"][field].clone();
            }
        }
        assert!(
            sa == sb,
            "all provenance except checked dependency bindings preserved"
        );
        let xml = fs::read(xml).unwrap();
        let before = selected::finalize(
            &xml,
            &old,
            &out.join(format!("original-{case:02}-prior-selection.json")),
        );
        let after = selected::finalize(
            &xml,
            &new,
            &out.join(format!("original-{case:02}-selection.json")),
        );
        let mut x = before["finalization"]["issues"].clone();
        let mut y = after["finalization"]["issues"].clone();
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(
            x, y,
            "numerical component does not close unresolved input requirements"
        );
        let mut x = selected::selection(&xml, &old);
        let mut y = selected::selection(&xml, &new);
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(x, y);
        write(
            out.join(format!("original-{case:02}-selected-report.json")),
            &after,
        );
        reports.push(json!({"original":case,"selected_issues":after["finalization"]["issues"].as_array().unwrap().len(),"calculation":"not_run"}));
    }
    check_measured_configuration_controls(&package, &out);
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"queries":110,"measured_configuration_controls":7,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
