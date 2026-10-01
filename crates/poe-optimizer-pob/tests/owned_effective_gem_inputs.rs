//! Complete-source effective-input evidence, not native or whole-build parity.
//! Untouched original observations precede separately labelled component inputs.
#![cfg(not(target_arch = "wasm32"))]
#[path = "../../poe-optimizer-import/tests/support/owned_effective_gem_fixture.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_engine::owned_rules::EffectDisposition;
use poe_optimizer_import::owned_effective_gem_recipe::{
    DenseGemLevelPolicy, EffectiveGemRecipeRole,
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

const CHILD: &str = "POE_EFFECTIVE_GEM_INPUT_SOURCE_CHILD";
const CLERIC: &str = "original_cleric_effective_inputs_retain_exact_property_provenance";
const SNIPER: &str = "original_sniper_and_components_keep_effective_input_stages_distinct";

#[test]
fn original_cleric_effective_inputs_retain_exact_property_provenance() {
    run(CLERIC, "build-01.xml", "SummonSkeletalClericsPlayer", false);
}

#[test]
fn original_sniper_and_components_keep_effective_input_stages_distinct() {
    run(SNIPER, "build-05.xml", "SummonSkeletalSnipersPlayer", true);
}

fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn run(test: &str, fixture: &str, skill: &str, components: bool) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root
        .join("runs/owned-effective-gem-inputs-01")
        .join(fixture.trim_end_matches(".xml"));
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join(fixture)).unwrap();
        let manifest = read(&fixtures.join("index.json"));
        let entry = manifest["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["xml"] == fixture)
            .unwrap();
        let xml_sha256 = format!("{:x}", Sha256::digest(xml.as_bytes()));
        assert_eq!(entry["xml_sha256"], xml_sha256);
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("effectiveInputJitEnabled", enabled)?;
            lua.globals().set("effectiveInputObservedSkill", skill)?;
            lua.globals().set("effectiveInputComponents", components)?;
            lua.load("if effectiveInputJitEnabled then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
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
        let files: Vec<_> = [
            "src/Classes/SkillsTab.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/CalcActiveSkill.lua",
            "src/Modules/CalcTools.lua",
            "src/Modules/CalcPerform.lua",
            "src/Classes/ModDB.lua",
            "src/Classes/ModList.lua",
            "src/Data/Gems.lua",
            "src/Data/Skills/act_int.lua",
            "src/Data/Skills/act_dex.lua",
            "src/Data/Skills/sup_int.lua",
            "src/Data/Skills/sup_str.lua",
        ]
        .iter()
        .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))
        .collect();
        result["evidence"] = json!({
            "manifest_sha256":pinned::manifest_sha256(), "files":files,
            "scope":"complete_source_effective_gem_inputs", "fixture":fixture,
            "xml_sha256":xml_sha256, "native_parity":false, "full_build_numeric_parity":false,
            "native_comparison_scope":if components { "closed_component_pre_support_and_preparation_scalar_recipes" } else { "none" },
        });
        // Persist actual observations before assertions, including component errors.
        fs::write(
            destination.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_original(&result["additional_observation"], components);
        if components {
            assert_components(&result["additional_observation"]);
            compare_native_components(&result["additional_observation"]);
        }
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
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

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-effective-gem-input-observation")
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

fn checked(value: &Json) -> &Json {
    assert_eq!(value["status"], "ok", "{}", value["error"]);
    &value["value"]
}

fn assert_original(result: &Json, sniper: bool) {
    let original = checked(&result["untouched_original"]);
    assert_eq!(original["main_selection_preserved"], true);
    assert_eq!(result["main_selection_restored"], true);
    assert_eq!(original["corrupt_delta"], 0);
    assert_eq!(original["corrupted"], false);
    assert_eq!(original["raw_level"], if sniper { 20 } else { 19 });
    assert_eq!(original["final_level"], if sniper { 22 } else { 30 });
    assert_eq!(original["actor_level"], if sniper { 44 } else { 60 });
    assert_eq!(original["raw_quality"], if sniper { 0 } else { 20 });
    assert_eq!(original["final_quality"], original["raw_quality"]);
    assert_eq!(original["is_main_skill"], sniper);
    assert_eq!(original["supported_properties_were_cached"], true);
    assert!(rows(&original["supported_properties"]).is_empty());
    assert_eq!(
        original["main_skill"],
        if sniper {
            "SummonSkeletalSnipersPlayer"
        } else {
            "SummonSandDjinnPlayer"
        }
    );
    if !sniper {
        assert_eq!(original["group_index"], 5);
    }
    let expected: &[(&str, f64)] = if sniper {
        &[
            ("Item:21:New Item, Iron Crown", 1.0),
            ("Item:23:New Item, Solar Amulet", 1.0),
            ("Many Sources:^x88FFFF0% Amulet Bonus Effect", 0.0),
        ]
    } else {
        &[
            ("Item:9:Morbid Roar, Rattling Sceptre", 4.0),
            ("Item:1:Plague Chant, Sacred Focus", 2.0),
            ("Item:8:Golem Guardian, Kamasan Tiara", 2.0),
            ("Item:6:Havoc Torc, Lapis Amulet", 3.0),
            ("Many Sources:^x88FFFF0% Amulet Bonus Effect", 0.0),
        ]
    };
    let mut delta = 0.0;
    let properties = rows(&original["external_properties"]);
    assert_eq!(properties.len(), expected.len());
    for (row, (source, amount)) in properties.iter().zip(expected) {
        assert_eq!(row["mod"]["name"], "GemProperty");
        assert_eq!(row["mod"]["type"], "LIST");
        assert_eq!(row["mod"]["source"], *source);
        assert_eq!(row["mod"]["flags"], 0);
        assert_eq!(row["mod"]["keyword_flags"], 0);
        assert!(rows(&row["mod"]["tags"]).is_empty());
        assert_eq!(row["value"]["key"], "level");
        assert_eq!(row["value"]["keyword"], "minion");
        assert_eq!(row["value"]["value"], *amount);
        delta += amount;
    }
    // The source records only the modifiers its original matcher actually used.
    // Summing those observations explains this input change; it is not a native
    // implementation or an independent whole-build output comparison.
    assert_eq!(delta, if sniper { 2.0 } else { 11.0 });
    assert_eq!(
        original["raw_level"].as_f64().unwrap() + delta,
        original["final_level"]
    );
    if !sniper {
        let meat_shield = rows(&original["supports"])
            .iter()
            .find(|row| row["effect"] == "SupportMeatShieldPlayerTwo")
            .unwrap();
        assert_eq!(
            meat_shield["external_properties"],
            original["external_properties"]
        );
        assert_eq!(meat_shield["raw_level"], 1);
        assert_eq!(meat_shield["level"], 1);
        assert_eq!(meat_shield["supports_physical_source"], true);
    }
    let definitions = rows(&result["physical_definitions"]);
    assert_eq!(definitions.len(), 5);
    let tags = [
        vec![
            "area",
            "attack",
            "dexterity",
            "duration",
            "grants_active_skill",
            "projectile",
            "repeatable",
            "wind",
        ],
        vec![
            "chaos",
            "fire",
            "grants_active_skill",
            "intelligence",
            "minion",
            "persistent",
            "physical",
        ],
        vec![
            "grants_active_skill",
            "intelligence",
            "minion",
            "persistent",
        ],
        vec!["attack", "support"],
        vec!["minion", "support"],
    ];
    for (position, (definition, tags)) in definitions.iter().zip(tags).enumerate() {
        let active = position < 3;
        assert_eq!(
            definition["level_keys"],
            json!((1..=if active { 40 } else { 1 }).collect::<Vec<_>>())
        );
        assert_eq!(definition["natural_maximum"], if active { 20 } else { 1 });
        assert_eq!(definition["tags"], json!(tags));
        assert_eq!(definition["keyword_matches"]["grants_active_skill"], active);
        assert_eq!(
            definition["keyword_matches"]["minion"],
            matches!(position, 1 | 2 | 4)
        );
        assert_eq!(definition["keyword_matches"]["spell"], false);
    }
}

fn assert_components(result: &Json) {
    let probes = rows(&result["setup_probes"]);
    assert_eq!(probes.len(), 6);
    for probe in probes {
        let value = checked(probe);
        assert_eq!(value["raw_level"], 12);
        assert_eq!(value["raw_quality"], 13);
        assert_eq!(value["quality"], 13);
        assert_eq!(value["support_raw_level"], 1);
        assert_eq!(value["support_raw_quality"], 7);
        assert_eq!(value["support_level"], 1);
        assert_eq!(value["support_quality"], 7);
        assert_eq!(value["support_delta"], 0.25);
        assert!(rows(&value["external_properties"]).is_empty());
        assert_eq!(value["level"], probe["expected_level"]);
        assert_eq!(value["corruption_condition"], probe["corrupted"]);
    }
    let properties = rows(&result["property_probes"]);
    assert_eq!(properties.len(), 6);
    for probe in properties {
        let value = checked(probe);
        assert_eq!(value["before_validation_level"], probe["expected_before"]);
        assert_eq!(value["level"], probe["expected_after"]);
        assert_eq!(value["quality"], probe["expected_quality"]);
        assert_eq!(value["matched_count"], probe["expected_matches"]);
    }
    let supported = checked(&result["supported_property_probe"]);
    assert_eq!(supported["before_level"], 12);
    assert_eq!(supported["before_quality"], 13);
    assert_eq!(supported["after_level"], 14);
    assert_eq!(supported["after_quality"], 18);
    assert_eq!(rows(&supported["matched"]).len(), 2);
    assert_eq!(supported["physical_level"], 12);
    assert_eq!(supported["physical_quality"], 13);
    assert!(rows(&supported["support_gem_properties"]).is_empty());
    let ordering = checked(&result["supported_property_ordering_probe"]);
    assert_eq!(ordering["physical_level"], 12);
    assert_eq!(ordering["corruption_delta"], 0.25);
    assert_eq!(ordering["before_support_level"], 12.25);
    assert_eq!(ordering["level_adjustment"], 0.75);
    assert_eq!(ordering["final_level"], 13);
    assert_eq!(ordering["physical_quality"], 13);
    assert_eq!(ordering["final_quality"], 18);
    assert_eq!(rows(&ordering["matched"]).len(), 2);
}

fn compare_native_components(result: &Json) {
    // The fixture compiles the production authoring recipe and executes its
    // ordinary owned rule programs. Explicit component facts close these scalar
    // inputs only; this does not establish real-build contributor coverage,
    // property targeting, post-support final inputs or generated activation.
    let mut fixture = native::Fixture::new();
    fixture.input.bindings[1].role = EffectiveGemRecipeRole::SupportPreparation {
        levels: DenseGemLevelPolicy {
            maximum: 1,
            natural_maximum: 1,
        },
    };
    let applied = |value| EffectDisposition::Applied { value };
    for probe in rows(&result["setup_probes"]) {
        let value = checked(probe);
        let actual = fixture.evaluate_values(
            0,
            native::Values {
                raw_level: value["raw_level"].as_i64().unwrap(),
                corruption: value["raw_delta"].as_f64().unwrap(),
                external_level: Some(0.0),
                external_quality: Some(0.0),
                quality: value["raw_quality"].as_f64(),
            },
        );
        assert_eq!(
            actual,
            vec![
                applied(native::qty(
                    value["prepared_level"].as_f64().unwrap(),
                    &fixture.count
                )),
                applied(native::qty(
                    value["prepared_quality"].as_f64().unwrap(),
                    &fixture.percent
                )),
            ],
            "observed pre-support phase: {}",
            probe["name"]
        );
        let support = fixture.evaluate_values(
            1,
            native::Values {
                raw_level: value["support_raw_level"].as_i64().unwrap(),
                corruption: value["support_delta"].as_f64().unwrap(),
                external_level: Some(0.0),
                external_quality: Some(0.0),
                quality: value["support_raw_quality"].as_f64(),
            },
        );
        assert_eq!(
            support,
            vec![
                applied(native::int(value["support_level"].as_i64().unwrap())),
                applied(native::qty(
                    value["support_quality"].as_f64().unwrap(),
                    &fixture.percent
                )),
            ],
            "observed support preparation phase: {}",
            probe["name"]
        );
    }
    for probe in rows(&result["property_probes"]) {
        let value = checked(probe);
        let support = value["source_role"] == "support";
        let sum = |key| {
            rows(&value["matched"])
                .iter()
                .filter(|row| row["value"]["key"] == key)
                .map(|row| row["value"]["value"].as_f64().unwrap())
                .sum()
        };
        let actual = fixture.evaluate_values(
            usize::from(support),
            native::Values {
                raw_level: value["raw_level"].as_i64().unwrap(),
                corruption: 0.0,
                external_level: Some(sum("level")),
                external_quality: Some(sum("quality")),
                quality: value["raw_quality"].as_f64(),
            },
        );
        let level = if support {
            native::int(value["level"].as_i64().unwrap())
        } else {
            native::qty(
                value["before_validation_level"].as_f64().unwrap(),
                &fixture.count,
            )
        };
        assert_eq!(
            actual,
            vec![
                applied(level),
                applied(native::qty(
                    value["quality"].as_f64().unwrap(),
                    &fixture.percent
                )),
            ],
            "observed eligible-property scalar phase: {}",
            probe["name"]
        );
    }
    let ordering = checked(&result["supported_property_ordering_probe"]);
    let actual = fixture.evaluate_values(
        0,
        native::Values {
            raw_level: ordering["physical_level"].as_i64().unwrap(),
            corruption: ordering["corruption_delta"].as_f64().unwrap(),
            external_level: Some(0.0),
            external_quality: Some(0.0),
            quality: ordering["physical_quality"].as_f64(),
        },
    );
    assert_eq!(
        actual,
        vec![
            applied(native::qty(
                ordering["before_support_level"].as_f64().unwrap(),
                &fixture.count
            )),
            applied(native::qty(
                ordering["before_support_quality"].as_f64().unwrap(),
                &fixture.percent
            )),
        ]
    );
    assert_ne!(ordering["before_support_level"], ordering["final_level"]);
}

const OBSERVATION: &str = r#"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line)
 return f
end
local function upvalue(f,wanted)
 for i=1,100 do local name,value=debug.getupvalue(f,i)
  if not name then break end
  if name==wanted then return value end
 end
 error("missing original closure "..wanted)
end
local load=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local init=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local create=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
local mods=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local apply=original(upvalue(init,"applyGemMods"),"Modules/CalcSetup.lua",550)
local supported=original(upvalue(mods,"getSourceGemPropertyInfo"),"Modules/CalcActiveSkill.lua",236)
local validate=original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local function observed(f)
 local ok,value=pcall(f)
 return ok and {status="ok",value=value} or {status="error",error=tostring(value)}
end
local function initialized()
 assert(debug.gethook()==nil,"component observer cannot replace an existing hook")
 local prepared
 local observer=function(event)
  if event=="call" and debug.getinfo(2,"f").func==create then
   local name,effect=debug.getlocal(2,1)
   assert(name=="activeEffect","original constructor argument identity changed")
   if effect.grantedEffect.id=="SparkPlayer" then
    assert(not prepared,"ambiguous component active effect")
    -- Read actual arguments of the unchanged constructor before active level
    -- validation and support-property delivery. No local/table is modified.
    prepared={level=effect.level,quality=effect.quality}
   end
  end
 end
 debug.sethook(observer,"c")
 local ok,env=pcall(init,build,"MAIN")
 local retained=debug.gethook()==observer
 debug.sethook()
 assert(retained,"source replaced the component observer")
 assert(ok,env)
 return env,assert(prepared,"component preparation was not observed")
end
local function plain(value,depth)
 local kind=type(value)
 if kind~="table" then assert(kind=="nil" or kind=="string" or kind=="number" or kind=="boolean");return value end
 depth=(depth or 0)+1;assert(depth<=8)
 local out={};local count=0
 for key,entry in pairs(value) do
  count=count+1;assert(count<=256 and (type(key)=="number" or type(key)=="string"))
  out[key]=plain(entry,depth)
 end
 return out
end
local function records(values)
 local out={}
 for _,row in ipairs(values or {}) do
  local mod=assert(row.mod);local tags={}
  for _,tag in ipairs(mod) do tags[#tags+1]=plain(tag) end
  out[#out+1]={value=plain(row.value),mod={name=mod.name,type=mod.type,source=mod.source,
   flags=mod.flags,keyword_flags=mod.keywordFlags,value=plain(mod.value),tags=tags}}
 end
 return out
end
local function diagnostic(skill)
 local out={}
 for _,mod in ipairs(skill.skillModList) do
  if mod.name=="GemLevel" or mod.name=="GemItemLevel" or mod.name=="GemItemQuality"
   or mod.name=="GemSupportLevel" or mod.name=="GemSupportQuality" or mod.name=="GemCorruptionLevel" then
   out[#out+1]={name=mod.name,type=mod.type,value=mod.value,source=mod.source}
  end
 end
 return out
end
local savedGroups,savedMain=build.skillsTab.socketGroupList,build.mainSocketGroup
local originalEnv=build.calcsTab.mainEnv
local savedMainSkill=originalEnv.player.mainSkill
local result={untouched_original=observed(function()
 local groupIndex,physical
 for i,group in ipairs(savedGroups) do for _,gem in ipairs(group.gemList) do
  if gem.gemData and gem.gemData.grantedEffect.id==effectiveInputObservedSkill then
   assert(not physical,"ambiguous original physical Gem");groupIndex=i;physical=gem
  end
 end end
 assert(physical,"original physical Gem absent")
 local skill
 for _,candidate in ipairs(originalEnv.player.activeSkillList) do
  if candidate.activeEffect.srcInstance==physical and candidate.activeEffect.grantedEffect.id==effectiveInputObservedSkill then
   assert(not skill,"ambiguous original effect");skill=candidate
  end
 end
 assert(skill,"original effect absent")
 local effect=skill.activeEffect
 local hadCache=originalEnv.sourceGemPropertyInfo and originalEnv.sourceGemPropertyInfo[physical]~=nil
 local supportProperties=supported(originalEnv,skill)
 local items={}
 for slot,item in pairs(originalEnv.player.itemList) do
  items[#items+1]={slot=slot,id=item.id,name=item.name,title=item.title,base_name=item.baseName}
 end
 table.sort(items,function(a,b) return a.slot<b.slot end)
 local supportRows={}
 for _,entry in ipairs(skill.supportList) do
  supportRows[#supportRows+1]={effect=entry.grantedEffect.id,level=entry.level,quality=entry.quality,
   raw_level=entry.srcInstance and entry.srcInstance.level,raw_quality=entry.srcInstance and entry.srcInstance.quality,
   supports_physical_source=not not (entry.isSupporting and entry.isSupporting[physical]),
   external_properties=records(entry.gemPropertyInfo)}
 end
 return {skill=effectiveInputObservedSkill,group_index=groupIndex,main_socket_group=savedMain,
  main_skill=savedMainSkill.activeEffect.grantedEffect.id,is_main_skill=skill==savedMainSkill,
  main_selection_preserved=build.mainSocketGroup==savedMain and originalEnv.player.mainSkill==savedMainSkill,
  raw_level=physical.level,raw_quality=physical.quality,corrupted=not not physical.corrupted,
  corrupt_delta=physical.corruptLevel,final_level=effect.level,final_quality=effect.quality,
  actor_level=skill.minion and skill.minion.level,external_properties=records(effect.gemPropertyInfo),
  supported_properties=records(supportProperties),supported_properties_were_cached=not not hadCache,
  diagnostics=diagnostic(skill),supports=supportRows,items=items}
end),setup_probes={},property_probes={},physical_definitions={}}
for _,id in ipairs({"TwisterPlayer","SummonSkeletalSnipersPlayer","SummonSkeletalClericsPlayer",
 "SupportElementalArmamentPlayerTwo","SupportMeatShieldPlayerTwo"}) do
 local effect=assert(data.skills[id]);local gem=assert(data.gems[assert(data.gemForSkill[effect])])
 local keys={};for level in pairs(effect.levels) do keys[#keys+1]=level end;table.sort(keys)
 local tags={};for tag,enabled in pairs(gem.tags) do if enabled then tags[#tags+1]=tag end end;table.sort(tags)
 local matches={}
 for _,keyword in ipairs({"all","grants_active_skill","minion","spell","attack","support","corrupted"}) do
  matches[keyword]=not not calcLib.gemIsType({gemData=gem,grantedEffect=effect,corrupted=false},keyword,true)
 end
 result.physical_definitions[#result.physical_definitions+1]={effect=id,gem_id=gem.id,
  game_id=gem.gameId,variant_id=gem.variantId,natural_maximum=gem.naturalMaxLevel,level_keys=keys,
  tags=tags,req_str=gem.reqStr,req_dex=gem.reqDex,req_int=gem.reqInt,keyword_matches=matches}
end

if effectiveInputComponents then
 local function group(skill,level,quality,flag,delta)
  local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
  load(tab,{elem="Skill",attrib={enabled="true"},{elem="Gem",attrib={skillId=skill,
   level=tostring(level),quality=tostring(quality),corrupted=tostring(flag),corruptLevel=tostring(delta)}}},1)
  return assert(tab.skillSets[1].socketGroupList[1])
 end
 for _,case in ipairs({
  {name="neutral",flag=false,delta=0,expected=12},
  {name="uncorrupted-positive",flag=false,delta=1,expected=13},
  {name="uncorrupted-fractional",flag=false,delta=0.25,expected=20},
  {name="corrupted-fractional",flag=true,delta=0.25,expected=20},
  {name="lower-clamp",flag=false,delta=-50,expected=1},
  {name="upper-clamp",flag=false,delta=50,expected=40},
 }) do
  local probe=observed(function()
   local active=group("SparkPlayer",12,13,case.flag,case.delta)
   local support=group("SupportRapidCastingPlayerTwo",1,7,true,0.25).gemList[1]
   active.gemList[2]=support
   build.skillsTab.socketGroupList={active};build.mainSocketGroup=1
   local env,prepared=initialized()
   local skill=env.player.mainSkill;local effect=skill.activeEffect;local physical=active.gemList[1]
   assert(effect.srcInstance==physical and effect.grantedEffect.id=="SparkPlayer")
   assert(support.supportEffect and support.supportEffect.srcInstance==support)
   return {raw_level=physical.level,raw_quality=physical.quality,raw_delta=physical.corruptLevel,
    prepared_level=prepared.level,prepared_quality=prepared.quality,
    level=effect.level,quality=effect.quality,corruption_condition=skill.skillCfg.skillCond.GemCorrupted==true,
    external_properties=records(effect.gemPropertyInfo),support_raw_level=support.level,
    support_raw_quality=support.quality,support_delta=support.corruptLevel,
    support_level=support.supportEffect.level,support_quality=support.supportEffect.quality}
  end)
  probe.name=case.name;probe.corrupted=case.flag;probe.expected_level=case.expected
  result.setup_probes[#result.setup_probes+1]=probe
 end
 -- Labelled component modifier inputs bypass parsing only. The exact original
 -- matcher and validator, including all their predicates, produce each result.
 local cases={
  {name="active-all",skill="SparkPlayer",level=12,quality=13,flag=false,
   properties={{keyword="all",key="level",value=2},{keyword="all",key="quality",value=5}},before=14,after=14,q=18,matches=2},
  {name="support-all",skill="SupportRapidCastingPlayerTwo",level=1,quality=7,flag=false,
   properties={{keyword="all",key="level",value=2},{keyword="all",key="quality",value=5}},before=3,after=1,q=12,matches=2},
  {name="active-fractional-property",skill="SparkPlayer",level=12,quality=13,flag=false,
   properties={{keyword="all",key="level",value=0.25}},before=12.25,after=20,q=13,matches=1},
  {name="corrupted-predicate-false",skill="SparkPlayer",level=12,quality=13,flag=false,
   properties={{keyword="corrupted",key="level",value=2}},before=12,after=12,q=13,matches=0},
  {name="corrupted-predicate-true",skill="SparkPlayer",level=12,quality=13,flag=true,
   properties={{keyword="corrupted",key="level",value=2}},before=14,after=14,q=13,matches=1},
  {name="all-keywords-and-strict-requirement",skill="SparkPlayer",level=12,quality=13,flag=false,
   properties={{keywordList={"grants_active_skill","spell"},key="quality",value=5,gemRequirements={reqInt=0}},
    {keywordList={"grants_active_skill","melee"},key="quality",value=100},
    {keyword="all",key="level",value=100,gemRequirements={reqInt=1000}},
    {keyword="all",key="level",value=100,gemRequirements={reqInt=100}}},before=12,after=12,q=18,matches=1},
 }
 for _,case in ipairs(cases) do
  local probe=observed(function()
   local effect=assert(data.skills[case.skill]);local gem=assert(data.gems[assert(data.gemForSkill[effect])])
   local instance={gemData=gem,grantedEffect=effect,level=case.level,quality=case.quality,corrupted=case.flag}
   local input={}
   for i,value in ipairs(case.properties) do
    input[#input+1]={value=value,mod={name="GemProperty",type="LIST",value=value,
     source="TestComponent:"..case.name..":"..i,flags=0,keywordFlags=0}}
   end
   apply(instance,input)
   local before=instance.level
   validate(instance)
   return {before_validation_level=before,level=instance.level,quality=instance.quality,
    raw_level=case.level,raw_quality=case.quality,source_role=effect.support and "support" or "active",
    natural_maximum=gem.naturalMaxLevel,level_rows=#effect.levels,matched_count=#(instance.gemPropertyInfo or {}),
    inputs=records(input),matched=records(instance.gemPropertyInfo)}
  end)
  probe.name=case.name;probe.expected_before=case.before;probe.expected_after=case.after
  probe.expected_quality=case.q;probe.expected_matches=case.matches
  result.property_probes[#result.property_probes+1]=probe
 end
 result.supported_property_probe=observed(function()
  local active=group("SparkPlayer",12,13,false,0)
  build.skillsTab.socketGroupList={active};build.mainSocketGroup=1
  local env=init(build,"MAIN");local skill=env.player.mainSkill;local effect=skill.activeEffect
  local beforeLevel,beforeQuality=effect.level,effect.quality
  -- Controlled component modifiers enter the original actor ModDB. Rebuild the
  -- skill with the unchanged complete function; do not assign computed inputs.
  skill.actor.modDB:NewMod("SupportedGemProperty","LIST",{keyword="grants_active_skill",key="level",value=2},"TestComponent:SupportedLevel")
  skill.actor.modDB:NewMod("SupportedGemProperty","LIST",{keyword="grants_active_skill",key="quality",value=5},"TestComponent:SupportedQuality")
  env.sourceGemPropertyInfo={}
  mods(env,skill)
  local support=group("SupportRapidCastingPlayerTwo",1,7,false,0).gemList[1]
  local supportProperties=supported(env,{activeEffect={srcInstance=support,gemData=support.gemData}})
  return {before_level=beforeLevel,before_quality=beforeQuality,after_level=effect.level,after_quality=effect.quality,
   physical_level=effect.srcInstance.level,physical_quality=effect.srcInstance.quality,
   matched=records(supported(env,skill)),support_gem_properties=records(supportProperties)}
 end)
 result.supported_property_ordering_probe=observed(function()
  local active=group("SparkPlayer",12,13,false,0.25)
  build.skillsTab.socketGroupList={active};build.mainSocketGroup=1
  local savedConfig=build.configTab.modList
  local componentConfig=new("ModList")
  componentConfig:AddList(savedConfig)
  componentConfig:NewMod("SupportedGemProperty","LIST",{keyword="grants_active_skill",key="level",value=0.75},"TestComponent:FractionalSupportedLevel")
  componentConfig:NewMod("SupportedGemProperty","LIST",{keyword="grants_active_skill",key="quality",value=5},"TestComponent:SupportedQuality")
  build.configTab.modList=componentConfig
  -- Complete original initialization with raw physical and modifier inputs.
  -- Early active validation would lose .25 and produce20 instead of13 here.
  local ok,env,prepared=pcall(initialized)
  build.configTab.modList=savedConfig
  assert(ok,env)
  local skill=env.player.mainSkill;local effect=skill.activeEffect
  return {physical_level=effect.srcInstance.level,corruption_delta=effect.srcInstance.corruptLevel,
   before_support_level=prepared.level,before_support_quality=prepared.quality,
   level_adjustment=0.75,final_level=effect.level,physical_quality=effect.srcInstance.quality,
   final_quality=effect.quality,matched=records(supported(env,skill))}
 end)
end
build.skillsTab.socketGroupList=savedGroups;build.mainSocketGroup=savedMain
result.main_selection_restored=build.skillsTab.socketGroupList==savedGroups and build.mainSocketGroup==savedMain
 and build.calcsTab.mainEnv==originalEnv and originalEnv.player.mainSkill==savedMainSkill
assert(calcs.initEnv==init and calcs.buildActiveSkillModList==mods and calcLib.validateGemLevel==validate)
assert(calcs.createActiveSkill==create and debug.gethook()==nil)
assert(upvalue(init,"applyGemMods")==apply and upvalue(mods,"getSourceGemPropertyInfo")==supported)
assert(build.skillsTab.LoadSkill==load)
return result
"#;
