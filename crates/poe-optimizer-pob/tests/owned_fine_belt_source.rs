//! Fine Belt physical input contrasts; no native owner or whole-build coverage authority.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[path = "support/fine_belt_source.rs"]
mod observer;
use observer::OBSERVE;
const TEST: &str = "fine_belt_preserves_physical_inputs_local_capacity_and_rate_lifecycle";
const CHILD: &str = "POE_FINE_BELT_SOURCE_CHILD";

#[test]
fn fine_belt_preserves_physical_inputs_local_capacity_and_rate_lifecycle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-fine-belt-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let manifest = read(&fixtures.join("index.json"));
        let entry = rows(&manifest["builds"])
            .iter()
            .find(|r| r["xml"] == "build-05.xml")
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([53; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let matches: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Item"
                    && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("27")
            })
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].occurrence().id().ordinal(), 590);
        let quality_twenty = edit_item(&xml, 27, |body| {
            body.replace("Crafted: true", "Quality: 20\nCrafted: true")
        });
        let range_zero = edit_item(&xml, 27, |body| {
            body.replace("<ModRange range=\"0.5\"", "<ModRange range=\"0\"")
        });
        let range_one = edit_item(&xml, 27, |body| {
            body.replace("<ModRange range=\"0.5\"", "<ModRange range=\"1\"")
        });
        let header_high = edit_item(&xml, 27, |body| {
            body.replace("Charm Slots: 2", "Charm Slots: 9")
        });
        let header_absent = edit_item(&xml, 27, |body| body.replace("Charm Slots: 2\n", ""));
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text) in [
            ("original", &xml),
            ("header-high", &header_high),
            ("header-absent", &header_absent),
            ("legacy-range-zero", &range_zero),
            ("legacy-range-one", &range_one),
            ("quality-twenty", &quality_twenty),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("fineBeltXml", text.as_str())?;
                lua.globals().set("fineBeltControls", name == "original")?;
                lua.globals().set("fineBeltJit", enabled)?;
                lua.load("if fineBeltJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                text,
                None,
                false,
                Some(&before),
                None,
                Some(&observe),
            )
            .unwrap();
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            if let Some(hash) = &source_hash {
                assert_eq!(&result["source_hash"], hash);
            } else {
                source_hash = Some(result["source_hash"].clone());
            }
            cases.push(json!({"name":name,"xml_sha256":digest(text.as_bytes()),"state":result["additional_observation"]}));
        }
        assert_eq!(
            fs::read_to_string(fixtures.join("build-05.xml")).unwrap(),
            xml
        );
        let result = json!({"source_hash":source_hash,"evidence":{
            "manifest_sha256":pinned::manifest_sha256(),"native_input_closure":false,"native_owner_coverage":false,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/CalcSetup.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/ModScalability.lua","src/Data/Bases/belt.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
        },"cases":cases});
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        check(&result);
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn edit_item(xml: &str, id: usize, edit: impl FnOnce(&str) -> String) -> String {
    let opening = format!("<Item id=\"{id}\">");
    assert_eq!(xml.matches(&opening).count(), 1);
    let begin = xml.find(&opening).unwrap() + opening.len();
    let end = begin + xml[begin..].find("</Item>").unwrap();
    let mut result = xml.to_owned();
    result.replace_range(begin..end, &edit(&xml[begin..end]));
    result
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@fine-belt-source-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn values(snapshot: &Json, name: &str) -> Vec<Json> {
    rows(&snapshot["active"])
        .iter()
        .filter(|row| row["name"] == name)
        .map(|row| row["value"].clone())
        .collect()
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 6);
    for (case, capacity) in cases.iter().zip([2, 2, 2, 1, 3, 2]) {
        let state = &case["state"];
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[flag], true, "{} {flag}", case["name"]);
        }
        for (key, id) in [
            ("selected_items", 2),
            ("selected_spec", 3),
            ("selected_skills", 4),
            ("selected_config", 1),
        ] {
            assert_eq!(state[key], id);
        }
        let item = &rows(&state["items"])[0];
        assert_eq!(item["id"], 27);
        assert_eq!(item["selected_slots"], json!(["Belt"]));
        assert_eq!(
            item["authored_receiving_uses"],
            json!([
                {"set":2,"slot":"Belt"},{"set":3,"slot":"Belt"},{"set":4,"slot":"Belt"},{"set":5,"slot":"Belt"}
            ])
        );
        assert_eq!(item["loaded"]["charmLimit"], capacity, "{}", case["name"]);
        assert!(
            values(&item["loaded"], "CharmLimit").is_empty(),
            "local capacity is consumed before the active list"
        );
        assert_eq!(
            values(&item["loaded"], "FlaskChargesGenerated"),
            vec![json!(0.17)]
        );
        assert_eq!(values(&item["loaded"], "Life"), vec![json!(10)]);
        for (name, expected, source) in [
            ("CharmLimit", json!(capacity), "New Item"),
            (
                "FlaskChargesGenerated",
                json!(0.17),
                "Item:27:New Item, Fine Belt",
            ),
            ("Life", json!(10), "Item:27:New Item, Fine Belt"),
        ] {
            let delivered = rows(&item["player_records"][name]);
            assert_eq!(
                delivered.len(),
                1,
                "{} {name}: one selected receiving use",
                case["name"]
            );
            assert_eq!(delivered[0]["value"], expected);
            assert_eq!(delivered[0]["source"], source);
            assert_eq!(delivered[0]["type"], "BASE");
            assert_eq!(delivered[0]["flags"], 0);
            assert_eq!(delivered[0]["keyword_flags"], 0);
            assert!(rows(&delivered[0]["tags"]).is_empty());
        }
        assert_eq!(rows(&item["slot_lists"]).len(), 1);
        assert_eq!(item["slot_lists"][0]["kind"], "shared-mod-list");
        assert_eq!(
            item["fresh"]["charmLimit"], 2,
            "fresh parse has no legacy XML overlay"
        );
    }
    let row = &cases[0]["state"]["items"][0];
    let loaded = &row["loaded"];
    assert_eq!(loaded["base"], "Fine Belt");
    assert_eq!(loaded["rarity"], "RARE");
    assert_eq!(loaded["crafted"], true);
    assert_eq!(loaded["requirements"]["level"], 62);
    assert_eq!(loaded["itemSocketCount"], 0);
    assert_eq!(loaded["jewelSocketCount"], 0);
    for field in [
        "itemLevel",
        "quality",
        "catalyst",
        "catalystQuality",
        "corrupted",
        "doubleCorrupted",
        "mirrored",
        "desecrated",
        "classRestriction",
    ] {
        assert_eq!(loaded["field_types"][field], "nil", "{field}");
    }
    assert_eq!(row["base_facts"]["charm_limit"], 0);
    assert_eq!(row["base_facts"]["requirements"]["level"], 62);
    assert_eq!(
        row["base_facts"]["implicit"],
        "Has (1-3) Charm Slot\nFlasks gain 0.17 charges per Second"
    );
    for field in ["quality", "socket_limit", "armour", "flask", "charm"] {
        assert!(row["base_facts"][field].is_null(), "{field}");
    }
    for category in ["buff", "enchant", "rune", "classRequirement"] {
        assert!(rows(&loaded["lists"][category]).is_empty());
    }
    assert_eq!(rows(&loaded["lists"]["implicit"]).len(), 2);
    assert_eq!(rows(&loaded["lists"]["explicit"]).len(), 1);
    for (category, index, text, stat, value, tags) in [
        (
            "implicit",
            0,
            "Has (1-3) Charm Slot",
            "CharmLimit",
            json!(2),
            json!(["charm"]),
        ),
        (
            "implicit",
            1,
            "Flasks gain 0.17 charges per Second",
            "FlaskChargesGenerated",
            json!(0.17),
            json!({}),
        ),
        (
            "explicit",
            0,
            "+10 to maximum Life",
            "Life",
            json!(10),
            json!({}),
        ),
    ] {
        let member = &loaded["lists"][category][index];
        assert_eq!(member["line"], text);
        assert_eq!(member["modTags"], tags);
        assert_eq!(member["field_types"]["extra"], "nil");
        assert_eq!(member["valueScalar"], 1);
        assert_eq!(member["catalyst_factor"], 1);
        for field in [
            "corruptedRange",
            "custom",
            "unscalable",
            "disabled",
            "desecrated",
            "fractured",
            "bonded",
            "prefix",
            "suffix",
            "rune",
            "enchant",
        ] {
            assert!(
                member[field].is_null() || member[field] == false,
                "{category} {index} {field}"
            );
        }
        let records = rows(&member["records"]);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["name"], stat);
        assert_eq!(records[0]["type"], "BASE");
        assert_eq!(records[0]["value"], value);
        assert_eq!(records[0]["flags"], 0);
        assert_eq!(records[0]["keyword_flags"], 0);
        assert!(rows(&records[0]["tags"]).is_empty());
    }
    assert_eq!(rows(&loaded["base_mods"]).len(), 3);
    assert_eq!(loaded["base_mods"][0]["name"], "CharmLimit");
    assert_eq!(rows(&loaded["active"]).len(), 2);
    assert_eq!(row["fresh"]["lists"], loaded["lists"]);
    assert_eq!(row["fresh"]["active"], loaded["active"]);
    assert_eq!(row["fresh_after_reparse"], row["fresh"]);
    assert_eq!(
        cases[0]["state"]["scalability"]["charm"],
        json!([{"isScalable":true}])
    );
    assert_eq!(
        cases[0]["state"]["scalability"]["flask"],
        json!([{"isScalable":true,"formats":["per_minute_to_per_second_2dp_if_required"]}])
    );
    assert_eq!(cases[5]["state"]["items"][0]["loaded"]["quality"], 20);
    assert_eq!(
        values(
            &cases[5]["state"]["items"][0]["loaded"],
            "Multiplier:QualityOnBelt"
        ),
        vec![json!(20)]
    );
    let probes = rows(&row["probes"]);
    assert_eq!(probes.len(), 43);
    let p = |name: &str| &probes.iter().find(|p| p["name"] == name).unwrap()["after"];
    for name in [
        "header-zero",
        "header-high",
        "header-absent",
        "header-duplicate",
    ] {
        assert_eq!(p(name)["charmLimit"], 2, "{name}");
        assert!(values(p(name), "CharmLimit").is_empty());
    }
    for (name, capacity, rate) in [
        ("range-low", 1, 0.17),
        ("range-high", 3, 0.17),
        ("implicit-magnitude", 3, 0.25),
        ("explicit-magnitude", 2, 0.17),
        ("charm-tag-magnitude", 3, 0.17),
        ("flask-range", 2, 0.2),
        ("flask-range-magnitude", 3, 0.3),
        ("flask-fraction", 2, 0.18),
        ("flask-eighth", 2, 0.13),
        ("flask-corrupted-range", 2, 0.25),
        ("charm-corrupted-range", 3, 0.17),
    ] {
        assert_eq!(p(name)["charmLimit"], capacity, "{name}");
        assert_eq!(
            values(p(name), "FlaskChargesGenerated"),
            vec![json!(rate)],
            "{name}"
        );
    }
    assert_eq!(values(p("explicit-magnitude"), "Life"), vec![json!(15)]);
    for name in [
        "catalyst-kind",
        "catalyst-zero",
        "catalyst-only-amount",
        "quality-alias",
    ] {
        assert_eq!(p(name)["charmLimit"], 2, "{name}");
        assert_eq!(values(p(name), "FlaskChargesGenerated"), vec![json!(0.17)]);
        assert_eq!(p(name)["lists"]["implicit"][0]["catalyst_factor"], 1);
    }
    assert_eq!(p("quality-zero")["quality"], 0);
    assert_eq!(p("quality-twenty")["quality"], 20);
    assert_eq!(p("quality-malformed")["field_types"]["quality"], "nil");
    assert_eq!(p("quality-alias")["field_types"]["quality"], "nil");
    assert_eq!(p("item-level")["itemLevel"], 77);
    assert_eq!(p("level-zero")["requirements"]["level"], 0);
    assert_eq!(p("level-absent")["requirements"]["level"], 62);
    assert_eq!(p("crafted-false")["crafted"], true);
    assert_eq!(p("saved-affix-label")["lists"], row["fresh"]["lists"]);
    assert_eq!(p("corrupted")["corrupted"], true);
    assert_eq!(p("twice-corrupted")["doubleCorrupted"], true);
    assert_eq!(p("empty-rune")["itemSocketCount"], 1);
    assert_eq!(p("empty-rune")["runes"], json!(["None"]));
    assert_eq!(p("unknown-rune")["runes"], json!(["OwnedUnknownRune"]));
    assert_eq!(rows(&p("implicit-one")["lists"]["implicit"]).len(), 1);
    assert_eq!(rows(&p("implicit-one")["lists"]["explicit"]).len(), 2);
    assert_eq!(rows(&p("implicit-three")["lists"]["implicit"]).len(), 3);
    assert!(rows(&p("implicit-three")["lists"]["explicit"]).is_empty());
    assert_eq!(p("missing-charm")["charmLimit"], 0);
    assert!(values(p("missing-flask"), "FlaskChargesGenerated").is_empty());
    assert_eq!(
        p("unknown-before")["lists"]["implicit"][0]["field_types"]["extra"],
        "string"
    );
    assert_eq!(rows(&p("extra-life")["lists"]["explicit"]).len(), 2);
    assert_eq!(rows(&p("enchant-charm")["lists"]["enchant"]).len(), 1);
    assert_eq!(row["reparsed"]["itemLevel"], 77);
    assert_eq!(row["reparsed"]["corrupted"], true);
    assert_eq!(row["reparsed"]["catalystQuality"], 37);
    assert_eq!(row["reparsed"]["field_types"]["quality"], "nil");
    let rebuilt = rows(&row["rebuilds"]);
    assert_eq!(rebuilt.len(), 2);
    assert_eq!(rebuilt[0]["form"], "fixed");
    assert_eq!(rebuilt[1]["form"], "ranged");
    for form in rebuilt {
        let expected = if form["form"] == "fixed" {
            [0.17, 0.17, 0.17]
        } else {
            [0.3, 0.4, 0.2]
        };
        let steps = rows(&form["steps"]);
        assert_eq!(steps.len(), 3);
        for ((step, rate), capacity) in steps.iter().zip(expected).zip([3, 4, 2]) {
            assert_eq!(step["state"]["charmLimit"], capacity);
            assert_eq!(
                values(&step["state"], "FlaskChargesGenerated"),
                vec![json!(rate)]
            );
        }
    }
    // Zero remains an independent fixed flask record; a rounded zero or a
    // parser's absent remainder alone does not establish that property.
    for (name, raw, effective) in [
        ("flask-zero", "0", json!(0)),
        ("flask-small", "0.001", json!(0)),
        ("flask-eighth", "0.125", json!(0.13)),
        ("flask-fraction", "0.175", json!(0.18)),
    ] {
        let probe = p(name);
        assert_eq!(rows(&probe["lists"]["implicit"]).len(), 2);
        assert_eq!(rows(&probe["lists"]["explicit"]).len(), 1);
        let member = &probe["lists"]["implicit"][1];
        assert_eq!(
            member["line"],
            format!("Flasks gain {raw} charges per Second")
        );
        assert_eq!(member["field_types"]["extra"], "nil");
        assert!(member["extra"].is_null());
        assert!(rows(&member["modTags"]).is_empty());
        assert_eq!(member["valueScalar"], 1);
        assert_eq!(member["catalyst_factor"], 1);
        let records = rows(&member["records"]);
        assert_eq!(records.len(), 1, "{name}");
        assert_eq!(records[0]["name"], "FlaskChargesGenerated");
        assert_eq!(records[0]["type"], "BASE");
        assert_eq!(records[0]["value"], effective);
        assert_eq!(records[0]["source"], "Item:27:New Item, Fine Belt");
        assert_eq!(records[0]["flags"], 0);
        assert_eq!(records[0]["keyword_flags"], 0);
        assert!(rows(&records[0]["tags"]).is_empty());
        assert_eq!(values(probe, "FlaskChargesGenerated"), vec![effective]);
        assert_eq!(values(probe, "Life"), vec![json!(10)]);
    }
    let negative = p("flask-negative");
    let member = &negative["lists"]["implicit"][1];
    assert_eq!(member["line"], "Flasks gain -0.17 charges per Second");
    assert_eq!(member["extra"], member["line"]);
    assert_eq!(member["field_types"]["extra"], "string");
    assert!(rows(&member["records"]).is_empty());
    assert!(values(negative, "FlaskChargesGenerated").is_empty());
    assert_eq!(values(negative, "Life"), vec![json!(10)]);

    let zero_charm = p("charm-zero");
    let member = &zero_charm["lists"]["implicit"][0];
    assert_eq!(member["line"], "Has (0-0) Charm Slot");
    assert_eq!(member["field_types"]["extra"], "nil");
    assert!(rows(&member["records"]).is_empty());
    assert_eq!(zero_charm["charmLimit"], 0);
    assert!(values(zero_charm, "CharmLimit").is_empty());
    assert_eq!(
        values(zero_charm, "FlaskChargesGenerated"),
        vec![json!(0.17)]
    );

    // A malformed header changes local consumption, despite the same clean
    // physical modifier. It is not equivalent to an absent or numeric header.
    let malformed = p("header-malformed");
    assert_eq!(malformed["field_types"]["charmLimit"], "nil");
    assert_eq!(values(malformed, "CharmLimit"), vec![json!(2)]);
    let active = rows(&malformed["active"]);
    assert_eq!(active.len(), 3);
    assert_eq!(active[0]["name"], "CharmLimit");
    assert_eq!(active[0]["source"], "Item:27:New Item, Fine Belt");
}
