//! Offering occurrence fields observed through original complete MAIN/CALCS.
//! This optional source witness grants no native inventory or build parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[path = "support/active_gem_occurrence_source.rs"]
mod occurrence;
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

const TEST: &str = "complete_source_offering_occurrence_fields_keep_their_consumers";
const CHILD: &str = "POE_OFFERING_OCCURRENCE_CHILD";
const ORIGINAL_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const EFFECT: &str = "PainOfferingPlayer";
const OBSERVE: &str = include_str!("support/offering_occurrence_inputs.lua");

#[test]
#[ignore = "optional complete pinned PoB runtime; writes authenticated source evidence"]
fn complete_source_offering_occurrence_fields_keep_their_consumers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-offering-occurrence-inputs-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
        let xml = fs::read_to_string(&path).unwrap();
        assert_eq!(sha256(xml.as_bytes()), ORIGINAL_HASH);
        println!("original05");
        let original = run(&root, &xml, None, enabled);
        let mut controls = Vec::new();
        for (name, gem, group) in [
            ("count-three", vec![("count", "3")], vec![]),
            (
                "group-four",
                vec![("count", "3")],
                vec![("groupCount", "4")],
            ),
            (
                "group-zero",
                vec![("count", "3")],
                vec![("groupCount", "0")],
            ),
            (
                "global-one-false",
                vec![("enableGlobal1", "false"), ("enableGlobal2", "true")],
                vec![],
            ),
            (
                "global-two-false",
                vec![("enableGlobal1", "true"), ("enableGlobal2", "false")],
                vec![],
            ),
            (
                "both-globals-false",
                vec![("enableGlobal1", "false"), ("enableGlobal2", "false")],
                vec![],
            ),
            (
                "legacy-statset-scalars",
                vec![("statSetIndex", "7"), ("statSetIndexCalcs", "9")],
                vec![],
            ),
            (
                "full-dps-one",
                vec![("count", "1")],
                vec![("includeInFullDPS", "true")],
            ),
            (
                "full-dps-three",
                vec![("count", "3")],
                vec![("includeInFullDPS", "true")],
            ),
            (
                "full-dps-group-four",
                vec![("count", "3")],
                vec![("includeInFullDPS", "true"), ("groupCount", "4")],
            ),
            (
                "full-dps-group-zero",
                vec![("count", "3")],
                vec![("includeInFullDPS", "true"), ("groupCount", "0")],
            ),
        ] {
            println!("{name}");
            let changed = mutate(&xml, &gem, &group);
            assert_ne!(changed, xml);
            controls.push(json!({
                "name":name, "xml_sha256":sha256(changed.as_bytes()),
                "state":run(&root,&changed,None,enabled),
            }));
        }
        println!("original05-repeat");
        let repeated_original = run(&root, &xml, None, enabled);
        println!("original05-after-disabled-warm-build");
        let warm = mutate(
            &xml,
            &[("enableGlobal1", "false"), ("enableGlobal2", "false")],
            &[],
        );
        let warm_original = run(&root, &xml, Some(&warm), enabled);
        let result = json!({
            "manifest_sha256":pinned::manifest_sha256(),
            "files":([
                "src/Classes/SkillsTab.lua", "src/Classes/CalcsTab.lua",
                "src/Modules/CalcSetup.lua", "src/Modules/CalcActiveSkill.lua",
                "src/Modules/CalcDefence.lua", "src/Modules/CalcPerform.lua",
                "src/Modules/Calcs.lua", "src/Data/Gems.lua", "src/Data/Skills/act_int.lua",
            ].map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
            "original_xml_sha256":ORIGINAL_HASH,
            "original":original, "controls":controls,
            "repeated_original":repeated_original, "warm_original":warm_original,
            "warm_xml_sha256":sha256(warm.as_bytes()),
            "native_inventory_authority":false, "native_build_parity":false,
        });
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), xml);
        check(&result);
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
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Offering occurrence JIT evidence",
    );
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn run(root: &Path, xml: &str, warm: Option<&str>, enabled: bool) -> Json {
    let before = |lua: &Lua| {
        lua.globals().set("occurrenceXml", xml)?;
        lua.globals().set("occurrenceOriginal", 5)?;
        lua.globals().set("occurrenceJit", enabled)?;
        lua.globals().set(
            "occurrenceReviewed",
            lua.to_value(&[
                "Metadata/Items/Gems/SkillGemPainOffering",
                "Metadata/Items/Gems/SkillGemFrostBomb",
            ])?,
        )?;
        lua.load("if occurrenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let value: Value = lua
            .load(occurrence::OBSERVE)
            .set_name("@owned-offering-base-observer")
            .eval()?;
        let baseline: Json = lua.from_value(value.clone())?;
        lua.globals().set("offeringOccurrenceBase", value)?;
        let offering: Value = lua
            .load(OBSERVE)
            .set_name("@owned-offering-occurrence-observer")
            .eval()?;
        let offering: Json = lua.from_value(offering)?;
        let after: Value = lua
            .load(occurrence::OBSERVE)
            .set_name("@owned-offering-preservation-observer")
            .eval()?;
        let after: Json = lua.from_value(after)?;
        assert_same(
            &baseline,
            &after,
            "Offering observer changed original runtime state",
        );
        Ok(json!({"occurrence":baseline,"offering":offering}))
    };
    let temp = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        temp.path(),
        xml,
        warm,
        false,
        Some(&before),
        None,
        Some(&observe),
    )
    .unwrap();
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    let mut observed = result["additional_observation"].clone();
    observed["source_hash"] = result["source_hash"].clone();
    observed
}

/// Change only the one selected physical occurrence/group in an isolated copy.
fn mutate(xml: &str, gem: &[(&str, &str)], group: &[(&str, &str)]) -> String {
    fn target<'a>(doc: &'a roxmltree::Document<'a>) -> roxmltree::Node<'a, 'a> {
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        let set = skills.attribute("activeSkillSet").unwrap();
        let targets: Vec<_> = skills
            .descendants()
            .filter(|n| {
                n.has_tag_name("Gem")
                    && n.attribute("skillId") == Some(EFFECT)
                    && n.ancestors()
                        .any(|p| p.has_tag_name("SkillSet") && p.attribute("id") == Some(set))
            })
            .collect();
        assert_eq!(targets.len(), 1, "one exact selected physical Offering");
        targets[0]
    }
    fn edit(xml: &str, node: roxmltree::Node<'_, '_>, attrs: &[(&str, &str)]) -> String {
        let range = node.range();
        let body = &xml[range.clone()];
        let end = body.find('>').unwrap();
        let mut start = format!("<{}", node.tag_name().name());
        for attr in node.attributes() {
            if !attrs.iter().any(|(name, _)| *name == attr.name()) {
                start.push_str(&format!(
                    " {}=\"{}\"",
                    attr.name(),
                    attr.value()
                        .replace('&', "&amp;")
                        .replace('"', "&quot;")
                        .replace('<', "&lt;")
                ));
            }
        }
        for (name, value) in attrs {
            start.push_str(&format!(" {name}=\"{value}\""));
        }
        if body[..end].ends_with('/') {
            start.push_str("/>");
        } else {
            start.push('>');
            start.push_str(&body[end + 1..]);
        }
        let mut result = xml.to_owned();
        result.replace_range(range, &start);
        result
    }
    let doc = roxmltree::Document::parse(xml).unwrap();
    let updated = edit(xml, target(&doc), gem);
    let doc = roxmltree::Document::parse(&updated).unwrap();
    edit(&updated, target(&doc).parent().unwrap(), group)
}

fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "expected source rows"
        );
        &[]
    }
}

fn assert_same(left: &Json, right: &Json, label: &str) {
    if let Some(difference) = json_evidence::first_difference(left, right, "$") {
        panic!(
            "{label}: {difference}; full evidence remains in runs/owned-offering-occurrence-inputs-01"
        );
    }
}

fn check(result: &Json) {
    let original = &result["original"];
    assert_same(original, &result["repeated_original"], "fresh repeat");
    assert_same(original, &result["warm_original"], "warm restoration");
    let control = |name: &str| -> &Json {
        &rows(&result["controls"])
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()["state"]
    };
    for state in
        std::iter::once(original).chain(rows(&result["controls"]).iter().map(|c| &c["state"]))
    {
        assert_eq!(state["source_hash"], original["source_hash"]);
        assert!(state["source_hash"].is_string());
        let observation = &state["occurrence"];
        for field in [
            "saved_instances_preserved",
            "selected_state_preserved",
            "main_and_calcs_outputs_preserved",
            "source_methods_preserved",
            "fresh_objects",
        ] {
            assert_eq!(observation[field], true, "{field}");
        }
        let offering = &state["offering"];
        assert_eq!(offering["source_methods_preserved"], true);
        assert_eq!(offering["source_ordinal"], 221);
        assert_eq!(offering["primary_effect"], EFFECT);
        assert_eq!(offering["vaal_gem"], false);
        assert!(rows(&offering["additional_effects"]).is_empty());
        let effects = rows(&offering["granted_effects"]);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0]["ordinal"], 1);
        assert_eq!(effects[0]["id"], EFFECT);
        assert_eq!(effects[0]["is_primary"], true);
        assert_eq!(effects[0]["has_global_effect"], true);
        assert_eq!(effects[0]["global_field"], "enableGlobal1");
        assert_eq!(effects[0]["support"], false);
        assert_eq!(effects[0]["hide_from_sidebar"], false);
        assert_eq!(rows(&effects[0]["stat_sets"]).len(), 1);
        for mode in ["MAIN", "CALCS"] {
            for action in rows(&offering["modes"][mode]["actions"]) {
                assert_eq!(action["physical_source"], true);
                assert_eq!(action["declared_ordinal"], 1);
                assert_eq!(action["global_field"], "enableGlobal1");
                assert_eq!(action["global_value"], true);
                assert_eq!(action["count_enabled"], true);
                for field in [
                    "has_reservation",
                    "supported_by_autoexertion",
                    "reservation_becomes_cost",
                    "summons_totem",
                    "multiple_reservation",
                ] {
                    assert_eq!(
                        action["reservation_guard_inputs"][field], false,
                        "{mode}/{field}"
                    );
                }
                assert_eq!(action["reservation"], json!({}));
                assert_eq!(action["umbral_guard_inputs"]["attached_minion"], false);
                assert_eq!(action["umbral_guard_inputs"]["minion_data"], false);
                assert_eq!(action["umbral_guard_inputs"]["flag"], false);
                assert!(action["umbral_guard_inputs"]["minion_limit"].is_null());
            }
        }
    }
    for mode in ["MAIN", "CALCS"] {
        let baseline = &rows(&original["offering"]["modes"][mode]["actions"])[0];
        assert_eq!(baseline["count"], 1);
        assert!(!rows(&baseline["buffs"]).is_empty());
        assert_eq!(
            original["offering"]["modes"][mode]["selected_minion_affected"],
            true
        );
        assert_eq!(
            original["offering"]["modes"][mode]["player_affected"],
            false
        );
        for (name, count) in [
            ("count-three", 3),
            ("group-four", 4),
            ("group-zero", 0),
            ("global-two-false", 1),
            ("legacy-statset-scalars", 1),
        ] {
            let state = control(name);
            let actions = rows(&state["offering"]["modes"][mode]["actions"]);
            assert_eq!(actions.len(), 1, "{name}/{mode}");
            assert_eq!(actions[0]["count"], count, "{name}/{mode}");
            assert_same(&actions[0]["buffs"], &baseline["buffs"], name);
            assert_same(
                &state["occurrence"]["outputs"][mode],
                &original["occurrence"]["outputs"][mode],
                name,
            );
            assert_same(
                &state["occurrence"]["actor_outputs"][mode],
                &original["occurrence"]["actor_outputs"][mode],
                name,
            );
            assert_eq!(
                state["offering"]["modes"][mode]["selected_minion_affected"],
                true
            );
        }
        for name in ["global-one-false", "both-globals-false"] {
            let state = control(name);
            assert!(rows(&state["offering"]["modes"][mode]["actions"]).is_empty());
            assert_eq!(
                state["offering"]["modes"][mode]["selected_minion_affected"],
                false
            );
            assert_same(
                &state["occurrence"]["outputs"][mode],
                &original["occurrence"]["outputs"][mode],
                name,
            );
            assert_same(
                &state["occurrence"]["actor_outputs"][mode]["player"],
                &original["occurrence"]["actor_outputs"][mode]["player"],
                name,
            );
            for metric in ["CombinedDPS", "Speed"] {
                let enabled = original["occurrence"]["actor_outputs"][mode]["minion"][metric]
                    .as_f64()
                    .unwrap();
                let disabled = state["occurrence"]["actor_outputs"][mode]["minion"][metric]
                    .as_f64()
                    .unwrap();
                assert!(
                    enabled.is_finite()
                        && disabled.is_finite()
                        && enabled > disabled
                        && disabled > 0.0,
                    "{name}/{mode}/{metric}"
                );
            }
        }
        for (name, count) in [
            ("full-dps-one", 1),
            ("full-dps-three", 3),
            ("full-dps-group-four", 4),
            ("full-dps-group-zero", 0),
        ] {
            let state = control(name);
            let actions = rows(&state["offering"]["modes"][mode]["actions"]);
            assert_eq!(actions.len(), 1, "{name}/{mode}");
            assert_eq!(actions[0]["count"], count);
            assert_eq!(actions[0]["full_dps_group_included"], true);
            assert_same(&actions[0]["buffs"], &baseline["buffs"], name);
            assert_eq!(
                state["offering"]["modes"][mode]["selected_minion_affected"],
                true
            );
            // Preserve exact source FullDPS/SkillDPS observations. The Offering
            // may have no own damaging row; absence does not close count usage.
            assert!(
                state["occurrence"]["outputs"][mode]["FullDPS"]
                    .as_f64()
                    .is_some_and(f64::is_finite)
            );
        }
        let legacy = rows(&control("legacy-statset-scalars")["occurrence"]["selected"])
            .iter()
            .find(|r| r["attributes"]["skillId"] == EFFECT)
            .unwrap();
        let actions = rows(&legacy[mode]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["selected_table_index"], 1);
        assert_eq!(legacy["fresh"]["stat_set"], json!({}));
        assert_eq!(legacy["fresh"]["stat_set_calcs"], json!({}));
        assert_eq!(legacy["attributes"]["statSetIndex"], "7");
        assert_eq!(legacy["attributes"]["statSetIndexCalcs"], "9");
    }
}
