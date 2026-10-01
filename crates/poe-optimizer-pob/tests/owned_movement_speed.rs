//! Complete original item observations; no native movement-metric coverage claim.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
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
const TEST: &str =
    "original_movement_lines_preserve_source_tags_scaling_and_fixed_overlay_semantics";
const CHILD: &str = "POE_MOVEMENT_SPEED_CHILD";

#[test]
fn original_movement_lines_preserve_source_tags_scaling_and_fixed_overlay_semantics() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-movement-speed-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let manifest = read(&fixtures.join("index.json"));
        let mut source_hash = None;
        let mut cases = Vec::new();
        for build in 1..=5 {
            let file = format!("build-{build:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&file)).unwrap();
            let entry = manifest["builds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["xml"] == file)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            let imported = ImportedBuildInstance::from_decoded(
                decode_build(xml.as_bytes()).unwrap(),
                BuildLineage::from_bytes([47; 16]),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let evidence =
                SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
            for (item, ordinal) in match build {
                1 => vec![(5, 298)],
                2 => vec![(4, 482), (22, 605)],
                3 => vec![(11, 295)],
                4 => vec![(12, 211)],
                5 => vec![(14, 543), (22, 578)],
                _ => unreachable!(),
            } {
                let id = item.to_string();
                let rows: Vec<_> = evidence
                    .rows()
                    .iter()
                    .filter(|r| {
                        r.occurrence().name() == "Item"
                            && r.attribute("id").and_then(|a| a.decoded().ok()) == Some(id.as_str())
                    })
                    .collect();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].occurrence().id().ordinal(), ordinal);
            }
            let mut variants = vec![("original", xml.clone())];
            if build == 5 {
                variants.push(("overlay-zero", overlay(&xml, "0")));
                variants.push(("overlay-one", overlay(&xml, "1")));
            }
            for (variant, text) in variants {
                let before = |lua: &Lua| {
                    lua.globals().set("movementXml", text.as_str())?;
                    lua.globals()
                        .set("movementControls", build == 5 && variant == "original")?;
                    lua.globals().set("movementJit", enabled)?;
                    lua.load("if movementJit then jit.on() else jit.off();jit.flush() end")
                        .exec()?;
                    Ok(())
                };
                let temp = tempfile::tempdir().unwrap();
                let result = source::observe_with_build_hook_unwrapped(
                    &root.join("vendor/path-of-building-poe2"),
                    temp.path(),
                    &text,
                    None,
                    false,
                    Some(&before),
                    None,
                    Some(&observe),
                )
                .unwrap();
                assert_eq!(result["configuration_method_wrappers"], false);
                assert_eq!(result["original_build_output_available"], true);
                if let Some(expected) = &source_hash {
                    assert_eq!(&result["source_hash"], expected);
                } else {
                    source_hash = Some(result["source_hash"].clone());
                }
                cases.push(json!({"build":build,"variant":variant,"xml_sha256":digest(text.as_bytes()),"state":result["additional_observation"]}));
            }
            assert_eq!(fs::read_to_string(fixtures.join(&file)).unwrap(), xml);
        }
        let result = json!({"source_hash":source_hash,"evidence":{
            "manifest_sha256":pinned::manifest_sha256(),"native_parity":false,"whole_movement_metric_coverage":false,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Classes/ModDB.lua","src/Modules/CalcSetup.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/ModScalability.lua","src/Data/ModItem.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
        },"cases":cases});
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
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if started.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", path.display());
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
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn overlay(xml: &str, fraction: &str) -> String {
    let opening = "<Item id=\"22\">";
    assert_eq!(xml.matches(opening).count(), 1);
    let begin = xml.find(opening).unwrap() + opening.len();
    let end = begin + xml[begin..].find("</Item>").unwrap();
    let raw = &xml[begin..end];
    assert_eq!(raw.matches("range=\"0.5\"").count(), 1);
    let changed = raw.replace("range=\"0.5\"", &format!("range=\"{fraction}\""));
    format!("{}{}{}", &xml[..begin], changed, &xml[end..])
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-movement-speed-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 7);
    let mut physical = 0;
    let mut untagged = 0;
    let mut custom = 0;
    for case in cases {
        let s = &case["state"];
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(s[flag], true);
        }
        assert_eq!(s["scalability"], json!([{"isScalable":true}]));
        for item in rows(&s["items"]) {
            assert_eq!(item["reparsed_equal"], true);
            let line = &item["loaded"]["lines"][0];
            assert_eq!(rows(&item["loaded"]["lines"]).len(), 1);
            assert_eq!(line["category"], "explicit");
            assert_eq!(line["field_types"]["extra"], "nil");
            assert_eq!(line["field_types"]["corrupted_range"], "nil");
            assert_eq!(line["value_scalar"], 1);
            assert_eq!(line["catalyst_factor"], 1);
            let active = rows(&item["loaded"]["active"]);
            assert_eq!(active.len(), 1);
            assert_eq!(active[0]["type"], "INC");
            assert_eq!(active[0]["flags"], 0);
            assert_eq!(active[0]["keyword_flags"], 0);
            assert!(rows(&active[0]["tags"]).is_empty());
            assert!(
                active[0]["source"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("Item:{}:", item["id"].as_u64().unwrap()))
            );
            if case["variant"] == "original" {
                physical += 1;
                if case["build"] == 2 && item["id"] == 4 {
                    custom += 1;
                    assert_eq!(line["mod_tags"], json!(["speed"]));
                    assert_eq!(line["custom"], true);
                } else {
                    untagged += 1;
                    assert!(rows(&line["mod_tags"]).is_empty());
                    assert!(line["custom"].is_null());
                }
                let expected = match (
                    case["build"].as_u64().unwrap(),
                    item["id"].as_u64().unwrap(),
                ) {
                    (1, 5) | (3, 11) => 30,
                    (2, 4) | (5, 22) => 10,
                    (2, 22) | (5, 14) => 20,
                    (4, 12) => 35,
                    _ => panic!("unexpected original item"),
                };
                assert_eq!(active[0]["value"], expected);
                if let Some((name, value)) = match (
                    case["build"].as_u64().unwrap(),
                    item["id"].as_u64().unwrap(),
                ) {
                    (1, 5) => Some(("EnergyShield", 44)),
                    (3, 11) => Some(("Evasion", 134)),
                    (4, 12) => Some(("Armour", 77)),
                    _ => None,
                } {
                    let defence = rows(&item["loaded"]["defence_lines"]);
                    assert_eq!(defence, rows(&item["fresh"]["defence_lines"]));
                    let matching: Vec<_> = defence
                        .iter()
                        .filter(|line| line["records"][0]["name"] == name)
                        .collect();
                    assert_eq!(matching.len(), 1);
                    let line = matching[0];
                    assert_eq!(line["category"], "explicit");
                    assert_eq!(line["value_scalar"], 1);
                    assert_eq!(line["catalyst_factor"], 1);
                    assert_eq!(line["field_types"]["extra"], "nil");
                    assert_eq!(line["field_types"]["corrupted_range"], "nil");
                    assert!(rows(&line["mod_tags"]).is_empty());
                    assert_eq!(rows(&line["records"]).len(), 1);
                    let record = &line["records"][0];
                    assert_eq!(record["name"], name);
                    assert_eq!(record["type"], "BASE");
                    assert_eq!(record["value"], value);
                    assert_eq!(record["flags"], 0);
                    assert_eq!(record["keyword_flags"], 0);
                    assert!(rows(&record["tags"]).is_empty());
                    assert_eq!(record["source"], active[0]["source"]);
                    // Local defence may be consumed into item armour data. The
                    // exact source record is not an assumed player contribution.
                    assert_eq!(
                        item["loaded"]["defence_active"],
                        item["fresh"]["defence_active"]
                    );
                }
            }
        }
        if case["build"] == 5 {
            assert_eq!(s["selected_items"], 2);
            assert_eq!(s["selected_spec"], 3);
            assert_eq!(s["selected_skills"], 4);
            assert_eq!(s["selected_config"], 1);
            let target = rows(&s["items"]).iter().find(|r| r["id"] == 22).unwrap();
            assert_eq!(target["loaded"]["base"], "Cryptic Leggings");
            assert_eq!(target["loaded"]["field_types"]["item_level"], "nil");
            assert_eq!(target["loaded"]["active"][0]["value"], 10);
            assert_eq!(
                target["loaded"]["lines"][0]["range"],
                match case["variant"].as_str().unwrap() {
                    "original" => 0.5,
                    "overlay-zero" => 0.0,
                    "overlay-one" => 1.0,
                    _ => unreachable!(),
                }
            );
            assert_eq!(target["catalogue_tags"], json!(["speed"]));
            for path in ["item_database", "player_database"] {
                let records = rows(&target[path]);
                assert_eq!(records.len(), 1);
                assert_eq!(records[0], target["loaded"]["active"][0]);
            }
        }
    }
    assert_eq!((physical, untagged, custom), (7, 6, 1));
    let s = &cases
        .iter()
        .find(|r| r["build"] == 5 && r["variant"] == "original")
        .unwrap()["state"];
    let probes = rows(&s["probes"]);
    assert_eq!(probes.len(), 19);
    for (name, value) in [
        ("fixed", 10),
        ("fraction-down", 10),
        ("fraction-half", 11),
        ("negative-half", -11),
        ("untagged-catalyst", 10),
        ("tagged-catalyst", 12),
        ("wrong-catalyst", 10),
        ("explicit-magnitude", 15),
        ("implicit-mismatch", 10),
        ("untagged-speed-magnitude", 10),
        ("tagged-speed-magnitude", 15),
        ("add-then-double", 30),
        ("double-then-add", 25),
        ("corrupted-fixed", 13),
        ("corrupted-fixed-magnitude", 15),
        ("corrupted-ranged-magnitude", 19),
        ("range-midpoint", 15),
        ("unscalable", 10),
    ] {
        let p = probes.iter().find(|r| r["name"] == name).unwrap();
        assert_eq!(rows(&p["after_build"]["active"]).len(), 1, "{name}");
        assert_eq!(p["after_build"]["active"][0]["value"], value, "{name}");
    }
    assert!(
        rows(&probes.iter().find(|r| r["name"] == "disabled").unwrap()["after_build"]["active"])
            .is_empty()
    );
    let levels = &s["item_level"];
    assert_eq!(levels["raw_required_level"], 80);
    for field in ["fresh_original", "fresh_after_removal"] {
        assert_eq!(levels[field]["field_type"], "nil");
    }
    assert_eq!(levels["header_present"]["value"], 37);
    assert_eq!(levels["reused_after_removal"]["value"], 37);
}
const OBSERVE: &str = r##"
local function original(f,path,first,last)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last) end;return f
end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local formatter=original(itemLib.formatValue,"Modules/ItemTools.lua",45,58)
local range=original(itemLib.applyRange,"Modules/ItemTools.lua",130)
local parser=original(modLib.parseMod,"Modules/ModParser.lua",7404)
local function plain(value,depth)
 if type(value)~="table" then assert(type(value)~="function" and type(value)~="userdata");return value end
 depth=(depth or 0)+1;assert(depth<9);local out={};local count=0;for k,v in pairs(value) do count=count+1;assert(count<=256);out[k]=plain(v,depth)end;return out
end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function record(mod)local tags={};for _,v in ipairs(mod)do tags[#tags+1]=plain(v)end;return{name=mod.name,type=mod.type,value=mod.value,source=mod.source,flags=mod.flags,keyword_flags=mod.keywordFlags,tags=tags}end
local function movement(list,source)
 local out={};for _,mod in ipairs(list or {})do if mod.name=="MovementSpeed" and mod.type=="INC" and (not source or mod.source==source)then out[#out+1]=record(mod)end end;return out
end
local function defence(list,source)
 local out={};for _,mod in ipairs(list or {})do if (mod.name=="EnergyShield" or mod.name=="Evasion" or mod.name=="Armour") and mod.type=="BASE" and (not source or mod.source==source)then out[#out+1]=record(mod)end end;return out
end
local function snapshot(item,list)
 local lines,defences={},{};for _,category in ipairs({"enchant","rune","implicit","explicit"})do
  for _,line in ipairs(item[category.."ModLines"])do if line.line:find(" Movement Speed",1,true) and #movement(line.modList)>0 then
   lines[#lines+1]={line=line.line,category=category,mod_tags=plain(line.modTags),custom=line.custom,prefix=line.prefix,suffix=line.suffix,unscalable=line.unscalable,disabled=line.disabled,
    range=line.range,corrupted_range=line.corruptedRange,value_scalar=line.valueScalar,extra=line.extra,
    catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality),records=movement(line.modList),field_types={extra=type(line.extra),corrupted_range=type(line.corruptedRange)}}
  end
  local records=defence(line.modList);if #records>0 then defences[#defences+1]={line=line.line,category=category,mod_tags=plain(line.modTags),value_scalar=line.valueScalar,
   catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality),records=records,field_types={extra=type(line.extra),corrupted_range=type(line.corruptedRange)}} end
  end
 end
 return{base=item.baseName,kind=item.type,raw=item.raw,item_level=item.itemLevel,required_level=item.requirements.level,catalyst=item.catalyst,catalyst_quality=item.catalystQuality,quality=item.quality,
  field_types={item_level=type(item.itemLevel),catalyst=type(item.catalyst),catalyst_quality=type(item.catalystQuality)},lines=lines,active=movement(list),
  defence_lines=defences,defence_active=defence(list),armour_data=plain(item.armourData)}
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);return item end
local doc,err=common.xml.ParseXML(movementXml);assert(doc and not err)
local node;for _,n in ipairs(doc[1])do if type(n)=="table" and n.elem=="Items"then assert(not node);node=n end end;assert(node)
local rawItems,ids={},{}
for _,n in ipairs(node)do if type(n)=="table" and n.elem=="Item"then
 local text={};for _,s in ipairs(n)do if type(s)=="string"then text[#text+1]=s end end;local raw=table.concat(text,"\n");local id=assert(tonumber(n.attrib.id));rawItems[id]=raw
 if raw:find("%% increased Movement Speed") then ids[#ids+1]=id end
end end;table.sort(ids)
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},probes={},scalability=plain(assert(data.modScalability["#% increased Movement Speed"]))}
local originals,snapshots={},{}
for _,id in ipairs(ids)do
 local item=assert(build.itemsTab.items[id]);originals[id]=item;local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local fresh=construct(rawItems[id],id);mods(fresh);local again=snapshot(fresh,active(fresh,1,false))
 -- Legacy range writes are loaded separately; they cannot alter this fixed line's value.
 assert(equal(loaded.active,again.active));local a,b=plain(loaded.lines),plain(again.lines);for _,line in ipairs(a)do line.range=nil end;for _,line in ipairs(b)do line.range=nil end;assert(equal(a,b))
 local selected={};for slot,v in pairs(env.player.itemList)do if v==item then selected[#selected+1]=slot end end;table.sort(selected)
 result.items[#result.items+1]={id=id,raw=rawItems[id],loaded=loaded,fresh=again,reparsed_equal=true,selected_slots=selected,
  item_database=movement(env.itemModDB.mods.MovementSpeed,item.modSource),player_database=movement(env.player.modDB.mods.MovementSpeed,item.modSource),
  catalogue_tags=item.affixes.MovementVelocity1 and plain(item.affixes.MovementVelocity1.modTags),defence_item_database={},defence_player_database={}}
 local target=result.items[#result.items];for _,name in ipairs({"Armour","Evasion","EnergyShield"})do
  target.defence_item_database[name]=defence(env.itemModDB.mods[name],item.modSource);target.defence_player_database[name]=defence(env.player.modDB.mods[name],item.modSource)
 end
end
if movementControls then
 local raw=assert(rawItems[22]);assert(not raw:find("Item Level:",1,true));local needle="10% increased Movement Speed";local at=assert(raw:find(needle,1,true));assert(not raw:find(needle,at+#needle,true))
 local function changed(line,headers,after)
  local value=raw:sub(1,at-1)..line..raw:sub(at+#needle)
  if headers then local pos=assert(value:find("Crafted: true",1,true));value=value:sub(1,pos-1)..headers.."\n"..value:sub(pos)end
  if after then value=value.."\n"..after end;return value
 end
 local cases={
  {"fixed","10% increased Movement Speed"},{"fraction-down","10.49% increased Movement Speed"},{"fraction-half","10.5% increased Movement Speed"},{"negative-half","-10.5% increased Movement Speed"},
  {"untagged-catalyst","10% increased Movement Speed","Catalyst: Skittering\nCatalystQuality: 20"},
  {"tagged-catalyst","{tags:speed}10% increased Movement Speed","Catalyst: Skittering\nCatalystQuality: 20"},
  {"wrong-catalyst","{tags:speed}10% increased Movement Speed","Catalyst: Flesh\nCatalystQuality: 20"},
  {"explicit-magnitude","10% increased Movement Speed",nil,"50% increased explicit modifier magnitudes"},
  {"implicit-mismatch","10% increased Movement Speed",nil,"50% increased implicit modifier magnitudes"},
  {"untagged-speed-magnitude","10% increased Movement Speed",nil,"50% increased speed modifier magnitudes"},
  {"tagged-speed-magnitude","{tags:speed}10% increased Movement Speed",nil,"50% increased speed modifier magnitudes"},
  {"add-then-double","10% increased Movement Speed",nil,"50% increased explicit modifier magnitudes\nExplicit Modifier magnitudes are doubled"},
  {"double-then-add","10% increased Movement Speed",nil,"Explicit Modifier magnitudes are doubled\n50% increased explicit modifier magnitudes"},
  {"corrupted-fixed","{corruptedRange:1.25}10% increased Movement Speed"},
  {"corrupted-fixed-magnitude","{corruptedRange:1.25}10% increased Movement Speed",nil,"50% increased explicit modifier magnitudes"},
  {"corrupted-ranged-magnitude","{range:0.5}{corruptedRange:1.25}(10-10)% increased Movement Speed",nil,"50% increased explicit modifier magnitudes"},
  {"range-midpoint","{range:0.5}(10-20)% increased Movement Speed"},
  {"unscalable","{unscalable}{tags:speed}10% increased Movement Speed","Catalyst: Skittering\nCatalystQuality: 20","50% increased explicit modifier magnitudes"},
  {"disabled","{disabled}10% increased Movement Speed"}
 }
 for _,case in ipairs(cases)do
  local text=changed(case[2],case[3],case[4]);local item=construct(text,9001);local before=snapshot(item,nil);mods(item);local after=snapshot(item,active(item,1,false));assert(equal(after,snapshot(item,active(item,1,false))))
  result.probes[#result.probes+1]={name=case[1],raw=text,input_line=case[2],before_build=before,after_build=after}
 end
 local function level(item)return{field_type=type(item.itemLevel),value=item.itemLevel}end
 local fresh=construct(raw,9002);local withHeader=changed(needle,"Item Level: 37");local reused=construct(withHeader,9003)
 result.item_level={raw_required_level=assert(tonumber(raw:match("LevelReq: (%d+)"))),fresh_original=level(fresh),header_present=level(reused)}
 parse(reused,raw);mods(reused);result.item_level.reused_after_removal=level(reused)
 local reset=construct(raw,9004);result.item_level.fresh_after_removal=level(reset)
end
for _,id in ipairs(ids)do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and itemLib.formatValue==formatter and itemLib.applyRange==range and modLib.parseMod==parser and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
return result
"##;
