//! Independent original Sand property observations, not native build coverage.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "sand_properties_preserve_exact_source_contributors";
const CHILD: &str = "POE_DJINN_PROPERTIES_SOURCE_CHILD";
const OUTPUT: &str = "POE_DJINN_PROPERTIES_SOURCE_OUT";
const OBSERVER: &str = include_str!("support/djinn_properties_source.lua");
const LIFECYCLE: &str = include_str!("support/djinn_provider_source.lua");
const SAND: &str = "SummonSandDjinnPlayer";
const COMMAND: &str = "CommandSandDjinnKnifeThrowPlayer";
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const FILES: [&str; 23] = [
    "src/HeadlessWrapper.lua",
    "src/Modules/Common.lua",
    "src/Modules/Build.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/CompareTab.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Classes/Item.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/ModParser.lua",
    "src/Data/Misc.lua",
    "src/Data/Minions.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/minion.lua",
    "src/Data/Skills/sup_int.lua",
    "src/Data/Gems.lua",
];

#[test]
#[ignore = "requires pinned PoB; observes original Sand properties in independent JIT modes"]
fn sand_properties_preserve_exact_source_contributors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-djinn-properties-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "use a fresh source report directory: {}",
        out.display()
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Sand source property JIT determinism",
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
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
    }
    let one = std::str::from_utf8(&originals[0]).unwrap();
    let five = std::str::from_utf8(&originals[4]).unwrap();
    let mut cases = vec![
        observe(root, "original-01", one, enabled, true, Json::Null),
        observe(root, "original-05", five, enabled, true, Json::Null),
    ];
    let controls = controls(five);
    for (name, xml, control) in &controls {
        cases.push(observe(root, name, xml, enabled, true, control.clone()));
    }
    for (name, xml) in [("repeat-original-01", one), ("repeat-original-05", five)] {
        cases.push(observe(root, name, xml, enabled, true, Json::Null));
    }
    for (name, xml) in [
        ("unhooked-original-01", one),
        ("unhooked-original-05", five),
    ] {
        cases.push(observe(root, name, xml, enabled, false, Json::Null));
    }
    let crown = controls
        .iter()
        .find(|c| c.0 == "crown-minion-level-two")
        .unwrap();
    cases.push(observe(
        root,
        "unhooked-crown-minion-level-two",
        &crown.1,
        enabled,
        false,
        crown.2.clone(),
    ));
    let report = json!({
        "schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),"test_sha256":digest(include_str!("owned_djinn_properties_source.rs").as_bytes()),
        "lifecycle_sha256":digest(LIFECYCLE.as_bytes()),
        "files":FILES.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"diagnostic_queries_as_consumption":false,
        "native_build_parity":false,"native_inventory_authority":false,"fallback_semantics_authority":false,
        "canonical_parity_lifecycle_selected":false,"lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check(&report);
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
}

fn install(lua: &Lua) -> Result<Function, RuntimeError> {
    lua.globals().set("djinnPropertiesPhase", "before")?;
    Ok(lua
        .load(OBSERVER)
        .set_name("@sand-original-property-observer-install")
        .eval()?)
}

fn observe(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    instrumented: bool,
    control: Json,
) -> Json {
    eprintln!(
        "Sand source properties {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("djinnXml", xml)?;
        lua.globals().set("djinnJit", enabled)?;
        lua.globals()
            .set("djinnPropertiesInstrumented", instrumented)?;
        lua.globals().set("djinnOccurrenceNumericEntries", true)?;
        lua.load("if djinnJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let lifecycle: Function = lua
            .load(LIFECYCLE)
            .set_name("@sand-source-lifecycle-authentication")
            .eval()?;
        let observer = install(lua)?;
        let combine: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(combine.call((observer, lifecycle))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_once = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_twice = stage(lua)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
    };
    let scratch = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&before_build),
        Some(&observer),
    )
    .unwrap_or_else(|e| panic!("{name}: complete source failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_hash":observed["source_hash"],"control":control,"states":observed["additional_observation"]})
}

fn occurrence(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("djinnPhase", "observe")?;
    let value: Value = lua
        .load(LIFECYCLE)
        .set_name("@sand-existing-occurrence-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}

fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let before = occurrence(lua)?;
    lua.globals().set("djinnPropertiesPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@sand-original-property-observations")
        .eval()?;
    let properties: Json = lua.from_value(value)?;
    let after = occurrence(lua)?;
    assert_eq!(
        json_evidence::first_difference(&before, &after, "source-preservation"),
        None
    );
    Ok(json!({"occurrences":before,"properties":properties}))
}

fn rebuild(lua: &Lua) -> Result<(), RuntimeError> {
    let cleanup = install(lua)?;
    let result = lua
        .load(
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
        .set_name("@sand-original-frame-rebuild")
        .exec();
    let removed = cleanup.call::<()>(());
    result?;
    removed?;
    Ok(())
}

fn controls(xml: &str) -> Vec<(String, String, Json)> {
    let mut controls = Vec::new();
    for (name, generated, field, value) in [
        ("manual-level-one", false, "level", "1"),
        ("manual-quality-fraction", false, "quality", "12.5"),
        ("manual-quality-negative", false, "quality", "-1"),
        ("manual-quality-above-hundred", false, "quality", "101.5"),
        ("tree-quality-fraction", true, "quality", "12.5"),
        ("manual-level-fraction-diagnostic", false, "level", "1.5"),
        ("manual-level-above-domain-diagnostic", false, "level", "41"),
        ("manual-level-domain-max", false, "level", "40"),
    ] {
        let changed = edit_selected_gem(xml, generated, SAND, field, value);
        controls.push((
            name.into(),
            changed,
            json!({"source":"selected Sand","generated":generated,"field":field,"value":value}),
        ));
    }
    controls.push((
        "disable-sand-bidding".into(),
        edit_selected_gem(xml, false, "SupportBiddingPlayerTwo", "enabled", "false"),
        json!({"source":"selected manual Sand Bidding II","enabled":false}),
    ));
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc.descendants().find(|n| n.has_tag_name("Tree")).unwrap();
    let active: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap();
    let nodes: Vec<_> = spec.attribute("nodes").unwrap().split(',').collect();
    assert_eq!(nodes.iter().filter(|n| **n == "13289").count(), 1);
    let retained = nodes
        .into_iter()
        .filter(|n| *n != "13289")
        .collect::<Vec<_>>()
        .join(",");
    controls.push((
        "remove-sand-allocation".into(),
        replace_attribute(xml, spec, "nodes", &retained),
        json!({"source":"selected tree","removed_node":13289}),
    ));
    let item = doc
        .descendants()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some("21"))
        .unwrap();
    let text = &xml[item.range()];
    assert_eq!(text.matches("+1 to Level of all Minion Skills").count(), 1);
    let mut changed = xml.to_owned();
    changed.replace_range(
        item.range(),
        &text.replace(
            "+1 to Level of all Minion Skills",
            "+2 to Level of all Minion Skills",
        ),
    );
    controls.push(("crown-minion-level-two".into(), changed, json!({"item":21,"before":"+1 to Level of all Minion Skills","after":"+2 to Level of all Minion Skills"})));
    for (_, changed, _) in &controls {
        assert_ne!(changed, xml);
    }
    assert_eq!(controls.len(), 11);
    controls
}

fn edit_selected_gem(xml: &str, generated: bool, effect: &str, field: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    let set = skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(selected))
        .unwrap();
    let groups: Vec<_> = set
        .children()
        .filter(|n| {
            n.has_tag_name("Skill")
                && (n.attribute("source") == Some("Tree:13289")) == generated
                && (generated || n.attribute("source").is_none())
                && n.children()
                    .any(|g| g.has_tag_name("Gem") && g.attribute("skillId") == Some(SAND))
        })
        .collect();
    assert_eq!(groups.len(), 1);
    let gems: Vec<_> = groups[0]
        .children()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(effect))
        .collect();
    assert_eq!(gems.len(), 1);
    replace_attribute(xml, gems[0], field, value)
}

fn replace_attribute(xml: &str, node: roxmltree::Node<'_, '_>, field: &str, value: &str) -> String {
    let attr = node.attributes().find(|a| a.name() == field).unwrap();
    let escaped = value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;");
    let mut output = xml.to_owned();
    output.replace_range(attr.range_value(), &escaped);
    output
}

fn rows(value: &Json) -> &[Json] {
    if value.as_object().is_some_and(serde_json::Map::is_empty) {
        &[]
    } else {
        value.as_array().expect("source sequence")
    }
}
fn case<'a>(report: &'a Json, name: &str) -> &'a Json {
    rows(&report["cases"])
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn roots(properties: &Json) -> impl Iterator<Item = &Json> {
    rows(&properties["contexts"])
        .iter()
        .filter(|r| r["effect"] == SAND || r["effect"] == COMMAND)
}
fn eq_json(a: &Json, b: &Json, context: &str) {
    assert_eq!(json_evidence::first_difference(a, b, context), None);
}

fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 18);
    for c in rows(&report["cases"]) {
        let name = c["name"].as_str().unwrap();
        let instrumented = !name.starts_with("unhooked-");
        for stage in STAGES {
            let p = &c["states"][stage]["properties"];
            assert_eq!(p["instrumented"], instrumented);
            for flag in [
                "original_methods_preserved",
                "hook_removed",
                "jit_mode_preserved",
            ] {
                assert_eq!(p[flag], true, "{name}/{stage}/{flag}");
            }
            assert_eq!(p["business_wrappers"], false);
            let table = rows(&p["minion_level_table"]);
            assert_eq!(table.len(), 40);
            for (i, row) in table.iter().enumerate() {
                assert_eq!(row["input"], i + 1);
                assert_eq!(row["output"], (i + 1) * 2);
            }
            let expected_roots = if name == "remove-sand-allocation" {
                4
            } else {
                8
            };
            assert_eq!(roots(p).count(), expected_roots, "{name}/{stage}");
            for row in roots(p) {
                assert_eq!(row["source_instance_present"], true);
                assert_eq!(row["catalog_present"], true);
                assert_eq!(row["catalog"]["tags"]["minion"], true);
                assert_eq!(row["source_cache_present"], true);
                let tree = row["group"]["group_source"] == "Tree:13289";
                assert_eq!(row["group"]["no_supports"], tree);
                if instrumented {
                    let ordinary = rows(&row["ordinary"]);
                    assert_eq!(ordinary.len(), 1, "{name}/{stage}: ordinary calls");
                    assert_eq!(ordinary[0]["query_store_is_actor"], true);
                    let mut sum_level = 0.0;
                    let mut sum_quality = 0.0;
                    for index in rows(&ordinary[0]["matched"]) {
                        let candidate =
                            &ordinary[0]["candidates"][index.as_u64().unwrap() as usize - 1];
                        assert_eq!(candidate["mod"]["name"], "GemProperty");
                        assert!(!candidate["mod"]["source"].as_str().unwrap().is_empty());
                        let value = candidate["value"]["value"].as_f64().unwrap();
                        match candidate["value"]["key"].as_str().unwrap() {
                            "level" => sum_level += value,
                            "quality" => sum_quality += value,
                            other => panic!("unreviewed property {other}"),
                        }
                    }
                    assert_eq!(
                        ordinary[0]["after"]["level"].as_f64().unwrap(),
                        ordinary[0]["before"]["level"].as_f64().unwrap() + sum_level
                    );
                    assert_eq!(
                        ordinary[0]["after"]["quality"].as_f64().unwrap(),
                        ordinary[0]["before"]["quality"].as_f64().unwrap() + sum_quality
                    );
                    let supported = rows(&row["supported"]);
                    assert_eq!(supported.len(), 1);
                    assert_eq!(supported[0]["query_parent_is_actor"], true);
                    assert_eq!(supported[0]["cache_identity_preserved"], true);
                    eq_json(
                        &supported[0]["before"],
                        &supported[0]["after"],
                        "property retrieval does not apply properties",
                    );
                    if tree {
                        assert!(rows(&supported[0]["supports"]).is_empty());
                    }
                    assert_eq!(rows(&row["assembly"]).len(), 1);
                    assert_eq!(rows(&row["assembly"][0]["merges"]).len(), 1);
                    assert_eq!(row["assembly"][0]["merges"][0]["exact_destination"], true);
                }
                if row["effect"] == SAND {
                    let population = &row["population"];
                    assert_eq!(population["present"], true);
                    assert_eq!(population["level"], population["table_output"]);
                    for flag in population["overrides"].as_object().unwrap().values() {
                        assert_eq!(flag["present"], false);
                    }
                    assert_eq!(rows(&population["children"]).len(), 3);
                    for child in rows(&population["children"]) {
                        assert_eq!(child["input"]["level"], 1);
                        assert_eq!(child["input"]["quality"], 0);
                        assert_eq!(child["actor_level"], population["level"]);
                        assert_eq!(child["source_instance_present"], false);
                        assert_eq!(child["catalog_present"], false);
                        assert_eq!(child["exact_parent"], true);
                        assert_eq!(child["exact_actor"], true);
                    }
                }
            }
            for row in rows(&p["contexts"])
                .iter()
                .filter(|r| r["parent_present"] == true)
            {
                assert_eq!(row["source_instance_present"], false);
                assert!(rows(&row["ordinary"]).is_empty());
                for retrieval in rows(&row["supported"]) {
                    assert_eq!(retrieval["source_instance_present"], false);
                    assert!(rows(&retrieval["properties"]).is_empty());
                }
            }
        }
        // Three fixed lifecycle observations; no retries or selection of a warm result.
        eq_json(
            &c["states"]["fresh"]["properties"]["outcomes"],
            &c["states"]["rebuilt_once"]["properties"]["outcomes"],
            name,
        );
        eq_json(
            &c["states"]["fresh"]["properties"]["outcomes"],
            &c["states"]["rebuilt_twice"]["properties"]["outcomes"],
            name,
        );
    }
    for (original, alternate) in [
        ("original-01", "repeat-original-01"),
        ("original-05", "repeat-original-05"),
        ("original-01", "unhooked-original-01"),
        ("original-05", "unhooked-original-05"),
        ("crown-minion-level-two", "unhooked-crown-minion-level-two"),
    ] {
        for stage in STAGES {
            eq_json(
                &case(report, original)["states"][stage]["occurrences"],
                &case(report, alternate)["states"][stage]["occurrences"],
                alternate,
            );
            eq_json(
                &case(report, original)["states"][stage]["properties"]["outcomes"],
                &case(report, alternate)["states"][stage]["properties"]["outcomes"],
                alternate,
            );
        }
    }
    for (name, manual_level, tree_level, quality_manual, quality_tree) in [
        ("original-01", 31, 12, 0.0, 0.0),
        ("original-05", 22, 3, 0.0, 0.0),
        ("manual-level-one", 3, 3, 0.0, 0.0),
        ("manual-quality-fraction", 22, 3, 12.5, 0.0),
        ("manual-quality-negative", 22, 3, -1.0, 0.0),
        ("manual-quality-above-hundred", 22, 3, 101.5, 0.0),
        ("tree-quality-fraction", 22, 3, 0.0, 12.5),
        ("disable-sand-bidding", 22, 3, 0.0, 0.0),
        ("crown-minion-level-two", 23, 4, 0.0, 0.0),
        ("manual-level-fraction-diagnostic", 22, 3, 0.0, 0.0),
        ("manual-level-above-domain-diagnostic", 40, 3, 0.0, 0.0),
        ("manual-level-domain-max", 40, 3, 0.0, 0.0),
    ] {
        for stage in STAGES {
            for r in roots(&case(report, name)["states"][stage]["properties"]) {
                let tree = r["group"]["group_source"] == "Tree:13289";
                assert_eq!(
                    r["final"]["level"],
                    if tree { tree_level } else { manual_level },
                    "{name}/{stage}"
                );
                assert_eq!(
                    r["final"]["quality"].as_f64().unwrap(),
                    if tree { quality_tree } else { quality_manual }
                );
            }
        }
    }
    for (name, loaded, processed) in [
        ("manual-level-one", 1.0, 1.0),
        ("manual-level-fraction-diagnostic", 1.5, 20.0),
        ("manual-level-above-domain-diagnostic", 41.0, 40.0),
        ("manual-level-domain-max", 40.0, 40.0),
    ] {
        for r in roots(&case(report, name)["states"]["fresh"]["properties"])
            .filter(|r| r["group"]["group_source"].is_null())
        {
            assert_eq!(r["loaded"]["input"]["level"].as_f64().unwrap(), loaded);
            assert_eq!(r["loaded"]["same_source"], true);
            assert_eq!(r["raw"]["level"].as_f64().unwrap(), processed);
        }
    }
    for (name, expected) in [
        ("original-01", vec![(1, 2.0), (6, 3.0), (8, 2.0), (9, 4.0)]),
        ("original-05", vec![(21, 1.0), (23, 1.0)]),
    ] {
        for stage in STAGES {
            for r in roots(&case(report, name)["states"][stage]["properties"]) {
                let ordinary = &r["ordinary"][0];
                let mut actual = BTreeMap::new();
                for i in rows(&ordinary["matched"]) {
                    let c = &ordinary["candidates"][i.as_u64().unwrap() as usize - 1];
                    if c["value"]["key"] != "level" {
                        continue;
                    }
                    assert_eq!(c["value"]["keyword"], "minion");
                    let amount = c["value"]["value"].as_f64().unwrap();
                    if amount > 0.0 {
                        let source = c["mod"]["source"].as_str().unwrap();
                        let parts: Vec<_> = source.splitn(3, ':').collect();
                        assert_eq!(parts[0], "Item");
                        let id: u32 = parts[1].parse().unwrap();
                        assert!(actual.insert(id, amount).is_none());
                    }
                }
                assert_eq!(
                    actual,
                    expected.iter().copied().collect(),
                    "{name}/{stage}: actual ordinary origins"
                );
                assert!(
                    rows(&r["supported"][0]["properties"]).is_empty(),
                    "unchanged originals have no supported level/quality properties"
                );
            }
        }
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .rev()
        .take(45)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}
