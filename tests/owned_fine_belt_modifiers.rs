//! Actual Fine Belt source admission; independent physical/static gaps remain open.
#[path = "support/owned_fine_belt_modifiers.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaState};
use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
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
fn publish(input: &Path, output: &Path) -> Value {
    let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
}
fn row(sidecar: &Value, ordinal: u64) -> &Value {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn line(text: &Value, index: u64) -> &Value {
    text["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["index"] == index)
        .unwrap()
}
fn belt_index(draft: &Value) -> usize {
    let found: Vec<_> = draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, v)| v["template"]["value"]["key"] == "def.0000000000001e84")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn input_value<'a>(modifier: &'a Value, slot: &impl Serialize) -> &'a Value {
    let slot = serde_json::to_value(slot).unwrap();
    let found: Vec<_> = modifier["rolls"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["slot"] == json!({"kind":"known","value":slot}))
        .collect();
    assert_eq!(found.len(), 1);
    &found[0]["value"]
}
fn check_modifier(m: &Value, b: &family::FamilyBindings, amount: f64, true_properties: &[&str]) {
    assert_eq!(m["definition"], json!({"kind":"known","value":b.modifier}));
    assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
    assert_eq!(
        m["rolls"]["members"].as_array().unwrap().len(),
        b.properties.len() + 3
    );
    assert_eq!(
        input_value(m, &b.amount),
        &json!({"kind":"known","value":{"kind":"quantity","value":{"value":amount,"unit":b.unit}}})
    );
    assert_eq!(
        input_value(m, &b.corrupted_base),
        &json!({"kind":"known","value":{"kind":"quantity","value":{"value":1.0,"unit":b.factor_unit}}})
    );
    assert_eq!(
        input_value(m, &b.category)["value"]["value"]["key"],
        "def.00000000000030e3"
    );
    for (property, slot) in &b.properties {
        assert_eq!(
            input_value(m, slot),
            &json!({"kind":"known","value":{"kind":"boolean","value":true_properties.contains(&property.as_str())}})
        );
    }
}
fn check_life(m: &Value) {
    assert_eq!(m["definition"]["value"]["key"], "def.0000000000003100");
    assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
    let rolls = m["rolls"]["members"].as_array().unwrap();
    assert_eq!(rolls.len(), 24);
    for r in rolls {
        assert_eq!(
            r["slot"]["value"]["declaration"]["definition"]["key"],
            "def.0000000000003100"
        );
        assert_eq!(r["slot"]["kind"], "known");
        assert_eq!(r["value"]["kind"], "known");
        let value = &r["value"]["value"];
        match r["slot"]["value"]["slot"]["key"].as_str().unwrap() {
            "def.0000000000003101" => {
                assert_eq!(value["kind"], "quantity");
                assert_eq!(value["value"]["value"], 10.0);
                assert_eq!(value["value"]["unit"]["key"], "def.000000000000295a");
            }
            "def.0000000000003117" => {
                assert_eq!(value["kind"], "quantity");
                assert_eq!(value["value"]["value"], 1.0);
                assert_eq!(value["value"]["unit"]["key"], "def.0000000000000001");
            }
            "def.0000000000003118" => {
                assert_eq!(value["kind"], "option");
                assert_eq!(value["value"]["key"], "def.00000000000030e2");
            }
            _ => assert_eq!(value, &json!({"kind":"boolean","value":false})),
        }
    }
}

#[test]
fn authored_belt_families_keep_exact_property_inputs_and_partial_owners() {
    let b = family::bindings();
    let e = family::extension();
    for (binding, count) in [(&b.charm, 25), (&b.flask, 24)] {
        assert_eq!(binding.properties.len() + 3, count);
        let d = e
            .schema
            .iter()
            .find_map(|v| match v {
                SchemaExtensionEntry::Definition(DefinitionDescriptor::Modifier(d))
                    if d.id == binding.modifier =>
                {
                    Some(d)
                }
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(d) = &d.schema else {
            panic!()
        };
        assert_eq!(d.declarations.parameters.members.len(), count);
        assert_eq!(d.declarations.parameters.closure, SchemaClosure::Complete);
        assert_eq!(binding.templates.len(), 1);
        assert_eq!(binding.templates[0].key().as_str(), "def.0000000000001e84");
    }
    assert!(b.charm.properties.contains_key("charm"));
    assert!(!b.flask.properties.contains_key("charm"));
    assert_eq!(b.charm.unit.key().as_str(), "def.000000000000295a");
    assert_eq!(b.flask.unit.key().as_str(), "def.00000000000031a5");
    assert_eq!(b.flask.effective.key().as_str(), "def.00000000000031a6");
    assert_ne!(b.flask.unit, b.charm.unit);
    assert_eq!(
        serde_json::to_value(family::source_property()).unwrap(),
        json!({"label":"charm","property":"charm"})
    );
    assert!(
        b.charm
            .properties
            .iter()
            .filter(|(name, _)| name.as_str() != "charm")
            .all(|(_, slot)| slot != &b.charm.properties["charm"])
    );
    assert!(
        e.owners
            .iter()
            .all(|v| matches!(v.programs.closure, SchemaClosure::Partial { .. }))
    );
    assert_eq!(family::item_rules().len(), 2);
    assert_eq!(family::source_conditions().len(), 2);
    assert_eq!(family::source_default().parameters.len(), 0);
    assert_eq!(
        serde_json::to_value(family::source_default().quality).unwrap(),
        "absent"
    );
    assert_eq!(
        serde_json::to_value(family::source_default().item_level).unwrap(),
        "absent"
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

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    assert_eq!(sa["schema_version"], 14);
    assert_eq!(sb["schema_version"], 14);
    let mut added = vec![];
    let mut retired = vec![];
    if case == 5 {
        let i = belt_index(&a);
        assert_eq!(i, belt_index(&b));
        let old_item = a["draft"]["items"]["members"][i].clone();
        let new_item = b["draft"]["items"]["members"][i].clone();
        assert!(
            old_item["modifiers"]["members"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let members = new_item["modifiers"]["members"].as_array().unwrap();
        assert_eq!(members.len(), 3);
        let bindings = family::bindings();
        check_modifier(&members[0], &bindings.charm, 2.0, &["charm"]);
        check_modifier(&members[1], &bindings.flask, 0.17, &[]);
        check_life(&members[2]);
        added = members.iter().map(|m| m["id"].clone()).collect();
        for field in ["item_level", "quality"] {
            assert_eq!(old_item[field]["kind"], "pending");
            assert_eq!(new_item[field], json!({"kind":"known","value":null}));
            retired.push(old_item[field]["id"].clone());
            a["draft"]["items"]["members"][i][field] = new_item[field].clone();
        }
        assert_eq!(new_item["parameters"]["completion"]["kind"], "pending");
        assert_eq!(new_item["modifiers"]["completion"]["kind"], "pending");
        assert_eq!(new_item["modifier_order"]["kind"], "pending");
        b["draft"]["items"]["members"][i]["modifiers"]["members"] = json!([]);
        let p = row(&sa, 590);
        let q = row(&sb, 590);
        assert_eq!(p["source"], q["source"]);
        assert_eq!(p["content_entry"], q["content_entry"]);
        assert_eq!(
            q["defaults"],
            json!({"parameters":[],"item_level_absent":true,"quality_absent":true})
        );
        assert_eq!(q["attribution"]["layout"], json!({"status":"proven"}));
        assert_eq!(
            q["attribution"]["default_scope"],
            json!({"kind":"proven","template":new_item["template"]["value"]})
        );
        assert_eq!(p["issues"], q["issues"]);
        assert!(q.get("parameter_inputs").is_none());
        assert_eq!(
            p["lines"].as_array().unwrap().len(),
            q["lines"].as_array().unwrap().len()
        );
        for (x, y) in p["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(q["lines"].as_array().unwrap())
        {
            assert_eq!(x["index"], y["index"]);
            assert_eq!(x["text"], y["text"]);
            if let Some(j) = [14, 15, 16].iter().position(|n| y["index"] == *n) {
                assert_eq!(x["outcome"]["kind"], "pending");
                assert!(x["modifiers"].as_array().unwrap().is_empty());
                assert_eq!(y["modifiers"], json!([added[j]]));
                assert_eq!(y["outcome"]["kind"], "known");
                assert_eq!(
                    y["outcome"]["value"]["rule"],
                    [
                        "ranged-charm-slots",
                        "fixed-flask-charges-per-second",
                        "fixed-life"
                    ][j]
                );
                let expected = &members[j];
                let emission = &y["outcome"]["value"]["emissions"][0]["value"];
                assert_eq!(emission["definition"], expected["definition"]["value"]);
                let rolls: Vec<_> = expected["rolls"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| json!({"slot":r["slot"]["value"],"value":r["value"]["value"]}))
                    .collect();
                assert_eq!(emission["rolls"], json!(rolls));
                assert!(emission.get("rolls_closure").is_none());
            } else {
                assert_eq!(x, y);
            }
        }
        // Header is an existing observation, not the new implicit property.
        assert_eq!(line(p, 4), line(q, 4));
        assert_eq!(
            line(q, 4)["outcome"]["value"]["rule"],
            "observed-charm-slots"
        );
        for (index, category, ordinal) in [
            (14, "implicit", 1),
            (15, "implicit", 2),
            (16, "explicit", 1),
        ] {
            let attr = q["attribution"]["lines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["index"] == index)
                .unwrap();
            assert_eq!(
                attr["member"],
                json!({"category":category,"ordinal":ordinal,"line":index})
            );
            assert_eq!(attr["blockers"], json!([]));
        }
        assert_eq!(
            p["attribution"]["lines"].as_array().unwrap().len(),
            q["attribution"]["lines"].as_array().unwrap().len()
        );
        for (old_attr, new_attr) in p["attribution"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(q["attribution"]["lines"].as_array().unwrap())
        {
            if old_attr["index"].as_u64().unwrap() < 14 {
                assert_eq!(old_attr["range"], json!({"status":"pending"}));
                assert_eq!(new_attr["range"], json!({"status":"absent"}));
                let mut restored = new_attr.clone();
                restored["range"] = old_attr["range"].clone();
                assert_eq!(
                    old_attr, &restored,
                    "nonmember range absence is the only attribution change"
                );
                continue;
            }
            for field in [
                "index",
                "decoded_span",
                "raw",
                "semantic_text",
                "presentation",
            ] {
                assert_eq!(old_attr[field], new_attr[field]);
            }
            let old_tokens = old_attr["property_tokens"].as_array().unwrap();
            let new_tokens = new_attr["property_tokens"].as_array().unwrap();
            assert_eq!(old_tokens.len(), new_tokens.len());
            for (old_token, new_token) in old_tokens.iter().zip(new_tokens) {
                assert_eq!(old_token["label"], "charm");
                assert_eq!(old_token["property"], Value::Null);
                assert_eq!(old_token["label"], new_token["label"]);
                assert_eq!(old_token["decoded_span"], new_token["decoded_span"]);
                assert_eq!(
                    new_token["property"],
                    serde_json::to_value(family::source_property()).unwrap()["property"]
                );
            }
            let index = new_attr["index"].as_u64().unwrap();
            let rule = match index {
                14 => "ranged-charm-slots",
                15 => "fixed-flask-charges-per-second",
                16 => "fixed-life",
                _ => panic!(),
            };
            assert_eq!(new_attr["rule"], rule);
            assert_eq!(new_attr["pending_candidates"], json!([rule]));
            assert_eq!(old_attr["range"], json!({"status":"pending"}));
            assert_eq!(
                new_attr["range"],
                json!({"status":"resolved","fraction":0.5,"winning_write":index-13})
            );
            let flags = if index == 14 {
                &bindings.charm.properties
            } else {
                &bindings.flask.properties
            };
            let expected: serde_json::Map<String, Value> = flags
                .keys()
                .filter(|k| k.as_str() != "unscalable")
                .map(|k| (k.clone(), json!(index == 14 && k == "charm")))
                .collect();
            assert_eq!(new_attr["properties"], Value::Object(expected));
            let mut restored = new_attr.clone();
            for field in [
                "rule",
                "pending_candidates",
                "member",
                "blockers",
                "range",
                "property_tokens",
                "properties",
            ] {
                restored[field] = old_attr[field].clone();
            }
            assert_eq!(
                old_attr, &restored,
                "only checked category/range/property attribution changes"
            );
        }
        let old_writes = p["attribution"]["writes"].as_array().unwrap();
        let new_writes = q["attribution"]["writes"].as_array().unwrap();
        assert_eq!(old_writes.len(), new_writes.len());
        for (old_write, new_write) in old_writes.iter().zip(new_writes) {
            let mut restored = new_write.clone();
            if let Some(id) = new_write["source_id"].as_u64() {
                assert!((1..=3).contains(&id));
                assert_eq!(new_write["target"], json!({"status":"line","value":13+id}));
                assert_eq!(old_write["target"], json!({"status":"pending"}));
                restored["target"] = old_write["target"].clone();
            }
            assert_eq!(old_write, &restored);
        }
        let target = sb["item_texts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["source"]["ordinal"] == 590)
            .unwrap();
        *target = p.clone();
    }
    let before_allocator = a["draft"]["allocator"].clone();
    let after_allocator = b["draft"]["allocator"].clone();
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&after_allocator) - issued(&before_allocator),
        u64::from(case == 5)
    );
    b["draft"]["allocator"] = before_allocator.clone();
    let mut ids = BTreeMap::new();
    identity::correspond(&a, &mut b, &mut ids, "all other canonical inputs");
    let mut old_removed = 0;
    let mut new_removed = 0;
    for origin in sa["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|l| {
            let remove = l["kind"] == "issue" && retired.contains(&l["value"]);
            old_removed += usize::from(remove);
            !remove
        });
    }
    for origin in sb["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|l| {
            let remove = l["kind"] == "modifier" && added.contains(&l["value"]);
            new_removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(old_removed, retired.len());
    assert_eq!(new_removed, added.len());
    for field in [
        "draft",
        "definitions",
        "registry",
        "mapping",
        "policy",
        "item_policy",
        "item_source_policy",
        "skill_roles",
        "reward_policy",
        "tree_policy",
        "allocator_after",
    ] {
        sb[field] = sa[field].clone();
    }
    for (p, q) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array_mut().unwrap())
    {
        for field in ["policy", "item_lines"] {
            q["attribution"][field] = p["attribution"][field].clone();
        }
    }
    identity::correspond(&sa, &mut sb, &mut ids, "all other source syntax/evidence");
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
    let mut actual = after["finalization"]["issues"].clone();
    selected::canonical(&mut expected);
    selected::canonical(&mut actual);
    expected
        .as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(&v["id"]));
    identity::relocate(&mut actual, &ids);
    assert_eq!(
        expected, actual,
        "only two proven Fine Belt absence obligations retire"
    );
    let mut selection_before = selected::selection(xml, old);
    let mut selection_after = selected::selection(xml, new);
    selected::canonical(&mut selection_before);
    selected::canonical(&mut selection_after);
    identity::relocate(&mut selection_after, &ids);
    assert_eq!(selection_before, selection_after);
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count - after_count, if case == 5 { 2 } else { 0 });
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"new_modifiers":added,"retired":retired,"allocator_before":before_allocator,"allocator_after":after_allocator,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"27\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let charm = "{tags:charm}{range:0.5}Has (1-3) Charm Slot";
    let flask = "Flasks gain 0.17 charges per Second";
    let run = |label: &str, changed: String| {
        assert_ne!(changed, raw, "{label}");
        let xml = out.join(format!("probe-{label}.xml"));
        fs::write(
            &xml,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &xml, 5, &dest);
        let d: Value = read(dest.join("draft.json"));
        let s: Value = read(dest.join("sidecar.json"));
        (
            d["draft"]["items"]["members"][belt_index(&d)].clone(),
            row(&s, 590).clone(),
        )
    };
    let negatives = [
        (
            "malformed-charm-header",
            raw.replace("Charm Slots: 2", "Charm Slots: Nope"),
        ),
        (
            "zero-charm-range",
            raw.replace("Has (1-3) Charm Slot", "Has (0-0) Charm Slot"),
        ),
        (
            "zero-charm-lower",
            raw.replace("Has (1-3) Charm Slot", "Has (0-3) Charm Slot"),
        ),
        (
            "unknown-precursor",
            raw.replace(charm, &format!("Unknown source semantics\n{charm}")),
        ),
        (
            "unknown-charm-property",
            raw.replace("{tags:charm}", "{tags:unreviewed_property}"),
        ),
        (
            "desecrated-charm",
            raw.replace(charm, &format!("{{desecrated}}{charm}")),
        ),
        (
            "scaled-charm",
            raw.replace(charm, &format!("{{corruptedRange:1.5}}{charm}")),
        ),
        (
            "variant-charm",
            raw.replace(charm, &format!("{{variant:1}}{charm}")),
        ),
        (
            "invalid-flask-number",
            raw.replace(flask, "Flasks gain NaN charges per Second"),
        ),
        (
            "scaled-flask",
            raw.replace(flask, &format!("{{corruptedRange:1.5}}{flask}")),
        ),
        (
            "malformed-overlay",
            raw.replace("range=\"0.5\" id=\"1\"", "range=\"bad\" id=\"1\""),
        ),
    ];
    for (label, changed) in &negatives {
        let (item, text) = run(label, changed.clone());
        assert_eq!(
            text["attribution"]["layout"]["status"], "pending",
            "{label}"
        );
        assert_eq!(text["defaults"]["item_level_absent"], false, "{label}");
        assert_eq!(text["defaults"]["quality_absent"], false, "{label}");
        assert_eq!(item["item_level"]["kind"], "pending", "{label}");
        assert_eq!(item["quality"]["kind"], "pending", "{label}");
        assert_eq!(
            item["parameters"]["completion"]["kind"], "pending",
            "{label}"
        );
        assert_eq!(
            item["modifiers"]["completion"]["kind"], "pending",
            "{label}"
        );
        assert_eq!(item["modifier_order"]["kind"], "pending", "{label}");
    }
    let b = family::bindings();
    let positive = [
        ("legacy-zero", "0", 1.0),
        ("legacy-one", "1", 3.0),
        ("legacy-quarter", "0.25", 1.5),
    ];
    for (label, range, amount) in positive {
        let (item, text) = run(
            label,
            raw.replace(
                "range=\"0.5\" id=\"1\"",
                &format!("range=\"{range}\" id=\"1\""),
            ),
        );
        assert_eq!(text["attribution"]["layout"]["status"], "proven");
        assert_eq!(item["modifiers"]["members"].as_array().unwrap().len(), 3);
        check_modifier(
            &item["modifiers"]["members"][0],
            &b.charm,
            amount,
            &["charm"],
        );
        check_modifier(&item["modifiers"]["members"][1], &b.flask, 0.17, &[]);
        check_life(&item["modifiers"]["members"][2]);
        assert_eq!(line(&text, 4)["text"], "Charm Slots: 2");
        assert_eq!(
            line(&text, 4)["outcome"]["value"]["rule"],
            "observed-charm-slots"
        );
        assert_eq!(item["item_level"], json!({"kind":"known","value":null}));
        assert_eq!(item["quality"], json!({"kind":"known","value":null}));
        assert_eq!(item["modifiers"]["completion"]["kind"], "pending");
    }
    let (item, text) = run(
        "explicit-item-level",
        raw.replace("LevelReq: 62", "Item Level: 77\nLevelReq: 62"),
    );
    assert_eq!(item["item_level"], json!({"kind":"known","value":77}));
    assert_eq!(text["defaults"]["item_level_absent"], false);
    assert_eq!(text["defaults"]["quality_absent"], true);
    let (item, text) = run(
        "fractional-flask",
        raw.replace(flask, "Flasks gain 0.125 charges per Second"),
    );
    assert_eq!(text["attribution"]["layout"]["status"], "proven");
    check_modifier(&item["modifiers"]["members"][1], &b.flask, 0.125, &[]);
    for (label, changed) in [
        (
            "changed-charm-header",
            raw.replace("Charm Slots: 2", "Charm Slots: 9"),
        ),
        ("absent-charm-header", raw.replace("Charm Slots: 2\n", "")),
    ] {
        let (item, text) = run(label, changed);
        assert_eq!(text["attribution"]["layout"]["status"], "proven");
        check_modifier(&item["modifiers"]["members"][0], &b.charm, 2.0, &["charm"]);
        assert_eq!(item["item_level"], json!({"kind":"known","value":null}));
        assert_eq!(item["quality"], json!({"kind":"known","value":null}));
        assert_eq!(item["parameters"]["completion"]["kind"], "pending");
        assert_eq!(item["modifiers"]["completion"]["kind"], "pending");
        assert_eq!(item["modifier_order"]["kind"], "pending");
    }
    let qualities = [
        ("quality-zero", "Quality: 0"),
        ("quality-twenty", "Quality: 20"),
        ("quality-malformed", "Quality: NaN"),
        ("quality-alias", "Quality (Mana Modifiers): +20%"),
        ("quality-duplicate", "Quality: 0\nQuality: 20"),
    ];
    for (label, header) in qualities {
        let (item, text) = run(
            label,
            raw.replace("LevelReq: 62", &format!("{header}\nLevelReq: 62")),
        );
        assert_eq!(text["defaults"]["quality_absent"], false, "{label}");
        assert_ne!(
            item["quality"],
            json!({"kind":"known","value":null}),
            "{label}"
        );
    }
    negatives.len() + positive.len() + 4 + qualities.len()
}

#[test]
#[ignore = "requires the exact checked Sapphire predecessor"]
fn fine_belt_layout_preserves_every_original_request_and_independent_gap() {
    let prior_dir = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FINE_BELT_PRIOR").expect("explicit prior"),
    );
    let inventory = release::inventory(&prior_dir);
    let prior = release::load(&prior_dir);
    let next = family::stage(&prior);
    family::preservation(&prior, &next);
    assert!(next.input().evaluation.is_none());
    assert_eq!(next.receipt().query_rows, 110);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_FINE_BELT_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(src, dst),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
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
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":110,"originals":reports,"probes":probe_count,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
