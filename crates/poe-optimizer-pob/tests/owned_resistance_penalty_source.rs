//! Fresh original loads prove the configured Player penalty, not final resistances.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "fresh_penalty_defaults_and_authored_values_reach_player_channels";
const CHILD: &str = "POE_RESISTANCE_PENALTY_SOURCE_CHILD";

#[test]
#[ignore = "complete pinned PoB loads; writes independently repeated JIT evidence"]
fn fresh_penalty_defaults_and_authored_values_reach_player_channels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-resistance-penalty-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let original =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
                .unwrap();
        assert!(!original.contains("name=\"resistancePenalty\""));
        assert_eq!(original.matches("</ConfigSet>").count(), 1);
        let cases = [
            ("default", "", -60.),
            (
                "zero",
                r#"<Input name="resistancePenalty" number="0"/>"#,
                0.,
            ),
            (
                "act",
                r#"<Input name="resistancePenalty" number="-30"/>"#,
                -30.,
            ),
            (
                "fraction",
                r#"<Input name="resistancePenalty" number="-12.5"/>"#,
                -12.5,
            ),
            (
                "positive",
                r#"<Input name="resistancePenalty" number="10"/>"#,
                10.,
            ),
            (
                "placeholder-only",
                r#"<Placeholder name="resistancePenalty" number="20"/>"#,
                -60.,
            ),
            (
                "zero-with-placeholder",
                r#"<Input name="resistancePenalty" number="0"/><Placeholder name="resistancePenalty" number="20"/>"#,
                0.,
            ),
        ];
        let mut reports = Vec::new();
        for (name, addition, expected) in cases {
            let xml = original.replace("</ConfigSet>", &format!("{addition}</ConfigSet>"));
            let mut first = None;
            for repeat in 0..2 {
                eprintln!("penalty {name} independent load {repeat}");
                let before = |lua: &Lua| {
                    lua.globals().set("penaltyJit", enabled)?;
                    lua.load("if penaltyJit then jit.on() else jit.off();jit.flush() end")
                        .exec()?;
                    Ok(())
                };
                let install = |lua: &Lua| -> Result<Function, RuntimeError> {
                    Ok(lua.load(r#"
local c=common.classes.ConfigTab
local calc=require('Modules.CalcBase')
local refs={c.Load,c.CreateConfigSet,c.UpdateControls,calc.initEnv}
for _,f in ipairs(refs)do assert(debug.getinfo(f,'S').what=='Lua')end
return function()
 assert(c.Load==refs[1] and c.CreateConfigSet==refs[2] and c.UpdateControls==refs[3] and calc.initEnv==refs[4])
end
"#).eval()?)
                };
                let observe = |lua: &Lua| -> Result<Value, RuntimeError> {
                    let raw = lua
                        .load(include_str!("support/resistance_penalty_source.lua"))
                        .eval()?;
                    Ok(lua.from_value(raw)?)
                };
                let temp = tempfile::tempdir().unwrap();
                let report = source::observe_with_build_hook_unwrapped(
                    &root.join("vendor/path-of-building-poe2"),
                    temp.path(),
                    &xml,
                    None,
                    false,
                    Some(&before),
                    Some(&install),
                    Some(&observe),
                )
                .unwrap();
                assert_eq!(report["configuration_method_wrappers"], false);
                assert_eq!(report["source_hash"], pinned::manifest_sha256());
                let value = &report["additional_observation"];
                assert_eq!(value["input"], expected);
                for stage in value["stages"].as_array().unwrap() {
                    for mode in ["MAIN", "CALCS"] {
                        for stat in ["FireResist", "ColdResist", "LightningResist"] {
                            let records = stage[mode][stat].as_array().unwrap();
                            assert_eq!(records.len(), 1);
                            assert_eq!(records[0].as_f64(), Some(expected), "{name}/{mode}/{stat}");
                        }
                        let chaos = stage[mode]["ChaosResist"].as_array().unwrap();
                        assert_eq!(chaos.len(), 1);
                        assert_eq!(chaos[0].as_f64(), Some(0.));
                    }
                    assert_eq!(stage, &value["stages"][0], "fixed rebuild disagreement");
                }
                if let Some(first) = &first {
                    assert_eq!(first, value, "independent replay disagreement");
                }
                first = Some(value.clone());
            }
            reports.push(json!({"name":name,"xml_sha256":format!("{:x}",Sha256::digest(xml.as_bytes())),"expected":expected,"observed":first.unwrap()}));
            fs::write(
                out.join(format!(
                    "source-jit-{}-progress.json",
                    if enabled { "on" } else { "off" }
                )),
                serde_json::to_vec_pretty(&reports).unwrap(),
            )
            .unwrap();
        }
        let report = json!({"source_hash":pinned::manifest_sha256(),"original_sha256":format!("{:x}",Sha256::digest(original.as_bytes())),"independent_replays":2,"fixed_rebuilds":2,"cases":reports});
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
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
                    "{}: {}",
                    path.display(),
                    fs::read_to_string(&path).unwrap()
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline; retained log {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap()
    );
}
