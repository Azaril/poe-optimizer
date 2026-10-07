//! Exact import inverses and per-source Boolean controls for the current packet.
use crate::{family, identity, preservation, release, selected};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

fn rows(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn usage(p: &Value) -> &Value {
    if p.get("intent").is_some() {
        &p["intent"]["usage"]
    } else {
        &p["usage_preferences"]
    }
}
fn usage_mut(p: &mut Value) -> &mut Value {
    if p.get("intent").is_some() {
        &mut p["intent"]["usage"]
    } else {
        &mut p["usage_preferences"]
    }
}
fn selection(row: &Value) -> &Value {
    row.get("selection").unwrap_or(row)
}
fn target(draft: &Value, side: &Value, row: &Value) -> Option<Value> {
    let ordinal = row["gem_ordinal"].as_u64().unwrap() as usize;
    let id = preservation::link(
        side,
        row["skill_set_ordinal"].as_u64().unwrap() as usize,
        "skill_preset",
    );
    let preset = preservation::member(draft, "skill_presets", &id);
    if row["group_attributes"].get("source").is_some() {
        // The source occurrence's authenticated generated-input link binds the
        // provider. A same-definition count elsewhere in the preset is not
        // evidence that this saved occurrence resolved.
        let links: Vec<_> = rows(&preservation::origin(side, ordinal)["links"])
            .iter()
            .filter(|l| l["kind"] == "generated_skill_input")
            .collect();
        assert!(links.len() <= 1, "one exact generated input correspondence");
        let link = links.first()?;
        assert_eq!(link["value"]["skill_preset"], id);
        let source = &link["value"]["target"];
        let c = family::packet("changes.json");
        let correspondence = &c["occurrences"][1]["after"]["target"]["correspondence"];
        assert_eq!(source["slot"], correspondence["provider"]["skill_supply"]);
        assert_eq!(source["provider"]["root"]["kind"], "allocation");
        assert_eq!(source["provider"]["grant_path"], json!([]));
        let target = json!({"kind":"skill","value":{"kind":"generated","value":{
            "provider":{"root":{"kind":"allocation","value":{"kind":"known","value":source["provider"]["root"]["value"]}},
                "grant_path":{"members":[],"completion":{"kind":"complete"}}},
            "slot":{"kind":"known","value":source["slot"]}
        }}});
        assert_eq!(
            rows(&usage(preset)["members"])
                .iter()
                .map(selection)
                .filter(|r| r["policy"]["value"]["key"] == "def.000000000000326a"
                    && r["target"] == target)
                .count(),
            1,
            "existing count agrees with this exact resolved provider"
        );
        return Some(target);
    }
    let skill = preservation::link(side, ordinal, "skill");
    let c = family::packet("changes.json");
    if let Some(p) = rows(&c["physical"]).iter().map(|r| &r["after"]).find(|p| {
        p["game_id"] == row["gem_attributes"]["gemId"]
            && p["variant_id"] == row["gem_attributes"]["variantId"]
    }) {
        Some(json!({"kind":"skill","value":{"kind":"generated","value":{
            "provider":{"root":{"kind":"skill_use","value":{"kind":"known","value":skill}},"grant_path":{"members":[],"completion":{"kind":"complete"}}},
            "slot":{"kind":"known","value":p["supply"]}
        }}}))
    } else {
        Some(
            json!({"kind":"skill","value":{"kind":"authored","value":{"kind":"known","value":skill}}}),
        )
    }
}
fn expected(target: Value, group: bool, occurrence: bool) -> Value {
    let p = family::packet("changes.json")["usage"].clone();
    json!({"policy":{"kind":"known","value":p["policy"]},"target":target,
        "parameters":{"members":[
            {"slot":{"kind":"known","value":p["parameters"][0]["slot"]},"value":{"kind":"known","value":{"kind":"boolean","value":group}}},
            {"slot":{"kind":"known","value":p["parameters"][1]["slot"]},"value":{"kind":"known","value":{"kind":"boolean","value":occurrence}}}
        ],"completion":{"kind":"complete"}}})
}
fn preset_id(side: &Value, row: &Value) -> Value {
    preservation::link(
        side,
        row["skill_set_ordinal"].as_u64().unwrap() as usize,
        "skill_preset",
    )
}
fn applicable(row: &Value) -> &'static str {
    if row["group_attributes"].get("source").is_some() {
        "when_exact_source_selected"
    } else {
        "required"
    }
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
pub fn census() {
    let c = family::packet("changes.json");
    let identities: Vec<_> = rows(&c["physical"])
        .iter()
        .map(|r| (&r["after"]["game_id"], &r["after"]["variant_id"]))
        .chain(rows(&c["occurrences"]).iter().map(|r| {
            let t = &r["after"]["target"];
            let t = t.get("correspondence").unwrap_or(t);
            (&t["game_id"], &t["variant_id"])
        }))
        .collect();
    for case in rows(&family::packet("evidence.json")["original_inventory"]) {
        let bytes =
            fs::read(family::root().join(case["fixture"]["path"].as_str().unwrap())).unwrap();
        let input = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([93; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let e = SourceProjectEvidence::collect(&input, SourceEvidenceLimits::default()).unwrap();
        let active = e
            .rows()
            .iter()
            .find(|r| r.occurrence().name() == "Skills")
            .unwrap()
            .attribute("activeSkillSet")
            .unwrap()
            .decoded()
            .unwrap();
        let mut found = vec![];
        for row in e.rows().iter().filter(|r| r.occurrence().name() == "Gem") {
            let Some(game) = row.attribute("gemId").and_then(|a| a.decoded().ok()) else {
                continue;
            };
            let Some(variant) = row.attribute("variantId").and_then(|a| a.decoded().ok()) else {
                continue;
            };
            if !identities
                .iter()
                .any(|(g, v)| **g == game && **v == variant)
            {
                continue;
            }
            let group = &e.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
            let set = &e.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
            let set_id = set.attribute("id").unwrap().decoded().unwrap();
            let attrs = |r: &poe_optimizer_import::owned_source::SourceEvidenceRow<'_>| -> BTreeMap<String,String> {
                r.attributes().iter().map(|a| (a.origin().name.clone(),a.decoded().unwrap().to_owned())).collect()
            };
            found.push(json!({"gem_ordinal":row.occurrence().id().ordinal(),"group_ordinal":group.occurrence().id().ordinal(),
                "skill_set_ordinal":set.occurrence().id().ordinal(),"skill_set":set_id,"selected":set_id==active,
                "gem_attributes":attrs(row),"group_attributes":attrs(group)}));
        }
        assert_eq!(json!(found), case["occurrences"]);
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
    let a = family::read(old.join("draft.json"));
    let mut b = family::read(new.join("draft.json"));
    let sa = family::read(old.join("sidecar.json"));
    let mut sb = family::read(new.join("sidecar.json"));
    preservation::authenticate(&sa, &old, prior_path, case, prior);
    preservation::authenticate(&sb, &new, package, case, next);
    let e = family::packet("evidence.json");
    let inventory = &e["original_inventory"][case - 1];
    let mut added = 0;
    let mut selected_added = 0;
    let mut unresolved = vec![];
    for row in rows(&inventory["occurrences"]) {
        let Some(t) = target(&b, &sb, row) else {
            unresolved.push(row["gem_ordinal"].clone());
            continue;
        };
        let id = preset_id(&sb, row);
        let wanted = expected(t, true, true);
        let preset = b["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["id"] == id)
            .unwrap();
        let actual = usage_mut(preset)["members"].as_array_mut().unwrap();
        let positions: Vec<_> = actual
            .iter()
            .enumerate()
            .filter(|(_, r)| *selection(r) == wanted)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            positions.len(),
            1,
            "one added participation for original{case} ordinal{}",
            row["gem_ordinal"]
        );
        let removed = actual.remove(positions[0]);
        if removed.get("selection").is_some() {
            assert_eq!(removed["applicability"], applicable(row));
        }
        added += 1;
        selected_added += usize::from(row["selected"] == true);
        // New source-to-preset links are permitted only for these actual emitted
        // selections. Existing count/global-policy links are preserved exactly.
        let old_id = preset_id(&sa, row);
        for field in ["gem_ordinal", "group_ordinal"] {
            let ordinal = row[field].as_u64().unwrap() as usize;
            let old_link = json!({"kind":"skill_preset","value":old_id});
            if !rows(&preservation::origin(&sa, ordinal)["links"]).contains(&old_link) {
                let new_link = json!({"kind":"skill_preset","value":id});
                let links = preservation::origin_mut(&mut sb, ordinal)["links"]
                    .as_array_mut()
                    .unwrap();
                if let Some(i) = links.iter().position(|l| *l == new_link) {
                    links.remove(i);
                }
            }
        }
    }
    if case == 5 {
        assert_eq!(selected_added, 8);
    }
    let oldlineage = a["draft"]["allocator"]["lineage"].clone();
    let newlineage = b["draft"]["allocator"]["lineage"].clone();
    assert_eq!(
        a["draft"]["allocator"]["last_issued"],
        b["draft"]["allocator"]["last_issued"]
    );
    b["draft"]["allocator"] = a["draft"]["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "whole occurrence participation draft inverse",
    );
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
        "whole occurrence participation sidecar inverse",
    );
    let xml =
        fs::read(family::root().join(inventory["fixture"]["path"].as_str().unwrap())).unwrap();
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
    json!({"original":case,"added":added,"selected_added":selected_added,"unresolved_generated_ordinals":unresolved,
        "whole_import_inverse":true,"selected_issues":rows(&issues).len(),"calculation":"not_run"})
}
fn edit(xml: &str, ordinal: usize, value: Option<&str>) -> String {
    let input = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&input, SourceEvidenceLimits::default()).unwrap();
    let row = &e.rows()[ordinal];
    assert_eq!(row.attribute("enabled").unwrap().decoded().unwrap(), "true");
    let start = row.occurrence().range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let header = &xml[start..end];
    assert_eq!(header.matches("enabled=\"true\"").count(), 1);
    let mut result = xml.to_owned();
    result.replace_range(
        start..end,
        &header.replace(
            "enabled=\"true\"",
            &value
                .map(|v| format!("enabled=\"{v}\""))
                .unwrap_or_default(),
        ),
    );
    result
}
pub fn controls(next: &StagedOwnedRelease, package: &Path, out: &Path) -> Vec<Value> {
    let e = family::packet("evidence.json");
    let c = &e["original_inventory"][4];
    let xml =
        fs::read_to_string(family::root().join(c["fixture"]["path"].as_str().unwrap())).unwrap();
    let selected_rows: Vec<_> = rows(&c["occurrences"])
        .iter()
        .filter(|r| r["selected"] == true)
        .collect();
    assert_eq!(selected_rows.len(), 8);
    let mut results = vec![];
    for row in &selected_rows {
        for (label, group, value) in [
            ("group-false", true, Some("false")),
            ("gem-false", false, Some("false")),
            ("group-missing", true, None),
            ("gem-missing", false, None),
            ("group-malformed", true, Some("TRUE")),
            ("gem-malformed", false, Some("TRUE")),
        ] {
            let ordinal = row[if group {
                "group_ordinal"
            } else {
                "gem_ordinal"
            }]
            .as_u64()
            .unwrap() as usize;
            let name = format!("control-{}-{label}", row["gem_ordinal"]);
            let path = out.join(format!("{name}.xml"));
            fs::write(&path, edit(&xml, ordinal, value)).unwrap();
            let dir = out.join(&name);
            release::normalize(package, &path, 5, &dir);
            let draft = family::read(dir.join("draft.json"));
            let side = family::read(dir.join("sidecar.json"));
            preservation::authenticate(&side, &dir, package, 5, next);
            let t = target(&draft, &side, row).unwrap();
            let id = preset_id(&side, row);
            let p = preservation::member(&draft, "skill_presets", &id);
            let found: Vec<_> = rows(&usage(p)["members"])
                .iter()
                .map(selection)
                .filter(|r| {
                    r["policy"]["value"]["key"] == "def.000000000000332b" && r["target"] == t
                })
                .collect();
            if value == Some("false") {
                assert_eq!(found, vec![&expected(t, !group, group)]);
            } else if let [r] = found.as_slice() {
                assert_eq!(r["parameters"]["completion"]["kind"], "pending");
                assert_eq!(
                    r["parameters"]["completion"]["code"],
                    "usage-parameters-not-converted"
                );
                assert_eq!(
                    r["parameters"]["members"],
                    json!([expected(t, true, true)["parameters"]["members"][usize::from(group)]])
                );
            } else {
                assert!(found.is_empty());
                assert!(
                    group && [206, 213, 224].contains(&row["gem_ordinal"].as_u64().unwrap()),
                    "only existing strict minion group guard can refuse the row"
                );
            }
            for other in &selected_rows {
                if other["gem_ordinal"] == row["gem_ordinal"] {
                    continue;
                }
                let want = expected(target(&draft, &side, other).unwrap(), true, true);
                assert_eq!(
                    rows(&usage(p)["members"])
                        .iter()
                        .map(selection)
                        .filter(|r| **r == want)
                        .count(),
                    1,
                    "other exact occurrences stay independent"
                );
            }
            assert_eq!(usage(p)["completion"]["kind"], "pending");
            results.push(json!({"name":name,"false_preserved":value==Some("false"),"independent_occurrences":true,"inventory_closed":false}));
        }
    }
    // These physical recipes share one admitted source frame across their
    // policies. An unknown field conservatively refuses the old global policy
    // together with participation; it cannot silently create a known value.
    for gem in [221_u64, 234] {
        let row = selected_rows
            .iter()
            .find(|r| r["gem_ordinal"] == gem)
            .unwrap();
        for (label, field) in [
            ("unknown-group-field", "group_ordinal"),
            ("unknown-gem-field", "gem_ordinal"),
        ] {
            let ordinal = row[field].as_u64().unwrap() as usize;
            let input = ImportedBuildInstance::from_decoded(
                decode_build(xml.as_bytes()).unwrap(),
                BuildLineage::from_bytes([93; 16]),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let evidence =
                SourceProjectEvidence::collect(&input, SourceEvidenceLimits::default()).unwrap();
            let start = evidence.rows()[ordinal].occurrence().range().start;
            let insert = start + xml[start..].find('>').unwrap();
            let insert = if xml.as_bytes()[insert - 1] == b'/' {
                insert - 1
            } else {
                insert
            };
            let mut control = xml.clone();
            control.insert_str(insert, " unknownUsageControl=\"true\"");
            let name = format!("control-{gem}-{label}");
            let path = out.join(format!("{name}.xml"));
            fs::write(&path, control).unwrap();
            let dir = out.join(&name);
            release::normalize(package, &path, 5, &dir);
            let draft = family::read(dir.join("draft.json"));
            let side = family::read(dir.join("sidecar.json"));
            preservation::authenticate(&side, &dir, package, 5, next);
            let refused_target = target(&draft, &side, row).unwrap();
            let id = preset_id(&side, row);
            let preset = preservation::member(&draft, "skill_presets", &id);
            assert_eq!(
                rows(&usage(preset)["members"])
                    .iter()
                    .map(selection)
                    .filter(|r| r["target"] == refused_target)
                    .count(),
                0,
                "unknown shared frame refuses all policies for only this exact occurrence"
            );
            assert_eq!(usage(preset)["completion"]["kind"], "pending");
            for other in &selected_rows {
                if other["gem_ordinal"] == gem {
                    continue;
                }
                let want = expected(target(&draft, &side, other).unwrap(), true, true);
                assert_eq!(
                    rows(&usage(preset)["members"])
                        .iter()
                        .map(selection)
                        .filter(|r| **r == want)
                        .count(),
                    1
                );
            }
            results.push(json!({"name":name,"shared_frame_refused":true,"other_occurrences_preserved":true,"inventory_closed":false}));
        }
    }
    results
}
