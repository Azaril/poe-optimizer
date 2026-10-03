//! Optional complete-source Ice Nova physical, usage, and reference evidence.
//! This observes the original evaluator; it does not claim native damage parity.
#![cfg(not(target_arch = "wasm32"))]
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

const TEST: &str = "complete_ice_nova_occurrence_inputs_preserve_source_semantics";
const CHILD: &str = "POE_ICE_NOVA_OCCURRENCE_SOURCE_CHILD";
const ICE: &str = "Metadata/Items/Gems/SkillGemIceNova";
const EFFECT: &str = "IceNovaPlayer";
const CATALOG_DIGEST: &str = "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea";
const MAIN_2_CALCS_1: &str = r#"<StatSetIndex grantedEffect="IceNovaPlayer" index="2"/><StatSetCalcsIndex grantedEffect="IceNovaPlayer" index="1"/>"#;
const MAIN_1_CALCS_2: &str = r#"<StatSetIndex grantedEffect="IceNovaPlayer" index="1"/><StatSetCalcsIndex grantedEffect="IceNovaPlayer" index="2"/>"#;

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for Ice Nova occurrence inputs"]
fn complete_ice_nova_occurrence_inputs_preserve_source_semantics() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-ice-nova-occurrence-source-01");
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
        .find(|gem| gem.key == ICE)
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
        cases.push(observe_case(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
        ));
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    let controls = [
        (
            "global-1-false",
            edit(
                xml,
                &[("enableGlobal1", Some("false"))],
                &[],
                None,
                false,
                false,
            ),
        ),
        (
            "global-2-false",
            edit(
                xml,
                &[("enableGlobal2", Some("false"))],
                &[],
                None,
                false,
                false,
            ),
        ),
        (
            "both-globals-false",
            edit(
                xml,
                &[
                    ("enableGlobal1", Some("false")),
                    ("enableGlobal2", Some("false")),
                ],
                &[],
                None,
                false,
                false,
            ),
        ),
        (
            "count-zero",
            edit(xml, &[("count", Some("0"))], &[], None, false, false),
        ),
        (
            "count-three",
            edit(xml, &[("count", Some("3"))], &[], None, false, false),
        ),
        (
            "count-missing",
            edit(xml, &[("count", None)], &[], None, false, false),
        ),
        (
            "count-malformed",
            edit(xml, &[("count", Some("bad"))], &[], None, false, false),
        ),
        (
            "group-zero-over-count-three",
            edit(
                xml,
                &[("count", Some("3"))],
                &[("groupCount", Some("0"))],
                None,
                false,
                false,
            ),
        ),
        (
            "group-four-over-count-three",
            edit(
                xml,
                &[("count", Some("3"))],
                &[("groupCount", Some("4"))],
                None,
                false,
                false,
            ),
        ),
        (
            "full-dps-count-one",
            edit(
                xml,
                &[],
                &[("includeInFullDPS", Some("true"))],
                None,
                false,
                false,
            ),
        ),
        (
            "full-dps-count-three",
            edit(
                xml,
                &[("count", Some("3"))],
                &[("includeInFullDPS", Some("true"))],
                None,
                false,
                false,
            ),
        ),
        (
            "disabled-gem",
            edit(xml, &[("enabled", Some("false"))], &[], None, false, false),
        ),
        (
            "disabled-group",
            edit(xml, &[], &[("enabled", Some("false"))], None, false, false),
        ),
        (
            "main-two-calcs-one",
            edit(xml, &[], &[], Some(MAIN_2_CALCS_1), false, false),
        ),
        (
            "main-one-calcs-two",
            edit(xml, &[], &[], Some(MAIN_1_CALCS_2), false, false),
        ),
        (
            "duplicate-independent-inputs",
            edit(xml, &[], &[], None, true, false),
        ),
        (
            "archived-only",
            edit(
                xml,
                &[
                    ("count", Some("3")),
                    ("enableGlobal1", Some("false")),
                    ("enableGlobal2", Some("false")),
                ],
                &[],
                Some(MAIN_2_CALCS_1),
                false,
                true,
            ),
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
        "business_wrappers":false,"native_inventory_authority":false,"native_build_parity":false,
        "phase_independent_activity_claimed":false,"universal_count_inertness_claimed":false,
        "canonical_parity_lifecycle_selected":false,"full_dps_rows_have_physical_identity":false,
        "lifecycle_stages":["fresh_complete_load","requested_original_frame_rebuild_1","requested_original_frame_rebuild_2"],
        "files":(["src/HeadlessWrapper.lua","src/Modules/Build.lua","src/Classes/CalcsTab.lua","src/Classes/SkillsTab.lua","src/Modules/Data.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Modules/Calcs.lua","src/Modules/CalcDefence.lua","src/Modules/CalcPerform.lua","src/Data/Skills/act_int.lua","src/Data/Gems.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "original_sources":originals.iter().enumerate().map(|(i, bytes)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(bytes)})).collect::<Vec<_>>(),
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
        "Ice occurrence case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("iceOccurrenceXml", xml)?;
        lua.globals().set("iceOccurrenceJit", enabled)?;
        lua.load("if iceOccurrenceJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        // Reuse authentication of the unchanged complete lifecycle only. This
        // helper's Djinn-specific observation branch is never invoked.
        lua.globals().set("djinnPhase", "before")?;
        Ok(lua
            .load(include_str!("support/djinn_provider_source.lua"))
            .set_name("@ice-occurrence-original-lifecycle-authentication")
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
        BuildLineage::from_bytes([81; 16]),
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
                && row.attribute("gemId").and_then(|attr| attr.decoded().ok()) == Some(ICE)
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
    let value: Value = lua
        .load(include_str!("support/ice_nova_occurrence_source.lua"))
        .set_name("@ice-nova-exact-occurrence-inputs")
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
    .set_name("@ice-occurrence-original-requested-frame")
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
    gem_edits: &[AttributeEdit<'_>],
    group_edits: &[AttributeEdit<'_>],
    children: Option<&str>,
    duplicate: bool,
    archived: bool,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|node| node.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    let mut edits = Vec::new();
    let mut matched = 0;
    for gem in skills
        .descendants()
        .filter(|node| node.has_tag_name("Gem") && node.attribute("gemId") == Some(ICE))
    {
        let set = gem
            .ancestors()
            .find(|node| node.has_tag_name("SkillSet"))
            .unwrap();
        if (set.attribute("id") == Some(selected)) == archived {
            continue;
        }
        matched += 1;
        assert!(gem.children().all(|node| !node.is_element()));
        if !group_edits.is_empty() {
            let group = gem.parent().unwrap();
            let start = group.range().start;
            let end = start + xml[start..].find('>').unwrap() + 1;
            edits.push((start..end, rewrite(group, group_edits)));
        }
        if !gem_edits.is_empty() || children.is_some() || duplicate {
            let mut replacement = format!(
                "{}{}</Gem>",
                rewrite(gem, gem_edits),
                children.unwrap_or("")
            );
            if duplicate {
                replacement.push_str(&format!(
                    "{}{MAIN_2_CALCS_1}</Gem>",
                    rewrite(
                        gem,
                        &[
                            ("count", Some("3")),
                            ("enableGlobal1", Some("false")),
                            ("enableGlobal2", Some("false"))
                        ]
                    )
                ));
            }
            edits.push((gem.range(), replacement));
        }
    }
    assert_eq!(matched, if archived { 3 } else { 1 });
    assert!(!edits.is_empty());
    edits.sort_by_key(|(range, _)| range.start);
    assert!(
        edits
            .windows(2)
            .all(|pair| pair[0].0.end <= pair[1].0.start)
    );
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
    let cases = result["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 23);
    let baseline = &cases[4];
    assert_eq!(baseline["states"], cases.last().unwrap()["states"]);
    assert_eq!(
        baseline["source_joins"],
        cases.last().unwrap()["source_joins"]
    );
    assert_eq!(
        baseline["states"]["fresh"]["selection"],
        json!({"skills":4,"items":2,"spec":3,"config":1,"group":3})
    );
    assert_eq!(
        selected(&baseline["states"]["fresh"])[0]["source_ordinal"],
        239
    );
    for (index, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap();
        let states = &case["states"];
        let initial_revision = states["fresh"]["output_revision"].as_u64().unwrap();
        let mut once = states["rebuilt_once"].clone();
        assert_eq!(once["output_revision"], initial_revision + 1);
        assert_eq!(
            states["rebuilt_twice"]["output_revision"],
            initial_revision + 2
        );
        once["output_revision"] = states["rebuilt_twice"]["output_revision"].clone();
        assert_eq!(
            once, states["rebuilt_twice"],
            "{name}: repeat requested rebuild"
        );
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &states[stage];
            for flag in [
                "source_methods_preserved",
                "exact_physical_objects",
                "physical_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(state[flag], true, "{name} {stage}: {flag}");
            }
            assert_eq!(state["build_flag"], false);
            assert_eq!(state["output_lifecycle"]["module_reloads"], 2);
            if index >= 4 {
                assert_eq!(
                    state["selection"], baseline["states"][stage]["selection"],
                    "{name}"
                );
                assert_eq!(
                    state["selectors"], baseline["states"][stage]["selectors"],
                    "{name}"
                );
                assert_eq!(state["selectors"]["MAIN"]["group"], 3);
                assert_eq!(state["selectors"]["CALCS"]["group"], 1);
                assert_eq!(
                    rows(&state["saved"]).len(),
                    if name == "duplicate-independent-inputs" {
                        5
                    } else {
                        4
                    }
                );
            }
            for row in rows(&state["saved"]) {
                let eligible = row["selected"] == true
                    && row["loaded"]["enabled"] == true
                    && row["group_state"]["enabled"] == true
                    && row["group_state"]["slot_enabled"] == true;
                for mode in ["MAIN", "CALCS"] {
                    let actions = rows(&row[mode]);
                    assert_eq!(
                        actions.len(),
                        usize::from(eligible),
                        "{name} {stage} {mode} source{}",
                        row["source_ordinal"]
                    );
                    for action in actions {
                        assert_eq!(action["source_ordinal"], row["source_ordinal"]);
                        assert_eq!(action["exact_physical_object"], true);
                        assert_eq!(action["exact_group"], true);
                        assert_eq!(action["actor_is_player"], true);
                        assert_eq!(action["effect"], EFFECT);
                        assert_eq!(action["effect_index"], 1);
                        assert_eq!(action["count_enabled"], true);
                        assert_eq!(action["has_global_effect"], false);
                        assert_eq!(action["has_parts"], false);
                        let candidates = rows(&action["count_candidates"]);
                        assert!(!candidates.is_empty());
                        let expected_count = if row["group_state"]["group_count"].is_number() {
                            &row["group_state"]["group_count"]
                        } else {
                            &candidates[0]["count"]
                        };
                        assert_eq!(
                            &action["count"], expected_count,
                            "{name} actual source count"
                        );
                        let set = action["stat_set_index"].as_u64().unwrap();
                        assert!(set == 1 || set == 2);
                        assert_eq!(
                            action["stat_description_scope"],
                            if set == 1 {
                                "ice_nova_statset_0"
                            } else {
                                "ice_nova_statset_1"
                            }
                        );
                    }
                }
            }
            if index < 4 {
                continue;
            }
            let selected_rows = selected(state);
            assert_eq!(
                selected_rows.len(),
                if name == "duplicate-independent-inputs" {
                    2
                } else {
                    1
                }
            );
            let first = selected_rows[0];
            let expected_raw_count = match name {
                "count-zero" => 0,
                "count-three"
                | "group-zero-over-count-three"
                | "group-four-over-count-three"
                | "full-dps-count-three" => 3,
                _ => 1,
            };
            assert_eq!(first["loaded"]["count"], expected_raw_count, "{name}");
            if matches!(name, "global-1-false" | "both-globals-false") {
                assert_eq!(first["loaded"]["global_1"], false);
            }
            if matches!(name, "global-2-false" | "both-globals-false") {
                assert_eq!(first["loaded"]["global_2"], false);
            }
            if name == "disabled-gem" {
                assert_eq!(first["loaded"]["enabled"], false);
            }
            if name == "disabled-group" {
                assert_eq!(first["group_state"]["enabled"], false);
            }
            for mode in ["MAIN", "CALCS"] {
                let expected_set = if (name == "main-two-calcs-one" && mode == "MAIN")
                    || (name == "main-one-calcs-two" && mode == "CALCS")
                {
                    2
                } else {
                    1
                };
                for action in rows(&first[mode]) {
                    assert_eq!(action["stat_set_index"], expected_set, "{name} {mode}");
                }
                if name == "duplicate-independent-inputs" {
                    assert_ne!(
                        selected_rows[0]["source_ordinal"],
                        selected_rows[1]["source_ordinal"]
                    );
                    assert_eq!(selected_rows[1]["loaded"]["count"], 3);
                    assert_eq!(selected_rows[1]["loaded"]["global_1"], false);
                    assert_eq!(selected_rows[1]["loaded"]["global_2"], false);
                    let second = &rows(&selected_rows[1][mode])[0];
                    assert_eq!(second["stat_set_index"], if mode == "MAIN" { 2 } else { 1 });
                    assert_eq!(
                        second["count"], 1,
                        "same-effect helper uses the first copy despite the second raw count3"
                    );
                    assert_eq!(rows(&second["count_candidates"]).len(), 2);
                }
                if name.starts_with("full-dps-count-") {
                    assert_eq!(first["group_state"]["include_in_full_dps"], true);
                    let dps: Vec<_> = rows(&state["full_dps"][mode])
                        .iter()
                        .filter(|row| row["name"] == "Ice Nova")
                        .collect();
                    assert_eq!(dps.len(), 1, "{name} {mode}: actual direct FullDPS row");
                    assert_eq!(dps[0]["count"], expected_raw_count);
                    assert!(dps[0]["dps"].as_f64().unwrap() > 0.0);
                }
            }
            if name == "archived-only" {
                assert_eq!(first, selected(&baseline["states"][stage])[0]);
                assert_eq!(state["outputs"], baseline["states"][stage]["outputs"]);
                assert_eq!(state["full_dps"], baseline["states"][stage]["full_dps"]);
                assert_eq!(
                    rows(&state["saved"])
                        .iter()
                        .filter(|row| row["loaded"]["count"] == 3)
                        .count(),
                    3
                );
            }
        }
    }
    for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
        let one = cases
            .iter()
            .find(|case| case["name"] == "full-dps-count-one")
            .unwrap();
        let three = cases
            .iter()
            .find(|case| case["name"] == "full-dps-count-three")
            .unwrap();
        for mode in ["MAIN", "CALCS"] {
            let direct = |case: &Json| {
                rows(&case["states"][stage]["full_dps"][mode])
                    .iter()
                    .find(|row| row["name"] == "Ice Nova")
                    .unwrap()["dps"]
                    .as_f64()
                    .unwrap()
            };
            assert_eq!(
                direct(one),
                direct(three),
                "count changes the FullDPS multiplicity, not this observed per-copy damage"
            );
        }
    }
}
