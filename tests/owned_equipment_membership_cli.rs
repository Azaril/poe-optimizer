//! Close only source-proven ordinary equipment inventory; calculations remain pending.
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected_request;
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemMetadataValue};
use poe_optimizer_import::{
    owned_normalize::EquipmentMembershipPolicy,
    owned_release::{OwnedReleaseProvenance, assemble_owned_release},
};
use selected_request::{canonical, finalize, selection};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ordinary-itemset-membership")
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
}
fn key(v: &str) -> poe_optimizer_core::owned_definitions::OwnedDefinitionKey {
    poe_optimizer_core::owned_definitions::OwnedDefinitionKey::new(v).unwrap()
}
fn run(name: &str, args: &[&Path]) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg(name)
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn truthy(value: Option<&ItemMetadataValue>) -> bool {
    match value {
        None | Some(ItemMetadataValue::Boolean(false)) => false,
        Some(
            ItemMetadataValue::Boolean(true)
            | ItemMetadataValue::Table(_)
            | ItemMetadataValue::Array(_),
        ) => true,
        other => panic!("unreviewed source field type {other:?}"),
    }
}
#[test]
fn reviewed_template_predicates_join_exact_owned_bases_and_typed_source_metadata() {
    let p: EquipmentMembershipPolicy = read(data().join("policy.json"));
    let wire = serde_json::to_value(p).unwrap();
    let a: Value = read(data().join("authoring.json"));
    assert_eq!(wire["kind"], "pob_ordinary_item_sets_v1");
    assert_eq!(wire["definitions"], a["definitions"]);
    let bytes = fs::read(root().join("data/owned/poe2/3887ae68/item-bases/policy.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["base_policy_sha256"]
    );
    let bases: Value = serde_json::from_slice(&bytes).unwrap();
    let snapshot = bundled_snapshot().unwrap();
    let loading = snapshot.item_loading();
    assert_eq!(
        loading.data().source.upstream_revision,
        a["source_revision"]
    );
    let rows = wire["templates"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    let mut ids = BTreeSet::new();
    for row in rows {
        let name = row["base_name"].as_str().unwrap();
        assert!(ids.insert(row["template"]["key"].as_str().unwrap()));
        let joined: Vec<_> = bases["templates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["source_base"] == name)
            .collect();
        assert_eq!(joined.len(), 1);
        assert_eq!(row["template"], joined[0]["template"]);
        let base = loading.base(name).unwrap();
        for field in ["weapon", "armour"] {
            assert_eq!(row[field], truthy(base.field(field)));
        }
        let tags = base.field("tags").unwrap().as_table().unwrap();
        for field in ["wand", "staff", "sceptre"] {
            assert_eq!(row[field], truthy(tags.fields.get(field)));
        }
        assert!(
            loading
                .data()
                .source
                .files
                .contains_key(&base.source_module)
        );
    }
    assert_eq!(
        wire["loader_jewel_fallback_titles"],
        json!(["Sekhema's Resolve", "Tabula Rasa"])
    );
    let mut names: BTreeSet<_> = loading.bases().iter().map(|v| v.name.as_str()).collect();
    if names.contains("Two-Toned Boots (Armour/Energy Shield)") {
        // ParseRaw's explicitly recognized legacy spelling, from the same
        // authenticated module. This inventory grants no converted base facts.
        names.insert("Two-Toned Boots");
    }
    assert_eq!(wire["source_base_names"], json!(names));
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest_bytes)),
        a["source_manifest_sha256"]
    );
    for pin in a["source_files"].as_array().unwrap() {
        let path = pin["path"].as_str().unwrap();
        let entry = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["path"] == path)
            .unwrap();
        assert_eq!(pin["sha256"], entry["sha256"]);
        if let Some(hash) = loading.data().source.files.get(path) {
            assert_eq!(pin["sha256"], *hash);
        }
    }
    assert_eq!(a["source_files"][1]["path"], "src/Classes/ItemsTab.lua");
    assert_eq!(
        a["source_files"][1]["sha256"],
        "d457907cb4f168df03d0c2bf2bc3975786524b6f9c01668ca9452144b966660d"
    );
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
}
fn compare_original(xml: &[u8], old: &Path, new: &Path, out: &Path, case: usize) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    canonical(&mut a);
    canonical(&mut b);
    let mut retired = BTreeSet::new();
    let ap = a["draft"]["equipment_presets"]["members"]
        .as_array()
        .unwrap();
    let bp = b["draft"]["equipment_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(ap.len(), bp.len());
    for (prior, next) in ap.iter().zip(bp) {
        if prior["equipment"]["completion"] != next["equipment"]["completion"] {
            assert_eq!(prior["equipment"]["completion"]["kind"], "pending");
            assert_eq!(
                prior["equipment"]["completion"]["code"],
                "equipment-membership-not-converted"
            );
            assert_eq!(next["equipment"]["completion"], json!({"kind":"complete"}));
            retired.insert(
                prior["equipment"]["completion"]["id"]["local"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
            next["equipment"]["completion"] = prior["equipment"]["completion"].clone();
        }
    }
    assert_eq!(
        a, b,
        "only equipment membership changes; preserves allocator watermark and all instances"
    );
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    canonical(&mut sa);
    canonical(&mut sb);
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|v| {
            v["kind"] != "issue"
                || !v["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id))
        });
    }
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(
        sa, sb,
        "only retired issue links and exact draft/policy/tree commitments"
    );
    let before = finalize(
        xml,
        old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = finalize(
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
    canonical(&mut x);
    canonical(&mut y);
    let prior_count = x.as_array().unwrap().len();
    x.as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "every other selected issue preserved");
    let removed = prior_count - y.as_array().unwrap().len();
    assert_eq!(removed, usize::from(case == 5));
    if case == 5 {
        let s = selection(xml, new);
        let d: Value = read(new.join("draft.json"));
        let p = d["draft"]["equipment_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == s["build"]["equipment"])
            .unwrap();
        assert_eq!(p["equipment"]["completion"]["kind"], "complete");
        let uses = p["equipment"]["members"].as_array().unwrap();
        assert_eq!(uses.len(), 9);
        let items: BTreeSet<_> = uses
            .iter()
            .map(|id| {
                d["draft"]["equipment"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["id"] == *id)
                    .unwrap()["item"]
                    .to_string()
            })
            .collect();
        assert_eq!(items.len(), 8);
    }
    json!({"original":case,"selected_before":prior_count,"selected_after":y.as_array().unwrap().len(),"membership_issues_removed":removed,"completed_presets":retired.len(),"selected_issue_summary":after["selected_issue_summary"],"status":"pending"})
}
#[test]
#[ignore = "requires the explicit checked support-origin-order release"]
fn real_publication_closes_only_proven_equipment_membership() {
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_EQUIPMENT_MEMBERSHIP_PRIOR")
            .expect("explicit predecessor"),
    );
    let hashes = release::inventory(&path);
    let prior = release::load(&path);
    let a: Value = read(data().join("authoring.json"));
    let p: EquipmentMembershipPolicy = read(data().join("policy.json"));
    assert_eq!(
        serde_json::to_value(prior.receipt().input).unwrap(),
        a["before"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().normalization).unwrap(),
        a["normalization"]
    );
    assert!(prior.normalization().equipment_membership.is_none());
    let mut policy = prior.normalization().clone();
    policy.equipment_membership = Some(p.clone());
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_EQUIPMENT_MEMBERSHIP_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("normalization.json"), &policy);
    let compact = out.join("compact");
    let report = run(
        "publish-owned-normalization",
        &[
            &path,
            Path::new("--normalization"),
            &out.join("normalization.json"),
            Path::new("--output"),
            &compact,
        ],
    );
    write(out.join("publication.json"), &report);
    let mut input = prior.input().clone();
    input.normalization = read(compact.join("normalization.json"));
    input.tree = Some(read(compact.join("tree-normalization.json")));
    assert_eq!(input.normalization, policy);
    assert_eq!(
        input.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-ordinary-itemset-membership"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ordinary-itemset-membership-v1",
            &(&a, &p),
            1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    let mut restored = input.clone();
    restored.normalization.equipment_membership = None;
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(restored, *prior.input());
    write(out.join("endpoint.json"), &input);
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            run("assemble-owned-release", &[src, Path::new("--output"), dst]),
            serde_json::to_value(staged.receipt()).unwrap()
        );
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    for (name, bytes) in staged.artifacts() {
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&name)
        {
            assert_eq!(bytes, fs::read(path.join(name)).unwrap());
        }
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&path, &source, case, &old);
        release::normalize(&package, &source, case, &new);
        reports.push(compare_original(
            &fs::read(&source).unwrap(),
            &old,
            &new,
            &out,
            case,
        ));
    }
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"19\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let item = &original[start..end];
    for (label, revised) in [
        (
            "unknown-rune",
            item.replacen("Rune: None", "Rune: UnknownAugment", 1),
        ),
        ("missing-none", item.replacen("Rune: None\n", "", 1)),
        (
            "loader-fallback",
            item.replacen("New Item", "Tabula Rasa", 1),
        ),
        (
            "loader-fallback-two",
            item.replacen("New Item", "Sekhema's Resolve", 1),
        ),
    ] {
        assert_ne!(revised, item);
        let xml = format!("{}{}{}", &original[..start], revised, &original[end..]);
        let source = out.join(format!("probe-{label}.xml"));
        write_probe(&source, &xml);
        let dst = out.join(format!("probe-{label}"));
        release::normalize(&package, &source, 5, &dst);
        let r = finalize(
            xml.as_bytes(),
            &dst,
            &out.join(format!("probe-{label}-selection.json")),
        );
        assert!(
            r["finalization"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["code"] == "equipment-membership-not-converted")
        );
    }
    assert_eq!(hashes, release::inventory(&path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.receipt().definitions,"registry":staged.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":staged.input().provenance.len(),"originals":reports,"queries":110,"selected_receiving_uses":9,"selected_items":8,"prior_unchanged":true,"rebuild_byte_identical":true,"probes":4,"complete_original_builds":0}),
    );
}
fn write_probe(path: &Path, text: &str) {
    fs::write(path, text.as_bytes()).unwrap();
}
