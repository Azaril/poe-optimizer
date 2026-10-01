//! Real selected staff inputs; raw grant supply is not full skill or build parity.
#[path = "support/owned_ashen_staff_item_inputs.rs"]
mod family;
use family::firebolt as grant;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    owned_rules::RuleEffectKind,
    owned_schema::{DefinitionDescriptor, SchemaState, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::assemble_owned_release,
    owned_value::ValueCodecKind,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
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
#[test]
fn authored_staff_grant_distinguishes_raw_level_physical_inputs_and_coverage() {
    let b = grant::bindings();
    let e = grant::extension();
    assert_eq!(e.schema.len(), 6);
    assert_eq!(e.owners.len(), 1);
    assert!(e.tables.is_empty() && e.receivers.is_empty());
    assert_eq!(b.modifier.key().as_str(), "def.00000000000031c6");
    assert!(!e.owners[0].programs.is_complete());
    let p = &e.owners[0].programs.members[0];
    assert_eq!(p.reads.len(), 1);
    assert_eq!(p.effects.len(), 2);
    assert!(
        matches!(&p.effects[0].effect, RuleEffectKind::ProjectSkillParameter {skill,parameter,..} if skill==&b.supply && parameter==&b.raw_level)
    );
    assert!(
        matches!(&p.effects[1].effect, RuleEffectKind::ActivateGrant {slot,..} if slot==&b.grant)
    );
    let slots: Vec<_> = e
        .schema
        .iter()
        .filter_map(|s| {
            if let SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(s)) = s {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].id, b.level);
    assert_eq!(slots[1].id, b.raw_level);
    let SchemaState::Known(s) = &slots[1].schema else {
        panic!()
    };
    assert!(
        s.sites.is_empty(),
        "generated input is not a physical Gem parameter"
    );
    let SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(s)) = &e.schema[0] else {
        panic!()
    };
    assert_eq!(s.id, b.skill);
    let SchemaState::Known(s) = &s.schema else {
        panic!()
    };
    assert!(!s.directly_selectable && !s.declarations.parameters.is_complete());
    assert_eq!(s.declarations.parameters.members, vec![b.raw_level]);
    assert_eq!(grant::item_rules().len(), 2);
    assert_eq!(grant::source_conditions().len(), 2);
    let numeric = grant::spell::bindings();
    let policy = grant::spell::numeric_policy(
        serde_json::from_value(grant::spell::authoring()["definitions"].clone()).unwrap(),
    );
    let binding = &policy.bindings[0];
    assert_eq!(binding.input, numeric.amount);
    assert_eq!(binding.unit, numeric.unit);
    assert_eq!(binding.output, numeric.effective);
    assert_eq!(numeric.properties.len(), 21);
    let numeric_extension = grant::spell::extension();
    for slot in numeric.properties.values().chain([
        &numeric.amount,
        &numeric.corrupted_base,
        &numeric.category,
    ]) {
        assert_eq!(
            numeric_extension
                .schema
                .iter()
                .filter(|entry| matches!(
                    entry, SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(p)) if &p.id == slot
                ))
                .count(),
            1
        );
    }
    let physical = family::extension();
    assert_eq!(physical.schema.len(), 7);
    assert_eq!(family::bindings().len(), 1);
    assert_eq!(family::membership().paired_templates.len(), 1);
    assert_eq!(family::catalyst_bindings().len(), 1);
    let template = family::bindings()[0].template.clone();
    assert_eq!(template, b.templates[0]);
    let defaults = family::defaults();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0].parameters.len(), 2);
    assert_eq!(
        serde_json::to_value(defaults[0].item_level).unwrap(),
        "absent"
    );
    assert_ne!(serde_json::to_value(defaults[0].quality).unwrap(), "absent");
    let pins = family::authoring();
    let manifest: Value =
        read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"));
    assert_eq!(pins["source_revision"], manifest["upstream_revision"]);
    assert!(!pins["source_files"].as_array().unwrap().is_empty());
    for pin in pins["source_files"].as_array().unwrap() {
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["path"] == pin["path"] && f["sha256"] == pin["sha256"])
        );
    }
}
fn source_item(sidecar: &Value, ordinal: u64) -> Value {
    let row = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap();
    let links: Vec<_> = row["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "item")
        .collect();
    assert_eq!(links.len(), 1);
    links[0]["value"].clone()
}
fn staff<'a>(draft: &'a Value, id: &Value) -> &'a Value {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| &i["id"] == id)
        .unwrap()
}
fn assert_actual_staff(item: &Value, level: i64, damage: f64, quality: f64) {
    let b = grant::bindings();
    assert_eq!(
        item["template"]["value"],
        serde_json::to_value(&b.templates[0]).unwrap()
    );
    assert_eq!(item["item_level"], json!({"kind":"known","value":null}));
    assert_eq!(
        item["quality"]["value"]["kind"]["value"]["key"],
        "def.0000000000000006"
    );
    assert_eq!(
        item["quality"]["value"]["amount"]["value"]["value"],
        quality
    );
    for field in ["parameters", "modifiers"] {
        assert_eq!(item[field]["completion"], json!({"kind":"complete"}));
    }
    let mods = item["modifiers"]["members"].as_array().unwrap();
    assert_eq!(mods.len(), 2);
    for (m, (id, count)) in mods
        .iter()
        .zip([("def.00000000000031c6", 1), ("def.00000000000031ad", 24)])
    {
        assert_eq!(m["definition"]["value"]["key"], id);
        assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
        assert_eq!(m["rolls"]["members"].as_array().unwrap().len(), count);
    }
    assert_eq!(
        mods[0]["rolls"]["members"][0]["slot"]["value"],
        serde_json::to_value(&b.level).unwrap()
    );
    assert_eq!(
        mods[0]["rolls"]["members"][0]["value"]["value"],
        json!({"kind":"integer","value":level})
    );
    let numeric = grant::spell::bindings();
    let amount = mods[1]["rolls"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["slot"]["value"] == serde_json::to_value(&numeric.amount).unwrap())
        .unwrap();
    assert_eq!(amount["value"]["value"]["value"]["value"], damage);
    assert_eq!(
        item["modifier_order"],
        json!({"kind":"known","value":mods.iter().map(|m|m["id"].clone()).collect::<Vec<_>>()})
    );
    let input = family::bindings().remove(0);
    let ValueCodecKind::Option { tokens } = &input.header_inputs[0].codec.codec else {
        panic!()
    };
    let rare = &tokens.iter().find(|v| v.token == "RARE").unwrap().value;
    let mut expected:Vec<_>=[(&input.header_inputs[0].slot,json!({"kind":"option","value":rare})),(&input.corruption_slot,json!({"kind":"boolean","value":false})),(&input.header_inputs[1].slot,json!({"kind":"integer","value":26})),(&input.capacity_slot,json!({"kind":"integer","value":4}))].into_iter().map(|(slot,value)|json!({"slot":{"kind":"known","value":slot},"value":{"kind":"known","value":value}})).collect();
    expected.extend(family::defaults().remove(0).parameters.into_iter().map(|p|json!({"slot":{"kind":"known","value":p.assignment.slot},"value":{"kind":"known","value":p.assignment.value}})));
    let actual = item["parameters"]["members"].as_array().unwrap();
    assert_eq!(actual.len(), 6);
    for p in expected {
        assert_eq!(actual.iter().filter(|v| **v == p).count(), 1);
    }
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], 15);
    let mut retired = BTreeSet::new();
    let mut added = vec![];
    if case == 4 {
        // The same injected template also admits an explicitly saved quality9
        // on Original04's staff. Its occupied rune and modifier inventory remain
        // unresolved; this is a quality-only improvement.
        let old_id = source_item(&sa, 236);
        let new_id = source_item(&sb, 236);
        let x = a["draft"]["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|i| i["id"] == old_id)
            .unwrap();
        let y = b["draft"]["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|i| i["id"] == new_id)
            .unwrap();
        assert_eq!(x["template"], y["template"]);
        assert_eq!(x["template"]["value"]["key"], "def.0000000000001d75");
        assert_eq!(x["quality"]["kind"], "pending");
        assert_eq!(x["quality"]["code"], "quality-not-converted");
        assert!(retired.insert(x["quality"]["id"]["local"].as_str().unwrap().to_owned()));
        assert_eq!(y["quality"]["kind"], "known");
        assert_eq!(
            y["quality"]["value"]["kind"]["value"]["key"],
            "def.0000000000000006"
        );
        assert_eq!(y["quality"]["value"]["amount"]["value"]["value"], 9.0);
        assert_eq!(y["parameters"]["completion"]["kind"], "pending");
        assert_eq!(y["modifiers"]["completion"]["kind"], "pending");
        assert!(y["modifiers"]["members"].as_array().unwrap().is_empty());
        x.as_object_mut().unwrap().remove("quality");
        y.as_object_mut().unwrap().remove("quality");
    }
    if case == 5 {
        let id = source_item(&sa, 594);
        let new_id = source_item(&sb, 594);
        assert_actual_staff(staff(&b, &new_id), 11, 128.0, 20.0);
        let x = a["draft"]["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|i| i["id"] == id)
            .unwrap();
        let y = b["draft"]["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|i| i["id"] == new_id)
            .unwrap();
        for (pointer, code) in [
            ("/item_level", "item-level-not-converted"),
            ("/quality", "quality-not-converted"),
            ("/parameters/completion", "item-parameters-not-converted"),
            ("/modifiers/completion", "item-modifiers-not-converted"),
            ("/modifier_order", "item-modifier-order-not-converted"),
        ] {
            let p = x.pointer(pointer).unwrap();
            assert_eq!(p["kind"], "pending");
            assert_eq!(p["code"], code);
            assert!(retired.insert(p["id"]["local"].as_str().unwrap().to_owned()));
        }
        assert!(x["parameters"]["members"].as_array().unwrap().is_empty());
        assert!(x["modifiers"]["members"].as_array().unwrap().is_empty());
        added = y["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].clone())
            .collect();
        for field in [
            "item_level",
            "quality",
            "parameters",
            "modifiers",
            "modifier_order",
        ] {
            x.as_object_mut().unwrap().remove(field);
            y.as_object_mut().unwrap().remove(field);
        }
        // Whole-project item/equipment censuses still contain unresolved archived
        // alternatives. Preserve them through the same correspondence below.
    }
    let before_allocator = a["draft"]["allocator"].clone();
    let after_allocator = b["draft"]["allocator"].clone();
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&before_allocator) + added.len() as u64,
        issued(&after_allocator) + retired.len() as u64
    );
    b["draft"]["allocator"] = before_allocator.clone();
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "all other original build values and relationships",
    );
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|l| {
            !(l["kind"] == "issue" && retired.contains(l["value"]["local"].as_str().unwrap()))
        });
    }
    let mut removed = 0;
    for row in sb["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|l| {
            let remove = l["kind"] == "modifier" && added.contains(&l["value"]);
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, added.len());
    identity::correspond(
        &sa["origins"],
        &mut sb["origins"],
        &mut ids,
        "source occurrence links",
    );
    // Raw source text and its ordinals/spans never change when a recipe becomes known.
    let old_texts = sa["item_texts"].as_array().unwrap();
    let new_texts = sb["item_texts"].as_array().unwrap();
    assert_eq!(old_texts.len(), new_texts.len());
    for (x, y) in old_texts.iter().zip(new_texts) {
        assert_eq!(x["source"], y["source"]);
        let xl = x["lines"].as_array().unwrap();
        let yl = y["lines"].as_array().unwrap();
        assert_eq!(xl.len(), yl.len());
        for (x, y) in xl.iter().zip(yl) {
            assert_eq!(x["index"], y["index"]);
            assert_eq!(x["text"], y["text"]);
        }
        if case == 5 && x["source"]["ordinal"] == 594 {
            assert_eq!(x["attribution"]["layout"]["status"], "pending");
            assert_eq!(y["attribution"]["layout"]["status"], "proven");
            assert_eq!(y["parameter_inputs"].as_array().unwrap().len(), 4);
        }
    }
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
    let count = expected.as_array().unwrap().len();
    expected
        .as_array_mut()
        .unwrap()
        .retain(|i| !retired.contains(i["id"]["local"].as_str().unwrap()));
    identity::relocate(&mut actual, &ids);
    assert_eq!(expected, actual);
    assert_eq!(
        count - actual.as_array().unwrap().len(),
        match case {
            4 => 1,
            5 => 5,
            _ => 0,
        }
    );
    if case == 5 {
        assert_eq!(count, 29);
        assert_eq!(actual.as_array().unwrap().len(), 24);
    }
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    identity::relocate(&mut second, &ids);
    assert_eq!(first, second);
    if case == 5 {
        let id = source_item(&sa, 594);
        let uses: Vec<_> = a["draft"]["equipment"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| u["item"]["value"] == id)
            .map(|u| u["id"].clone())
            .collect();
        assert_eq!(uses.len(), 1);
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
    json!({"original":case,"selected_before":count,"selected_after":actual.as_array().unwrap().len(),"retired":retired,"new_modifiers":added.len(),"allocator_before":before_allocator,"allocator_after":after_allocator,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}
fn edit_item(xml: &str, from: &str, to: &str) -> String {
    let start = xml.find("<Item id=\"28\"").unwrap();
    let end = start + xml[start..].find("</Item>").unwrap();
    let body = &xml[start..end];
    assert!(body.contains(from));
    format!(
        "{}{}{}",
        &xml[..start],
        body.replacen(from, to, 1),
        &xml[end..]
    )
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let mut count = 0;
    for (name, from, to, valid, level, damage, quality) in [
        (
            "fixed-grant",
            "{range:0.5}Grants Skill: Level (1-20) Firebolt",
            "Grants Skill: Level 7 Firebolt",
            true,
            7,
            128.0,
            20.0,
        ),
        (
            "fixed-zero",
            "{range:0.5}Grants Skill: Level (1-20) Firebolt",
            "Grants Skill: Level 0 Firebolt",
            true,
            0,
            128.0,
            20.0,
        ),
        (
            "fixed-hundred",
            "{range:0.5}Grants Skill: Level (1-20) Firebolt",
            "Grants Skill: Level 100 Firebolt",
            true,
            100,
            128.0,
            20.0,
        ),
        ("zero-lower", "(1-20)", "(0-20)", false, 0, 0.0, 0.0),
        ("zero-range", "(1-20)", "(0-0)", false, 0, 0.0, 0.0),
        ("above-bound", "(1-20)", "(1-101)", false, 0, 0.0, 0.0),
        (
            "positive-below-half",
            "(1-20)",
            "(1-2)",
            true,
            1,
            128.0,
            20.0,
        ),
        ("positive-at-half", "(1-20)", "(1-2)", true, 2, 128.0, 20.0),
        (
            "spell-zero",
            "128% increased Spell Damage",
            "0% increased Spell Damage",
            true,
            11,
            0.0,
            20.0,
        ),
        (
            "quality-zero",
            "Quality: 20",
            "Quality: 0",
            true,
            11,
            128.0,
            0.0,
        ),
        (
            "unknown-grant",
            "Firebolt",
            "UnknownSpell",
            false,
            0,
            0.0,
            0.0,
        ),
        ("negative-grant", "(1-20)", "(-1-20)", false, 0, 0.0, 0.0),
        (
            "duplicate-grant",
            "128% increased Spell Damage",
            "Grants Skill: Level 11 Firebolt\n128% increased Spell Damage",
            false,
            0,
            0.0,
            0.0,
        ),
        (
            "missing-grant",
            "{range:0.5}Grants Skill: Level (1-20) Firebolt",
            "",
            false,
            0,
            0.0,
            0.0,
        ),
        (
            "unknown-spell-line",
            "128% increased Spell Damage",
            "128% invented Spell Damage",
            false,
            0,
            0.0,
            0.0,
        ),
        (
            "occupied-rune",
            "Rune: None",
            "Rune: Unknown",
            false,
            0,
            0.0,
            0.0,
        ),
        (
            "bad-quality",
            "Quality: 20",
            "Quality: Nope",
            false,
            0,
            0.0,
            0.0,
        ),
        ("absent-quality", "Quality: 20\n", "", false, 0, 0.0, 0.0),
        (
            "wrong-implicit-count",
            "Implicits: 1",
            "Implicits: 2",
            false,
            0,
            0.0,
            0.0,
        ),
        (
            "duplicate-requirement",
            "LevelReq: 26",
            "LevelReq: 26\nLevelReq: 99",
            false,
            0,
            0.0,
            0.0,
        ),
    ] {
        let dir = out.join(format!("probe-{name}"));
        let xml = out.join(format!("probe-{name}.xml"));
        let mut changed = edit_item(&original, from, to);
        if name == "positive-below-half" {
            changed = edit_item(
                &changed,
                "<ModRange range=\"0.5\" id=\"1\"/>",
                "<ModRange range=\"0.4999999999999998\" id=\"1\"/>",
            );
        }
        fs::write(&xml, changed).unwrap();
        release::normalize(package, &xml, 5, &dir);
        let draft: Value = read(dir.join("draft.json"));
        let sidecar: Value = read(dir.join("sidecar.json"));
        let id = source_item(&sidecar, 594);
        let item = staff(&draft, &id);
        if valid {
            assert_actual_staff(item, level, damage, quality);
        } else if name == "absent-quality" {
            // Physical headers and member census do not authorize a missing
            // ordinary-quality value. Keep that independent obligation open.
            assert_eq!(item["parameters"]["completion"], json!({"kind":"complete"}));
            assert_eq!(item["quality"]["kind"], "pending");
            assert_eq!(item["quality"]["code"], "quality-not-converted");
        } else {
            assert_eq!(
                item["parameters"]["completion"]["kind"], "pending",
                "{name}"
            );
        }
        count += 1;
    }
    count
}
#[test]
#[ignore = "requires exact checked Fine Belt inputs predecessor"]
fn real_staff_publication_preserves_five_original_selections_and_queries() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ASHEN_STAFF_PRIOR").expect("explicit prior"),
    );
    let prior = release::load(&prior_path);
    let before = release::inventory(&prior_path);
    let next = family::stage(&prior);
    family::preservation(&prior, &next);
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ASHEN_STAFF_OUTPUT").expect("fresh explicit output"),
    );
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
    let inventory = release::inventory(&package);
    assert_eq!(inventory, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if name.starts_with("queries-") {
            assert_eq!(inventory.get(name), Some(hash));
        }
    }
    for policy in ["item_modifier_membership", "item_parameter_inputs"] {
        for field in ["definitions", "item_lines", "item_source"] {
            let mut bad = serde_json::to_value(next.input()).unwrap();
            bad["normalization"][policy][field] =
                serde_json::to_value(prior.input()).unwrap()["normalization"][policy][field]
                    .clone();
            assert!(
                assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                    .is_err()
            );
        }
    }
    family::assert_cross_owner_rejected(&next);
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(&xml).unwrap(), &old, &new, &out));
    }
    let count = probes(&package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":next.receipt().query_rows,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"originals":reports,"probes":count,"stale_binding_rejections":6,"cross_owner_rejections":2,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
