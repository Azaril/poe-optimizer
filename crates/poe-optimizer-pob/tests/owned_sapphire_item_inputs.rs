//! Sapphire Ring physical input contrasts; no native owner or whole-build coverage authority.
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
#[path = "support/sapphire_item_inputs_source.rs"]
mod observer;
use observer::OBSERVE;
const TEST: &str = "sapphire_items_preserve_absence_and_finite_implicit_explicit_inputs";
const CHILD: &str = "POE_SAPPHIRE_ITEM_INPUTS_CHILD";

#[test]
fn sapphire_items_preserve_absence_and_finite_implicit_explicit_inputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-sapphire-item-inputs-source-01");
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
                    && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("26")
            })
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].occurrence().id().ordinal(), 587);
        let quality_twenty = edit_item(&xml, 26, |body| {
            body.replace("Crafted: true", "Quality: 20\nCrafted: true")
        });
        let range_zero = edit_item(&xml, 26, |body| {
            body.replace("<ModRange range=\"0.5\"", "<ModRange range=\"0\"")
        });
        let range_one = edit_item(&xml, 26, |body| {
            body.replace("<ModRange range=\"0.5\"", "<ModRange range=\"1\"")
        });
        let changed_variants = edit_item(&xml, 26, |body| {
            body.replace(
                "Crafted: true",
                "Variant: First\nVariant: Second\nCrafted: true",
            )
            .replace(
                "+10 to maximum Life",
                "+10 to maximum Life\n{variant:1}+3 to maximum Life\n{variant:2}+5 to maximum Life",
            )
        })
        .replace("<Item id=\"26\">", "<Item id=\"26\" variant=\"2\">");
        let fallback_title = edit_item(&xml, 26, |body| {
            body.replace("New Item", "Sekhema's Resolve")
        });
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text) in [
            ("original", &xml),
            ("quality-twenty", &quality_twenty),
            ("legacy-range-zero", &range_zero),
            ("legacy-range-one", &range_one),
            ("xml-variant-two", &changed_variants),
            ("loader-fallback-title", &fallback_title),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("sapphireItemXml", text.as_str())?;
                lua.globals()
                    .set("sapphireItemControls", name == "original")?;
                lua.globals().set("sapphireItemJit", enabled)?;
                lua.load("if sapphireItemJit then jit.on() else jit.off();jit.flush() end")
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
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/CalcSetup.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/ModScalability.lua","src/Data/Bases/ring.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
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
        .set_name("@sapphire-item-inputs-observer")
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
    for case in cases {
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
        assert!(
            state["main_output"]
                .as_object()
                .is_some_and(|o| !o.is_empty())
        );
        assert_eq!(rows(&state["items"]).len(), 1);
        let item = &state["items"][0];
        assert_eq!(item["id"], 26);
        assert_eq!(item["loaded"]["base"], "Sapphire Ring");
        assert_eq!(item["selected_slots"], json!(["Ring 1", "Ring 2"]));
        assert_eq!(rows(&item["slot_lists"]).len(), 3);
        assert_eq!(
            item["authored_receiving_uses"],
            json!([
                {"set":2,"slot":"Ring 1"},{"set":2,"slot":"Ring 2"},
                {"set":3,"slot":"Ring 1"},{"set":3,"slot":"Ring 2"},
                {"set":4,"slot":"Ring 1"},{"set":4,"slot":"Ring 2"},
                {"set":5,"slot":"Ring 1"},{"set":5,"slot":"Ring 2"}
            ])
        );
    }
    let row = &cases[0]["state"]["items"][0];
    let loaded = &row["loaded"];
    assert_eq!(loaded["rarity"], "RARE");
    assert_eq!(loaded["crafted"], true);
    assert_eq!(loaded["requirements"]["level"], 12);
    for field in [
        "quality",
        "corrupted",
        "doubleCorrupted",
        "catalyst",
        "catalystQuality",
        "itemLevel",
        "variant",
        "variantAlt",
        "hasAltVariant",
        "allowDuplicateVariants",
        "classRestriction",
    ] {
        assert_eq!(loaded["field_types"][field], "nil", "{field}");
    }
    assert_eq!(loaded["itemSocketCount"], 0);
    assert_eq!(loaded["jewelSocketCount"], 0);
    assert!(rows(&loaded["sockets"]).is_empty());
    assert!(rows(&loaded["runes"]).is_empty());
    for field in ["quality", "socket_limit", "flask", "charm", "armour"] {
        assert_eq!(row["base_facts"][field], Json::Null, "{field}");
    }
    assert_eq!(
        row["base_facts"]["implicit"],
        "+(20-30)% to Cold Resistance"
    );
    assert_eq!(row["base_facts"]["requirements"]["level"], 12);
    assert_eq!(
        cases[0]["state"]["scalability"],
        json!([{"isScalable":true}])
    );
    for category in ["buff", "enchant", "rune", "classRequirement"] {
        assert!(rows(&loaded["lists"][category]).is_empty());
    }
    let tags = json!([
        "cold_resistance",
        "elemental_resistance",
        "elemental",
        "cold",
        "resistance"
    ]);
    for (category, text, stat, value) in [
        ("implicit", "+(20-30)% to Cold Resistance", "ColdResist", 25),
        ("explicit", "+10 to maximum Life", "Life", 10),
    ] {
        let lines = rows(&loaded["lists"][category]);
        assert_eq!(lines.len(), 1);
        let member = &lines[0];
        assert_eq!(member["line"], text);
        assert_eq!(member["field_types"]["extra"], "nil");
        assert_eq!(member["catalyst_factor"], 1);
        if category == "implicit" {
            assert_eq!(member["modTags"], tags);
        } else {
            assert!(rows(&member["modTags"]).is_empty());
        }
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
                "{category} {field}: {}",
                member[field]
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
    assert_eq!(rows(&loaded["base_mods"]).len(), 2);
    assert_eq!(loaded["base_mods"][0]["name"], "ColdResist");
    assert_eq!(loaded["base_mods"][1]["name"], "Life");
    assert_eq!(rows(&loaded["active"]).len(), 2);
    assert_eq!(loaded["lists"]["explicit"][0]["range"], 0.5);
    assert_eq!(
        row["fresh"]["lists"]["explicit"][0]["field_types"]["range"],
        "number"
    );
    assert_eq!(row["fresh"]["lists"]["explicit"][0]["range"], 0.5);
    assert_eq!(row["fresh"]["lists"], loaded["lists"]);
    assert_eq!(row["fresh"]["active"], loaded["active"]);
    assert_eq!(row["fresh_after_reparse"], row["fresh"]);
    for (name, value) in [("ColdResist", 25), ("Life", 10)] {
        let records = rows(&row["player_records"][name]);
        assert_eq!(records.len(), 2, "two uses of one physical ring: {name}");
        let mut slots = records
            .iter()
            .map(|r| r["source_slot"].as_str().unwrap())
            .collect::<Vec<_>>();
        slots.sort_unstable();
        assert_eq!(slots, ["Ring 1", "Ring 2"]);
        for record in records {
            assert_eq!(record["value"], value);
            assert_eq!(record["type"], "BASE");
            assert_eq!(record["source"], "Item:26:New Item, Sapphire Ring");
        }
    }
    for slot in rows(&row["slot_lists"]) {
        let active = json!({"active":slot["active"]});
        assert_eq!(values(&active, "ColdResist"), vec![json!(25)]);
        assert_eq!(values(&active, "Life"), vec![json!(10)]);
    }
    let quality = &cases[1]["state"]["items"][0];
    assert_eq!(quality["loaded"]["quality"], 20);
    assert_eq!(quality["loaded"]["lists"], loaded["lists"]);
    assert_eq!(quality["loaded"]["base_mods"], loaded["base_mods"]);
    for slot in rows(&quality["slot_lists"]) {
        let active = json!({"active":slot["active"]});
        let name = if slot["slot"] == 2 {
            "Multiplier:QualityOnRing 2"
        } else {
            "Multiplier:QualityOnRing 1"
        };
        assert_eq!(values(&active, name), vec![json!(20)]);
        assert_eq!(values(&active, "ColdResist"), vec![json!(25)]);
    }
    for (index, cold, fraction) in [(2, 20, 0), (3, 30, 1)] {
        let current = &cases[index]["state"]["items"][0];
        assert_eq!(current["loaded"]["lists"]["implicit"][0]["range"], fraction);
        assert_eq!(values(&current["loaded"], "ColdResist"), vec![json!(cold)]);
        assert_eq!(values(&current["loaded"], "Life"), vec![json!(10)]);
        // Fresh ParseRaw sees only inline0.5; complete loader applies ModRange children afterward.
        assert_eq!(values(&current["fresh"], "ColdResist"), vec![json!(25)]);
        assert_eq!(
            rows(&current["player_records"]["ColdResist"])
                .iter()
                .map(|r| r["value"].clone())
                .collect::<Vec<_>>(),
            vec![json!(cold), json!(cold)]
        );
    }
    let variant = &cases[4]["state"]["items"][0]["loaded"];
    assert_eq!(variant["variant"], 2);
    assert_eq!(rows(&variant["lists"]["explicit"]).len(), 3);
    assert_eq!(values(variant, "Life"), vec![json!(10), json!(5)]);
    let fallback = &cases[5]["state"]["items"][0];
    assert_eq!(fallback["loaded"]["jewelSocketCount"], 1);
    assert_eq!(fallback["fresh"]["jewelSocketCount"], 0);
    let probes = rows(&row["probes"]);
    assert_eq!(probes.len(), 44);
    let p = |name: &str| &probes.iter().find(|p| p["name"] == name).unwrap()["after"];
    assert_eq!(p("quality-zero")["quality"], 0);
    assert_eq!(p("quality-duplicate")["quality"], 20);
    assert_eq!(p("quality-malformed")["field_types"]["quality"], "nil");
    assert_eq!(p("catalyst-style-quality")["field_types"]["quality"], "nil");
    assert_eq!(p("catalyst-kind")["field_types"]["catalystQuality"], "nil");
    assert_eq!(p("catalyst-zero")["catalystQuality"], 0);
    assert_eq!(p("catalyst-amount-only")["field_types"]["catalyst"], "nil");
    assert_eq!(p("catalyst-amount-only")["catalystQuality"], 37);
    for (name, cold) in [
        ("catalyst-style-quality", 30),
        ("catalyst-kind", 30),
        ("catalyst-zero", 25),
        ("catalyst-amount-only", 25),
        ("catalyst-wrong", 25),
        ("fractional-range", 26),
        ("implicit-magnitude", 37),
        ("explicit-magnitude", 25),
        ("corrupted-range", 38),
        ("range-reversed", 25),
        ("fixed-positive", 25),
    ] {
        assert_eq!(values(p(name), "ColdResist"), vec![json!(cold)], "{name}");
    }
    assert_eq!(
        p("fixed-positive")["lists"]["implicit"][0]["field_types"]["extra"],
        "nil"
    );
    assert!(rows(&p("fixed-positive")["lists"]["implicit"][0]["modTags"]).is_empty());
    for (name, level) in [
        ("level-zero", 0),
        ("level-high", 77),
        ("level-absent", 12),
        ("level-alias", 77),
        ("level-duplicate", 77),
        ("level-malformed", 12),
    ] {
        assert_eq!(p(name)["requirements"]["level"], level, "{name}");
    }
    assert_eq!(p("crafted-false")["crafted"], true);
    assert_eq!(p("saved-affix-label")["lists"], row["fresh"]["lists"]);
    for (name, field) in [
        ("corrupted", "corrupted"),
        ("twice-corrupted", "doubleCorrupted"),
        ("mirrored", "mirrored"),
        ("desecrated-setter", "desecrated"),
    ] {
        assert_eq!(p(name)[field], true);
    }
    assert_eq!(p("empty-rune-socket")["itemSocketCount"], 1);
    assert_eq!(p("empty-rune-socket")["runes"], json!(["None"]));
    assert_eq!(p("jewel-socket")["jewelSocketCount"], 1);
    assert_eq!(p("unknown-rune")["runes"], json!(["OwnedUnknownRune"]));
    for name in ["no-implicit-count", "implicit-zero"] {
        assert!(rows(&p(name)["lists"]["implicit"]).is_empty());
        assert_eq!(rows(&p(name)["lists"]["explicit"]).len(), 2);
    }
    assert_eq!(rows(&p("implicit-two")["lists"]["implicit"]).len(), 2);
    assert!(rows(&p("implicit-two")["lists"]["explicit"]).is_empty());
    assert!(values(p("missing-cold"), "ColdResist").is_empty());
    assert!(rows(&p("missing-life")["lists"]["explicit"]).is_empty());
    assert_eq!(rows(&p("duplicate-cold")["lists"]["explicit"]).len(), 2);
    assert_eq!(rows(&p("extra-member")["lists"]["explicit"]).len(), 2);
    assert_eq!(
        p("unknown-before")["lists"]["implicit"][0]["field_types"]["extra"],
        "string"
    );
    assert_eq!(rows(&p("enchant-cold")["lists"]["enchant"]).len(), 1);
    assert_eq!(
        p("reversed-lines")["lists"]["implicit"][0]["line"],
        "+10 to maximum Life"
    );
    assert_eq!(
        p("reversed-lines")["lists"]["explicit"][0]["line"],
        "+(20-30)% to Cold Resistance"
    );
    assert_eq!(p("class-restricted")["classRestriction"], "Witch");
    assert_eq!(row["reparsed"]["field_types"]["quality"], "nil");
    assert_eq!(row["reparsed"]["corrupted"], true);
    assert_eq!(row["reparsed"]["catalyst"], p("catalyst-kind")["catalyst"]);
    assert_eq!(row["reparsed"]["catalystQuality"], 37);
    assert_eq!(row["reparsed"]["itemSocketCount"], 0);
    // Literal +0 is an independent member. A partial parsed record alone does
    // not establish that for other zero/bare spellings: their remainder keeps
    // ColdResist out of the complete active list.
    let zero = p("fixed-zero");
    assert_eq!(rows(&zero["lists"]["implicit"]).len(), 1);
    assert_eq!(rows(&zero["lists"]["explicit"]).len(), 1);
    let zero_member = &zero["lists"]["implicit"][0];
    assert_eq!(zero_member["line"], "+0% to Cold Resistance");
    assert_eq!(zero_member["field_types"]["extra"], "nil");
    assert!(zero_member["extra"].is_null());
    assert!(rows(&zero_member["modTags"]).is_empty());
    assert_eq!(rows(&zero_member["records"]).len(), 1);
    assert_eq!(zero_member["records"][0]["name"], "ColdResist");
    assert_eq!(zero_member["records"][0]["type"], "BASE");
    assert_eq!(zero_member["records"][0]["value"], 0);
    assert_eq!(zero_member["records"][0]["flags"], 0);
    assert_eq!(zero_member["records"][0]["keyword_flags"], 0);
    assert!(rows(&zero_member["records"][0]["tags"]).is_empty());
    assert_eq!(
        zero_member["records"][0]["source"],
        "Item:26:New Item, Sapphire Ring"
    );
    assert_eq!(rows(&zero["active"]).len(), 2);
    assert_eq!(values(zero, "ColdResist"), vec![json!(0)]);
    assert_eq!(values(zero, "Life"), vec![json!(10)]);
    for (name, line, remainder, partial) in [
        (
            "fixed-negative-zero",
            "-0% to Cold Resistance",
            "  +10 to maximum Life ",
            0.0,
        ),
        (
            "bare-positive",
            "25% to Cold Resistance",
            "% to  +10 to maximum Life ",
            25.0,
        ),
        (
            "range-zero",
            "+(0-0)% to Cold Resistance",
            "  +10 to maximum Life ",
            0.0,
        ),
    ] {
        let current = p(name);
        assert_eq!(rows(&current["lists"]["implicit"]).len(), 1, "{name}");
        assert_eq!(rows(&current["lists"]["explicit"]).len(), 1, "{name}");
        let member = &current["lists"]["implicit"][0];
        assert_eq!(member["line"], line, "{name}");
        assert_eq!(member["field_types"]["extra"], "string", "{name}");
        assert_eq!(member["extra"], remainder, "{name}");
        assert_eq!(rows(&member["records"]).len(), 1, "{name}");
        assert_eq!(member["records"][0]["name"], "ColdResist", "{name}");
        assert_eq!(
            member["records"][0]["value"].as_f64(),
            Some(partial),
            "{name}"
        );
        assert!(member["records"][0]["source"].is_null(), "{name}");
        assert_eq!(rows(&current["active"]).len(), 1, "{name}");
        assert!(values(current, "ColdResist").is_empty(), "{name}");
        assert_eq!(values(current, "Life"), vec![json!(10)], "{name}");
    }
}
