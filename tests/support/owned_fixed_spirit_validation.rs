//! Exact retained-source authentication and import-delta checks for fixed Spirit.
use super::{identity, preservation, selected};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn pin(path: &Path, expected: &Value) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(expected["bytes"], bytes.len());
    assert_eq!(expected["sha256"], hash(&bytes));
}

pub fn verify_source(full: bool) {
    let v = read(root().join("data/owned/poe2/3887ae68/fixed-spirit/source-vectors.json"));
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["scope"],
        json!({"retained_source_witness":true,"new_source_execution":false,"native_parity":false,"whole_item_inventory":false,"whole_contributor_coverage":false})
    );
    for (path, expected) in v["pins"].as_object().unwrap() {
        pin(&root().join(path), expected);
    }
    let report = &v["report"];
    assert_eq!(rows(&report["cases"]).len(), 3);
    let manifest_text =
        fs::read_to_string(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .unwrap()
            .replace("\r\n", "\n");
    let manifest: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(report["source_hash"], hash(manifest_text.as_bytes()));
    assert_eq!(report["evidence"]["manifest_sha256"], report["source_hash"]);
    for key in [
        "native_parity",
        "whole_contributor_coverage",
        "complete_item_inventory",
    ] {
        assert_eq!(report["evidence"][key], false);
    }
    for p in rows(&report["evidence"]["files"]) {
        assert!(
            rows(&manifest["files"])
                .iter()
                .any(|entry| { entry["path"] == p["path"] && entry["sha256"] == p["sha256"] })
        );
        if full {
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(p["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(p["sha256"], hash(text.as_bytes()));
        }
    }
    for (i, case) in rows(&report["cases"]).iter().enumerate() {
        let build = [1, 4, 5][i];
        let expected_xml = &v["xml_pins"][i];
        assert_eq!(
            expected_xml["path"],
            format!("tests/fixtures/builds/breadth-20260908/build-{build:02}.xml")
        );
        pin(
            &root().join(expected_xml["path"].as_str().unwrap()),
            expected_xml,
        );
        assert_eq!(case["xml_sha256"], expected_xml["sha256"]);
        let state = &case["state"];
        for key in [
            "main_scalar_output_preserved",
            "observed_item_fields_preserved",
            "original_functions_preserved",
            "saved_selections_preserved",
        ] {
            assert_eq!(state[key], true);
        }
        assert_eq!(state["loaded"], state["fresh"]);
        assert_eq!(state["base_facts"]["has_buff"], false);
        let amount = [34, 48, 49][i];
        assert_eq!(case["line"], format!("+{amount} to Spirit"));
        assert_eq!(case["player_participating"], i != 2);
        assert_eq!(rows(&state["selected_slots"]).is_empty(), i == 2);
        if i == 2 {
            assert!(case["slot"].is_null());
            assert_eq!(case["next_line"], "+2 to Level of all Minion Skills");
        }
        let spirit: Vec<_> = rows(&state["loaded"]["base_mods"])
            .iter()
            .filter(|m| m["name"] == "Spirit")
            .collect();
        assert_eq!(spirit.len(), 1);
        assert_eq!(spirit[0]["value"], amount);
        assert_eq!(spirit[0]["type"], "BASE");
        assert_eq!(spirit[0]["flags"], 0);
        assert_eq!(spirit[0]["keyword_flags"], 0);
        assert!(rows(&spirit[0]["tags"]).is_empty());
        let names: Vec<_> = rows(&state["probes"])
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "integer-1",
                "integer-34",
                "integer-48",
                "integer-1000000",
                "untagged-neural",
                "untagged-neural-negative-quality",
                "tagged-neural",
                "zero",
                "decimal",
                "negative",
                "unknown-predecessor",
                "explicit-magnitude",
                "corrupted-range",
                "disabled"
            ]
        );
        for probe in rows(&state["probes"]) {
            let name = probe["name"].as_str().unwrap();
            let values: Vec<_> = rows(&probe["after"]["active"])
                .iter()
                .filter(|m| m["name"] == "Spirit")
                .map(|m| m["value"].as_f64().unwrap())
                .collect();
            let expected = match name {
                "integer-1" => Some(1.),
                "integer-34" => Some(34.),
                "integer-48" => Some(48.),
                "integer-1000000" => Some(1_000_000.),
                "untagged-neural"
                | "untagged-neural-negative-quality"
                | "unknown-predecessor"
                | "explicit-magnitude" => Some(f64::from(amount)),
                "tagged-neural" => Some([40., 57., 58.][i]),
                "zero" => Some(0.),
                "decimal" => Some(35.),
                "negative" => Some(-34.),
                "corrupted-range" => Some([51., 72., 74.][i]),
                "disabled" => None,
                _ => unreachable!(),
            };
            assert_eq!(values, expected.into_iter().collect::<Vec<_>>());
        }
    }
    assert_eq!(rows(&v["reports"]).len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    if full {
        for expected in rows(&v["reports"]) {
            let path = root().join(expected["path"].as_str().unwrap());
            pin(&path, expected);
            assert_eq!(&read(path), report);
        }
    }
}

fn definition(kind: &str, number: u64) -> Value {
    json!({"kind":kind,"namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":format!("def.{number:016x}")})
}
fn pending(reason: &str, candidates: &[&str]) -> Value {
    let reason = if reason == "missing_context_option" {
        json!({"kind":reason,"value":{"input":"modifier-source-category"}})
    } else {
        json!({"kind":reason})
    };
    json!({"kind":"pending","value":{"reason":reason,"candidates":candidates}})
}
fn item_text(v: &Value, ordinal: usize) -> &Value {
    let found: Vec<_> = rows(&v["item_texts"])
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["content_entry"], 0);
    found[0]
}
fn item_text_mut(v: &mut Value, ordinal: usize) -> &mut Value {
    v["item_texts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn line(v: &Value, index: usize) -> &Value {
    let r = &v["lines"][index - 1];
    assert_eq!(r["index"], index);
    r
}
fn line_mut(v: &mut Value, index: usize) -> &mut Value {
    let r = &mut v["lines"][index - 1];
    assert_eq!(r["index"], index);
    r
}
fn invert_field(old: &Value, new: &mut Value, field: &str, a: Value, b: Value) {
    assert_eq!(old[field], a, "old {field}");
    assert_eq!(new[field], b, "new {field}");
    new[field] = a;
}
fn properties() -> Value {
    let mut p = serde_json::Map::new();
    for key in [
        "armour",
        "attack",
        "attribute",
        "caster",
        "chaos",
        "cold",
        "cold_resistance",
        "defences",
        "elemental",
        "elemental_resistance",
        "energyshield",
        "evasion",
        "fire",
        "life",
        "lightning",
        "mana",
        "minion",
        "physical",
        "resistance",
        "speed",
    ] {
        p.insert(key.to_owned(), json!(false));
    }
    Value::Object(p)
}
fn member(category: &str, ordinal: usize, line: usize) -> Value {
    json!({"category":category,"ordinal":ordinal,"line":line})
}
fn independent_member(
    old: &Value,
    new: &mut Value,
    index: usize,
    blockers: &[&str],
    category: &str,
    ordinal: usize,
) {
    let a = line(&old["attribution"], index);
    let b = line_mut(&mut new["attribution"], index);
    invert_field(a, b, "blockers", json!(blockers), json!([]));
    invert_field(a, b, "properties", json!({}), properties());
    invert_field(
        a,
        b,
        "member",
        Value::Null,
        member(category, ordinal, index),
    );
}
fn spirit_pending(
    old: &Value,
    new: &mut Value,
    index: usize,
    amount: i64,
    blockers: &[&str],
    category: &str,
    ordinal: usize,
) {
    let a = line(old, index);
    let b = line_mut(new, index);
    assert_eq!(a["text"], format!("+{amount} to Spirit"));
    assert_eq!(a["text"], b["text"]);
    assert_eq!(a["modifiers"], json!([]));
    assert_eq!(b["modifiers"], json!([]));
    invert_field(
        a,
        b,
        "outcome",
        pending("source_meaning_unresolved", &[]),
        pending("missing_context_option", &["fixed-spirit"]),
    );
    let a = line(&old["attribution"], index);
    let b = line_mut(&mut new["attribution"], index);
    invert_field(a, b, "rule", Value::Null, json!("fixed-spirit"));
    invert_field(
        a,
        b,
        "pending_candidates",
        json!([]),
        json!(["fixed-spirit"]),
    );
    independent_member(old, new, index, blockers, category, ordinal);
}
fn shifted_member(
    old: &Value,
    new: &mut Value,
    index: usize,
    oldordinal: usize,
    newordinal: usize,
) {
    invert_field(
        line(&old["attribution"], index),
        line_mut(&mut new["attribution"], index),
        "member",
        member("implicit", oldordinal, index),
        member("explicit", newordinal, index),
    );
}
#[allow(clippy::too_many_arguments)] // Exact source location and canonical record are checked together.
fn added_attribute(
    old: &Value,
    new: &mut Value,
    index: usize,
    rule: &str,
    number: u64,
    amount: i64,
    draft: &mut Value,
    item: &Value,
) -> (Value, Value) {
    let a = line(old, index);
    let b = line_mut(new, index);
    assert_eq!(a["outcome"], pending("source_meaning_unresolved", &[rule]));
    assert_eq!(a["modifiers"], json!([]));
    assert_eq!(rows(&b["modifiers"]).len(), 1);
    let id = b["modifiers"][0].clone();
    let def = definition("modifier", number);
    let rolls: Vec<_>=(1..=23).map(|offset| {
        let value=if offset==1 { json!({"kind":"quantity","value":{"value":amount as f64,"unit":definition("unit",0x295a)}}) }
        else if offset==23 { json!({"kind":"quantity","value":{"value":1.0,"unit":definition("unit",1)}}) }
        else { json!({"kind":"boolean","value":false}) };
        json!({"slot":{"declaration":{"kind":"modifier","definition":def},"slot":definition("parameter_slot",number+offset)},"value":value})
    }).collect();
    let closure = json!({"kind":"partial","value":{"gaps":[{"subject":{"kind":"definition","value":{"kind":"modifier","value":def}},"facet":"input_schema","code":"modifier-eligibility-inputs-unconverted"}]}});
    assert_eq!(
        b["outcome"],
        json!({"kind":"known","value":{"rule":rule,"emissions":[{"kind":"modifier","value":{"definition":def,"rolls":rolls,"rolls_closure":closure}}]}})
    );
    let owner = draft["draft"]["items"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| &r["id"] == item)
        .unwrap();
    let mods = owner["modifiers"]["members"].as_array_mut().unwrap();
    let matching: Vec<_> = mods
        .iter()
        .enumerate()
        .filter(|(_, m)| m["id"] == id)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(matching.len(), 1);
    let modifier = mods.remove(matching[0]);
    let issue = modifier["rolls"]["completion"]["id"].clone();
    assert_eq!(
        modifier,
        json!({"id":id,"definition":{"kind":"known","value":def},"rolls":{"members":rolls.iter().map(|r|json!({"slot":{"kind":"known","value":r["slot"]},"value":{"kind":"known","value":r["value"]}})).collect::<Vec<_>>(),"completion":{"kind":"pending","id":issue,"code":"modifier-roll-schema-partial"}}})
    );
    b["outcome"] = a["outcome"].clone();
    b["modifiers"] = json!([]);
    (id, issue)
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

pub fn compare_original(
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
    let xml = fs::read(root().join(format!(
        "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
    )))
    .unwrap();
    assert_eq!(sa["source_sha256"], hash(&xml));
    assert_eq!(sb["source_sha256"], hash(&xml));
    let mut additions = Vec::new();
    if case == 1 || case == 4 {
        let (
            ordinal,
            index,
            amount,
            attribute,
            number,
            rule,
            category,
            memberordinal,
            blockers,
            attribute_blockers,
        ) = if case == 1 {
            (
                284,
                19,
                34,
                29,
                0x298c,
                "fixed-intelligence",
                "implicit",
                2,
                vec!["unknown_member"],
                vec!["possible_combined_line"],
            )
        } else {
            (
                257,
                12,
                48,
                11,
                0x29a4,
                "fixed-all-attributes",
                "implicit",
                1,
                vec!["unknown_member", "unknown_header"],
                vec!["possible_combined_line", "unknown_header"],
            )
        };
        let item = preservation::link(&sb, ordinal, "item");
        let x = item_text(&sa, ordinal);
        let y = item_text_mut(&mut sb, ordinal);
        spirit_pending(x, y, index, amount, &blockers, category, memberordinal);
        let added = added_attribute(x, y, index + 1, rule, number, attribute, &mut b, &item);
        independent_member(
            x,
            y,
            index + 1,
            &attribute_blockers,
            "implicit",
            memberordinal + 1,
        );
        shifted_member(x, y, index + 2, memberordinal, 1);
        let links = preservation::origin_mut(&mut sb, ordinal)["links"]
            .as_array_mut()
            .unwrap();
        for expected in [
            json!({"kind":"modifier","value":added.0}),
            json!({"kind":"issue","value":added.1}),
        ] {
            assert_eq!(links.iter().filter(|v| **v == expected).count(), 1);
            links.retain(|v| *v != expected);
        }
        additions.push(added);
    }
    if case == 4 {
        let x = item_text(&sa, 219);
        let y = item_text_mut(&mut sb, 219);
        let a = line(x, 12);
        let b = line_mut(y, 12);
        assert_eq!(a["text"], "{enchant}{rune}+15 to Spirit");
        invert_field(
            a,
            b,
            "outcome",
            pending("source_meaning_unresolved", &[]),
            pending("source_meaning_unresolved", &["fixed-spirit"]),
        );
        let a = line(&x["attribution"], 12);
        let b = line_mut(&mut y["attribution"], 12);
        invert_field(a, b, "rule", Value::Null, json!("fixed-spirit"));
        invert_field(
            a,
            b,
            "pending_candidates",
            json!([]),
            json!(["fixed-spirit"]),
        );
        invert_field(
            a,
            b,
            "blockers",
            json!([
                "rune_lifecycle",
                "unknown_member",
                "unproved_tagged_line",
                "unknown_header"
            ]),
            json!([
                "rune_lifecycle",
                "unproved_member_conditions",
                "unproved_tagged_line",
                "unknown_header"
            ]),
        );
    }
    if case == 5 {
        let x = item_text(&sa, 469);
        let y = item_text_mut(&mut sb, 469);
        spirit_pending(x, y, 11, 49, &["unknown_member"], "implicit", 2);
        let a = line(x, 12);
        let b = line_mut(y, 12);
        assert_eq!(a["text"], "+2 to Level of all Minion Skills");
        invert_field(
            a,
            b,
            "outcome",
            pending("source_meaning_unresolved", &["fixed-global-minion-level"]),
            pending("missing_context_option", &["fixed-global-minion-level"]),
        );
        independent_member(x, y, 12, &["possible_combined_line"], "explicit", 1);
        shifted_member(x, y, 13, 2, 2);
    }
    assert_eq!(additions.len(), usize::from(case == 1 || case == 4));
    let count_modifiers = |v: &Value| {
        rows(&v["draft"]["items"]["members"])
            .iter()
            .map(|r| rows(&r["modifiers"]["members"]).len())
            .sum::<usize>()
    };
    let original_b = read(new.join("draft.json"));
    assert_eq!(
        count_modifiers(&original_b) - count_modifiers(&a),
        additions.len()
    );
    assert_eq!(
        rows(&original_b["draft"]["items"]["members"])
            .iter()
            .flat_map(|i| rows(&i["modifiers"]["members"]))
            .filter(|m| m["definition"]["value"] == definition("modifier", 0x314d))
            .count(),
        usize::from(case == 5),
        "fixed lines stay Pending; Original05's existing ranged Solar Spirit survives"
    );
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&b["draft"]["allocator"]) - issued(&a["draft"]["allocator"]),
        2 * additions.len() as u64
    );
    assert_eq!(sb["allocator_after"], original_b["draft"]["allocator"]);
    assert_eq!(sa["allocator_after"], a["draft"]["allocator"]);
    let oldlineage = a["draft"]["allocator"]["lineage"].clone();
    let newlineage = b["draft"]["allocator"]["lineage"].clone();
    b["draft"]["allocator"] = a["draft"]["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(&a, &mut b, &mut ids, "complete fixed-Spirit draft inverse");
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
        sb[field] = sa[field].clone();
    }
    for row in sb["item_texts"].as_array_mut().unwrap() {
        row["attribution"]["policy"] = json!(prior.receipt().item_source);
        row["attribution"]["item_lines"] = json!(prior.receipt().items);
    }
    // The already-restored allocator_after must not be relabelled twice.
    let restored_allocator = sb["allocator_after"].take();
    bare_lineages(&mut sb, &oldlineage, &newlineage);
    sb["allocator_after"] = restored_allocator;
    identity::correspond(
        &sa,
        &mut sb,
        &mut ids,
        "complete fixed-Spirit sidecar inverse",
    );
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
    for (r, s) in [(&before, &sa), (&after, &read(new.join("sidecar.json")))] {
        assert_eq!(r["intent_validation"]["schema_issues"], json!([]));
        assert_eq!(r["draft_digest"], s["draft"]);
        assert_eq!(r["finalization"]["draft_digest"], s["draft"]);
    }
    let oldissues = &before["finalization"]["issues"];
    let mut newissues = after["finalization"]["issues"].clone();
    let before_count = rows(oldissues).len();
    let after_count = rows(&newissues).len();
    assert_eq!(before_count, [106, 117, 109, 122, 5][case - 1]);
    for (_, issue) in &additions {
        let matching: Vec<_> = rows(&newissues)
            .iter()
            .filter(|r| r["id"] == *issue)
            .collect();
        assert_eq!(matching.len(), 1, "selected new attribute roll obligation");
        let (item_index, modifier_index) = if case == 1 { (2, 1) } else { (18, 0) };
        assert_eq!(
            matching[0],
            &json!({
                "id":issue,
                "owner":original_b["draft"]["items"]["members"][item_index]["id"],
                "path":format!("items.members[{item_index}].modifiers.members[{modifier_index}].rolls.completion"),
                "code":"modifier-roll-schema-partial"
            })
        );
        newissues
            .as_array_mut()
            .unwrap()
            .retain(|r| r["id"] != *issue);
    }
    if case == 1 || case == 4 {
        let (item_index, old_modifier_index) = if case == 1 { (2, 1) } else { (18, 0) };
        let prior_issue = &a["draft"]["items"]["members"][item_index]["modifiers"]["members"]
            [old_modifier_index]["rolls"]["completion"]["id"];
        let matching: Vec<_> = newissues
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter(|r| ids.get(&serde_json::to_string(&r["id"]).unwrap()) == Some(prior_issue))
            .collect();
        assert_eq!(matching.len(), 1, "exact existing Fire roll obligation");
        let r = matching.into_iter().next().unwrap();
        assert_eq!(
            r["owner"],
            original_b["draft"]["items"]["members"][item_index]["id"]
        );
        assert_eq!(r["code"], "modifier-roll-schema-partial");
        assert_eq!(
            r["path"],
            format!(
                "items.members[{item_index}].modifiers.members[{}].rolls.completion",
                old_modifier_index + 1
            )
        );
        r["path"] = json!(format!(
            "items.members[{item_index}].modifiers.members[{old_modifier_index}].rolls.completion"
        ));
    }
    identity::relocate(&mut newissues, &ids);
    assert_eq!(rows(&newissues).len(), rows(oldissues).len());
    for (i, (a, b)) in rows(oldissues).iter().zip(rows(&newissues)).enumerate() {
        assert_eq!(a, b, "unchanged original {case} selected issue {i}");
    }
    assert_eq!(after_count, before_count + additions.len());
    json!({"original":case,"known_attribute_modifiers_added":additions.len(),"spirit_modifiers_added":0,"selected_before":before_count,"selected_after":after_count,"saved_selection_preserved":true,"complete_draft_and_sidecar_inverse":true,"calculation":"not_run"})
}
