//! The saved Solar Amulet's full source lifecycle; no native coverage authority.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/ranged_spirit_source.rs"]
mod observer;
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
const TEST: &str = "solar_spirit_keeps_source_range_rounding_and_item_members_separate";
const CHILD: &str = "POE_RANGED_SPIRIT_CHILD";

#[test]
fn solar_spirit_keeps_source_range_rounding_and_item_members_separate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-ranged-spirit-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let manifest = read(&fixtures.join("index.json"));
        let entry = rows(&manifest["builds"])
            .iter()
            .find(|row| row["xml"] == "build-05.xml")
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([61; 16]),
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
        let zero = legacy_range(&xml, "0");
        let one = legacy_range(&xml, "1");
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text) in [
            ("original", &xml),
            ("legacy-zero", &zero),
            ("legacy-one", &one),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("spiritXml", text.as_str())?;
                lua.globals().set("spiritControls", name == "original")?;
                lua.globals().set("spiritJit", enabled)?;
                lua.load("if spiritJit then jit.on() else jit.off();jit.flush() end")
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
            "manifest_sha256":pinned::manifest_sha256(),"native_parity":false,"whole_contributor_coverage":false,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Classes/ModStore.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Modules/CalcSetup.lua","src/Data/ModScalability.lua","src/Data/Bases/amulet.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
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
fn legacy_range(xml: &str, value: &str) -> String {
    let start = xml.find("<Item id=\"23\">").unwrap();
    let end = start + xml[start..].find("</Item>").unwrap();
    let body = &xml[start..end];
    let needle = "<ModRange range=\"0.5\" id=\"1\"/>";
    assert_eq!(body.matches(needle).count(), 1);
    format!(
        "{}{}{}",
        &xml[..start],
        body.replace(needle, &format!("<ModRange range=\"{value}\" id=\"1\"/>")),
        &xml[end..]
    )
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "not a source list: {value}"
        );
        &[]
    }
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(observer::OBSERVE)
        .set_name("@ranged-spirit-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn spirit(snapshot: &Json) -> Vec<&Json> {
    rows(&snapshot["active"])
        .iter()
        .filter(|m| m["name"] == "Spirit" && m["type"] == "BASE")
        .collect()
}
fn check(result: &Json) {
    assert_eq!(rows(&result["cases"]).len(), 3);
    for (case, expected) in rows(&result["cases"]).iter().zip([13, 10, 15]) {
        let state = &case["state"];
        for key in [
            "saved_item_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[key], true);
        }
        assert_eq!(state["selected"]["items"], 2);
        assert_eq!(state["selected"]["spec"], 3);
        assert_eq!(state["selected"]["skills"], 4);
        assert_eq!(state["selected"]["config"], 1);
        assert_eq!(state["selected_slots"], json!(["Amulet"]));
        let loaded = &state["loaded"];
        assert_eq!(loaded["base"], "Solar Amulet");
        assert_eq!(loaded["field_types"]["item_level"], "nil");
        for category in ["buff", "enchant", "rune", "classRequirement"] {
            assert!(rows(&loaded["lists"][category]).is_empty());
        }
        assert_eq!(rows(&loaded["lists"]["implicit"]).len(), 1);
        assert_eq!(rows(&loaded["lists"]["explicit"]).len(), 1);
        assert_eq!(loaded["lists"]["implicit"][0]["line"], "+(10-15) to Spirit");
        assert_eq!(
            loaded["lists"]["explicit"][0]["line"],
            "+1 to Level of all Minion Skills"
        );
        for category in ["implicit", "explicit"] {
            let line = &loaded["lists"][category][0];
            assert_eq!(line["field_types"]["extra"], "nil");
            assert!(rows(&line["modTags"]).is_empty());
            assert_eq!(line["catalyst_factor"], 1);
        }
        let records = spirit(loaded);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["value"], expected);
        assert_eq!(records[0]["source"], "Item:23:New Item, Solar Amulet");
        assert_eq!(records[0]["source_slot"], "Amulet");
        assert_eq!(records[0]["flags"], 0);
        assert_eq!(records[0]["keyword_flags"], 0);
        assert!(rows(&records[0]["tags"]).is_empty());
        let player: Vec<_> = rows(&state["player_spirit"])
            .iter()
            .filter(|m| m["source"] == "Item:23:New Item, Solar Amulet")
            .collect();
        assert_eq!(player.len(), 1);
        assert_eq!(player[0]["value"], expected);
        assert_eq!(state["base_facts"]["implicit"], "+(10-15) to Spirit");
        assert!(rows(&state["base_facts"]["implicit_mod_types"][0]).is_empty());
        assert_eq!(state["scalability"][0]["isScalable"], true);
    }
    let state = &result["cases"][0]["state"];
    assert_eq!(rows(&state["probes"]).len(), 20);
    for (name, value) in [
        ("range-zero", 10),
        ("range-low", 11),
        ("range-mid", 13),
        ("range-high", 15),
        ("range-one", 15),
        ("untagged-neural", 13),
        ("tagged-neural", 15),
        ("tagged-wrong-catalyst", 13),
        ("implicit-magnitude", 19),
        ("explicit-magnitude", 13),
        ("corrupted-range", 20),
    ] {
        let p = rows(&state["probes"])
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let mods = spirit(&p["after"]);
        assert_eq!(mods.len(), 1, "{name}");
        assert_eq!(mods[0]["value"], value, "{name}");
    }
    let absent = rows(&state["probes"])
        .iter()
        .find(|p| p["name"] == "range-absent")
        .unwrap();
    assert_eq!(
        absent["after"]["lists"]["implicit"][0]["range"],
        state["default_range"]
    );
    let probe = |name: &str| {
        rows(&state["probes"])
            .iter()
            .find(|p| p["name"] == name)
            .unwrap()
    };
    let zero = probe("zero-range");
    assert!(spirit(&zero["after"]).is_empty());
    assert_eq!(
        zero["after"]["lists"]["implicit"][0]["field_types"]["extra"],
        "string"
    );
    let disabled = probe("disabled");
    assert!(spirit(&disabled["after"]).is_empty());
    assert_eq!(disabled["after"]["lists"]["implicit"][0]["disabled"], true);
    let unknown = probe("unknown-before");
    assert_eq!(
        unknown["after"]["lists"]["implicit"][0]["field_types"]["extra"],
        "string"
    );
    // A prior unknown member consumes the source implicit boundary. Finding
    // Spirit later does not authorize the importer to guess its category.
    assert_eq!(
        unknown["after"]["lists"]["explicit"][0]["line"],
        "+(10-15) to Spirit"
    );
    let tagged = probe("tagged-neural");
    assert_eq!(
        tagged["after"]["lists"]["implicit"][0]["modTags"],
        json!(["mana"])
    );
    assert_eq!(
        tagged["after"]["lists"]["implicit"][0]["catalyst_factor"],
        1.2
    );
    let enchant = probe("enchant");
    assert!(rows(&enchant["after"]["lists"]["implicit"]).is_empty());
    assert_eq!(
        enchant["after"]["lists"]["enchant"][0]["line"],
        "+(10-15) to Spirit"
    );
    assert_eq!(
        state["reused_item_level"],
        json!({"field_type":"number","value":77})
    );
    assert_eq!(state["fresh_item_level"], json!({"field_type":"nil"}));
    let present = rows(&state["probes"])
        .iter()
        .find(|p| p["name"] == "item-level-present")
        .unwrap();
    assert_eq!(present["after"]["item_level"], 77);
    assert_eq!(rows(&state["copy_probes"]).len(), 4);
    for (p, value) in rows(&state["copy_probes"]).iter().zip([0, 3, 6, 13]) {
        assert_eq!(rows(&p["records"]).len(), 1);
        assert_eq!(p["records"][0]["value"], value);
    }
}
