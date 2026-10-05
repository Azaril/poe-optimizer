//! Optional complete-loader ownership evidence; no native numerical authority.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/generated_global_switch_census.rs"]
mod generated_global_switch_census;
#[path = "support/generated_skill_usage.rs"]
mod generated_skill_usage;
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

const TEST: &str = "complete_source_separates_authored_roots_from_generated_groups";
const CHILD: &str = "POE_AUTHORED_MEMBERSHIP_SOURCE_CHILD";
const FIREBOLT: &str = "FireboltPlayer";
const STAFF_SOURCE: &str = "Item:28:New Item, Ashen Staff";
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
type DataReadyObserver<'a> = dyn Fn(&Lua) -> Result<(), RuntimeError> + 'a;

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for authored/generated membership evidence"]
fn complete_source_separates_authored_roots_from_generated_groups() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-authored-skill-membership-source-01");
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
        "complete ownership evidence must be byte-identical across JIT modes"
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
        let doc = roxmltree::Document::parse(xml).unwrap();
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        for set in skills.children().filter(|n| n.has_tag_name("SkillSet")) {
            let id = set.attribute("id").unwrap();
            if skills.attribute("activeSkillSet") == Some(id) {
                continue;
            }
            let changed = change_attributes(xml, skills, &[("activeSkillSet", Some(id))]);
            cases.push(observe_case(
                root,
                &format!("original-{:02}-activate-preset-{id}", i + 1),
                &changed,
                enabled,
            ));
        }
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    for (name, changed) in controls(xml) {
        assert_ne!(changed, xml, "control must change its input: {name}");
        cases.push(observe_case(root, name, &changed, enabled));
    }
    cases.push(observe_case(root, "repeat-original-05", xml, enabled));
    let files = [
        "src/HeadlessWrapper.lua",
        "src/Modules/Build.lua",
        "src/Classes/CalcsTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/Item.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Modules/Data.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/Calcs.lua",
        "src/Modules/CalcPerform.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/act_int.lua",
    ];
    let report = json!({
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "business_wrappers":false,"native_inventory_authority":false,"native_build_parity":false,
        "canonical_parity_lifecycle_selected":false,
        "lifecycle_stages":["fresh_complete_load","requested_original_frame_rebuild_1","requested_original_frame_rebuild_2"],
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,bytes)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(bytes)})).collect::<Vec<_>>(),
        "cases":cases,
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "complete report has {} bytes, above 64 MiB",
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
    // Save diagnostics before assertions; only a passing child authenticates them.
    check(&report);
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(directory.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
}

fn observe_case(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    observe_case_with_stage(root, name, xml, enabled, &observe_stage)
}

fn observe_case_with_stage(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    stage: &dyn Fn(&Lua) -> Result<Json, RuntimeError>,
) -> Json {
    observe_case_with_stage_and_data_hook(root, name, xml, enabled, stage, None)
}

fn observe_case_with_stage_and_data_hook(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    stage: &dyn Fn(&Lua) -> Result<Json, RuntimeError>,
    data_ready: Option<&DataReadyObserver<'_>>,
) -> Json {
    eprintln!(
        "Authored membership case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("membershipXml", xml)?;
        lua.globals().set("sniperActorJit", enabled)?;
        lua.load("if sniperActorJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        if let Some(observe_data) = data_ready {
            observe_data(lua)?;
        }
        // Only the original-method authentication and exact LoadSkill return
        // capture branches are reused. Their family-specific observers do not run.
        lua.globals().set("djinnPhase", "before")?;
        let finish: Function = lua
            .load(include_str!("support/djinn_provider_source.lua"))
            .set_name("@membership-original-lifecycle-authentication")
            .eval()?;
        lua.globals().set("sniperOriginalFinish", finish)?;
        lua.globals().set("sniperActorPhase", "before")?;
        Ok(lua
            .load(include_str!("support/sniper_actor_action_source.lua"))
            .set_name("@membership-original-loader-object-observer")
            .eval()?)
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        original_frame(lua)?;
        let rebuilt_once = stage(lua)?;
        original_frame(lua)?;
        let rebuilt_twice = stage(lua)?;
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
        BuildLineage::from_bytes([97; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let sources: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Skill")
        .collect();
    let mut joins = Vec::new();
    for stage in STAGES {
        assert_eq!(sources.len(), rows(&states[stage]["saved_groups"]).len());
        for source in &sources {
            let ordinal = source.occurrence().id().ordinal();
            let saved = rows(&states[stage]["saved_groups"])
                .iter()
                .find(|r| r["source_ordinal"] == ordinal)
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
            for gem in rows(&saved["gems"]) {
                let occurrence = &evidence.rows()[gem["source_ordinal"].as_u64().unwrap() as usize];
                assert_eq!(gem["source"]["name"], occurrence.occurrence().name());
                assert_eq!(
                    gem["source"]["attributes"].as_object().unwrap().len(),
                    occurrence.attributes().len()
                );
                for attribute in occurrence.attributes() {
                    assert_eq!(
                        gem["source"]["attributes"][&attribute.origin().name],
                        attribute.decoded().unwrap()
                    );
                }
            }
            if stage == "fresh" {
                joins.push(json!({"source":source.occurrence().id(),"source_ordinal":ordinal,"preset":saved["preset"]}));
            }
        }
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),
        "source_joins":joins,"source_hash":result["source_hash"],"states":states})
}

fn observe_stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(include_str!("support/authored_skill_membership_source.lua"))
        .set_name("@exact-authored-and-generated-source-ownership")
        .eval()?;
    Ok(lua.from_value(value)?)
}

fn original_frame(lua: &Lua) -> Result<(), RuntimeError> {
    lua.load(
        r#"
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local name=info.source:gsub("\\","/")
 assert(info.what=="Lua" and name:sub(-#path)==path and info.linedefined==line);return f
end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
local output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486)
assert(output==djinnOriginals.refs.calcs_tab_output and build.buildFlag==false)
local revision=build.outputRevision
local mainEnv,calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
assert(build.buildFlag==false and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=mainEnv and build.calcsTab.calcsEnv~=calcsEnv)
"#,
    )
    .set_name("@membership-original-requested-frame")
    .exec()?;
    Ok(())
}

fn controls(xml: &str) -> Vec<(&'static str, String)> {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let group = fire_group(&doc, "4");
    let gem = group.children().find(|n| n.has_tag_name("Gem")).unwrap();
    let raw = &xml[group.range()];
    let mut removed = xml.to_owned();
    removed.replace_range(group.range(), "");
    let mut duplicate = xml.to_owned();
    duplicate.insert_str(group.range().end, raw);
    let mut reordered = removed.clone();
    let removed_doc = roxmltree::Document::parse(&removed).unwrap();
    let last_group = removed_doc
        .descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some("4"))
        .unwrap()
        .children()
        .rfind(|n| n.has_tag_name("Skill"))
        .unwrap();
    reordered.insert_str(last_group.range().start, raw);
    let item = doc
        .descendants()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some("28"))
        .unwrap();
    let mut item_removed = xml.to_owned();
    item_removed.replace_range(item.range(), "");
    let manual = change_attributes(xml, group, &[("source", None)]);
    let empty = change_attributes(xml, group, &[("source", Some(""))]);
    vec![
        ("remove-generated-firebolt", removed),
        ("reinsert-generated-firebolt-earlier", reordered),
        ("duplicate-generated-firebolt", duplicate),
        ("unequip-granting-staff-leave-saved-group", unequip(xml)),
        ("remove-granting-item-leave-saved-group", item_removed),
        ("manual-firebolt-with-staff", manual.clone()),
        ("manual-firebolt-without-staff", unequip(&manual)),
        ("empty-source-firebolt-with-staff", empty.clone()),
        ("empty-source-firebolt-without-staff", unequip(&empty)),
        (
            "unknown-source-firebolt",
            change_attributes(xml, group, &[("source", Some("Unreviewed:Firebolt"))]),
        ),
        (
            "literal-nil-source-firebolt",
            change_attributes(xml, group, &[("source", Some("nil"))]),
        ),
        (
            "wrong-item-id-source-firebolt",
            change_attributes(
                xml,
                group,
                &[("source", Some("Item:9999:New Item, Ashen Staff"))],
            ),
        ),
        (
            "wrong-item-label-source-firebolt",
            change_attributes(
                xml,
                group,
                &[("source", Some("Item:28:Changed Name, Ashen Staff"))],
            ),
        ),
        (
            "wrong-slot-generated-firebolt",
            change_attributes(xml, group, &[("slot", Some("Weapon 2"))]),
        ),
        (
            "wrong-cached-level-generated-firebolt",
            change_attributes(xml, gem, &[("level", Some("1"))]),
        ),
        (
            "disabled-generated-firebolt-group",
            change_attributes(xml, group, &[("enabled", Some("false"))]),
        ),
        (
            "disabled-generated-firebolt-gem",
            change_attributes(xml, gem, &[("enabled", Some("false"))]),
        ),
        (
            "archived-only-firebolt-edit",
            change_attributes(
                xml,
                fire_group(&doc, "2"),
                &[("source", Some("Unreviewed:ArchivedFirebolt"))],
            ),
        ),
    ]
}

fn fire_group<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    preset: &str,
) -> roxmltree::Node<'a, 'input> {
    doc.descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(preset))
        .unwrap()
        .children()
        .find(|n| {
            n.has_tag_name("Skill")
                && n.children()
                    .any(|c| c.has_tag_name("Gem") && c.attribute("skillId") == Some(FIREBOLT))
        })
        .unwrap()
}

fn unequip(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = doc
        .descendants()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some("2"))
        .unwrap();
    let slot = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Weapon 1"))
        .unwrap();
    assert_eq!(slot.attribute("itemId"), Some("28"));
    change_attributes(xml, slot, &[("itemId", Some("0"))])
}

fn change_attributes(
    xml: &str,
    node: roxmltree::Node<'_, '_>,
    edits: &[(&str, Option<&str>)],
) -> String {
    let start = node.range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let mut text = format!("<{}", node.tag_name().name());
    for attr in node.attributes() {
        if !edits.iter().any(|(name, _)| *name == attr.name()) {
            text.push_str(&format!(" {}=\"{}\"", attr.name(), escape(attr.value())));
        }
    }
    for (name, value) in edits {
        if let Some(value) = value {
            text.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
    }
    if xml[start..end].ends_with("/>") {
        text.push('/');
    }
    text.push('>');
    let mut result = xml.to_owned();
    result.replace_range(start..end, &text);
    result
}

fn check(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 34);
    let baseline = case(report, "original-05");
    assert_eq!(
        baseline["states"],
        case(report, "repeat-original-05")["states"]
    );
    for entry in cases {
        let name = entry["name"].as_str().unwrap();
        for stage in STAGES {
            let state = &entry["states"][stage];
            assert_eq!(state["exact_loader_capture"], true);
            assert_eq!(state["observer_removed_before_evaluation"], true);
            assert_eq!(state["requested_jit_mode_preserved"], true);
            for group in rows(&state["saved_groups"]) {
                assert_eq!(group["loaded_object_exact"], true);
                if group["selected"] == false || group["attributes"].get("source").is_none() {
                    assert_eq!(
                        group["runtime_present"], true,
                        "{name} {stage} authored/archived object"
                    );
                }
                for gem in rows(&group["gems"]) {
                    assert_eq!(gem["loaded_object_exact"], true);
                }
            }
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(state["outputs"][mode]["available"], true);
                for action in rows(&state["actions"][mode]) {
                    assert_eq!(action["actor_is_player"], true);
                }
            }
            if let Some(id) = name.split("-activate-preset-").nth(1) {
                assert_eq!(state["selection"]["skills"], id.parse::<u64>().unwrap());
            }
        }
    }
    for stage in STAGES {
        let base = &baseline["states"][stage];
        assert_eq!(
            base["selection"],
            json!({"skills":4,"items":2,"spec":3,"config":1,"group":3})
        );
        assert_eq!(selected_saved(base).len(), 12);
        assert_eq!(
            selected_saved(base)
                .iter()
                .filter(|g| g["attributes"].get("source").is_none())
                .count(),
            9
        );
        assert_eq!(runtime_fire(base).len(), 1);
        assert_fire_grant(base, true);
        for name in [
            "remove-generated-firebolt",
            "reinsert-generated-firebolt-earlier",
            "duplicate-generated-firebolt",
            "unknown-source-firebolt",
            "literal-nil-source-firebolt",
            "wrong-item-id-source-firebolt",
            "wrong-item-label-source-firebolt",
            "wrong-slot-generated-firebolt",
            "wrong-cached-level-generated-firebolt",
            "disabled-generated-firebolt-group",
            "disabled-generated-firebolt-gem",
            "empty-source-firebolt-with-staff",
        ] {
            let state = &case(report, name)["states"][stage];
            assert_eq!(runtime_fire(state).len(), 1, "{name} {stage}");
            assert_fire_grant(state, true);
            assert_eq!(
                runtime_fire(state)[0]["state"]["fields"]["source"],
                STAFF_SOURCE
            );
        }
        for name in [
            "unequip-granting-staff-leave-saved-group",
            "remove-granting-item-leave-saved-group",
            "empty-source-firebolt-without-staff",
        ] {
            let state = &case(report, name)["states"][stage];
            assert!(runtime_fire(state).is_empty(), "{name} {stage}");
            assert_fire_grant(state, false);
            assert_eq!(saved_fire(state).len(), 1);
            assert_eq!(saved_fire(state)[0]["runtime_present"], false);
        }
        for (name, expected) in [
            ("manual-firebolt-with-staff", 2),
            ("manual-firebolt-without-staff", 1),
        ] {
            let state = &case(report, name)["states"][stage];
            assert_eq!(runtime_fire(state).len(), expected, "{name} {stage}");
            assert_eq!(saved_fire(state).len(), 1);
            assert_eq!(saved_fire(state)[0]["runtime_present"], true);
            assert!(saved_fire(state)[0]["attributes"].get("source").is_none());
            assert_fire_grant(state, expected == 2);
        }
        for name in [
            "empty-source-firebolt-with-staff",
            "empty-source-firebolt-without-staff",
        ] {
            let state = &case(report, name)["states"][stage];
            assert_eq!(saved_fire(state)[0]["attributes"]["source"], "");
            assert_eq!(saved_fire(state)[0]["state"]["fields"]["source"], "");
            assert_eq!(saved_fire(state)[0]["runtime_present"], false);
        }
        for name in [
            "unknown-source-firebolt",
            "literal-nil-source-firebolt",
            "wrong-item-id-source-firebolt",
            "wrong-item-label-source-firebolt",
            "wrong-slot-generated-firebolt",
            "wrong-cached-level-generated-firebolt",
        ] {
            assert_eq!(
                saved_fire(&case(report, name)["states"][stage])[0]["runtime_present"],
                false,
                "{name} {stage}"
            );
        }
        let duplicate = &case(report, "duplicate-generated-firebolt")["states"][stage];
        assert_eq!(saved_fire(duplicate).len(), 2);
        assert_ne!(
            saved_fire(duplicate)[0]["source_ordinal"],
            saved_fire(duplicate)[1]["source_ordinal"]
        );
        assert_eq!(
            saved_fire(duplicate)
                .iter()
                .filter(|g| g["runtime_present"] == true)
                .count(),
            1
        );
        let archived = &case(report, "archived-only-firebolt-edit")["states"][stage];
        assert_eq!(selected_saved(archived), selected_saved(base));
        assert_eq!(runtime_fire(archived), runtime_fire(base));
        for mode in ["MAIN", "CALCS"] {
            assert_eq!(archived["actions"][mode], base["actions"][mode]);
        }
    }
}

fn assert_fire_grant(state: &Json, present: bool) {
    for mode in ["MAIN", "CALCS"] {
        let grants: Vec<_> = rows(&state["granted_skills"][mode])
            .iter()
            .filter(|g| g["fields"]["skillId"] == FIREBOLT)
            .collect();
        assert_eq!(grants.len(), usize::from(present));
        if present {
            assert_eq!(grants[0]["source_item_id"], 28);
            assert_eq!(grants[0]["fields"]["source"], STAFF_SOURCE);
            assert_eq!(rows(&grants[0]["matched_groups"]).len(), 1);
            assert_eq!(
                rows(&grants[0]["matched_groups"])[0]["group_source_item_exact"],
                true
            );
            let group = runtime_fire(state)
                .into_iter()
                .find(|g| g["state"]["fields"]["source"] == STAFF_SOURCE)
                .unwrap();
            assert_eq!(group["state"]["source_item_id"], 28);
            assert_eq!(
                rows(&group["gems"])[0]["state"]["fields"]["level"],
                grants[0]["fields"]["level"]
            );
        }
    }
}

fn case<'a>(report: &'a Json, name: &str) -> &'a Json {
    rows(&report["cases"])
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn selected_saved(state: &Json) -> Vec<&Json> {
    rows(&state["saved_groups"])
        .iter()
        .filter(|g| g["selected"] == true)
        .collect()
}
fn saved_fire(state: &Json) -> Vec<&Json> {
    selected_saved(state)
        .into_iter()
        .filter(|g| {
            rows(&g["gems"])
                .iter()
                .any(|gem| gem["source"]["attributes"]["skillId"] == FIREBOLT)
        })
        .collect()
}
fn runtime_fire(state: &Json) -> Vec<&Json> {
    rows(&state["runtime_groups"])
        .iter()
        .filter(|g| {
            g["selected"] == true
                && rows(&g["gems"])
                    .iter()
                    .any(|gem| gem["state"]["fields"]["skillId"] == FIREBOLT)
        })
        .collect()
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|r| r.is_empty()),
            "expected empty Lua list: {value}"
        );
        &[]
    }
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
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
