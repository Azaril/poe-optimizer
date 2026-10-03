//! Constructed source stat sets and actual physical occurrence selections.
//! No owned action identity, native selection importer, or numeric parity is granted.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/active_gem_occurrence_source.rs"]
mod occurrence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_source_spell_stat_sets_preserve_constructed_correspondence";
const CHILD: &str = "POE_SPELL_STAT_SET_SOURCE_CHILD";
const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
const XML_SHA256: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const CATALOG: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
const CATALOG_DIGEST: &str = "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea";
const ICE: &str = "IceNovaPlayer";

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for source correspondence"]
fn complete_source_spell_stat_sets_preserve_constructed_correspondence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-spell-stat-set-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        let xml = fs::read_to_string(root.join(XML)).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(xml.as_bytes())), XML_SHA256);
        let catalog = SkillIdentityCatalog::new(
            serde_json::from_slice(&fs::read(root.join(CATALOG)).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
            .to_string(),
            CATALOG_DIGEST
        );
        let reviewed: Vec<_> = [
            "Metadata/Items/Gems/SkillGemFrostBomb",
            "Metadata/Items/Gems/SkillGemIceNova",
        ]
        .iter()
        .map(|key| {
            catalog
                .data()
                .gems
                .iter()
                .find(|gem| gem.key == *key)
                .unwrap()
                .clone()
        })
        .collect();
        for gem in &reviewed {
            assert_eq!(
                gem.effect_list,
                std::slice::from_ref(&gem.primary_effect_id)
            );
            assert!(
                gem.declared_additional_effects.is_empty()
                    && gem.constructed_additional_effects.is_empty()
                    && gem.additional_effects.is_empty()
            );
        }
        let reviewed = serde_json::to_value(reviewed).unwrap();
        let enabled = mode == "on";
        let baseline = run(&root, &xml, enabled, &reviewed);
        assert_eq!(baseline["kind"], "complete", "{baseline}");
        let inventory = baseline["constructed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["effect"] == ICE)
            .unwrap();
        let count = inventory["stat_sets"].as_array().unwrap().len();
        assert!((1..=64).contains(&count));
        let mut cases = vec![];
        for index in 1..=count {
            let calcs = index % count + 1;
            cases.push((
                format!("constructed-{index}-calcs-{calcs}"),
                mutate(
                    &xml,
                    &[],
                    &maps(ICE, &index.to_string(), &calcs.to_string()),
                    false,
                ),
                Some((index, calcs)),
            ));
        }
        cases.push((
            "legacy-scalars-overwritten".into(),
            mutate(
                &xml,
                &[("statSetIndex", "7"), ("statSetIndexCalcs", "9")],
                "",
                false,
            ),
            Some((1, 1)),
        ));
        cases.push((
            "unrelated-effect-key".into(),
            mutate(&xml, &[], &maps("UnrelatedEffect", "2", "2"), false),
            Some((1, 1)),
        ));
        cases.push((
            "malformed-index".into(),
            mutate(&xml, &[], &maps(ICE, "bad", "bad"), false),
            Some((1, 1)),
        ));
        cases.push(("missing-index".into(), mutate(&xml, &[], &format!(r#"<StatSetIndex grantedEffect="{ICE}"/><StatSetCalcsIndex grantedEffect="{ICE}"/>"#), false), Some((1, 1))));
        for index in [
            "0".to_string(),
            "-1".to_string(),
            "1.5".to_string(),
            (count + 1).to_string(),
        ] {
            cases.push((
                format!("invalid-numeric-{index}"),
                mutate(&xml, &[], &maps(ICE, &index, &index), false),
                None,
            ));
        }
        if count > 1 {
            for (first, last) in [(1, count), (count, 1)] {
                let children = format!(
                    "{}{}",
                    maps(ICE, &first.to_string(), &first.to_string()),
                    maps(ICE, &last.to_string(), &last.to_string())
                );
                cases.push((
                    format!("duplicate-{first}-then-{last}"),
                    mutate(&xml, &[], &children, false),
                    Some((last, last)),
                ));
            }
            cases.push((
                "independent-physical-siblings".into(),
                mutate(&xml, &[], &maps(ICE, &count.to_string(), "1"), true),
                None,
            ));
        }
        assert!(cases.len() <= 80);
        let mut observations = Vec::new();
        for (name, changed, expected) in cases {
            assert_ne!(changed, xml);
            let state = run(&root, &changed, enabled, &reviewed);
            observations.push(json!({"name":name,"expected_selection":expected,"state":state}));
        }
        let repeat = run(&root, &xml, enabled, &reviewed);
        let result = json!({"manifest_sha256":pinned::manifest_sha256(),"xml_sha256":XML_SHA256,"catalog_digest":CATALOG_DIGEST,
            "native_action_identity":false,"native_parity":false,"source_coverage":"two_selected_physical_spell_families",
            "files":(["src/Modules/Data.lua","src/Classes/SkillsTab.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Data/Skills/act_int.lua","src/Data/Gems.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
            "baseline":baseline,"cases":observations,"repeat":repeat});
        let bytes = serde_json::to_vec_pretty(&result).unwrap();
        assert!(bytes.len() <= 32 * 1024 * 1024);
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            bytes,
        )
        .unwrap();
        check(&result);
        assert_eq!(fs::read_to_string(root.join(XML)).unwrap(), xml);
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
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if started.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = read(&out.join("source-jit-off.json"));
    let on = read(&out.join("source-jit-on.json"));
    assert_eq!(off["baseline"], on["baseline"]);
    assert_eq!(off["repeat"], on["repeat"]);
    for (left, right) in off["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(on["cases"].as_array().unwrap())
    {
        assert_eq!(left["name"], right["name"]);
        assert_eq!(left["state"]["kind"], right["state"]["kind"]);
        if left["state"]["kind"] == "complete" {
            assert_eq!(left, right);
        }
    }
}

fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn maps(effect: &str, main: &str, calcs: &str) -> String {
    format!(
        r#"<StatSetIndex grantedEffect="{effect}" index="{main}"/><StatSetCalcsIndex grantedEffect="{effect}" index="{calcs}"/>"#
    )
}
fn run(root: &Path, xml: &str, enabled: bool, reviewed: &Json) -> Json {
    let reached_observer = Cell::new(false);
    let before = |lua: &Lua| {
        lua.globals().set("occurrenceXml", xml)?;
        lua.globals().set("occurrenceOriginal", 0)?;
        lua.globals().set("occurrenceJit", enabled)?;
        lua.globals().set(
            "occurrenceReviewed",
            lua.to_value(
                &reviewed
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| row["key"].as_str().unwrap())
                    .collect::<Vec<_>>(),
            )?,
        )?;
        lua.globals()
            .set("spellStatSetReviewed", lua.to_value(reviewed)?)?;
        lua.load("if occurrenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        reached_observer.set(true);
        let constructed: Value = lua
            .load(include_str!("support/spell_stat_set_source.lua"))
            .set_name("@owned-spell-stat-set-inventory")
            .eval()?;
        let state: Value = lua
            .load(occurrence::OBSERVE)
            .set_name("@owned-spell-stat-set-occurrences")
            .eval()?;
        Ok(
            json!({"kind":"complete","constructed":lua.from_value::<Json>(constructed)?,"occurrences":lua.from_value::<Json>(state)?}),
        )
    };
    let temp = tempfile::tempdir().unwrap();
    match source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        temp.path(),
        xml,
        None,
        false,
        Some(&before),
        None,
        Some(&observe),
    ) {
        Ok(value) => {
            assert_eq!(value["configuration_method_wrappers"], false);
            assert_eq!(value["original_build_output_available"], true);
            value["additional_observation"].clone()
        }
        Err(error) => {
            json!({"kind":if reached_observer.get(){"failed_observation_after_lifecycle"}else{"failed_complete_lifecycle"},"error":error.to_string()})
        }
    }
}

fn mutate(xml: &str, attributes: &[(&str, &str)], children: &str, duplicate: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|node| node.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    let gem = skills
        .descendants()
        .find(|node| {
            node.has_tag_name("Gem")
                && node.attribute("skillId") == Some(ICE)
                && node.ancestors().any(|ancestor| {
                    ancestor.has_tag_name("SkillSet") && ancestor.attribute("id") == Some(selected)
                })
        })
        .unwrap();
    assert!(gem.children().all(|node| !node.is_element()));
    let mut replacement = String::from("<Gem");
    for attribute in gem.attributes() {
        if !attributes.iter().any(|(name, _)| *name == attribute.name()) {
            replacement.push_str(&format!(
                " {}=\"{}\"",
                attribute.name(),
                attribute
                    .value()
                    .replace('&', "&amp;")
                    .replace('"', "&quot;")
                    .replace('<', "&lt;")
            ));
        }
    }
    for (name, value) in attributes {
        replacement.push_str(&format!(" {name}=\"{value}\""));
    }
    replacement.push('>');
    replacement.push_str(children);
    replacement.push_str("</Gem>");
    if duplicate {
        replacement = format!("{}{}", &xml[gem.range()], replacement);
    }
    let mut result = xml.to_owned();
    result.replace_range(gem.range(), &replacement);
    result
}

fn selections(state: &Json, effect: &str, mode: &str) -> Vec<usize> {
    state["occurrences"]["selected"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["attributes"]["skillId"] == effect)
        .map(|row| {
            let action = &row[mode].as_array().unwrap()[0];
            assert_eq!(action["physical_source"], true);
            assert_eq!(action["selected_index"], action["selected_table_index"]);
            action["selected_table_index"].as_u64().unwrap() as usize
        })
        .collect()
}
fn check(result: &Json) {
    assert_eq!(result["baseline"], result["repeat"]);
    let baseline = &result["baseline"];
    assert_eq!(selections(baseline, ICE, "MAIN"), [1]);
    assert_eq!(selections(baseline, ICE, "CALCS"), [1]);
    assert_eq!(selections(baseline, "FrostBombPlayer", "MAIN"), [1]);
    let inventory = baseline["constructed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["effect"] == ICE)
        .unwrap();
    let count = inventory["stat_sets"].as_array().unwrap().len();
    for alias in inventory["aliases"].as_array().unwrap() {
        assert_eq!(alias["standalone_skill"], false);
        assert_eq!(alias["in_effect_list"], false);
    }
    for case in result["cases"].as_array().unwrap() {
        if let Some(expected) = case["expected_selection"].as_array() {
            assert_eq!(
                case["state"]["kind"], "complete",
                "{}: {}",
                case["name"], case["state"]
            );
            assert_eq!(
                selections(&case["state"], ICE, "MAIN"),
                [expected[0].as_u64().unwrap() as usize]
            );
            assert_eq!(
                selections(&case["state"], ICE, "CALCS"),
                [expected[1].as_u64().unwrap() as usize]
            );
        } else if case["name"] == "independent-physical-siblings" {
            assert_eq!(case["state"]["kind"], "complete");
            assert_eq!(selections(&case["state"], ICE, "MAIN"), [1, count]);
            assert_eq!(selections(&case["state"], ICE, "CALCS"), [1, 1]);
        } else {
            // Preserve actual whole-lifecycle failure, or an exact observed
            // selection if source construction succeeds. Do not infer a clamp.
            assert!(
                case["name"]
                    .as_str()
                    .unwrap()
                    .starts_with("invalid-numeric-")
            );
            assert!(matches!(
                case["state"]["kind"].as_str(),
                Some(
                    "complete" | "failed_complete_lifecycle" | "failed_observation_after_lifecycle"
                )
            ));
        }
    }
}
