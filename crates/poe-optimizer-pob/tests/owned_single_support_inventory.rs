//! Complete unchanged source loading witnesses physical input accounting only.
//! This does not close native declarations, support routing, or build evaluation.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
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

const TEST: &str = "complete_source_single_support_inventory_accounts_for_saved_fields";
const CHILD: &str = "POE_SINGLE_SUPPORT_INVENTORY_CHILD";
const HASHES: [&str; 5] = [
    "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194",
    "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
    "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490",
    "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711",
    "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
];

#[test]
fn complete_source_single_support_inventory_accounts_for_saved_fields() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-single-support-inventory-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let reviewed = reviewed_gems(&root);
        let mut originals = Vec::new();
        for (index, expected_hash) in HASHES.iter().enumerate() {
            let original = index + 1;
            let path = root.join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{original:02}.xml"
            ));
            let xml = fs::read_to_string(&path).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(xml.as_bytes())),
                *expected_hash
            );
            let before = |lua: &Lua| {
                lua.globals().set("inventoryXml", xml.as_str())?;
                lua.globals().set("inventoryOriginal", original)?;
                lua.globals().set("inventoryJit", enabled)?;
                lua.globals()
                    .set("inventoryReviewed", lua.to_value(&reviewed)?)?;
                lua.load("if inventoryJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                &xml,
                None,
                false,
                Some(&before),
                None,
                Some(&observe),
            )
            .unwrap();
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            assert_eq!(fs::read_to_string(path).unwrap(), xml);
            originals.push(json!({
                "original":original,"xml_sha256":expected_hash,
                "source_hash":result["source_hash"],"state":result["additional_observation"],
            }));
        }
        let result = json!({
            "manifest_sha256":pinned::manifest_sha256(),
            "files":(["src/Classes/SkillsTab.lua","src/Modules/CalcSetup.lua",
                "src/Modules/CalcTools.lua","src/Modules/CalcDefence.lua"].map(|path|
                json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
            "reviewed_definitions":reviewed.len(),"originals":originals,
            "native_inventory_authority":false,"native_build_parity":false,
        });
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        check(&result);
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}

fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn reviewed_gems(root: &Path) -> Vec<String> {
    let policy = read(&root.join("data/owned/poe2/3887ae68/support-gem-inputs/policy.json"));
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_value(read(
            &root.join("data/owned/poe2/3887ae68/import/skill-identities.json"),
        ))
        .unwrap(),
    )
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
    let effects: BTreeMap<_, _> = catalog
        .data()
        .skills
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect();
    let mut expected: Vec<_> = catalog
        .data()
        .gems
        .iter()
        .filter(|gem| {
            let effect = effects[gem.primary_effect_id.as_str()];
            effect.support == Some(true)
                && effect.from_tree != Some(true)
                && gem.declared_additional_effects.is_empty()
                && gem.constructed_additional_effects.is_empty()
                && gem.additional_effects.is_empty()
                && gem.declared_additional_stat_sets.is_empty()
                && gem.effect_list == [gem.primary_effect_id.clone()]
        })
        .map(|gem| gem.key.clone())
        .collect();
    expected.sort();
    let selected: Vec<String> = serde_json::from_value(policy["source_gems"].clone()).unwrap();
    assert_eq!(selected, expected);
    assert_eq!(selected.len(), 514);
    selected
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-single-support-inventory-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let mut definitions = BTreeSet::new();
    let mut selected = 0;
    for (original, (expected, full)) in
        rows(&result["originals"])
            .iter()
            .zip([(43, 43), (28, 90), (47, 47), (46, 46), (16, 111)])
    {
        let state = &original["state"];
        for field in [
            "saved_gems_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
            "fresh_objects",
            "reused_tab_fresh_instances",
        ] {
            assert_eq!(
                state[field], true,
                "original{}/{field}",
                original["original"]
            );
        }
        let actual = rows(&state["selected"]);
        assert_eq!(actual.len(), expected);
        assert_eq!(rows(&state["all_saved"]).len(), full);
        for row in rows(&state["all_saved"]) {
            assert_eq!(row["loaded"], row["fresh"]);
        }
        for row in actual {
            definitions.insert(row["physical_id"].as_str().unwrap());
            assert_eq!(row["loaded"], row["fresh"]);
            assert_eq!(row["loaded"]["level"], 1);
            assert_eq!(row["loaded"]["quality"], 0);
            assert_eq!(row["loaded"]["count"], 1);
            assert_eq!(row["loaded"]["enabled"], true);
            assert_eq!(row["loaded"]["enable_global_1"], true);
            assert_eq!(row["loaded"]["enable_global_2"], true);
            assert_eq!(row["loaded"]["corrupted"], false);
            assert_eq!(row["loaded"]["corrupt_level"], 0);
            assert!(rows(&row["loaded"]["stat_set"]).is_empty());
            assert!(rows(&row["loaded"]["stat_set_calcs"]).is_empty());
        }
        selected += actual.len();
    }
    assert_eq!((selected, definitions.len()), (180, 94));
    let state = &result["originals"][4]["state"];
    assert_eq!(rows(&state["catalog"]).len(), 514);
    let probes = rows(&state["probes"]);
    let p = |name| &probes.iter().find(|row| row["name"] == name).unwrap()["gem"];
    for name in [
        "baseline",
        "legacy-absent",
        "legacy-numeric",
        "unknown-attribute",
        "display-note",
    ] {
        assert!(rows(&p(name)["stat_set"]).is_empty());
        assert!(rows(&p(name)["stat_set_calcs"]).is_empty());
        assert_eq!(p(name)["level"], 1);
        assert_eq!(p(name)["quality"], 0);
    }
    assert_eq!(p("display-note")["note"], "source observation only");
    assert_eq!(p("enabled-false")["enabled"], false);
    assert_eq!(p("enabled-absent")["enabled"], true);
    assert_eq!(p("enabled-malformed")["enabled"], false);
    assert_eq!(p("global-false")["enable_global_1"], false);
    assert_eq!(p("global-false")["enable_global_2"], false);
    assert_eq!(p("global-absent")["enable_global_1"], true);
    assert_eq!(p("global-absent")["enable_global_2"], false);
    assert_eq!(p("count-three")["count"], 3);
    assert_eq!(p("count-zero")["count"], 0);
    assert_eq!(p("count-malformed")["count"], 1);
    assert_eq!(p("corruption-fractional")["corrupted"], true);
    assert_eq!(p("corruption-fractional")["corrupt_level"], 0.5);
    assert_eq!(p("corruption-nil")["corrupted"], false);
    assert_eq!(p("corruption-nil")["corrupt_level"], 0);
    assert_eq!(p("quality-twenty")["quality"], 20);
    assert_eq!(p("quality-absent")["quality"], Json::Null);
    assert_eq!(p("quality-malformed")["quality"], Json::Null);
    assert_eq!(p("level-zero")["level"], 1);
    assert_eq!(p("level-fractional")["level"], 1);
    assert_eq!(
        p("child-stat-set")["stat_set"],
        json!({"SupportRapidCastingPlayer":7})
    );
    assert_eq!(
        p("child-stat-set")["stat_set_calcs"],
        json!({"SupportRapidCastingPlayer":9})
    );
    assert_eq!(
        p("child-minion-map")["minion_stat_sets"],
        json!({"SupportRapidCastingPlayer":[3]})
    );
    assert_eq!(p("skill-context")["skill_part"], 3);
    assert_eq!(p("skill-context")["skill_stage_count"], 4);
    assert_eq!(p("skill-context")["skill_mine_count"], 5);
    assert_eq!(p("skill-context")["skill_minion"], "LivingLightning");
    assert_eq!(p("skill-context")["skill_minion_item_set"], 2);
    assert_eq!(state["group_override"]["first"]["skill_part"], 7);
    assert_eq!(state["group_override"]["second"]["skill_part"], 5);
    assert_eq!(p("mismatched-skill-id")["gem_id"], p("baseline")["gem_id"]);
    assert_eq!(
        p("mismatched-display-name")["gem_id"],
        p("baseline")["gem_id"]
    );
    assert_eq!(p("mismatched-display-name")["name"], p("baseline")["name"]);
    assert_eq!(p("legacy-skill-id")["gem_id"], p("baseline")["gem_id"]);
    assert_ne!(p("invalid-variant")["gem_id"], Json::Null);
    assert_ne!(p("missing-variant")["gem_id"], Json::Null);
    assert_eq!(state["mixed_effect_control"]["reviewed"], false);
    assert_eq!(state["mixed_effect_control"]["active_effects"], 1);
}

const OBSERVE: &str = r##"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==line);return f
end
local load=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
local validate=original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local function copy(t)if type(t)~="table"then return t end;local r={};for k,v in pairs(t)do r[k]=copy(v)end;return r end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function fields(g)
 return{gem_id=g.gemId,skill_id=g.skillId,name=g.nameSpec,note=g.note,level=g.level,quality=g.quality,
  corrupted=g.corrupted,corrupt_level=g.corruptLevel,enabled=g.enabled,count=g.count,
  enable_global_1=g.enableGlobal1,enable_global_2=g.enableGlobal2,
  stat_set=copy(g.statSet),stat_set_calcs=copy(g.statSetCalcs),
  minion_stat_sets=copy(g.skillMinionSkillStatSetIndexLookup),
  minion_stat_sets_calcs=copy(g.skillMinionSkillStatSetIndexLookupCalcs),
  skill_part=g.skillPart,skill_part_calcs=g.skillPartCalcs,
  skill_stage_count=g.skillStageCount,skill_stage_count_calcs=g.skillStageCountCalcs,
  skill_mine_count=g.skillMineCount,skill_mine_count_calcs=g.skillMineCountCalcs,
  skill_minion=g.skillMinion,skill_minion_calcs=g.skillMinionCalcs,
  skill_minion_item_set=g.skillMinionItemSet,skill_minion_item_set_calcs=g.skillMinionItemSetCalcs,
  skill_minion_skill=g.skillMinionSkill,skill_minion_skill_calcs=g.skillMinionSkillCalcs}
end
local function tab()return setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})end
local function fresh(node,receiver)
 receiver=receiver or tab();local before=#receiver.skillSets[1].socketGroupList
 load(receiver,node,1);assert(#receiver.skillSets[1].socketGroupList==before+1)
 return receiver.skillSets[1].socketGroupList[before+1]
end
local reviewed={};for _,id in ipairs(inventoryReviewed)do assert(not reviewed[id]);reviewed[id]=true end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local primitiveOutput={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="string"or type(v)=="boolean"then primitiveOutput[k]=v end end
local selected={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local originals={}
for setId,set in pairs(build.skillsTab.skillSets)do
 for groupIndex,group in ipairs(set.socketGroupList)do
  for gemIndex,gem in ipairs(group.gemList)do originals[#originals+1]={set=setId,group=groupIndex,index=gemIndex,group_ref=group,gem_ref=gem,fields=fields(gem)}end
 end
end
local doc,err=common.xml.ParseXML(inventoryXml);assert(doc and not err)
local ordinal={};local nextOrdinal=0
local function enumerate(node)
 if type(node)~="table"or not node.elem then return end
 ordinal[node]=nextOrdinal;nextOrdinal=nextOrdinal+1
 for _,child in ipairs(node)do enumerate(child)end
end
enumerate(doc[1]);local skills
for _,node in ipairs(doc[1])do if type(node)=="table"and node.elem=="Skills"then assert(not skills);skills=node end end;assert(skills)
local result={selected={},all_saved={},catalog={},probes={}}
local allowedAttributes={gemId=true,variantId=true,skillId=true,nameSpec=true,level=true,quality=true,
 corrupted=true,corruptLevel=true,enabled=true,count=true,enableGlobal1=true,enableGlobal2=true,
 statSetIndex=true,statSetIndexCalcs=true}
for _,setNode in ipairs(skills)do
 if type(setNode)=="table"and setNode.elem=="SkillSet"then
  local setId=assert(tonumber(setNode.attrib.id))
  local groupIndex=0
  for _,node in ipairs(setNode)do
   if type(node)=="table"and node.elem=="Skill"then
    groupIndex=groupIndex+1;local loaded=assert(build.skillsTab.skillSets[setId].socketGroupList[groupIndex])
    if not node.attrib.source or node.attrib.source==""then
     local isolated=fresh(node);assert(isolated~=loaded)
     for index,child in ipairs(node)do
      assert(type(child)=="table"and child.elem=="Gem")
      local originalGem=assert(loaded.gemList[index]);local freshGem=assert(isolated.gemList[index]);assert(freshGem~=originalGem)
      if freshGem.gemData and reviewed[freshGem.gemData.id]then
       local d=freshGem.gemData;assert(originalGem.gemData==d and d.grantedEffect.support==true)
       assert(data.gemsByGameId[child.attrib.gemId][child.attrib.variantId]==d)
       assert(#child==0 and not node.attrib.skillPart)
       for name in pairs(child.attrib)do assert(allowedAttributes[name],"unexpected saved attribute "..name)end
       assert(child.attrib.count=="1"and child.attrib.enableGlobal1=="true"and child.attrib.enableGlobal2=="true")
       assert(not child.attrib.statSetIndex or child.attrib.statSetIndex=="nil")
       assert(not child.attrib.statSetIndexCalcs or child.attrib.statSetIndexCalcs=="nil")
       assert(equal(fields(originalGem),fields(freshGem)))
       local row={source_ordinal=ordinal[child],set=setId,group=groupIndex,index=index,
        physical_id=d.id,attributes=copy(child.attrib),group_attributes=copy(node.attrib),loaded=fields(originalGem),fresh=fields(freshGem)}
       result.all_saved[#result.all_saved+1]=row
       if setId==selected.skills then result.selected[#result.selected+1]=row end
      end
     end
    end
   end
  end
 end
end
local d=assert(data.gems["Metadata/Items/Gems/SkillGemRapidCastingSupport"])
local base={gemId=d.gameId,variantId=d.variantId,skillId=d.grantedEffect.id,nameSpec=d.name,
 level="1",quality="0",corrupted="false",corruptLevel="0",enabled="true",count="1",
 enableGlobal1="true",enableGlobal2="true",statSetIndex="nil",statSetIndexCalcs="nil"}
local function node(attributes,children,group)
 local gem={elem="Gem",attrib=attributes};for _,child in ipairs(children or{})do gem[#gem+1]=child end
 return{elem="Skill",attrib=group or{enabled="true"},gem}
end
local receiver=tab();local firstNode=node(copy(base));firstNode[1].attrib.skillPart="8"
local first=fresh(firstNode,receiver);local second=fresh(node(copy(base)),receiver)
assert(first~=second and first.gemList[1]~=second.gemList[1]and first.gemList[1].skillPart==8 and second.gemList[1].skillPart==nil)
result.reused_tab_fresh_instances=true;result.fresh_objects=true
if inventoryOriginal==5 then
 for _,id in ipairs(inventoryReviewed)do
  local gem=assert(data.gems[id]);local effect=gem.grantedEffect
  assert(effect.support==true and not effect.fromTree and not effect.hideFromSideBar)
  assert(#gem.grantedEffectList==1 and gem.grantedEffectList[1]==effect and #gem.additionalGrantedEffects==0)
  for key in pairs(gem)do assert(not key:match("^additionalGrantedEffectId%d+$")and not key:match("^additionalStatSet%d+$"))end
  local attrs=copy(base);attrs.gemId=gem.gameId;attrs.variantId=gem.variantId;attrs.skillId=effect.id;attrs.nameSpec=gem.name
  local group=fresh(node(attrs));local loaded=group.gemList[1]
  assert(loaded.gemData==gem and loaded.level==1 and loaded.quality==0)
  result.catalog[#result.catalog+1]={physical_id=id,skill_id=effect.id,loaded=fields(loaded)}
 end
 local function probe(name,edit,children)
  local attrs=copy(base);if edit then edit(attrs)end
  local group=fresh(node(attrs,children));result.probes[#result.probes+1]={name=name,attributes=attrs,gem=fields(group.gemList[1])}
 end
 probe("baseline")
 probe("legacy-absent",function(a)a.statSetIndex=nil;a.statSetIndexCalcs=nil end)
 probe("legacy-numeric",function(a)a.statSetIndex="7";a.statSetIndexCalcs="9"end)
 probe("unknown-attribute",function(a)a.ownedWitnessUnknown="1"end)
 probe("display-note",function(a)a.note="source observation only"end)
 probe("enabled-false",function(a)a.enabled="false"end)
 probe("enabled-absent",function(a)a.enabled=nil end)
 probe("enabled-malformed",function(a)a.enabled="TRUE"end)
 probe("global-false",function(a)a.enableGlobal1="false";a.enableGlobal2="false"end)
 probe("global-absent",function(a)a.enableGlobal1=nil;a.enableGlobal2=nil end)
 probe("count-three",function(a)a.count="3"end)
 probe("count-zero",function(a)a.count="0"end)
 probe("count-malformed",function(a)a.count="bad"end)
 probe("corruption-fractional",function(a)a.corrupted="true";a.corruptLevel="0.5"end)
 probe("corruption-nil",function(a)a.corrupted="nil";a.corruptLevel="nil"end)
 probe("quality-twenty",function(a)a.quality="20"end)
 probe("quality-absent",function(a)a.quality=nil end)
 probe("quality-malformed",function(a)a.quality="bad"end)
 probe("level-zero",function(a)a.level="0"end)
 probe("level-fractional",function(a)a.level="1.5"end)
 probe("child-stat-set",nil,{{elem="StatSetIndex",attrib={grantedEffect=d.grantedEffect.id,index="7"}},
  {elem="StatSetCalcsIndex",attrib={grantedEffect=d.grantedEffect.id,index="9"}}})
 probe("child-minion-map",nil,{{elem="MinionSkillIndexLookup",attrib={grantedEffect=d.grantedEffect.id},
  {elem="Map",attrib={skillIndex="1",statSetIndex="3"}}}})
 probe("skill-context",function(a)a.skillPart="3";a.skillStageCount="4";a.skillMineCount="5";a.skillMinion="LivingLightning";a.skillMinionItemSet="2"end)
 probe("mismatched-skill-id",function(a)a.skillId="SparkPlayer"end)
 probe("mismatched-display-name",function(a)a.nameSpec="wrong source display name"end)
 probe("legacy-skill-id",function(a)a.gemId=nil;a.variantId=nil end)
 probe("invalid-variant",function(a)a.variantId="OwnedWitnessUnknown"end)
 probe("missing-variant",function(a)a.variantId=nil end)
 local a=copy(base);a.skillPart="3";local b=copy(base);b.skillPart="5"
 local override=fresh({elem="Skill",attrib={enabled="true",skillPart="7"},{elem="Gem",attrib=a},{elem="Gem",attrib=b}})
 result.group_override={first=fields(override.gemList[1]),second=fields(override.gemList[2])}
 local mixed=assert(data.gems["Metadata/Items/Gems/SkillGemLivingLightningSupportTwo"]);local active=0
 for _,effect in ipairs(mixed.grantedEffectList)do if not effect.support then active=active+1 end end
 result.mixed_effect_control={physical_id=mixed.id,reviewed=reviewed[mixed.id]==true,active_effects=active}
end
for _,entry in ipairs(originals)do
 local group=build.skillsTab.skillSets[entry.set].socketGroupList[entry.group]
 assert(group==entry.group_ref and group.gemList[entry.index]==entry.gem_ref and equal(fields(entry.gem_ref),entry.fields))
end;result.saved_gems_preserved=true
assert(build.skillsTab.activeSkillSetId==selected.skills and build.itemsTab.activeItemSetId==selected.items and build.treeTab.activeSpec==selected.spec and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(primitiveOutput)do assert(output[k]==v)end;result.main_output_preserved=true
assert(build.skillsTab.LoadSkill==load and build.skillsTab.ProcessSocketGroup==process and calcLib.validateGemLevel==validate);result.original_functions_preserved=true
return result
"##;
