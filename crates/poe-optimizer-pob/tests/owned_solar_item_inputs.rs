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

const OWNERSHIP_TEST: &str = "solar_ownership_observes_original_quality_and_requirement_consumers";
const OWNERSHIP_CHILD: &str = "POE_SOLAR_OWNERSHIP_CHILD";
const OWNERSHIP_OUTPUT: &str = "POE_SOLAR_OWNERSHIP_SOURCE_OUT";
const OWNERSHIP_STAGES: [&str; 3] = ["fresh", "rebuild-one", "rebuild-two"];

#[test]
#[ignore = "complete original PoB calls in fresh isolated JIT modes; source evidence only"]
fn solar_ownership_observes_original_quality_and_requirement_consumers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OWNERSHIP_OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("runs/owned-solar-ownership-source-01"));
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(mode) = std::env::var_os(OWNERSHIP_CHILD) {
        assert!(mode == "off" || mode == "on");
        solar_ownership_child(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "source destination must be new: {}",
        out.display()
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", OWNERSHIP_TEST, "--nocapture"])
            .env(OWNERSHIP_CHILD, mode)
            .env(OWNERSHIP_OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "Solar ownership source child failed: {}",
                    path.display()
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(900) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Solar ownership source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}

fn solar_ownership_child(root: &Path, out: &Path, enabled: bool) {
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let xml = fs::read_to_string(dir.join("build-05.xml")).unwrap();
    let manifest = read(&dir.join("index.json"));
    let original = rows(&manifest["builds"])
        .iter()
        .find(|r| r["xml"] == "build-05.xml")
        .unwrap();
    assert_eq!(original["xml_sha256"], digest(xml.as_bytes()));
    let quality = |n| {
        edit_item(&xml, 23, |s| {
            s.replace("Crafted: true", &format!("Quality: {n}\nCrafted: true"))
        })
    };
    let zero = quality(0);
    let twenty = quality(20);
    let level = edit_item(&xml, 23, |s| s.replace("LevelReq: 30", "LevelReq: 77"));
    let mut cases = Vec::new();
    for (name, input, warm, raw_quality, raw_level, diagnostic) in [
        ("original", xml.as_str(), None, None, 30, false),
        ("independent-repeat", xml.as_str(), None, None, 30, false),
        (
            "warm-quality-twenty-then-original",
            xml.as_str(),
            Some(twenty.as_str()),
            None,
            30,
            false,
        ),
        (
            "source-quality-zero",
            zero.as_str(),
            None,
            Some(0),
            30,
            true,
        ),
        (
            "source-quality-twenty",
            twenty.as_str(),
            None,
            Some(20),
            30,
            true,
        ),
        (
            "raw-level-seventy-seven",
            level.as_str(),
            None,
            None,
            77,
            false,
        ),
    ] {
        let observed = solar_ownership_observe(root, input, warm, enabled, true, name);
        let unhooked = solar_ownership_observe(root, input, warm, enabled, false, name);
        for stage in OWNERSHIP_STAGES {
            assert_eq!(
                solar_ownership_without_trace(&observed["states"][stage]),
                solar_ownership_without_trace(&unhooked["states"][stage]),
                "hooked/unhooked {name} {stage}"
            );
        }
        cases.push(json!({"name": name, "expected_raw_quality": raw_quality, "expected_raw_level": raw_level,
            "source_only_quality_control": diagnostic, "game_obtainability_authority": false,
            "observed": observed, "unhooked": unhooked}));
    }
    let files = [
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Modules/Build.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/ModParser.lua",
        "src/Data/Bases/amulet.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcPerform.lua",
        "src/HeadlessWrapper.lua",
    ];
    let result = json!({"schema_version":1, "source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(), "observer_sha256":digest(observer::OWNERSHIP_OBSERVER.as_bytes()),
        "files":files.map(|p| json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})),
        "native_input_closure":false, "native_owner_coverage":false, "cases":cases});
    let bytes = serde_json::to_vec_pretty(&result).unwrap();
    assert!(bytes.len() < 16 * 1024 * 1024);
    let suffix = if enabled { "on" } else { "off" };
    fs::write(out.join(format!("source-jit-{suffix}.raw.json")), &bytes).unwrap();
    solar_ownership_check(&result);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    assert_eq!(fs::read_to_string(dir.join("build-05.xml")).unwrap(), xml);
}

fn solar_ownership_observe(
    root: &Path,
    xml: &str,
    warm: Option<&str>,
    enabled: bool,
    instrumented: bool,
    name: &str,
) -> Json {
    eprintln!("Solar ownership {name}, instrumented={instrumented}, JIT={enabled}");
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("solarOwnershipJit", enabled)?;
        lua.globals()
            .set("solarOwnershipInstrumented", instrumented)?;
        lua.load("if solarOwnershipJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<mlua::Function, RuntimeError> {
        let api: mlua::Table = lua
            .load(observer::OWNERSHIP_OBSERVER)
            .set_name("@solar-ownership-observer")
            .eval()?;
        let install: mlua::Function = api.get("install")?;
        let cleanup: mlua::Function = install.call(())?;
        lua.globals().set("solarOwnershipApi", api)?;
        Ok(cleanup)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("solarOwnershipApi")?;
        let observe: mlua::Function = api.get("observe")?;
        let rebuild: mlua::Function = api.get("rebuild")?;
        let mut states = serde_json::Map::new();
        for (index, stage) in OWNERSHIP_STAGES.iter().enumerate() {
            if index > 0 {
                rebuild.call::<()>(())?;
            }
            let a: Value = observe.call(())?;
            let a: Json = lua.from_value(a)?;
            let b: Value = observe.call(())?;
            let b: Json = lua.from_value(b)?;
            assert_eq!(a, b, "observer noninterference {name} {stage}");
            states.insert((*stage).into(), a);
        }
        Ok(Json::Object(states))
    };
    let scratch = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        warm,
        false,
        Some(&before),
        Some(&before_build),
        Some(&after),
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([53; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let items: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().name() == "Item"
                && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("23")
        })
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].occurrence().id().ordinal(), 580);
    json!({"source_hash": result["source_hash"], "xml_sha256":digest(xml.as_bytes()),
        "warm_xml_sha256":warm.map(|s|digest(s.as_bytes())), "source_identity":evidence.identity(),
        "source_item_ordinal":580, "source_item_id":23, "independent_source_binding_verified":true,
        "instrumented":instrumented, "states":result["additional_observation"]})
}

fn solar_ownership_without_trace(state: &Json) -> Json {
    let mut state = state.clone();
    state.as_object_mut().unwrap().remove("trace");
    state
}

fn solar_ownership_check(report: &Json) {
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    assert_eq!(
        report["observer_sha256"],
        digest(observer::OWNERSHIP_OBSERVER.as_bytes())
    );
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 6);
    let absent = json!({"kind":"absent"});
    for case in cases {
        let expected_quality = if case["expected_raw_quality"].is_null() {
            absent.clone()
        } else {
            case["expected_raw_quality"].clone()
        };
        for stage in OWNERSHIP_STAGES {
            let state = &case["observed"]["states"][stage];
            assert_eq!(state["original_functions_preserved"], true);
            assert_eq!(state["method_wrappers"], false);
            assert_eq!(state["observer_installed"], false);
            assert_eq!(state["base_catalogue_identity"], true);
            assert_eq!(state["item"]["exact_registered"], true);
            assert_eq!(state["item"]["exact_catalogue_base"], true);
            assert_eq!(state["item"]["quality"], expected_quality);
            assert_eq!(state["item"]["crafted_quality"], 0);
            assert_eq!(state["item"]["rarity"], "RARE");
            assert_eq!(state["item"]["corrupted"], absent);
            assert_eq!(state["item"]["catalyst"], absent);
            assert_eq!(state["item"]["catalyst_quality"], absent);
            assert_eq!(state["item"]["socket_count"], 0);
            assert!(rows(&state["item"]["granted_skills"]).is_empty());
            assert_eq!(
                state["item"]["requirements"]["level"],
                case["expected_raw_level"]
            );
            assert_eq!(
                state["base"],
                cases[0]["observed"]["states"]["fresh"]["base"]
            );
            assert_eq!(state["base"]["type"], "Amulet");
            assert_eq!(state["base"]["req"], json!({"level":30}));
            for key in [
                "quality",
                "weapon",
                "armour",
                "flask",
                "charm",
                "socketLimit",
            ] {
                assert!(
                    state["base"].get(key).is_none(),
                    "unexpected intrinsic Solar {key}"
                );
            }
            for (key, id) in [("items", 2), ("spec", 3), ("skills", 4), ("config", 1)] {
                assert_eq!(state["selected"][key], id);
            }
            for category in ["buff", "enchant", "rune", "classRequirement"] {
                assert!(rows(&state["members"][category]).is_empty());
            }
            assert_eq!(rows(&state["members"]["implicit"]).len(), 1);
            assert_eq!(rows(&state["members"]["explicit"]).len(), 1);
            let active = rows(&state["item"]["active_modifiers"]);
            assert_eq!(active.iter().filter(|m| m["name"] == "Spirit").count(), 1);
            assert_eq!(
                active.iter().find(|m| m["name"] == "Spirit").unwrap()["value"],
                13
            );
            assert_eq!(
                active.iter().filter(|m| m["name"] == "GemProperty").count(),
                1
            );
            assert_eq!(
                active.iter().find(|m| m["name"] == "GemProperty").unwrap()["value"]["value"],
                1
            );
            let qualities: Vec<_> = active
                .iter()
                .filter(|m| m["name"] == "Multiplier:QualityOnAmulet")
                .collect();
            if expected_quality == absent {
                assert!(qualities.is_empty());
                assert_eq!(active.len(), 2);
            } else {
                assert_eq!(qualities.len(), 1);
                assert_eq!(qualities[0]["value"], expected_quality);
                assert_eq!(active.len(), 3);
            }
            let envs = rows(&state["environments"]);
            assert_eq!(envs.len(), 2);
            for env in envs {
                assert_eq!(env["selected_item_exact"], true);
                let requirements = rows(&env["requirements_rows"]);
                assert_eq!(requirements.len(), 1);
                for attr in ["Str", "Dex", "Int"] {
                    assert_eq!(requirements[0][attr], 0);
                    assert_eq!(env["amulet_requirement_outputs"][attr], absent);
                }
            }
            let trace = &state["trace"];
            let traced = rows(&trace["environments"]);
            for mode in ["MAIN", "CALCS"] {
                let matching: Vec<_> = traced
                    .iter()
                    .filter(|e| e["mode"] == mode && e["exact_current_environment"] == true)
                    .collect();
                assert_eq!(matching.len(), 1, "exact current {mode} environment");
                for env in matching {
                    assert_eq!(env["performed"], true);
                    assert_eq!(rows(&env["deliveries"]).len(), 1);
                    assert_eq!(env["deliveries"][0]["exact_merged_row"], true);
                    assert!(rows(&env["positive_branches"]).is_empty());
                    let reads = rows(&env["consumers"]);
                    for attr in ["Str", "Dex", "Int"] {
                        let relevant: Vec<_> =
                            reads.iter().filter(|r| r["attribute"] == attr).collect();
                        assert_eq!(relevant.len(), 1, "exact original predicate visit");
                        assert_eq!(relevant[0]["input"], 0);
                        assert_eq!(relevant[0]["exact_delivered_row"], true);
                        assert_eq!(relevant[0]["row"]["exact_source_item"], true);
                    }
                    assert_eq!(rows(&env["item_returns"]).len(), 1);
                    assert_eq!(
                        env["item_returns"][0]["records"],
                        state["item"]["active_modifiers"]
                    );
                }
            }
            for call in rows(&trace["local_calls"]) {
                assert_eq!(call["original_call"], true);
                assert_eq!(call["return_observed"], true);
                assert_eq!(
                    call["before"], call["after"],
                    "actual pair supplies no consumed local record"
                );
                if call["type"] == "FLAG" {
                    assert_eq!(call["result"], false);
                } else {
                    assert_eq!(call["result"], 0);
                }
            }
            if stage == "fresh" {
                assert!(!rows(&trace["item_builds"]).is_empty());
                assert!(
                    rows(&trace["local_calls"])
                        .iter()
                        .any(|c| c["name"] == "Quality" && c["caller_line"] == 2443)
                );
                for attr in ["StrRequirement", "DexRequirement", "IntRequirement"] {
                    for kind in ["BASE", "INC"] {
                        assert!(
                            rows(&trace["local_calls"])
                                .iter()
                                .any(|c| c["name"] == attr && c["type"] == kind)
                        );
                    }
                }
            }
        }
        assert_eq!(
            solar_ownership_without_trace(&case["observed"]["states"]["rebuild-one"]),
            solar_ownership_without_trace(&case["observed"]["states"]["rebuild-two"]),
            "fixed rebuilt determinism"
        );
    }
    for index in [1, 2] {
        for stage in OWNERSHIP_STAGES {
            assert_eq!(
                solar_ownership_without_trace(&cases[0]["observed"]["states"][stage]),
                solar_ownership_without_trace(&cases[index]["observed"]["states"][stage]),
                "fresh replay/warm restoration"
            );
        }
    }
}
