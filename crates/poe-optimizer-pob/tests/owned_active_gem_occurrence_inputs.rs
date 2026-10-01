//! Physical Gem values and skill-use settings observed through full source MAIN/CALCS.
//! This grants no native input, action, usage, or whole-build coverage authority.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "support/active_gem_occurrence_source.rs"]
mod witness;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
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

const TEST: &str = "complete_source_active_occurrence_settings_keep_their_consumers";
const CHILD: &str = "POE_ACTIVE_OCCURRENCE_CHILD";
const HASHES: [&str; 5] = [
    "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194",
    "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
    "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490",
    "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711",
    "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
];

#[test]
fn complete_source_active_occurrence_settings_keep_their_consumers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-active-gem-occurrence-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let reviewed = reviewed_gems(&root);
        let mut originals = Vec::new();
        let mut xmls = Vec::new();
        for (i, hash) in HASHES.iter().enumerate() {
            let path = root.join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                i + 1
            ));
            let xml = fs::read_to_string(&path).unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(xml.as_bytes())), *hash);
            originals.push(run(&root, &xml, i + 1, enabled, &reviewed));
            assert_eq!(fs::read_to_string(path).unwrap(), xml);
            xmls.push(xml);
        }
        let mut controls = Vec::new();
        for (name, original, effect, gem, group, children) in [
            (
                "twister-count-three",
                2,
                "TwisterPlayer",
                vec![("count", "3")],
                vec![],
                "",
            ),
            (
                "twister-full-dps-one",
                2,
                "TwisterPlayer",
                vec![],
                vec![("includeInFullDPS", "true")],
                "",
            ),
            (
                "twister-full-dps-three",
                2,
                "TwisterPlayer",
                vec![("count", "3")],
                vec![("includeInFullDPS", "true")],
                "",
            ),
            (
                "twister-group-four",
                2,
                "TwisterPlayer",
                vec![("count", "3")],
                vec![("groupCount", "4")],
                "",
            ),
            (
                "twister-group-zero",
                2,
                "TwisterPlayer",
                vec![("count", "3")],
                vec![("groupCount", "0")],
                "",
            ),
            (
                "pain-count-three",
                5,
                "PainOfferingPlayer",
                vec![("count", "3")],
                vec![],
                "",
            ),
            (
                "pain-global-false",
                5,
                "PainOfferingPlayer",
                vec![("enableGlobal1", "false"), ("enableGlobal2", "false")],
                vec![],
                "",
            ),
            (
                "frost-legacy-statset",
                5,
                "FrostBombPlayer",
                vec![("statSetIndex", "7"), ("statSetIndexCalcs", "9")],
                vec![],
                "",
            ),
            (
                "frost-explicit-statsets",
                5,
                "FrostBombPlayer",
                vec![],
                vec![],
                "<StatSetIndex grantedEffect=\"FrostBombPlayer\" index=\"1\"/><StatSetCalcsIndex grantedEffect=\"FrostBombPlayer\" index=\"1\"/>",
            ),
            (
                "frost-parent-part",
                5,
                "FrostBombPlayer",
                vec![("skillPart", "3")],
                vec![("skillPart", "7")],
                "",
            ),
        ] {
            let xml = mutate(&xmls[original - 1], effect, &gem, &group, children);
            assert_ne!(xml, xmls[original - 1]);
            controls.push(json!({"name":name,"effect":effect,"original":original,
                "state":run(&root,&xml,original,enabled,&reviewed)}));
        }
        let result = json!({
            "manifest_sha256":pinned::manifest_sha256(),
            "files":(["src/Classes/SkillsTab.lua","src/Classes/CalcsTab.lua",
                "src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua",
                "src/Modules/CalcDefence.lua","src/Modules/CalcPerform.lua","src/Modules/Calcs.lua"].map(|path|
                json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
            "original_xml_sha256":HASHES,"originals":originals,"controls":controls,
            "native_inventory_authority":false,"native_build_parity":false,
        });
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
            if start.elapsed() > Duration::from_secs(240) {
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
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "expected rows: {value}"
        );
        &[]
    }
}
fn reviewed_gems(root: &Path) -> Vec<String> {
    let policy = read(&root.join("data/owned/poe2/3887ae68/active-gem-inputs/singleton.json"));
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_value(read(
            &root.join("data/owned/poe2/3887ae68/import/skill-identities.json"),
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        policy["catalog_digest"],
        serde_json::to_value(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
        .unwrap()
    );
    let mut keys: Vec<String> = serde_json::from_value(policy["source_gems"].clone()).unwrap();
    // Twister's physical input recipe predates the shared singleton authoring policy.
    keys.push("Metadata/Items/Gems/SkillGemTwister".into());
    keys.sort();
    keys.dedup();
    for key in &keys {
        let gem = catalog.data().gems.iter().find(|g| &g.key == key).unwrap();
        let effect = catalog
            .data()
            .skills
            .iter()
            .find(|s| s.id == gem.primary_effect_id)
            .unwrap();
        assert_ne!(effect.support, Some(true));
        assert_ne!(effect.from_tree, Some(true));
        assert!(
            gem.declared_additional_effects.is_empty()
                && gem.constructed_additional_effects.is_empty()
                && gem.additional_effects.is_empty()
                && gem.declared_additional_stat_sets.is_empty()
        );
        assert_eq!(
            gem.effect_list.as_slice(),
            std::slice::from_ref(&gem.primary_effect_id)
        );
    }
    keys
}
fn run(root: &Path, xml: &str, original: usize, enabled: bool, reviewed: &[String]) -> Json {
    let before = |lua: &Lua| {
        lua.globals().set("occurrenceXml", xml)?;
        lua.globals().set("occurrenceOriginal", original)?;
        lua.globals().set("occurrenceJit", enabled)?;
        lua.globals()
            .set("occurrenceReviewed", lua.to_value(reviewed)?)?;
        lua.load("if occurrenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let value: Value = lua
            .load(witness::OBSERVE)
            .set_name("@owned-active-occurrence-observer")
            .eval()?;
        Ok(lua.from_value(value)?)
    };
    let temp = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        temp.path(),
        xml,
        None,
        false,
        Some(&before),
        None,
        Some(&observe),
    )
    .unwrap();
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    result["additional_observation"].clone()
}

/// Change only the selected occurrence/group inside an isolated original XML copy.
fn mutate(
    xml: &str,
    effect: &str,
    gem: &[(&str, &str)],
    group: &[(&str, &str)],
    children: &str,
) -> String {
    fn target<'a>(doc: &'a roxmltree::Document<'a>, effect: &str) -> roxmltree::Node<'a, 'a> {
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        let set = skills.attribute("activeSkillSet").unwrap();
        skills
            .descendants()
            .find(|n| {
                n.has_tag_name("Gem")
                    && n.attribute("skillId") == Some(effect)
                    && n.ancestors()
                        .any(|p| p.has_tag_name("SkillSet") && p.attribute("id") == Some(set))
            })
            .unwrap()
    }
    fn edit(
        xml: &str,
        node: roxmltree::Node<'_, '_>,
        attrs: &[(&str, &str)],
        children: &str,
    ) -> String {
        let range = node.range();
        let body = &xml[range.clone()];
        let end = body.find('>').unwrap();
        let mut start = format!("<{}", node.tag_name().name());
        for a in node.attributes() {
            if !attrs.iter().any(|(name, _)| *name == a.name()) {
                start.push_str(&format!(
                    " {}=\"{}\"",
                    a.name(),
                    a.value()
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
            start.push('>');
            start.push_str(children);
            start.push_str(&format!("</{}>", node.tag_name().name()));
        } else {
            start.push('>');
            start.push_str(children);
            start.push_str(&body[end + 1..]);
        }
        let mut result = xml.to_owned();
        result.replace_range(range, &start);
        result
    }
    let doc = roxmltree::Document::parse(xml).unwrap();
    let updated = edit(xml, target(&doc, effect), gem, children);
    let doc = roxmltree::Document::parse(&updated).unwrap();
    edit(&updated, target(&doc, effect).parent().unwrap(), group, "")
}

fn selected<'a>(state: &'a Json, effect: &str) -> &'a Json {
    rows(&state["selected"])
        .iter()
        .find(|r| r["attributes"]["skillId"] == effect)
        .unwrap()
}
fn check(result: &Json) {
    let mut definitions = BTreeSet::new();
    for (state, count) in rows(&result["originals"]).iter().zip([1, 7, 4, 8, 2]) {
        assert_eq!(rows(&state["selected"]).len(), count);
        for field in [
            "saved_instances_preserved",
            "selected_state_preserved",
            "main_and_calcs_outputs_preserved",
            "source_methods_preserved",
            "fresh_objects",
        ] {
            assert_eq!(state[field], true);
        }
        for row in rows(&state["selected"]) {
            definitions.insert(row["physical_id"].as_str().unwrap());
            assert_eq!(row["loaded"]["count"], 1);
            assert_ne!(row["group_state"]["include"], true);
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(rows(&row[mode]).len(), 1);
                for action in rows(&row[mode]) {
                    assert_eq!(action["physical_source"], true);
                    assert_eq!(action["count"], 1);
                    assert_eq!(action["selected_index"], 1);
                    assert_eq!(action["selected_table_index"], 1);
                    assert_eq!(rows(&action["stat_sets"]).len(), 1);
                    assert_eq!(action["skill_types_present"], true);
                    assert_eq!(action["skill_data_present"], true);
                    assert_eq!(action["selected_stat_set_flags_present"], true);
                    assert!(action["instance_flags_present"].is_boolean());
                    assert_eq!(action["multiple_reservation"], false);
                    assert_eq!(action["umbral_environment_db_present"], true);
                    assert_eq!(action["umbral_environment_flag"], false);
                    assert_eq!(action["minion_limit_present"], false);
                }
            }
        }
    }
    assert_eq!(definitions.len(), 19);
    let control = |name: &str| -> &Json {
        &rows(&result["controls"])
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()["state"]
    };
    for (name, expected) in [
        ("twister-count-three", 3),
        ("twister-group-four", 4),
        ("twister-group-zero", 0),
        ("twister-full-dps-three", 3),
    ] {
        for mode in ["MAIN", "CALCS"] {
            let actions = rows(&selected(control(name), "TwisterPlayer")[mode]);
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0]["count"], expected);
        }
    }
    for name in ["frost-legacy-statset", "frost-explicit-statsets"] {
        for mode in ["MAIN", "CALCS"] {
            let actions = rows(&selected(control(name), "FrostBombPlayer")[mode]);
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0]["selected_table_index"], 1);
        }
    }
    let second = &rows(&result["originals"])[1];
    let fifth = &rows(&result["originals"])[4];
    for mode in ["MAIN", "CALCS"] {
        // Count is a per-use reporting input. It does not multiply this build's
        // direct CombinedDPS, and the best Ignite contribution is not stacked.
        assert_eq!(
            control("twister-count-three")["outputs"][mode],
            second["outputs"][mode]
        );
        let single = control("twister-full-dps-one");
        let triple = control("twister-full-dps-three");
        assert_eq!(
            single["outputs"][mode]["CombinedDPS"],
            triple["outputs"][mode]["CombinedDPS"]
        );
        let one_rows = rows(&single["full_dps"][mode]);
        let three_rows = rows(&triple["full_dps"][mode]);
        assert_eq!(one_rows.len(), 2);
        assert_eq!(three_rows.len(), 2);
        for name in ["Twister", "Best Ignite DPS"] {
            let one = one_rows.iter().find(|r| r["name"] == name).unwrap();
            let three = three_rows.iter().find(|r| r["name"] == name).unwrap();
            assert_eq!(one["dps"], three["dps"]);
            assert_eq!(one["count"], 1);
            assert_eq!(three["count"], if name == "Twister" { 3 } else { 1 });
        }
        let one = single["outputs"][mode]["FullDPS"].as_f64().unwrap();
        let three = triple["outputs"][mode]["FullDPS"].as_f64().unwrap();
        assert!(three > one && three < 3.0 * one);
        let pain = &rows(&selected(fifth, "PainOfferingPlayer")[mode])[0];
        assert_eq!(pain["creates_minion"], true);
        assert_eq!(pain["minion_types_present"], true);
        assert_eq!(pain["minion_types"], json!(["Aura"]));
        assert_eq!(pain["minion_present"], false);
        assert_eq!(pain["has_global_effect"], true);
        let count_pain =
            &rows(&selected(control("pain-count-three"), "PainOfferingPlayer")[mode])[0];
        assert_eq!(count_pain["count"], 3);
        assert_eq!(count_pain["minion_present"], false);
        assert_eq!(
            control("pain-count-three")["outputs"][mode],
            fifth["outputs"][mode]
        );
        assert_eq!(
            control("pain-count-three")["actor_outputs"][mode],
            fifth["actor_outputs"][mode]
        );
        let disabled = control("pain-global-false");
        assert_eq!(disabled["outputs"][mode], fifth["outputs"][mode]);
        assert_eq!(
            disabled["actor_outputs"][mode]["player"],
            fifth["actor_outputs"][mode]["player"]
        );
        // An offering can affect another action's actor even though the offering
        // itself has no attached minion. Count does not stand in for this gate.
        for metric in ["CombinedDPS", "Speed"] {
            let enabled_value = fifth["actor_outputs"][mode]["minion"][metric]
                .as_f64()
                .unwrap();
            let disabled_value = disabled["actor_outputs"][mode]["minion"][metric]
                .as_f64()
                .unwrap();
            assert!(enabled_value.is_finite() && disabled_value.is_finite());
            assert!(enabled_value > disabled_value && disabled_value > 0.0);
        }
        assert!(
            rows(&selected(control("pain-global-false"), "PainOfferingPlayer")[mode]).is_empty()
        );
    }
    let overridden = selected(control("frost-parent-part"), "FrostBombPlayer");
    assert_eq!(overridden["fresh"]["skill_part"], 7);
    assert!(overridden["loaded"]["skill_part"].is_null());
    let probe = |name: &str| -> &Json {
        &rows(&fifth["fresh_probes"])
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()["gem"]
    };
    assert_eq!(probe("count-absent")["count"], 1);
    assert_eq!(probe("count-malformed")["count"], 1);
    assert_eq!(probe("count-zero")["count"], 0);
    assert_eq!(probe("global-absent")["global_1"], true);
    assert_eq!(probe("global-absent")["global_2"], false);
    assert_eq!(probe("global-false")["global_1"], false);
    assert_eq!(probe("global-false")["global_2"], false);
    assert_eq!(probe("legacy-statset")["stat_set"], json!({}));
    assert_eq!(probe("legacy-statset")["stat_set_calcs"], json!({}));
    assert_eq!(
        probe("effect-statset-maps")["stat_set"]["FrostBombPlayer"],
        1
    );
    assert_eq!(
        probe("effect-statset-maps")["stat_set_calcs"]["FrostBombPlayer"],
        2
    );
    assert_eq!(probe("first-gem-parent-override")["skill_part"], 7);
}
