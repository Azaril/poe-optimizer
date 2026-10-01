//! Source-backed ranged Spirit components and unchanged original requests.
#[path = "support/owned_ranged_spirit.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_family_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
#[allow(dead_code)]
mod selected;

use poe_optimizer_core::owned_schema::SchemaClosure;
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
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap()
}
#[test]
fn authored_domain_is_positive_source_bound_and_keeps_final_spirit_unmapped() {
    let b = family::bindings();
    let e = family::extension();
    assert_eq!(e.schema.len(), 26);
    assert_eq!(b.properties.len(), 21);
    assert_eq!(b.unit.key().as_str(), "def.000000000000295a");
    assert_eq!(b.contribution_unit.key().as_str(), "def.0000000000000004");
    assert_eq!(b.contribution.key().as_str(), "def.0000000000003166");
    assert_eq!(b.amount.slot.key().as_str(), "def.000000000000314e");
    assert_eq!(b.corrupted_base.slot.key().as_str(), "def.0000000000003164");
    assert_eq!(b.category.slot.key().as_str(), "def.0000000000003165");
    assert_eq!(b.effective.key().as_str(), "def.000000000000295b");
    assert_eq!(b.templates.len(), 1);
    assert_eq!(b.templates[0].key().as_str(), "def.0000000000002343");
    assert!(matches!(
        e.owners[0].programs.closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(
        family::source_default().quality,
        poe_optimizer_import::owned_item_source::ItemSourceAbsentPolicy::Pending
    );
    let condition = serde_json::to_value(family::source_condition()).unwrap();
    assert_eq!(
        condition["all"],
        json!([{"kind":"initial_scaling_is_one"},{"kind":"no_generated_buff_members"},{"kind":"unsigned_integer_capture","value":{"capture":"lower","min":1,"max":1000000}},{"kind":"unsigned_integer_capture","value":{"capture":"upper","min":1,"max":1000000}}])
    );
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
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
fn row(sidecar: &Value, ordinal: u64) -> &Value {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn item_index(draft: &Value, template: &str) -> usize {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .position(|v| v["template"]["value"]["key"] == template)
        .unwrap()
}
fn check_rolls(m: &Value, modifier: &str, amount: f64, category: &str) {
    assert_eq!(m["definition"]["value"]["key"], modifier);
    assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
    let rolls = m["rolls"]["members"].as_array().unwrap();
    assert_eq!(rolls.len(), 24);
    assert_eq!(
        rolls[0]["value"]["value"],
        json!({"kind":"quantity","value":{"value":amount,"unit":family::bindings().unit}})
    );
    for flag in &rolls[1..22] {
        assert_eq!(
            flag["value"],
            json!({"kind":"known","value":{"kind":"boolean","value":false}})
        );
    }
    assert_eq!(
        rolls[22]["value"]["value"],
        json!({"kind":"quantity","value":{"value":1.0,"unit":family::bindings().factor_unit}})
    );
    assert_eq!(rolls[23]["value"]["value"]["value"]["key"], category);
    assert!(
        rolls
            .iter()
            .all(|r| r["slot"]["value"]["declaration"]["definition"]["key"] == modifier)
    );
}
// Fresh imports deliberately allocate different lineages. Source metadata has
// bare lineage fields as well as local-ID objects. Rejoin only the former;
// every local ID still goes through the single injective correspondence map.
fn source_lineage(value: &mut Value, expected: &Value, prior: &Value) {
    match value {
        Value::Object(o) => {
            if !o.contains_key("local")
                && let Some(lineage) = o.get_mut("lineage")
            {
                assert_eq!(
                    lineage, expected,
                    "bare source lineage belongs to fresh import"
                );
                *lineage = prior.clone();
            }
            for v in o.values_mut() {
                source_lineage(v, expected, prior);
            }
        }
        Value::Array(a) => {
            for v in a {
                source_lineage(v, expected, prior);
            }
        }
        _ => {}
    }
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    let mut old_draft = a["draft"].clone();
    let mut new_draft = b["draft"].clone();
    let mut old_side = sa.clone();
    let mut new_side = sb.clone();
    source_lineage(
        &mut new_side,
        &b["draft"]["allocator"]["lineage"],
        &a["draft"]["allocator"]["lineage"],
    );
    let mut added = vec![];
    let mut retired = vec![];
    if case == 5 {
        let i = item_index(&a, "def.0000000000002343");
        assert_eq!(i, item_index(&b, "def.0000000000002343"));
        let x = &a["draft"]["items"]["members"][i];
        let y = &b["draft"]["items"]["members"][i];
        assert!(x["modifiers"]["members"].as_array().unwrap().is_empty());
        assert_eq!(y["modifiers"]["members"].as_array().unwrap().len(), 2);
        check_rolls(
            &y["modifiers"]["members"][0],
            "def.000000000000314d",
            12.5,
            "def.00000000000030e3",
        );
        check_rolls(
            &y["modifiers"]["members"][1],
            "def.00000000000030ca",
            1.0,
            "def.00000000000030e2",
        );
        added = y["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].clone())
            .collect();
        assert_eq!(x["item_level"]["kind"], "pending");
        assert_eq!(y["item_level"], json!({"kind":"known","value":null}));
        retired.push(x["item_level"]["id"].clone());
        for field in ["quality", "parameters", "modifier_order"] {
            assert_eq!(x[field]["kind"].as_str().unwrap_or("pending"), "pending");
        }
        assert_eq!(y["modifiers"]["completion"]["kind"], "pending");
        assert_eq!(y["parameters"]["completion"]["kind"], "pending");
        old_draft["items"]["members"][i]["item_level"] = y["item_level"].clone();
        new_draft["items"]["members"][i]["modifiers"]["members"] = json!([]);
        let p = row(&sa, 580);
        let q = row(&sb, 580);
        assert_eq!(q["source"], p["source"]);
        assert_eq!(q["content_entry"], p["content_entry"]);
        assert_eq!(
            q["defaults"],
            json!({"parameters":[],"item_level_absent":true,"quality_absent":false})
        );
        assert_eq!(q["attribution"]["layout"], json!({"status":"proven"}));
        assert_eq!(q["attribution"]["default_scope"]["kind"], "proven");
        assert_eq!(
            q["lines"].as_array().unwrap().len(),
            p["lines"].as_array().unwrap().len()
        );
        for (old_line, new_line) in p["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(q["lines"].as_array().unwrap())
        {
            assert_eq!(old_line["index"], new_line["index"]);
            assert_eq!(old_line["text"], new_line["text"]);
            if new_line["index"] == 13 || new_line["index"] == 14 {
                let j = usize::from(new_line["index"] == 14);
                assert_eq!(new_line["modifiers"], json!([added[j]]));
                assert_eq!(new_line["outcome"]["kind"], "known");
                assert_eq!(
                    new_line["outcome"]["value"]["rule"],
                    if j == 0 {
                        "ranged-spirit"
                    } else {
                        "fixed-global-minion-level"
                    }
                );
            } else {
                assert_eq!(old_line, new_line);
            }
        }
        for (index, category) in [(12, "implicit"), (13, "explicit")] {
            let line = &q["attribution"]["lines"][index];
            assert_eq!(
                line["member"],
                json!({"category":category,"ordinal":1,"line":index+1})
            );
            assert_eq!(line["blockers"], json!([]));
        }
        assert_eq!(q["attribution"]["lines"][12]["range"]["fraction"], 0.5);
        let writes = q["attribution"]["writes"].as_array().unwrap();
        assert!(
            writes
                .iter()
                .any(|w| w["source_id"] == 1 && w["target"] == json!({"status":"line","value":13}))
        );
        let target = new_side["item_texts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["source"]["ordinal"] == 580)
            .unwrap();
        *target = p.clone();
    }
    new_draft["allocator"] = old_draft["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(
        &old_draft,
        &mut new_draft,
        &mut ids,
        "all other canonical inputs",
    );
    let mut removed_old = 0;
    let mut removed_new = 0;
    for origin in old_side["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue" && retired.contains(&link["value"]);
            removed_old += usize::from(remove);
            !remove
        });
    }
    for origin in new_side["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "modifier" && added.contains(&link["value"]);
            removed_new += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed_old, retired.len());
    assert_eq!(removed_new, added.len());
    // Dependency digests change, while all source syntax and unrelated outcomes
    // must correspond exactly, including source allocator and repeated links.
    for field in [
        "policy",
        "mapping",
        "registry",
        "definitions",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
        "draft",
        "allocator_after",
    ] {
        new_side[field] = old_side[field].clone();
    }
    for (p, q) in old_side["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new_side["item_texts"].as_array_mut().unwrap())
    {
        for field in ["policy", "item_lines"] {
            q["attribution"][field] = p["attribution"][field].clone();
        }
    }
    identity::correspond(
        &old_side,
        &mut new_side,
        &mut ids,
        "all other source evidence",
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
    let mut expected = before["finalization"]["issues"].clone();
    expected
        .as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(&v["id"]));
    let mut actual = after["finalization"]["issues"].clone();
    identity::relocate(&mut actual, &ids);
    assert_eq!(
        actual, expected,
        "only the proven Solar item-level issue retires"
    );
    let mut selection = selected::selection(xml, new);
    identity::relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count - after_count, usize::from(case == 5));
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"new_modifiers":added,"retired_issues":retired,"allocator_before":a["draft"]["allocator"],"allocator_after":b["draft"]["allocator"],"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"23\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let line = "{range:0.5}+(10-15) to Spirit";
    let negative = [
        (
            "zero-range",
            raw.replace(line, "{range:0.5}+(0-0) to Spirit"),
        ),
        (
            "zero-lower",
            raw.replace(line, "{range:0.5}+(0-15) to Spirit"),
        ),
        (
            "negative-lower",
            raw.replace(line, "{range:0.5}+(-1-15) to Spirit"),
        ),
        (
            "reversed",
            raw.replace(line, "{range:0.5}+(15-10) to Spirit"),
        ),
        (
            "unknown-prefix",
            raw.replace(line, &format!("Unknown source semantics\n{line}")),
        ),
        (
            "desecrated",
            raw.replace(line, &format!("{{desecrated}}{line}")),
        ),
        (
            "corrupted",
            raw.replace(line, &format!("{{corruptedRange:1.5}}{line}")),
        ),
        (
            "malformed-overlay",
            raw.replace("range=\"0.5\" id=\"1\"", "range=\"bad\" id=\"1\""),
        ),
    ];
    for (label, changed) in &negative {
        assert_ne!(changed, raw);
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let file = out.join(format!("probe-{label}.xml"));
        fs::write(&file, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &file, 5, &dest);
        let d: Value = read(dest.join("draft.json"));
        let s: Value = read(dest.join("sidecar.json"));
        let item = &d["draft"]["items"]["members"][item_index(&d, "def.0000000000002343")];
        assert!(
            !item["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["definition"]["value"]["key"] == "def.000000000000314d"),
            "{label}"
        );
        if *label == "reversed" {
            // Original source still has one positive Spirit member here. The
            // owned numeric converter has a narrower ordered-range domain;
            // that numeric gap does not erase independent layout evidence.
            assert_eq!(row(&s, 580)["attribution"]["layout"]["status"], "proven");
            assert_eq!(
                row(&s, 580)["lines"][12]["outcome"]["value"]["reason"],
                json!({"kind":"invalid_range"})
            );
            assert_eq!(item["item_level"], json!({"kind":"known","value":null}));
            assert_eq!(row(&s, 580)["defaults"]["item_level_absent"], true);
            assert_eq!(item["modifiers"]["members"].as_array().unwrap().len(), 1);
            check_rolls(
                &item["modifiers"]["members"][0],
                "def.00000000000030ca",
                1.0,
                "def.00000000000030e2",
            );
            assert_eq!(item["modifiers"]["completion"]["kind"], "pending");
            continue;
        }
        assert_eq!(item["item_level"]["kind"], "pending", "{label}");
        assert_eq!(
            row(&s, 580)["attribution"]["layout"]["status"],
            "pending",
            "{label}"
        );
        assert_eq!(
            row(&s, 580)["defaults"]["item_level_absent"],
            false,
            "{label}"
        );
    }
    let positive = [
        (
            "legacy-zero",
            raw.replace("range=\"0.5\" id=\"1\"", "range=\"0\" id=\"1\""),
            10.0,
            None,
        ),
        (
            "legacy-one",
            raw.replace("range=\"0.5\" id=\"1\"", "range=\"1\" id=\"1\""),
            15.0,
            None,
        ),
        (
            "explicit-level",
            raw.replace("LevelReq: 30", "Item Level: 77\nLevelReq: 30"),
            12.5,
            Some(77),
        ),
    ];
    for (label, changed, amount, level) in &positive {
        assert_ne!(changed, raw);
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let file = out.join(format!("probe-{label}.xml"));
        fs::write(&file, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &file, 5, &dest);
        let d: Value = read(dest.join("draft.json"));
        let s: Value = read(dest.join("sidecar.json"));
        let item = &d["draft"]["items"]["members"][item_index(&d, "def.0000000000002343")];
        check_rolls(
            &item["modifiers"]["members"][0],
            "def.000000000000314d",
            *amount,
            "def.00000000000030e3",
        );
        check_rolls(
            &item["modifiers"]["members"][1],
            "def.00000000000030ca",
            1.0,
            "def.00000000000030e2",
        );
        assert_eq!(item["item_level"], json!({"kind":"known","value":level}));
        assert_eq!(
            row(&s, 580)["defaults"]["item_level_absent"],
            level.is_none()
        );
        assert_eq!(item["parameters"]["completion"]["kind"], "pending");
    }
    negative.len() + positive.len()
}

#[test]
#[ignore = "requires the exact checked armour-input predecessor"]
fn actual_solar_layout_preserves_every_original_request_and_unrelated_gap() {
    let prior_dir = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RANGED_SPIRIT_PRIOR").expect("explicit checked prior"),
    );
    let inventory = release::inventory(&prior_dir);
    let prior = release::load(&prior_dir);
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
            last_issued: 0x3166,
            rule: &family::item_rule(),
            condition: &family::source_condition(),
            rule_change: preservation::RuleChange::Append,
            default: Some(&family::source_default()),
        },
    );
    assert!(next.input().evaluation.is_none());
    assert_eq!(next.receipt().query_rows, 110);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_RANGED_SPIRIT_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
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
            fs::read(prior_dir.join(&query)).unwrap(),
            fs::read(package.join(query)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_dir, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    let probe_count = probes(&package, &out);
    assert_eq!(inventory, release::inventory(&prior_dir));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":110,"originals":reports,"probes":probe_count,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
