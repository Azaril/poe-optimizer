//! Complete source item inventories; no native input/owner closure authority.
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
#[path = "support/armour_item_input_source.rs"]
mod observer;
use observer::OBSERVE;
const TEST: &str = "armour_items_have_bounded_physical_inputs_and_singleton_source_members";
const CHILD: &str = "POE_ARMOUR_ITEM_INPUTS_CHILD";

#[test]
fn armour_items_have_bounded_physical_inputs_and_singleton_source_members() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-armour-item-inputs-source-01");
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
        for (id, ordinal) in [("21", 576), ("22", 578)] {
            let matches: Vec<_> = evidence
                .rows()
                .iter()
                .filter(|r| {
                    r.occurrence().name() == "Item"
                        && r.attribute("id").and_then(|a| a.decoded().ok()) == Some(id)
                })
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].occurrence().id().ordinal(), ordinal);
        }
        let changed_headers = edit_item(&edit_item(&xml, 21, change_headers), 22, change_headers);
        let changed_variants = edit_item(&xml, 22, |body| {
            body.replace(
                "Crafted: true",
                "Variant: First\nVariant: Second\nCrafted: true",
            )
            .replace(
                "10% increased Movement Speed",
                "10% increased Movement Speed\n{variant:1}3% increased Movement Speed\n{variant:2}5% increased Movement Speed",
            )
        })
        .replace("<Item id=\"22\">", "<Item id=\"22\" variant=\"2\">");
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text) in [
            ("original", &xml),
            ("display-and-affix-headers", &changed_headers),
            ("xml-variant-two", &changed_variants),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("simpleItemXml", text.as_str())?;
                lua.globals()
                    .set("simpleItemControls", name == "original")?;
                lua.globals().set("simpleItemJit", enabled)?;
                lua.load("if simpleItemJit then jit.on() else jit.off();jit.flush() end")
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
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/Bases/helmet.lua","src/Data/Bases/boots.lua","src/Data/Bases/flask.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
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
fn change_headers(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            if line.starts_with("Energy Shield:") {
                "Energy Shield: 9999"
            } else if line.starts_with("Armour:") {
                "Armour: 9998"
            } else if line.starts_with("Prefix:") {
                "Prefix: None"
            } else if line.starts_with("Suffix:") {
                "Suffix: None"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-armour-item-inputs-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}

fn named_values(snapshot: &Json, name: &str) -> Vec<Json> {
    rows(&snapshot["active"])
        .iter()
        .filter(|r| r["name"] == name)
        .map(|r| r["value"].clone())
        .collect()
}

fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 3);
    for case in cases {
        let state = &case["state"];
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[flag], true, "{flag}");
        }
        assert_eq!(state["selected_items"], 2);
        assert_eq!(state["selected_spec"], 3);
        assert_eq!(state["selected_skills"], 4);
        assert_eq!(state["selected_config"], 1);
        assert!(
            state["main_output"]
                .as_object()
                .is_some_and(|o| !o.is_empty())
        );
    }
    let original = rows(&cases[0]["state"]["items"]);
    assert_eq!(original.len(), 2);
    for (index, (id, base, slot, level, armour, es, kind)) in [
        (21, "Iron Crown", "Helmet", 5, 26, 13, 13),
        (22, "Cryptic Leggings", "Boots", 80, 134, 37, 11),
    ]
    .into_iter()
    .enumerate()
    {
        let item = &original[index];
        let loaded = &item["loaded"];
        assert_eq!(item["id"], id);
        assert_eq!(item["selected_slots"], json!([slot]));
        assert_eq!(
            item["xml"]["children"],
            json!([{"name":"ModRange","attributes":{"id":"1","range":"0.5"}}])
        );
        assert_eq!(loaded["base"], base);
        assert_eq!(loaded["rarity"], "RARE");
        assert_eq!(loaded["crafted"], true);
        assert_eq!(loaded["quality"], 20);
        assert_eq!(loaded["requirements"]["level"], level);
        for key in [
            "itemLevel",
            "corrupted",
            "catalyst",
            "catalystQuality",
            "variant",
            "variantAlt",
            "hasAltVariant",
            "allowDuplicateVariants",
            "classRestriction",
        ] {
            assert_eq!(loaded["field_types"][key], "nil", "{base}/{key}");
        }
        assert_eq!(loaded["itemSocketCount"], 3);
        assert_eq!(loaded["jewelSocketCount"], 0);
        assert_eq!(loaded["runes"], json!(["None", "None", "None"]));
        for category in ["buff", "implicit", "enchant", "rune", "classRequirement"] {
            assert!(
                rows(&loaded["lists"][category]).is_empty(),
                "{base}/{category}"
            );
        }
        let physical = rows(&loaded["lists"]["explicit"]);
        assert_eq!(physical.len(), 1);
        assert_eq!(physical[0]["field_types"]["extra"], "nil");
        assert!(rows(&physical[0]["modTags"]).is_empty());
        assert_eq!(physical[0]["catalyst_factor"], 1);
        assert_eq!(rows(&physical[0]["records"]).len(), 1);
        assert_eq!(rows(&loaded["base_mods"]).len(), 1);
        assert_eq!(item["base_facts"]["implicit"], Json::Null);
        assert_eq!(item["base_facts"]["flask"], Json::Null);
        assert_eq!(item["base_facts"]["charm"], Json::Null);
        assert!(rows(&item["base_facts"]["implicit_mod_types"]).is_empty());
        assert_eq!(item["base_facts"]["socket_limit"], 3);
        assert_eq!(item["base_facts"]["armour"]["Armour"], armour);
        assert_eq!(item["base_facts"]["armour"]["EnergyShield"], es);
        assert_eq!(item["fresh"]["active"], loaded["active"]);
        assert_eq!(item["fresh"]["base_mods"], loaded["base_mods"]);
        assert_eq!(item["fresh_after_reparse"], item["fresh"]);
        assert_eq!(item["reparsed"]["corrupted"], true);
        assert_eq!(item["reparsed"]["itemLevel"], 77);
        assert_eq!(
            named_values(loaded, &format!("Multiplier:QualityOn{slot}")),
            vec![json!(20)]
        );
        // The complete source active list carries this derived quality record;
        // it is separate from the singleton physical explicit modifier.
        assert_eq!(rows(&loaded["active"]).len(), 2);
        assert!(!rows(&item["slot_lists"]).is_empty());
        for per_slot in rows(&item["slot_lists"]) {
            assert_eq!(per_slot["active"], loaded["active"]);
        }
        let changed = &cases[1]["state"]["items"][index]["loaded"];
        assert_eq!(changed["base_mods"], loaded["base_mods"]);
        assert_eq!(changed["active"], loaded["active"]);
        assert_eq!(changed["armourData"], loaded["armourData"]);
        assert_eq!(changed["prefixes"][0]["modId"], "None");
        assert_eq!(changed["suffixes"][0]["modId"], "None");
        let probes = rows(&item["probes"]);
        assert_eq!(probes.len(), if index == 0 { 19 } else { 16 });
        let p = |name: &str| &probes.iter().find(|r| r["name"] == name).unwrap()["after"];
        assert_eq!(rows(&p("second-member")["lists"]["explicit"]).len(), 2);
        assert!(
            rows(&p("unknown-member")["lists"]["explicit"])
                .iter()
                .any(|r| r["extra"].is_string())
        );
        assert!(rows(&p("header-implicit")["lists"]["explicit"]).is_empty());
        assert_eq!(rows(&p("header-implicit")["lists"]["implicit"]).len(), 1);
        assert_eq!(rows(&p("enchant-member")["lists"]["enchant"]).len(), 1);
        assert!(!rows(&p("occupied-rune")["lists"]["rune"]).is_empty());
        assert!(
            rows(&p("unknown-rune")["runes"])
                .iter()
                .any(|r| r == "OwnedWitnessUnknownRune")
        );
        assert_eq!(p("level-high")["requirements"]["level"], 77);
        assert_eq!(p("level-absent")["requirements"]["level"], level);
        assert_eq!(p("quality-zero")["armourData"]["Armour"], armour);
        assert_eq!(p("quality-zero")["armourData"]["EnergyShield"], es);
        assert_eq!(p("corrupted")["corrupted"], true);
        assert_eq!(p("catalyst-kind")["catalyst"], kind);
        assert_eq!(p("catalyst-kind")["field_types"]["catalystQuality"], "nil");
        assert_eq!(p("catalyst-zero")["catalystQuality"], 0);
        assert_eq!(p("catalyst-amount-only")["field_types"]["catalyst"], "nil");
        assert_eq!(p("catalyst-amount-only")["catalystQuality"], 37);
        for name in [
            "catalyst-kind",
            "catalyst-zero",
            "catalyst-amount-only",
            "catalyst-wrong-tag",
        ] {
            assert_eq!(p(name)["lists"]["explicit"][0]["catalyst_factor"], 1);
        }
        assert_eq!(
            p("catalyst-matching-tag")["lists"]["explicit"][0]["catalyst_factor"],
            1.2
        );
        if index == 0 {
            assert_eq!(p("class-restricted")["classRestriction"], "Witch");
            assert_eq!(p("different-base")["base"], "Horned Crown");
            assert_ne!(p("different-base")["armourData"], loaded["armourData"]);
            assert!(!rows(&p("buff-base")["lists"]["buff"]).is_empty());
        }
    }
    assert_eq!(
        original[0]["loaded"]["lists"]["explicit"][0]["records"][0]["name"],
        "GemProperty"
    );
    assert_eq!(
        original[0]["loaded"]["lists"]["explicit"][0]["records"][0]["value"]["value"],
        1
    );
    assert_eq!(
        named_values(&original[1]["loaded"], "MovementSpeed"),
        vec![json!(10)]
    );
    let variant = &cases[2]["state"]["items"][1]["loaded"];
    assert_eq!(variant["variant"], 2);
    assert_eq!(rows(&variant["lists"]["explicit"]).len(), 3);
    assert_eq!(
        named_values(variant, "MovementSpeed"),
        vec![json!(10), json!(5)]
    );
}

const LOCAL_DEFENCE_TEST: &str =
    "armour_local_defences_observe_original_composition_and_slot_consumers";
const LOCAL_DEFENCE_CHILD: &str = "POE_ARMOUR_LOCAL_DEFENCE_CHILD";
const LOCAL_DEFENCE_OUT: &str = "POE_ARMOUR_LOCAL_DEFENCE_SOURCE_OUT";
const LOCAL_DEFENCE_STAGES: [&str; 3] = ["fresh", "rebuild-one", "rebuild-two"];

#[test]
#[ignore = "bounded complete original PoB calls; native closure and obtainability are separate"]
fn armour_local_defences_observe_original_composition_and_slot_consumers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(LOCAL_DEFENCE_OUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("runs/owned-armour-local-defence-source-01"));
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(mode) = std::env::var_os(LOCAL_DEFENCE_CHILD) {
        assert!(mode == "off" || mode == "on");
        local_defence_child(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "new source output required: {}",
        out.display()
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", LOCAL_DEFENCE_TEST, "--nocapture"])
            .env(LOCAL_DEFENCE_CHILD, mode)
            .env(LOCAL_DEFENCE_OUT, &out)
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
            if start.elapsed() > Duration::from_secs(900) {
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

fn local_defence_child(root: &Path, out: &Path, enabled: bool) {
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let xml = fs::read_to_string(dir.join("build-05.xml")).unwrap();
    let index = read(&dir.join("index.json"));
    let entry = rows(&index["builds"])
        .iter()
        .find(|x| x["xml"] == "build-05.xml")
        .unwrap();
    assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
    let quality = |q| {
        let change = |s: &str| {
            assert_eq!(s.matches("Quality: 20").count(), 1);
            s.replace("Quality: 20", &format!("Quality: {q}"))
        };
        edit_item(&edit_item(&xml, 21, change), 22, change)
    };
    let zero = quality(0);
    let thirty = quality(30);
    let lines = "\n+5 to Armour\n+7 to maximum Energy Shield\n+11 to Armour and Energy Shield\n+13 to Armour and Evasion Rating\n+17 to Evasion Rating and Energy Shield\n19% increased Armour\n23% increased Energy Shield\n29% increased Armour and Energy Shield\n31% increased Armour and Evasion\n37% increased Evasion and Energy Shield\n41% increased Defences\n";
    let insert_local_lines = |body: &str| {
        let child = body.find('<').expect("fixture ModRange child");
        assert!(body[..child].contains("Rarity: RARE"));
        assert_eq!(
            body[child..].trim(),
            r#"<ModRange range="0.5" id="1"/>"#,
            "exact single child after the original raw item text"
        );
        assert!(!body.contains(lines));
        let mut changed = body.to_owned();
        changed.insert_str(child, lines);
        assert_eq!(&changed[child + lines.len()..], &body[child..]);
        assert_eq!(changed.matches(lines).count(), 1);
        assert_eq!(changed.replacen(lines, "", 1), body);
        changed
    };
    let synthetic = edit_item(
        &edit_item(&xml, 21, insert_local_lines),
        22,
        insert_local_lines,
    );
    let mut cases = Vec::new();
    for (name, text, warm, q, nonzero) in [
        ("original", xml.as_str(), None, 20, false),
        ("independent-repeat", xml.as_str(), None, 20, false),
        (
            "warm-quality-thirty-then-original",
            xml.as_str(),
            Some(thirty.as_str()),
            20,
            false,
        ),
        ("raw-quality-zero", zero.as_str(), None, 0, false),
        ("raw-quality-thirty", thirty.as_str(), None, 30, false),
        (
            "synthetic-local-defences",
            synthetic.as_str(),
            None,
            20,
            true,
        ),
    ] {
        let observed = local_defence_observe(root, text, warm, enabled, true, name);
        let unhooked = local_defence_observe(root, text, warm, enabled, false, name);
        for stage in LOCAL_DEFENCE_STAGES {
            assert_eq!(
                local_defence_without_trace(&observed["states"][stage]),
                local_defence_without_trace(&unhooked["states"][stage]),
                "hook interference {name} {stage}"
            );
        }
        assert_eq!(
            local_defence_without_trace(&observed["constructors"]),
            local_defence_without_trace(&unhooked["constructors"]),
            "constructor hook interference {name}"
        );
        cases.push(json!({"name":name,"raw_quality":q,"synthetic_local_control":nonzero,"game_obtainability_authority":false,"native_source_admission_authority":false,"observed":observed,"unhooked":unhooked}));
    }
    let files = [
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Modules/Build.lua",
        "src/Modules/Common.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/ModParser.lua",
        "src/Classes/ModStore.lua",
        "src/Modules/CalcDefence.lua",
        "src/HeadlessWrapper.lua",
        "src/Data/Bases/helmet.lua",
        "src/Data/Bases/boots.lua",
    ];
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(observer::LOCAL_DEFENCE_OBSERVER.as_bytes()),"files":files.map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})),
        "native_owner_closure":false,"preoverride_and_slot_consumption_are_separate":true,"cases":cases});
    let bytes = serde_json::to_vec_pretty(&report).unwrap();
    assert!(bytes.len() < 32 * 1024 * 1024, "bounded source report");
    let suffix = if enabled { "on" } else { "off" };
    fs::write(out.join(format!("source-jit-{suffix}.raw.json")), &bytes).unwrap();
    local_defence_check(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    assert_eq!(fs::read_to_string(dir.join("build-05.xml")).unwrap(), xml);
}

fn local_defence_observe(
    root: &Path,
    xml: &str,
    warm: Option<&str>,
    enabled: bool,
    instrumented: bool,
    name: &str,
) -> Json {
    eprintln!("Armour local defence {name}, instrumented={instrumented}, JIT={enabled}");
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("armourDefenceJit", enabled)?;
        lua.globals()
            .set("armourDefenceInstrumented", instrumented)?;
        lua.globals().set("armourDefenceXml", xml)?;
        lua.load("if armourDefenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<mlua::Function, RuntimeError> {
        let api: mlua::Table = lua
            .load(observer::LOCAL_DEFENCE_OBSERVER)
            .set_name("@armour-local-defence-observer")
            .eval()?;
        let install: mlua::Function = api.get("install")?;
        let cleanup: mlua::Function = install.call(())?;
        lua.globals().set("armourLocalDefenceApi", api)?;
        Ok(cleanup)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("armourLocalDefenceApi")?;
        let observe: mlua::Function = api.get("observe")?;
        let rebuild: mlua::Function = api.get("rebuild")?;
        let mut states = serde_json::Map::new();
        for (i, stage) in LOCAL_DEFENCE_STAGES.iter().enumerate() {
            if i > 0 {
                rebuild.call::<()>(())?;
            }
            let a: Value = observe.call(())?;
            let a: Json = lua.from_value(a)?;
            let b: Value = observe.call(())?;
            let b: Json = lua.from_value(b)?;
            assert_eq!(a, b, "observer mutation {name} {stage}");
            states.insert((*stage).into(), a);
        }
        let probes: mlua::Function = api.get("constructor_probes")?;
        let constructor: Value = probes.call(())?;
        let constructor: Json = lua.from_value(constructor)?;
        let retained: Value = observe.call(())?;
        let retained: Json = lua.from_value(retained)?;
        assert_eq!(
            retained, states["rebuild-two"],
            "constructor touched selected build"
        );
        Ok(json!({"states":states,"constructors":constructor}))
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
    for (id, ordinal) in [("21", 576), ("22", 578)] {
        let matching: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Item"
                    && r.attribute("id").and_then(|a| a.decoded().ok()) == Some(id)
            })
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].occurrence().id().ordinal(), ordinal);
    }
    json!({"source_hash":result["source_hash"],"xml_sha256":digest(xml.as_bytes()),"warm_xml_sha256":warm.map(|s|digest(s.as_bytes())),"source_identity":evidence.identity(),"source_bindings":[{"item_id":21,"ordinal":576},{"item_id":22,"ordinal":578}],"independent_source_bindings_verified":true,"instrumented":instrumented,"states":result["additional_observation"]["states"],"constructors":result["additional_observation"]["constructors"]})
}
fn local_defence_without_trace(value: &Json) -> Json {
    let mut value = value.clone();
    value.as_object_mut().unwrap().remove("trace");
    value
}
fn local_defence_check(report: &Json) {
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    assert_eq!(
        report["observer_sha256"],
        digest(observer::LOCAL_DEFENCE_OBSERVER.as_bytes())
    );
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 6);
    let absent = json!({"kind":"absent"});
    for case in cases {
        let nonzero = case["synthetic_local_control"] == true;
        for stage in LOCAL_DEFENCE_STAGES {
            eprintln!(
                "Check armour local defence case {} stage {stage}",
                case["name"]
            );
            let state = &case["observed"]["states"][stage];
            assert_eq!(state["original_functions_preserved"], true);
            assert_eq!(state["method_wrappers"], false);
            for (key, id) in [("items", 2), ("spec", 3), ("skills", 4), ("config", 1)] {
                assert_eq!(state["selected"][key], id);
            }
            let items = rows(&state["items"]);
            assert_eq!(items.len(), 2);
            for (id, name, raw_armour, raw_es) in [
                (21, "Iron Crown", 26, 13),
                (22, "Cryptic Leggings", 134, 37),
            ] {
                let item = items.iter().find(|i| i["id"] == id).unwrap();
                assert_eq!(item["base_name"], name);
                assert_eq!(item["quality"], case["raw_quality"]);
                assert_eq!(item["crafted_quality"], 0);
                assert_eq!(item["exact_registered"], true);
                assert_eq!(item["exact_catalogue_base"], true);
                assert_eq!(item["base"]["armour"]["Armour"], raw_armour);
                assert_eq!(item["base"]["armour"]["EnergyShield"], raw_es);
                for k in ["EvasionPerLevel", "EnergyShieldPerLevel", "WardPerLevel"] {
                    assert_eq!(item["armour"][k], 0);
                }
                for cat in ["buff", "implicit", "enchant", "rune", "classRequirement"] {
                    assert!(rows(&item["members"][cat]).is_empty());
                }
                assert_eq!(
                    rows(&item["members"]["explicit"]).len(),
                    if nonzero { 12 } else { 1 }
                );
                for member in rows(&item["members"]["explicit"]) {
                    assert_eq!(member["unparsed"], absent);
                }
            }
            let trace = &state["trace"];
            for category in [
                "locals",
                "assemblies",
                "overrides",
                "normalisations",
                "rounds",
            ] {
                for row in rows(&trace[category]) {
                    check_local_defence_item_binding(row);
                }
            }
            for consumer in rows(&trace["consumers"]) {
                for read in rows(&consumer["reads"]) {
                    check_local_defence_item_binding(read);
                }
            }
            for mode in ["MAIN", "CALCS"] {
                let current: Vec<_> = rows(&trace["consumers"])
                    .iter()
                    .filter(|r| r["mode"] == mode && r["exact_current_environment"] == true)
                    .collect();
                assert_eq!(current.len(), 1, "exact current Player {mode}");
                let consumer = current[0];
                assert_eq!(consumer["original_defence_return"], true);
                assert_eq!(rows(&consumer["reads"]).len(), 4);
                for id in [21, 22] {
                    let item = items.iter().find(|i| i["id"] == id).unwrap();
                    let output = rows(&consumer["outputs"])
                        .iter()
                        .find(|o| o["item_id"] == id)
                        .unwrap();
                    for field in ["Armour", "EnergyShield"] {
                        let read: Vec<_> = rows(&consumer["reads"])
                            .iter()
                            .filter(|r| r["item_id"] == id && r["name"] == field)
                            .collect();
                        assert_eq!(read.len(), 1);
                        let read = read[0];
                        for flag in [
                            "exact_selected_item",
                            "exact_current_item",
                            "original_call",
                            "original_return",
                            "exact_return_to_consumer",
                        ] {
                            assert_eq!(read[flag], true);
                        }
                        assert_eq!(read["stored_armour"][field], item["armour"][field]);
                        assert_eq!(read["returned_value"], item["armour"][field]);
                        assert_eq!(output[field], read["returned_value"]);
                    }
                }
            }
            for assembly in rows(&trace["assemblies"])
                .iter()
                .filter(|r| r["exact_current_item"] == true)
            {
                check_local_defence_assembly(
                    assembly,
                    nonzero,
                    case["raw_quality"].as_i64().unwrap(),
                );
                let item = items
                    .iter()
                    .find(|i| i["id"] == assembly["item_id"])
                    .unwrap();
                for field in ["Armour", "EnergyShield"] {
                    assert_eq!(
                        assembly["preoverride"][field], item["armour"][field],
                        "current item preoverride to stored/getter binding: case {} stage {stage} item {} {field}",
                        case["name"], assembly["item_id"]
                    );
                }
            }
            for overwrite in rows(&trace["overrides"]) {
                assert_eq!(overwrite["original_return"], true);
                assert_eq!(overwrite["exact_local_store"], true);
                assert!(rows(&overwrite["result"]).is_empty());
            }
            if stage == "fresh" {
                for id in [21, 22] {
                    assert!(
                        rows(&trace["assemblies"])
                            .iter()
                            .any(|r| r["item_id"] == id && r["exact_current_item"] == true),
                        "current assembly: case {} stage {stage} item {id}",
                        case["name"]
                    );
                    assert!(
                        rows(&trace["overrides"])
                            .iter()
                            .any(|r| r["item_id"] == id && r["exact_current_item"] == true),
                        "current override lookup: case {} stage {stage} item {id}",
                        case["name"]
                    );
                    for line in [2547, 2551] {
                        assert!(
                            rows(&trace["rounds"]).iter().any(|r| r["item_id"] == id
                                && r["exact_current_item"] == true
                                && r["call_line"] == line
                                && r["original_call"] == true),
                            "current rounding call: case {} stage {stage} item {id} line {line}",
                            case["name"]
                        );
                    }
                }
                check_local_defence_calls(trace, nonzero);
            }
        }
        let constructors = &case["observed"]["constructors"];
        assert_eq!(constructors["selected_items_preserved"], true);
        assert_eq!(rows(&constructors["items"]).len(), 2);
        for category in [
            "locals",
            "assemblies",
            "overrides",
            "normalisations",
            "rounds",
        ] {
            for row in rows(&constructors["trace"][category]) {
                check_local_defence_item_binding(row);
                assert_eq!(
                    row["exact_current_item"], false,
                    "independent constructor object"
                );
            }
        }
        for probe in rows(&constructors["items"]) {
            assert_eq!(probe["raw_quality"], case["raw_quality"]);
            assert_eq!(probe["direct"]["quality"], case["raw_quality"]);
            assert_eq!(probe["explicit_constructor_controls"], true);
            assert_eq!(probe["direct"]["exact_registered"], false);
            assert_eq!(probe["explicit_normalise"]["exact_registered"], false);
            let expected = if case["raw_quality"] == 0 {
                probe["default_quality"].clone()
            } else {
                case["raw_quality"].clone()
            };
            assert_eq!(probe["explicit_normalise"]["quality"], expected);
            let loaded = rows(&case["observed"]["states"]["rebuild-two"]["items"])
                .iter()
                .find(|i| i["id"] == probe["item_id"])
                .unwrap();
            assert_eq!(probe["direct"]["armour"], loaded["armour"]);
        }
        assert_eq!(
            local_defence_without_trace(&case["observed"]["states"]["rebuild-one"]),
            local_defence_without_trace(&case["observed"]["states"]["rebuild-two"]),
            "fixed rebuild determinism"
        );
    }
    for index in [1, 2] {
        for stage in LOCAL_DEFENCE_STAGES {
            assert_eq!(
                local_defence_without_trace(&cases[0]["observed"]["states"][stage]),
                local_defence_without_trace(&cases[index]["observed"]["states"][stage]),
                "repeat/restoration"
            );
        }
    }
}
fn check_local_defence_item_binding(row: &Json) {
    for field in [
        "exact_registered_item",
        "exact_main_item",
        "exact_calcs_item",
        "exact_current_item",
    ] {
        assert!(
            row[field].is_boolean(),
            "missing actual object binding {field}"
        );
    }
    assert_eq!(
        row["exact_current_item"],
        row["exact_registered_item"] == true
            && row["exact_main_item"] == true
            && row["exact_calcs_item"] == true
    );
}
fn check_local_defence_assembly(row: &Json, nonzero: bool, quality: i64) {
    assert_eq!(row["original_assignment_observed"], true);
    let v = &row["operands"];
    assert_eq!(v["qualityScalar"], quality);
    assert_eq!(v["craftedQuality"], 0);
    for k in ["evasionPerLevel", "energyShieldPerLevel", "wardPerLevel"] {
        assert_eq!(v[k], 0);
    }
    for (k, n) in [
        ("armourEvasionBase", 13),
        ("armourEnergyShieldBase", 11),
        ("evasionEnergyShieldBase", 17),
        ("armourInc", 19),
        ("energyShieldInc", 23),
        ("armourEnergyShieldInc", 29),
        ("armourEvasionInc", 31),
        ("evasionEnergyShieldInc", 37),
        ("defencesInc", 41),
    ] {
        assert_eq!(v[k], if nonzero { n } else { 0 });
    }
}
fn check_local_defence_calls(trace: &Json, nonzero: bool) {
    let expected = [
        ("Quality", "BASE", 0),
        ("Armour", "BASE", 5),
        ("ArmourAndEvasion", "BASE", 13),
        ("Evasion", "BASE", 0),
        ("EvasionAndEnergyShield", "BASE", 17),
        ("EnergyShield", "BASE", 7),
        ("ArmourAndEnergyShield", "BASE", 11),
        ("Ward", "BASE", 0),
        ("EvasionPerLevel", "BASE", 0),
        ("EnergyShieldPerLevel", "BASE", 0),
        ("WardPerLevel", "BASE", 0),
        ("Armour", "INC", 19),
        ("ArmourAndEvasion", "INC", 31),
        ("Evasion", "INC", 0),
        ("EvasionAndEnergyShield", "INC", 37),
        ("EnergyShield", "INC", 23),
        ("Ward", "INC", 0),
        ("ArmourAndEnergyShield", "INC", 29),
        ("Defences", "INC", 41),
        ("AlternateQualityArmour", "BASE", 0),
    ];
    for id in [21, 22] {
        let calls: Vec<_> = rows(&trace["locals"])
            .iter()
            .filter(|r| r["item_id"] == id && r["exact_current_item"] == true)
            .collect();
        assert!(!calls.is_empty(), "current item {id} original local calls");
        assert_eq!(
            calls.len() % expected.len(),
            0,
            "complete original local call batches"
        );
        for chunk in calls.chunks(expected.len()) {
            for (call, (name, kind, amount)) in chunk.iter().zip(expected) {
                assert_eq!(call["name"], name);
                assert_eq!(call["type"], kind);
                assert_eq!(call["flags"], 0);
                assert_eq!(call["original_call"], true);
                assert_eq!(call["original_return"], true);
                assert_eq!(call["result"], if nonzero { amount } else { 0 });
                if nonzero && amount != 0 {
                    assert_eq!(
                        rows(&call["before"]).len(),
                        rows(&call["after"]).len() + 1,
                        "one actual local record consumed"
                    );
                } else {
                    assert_eq!(call["before"], call["after"], "empty local group");
                }
            }
        }
    }
}
