//! Exact whole-corpus retirement of explicit empty character-rune receiving uses.
#[path = "support/owned_empty_character_runes.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_draft::{DraftLimits, decode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_normalize::{
        EquipmentMembershipPolicy, NormalizationLimits, PassiveSocketMembershipPolicy,
        equipment_membership_identity,
    },
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
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
fn authored_empty_character_runes_are_source_bound_without_definitions() {
    family::check_authored();
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, key: &str) -> &'a str {
    row.attribute(key).unwrap().decoded().unwrap()
}
fn origin(sidecar: &Value, source: SourceOccurrenceId) -> &Value {
    sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"] == json!(source))
        .unwrap()
}
fn origin_mut(sidecar: &mut Value, source: SourceOccurrenceId) -> &mut Value {
    sidecar["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["source"] == json!(source))
        .unwrap()
}
fn link(sidecar: &Value, source: SourceOccurrenceId, kind: &str) -> Value {
    let rows: Vec<_> = origin(sidecar, source)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == kind)
        .collect();
    assert_eq!(rows.len(), 1, "exact {kind} source link");
    rows[0]["value"].clone()
}
fn index(rows: &Value, id: &Value) -> usize {
    rows.as_array()
        .unwrap()
        .iter()
        .position(|r| &r["id"] == id)
        .unwrap()
}
// Only fresh normalization lineage changes; every local ID remains exact.
fn relocate_lineage(value: &mut Value, current: &Value, prior: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(lineage) = fields.get_mut("lineage") {
                assert_eq!(lineage, current);
                *lineage = prior.clone();
            }
            for value in fields.values_mut() {
                relocate_lineage(value, current, prior);
            }
        }
        Value::Array(rows) => {
            for value in rows {
                relocate_lineage(value, current, prior);
            }
        }
        _ => (),
    }
}
fn check_commitments(package: &StagedOwnedRelease, case: usize, directory: &Path, sidecar: &Value) {
    let bytes = fs::read(directory.join("draft.json")).unwrap();
    let draft = decode_draft(&bytes, DraftLimits::default()).unwrap();
    assert_eq!(
        sidecar["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let name = format!("original-{case:02}");
    let queries = &package
        .input()
        .query_sets
        .iter()
        .find(|q| q.name.as_str() == name)
        .unwrap()
        .queries;
    let digest = digest_owned(
        "owned-normalization-policy-v3",
        &(package.normalization(), queries),
        NormalizationLimits::default().max_policy_bytes,
    )
    .unwrap();
    assert_eq!(sidecar["policy"], json!(digest));
    assert_eq!(sidecar["tree_policy"], json!(package.receipt().tree));
}
fn compare(
    case: usize,
    xml: &[u8],
    old: &Path,
    new: &Path,
    out: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
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
    check_commitments(prior, case, old, &sa);
    check_commitments(next, case, new, &sb);
    assert_eq!(a["schema_version"], b["schema_version"]);
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], sa["schema_version"]);
    let old_lineage = &a["draft"]["allocator"]["lineage"];
    let new_lineage = &b["draft"]["allocator"]["lineage"];
    let mut expected = a["draft"].clone();
    let mut actual = b["draft"].clone();
    let mut expected_side = sa.clone();
    let mut actual_side = sb.clone();
    relocate_lineage(&mut actual, new_lineage, old_lineage);
    relocate_lineage(&mut actual_side, new_lineage, old_lineage);
    let policy = family::policy();
    let mut retired = Vec::new();
    let mut removals = Vec::new();
    for row in evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "RuneSlot")
    {
        assert_eq!(attr(row, "runeName"), policy.explicit_empty_selection);
        assert!(policy.slot_names.iter().any(|s| s == attr(row, "slotName")));
        let source = row.occurrence().id();
        let parent = row.occurrence().parent().unwrap();
        assert_eq!(
            evidence.rows()[parent.ordinal() as usize]
                .occurrence()
                .name(),
            "ItemSet"
        );
        let equipment = link(&sa, source, "equipment");
        let preset = link(&sa, parent, "equipment_preset");
        let use_index = index(&expected["equipment"]["members"], &equipment);
        let old_use = expected["equipment"]["members"][use_index].clone();
        let mut issues = Vec::new();
        for (field, code) in [
            ("item", "item-reference-unresolved"),
            ("scope", "equipment-scope-not-converted"),
        ] {
            assert_eq!(old_use[field]["kind"], "pending");
            assert_eq!(old_use[field]["code"], code);
            assert_eq!(old_use[field]["candidates"], json!([]));
            issues.push(old_use[field]["id"].clone());
        }
        assert_eq!(old_use["destination"]["kind"], "pending");
        assert_eq!(
            old_use["destination"]["value"]["code"],
            "socket-destination-not-converted"
        );
        assert_eq!(old_use["destination"]["value"]["candidates"], json!([]));
        issues.push(old_use["destination"]["value"]["id"].clone());
        let source_origin = origin_mut(&mut expected_side, source);
        assert_eq!(source_origin["disposition"], json!({"kind":"contributes"}));
        let links = source_origin["links"].as_array().unwrap();
        assert_eq!(links.len(), 4);
        assert_eq!(
            links
                .iter()
                .filter(|l| l["kind"] == "equipment" && l["value"] == equipment)
                .count(),
            1
        );
        for issue in &issues {
            assert_eq!(
                links
                    .iter()
                    .filter(|l| l["kind"] == "issue" && &l["value"] == issue)
                    .count(),
                1
            );
        }
        source_origin["links"] = json!([]);
        source_origin["disposition"] =
            json!({"kind":"source_only","value":"explicit-empty-character-rune-selection"});
        expected["equipment"]["members"]
            .as_array_mut()
            .unwrap()
            .remove(use_index);
        let preset_index = index(&expected["equipment_presets"]["members"], &preset);
        let inventory = &mut expected["equipment_presets"]["members"][preset_index]["equipment"];
        assert_eq!(inventory["completion"]["kind"], "pending");
        assert_eq!(
            inventory["completion"]["code"],
            "equipment-membership-not-converted"
        );
        let members = inventory["members"].as_array_mut().unwrap();
        let before = members.len();
        members.retain(|id| id != &equipment);
        assert_eq!(members.len() + 1, before);
        retired.extend(issues);
        removals.push(json!({"source":source,"parent":parent,"equipment":equipment,"preset":preset,"slot":attr(row,"slotName")}));
    }
    assert_eq!(removals.len(), if case == 4 { 5 } else { 0 });
    assert_eq!(retired.len(), if case == 4 { 15 } else { 0 });
    assert_eq!(
        expected["allocator"], actual["allocator"],
        "retain spent IDs and watermark"
    );
    assert_eq!(expected, actual, "only proved empty receiving uses retire");
    // Authenticate the three changed commitments; all other evidence is exact.
    for field in ["policy", "tree_policy", "draft"] {
        actual_side[field] = expected_side[field].clone();
    }
    assert_eq!(
        expected_side, actual_side,
        "exact remaining source provenance"
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
    relocate_lineage(&mut actual_issues, new_lineage, old_lineage);
    assert_eq!(
        expected_issues, actual_issues,
        "only 15 explicit empty-rune issues retire"
    );
    let mut selection = selected::selection(xml, new);
    relocate_lineage(&mut selection, new_lineage, old_lineage);
    assert_eq!(selection, selected::selection(xml, old));
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count, [116, 117, 109, 137, 20][case - 1]);
    assert_eq!(after_count, [116, 117, 109, 122, 20][case - 1]);
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"removed_receiving_uses":removals,"retired_issues":retired,"allocator_unchanged":true,"calculation":"not_run"})
}

#[test]
#[ignore = "requires the checked passive-placement predecessor and complete empty-rune source evidence"]
fn explicit_empty_character_runes_preserve_all_five_requests_and_remaining_obligations() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_CHARACTER_RUNES_PRIOR")
            .expect("explicit checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_CHARACTER_RUNES_OUTPUT")
            .expect("explicit fresh output"),
    );
    assert!(!out.exists());
    let prior_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (input, output) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(publish(input, output), json!(next.receipt()));
    }
    let published = release::inventory(&package);
    assert_eq!(published, release::inventory(&rebuilt));
    assert_eq!(
        prior_inventory.keys().collect::<Vec<_>>(),
        published.keys().collect::<Vec<_>>()
    );
    for (name, digest) in &prior_inventory {
        if !matches!(
            name.as_str(),
            "normalization.json" | "tree-normalization.json" | "release.json"
        ) {
            assert_eq!(
                published.get(name),
                Some(digest),
                "unchanged artifact {name}"
            );
        }
    }
    // Rebind outer consumers to isolate the inner source-identity rejection.
    let mut bad = next.input().clone();
    let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        empty_character_runes,
        ..
    }) = &mut bad.normalization.equipment_membership
    else {
        panic!()
    };
    empty_character_runes.mapping_source = serde_json::from_value(json!("00".repeat(32))).unwrap();
    family::rebind_equipment(&mut bad);
    // This intentionally invalid input cannot pass bind_new. Supply its exact
    // outer commitment directly so assembly must reject the inner source proof.
    bad.tree.as_mut().unwrap().normalization = digest_owned(
        "owned-normalization-policy-v3",
        &bad.normalization,
        TreePolicyLimits::default().max_base_policy_bytes,
    )
    .unwrap();
    assert!(
        assemble_owned_release(bad, Default::default()).is_err(),
        "stale inner source despite current outer commitments"
    );
    let mut bad = next.input().clone();
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 { equipment, .. }) =
        &mut bad.normalization.passive_socket_membership
    else {
        panic!()
    };
    *equipment = equipment_membership_identity(
        prior.normalization().equipment_membership.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    bad.tree.as_mut().unwrap().normalization = digest_owned(
        "owned-normalization-policy-v3",
        &bad.normalization,
        TreePolicyLimits::default().max_base_policy_bytes,
    )
    .unwrap();
    assert!(
        assemble_owned_release(bad, Default::default()).is_err(),
        "stale passive equipment binding"
    );
    let mut bad = next.input().clone();
    bad.tree = prior.input().tree.clone();
    assert!(
        assemble_owned_release(bad, Default::default()).is_err(),
        "missing full tree rebind"
    );
    let mut reports = Vec::new();
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
        reports.push(compare(
            case,
            &fs::read(xml).unwrap(),
            &old,
            &new,
            &out,
            &prior,
            &next,
        ));
    }
    assert_eq!(prior_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,
            "definitions":next.receipt().definitions,"registry":next.receipt().registry,
            "originals":reports,"queries":110,"source_occurrences":5,
            "removed_receiving_uses":5,"retired_selected_issues":15,"stale_bindings":3,
            "prior_unchanged":true,"rebuild_byte_identical":true,
            "definitions_unchanged":true,"complete_original_builds":0
        }),
    );
}
