//! Physical scalar evidence for explicitly reviewed Gems with unresolved Command
//! references. Missing effects, minion/action inventory and native parity remain
//! separate obligations; neither source fallbacks nor table keys prove legality.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::skill_identities::{GemIdentity, SkillIdentityCatalog};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_source_minion_physical_inputs_preserve_unresolved_commands";
const CHILD: &str = "POE_MINION_PHYSICAL_GEM_INPUT_SOURCE_CHILD";
const CATALOG: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
const CATALOG_DIGEST: &str = "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea";
const FIXTURE_SHA256: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";

#[test]
fn complete_source_minion_physical_inputs_preserve_unresolved_commands() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-minion-physical-gem-inputs-02");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        let enabled = mode == "on";
        let catalog_bytes = fs::read(root.join(CATALOG)).unwrap();
        let catalog =
            SkillIdentityCatalog::new(serde_json::from_slice(&catalog_bytes).unwrap()).unwrap();
        let reviewed = reviewed_inputs(&catalog);
        let cases = input_cases();
        let supplied: Vec<_> = cases
            .iter()
            .map(|case| json!({"label":case.label,"attributes":case.attributes}))
            .collect();
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
                .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(xml.as_bytes())),
            FIXTURE_SHA256
        );
        let before = |lua: &Lua| {
            lua.globals().set("minionPhysicalJit", enabled)?;
            lua.globals().set("minionPhysicalXml", xml.as_str())?;
            lua.globals()
                .set("minionPhysicalReviewed", lua.to_value(&reviewed)?)?;
            lua.globals()
                .set("minionPhysicalCases", lua.to_value(&supplied)?)?;
            lua.load("if minionPhysicalJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let scratch = tempfile::tempdir().unwrap();
        let mut result = source::observe_with_build_hook_unwrapped(
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
        result["evidence"] = json!({
            "scope":"reviewed_minion_physical_scalars_with_unresolved_command_references",
            "manifest_sha256":pinned::manifest_sha256(),
            "catalog_sha256":format!("{:x}", Sha256::digest(&catalog_bytes)),
            "catalog_digest":CATALOG_DIGEST,
            "fixture":"build-05.xml", "xml_sha256":FIXTURE_SHA256,
            "native_parity":false, "full_build_numeric_parity":false,
            "physical_input_inventory_complete":false,
            "minion_action_inventory_complete":false,
            "missing_commands_resolved":false,
            "reviewed_gems":reviewed,
            "missing_references":catalog.data().missing_references.iter()
                .filter(|reference| reviewed.iter().any(|gem|gem.key==reference.gem_key))
                .collect::<Vec<_>>(),
            "winning_declarations":reviewed.iter().map(|gem|
                &catalog.data().gem_declarations[gem.winning_declaration as usize-1])
                .collect::<Vec<_>>(),
            "files":([
                "src/Classes/SkillsTab.lua","src/Modules/Data.lua",
                "src/Modules/CalcTools.lua","src/Modules/CalcSetup.lua",
                "src/Modules/CalcActiveSkill.lua","src/Modules/CalcDefence.lua",
                "src/Modules/Calcs.lua","src/Data/Gems.lua",
                "src/Data/Skills/act_int.lua",
            ].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        });
        // Wall time is execution telemetry, not source evidence. Keep measured
        // observations reproducible when the same source witness is rerun.
        assert!(
            result
                .as_object_mut()
                .unwrap()
                .remove("elapsed_ms")
                .is_some()
        );
        // Preserve complete observations even if a provisional expectation fails.
        fs::write(
            destination.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_eq!(result["configuration_method_wrappers"], false);
        assert_eq!(result["original_build_output_available"], true);
        assert_observations(&reviewed, &cases, &result["additional_observation"]);
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source child failed: {}",
                    log_path.display()
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", log_path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = read(&destination.join("source-jit-off.json"));
    let on = read(&destination.join("source-jit-on.json"));
    assert!(
        off == on,
        "complete source observations agree across JIT modes"
    );
}

fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

// These test callers choose a finite review scope. Every identity, unresolved
// reference and winning declaration is obtained from the authenticated catalog;
// the source observer has no name-specific dispatch or native definition IDs.
fn reviewed_inputs(catalog: &SkillIdentityCatalog) -> Vec<GemIdentity> {
    assert_eq!(
        serde_json::to_value(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
        .unwrap(),
        json!(CATALOG_DIGEST)
    );
    [
        "Metadata/Items/Gems/SkillGemSkeletalArsonist",
        "Metadata/Items/Gems/SkillGemSkeletalFrostMage",
        "Metadata/Items/Gems/SkillGemSkeletalReaver",
        "Metadata/Items/Gems/SkillGemSkeletalSniper",
    ]
    .map(|key| {
        let gem = catalog.gem_by_key(key).unwrap();
        let primary = catalog.skill_by_id(&gem.primary_effect_id).unwrap();
        assert_ne!(primary.support, Some(true));
        assert_ne!(primary.from_tree, Some(true));
        assert_eq!(
            gem.effect_list.as_slice(),
            std::slice::from_ref(&gem.primary_effect_id)
        );
        assert!(gem.additional_effects.is_empty());
        assert!(gem.declared_additional_stat_sets.is_empty());
        assert_eq!(gem.declared_additional_effects.len(), 1);
        assert_eq!(
            gem.declared_additional_effects,
            gem.constructed_additional_effects
        );
        let declaration = &catalog.data().gem_declarations[gem.winning_declaration as usize - 1];
        assert_eq!(declaration.key, key);
        assert_eq!(
            declaration.identity.additional_effects,
            gem.declared_additional_effects
        );
        let reference = &gem.declared_additional_effects[0];
        assert_eq!(reference.index, 1);
        assert!(catalog.skill_by_id(&reference.id).is_none());
        let missing: Vec<_> = catalog
            .data()
            .missing_references
            .iter()
            .filter(|row| row.gem_key == key)
            .collect();
        assert_eq!(missing.len(), 2);
        for kind in [
            "declared_additional_effect",
            "constructed_additional_effect",
        ] {
            assert!(missing.iter().any(|row| serde_json::to_value(row).unwrap()
                == json!({
                    "gem_key":key,"kind":kind,"index":reference.index,"effect_id":reference.id,
                })));
        }
        gem.clone()
    })
    .into()
}

struct InputCase {
    label: &'static str,
    attributes: Json,
    level: Option<f64>,
    quality: Option<f64>,
    corrupted: bool,
    corruption_delta: f64,
}

fn input_cases() -> Vec<InputCase> {
    let base = || json!({"level":"20","quality":"20","corrupted":"false","corruptLevel":"0"});
    let mut cases = vec![InputCase {
        label: "original-scalars",
        attributes: json!({"level":"20","quality":"0","corrupted":"false","corruptLevel":"0"}),
        level: Some(20.0),
        quality: Some(0.0),
        corrupted: false,
        corruption_delta: 0.0,
    }];
    for (label, text, level) in [
        ("level-one", Some("1"), Some(1.0)),
        ("level-twenty", Some("20"), Some(20.0)),
        ("level-twenty-one", Some("21"), Some(21.0)),
        ("level-forty", Some("40"), Some(40.0)),
        ("level-zero", Some("0"), Some(1.0)),
        ("level-negative", Some("-3"), Some(1.0)),
        ("level-forty-one", Some("41"), Some(40.0)),
        ("level-fractional", Some("1.5"), Some(20.0)),
        ("level-missing", None, None),
        ("level-malformed", Some("bad"), None),
        ("level-literal-nil", Some("nil"), None),
    ] {
        let mut attributes = base();
        set_attribute(&mut attributes, "level", text);
        cases.push(InputCase {
            label,
            attributes,
            level,
            quality: Some(20.0),
            corrupted: false,
            corruption_delta: 0.0,
        });
    }
    for (label, text, quality) in [
        ("quality-zero", Some("0"), Some(0.0)),
        ("quality-five", Some("5"), Some(5.0)),
        ("quality-twenty", Some("20"), Some(20.0)),
        ("quality-twenty-three", Some("23"), Some(23.0)),
        ("quality-twenty-four", Some("24"), Some(24.0)),
        ("quality-hundred", Some("100"), Some(100.0)),
        ("quality-million", Some("1000000"), Some(1_000_000.0)),
        ("quality-negative", Some("-2"), Some(-2.0)),
        ("quality-fractional", Some("0.5"), Some(0.5)),
        ("quality-missing", None, None),
        ("quality-malformed", Some("bad"), None),
        ("quality-literal-nil", Some("nil"), None),
    ] {
        let mut attributes = base();
        set_attribute(&mut attributes, "quality", text);
        cases.push(InputCase {
            label,
            attributes,
            level: Some(20.0),
            quality,
            corrupted: false,
            corruption_delta: 0.0,
        });
    }
    // Source storage keeps the flag/delta independent of the saved level. These
    // controls do not claim effective-level evaluation or owned scalar admission.
    for (label, flag, delta, corrupted, corruption_delta) in [
        (
            "corrupt-false-positive",
            Some("false"),
            Some("1"),
            false,
            1.0,
        ),
        ("corrupt-true-positive", Some("true"), Some("1"), true, 1.0),
        (
            "corrupt-false-negative",
            Some("false"),
            Some("-1"),
            false,
            -1.0,
        ),
        (
            "corrupt-true-negative",
            Some("true"),
            Some("-1"),
            true,
            -1.0,
        ),
        (
            "corrupt-true-fractional",
            Some("true"),
            Some("0.5"),
            true,
            0.5,
        ),
        ("corrupt-missing", None, None, false, 0.0),
        ("corrupt-malformed", Some("bad"), Some("bad"), false, 0.0),
        ("corrupt-literal-nil", Some("nil"), Some("nil"), false, 0.0),
        (
            "corrupt-flag-malformed",
            Some("TRUE"),
            Some("1"),
            false,
            1.0,
        ),
        (
            "corrupt-delta-malformed",
            Some("true"),
            Some("bad"),
            true,
            0.0,
        ),
        ("corrupt-delta-missing", Some("true"), None, true, 0.0),
    ] {
        let mut attributes = base();
        set_attribute(&mut attributes, "corrupted", flag);
        set_attribute(&mut attributes, "corruptLevel", delta);
        cases.push(InputCase {
            label,
            attributes,
            level: Some(20.0),
            quality: Some(20.0),
            corrupted,
            corruption_delta,
        });
    }
    assert_eq!(cases.len(), 35);
    cases
}

fn set_attribute(attributes: &mut Json, name: &str, text: Option<&str>) {
    if let Some(text) = text {
        attributes[name] = text.into();
    } else {
        attributes.as_object_mut().unwrap().remove(name);
    }
}

fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|rows| rows.is_empty()));
        &[]
    }
}

fn assert_observations(reviewed: &[GemIdentity], cases: &[InputCase], observation: &Json) {
    let catalog = rows(&observation["catalog"]);
    let controls = rows(&observation["load_cases"]);
    let selected = rows(&observation["selected"]);
    assert_eq!(catalog.len(), 4);
    assert_eq!(controls.len(), reviewed.len() * cases.len());
    assert_eq!(selected.len(), 4);
    assert_eq!(observation["selection"]["skills"], 4);
    for name in [
        "saved_instances_preserved",
        "selected_state_preserved",
        "main_and_calcs_outputs_preserved",
        "source_methods_preserved",
        "source_catalog_preserved",
    ] {
        assert_eq!(observation[name], true, "{name}");
    }
    for (gem, actual) in reviewed.iter().zip(catalog) {
        assert_eq!(actual["id"], gem.key);
        assert_eq!(actual["primary"], gem.primary_effect_id);
        assert_eq!(actual["game_id"], gem.game_id);
        assert_eq!(actual["variant_id"], gem.variant_id);
        assert_eq!(actual["primary_data_identity"], true);
        assert_eq!(actual["primary_support"], false);
        assert_eq!(actual["primary_from_tree"], false);
        assert_eq!(actual["natural_max_level"], 20);
        assert_eq!(
            actual["primary_level_keys"],
            json!((1..=40).collect::<Vec<_>>())
        );
        assert_eq!(
            actual["first_stat_set_level_keys"],
            actual["primary_level_keys"]
        );
        assert_eq!(actual["effects"], json!(gem.effect_list));
        assert!(rows(&actual["resolved_additional"]).is_empty());
        let unresolved = rows(&actual["unresolved_references"]);
        assert_eq!(unresolved.len(), gem.constructed_additional_effects.len());
        for (reference, row) in gem.constructed_additional_effects.iter().zip(unresolved) {
            assert_eq!(row["index"], reference.index);
            assert_eq!(row["id"], reference.id);
            assert_eq!(row["standalone_skill"], false);
            assert_eq!(row["in_effect_list"], false);
        }
        let saved = selected
            .iter()
            .find(|row| row["physical_id"] == gem.key)
            .unwrap();
        assert!(saved["source_ordinal"].as_u64().is_some());
        assert_eq!(saved["loaded"]["level"], 20);
        assert_eq!(saved["loaded"]["quality"], 0);
        assert_eq!(saved["loaded"]["corrupted"], false);
        assert_eq!(saved["loaded"]["corrupt_level"], 0);
        for field in [
            "level",
            "quality",
            "corrupted",
            "corrupt_level",
            "gem_id",
            "skill_id",
        ] {
            assert_eq!(saved["loaded"][field], saved["fresh"][field]);
        }
        for mode in ["MAIN", "CALCS"] {
            let actions = rows(&saved[mode]);
            assert_eq!(actions.len(), 1, "{}/{mode}", gem.key);
            let action = &actions[0];
            assert_eq!(action["effect"], gem.primary_effect_id);
            assert_eq!(action["physical_source"], true);
            assert_eq!(action["multiple_reservation"], true);
            for child in rows(&action["minion_skills"]) {
                assert_eq!(child["summon_parent"], true);
                assert!(
                    gem.constructed_additional_effects
                        .iter()
                        .all(|missing| child["effect"] != missing.id)
                );
            }
        }
    }
    for (gem, per_gem) in reviewed.iter().zip(controls.chunks_exact(cases.len())) {
        for (case, row) in cases.iter().zip(per_gem) {
            let context = format!("{}/{}", gem.key, case.label);
            assert_eq!(row["id"], gem.key, "{context}");
            assert_eq!(row["label"], case.label, "{context}");
            assert_eq!(
                row["ok"].as_bool(),
                Some(case.level.is_some()),
                "{context}: {}",
                row["error"]
            );
            if case.level.is_some() {
                assert_eq!(row["physical_gems"], 1, "{context}");
                assert_eq!(row["published_groups"], 1, "{context}");
                assert_eq!(row["same_gem_data"], true, "{context}");
                assert_eq!(row["after"]["gem_id"], gem.key, "{context}");
                assert_eq!(row["after"]["skill_id"], gem.primary_effect_id, "{context}");
                assert_eq!(row["after"]["level"].as_f64(), case.level, "{context}");
                assert_eq!(row["after"]["quality"].as_f64(), case.quality, "{context}");
                assert_eq!(
                    row["after"]["corrupted"].as_bool(),
                    Some(case.corrupted),
                    "{context}"
                );
                assert_eq!(
                    row["after"]["corrupt_level"].as_f64(),
                    Some(case.corruption_delta),
                    "{context}"
                );
                assert_eq!(row["reprocessed"], row["after"], "{context}");
                assert_eq!(row["effects"], json!(gem.effect_list), "{context}");
            } else {
                assert_eq!(row["published_groups"], 0, "{context}");
                assert!(row["after"].is_null(), "{context}");
                assert!(
                    row["error"].as_str().is_some_and(|error| !error.is_empty()),
                    "{context}"
                );
            }
        }
    }
    assert_eq!(controls.iter().filter(|row| row["ok"] == false).count(), 12);
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(include_str!("support/minion_physical_gem_inputs.lua"))
        .set_name("@owned-minion-physical-gem-input-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
