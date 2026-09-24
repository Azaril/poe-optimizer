//! Complete authenticated source loading of reviewed active physical Gems.
//! Natural/default levels, effect table keys and saved input behavior are distinct
//! observations; none proves physical game legality or native action activation.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::skill_identities::{GemIdentity, SkillIdentityCatalog};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn complete_source_active_gem_inputs_preserve_physical_levels_quality_and_identity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-active-gem-inputs-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os("POE_ACTIVE_GEM_INPUT_SOURCE_CHILD") {
        assert!(mode == "on" || mode == "off");
        let jit_enabled = mode == "on";
        let reviewed = reviewed_inputs(&root);
        let cases = input_cases();
        let supplied: Vec<_> = cases
            .iter()
            .map(|case| {
                json!({
                    "label":case.label, "attributes":case.attributes,
                })
            })
            .collect();
        // This existing build supplies the complete original runtime. Every
        // observation below creates a separate physical group from injected IDs.
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("activeGemJitEnabled", jit_enabled)?;
            lua.globals()
                .set("reviewedActiveGems", lua.to_value(&reviewed)?)?;
            lua.globals()
                .set("activeGemCases", lua.to_value(&supplied)?)?;
            lua.load("if activeGemJitEnabled then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let result = source::observe_with_build_hook_unwrapped(
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
        // Preserve actual complete-method observations before asserting the
        // provisional domain expectations derived from pinned source inspection.
        fs::write(
            destination.join(if jit_enabled {
                "source-jit-on.json"
            } else {
                "source-jit-off.json"
            }),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_observations(&reviewed, &cases, &result["additional_observation"]);
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "complete_source_active_gem_inputs_preserve_physical_levels_quality_and_identity",
                "--nocapture",
            ])
            .env("POE_ACTIVE_GEM_INPUT_SOURCE_CHILD", mode)
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
    let off: Json = read(&destination, "source-jit-off.json");
    let on: Json = read(&destination, "source-jit-on.json");
    assert_eq!(off["source_hash"], on["source_hash"]);
    assert!(
        off["additional_observation"] == on["additional_observation"],
        "JIT modes differ in active physical Gem input observations"
    );
}

fn read<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> T {
    serde_json::from_slice(&fs::read(root.join(relative)).unwrap()).unwrap()
}

// Policy selects a finite scope; all role/topology facts are independently
// obtained from the validated catalog and then checked against actual source
// objects. No native schema compiler or evaluator participates in this oracle.
fn reviewed_inputs(root: &Path) -> Vec<GemIdentity> {
    let catalog = SkillIdentityCatalog::new(read(
        root,
        "data/owned/poe2/3887ae68/import/skill-identities.json",
    ))
    .unwrap();
    let digest = serde_json::to_value(
        digest_owned(
            "owned-skill-source-catalog-v1",
            catalog.data(),
            64 * 1024 * 1024,
        )
        .unwrap(),
    )
    .unwrap();
    let mut selected = BTreeMap::new();
    for (file, expected_count, multiple) in [
        ("singleton.json", 26, false),
        ("multieffect.json", 10, true),
    ] {
        let policy: Json = read(
            root,
            &format!("data/owned/poe2/3887ae68/active-gem-inputs/{file}"),
        );
        assert_eq!(policy["catalog_digest"], digest);
        let keys: Vec<String> = serde_json::from_value(policy["source_gems"].clone()).unwrap();
        assert_eq!(keys.len(), expected_count);
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        for key in keys {
            let gem = catalog.gem_by_key(&key).unwrap();
            let primary = catalog.skill_by_id(&gem.primary_effect_id).unwrap();
            assert_ne!(primary.support, Some(true));
            assert_ne!(primary.from_tree, Some(true));
            assert_eq!(gem.effect_list.len() > 1, multiple);
            assert!(gem.declared_additional_stat_sets.is_empty());
            assert!(
                !catalog
                    .data()
                    .missing_references
                    .iter()
                    .any(|row| row.gem_key == key)
            );
            let declared: BTreeSet<_> = gem
                .declared_additional_effects
                .iter()
                .map(|row| row.id.as_str())
                .collect();
            let constructed: BTreeSet<_> = gem
                .constructed_additional_effects
                .iter()
                .map(|row| row.id.as_str())
                .collect();
            let additional: BTreeSet<_> =
                gem.additional_effects.iter().map(String::as_str).collect();
            let effects: BTreeSet<_> = gem.effect_list.iter().map(String::as_str).collect();
            assert_eq!(declared.len(), gem.declared_additional_effects.len());
            assert_eq!(constructed.len(), gem.constructed_additional_effects.len());
            assert_eq!(additional.len(), gem.additional_effects.len());
            assert_eq!(effects.len(), gem.effect_list.len());
            assert!(declared.is_subset(&constructed));
            assert_eq!(constructed, additional);
            let mut expected = additional;
            assert!(expected.insert(&gem.primary_effect_id));
            assert_eq!(expected, effects);
            assert!(effects.iter().all(|id| catalog.skill_by_id(id).is_some()));
            assert!(selected.insert(key, gem.clone()).is_none());
        }
    }
    assert_eq!(selected.len(), 36);
    selected.into_values().collect()
}

struct InputCase {
    label: &'static str,
    attributes: Json,
    level_before: Option<f64>,
    level_after: Option<f64>,
    quality: Option<f64>,
    succeeds: bool,
    corrupted: bool,
    corruption_delta: f64,
}
fn input_cases() -> Vec<InputCase> {
    let base = || json!({"level":"20", "quality":"20", "corrupted":"false", "corruptLevel":"0"});
    let mut cases = vec![];
    // These are explicit finite witnesses, not a reimplementation of source
    // validation. A table maximum is not the physical gameplay level cap.
    for (label, text, before, after) in [
        ("level-one", Some("1"), Some(1.0), Some(1.0)),
        ("level-twenty", Some("20"), Some(20.0), Some(20.0)),
        ("level-twenty-one", Some("21"), Some(21.0), Some(21.0)),
        ("level-forty", Some("40"), Some(40.0), Some(40.0)),
        ("level-zero", Some("0"), Some(0.0), Some(1.0)),
        ("level-negative", Some("-3"), Some(-3.0), Some(1.0)),
        ("level-forty-one", Some("41"), Some(41.0), Some(40.0)),
        ("level-fractional", Some("1.5"), Some(1.5), Some(20.0)),
        ("level-missing", None, None, None),
        ("level-malformed", Some("bad"), None, None),
        ("level-literal-nil", Some("nil"), None, None),
    ] {
        let mut attributes = base();
        if let Some(text) = text {
            attributes["level"] = text.into();
        } else {
            attributes.as_object_mut().unwrap().remove("level");
        }
        cases.push(InputCase {
            label,
            attributes,
            level_before: before,
            level_after: after,
            quality: Some(20.0),
            succeeds: after.is_some(),
            corrupted: false,
            corruption_delta: 0.0,
        });
    }
    // Saved quality is numerically loaded without the UI default's 0..23 clamp.
    // Negative and beyond-envelope values are source-only contrasts, not owned
    // admission or an asserted gameplay quality range.
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
        if let Some(text) = text {
            attributes["quality"] = text.into();
        } else {
            attributes.as_object_mut().unwrap().remove("quality");
        }
        cases.push(InputCase {
            label,
            attributes,
            level_before: Some(20.0),
            level_after: Some(20.0),
            quality,
            succeeds: true,
            corrupted: false,
            corruption_delta: 0.0,
        });
    }
    // Corruption is stored independently of physical level during these methods;
    // effective level adjustment happens later and is deliberately not asserted.
    for (label, flag, delta, corrupted, corruption_delta) in [
        ("corrupt-false-positive", "false", "1", false, 1.0),
        ("corrupt-true-positive", "true", "1", true, 1.0),
        ("corrupt-false-negative", "false", "-1", false, -1.0),
        ("corrupt-true-negative", "true", "-1", true, -1.0),
        ("corrupt-true-fractional", "true", "0.5", true, 0.5),
        ("corrupt-literal-nil", "nil", "nil", false, 0.0),
    ] {
        let mut attributes = base();
        attributes["corrupted"] = flag.into();
        attributes["corruptLevel"] = delta.into();
        cases.push(InputCase {
            label,
            attributes,
            level_before: Some(20.0),
            level_after: Some(20.0),
            quality: Some(20.0),
            succeeds: true,
            corrupted,
            corruption_delta,
        });
    }
    assert_eq!(cases.len(), 29);
    cases
}

fn assert_observations(reviewed: &[GemIdentity], cases: &[InputCase], observation: &Json) {
    let catalog = observation["catalog"].as_array().unwrap();
    let rows = observation["load_cases"].as_array().unwrap();
    assert_eq!(catalog.len(), 36);
    assert_eq!(rows.len(), 36 * cases.len());
    let expected_levels = json!((1..=40).collect::<Vec<u32>>());
    for (expected, actual) in reviewed.iter().zip(catalog) {
        assert_eq!(actual["id"], expected.key);
        assert_eq!(actual["primary"], expected.primary_effect_id);
        assert_eq!(actual["game_id"], expected.game_id);
        assert_eq!(actual["variant_id"], expected.variant_id);
        assert_eq!(actual["primary_data_identity"], true);
        assert_eq!(actual["primary_support"], false);
        assert_eq!(actual["primary_from_tree"], false);
        assert_eq!(actual["natural_max_level"], 20);
        assert_eq!(actual["primary_level_keys"], expected_levels);
        assert_eq!(actual["first_stat_set_level_keys"], expected_levels);
        assert_eq!(actual["effects"], json!(expected.effect_list));
        assert_eq!(
            actual["primary_index"].as_u64(),
            expected
                .effect_list
                .iter()
                .position(|id| id == &expected.primary_effect_id)
                .map(|index| index as u64 + 1)
        );
        let additional = actual["additional"].as_array();
        assert_eq!(
            additional.map_or(0, Vec::len),
            expected.additional_effects.len()
        );
        if let Some(additional) = additional {
            for (expected_id, actual) in expected.additional_effects.iter().zip(additional) {
                assert_eq!(actual["id"], *expected_id);
                assert!(!actual["level_keys"].as_array().unwrap().is_empty());
            }
        }
    }
    for (expected, per_gem) in reviewed.iter().zip(rows.chunks_exact(cases.len())) {
        for (case, row) in cases.iter().zip(per_gem) {
            let context = format!("{}/{}", expected.key, case.label);
            assert_eq!(row["id"], expected.key, "{context}");
            assert_eq!(row["label"], case.label, "{context}");
            assert_eq!(
                row["ok"].as_bool(),
                Some(case.succeeds),
                "{context}: {}",
                row["error"]
            );
            assert_eq!(
                row["before"]["level"].as_f64(),
                case.level_before,
                "{context}"
            );
            assert_eq!(row["before"]["quality"].as_f64(), case.quality, "{context}");
            assert_eq!(
                row["before"]["corrupted"].as_bool(),
                Some(case.corrupted),
                "{context}"
            );
            assert_eq!(
                row["before"]["corrupt_level"].as_f64(),
                Some(case.corruption_delta),
                "{context}"
            );
            assert_eq!(row["before"]["gem_id"], expected.key, "{context}");
            assert_eq!(
                row["before"]["skill_id"], expected.primary_effect_id,
                "{context}"
            );
            if case.succeeds {
                assert_eq!(row["same_gem_data"], true, "{context}");
                assert_eq!(row["physical_gems"], 1, "{context}");
                assert_eq!(
                    row["after"]["level"].as_f64(),
                    case.level_after,
                    "{context}"
                );
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
                assert_eq!(
                    row["reprocessed"], row["after"],
                    "{context}: repeated processing changed physical inputs"
                );
                assert_eq!(row["effects"], json!(expected.effect_list), "{context}");
            } else {
                assert!(row["after"].is_null(), "{context}");
                assert_eq!(row["published_groups"], 0, "{context}");
                assert!(
                    row["error"].as_str().is_some_and(|error| !error.is_empty()),
                    "{context}"
                );
            }
        }
    }
    assert_eq!(rows.iter().filter(|row| row["ok"] == false).count(), 36 * 3);
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-active-gem-input-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
const OBSERVATION: &str = r#"
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line,
  "unexpected complete source method: "..path..":"..tostring(info.linedefined))
 return f
end
local loadSkill=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local function fields(gem)
 return {level=gem.level,quality=gem.quality,corrupted=gem.corrupted,corrupt_level=gem.corruptLevel,
  gem_id=gem.gemId,skill_id=gem.skillId}
end
local function levelKeys(effect)
 local keys={};for key in pairs(effect.levels) do keys[#keys+1]=key end
 table.sort(keys);return keys
end
local function effects(gem)
 local ids={};for index,effect in ipairs(gem.grantedEffectList) do ids[index]=effect.id end
 return ids
end
local function load(attributes,gem)
 local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
 local row={}
 function tab:ProcessSocketGroup(group)
  row.before=fields(group.gemList[1])
  -- This observer delegates the complete original method and retains failures.
  local result=process(self,group)
  row.after=fields(group.gemList[1]);return result
 end
 local ok,err=pcall(loadSkill,tab,{elem="Skill",attrib={enabled="true"},{elem="Gem",attrib=attributes}},1)
 row.ok=ok;row.error=not ok and tostring(err) or nil
 local groups=tab.skillSets[1].socketGroupList;row.published_groups=#groups
 local group=groups[1]
 if group then
  local physical=group.gemList[1]
  row.physical_gems=#group.gemList;row.same_gem_data=physical.gemData==gem
  row.effects=effects(gem)
  -- Same physical group, same complete original processing method. This is
  -- explicit object reuse, independently of the LuaJIT on/off process lane.
  process(tab,group);row.reprocessed=fields(physical)
 end
 assert(build.skillsTab.LoadSkill==loadSkill and build.skillsTab.ProcessSocketGroup==process)
 return row
end
local result={catalog={},load_cases={}}
for _,reviewed in ipairs(reviewedActiveGems) do
 local gem=data.gems[reviewed.key];assert(gem and gem.id==reviewed.key)
 local additional={};for index,effect in ipairs(gem.additionalGrantedEffects) do
  additional[index]={id=effect.id,level_keys=levelKeys(effect),support=effect.support==true,
   hidden=effect.hideFromSideBar==true,global=effect.hasGlobalEffect==true}
 end
 local primaryIndex
 for index,effect in ipairs(gem.grantedEffectList) do
  assert(effect==data.skills[effect.id])
  if effect==gem.grantedEffect then primaryIndex=index end
 end
 result.catalog[#result.catalog+1]={id=gem.id,primary=gem.grantedEffectId,game_id=gem.gameId,variant_id=gem.variantId,
  primary_data_identity=gem.grantedEffect==data.skills[reviewed.primary_effect_id],
  natural_max_level=gem.naturalMaxLevel,primary_support=gem.grantedEffect.support==true,
  primary_from_tree=gem.grantedEffect.fromTree==true,primary_level_keys=levelKeys(gem.grantedEffect),
  first_stat_set_level_keys=gem.grantedEffect.statSets[1] and levelKeys(gem.grantedEffect.statSets[1]),
  primary_index=primaryIndex,effects=effects(gem),additional=additional}
 for _,case in ipairs(activeGemCases) do
  local attributes={};for name,value in pairs(case.attributes) do attributes[name]=value end
  attributes.gemId=gem.gameId;attributes.variantId=gem.variantId
  local row=load(attributes,gem);row.id=gem.id;row.label=case.label;row.attributes=attributes
  result.load_cases[#result.load_cases+1]=row
 end
end
return result
"#;
