//! Equipped-item delivery and breakdown observation through the existing full PoB host.
//! Control expectations are predictions until this test executes successfully.
use super::{digest, edit_item, read, rows, source};
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str =
    "flask_charge_delivery_source::equipped_belt_flask_charge_rate_reaches_player_and_breakdown";
const CHILD: &str = "POE_FLASK_CHARGE_DELIVERY_SOURCE_CHILD";
const OUTPUT: &str = "POE_FLASK_CHARGE_DELIVERY_SOURCE_OUTPUT";
const OBSERVE: &str = include_str!("flask_charge_delivery_source.lua");
const SOURCE_FILES: &[&str] = &[
    "src/Classes/CalcsTab.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Modules/CalcBase.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ModParser.lua",
    "src/Data/ModScalability.lua",
];

fn once(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "exact fixture edit: {old}");
    text.replacen(old, new, 1)
}

struct Case {
    name: &'static str,
    xml: String,
    equipped: bool,
    effective: Option<f64>,
}

fn cases(xml: &str) -> Vec<Case> {
    const RATE: &str = "Flasks gain 0.17 charges per Second";
    let removed = edit_item(xml, 27, |body| {
        let body = once(body, &format!("{RATE}\n"), "");
        let body = once(&body, "Implicits: 2", "Implicits: 1");
        let body = once(&body, "\t\t\t<ModRange range=\"0.5\" id=\"2\"/>\n", "");
        once(
            &body,
            "<ModRange range=\"0.5\" id=\"3\"/>",
            "<ModRange range=\"0.5\" id=\"2\"/>",
        )
    });
    // Only the active equipment preset changes; other saved uses keep their item.
    let opening =
        "<ItemSet title=\"Act 1 &amp; 2 (Pre-Ascend)\" useSecondWeaponSet=\"nil\" id=\"2\">";
    assert_eq!(xml.matches(opening).count(), 1);
    let start = xml.find(opening).unwrap() + opening.len();
    let end = start + xml[start..].find("</ItemSet>").unwrap();
    let mut unequipped = xml.to_owned();
    unequipped.replace_range(
        start..end,
        &once(
            &xml[start..end],
            "<Slot itemId=\"27\" itemPbURL=\"\" name=\"Belt\"/>",
            "<Slot itemId=\"0\" itemPbURL=\"\" name=\"Belt\"/>",
        ),
    );
    vec![
        Case {
            name: "original",
            xml: xml.to_owned(),
            equipped: true,
            effective: Some(0.17),
        },
        Case {
            name: "formatted-nonneutral",
            xml: edit_item(xml, 27, |body| {
                once(body, RATE, "Flasks gain 0.175 charges per Second")
            }),
            equipped: true,
            effective: Some(0.18),
        },
        Case {
            name: "present-zero",
            xml: edit_item(xml, 27, |body| {
                once(body, RATE, "Flasks gain 0 charges per Second")
            }),
            equipped: true,
            effective: Some(0.0),
        },
        Case {
            name: "removed-modifier",
            xml: removed,
            equipped: true,
            effective: None,
        },
        Case {
            name: "unequipped-source",
            xml: unequipped,
            equipped: false,
            effective: Some(0.17),
        },
    ]
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@flask-charge-delivery-source")
        .eval()?;
    Ok(lua.from_value(value)?)
}

fn check_records(records: &Json, source: &Json, amount: Option<f64>) {
    let records = rows(records);
    assert_eq!(records.len(), usize::from(amount.is_some()));
    if let Some(amount) = amount {
        let row = &records[0];
        assert_eq!(row["name"], "FlaskChargesGenerated");
        assert_eq!(row["type"], "BASE");
        assert_eq!(row["source"], *source);
        assert_eq!(row["value"], amount);
        assert_eq!(row["flags"], 0);
        assert_eq!(row["keyword_flags"], 0);
        assert!(rows(&row["tags"]).is_empty());
    }
}

fn check_case(case: &Case, state: &Json) {
    for flag in [
        "original_functions_preserved",
        "loaded_item_preserved",
        "saved_selection_preserved",
        "outputs_preserved",
    ] {
        assert_eq!(state[flag], true, "{}: {flag}", case.name);
    }
    assert_eq!(state["item_id"], 27);
    assert_eq!(state["saved"]["items"], 2);
    assert_eq!(state["saved"]["spec"], 3);
    assert_eq!(state["saved"]["skills"], 4);
    assert_eq!(state["saved"]["config"], 1);
    let source = &state["item_source"];
    check_records(&state["loaded_item_records"], source, case.effective);
    let delivered = if case.equipped { case.effective } else { None };
    let expected = delivered.unwrap_or(0.0);
    for mode in ["MAIN", "CALCS"] {
        let row = &state["modes"][mode];
        assert_eq!(row["mode"], mode);
        assert_eq!(row["build_breakdown"], true);
        assert_eq!(row["player_breakdown_present"], true);
        assert_eq!(
            row["selected_belt_id"],
            if case.equipped { json!(27) } else { Json::Null }
        );
        assert!(row["selected_flask_1"].is_null());
        assert!(row["selected_flask_2"].is_null());
        // This exact fixture has one possible general rate source. Any newly
        // observed other source fails the census rather than being hidden.
        check_records(&row["item_records"], source, delivered);
        check_records(&row["player_records"], source, delivered);
        for value in [
            "item_total_base",
            "item_belt_base",
            "player_total_base",
            "player_belt_base",
            "output_flask_charge_gen",
        ] {
            assert_eq!(row[value], expected, "{} {mode} {value}", case.name);
        }
    }
}

fn run_child(root: &Path, out: &Path, mode: &str) {
    assert!(matches!(mode, "off-0" | "off-1" | "on-0" | "on-1"));
    let enabled = mode.starts_with("on-");
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
    let manifest = read(&fixtures.join("index.json"));
    let entry = rows(&manifest["builds"])
        .iter()
        .find(|row| row["xml"] == "build-05.xml")
        .unwrap();
    assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
    let mut observed = Vec::new();
    let mut source_hash = None;
    for case in cases(&xml) {
        let before = |lua: &Lua| {
            lua.globals().set("flaskChargeDeliveryJit", enabled)?;
            lua.load("if flaskChargeDeliveryJit then jit.on() else jit.off();jit.flush() end; assert(jit.status() == flaskChargeDeliveryJit)").exec()?;
            Ok(())
        };
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            None,
            false,
            Some(&before),
            None,
            Some(&observe),
        );
        match result {
            Ok(result) => {
                assert_eq!(result["configuration_method_wrappers"], false);
                assert_eq!(result["original_build_output_available"], true);
                if let Some(prior) = &source_hash {
                    assert_eq!(&result["source_hash"], prior);
                } else {
                    source_hash = Some(result["source_hash"].clone());
                }
                observed.push(json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"state":result["additional_observation"]}));
                // Persist actual observations before an expectation can fail.
                fs::write(
                    out.join(format!("{mode}.json")),
                    serde_json::to_vec_pretty(&json!({"source_hash":source_hash,"cases":observed}))
                        .unwrap(),
                )
                .unwrap();
                check_case(&case, &observed.last().unwrap()["state"]);
            }
            Err(error) => {
                fs::write(out.join(format!("{mode}-failure.json")), serde_json::to_vec_pretty(&json!({"case":case.name,"xml_sha256":digest(case.xml.as_bytes()),"error":error.to_string(),"completed":observed})).unwrap()).unwrap();
                panic!("source observation failed for {}: {error}", case.name);
            }
        }
    }
    assert_eq!(
        fs::read_to_string(fixtures.join("build-05.xml")).unwrap(),
        xml
    );
}

#[test]
fn equipped_belt_flask_charge_rate_reaches_player_and_breakdown() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        let out = PathBuf::from(std::env::var_os(OUTPUT).unwrap())
            .canonicalize()
            .unwrap();
        assert!(out.starts_with(root.join("runs")));
        run_child(&root, &out, &mode.into_string().unwrap());
        return;
    }
    let out = tempfile::Builder::new()
        .prefix("owned-flask-charge-delivery-source-")
        .tempdir_in(root.join("runs"))
        .unwrap()
        .keep();
    let mut reports = Vec::new();
    for mode in ["off-0", "off-1", "on-0", "on-1"] {
        let log_path = out.join(format!("{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
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
                    "source child failed: {}",
                    log_path.display()
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", log_path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        reports.push(read(&out.join(format!("{mode}.json"))));
    }
    for report in &reports[1..] {
        assert_eq!(
            report, &reports[0],
            "all fresh VMs and JIT modes must agree"
        );
    }
    assert_eq!(rows(&reports[0]["cases"]).len(), 5);
    let summary = json!({
        "status":"source_controls_passed", "fresh_vms":20, "fresh_replays_per_case":4,
        "jit_modes":["off","on"], "cases_per_replay":5,
        "source_hash":reports[0]["source_hash"], "manifest_sha256":pinned::manifest_sha256(),
        "source_files":SOURCE_FILES.iter().map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "fixture_sha256":digest(&fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap()),
        "scope":"Original equipped-belt BASE delivery and breakdown-enabled FlaskChargeGen, including explicit zero and source removal; no per-flask uptime or whole-build parity",
        "source_calculation_mocked":false, "native_parity":false, "owner_closure_authority":false,
        "complete_build":false
    });
    fs::write(
        out.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("Flask charge delivery source evidence: {}", out.display());
}
