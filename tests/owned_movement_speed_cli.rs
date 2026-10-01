//! Actual saved-build movement source admission, not whole-build numerical coverage.
#[path = "support/owned_movement_speed.rs"]
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
    assert_eq!(b.amount.slot.key().as_str(), "def.00000000000030e7");
    assert_eq!(b.properties.len(), 21);
    assert_eq!(b.corrupted_base.slot.key().as_str(), "def.00000000000030fd");
    assert_eq!(b.effective.key().as_str(), "def.000000000000253e");
    assert_eq!(e.schema.len(), 26);
    assert_eq!(b.category.slot.key().as_str(), "def.00000000000030fe");
    assert_eq!(b.contribution.key().as_str(), "def.00000000000030ff");
    assert_eq!(b.templates.len(), 6);
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
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    if case == 2 {
        let mut before = a.clone();
        let mut after = b.clone();
        selected::canonical(&mut before);
        selected::canonical(&mut after);
        assert!(
            before == after,
            "original 02 canonical input and local IDs changed"
        );
    }
    let expected_addition = match case {
        1 => Some((298, 17, "def.0000000000002a33")),
        3 => Some((295, 16, "def.0000000000002a1b")),
        4 => Some((211, 16, "def.0000000000002a03")),
        5 => Some((578, 20, "def.00000000000030e6")),
        _ => None,
    };
    let mut added = vec![];
    let mut census = vec![];
    let mut retained_pairs = vec![];
    assert_eq!(
        sa["item_texts"].as_array().unwrap().len(),
        sb["item_texts"].as_array().unwrap().len()
    );
    for (x, y) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array().unwrap())
    {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(x["content_entry"], y["content_entry"]);
        assert_eq!(
            x["lines"].as_array().unwrap().len(),
            y["lines"].as_array().unwrap().len()
        );
        for (p, q) in x["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(y["lines"].as_array().unwrap())
        {
            assert_eq!(p["index"], q["index"]);
            assert_eq!(p["text"], q["text"]);
            let before = p["modifiers"].as_array().unwrap();
            let after = q["modifiers"].as_array().unwrap();
            if expected_addition
                .is_some_and(|(o, l, _)| y["source"]["ordinal"] == o && q["index"] == l)
            {
                assert!(before.is_empty());
                assert_eq!(after.len(), 1);
                assert_eq!(q["outcome"]["kind"], "known");
                added.push(after[0].clone());
                if case == 5 {
                    assert_eq!(y["attribution"]["layout"]["status"], "proven");
                    assert_eq!(y["defaults"]["item_level_absent"], true);
                } else {
                    assert_eq!(y["attribution"]["layout"]["status"], "pending");
                }
            } else {
                assert_eq!(
                    before.len(),
                    after.len(),
                    "unexpected source admission {case}/{}/{index}",
                    y["source"]["ordinal"],
                    index = q["index"]
                );
                retained_pairs.extend(before.iter().cloned().zip(after.iter().cloned()));
            }
            if q["text"]
                .as_str()
                .unwrap()
                .contains("increased Movement Speed")
            {
                census.push(json!({"source":y["source"],"line":q["index"],"raw":q["text"],"outcome":q["outcome"],"layout":y["attribution"]["layout"],"modifiers":q["modifiers"]}));
                if !(case == 5 && y["source"]["ordinal"] == 578) {
                    assert_eq!(q["outcome"]["kind"], "pending");
                    assert_eq!(y["attribution"]["layout"]["status"], "pending");
                }
            }
        }
    }
    assert_eq!(added.len(), usize::from(expected_addition.is_some()));
    assert_eq!(census.len(), if matches!(case, 2 | 5) { 2 } else { 1 });
    let mut restored = b["draft"].clone();
    restored["allocator"] = a["draft"]["allocator"].clone();
    let mut retired = vec![];
    let mut canonical_added = vec![];
    let mut fresh_issues = vec![];
    for (i, item) in restored["items"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        item["modifiers"]["members"]
            .as_array_mut()
            .unwrap()
            .retain(|m| {
                if !added.contains(&m["id"]) {
                    return true;
                }
                assert_eq!(
                    m["definition"]["value"]["key"],
                    expected_addition.unwrap().2
                );
                if case == 5 {
                    assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
                    assert_eq!(m["rolls"]["members"].as_array().unwrap().len(), 24);
                    assert_eq!(
                        m["rolls"]["members"][0]["value"]["value"]["value"],
                        json!({"value":10.0,"unit":family::bindings().unit})
                    );
                    assert_eq!(
                        m["rolls"]["members"][23]["value"]["value"]["value"]["key"],
                        "def.00000000000030e2"
                    );
                } else {
                    let (amount, slot) = match case {
                        1 => (44.0, "def.0000000000002a34"),
                        3 => (134.0, "def.0000000000002a1c"),
                        4 => (77.0, "def.0000000000002a04"),
                        _ => unreachable!(),
                    };
                    assert_eq!(m["rolls"]["members"].as_array().unwrap().len(), 23);
                    let raw = &m["rolls"]["members"][0];
                    assert_eq!(raw["slot"]["value"]["slot"]["key"], slot);
                    assert_eq!(raw["value"]["kind"], "known");
                    assert_eq!(raw["value"]["value"]["kind"], "quantity");
                    assert_eq!(raw["value"]["value"]["value"]["value"], amount);
                    assert_eq!(
                        raw["value"]["value"]["value"]["unit"]["key"],
                        "def.000000000000295a"
                    );
                    for flag in &m["rolls"]["members"].as_array().unwrap()[1..22] {
                        assert_eq!(
                            flag["value"],
                            json!({"kind":"known","value":{"kind":"boolean","value":false}})
                        );
                    }
                    let factor = &m["rolls"]["members"][22]["value"];
                    assert_eq!(factor["kind"], "known");
                    assert_eq!(factor["value"]["kind"], "quantity");
                    assert_eq!(factor["value"]["value"]["value"], 1.0);
                    assert_eq!(
                        factor["value"]["value"]["unit"]["key"],
                        "def.0000000000000001"
                    );
                    assert_eq!(
                        m["rolls"]["completion"]["code"],
                        "modifier-roll-schema-partial"
                    );
                    fresh_issues.push(m["rolls"]["completion"]["id"].clone());
                }
                canonical_added.push(m.clone());
                false
            });
        if case == 5 && item["template"]["value"]["key"] == "def.0000000000001e0e" {
            let level = &a["draft"]["items"]["members"][i]["item_level"];
            assert_eq!(level["kind"], "pending");
            assert_eq!(level["code"], "item-level-not-converted");
            assert_eq!(item["item_level"], json!({"kind":"known","value":null}));
            retired.push(level["id"].clone());
            item["item_level"] = level.clone();
        }
    }
    assert_eq!(canonical_added.len(), added.len());
    assert_eq!(retired.len(), usize::from(case == 5));
    let mut ids = BTreeMap::new();
    correspond(&a["draft"], &mut restored, &mut ids, "draft");
    for (old, mut new) in retained_pairs {
        relocate(&mut new, &ids);
        assert_eq!(old, new);
    }
    let mut before_origins = sa["origins"].clone();
    let mut after_origins = sb["origins"].clone();
    for r in before_origins.as_array_mut().unwrap() {
        r["links"]
            .as_array_mut()
            .unwrap()
            .retain(|v| !(v["kind"] == "issue" && retired.contains(&v["value"])));
    }
    for r in after_origins.as_array_mut().unwrap() {
        r["links"].as_array_mut().unwrap().retain(|v| {
            !((v["kind"] == "modifier" && added.contains(&v["value"]))
                || (v["kind"] == "issue" && fresh_issues.contains(&v["value"])))
        });
    }
    correspond(
        &before_origins,
        &mut after_origins,
        &mut ids,
        "all retained source links",
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
    let mut expected = before["finalization"]["issues"].as_array().unwrap().clone();
    expected.retain(|v| !retired.contains(&v["id"]));
    let mut actual = after["finalization"]["issues"].clone();
    let introduced: Vec<_> = actual
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| fresh_issues.contains(&v["id"]))
        .cloned()
        .collect();
    assert_eq!(introduced.len(), usize::from(matches!(case, 1 | 3 | 4)));
    actual
        .as_array_mut()
        .unwrap()
        .retain(|v| !fresh_issues.contains(&v["id"]));
    relocate(&mut actual, &ids);
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
            if added.contains(&m["id"]) {
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
    for old in &expected {
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
        json!(expected) == actual,
        "other selected issue content changed in original {case}"
    );
    let mut selection = selected::selection(xml, new);
    relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    json!({"original":case,"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"source_census":census,"added_modifiers":canonical_added,"retired_item_level_issues":retired,"introduced_selected_issues":introduced,"relocated_issue_paths":changed_paths,"allocator_before":a["draft"]["allocator"],"allocator_after":b["draft"]["allocator"],"selected_issue_summary":after["selected_issue_summary"]})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"22\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let cases = [
        ("fractional", "10.5% increased Movement Speed", false),
        ("negative", "-10% increased Movement Speed", false),
        ("tagged", "{tags:speed}10% increased Movement Speed", false),
        (
            "desecrated",
            "{desecrated}10% increased Movement Speed",
            false,
        ),
        (
            "corruption",
            "{corruptedRange:1.5}10% increased Movement Speed",
            false,
        ),
        ("range", "(10-10)% increased Movement Speed", false),
        (
            "unknown-prefix",
            "Unknown source semantics\n10% increased Movement Speed",
            false,
        ),
        ("implicit", "10% increased Movement Speed", true),
        ("enchant", "{enchant}10% increased Movement Speed", true),
    ];
    for (label, text, admitted) in cases {
        let mut changed = raw.replace("10% increased Movement Speed", text);
        if label == "implicit" {
            changed = changed.replace("Implicits: 0", "Implicits: 1");
        }
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let s: Value = read(dest.join("sidecar.json"));
        let r = row(&s, 578);
        let l = r["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["text"]
                    .as_str()
                    .unwrap()
                    .contains("increased Movement Speed")
            })
            .unwrap();
        assert_eq!(
            l["modifiers"].as_array().unwrap().len(),
            usize::from(admitted),
            "{label}"
        );
        assert_eq!(r["defaults"]["item_level_absent"], admitted, "{label}");
        if admitted {
            let d: Value = read(dest.join("draft.json"));
            let m = d["draft"]["items"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v["modifiers"]["members"].as_array().unwrap())
                .find(|v| v["id"] == l["modifiers"][0])
                .unwrap();
            assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
            assert_eq!(
                m["rolls"]["members"][23]["value"]["value"]["value"]["key"],
                if label == "implicit" {
                    "def.00000000000030e3"
                } else {
                    "def.00000000000030e4"
                }
            );
        } else {
            assert_eq!(r["attribution"]["layout"]["status"], "pending");
        }
    }
    cases.len()
}

#[test]
#[ignore = "requires exact item-level-absence publication predecessor"]
fn real_movement_family_preserves_originals_and_admits_only_proven_occurrences() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MOVEMENT_PRIOR")
            .expect("explicit checked predecessor"),
    );
    let inventory = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let bindings = family::bindings();
    preservation::check(
        &prior,
        &next,
        preservation::FamilyChange {
            modifier: &bindings.modifier,
            templates: &bindings.templates,
            dependencies: &family::dependency_definitions(),
            extension: &family::extension(),
            last_issued: 0x30ff,
            rule: &family::item_rule(),
            condition: &family::source_condition(),
            default: &family::source_default(),
        },
    );
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_MOVEMENT_OUTPUT")
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
