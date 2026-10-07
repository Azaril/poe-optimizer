//! Authenticated source observations and complete import inverses for global ES.
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
fn physical_line<'a>(snapshot: &'a Value, text: &str) -> (&'a str, &'a Value) {
    let matches: Vec<_> = snapshot["lists"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(category, lines)| rows(lines).iter().map(move |r| (category.as_str(), r)))
        .filter(|(_, r)| r["line"] == text)
        .collect();
    assert_eq!(matches.len(), 1, "exact physical source line {text}");
    matches[0]
}
fn es_record(record: &Value, amount: f64, global: bool) {
    assert_eq!(record["name"], "EnergyShield");
    assert_eq!(record["type"], "INC");
    assert_eq!(record["value"].as_f64(), Some(amount));
    assert_eq!(record["flags"], 0);
    assert_eq!(record["keyword_flags"], 0);
    if global {
        assert_eq!(record["tags"], json!([{"type":"Global"}]))
    } else {
        assert!(rows(&record["tags"]).is_empty())
    }
}
fn source_observations(v: &Value) {
    let builds = rows(&v["report"]["builds"]);
    assert_eq!(builds.len(), 2);
    assert_eq!(rows(&v["xml_pins"]).len(), 2);
    for (index, build) in builds.iter().enumerate() {
        let original = index + 4;
        assert_eq!(build["name"], format!("original-{original:02}"));
        assert_eq!(
            v["xml_pins"][index]["path"],
            format!("tests/fixtures/builds/breadth-20260908/build-{original:02}.xml")
        );
        assert_eq!(build["xml_sha256"], v["xml_pins"][index]["sha256"]);
        let state = &build["state"];
        for key in [
            "observed_item_fields_preserved",
            "saved_selections_preserved",
            "main_scalar_output_preserved",
            "player_contributions_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[key], true);
        }
        assert_eq!(state["scalability"], json!([{"isScalable":true}]));
        let expected: Vec<_> = if original == 4 {
            vec![(19, "Gold Amulet", 44., "+68 to maximum Life")]
        } else {
            vec![
                (17, "Sapphire", 20., "11% increased Critical Hit Chance"),
                (
                    18,
                    "Sapphire",
                    16.,
                    "14% faster start of Energy Shield Recharge",
                ),
            ]
        };
        assert_eq!(rows(&state["cases"]).len(), expected.len());
        for (case, (id, base, amount, next_line)) in rows(&state["cases"]).iter().zip(expected) {
            assert_eq!(case["item_id"], id);
            assert_eq!(case["base"], base);
            assert_eq!(
                case["line"],
                format!("{amount:.0}% increased maximum Energy Shield")
            );
            assert_eq!(case["next_line"], next_line);
            assert_eq!(case["base_facts"]["has_buff"], false);
            assert_eq!(case["player_participating"], original == 4);
            assert_eq!(case["loaded"], case["fresh"]);
            for phase in ["loaded", "fresh_before", "fresh"] {
                let (category, line) = physical_line(&case[phase], case["line"].as_str().unwrap());
                assert_eq!(category, "explicit");
                assert_eq!(line["field_types"]["extra"], "nil");
                assert_eq!(rows(&line["records"]).len(), 1);
                es_record(&line["records"][0], amount, true);
                assert!(line.get("modTags").is_none_or(|v| rows(v).is_empty()));
                assert_eq!(line["catalyst_factor"].as_f64(), Some(1.));
                let (_, successor) = physical_line(&case[phase], next_line);
                assert_eq!(successor["field_types"]["extra"], "nil");
                assert!(!rows(&successor["records"]).is_empty());
            }
            assert_eq!(rows(&case["fresh"]["active"]).len(), 1);
            es_record(&case["fresh"]["active"][0], amount, true);
            if original == 4 {
                assert_eq!(case["selected_slots"], json!(["Amulet"]));
                assert_eq!(case["source_player_records"], case["fresh"]["active"])
            } else {
                assert!(rows(&case["selected_slots"]).is_empty());
                assert!(rows(&case["source_player_records"]).is_empty());
            }
        }
        if original == 5 {
            assert_eq!(state["selected"]["spec"], 3);
        }
        // Directly checked source results; this table is not a source tag or
        // formatter interpreter and does not broaden the owned input grammar.
        let expected = [
            ("integer-0", Some(0.)),
            ("integer-1", Some(1.)),
            ("integer-16", Some(16.)),
            ("integer-20", Some(20.)),
            ("integer-44", Some(44.)),
            ("integer-1000000", Some(1_000_000.)),
            ("integer-1000001", Some(1_000_001.)),
            ("leading-zero", Some(44.)),
            ("untagged-carapace", Some(44.)),
            ("untagged-negative-quality", Some(44.)),
            ("tagged-carapace", Some(52.)),
            ("decimal", Some(11.)),
            ("reduced", Some(-11.)),
            ("negative", Some(-11.)),
            ("ranged", Some(15.)),
            ("corrupted-range", Some(17.)),
            ("disabled", None),
            ("unknown-predecessor", Some(44.)),
            ("ordinary-magnitude", Some(25.)),
            ("crafted-magnitude", Some(30.)),
            ("crafted-cancellation-up-down", Some(30.)),
            ("crafted-cancellation-down-up", Some(20.)),
            ("armour-local", None),
            ("armour-global", Some(11.)),
        ];
        assert_eq!(rows(&state["constructors"]).len(), expected.len());
        for (probe, (name, amount)) in rows(&state["constructors"]).iter().zip(expected) {
            assert_eq!(probe["name"], name);
            if let Some(amount) = amount {
                assert_eq!(rows(&probe["after"]["active"]).len(), 1);
                es_record(
                    &probe["after"]["active"][0],
                    amount,
                    !matches!(name, "reduced" | "negative"),
                );
            } else {
                assert!(rows(&probe["after"]["active"]).is_empty());
            }
        }
        let constructors = rows(&state["constructors"]);
        let tagged = physical_line(
            &constructors[10]["after"],
            "44% increased maximum Energy Shield",
        )
        .1;
        assert_eq!(tagged["modTags"], json!(["energyshield"]));
        assert_eq!(tagged["catalyst_factor"].as_f64(), Some(1.2));
        assert_eq!(constructors[22]["after"]["armour_data"]["EnergyShield"], 30);
        assert_eq!(constructors[23]["after"]["armour_data"]["EnergyShield"], 27);
        let formats = [
            ("quantize-before-magnitude", 13.),
            ("magnitude-truncation", 5.),
            ("corruption-rounding", 6.),
            ("all-stages", 7.),
            ("source-quality-grouping", 1007.),
            ("reassociated-quality-contrast", 1006.),
        ];
        assert_eq!(rows(&state["formats"]).len(), formats.len());
        for (row, (name, amount)) in rows(&state["formats"]).iter().zip(formats) {
            assert_eq!(row["name"], name);
            assert_eq!(row["extra_type"], "nil");
            assert_eq!(rows(&row["records"]).len(), 1);
            es_record(&row["records"][0], amount, true);
        }
        let direct = rows(&state["direct_parses"]);
        assert_eq!(direct.len(), 5);
        assert_eq!(rows(&direct[0]["records"]).len(), 1);
        es_record(&direct[0]["records"][0], 44., true);
        assert_eq!(rows(&direct[1]["records"]).len(), 1);
        es_record(&direct[1]["records"][0], 11., false);
        assert!(rows(&direct[4]["records"]).is_empty() || direct[4]["extra_type"] != "nil");
    }
}

pub fn verify_source(full: bool) {
    let v = read(
        root().join("data/owned/poe2/3887ae68/global-energy-shield-inputs/source-vectors.json"),
    );
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["scope"],
        json!({"retained_source_witness":true,"new_source_execution":false,"native_parity":false,"whole_item_inventory":false,"whole_contributor_coverage":false})
    );
    for (path, expected) in v["pins"].as_object().unwrap() {
        pin(&root().join(path), expected);
    }
    for expected in rows(&v["xml_pins"]) {
        pin(&root().join(expected["path"].as_str().unwrap()), expected);
    }
    let report = &v["report"];
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
                .any(|entry| entry["path"] == p["path"] && entry["sha256"] == p["sha256"])
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
    source_observations(&v);
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
const GLOBAL_RULE: &str = "fixed-global-energy-shield-increase";
fn pending(reason: &str, candidates: &[&str]) -> Value {
    let reason = if reason == "missing_context_option" {
        json!({"kind":reason,"value":{"input":"modifier-source-category"}})
    } else {
        json!({"kind":reason})
    };
    json!({"kind":"pending","value":{"reason":reason,"candidates":candidates}})
}
fn item_text(v: &Value, ordinal: usize) -> &Value {
    let matching: Vec<_> = rows(&v["item_texts"])
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["content_entry"], 0);
    matching[0]
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
fn invert(old: &Value, new: &mut Value, key: &str, a: Value, b: Value) {
    assert_eq!(old[key], a, "before field {key}");
    assert_eq!(new[key], b, "after field {key}");
    new[key] = a;
}
fn properties(critical: bool) -> Value {
    let mut object = serde_json::Map::new();
    for name in [
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
        object.insert(name.into(), json!(false));
    }
    if critical {
        for name in [
            "critical",
            "damage",
            "desecrated",
            "fractured",
            "physical_damage",
        ] {
            object.insert(name.into(), json!(false));
        }
    }
    Value::Object(object)
}
fn member(category: &str, ordinal: usize, index: usize) -> Value {
    json!({"category":category,"ordinal":ordinal,"line":index})
}
fn member_change(x: &Value, y: &mut Value, index: usize, before: Value, after: Value) {
    invert(
        line(&x["attribution"], index),
        line_mut(&mut y["attribution"], index),
        "member",
        before,
        after,
    );
}
fn blocker_change(x: &Value, y: &mut Value, index: usize, before: &[&str], after: &[&str]) {
    invert(
        line(&x["attribution"], index),
        line_mut(&mut y["attribution"], index),
        "blockers",
        json!(before),
        json!(after),
    );
}
fn properties_added(x: &Value, y: &mut Value, index: usize, critical: bool) {
    invert(
        line(&x["attribution"], index),
        line_mut(&mut y["attribution"], index),
        "properties",
        json!({}),
        properties(critical),
    );
}
fn new_global_candidate(x: &Value, y: &mut Value, index: usize, text: &str, reason: &str) {
    let a = line(x, index);
    let b = line_mut(y, index);
    assert_eq!(a["text"], text);
    assert_eq!(b["text"], text);
    assert_eq!(a["modifiers"], json!([]));
    assert_eq!(b["modifiers"], json!([]));
    invert(
        a,
        b,
        "outcome",
        pending("source_meaning_unresolved", &[]),
        pending(reason, &[GLOBAL_RULE]),
    );
    let a = line(&x["attribution"], index);
    let b = line_mut(&mut y["attribution"], index);
    invert(a, b, "rule", Value::Null, json!(GLOBAL_RULE));
    invert(a, b, "pending_candidates", json!([]), json!([GLOBAL_RULE]));
}
fn definition(kind: &str, number: u64) -> Value {
    json!({"kind":kind,"namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":format!("def.{number:016x}")})
}
fn critical_emission() -> Value {
    let modifier = definition("modifier", 0x25aa);
    let rolls:Vec<_>=(0x25ab..=0x25c6).chain([0x2678]).map(|slot| {
        let value=match slot {
            0x25ab=>json!({"kind":"quantity","value":{"value":11.,"unit":definition("unit",2)}}),
            0x2678=>json!({"kind":"quantity","value":{"value":1.,"unit":definition("unit",1)}}),
            _=>json!({"kind":"boolean","value":false})
        };
        json!({"slot":{"declaration":{"kind":"modifier","definition":modifier},"slot":definition("parameter_slot",slot)},"value":value})
    }).collect();
    assert_eq!(rolls.len(), 29);
    json!({"kind":"known","value":{"rule":"local-critical-increase-increase-fixed","emissions":[{"kind":"modifier","value":{"definition":modifier,"rolls":rolls,"rolls_closure":{"kind":"partial","value":{"gaps":[{"subject":{"kind":"definition","value":{"kind":"modifier","value":modifier}},"facet":"input_schema","code":"modifier-eligibility-inputs-unconverted"}]}}}}]}})
}
fn inverse_diagnostics(case: usize, old: &Value, new: &mut Value) {
    if case == 1 || case == 3 {
        let (ordinal, index, text, header) = if case == 1 {
            (
                305,
                10,
                "{desecrated}56% increased maximum Energy Shield",
                true,
            )
        } else {
            (
                272,
                14,
                "{desecrated}52% increased maximum Energy Shield",
                false,
            )
        };
        let x = item_text(old, ordinal);
        let y = item_text_mut(new, ordinal);
        new_global_candidate(x, y, index, text, "source_meaning_unresolved");
        let mut before = vec!["unknown_member", "unproved_tagged_line"];
        let mut after = vec!["unproved_member_conditions", "unproved_tagged_line"];
        if header {
            before.push("unknown_header");
            after.push("unknown_header");
        }
        blocker_change(x, y, index, &before, &after);
    }
    if case == 4 {
        let x = item_text(old, 257);
        let y = item_text_mut(new, 257);
        new_global_candidate(
            x,
            y,
            10,
            "44% increased maximum Energy Shield",
            "missing_context_option",
        );
        blocker_change(x, y, 10, &["unknown_member", "unknown_header"], &[]);
        properties_added(x, y, 10, false);
        member_change(x, y, 10, Value::Null, member("implicit", 1, 10));
        let a = line(x, 11);
        let b = line_mut(y, 11);
        assert_eq!(a["text"], "+68 to maximum Life");
        assert_eq!(b["modifiers"], json!([]));
        invert(
            a,
            b,
            "outcome",
            pending("source_meaning_unresolved", &["fixed-life"]),
            pending("missing_context_option", &["fixed-life"]),
        );
        blocker_change(x, y, 11, &["possible_combined_line", "unknown_header"], &[]);
        properties_added(x, y, 11, false);
        member_change(x, y, 11, Value::Null, member("implicit", 2, 11));
        member_change(
            x,
            y,
            12,
            member("implicit", 1, 12),
            member("explicit", 1, 12),
        );
        member_change(
            x,
            y,
            13,
            member("implicit", 2, 13),
            member("explicit", 2, 13),
        );
        member_change(
            x,
            y,
            14,
            member("explicit", 1, 14),
            member("explicit", 3, 14),
        );
    }
    if case == 5 {
        let x = item_text(old, 562);
        let y = item_text_mut(new, 562);
        new_global_candidate(
            x,
            y,
            9,
            "20% increased maximum Energy Shield",
            "source_meaning_unresolved",
        );
        blocker_change(
            x,
            y,
            9,
            &["possible_combined_line", "unknown_member", "unknown_header"],
            &["possible_combined_line", "unknown_header"],
        );
        let a = line(x, 10);
        let b = line_mut(y, 10);
        assert_eq!(a["text"], "11% increased Critical Hit Chance");
        assert_eq!(a["modifiers"], json!([]));
        assert_eq!(b["modifiers"], json!([]));
        invert(
            a,
            b,
            "outcome",
            pending(
                "source_meaning_unresolved",
                &["local-critical-increase-increase-fixed"],
            ),
            critical_emission(),
        );
        invert(
            x,
            y,
            "issues",
            json!([{"problem":"schema_partial","lines":[]}]),
            json!([{"problem":"schema_partial","lines":[10]},{"problem":"schema_partial","lines":[]} ]),
        );
        member_change(x, y, 10, Value::Null, member("explicit", 1, 10));
        blocker_change(x, y, 10, &["possible_combined_line", "unknown_header"], &[]);
        properties_added(x, y, 10, true);
        blocker_change(
            x,
            y,
            11,
            &["unknown_member", "unknown_header"],
            &["unknown_member"],
        );
        let x = item_text(old, 567);
        let y = item_text_mut(new, 567);
        new_global_candidate(
            x,
            y,
            8,
            "16% increased maximum Energy Shield",
            "missing_context_option",
        );
        member_change(x, y, 8, Value::Null, member("explicit", 1, 8));
        blocker_change(x, y, 8, &["unknown_member", "unknown_header"], &[]);
        properties_added(x, y, 8, false);
        blocker_change(
            x,
            y,
            9,
            &["possible_combined_line", "unknown_member", "unknown_header"],
            &["unknown_member"],
        );
        for index in [10, 11] {
            blocker_change(
                x,
                y,
                index,
                &["possible_combined_line", "unknown_member", "unknown_header"],
                &["possible_combined_line", "unknown_member"],
            );
        }
        invert(
            &x["attribution"]["layout"],
            &mut y["attribution"]["layout"],
            "problems",
            json!(["unknown_member", "unknown_header", "possible_combined_line"]),
            json!(["unknown_member", "possible_combined_line"]),
        );
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
    let oldlineage = a["draft"]["allocator"]["lineage"].clone();
    let newlineage = b["draft"]["allocator"]["lineage"].clone();
    assert_eq!(
        b["draft"]["allocator"]["last_issued"], a["draft"]["allocator"]["last_issued"],
        "no new canonical occurrence or obligation"
    );
    b["draft"]["allocator"] = a["draft"]["allocator"].clone();
    let mut ids = BTreeMap::new();
    inverse_diagnostics(case, &sa, &mut sb);
    identity::correspond(&a, &mut b, &mut ids, "whole global ES draft inverse");
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
    let restored = sb["allocator_after"].take();
    bare_lineages(&mut sb, &oldlineage, &newlineage);
    sb["allocator_after"] = restored;
    identity::correspond(&sa, &mut sb, &mut ids, "whole global ES sidecar inverse");
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
    for (report, sidecar) in [(&before, &sa), (&after, &read(new.join("sidecar.json")))] {
        assert_eq!(report["intent_validation"]["schema_issues"], json!([]));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    let oldissues = &before["finalization"]["issues"];
    let mut newissues = after["finalization"]["issues"].clone();
    identity::relocate(&mut newissues, &ids);
    assert_eq!(rows(oldissues).len(), [107, 117, 109, 123, 5][case - 1]);
    assert_eq!(rows(&newissues).len(), rows(oldissues).len());
    for (i, (a, b)) in rows(oldissues).iter().zip(rows(&newissues)).enumerate() {
        assert_eq!(a, b, "original{case} selected issue{i}");
    }
    json!({"original":case,"selected_before":rows(oldissues).len(),"selected_after":rows(&newissues).len(),"global_es_modifiers_added":0,"lexical_critical_emissions_added":usize::from(case==5),"saved_selection_preserved":true,"complete_draft_and_sidecar_inverse":true,"calculation":"not_run"})
}
