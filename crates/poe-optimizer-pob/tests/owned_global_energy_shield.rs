//! Global Energy Shield input evidence from complete original loading and fresh
//! original Item construction. No final resource or whole-build parity claim.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/global_energy_shield_source.rs"]
mod observer;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "global_energy_shield_preserves_actual_scope_and_dormant_jewels";
const CHILD: &str = "POE_GLOBAL_ENERGY_SHIELD_SOURCE_CHILD";

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}

#[test]
#[ignore = "requires full pinned original04/original05 loading and a fresh evidence directory"]
fn global_energy_shield_preserves_actual_scope_and_dormant_jewels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_GLOBAL_ENERGY_SHIELD_OUTPUT").expect(
            "set POE_OPTIMIZER_GLOBAL_ENERGY_SHIELD_OUTPUT to a fresh absolute evidence directory",
        ),
    );
    assert!(out.is_absolute());
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let manifest = read(&fixtures.join("index.json"));
        let mut builds = Vec::new();
        let mut source_hash = None;
        for index in [4, 5] {
            let name = format!("build-{index:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&name)).unwrap();
            let entry = rows(&manifest["builds"])
                .iter()
                .find(|r| r["xml"] == name)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            let targets = if index == 4 {
                json!([{"item_id":19,"base":"Gold Amulet","line":"44% increased maximum Energy Shield","next_line":"+68 to maximum Life","player_participating":true}])
            } else {
                json!([
                    {"item_id":17,"base":"Sapphire","line":"20% increased maximum Energy Shield","next_line":"11% increased Critical Hit Chance","player_participating":false},
                    {"item_id":18,"base":"Sapphire","line":"16% increased maximum Energy Shield","next_line":"14% faster start of Energy Shield Recharge","player_participating":false}
                ])
            };
            let before = |lua: &Lua| {
                lua.globals().set("globalEsXml", xml.as_str())?;
                lua.globals()
                    .set("globalEsTargets", lua.to_value(&targets)?)?;
                lua.globals().set("globalEsJit", enabled)?;
                lua.load("if globalEsJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
                let value: Value = lua
                    .load(observer::OBSERVE)
                    .set_name("@global-energy-shield-observer")
                    .eval()?;
                Ok(lua.from_value(value)?)
            };
            let scratch = tempfile::tempdir().unwrap();
            let observed = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                scratch.path(),
                &xml,
                None,
                false,
                Some(&before),
                None,
                Some(&observe),
            )
            .unwrap();
            assert_eq!(observed["configuration_method_wrappers"], false);
            assert_eq!(observed["original_build_output_available"], true);
            if let Some(hash) = &source_hash {
                assert_eq!(hash, &observed["source_hash"]);
            } else {
                source_hash = Some(observed["source_hash"].clone());
            }
            builds.push(json!({"name":format!("original-{index:02}"),"xml_sha256":digest(xml.as_bytes()),"state":observed["additional_observation"]}));
            assert_eq!(fs::read_to_string(fixtures.join(name)).unwrap(), xml);
        }
        let result = json!({"source_hash":source_hash,"evidence":{
            "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(observer::OBSERVE.as_bytes()),
            "native_parity":false,"whole_contributor_coverage":false,"complete_item_inventory":false,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Classes/ModStore.lua","src/Modules/Common.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Modules/CalcSetup.lua","src/Data/ModScalability.lua","src/Data/Bases/amulet.lua","src/Data/Bases/jewel.lua","src/Data/Bases/gloves.lua"]
                .map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
        },"builds":builds});
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
    fs::create_dir(&out).expect("global Energy Shield evidence directory must be fresh");
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
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
                    "global Energy Shield source child failed: {}",
                    path.display()
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("global Energy Shield source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}

fn line<'a>(snapshot: &'a Json, text: &str) -> (&'a str, &'a Json) {
    let matches: Vec<_> = snapshot["lists"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(category, lines)| rows(lines).iter().map(move |row| (category.as_str(), row)))
        .filter(|(_, row)| row["line"] == text)
        .collect();
    assert_eq!(matches.len(), 1, "one exact physical source line: {text}");
    matches[0]
}
fn global_record(record: &Json, amount: f64) {
    assert_eq!(record["name"], "EnergyShield");
    assert_eq!(record["type"], "INC");
    assert_eq!(record["value"].as_f64(), Some(amount));
    assert_eq!(record["flags"], 0);
    assert_eq!(record["keyword_flags"], 0);
    assert_eq!(record["tags"], json!([{"type":"Global"}]));
}
fn one_global(records: &Json, amount: f64) {
    assert_eq!(rows(records).len(), 1);
    global_record(&rows(records)[0], amount);
}
fn global_line(snapshot: &Json, text: &str, amount: f64) {
    let (category, row) = line(snapshot, text);
    assert_eq!(category, "explicit");
    assert_eq!(row["field_types"]["extra"], "nil");
    one_global(&row["records"], amount);
    assert!(row.get("modTags").is_none_or(|v| rows(v).is_empty()));
    assert_eq!(row["catalyst_factor"].as_f64(), Some(1.));
}
fn check(result: &Json) {
    assert_eq!(rows(&result["builds"]).len(), 2);
    for (index, build) in rows(&result["builds"]).iter().enumerate() {
        let state = &build["state"];
        for key in [
            "observed_item_fields_preserved",
            "saved_selections_preserved",
            "main_scalar_output_preserved",
            "player_contributions_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[key], true);
        }
        assert_eq!(state["scalability"], json!([{"isScalable":true}]));
        let amounts: &[f64] = if index == 0 { &[44.] } else { &[20., 16.] };
        assert_eq!(rows(&state["cases"]).len(), amounts.len());
        if index == 1 {
            assert_eq!(state["selected"]["spec"], 3);
        }
        for (case, amount) in rows(&state["cases"]).iter().zip(amounts) {
            assert_eq!(case["base_facts"]["has_buff"], false);
            for field in ["loaded", "fresh_before", "fresh"] {
                global_line(&case[field], case["line"].as_str().unwrap(), *amount);
                let (_, successor) = line(&case[field], case["next_line"].as_str().unwrap());
                assert_eq!(successor["field_types"]["extra"], "nil");
                assert!(!rows(&successor["records"]).is_empty());
            }
            assert_eq!(case["loaded"], case["fresh"]);
            let direct: Vec<_> = rows(&case["fresh"]["active"])
                .iter()
                .filter(|m| m["type"] == "INC")
                .collect();
            assert_eq!(direct.len(), 1);
            global_record(direct[0], *amount);
            if case["player_participating"] == true {
                assert_eq!(case["selected_slots"], json!(["Amulet"]));
                assert!(
                    rows(&case["source_player_records"])
                        .iter()
                        .any(|m| m == direct[0])
                );
            } else {
                assert!(rows(&case["selected_slots"]).is_empty());
                assert!(rows(&case["source_player_records"]).is_empty());
            }
        }
        let probe = |name: &str| {
            rows(&state["constructors"])
                .iter()
                .find(|p| p["name"] == name)
                .unwrap()
        };
        for amount in [0, 1, 16, 20, 44, 1_000_000, 1_000_001] {
            let p = probe(&format!("integer-{amount}"));
            for field in ["before", "after"] {
                global_line(
                    &p[field],
                    &format!("{amount}% increased maximum Energy Shield"),
                    f64::from(amount),
                );
            }
            one_global(&p["after"]["active"], f64::from(amount));
        }
        for name in [
            "leading-zero",
            "untagged-carapace",
            "untagged-negative-quality",
        ] {
            let text = if name == "leading-zero" {
                "0000044% increased maximum Energy Shield"
            } else {
                "44% increased maximum Energy Shield"
            };
            for field in ["before", "after"] {
                global_line(&probe(name)[field], text, 44.);
            }
        }
        let (_, tagged) = line(
            &probe("tagged-carapace")["after"],
            "44% increased maximum Energy Shield",
        );
        assert_eq!(tagged["modTags"], json!(["energyshield"]));
        assert_eq!(tagged["catalyst_factor"].as_f64(), Some(1.2));
        one_global(&tagged["records"], 52.);
        one_global(&probe("decimal")["after"]["active"], 11.);
        one_global(&probe("corrupted-range")["after"]["active"], 17.);
        assert!(rows(&probe("disabled")["after"]["active"]).is_empty());
        for (name, amount) in [
            ("ordinary-magnitude", 25.),
            ("crafted-magnitude", 30.),
            ("crafted-cancellation-up-down", 30.),
            ("crafted-cancellation-down-up", 20.),
        ] {
            one_global(&probe(name)["after"]["active"], amount);
        }
        assert_eq!(
            probe("armour-local")["after"]["armour_data"]["EnergyShield"],
            30
        );
        assert!(rows(&probe("armour-local")["after"]["active"]).is_empty());
        assert_eq!(
            probe("armour-global")["after"]["armour_data"]["EnergyShield"],
            27
        );
        one_global(&probe("armour-global")["after"]["active"], 11.);
        for (entry, amount) in rows(&state["formats"])
            .iter()
            .zip([13., 5., 6., 7., 1007., 1006.])
        {
            assert_eq!(entry["extra_type"], "nil");
            one_global(&entry["records"], amount);
        }
        assert_eq!(rows(&state["formats"]).len(), 6);
        let direct = rows(&state["direct_parses"]);
        assert_eq!(direct.len(), 5);
        one_global(&direct[0]["records"], 44.);
        assert_eq!(rows(&direct[1]["records"]).len(), 1);
        assert!(rows(&direct[1]["records"][0]["tags"]).is_empty());
        assert!(rows(&direct[4]["records"]).is_empty() || direct[4]["extra_type"] != "nil");
        // Reduced/negative/ranged forms and values above the chosen owned bound
        // are source observations, not permission to broaden native admission.
        // Crafted cancellation is retained history evidence, not native semantics.
    }
}
