//! Actual saved-build flat Life source admission, not whole-build numerical coverage.
#[path = "support/owned_flat_life.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_family_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaState};
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
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
}
fn publish(input: &Path, out: &Path) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn row(sidecar: &Value, ordinal: u64) -> &Value {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"]["ordinal"] == ordinal)
        .unwrap()
}
#[test]
fn authored_family_keeps_finite_inputs_and_conservative_source_scope() {
    let b = family::bindings();
    let e = family::extension();
    assert_eq!(b.amount.slot.key().as_str(), "def.0000000000003101");
    assert_eq!(b.properties.len(), 21);
    assert_eq!(b.corrupted_base.slot.key().as_str(), "def.0000000000003117");
    assert_eq!(b.effective.key().as_str(), "def.000000000000295b");
    assert_eq!(e.schema.len(), 27);
    assert_eq!(b.category.slot.key().as_str(), "def.0000000000003118");
    assert_eq!(b.contribution.key().as_str(), "def.000000000000311a");
    assert_eq!(b.templates.len(), 22);
    let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
        DefinitionDescriptor::Modifier(m),
    ) = &e.schema[0]
    else {
        panic!()
    };
    let SchemaState::Known(m) = &m.schema else {
        panic!()
    };
    assert_eq!(m.declarations.parameters.members.len(), 24);
    assert_eq!(m.declarations.parameters.closure, SchemaClosure::Complete);
    assert!(matches!(
        e.owners[0].programs.closure,
        SchemaClosure::Partial { .. }
    ));
    let rule = serde_json::to_value(family::item_rule()).unwrap();
    assert_eq!(
        rule["emissions"][0]["value"]["rolls"]
            .as_array()
            .unwrap()
            .len(),
        24
    );
    assert_eq!(
        rule["emissions"][0]["value"]["rolls"][23]["value"],
        json!({"kind":"context_option","value":{"input":"modifier-source-category"}})
    );
    let condition = serde_json::to_value(family::source_condition()).unwrap();
    assert_eq!(
        condition["all"],
        json!([{"kind":"no_source_scaling_tags"},{"kind":"initial_scaling_is_one"},{"kind":"no_generated_buff_members"},{"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}])
    );
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let m: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(m["upstream_revision"], a["source_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert!(m["files"].as_array().unwrap().contains(pin));
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
    assert_ne!(b.unit, b.contribution_unit);
    assert_eq!(b.unit.key().as_str(), "def.000000000000295a");
    assert_eq!(b.contribution_unit.key().as_str(), "def.0000000000003119");
    assert_eq!(family::item_rule().id, family::prior_item_rule().id);
    assert_eq!(
        family::source_condition().rule,
        family::prior_source_condition().rule
    );
    assert_eq!(
        serde_json::to_value(family::prior_item_rule()).unwrap()["emissions"][0]["value"]["definition"]
            ["key"],
        "def.00000000000009da"
    );
}

fn modifier<'a>(draft: &'a Value, id: &Value) -> &'a Value {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| v["modifiers"]["members"].as_array().unwrap())
        .find(|v| &v["id"] == id)
        .unwrap()
}

fn check_rolls(m: &Value, amount: f64, category: &str) {
    assert_eq!(m["definition"]["value"]["key"], "def.0000000000003100");
    assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
    let r = m["rolls"]["members"].as_array().unwrap();
    assert_eq!(r.len(), 24);
    assert_eq!(
        r[0]["value"]["value"]["value"],
        json!({"value":amount,"unit":family::bindings().unit})
    );
    for flag in &r[1..22] {
        assert_eq!(
            flag["value"],
            json!({"kind":"known","value":{"kind":"boolean","value":false}})
        );
    }
    assert_eq!(
        r[22]["value"]["value"]["value"],
        json!({"value":1.0,"unit":family::bindings().factor_unit})
    );
    assert_eq!(r[23]["value"]["value"]["value"]["key"], category);
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    if matches!(case, 1 | 3 | 4) {
        let mut before = a.clone();
        let mut after = b.clone();
        selected::canonical(&mut before);
        selected::canonical(&mut after);
        assert!(
            before == after,
            "unaffected original {case} canonical input changed"
        );
    }
    let expected: &[(u64, u64, f64)] = match case {
        2 => &[(501, 19, 69.0), (506, 21, 25.0)],
        5 => &[(572, 20, 17.0), (574, 20, 16.0), (587, 14, 10.0)],
        _ => &[],
    };
    let mut new_ids = vec![];
    let mut old_ids = vec![];
    let mut census = vec![];
    let mut retained_pairs = vec![];
    let originals = sa["item_texts"].as_array().unwrap();
    let successors = sb["item_texts"].as_array().unwrap();
    assert_eq!(originals.len(), successors.len());
    for (x, y) in originals.iter().zip(successors) {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(x["content_entry"], y["content_entry"]);
        assert_eq!(x["defaults"], y["defaults"], "no item default changes");
        let before_lines = x["lines"].as_array().unwrap();
        let after_lines = y["lines"].as_array().unwrap();
        assert_eq!(before_lines.len(), after_lines.len());
        for (p, q) in before_lines.iter().zip(after_lines) {
            assert_eq!(p["index"], q["index"]);
            assert_eq!(p["text"], q["text"]);
            let before = p["modifiers"].as_array().unwrap();
            let after = q["modifiers"].as_array().unwrap();
            let admitted = expected
                .iter()
                .find(|(o, l, _)| y["source"]["ordinal"] == *o && q["index"] == *l);
            if let Some((ordinal, _, amount)) = admitted {
                assert_eq!(after.len(), 1);
                assert_eq!(q["outcome"]["kind"], "known");
                assert_eq!(y["attribution"]["layout"]["status"], "proven");
                check_rolls(modifier(&b, &after[0]), *amount, "def.00000000000030e2");
                new_ids.push(after[0].clone());
                if case == 5 && *ordinal == 587 {
                    assert_eq!(before.len(), 1);
                    let legacy = modifier(&a, &before[0]);
                    assert_eq!(legacy["definition"]["value"]["key"], "def.00000000000009da");
                    assert_eq!(legacy["rolls"]["members"].as_array().unwrap().len(), 1);
                    assert_eq!(
                        legacy["rolls"]["members"][0]["value"]["value"],
                        json!({"kind":"integer","value":10})
                    );
                    assert_eq!(legacy["rolls"]["completion"], json!({"kind":"complete"}));
                    old_ids.push(before[0].clone());
                } else {
                    assert!(before.is_empty());
                }
            } else {
                assert_eq!(
                    before.len(),
                    after.len(),
                    "unexpected member change {case}/{}/{}",
                    y["source"]["ordinal"],
                    q["index"]
                );
                retained_pairs.extend(before.iter().cloned().zip(after.iter().cloned()));
            }
            let text = q["text"].as_str().unwrap();
            if text
                .strip_prefix('+')
                .and_then(|v| v.strip_suffix(" to maximum Life"))
                .is_some_and(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()))
            {
                if admitted.is_none() {
                    assert!(
                        after.is_empty(),
                        "unproved category cannot create canonical Life"
                    );
                    assert_eq!(q["outcome"]["kind"], "pending");
                }
                census.push(json!({"source":y["source"],"line":q["index"],"raw":q["text"],"before":p["outcome"],"after":q["outcome"],"layout":y["attribution"]["layout"],"old_modifiers":before,"new_modifiers":after}));
            }
        }
    }
    assert_eq!(new_ids.len(), expected.len());
    assert_eq!(old_ids.len(), usize::from(case == 5));
    assert_eq!(census.len(), [1, 8, 1, 7, 7][case - 1]);
    let mut before_draft = a["draft"].clone();
    let mut after_draft = b["draft"].clone();
    after_draft["allocator"] = before_draft["allocator"].clone();
    let mut removed = [0, 0];
    for (side, draft, ids) in [
        (0, &mut before_draft, &old_ids),
        (1, &mut after_draft, &new_ids),
    ] {
        for item in draft["items"]["members"].as_array_mut().unwrap() {
            item["modifiers"]["members"]
                .as_array_mut()
                .unwrap()
                .retain(|m| {
                    if ids.contains(&m["id"]) {
                        removed[side] += 1;
                        false
                    } else {
                        true
                    }
                });
        }
    }
    assert_eq!(removed, [old_ids.len(), new_ids.len()]);
    let mut ids = BTreeMap::new();
    correspond(
        &before_draft,
        &mut after_draft,
        &mut ids,
        "all unaffected canonical inputs",
    );
    for (old, mut new) in retained_pairs {
        relocate(&mut new, &ids);
        assert_eq!(old, new);
    }
    let mut before_origins = sa["origins"].clone();
    let mut after_origins = sb["origins"].clone();
    for (origins, changed) in [
        (&mut before_origins, &old_ids),
        (&mut after_origins, &new_ids),
    ] {
        for r in origins.as_array_mut().unwrap() {
            r["links"]
                .as_array_mut()
                .unwrap()
                .retain(|v| !(v["kind"] == "modifier" && changed.contains(&v["value"])));
        }
    }
    correspond(
        &before_origins,
        &mut after_origins,
        &mut ids,
        "all unaffected source links including issues",
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
    let mut actual = after["finalization"]["issues"].clone();
    relocate(&mut actual, &ids);
    let expected_issues = before["finalization"]["issues"].as_array().unwrap();
    assert_eq!(actual.as_array().unwrap().len(), expected_issues.len());
    let mut paths = BTreeMap::new();
    for (i, item) in b["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for (j, m) in item["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if new_ids.contains(&m["id"]) {
                continue;
            }
            let mut id = m["id"].clone();
            relocate(&mut id, &ids);
            let old_index = a["draft"]["items"]["members"][i]["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .position(|v| v["id"] == id)
                .unwrap();
            paths.insert(
                format!("items.members[{i}].modifiers.members[{old_index}]."),
                format!("items.members[{i}].modifiers.members[{j}]."),
            );
        }
    }
    let mut changed_paths = vec![];
    for old in expected_issues {
        let new = actual
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["id"] == old["id"])
            .unwrap();
        if old["path"] != new["path"] {
            let path = old["path"].as_str().unwrap();
            let (from, to) = paths
                .iter()
                .find(|(p, _)| path.starts_with(p.as_str()))
                .unwrap();
            assert_eq!(new["path"], format!("{to}{}", &path[from.len()..]));
            changed_paths.push(json!({"id":old["id"],"before":old["path"],"after":new["path"]}));
            new["path"] = old["path"].clone();
        }
    }
    assert!(
        actual == before["finalization"]["issues"],
        "other selected issue content changed in original {case}"
    );
    let mut selection = selected::selection(xml, new);
    relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    json!({"original":case,"selected_before":expected_issues.len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"source_census":census,"new_modifiers":new_ids,"replaced_modifiers":old_ids,"relocated_issue_paths":changed_paths,"allocator_before":a["draft"]["allocator"],"allocator_after":b["draft"]["allocator"],"selected_issue_summary":after["selected_issue_summary"]})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"19\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let cases = [
        ("zero", "+0 to maximum Life", true, 0.0),
        ("bare", "17 to maximum Life", false, 0.0),
        ("fractional", "+17.5 to maximum Life", false, 0.0),
        ("negative", "-17 to maximum Life", false, 0.0),
        ("tagged", "{tags:life}+17 to maximum Life", false, 0.0),
        ("desecrated", "{desecrated}+17 to maximum Life", false, 0.0),
        (
            "corruption",
            "{corruptedRange:1.5}+17 to maximum Life",
            false,
            0.0,
        ),
        ("range", "+(17-17) to maximum Life", false, 0.0),
        (
            "unknown-prefix",
            "Unknown source semantics\n+17 to maximum Life",
            false,
            0.0,
        ),
        ("implicit", "+17 to maximum Life", true, 17.0),
        ("enchant", "{enchant}+17 to maximum Life", true, 17.0),
    ];
    for (label, text, admitted, amount) in cases {
        let mut changed = raw.replace("+17 to maximum Life", text);
        if label == "implicit" {
            changed = changed.replace("Implicits: 0", "Implicits: 1");
        }
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let s: Value = read(dest.join("sidecar.json"));
        let r = row(&s, 572);
        let l = r["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["text"].as_str().unwrap().contains("to maximum Life"))
            .unwrap();
        assert_eq!(
            l["modifiers"].as_array().unwrap().len(),
            usize::from(admitted),
            "{label}"
        );
        if admitted {
            let d: Value = read(dest.join("draft.json"));
            check_rolls(
                modifier(&d, &l["modifiers"][0]),
                amount,
                match label {
                    "implicit" => "def.00000000000030e3",
                    "enchant" => "def.00000000000030e4",
                    _ => "def.00000000000030e2",
                },
            );
        } else {
            assert_eq!(r["attribution"]["layout"]["status"], "pending", "{label}");
        }
    }
    cases.len()
}

#[test]
#[ignore = "requires exact movement-speed publication predecessor"]
fn real_life_family_preserves_originals_and_replaces_only_reviewed_occurrences() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FLAT_LIFE_PRIOR")
            .expect("explicit checked predecessor"),
    );
    let inventory = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let b = family::bindings();
    preservation::check(
        &prior,
        &next,
        preservation::FamilyChange {
            modifier: &b.modifier,
            templates: &b.templates,
            dependencies: &family::dependency_definitions(),
            extension: &family::extension(),
            last_issued: 0x311a,
            rule: &family::item_rule(),
            condition: &family::source_condition(),
            default: None,
            rule_change: preservation::RuleChange::ReplaceExisting {
                prior_rule: &family::prior_item_rule(),
                prior_condition: &family::prior_source_condition(),
            },
        },
    );
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_FLAT_LIFE_OUTPUT")
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
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    let mut reports = vec![];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(p.join(&query)).unwrap(),
            fs::read(package.join(query)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    let count = probes(&package, &out);
    assert_eq!(inventory, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"provenance":next.input().provenance.len(),"queries":110,"originals":reports,"probes":count,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
