//! A selected Ruby quality gate, with exact corpus and collateral preservation.
#[path = "support/owned_fire_damage_modifier.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
#[allow(dead_code)]
mod selected;

use poe_optimizer_core::owned_schema::SchemaClosure;
use poe_optimizer_import::owned_item_source::ItemSourceAbsentPolicy;
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
fn authored_fixed_fire_family_admits_only_ruby_and_quality_absence() {
    let b = family::bindings();
    let e = family::extension();
    assert_eq!(e.schema.len(), 25);
    assert_eq!(e.owners.len(), 1);
    assert_eq!(e.owners[0].programs.members.len(), 3);
    assert_eq!(
        e.operations_version.as_ref().unwrap().as_str(),
        "owned-domain-operations-v14"
    );
    assert!(e.receivers.is_empty());
    assert!(matches!(
        e.owners[0].programs.closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(b.modifier.key().as_str(), "def.00000000000031e4");
    assert_eq!(b.amount.slot.key().as_str(), "def.00000000000031e5");
    assert_eq!(b.properties.len(), 21);
    assert_eq!(b.corrupted_base.slot.key().as_str(), "def.00000000000031fb");
    assert_eq!(b.category.slot.key().as_str(), "def.00000000000031fc");
    assert_eq!(b.unit.key().as_str(), "def.0000000000000002");
    assert_eq!(b.effective.key().as_str(), "def.000000000000253e");
    assert_eq!(b.templates.len(), 1);
    assert_eq!(b.templates[0].key().as_str(), "def.000000000000200b");
    let d = family::source_default();
    assert_eq!(d.template, b.templates[0]);
    assert_eq!(d.quality, ItemSourceAbsentPolicy::Absent);
    assert_eq!(d.item_level, ItemSourceAbsentPolicy::Pending);
    assert!(d.parameters.is_empty());
    assert_eq!(
        serde_json::to_value(family::source_condition()).unwrap()["all"],
        json!([
            {"kind":"no_source_scaling_tags"}, {"kind":"initial_scaling_is_one"},
            {"kind":"no_generated_buff_members"},
            {"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
        ])
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
    assert!(
        a["source_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["path"] == "src/Data/Bases/jewel.lua")
    );
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
fn row_mut(sidecar: &mut Value, ordinal: u64) -> &mut Value {
    sidecar["item_texts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn ruby_index(draft: &Value) -> usize {
    let matches: Vec<_> = draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, v)| v["template"]["value"]["key"] == "def.000000000000200b")
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0].0
}
fn false_properties() -> Value {
    let m: BTreeMap<_, _> = family::bindings()
        .properties
        .keys()
        .filter(|k| k.as_str() != "unscalable")
        .map(|k| (k.clone(), false))
        .collect();
    assert_eq!(m.len(), 20);
    json!(m)
}
fn source_item_index(draft: &Value, sidecar: &Value, ordinal: u64) -> usize {
    let origin = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap();
    let ids: Vec<_> = origin["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == "item")
        .map(|v| &v["value"])
        .collect();
    assert_eq!(ids.len(), 1);
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .position(|v| &v["id"] == ids[0])
        .unwrap()
}
fn check_rolls(m: &Value, amount: f64) {
    let b = family::bindings();
    assert_eq!(m["definition"], json!({"kind":"known","value":b.modifier}));
    assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
    let rolls = m["rolls"]["members"].as_array().unwrap();
    assert_eq!(rolls.len(), 24);
    let mut expected = BTreeMap::new();
    expected.insert(
        serde_json::to_value(&b.amount).unwrap().to_string(),
        json!({"kind":"quantity","value":{"value":amount,"unit":b.unit}}),
    );
    for slot in b.properties.values() {
        assert!(
            expected
                .insert(
                    serde_json::to_value(slot).unwrap().to_string(),
                    json!({"kind":"boolean","value":false})
                )
                .is_none()
        );
    }
    expected.insert(
        serde_json::to_value(&b.corrupted_base).unwrap().to_string(),
        json!({"kind":"quantity","value":{"value":1.0,"unit":b.factor_unit}}),
    );
    expected.insert(serde_json::to_value(&b.category).unwrap().to_string(), json!({"kind":"option","value":{"kind":"option","namespace":b.modifier.namespace(),"key":"def.00000000000030e2"}}));
    for roll in rolls {
        assert_eq!(roll["slot"]["kind"], "known");
        let want = expected
            .remove(&roll["slot"]["value"].to_string())
            .expect("exact authored parameter identity");
        assert_eq!(roll["value"], json!({"kind":"known","value":want}));
    }
    assert!(expected.is_empty());
}
fn source_lineage(value: &mut Value, expected: &Value, prior: &Value) {
    match value {
        Value::Object(o) => {
            if !o.contains_key("local")
                && let Some(lineage) = o.get_mut("lineage")
            {
                assert_eq!(lineage, expected);
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
fn pending_category(rule: &str) -> Value {
    json!({"kind":"pending","value":{"reason":{"kind":"missing_context_option","value":{"input":"modifier-source-category"}},"candidates":[rule]}})
}
fn source_meaning(rule: &str) -> Value {
    json!({"kind":"pending","value":{"reason":{"kind":"source_meaning_unresolved"},"candidates":[rule]}})
}
fn set_member(line: &mut Value, category: &str, ordinal: usize) {
    line["member"] = json!({"category":category,"ordinal":ordinal,"line":line["index"]});
    line["blockers"] = json!([]);
}
// These are exact expected source-report deltas, not ignored report fields.
// An unchanged pending ring still gains diagnostic positions after its formerly
// unknown Fire line is recognized. Category inputs remain withheld throughout.
fn expected_collateral(r: &mut Value, ordinal: u64) {
    let rule = "fixed-increased-fire-damage";
    r["attribution"]["lines"][8]["rule"] = json!(rule);
    r["attribution"]["lines"][8]["pending_candidates"] = json!([rule]);
    if ordinal == 508 {
        r["lines"][8]["outcome"] = source_meaning(rule);
        r["attribution"]["lines"][8]["blockers"] =
            json!(["possible_combined_line", "unknown_header"]);
        set_member(&mut r["attribution"]["lines"][9], "implicit", 1);
        r["attribution"]["lines"][9]["properties"] = false_properties();
        // The caller installs the existing Partial lightning recipe's exact
        // known emission; this archival scope still cannot prove category.
        for line in r["attribution"]["lines"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .skip(10)
        {
            line["blockers"]
                .as_array_mut()
                .unwrap()
                .retain(|p| p != "unknown_header");
        }
        return;
    }
    r["lines"][8]["outcome"] = pending_category(rule);
    set_member(&mut r["attribution"]["lines"][8], "explicit", 1);
    r["attribution"]["lines"][8]["properties"] = false_properties();
    let next_rule = if ordinal == 192 {
        "fixed-rarity"
    } else {
        "fixed-life"
    };
    if ordinal == 192 {
        // The existing implicit rarity recipe also describes this newly
        // independent explicit line, including its unresolved input closure.
        assert_eq!(r["lines"][7]["outcome"]["value"]["rule"], "fixed-rarity");
        assert_eq!(r["lines"][9]["text"], "16% increased Rarity of Items found");
        let mut outcome = r["lines"][7]["outcome"].clone();
        outcome["value"]["emissions"][0]["value"]["rolls"][0]["value"]["value"]["value"] =
            json!(16.0);
        r["lines"][9]["outcome"] = outcome;
    } else {
        r["lines"][9]["outcome"] = pending_category(next_rule);
    }
    set_member(&mut r["attribution"]["lines"][9], "explicit", 2);
    r["attribution"]["lines"][9]["properties"] = false_properties();
    for line in r["attribution"]["lines"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .skip(10)
    {
        if line["member"]["category"] == "explicit" {
            line["member"]["ordinal"] = json!(line["member"]["ordinal"].as_u64().unwrap() + 2);
        }
    }
    if ordinal == 247 {
        r["attribution"]["layout"]["problems"] = json!(["unknown_member"]);
    }
}
fn expected_ruby(r: &mut Value, modifier: &Value) {
    let b = family::bindings();
    let rolls: Vec<_> = modifier["rolls"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| json!({"slot":v["slot"]["value"],"value":v["value"]["value"]}))
        .collect();
    r["lines"][7]["outcome"] = json!({"kind":"known","value":{"rule":"fixed-increased-fire-damage","emissions":[{"kind":"modifier","value":{"definition":b.modifier,"rolls":rolls}}]}});
    r["lines"][7]["modifiers"] = json!([modifier["id"]]);
    r["defaults"]["quality_absent"] = json!(true);
    r["attribution"]["layout"] = json!({"status":"proven"});
    r["attribution"]["default_scope"] = json!({"kind":"proven","template":b.templates[0]});
    for line in r["attribution"]["lines"].as_array_mut().unwrap() {
        line["range"] = json!({"status":"absent"});
    }
    let line = &mut r["attribution"]["lines"][7];
    line["rule"] = json!("fixed-increased-fire-damage");
    line["pending_candidates"] = json!(["fixed-increased-fire-damage"]);
    set_member(line, "explicit", 1);
    line["properties"] = false_properties();
    line["range"] = json!({"status":"resolved","fraction":0.5,"winning_write":0});
    assert_eq!(r["attribution"]["writes"].as_array().unwrap().len(), 1);
    r["attribution"]["writes"][0]["target"] = json!({"status":"line","value":8});
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    let mut expected = a["draft"].clone();
    let mut actual = b["draft"].clone();
    let mut expected_side = sa.clone();
    let mut actual_side = sb.clone();
    source_lineage(
        &mut actual_side,
        &b["draft"]["allocator"]["lineage"],
        &a["draft"]["allocator"]["lineage"],
    );
    let mut retired = vec![];
    let mut added = vec![];
    let mut added_issues = vec![];
    if case == 3 {
        // Original03's separate Ruby remains unresolved. Configuring a source
        // default changes only its proof diagnostic, never its actual inputs.
        let original = row(&sa, 251);
        assert_eq!(original["lines"][2]["text"], "Ruby");
        assert_eq!(
            original["attribution"]["default_scope"],
            json!({"kind":"unconfigured"})
        );
        assert_eq!(original["attribution"]["layout"]["status"], "pending");
        assert_eq!(
            row(&sb, 251)["defaults"],
            json!({"parameters":[],"item_level_absent":false,"quality_absent":false})
        );
        row_mut(&mut expected_side, 251)["attribution"]["default_scope"] =
            json!({"kind":"unproved"});
    }
    if case == 4 {
        let i = ruby_index(&a);
        assert_eq!(i, ruby_index(&b));
        let x = &a["draft"]["items"]["members"][i];
        let y = &b["draft"]["items"]["members"][i];
        assert_eq!(x["id"]["local"], "0000000000000094");
        assert_eq!(x["quality"]["id"]["local"], "0000000000000095");
        assert_eq!(x["modifiers"]["members"], json!([]));
        assert_eq!(y["modifiers"]["members"].as_array().unwrap().len(), 1);
        let modifier = &y["modifiers"]["members"][0];
        check_rolls(modifier, 14.0);
        added.push(modifier["id"].clone());
        retired.push(x["quality"]["id"].clone());
        assert_eq!(y["quality"], json!({"kind":"known","value":null}));
        assert_eq!(y["item_level"], json!({"kind":"known","value":55}));
        assert_eq!(y["parameters"]["completion"]["kind"], "pending");
        assert_eq!(y["modifiers"]["completion"]["kind"], "pending");
        assert_eq!(y["modifier_order"]["kind"], "pending");
        expected["items"]["members"][i]["quality"] = y["quality"].clone();
        actual["items"]["members"][i]["modifiers"]["members"] = json!([]);
        let mut ruby = row(&sa, 171).clone();
        expected_ruby(&mut ruby, modifier);
        // Compare the whole expected Ruby scope before restoring its old IDs for
        // the single global source-occurrence correspondence below.
        for field in ["policy", "item_lines"] {
            ruby["attribution"][field] = row(&sb, 171)["attribution"][field].clone();
        }
        let mut scope = row(&actual_side, 171).clone();
        assert_eq!(scope, ruby, "exact selected Ruby source evidence");
        scope = row(&sa, 171).clone();
        *row_mut(&mut actual_side, 171) = scope;
        for ordinal in [192, 247] {
            expected_collateral(row_mut(&mut expected_side, ordinal), ordinal);
        }
        let ring_index = source_item_index(&a, &sa, 192);
        assert_eq!(ring_index, source_item_index(&b, &sb, 192));
        let old_ring = &a["draft"]["items"]["members"][ring_index];
        let new_ring = &b["draft"]["items"]["members"][ring_index];
        assert_eq!(
            old_ring["modifiers"]["members"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            new_ring["modifiers"]["members"].as_array().unwrap().len(),
            2
        );
        let rarity = &new_ring["modifiers"]["members"][1];
        let mut exact_rarity = old_ring["modifiers"]["members"][0].clone();
        assert_eq!(
            exact_rarity["definition"]["value"]["key"],
            "def.00000000000029bc"
        );
        assert_eq!(
            exact_rarity["rolls"]["members"].as_array().unwrap().len(),
            24
        );
        assert_eq!(
            exact_rarity["rolls"]["members"][0]["value"]["value"]["value"]["value"],
            15.0
        );
        exact_rarity["id"] = rarity["id"].clone();
        exact_rarity["rolls"]["completion"]["id"] = rarity["rolls"]["completion"]["id"].clone();
        exact_rarity["rolls"]["members"][0]["value"]["value"]["value"]["value"] = json!(16.0);
        assert_eq!(
            rarity, &exact_rarity,
            "only rarity amount and new instance/issue IDs differ"
        );
        assert_eq!(rarity["rolls"]["completion"]["kind"], "pending");
        assert_eq!(
            rarity["rolls"]["completion"]["code"],
            "modifier-roll-schema-partial"
        );
        added.push(rarity["id"].clone());
        added_issues.push(json!({"id":rarity["rolls"]["completion"]["id"],"code":"modifier-roll-schema-partial","owner":new_ring["id"],"path":format!("items.members[{ring_index}].modifiers.members[1].rolls.completion")}));
        actual["items"]["members"][ring_index]["modifiers"]["members"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        assert_eq!(
            row(&sb, 192)["lines"][9]["modifiers"],
            json!([rarity["id"]])
        );
        row_mut(&mut actual_side, 192)["lines"][9]["modifiers"] = json!([]);
    }
    if case == 5 {
        expected_collateral(row_mut(&mut expected_side, 508), 508);
        let donor_index = source_item_index(&a, &sa, 487);
        let donor = a["draft"]["items"]["members"][donor_index]["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["definition"]["value"]["key"] == "def.0000000000002942")
            .unwrap();
        let ring_index = source_item_index(&a, &sa, 508);
        assert_eq!(ring_index, source_item_index(&b, &sb, 508));
        let old_ring = &a["draft"]["items"]["members"][ring_index];
        let new_ring = &b["draft"]["items"]["members"][ring_index];
        assert_eq!(old_ring["modifiers"]["members"], json!([]));
        assert_eq!(
            new_ring["modifiers"]["members"].as_array().unwrap().len(),
            1
        );
        let lightning = &new_ring["modifiers"]["members"][0];
        let mut exact = donor.clone();
        assert_eq!(exact["rolls"]["members"].as_array().unwrap().len(), 23);
        assert_eq!(
            exact["rolls"]["members"][0]["value"]["value"]["value"]["value"],
            40.0
        );
        exact["id"] = lightning["id"].clone();
        exact["rolls"]["completion"]["id"] = lightning["rolls"]["completion"]["id"].clone();
        exact["rolls"]["members"][0]["value"]["value"]["value"]["value"] = json!(20.0);
        assert_eq!(
            lightning, &exact,
            "archived lightning retains the exact existing Partial family"
        );
        assert_eq!(lightning["rolls"]["completion"]["kind"], "pending");
        assert_eq!(
            lightning["rolls"]["completion"]["code"],
            "modifier-roll-schema-partial"
        );
        added.push(lightning["id"].clone());
        added_issues.push(json!({"id":lightning["rolls"]["completion"]["id"],"code":"modifier-roll-schema-partial","owner":new_ring["id"],"path":format!("items.members[{ring_index}].modifiers.members[0].rolls.completion")}));
        actual["items"]["members"][ring_index]["modifiers"]["members"] = json!([]);
        assert_eq!(
            row(&sb, 508)["lines"][9]["modifiers"],
            json!([lightning["id"]])
        );
        row_mut(&mut actual_side, 508)["lines"][9]["modifiers"] = json!([]);
        let mut outcome = row(&sa, 487)["lines"][13]["outcome"].clone();
        assert_eq!(outcome["kind"], "known");
        assert_eq!(outcome["value"]["rule"], "fixed-lightning");
        outcome["value"]["emissions"][0]["value"]["rolls"][0]["value"]["value"]["value"] =
            json!(20.0);
        row_mut(&mut expected_side, 508)["lines"][9]["outcome"] = outcome;
    }
    let issued = |d: &Value| {
        u64::from_str_radix(d["allocator"]["last_issued"].as_str().unwrap(), 16).unwrap()
    };
    assert_eq!(
        issued(&actual),
        issued(&expected) + if matches!(case, 4 | 5) { 2 } else { 0 },
        "Ruby replaces one quality allocation; only each exposed legacy modifier and Partial-roll issue add allocations"
    );
    actual["allocator"] = expected["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(
        &expected,
        &mut actual,
        &mut ids,
        "all other canonical inputs",
    );
    let mut removed_old = 0;
    let mut removed_new = 0;
    for origin in expected_side["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue" && retired.contains(&link["value"]);
            removed_old += usize::from(remove);
            !remove
        });
    }
    for origin in actual_side["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "modifier" && added.contains(&link["value"])
                || link["kind"] == "issue" && added_issues.iter().any(|v| v["id"] == link["value"]);
            removed_new += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed_old, retired.len());
    assert_eq!(removed_new, added.len() + added_issues.len());
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
        actual_side[field] = expected_side[field].clone();
    }
    for (p, q) in expected_side["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(actual_side["item_texts"].as_array_mut().unwrap())
    {
        for field in ["policy", "item_lines"] {
            q["attribution"][field] = p["attribution"][field].clone();
        }
    }
    identity::correspond(
        &expected_side,
        &mut actual_side,
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
    let mut expected_issues = before["finalization"]["issues"].clone();
    expected_issues
        .as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(&v["id"]));
    let mut actual_issues = after["finalization"]["issues"].clone();
    for added in &added_issues {
        let matches: Vec<_> = actual_issues
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["id"] == added["id"])
            .collect();
        if case == 4 {
            assert_eq!(
                matches,
                vec![added],
                "exact newly exposed selected rarity gap"
            );
        } else {
            assert!(
                matches.is_empty(),
                "the archived Emerald Ring remains unselected"
            );
        }
    }
    actual_issues
        .as_array_mut()
        .unwrap()
        .retain(|v| !added_issues.iter().any(|added| added["id"] == v["id"]));
    identity::relocate(&mut actual_issues, &ids);
    assert_eq!(
        actual_issues, expected_issues,
        "only selected Ruby quality0095 retires"
    );
    let mut selection = selected::selection(xml, new);
    identity::relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count, [123, 124, 116, 153, 20][case - 1]);
    let selected_additions = if case == 4 {
        added_issues.clone()
    } else {
        vec![]
    };
    assert_eq!(
        after_count,
        before_count - retired.len() + selected_additions.len()
    );
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"new_modifiers":added,"retired_issues":retired,"newly_exposed_issues":added_issues,"newly_exposed_selected_issues":selected_additions,"allocator_before":a["draft"]["allocator"],"allocator_after":b["draft"]["allocator"],"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-04.xml"))
            .unwrap();
    let start = original.find("<Item id=\"1\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let line = "14% increased Fire Damage";
    let replacements = [
        (
            "quality-zero",
            raw.replace("Item Level: 55", "Item Level: 55\nQuality: 0"),
        ),
        (
            "quality-twenty",
            raw.replace("Item Level: 55", "Item Level: 55\nQuality: 20"),
        ),
        (
            "quality-malformed",
            raw.replace("Item Level: 55", "Item Level: 55\nQuality: bad"),
        ),
        (
            "quality-duplicate",
            raw.replace("Item Level: 55", "Item Level: 55\nQuality: 0\nQuality: 20"),
        ),
        (
            "quality-alias",
            raw.replace(
                "Item Level: 55",
                "Item Level: 55\nQuality (Elemental Damage Modifiers): 20%",
            ),
        ),
        (
            "quality-catalyst-alias",
            raw.replace(
                "Item Level: 55",
                "Item Level: 55\nQuality (Fire Modifiers): 20%",
            ),
        ),
        (
            "unknown",
            raw.replace(line, &format!("Unknown source semantics\n{line}")),
        ),
        (
            "variant",
            raw.replace(line, &format!("{{variant:1}}{line}")),
        ),
        (
            "desecrated",
            raw.replace(line, &format!("{{desecrated}}{line}")),
        ),
        (
            "fractured",
            raw.replace(line, &format!("{{fractured}}{line}")),
        ),
        ("crafted", raw.replace(line, &format!("{{crafted}}{line}"))),
        ("tagged", raw.replace(line, &format!("{{tags:fire}}{line}"))),
        (
            "scaled",
            raw.replace(line, &format!("{{corruptedRange:1.5}}{line}")),
        ),
        (
            "fractional",
            raw.replace(line, "14.5% increased Fire Damage"),
        ),
        (
            "overflow",
            raw.replace(line, "1000001% increased Fire Damage"),
        ),
        ("negative", raw.replace(line, "-14% increased Fire Damage")),
        (
            "overlay-malformed",
            raw.replace("range=\"0.5\"", "range=\"bad\""),
        ),
    ];
    for (label, changed) in &replacements {
        assert_ne!(changed, raw);
        let file = out.join(format!("probe-{label}.xml"));
        fs::write(
            &file,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &file, 4, &dest);
        let d: Value = read(dest.join("draft.json"));
        let s: Value = read(dest.join("sidecar.json"));
        let item = &d["draft"]["items"]["members"][ruby_index(&d)];
        assert_ne!(
            item["quality"],
            json!({"kind":"known","value":null}),
            "no false absence: {label}"
        );
        assert_eq!(row(&s, 171)["defaults"]["quality_absent"], false, "{label}");
        assert_eq!(
            item["parameters"]["completion"]["kind"], "pending",
            "{label}"
        );
        assert_eq!(
            item["modifiers"]["completion"]["kind"], "pending",
            "{label}"
        );
        assert_eq!(item["modifier_order"]["kind"], "pending", "{label}");
        if !label.starts_with("quality-") {
            assert_eq!(
                row(&s, 171)["attribution"]["layout"]["status"],
                "pending",
                "{label}"
            );
            assert!(
                item["modifiers"]["members"].as_array().unwrap().is_empty(),
                "{label}"
            );
        }
    }
    let positives = [
        ("amount-zero", 0),
        ("amount-one", 1),
        ("amount-high", 1_000_000),
    ];
    for (label, amount) in positives {
        let changed = raw.replace(line, &format!("{amount}% increased Fire Damage"));
        let file = out.join(format!("probe-{label}.xml"));
        fs::write(
            &file,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &file, 4, &dest);
        let d: Value = read(dest.join("draft.json"));
        let s: Value = read(dest.join("sidecar.json"));
        let item = &d["draft"]["items"]["members"][ruby_index(&d)];
        assert_eq!(
            item["quality"],
            json!({"kind":"known","value":null}),
            "{label}"
        );
        assert_eq!(
            item["item_level"],
            json!({"kind":"known","value":55}),
            "{label}"
        );
        assert_eq!(
            item["modifiers"]["members"].as_array().unwrap().len(),
            1,
            "{label}"
        );
        check_rolls(&item["modifiers"]["members"][0], f64::from(amount));
        assert_eq!(
            row(&s, 171)["attribution"]["layout"],
            json!({"status":"proven"})
        );
        assert_eq!(row(&s, 171)["defaults"]["quality_absent"], true);
        assert_eq!(item["parameters"]["completion"]["kind"], "pending");
        assert_eq!(item["modifiers"]["completion"]["kind"], "pending");
        assert_eq!(item["modifier_order"]["kind"], "pending");
    }
    replacements.len() + positives.len()
}
#[test]
#[ignore = "requires passed complete-source evidence and exact configured-rating predecessor"]
fn actual_ruby_quality_preserves_every_original_request_and_unrelated_gap() {
    let prior_dir = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FIRE_DAMAGE_PRIOR").expect("explicit checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FIRE_DAMAGE_OUTPUT").expect("explicit fresh output"),
    );
    assert!(!out.exists());
    let authoring = family::authoring();
    assert_eq!(authoring["source_validation"]["status"], "passed");
    let proof = &authoring["source_validation"];
    let evidence = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        evidence.len() as u64,
        proof["evidence_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&evidence)),
        proof["evidence_sha256"]
    );
    assert!(
        root()
            .join(authoring["source_test"].as_str().unwrap())
            .is_file()
    );
    let inventory = release::inventory(&prior_dir);
    let prior = release::load(&prior_dir);
    let next = family::stage(&prior);
    assert!(next.input().evaluation.is_none());
    assert_eq!(next.receipt().query_rows, 110);
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
