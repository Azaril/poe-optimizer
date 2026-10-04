//! Independent source intrinsic-stat assembly, before native damage or final-input authority.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
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
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_ice_nova_intrinsic_stats_use_original_assembler";
const CHILD: &str = "POE_ICE_NOVA_INTRINSIC_SOURCE_CHILD";
const ICE: &str = "Metadata/Items/Gems/SkillGemIceNova";
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const FILES: [&str; 14] = [
    "src/HeadlessWrapper.lua",
    "src/Modules/Build.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/SkillsTab.lua",
    "src/Modules/Data.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcPerform.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Gems.lua",
    "src/Data/SkillStatMap.lua",
];

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for intrinsic Ice Nova source stats"]
fn complete_ice_nova_intrinsic_stats_use_original_assembler() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-ice-nova-intrinsic-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
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
                    "source failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Ice intrinsic JIT parity",
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    for (name, level, quality, main, calcs, duplicate) in [
        ("main-two-calcs-one", None, None, 2, 1, false),
        ("main-one-calcs-two", None, None, 1, 2, false),
        ("raw-level-one", Some("1"), None, 1, 2, false),
        ("raw-level-forty", Some("40"), None, 2, 1, false),
        ("raw-quality-fractional", None, Some("20.5"), 2, 1, false),
        ("independent-copy-level-one", Some("1"), None, 2, 1, true),
    ] {
        let changed = edit(original, level, quality, main, calcs, duplicate);
        assert_ne!(changed, original);
        cases.push(observe(root, name, &changed, enabled));
    }
    cases.push(observe(root, "repeat-original-05", original, enabled));
    let result = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4", "manifest_sha256":pinned::manifest_sha256(),
        "files":FILES.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"native_build_parity":false,
        "native_inventory_authority":false,"final_input_authority":false,"canonical_parity_lifecycle_selected":false,
        "admitted_helper_domain":{"minimum_level":1,"maximum_level":40,"stat_sets":2,"quality":0,"alt_quality":false},
        "lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&result).unwrap();
    assert!(
        bytes.len() <= 32 * 1024 * 1024,
        "intrinsic evidence is {} bytes",
        bytes.len()
    );
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check(&result);
    for (i, b) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            b
        );
    }
}

fn observe(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    eprintln!(
        "Ice intrinsic case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("iceOccurrenceXml", xml)?;
        lua.globals().set("iceOccurrenceJit", enabled)?;
        lua.load("if iceOccurrenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        Ok(lua
            .load(include_str!("support/djinn_provider_source.lua"))
            .set_name("@ice-intrinsic-original-lifecycle-authentication")
            .eval()?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_once = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_twice = stage(lua)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
    };
    let temp = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        temp.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&install),
        Some(&observer),
    )
    .unwrap_or_else(|e| panic!("{name}: complete source failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    let states = &observed["additional_observation"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([97; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let sources: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().name() == "Gem"
                && row.attribute("gemId").and_then(|a| a.decoded().ok()) == Some(ICE)
        })
        .collect();
    for state in STAGES {
        let saved = rows(&states[state]["occurrences"]["saved"]);
        assert_eq!(sources.len(), saved.len());
        for source in &sources {
            let row = saved
                .iter()
                .find(|r| r["source_ordinal"] == source.occurrence().id().ordinal())
                .unwrap();
            assert_eq!(
                row["attributes"].as_object().unwrap().len(),
                source.attributes().len()
            );
            for attribute in source.attributes() {
                assert_eq!(
                    row["attributes"][&attribute.origin().name],
                    attribute.decoded().unwrap()
                );
            }
        }
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"states":states})
}

fn occurrence(lua: &Lua) -> Result<Json, RuntimeError> {
    let v: Value = lua
        .load(include_str!("support/ice_nova_occurrence_source.lua"))
        .set_name("@ice-intrinsic-existing-occurrence-observer")
        .eval()?;
    Ok(lua.from_value(v)?)
}
fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let before = occurrence(lua)?;
    let v: Value = lua
        .load(include_str!("support/ice_nova_intrinsic_stats.lua"))
        .set_name("@ice-intrinsic-original-helper-replay")
        .eval()?;
    let intrinsic: Json = lua.from_value(v)?;
    let after = occurrence(lua)?;
    assert_eq!(
        json_evidence::first_difference(&before, &after, "occurrences"),
        None,
        "helper probes must preserve complete observed state"
    );
    Ok(json!({"occurrences":before,"intrinsic":intrinsic}))
}
fn rebuild(lua: &Lua) -> Result<(), RuntimeError> {
    lua.load(
        r#"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line);return f
end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
local output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486)
assert(output==djinnOriginals.refs.calcs_tab_output and build.buildFlag==false)
local revision=build.outputRevision;local main,calcs=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(build.buildFlag==false and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calcs)
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
"#,
    )
    .set_name("@ice-intrinsic-original-frame-rebuild")
    .exec()?;
    Ok(())
}

fn edit(
    xml: &str,
    level: Option<&str>,
    quality: Option<&str>,
    main: u8,
    calcs: u8,
    duplicate: bool,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    let gem = skills
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem")
                && n.attribute("gemId") == Some(ICE)
                && n.ancestors()
                    .any(|p| p.has_tag_name("SkillSet") && p.attribute("id") == Some(selected))
        })
        .unwrap();
    assert!(gem.children().all(|n| !n.is_element()));
    let mut replacement = String::from("<Gem");
    for a in gem.attributes() {
        let value = match a.name() {
            "level" => level.unwrap_or(a.value()),
            "quality" => quality.unwrap_or(a.value()),
            _ => a.value(),
        };
        replacement.push_str(&format!(
            " {}=\"{}\"",
            a.name(),
            value
                .replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
        ));
    }
    replacement.push_str(&format!("><StatSetIndex grantedEffect=\"IceNovaPlayer\" index=\"{main}\"/><StatSetCalcsIndex grantedEffect=\"IceNovaPlayer\" index=\"{calcs}\"/></Gem>"));
    if duplicate {
        replacement = format!("{}{replacement}", &xml[gem.range()]);
    }
    let mut out = xml.to_owned();
    out.replace_range(gem.range(), &replacement);
    roxmltree::Document::parse(&out).unwrap();
    out
}

fn rows(v: &Json) -> &[Json] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn check(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 12);
    let original = &cases[4]["states"];
    assert_eq!(
        json_evidence::first_difference(original, &cases[11]["states"], "repeat"),
        None
    );
    let constructed = &original["fresh"]["intrinsic"]["constructed"];
    let vectors = &original["fresh"]["intrinsic"]["admitted"];
    for set in rows(&constructed["stat_sets"]) {
        assert_eq!(rows(&set["levels"]).len(), 40);
        for row in rows(&set["levels"]) {
            assert!(!rows(&row["row"]["interpolation"]).is_empty());
            assert!(
                rows(&row["row"]["interpolation"])
                    .iter()
                    .all(|entry| entry["value"] == 1)
            );
        }
    }
    for case in cases {
        for stage in STAGES {
            let state = &case["states"][stage];
            let intrinsic = &state["intrinsic"];
            assert_eq!(&intrinsic["constructed"], constructed);
            assert_eq!(&intrinsic["admitted"], vectors);
            assert_eq!(rows(&intrinsic["admitted"]).len(), 80);
            assert_eq!(rows(&intrinsic["boundaries"]).len(), 36);
            for flag in [
                "original_function_preserved",
                "constructed_inputs_preserved",
                "caller_inputs_preserved",
                "requested_jit_verified",
            ] {
                assert_eq!(intrinsic[flag], true);
            }
            for (position, probe) in rows(&intrinsic["admitted"]).iter().enumerate() {
                assert_eq!(probe["stat_set"], position / 40 + 1);
                assert_eq!(probe["instance"]["level"], position % 40 + 1);
                assert_eq!(probe["instance"]["quality"], 0);
                assert_eq!(probe["include_alt_quality"], false);
                assert_eq!(probe["success"], true);
                let index = probe["stat_set"].as_u64().unwrap();
                assert_eq!(
                    probe["stats"]["active_skill_base_area_of_effect_radius"],
                    if index == 1 { 32 } else { 48 }
                );
                assert!(
                    probe["stats"]["spell_minimum_base_cold_damage"]
                        .as_f64()
                        .unwrap()
                        > 0.
                );
                assert!(
                    probe["stats"]["spell_maximum_base_cold_damage"]
                        .as_f64()
                        .unwrap()
                        >= probe["stats"]["spell_minimum_base_cold_damage"]
                            .as_f64()
                            .unwrap()
                );
            }
            for actual in rows(&intrinsic["actual"]) {
                assert_eq!(actual["physical_source_exact"], true);
                assert_eq!(actual["constructed_table_exact"], true);
                for (source, target) in [
                    ("spell_minimum_base_cold_damage", "ColdMin"),
                    ("spell_maximum_base_cold_damage", "ColdMax"),
                    ("active_skill_base_area_of_effect_radius", "radius"),
                ] {
                    assert_eq!(
                        actual["stats"][source], actual["skill_data"][target],
                        "{} {stage} {source}",
                        case["name"]
                    );
                }
            }
            for probe in rows(&intrinsic["boundaries"]) {
                let label = probe["name"].as_str().unwrap();
                if label.starts_with("missing-quality") || label.starts_with("missing-stat-set") {
                    assert_eq!(probe["success"], false);
                    assert!(probe["error"].as_str().unwrap().contains("CalcTools.lua"));
                } else {
                    assert_eq!(probe["success"], true);
                }
            }
        }
    }
    for (name, main, calcs) in [
        ("main-two-calcs-one", 2, 1),
        ("main-one-calcs-two", 1, 2),
        ("raw-level-one", 1, 2),
        ("raw-level-forty", 2, 1),
        ("raw-quality-fractional", 2, 1),
    ] {
        let case = cases.iter().find(|c| c["name"] == name).unwrap();
        for stage in STAGES {
            let actual = rows(&case["states"][stage]["intrinsic"]["actual"]);
            assert_eq!(actual.len(), 2);
            for row in actual {
                assert_eq!(
                    row["selected"],
                    if row["mode"] == "MAIN" { main } else { calcs },
                    "{name} {stage}"
                );
            }
        }
    }
    for (name, raw) in [("raw-level-one", "1"), ("raw-level-forty", "40")] {
        let case = cases.iter().find(|c| c["name"] == name).unwrap();
        for stage in STAGES {
            let saved = rows(&case["states"][stage]["occurrences"]["saved"]);
            assert_eq!(
                saved.iter().find(|x| x["selected"] == true).unwrap()["attributes"]["level"],
                raw
            );
            assert!(
                rows(&case["states"][stage]["intrinsic"]["actual"])
                    .iter()
                    .all(|x| x["prepared"]["level"] != 17)
            );
        }
    }
    let quality = cases
        .iter()
        .find(|c| c["name"] == "raw-quality-fractional")
        .unwrap();
    let duplicate = cases
        .iter()
        .find(|c| c["name"] == "independent-copy-level-one")
        .unwrap();
    for stage in STAGES {
        let saved = rows(&quality["states"][stage]["occurrences"]["saved"]);
        assert_eq!(
            saved.iter().find(|x| x["selected"] == true).unwrap()["attributes"]["quality"],
            "20.5"
        );
        assert!(
            rows(&quality["states"][stage]["intrinsic"]["actual"])
                .iter()
                .all(|x| x["prepared"]["quality"].as_f64().unwrap() > 0.)
        );
        let actual = rows(&duplicate["states"][stage]["intrinsic"]["actual"]);
        assert_eq!(actual.len(), 4);
        let saved = rows(&duplicate["states"][stage]["occurrences"]["saved"]);
        let selected: Vec<_> = saved.iter().filter(|x| x["selected"] == true).collect();
        assert_eq!(selected.len(), 2);
        assert_ne!(selected[0]["source_ordinal"], selected[1]["source_ordinal"]);
        for mode in ["MAIN", "CALCS"] {
            let actions: Vec<_> = actual.iter().filter(|x| x["mode"] == mode).collect();
            assert_eq!(actions.len(), 2);
            assert_ne!(actions[0]["source_ordinal"], actions[1]["source_ordinal"]);
            assert_ne!(
                actions[0]["prepared"]["level"],
                actions[1]["prepared"]["level"]
            );
            for row in actions {
                let input = selected
                    .iter()
                    .find(|s| s["source_ordinal"] == row["source_ordinal"])
                    .unwrap();
                if input["attributes"]["level"] == "1" {
                    assert_eq!(row["selected"], if mode == "MAIN" { 2 } else { 1 });
                }
            }
        }
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_default();
    String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(6000)..]).into_owned()
}
