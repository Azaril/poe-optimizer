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
