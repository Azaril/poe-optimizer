//! Whole-corpus preservation for source-proved passive-jewel receiving uses.
#[path = "support/owned_passive_jewel_placement.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    owned_build::EquipmentDestination,
    owned_draft::{DraftEquipmentDestination, DraftLimits, decode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_release::assemble_owned_release,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
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
fn authored_sockets_add_only_explicit_placement_memberships() {
    family::check_authored_memberships();
    let authoring = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], authoring["source_revision"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, key: &str) -> &'a str {
    row.attribute(key).unwrap().decoded().unwrap()
}
fn source_row_mut(sidecar: &mut Value, source: SourceOccurrenceId) -> &mut Value {
    sidecar["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["source"] == json!(source))
        .unwrap()
}
fn link(sidecar: &Value, source: SourceOccurrenceId, kind: &str) -> Value {
    let row = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source"] == json!(source))
        .unwrap();
    let values: Vec<_> = row["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["kind"] == kind)
        .collect();
    assert_eq!(values.len(), 1, "exact {kind} source join");
    values[0]["value"].clone()
}
fn index(rows: &Value, id: &Value) -> usize {
    rows.as_array()
        .unwrap()
        .iter()
        .position(|row| &row["id"] == id)
        .unwrap()
}
fn normalize_source_lineage(value: &mut Value, current: &Value, prior: &Value) {
    match value {
        Value::Object(fields) => {
            if !fields.contains_key("local")
                && let Some(lineage) = fields.get_mut("lineage")
            {
                assert_eq!(lineage, current);
                *lineage = prior.clone();
            }
            for value in fields.values_mut() {
                normalize_source_lineage(value, current, prior);
            }
        }
        Value::Array(rows) => {
            for value in rows {
                normalize_source_lineage(value, current, prior);
            }
        }
        _ => (),
    }
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let old_bytes = fs::read(old.join("draft.json")).unwrap();
    let old_draft = decode_draft(&old_bytes, DraftLimits::default()).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        old_draft.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let a: Value = serde_json::from_slice(&old_bytes).unwrap();
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], sa["schema_version"]);
    assert_eq!(a["schema_version"], b["schema_version"]);
    let mut expected = a["draft"].clone();
    let mut actual = b["draft"].clone();
    let mut expected_side = sa.clone();
    let mut actual_side = sb.clone();
    normalize_source_lineage(
        &mut actual_side,
        &b["draft"]["allocator"]["lineage"],
        &a["draft"]["allocator"]["lineage"],
    );
    let bindings = family::bindings();
    let mut retired = vec![];
    let mut changed_specs = BTreeSet::new();
    let mut joins = vec![];
    for row in evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Socket")
    {
        let container = &evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
        assert_eq!(container.occurrence().name(), "Sockets");
        let spec = &evidence.rows()[container.occurrence().parent().unwrap().ordinal() as usize];
        assert_eq!(spec.occurrence().name(), "Spec");
        let binding = bindings
            .iter()
            .find(|binding| binding.node_token == attr(row, "nodeId"))
            .expect("every real assigned node is reviewed");
        let item_sources: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Item"
                    && r.attribute("id")
                        .is_some_and(|a| a.decoded().is_ok_and(|v| v == attr(row, "itemId")))
            })
            .collect();
        assert_eq!(item_sources.len(), 1);
        let item = link(&sa, item_sources[0].occurrence().id(), "item");
        let physical =
            &a["draft"]["items"]["members"][index(&a["draft"]["items"]["members"], &item)];
        assert!(
            binding
                .templates
                .iter()
                .any(|template| json!(template) == physical["template"]["value"])
        );
        let preset = link(&sa, spec.occurrence().id(), "allocation_preset");
        let preset_index = index(&a["draft"]["allocation_presets"]["members"], &preset);
        let members =
            a["draft"]["allocation_presets"]["members"][preset_index]["allocations"]["members"]
                .as_array()
                .unwrap();
        let allocations: Vec<_> = a["draft"]["allocations"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|allocation| {
                members.contains(&allocation["id"])
                    && allocation["node"]["value"] == json!(binding.node)
            })
            .collect();
        assert_eq!(allocations.len(), 1, "exact same-Spec allocation");
        let allocation = allocations[0];
        assert_eq!(
            allocation["scope"],
            json!({"kind":"known","value":{"kind":"shared"}})
        );
        assert_eq!(allocation["access"], json!({"kind":"ordinary"}));
        let equipment = link(&sa, row.occurrence().id(), "equipment");
        assert_eq!(link(&sa, row.occurrence().id(), "item_reference"), item);
        let use_index = index(&a["draft"]["equipment"]["members"], &equipment);
        let old_use = &a["draft"]["equipment"]["members"][use_index];
        assert_eq!(old_use["item"], json!({"kind":"known","value":item}));
        assert!(
            a["draft"]["allocation_presets"]["members"][preset_index]["equipment"]["members"]
                .as_array()
                .unwrap()
                .contains(&equipment)
        );
        let DraftEquipmentDestination::Pending(pending) =
            serde_json::from_value(old_use["destination"].clone()).unwrap()
        else {
            panic!("previous destination obligation")
        };
        assert_eq!(pending.code.as_str(), "socket-destination-not-converted");
        retired.push(json!(pending.id));
        assert_eq!(old_use["scope"]["kind"], "pending");
        assert_eq!(old_use["scope"]["code"], "equipment-scope-not-converted");
        retired.push(old_use["scope"]["id"].clone());
        let destination = DraftEquipmentDestination::from(EquipmentDestination::PassiveSocket {
            allocation: serde_json::from_value(allocation["id"].clone()).unwrap(),
            slot: binding.slot.clone(),
        });
        expected["equipment"]["members"][use_index]["destination"] = json!(destination);
        expected["equipment"]["members"][use_index]["scope"] =
            json!({"kind":"known","value":{"kind":"shared"}});
        source_row_mut(&mut expected_side, row.occurrence().id())["links"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"allocation","value":allocation["id"]}));
        changed_specs.insert(preset_index);
        joins.push(json!({"socket_source":row.occurrence().id(),"spec_source":spec.occurrence().id(),"equipment":equipment,"item":item,"allocation":allocation["id"],"node":binding.node,"slot":binding.slot}));
    }
    assert_eq!(joins.len(), [3, 3, 3, 6, 6][case - 1]);
    assert_eq!(changed_specs.len(), [1, 1, 1, 1, 2][case - 1]);
    for preset_index in changed_specs {
        let equipment = &mut expected["allocation_presets"]["members"][preset_index]["equipment"];
        assert_eq!(equipment["completion"]["kind"], "pending");
        assert_eq!(
            equipment["completion"]["code"],
            "allocation-equipment-membership-not-converted"
        );
        retired.push(equipment["completion"]["id"].clone());
        equipment["completion"] = json!({"kind":"complete"});
    }
    // The post-allocation proof retires spent obligations, without refunding IDs.
    assert_eq!(
        a["draft"]["allocator"]["last_issued"],
        b["draft"]["allocator"]["last_issued"]
    );
    actual["allocator"] = expected["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(&expected, &mut actual, &mut ids, "all canonical facts");
    for (new, old) in &ids {
        let new: Value = serde_json::from_str(new).unwrap();
        assert_eq!(
            new["local"], old["local"],
            "placement preserves spent occurrence IDs"
        );
    }
    let mut removed = 0;
    for origin in expected_side["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue" && retired.contains(&link["value"]);
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
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
        actual_side[field] = expected_side[field].clone();
    }
    assert_eq!(
        expected_side["item_texts"].as_array().unwrap().len(),
        actual_side["item_texts"].as_array().unwrap().len()
    );
    for (old, new) in expected_side["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(actual_side["item_texts"].as_array_mut().unwrap())
    {
        for field in ["policy", "item_lines"] {
            new["attribution"][field] = old["attribution"][field].clone();
        }
    }
    identity::correspond(
        &expected_side,
        &mut actual_side,
        &mut ids,
        "all source evidence",
    );
    let before = selected::finalize(
        xml,
        old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        new,
        &out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        out.join(format!("original-{case:02}-prior-selected-report.json")),
        &before,
    );
    write(
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut expected_issues = before["finalization"]["issues"].clone();
    expected_issues
        .as_array_mut()
        .unwrap()
        .retain(|i| !retired.contains(&i["id"]));
    let mut actual_issues = after["finalization"]["issues"].clone();
    identity::relocate(&mut actual_issues, &ids);
    assert_eq!(
        actual_issues, expected_issues,
        "only proved placement obligations retire"
    );
    let mut selection = selected::selection(xml, new);
    identity::relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count, [123, 124, 116, 150, 20][case - 1]);
    assert_eq!(after_count, [116, 117, 109, 137, 20][case - 1]);
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"retired_issues":retired,"joins":joins,"allocator_unchanged":true,"calculation":"not_run"})
}

#[test]
#[ignore = "requires the checked Ruby predecessor and complete placement source evidence"]
fn real_passive_jewel_placement_preserves_all_five_requests_and_archived_occurrences() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PASSIVE_JEWEL_PLACEMENT_PRIOR")
            .expect("explicit checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PASSIVE_JEWEL_PLACEMENT_OUTPUT")
            .expect("explicit fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (input, output) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(publish(input, output), json!(next.receipt()));
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    let policy =
        &serde_json::to_value(next.input()).unwrap()["normalization"]["passive_socket_membership"];
    for field in [
        "definitions",
        "mapping",
        "mapping_source",
        "item_lines",
        "item_source",
        "equipment",
        "tree_content",
    ] {
        let mut changed = serde_json::to_value(next.input()).unwrap();
        if field == "definitions" {
            changed["normalization"]["passive_socket_membership"][field] =
                json!(prior.receipt().definitions);
        } else {
            changed["normalization"]["passive_socket_membership"][field] = json!("00".repeat(32));
        }
        assert_ne!(
            changed["normalization"]["passive_socket_membership"][field],
            policy[field]
        );
        assert!(
            assemble_owned_release(serde_json::from_value(changed).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior_path.join(&query)).unwrap(),
            fs::read(package.join(query)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"originals":reports,"queries":110,"source_assignments":21,"selected_assignments":15,"stale_bindings":7,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
