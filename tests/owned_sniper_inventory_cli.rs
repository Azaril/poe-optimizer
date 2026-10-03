//! Physical list completeness preserves unresolved usage and exact source ownership.
#[path = "support/owned_sniper_inventory.rs"]
mod family;
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use preservation::{header, link, member, origin, preset_usage};

use poe_optimizer_import::owned_release::assemble_owned_release;
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
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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
fn sniper_disposition_packet_binds_physical_values_and_defers_usage() {
    family::check_authored();
}

fn frame(xml: &[u8]) -> preservation::Frame {
    preservation::frame(xml, &[&family::disposition().physical])
}

fn controls(prior_package: &Path, package: &Path, out: &Path) -> Vec<Value> {
    let path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&path).unwrap();
    let f = frame(original.as_bytes());
    let target = f.locations.iter().find(|l| l.selected).unwrap();
    assert_eq!(f.locations.iter().filter(|l| l.selected).count(), 1);
    let effect = family::disposition().physical.skill_id;
    let maps = format!(
        "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"1\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"1\"/></MinionSkillIndexLookupCalcs>"
    );
    let unknown_child = format!(
        "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><Unknown skillIndex=\"1\" statSetIndex=\"1\"/></MinionSkillIndexLookup>"
    );
    let duplicate = format!(
        "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"1\"/><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"1\"/></MinionSkillIndexLookup>"
    );
    let gas_map = format!(
        "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"2\" statSetIndex=\"1\"/></MinionSkillIndexLookup>"
    );
    struct Control<'a> {
        name: &'a str,
        gem: Vec<(&'a str, Option<&'a str>)>,
        group: Vec<(&'a str, Option<&'a str>)>,
        children: &'a str,
        complete: bool,
    }
    let cases = [
        Control {
            name: "unknown-gem-field",
            gem: vec![("unreviewed", Some("true"))],
            group: vec![],
            children: "",
            complete: false,
        },
        Control {
            name: "unknown-minion-name",
            gem: vec![("skillMinion", Some("unreviewed-actor"))],
            group: vec![],
            children: "",
            complete: false,
        },
        Control {
            name: "unknown-nested-child",
            gem: vec![],
            group: vec![],
            children: &unknown_child,
            complete: false,
        },
        Control {
            name: "duplicate-nested-map",
            gem: vec![],
            group: vec![],
            children: &duplicate,
            complete: false,
        },
        Control {
            name: "gas-action",
            gem: vec![("skillMinionSkill", Some("2"))],
            group: vec![],
            children: "",
            complete: false,
        },
        Control {
            name: "gas-map",
            gem: vec![],
            group: vec![],
            children: &gas_map,
            complete: false,
        },
        Control {
            name: "explicit-basic-maps",
            gem: vec![("skillMinionCalcs", Some("RaisedSkeletonSniper"))],
            group: vec![],
            children: &maps,
            complete: true,
        },
        Control {
            name: "absent-singleton-selections",
            gem: vec![
                ("skillMinion", None),
                ("skillMinionCalcs", None),
                ("skillMinionSkill", None),
                ("skillMinionSkillCalcs", None),
            ],
            group: vec![],
            children: "",
            complete: true,
        },
    ];
    let mut reports = vec![];
    for c in cases {
        let mut xml = original.clone();
        xml.replace_range(
            target.range.clone(),
            &format!(
                "{}{}</Gem>",
                header("Gem", &target.attributes, &c.gem),
                c.children
            ),
        );
        if !c.group.is_empty() {
            xml.replace_range(
                target.group_header.clone(),
                &header("Skill", &target.group_attributes, &c.group),
            );
        }
        assert_ne!(xml, original);
        let file = out.join(format!("control-{}.xml", c.name));
        fs::write(&file, &xml).unwrap();
        let dir = out.join(format!("control-{}", c.name));
        let prior_dir = out.join(format!("prior-control-{}", c.name));
        // Existing strict source census can itself decline unrelated usage
        // recipes after a mutation. Compare both releases on this exact XML.
        release::normalize(prior_package, &file, 5, &prior_dir);
        release::normalize(package, &file, 5, &dir);
        let mut draft: Value = read(dir.join("draft.json"));
        let mut sidecar: Value = read(dir.join("sidecar.json"));
        selected::canonical(&mut draft);
        selected::canonical(&mut sidecar);
        let mut old: Value = read(prior_dir.join("draft.json"));
        let mut old_sidecar: Value = read(prior_dir.join("sidecar.json"));
        selected::canonical(&mut old);
        selected::canonical(&mut old_sidecar);
        assert_eq!(draft["draft"]["allocator"], old["draft"]["allocator"]);
        assert_eq!(sidecar["source_sha256"], old_sidecar["source_sha256"]);
        let changed = frame(xml.as_bytes());
        assert_eq!(changed.locations.len(), 5);
        for loc in &changed.locations {
            let id = link(&sidecar, loc.ordinal, "gem");
            let old_id = link(&old_sidecar, loc.ordinal, "gem");
            assert_eq!(id, old_id);
            let gem = member(&draft, "gems", &id);
            let old_gem = member(&old, "gems", &old_id);
            for field in ["definition", "level", "quality"] {
                assert_eq!(gem[field], old_gem[field]);
            }
            assert_eq!(
                gem["parameters"]["members"],
                old_gem["parameters"]["members"]
            );
            let complete = !loc.selected || c.complete;
            assert_eq!(
                gem["parameters"]["completion"]["kind"],
                if complete { "complete" } else { "pending" },
                "{} preset {}",
                c.name,
                loc.set_id
            );
            if !complete {
                assert_eq!(
                    gem["parameters"]["completion"]["code"],
                    "gem-parameters-not-converted"
                );
            }
            let (_, usage) = preset_usage(&draft, &sidecar, loc);
            assert!(
                usage == preset_usage(&old, &old_sidecar, loc).1,
                "{}: same edited source preserves existing usage values and obligations",
                c.name
            );
            if loc.selected && c.name == "explicit-basic-maps" {
                assert_eq!(loc.children.len(), 4);
                let skill = link(&sidecar, loc.ordinal, "skill");
                for child in &loc.children {
                    assert_eq!(
                        origin(&sidecar, *child)["links"],
                        json!([{"kind":"gem","value":id},{"kind":"skill","value":skill}])
                    );
                }
            }
        }
        reports.push(json!({"name":c.name,"selected_physical_complete":c.complete,"usage_pending":true,"same_mutated_source":true,"archived_physical_values_preserved":true,"source_sha256":format!("{:x}",Sha256::digest(xml.as_bytes()))}));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    reports
}

fn reference_target(package: &Path, out: &Path) -> Value {
    let source_path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = fs::read(&source_path).unwrap();
    let f = frame(&xml);
    let selected = f.locations.iter().find(|row| row.selected).unwrap();
    let row = family::disposition();
    let adapter = out.join("adapter.json");
    write(&adapter, &row.reference_action);
    let queries: Value = read(package.join("queries-original-05.json"));
    let expected: Vec<_> = queries
        .as_array()
        .unwrap()
        .iter()
        .filter(|query| matches!(query["id"].as_str(), Some("reference-14" | "reference-16")))
        .collect();
    assert_eq!(expected.len(), 2);
    assert_eq!(expected[0]["target"], expected[1]["target"]);
    for context in ["main", "calcs"] {
        let request = out.join(format!("reference-{context}.json"));
        write(
            &request,
            &json!({"skill_use":{"source_sha256":format!("{:x}",Sha256::digest(&xml)),"occurrence_ordinal":selected.ordinal,"expected_gem":row.physical.gem},"context":context}),
        );
        let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .arg("resolve-owned-action")
            .arg(&source_path)
            .arg("--release")
            .arg(package)
            .arg("--correspondence")
            .arg(&adapter)
            .arg("--request")
            .arg(&request)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["report"]["target"], expected[0]["target"]);
        for field in ["source_execution", "calculation", "whole_build_parity"] {
            assert_eq!(report[field], false);
        }
        write(
            out.join(format!("reference-{context}-report.json")),
            &report,
        );
    }
    // Actual normalization imports these providers into the two unchanged owned
    // query requests. This is not a claim of complete native input/evaluation.
    let draft: Value = read(out.join("original-05/draft.json"));
    let sidecar: Value = read(out.join("original-05/sidecar.json"));
    let skill = link(&sidecar, selected.ordinal, "skill");
    let reference = json!(row.reference_action);
    let basic = &reference["actions"][0];
    let known = |v: Value| json!({"kind":"known","value":v});
    let provider = |path: Vec<Value>| {
        json!({"root":{"kind":"skill_use","value":known(skill.clone())},
        "grant_path":{"members":path.into_iter().map(known).collect::<Vec<_>>(),"completion":{"kind":"complete"}}})
    };
    let actor = json!({"kind":"owned","value":{"provider":provider(vec![reference["entering_grant"].clone()]),
        "slot":known(reference["minion"]["population"].clone())}});
    let expected_owned = json!({"kind":"action","value":{"action":{"actor":actor,
        "provider":provider(vec![reference["entering_grant"].clone(),reference["minion"]["entering_grant"].clone(),basic["entering_grant"].clone()]),
        "output":known(basic["output"].clone())},"part":known(basic["part"].clone()),
        "mode":known(basic["mode"].clone()),"stat_set":known(basic["stat_sets"][0]["stat_set"].clone())}});
    let imported: Vec<_> =
        draft["draft"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|query| query["target"]["kind"] == "action")
            .collect();
    assert_eq!(imported.len(), 2);
    for query in imported {
        assert_eq!(query["target"], expected_owned);
    }
    json!({"contexts":2,"existing_reference_targets_preserved":true,
        "normalized_owned_query_targets_checked":true,"whole_build_evaluation":false})
}

#[test]
#[ignore = "requires the exact Ice Nova inventory release and authenticated occurrence source reports"]
fn publish_sniper_physical_inventory_preserving_usage_and_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_INVENTORY_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_INVENTORY_OUTPUT").expect("new output"),
    );
    assert!(!out.exists());
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("prior-receipt.json"), prior.receipt());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let after = release::inventory(&package);
    assert_eq!(after.len(), 18);
    assert_eq!(after, release::inventory(&out.join("rebuilt")));
    let changed: Vec<_> = before
        .iter()
        .filter(|(name, hash)| after.get(*name) != Some(*hash))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        changed,
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    for field in ["roles", "catalog", "scalar_inputs", "usage_inputs"] {
        let mut bad = json!(next.input());
        bad["normalization"]["gem_inventory"][field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    for field in ["roles", "catalog"] {
        let mut bad = json!(next.input());
        bad["normalization"]["gem_inventory"]["primary_dispositions"][1]["reference_action"]
            [field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale reference {field}"
        );
    }
    let row = family::disposition();
    let expectations = [preservation::InventoryExpectation {
        physical: &row.physical,
        occurrences: [1, 0, 0, 0, 5],
        parameter_count: 2,
    }];
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &expectations,
        selected_before: [119, 116, 108, 121, 18],
        selected_after: [118, 116, 108, 121, 17],
        rebind_definitions: false,
    };
    let mut originals = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &xml,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(preservation::compare_original(
            case,
            &fs::read(xml).unwrap(),
            &comparison,
        ));
    }
    let reference = reference_target(&package, &out);
    let controls = controls(&prior_path, &package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"controls":controls,"reference":reference,"queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,"complete_original_builds":0,"usage_inventory":"pending"}),
    );
}
