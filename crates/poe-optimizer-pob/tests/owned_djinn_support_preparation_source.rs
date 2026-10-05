//! Original source support admission, distinct from numerical delivery or native build parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/bidding_support_source.rs"]
mod bidding_support;
#[path = "support/json_evidence.rs"]
mod json_evidence;
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

const TEST: &str = "complete_djinn_support_preparation_observes_original_admission";
const CHILD: &str = "POE_DJINN_SUPPORT_PREPARATION_CHILD";
const OBSERVER: &str = include_str!("support/djinn_support_preparation.lua");
const LIFECYCLE: &str = include_str!("support/djinn_provider_source.lua");
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const SAND: &str = "SummonSandDjinnPlayer";
const WATER: &str = "SummonWaterDjinnPlayer";
const SUPPORTS: [&str; 10] = [
    "SupportBiddingPlayerTwo",
    "SupportMagnifiedAreaPlayer",
    "SupportMusterPlayer",
    "SupportChillingIcePlayer",
    "SupportBiddingPlayerThree",
    "SupportMagnifiedAreaPlayerTwo",
    "ProlongedDurationSupportPlayerTwo",
    "SupportHulkingMinionsPlayer",
    "SupportKurgalsLeashPlayer",
    "SupportRapidCastingPlayerTwo",
];
const FILES: [&str; 20] = [
    "src/HeadlessWrapper.lua",
    "src/Modules/Common.lua",
    "src/Modules/Build.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/CompareTab.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Modules/Data.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/CalcPerform.lua",
    "src/Data/Global.lua",
    "src/Data/Minions.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/minion.lua",
    "src/Data/Skills/sup_int.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Gems.lua",
];

#[test]
#[ignore = "requires complete pinned PoB runtime; original Djinn support preparation only"]
fn complete_djinn_support_preparation_observes_original_admission() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-djinn-support-preparation-source-02");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
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
        "Djinn support preparation JIT parity",
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
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    for (family, effect, suffix, support) in [
        ("sand", SAND, "bidding", SUPPORTS[0]),
        ("sand", SAND, "magnified-area", SUPPORTS[1]),
        ("sand", SAND, "muster", SUPPORTS[2]),
        ("water", WATER, "bidding", SUPPORTS[0]),
        ("water", WATER, "muster", SUPPORTS[2]),
        ("water", WATER, "frost-nexus", SUPPORTS[3]),
    ] {
        let (changed, control) = disable_support(original, effect, support);
        assert_ne!(changed, original);
        cases.push(observe(
            root,
            &format!("disable-{family}-{suffix}"),
            &changed,
            enabled,
            Some(control),
        ));
    }
    cases.push(observe(root, "repeat-original-05", original, enabled, None));
    let report = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "manifest_sha256":pinned::manifest_sha256(),
        "files":FILES.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"native_build_parity":false,
        "native_inventory_authority":false,"numerical_delivery_authority":false,
        "canonical_parity_lifecycle_selected":false,"lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "support evidence is {} bytes",
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
    check(&report);
    for (i, b) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            b
        );
    }
}

fn install_hook(lua: &Lua) -> Result<Function, RuntimeError> {
    lua.globals().set("djinnSupportPhase", "before")?;
    Ok(lua
        .load(OBSERVER)
        .set_name("@djinn-support-original-call-observer-install")
        .eval()?)
}
fn observe(root: &Path, name: &str, xml: &str, enabled: bool, control: Option<Json>) -> Json {
    observe_with_extra(root, name, xml, enabled, control, None)
}
fn observe_with_extra(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    control: Option<Json>,
    extra: Option<&str>,
) -> Json {
    let started = Instant::now();
    eprintln!(
        "Djinn support case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("djinnXml", xml)?;
        lua.globals().set("djinnJit", enabled)?;
        lua.globals()
            .set("djinnOccurrenceNumericEntries", extra.is_some())?;
        lua.load("if djinnJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let auth: Function = lua
            .load(LIFECYCLE)
            .set_name("@djinn-support-original-lifecycle-authentication")
            .eval()?;
        let observer = install_hook(lua)?;
        let cleanup: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(cleanup.call((observer, auth))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua, extra)?;
        rebuild(lua)?;
        let rebuilt_once = stage(lua, extra)?;
        rebuild(lua)?;
        let rebuilt_twice = stage(lua, extra)?;
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
        Some(&install),
        Some(&observer),
    )
    .unwrap_or_else(|e| panic!("{name}: complete source failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    let states = &observed["additional_observation"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([103; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    for stage in STAGES {
        for group in rows(&states[stage]["occurrences"]["raw_groups"]) {
            for gem in rows(&group["gems"]) {
                let source = evidence
                    .rows()
                    .iter()
                    .find(|r| {
                        Some(u64::from(r.occurrence().id().ordinal())) == gem["source"].as_u64()
                    })
                    .unwrap();
                assert_eq!(source.occurrence().name(), "Gem");
                assert_eq!(
                    source.attributes().len(),
                    gem["attributes"].as_object().unwrap().len()
                );
                for attribute in source.attributes() {
                    assert_eq!(
                        gem["attributes"][&attribute.origin().name],
                        attribute.decoded().unwrap()
                    );
                }
            }
        }
    }
    if extra.is_some() {
        eprintln!(
            "Djinn support completed {name} in {:.2}s",
            started.elapsed().as_secs_f64()
        );
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"control":control,"states":states})
}
fn occurrence(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("djinnPhase", "observe")?;
    let value: Value = lua
        .load(LIFECYCLE)
        .set_name("@djinn-support-existing-occurrence-observer")
        .eval()?;
    Ok(lua
        .from_value(value)
        .map_err(|error| mlua::Error::external(format!("Djinn occurrence projection: {error}")))?)
}
fn stage(lua: &Lua, extra: Option<&str>) -> Result<Json, RuntimeError> {
    let before = occurrence(lua)?;
    lua.globals().set("djinnSupportPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@djinn-support-original-call-observation")
        .eval()?;
    let preparation: Json = lua
        .from_value(value)
        .map_err(|error| mlua::Error::external(format!("Djinn preparation projection: {error}")))?;
    let delivery = extra
        .map(|observer| {
            let value: Value = lua
                .load(observer)
                .set_name("@djinn-support-numerical-observer")
                .eval()?;
            lua.from_value::<Json>(value).map_err(|error| {
                mlua::Error::external(format!("Bidding delivery projection: {error}"))
            })
        })
        .transpose()?;
    let after = occurrence(lua)?;
    assert_eq!(
        json_evidence::first_difference(&before, &after, "source-preservation"),
        None
    );
    let mut result = json!({"occurrences":before,"preparation":preparation});
    if let Some(delivery) = delivery {
        result["delivery"] = delivery;
    }
    Ok(result)
}
fn rebuild(lua: &Lua) -> Result<(), RuntimeError> {
    let cleanup = install_hook(lua)?;
    let result = lua
        .load(
            r#"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua"and p:sub(-#path)==path and i.linedefined==line);return f
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
        .set_name("@djinn-support-original-frame-rebuild")
        .exec();
    let removed = cleanup.call::<()>(());
    result?;
    removed?;
    Ok(())
}
fn disable_support(xml: &str, effect: &str, support: &str) -> (String, Json) {
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
    let group = set
        .children()
        .find(|n| {
            n.has_tag_name("Skill")
                && n.attribute("source").is_none()
                && n.children()
                    .find(|g| g.has_tag_name("Gem"))
                    .is_some_and(|g| g.attribute("skillId") == Some(effect))
        })
        .unwrap();
    let targets: Vec<_> = group
        .children()
        .filter(|g| g.has_tag_name("Gem") && g.attribute("skillId") == Some(support))
        .collect();
    assert_eq!(targets.len(), 1);
    let gem = targets[0];
    assert_eq!(gem.attribute("enabled"), Some("true"));
    let ordinal = doc
        .descendants()
        .filter(|n| n.is_element())
        .position(|n| n == gem)
        .unwrap();
    let replacement = xml[gem.range()].replacen("enabled=\"true\"", "enabled=\"false\"", 1);
    assert_ne!(replacement, xml[gem.range()]);
    let mut changed = xml.to_owned();
    changed.replace_range(gem.range(), &replacement);
    roxmltree::Document::parse(&changed).unwrap();
    (
        changed,
        json!({"preset":selected.parse::<u64>().unwrap(),"effect":effect,"support":support,"source_ordinal":ordinal}),
    )
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn names(value: &Json) -> Vec<&str> {
    rows(&value["values"])
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect()
}
fn contains_type(value: &Json, name: &str) -> bool {
    rows(&value["values"])
        .iter()
        .any(|v| v["name"] == name && v["value"] == true)
}
fn check(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 12);
    assert_eq!(
        json_evidence::first_difference(&cases[4]["states"], &cases[11]["states"], "repeat"),
        None
    );
    let definitions = &cases[4]["states"]["fresh"]["preparation"]["definitions"];
    let defs = rows(definitions);
    assert_eq!(defs.len(), SUPPORTS.len());
    // The first four definition observations are byte-for-byte canonical JSON
    // from the authenticated source-01 receipt, independent of its local file.
    assert_eq!(
        digest(&serde_json::to_vec(&defs[..4]).unwrap()),
        "aa28f6945daf85dec1ef78bec6fbd1cee7ff6911e8807dd4ed5a272f57ee459d",
    );
    for (i, effect) in SUPPORTS.iter().enumerate() {
        assert_eq!(defs[i]["effect"], *effect);
        assert_eq!(defs[i]["support"], true);
        if i != 9 {
            assert!(names(&defs[i]["exclude_types"]).is_empty());
        }
    }
    assert_eq!(
        names(&defs[0]["require_types"]),
        ["CommandableMinion", "CommandsMinions"]
    );
    assert_eq!(
        names(&defs[1]["require_types"]),
        ["Area", "MinionsCanExplode"]
    );
    assert_eq!(names(&defs[2]["require_types"]), ["CreatesMinion"]);
    assert_eq!(
        names(&defs[3]["require_types"]),
        ["Damage", "Attack", "CrossbowAmmoSkill"]
    );
    assert_eq!(names(&defs[3]["add_types"]), ["CreatesGroundEffect"]);
    for (index, required, family) in [
        (4, vec!["CommandableMinion", "CommandsMinions"], "Bidding"),
        (
            5,
            vec!["Area", "MinionsCanExplode"],
            "IncreasedAreaOfEffect",
        ),
        (6, vec!["Duration"], "ProlongedDuration"),
        // AND is an expression token; it is not an admitted skill type.
        (7, vec!["Minion", "Persistent", "AND"], "HulkingMinions"),
        (
            8,
            vec!["CommandsMinions", "CommandableMinion"],
            "KurgalLineage",
        ),
        (9, vec!["Spell"], "RapidCasting"),
    ] {
        let definition = &defs[index];
        assert_eq!(names(&definition["require_types"]), required);
        assert_eq!(definition["family"], json!([family]));
        assert_eq!(definition["family_present"], true);
        assert!(names(&definition["add_types"]).is_empty());
        assert_eq!(definition["skill_types"]["present"], false);
        assert_eq!(definition["minion_types"]["present"], false);
        for flag in [
            "support_gems_only",
            "ignore_minion_types",
            "from_item",
            "is_trigger",
            "add_flags",
        ] {
            assert!(definition.get(flag).is_none(), "unexpected {flag}");
        }
    }
    assert_eq!(
        names(&defs[9]["exclude_types"]),
        ["Instant", "FixedCastTime", "NoAttackOrCastTime"],
    );
    let mut observed_predicates = std::collections::BTreeSet::new();
    for case in cases {
        for stage in STAGES {
            let state = &case["states"][stage];
            let p = &state["preparation"];
            assert_eq!(
                json_evidence::first_difference(&p["definitions"], definitions, "definitions"),
                None
            );
            for field in [
                "original_methods_preserved",
                "hook_removed",
                "jit_mode_preserved",
            ] {
                assert_eq!(p[field], true);
            }
            for field in [
                "original_functions_preserved",
                "saved_instances_preserved",
                "selected_state_preserved",
                "outputs_preserved",
            ] {
                assert_eq!(state["occurrences"][field], true);
            }
            let contexts = rows(&p["contexts"]);
            assert_eq!(
                p["retained_contexts"].as_u64().unwrap() as usize,
                contexts.len()
            );
            for context in contexts {
                for field in [
                    "exact_constructor_object",
                    "exact_actor",
                    "exact_group",
                    "exact_parent",
                ] {
                    assert_eq!(context[field], true);
                }
                if let Some(parent) = context["parent_context"].as_u64() {
                    assert_eq!(context["parent_present"], true);
                    let parent = &contexts[parent as usize - 1];
                    assert_eq!(parent["mode"], context["mode"]);
                    assert!(parent["effect"] == SAND || parent["effect"] == WATER);
                    assert_eq!(context["actor_is_player"], false);
                    assert_eq!(context["source_instance_present"], false);
                    for call in rows(&context["calls"]) {
                        assert_eq!(call["parent_present"], true);
                        assert_eq!(call["effective_type_owner"], "summoner");
                    }
                }
                for candidate in rows(&context["candidates"]) {
                    assert_eq!(candidate["source"]["enabled"], true);
                    assert_eq!(candidate["exact_definition"], true);
                    assert!(SUPPORTS.contains(&candidate["effect"].as_str().unwrap()));
                }
                for call in rows(&context["calls"]) {
                    observed_predicates.insert(call["support"].as_str().unwrap());
                }
                if context["group"]["source_present"] == true {
                    assert!(
                        rows(&context["candidates"]).is_empty(),
                        "allocated groups must not acquire manual supports"
                    );
                }
            }
            if let Some(control) = case["control"].as_object() {
                let target = &control["source_ordinal"];
                let saved = rows(&state["occurrences"]["raw_groups"])
                    .iter()
                    .flat_map(|g| rows(&g["gems"]))
                    .find(|g| &g["source"] == target)
                    .unwrap();
                assert_eq!(saved["attributes"]["enabled"], "false");
                let mut affected = 0;
                for context in contexts {
                    if context["group"]["preset"] == control["preset"]
                        && context["group"]["source_present"] == false
                        && context["group"]["key"]
                            .as_str()
                            .unwrap()
                            .contains(control["effect"].as_str().unwrap())
                    {
                        affected += 1;
                        assert!(
                            rows(&context["candidates"])
                                .iter()
                                .all(|c| &c["source"]["source_ordinal"] != target)
                        );
                        assert!(
                            rows(&context["calls"])
                                .iter()
                                .all(|c| c["support"] != control["support"])
                        );
                        let baseline = rows(&cases[4]["states"][stage]["preparation"]["contexts"])
                            .iter()
                            .find(|original| {
                                original["mode"] == context["mode"]
                                    && original["effect"] == context["effect"]
                                    && original["group"]["key"] == context["group"]["key"]
                            })
                            .unwrap();
                        assert_eq!(
                            json_evidence::first_difference(
                                &retained_candidates(context, target),
                                &retained_candidates(baseline, target),
                                "remaining-candidate-occurrences",
                            ),
                            None,
                        );
                        assert_eq!(
                            accepted_sources(context, target),
                            accepted_sources(baseline, target),
                        );
                        if control["support"] == SUPPORTS[3] && context["effect"] == WATER {
                            assert_eq!(
                                json_evidence::first_difference(
                                    &context["initial_definition"],
                                    &baseline["initial_definition"],
                                    "unchanged-initial-definition",
                                ),
                                None,
                            );
                            assert!(!contains_type(
                                &context["constructor_types"]["skill_types"],
                                "CreatesGroundEffect",
                            ));
                        }
                    }
                }
                assert!(affected >= 10);
            }
        }
    }
    assert_eq!(
        observed_predicates,
        SUPPORTS
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        "every authored support must have an original predicate-call witness",
    );
    // These are actual original source admission results, not native delivery values.
    for stage in STAGES {
        let contexts = rows(&cases[4]["states"][stage]["preparation"]["contexts"]);
        for effect in [SAND, WATER] {
            for mode in ["MAIN", "CALCS"] {
                let root = contexts
                    .iter()
                    .find(|c| {
                        c["effect"] == effect
                            && c["mode"] == mode
                            && c["group"]["source_present"] == false
                            && c["group"]["preset"] == 4
                    })
                    .unwrap();
                assert_eq!(rows(&root["candidates"]).len(), 3);
                assert_eq!(rows(&root["accepted_indices"]).len(), 3);
                assert!(!rows(&root["calls"]).is_empty());
                if effect == WATER {
                    assert!(!contains_type(
                        &root["initial_definition"]["skill_types"],
                        "CreatesGroundEffect"
                    ));
                    assert!(contains_type(
                        &root["constructor_types"]["skill_types"],
                        "CreatesGroundEffect"
                    ));
                }
            }
        }
    }
}
// The original constructor compacts candidate positions after a disabled source
// is removed. All other fields, including exact source identity and acceptance,
// must match the original context after removing only that occurrence.
fn retained_candidates(context: &Json, disabled: &Json) -> Json {
    Json::Array(
        rows(&context["candidates"])
            .iter()
            .filter(|candidate| &candidate["source"]["source_ordinal"] != disabled)
            .map(|candidate| {
                let mut retained = candidate.clone();
                retained.as_object_mut().unwrap().remove("index");
                retained
            })
            .collect(),
    )
}
fn accepted_sources<'a>(context: &'a Json, disabled: &Json) -> Vec<&'a Json> {
    rows(&context["candidates"])
        .iter()
        .filter(|candidate| {
            candidate["accepted"] == true && &candidate["source"]["source_ordinal"] != disabled
        })
        .map(|candidate| &candidate["source"]["source_ordinal"])
        .collect()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap();
    text.chars()
        .rev()
        .take(6000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}
