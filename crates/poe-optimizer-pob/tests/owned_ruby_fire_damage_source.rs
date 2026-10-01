//! Complete original source evidence for the selected Ruby and fixed FireDamage INC.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Value};
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
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "selected_ruby_preserves_quality_absence_and_fire_damage_source_lifecycle";
const CHILD: &str = "POE_RUBY_FIRE_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_ruby_fire_damage_source.lua");
const PINNED_FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/CalcSetup.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Data/ModScalability.lua",
    "src/Data/Bases/jewel.lua",
    "runtime/lua/xml.lua",
];
#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    target: bool,
    controls: bool,
}
#[test]
fn selected_ruby_preserves_quality_absence_and_fire_damage_source_lifecycle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-ruby-fire-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
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
                assert!(
                    status.success(),
                    "source child failed {}; evidence {}\n{}",
                    path.display(),
                    out.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "source deadline {}; evidence {}\n{}",
                    path.display(),
                    out.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    same(
        &read(&out.join("source-jit-off.json")),
        &read(&out.join("source-jit-on.json")),
        "exact scoped JIT evidence",
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index = read(&fixtures.join("index.json"));
    let originals: Vec<_> = (1..=5)
        .map(|n| {
            let filename = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&filename)).unwrap();
            let row = rows(&index["builds"])
                .iter()
                .find(|r| r["xml"] == filename)
                .unwrap();
            assert_eq!(row["xml_sha256"], digest(xml.as_bytes()));
            Case {
                name: format!("original-{n:02}"),
                xml,
                target: n == 4,
                controls: n == 4,
            }
        })
        .collect();
    let original = &originals[3].xml;
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(original.as_bytes()).unwrap(),
        BuildLineage::from_bytes([79; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let occurrences: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().name() == "Item"
                && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("1")
        })
        .collect();
    assert_eq!(occurrences.len(), 1);
    assert_eq!(occurrences[0].occurrence().id().ordinal(), 171);
    let mut inputs = originals.clone();
    for (name, header) in [
        ("quality-zero", "Quality: 0"),
        ("quality-twenty", "Quality: 20"),
        ("quality-malformed", "Quality: Nope"),
        ("quality-duplicate", "Quality: 0\nQuality: 20"),
        ("quality-alias", "Quality (Fire Modifiers): 20%"),
        ("catalyst-quality-only", "CatalystQuality: 20"),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: edit_item(original, |s| {
                s.replace("Item Level: 55", &format!("{header}\nItem Level: 55"))
            }),
            target: true,
            controls: false,
        });
    }
    inputs.push(Case {
        name: "unknown-member".into(),
        xml: edit_item(original, |s| {
            s.replace(
                "14% increased Fire Damage",
                "14% increased Fire Damage\nOwned unknown Ruby member",
            )
        }),
        target: true,
        controls: false,
    });
    assert_eq!(inputs.len(), 12);
    let mut cases = Vec::new();
    for case in &inputs {
        eprintln!("complete Ruby source case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("rubySourceXml", case.xml.as_str())?;
            lua.globals().set("rubySourceTarget", case.target)?;
            lua.globals().set("rubySourceControls", case.controls)?;
            lua.globals().set("rubySourceJit", enabled)?;
            lua.load("if rubySourceJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("rubySourcePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@ruby-fire-source-authentication")
                .eval::<Function>()?)
        };
        let temp = tempfile::tempdir().unwrap();
        let value = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &case.xml,
            None,
            !case.name.starts_with("original-"),
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let row = match value {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"expected_selections":selections(&case.xml),"available":true,"state":value["additional_observation"]})
            }
            Err(error) => {
                json!({"name":case.name,"available":false,"source_error":error.to_string()})
            }
        };
        cases.push(row);
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
    for (n, original) in originals.iter().enumerate() {
        assert_eq!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", n + 1))).unwrap(),
            original.xml
        );
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"evidence":{
        "complete_load_attempts_per_jit":12,"full_controls":7,"isolated_item_controls":29,"source_ordinal":171,"item_id":1,"selected_jewel_node":46882,
        "native_effect_coverage":false,"whole_build_parity":false,"business_method_wrappers":false,"legacy_range_only_excluded_from_fresh_comparison":true,
        "observer_sha256":digest(OBSERVE.as_bytes()),"originals":originals.iter().map(|c|json!({"name":c.name,"sha256":digest(c.xml.as_bytes())})).collect::<Vec<_>>(),
        "files":PINNED_FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>()},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result);
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("rubySourcePhase", "after")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@ruby-fire-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 12);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(case["available"], true, "{name}: {}", case["source_error"]);
        let state = &case["state"];
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "calcs_output_preserved",
            "original_functions_preserved",
            "fresh_loaded_items",
        ] {
            assert_eq!(state[flag], true, "{name} {flag}");
        }
        same(&state["selected"], &case["expected_selections"], name);
        if name.starts_with("original-") && name != "original-04" {
            assert!(rows(&state["items"]).is_empty());
            continue;
        }
        assert_eq!(state["loaded_fresh_equivalent_except_legacy_range"], true);
        let item = &rows(&state["items"])[0];
        assert_eq!(item["id"], 1);
        assert_eq!(item["node"], 46882);
        assert_eq!(item["selected_slots"], json!(["Jewel 46882"]));
        assert_eq!(item["base_facts"]["type"], "Jewel");
        assert_eq!(item["base_facts"]["quality_type"], "nil");
        assert!(rows(&item["base_facts"]["implicit_mod_types"]).is_empty());
        assert!(
            item["base_facts"]["requirements"]
                .as_object()
                .is_some_and(|m| m.is_empty())
        );
        assert_eq!(item["loaded"]["itemLevel"], 55);
        for mode in ["main", "calcs"] {
            let delivered = rows(&item["delivery"][mode]);
            assert_eq!(delivered.len(), 1, "{name} {mode}");
            check_fire(&delivered[0], 14.0, &format!("{name} {mode} delivery"));
            assert_eq!(delivered[0]["source"], "Item:1:Viper Wound, Ruby");
        }
    }
    let state = &cases[3]["state"];
    let item = &state["items"][0];
    let loaded = &item["loaded"];
    for category in ["buff", "enchant", "rune", "classRequirement", "implicit"] {
        assert!(rows(&loaded["lists"][category]).is_empty());
    }
    assert_eq!(rows(&loaded["lists"]["explicit"]).len(), 1);
    let line = &loaded["lists"]["explicit"][0];
    assert_eq!(line["line"], "14% increased Fire Damage");
    assert_eq!(line["ordinal"], 1);
    assert_eq!(line["field_types"]["extra"], "nil");
    assert!(rows(&line["modTags"]).is_empty());
    assert_eq!(line["valueScalar"], 1);
    assert_eq!(line["catalyst_factor"], 1);
    assert_eq!(line["formatted"], "14% increased Fire Damage");
    for field in [
        "quality",
        "catalyst",
        "catalystQuality",
        "corrupted",
        "doubleCorrupted",
    ] {
        assert_eq!(loaded["field_types"][field], "nil", "original {field}");
    }
    for property in [
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
        "crafted",
        "mutated",
    ] {
        assert_ne!(line[property], true, "original {property}");
    }
    assert_eq!(rows(&loaded["active"]).len(), 1);
    check_fire(&loaded["active"][0], 14.0, "original active");
    assert_eq!(rows(&line["records"]).len(), 1);
    check_fire(&line["records"][0], 14.0, "original physical line");
    assert_eq!(rows(&item["probes"]).len(), 29);
    assert_eq!(rows(&state["numeric_cases"]).len(), 29);
    for (name, expected) in [
        ("amount-zero", 0.0),
        ("amount-one", 1.0),
        ("amount-high", 1000000.0),
        ("untagged-catalyst-twenty", 14.0),
        ("tagged-fire-catalyst-twenty", 16.0),
        ("tagged-fire-wrong-catalyst", 14.0),
        ("tagged-fire-catalyst-zero", 14.0),
        ("explicit-magnitude", 14.0),
        ("crafted-explicit-magnitude", 21.0),
        ("crafted-implicit-magnitude-contrast", 14.0),
        ("crafted-implicit-category", 14.0),
        ("implicit-magnitude-contrast", 14.0),
        ("implicit-category", 14.0),
        ("enchant-category", 14.0),
        ("corrupted-range", 21.0),
        ("unscalable", 14.0),
    ] {
        let row = rows(&state["numeric_cases"])
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        let active = rows(&row["active"]);
        assert_eq!(active.len(), 1, "{name}");
        check_fire(&active[0], expected, name);
    }
    for name in ["disabled", "variant"] {
        let row = rows(&state["numeric_cases"])
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        assert!(
            rows(&row["active"]).is_empty(),
            "{name} must not reach active list"
        );
    }
    for (name, crafted, factor) in [
        ("explicit-magnitude", false, 1.0),
        ("crafted-explicit-magnitude", true, 1.5),
        ("crafted-implicit-category", true, 1.0),
    ] {
        let probe = rows(&item["probes"])
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        assert_eq!(
            probe["after"]["crafted"].as_bool().unwrap_or(false),
            crafted,
            "{name} construction lane"
        );
        assert_eq!(
            rows(&probe["after"]["modMagnitudeMods"]).len(),
            1,
            "{name} retained magnitude recipe"
        );
        let numeric = rows(&state["numeric_cases"])
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        assert_eq!(
            numeric["magnitude_factor"].as_f64(),
            Some(factor),
            "{name} effective factor"
        );
    }
    for (name, quality) in [
        ("quality-zero", 0),
        ("quality-twenty", 20),
        ("quality-duplicate", 20),
    ] {
        let row = cases.iter().find(|r| r["name"] == name).unwrap();
        assert_eq!(row["state"]["items"][0]["loaded"]["quality"], quality);
    }
    // Unknown/malformed syntax is retained for the stricter adapter to reject;
    // source permissiveness never authorizes canonical quality absence.
    let unknown = cases
        .iter()
        .find(|r| r["name"] == "unknown-member")
        .unwrap();
    assert!(
        rows(&unknown["state"]["items"][0]["loaded"]["lists"]["explicit"])
            .iter()
            .any(|r| r["extra"].is_string())
    );
}
fn check_fire(record: &Json, expected: f64, context: &str) {
    assert_eq!(record["name"], "FireDamage");
    assert_eq!(record["type"], "INC");
    assert_eq!(record["value"].as_f64(), Some(expected), "{context}");
    assert_eq!(record["flags"], 0);
    assert_eq!(record["keyword_flags"], 0);
    assert!(rows(&record["tags"]).is_empty());
}
fn selections(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut result = serde_json::Map::new();
    for (key, element, attribute) in [
        ("config", "Config", "activeConfigSet"),
        ("items", "Items", "activeItemSet"),
        ("skills", "Skills", "activeSkillSet"),
        ("spec", "Tree", "activeSpec"),
        ("group", "Build", "mainSocketGroup"),
    ] {
        let node = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name(element))
            .unwrap();
        result.insert(
            key.into(),
            json!(node.attribute(attribute).unwrap().parse::<usize>().unwrap()),
        );
    }
    Json::Object(result)
}
fn edit_item(xml: &str, edit: impl FnOnce(&str) -> String) -> String {
    let opening = "<Item id=\"1\">";
    assert_eq!(xml.matches(opening).count(), 1);
    let start = xml.find(opening).unwrap() + opening.len();
    let end = start + xml[start..].find("</Item>").unwrap();
    let mut result = xml.to_owned();
    result.replace_range(start..end, &edit(&xml[start..end]));
    result
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
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "not source list: {value}"
        );
        &[]
    }
}
fn tail(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.chars().take(600).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn same(a: &Json, b: &Json, context: &str) {
    fn first(a: &Json, b: &Json, path: String) -> String {
        if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
            for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
                if a.get(key) != b.get(key) {
                    return first(
                        a.get(key).unwrap_or(&Json::Null),
                        b.get(key).unwrap_or(&Json::Null),
                        format!("{path}.{key}"),
                    );
                }
            }
        } else if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
            if a.len() != b.len() {
                return format!("{path}: array lengths {} != {}", a.len(), b.len());
            }
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                if a != b {
                    return first(a, b, format!("{path}[{index}]"));
                }
            }
        }
        format!(
            "{path}: {} != {}",
            a.to_string().chars().take(180).collect::<String>(),
            b.to_string().chars().take(180).collect::<String>()
        )
    }
    assert!(
        a == b,
        "{context}: {}; full evidence retained",
        first(a, b, "$".into())
    );
}
