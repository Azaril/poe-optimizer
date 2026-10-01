//! Full-source inventory proof never supplies new scalar or numerical values.
#[path = "support/owned_support_gem_inventory.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_definitions::GemDefId;
use poe_optimizer_import::owned_release::assemble_owned_release;
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
fn write(path: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(path, serde_json::to_vec(v).unwrap()).unwrap()
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
fn inventory() -> Vec<Value> {
    serde_json::to_value(family::inventory())
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn inventory_uses_the_whole_reviewed_single_support_catalog_without_fixture_membership() {
    let a = family::authoring();
    let mut loaded = BTreeMap::new();
    for field in [
        "review",
        "source_catalog",
        "source_manifest",
        "identity_mapping",
    ] {
        let item = &a[field];
        assert_eq!(item["hash_encoding"], "utf8_lf");
        let text = fs::read_to_string(root().join(item["path"].as_str().unwrap()))
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(
            format!("{:x}", Sha256::digest(text.as_bytes())),
            item["sha256"]
        );
        loaded.insert(field, serde_json::from_str::<Value>(&text).unwrap());
    }
    let review = &loaded["review"];
    let catalog = &loaded["source_catalog"];
    assert_eq!(review["catalog_digest"], a["catalog"]);
    assert_eq!(catalog["source"]["upstream_revision"], a["source_revision"]);
    assert_eq!(
        loaded["source_manifest"]["upstream_revision"],
        a["source_revision"]
    );
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
    let expected: BTreeSet<_> = review["source_gems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(expected.len(), 514);
    let rows = inventory();
    assert_eq!(rows.len(), 514);
    let mut sources = BTreeSet::new();
    let mut owners = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for row in rows {
        let candidates: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|g| g["game_id"] == row["game_id"] && g["variant_id"] == row["variant_id"])
            .collect();
        assert_eq!(candidates.len(), 1);
        let g = candidates[0];
        let key = g["key"].as_str().unwrap();
        assert!(expected.contains(key));
        assert!(sources.insert(key.to_owned()));
        assert!(owners.insert(row["gem"]["key"].as_str().unwrap().to_owned()));
        assert_eq!(row["skill_id"], g["primary_effect_id"]);
        let matches: Vec<_> = loaded["identity_mapping"]["entries"]
            .as_array().unwrap().iter().filter(|entry| {
                entry["source"] == json!({"kind":"definition","value":{"kind":"gem","value":{"game_id":{"kind":"text","value":row["game_id"]},"variant_id":{"kind":"text","value":row["variant_id"]}}}})
            }).collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0]["outcome"],
            json!({"kind":"mapped","value":{"target":{"kind":"definition","value":{"kind":"gem","value":row["gem"]}},"basis":{"kind":"exact"}}})
        );
        let name = if g["name_spec"].is_null() {
            &g["name"]
        } else {
            &g["name_spec"]
        };
        assert_eq!(&row["name_spec"], name);
        assert_eq!(g["effect_list"], json!([row["skill_id"]]));
        for f in [
            "declared_additional_effects",
            "declared_additional_stat_sets",
            "constructed_additional_effects",
            "additional_effects",
        ] {
            assert_eq!(g[f], json!([]));
        }
        let s = catalog["skills"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == row["skill_id"])
            .unwrap();
        assert_eq!(s["support"], true);
        assert_ne!(s["from_tree"], true);
        for field in ["corrupted", "corruption_level"] {
            assert_eq!(
                row[field]["declaration"],
                json!({"kind":"gem","definition":row["gem"]})
            );
            assert!(slots.insert(row[field]["slot"]["key"].as_str().unwrap().to_owned()));
        }
    }
    assert_eq!(sources, expected.into_iter().map(str::to_owned).collect());
    assert_eq!(slots.len(), 1028);
    assert_eq!(a["new_definitions"], 0);
    assert_eq!(a["new_scalar_values"], 0);
    assert_eq!(a["new_programs"], 0);
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    let full_expected = [43, 90, 47, 46, 111][case - 1];
    let selected_expected = [43, 28, 47, 46, 16][case - 1];
    let selected_before = [295, 300, 292, 358, 120][case - 1];
    let domain: BTreeSet<_> = family::inventory().into_iter().map(|r| r.gem).collect();
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(issued(&pa) - issued(&na), full_expected);
    b["draft"]["allocator"] = pa.clone();
    let aa = a["draft"]["gems"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["gems"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    let mut retired = BTreeSet::new();
    let mut scalar_count = 0;
    for (x, y) in aa.iter_mut().zip(bb) {
        if x["definition"]["kind"] == "known"
            && domain.contains(
                &serde_json::from_value::<GemDefId>(x["definition"]["value"].clone()).unwrap(),
            )
        {
            assert_eq!(x["definition"], y["definition"]);
            assert_eq!(x["level"], y["level"]);
            assert_eq!(x["quality"], y["quality"]);
            assert_eq!(x["parameters"]["members"], y["parameters"]["members"]);
            let members = x["parameters"]["members"].as_array().unwrap();
            assert_eq!(members.len(), 2);
            assert!(
                members
                    .iter()
                    .all(|m| m["slot"]["kind"] == "known" && m["value"]["kind"] == "known")
            );
            scalar_count += members.len();
            assert_eq!(
                x["parameters"]["completion"]["code"],
                "gem-parameters-not-converted"
            );
            assert!(
                retired.insert(
                    x["parameters"]["completion"]["id"]["local"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                )
            );
            assert_eq!(y["parameters"]["completion"], json!({"kind":"complete"}));
            for row in [x, y] {
                row["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
            }
        }
    }
    assert_eq!(retired.len() as u64, full_expected);
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "only reviewed concrete Gem inventories may complete",
    );
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue"
                && link["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id));
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
    // No source-side evidence shape or item policy changes in this publication.
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], pa);
    assert_eq!(sb["allocator_after"], na);
    sb["allocator_after"] = sa["allocator_after"].clone();
    correspond(
        &sa,
        &mut sb,
        &mut ids,
        "exact source observations and all retained links survive",
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
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y);
    assert_eq!(count, selected_before);
    assert_eq!(count - y.as_array().unwrap().len(), selected_expected);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"full_lists_completed":retired.len(),"selected_lists_completed":selected_expected,"selected_before":count,"selected_after":y.as_array().unwrap().len(),"unchanged_scalar_assignments":scalar_count,"allocator_before":pa,"allocator_after":na,"retired":retired,"selected_issue_summary":after["selected_issue_summary"]})
}

fn source_gem<'a>(sidecar: &'a Value, ordinal: &Value) -> &'a Value {
    optional_source_gem(sidecar, ordinal).unwrap()
}
fn optional_source_gem<'a>(sidecar: &'a Value, ordinal: &Value) -> Option<&'a Value> {
    let row = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| &r["source"]["ordinal"] == ordinal)
        .unwrap();
    let gems: Vec<_> = row["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "gem")
        .collect();
    assert!(gems.len() <= 1);
    gems.first().map(|g| &g["value"])
}
fn preserve_probe_members(
    prior: &Path,
    source: &Path,
    out: &Path,
    label: &str,
    ordinal: &Value,
    gem: Option<&Value>,
) {
    let old = out.join(format!("probe-{label}-prior"));
    release::normalize(prior, source, 5, &old);
    let s: Value = read(old.join("sidecar.json"));
    let d: Value = read(old.join("draft.json"));
    let before = optional_source_gem(&s, ordinal).map(|id| {
        d["draft"]["gems"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == *id)
            .unwrap()
    });
    match (before, gem) {
        (None, None) => {}
        (Some(before), Some(gem)) => {
            for field in ["definition", "level", "quality"] {
                assert_eq!(before[field], gem[field], "{label}: {field}");
            }
            assert_eq!(
                before["parameters"]["members"], gem["parameters"]["members"],
                "existing partial scalar conversion stays exact: {label}"
            );
            for row in [before, gem] {
                assert_eq!(row["parameters"]["completion"]["kind"], "pending");
            }
        }
        _ => panic!("physical instance presence changed for {label}"),
    }
}
fn probes(prior: &Path, package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let rows = inventory();
    // Select by authored exact physical identity; no fixture source ordinal or
    // English skill-name dispatch participates in conversion.
    let row = rows
        .iter()
        .find(|row| original.contains(&format!("gemId=\"{}\"", row["game_id"].as_str().unwrap())))
        .unwrap();
    let id = format!("gemId=\"{}\"", row["game_id"].as_str().unwrap());
    let at = original.find(&id).unwrap();
    let start = original[..at].rfind("<Gem ").unwrap();
    let end = at + original[at..].find("/>").unwrap() + 2;
    let raw = &original[start..end];
    let cases = vec![
        (
            "unknown-field",
            raw.replace("<Gem ", "<Gem unknownInput=\"1\" "),
        ),
        (
            "namespace-field",
            raw.replace("<Gem ", "<Gem xmlns:q=\"urn:unreviewed\" q:count=\"1\" "),
        ),
        (
            "child-map",
            raw.replace("/>", "><StatSetIndices index=\"2\"/></Gem>"),
        ),
        ("count", raw.replace("count=\"1\"", "count=\"2\"")),
        (
            "global-one",
            raw.replace("enableGlobal1=\"true\"", "enableGlobal1=\"false\""),
        ),
        (
            "global-two",
            raw.replace("enableGlobal2=\"true\"", "enableGlobal2=\"false\""),
        ),
        (
            "legacy-number",
            raw.replace("<Gem ", "<Gem statSetIndex=\"2\" "),
        ),
        (
            "missing-corrupted",
            raw.replace("corrupted=\"false\"", "")
                .replace("corrupted=\"nil\"", "")
                .replace("corrupted=\"true\"", ""),
        ),
        (
            "bad-corrupted",
            raw.replace("corrupted=\"false\"", "corrupted=\"unknown\"")
                .replace("corrupted=\"nil\"", "corrupted=\"unknown\"")
                .replace("corrupted=\"true\"", "corrupted=\"unknown\""),
        ),
        (
            "missing-delta",
            raw.replace("corruptLevel=\"nil\"", "")
                .replace("corruptLevel=\"0\"", ""),
        ),
        (
            "bad-delta",
            raw.replace("corruptLevel=\"nil\"", "corruptLevel=\"NaN\"")
                .replace("corruptLevel=\"0\"", "corruptLevel=\"NaN\""),
        ),
    ];
    let mut ordinal = None;
    for (label, changed) in &cases {
        assert_ne!(changed, raw, "{label}");
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let s: Value = read(dest.join("sidecar.json"));
        let d: Value = read(dest.join("draft.json"));
        let origin = s["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                if let Some(o) = &ordinal {
                    return &r["source"]["ordinal"] == o;
                }
                let Some(id) = optional_source_gem(&s, &r["source"]["ordinal"]) else {
                    return false;
                };
                d["draft"]["gems"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g["id"] == *id && g["definition"]["value"] == row["gem"])
            })
            .unwrap();
        let o = origin["source"]["ordinal"].clone();
        if let Some(old) = &ordinal {
            assert_eq!(&o, old);
        } else {
            ordinal = Some(o.clone());
        }
        let gem = optional_source_gem(&s, &o).map(|id| {
            d["draft"]["gems"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|g| g["id"] == *id)
                .unwrap()
        });
        preserve_probe_members(prior, &path, out, label, &o, gem);
    }
    let group_start = original[..start].rfind("<Skill ").unwrap();
    let mut xml = original.clone();
    xml.insert_str(group_start + 7, "skillPart=\"2\" ");
    let path = out.join("probe-group-part.xml");
    fs::write(&path, xml).unwrap();
    let dest = out.join("probe-group-part");
    release::normalize(package, &path, 5, &dest);
    let s: Value = read(dest.join("sidecar.json"));
    let d: Value = read(dest.join("draft.json"));
    let ordinal = ordinal.unwrap();
    let id = source_gem(&s, &ordinal);
    let gem = d["draft"]["gems"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == *id)
        .unwrap();
    assert_eq!(gem["parameters"]["completion"]["kind"], "pending");
    preserve_probe_members(prior, &path, out, "group-part", &ordinal, Some(gem));
    cases.len() + 1
}

#[test]
#[ignore = "requires the exact checked requested-metric predecessor"]
fn real_support_inventories_preserve_all_scalars_requests_and_static_coverage() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_INVENTORY_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_INVENTORY_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("policy.json"), &family::policy(&prior));
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(src, dst),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
    let inv = release::inventory(&package);
    assert_eq!(inv, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if ![
            "release.json",
            "normalization.json",
            "tree-normalization.json",
        ]
        .contains(&name.as_str())
        {
            assert_eq!(inv.get(name), Some(hash), "unchanged {name}");
        }
    }
    for field in ["roles", "catalog", "scalar_inputs"] {
        let mut bad = serde_json::to_value(next.input()).unwrap();
        bad["normalization"]["gem_inventory"][field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let a = out.join(format!("prior-original-{case:02}"));
        let b = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &a);
        release::normalize(&package, &xml, case, &b);
        reports.push(compare(case, &fs::read(xml).unwrap(), &a, &b, &out));
    }
    let count = probes(&p, &package, &out);
    assert_eq!(before, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"reviewed_definitions":514,"full_lists_completed":337,"selected_lists_completed":180,"unchanged_domain_scalar_values":674,"originals":reports,"probes":count,"stale_binding_rejections":3,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
