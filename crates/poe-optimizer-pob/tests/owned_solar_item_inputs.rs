//! Solar physical input contrasts; no native owner or whole-build coverage authority.
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
#[path = "support/solar_item_inputs_source.rs"]
mod observer;
use observer::OBSERVE;
const TEST: &str = "solar_items_preserve_absence_and_finite_implicit_explicit_inputs";
const CHILD: &str = "POE_SOLAR_ITEM_INPUTS_CHILD";

#[test]
fn solar_items_preserve_absence_and_finite_implicit_explicit_inputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-solar-item-inputs-source-01");
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
                    && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("23")
            })
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].occurrence().id().ordinal(), 580);
        let quality_zero = edit_item(&xml, 23, |body| {
            body.replace("Crafted: true", "Quality: 0\nCrafted: true")
        });
        let quality_twenty = edit_item(&xml, 23, |body| {
            body.replace("Crafted: true", "Quality: 20\nCrafted: true")
        });
        let changed_headers = edit_item(&xml, 23, |body| {
            body.replace(
                "Suffix: {range:0}GlobalMinionSpellSkillGemLevel1",
                "Suffix: None",
            )
        });
        let changed_variants=edit_item(&xml,23,|body|body.replace("Crafted: true","Variant: First\nVariant: Second\nCrafted: true").replace("+1 to Level of all Minion Skills","+1 to Level of all Minion Skills\n{variant:1}+3 to maximum Life\n{variant:2}+5 to maximum Life")).replace("<Item id=\"23\">","<Item id=\"23\" variant=\"2\">");
        let fallback_title = edit_item(&xml, 23, |body| {
            body.replace("New Item", "Sekhema's Resolve")
        });
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text) in [
            ("original", &xml),
            ("quality-zero", &quality_zero),
            ("quality-twenty", &quality_twenty),
            ("display-and-affix-headers", &changed_headers),
            ("xml-variant-two", &changed_variants),
            ("loader-fallback-title", &fallback_title),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("solarItemXml", text.as_str())?;
                lua.globals().set("solarItemControls", name == "original")?;
                lua.globals().set("solarItemJit", enabled)?;
                lua.load("if solarItemJit then jit.on() else jit.off();jit.flush() end")
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
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/Bases/amulet.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
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
        .set_name("@solar-item-inputs-observer")
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
        let s = &case["state"];
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(s[flag], true);
        }
        for (key, id) in [
            ("selected_items", 2),
            ("selected_spec", 3),
            ("selected_skills", 4),
            ("selected_config", 1),
        ] {
            assert_eq!(s[key], id);
        }
        assert!(s["main_output"].as_object().is_some_and(|o| !o.is_empty()));
        assert_eq!(rows(&s["items"]).len(), 1);
        let row = &s["items"][0];
        assert_eq!(row["id"], 23);
        assert_eq!(row["selected_slots"], json!(["Amulet"]));
        assert_eq!(row["loaded"]["base"], "Solar Amulet");
    }
    let row = &cases[0]["state"]["items"][0];
    let s = &row["loaded"];
    assert_eq!(s["rarity"], "RARE");
    assert_eq!(s["crafted"], true);
    assert_eq!(s["requirements"]["level"], 30);
    for key in [
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
        assert_eq!(s["field_types"][key], "nil", "{key}");
    }
    assert_eq!(s["itemSocketCount"], 0);
    assert_eq!(s["jewelSocketCount"], 0);
    assert!(rows(&s["sockets"]).is_empty());
    assert!(rows(&s["runes"]).is_empty());
    assert_eq!(row["base_facts"]["quality"], Json::Null);
    assert_eq!(row["base_facts"]["socket_limit"], Json::Null);
    assert_eq!(row["base_facts"]["implicit"], "+(10-15) to Spirit");
    assert_eq!(row["base_facts"]["flask"], Json::Null);
    assert_eq!(row["base_facts"]["charm"], Json::Null);
    for category in ["buff", "enchant", "rune", "classRequirement"] {
        assert!(rows(&s["lists"][category]).is_empty());
    }
    for (category, line) in [
        ("implicit", "+(10-15) to Spirit"),
        ("explicit", "+1 to Level of all Minion Skills"),
    ] {
        assert_eq!(rows(&s["lists"][category]).len(), 1);
        let member = &s["lists"][category][0];
        assert_eq!(member["line"], line);
        assert_eq!(member["field_types"]["extra"], "nil");
        assert!(rows(&member["modTags"]).is_empty());
        assert_eq!(member["catalyst_factor"], 1);
    }
    assert_eq!(rows(&s["base_mods"]).len(), 2);
    assert_eq!(rows(&s["active"]).len(), 2);
    assert_eq!(s["base_mods"][0]["name"], "Spirit");
    assert_eq!(s["base_mods"][1]["name"], "GemProperty");
    assert_eq!(values(s, "Spirit"), vec![json!(13)]);
    assert_eq!(s["active"][1]["value"]["value"], 1);
    assert!(values(s, "Multiplier:QualityOnAmulet").is_empty());
    assert_eq!(row["fresh"]["active"], s["active"]);
    assert_eq!(row["fresh"]["lists"], s["lists"]);
    assert_eq!(row["fresh_after_reparse"], row["fresh"]);
    assert_eq!(row["reparsed"]["field_types"]["quality"], "nil");
    assert_eq!(row["reparsed"]["corrupted"], true);
    assert_eq!(row["reparsed"]["catalyst"], 2);
    assert_eq!(row["reparsed"]["catalystQuality"], 37);
    assert_eq!(row["reparsed"]["itemSocketCount"], 0);
    for (index, quality) in [(1, 0), (2, 20)] {
        let changed = &cases[index]["state"]["items"][0]["loaded"];
        assert_eq!(changed["quality"], quality);
        assert_eq!(changed["field_types"]["catalyst"], "nil");
        assert_eq!(
            values(changed, "Multiplier:QualityOnAmulet"),
            vec![json!(quality)]
        );
        assert_eq!(changed["lists"], s["lists"]);
        assert_eq!(changed["base_mods"], s["base_mods"]);
        assert_eq!(values(changed, "Spirit"), vec![json!(13)]);
    }
    let labels = &cases[3]["state"]["items"][0]["loaded"];
    assert_eq!(labels["active"], s["active"]);
    assert_eq!(labels["lists"], s["lists"]);
    assert_eq!(labels["suffixes"][0]["modId"], "None");
    let variants = &cases[4]["state"]["items"][0]["loaded"];
    assert_eq!(variants["variant"], 2);
    assert_eq!(rows(&variants["lists"]["explicit"]).len(), 3);
    assert_eq!(values(variants, "Life"), vec![json!(5)]);
    let fallback = &cases[5]["state"]["items"][0];
    assert_eq!(fallback["loaded"]["jewelSocketCount"], 1);
    assert_eq!(fallback["fresh"]["jewelSocketCount"], 0);
    let probes = rows(&row["probes"]);
    assert_eq!(probes.len(), 32);
    let p = |name: &str| &probes.iter().find(|p| p["name"] == name).unwrap()["after"];
    assert_eq!(p("quality-duplicate")["quality"], 20);
    assert_eq!(p("catalyst-style-quality")["field_types"]["quality"], "nil");
    assert_eq!(p("catalyst-style-quality")["catalyst"], 2);
    assert_eq!(p("catalyst-style-quality")["catalystQuality"], 20);
    assert_eq!(p("catalyst-kind")["catalyst"], 2);
    assert_eq!(p("catalyst-kind")["field_types"]["catalystQuality"], "nil");
    assert_eq!(p("catalyst-zero")["catalystQuality"], 0);
    assert_eq!(p("catalyst-amount-only")["field_types"]["catalyst"], "nil");
    assert_eq!(p("catalyst-amount-only")["catalystQuality"], 37);
    for name in [
        "catalyst-kind",
        "catalyst-zero",
        "catalyst-amount-only",
        "catalyst-style-quality",
    ] {
        assert_eq!(p(name)["lists"]["implicit"][0]["catalyst_factor"], 1);
        assert_eq!(values(p(name), "Spirit"), vec![json!(13)]);
    }
    assert_eq!(
        p("catalyst-matching-tag")["lists"]["implicit"][0]["catalyst_factor"],
        1.2
    );
    assert_eq!(
        values(p("catalyst-matching-tag"), "Spirit"),
        vec![json!(15)]
    );
    for (name, level) in [
        ("level-zero", 0),
        ("level-high", 77),
        ("level-absent", 30),
        ("level-alias", 77),
        ("level-duplicate", 77),
    ] {
        assert_eq!(p(name)["requirements"]["level"], level, "{name}");
    }
    assert_eq!(p("crafted-false")["crafted"], true);
    assert_eq!(p("corrupted")["corrupted"], true);
    assert_eq!(p("twice-corrupted")["doubleCorrupted"], true);
    assert_eq!(p("mirrored")["mirrored"], true);
    assert_eq!(p("desecrated-setter")["desecrated"], true);
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
    // Base.implicit is a definition, not a license to invent a missing raw member.
    assert!(values(p("missing-spirit"), "Spirit").is_empty());
    assert_eq!(rows(&p("missing-spirit")["lists"]["implicit"]).len(), 1);
    assert!(rows(&p("missing-minion")["lists"]["explicit"]).is_empty());
    assert_eq!(rows(&p("duplicate-spirit")["lists"]["explicit"]).len(), 2);
    assert_eq!(rows(&p("extra-member")["lists"]["explicit"]).len(), 2);
    assert_eq!(
        p("unknown-before")["lists"]["implicit"][0]["field_types"]["extra"],
        "string"
    );
    assert_eq!(rows(&p("enchant-spirit")["lists"]["enchant"]).len(), 1);
    assert_eq!(
        p("reversed-lines")["lists"]["implicit"][0]["line"],
        "+1 to Level of all Minion Skills"
    );
    assert_eq!(
        p("reversed-lines")["lists"]["explicit"][0]["line"],
        "+(10-15) to Spirit"
    );
    assert_eq!(p("class-restricted")["classRestriction"], "Witch");
}
