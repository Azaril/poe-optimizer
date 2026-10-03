//! Optional complete-source Sniper physical, actor, and child-action correspondence evidence.
//! This observes the original evaluator; it does not claim native damage parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/skeletal_actor_families.rs"]
mod skeletal_families;
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

const TEST: &str = "complete_sniper_actor_action_correspondence_preserves_source_selection";
const CHILD: &str = "POE_SNIPER_ACTOR_ACTION_SOURCE_CHILD";
const GEM: &str = "Metadata/Items/Gems/SkillGemSkeletalSniper";
const EFFECT: &str = "SummonSkeletalSnipersPlayer";
const CATALOG_DIGEST: &str = "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea";
#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for Sniper actor/action correspondence"]
fn complete_sniper_actor_action_correspondence_preserves_source_selection() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-sniper-actor-action-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    run_modes(&root, &out, TEST, CHILD);
    assert_eq!(
        digest(&fs::read(out.join("source-jit-off.json")).unwrap()),
        "c854302d2d3516d20b09a4da02934bdc9eab854b6f71b53d6d3f7e2c9d67e005",
        "published Sniper evidence must remain byte-identical"
    );
}

fn run_modes(root: &Path, out: &Path, test: &str, child_variable: &str) {
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--ignored", "--nocapture"])
            .env(child_variable, mode)
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
    let physical = catalog.data().gems.iter().find(|g| g.key == GEM).unwrap();
    assert_eq!(physical.primary_effect_id, EFFECT);
    assert_eq!(physical.effect_list, [EFFECT]);
    for references in [
        &physical.declared_additional_effects,
        &physical.constructed_additional_effects,
    ] {
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].index, 1);
        assert_eq!(references[0].id, "CommandSkeletalSniperPlayer");
    }
    assert!(physical.additional_effects.is_empty());
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        cases.push(observe_case(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
        ));
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    // Each archived source becomes selected through the original XML loader.
    // MAIN selects BasicAttack and CALCS selects GasArrow on that exact source.
    for preset in [3, 4, 5, 6, 1] {
        let changed = edit(
            xml,
            preset,
            &[
                ("skillMinionSkill", Some("1")),
                ("skillMinionSkillCalcs", Some("2")),
            ],
            &[],
            None,
            false,
        );
        let activation = observe_case(
            root,
            &format!("preset-{preset}-activation"),
            &focus(&changed, preset, None),
            enabled,
        );
        let runtime_group = selected(&activation["states"]["fresh"])[0]["group"]
            .as_u64()
            .unwrap();
        cases.push(activation);
        cases.push(observe_case(
            root,
            &format!("preset-{preset}-selected-basic-gas"),
            &focus(&changed, preset, Some(runtime_group)),
            enabled,
        ));
    }
    let focused = focus(xml, 4, None);
    let controls = [
        (
            "missing-actor-names-focused",
            edit(
                &focused,
                4,
                &[("skillMinion", None), ("skillMinionCalcs", None)],
                &[],
                None,
                false,
            ),
        ),
        (
            "invalid-actor-names-focused",
            edit(
                &focused,
                4,
                &[
                    ("skillMinion", Some("unknown-main")),
                    ("skillMinionCalcs", Some("unknown-calcs")),
                ],
                &[],
                None,
                false,
            ),
        ),
        (
            "nonmain-calcs-name-unused",
            edit(
                xml,
                4,
                &[("skillMinionCalcs", Some("unknown-calcs"))],
                &[],
                None,
                false,
            ),
        ),
        (
            "nonmain-missing-actor-names",
            edit(
                xml,
                4,
                &[("skillMinion", None), ("skillMinionCalcs", None)],
                &[],
                None,
                false,
            ),
        ),
        (
            "missing-child-selectors",
            edit(
                &focused,
                4,
                &[("skillMinionSkill", None), ("skillMinionSkillCalcs", None)],
                &[],
                None,
                false,
            ),
        ),
        (
            "malformed-child-selectors",
            edit(
                &focused,
                4,
                &[
                    ("skillMinionSkill", Some("bad")),
                    ("skillMinionSkillCalcs", Some("bad")),
                ],
                &[],
                None,
                false,
            ),
        ),
        (
            "clamped-child-selectors",
            edit(
                &focused,
                4,
                &[
                    ("skillMinionSkill", Some("0")),
                    ("skillMinionSkillCalcs", Some("999")),
                ],
                &[],
                None,
                false,
            ),
        ),
        (
            "main-gas-calcs-basic",
            edit(
                xml,
                4,
                &[
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("1")),
                ],
                &[],
                None,
                false,
            ),
        ),
        (
            "main-basic-calcs-gas-nonmain",
            edit(
                xml,
                4,
                &[
                    ("skillMinionSkill", Some("1")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                &[],
                None,
                false,
            ),
        ),
        (
            "gas-main-three-calcs-one",
            edit(
                &focused,
                4,
                &[
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                &[],
                Some(&child_maps(3, 1)),
                false,
            ),
        ),
        (
            "gas-main-one-calcs-three",
            edit(
                &focused,
                4,
                &[
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                &[],
                Some(&child_maps(1, 3)),
                false,
            ),
        ),
        (
            "explicit-basic-maps",
            edit(
                &focused,
                4,
                &[
                    ("skillMinion", Some("RaisedSkeletonSniper")),
                    ("skillMinionCalcs", Some("RaisedSkeletonSniper")),
                ],
                &[],
                Some(
                    r#"<MinionSkillIndexLookup grantedEffect="SummonSkeletalSnipersPlayer"><MinionSkillIndexMap skillIndex="1" statSetIndex="1"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect="SummonSkeletalSnipersPlayer"><MinionSkillIndexMap skillIndex="1" statSetIndex="1"/></MinionSkillIndexLookupCalcs>"#,
                ),
                false,
            ),
        ),
        (
            "duplicate-child-map-key",
            edit(
                &focused,
                4,
                &[
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                &[],
                Some(
                    r#"<MinionSkillIndexLookup grantedEffect="SummonSkeletalSnipersPlayer"><MinionSkillIndexMap skillIndex="2" statSetIndex="2"/><MinionSkillIndexMap skillIndex="2" statSetIndex="3"/></MinionSkillIndexLookup>"#,
                ),
                false,
            ),
        ),
        (
            "duplicate-independent-physical-sources",
            edit(xml, 4, &[], &[], None, true),
        ),
        (
            "disabled-gem-original-selection",
            edit(xml, 4, &[("enabled", Some("false"))], &[], None, false),
        ),
        (
            "disabled-group-original-selection",
            edit(xml, 4, &[], &[("enabled", Some("false"))], None, false),
        ),
        (
            "disabled-gem-focused",
            edit(&focused, 4, &[("enabled", Some("false"))], &[], None, false),
        ),
        (
            "archived-only-child-edit",
            edit(
                xml,
                3,
                &[
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                &[],
                Some(&child_maps(3, 1)),
                false,
            ),
        ),
    ];
    for (name, changed) in controls {
        assert_ne!(changed, xml);
        cases.push(observe_case(root, name, &changed, enabled));
    }
    cases.push(observe_case(root, "repeat-original-05", xml, enabled));
    let result = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","manifest_sha256":pinned::manifest_sha256(),
        "catalog_digest":CATALOG_DIGEST,"physical_identity":physical,"business_wrappers":false,
        "native_inventory_authority":false,"native_build_parity":false,"canonical_parity_lifecycle_selected":false,
        "lifecycle_stages":["fresh_complete_load","requested_original_frame_rebuild_1","requested_original_frame_rebuild_2"],
        "files":(["src/HeadlessWrapper.lua","src/Modules/Build.lua","src/Classes/CalcsTab.lua","src/Classes/SkillsTab.lua","src/Modules/Data.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Modules/Calcs.lua","src/Modules/CalcPerform.lua","src/Data/Minions.lua","src/Data/Skills/minion.lua","src/Data/Skills/act_int.lua","src/Data/Gems.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "original_sources":originals.iter().enumerate().map(|(i,bytes)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(bytes)})).collect::<Vec<_>>(),
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
    observe_families(root, name, xml, enabled, &[(GEM, EFFECT)])
}
fn observe_families(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    families: &[(&str, &str)],
) -> Json {
    eprintln!(
        "Sniper actor/action case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("sniperActorXml", xml)?;
        lua.globals().set("sniperActorJit", enabled)?;
        lua.globals().set(
            "sniperActorFamilies",
            lua.create_table_from(families.iter().copied())?,
        )?;
        lua.load("if sniperActorJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        // Reuse authentication of the unchanged complete lifecycle only. This
        // helper's Djinn-specific observation branch is never invoked.
        lua.globals().set("djinnPhase", "before")?;
        let finish: Function = lua
            .load(include_str!("support/djinn_provider_source.lua"))
            .set_name("@sniper-actor-action-original-lifecycle-authentication")
            .eval()?;
        lua.globals().set("sniperOriginalFinish", finish)?;
        lua.globals().set("sniperActorPhase", "before")?;
        Ok(lua
            .load(include_str!("support/sniper_actor_action_source.lua"))
            .set_name("@sniper-original-loader-object-observer")
            .eval()?)
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = observe_stage(lua)?;
        original_frame(lua)?;
        let rebuilt_once = observe_stage(lua)?;
        original_frame(lua)?;
        let rebuilt_twice = observe_stage(lua)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
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
    let states = &result["additional_observation"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([83; 16]),
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
                && families.iter().any(|(gem, _)| {
                    row.attribute("gemId").and_then(|attr| attr.decoded().ok()) == Some(*gem)
                })
        })
        .collect();
    assert_eq!(sources.len(), rows(&states["fresh"]["saved"]).len());
    let mut joins = Vec::new();
    for source in sources {
        let ordinal = source.occurrence().id().ordinal();
        let saved = rows(&states["fresh"]["saved"])
            .iter()
            .find(|row| row["source_ordinal"] == ordinal)
            .unwrap();
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let saved = rows(&states[stage]["saved"])
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
        }
        joins.push(json!({"source":source.occurrence().id(),"source_ordinal":ordinal,"preset":saved["preset"],"group_source_ordinal":saved["group_source_ordinal"]}));
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),
        "source_joins":joins,"source_hash":result["source_hash"],"states":states})
}

fn observe_stage(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("sniperActorPhase", "observe")?;
    let value: Value = lua
        .load(include_str!("support/sniper_actor_action_source.lua"))
        .set_name("@sniper-exact-actor-actions")
        .eval()?;
    Ok(lua.from_value(value)?)
}

fn original_frame(lua: &Lua) -> Result<(), RuntimeError> {
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
-- Original UI scheduling flag; the original frame performs both output passes.
build.buildFlag=true
callback("OnFrame")
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
assert(build.buildFlag==false and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=mainEnv and build.calcsTab.calcsEnv~=calcsEnv)
"#,
    )
    .set_name("@sniper-actor-action-original-requested-frame")
    .exec()?;
    Ok(())
}

type AttributeEdit<'a> = (&'a str, Option<&'a str>);
fn rewrite(node: roxmltree::Node<'_, '_>, changes: &[AttributeEdit<'_>]) -> String {
    let mut text = format!("<{}", node.tag_name().name());
    for attr in node.attributes() {
        if changes.iter().any(|(name, _)| *name == attr.name()) {
            continue;
        }
        text.push_str(&format!(" {}=\"{}\"", attr.name(), escape(attr.value())));
    }
    for (name, value) in changes {
        if let Some(value) = value {
            text.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
    }
    text.push('>');
    text
}
fn edit(
    xml: &str,
    preset: u64,
    gem_edits: &[AttributeEdit<'_>],
    group_edits: &[AttributeEdit<'_>],
    children: Option<&str>,
    duplicate: bool,
) -> String {
    let maps = child_maps(3, 1);
    let duplicate_edits = [
        ("count", Some("3")),
        ("skillMinionSkill", Some("2")),
        ("skillMinionSkillCalcs", Some("2")),
    ];
    edit_physical(
        xml,
        preset,
        GEM,
        gem_edits,
        group_edits,
        children,
        duplicate.then_some((&duplicate_edits, &maps)),
    )
}
fn edit_physical(
    xml: &str,
    preset: u64,
    gem_key: &str,
    gem_edits: &[AttributeEdit<'_>],
    group_edits: &[AttributeEdit<'_>],
    children: Option<&str>,
    duplicate: Option<(&[AttributeEdit<'_>], &str)>,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = doc
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem")
                && n.attribute("gemId") == Some(gem_key)
                && n.ancestors().any(|s| {
                    s.has_tag_name("SkillSet")
                        && s.attribute("id").and_then(|v| v.parse::<u64>().ok()) == Some(preset)
                })
        })
        .unwrap();
    assert!(gem.children().all(|n| !n.is_element()));
    let mut edits = Vec::new();
    if !group_edits.is_empty() {
        let group = gem.parent().unwrap();
        let start = group.range().start;
        edits.push((
            start..start + xml[start..].find('>').unwrap() + 1,
            rewrite(group, group_edits),
        ));
    }
    if !gem_edits.is_empty() || children.is_some() || duplicate.is_some() {
        let mut replacement = format!(
            "{}{}</Gem>",
            rewrite(gem, gem_edits),
            children.unwrap_or("")
        );
        if let Some((duplicate_edits, maps)) = duplicate {
            replacement.push_str(&format!("{}{}</Gem>", rewrite(gem, duplicate_edits), maps));
        }
        edits.push((gem.range(), replacement));
    }
    replace(xml, edits)
}
fn focus(xml: &str, preset: u64, runtime_group: Option<u64>) -> String {
    focus_physical(xml, preset, GEM, runtime_group)
}
fn focus_physical(xml: &str, preset: u64, gem_key: &str, runtime_group: Option<u64>) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let set = skills
        .children()
        .find(|n| {
            n.has_tag_name("SkillSet")
                && n.attribute("id").and_then(|v| v.parse::<u64>().ok()) == Some(preset)
        })
        .unwrap();
    let group = set
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .position(|g| {
            g.children()
                .any(|n| n.has_tag_name("Gem") && n.attribute("gemId") == Some(gem_key))
        })
        .unwrap()
        + 1;
    let preset = preset.to_string();
    let group = runtime_group.unwrap_or(group as u64).to_string();
    let build = doc.descendants().find(|n| n.has_tag_name("Build")).unwrap();
    let mut edits = Vec::new();
    for (node, key, value) in [
        (skills, "activeSkillSet", preset.as_str()),
        (build, "mainSocketGroup", group.as_str()),
    ] {
        let start = node.range().start;
        edits.push((
            start..start + xml[start..].find('>').unwrap() + 1,
            rewrite(node, &[(key, Some(value))]),
        ));
    }
    let input = doc
        .descendants()
        .find(|n| {
            n.has_tag_name("Input")
                && n.attribute("name") == Some("skill_number")
                && n.ancestors().any(|n| n.has_tag_name("Calcs"))
        })
        .unwrap();
    edits.push((
        input.range(),
        format!("{}</Input>", rewrite(input, &[("number", Some(&group))])),
    ));
    replace(xml, edits)
}
fn replace(xml: &str, mut edits: Vec<(std::ops::Range<usize>, String)>) -> String {
    assert!(!edits.is_empty());
    edits.sort_by_key(|(r, _)| r.start);
    assert!(edits.windows(2).all(|p| p[0].0.end <= p[1].0.start));
    let mut result = xml.to_owned();
    for (range, text) in edits.into_iter().rev() {
        result.replace_range(range, &text);
    }
    result
}
fn child_maps(main: u64, calcs: u64) -> String {
    format!(
        r#"<MinionSkillIndexLookup grantedEffect="{EFFECT}"><MinionSkillIndexMap skillIndex="2" statSetIndex="{main}"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect="{EFFECT}"><MinionSkillIndexMap skillIndex="2" statSetIndex="{calcs}"/></MinionSkillIndexLookupCalcs>"#
    )
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
fn selected(state: &Json) -> Vec<&Json> {
    rows(&state["saved"])
        .iter()
        .filter(|row| row["selected"] == true)
        .collect()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
        .collect::<Vec<_>>()
        .join("\n")
}

fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 34);
    let baseline = &cases[4];
    assert_eq!(baseline["states"], cases.last().unwrap()["states"]);
    assert_eq!(
        baseline["source_joins"],
        cases.last().unwrap()["source_joins"]
    );
    let originals = rows(&baseline["states"]["fresh"]["saved"]);
    assert_eq!(
        originals
            .iter()
            .map(|r| r["source_ordinal"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [193, 211, 254, 310, 375]
    );
    let original_one = rows(&cases[0]["states"]["fresh"]["saved"]);
    assert_eq!(original_one.len(), 1);
    assert_eq!(original_one[0]["source_ordinal"], 158);
    assert_eq!(original_one[0]["attributes"]["statSetIndex"], "nil");
    assert_eq!(original_one[0]["attributes"]["statSetIndexCalcs"], "nil");
    assert_eq!(
        baseline["states"]["fresh"]["selection"],
        json!({"skills":4,"items":2,"spec":3,"config":1,"group":3})
    );
    for (ci, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap();
        let revision = case["states"]["fresh"]["output_revision"].as_u64().unwrap();
        for (si, stage) in ["fresh", "rebuilt_once", "rebuilt_twice"]
            .into_iter()
            .enumerate()
        {
            let state = &case["states"][stage];
            for flag in [
                "source_methods_preserved",
                "loader_observer_removed",
                "requested_jit_mode_verified",
                "exact_physical_objects",
                "physical_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(state[flag], true, "{name} {stage} {flag}");
            }
            assert_eq!(state["output_revision"], revision + si as u64);
            assert_eq!(state["output_lifecycle"]["module_reloads"], 2);
            assert_eq!(state["build_flag"], false);
            if ci >= 4 {
                assert_eq!(
                    rows(&state["saved"]).len(),
                    if name == "duplicate-independent-physical-sources" {
                        6
                    } else {
                        5
                    }
                );
            }
            for row in rows(&state["saved"]) {
                assert_eq!(
                    row["resolved_additional_count"], 0,
                    "unresolved Command is not an observed child action"
                );
                for mode in ["MAIN", "CALCS"] {
                    let actions = rows(&row[mode]);
                    if row["selected"] == false {
                        assert!(actions.is_empty(), "archived {name} {stage} {mode}");
                    }
                    assert!(actions.len() <= 1);
                    for action in actions {
                        assert_eq!(action["source_ordinal"], row["source_ordinal"]);
                        assert_eq!(action["exact_physical_object"], true);
                        assert_eq!(action["exact_group"], true);
                        assert_eq!(action["minion_choices"], json!(["RaisedSkeletonSniper"]));
                        let actor = &action["actor"];
                        assert_eq!(actor["type"], "RaisedSkeletonSniper");
                        assert_eq!(actor["unique_actor_identity"], true);
                        assert_eq!(actor["source_data_identity"], true);
                        assert_eq!(actor["item_set_present"], false);
                        assert_eq!(actor["is_selected_actor"], action["is_main_skill"]);
                        let expected_name = if mode == "CALCS" && action["is_main_skill"] == true {
                            "skillMinionCalcs"
                        } else {
                            "skillMinion"
                        };
                        assert_eq!(action["actor_selector_field"], expected_name);
                        assert_eq!(action["actor_selector_value"], "RaisedSkeletonSniper");
                        assert_eq!(
                            action["child_selector_field"],
                            if mode == "CALCS" {
                                "skillMinionSkillCalcs"
                            } else {
                                "skillMinionSkill"
                            }
                        );
                        assert_eq!(
                            action["child_selector_value"],
                            actor["selected_child_index"]
                        );
                        let children = rows(&actor["children"]);
                        assert_eq!(children.len(), 2);
                        assert_eq!(children[0]["effect"], "MinionMeleeBow");
                        assert_eq!(children[1]["effect"], "GasShotSkeletonSniperMinion");
                        assert_eq!(rows(&children[0]["stat_sets"]).len(), 1);
                        assert_eq!(rows(&children[1]["stat_sets"]).len(), 3);
                        for child in children {
                            assert_eq!(child["exact_actor"], true);
                            assert_eq!(child["exact_summon"], true);
                            assert_eq!(child["source_instance_present"], false);
                            assert_eq!(child["stat_set"]["declared_table_identity"], true);
                        }
                    }
                }
            }
            if ci < 4 {
                continue;
            }
            let selected = selected(state);
            assert_eq!(
                selected.len(),
                if name == "duplicate-independent-physical-sources" {
                    2
                } else {
                    1
                }
            );
            let first = selected[0];
            for mode in ["MAIN", "CALCS"] {
                let actions = rows(&first[mode]);
                let disabled = matches!(
                    name,
                    "disabled-gem-original-selection" | "disabled-gem-focused"
                ) || (mode == "CALCS"
                    && name == "disabled-group-original-selection");
                assert_eq!(
                    actions.len(),
                    usize::from(!disabled),
                    "{name} {stage} {mode}"
                );
                if let Some(action) = actions.first() {
                    let child = if (mode == "CALCS"
                        && (name.starts_with("preset-")
                            || matches!(
                                name,
                                "clamped-child-selectors" | "main-basic-calcs-gas-nonmain"
                            )))
                        || (mode == "MAIN" && name == "main-gas-calcs-basic")
                        || name.starts_with("gas-")
                        || name == "duplicate-child-map-key"
                    {
                        2
                    } else {
                        1
                    };
                    assert_eq!(
                        action["actor"]["selected_child_index"], child,
                        "{name} {stage} {mode}"
                    );
                    let gas_set = if (mode == "MAIN"
                        && matches!(name, "gas-main-three-calcs-one" | "duplicate-child-map-key"))
                        || (mode == "CALCS" && name == "gas-main-one-calcs-three")
                    {
                        3
                    } else {
                        1
                    };
                    assert_eq!(
                        rows(&action["actor"]["children"])[1]["stat_set"]["index"],
                        gas_set,
                        "{name} {stage} {mode}"
                    );
                    if name.ends_with("-selected-basic-gas")
                        || name.ends_with("focused")
                        || name.starts_with("gas-")
                        || name == "duplicate-child-map-key"
                    {
                        assert_eq!(action["is_main_skill"], true);
                    }
                }
                if name == "duplicate-independent-physical-sources" {
                    let second = &rows(&selected[1][mode])[0];
                    assert_ne!(second["source_ordinal"], first["source_ordinal"]);
                    assert_eq!(second["actor"]["selected_child_index"], 2);
                    assert_eq!(
                        rows(&second["actor"]["children"])[1]["stat_set"]["index"],
                        if mode == "MAIN" { 3 } else { 1 }
                    );
                }
            }
            if name == "nonmain-calcs-name-unused" {
                assert_eq!(first["loaded"]["actor_calcs"], "unknown-calcs");
                assert_eq!(rows(&first["CALCS"])[0]["is_main_skill"], false);
            }
            if name.starts_with("preset-") {
                let preset = name.split('-').nth(1).unwrap().parse::<u64>().unwrap();
                assert_eq!(state["selection"]["skills"], preset);
            }
            if name == "archived-only-child-edit" {
                let previous = selected_row(&baseline["states"][stage]);
                for field in [
                    "attributes",
                    "loaded",
                    "group_state",
                    "group_attributes",
                    "effects",
                ] {
                    assert_eq!(first[field], previous[field]);
                }
                for mode in ["MAIN", "CALCS"] {
                    let mut current = first[mode].clone();
                    let mut original = previous[mode].clone();
                    for actions in [&mut current, &mut original] {
                        for action in actions.as_array_mut().unwrap() {
                            action.as_object_mut().unwrap().remove("source_ordinal");
                        }
                    }
                    assert_eq!(current, original);
                }
                assert_eq!(state["outputs"], baseline["states"][stage]["outputs"]);
            }
        }
    }
}
fn selected_row(state: &Json) -> &Json {
    selected(state)[0]
}
