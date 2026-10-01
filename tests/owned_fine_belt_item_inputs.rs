//! Complete the reviewed Fine Belt census and physical inputs; static coverage stays Partial.
#[path = "support/owned_fine_belt_item_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_schema::{
    DefinitionDescriptor, SchemaClosure, SchemaState, SlotDescriptor, SlotPresence,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::assemble_owned_release,
    owned_value::ValueCodecKind,
};
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
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
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
#[test]
fn authored_fine_belt_inputs_reuse_existing_types_and_keep_static_coverage_open() {
    let e = family::extension();
    let bindings = family::bindings();
    assert_eq!(e.schema.len(), 7);
    assert_eq!(bindings.len(), 1);
    assert_eq!(e.owners.len(), 1);
    assert!(e.tables.is_empty() && e.receivers.is_empty());
    assert!(e.operations_version.is_none());
    let membership = family::membership();
    assert!(
        membership
            .modifier_rules
            .iter()
            .map(|v| v.as_str())
            .eq(["ranged-charm-slots", "fixed-flask-charges-per-second"])
    );
    assert_eq!(membership.census_templates.len(), 1);
    assert_eq!(membership.census_templates[0].implicit_members, 2);
    assert_eq!(membership.census_templates[0].explicit_members, 1);
    assert_eq!(
        membership.census_templates[0].template,
        bindings[0].template
    );
    assert_eq!(
        serde_json::to_value(membership.census_templates[0].generated_members).unwrap(),
        "no_buff_enchant_rune_or_class_members"
    );
    assert!(e.owners.iter().all(
        |o| matches!(o.programs.closure, SchemaClosure::Partial { .. })
            && o.programs.members.len() == 1
            && o.programs.members[0].id.as_str() == "catalyst-inputs"
    ));
    for b in bindings {
        assert_eq!(
            serde_json::to_value(b.construction).unwrap(),
            json!({"fresh_rare_saved_category_census_v3":{"derived_observations":[{"rule":"observed-charm-slots","capture":"amount","kind":"charm_slots"}]}})
        );
        assert_eq!(b.header_inputs.len(), 2);
        assert_eq!(b.header_inputs[0].rule.as_str(), "rarity");
        assert_eq!(b.header_inputs[1].rule.as_str(), "level-requirement");
        let ValueCodecKind::Option { tokens } = &b.header_inputs[0].codec.codec else {
            panic!()
        };
        assert_eq!(
            tokens.iter().map(|t| t.token.as_str()).collect::<Vec<_>>(),
            ["NORMAL", "MAGIC", "RARE", "UNIQUE", "RELIC"]
        );
        let slots = [
            &b.header_inputs[0].slot,
            &b.corruption_slot,
            &b.header_inputs[1].slot,
            &b.capacity_slot,
        ];
        for (i, slot) in slots.iter().enumerate() {
            let d = e
                .schema
                .iter()
                .find_map(|d| match d {
                    SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(d)) if &d.id == *slot => {
                        Some(d)
                    }
                    _ => None,
                })
                .unwrap();
            let SchemaState::Known(s) = &d.schema else {
                panic!()
            };
            assert_eq!(
                s.presence,
                if i == 2 {
                    SlotPresence::OptionalOnce
                } else {
                    SlotPresence::RequiredOnce
                }
            );
            assert_eq!(
                serde_json::to_value(&s.sites).unwrap(),
                json!(["item_parameter"])
            );
        }
        let d = e
            .schema
            .iter()
            .find_map(|d| match d {
                SchemaExtensionEntry::Definition(DefinitionDescriptor::ItemTemplate(d))
                    if d.id == b.template =>
                {
                    Some(d)
                }
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(s) = &d.schema else {
            panic!()
        };
        assert_eq!(s.declarations.parameters.members.len(), 6);
        assert!(matches!(
            s.declarations.parameters.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(matches!(
            s.quality.allowed_kinds.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(s.quality.allowed_kinds.members.is_empty());
        let default = family::defaults()
            .into_iter()
            .find(|d| d.template == b.template)
            .unwrap();
        assert_eq!(default.parameters.len(), 2);
        assert_eq!(serde_json::to_value(default.quality).unwrap(), "absent");
        for p in default.parameters {
            assert!(
                s.declarations
                    .parameters
                    .members
                    .contains(&p.assignment.slot)
            );
        }
    }
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest"]["sha256"]
    );
    let m: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], m["upstream_revision"]);
    assert!(!a["source_files"].as_array().unwrap().is_empty());
    for pin in a["source_files"].as_array().unwrap() {
        let rows: Vec<_> = m["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["path"] == pin["path"])
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["sha256"], pin["sha256"]);
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
    // Existing native tests exercise this exact program's scope/isolation and
    // absent/zero catalyst semantics. Only the two declared input reads change.
    let old: Value =
        read(root().join("data/owned/poe2/3887ae68/ordinary-item-catalyst-inputs/extension.json"));
    let old_bindings: Value =
        read(root().join("data/owned/poe2/3887ae68/ordinary-item-catalyst-inputs/bindings.json"));
    for binding in family::catalyst_bindings() {
        let binding = serde_json::to_value(binding).unwrap();
        let owners = serde_json::to_value(&e.owners).unwrap();
        let owner = owners
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["owner"]["value"]["value"] == binding["selection"]["declaration"]["definition"]
            })
            .unwrap();
        let mut programs = owner["programs"]["members"].clone();
        for field in ["selection", "amount"] {
            assert_eq!(
                replace_exact(&mut programs, &binding[field], &old_bindings[0][field]),
                1
            );
        }
        assert_eq!(programs, old["owners"][0]["programs"]["members"]);
    }
}
fn replace_exact(value: &mut Value, from: &Value, to: &Value) -> usize {
    if value == from {
        *value = to.clone();
        return 1;
    }
    match value {
        Value::Object(v) => v.values_mut().map(|v| replace_exact(v, from, to)).sum(),
        Value::Array(v) => v.iter_mut().map(|v| replace_exact(v, from, to)).sum(),
        _ => 0,
    }
}
fn source_item(sidecar: &Value, ordinal: u64) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    let links: Vec<_> = rows[0]["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "item")
        .collect();
    assert_eq!(links.len(), 1);
    &links[0]["value"]
}
fn item<'a>(draft: &'a Value, id: &Value) -> &'a Value {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| &r["id"] == id)
        .unwrap()
}
fn raw_assignments(template: &Value, requirement: u64, capacity: u64) -> Vec<Value> {
    let b = family::bindings()
        .into_iter()
        .find(|b| serde_json::to_value(&b.template).unwrap() == *template)
        .unwrap();
    let ValueCodecKind::Option { tokens } = &b.header_inputs[0].codec.codec else {
        panic!()
    };
    let rare = &tokens.iter().find(|v| v.token == "RARE").unwrap().value;
    [(&b.header_inputs[0].slot,json!({"kind":"option","value":rare})),(&b.corruption_slot,json!({"kind":"boolean","value":false})),(&b.header_inputs[1].slot,json!({"kind":"integer","value":requirement})),(&b.capacity_slot,json!({"kind":"integer","value":capacity}))].into_iter().map(|(slot,value)|json!({"slot":{"kind":"known","value":slot},"value":{"kind":"known","value":value}})).collect()
}
fn catalyst_assignments(template: &Value) -> Vec<Value> {
    family::defaults().into_iter().find(|v| serde_json::to_value(&v.template).unwrap() == *template).unwrap().parameters.into_iter().map(|p| json!({"slot":{"kind":"known","value":p.assignment.slot},"value":{"kind":"known","value":p.assignment.value}})).collect()
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(sa["schema_version"], 14);
    assert_eq!(sb["schema_version"], 15);
    let targets: Vec<_> = if case == 5 {
        [(590, 62, 0)]
            .map(|(o, l, c)| {
                (
                    source_item(&sa, o).clone(),
                    source_item(&sb, o).clone(),
                    l,
                    c,
                )
            })
            .to_vec()
    } else {
        vec![]
    };
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let issued = |a: &Value| u64::from_str_radix(a["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(issued(&pa) - issued(&na), if case == 5 { 3 } else { 0 });
    b["draft"]["allocator"] = pa.clone();
    let mut retired = BTreeSet::new();
    let aa = a["draft"]["items"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["items"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (x, y) in aa.iter_mut().zip(bb) {
        if let Some((_, new_id, l, c)) = targets.iter().find(|(old_id, _, _, _)| old_id == &x["id"])
        {
            assert_eq!(&y["id"], new_id);
            for (pending, code) in [
                (
                    &x["parameters"]["completion"],
                    "item-parameters-not-converted",
                ),
                (
                    &x["modifiers"]["completion"],
                    "item-modifiers-not-converted",
                ),
                (&x["modifier_order"], "item-modifier-order-not-converted"),
            ] {
                assert_eq!(pending["kind"], "pending");
                assert_eq!(pending["code"], code);
                assert!(retired.insert(pending["id"]["local"].as_str().unwrap().to_owned()));
            }
            assert_eq!(y["parameters"]["completion"], json!({"kind":"complete"}));
            assert_eq!(x["modifiers"]["members"].as_array().unwrap().len(), 3);
            assert_eq!(y["modifiers"]["completion"], json!({"kind":"complete"}));
            assert_eq!(
                y["modifier_order"],
                json!({"kind":"known","value":y["modifiers"]["members"].as_array().unwrap().iter().map(|m|m["id"].clone()).collect::<Vec<_>>()})
            );
            for (modifier, (key, rolls)) in
                y["modifiers"]["members"].as_array().unwrap().iter().zip([
                    ("def.0000000000003172", 25),
                    ("def.000000000000318c", 24),
                    ("def.0000000000003100", 24),
                ])
            {
                assert_eq!(modifier["definition"]["value"]["key"], key);
                assert_eq!(modifier["rolls"]["completion"], json!({"kind":"complete"}));
                assert_eq!(
                    modifier["rolls"]["members"].as_array().unwrap().len(),
                    rolls
                );
            }
            assert_eq!(x["quality"], json!({"kind":"known","value":null}));
            assert_eq!(y["quality"], x["quality"]);
            assert_eq!(x["item_level"], json!({"kind":"known","value":null}));
            assert_eq!(y["item_level"], x["item_level"]);
            let mut expected = raw_assignments(&x["template"]["value"], *l, *c);
            expected.extend(catalyst_assignments(&x["template"]["value"]));
            let members = y["parameters"]["members"].as_array_mut().unwrap();
            assert_eq!(members.len(), 6);
            for value in expected {
                let at = members
                    .iter()
                    .position(|v| v == &value)
                    .expect("exact raw input assignment");
                members.remove(at);
            }
            assert!(members.is_empty());
            assert_eq!(*members, *x["parameters"]["members"].as_array().unwrap());
            for v in [x, y] {
                v["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
                v["modifiers"].as_object_mut().unwrap().remove("completion");
                v.as_object_mut().unwrap().remove("modifier_order");
            }
        }
    }
    assert_eq!(retired.len(), if case == 5 { 3 } else { 0 });
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "only six assignments and three Fine Belt obligations change",
    );
    if case != 5 {
        assert!(
            ids.iter()
                .all(|(k, v)| serde_json::from_str::<Value>(k).unwrap() == *v),
            "unaffected IDs stay exact"
        );
    }
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|r| {
            let remove = r["kind"] == "issue"
                && r["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id));
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
    let policy = sb["item_source_policy"].clone();
    let item_lines = sb["item_policy"].clone();
    let old_rows = sa["item_texts"].as_array().unwrap();
    let new_rows = sb["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_rows.len(), new_rows.len());
    let mut evidence = 0;
    for (x, y) in old_rows.iter().zip(new_rows) {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(y["attribution"]["policy"], policy);
        assert_eq!(y["attribution"]["item_lines"], item_lines);
        y["attribution"]["policy"] = x["attribution"]["policy"].clone();
        y["attribution"]["item_lines"] = x["attribution"]["item_lines"].clone();
        let ordinal = x["source"]["ordinal"].as_u64().unwrap();
        if case == 5 && ordinal == 590 {
            assert!(x.get("parameter_inputs").is_none());
            let proof = y
                .as_object_mut()
                .unwrap()
                .remove("parameter_inputs")
                .unwrap();
            assert_eq!(proof.as_array().unwrap().len(), 4);
            let (requirement, capacity) = (62, 0);
            let template = &x["attribution"]["default_scope"]["template"];
            let defaults: Vec<_> = family::defaults()
                .into_iter()
                .find(|v| serde_json::to_value(&v.template).unwrap() == *template)
                .unwrap()
                .parameters
                .into_iter()
                .map(|v| v.assignment)
                .collect();
            assert_eq!(
                y["defaults"]["parameters"],
                serde_json::to_value(defaults).unwrap()
            );
            assert_eq!(x["defaults"]["parameters"], json!([]));
            y["defaults"]["parameters"] = x["defaults"]["parameters"].clone();
            assert_eq!(x["defaults"]["quality_absent"], true);
            assert_eq!(y["defaults"]["quality_absent"], true);
            let binding = family::bindings()
                .into_iter()
                .find(|b| serde_json::to_value(&b.template).unwrap() == *template)
                .unwrap();
            for expected in raw_assignments(template, requirement, capacity) {
                let rows: Vec<_> = proof
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| {
                        r["slot"] == expected["slot"]["value"]
                            && r["value"] == expected["value"]["value"]
                    })
                    .collect();
                assert_eq!(
                    rows.len(),
                    1,
                    "each proven value has one exact evidence row"
                );
                let origin = &rows[0]["origin"];
                if rows[0]["slot"] == serde_json::to_value(&binding.corruption_slot).unwrap() {
                    assert_eq!(*origin, json!({"kind":"fresh_uncorrupted"}));
                } else if rows[0]["slot"] == serde_json::to_value(&binding.capacity_slot).unwrap() {
                    assert_eq!(*origin, json!({"kind":"absent_socket_header"}));
                    assert!(x["lines"].as_array().unwrap().iter().all(|line| {
                        let text = line["text"].as_str().unwrap();
                        !text.starts_with("Sockets:") && !text.starts_with("Rune:")
                    }));
                } else {
                    let line = x["lines"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|v| v["index"] == origin["line"])
                        .unwrap();
                    {
                        let header = binding
                            .header_inputs
                            .iter()
                            .find(|h| serde_json::to_value(&h.slot).unwrap() == rows[0]["slot"])
                            .unwrap();
                        assert_eq!(origin["kind"], "header");
                        assert_eq!(origin["rule"], header.rule.as_str());
                        assert_eq!(origin["capture"], header.capture.as_str());
                        assert_eq!(
                            line["text"],
                            if header.rule.as_str() == "rarity" {
                                "Rarity: RARE".into()
                            } else {
                                format!("LevelReq: {requirement}")
                            }
                        );
                    }
                }
            }
            evidence += 1;
        }
    }
    assert_eq!(evidence, targets.len());
    sb["schema_version"] = sa["schema_version"].clone();
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
    ] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], pa);
    assert_eq!(sb["allocator_after"], na);
    sb["allocator_after"] = sa["allocator_after"].clone();
    correspond(
        &sa,
        &mut sb,
        &mut ids,
        "source text/defaults/attribution and retained links stay exact",
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
        .retain(|r| !retired.contains(r["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y);
    assert_eq!(count - y.as_array().unwrap().len(), retired.len());
    if case == 5 {
        assert_eq!(count, 32);
        assert_eq!(y.as_array().unwrap().len(), 29);
    }
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    if case == 5 {
        let belt = &targets[0].0;
        let uses: Vec<_> = a["draft"]["equipment"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| &u["item"]["value"] == belt)
            .map(|u| u["id"].clone())
            .collect();
        assert_eq!(uses.len(), 4, "all saved physical receiving uses retained");
        let preset = a["draft"]["equipment_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == first["build"]["equipment"])
            .unwrap();
        assert_eq!(
            preset["equipment"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|id| uses.contains(id))
                .count(),
            1
        );
    }
    json!({"original":case,"selected_before":count,"selected_after":y.as_array().unwrap().len(),"retired":retired,"raw_assignments":4*evidence,"evidence_rows":evidence,"allocator_before":pa,"allocator_after":na,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"27\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let run = |label: &str, changed: String| {
        assert_ne!(changed, raw, "{label}");
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(
            &path,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let draft: Value = read(dest.join("draft.json"));
        let sidecar: Value = read(dest.join("sidecar.json"));
        let physical = item(&draft, source_item(&sidecar, 590)).clone();
        let text = sidecar["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["source"]["ordinal"] == 590)
            .unwrap()
            .clone();
        (physical, text)
    };
    // These inputs are outside the sealed three-member/fresh-construction scope.
    // Independent item level, known modifier values and quality remain untouched.
    let cases = [
        ("missing-level", raw.replace("LevelReq: 62\n", "")),
        (
            "duplicate-level",
            raw.replace("LevelReq: 62", "LevelReq: 62\nLevelReq: 77"),
        ),
        (
            "alternate-level",
            raw.replace("LevelReq: 62", "Requires Level: 62"),
        ),
        (
            "malformed-level",
            raw.replace("LevelReq: 62", "LevelReq: NaN"),
        ),
        (
            "corrupted",
            raw.replace("Implicits: 2", "Corrupted\nImplicits: 2"),
        ),
        (
            "duplicate-rarity",
            raw.replace("Rarity: RARE", "Rarity: RARE\nRarity: RARE"),
        ),
        (
            "unknown-member",
            raw.replace("Implicits: 2", "Implicits: 2\nUnknown modifier"),
        ),
        (
            "extra-member",
            raw.replace(
                "+10 to maximum Life",
                "+10 to maximum Life\n+3 to maximum Life",
            ),
        ),
        ("missing-explicit", raw.replace("+10 to maximum Life", "")),
        (
            "missing-implicit",
            raw.replace("Flasks gain 0.17 charges per Second", ""),
        ),
        ("missing-count", raw.replace("Implicits: 2\n", "")),
        (
            "duplicate-count",
            raw.replace("Implicits: 2", "Implicits: 2\nImplicits: 2"),
        ),
        ("all-explicit", raw.replace("Implicits: 2", "Implicits: 0")),
        ("all-implicit", raw.replace("Implicits: 2", "Implicits: 3")),
        ("wrong-count", raw.replace("Implicits: 2", "Implicits: 1")),
        (
            "occupied-rune",
            raw.replace(
                "Implicits: 2",
                "Sockets: S\nRune: Lesser Iron Rune\nImplicits: 2",
            ),
        ),
        (
            "explicit-catalyst",
            raw.replace("Implicits: 2", "Catalyst: Neural\nImplicits: 2"),
        ),
        (
            "explicit-catalyst-amount",
            raw.replace("Implicits: 2", "CatalystQuality: 0\nImplicits: 2"),
        ),
        (
            "raw-charm-setter",
            raw.replace("Implicits: 2", "CharmLimit: 77\nImplicits: 2"),
        ),
        (
            "zero-range",
            raw.replace("Has (1-3) Charm Slot", "Has (0-0) Charm Slot"),
        ),
        (
            "duplicate-observation",
            raw.replace("Charm Slots: 2", "Charm Slots: 2\nCharm Slots: 9"),
        ),
        (
            "malformed-observation",
            raw.replace("Charm Slots: 2", "Charm Slots: NaN"),
        ),
        (
            "aliased-observation",
            raw.replace("Charm Slots: 2", "Charm Slots: 02"),
        ),
        (
            "fractional-observation",
            raw.replace("Charm Slots: 2", "Charm Slots: 2.5"),
        ),
    ];
    for (label, changed) in &cases {
        let (physical, text) = run(label, changed.clone());
        assert_eq!(
            physical["parameters"]["completion"]["kind"], "pending",
            "{label}"
        );
        assert!(text.get("parameter_inputs").is_none(), "{label}");
    }
    let quality = [
        ("quality-zero", "Quality: 0"),
        ("quality-twenty", "Quality: 20"),
        ("quality-malformed", "Quality: NaN"),
        ("quality-alias", "Quality (Mana Modifiers): +20%"),
        ("quality-catalyst-style", "Quality: +20% (Mana Modifiers)"),
        ("quality-duplicate", "Quality: 0\nQuality: 20"),
    ];
    for (label, header) in quality {
        let (physical, text) = run(
            label,
            raw.replace("Implicits: 2", &format!("{header}\nImplicits: 2")),
        );
        assert_eq!(text["defaults"]["quality_absent"], false, "{label}");
        assert_ne!(
            physical["quality"],
            json!({"kind":"known","value":null}),
            "{label}"
        );
        // The existing Partial quality-kind declaration can leave an explicit
        // quality unresolved. This proof never changes that schema or defaults it.
    }
    let (physical, _) = run(
        "explicit-level-77",
        raw.replace("LevelReq: 62", "LevelReq: 77"),
    );
    assert_eq!(
        physical["parameters"]["completion"],
        json!({"kind":"complete"})
    );
    for expected in raw_assignments(&physical["template"]["value"], 77, 0) {
        assert!(
            physical["parameters"]["members"]
                .as_array()
                .unwrap()
                .contains(&expected)
        );
    }
    let (physical, text) = run(
        "explicit-empty-socket",
        raw.replace("Implicits: 2", "Sockets: S\nRune: None\nImplicits: 2"),
    );
    assert_eq!(
        physical["parameters"]["completion"],
        json!({"kind":"complete"})
    );
    for expected in raw_assignments(&physical["template"]["value"], 62, 1) {
        assert!(
            physical["parameters"]["members"]
                .as_array()
                .unwrap()
                .contains(&expected)
        );
    }
    let capacity = serde_json::to_value(&family::bindings()[0].capacity_slot).unwrap();
    let evidence = text["parameter_inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["slot"] == capacity)
        .unwrap();
    assert_eq!(evidence["origin"]["kind"], "empty_socket_capacity");
    let line = text["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["index"] == evidence["origin"]["line"])
        .unwrap();
    assert_eq!(line["text"], "Sockets: S");
    for (label, replacement) in [("derived-nine", "Charm Slots: 9\n"), ("derived-absent", "")] {
        let (physical, text) = run(label, raw.replace("Charm Slots: 2\n", replacement));
        assert_eq!(
            physical["parameters"]["completion"],
            json!({"kind":"complete"})
        );
        for expected in raw_assignments(&physical["template"]["value"], 62, 0) {
            assert!(
                physical["parameters"]["members"]
                    .as_array()
                    .unwrap()
                    .contains(&expected)
            );
        }
        let evidence = text["parameter_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["slot"] == capacity)
            .unwrap();
        assert_eq!(evidence["origin"], json!({"kind":"absent_socket_header"}));
    }
    cases.len() + quality.len() + 4
}
#[test]
#[ignore = "requires the exact checked Fine Belt modifiers predecessor"]
fn real_fine_belt_item_inputs_preserve_all_requests_and_independent_coverage() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FINE_BELT_ITEM_INPUTS_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_FINE_BELT_ITEM_INPUTS_OUTPUT")
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
    let inv = release::inventory(&package);
    assert_eq!(inv, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if name.starts_with("queries-") {
            assert_eq!(inv.get(name), Some(hash));
        }
    }
    for policy in ["item_modifier_membership", "item_parameter_inputs"] {
        for field in ["definitions", "item_lines", "item_source"] {
            let mut bad = serde_json::to_value(next.input()).unwrap();
            let old = match field {
                "definitions" => serde_json::to_value(&prior.receipt().definitions).unwrap(),
                "item_lines" => serde_json::to_value(prior.receipt().items).unwrap(),
                _ => serde_json::to_value(prior.receipt().item_source).unwrap(),
            };
            bad["normalization"][policy][field] = old;
            assert!(
                assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                    .is_err(),
                "stale {policy}.{field}"
            );
        }
    }
    family::assert_cross_owner_rejected(&next);
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
    let count = probes(&package, &out);
    assert_eq!(before, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"originals":reports,"probes":count,"stale_binding_rejections":6,"cross_owner_rejections":2,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
