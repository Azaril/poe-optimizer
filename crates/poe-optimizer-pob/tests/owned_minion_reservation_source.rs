//! Optional source inputs/intermediates for flat Spirit reservation.
//! Captures original arithmetic; does not install native constants or choose a parity lifecycle.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_core::{build_identity::BuildLineage, owned_content::digest_owned};
use poe_optimizer_data::skill_identities::{GemIdentity, SkillIdentityCatalog};
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

const TEST: &str = "complete_minion_reservation_records_original_inputs_and_rounding";
const CHILD: &str = "POE_MINION_RESERVATION_CHILD";
const SNIPER: &str = "SummonSkeletalSnipersPlayer";
const WARRIOR: &str = "SummonSkeletalWarriorsPlayer";
const CAPTURE: &str = include_str!("support/minion_reservation_capture.lua");
const OBSERVE: &str = include_str!("support/minion_reservation_observe.lua");

#[test]
#[ignore = "requires optional complete pinned PoB runtime; explicit reservation source evidence"]
fn complete_minion_reservation_records_original_inputs_and_rounding() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-minion-reservation-source-01");
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
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap(),
        "full deterministic source evidence differs across JIT modes"
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    assert_eq!(
        digest(&originals[4]),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_slice(
            &fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let catalog_digest = digest_owned(
        "owned-skill-source-catalog-v1",
        catalog.data(),
        64 * 1024 * 1024,
    )
    .unwrap()
    .to_string();
    assert_eq!(
        catalog_digest,
        "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea"
    );
    let reviewed: Vec<_> = [
        "Metadata/Items/Gems/SkillGemSkeletalArsonist",
        "Metadata/Items/Gems/SkillGemSkeletalSniper",
        "Metadata/Items/Gems/SkillGemSkeletalFrostMage",
        "Metadata/Items/Gems/SkillGemSkeletalReaver",
        "Metadata/Items/Gems/SkillGemSkeletalWarrior",
    ]
    .iter()
    .map(|key| catalog.gem_by_key(key).unwrap().clone())
    .collect();
    let warrior = reviewed
        .iter()
        .find(|gem| gem.primary_effect_id == WARRIOR)
        .unwrap();
    let hulking = catalog
        .data()
        .gems
        .iter()
        .find(|gem| gem.primary_effect_id == "SupportHulkingMinionsPlayer")
        .unwrap();
    let mut inputs = vec![("original".to_string(), xml.to_string())];
    for (name, attrs, group) in [
        ("count-zero", vec![("count", "0")], vec![]),
        ("count-two", vec![("count", "2")], vec![]),
        ("count-three", vec![("count", "3")], vec![]),
        ("count-four", vec![("count", "4")], vec![]),
        (
            "group-zero",
            vec![("count", "3")],
            vec![("groupCount", "0")],
        ),
        (
            "group-four",
            vec![("count", "3")],
            vec![("groupCount", "4")],
        ),
        ("raw-level-one", vec![("level", "1")], vec![]),
        ("raw-level-nineteen", vec![("level", "19")], vec![]),
    ] {
        inputs.push((name.into(), edit_primary(xml, &attrs, &group, None)));
    }
    inputs.push((
        "duplicates-one-three".into(),
        edit_primary(xml, &[("count", "1")], &[], Some("3")),
    ));
    inputs.push((
        "duplicates-three-one".into(),
        edit_primary(xml, &[("count", "3")], &[], Some("1")),
    ));
    for (name, text) in [
        (
            "reduced-reservation-seven",
            "Minions have 7% reduced Reservation",
        ),
        (
            "efficiency-seven",
            "7% increased Spirit Reservation Efficiency of Skills",
        ),
        ("less-reservation-seven", "Skills reserve 7% less Spirit"),
        (
            "combined-modifiers",
            "Minions have 7% reduced Reservation\n7% increased Spirit Reservation Efficiency of Skills\nSkills reserve 7% less Spirit",
        ),
    ] {
        inputs.push((
            name.into(),
            custom(&edit_primary(xml, &[("count", "3")], &[], None), text),
        ));
    }
    inputs.push(("added-hulking-support".into(), append_support(xml, hulking)));
    for count in [1, 2, 3] {
        inputs.push((
            format!("warrior-free-count-{count}"),
            append_warrior(xml, warrior, count),
        ));
    }
    inputs.push((
        "original-01".into(),
        std::str::from_utf8(&originals[0]).unwrap().to_owned(),
    ));
    inputs.push(("repeat-original".into(), xml.to_string()));
    assert_eq!(inputs.len(), 21);
    let mut cases = vec![];
    for (name, text) in inputs {
        eprintln!(
            "reservation {name}, JIT {}",
            if enabled { "on" } else { "off" }
        );
        cases.push(run(root, &name, &text, enabled, &reviewed));
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","manifest_sha256":pinned::manifest_sha256(),"catalog_digest":catalog_digest,"reviewed":reviewed,
        "native_inventory_authority":false,"native_build_parity":false,"canonical_lifecycle_selected":false,"business_wrappers":false,"copied_arithmetic_as_evidence":false,
        "capture":"nonmutating original floor/round/count/max call-boundary locals, joined to exact final MAIN/CALCS actors and source occurrences",
        "files":(["src/HeadlessWrapper.lua","src/Modules/Build.lua","src/Classes/CalcsTab.lua","src/Classes/SkillsTab.lua","src/Modules/Common.lua","src/Modules/CalcDefence.lua","src/Modules/CalcActiveSkill.lua","src/Modules/CalcPerform.lua","src/Modules/CalcSetup.lua","src/Modules/Calcs.lua","src/Classes/ConfigTab.lua","src/Classes/ModStore.lua","src/Classes/ModList.lua","src/Classes/ModDB.lua","src/Modules/ModParser.lua","src/Modules/Data.lua","src/Data/Minions.lua","src/Data/Skills/act_int.lua","src/Data/Skills/sup_int.lua","src/Data/SkillStatMap.lua","src/Data/Gems.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),"cases":cases});
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
    for (i, original) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            original
        );
    }
    check(&result);
}

fn run(root: &Path, name: &str, xml: &str, enabled: bool, reviewed: &[GemIdentity]) -> Json {
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("reservationXml", xml)?;
        lua.globals()
            .set("reservationReviewed", lua.to_value(reviewed)?)?;
        lua.globals().set("reservationJit", enabled)?;
        lua.load("if reservationJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let authenticate: Function = lua
            .load(include_str!("support/djinn_provider_source.lua"))
            .set_name("@reservation-lifecycle-authentication")
            .eval()?;
        let finish: Function = lua
            .load(CAPTURE)
            .set_name("@reservation-call-capture")
            .eval()?;
        Ok(lua.create_function(move |_, ()| {
            finish.call::<()>(())?;
            authenticate.call::<()>(())
        })?)
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        let mut rebuilt = vec![];
        for _ in 0..2 {
            let finish: Function = lua
                .load(CAPTURE)
                .set_name("@reservation-rebuild-capture")
                .eval()?;
            let result=lua.load(r#"
local function original(f,path,line)local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/");assert(i.what=="Lua"and p:sub(-#path)==path and i.linedefined==line);return f end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
assert(build.buildFlag==false)
local revision=build.outputRevision;local main,calcs=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calcs)
assert(runCallback==callback and build.OnFrame==frame)
"#).set_name("@reservation-original-requested-frame").exec();
            finish.call::<()>(())?;
            result?;
            rebuilt.push(stage(lua)?);
        }
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt[0],"rebuilt_twice":rebuilt[1]}))
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
    .unwrap_or_else(|error| panic!("{name}: {error}"));
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    let states = &result["additional_observation"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([77; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let mut joins = vec![];
    for row in rows(&states["fresh"]["saved"]) {
        let ordinal = row["source_ordinal"].as_u64().unwrap();
        let source = evidence
            .rows()
            .iter()
            .find(|entry| u64::from(entry.occurrence().id().ordinal()) == ordinal)
            .unwrap();
        assert_eq!(source.occurrence().name(), "Gem");
        assert_eq!(
            source.attributes().len(),
            row["attributes"].as_object().unwrap().len()
        );
        for attribute in source.attributes() {
            assert_eq!(
                row["attributes"][&attribute.origin().name],
                attribute.decoded().unwrap()
            );
        }
        joins.push(json!({"source":source.occurrence().id(),"source_ordinal":ordinal,"preset":row["preset"]}));
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"source_joins":joins,"states":states})
}
fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let result: Value = lua
        .load(OBSERVE)
        .set_name("@reservation-exact-source-observer")
        .eval()?;
    Ok(lua.from_value(result)?)
}

fn selected_set<'a, 'input>(doc: &'a roxmltree::Document<'input>) -> roxmltree::Node<'a, 'input> {
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let id = skills.attribute("activeSkillSet").unwrap();
    skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(id))
        .unwrap()
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
fn header(node: roxmltree::Node<'_, '_>, attrs: &[(&str, &str)]) -> String {
    let mut result = format!("<{}", node.tag_name().name());
    for attr in node.attributes() {
        if !attrs.iter().any(|(name, _)| *name == attr.name()) {
            result.push_str(&format!(" {}=\"{}\"", attr.name(), escape(attr.value())));
        }
    }
    for (name, value) in attrs {
        result.push_str(&format!(" {name}=\"{}\"", escape(value)));
    }
    result.push('>');
    result
}
fn edit_primary(
    xml: &str,
    attrs: &[(&str, &str)],
    group_attrs: &[(&str, &str)],
    duplicate: Option<&str>,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let gem = set
        .descendants()
        .find(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(SNIPER))
        .unwrap();
    assert!(gem.children().all(|n| !n.is_element()));
    let mut replacement = format!("{}</Gem>", header(gem, attrs));
    if let Some(count) = duplicate {
        replacement.push_str(&format!("{}</Gem>", header(gem, &[("count", count)])));
    }
    let mut result = xml.to_owned();
    result.replace_range(gem.range(), &replacement);
    if !group_attrs.is_empty() {
        let group = gem.parent().unwrap();
        let start = group.range().start;
        let end = start + xml[start..].find('>').unwrap() + 1;
        result.replace_range(start..end, &header(group, group_attrs));
    }
    result
}
fn gem_xml(gem: &GemIdentity, count: u32, level: u32) -> String {
    format!(
        r#"<Gem gemId="{}" variantId="{}" skillId="{}" nameSpec="{}" level="{level}" quality="0" count="{count}" corrupted="false" corruptLevel="0" enabled="true" enableGlobal1="true" enableGlobal2="true"/>"#,
        escape(&gem.game_id),
        escape(&gem.variant_id),
        escape(&gem.primary_effect_id),
        escape(&gem.name)
    )
}
fn append_support(xml: &str, gem: &GemIdentity) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let primary = set
        .descendants()
        .find(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(SNIPER))
        .unwrap();
    let group = primary.parent().unwrap();
    let at = group.range().end - "</Skill>".len();
    let mut result = xml.to_owned();
    result.insert_str(at, &gem_xml(gem, 1, 1));
    result
}
fn append_warrior(xml: &str, gem: &GemIdentity, count: u32) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let at = set.range().end - "</SkillSet>".len();
    let addition = format!(
        r#"<Skill enabled="true" label="Reservation source control" mainActiveSkill="1" mainActiveSkillCalcs="1">{}</Skill>"#,
        gem_xml(gem, count, 20)
    );
    let mut result = xml.to_owned();
    result.insert_str(at, &addition);
    result
}
fn custom(xml: &str, text: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .descendants()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let id = config.attribute("activeConfigSet").unwrap();
    let set = config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(id))
        .unwrap();
    let at = set.range().end - "</ConfigSet>".len();
    let mut result = xml.to_owned();
    result.insert_str(at,&format!(r#"<CustomModifierBlock title="Reservation witness" enabled="true">{}</CustomModifierBlock>"#,escape(text)));
    result
}
fn rows(value: &Json) -> &[Json] {
    if let Some(array) = value.as_array() {
        array
    } else {
        assert!(
            value.as_object().is_some_and(|map| map.is_empty()),
            "expected empty Lua list: {value}"
        );
        &[]
    }
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

fn selected_action<'a>(state: &'a Json, effect: &str, mode: &str) -> &'a Json {
    let found: Vec<_> = rows(&state["saved"])
        .iter()
        .filter(|row| row["selected"] == true)
        .flat_map(|row| rows(&row[mode]))
        .filter(|action| action["effect"] == effect)
        .collect();
    assert_eq!(found.len(), 1, "baseline source must be unambiguous");
    found[0]
}

fn close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
        "{context}: actual {actual}, expected {expected}"
    );
}

fn modifier_occurrences(action: &Json, channel: &str, kind: &str, value: f64) -> usize {
    rows(&action["modifiers"][channel])
        .iter()
        .filter(|row| {
            row["mod"]["type"] == kind
                && row["value"].as_f64() == Some(value)
                && row["mod"]["source"]
                    .as_str()
                    .is_some_and(|source| !source.is_empty())
        })
        .count()
}

fn check(report: &Json) {
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 21);
    assert_eq!(cases[0]["states"], cases.last().unwrap()["states"]);
    for case in cases {
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &case["states"][stage];
            let expected_selection = if case["name"] == "original-01" {
                json!({"skills":1,"spec":1,"items":1,"config":1,"group":1})
            } else {
                json!({"skills":4,"spec":3,"items":2,"config":1,"group":3})
            };
            assert_eq!(state["selection"], expected_selection);
            for flag in [
                "source_methods_preserved",
                "hook_removed",
                "physical_objects_preserved",
            ] {
                assert_eq!(state[flag], true);
            }
            for row in rows(&state["saved"]) {
                for mode in ["MAIN", "CALCS"] {
                    for action in rows(&row[mode]) {
                        assert_eq!(row["selected"], true);
                        assert_eq!(action["physical_source_exact"], true);
                        assert_eq!(action["group_exact"], true);
                        assert_eq!(action["reservation"]["has_reservation"], true);
                        assert_eq!(action["reservation"]["multiple"], true);
                        assert_eq!(action["reservation"]["becomes_cost"], false);
                        let pool = &action["captured"]["pools"]["Spirit"];
                        assert!(pool["before_count"].is_object());
                        assert!(pool["count_application"].is_object());
                        assert_eq!(pool["count_application"]["count"], action["count"]);
                        assert_eq!(
                            pool["count_application"]["free"],
                            action["reservation"]["free_count"]
                        );
                        let before_count = &pool["before_count"];
                        let values = &before_count["values"];
                        let ordinary_round_branch = action["reservation"]["forced_flat_present"]
                            == false
                            && values["more"].as_f64().unwrap() > 0.0
                            && values["inc"].as_f64().unwrap() > -100.0
                            && values["baseFlat"].as_f64().unwrap() != 0.0
                            && before_count["multiplier"].as_f64().unwrap() != 0.0;
                        if ordinary_round_branch {
                            assert_eq!(pool["flat_round"]["decimals"], 0);
                            assert!(pool["flat_round"]["argument"].as_f64().is_some());
                        } else {
                            assert!(
                                pool.get("flat_round").is_none(),
                                "unexecuted round must remain absent"
                            );
                        }
                        assert_eq!(action["captured"]["multiplier_floor"]["decimals"], 4);
                        assert_eq!(
                            action["captured"]["multiplier_floor"]["argument"],
                            action["inputs"]["reservation_multiplier"]
                        );
                        assert_eq!(
                            pool["before_count"]["values"]["inc"],
                            action["inputs"]["reserved_inc"]
                        );
                        assert_eq!(
                            pool["before_count"]["values"]["more"],
                            action["inputs"]["reserved_more"]
                        );
                        assert_eq!(
                            pool["before_count"]["values"]["efficiencyMore"],
                            action["inputs"]["efficiency_more"]
                        );
                        if action["effect"] == SNIPER {
                            let expected = match case["name"].as_str().unwrap() {
                                "count-zero" | "group-zero" => 0,
                                "count-two" => 2,
                                "count-three"
                                | "duplicates-three-one"
                                | "reduced-reservation-seven"
                                | "efficiency-seven"
                                | "less-reservation-seven"
                                | "combined-modifiers" => 3,
                                "group-four" | "count-four" => 4,
                                _ => 1,
                            };
                            assert_eq!(
                                action["count"], expected,
                                "{} {mode} {stage}",
                                case["name"]
                            );
                            assert_eq!(action["reservation"]["free_count"], 0);
                            let name = case["name"].as_str().unwrap();
                            if matches!(
                                name,
                                "reduced-reservation-seven"
                                    | "efficiency-seven"
                                    | "less-reservation-seven"
                                    | "combined-modifiers"
                            ) {
                                let baseline_case = cases
                                    .iter()
                                    .find(|candidate| candidate["name"] == "count-three")
                                    .unwrap();
                                let baseline =
                                    selected_action(&baseline_case["states"][stage], SNIPER, mode);
                                let reduced = matches!(
                                    name,
                                    "reduced-reservation-seven" | "combined-modifiers"
                                );
                                let efficiency =
                                    matches!(name, "efficiency-seven" | "combined-modifiers");
                                let less =
                                    matches!(name, "less-reservation-seven" | "combined-modifiers");
                                close(
                                    action["inputs"]["reserved_inc"].as_f64().unwrap(),
                                    baseline["inputs"]["reserved_inc"].as_f64().unwrap()
                                        - if reduced { 7.0 } else { 0.0 },
                                    name,
                                );
                                close(
                                    action["inputs"]["efficiency_inc"].as_f64().unwrap(),
                                    baseline["inputs"]["efficiency_inc"].as_f64().unwrap()
                                        + if efficiency { 7.0 } else { 0.0 },
                                    name,
                                );
                                close(
                                    action["inputs"]["reserved_more"].as_f64().unwrap(),
                                    baseline["inputs"]["reserved_more"].as_f64().unwrap()
                                        * if less { 0.93 } else { 1.0 },
                                    name,
                                );
                                for (active, channel, kind, value) in [
                                    (reduced, "Reserved", "INC", -7.0),
                                    (efficiency, "SpiritReservationEfficiency", "INC", 7.0),
                                    (less, "SpiritReserved", "MORE", -7.0),
                                ] {
                                    if active {
                                        assert_eq!(
                                            modifier_occurrences(action, channel, kind, value),
                                            modifier_occurrences(baseline, channel, kind, value)
                                                + 1,
                                            "{name}: original parsed modifier provenance"
                                        );
                                    }
                                }
                            }
                            if name == "added-hulking-support" {
                                let baseline =
                                    selected_action(&cases[0]["states"][stage], SNIPER, mode);
                                let support = rows(&action["accepted_supports"])
                                    .iter()
                                    .find(|support| {
                                        support["effect"] == "SupportHulkingMinionsPlayer"
                                    })
                                    .expect("actual source must accept added Hulking support");
                                let more = support["source_level_row"]["reservationMultiplier"]
                                    .as_f64()
                                    .unwrap();
                                assert_ne!(more, 0.0);
                                close(
                                    action["inputs"]["reservation_multiplier"].as_f64().unwrap(),
                                    baseline["inputs"]["reservation_multiplier"]
                                        .as_f64()
                                        .unwrap()
                                        * (1.0 + more / 100.0),
                                    name,
                                );
                                assert_eq!(
                                    modifier_occurrences(
                                        action,
                                        "ReservationMultiplier",
                                        "MORE",
                                        more
                                    ),
                                    modifier_occurrences(
                                        baseline,
                                        "ReservationMultiplier",
                                        "MORE",
                                        more
                                    ) + 1
                                );
                            }
                        }
                        if action["effect"] == WARRIOR {
                            assert_eq!(action["reservation"]["free_count"], 2);
                        }
                        let count = action["count"].as_f64().unwrap();
                        let free = action["reservation"]["free_count"].as_f64().unwrap();
                        // These cases exercise the flat branch. A skipped or
                        // zero unit reservation must not invent a round call or
                        // a present per-skill output field.
                        assert_eq!(values["reservedPercent"].as_f64().unwrap(), 0.0);
                        if count <= free || values["reservedFlat"].as_f64().unwrap() == 0.0 {
                            assert_eq!(action["reservation"]["spirit_result_present"], false);
                        } else {
                            assert!(
                                action["reservation"]["spirit_result"]
                                    .as_f64()
                                    .is_some_and(|v| v > 0.0)
                            );
                        }
                    }
                }
            }
        }
        let mut first = case["states"]["rebuilt_once"].clone();
        let second = &case["states"]["rebuilt_twice"];
        assert_eq!(
            first["output_revision"].as_u64().unwrap() + 1,
            second["output_revision"].as_u64().unwrap()
        );
        first["output_revision"] = second["output_revision"].clone();
        assert_eq!(&first, second);
    }
}
