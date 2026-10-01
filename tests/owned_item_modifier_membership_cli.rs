//! Singleton physical modifier membership; all other input/coverage facets remain open.
#[path = "support/owned_item_modifier_membership.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
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
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
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

#[test]
fn injected_singleton_policy_binds_reviewed_templates_and_source_packages() {
    let policy = serde_json::to_value(family::policy()).unwrap();
    assert_eq!(policy["kind"], "pob_fresh_ordinary_singleton_v1");
    assert_eq!(policy["modifier_rules"], json!(["fixed-life"]));
    let templates = policy["templates"].as_array().unwrap();
    assert_eq!(templates.len(), 2);
    for (row, key) in templates
        .iter()
        .zip(["def.0000000000002007", "def.000000000000238c"])
    {
        assert_eq!(row["template"]["key"], key);
        assert_eq!(
            row["generated_members"],
            "no_buff_implicit_rune_or_class_members"
        );
    }
    let a = family::authoring();
    assert_eq!(policy["definitions"], a["definitions"]);
    assert_eq!(policy["item_lines"], a["items"]);
    assert_eq!(policy["item_source"], a["item_source"]);
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    assert_eq!(a["source_files"].as_array().unwrap().len(), 5);
    for pin in a["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
}

fn source_item(sidecar: &Value, ordinal: u64) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    let links: Vec<_> = rows[0]["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "item")
        .collect();
    assert_eq!(links.len(), 1);
    &links[0]["value"]
}
fn item<'a>(draft: &'a Value, id: &Value) -> &'a Value {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| &r["id"] == id)
        .unwrap()
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    let expected_ids: Vec<_> = if case == 5 {
        [572, 574]
            .map(|o| (source_item(&sa, o).clone(), source_item(&sb, o).clone()))
            .to_vec()
    } else {
        vec![]
    };
    let mut retired = BTreeSet::new();
    if case != 5 {
        assert_eq!(a, b, "unaffected originals preserve exact IDs");
    }
    let prior_allocator = a["draft"]["allocator"].clone();
    let next_allocator = b["draft"]["allocator"].clone();
    let issued = |a: &Value| u64::from_str_radix(a["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&prior_allocator) - issued(&next_allocator),
        if case == 5 { 4 } else { 0 }
    );
    b["draft"]["allocator"] = prior_allocator.clone();
    let aa = a["draft"]["items"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["items"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (x, y) in aa.iter_mut().zip(bb) {
        if let Some((_, new_id)) = expected_ids.iter().find(|(old_id, _)| old_id == &x["id"]) {
            assert_eq!(&y["id"], new_id, "join the exact source occurrence");
            assert_eq!(y["parameters"]["completion"]["kind"], "pending");
            assert_eq!(y["modifiers"]["members"].as_array().unwrap().len(), 1);
            assert_eq!(y["modifiers"]["completion"], json!({"kind":"complete"}));
            assert_eq!(
                y["modifier_order"],
                json!({"kind":"known","value":[y["modifiers"]["members"][0]["id"]]})
            );
            for (old_issue, code) in [
                (
                    &x["modifiers"]["completion"],
                    "item-modifiers-not-converted",
                ),
                (&x["modifier_order"], "item-modifier-order-not-converted"),
            ] {
                assert_eq!(old_issue["kind"], "pending");
                assert_eq!(old_issue["code"], code);
                assert!(retired.insert(old_issue["id"]["local"].as_str().unwrap().to_owned()));
            }
            for v in [x, y] {
                v["modifiers"].as_object_mut().unwrap().remove("completion");
                v.as_object_mut().unwrap().remove("modifier_order");
            }
        }
    }
    assert_eq!(retired.len(), if case == 5 { 4 } else { 0 });
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "only the two exact structural facets change",
    );
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|r| {
            let remove = r["kind"] == "issue"
                && r["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id));
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(
        removed,
        retired.len(),
        "retire one exact source link per issue"
    );
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], prior_allocator);
    assert_eq!(sb["allocator_after"], next_allocator);
    sb["allocator_after"] = sa["allocator_after"].clone();
    correspond(
        &sa,
        &mut sb,
        &mut ids,
        "all source content, defaults, attribution and retained links stay exact",
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
    relocate(&mut y, &ids);
    let count = x.as_array().unwrap().len();
    x.as_array_mut()
        .unwrap()
        .retain(|r| !retired.contains(r["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "every other selected issue remains exact");
    assert_eq!(count - y.as_array().unwrap().len(), retired.len());
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"selected_before":count,"selected_after":y.as_array().unwrap().len(),"retired":retired,"allocator_before":prior_allocator,"allocator_after":next_allocator,"selected_issue_summary":after["selected_issue_summary"]})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"19\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let cases = [
        (
            "second-member",
            raw.replace(
                "+17 to maximum Life",
                "+17 to maximum Life\n+3 to maximum Life",
            ),
        ),
        (
            "unknown-member",
            raw.replace(
                "+17 to maximum Life",
                "+17 to maximum Life\nUnknown source semantics",
            ),
        ),
        ("implicit", raw.replace("Implicits: 0", "Implicits: 1")),
        (
            "enchant",
            raw.replace("+17 to maximum Life", "{enchant}+17 to maximum Life"),
        ),
        (
            "occupied-rune",
            raw.replacen("Rune: None", "Rune: Lesser Iron Rune", 1),
        ),
        (
            "legacy-sockets",
            raw.replace("Sockets: S S S S", "Sockets: S S S S J"),
        ),
        (
            "variant",
            raw.replace("+17 to maximum Life", "{variant:1}+17 to maximum Life"),
        ),
        (
            "unknown-child",
            raw.replace("</Item>", "<Unknown/>\n</Item>"),
        ),
    ];
    for (label, changed) in &cases {
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, &xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let s: Value = read(dest.join("sidecar.json"));
        let d: Value = read(dest.join("draft.json"));
        let i = item(&d, source_item(&s, 572));
        assert_eq!(i["modifiers"]["completion"]["kind"], "pending", "{label}");
        assert_eq!(i["modifier_order"]["kind"], "pending", "{label}");
        assert_eq!(i["parameters"]["completion"]["kind"], "pending");
        if *label == "second-member" {
            assert_eq!(i["modifiers"]["members"].as_array().unwrap().len(), 2);
        }
    }
    for (label, changed, amount) in [
        (
            "other-roll",
            raw.replace("+17 to maximum Life", "+29 to maximum Life"),
            29.0,
        ),
        (
            "fixed-overlay",
            raw.replace("range=\"0.5\"", "range=\"1\""),
            17.0,
        ),
    ] {
        assert_ne!(changed, raw);
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, &xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let s: Value = read(dest.join("sidecar.json"));
        let d: Value = read(dest.join("draft.json"));
        let i = item(&d, source_item(&s, 572));
        assert_eq!(
            i["modifiers"]["completion"],
            json!({"kind":"complete"}),
            "{label}"
        );
        assert_eq!(i["modifiers"]["members"].as_array().unwrap().len(), 1);
        let m = &i["modifiers"]["members"][0];
        assert_eq!(
            m["rolls"]["members"][0]["value"]["value"]["value"]["value"],
            amount
        );
        assert_eq!(
            i["modifier_order"],
            json!({"kind":"known","value":[m["id"]]})
        );
        assert_eq!(i["parameters"]["completion"]["kind"], "pending");
    }
    cases.len() + 2
}

#[test]
#[ignore = "requires the exact checked flat-Life publication predecessor"]
fn real_publication_closes_only_proven_singleton_modifier_membership_and_order() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ITEM_MODIFIER_MEMBERSHIP_PRIOR")
            .expect("explicit predecessor"),
    );
    let inventory = release::inventory(&p);
    let prior = release::load(&p);
    let staged = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_ITEM_MODIFIER_MEMBERSHIP_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("normalization.json"), staged.normalization());
    let compact = out.join("compact");
    write(
        out.join("publication.json"),
        &run(
            "publish-owned-normalization",
            &[
                &p,
                Path::new("--normalization"),
                &out.join("normalization.json"),
                Path::new("--output"),
                &compact,
            ],
        ),
    );
    assert_eq!(
        read::<Value>(compact.join("normalization.json")),
        serde_json::to_value(staged.normalization()).unwrap()
    );
    assert_eq!(
        read::<Value>(compact.join("tree-normalization.json")),
        serde_json::to_value(staged.input().tree.as_ref().unwrap()).unwrap()
    );
    write(out.join("endpoint.json"), staged.input());
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
            assert_eq!(bytes, fs::read(p.join(name)).unwrap());
        }
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&p, &source, case, &old);
        release::normalize(&package, &source, case, &new);
        reports.push(compare(case, &fs::read(source).unwrap(), &old, &new, &out));
    }
    let count = probes(&package, &out);
    assert_eq!(inventory, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.receipt().definitions,"registry":staged.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":staged.input().provenance.len(),"queries":110,"originals":reports,"probes":count,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
