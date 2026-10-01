//! Template-scoped source absence; no numerical or whole-input coverage grant.
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_item_source::{ItemSourceAbsentPolicy, ItemSourceTemplateDefaults},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/item-level-absence")
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn defaults() -> Vec<ItemSourceTemplateDefaults> {
    read(data().join("defaults.json"))
}
fn publish(input: &Path, output: &Path) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .args(["assemble-owned-release"])
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let authoring: Value = read(data().join("authoring.json"));
    assert_eq!(prior.receipt().input.to_string(), authoring["before"]);
    let mut input = prior.input().clone();
    for row in defaults() {
        assert!(
            !input
                .item_source
                .template_defaults
                .iter()
                .any(|v| v.template == row.template)
        );
        input.item_source.template_defaults.push(row);
    }
    input.item_source.version = key("item-level-absence-source-v1");
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-item-level-absence"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-item-level-absence-v1",
            &(defaults(), authoring),
            1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().clone();
    let n = prior.input().item_source.template_defaults.len();
    assert_eq!(&restored.item_source.template_defaults[n..], defaults());
    restored.item_source.template_defaults.truncate(n);
    restored.item_source.version = prior.input().item_source.version.clone();
    assert_eq!(
        restored.provenance.len(),
        prior.input().provenance.len() + 1
    );
    restored.provenance.pop();
    assert_eq!(
        restored,
        *prior.input(),
        "only the two defaults, policy identity and provenance change"
    );
    next
}

#[test]
fn absent_level_facts_are_scoped_and_bind_authenticated_complete_source() {
    let rows = defaults();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .map(|v| v.template.key().as_str())
            .collect::<Vec<_>>(),
        ["def.0000000000002007", "def.000000000000238c"]
    );
    for row in rows {
        assert_eq!(row.item_level, ItemSourceAbsentPolicy::Absent);
        assert_eq!(row.quality, ItemSourceAbsentPolicy::Pending);
        assert!(row.parameters.is_empty());
    }
    let a: Value = read(data().join("authoring.json"));
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    assert_eq!(a["source_files"].as_array().unwrap().len(), 5);
    for pin in a["source_files"].as_array().unwrap() {
        let source = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["path"] == pin["path"])
            .unwrap();
        assert_eq!(pin["sha256"], source["sha256"]);
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
}

fn source_item<'a>(sidecar: &'a Value, row: &Value) -> &'a Value {
    let origin = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"] == row["source"])
        .unwrap();
    let items: Vec<_> = origin["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == "item")
        .collect();
    assert_eq!(items.len(), 1);
    &items[0]["value"]
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    let mut right = b["draft"].clone();
    let mut retired = vec![];
    let mut changes = vec![];
    let old_items = a["draft"]["items"]["members"].as_array().unwrap();
    let new_items = right["items"]["members"].as_array_mut().unwrap();
    assert_eq!(old_items.len(), new_items.len());
    for (x, y) in old_items.iter().zip(new_items.iter_mut()) {
        if x["item_level"] == y["item_level"] {
            continue;
        }
        // Pending IDs differ across fresh lineages; compare their semantic kind first.
        if x["item_level"]["kind"] == y["item_level"]["kind"] {
            continue;
        }
        assert_eq!(case, 5);
        assert_eq!(x["item_level"]["kind"], "pending");
        assert_eq!(x["item_level"]["code"], "item-level-not-converted");
        assert_eq!(y["item_level"], json!({"kind":"known","value":null}));
        assert!(
            defaults()
                .iter()
                .any(|v| serde_json::to_value(&v.template).unwrap() == x["template"]["value"])
        );
        let row = sb["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| source_item(&sb, r) == &y["id"])
            .unwrap();
        assert_eq!(row["attribution"]["layout"]["status"], "proven");
        assert_eq!(row["defaults"]["item_level_absent"], true);
        assert!(
            row["lines"]
                .as_array()
                .unwrap()
                .iter()
                .all(|v| !v["text"].as_str().unwrap().starts_with("Item Level:"))
        );
        retired.push(x["item_level"]["id"].clone());
        changes.push(json!({"source":row["source"],"template":x["template"]["value"],"before":x["item_level"],"after":y["item_level"]}));
        y["item_level"] = x["item_level"].clone();
    }
    assert_eq!(retired.len(), if case == 5 { 2 } else { 0 });
    let parse = |v: &Value| u64::from_str_radix(v.as_str().unwrap(), 16).unwrap();
    assert_eq!(
        parse(&a["draft"]["allocator"]["last_issued"])
            - parse(&b["draft"]["allocator"]["last_issued"]),
        retired.len() as u64
    );
    right["allocator"] = a["draft"]["allocator"].clone();
    let mut ids = BTreeMap::new();
    correspond(&a["draft"], &mut right, &mut ids, "draft");
    // Every origin link is retained except the exact retired item-level issues.
    let mut old_origins = sa["origins"].clone();
    for row in old_origins.as_array_mut().unwrap() {
        row["links"]
            .as_array_mut()
            .unwrap()
            .retain(|v| !(v["kind"] == "issue" && retired.contains(&v["value"])));
    }
    let mut new_origins = sb["origins"].clone();
    correspond(&old_origins, &mut new_origins, &mut ids, "source origins");
    for (new, old) in &ids {
        let new: Value = serde_json::from_str(new).unwrap();
        if new["lineage"] == old["lineage"] {
            // Restored retired issue IDs are comparison placeholders only.
            assert!(retired.contains(old));
            continue;
        }
        let old_local = parse(&old["local"]);
        let shift = retired
            .iter()
            .filter(|v| parse(&v["local"]) < old_local)
            .count();
        assert_eq!(
            parse(&new["local"]),
            old_local - shift as u64,
            "only the retired Pending allocations may shift retained IDs"
        );
    }
    let mut old_texts = sa["item_texts"].clone();
    let mut new_texts = sb["item_texts"].clone();
    assert_eq!(
        old_texts.as_array().unwrap().len(),
        new_texts.as_array().unwrap().len()
    );
    for (x, y) in old_texts
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(new_texts.as_array_mut().unwrap())
    {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(y["attribution"]["policy"], sb["item_source_policy"]);
        y["attribution"]["policy"] = x["attribution"]["policy"].clone();
        if changes.iter().any(|v| v["source"] == x["source"]) {
            assert_eq!(x["defaults"]["item_level_absent"], false);
            assert_eq!(y["defaults"]["item_level_absent"], true);
            assert_eq!(y["attribution"]["default_scope"]["kind"], "proven");
            y["defaults"]["item_level_absent"] = x["defaults"]["item_level_absent"].clone();
            y["attribution"]["default_scope"] = x["attribution"]["default_scope"].clone();
        }
        relocate(y, &ids);
        selected::canonical(x);
        selected::canonical(y);
        assert_eq!(
            x, y,
            "source raw text, members, categories, quality and all other evidence preserved"
        );
    }
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
    let mut expected = before["finalization"]["issues"].as_array().unwrap().clone();
    expected.retain(|v| !retired.contains(&v["id"]));
    let mut actual = after["finalization"]["issues"].clone();
    relocate(&mut actual, &ids);
    assert_eq!(
        json!(expected),
        actual,
        "all other selected issues preserved"
    );
    let mut new_selection = selected::selection(xml, new);
    relocate(&mut new_selection, &ids);
    assert_eq!(selected::selection(xml, old), new_selection);
    json!({"original":case,"changes":changes,"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"allocator_before":a["draft"]["allocator"],"allocator_after":b["draft"]["allocator"],"selected_issue_summary":after["finalization"]["issue_summary"]})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    for item in [19, 20] {
        let start = original.find(&format!("<Item id=\"{item}\">")).unwrap();
        let end = start + original[start..].find("</Item>").unwrap() + 7;
        let raw = &original[start..end];
        for (label, header) in [
            ("explicit", "Item Level: 17\n"),
            ("unknown-prefix", "Unreviewed source input\n"),
            ("malformed", "Item Level: unknown\n"),
            ("duplicate", "Item Level: 17\nItem Level: 18\n"),
        ] {
            let changed = raw.replace("Quality: 20", &format!("{header}Quality: 20"));
            assert_ne!(changed, raw);
            let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
            let path = out.join(format!("probe-{item}-{label}.xml"));
            write_raw(&path, xml.as_bytes());
            let dest = out.join(format!("probe-{item}-{label}"));
            release::normalize(package, &path, 5, &dest);
            let s: Value = read(dest.join("sidecar.json"));
            let d: Value = read(dest.join("draft.json"));
            let row = s["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["source"]["ordinal"] == if item == 19 { 572 } else { 574 })
                .unwrap();
            assert_eq!(row["defaults"]["item_level_absent"], false);
            let id = source_item(&s, row);
            let value = &d["draft"]["items"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| &i["id"] == id)
                .unwrap()["item_level"];
            if label == "explicit" {
                assert_eq!(value, &json!({"kind":"known","value":17}));
            } else {
                assert_eq!(value["kind"], "pending");
            }
        }
    }
    8
}
fn write_raw(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
}

#[test]
#[ignore = "requires exact checked category-input predecessor"]
fn real_two_template_absence_preserves_saved_requests_and_all_other_inputs() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ITEM_LEVEL_PRIOR").expect("explicit prior"),
    );
    let hashes = release::inventory(&p);
    let prior = release::load(&p);
    let next = stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_ITEM_LEVEL_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let endpoint = out.join("endpoint.json");
    write(&endpoint, next.input());
    let package = out.join("package");
    assert_eq!(
        publish(&endpoint, &package),
        serde_json::to_value(next.receipt()).unwrap()
    );
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&package, &rebuilt),
        serde_json::to_value(next.receipt()).unwrap()
    );
    let published = release::inventory(&package);
    assert_eq!(published, release::inventory(&rebuilt));
    for (name, hash) in &hashes {
        if !matches!(name.as_str(), "item-source.json" | "release.json") {
            assert_eq!(published.get(name), Some(hash), "{name}");
        }
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    let probe_count = probes(&package, &out);
    assert_eq!(hashes, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"provenance":next.input().provenance.len(),"queries":110,"originals":reports,"probes":probe_count,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
