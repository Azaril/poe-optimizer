//! Optional original Full DPS contribution attribution; no native parity authority.
#![cfg(not(target_arch = "wasm32"))]
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
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_full_dps_attribution_observes_original_contributors";
const CHILD: &str = "POE_FULL_DPS_ATTRIBUTION_CHILD";
const DIAGNOSTIC_CASE: &str = "POE_FULL_DPS_ATTRIBUTION_CASE";
const SAVED_REPORT: &str = "POE_FULL_DPS_ATTRIBUTION_REPORT";
const OBSERVER: &str = include_str!("support/full_dps_attribution_source.lua");
const LIFECYCLE: &str = include_str!("support/djinn_provider_source.lua");
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const SOURCES: [(&str, &str, Option<&str>); 5] = [
    (
        "generated-sand",
        "SummonSandDjinnPlayer",
        Some("Tree:13289"),
    ),
    (
        "generated-water",
        "SummonWaterDjinnPlayer",
        Some("Tree:32705"),
    ),
    (
        "generated-firebolt",
        "FireboltPlayer",
        Some("Item:28:New Item, Ashen Staff"),
    ),
    ("manual-sand", "SummonSandDjinnPlayer", None),
    ("manual-water", "SummonWaterDjinnPlayer", None),
];
const FILES: [&str; 17] = [
    "src/HeadlessWrapper.lua",
    "src/Modules/Common.lua",
    "src/Modules/Build.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/CompareTab.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Modules/Data.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/CalcPerform.lua",
    "src/Data/Gems.lua",
    "src/Data/Skills/act_int.lua",
];

#[test]
#[ignore = "requires complete pinned PoB; observed reporting contributions do not certify native usage or numerical coverage"]
fn complete_full_dps_attribution_observes_original_contributors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let diagnostic = diagnostic_case();
    let out = if let Some(name) = &diagnostic {
        root.join("runs/owned-full-dps-attribution-diagnostic")
            .join(name)
    } else {
        root.join("runs/owned-full-dps-attribution-source-03")
    };
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
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
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(900) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "Full DPS attribution deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "observed Full DPS contributions must agree across JIT modes"
    );
}

#[test]
#[ignore = "read-only validation of a supplied captured report; does not execute PoB or establish a fresh/JIT-parity result"]
fn validate_saved_full_dps_attribution_report() {
    let path =
        PathBuf::from(std::env::var_os(SAVED_REPORT).expect("set POE_FULL_DPS_ATTRIBUTION_REPORT"));
    const MAX_BYTES: u64 = 32 * 1024 * 1024;
    assert!(fs::metadata(&path).unwrap().len() <= MAX_BYTES);
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.len() as u64 <= MAX_BYTES);
    let report: Json = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    check(&report, true);
    eprintln!(
        "saved-report assertions passed: {} bytes, SHA256 {}; no fresh runtime or cross-JIT claim; final display identity {}",
        bytes.len(),
        digest(&bytes),
        if report["attribution_schema_version"] == 2 {
            "present and checked"
        } else {
            "absent in historical report; only the displayed-value permutation is checked"
        }
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
    let diagnostic = diagnostic_case();
    let selected = |name: &str| diagnostic.as_deref().is_none_or(|wanted| wanted == name);
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        let name = format!("original-{:02}", i + 1);
        if selected(&name) {
            cases.push(observe(
                root,
                &name,
                std::str::from_utf8(bytes).unwrap(),
                enabled,
            ));
        }
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    for (name, changed) in controls(xml) {
        assert_ne!(changed, xml);
        if selected(&name) {
            cases.push(observe(root, &name, &changed, enabled));
        }
    }
    if selected("repeat-original-05") {
        cases.push(observe(root, "repeat-original-05", xml, enabled));
    }
    assert!(!cases.is_empty(), "unknown diagnostic source case");
    let report = json!({"attribution_schema_version":2,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "business_wrappers":false,"formulas_reimplemented":false,"native_build_parity":false,
        "native_inventory_authority":false,"canonical_parity_lifecycle_selected":false,
        "files":FILES.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "diagnostic_case":diagnostic,"complete_case_inventory":diagnostic.is_none(),"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 32 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check(&report, false);
    for (i, b) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            b
        );
    }
}

fn install(lua: &Lua) -> Result<Function, RuntimeError> {
    lua.globals().set("fullDpsPhase", "install")?;
    Ok(lua
        .load(OBSERVER)
        .set_name("@full-dps-original-contribution-observer")
        .eval()?)
}
fn observe(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    eprintln!(
        "Full DPS contribution case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let measured = run(root, xml, enabled, true);
    let unobserved = run(root, xml, enabled, false);
    for stage in STAGES {
        assert!(
            measured[stage]["outputs"] == unobserved[stage]["outputs"],
            "observer changed outputs: {name} {stage}"
        );
        assert!(
            measured[stage]["selection"] == unobserved[stage]["selection"],
            "observer changed selection: {name} {stage}"
        );
    }
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([107; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    for stage in STAGES {
        let expected_groups: BTreeSet<_> = evidence
            .rows()
            .iter()
            .filter(|r| r.occurrence().name() == "Skill")
            .map(|r| u64::from(r.occurrence().id().ordinal()))
            .collect();
        let loaded = rows(&measured[stage]["loaded_sources"]);
        let observed_groups: BTreeSet<_> = loaded
            .iter()
            .map(|r| r["source_ordinal"].as_u64().unwrap())
            .collect();
        assert_eq!(
            loaded.len(),
            observed_groups.len(),
            "duplicate source group"
        );
        assert_eq!(
            expected_groups, observed_groups,
            "incomplete loaded group census: {name} {stage}"
        );
        for group in rows(&measured[stage]["loaded_sources"]) {
            let group_row = &evidence.rows()[group["source_ordinal"].as_u64().unwrap() as usize];
            let expected_gems: BTreeSet<_> = group_row
                .children()
                .iter()
                .filter(|id| evidence.rows()[id.ordinal() as usize].occurrence().name() == "Gem")
                .map(|id| u64::from(id.ordinal()))
                .collect();
            let observed_gems: BTreeSet<_> = rows(&group["gems"])
                .iter()
                .map(|r| r["source_ordinal"].as_u64().unwrap())
                .collect();
            assert_eq!(
                rows(&group["gems"]).len(),
                observed_gems.len(),
                "duplicate source Gem"
            );
            assert_eq!(
                expected_gems, observed_gems,
                "incomplete loaded Gem census: {name} {stage}"
            );
            for source in std::iter::once(group).chain(rows(&group["gems"]).iter()) {
                let ordinal = source["source_ordinal"].as_u64().unwrap() as usize;
                let row = &evidence.rows()[ordinal];
                assert_eq!(
                    row.attributes().len(),
                    source["attributes"].as_object().unwrap().len()
                );
                for a in row.attributes() {
                    assert_eq!(source["attributes"][&a.origin().name], a.decoded().unwrap());
                }
            }
        }
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),
        "unobserved_outputs_equal":true,"unobserved_selections_equal":true,"states":measured})
}
fn run(root: &Path, xml: &str, enabled: bool, instrument: bool) -> Json {
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("fullDpsXml", xml)?;
        lua.globals().set("fullDpsJit", enabled)?;
        lua.load("if fullDpsJit then jit.on()else jit.off();jit.flush()end")
            .exec()?;
        Ok(())
    };
    let setup = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let auth: Function = lua
            .load(LIFECYCLE)
            .set_name("@full-dps-source-lifecycle")
            .eval()?;
        if !instrument {
            return Ok(auth);
        }
        let observer = install(lua)?;
        let compose:Function=lua.load("return function(observer,auth)return function()local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(compose.call((observer, auth))?)
    };
    let observation = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua, instrument)?;
        rebuild(lua, instrument)?;
        let rebuilt_once = stage(lua, instrument)?;
        rebuild(lua, instrument)?;
        let rebuilt_twice = stage(lua, instrument)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
    };
    let scratch = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&setup),
        Some(&observation),
    )
    .unwrap_or_else(|e| panic!("complete Full DPS source lifecycle failed: {e}"));
    assert_eq!(result["configuration_method_wrappers"], false);
    result["additional_observation"].clone()
}
fn stage(lua: &Lua, instrument: bool) -> Result<Json, RuntimeError> {
    lua.globals().set(
        "fullDpsPhase",
        if instrument { "observe" } else { "output_only" },
    )?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@full-dps-original-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn rebuild(lua: &Lua, instrument: bool) -> Result<(), RuntimeError> {
    let cleanup = if instrument {
        Some(install(lua)?)
    } else {
        None
    };
    let result = lua
        .load(
            r#"
local callback=runCallback;local frame=build.OnFrame;local output=build.calcsTab.BuildOutput
assert(debug.getinfo(callback,"S").linedefined==17 and debug.getinfo(frame,"S").linedefined==1285)
assert(output==djinnOriginals.refs.calcs_tab_output and not build.buildFlag)
local revision=build.outputRevision;local main,calcs=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(not build.buildFlag and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calcs)
assert(callback==runCallback and frame==build.OnFrame and output==build.calcsTab.BuildOutput)
"#,
        )
        .set_name("@full-dps-original-frame-rebuild")
        .exec();
    let removed = cleanup.map(|f| f.call::<()>(())).transpose();
    result?;
    removed?;
    Ok(())
}

fn group<'a, 'i>(
    doc: &'a roxmltree::Document<'i>,
    effect: &str,
    source: Option<&str>,
) -> roxmltree::Node<'a, 'i> {
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let set = skills
        .children()
        .find(|n| {
            n.has_tag_name("SkillSet") && n.attribute("id") == skills.attribute("activeSkillSet")
        })
        .unwrap();
    let matched: Vec<_> = set
        .children()
        .filter(|n| {
            n.has_tag_name("Skill")
                && n.attribute("source") == source
                && n.children()
                    .any(|c| c.has_tag_name("Gem") && c.attribute("skillId") == Some(effect))
        })
        .collect();
    assert_eq!(matched.len(), 1);
    matched[0]
}
fn change(xml: &str, node: roxmltree::Node<'_, '_>, changes: &[(&str, Option<&str>)]) -> String {
    let start = node.range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let escape = |value: &str| {
        value
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let mut head = format!("<{}", node.tag_name().name());
    for a in node.attributes() {
        if !changes.iter().any(|(name, _)| *name == a.name()) {
            head.push_str(&format!(" {}=\"{}\"", a.name(), escape(a.value())));
        }
    }
    for (name, value) in changes {
        if let Some(value) = value {
            head.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
    }
    if xml[start..end].ends_with("/>") {
        head.push('/');
    }
    head.push('>');
    let mut result = xml.to_owned();
    result.replace_range(start..end, &head);
    result
}
fn controls(xml: &str) -> Vec<(String, String)> {
    let mut result = vec![];
    for (name, effect, source) in SOURCES {
        for (control, count, override_count, included, disable_source, disable_group) in [
            ("count-one", "1", None, true, false, false),
            ("count-three", "3", None, true, false, false),
            ("group-zero", "3", Some("0"), true, false, false),
            ("group-four", "3", Some("4"), true, false, false),
            ("excluded", "3", None, false, false, false),
            ("disabled-source", "3", None, true, true, false),
            ("disabled-group", "3", None, true, false, true),
        ] {
            let doc = roxmltree::Document::parse(xml).unwrap();
            let gem = group(&doc, effect, source)
                .children()
                .find(|n| n.has_tag_name("Gem"))
                .unwrap();
            let changed = change(
                xml,
                gem,
                &[
                    ("count", Some(count)),
                    (
                        "enabled",
                        Some(if disable_source { "false" } else { "true" }),
                    ),
                ],
            );
            let doc = roxmltree::Document::parse(&changed).unwrap();
            let g = group(&doc, effect, source);
            let changed = change(
                &changed,
                g,
                &[
                    (
                        "includeInFullDPS",
                        Some(if included { "true" } else { "false" }),
                    ),
                    ("groupCount", override_count),
                    (
                        "enabled",
                        Some(if disable_group { "false" } else { "true" }),
                    ),
                ],
            );
            result.push((format!("{name}-{control}"), changed));
        }
    }
    for (family, effect, node) in [
        ("sand", SOURCES[0].1, SOURCES[0].2),
        ("water", SOURCES[1].1, SOURCES[1].2),
    ] {
        let mut changed = xml.to_owned();
        for (source, count) in [(node, "1"), (None, "3")] {
            let doc = roxmltree::Document::parse(&changed).unwrap();
            let gem = group(&doc, effect, source)
                .children()
                .find(|n| n.has_tag_name("Gem"))
                .unwrap();
            changed = change(&changed, gem, &[("count", Some(count))]);
            let doc = roxmltree::Document::parse(&changed).unwrap();
            changed = change(
                &changed,
                group(&doc, effect, source),
                &[("includeInFullDPS", Some("true"))],
            );
        }
        result.push((format!("manual-and-generated-{family}"), changed));
    }
    // Two distinct but equivalent manual sources exercise a positive max tie;
    // their equal labels are explicitly insufficient for attribution.
    let doc = roxmltree::Document::parse(xml).unwrap();
    let g = group(&doc, SOURCES[2].1, SOURCES[2].2);
    let raw = &xml[g.range()];
    let raw_doc = roxmltree::Document::parse(raw).unwrap();
    let manual = change(
        raw,
        raw_doc.root_element(),
        &[("source", None), ("includeInFullDPS", Some("true"))],
    );
    let mut changed = xml.to_owned();
    changed.insert_str(g.range().end, &format!("{manual}{manual}"));
    result.push(("same-label-manual-firebolt-tie".into(), changed));
    result
}
fn rows(value: &Json) -> &[Json] {
    value.as_array().map(Vec::as_slice).unwrap_or_else(|| {
        assert!(value.is_object() && value.as_object().unwrap().is_empty());
        &[]
    })
}
fn diagnostic_case() -> Option<String> {
    std::env::var(DIAGNOSTIC_CASE).ok().inspect(|name| {
        assert!(
            !name.is_empty()
                && name.len() <= 80
                && name
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
            "invalid diagnostic source case"
        );
    })
}
fn actor_reference<'a>(call: &'a Json, reference: &Json) -> &'a Json {
    assert_eq!(reference.as_object().unwrap().len(), 2);
    let pass = usize::try_from(reference["pass"].as_u64().unwrap())
        .unwrap()
        .checked_sub(1)
        .unwrap();
    let actor = usize::try_from(reference["actor"].as_u64().unwrap())
        .unwrap()
        .checked_sub(1)
        .unwrap();
    &rows(&rows(&call["passes"])[pass]["actors"])[actor]
}
fn maximum_field(field: &str) -> Option<&'static str> {
    match field {
        "bleedDPS" => Some("BleedDPS"),
        "corruptingBloodDPS" => Some("CorruptingBloodDPS"),
        "igniteDPS" => Some("IgniteDPS"),
        "burningGroundDPS" => Some("BurningGroundDPS"),
        "poisonDPS" => Some("PoisonDPS"),
        "causticGroundDPS" => Some("CausticGroundDPS"),
        "cullingMulti" => Some("CullMultiplier"),
        _ => None,
    }
}
/// Check observed references and actual values without recomputing a damage formula.
fn check_call_integrity(call: &Json) {
    for pass in rows(&call["passes"]) {
        assert!(pass["source"]["effect"].as_str().is_some());
        assert_eq!(pass["source"]["stat_set_declaration_exact"], true);
        assert!(!rows(&pass["actors"]).is_empty());
        for actor in rows(&pass["actors"]) {
            let identity = &actor["identity"];
            assert_eq!(identity["exact_output_object"], true);
            assert!(
                identity["source"] == pass["source"],
                "harvested actor points to another source pass"
            );
            assert_eq!(identity["action"]["stat_set_declaration_exact"], true);
            match identity["kind"].as_str().unwrap() {
                "player" => {
                    assert_eq!(identity["source_skill_is_actor_main"], true);
                    assert_eq!(identity["action"]["actor_is_player"], true);
                    assert!(identity["action"] == pass["source"]);
                }
                "minion" => {
                    assert_eq!(identity["actor_is_source_minion"], true);
                    assert_eq!(identity["action"]["actor_is_player"], false);
                }
                "mirage" => assert_eq!(identity["action"]["actor_is_player"], false),
                other => panic!("unreviewed harvested actor role {other}"),
            }
        }
    }
    let mutations = rows(&call["mutations"]);
    let mut replacements = BTreeMap::new();
    let mut attributed: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, mutation) in mutations.iter().enumerate() {
        let field = mutation["field"].as_str().unwrap();
        assert_ne!(mutation["before"], mutation["after"]);
        if !mutation["actor"].is_null() {
            let actor = actor_reference(call, &mutation["actor"]);
            attributed.entry(field).or_default().push(index + 1);
            if matches!(mutation["line"].as_u64(), Some(182 | 190)) {
                let source_field =
                    maximum_field(field).expect("maximum write has a reviewed output field");
                assert_eq!(actor["values"][source_field], mutation["after"]);
                replacements.insert(field, &mutation["actor"]);
            }
        }
        // Parent-frame deltas (e.g. line373 after a merge) describe the same
        // update at another stack boundary. Only actor-bound events contribute.
    }
    let contributors = call["contributors"].as_object().unwrap();
    assert_eq!(contributors.len(), attributed.len());
    for (field, expected) in attributed {
        let actual: Vec<_> = rows(&contributors[field])
            .iter()
            .map(|v| usize::try_from(v.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(
            actual, expected,
            "missing, duplicate or parent-frame contributor event"
        );
    }
    let winners = call["winners"].as_object().unwrap();
    assert_eq!(winners.len(), replacements.len());
    for (field, expected) in replacements {
        assert_eq!(&winners[field], expected);
        actor_reference(call, &winners[field]);
    }
    for comparison in rows(&call["comparisons"]) {
        let actor = actor_reference(call, &comparison["actor"]);
        let field = comparison["field"].as_str().unwrap();
        assert_eq!(
            actor["values"][maximum_field(field).unwrap()],
            comparison["candidate"]
        );
        let writes: Vec<_> = mutations
            .iter()
            .filter(|m| {
                m["line"] == 182 && m["field"] == field && m["actor"] == comparison["actor"]
            })
            .collect();
        if comparison["replacement_entered"] == true {
            assert_eq!(writes.len(), 1);
            assert_eq!(writes[0]["before"], comparison["prior"]);
            assert_eq!(writes[0]["after"], comparison["candidate"]);
        } else {
            assert_eq!(comparison["replacement_entered"], false);
            assert!(
                writes.is_empty(),
                "unentered maximum branch acquired a write"
            );
        }
    }
    let final_rows = rows(&call["result"]["skills"]);
    assert_eq!(rows(&call["rows"]).len(), final_rows.len());
    for (index, row) in rows(&call["rows"]).iter().enumerate() {
        assert_eq!(row["index"], index + 1);
        assert!(
            row["value"] == final_rows[index],
            "recorded row differs from its actual final array position"
        );
        match row["attribution"].as_str().unwrap() {
            "exact_harvested_actor" => {
                let actor = actor_reference(call, &row["actor"]);
                assert_eq!(row["value"]["dps"], actor["values"]["TotalDPS"]);
                assert_eq!(row["value"]["count"], actor["count"]);
                assert_eq!(row["line"], 174);
            }
            "observed_aggregate_write" => {
                let field = row["aggregate_field"].as_str().unwrap();
                assert_eq!(row["value"]["dps"], call["result"][field]);
                assert_eq!(row["value"]["count"], 1);
                if maximum_field(field).is_some() {
                    assert!(row["winner"].is_object());
                    assert_eq!(row["winner"], call["winners"][field]);
                    actor_reference(call, &row["winner"]);
                }
                let expected = contributors.get(field).map(rows).unwrap_or(&[]);
                assert_eq!(rows(&row["contributors"]), expected);
            }
            other => panic!("unresolved reporting-row attribution {other}"),
        }
    }
}
fn matching_source<'a>(
    state: &'a Json,
    primary: &str,
    source: Option<&str>,
) -> (&'a Json, &'a Json) {
    let matched: Vec<_> = rows(&state["loaded_sources"])
        .iter()
        .filter(|g| {
            g["preset"] == state["selection"]["skills"]
                && g["attributes"]["source"].as_str() == source
                && rows(&g["gems"])
                    .iter()
                    .any(|gem| gem["attributes"]["skillId"] == primary)
        })
        .collect();
    assert_eq!(matched.len(), 1, "exact control source");
    let gems: Vec<_> = rows(&matched[0]["gems"])
        .iter()
        .filter(|g| g["attributes"]["skillId"] == primary)
        .collect();
    assert_eq!(gems.len(), 1);
    (matched[0], gems[0])
}
fn check_source_passes(
    call: &Json,
    state: &Json,
    source_index: usize,
    count: i64,
) -> BTreeSet<u64> {
    let (_, primary, provider) = SOURCES[source_index];
    let (group, gem) = matching_source(state, primary, provider);
    let (command, child) = match source_index {
        0 => (
            Some("CommandSandDjinnKnifeThrowPlayer"),
            Some("ExplosiveTeleportSandDjinn"),
        ),
        1 | 4 => (
            Some("CommandWaterDjinnBubblePlayer"),
            Some("WaterBubbleWaterDjinn"),
        ),
        2 => (None, None),
        3 => (
            Some("CommandSandDjinnKnifeThrowPlayer"),
            Some("KnifeThrowSandDjinn"),
        ),
        _ => unreachable!(),
    };
    let expected: BTreeSet<_> = std::iter::once(primary).chain(command).collect();
    let passes: Vec<_> = rows(&call["passes"])
        .iter()
        .filter(|p| p["source"]["source_ordinal"] == gem["source_ordinal"])
        .collect();
    assert_eq!(passes.len(), expected.len());
    let actual: BTreeSet<_> = passes
        .iter()
        .map(|p| p["source"]["effect"].as_str().unwrap())
        .collect();
    assert_eq!(actual, expected, "missing or duplicate supplied effect");
    for pass in passes {
        assert_eq!(
            pass["source"]["group_source_ordinal"],
            group["source_ordinal"]
        );
        assert_eq!(pass["source"]["preset"], group["preset"]);
        assert_eq!(pass["source"]["source_object_joined"], true);
        assert_eq!(pass["source"]["group_object_joined"], true);
        assert_eq!(pass["source"]["source"].as_str(), provider);
        let actors = rows(&pass["actors"]);
        let expected_child = if pass["source"]["effect"] == primary {
            child
        } else {
            None
        };
        assert_eq!(actors.len(), if expected_child.is_some() { 2 } else { 1 });
        let roles: Vec<_> = actors
            .iter()
            .map(|a| a["identity"]["kind"].as_str().unwrap())
            .collect();
        assert_eq!(
            roles,
            if expected_child.is_some() {
                vec!["minion", "player"]
            } else {
                vec!["player"]
            }
        );
        if let Some(child) = expected_child {
            assert_eq!(actors[0]["identity"]["action"]["effect"], child);
        }
        for actor in actors {
            assert_eq!(actor["count"], count);
        }
    }
    BTreeSet::from([gem["source_ordinal"].as_u64().unwrap()])
}
fn check_display_order(call: &Json, output: &Json, historical: bool, context: &str) {
    let calculated = rows(&call["result"]["skills"]);
    let displayed = rows(&output["skill_dps"]);
    if historical {
        // Old evidence predates post-display pointer capture. It can prove the
        // row values survive an ordering change, but cannot attribute display
        // positions to equal-valued source objects. Never use this live.
        let mut before: Vec<_> = calculated
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();
        let mut after: Vec<_> = displayed
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();
        before.sort_unstable();
        after.sort_unstable();
        assert_eq!(before, after, "historical report values changed: {context}");
        return;
    }
    assert_eq!(call["final_array_object_exact"], true, "{context}");
    let final_order = rows(&call["final_order"]);
    assert_eq!(final_order.len(), calculated.len(), "{context}");
    assert_eq!(final_order.len(), displayed.len(), "{context}");
    let mut used = BTreeSet::new();
    for (index, row) in final_order.iter().enumerate() {
        assert_eq!(row["output_index"], index + 1, "{context}");
        assert_eq!(row["exact_row_object"], true, "{context}");
        let original = usize::try_from(row["calculation_index"].as_u64().unwrap()).unwrap();
        assert!(
            original >= 1 && original <= calculated.len() && used.insert(original),
            "invalid/duplicate final row reference: {context}"
        );
        assert!(
            row["value"] == calculated[original - 1],
            "display changed its exact calculation row: {context}"
        );
        assert!(
            row["value"] == displayed[index],
            "display position differs from its exact observed row: {context}"
        );
    }
}
fn check(report: &Json, saved_report_only: bool) {
    let historical = report["attribution_schema_version"].is_null();
    assert!(
        historical && saved_report_only || report["attribution_schema_version"] == 2,
        "historical display checks are available only in explicit saved-report validation"
    );
    let cases = rows(&report["cases"]);
    let diagnostic = report["diagnostic_case"].as_str();
    assert_eq!(cases.len(), if diagnostic.is_some() { 1 } else { 44 });
    let case = |name: &str| cases.iter().find(|c| c["name"] == name).unwrap();
    if diagnostic.is_none() {
        assert!(case("original-05")["states"] == case("repeat-original-05")["states"]);
    }
    for case in cases {
        for stage in STAGES {
            let state = &case["states"][stage];
            assert_eq!(state["hook_removed"], true);
            assert_eq!(state["original_functions_preserved"], true);
            for call in rows(&state["calls"]) {
                check_call_integrity(call);
            }
            for mode in ["MAIN", "CALCS"] {
                let calls: Vec<_> = rows(&state["calls"])
                    .iter()
                    .filter(|c| c["outer"] == mode)
                    .collect();
                assert_eq!(calls.len(), 1, "{} {stage} {mode}", case["name"]);
                let c = calls[0];
                assert_eq!(c["cache_present"], false);
                assert_eq!(
                    c["result"]["combinedDPS"],
                    state["outputs"][mode]["full_dps"]
                );
                assert_eq!(
                    c["result"]["TotalDotDPS"],
                    state["outputs"][mode]["full_dot_dps"]
                );
                check_display_order(
                    c,
                    &state["outputs"][mode],
                    historical,
                    &format!("{} {stage} {mode}", case["name"]),
                );
                assert_eq!(rows(&c["rows"]).len(), rows(&c["result"]["skills"]).len());
                for row in rows(&c["rows"]) {
                    assert_ne!(
                        row["attribution"], "unresolved_aggregate_write",
                        "{} {stage} {row}",
                        case["name"]
                    );
                }
                for pass in rows(&c["passes"]) {
                    let source = &pass["source"];
                    if SOURCES.iter().any(|(_, e, _)| source["effect"] == *e)
                        || source["effect"].as_str().is_some_and(|s| {
                            s.starts_with("CommandSandDjinn") || s.starts_with("CommandWaterDjinn")
                        })
                    {
                        assert_eq!(source["source_object_joined"], true);
                        assert_eq!(source["group_object_joined"], true);
                        for actor in rows(&pass["actors"]) {
                            assert_eq!(actor["identity"]["exact_output_object"], true);
                        }
                    }
                }
            }
        }
    }
    if diagnostic.is_some_and(|name| name != "same-label-manual-firebolt-tie") {
        return;
    }
    if diagnostic.is_none() {
        for (source_index, (name, _, _)) in SOURCES.into_iter().enumerate() {
            for (stage, mode) in STAGES
                .into_iter()
                .flat_map(|s| ["MAIN", "CALCS"].map(|m| (s, m)))
            {
                for (control, count) in [
                    ("count-one", 1),
                    ("count-three", 3),
                    ("group-zero", 0),
                    ("group-four", 4),
                ] {
                    let c = case(&format!("{name}-{control}"));
                    let calls = rows(&c["states"][stage]["calls"]);
                    let call = calls.iter().find(|c| c["outer"] == mode).unwrap();
                    check_source_passes(call, &c["states"][stage], source_index, count);
                    assert_eq!(
                        rows(&call["passes"]).len(),
                        if source_index == 2 { 1 } else { 2 }
                    );
                }
                for control in ["excluded", "disabled-source", "disabled-group"] {
                    let c = case(&format!("{name}-{control}"));
                    let call = rows(&c["states"][stage]["calls"])
                        .iter()
                        .find(|c| c["outer"] == mode)
                        .unwrap();
                    assert!(
                        rows(&call["passes"]).is_empty(),
                        "{name}-{control} {stage} {mode}"
                    );
                }
            }
        }
        for (family, generated, manual) in [("sand", 0, 3), ("water", 1, 4)] {
            for stage in STAGES {
                for mode in ["MAIN", "CALCS"] {
                    let state = &case(&format!("manual-and-generated-{family}"))["states"][stage];
                    let call = rows(&state["calls"])
                        .iter()
                        .find(|c| c["outer"] == mode)
                        .unwrap();
                    let generated_source = check_source_passes(call, state, generated, 1);
                    let manual_source = check_source_passes(call, state, manual, 3);
                    assert!(generated_source.is_disjoint(&manual_source));
                    assert_eq!(rows(&call["passes"]).len(), 4);
                }
            }
        }
    }
    let mut positive_ties = 0;
    for stage in STAGES {
        for call in rows(&case("same-label-manual-firebolt-tie")["states"][stage]["calls"]) {
            let passes = rows(&call["passes"]);
            assert_eq!(passes.len(), 2);
            assert_ne!(
                passes[0]["source"]["source_ordinal"],
                passes[1]["source"]["source_ordinal"]
            );
            assert_ne!(
                passes[0]["source"]["group_source_ordinal"],
                passes[1]["source"]["group_source_ordinal"]
            );
            for pass in passes {
                assert_eq!(pass["source"]["effect"], "FireboltPlayer");
                assert!(pass["source"]["source"].is_null());
            }
            let mut phase_ties = 0;
            for comparison in rows(&call["comparisons"]) {
                if comparison["field"] == "igniteDPS"
                    && comparison["candidate"].as_f64().is_some_and(|x| x > 0.)
                    && comparison["candidate"] == comparison["prior"]
                {
                    assert_eq!(comparison["replacement_entered"], false);
                    positive_ties += 1;
                    phase_ties += 1;
                }
            }
            assert_eq!(
                phase_ties, 1,
                "each exact reporting phase must observe the positive tie"
            );
        }
    }
    assert!(
        positive_ties > 0,
        "positive Ignite tie must be observed in an original branch"
    );
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
