//! Whole-import inverses and independent source-occurrence controls.
use crate::{identity, preservation, release, selected};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn packet() -> Value {
    read(root().join("data/owned/poe2/3887ae68/generated-participation/source-vectors.json"))
}
fn bindings() -> Value {
    read(root().join("data/owned/poe2/3887ae68/skill-participation/bindings.json"))
}
fn rows(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn resolved(row: &Value) -> bool {
    matches!(
        row["normalization_status"].as_str().unwrap(),
        "authored_direct" | "exact_generated_provider_resolved"
    )
}
fn target(draft: &Value, side: &Value, row: &Value) -> Value {
    if row["kind"] == "manual" {
        return json!({"kind":"skill","value":{"kind":"authored","value":{"kind":"known","value":preservation::link(side,row["gem_ordinal"].as_u64().unwrap() as usize,"skill")}}});
    }
    let preset = preservation::link(
        side,
        row["skill_set_ordinal"].as_u64().unwrap() as usize,
        "skill_preset",
    );
    let p = preservation::member(draft, "skill_presets", &preset);
    let supply = if row["gem_attributes"]["skillId"] == "FireboltPlayer" {
        "def.00000000000031c9"
    } else {
        "def.00000000000032d2"
    };
    let matching: Vec<_> = rows(&p["intent"]["generated_inputs"]["members"])
        .iter()
        .filter(|r| r["target"]["slot"]["value"]["slot"]["key"] == supply)
        .collect();
    assert_eq!(matching.len(), 1, "exact preset-owned provider input");
    json!({"kind":"skill","value":{"kind":"generated","value":matching[0]["target"]}})
}
fn expected(target: Value, manual: bool, group: bool, occurrence: bool) -> Value {
    let b = bindings();
    json!({"selection":{"policy":{"kind":"known","value":b["policy"]},"target":target,
        "parameters":{"members":[
            {"slot":{"kind":"known","value":b["group"]},"value":{"kind":"known","value":{"kind":"boolean","value":group}}},
            {"slot":{"kind":"known","value":b["occurrence"]},"value":{"kind":"known","value":{"kind":"boolean","value":occurrence}}}
        ],"completion":{"kind":"complete"}}},"applicability":if manual {"required"} else {"when_exact_source_selected"}})
}
fn bare_lineages(v: &mut Value, old: &Value, new: &Value) {
    match v {
        Value::Object(o) => {
            if o.contains_key("local") && o.contains_key("lineage") {
                return;
            }
            if let Some(l) = o.get_mut("lineage") {
                assert_eq!(l, new);
                *l = old.clone();
            }
            for v in o.values_mut() {
                bare_lineages(v, old, new);
            }
        }
        Value::Array(a) => {
            for v in a {
                bare_lineages(v, old, new);
            }
        }
        _ => {}
    }
}
pub fn original(
    case: usize,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    prior_path: &Path,
    package: &Path,
    out: &Path,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let a = read(old.join("draft.json"));
    let mut b = read(new.join("draft.json"));
    let sa = read(old.join("sidecar.json"));
    let mut sb = read(new.join("sidecar.json"));
    preservation::authenticate(&sa, &old, prior_path, case, prior);
    preservation::authenticate(&sb, &new, package, case, next);
    let evidence = packet();
    let census = &evidence["original_inventory"][case - 1];
    assert_eq!(census["original"], case);
    let mut added = 0;
    for row in rows(&census["occurrences"]).iter().filter(|r| resolved(r)) {
        let id = preservation::link(
            &sb,
            row["skill_set_ordinal"].as_u64().unwrap() as usize,
            "skill_preset",
        );
        let wanted = target(&b, &sb, row);
        let p = preservation::member(&b, "skill_presets", &id);
        assert_eq!(
            rows(&p["intent"]["usage"]["members"])
                .iter()
                .filter(
                    |u| u["selection"]["policy"]["value"]["key"] == "def.000000000000326a"
                        && u["selection"]["target"] == wanted
                )
                .count(),
            1
        );
        assert_eq!(row["group_attributes"]["enabled"], "true");
        assert_eq!(row["gem_attributes"]["enabled"], "true");
        let expected = expected(wanted, row["kind"] == "manual", true, true);
        let p = b["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["id"] == id)
            .unwrap();
        let usage = p["intent"]["usage"]["members"].as_array_mut().unwrap();
        let indexes: Vec<_> = usage
            .iter()
            .enumerate()
            .filter(|(_, u)| **u == expected)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            indexes.len(),
            1,
            "one exact new preference: original{case}, ordinal{}",
            row["gem_ordinal"]
        );
        usage.remove(indexes[0]);
        added += 1;
    }
    assert_eq!(added, [2, 0, 0, 0, 7][case - 1]);
    let oldlineage = a["draft"]["allocator"]["lineage"].clone();
    let newlineage = b["draft"]["allocator"]["lineage"].clone();
    assert_eq!(
        a["draft"]["allocator"]["last_issued"],
        b["draft"]["allocator"]["last_issued"]
    );
    b["draft"]["allocator"] = a["draft"]["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(&a, &mut b, &mut ids, "whole participation draft inverse");
    // Both real dependency commitments have been authenticated above. Only
    // normalization/tree rebinding and the changed draft can differ.
    for field in ["policy", "tree_policy", "draft", "allocator_after"] {
        sb[field] = sa[field].clone();
    }
    let allocator = sb["allocator_after"].take();
    bare_lineages(&mut sb, &oldlineage, &newlineage);
    sb["allocator_after"] = allocator;
    identity::correspond(
        &sa,
        &mut sb,
        &mut ids,
        "whole participation sidecar inverse",
    );
    let xml = fs::read(root().join(census["fixture"].as_str().unwrap())).unwrap();
    let mut selection = selected::selection(&xml, &new);
    identity::relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(&xml, &old));
    let before = selected::finalize_with_definitions(
        &xml,
        &old,
        &out.join(format!("prior-selected-{case:02}.json")),
        &prior_path.join("schema.json"),
    );
    let after = selected::finalize_with_definitions(
        &xml,
        &new,
        &out.join(format!("selected-{case:02}.json")),
        &package.join("schema.json"),
    );
    for r in [&before, &after] {
        assert_eq!(r["intent_validation"]["schema_issues"], json!([]));
    }
    let mut issues = after["finalization"]["issues"].clone();
    identity::relocate(&mut issues, &ids);
    assert_eq!(issues, before["finalization"]["issues"]);
    assert_eq!(rows(&issues).len(), [107, 117, 109, 123, 5][case - 1]);
    json!({"original":case,"participation_preferences_added":added,"selected_pending":rows(&issues).len(),"complete_import_inverse":true,"calculation":"not_run"})
}

fn edit(xml: &str, ordinal: usize, value: Option<&str>) -> String {
    let input = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&input, SourceEvidenceLimits::default()).unwrap();
    let row = &evidence.rows()[ordinal];
    let start = row.occurrence().range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let header = &xml[start..end];
    assert_eq!(row.attribute("enabled").unwrap().decoded().unwrap(), "true");
    assert_eq!(header.matches("enabled=\"true\"").count(), 1);
    let replacement = value
        .map(|v| format!("enabled=\"{v}\""))
        .unwrap_or_default();
    let mut changed = xml.to_owned();
    changed.replace_range(
        start..end,
        &header.replace("enabled=\"true\"", &replacement),
    );
    changed
}

pub fn controls(next: &StagedOwnedRelease, package: &Path, out: &Path) -> Vec<Value> {
    let evidence = packet();
    let census = &evidence["original_inventory"][4];
    let xml = fs::read_to_string(root().join(census["fixture"].as_str().unwrap())).unwrap();
    let mut results = Vec::new();
    for gem in [209, 216, 244] {
        let row = rows(&census["occurrences"])
            .iter()
            .find(|r| r["gem_ordinal"] == gem)
            .unwrap();
        for (label, group, value) in [
            ("group-false", true, Some("false")),
            ("occurrence-false", false, Some("false")),
            ("group-missing", true, None),
            ("occurrence-missing", false, None),
            ("group-malformed", true, Some("TRUE")),
            ("occurrence-malformed", false, Some("TRUE")),
        ] {
            let ordinal = row[if group {
                "group_ordinal"
            } else {
                "gem_ordinal"
            }]
            .as_u64()
            .unwrap() as usize;
            let name = format!("control-{gem}-{label}");
            let changed = edit(&xml, ordinal, value);
            let path = out.join(format!("{name}.xml"));
            fs::write(&path, changed).unwrap();
            let directory = out.join(&name);
            release::normalize(package, &path, 5, &directory);
            let draft = read(directory.join("draft.json"));
            let side = read(directory.join("sidecar.json"));
            preservation::authenticate(&side, &directory, package, 5, next);
            let wanted = target(&draft, &side, row);
            let id = preservation::link(
                &side,
                row["skill_set_ordinal"].as_u64().unwrap() as usize,
                "skill_preset",
            );
            let p = preservation::member(&draft, "skill_presets", &id);
            let found: Vec<_> = rows(&p["intent"]["usage"]["members"])
                .iter()
                .filter(|r| {
                    r["selection"]["policy"]["value"]["key"] == "def.000000000000332b"
                        && r["selection"]["target"] == wanted
                })
                .collect();
            if value == Some("false") {
                assert_eq!(found.len(), 1);
                assert_eq!(
                    *found[0],
                    expected(wanted, row["kind"] == "manual", !group, group)
                );
            } else {
                assert_eq!(
                    found.len(),
                    1,
                    "retain the exact target with unresolved input"
                );
                let parameters = &found[0]["selection"]["parameters"];
                assert_eq!(parameters["completion"]["kind"], "pending");
                assert_eq!(
                    parameters["completion"]["code"],
                    "usage-parameters-not-converted"
                );
                let admitted = expected(wanted, row["kind"] == "manual", true, true);
                assert_eq!(
                    parameters["members"],
                    json!([admitted["selection"]["parameters"]["members"][usize::from(group)]]),
                    "only the other independent Boolean survives"
                );
            }
            // Other exact occurrences keep independent true/true preferences.
            for other in rows(&census["occurrences"])
                .iter()
                .filter(|r| r["skill_set"] == "4" && r["gem_ordinal"] != gem)
            {
                let t = target(&draft, &side, other);
                let expected = expected(t, other["kind"] == "manual", true, true);
                assert_eq!(
                    rows(&p["intent"]["usage"]["members"])
                        .iter()
                        .filter(|r| **r == expected)
                        .count(),
                    1
                );
            }
            assert_eq!(p["intent"]["usage"]["completion"]["kind"], "pending");
            results.push(json!({"name":name,"source_ordinal":ordinal,"known_false":value==Some("false"),"other_occurrences_independent":true}));
        }
    }
    results
}
