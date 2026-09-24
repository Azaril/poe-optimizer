//! Complete-source observations of physical support inputs and potential effects.
//! Derived active effects retain the same physical source instance; this does not
//! grant native skill activation, support compatibility, or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::ParameterValue, owned_content::digest_owned,
};
use poe_optimizer_data::skill_identities::{GemIdentity, SkillIdentityCatalog};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::SourceAttributeRef,
    owned_value_policy::{
        CandidateValue, MissingValuePolicy, ValueCandidate, ValueLane, ValueOutcome, ValueRecipe,
        ValueRecipeInput,
    },
};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::Value as Json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn complete_source_multieffect_support_inputs_preserve_identity_and_conditional_levels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-multieffect-gem-inputs-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os("POE_MULTIEFFECT_GEM_SOURCE_CHILD") {
        assert!(mode == "on" || mode == "off");
        let jit_enabled = mode == "on";
        let (reviewed, recipes) = reviewed_inputs(&root);
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("multieffectGemJitEnabled", jit_enabled)?;
            lua.globals()
                .set("reviewedMultieffectGems", lua.to_value(&reviewed)?)?;
            lua.load("if multieffectGemJitEnabled then jit.on() else jit.off();jit.flush() end")
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
        compare_native_inputs(&recipes, &result["additional_observation"]);
        fs::write(
            destination.join(if jit_enabled {
                "source-jit-on.json"
            } else {
                "source-jit-off.json"
            }),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "complete_source_multieffect_support_inputs_preserve_identity_and_conditional_levels", "--nocapture"])
            .env("POE_MULTIEFFECT_GEM_SOURCE_CHILD", mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap())).stderr(Stdio::from(log)).spawn().unwrap();
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
            if started.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", log_path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off: Json =
        serde_json::from_slice(&fs::read(destination.join("source-jit-off.json")).unwrap())
            .unwrap();
    let on: Json =
        serde_json::from_slice(&fs::read(destination.join("source-jit-on.json")).unwrap()).unwrap();
    assert_eq!(off["source_hash"], on["source_hash"]);
    assert_eq!(off["additional_observation"], on["additional_observation"]);
}

fn read<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> T {
    serde_json::from_slice(&fs::read(root.join(relative)).unwrap()).unwrap()
}
fn reviewed_inputs(root: &Path) -> (Vec<GemIdentity>, BTreeMap<String, ValueRecipeInput>) {
    let policy: Json = read(
        root,
        "data/owned/poe2/3887ae68/multieffect-support-gem-inputs/policy.json",
    );
    let catalog = SkillIdentityCatalog::new(read(
        root,
        "data/owned/poe2/3887ae68/import/skill-identities.json",
    ))
    .unwrap();
    assert_eq!(
        policy["catalog_digest"],
        serde_json::to_value(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
        .unwrap()
    );
    let skills: BTreeMap<_, _> = catalog
        .data()
        .skills
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect();
    let mut selected: Vec<_> = catalog
        .data()
        .gems
        .iter()
        .filter(|gem| {
            let primary = skills[gem.primary_effect_id.as_str()];
            primary.support == Some(true)
                && primary.from_tree != Some(true)
                && gem.effect_list.len() > 1
                && gem.declared_additional_stat_sets.is_empty()
                && gem
                    .effect_list
                    .iter()
                    .all(|effect| skills.contains_key(effect.as_str()))
                && !catalog
                    .data()
                    .missing_references
                    .iter()
                    .any(|missing| missing.gem_key == gem.key)
        })
        .cloned()
        .collect();
    selected.sort_by(|a, b| a.key.cmp(&b.key));
    let authored: Vec<String> = serde_json::from_value(policy["source_gems"].clone()).unwrap();
    assert_eq!(
        authored,
        selected
            .iter()
            .map(|gem| gem.key.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(selected.len(), 52);
    assert_eq!(
        selected
            .iter()
            .map(|gem| gem.effect_list.len())
            .sum::<usize>(),
        105
    );
    assert_eq!(
        selected
            .iter()
            .filter(|gem| gem.effect_list.len() == 2)
            .count(),
        51
    );
    assert_eq!(
        selected
            .iter()
            .filter(|gem| gem.declared_additional_effects.is_empty()
                && !gem.constructed_additional_effects.is_empty())
            .count(),
        3
    );
    let primary_last: Vec<_> = selected
        .iter()
        .filter(|gem| gem.effect_list.first() != Some(&gem.primary_effect_id))
        .collect();
    assert_eq!(primary_last.len(), 1);
    assert_eq!(
        primary_last[0].key,
        "Metadata/Items/Gems/SkillGemEmpoweredSparksSupport"
    );
    assert_eq!(
        primary_last[0].effect_list.last(),
        Some(&primary_last[0].primary_effect_id)
    );
    assert!(
        !selected
            .iter()
            .any(|gem| gem.key == "Metadata/Items/Gems/SkillGemConcussiveRunesSupport")
    );
    let mut recipes = BTreeMap::new();
    for parameter in policy["parameters"].as_array().unwrap() {
        let recipe: ValueRecipeInput = serde_json::from_value(parameter["value"].clone()).unwrap();
        assert!(matches!(recipe.missing, MissingValuePolicy::Pending));
        assert_eq!(recipe.tiers.len(), 1);
        assert_eq!(recipe.tiers[0].selectors.len(), 1);
        let selector = &recipe.tiers[0].selectors[0];
        assert_eq!(selector.lane, ValueLane::Attribute);
        assert!(recipes.insert(selector.name.clone(), recipe).is_none());
    }
    assert_eq!(recipes.len(), 2);
    (selected, recipes)
}

fn compare_native_inputs(recipes: &BTreeMap<String, ValueRecipeInput>, observation: &Json) {
    let rows = observation["load_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 52 * 8);
    for row in rows {
        for (attribute, field) in [
            ("corrupted", "corrupted"),
            ("corruptLevel", "corrupt_level"),
        ] {
            let input = &recipes[attribute];
            let text = row["attributes"][attribute].as_str().unwrap();
            let xml = format!("<PathOfBuilding2><Gem {attribute}=\"{text}\"/></PathOfBuilding2>");
            let evidence = ImportedBuildInstance::from_decoded(
                decode_build(xml.as_bytes()).unwrap(),
                BuildLineage::from_bytes([0x7a; 16]),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let origin = SourceAttributeRef {
                occurrence: evidence
                    .occurrences()
                    .iter()
                    .find(|source| source.name() == "Gem")
                    .unwrap()
                    .id(),
                index: 0,
            };
            let recipe = ValueRecipe::new(input.clone(), Default::default()).unwrap();
            let outcome = recipe
                .decide(&[ValueCandidate {
                    selector: &input.tiers[0].selectors[0],
                    origin,
                    value: CandidateValue::Decoded(text),
                }])
                .unwrap()
                .outcome;
            let ValueOutcome::Selected {
                origin: selected,
                value,
            } = outcome
            else {
                panic!("reviewed source input must remain selected")
            };
            assert_eq!(selected, origin);
            for state in ["before", "after"] {
                match &value {
                    ParameterValue::Boolean(value) => {
                        assert_eq!(row[state][field].as_bool(), Some(*value))
                    }
                    ParameterValue::Quantity(value) => {
                        assert_eq!(row[state][field].as_f64(), Some(value.value()))
                    }
                    _ => panic!("unexpected multi-effect scalar input"),
                }
                assert_eq!(row[state]["level"].as_f64(), Some(1.0));
                assert_eq!(
                    row[state]["quality"].as_f64(),
                    row["attributes"]["quality"]
                        .as_str()
                        .map(|s| s.parse::<f64>().unwrap())
                );
            }
        }
    }
    assert_eq!(observation["catalog"].as_array().unwrap().len(), 52);
    assert_eq!(observation["setups"].as_array().unwrap().len(), 52 * 6);
    assert_eq!(observation["additional_level_tables"]["forty"], 29);
    assert_eq!(observation["additional_level_tables"]["single"], 24);
    assert!(observation["inherited_effects"].as_u64().unwrap() > 0);
    assert!(observation["unlinked_effects"].as_u64().unwrap() > 0);
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-multieffect-gem-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
const OBSERVATION: &str = r#"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line)
 return f
end
local loadSkill=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
original(calcs.initEnv,"Modules/CalcSetup.lua",717)
original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local function fields(gem)
 return {level=gem.level,quality=gem.quality,corrupted=gem.corrupted,corrupt_level=gem.corruptLevel,
  gem_id=gem.gemId,skill_id=gem.skillId}
end
local function load(attributes)
 local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
 local row={attributes=attributes}
 function tab:ProcessSocketGroup(group)
  row.before=fields(group.gemList[1]);local result=process(self,group);row.after=fields(group.gemList[1]);return result
 end
 loadSkill(tab,{elem="Skill",attrib={enabled="true"},{elem="Gem",attrib=attributes}},1)
 assert(build.skillsTab.LoadSkill==loadSkill and build.skillsTab.ProcessSocketGroup==process)
 return row,tab.skillSets[1].socketGroupList[1]
end
local function supportLoad(gem,case)
 local row,group=load({gemId=gem.gameId,variantId=gem.variantId,level="1",quality=tostring(case.quality or 20),
  corrupted=case.flag or "false",corruptLevel=case.delta or "0"})
 assert(group and #group.gemList==1 and group.gemList[1].gemData==gem)
 assert(row.after.gem_id==gem.id and row.after.skill_id==gem.grantedEffectId)
 assert(row.before.level==1 and row.after.level==1 and row.after.corrupted==(case.flag=="true"))
 assert(row.before.corrupt_level==(tonumber(case.delta) or 0) and row.after.corrupt_level==row.before.corrupt_level)
 row.id=gem.id;row.label=case.label
 return row,group
end
local result={catalog={},load_cases={},setups={},additional_level_tables={forty=0,single=0},inherited_effects=0,unlinked_effects=0}
local selected={};local references=0;local constructedOnly=0;local reordered=0
assert(#reviewedMultieffectGems==52)
for _,reviewed in ipairs(reviewedMultieffectGems) do
 local gem=data.gems[reviewed.key];assert(gem and not selected[gem.id]);selected[gem.id]=true
 assert(gem.grantedEffectId==reviewed.primary_effect_id and gem.grantedEffect==data.skills[reviewed.primary_effect_id])
 assert(gem.grantedEffect.support and not gem.grantedEffect.fromTree and gem.naturalMaxLevel==1)
 local primaryLevels=0;for key in pairs(gem.grantedEffect.levels) do assert(key==1);primaryLevels=primaryLevels+1 end;assert(primaryLevels==1)
 assert(#gem.grantedEffectList==#reviewed.effect_list and #gem.additionalGrantedEffects==#reviewed.additional_effects)
 local effects={};local primaryIndex
 for index,effect in ipairs(gem.grantedEffectList) do
  assert(effect==data.skills[reviewed.effect_list[index]]);effects[index]=effect.id;references=references+1
  if effect==gem.grantedEffect then primaryIndex=index end
 end
 assert(primaryIndex)
 if primaryIndex~=1 then reordered=reordered+1;assert(gem.id=="Metadata/Items/Gems/SkillGemEmpoweredSparksSupport" and primaryIndex==3) end
 local additional={}
 for index,effect in ipairs(gem.additionalGrantedEffects) do
  assert(effect.id==reviewed.additional_effects[index] and not effect.support)
  local count=0;for key in pairs(effect.levels) do assert(type(key)=="number" and key>=1);count=count+1 end
  assert(count==1 or count==40)
  if count==40 then result.additional_level_tables.forty=result.additional_level_tables.forty+1 else result.additional_level_tables.single=result.additional_level_tables.single+1 end
  additional[index]={id=effect.id,level_count=count,hidden=effect.hideFromSideBar==true,global=effect.hasGlobalEffect==true}
 end
 if #reviewed.declared_additional_effects==0 then constructedOnly=constructedOnly+1;assert(#reviewed.constructed_additional_effects>0 and gem.name:sub(1,7)=="Barbs I") end
 result.catalog[#result.catalog+1]={id=gem.id,primary=gem.grantedEffectId,primary_index=primaryIndex,effects=effects,additional=additional}
 for _,case in ipairs({{label="quality-zero",quality=0,flag="false",delta="0"},{label="quality-twenty",quality=20,flag="false",delta="0"},
  {label="negative-false",flag="false",delta="-1"},{label="negative-true",flag="true",delta="-1"},
  {label="fractional-false",flag="false",delta="0.5"},{label="fractional-true",flag="true",delta="0.5"},
  {label="nil-false",flag="false",delta="nil"},{label="nil-true",flag="true",delta="nil"}}) do
  local row=supportLoad(gem,case);result.load_cases[#result.load_cases+1]=row
 end
end
assert(references==105 and constructedOnly==3 and reordered==1)
assert(result.additional_level_tables.forty==29 and result.additional_level_tables.single==24)
local excluded=data.gems["Metadata/Items/Gems/SkillGemConcussiveRunesSupport"]
assert(excluded and not selected[excluded.id] and excluded.additionalGrantedEffectId1=="ConcussiveRunesPlayer")
assert(data.skills.ConcussiveRunesPlayer==nil and #excluded.grantedEffectList==1)
local savedGroups,savedMain=build.skillsTab.socketGroupList,build.mainSocketGroup
for _,reviewed in ipairs(reviewedMultieffectGems) do
 local gem=data.gems[reviewed.key]
 for _,case in ipairs({{label="anchor-twenty",anchor=20,flag="false",delta="0"},{label="anchor-ten",anchor=10,flag="false",delta="0"},
  {label="negative-false",anchor=20,flag="false",delta="-1"},{label="negative-true",anchor=20,flag="true",delta="-1"},
  {label="fractional-false",anchor=20,flag="false",delta="0.5"},{label="fractional-true",anchor=20,flag="true",delta="0.5"}}) do
  local _,group=load({skillId="TwisterPlayer",level=tostring(case.anchor),quality="0",corrupted="false",corruptLevel="0"})
  local _,supportGroup=supportLoad(gem,case)
  local anchor=group.gemList[1];local physical=supportGroup.gemList[1];group.gemList[2]=physical
  build.skillsTab.socketGroupList={group};build.mainSocketGroup=1
  -- A fresh complete environment each time; no accelerated specEnv/cache input.
  local ok,env=pcall(calcs.initEnv,build,"MAIN")
  assert(ok,gem.id.."/"..case.label..": "..tostring(env))
  assert(#group.gemList==2 and physical.gemData==gem and physical.level==1)
  local support=physical.supportEffect
  assert(support and support.srcInstance==physical and support.grantedEffect==gem.grantedEffect and support.level==1)
  local anchorEffect
  for _,skill in ipairs(env.player.activeSkillList) do if skill.activeEffect.srcInstance==anchor then anchorEffect=skill.activeEffect end end
  assert(anchorEffect and anchor.level==case.anchor)
  local inherited=support.activeSkillLevel
  if inherited then assert(support.isSupporting[anchor] and inherited==anchorEffect.level) else assert(not support.isSupporting[anchor]) end
  local observed={};local expected={}
  for index,effect in ipairs(gem.grantedEffectList) do
   if not effect.support and not effect.hideFromSideBar and (not effect.hasGlobalEffect or physical["enableGlobal"..index]) then expected[#expected+1]=effect.id end
  end
  for _,skill in ipairs(env.player.activeSkillList) do
   local effect=skill.activeEffect
   if effect.srcInstance==physical then
    assert(effect.gemData==gem and not effect.grantedEffect.support and effect.grantedEffect==data.skills[effect.grantedEffect.id])
    assert(effect.grantedEffect.levels[effect.level])
    local count=#effect.grantedEffect.levels
    local propertyLevel=0
    for _,mod in ipairs(effect.gemPropertyInfo or {}) do if mod.value.key=="level" then propertyLevel=propertyLevel+mod.value.value end end
    -- Supported effects inherit the anchor, then their own level table limits
    -- the value. Unlinked effects can independently receive carrier item mods;
    -- only the zero-property contrast asserts the natural-level fallback.
    local expectedLevel=inherited and math.min(inherited,count) or (propertyLevel==0 and 1)
    if expectedLevel then assert(effect.level==expectedLevel,gem.id.."/"..case.label.."/"..effect.grantedEffect.id.." level "..effect.level.." expected "..expectedLevel) end
    if inherited then result.inherited_effects=result.inherited_effects+1 else result.unlinked_effects=result.unlinked_effects+1 end
    observed[#observed+1]={id=effect.grantedEffect.id,level=effect.level,quality=effect.quality,property_level=propertyLevel,
     corrupted=physical.corrupted,corrupt_level=physical.corruptLevel,same_physical_instance=effect.srcInstance==physical}
   end
  end
  assert(#observed==#expected)
  for index,id in ipairs(expected) do assert(observed[index].id==id) end
  result.setups[#result.setups+1]={id=gem.id,label=case.label,physical=fields(physical),anchor_id=anchorEffect.grantedEffect.id,anchor_raw_level=anchor.level,
   anchor_effect_level=anchorEffect.level,primary_support_level=support.level,inherited_level=inherited,
   supports_anchor=support.isSupporting[anchor]==true,effects=observed}
 end
end
build.skillsTab.socketGroupList=savedGroups;build.mainSocketGroup=savedMain
assert(result.inherited_effects>0 and result.unlinked_effects>0)
return result
"#;
