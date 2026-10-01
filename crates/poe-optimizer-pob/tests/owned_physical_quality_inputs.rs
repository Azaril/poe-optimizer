//! Physical quality is a scalar input; alternate quality effects are computed.
//! Authenticated complete source evidence only, with no native coverage promotion.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_source_physical_quality_is_independent_of_computed_alt_stats";
const CHILD: &str = "POE_PHYSICAL_QUALITY_SOURCE_CHILD";

#[test]
fn complete_source_physical_quality_is_independent_of_computed_alt_stats() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-physical-quality-inputs-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        let enabled = mode == "on";
        let catalog_bytes =
            fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap();
        let catalog =
            SkillIdentityCatalog::new(serde_json::from_slice(&catalog_bytes).unwrap()).unwrap();
        let gems: Vec<_> = catalog
            .data()
            .gems
            .iter()
            .map(|gem| {
                json!({
                    "key":gem.key, "game_id":gem.game_id, "variant_id":gem.variant_id,
                    "primary_effect_id":gem.primary_effect_id,
                })
            })
            .collect();
        assert_eq!(gems.len(), 966);
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-02.xml")).unwrap();
        let index = read(&fixtures.join("index.json"));
        let fixture = index["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["xml"] == "build-02.xml")
            .unwrap();
        let xml_sha256 = format!("{:x}", Sha256::digest(xml.as_bytes()));
        assert_eq!(fixture["xml_sha256"], xml_sha256);
        let before = |lua: &Lua| {
            lua.globals().set("physicalQualityJit", enabled)?;
            lua.globals()
                .set("physicalQualityCatalog", lua.to_value(&gems)?)?;
            lua.load("if physicalQualityJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let scratch = tempfile::tempdir().unwrap();
        let mut result = source::observe_with_build_hook_unwrapped(
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
        result["evidence"] = json!({
            "scope":"physical_quality_input_and_computed_alt_stats",
            "manifest_sha256":pinned::manifest_sha256(),
            "catalog_sha256":format!("{:x}",Sha256::digest(&catalog_bytes)),
            "xml_sha256":xml_sha256,
            "fixture":"build-02.xml",
            "native_parity":false,
            "full_build_numeric_parity":false,
            "files":([
                "src/Classes/SkillsTab.lua","src/Modules/CalcSetup.lua",
                "src/Modules/CalcTools.lua","src/Modules/CalcActiveSkill.lua",
                "src/Modules/CalcDefence.lua","src/Data/Gems.lua",
                "src/Data/Skills/act_int.lua","src/Data/Skills/act_dex.lua",
                "src/Modules/ModParser.lua","src/TreeData/0_5/tree.lua"
            ].map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        });
        fs::write(
            destination.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_observations(&result["additional_observation"], &gems);
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
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
    let off = read(&destination.join("source-jit-off.json"));
    let on = read(&destination.join("source-jit-on.json"));
    assert_eq!(off["source_hash"], on["source_hash"]);
    assert_eq!(off["evidence"], on["evidence"]);
    assert_eq!(off["additional_observation"], on["additional_observation"]);
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-physical-quality-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|rows| rows.is_empty()));
        &[]
    }
}
fn assert_observations(result: &Json, gems: &[Json]) {
    assert_eq!(result["original_main_preserved"], true);
    assert_eq!(result["original_allocations_preserved"], true);
    assert_eq!(result["original_main_output_preserved"], true);
    assert_eq!(result["original_functions_preserved"], true);
    let census = rows(&result["census"]);
    assert_eq!(census.len(), gems.len());
    let mut admitted = 0;
    for (row, gem) in census.iter().zip(gems) {
        assert_eq!(row["key"], gem["key"]);
        if row["excluded"].as_bool().unwrap() {
            assert!(matches!(
                row["reason"].as_str(),
                Some("missing-primary" | "hidden-primary" | "nonwinning-external-variant")
            ));
            continue;
        }
        admitted += 1;
        let cases = rows(&row["cases"]);
        assert_eq!(cases.len(), 9);
        for case in cases {
            assert_eq!(
                case["ok"], true,
                "{}: {}: {}",
                row["key"], case["label"], case["error"]
            );
            assert_eq!(case["same_gem"], true);
            assert_eq!(case["physical_gems"], 1);
            assert_eq!(case["quality_id_present"], false);
            assert_eq!(case["reprocessed_quality_id_present"], false);
            let quality = match case["label"].as_str().unwrap() {
                "missing" | "literal-nil" | "malformed" => None,
                "zero" => Some(0.0),
                "fractional" => Some(0.5),
                "ordinary" | "bogus-kind" | "numeric-kind" | "legacy-ui" => Some(20.0),
                other => panic!("unknown quality case: {other}"),
            };
            assert_eq!(case["quality"].as_f64(), quality);
            assert_eq!(case["reprocessed_quality"].as_f64(), quality);
            assert!(rows(&case["stat_set_keys"]).is_empty());
            assert!(rows(&case["calcs_stat_set_keys"]).is_empty());
        }
    }
    assert_eq!(
        admitted, 966,
        "the pinned physical census has no exclusions"
    );
    let flag = &result["computed_flag"];
    assert_eq!(flag["without_node"], false);
    assert_eq!(flag["without_node_value_kind"], "nil");
    assert_eq!(flag["with_node"], true);
    assert_eq!(flag["saved_quality_unchanged"], true);
    assert_eq!(flag["node_name"], "Advanced Thaumaturgy");
    let focused = rows(&result["focused"]);
    assert_eq!(focused.len(), 3);
    for row in focused {
        assert_eq!(row["quality"], 20);
        assert_eq!(row["quality_id_present"], false);
        assert_eq!(row["quality_after_stats"], 20);
        let zero = &row["zero"];
        assert_eq!(zero["off"], zero["on"]);
        let (ordinary, alternate, ordinary_value, alternate_value) = match row["effect"]
            .as_str()
            .unwrap()
        {
            "TwisterPlayer" => (
                "twister_%_chance_for_additional_twister",
                Some("active_skill_projectile_speed_+%_final"),
                20,
                20,
            ),
            "SummonSkeletalClericsPlayer" => (
                "skeletal_cleric_revived_skeletons_immune_for_X_ms",
                Some("minion_damage_taken_+%"),
                3000,
                -40,
            ),
            "SummonSkeletalSnipersPlayer" => ("active_skill_minion_damage_+%_final", None, 20, 0),
            other => panic!("unexpected focused effect: {other}"),
        };
        // Expected constants describe source data; the observed values are
        // produced by the original complete stat constructor, not copied math.
        assert_eq!(
            row["off"][ordinary].as_i64().unwrap_or(0)
                - zero["off"][ordinary].as_i64().unwrap_or(0),
            ordinary_value
        );
        assert_eq!(row["on"][ordinary], row["off"][ordinary]);
        if let Some(alternate) = alternate {
            assert_eq!(row["off"][alternate], zero["off"][alternate]);
            assert_eq!(
                row["on"][alternate].as_i64().unwrap_or(0)
                    - row["off"][alternate].as_i64().unwrap_or(0),
                alternate_value
            );
            assert_ne!(row["on"], row["off"]);
        } else {
            assert_eq!(row["on"], row["off"]);
        }
        let maps = &row["maps"];
        assert_eq!(maps["main"], 2);
        assert_eq!(maps["calcs"], 3);
        assert_eq!(maps["minion_main"], 4);
        assert_eq!(maps["minion_calcs"], 5);
        assert_eq!(maps["legacy_scalar_retained"], false);
        let counts = rows(&row["counts"]);
        assert_eq!(counts.len(), 3);
        for (count, expected) in counts.iter().zip([3, 7, 0]) {
            assert_eq!(count["observed"], expected);
            assert_eq!(count["enabled"], true);
        }
    }
}
const OBSERVATION: &str = r#"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line)
 return f
end
local load=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
local init=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local stats=original(calcLib.buildSkillInstanceStats,"Modules/CalcTools.lua",161)
local count=original(calcs.getActiveSkillCount,"Modules/CalcDefence.lua",149)
local main=build.calcsTab.mainEnv
local mainSkill=main.player.mainSkill
local mainIndex=build.mainSocketGroup
local activeSet=build.skillsTab.activeSkillSetId
local function keys(t)
 local result={};for key in pairs(t or {}) do result[#result+1]=key end
 table.sort(result);return result
end
local function tab()
 return setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
end
local function attributes(gem,quality)
 return {gemId=gem.gameId,variantId=gem.variantId,level=tostring(gem.naturalMaxLevel or 20),
  quality=quality,corrupted="false",corruptLevel="0",enabled="true",count="1",
  enableGlobal1="true",enableGlobal2="true"}
end
local function loaded(gem,attributes,children,groupAttributes)
 local t=tab();local child={elem="Gem",attrib=attributes}
 for _,value in ipairs(children or {}) do child[#child+1]=value end
 load(t,{elem="Skill",attrib=groupAttributes or {enabled="true"},child},1)
 local group=assert(t.skillSets[1].socketGroupList[1])
 return t,group,assert(group.gemList[1])
end
local result={census={},focused={}}
local definitions={}
local cases={
 {label="missing"}, {label="literal-nil",quality="nil"},
 {label="malformed",quality="not-a-number"}, {label="zero",quality="0"},
 {label="fractional",quality="0.5"}, {label="ordinary",quality="20"},
 {label="bogus-kind",quality="20",qualityId="unrecognized-alternate-kind"},
 {label="numeric-kind",quality="20",qualityId="2"},
 {label="legacy-ui",quality="20",qualityId="1",legacy=true},
}
for _,reviewed in ipairs(physicalQualityCatalog) do
 local gem=assert(data.gems[reviewed.key])
 assert(gem.id==reviewed.key and gem.gameId==reviewed.game_id and gem.variantId==reviewed.variant_id)
 assert(gem.grantedEffectId==reviewed.primary_effect_id)
 local row={key=reviewed.key,excluded=false,cases={}}
 local effect=gem.grantedEffect
 if not effect then row.excluded=true;row.reason="missing-primary"
 elseif effect.hideFromSideBar then row.excluded=true;row.reason="hidden-primary"
 elseif not data.gemsByGameId[gem.gameId] or data.gemsByGameId[gem.gameId][gem.variantId]~=gem then
  row.excluded=true;row.reason="nonwinning-external-variant"
 else
  assert(effect==data.skills[reviewed.primary_effect_id])
  definitions[reviewed.primary_effect_id]=gem
  for _,case in ipairs(cases) do
   local a=attributes(gem,case.quality);a.qualityId=case.qualityId
   if case.legacy then a.statSetIndex="99";a.statSetIndexCalcs="98";a.note="display annotation" end
   local output={label=case.label}
   local ok,err=pcall(function()
    local t,group,physical=loaded(gem,a)
    output.same_gem=physical.gemData==gem;output.physical_gems=#group.gemList
    output.quality=physical.quality;output.quality_id_present=physical.qualityId~=nil
    output.stat_set_keys=keys(physical.statSet);output.calcs_stat_set_keys=keys(physical.statSetCalcs)
    process(t,group)
    output.reprocessed_quality=physical.quality
    output.reprocessed_quality_id_present=physical.qualityId~=nil
   end)
   output.ok=ok;output.error=not ok and tostring(err) or nil
   row.cases[#row.cases+1]=output
  end
 end
 result.census[#result.census+1]=row
end

-- The actual parsed source node is an explicit component input. We do not
-- mutate the saved build or replace the flag producer with a test function.
local flagNodes={}
for _,node in pairs(build.spec.nodes) do
 if node.modList and node.modList:Flag(nil,"GemlingQuality") then flagNodes[#flagNodes+1]=node end
end
table.sort(flagNodes,function(a,b)return a.id<b.id end)
assert(#flagNodes==1,"expected one explicit source GemlingQuality node")
local node=flagNodes[1]
local savedGroups=build.skillsTab.socketGroupList
local physicalBefore={}
for _,group in ipairs(savedGroups) do
 for _,gem in ipairs(group.gemList) do physicalBefore[#physicalBefore+1]={gem=gem,quality=gem.quality,kind=gem.qualityId} end
end
local allocatedBefore={}
for id,n in pairs(build.spec.allocNodes) do
 allocatedBefore[id]={node=n,allocMode=n.allocMode,modList=n.modList,modCount=#n.modList}
end
local outputBefore={}
for _,key in ipairs({"TotalDPS","CombinedDPS","Life","Mana","EnergyShield"}) do
 outputBefore[key]=main.player.output[key]
end
local remove={};for _,n in ipairs(flagNodes) do remove[n]=true end
local function initialize(override)
 local gem=assert(definitions.TwisterPlayer)
 local _,group=loaded(gem,attributes(gem,"20"))
 build.skillsTab.socketGroupList={group};build.mainSocketGroup=1
 local ok,env=pcall(init,build,"CALCULATOR",override)
 build.skillsTab.socketGroupList=savedGroups;build.mainSocketGroup=mainIndex
 assert(ok,env)
 return env
end
local off=initialize({removeNodes=remove})
-- Spec nodes inherit immutable tree fields through a metatable. Preserve that
-- exact source lookup while keeping any component allocation writes isolated.
local componentNode=setmetatable({},{__index=node})
local on=initialize({addNodes={[componentNode]=true},removeNodes=remove})
assert(off.useAltGemQualityStats==nil and on.useAltGemQualityStats==true,
 "computed flags: off="..tostring(off.useAltGemQualityStats)..", on="..tostring(on.useAltGemQualityStats))
result.computed_flag={without_node=not not off.useAltGemQualityStats,without_node_value_kind=type(off.useAltGemQualityStats),with_node=on.useAltGemQualityStats,
 node_id=node.id,node_name=node.dn or node.name,saved_quality_unchanged=true}
for _,id in ipairs({"TwisterPlayer","SummonSkeletalSnipersPlayer","SummonSkeletalClericsPlayer"}) do
 local gem=assert(definitions[id]);local effect=assert(gem.grantedEffect)
 local a=attributes(gem,"20");a.qualityId="unrecognized-alternate-kind"
 local _,_,physical=loaded(gem,a)
 local instance={level=physical.level,quality=physical.quality}
 local row={effect=id,quality=physical.quality,quality_id_present=physical.qualityId~=nil,
  off=stats(instance,effect,effect.statSets[1],off.useAltGemQualityStats),
  on=stats(instance,effect,effect.statSets[1],on.useAltGemQualityStats)}
 row.quality_after_stats=physical.quality
 local zero={level=physical.level,quality=0}
 row.zero={off=stats(zero,effect,effect.statSets[1],false),on=stats(zero,effect,effect.statSets[1],true)}
 local attributesWithLegacy=attributes(gem,"20")
 attributesWithLegacy.statSetIndex="99";attributesWithLegacy.statSetIndexCalcs="98"
 local children={
  {elem="StatSetIndex",attrib={grantedEffect=id,index="2"}},
  {elem="StatSetCalcsIndex",attrib={grantedEffect=id,index="3"}},
  {elem="MinionSkillIndexLookup",attrib={grantedEffect=id},
   {elem="MinionSkillIndexMap",attrib={skillIndex="1",statSetIndex="4"}}},
  {elem="MinionSkillIndexLookupCalcs",attrib={grantedEffect=id},
   {elem="MinionSkillIndexMap",attrib={skillIndex="1",statSetIndex="5"}}},
 }
 local _,_,selected=loaded(gem,attributesWithLegacy,children)
 row.maps={main=selected.statSet[id],calcs=selected.statSetCalcs[id],
  minion_main=selected.skillMinionSkillStatSetIndexLookup[id][1],
  minion_calcs=selected.skillMinionSkillStatSetIndexLookupCalcs[id][1],
  legacy_scalar_retained=selected.statSet.index~=nil or selected.statSetCalcs.index~=nil}
 row.counts={}
 for _,case in ipairs({{count="3"},{count="3",groupCount="7"},{count="3",groupCount="0"}}) do
  local a=attributes(gem,"20");a.count=case.count
  local _,group=loaded(gem,a,nil,{enabled="true",groupCount=case.groupCount})
  local amount,enabled=count({socketGroup=group,activeEffect={grantedEffect=effect}})
  row.counts[#row.counts+1]={physical=case.count,group=case.groupCount,observed=amount,enabled=enabled}
 end
 result.focused[#result.focused+1]=row
end
for _,row in ipairs(physicalBefore) do
 assert(row.gem.quality==row.quality and row.gem.qualityId==row.kind)
end
for id,row in pairs(allocatedBefore) do
 assert(build.spec.allocNodes[id]==row.node and row.node.allocMode==row.allocMode)
 assert(row.node.modList==row.modList and #row.node.modList==row.modCount)
end
for id in pairs(build.spec.allocNodes) do assert(allocatedBefore[id]) end
result.original_allocations_preserved=true
for _,key in ipairs({"TotalDPS","CombinedDPS","Life","Mana","EnergyShield"}) do
 assert(main.player.output[key]==outputBefore[key])
end
result.original_main_output_preserved=true
assert(build.calcsTab.mainEnv==main and main.player.mainSkill==mainSkill)
assert(build.mainSocketGroup==mainIndex and build.skillsTab.activeSkillSetId==activeSet)
assert(build.skillsTab.socketGroupList==savedGroups)
result.original_main_preserved=true
assert(build.skillsTab.LoadSkill==load and build.skillsTab.ProcessSocketGroup==process)
assert(calcs.initEnv==init and calcLib.buildSkillInstanceStats==stats and calcs.getActiveSkillCount==count)
result.original_functions_preserved=true
return result
"#;
