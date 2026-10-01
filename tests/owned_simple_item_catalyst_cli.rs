//! Real input transport and preservation; whole-build numerical coverage stays open.
#[path = "support/owned_simple_item_catalyst.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{build_identity::BuildLineage, owned_schema::SchemaClosure};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemEmission, OwnedItemLinePolicy},
    owned_item_source::ItemSourceLayoutPolicy,
    owned_normalize::ItemModifierMembershipPolicy,
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::SourceProjectEvidence,
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
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
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

#[test]
fn two_template_authoring_keeps_required_transport_and_partial_owners() {
    let bindings = family::bindings();
    let e = family::extension();
    assert_eq!(bindings.len(), 2);
    assert_eq!(
        bindings
            .iter()
            .map(|v| v.template.key().as_str())
            .collect::<Vec<_>>(),
        ["def.0000000000002007", "def.000000000000238c"]
    );
    assert_eq!(e.schema.len(), 6);
    assert_eq!(e.owners.len(), 2);
    assert!(e.owners.iter().all(
        |v| matches!(v.programs.closure, SchemaClosure::Partial { .. })
            && v.programs.members.len() == 1
            && v.programs.members[0].id.as_str() == "catalyst-inputs"
    ));
    assert_eq!(family::defaults().len(), 2);
    assert_eq!(family::dependency_definitions().len(), 17);
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    assert_eq!(a["source_files"].as_array().unwrap().len(), 7);
    for pin in a["source_files"].as_array().unwrap() {
        let exact = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["path"] == pin["path"])
            .unwrap();
        assert_eq!(pin["sha256"], exact["sha256"]);
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    let mut changed = vec![];
    for (x, y) in a["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["draft"]["items"]["members"].as_array_mut().unwrap())
    {
        let Some(default) = family::defaults()
            .into_iter()
            .find(|v| serde_json::to_value(&v.template).unwrap() == x["template"]["value"])
        else {
            continue;
        };
        assert_eq!(case, 5);
        assert!(x["parameters"]["members"].as_array().unwrap().is_empty());
        let expected: Vec<_> = default.parameters.iter().map(|v| json!({"slot":{"kind":"known","value":v.assignment.slot},"value":{"kind":"known","value":v.assignment.value}})).collect();
        assert_eq!(y["parameters"]["members"], json!(expected));
        assert_eq!(y["parameters"]["completion"]["kind"], "pending");
        assert_eq!(y["modifiers"]["completion"]["kind"], "complete");
        changed.push(json!({"item":y["id"],"template":default.template,"assignments":expected}));
        y["parameters"]["members"] = json!([]);
    }
    assert_eq!(changed.len(), if case == 5 { 2 } else { 0 });
    // The new assignments allocate no instance IDs. Only independently generated
    // lineage differs; local IDs, allocator watermarks and every reference stay.
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    assert!(
        a == b,
        "original {case}: only four known assignments may differ"
    );
    assert_eq!(
        sa["origins"].as_array().unwrap().len(),
        sb["origins"].as_array().unwrap().len()
    );
    let source_policy = sb["item_source_policy"].clone();
    let item_policy = sb["item_policy"].clone();
    let old_rows = sa["item_texts"].as_array().unwrap();
    let new_rows = sb["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_rows.len(), new_rows.len());
    let mut evidence_changes = 0;
    for (x, y) in old_rows.iter().zip(new_rows) {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(y["attribution"]["policy"], source_policy);
        assert_eq!(y["attribution"]["item_lines"], item_policy);
        y["attribution"]["policy"] = x["attribution"]["policy"].clone();
        y["attribution"]["item_lines"] = x["attribution"]["item_lines"].clone();
        if let Some(default) = family::defaults().into_iter().find(|v| {
            serde_json::to_value(&v.template).unwrap()
                == y["attribution"]["default_scope"]["template"]
        }) {
            assert_eq!(case, 5);
            assert_eq!(y["attribution"]["layout"]["status"], "proven");
            assert_eq!(
                y["defaults"]["parameters"],
                serde_json::to_value(
                    default
                        .parameters
                        .iter()
                        .map(|v| &v.assignment)
                        .collect::<Vec<_>>()
                )
                .unwrap()
            );
            assert!(x["defaults"]["parameters"].as_array().unwrap().is_empty());
            y["defaults"]["parameters"] = json!([]);
            evidence_changes += 1;
        }
    }
    assert_eq!(evidence_changes, changed.len());
    // Each top-level commitment is checked by normalizer/decoder; preserve all
    // other source evidence, issue links, ordering and input source fields.
    for field in [
        "draft",
        "definitions",
        "registry",
        "mapping",
        "policy",
        "item_policy",
        "item_source_policy",
        "skill_roles",
        "reward_policy",
        "tree_policy",
    ] {
        sb[field] = sa[field].clone();
    }
    selected::canonical(&mut sa);
    selected::canonical(&mut sb);
    assert!(
        sa == sb,
        "original {case}: unrelated sidecar evidence changed"
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
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    assert_eq!(x, y);
    let mut x = selected::selection(xml, old);
    let mut y = selected::selection(xml, new);
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    assert_eq!(x, y);
    json!({"original":case,"changes":changed,"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"unchanged_local_ids_and_issues":true})
}

fn probes(next: &StagedOwnedRelease) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let cases = [
        ("absent", "", Some("09eb"), Some(20.0)),
        ("kind-only", "Catalyst: Flesh\n", Some("09ec"), Some(20.0)),
        (
            "zero",
            "Catalyst: Flesh\nCatalystQuality: 0\n",
            Some("09ec"),
            Some(0.0),
        ),
        (
            "twenty",
            "Catalyst: Flesh\nCatalystQuality: 20\n",
            Some("09ec"),
            Some(20.0),
        ),
        (
            "amount-only",
            "CatalystQuality: 37\n",
            Some("09eb"),
            Some(37.0),
        ),
        ("unknown-kind", "Catalyst: unknown\n", None, None),
        ("malformed-amount", "CatalystQuality: NaN\n", None, None),
        (
            "duplicate-kind",
            "Catalyst: Flesh\nCatalyst: Neural\n",
            None,
            None,
        ),
        (
            "duplicate-amount",
            "CatalystQuality: 20\nCatalystQuality: 0\n",
            None,
            None,
        ),
        ("unknown-prefix", "Unknown unreviewed field\n", None, None),
    ];
    for (item, ordinal, key) in [(19, 572, "238c"), (20, 574, "2007")] {
        let binding = family::bindings()
            .into_iter()
            .find(|v| v.template.key().as_str().ends_with(key))
            .unwrap();
        for (label, header, kind, amount) in cases {
            let start = original.find(&format!("<Item id=\"{item}\">")).unwrap();
            let end = start + original[start..].find("</Item>").unwrap() + 7;
            let raw = &original[start..end];
            let changed = raw.replace("Quality: 20", &format!("{header}Quality: 20"));
            let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
            let imported = ImportedBuildInstance::from_decoded(
                decode_build(xml.as_bytes()).unwrap(),
                BuildLineage::from_bytes([97; 16]),
                Default::default(),
            )
            .unwrap();
            let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
            let row = evidence
                .rows()
                .iter()
                .find(|v| serde_json::to_value(v.occurrence().id()).unwrap()["ordinal"] == ordinal)
                .unwrap();
            let attributed = next
                .item_source()
                .attribute(&evidence, row.occurrence().id(), next.items())
                .unwrap();
            let converted = attributed.convert(next.items()).unwrap();
            let assignments: Vec<_> = converted
                .parameters
                .iter()
                .map(|v| &v.assignment)
                .chain(&converted.defaults.parameters)
                .collect();
            let actual_kind = assignments.iter().find(|v| v.slot == binding.selection);
            let actual_amount = assignments.iter().find(|v| v.slot == binding.amount);
            if let Some(kind) = kind {
                assert_eq!(
                    serde_json::to_value(actual_kind.unwrap().value.clone()).unwrap()["value"]["key"],
                    format!("def.000000000000{kind}"),
                    "{item} {label}"
                );
                assert_eq!(serde_json::to_value(actual_amount.unwrap().value.clone()).unwrap()["value"]["value"].as_f64(), amount, "{item} {label}");
            } else {
                assert!(
                    converted.defaults.parameters.is_empty(),
                    "{item} {label}: no absence authority"
                );
                if label == "unknown-kind" || label == "duplicate-kind" {
                    assert!(actual_kind.is_none());
                }
                if label == "malformed-amount" || label == "duplicate-amount" {
                    assert!(actual_amount.is_none());
                }
            }
        }
    }
    20
}

fn stale_bindings_reject(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    for which in 0..3 {
        let mut bad = next.input().clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions,
            item_lines,
            item_source,
            ..
        }) = &mut bad.normalization.item_modifier_membership
        else {
            panic!()
        };
        match which {
            0 => *definitions = prior.receipt().definitions.clone(),
            1 => *item_lines = prior.receipt().items,
            _ => *item_source = prior.receipt().item_source,
        }
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale singleton commitment {which}"
        );
    }
    // A changed source interpretation cannot become trusted merely by replacing
    // fixed-width hashes: the checked member-role contract must still hold.
    let mut bad = next.input().items.clone();
    let fixed = bad
        .rules
        .iter_mut()
        .find(|v| v.id.as_str() == "fixed-life")
        .unwrap();
    fixed.emissions = vec![ItemEmission::Metadata {
        role: fixed.id.clone(),
    }];
    let checked =
        OwnedItemLinePolicy::new(bad, next.assembled().schema(), Default::default()).unwrap();
    let mut source = next.input().item_source.clone();
    source.item_lines = *checked.identity();
    assert!(
        ItemSourceLayoutPolicy::new(
            source,
            &checked,
            next.assembled().schema(),
            Default::default()
        )
        .is_err()
    );
}

#[test]
#[ignore = "requires explicit checked singleton-member predecessor"]
fn real_catalyst_inputs_preserve_all_saved_requests_and_partial_coverage() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SIMPLE_CATALYST_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_SIMPLE_CATALYST_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let endpoint = out.join("endpoint.json");
    write(&endpoint, next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&endpoint, &package),
        serde_json::to_value(next.receipt()).unwrap()
    );
    assert_eq!(
        publish(&package, &rebuilt),
        serde_json::to_value(next.receipt()).unwrap()
    );
    let inventory = release::inventory(&package);
    assert_eq!(inventory, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if name.starts_with("queries-") {
            assert_eq!(inventory.get(name), Some(hash));
        }
    }
    stale_bindings_reject(&prior, &next);
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let a = out.join(format!("prior-original-{case:02}"));
        let b = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &a);
        release::normalize(&package, &xml, case, &b);
        reports.push(compare(case, &fs::read(xml).unwrap(), &a, &b, &out));
    }
    let probes = probes(&next);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"originals":reports,"header_probes":probes,"stale_and_changed_semantics_rejections":4,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
