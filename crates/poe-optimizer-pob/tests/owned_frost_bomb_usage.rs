//! Optional complete-source evidence for Frost Bomb's physical occurrence gate.
//! No exposure arithmetic, native whole-build parity, or count domain expansion.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/active_gem_occurrence_source.rs"]
mod occurrence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_core::{build_identity::BuildLineage, owned_content::digest_owned};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
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

const TEST: &str = "complete_frost_bomb_usage_preserves_exact_physical_occurrences";
const CHILD: &str = "POE_FROST_BOMB_USAGE_CHILD";
const FROST: &str = "Metadata/Items/Gems/SkillGemFrostBomb";
const EFFECT: &str = "FrostBombPlayer";
const CATALOG_DIGEST: &str = "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea";

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for Frost Bomb usage"]
fn complete_frost_bomb_usage_preserves_exact_physical_occurrences() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-frost-bomb-usage-source-01");
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
                assert!(
                    status.success(),
                    "source child failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap(),
        "complete evidence must be byte-identical across JIT modes"
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let directory = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json =
        serde_json::from_slice(&fs::read(directory.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(directory.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_slice(
            &fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
        )
        .unwrap(),
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
    let physical = catalog
        .data()
        .gems
        .iter()
        .find(|gem| gem.key == FROST)
        .unwrap();
    assert_eq!(physical.primary_effect_id, EFFECT);
    assert_eq!(physical.effect_list, [EFFECT]);
    assert!(
        physical.declared_additional_effects.is_empty()
            && physical.constructed_additional_effects.is_empty()
            && physical.additional_effects.is_empty()
    );
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        let xml = std::str::from_utf8(bytes).unwrap();
        cases.push(observe_case(
            root,
            &format!("original-{:02}", i + 1),
            xml,
            enabled,
        ));
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    let controls = [
        (
            "global-1-false",
            edit(
                xml,
                Some(("enableGlobal1", Some("false"))),
                None,
                None,
                false,
            ),
        ),
        (
            "global-2-false",
            edit(
                xml,
                Some(("enableGlobal2", Some("false"))),
                None,
                None,
                false,
            ),
        ),
        (
            "disabled-gem",
            edit(xml, Some(("enabled", Some("false"))), None, None, false),
        ),
        (
            "disabled-group",
            edit(xml, None, Some(("enabled", Some("false"))), None, false),
        ),
        (
            "duplicate-first-active",
            edit(xml, None, None, Some(false), false),
        ),
        (
            "duplicate-second-active",
            edit(
                xml,
                Some(("enableGlobal1", Some("false"))),
                None,
                Some(true),
                false,
            ),
        ),
        (
            "archived-only",
            edit(
                xml,
                Some(("enableGlobal1", Some("false"))),
                None,
                None,
                true,
            ),
        ),
        (
            "global-1-missing",
            edit(xml, Some(("enableGlobal1", None)), None, None, false),
        ),
        (
            "global-1-malformed",
            edit(xml, Some(("enableGlobal1", Some("bad"))), None, None, false),
        ),
    ];
    for (name, changed) in controls {
        assert_ne!(changed, xml);
        cases.push(observe_case(root, name, &changed, enabled));
    }
    cases.push(observe_case(root, "repeat-original-05", xml, enabled));
    let result = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4", "manifest_sha256":pinned::manifest_sha256(),
        "catalog_digest":CATALOG_DIGEST,"physical_identity":physical,
        "business_wrappers":false,"native_parity":false,"native_inventory_authority":false,"native_build_parity":false,"exposure_arithmetic_claimed":false,"phase_independent_activity_claimed":false,"count_domain":[1],
        "lifecycle_stages":["fresh_complete_load","passive_original_frame","requested_original_frame_rebuild_1","requested_original_frame_rebuild_2"],
        "files":(["src/HeadlessWrapper.lua","src/Modules/Build.lua","src/Classes/CalcsTab.lua","src/Classes/SkillsTab.lua","src/Modules/Data.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Modules/Calcs.lua","src/Data/Skills/act_int.lua","src/Data/Gems.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":cases
    });
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
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(directory.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
    check(&result);
}

fn observe_case(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    eprintln!(
        "Frost source case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("occurrenceXml", xml)?;
        lua.globals().set("occurrenceOriginal", 0)?;
        lua.globals()
            .set("occurrenceReviewed", lua.to_value(&[FROST])?)?;
        lua.globals().set("occurrenceJit", enabled)?;
        lua.load("if occurrenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        // This existing helper's before phase authenticates the complete original
        // lifecycle only; its Djinn-specific observation branch is never called.
        lua.globals().set("djinnPhase", "before")?;
        Ok(lua
            .load(include_str!("support/djinn_provider_source.lua"))
            .set_name("@frost-original-lifecycle-authentication")
            .eval()?)
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = observe_stage(lua)?;
        original_frame(lua, false)?;
        let passive = observe_stage(lua)?;
        original_frame(lua, true)?;
        let rebuilt_once = observe_stage(lua)?;
        original_frame(lua, true)?;
        let rebuilt_twice = observe_stage(lua)?;
        Ok(
            json!({"fresh":fresh,"passive":passive,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}),
        )
    };
    let temp = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        temp.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&install),
        Some(&observe),
    )
    .unwrap_or_else(|error| panic!("{name}: complete source failed: {error}"));
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    let stages = &result["additional_observation"];
    let state = &stages["fresh"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([76; 16]),
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
                && row.attribute("gemId").and_then(|attr| attr.decoded().ok()) == Some(FROST)
        })
        .collect();
    assert_eq!(sources.len(), rows(&state["exact"]["saved"]).len());
    let mut joins = Vec::new();
    for source in sources {
        let ordinal = source.occurrence().id().ordinal();
        let saved = rows(&state["exact"]["saved"])
            .iter()
            .find(|row| row["source_ordinal"] == ordinal)
            .unwrap();
        assert_eq!(
            saved["attributes"].as_object().unwrap().len(),
            source.attributes().len()
        );
        for attribute in source.attributes() {
            assert_eq!(
                saved["attributes"][&attribute.origin().name],
                attribute.decoded().unwrap()
            );
        }
        joins.push(json!({"source":source.occurrence().id(),"source_ordinal":ordinal,"preset":saved["preset"],"group_source_ordinal":saved["group_source_ordinal"]}));
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"source_joins":joins,"source_hash":result["source_hash"],"state":state,
        "passive_frame":stages["passive"],"rebuilt_once":stages["rebuilt_once"],"rebuilt_twice":stages["rebuilt_twice"]})
}

fn observe_stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let occurrences: Value = lua
        .load(occurrence::OBSERVE)
        .set_name("@frost-physical-occurrences")
        .eval()?;
    let exact: Value = lua
        .load(include_str!("support/frost_bomb_usage_source.lua"))
        .set_name("@frost-saved-source-ownership")
        .eval()?;
    Ok(
        json!({"occurrences":lua.from_value::<Json>(occurrences)?,"exact":lua.from_value::<Json>(exact)?}),
    )
}

fn original_frame(lua: &Lua, request_rebuild: bool) -> Result<(), RuntimeError> {
    lua.globals().set("frostRequestRebuild", request_rebuild)?;
    lua.load(
        r#"
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local name=info.source:gsub("\\","/")
 assert(info.what=="Lua"and name:sub(-#path)==path and info.linedefined==line)
 return f
end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
local output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486)
assert(output==djinnOriginals.refs.calcs_tab_output and build.buildFlag==false)
local revision=build.outputRevision
local mainEnv,calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
-- This is the same scheduling flag set by original UI edits. The original
-- frame owns cache clearing, revision advancement and both full output passes.
if frostRequestRebuild then build.buildFlag=true end
callback("OnFrame")
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
assert(build.buildFlag==false)
assert(build.outputRevision==revision+(frostRequestRebuild and 1 or 0))
assert((build.calcsTab.mainEnv~=mainEnv)==frostRequestRebuild)
assert((build.calcsTab.calcsEnv~=calcsEnv)==frostRequestRebuild)
"#,
    )
    .set_name("@frost-original-frame-control")
    .exec()?;
    Ok(())
}

type AttributeEdit<'a> = Option<(&'a str, Option<&'a str>)>;
fn rewrite(node: roxmltree::Node<'_, '_>, change: AttributeEdit<'_>) -> String {
    let mut text = format!("<{}", node.tag_name().name());
    for attr in node.attributes() {
        if change.is_some_and(|(name, _)| name == attr.name()) {
            continue;
        }
        text.push_str(&format!(" {}=\"{}\"", attr.name(), escape(attr.value())));
    }
    if let Some((name, Some(value))) = change {
        text.push_str(&format!(" {name}=\"{}\"", escape(value)));
    }
    text.push('>');
    text
}
fn edit(
    xml: &str,
    gem_edit: AttributeEdit<'_>,
    group_edit: AttributeEdit<'_>,
    duplicate: Option<bool>,
    archived: bool,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|node| node.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    let mut edits = Vec::new();
    for gem in skills
        .descendants()
        .filter(|node| node.has_tag_name("Gem") && node.attribute("gemId") == Some(FROST))
    {
        let set = gem
            .ancestors()
            .find(|node| node.has_tag_name("SkillSet"))
            .unwrap();
        if (set.attribute("id") == Some(selected)) == archived {
            continue;
        }
        assert!(gem.children().all(|node| !node.is_element()));
        let group = gem.parent().unwrap();
        if group_edit.is_some() {
            let start = group.range().start;
            let end = start + xml[start..].find('>').unwrap() + 1;
            edits.push((start..end, rewrite(group, group_edit)));
        }
        if gem_edit.is_some() || duplicate.is_some() {
            let mut replacement = format!("{}</Gem>", rewrite(gem, gem_edit));
            if let Some(active) = duplicate {
                replacement.push_str(&format!(
                    "{}</Gem>",
                    rewrite(
                        gem,
                        Some(("enableGlobal1", Some(if active { "true" } else { "false" })))
                    )
                ));
            }
            edits.push((gem.range(), replacement));
        }
    }
    assert_eq!(edits.len(), if archived { 3 } else { 1 });
    edits.sort_by_key(|(range, _)| range.start);
    let mut result = xml.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        result.replace_range(range, &replacement);
    }
    result
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|rows| rows.is_empty()),
            "expected empty Lua list: {value}"
        );
        &[]
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .rev()
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

fn check(result: &Json) {
    let cases = result["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 15);
    let baseline = &cases[4]["state"];
    assert_eq!(baseline, &cases.last().unwrap()["state"]);
    assert_eq!(
        cases[4]["source_joins"],
        cases.last().unwrap()["source_joins"]
    );
    assert_eq!(
        baseline["occurrences"]["selection"],
        json!({"skills":4,"items":2,"spec":3,"config":1,"group":3})
    );
    assert_eq!(baseline["exact"]["selectors"]["MAIN"]["group"], 3);
    assert_eq!(baseline["exact"]["selectors"]["CALCS"]["group"], 1);
    assert_eq!(
        rows(&baseline["occurrences"]["selected"])[0]["source_ordinal"],
        234
    );
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(
            case["state"], case["passive_frame"],
            "{} passive frame unexpectedly rebuilt output",
            case["name"]
        );
        assert_eq!(
            case["rebuilt_once"]["occurrences"], case["rebuilt_twice"]["occurrences"],
            "{} repeat requested rebuild did not settle",
            case["name"]
        );
        let mut exact_once = case["rebuilt_once"]["exact"].clone();
        let exact_twice = &case["rebuilt_twice"]["exact"];
        let initial_revision = case["state"]["exact"]["output_revision"].as_u64().unwrap();
        assert_eq!(exact_once["output_revision"], initial_revision + 1);
        assert_eq!(exact_twice["output_revision"], initial_revision + 2);
        exact_once["output_revision"] = exact_twice["output_revision"].clone();
        assert_eq!(&exact_once, exact_twice);
        for stage in ["state", "passive_frame", "rebuilt_once", "rebuilt_twice"] {
            let baseline = &cases[4][stage];
            assert_eq!(
                baseline,
                &cases.last().unwrap()[stage],
                "repeat original at {stage}"
            );
            let state = &case[stage];
            let exact = &state["exact"];
            let occurrence = &state["occurrences"];
            for flag in [
                "saved_instances_preserved",
                "selected_state_preserved",
                "main_and_calcs_outputs_preserved",
                "source_methods_preserved",
                "fresh_objects",
            ] {
                assert_eq!(occurrence[flag], true, "{}: {flag}", case["name"]);
            }
            assert_eq!(exact["source_methods_preserved"], true);
            assert_eq!(exact["exact_physical_objects"], true);
            assert_eq!(exact["physical_objects_preserved_across_stages"], true);
            assert_eq!(exact["build_flag"], false);
            assert_eq!(exact["output_lifecycle"]["module_reloads"], 2);
            let saved = rows(&exact["saved"]);
            if index < 5 {
                assert_eq!(saved.len(), [0, 0, 0, 1, 4][index]);
            }
            if index >= 5 {
                assert_eq!(
                    occurrence["selection"],
                    baseline["occurrences"]["selection"]
                );
                assert_eq!(exact["selectors"], baseline["exact"]["selectors"]);
            }
            for row in saved {
                assert_eq!(row["loaded"]["count"], 1);
                let eligible = row["selected"] == true
                    && row["loaded"]["enabled"] == true
                    && row["group_enabled"] == true
                    && row["slot_enabled"] == true;
                for mode in ["MAIN", "CALCS"] {
                    // Preserve the cold MAIN inconsistency instead of replacing it
                    // with a warmed interpretation: the first pass sees the lazy
                    // global-effect metadata only after constructing these actions.
                    let cold_main = matches!(stage, "state" | "passive_frame") && mode == "MAIN";
                    let present = eligible && (row["loaded"]["global_1"] == true || cold_main);
                    let actions = rows(&row[mode]);
                    assert_eq!(
                        actions.len(),
                        usize::from(present),
                        "{} source{} {mode} {stage}",
                        case["name"],
                        row["source_ordinal"]
                    );
                    for action in actions {
                        assert_eq!(row["global_effect_flag_present"], true);
                        assert_eq!(row["global_effect_flag"], true);
                        assert_eq!(action["source_ordinal"], row["source_ordinal"]);
                        assert_eq!(action["exact_physical_object"], true);
                        assert_eq!(action["effect"], EFFECT);
                        assert_eq!(action["effect_index"], 1);
                        assert_eq!(action["stat_set_index"], 1);
                    }
                }
            }
            let selected = rows(&occurrence["selected"]);
            for row in selected {
                let source = saved
                    .iter()
                    .find(|saved| saved["source_ordinal"] == row["source_ordinal"])
                    .unwrap();
                for mode in ["MAIN", "CALCS"] {
                    assert_eq!(rows(&row[mode]).len(), rows(&source[mode]).len());
                }
            }
            match case["name"].as_str().unwrap() {
                "global-1-missing" => assert_eq!(selected[0]["loaded"]["global_1"], true),
                "global-1-malformed" | "global-1-false" => {
                    assert_eq!(selected[0]["loaded"]["global_1"], false)
                }
                "global-2-false" => assert_eq!(selected[0]["loaded"]["global_2"], false),
                "duplicate-first-active" | "duplicate-second-active" => {
                    assert_eq!(selected.len(), 2);
                    assert_ne!(selected[0]["source_ordinal"], selected[1]["source_ordinal"]);
                    assert_eq!(selected[0]["source_ordinal"], 234);
                    assert_eq!(
                        selected[0]["loaded"]["global_1"],
                        case["name"] == "duplicate-first-active"
                    );
                    assert_eq!(
                        selected[1]["loaded"]["global_1"],
                        case["name"] == "duplicate-second-active"
                    );
                }
                _ => {}
            }
            if matches!(
                case["name"].as_str(),
                Some("global-2-false" | "global-1-missing" | "archived-only")
            ) {
                for field in ["outputs", "actor_outputs", "full_dps"] {
                    assert_eq!(
                        occurrence[field], baseline["occurrences"][field],
                        "{} {field}",
                        case["name"]
                    );
                }
                for mode in ["MAIN", "CALCS"] {
                    assert_eq!(
                        selected[0][mode],
                        rows(&baseline["occurrences"]["selected"])[0][mode]
                    );
                }
            }
            if case["name"] == "archived-only" {
                assert_eq!(occurrence, &baseline["occurrences"]);
                assert_eq!(
                    saved
                        .iter()
                        .filter(|row| row["loaded"]["global_1"] == false)
                        .count(),
                    3
                );
            }
        }
    }
}
