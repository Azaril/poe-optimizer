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

const CENSUS_TEST: &str =
    "unchanged_original05_movement_supplier_census_preserves_raw_and_filtered_sources";
const CENSUS_CHILD: &str = "POE_MOVEMENT_SUPPLIER_CENSUS_CHILD";
const CENSUS_OUTPUT: &str = "POE_OPTIMIZER_MOVEMENT_SUPPLIER_CENSUS_OUTPUT";
const ORIGINAL05_SHA256: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";

/// Source inventory at one fixed normal-load lifecycle, not native closure or parity.
#[test]
fn unchanged_original05_movement_supplier_census_preserves_raw_and_filtered_sources() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let explicit_output = std::env::var_os(CENSUS_OUTPUT).map(PathBuf::from);
    let child_mode = std::env::var_os(CENSUS_CHILD);
    let out = if let Some(path) = &explicit_output {
        path.clone()
    } else {
        assert!(
            child_mode.is_none(),
            "child requires the parent's exact evidence directory"
        );
        let runs = root.join("runs");
        fs::create_dir_all(&runs).unwrap();
        tempfile::Builder::new()
            .prefix("owned-movement-supplier-census-")
            .tempdir_in(runs)
            .unwrap()
            .keep()
    };
    assert!(out.is_absolute(), "census evidence path must be absolute");
    let input = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = fs::read_to_string(&input).unwrap();
    let owned_joins = movement_census_owned_joins(&root);
    assert_eq!(digest(xml.as_bytes()), ORIGINAL05_SHA256);
    if let Some(mode) = child_mode {
        assert!(
            out.is_dir(),
            "parent must establish the fresh evidence directory"
        );
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let mut replays = Vec::new();
        for replay in 1..=2 {
            let before = |lua: &Lua| {
                lua.globals().set("movementCensusJit", enabled)?;
                lua.load("if movementCensusJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let scratch = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                scratch.path(),
                &xml,
                None,
                false,
                Some(&before),
                None,
                Some(&movement_census_observe),
            )
            .unwrap();
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            let observed = json!({
                "source_hash": result["source_hash"],
                "xml_sha256": ORIGINAL05_SHA256,
                "owned_join_evidence": owned_joins,
                "lifecycle": "fresh VM; normal unchanged XML load; read-only post-load observer",
                "state": result["additional_observation"],
                "evidence": {
                    "manifest_sha256": pinned::manifest_sha256(),
                    "files": ["src/Classes/ModDB.lua", "src/Classes/ModStore.lua",
                        "src/Classes/Item.lua", "src/Modules/CalcSetup.lua",
                        "src/Modules/CalcDefence.lua", "src/Modules/CalcPerform.lua",
                        "src/Modules/CalcTools.lua", "src/Modules/Data.lua"]
                        .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
                    "native_parity": false,
                    "complete_native_supplier_coverage": false,
                    "whole_build_legality": false
                }
            });
            // Keep each attempt before any comparison; a discrepancy is never overwritten.
            movement_census_write_new(
                &out.join(format!(
                    "jit-{}-fresh-{replay}.json",
                    if enabled { "on" } else { "off" }
                )),
                &serde_json::to_vec_pretty(&observed).unwrap(),
            );
            movement_census_check(&observed);
            replays.push(observed);
            assert_eq!(fs::read_to_string(&input).unwrap(), xml);
        }
        assert_eq!(
            replays[0], replays[1],
            "independent fresh ordinary loads diverged"
        );
        return;
    }
    if explicit_output.is_some() {
        fs::create_dir(&out)
            .expect("use a fresh census output directory; never replace earlier evidence");
    }
    eprintln!(
        "retained movement supplier census evidence: {}",
        out.display()
    );
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CENSUS_TEST, "--nocapture"])
            .env(CENSUS_CHILD, mode)
            .env(CENSUS_OUTPUT, &out)
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
    let first = fs::read(out.join("jit-off-fresh-1.json")).unwrap();
    for name in [
        "jit-off-fresh-2.json",
        "jit-on-fresh-1.json",
        "jit-on-fresh-2.json",
    ] {
        assert_eq!(
            first,
            fs::read(out.join(name)).unwrap(),
            "fresh/JIT evidence differs: {name}"
        );
    }
    movement_census_write_new(
        &out.join("validated.json"),
        &serde_json::to_vec_pretty(&json!({
            "xml_sha256": ORIGINAL05_SHA256,
            "observations": 4,
            "matched_fresh_lifecycle_and_jit": true,
            "observation_sha256": digest(&first),
            "observation_bytes": first.len(),
            "native_coverage": "not established"
        }))
        .unwrap(),
    );
}
fn movement_census_owned_joins(root: &Path) -> Json {
    let base = root.join("data/owned/poe2/3887ae68/movement-speed");
    let inputs = [
        (
            "bindings.json",
            "4580598b8b4db6f22be3913c25127057fd478cc5c6900baa89da852cc960f8e6",
        ),
        (
            "equipment-penalty.json",
            "e191d8acc73ebb014c99bd2489d4b2b516dafeb9195c018e6a828a9d72a7e80d",
        ),
        (
            "equipment-penalty-source.json",
            "6c0cef67702d866854110b8556cbf91b1c4d9d639d12bc16caa03419f20c810e",
        ),
    ];
    let mut values = Vec::new();
    let mut pins = Vec::new();
    for (path, expected) in inputs {
        let bytes = fs::read(base.join(path)).unwrap();
        assert_eq!(
            digest(&bytes),
            expected,
            "owned join evidence changed: {path}"
        );
        values.push(serde_json::from_slice::<Json>(&bytes).unwrap());
        pins.push(json!({"path":path,"sha256":expected,"bytes":bytes.len()}));
    }
    assert_eq!(values[0]["modifier"]["key"], "def.00000000000030e6");
    assert_eq!(values[0]["contribution"]["key"], "def.00000000000030ff");
    assert_eq!(
        values[1]["program"]["effects"][0]["effect"]["stat"]["key"],
        "def.00000000000033cc"
    );
    assert_eq!(
        values[2]["selected_original05"],
        json!({
            "item":19,"base":"Tattered Robe","template":"def.000000000000238c",
            "slot":"def.0000000000000067","raw":0.03
        })
    );
    json!({"artifacts":pins,"meaning":"exact source-to-authored-channel joins; not complete owner coverage"})
}
fn movement_census_write_new(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
fn movement_census_observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(MOVEMENT_CENSUS_OBSERVE)
        .set_name("@owned-movement-supplier-census-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn movement_census_check(report: &Json) {
    let state = &report["state"];
    assert_eq!(state["recipient"], "Player");
    assert_eq!(state["mode"], "MAIN");
    assert_eq!(
        state["selected"],
        json!({"items":2,"spec":3,"skills":4,"config":1})
    );
    for field in [
        "main_output_graph_preserved",
        "source_database_records_preserved",
        "saved_items_preserved",
        "saved_selections_preserved",
        "original_functions_preserved",
    ] {
        assert_eq!(state[field], true, "{field}");
    }
    let expected = [
        ("MovementSpeed", "BASE"),
        ("MovementSpeed", "INC"),
        ("MovementSpeed", "MORE"),
        ("MovementSpeed", "OVERRIDE"),
        ("Condition:IgnoreMovementPenalties", "FLAG"),
        ("MovementSpeedEqualHighestLinkedPlayers", "FLAG"),
        ("MovementSpeedCannotBeBelowBase", "FLAG"),
        ("ActionSpeed", "INC"),
        ("TemporalChainsActionSpeed", "INC"),
        ("MinimumActionSpeed", "MAX"),
        ("MaximumActionSpeedReduction", "MAX"),
        ("UnaffectedBySlows", "FLAG"),
    ];
    let channels = rows(&state["channels"]);
    assert_eq!(channels.len(), expected.len());
    let mut retained_but_not_tabulated = 0;
    for (channel, (name, kind)) in channels.iter().zip(expected) {
        assert_eq!(channel["name"], name);
        assert_eq!(channel["kind"], kind);
        assert_eq!(channel["recipient"], "Player");
        let raw = rows(&channel["raw"]);
        let filtered = rows(&channel["tabulated"]);
        for (ordinal, source) in raw.iter().enumerate() {
            assert_eq!(source["raw_index"], ordinal + 1);
            assert_eq!(source["mod"]["name"], name);
            assert_eq!(source["mod"]["type"], kind);
            if source["tabulated_count"] == 0 {
                retained_but_not_tabulated += 1;
            }
        }
        for source in filtered {
            let index = source["raw_index"].as_u64().unwrap() as usize;
            assert!((1..=raw.len()).contains(&index));
            assert_eq!(source["parent_depth"], raw[index - 1]["parent_depth"]);
            assert_eq!(source["bucket_index"], raw[index - 1]["bucket_index"]);
        }
        assert!(channel["aggregate"]["type"].is_string());
    }
    assert!(
        retained_but_not_tabulated > 0,
        "retain zero/conditional sources rather than only active totals"
    );
    let source_row = |channel: usize, item: u64| {
        let selected = rows(&state["selected_items"])
            .iter()
            .find(|v| v["id"] == item)
            .unwrap();
        let matching: Vec<_> = rows(&channels[channel]["raw"])
            .iter()
            .filter(|v| v["mod"]["source"] == selected["mod_source"])
            .collect();
        assert_eq!(matching.len(), 1);
        matching[0]
    };
    let body = source_row(0, 19);
    assert_eq!(body["mod"]["value"], -0.03);
    assert_eq!(
        body["mod"]["tags"],
        json!([{"type":"Condition","var":"IgnoreMovementPenalties","neg":true}])
    );
    assert_eq!(body["tabulated_count"], 1);
    let boots = source_row(1, 22);
    assert_eq!(boots["mod"]["value"], 10);
    assert!(rows(&boots["mod"]["tags"]).is_empty());
    assert_eq!(boots["tabulated_count"], 1);
    assert_eq!(
        state["existing_owned_joins"]["boots_increased"],
        "def.00000000000030ff"
    );
    assert_eq!(
        state["existing_owned_joins"]["body_base"],
        "def.00000000000033cc"
    );
    assert_eq!(
        state["action_speed_recomputed"],
        state["output"]["ActionSpeedMod"]
    );
}
const MOVEMENT_CENSUS_OBSERVE: &str = r##"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==line);return f
end
assert(jit.status()==movementCensusJit,"requested JIT mode changed during load")
local store=common.classes.ModStore;local dbclass=common.classes.ModDB
local methods={Sum=original(store.Sum,"Classes/ModStore.lua",202),More=original(store.More,"Classes/ModStore.lua",261),Flag=original(store.Flag,"Classes/ModStore.lua",281),Override=original(store.Override,"Classes/ModStore.lua",301),Tabulate=original(store.Tabulate,"Classes/ModStore.lua",345),Max=original(store.Max,"Classes/ModStore.lua",369),GetCondition=original(store.GetCondition,"Classes/ModStore.lua",409),EvalMod=original(store.EvalMod,"Classes/ModStore.lua",490),SumPositiveValues=original(store.SumPositiveValues,"Classes/ModStore.lua",228)}
local internals={SumInternal=original(dbclass.SumInternal,"Classes/ModDB.lua",137),MoreInternal=original(dbclass.MoreInternal,"Classes/ModDB.lua",214),FlagInternal=original(dbclass.FlagInternal,"Classes/ModDB.lua",297),OverrideInternal=original(dbclass.OverrideInternal,"Classes/ModDB.lua",344),TabulateInternal=original(dbclass.TabulateInternal,"Classes/ModDB.lua",438)}
local actionSpeed=original(calcs.actionSpeedMod,"Modules/CalcPerform.lua",1021)
local function plain(value,depth,seen)
 local t=type(value);assert(t=="nil" or t=="number" or t=="string" or t=="boolean" or t=="table","unsupported evidence value "..t)
 if t~="table" then if t=="number" then assert(value==value and math.abs(value)<math.huge) end;return value end
 depth=(depth or 0)+1;assert(depth<=16);seen=seen or {};assert(not seen[value],"cyclic evidence value");seen[value]=true
 local out,n={},0;for k,v in pairs(value) do n=n+1;assert(n<=16384 and (type(k)=="string" or type(k)=="number"));out[k]=plain(v,depth,seen)end;seen[value]=nil;return out
end
local function typed(value)return{type=type(value),value=plain(value)}end
-- Snapshot every reachable table/field without serializing pointers or dropping nested output.
local function guard(roots)
 local visited,records,total={},{},0
 local function walk(t)
  if type(t)~="table" or visited[t] then return end;visited[t]=true;assert(#records<100000)
  local r={value=t,meta=getmetatable(t),entries={}};records[#records+1]=r
  for k,v in pairs(t) do total=total+1;assert(total<=1000000);r.entries[#r.entries+1]={k,v};walk(k);walk(v)end
 end
 for _,v in ipairs(roots) do walk(v)end
 return function()
  for _,r in ipairs(records)do
   assert(getmetatable(r.value)==r.meta);local n=0;for _ in pairs(r.value)do n=n+1 end;assert(n==#r.entries)
   for _,p in ipairs(r.entries)do local actual=rawget(r.value,p[1]);assert(actual==p[2] or (type(actual)=="number" and actual~=actual and p[2]~=p[2]),"source table changed during observation")end
  end
 end
end
local env=assert(build.calcsTab.mainEnv);local output=assert(build.calcsTab.mainOutput);assert(env.mode=="MAIN")
local actor=assert(env.player);local db=assert(actor.modDB);assert(db==env.modDB)
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId}
local savedGroup=build.mainSocketGroup
local checkOutput=guard({output})
local channels={{"MovementSpeed","BASE"},{"MovementSpeed","INC"},{"MovementSpeed","MORE"},{"MovementSpeed","OVERRIDE"},{"Condition:IgnoreMovementPenalties","FLAG"},{"MovementSpeedEqualHighestLinkedPlayers","FLAG"},{"MovementSpeedCannotBeBelowBase","FLAG"},{"ActionSpeed","INC"},{"TemporalChainsActionSpeed","INC"},{"MinimumActionSpeed","MAX"},{"MaximumActionSpeedReduction","MAX"},{"UnaffectedBySlows","FLAG"}}
local roots,chain,seen={}, {}, {};local current=db
while current do
 assert(type(current)=="table" and not seen[current] and #chain<32);seen[current]=true;assert(type(current.mods)=="table")
 chain[#chain+1]=current
 roots[#roots+1]=current.conditions;roots[#roots+1]=current.multipliers
 for _,c in ipairs(channels)do local bucket=current.mods[c[1]];if bucket then roots[#roots+1]=bucket end end
 current=current.parent
end
local layerSnapshots={}
for _,layer in ipairs(chain)do
 local s={value=layer,parent=layer.parent,mods=layer.mods,conditions=layer.conditions,multipliers=layer.multipliers,buckets={}}
 for index,c in ipairs(channels)do s.buckets[index]={name=c[1],value=layer.mods[c[1]]}end
 layerSnapshots[#layerSnapshots+1]=s
end
local checkRecords=guard(roots)
local result={recipient="Player",mode=env.mode,selected=saved,channels={},database_layers={},selected_items={},selected_groups={},active_skills={},allocated_nodes={},output={},existing_owned_joins={boots_increased="def.00000000000030ff",body_base="def.00000000000033cc"}}
for depth,layer in ipairs(chain)do result.database_layers[#result.database_layers+1]={parent_depth=depth-1,conditions=plain(layer.conditions),multipliers=plain(layer.multipliers)}end
local function record(mod)
 assert(mod.tags==nil,"unexpected named tags field must not be overwritten")
 local tags,fields={},{};for _,tag in ipairs(mod)do tags[#tags+1]=plain(tag)end
 for k,v in pairs(mod)do if type(k)=="string" then fields[k]=plain(v) else assert(type(k)=="number" and k>=1 and k<=#mod and k==math.floor(k))end end
 fields.tags=tags;return fields
end
for _,channel in ipairs(channels)do
 local name,kind=channel[1],channel[2];local row={name=name,kind=kind,recipient="Player",query_configuration="nil",raw={},tabulated={}};local pointers={}
 for depth,layer in ipairs(chain)do
  for bucketIndex,mod in ipairs(layer.mods[name] or {})do if mod.type==kind then
   local index=#row.raw+1;row.raw[index]={raw_index=index,parent_depth=depth-1,bucket_index=bucketIndex,mod=record(mod),tabulated_count=0}
   pointers[mod]=pointers[mod] or {};pointers[mod][#pointers[mod]+1]=index
  end end
 end
 local used={}
 for _,entry in ipairs(db:Tabulate(kind,nil,name))do
  local indices=assert(pointers[entry.mod],"tabulated contribution absent from complete raw parent chain");local count=(used[entry.mod] or 0)+1;used[entry.mod]=count
  local index=assert(indices[count],"tabulated pointer multiplicity exceeds raw records");local raw=row.raw[index];raw.tabulated_count=raw.tabulated_count+1
  row.tabulated[#row.tabulated+1]={raw_index=index,parent_depth=raw.parent_depth,bucket_index=raw.bucket_index,value=typed(entry.value)}
 end
 if kind=="BASE" or kind=="INC" then row.aggregate=typed(db:Sum(kind,nil,name))
 elseif kind=="MORE" then row.aggregate=typed(db:More(nil,name))
 elseif kind=="FLAG" then row.aggregate=typed(db:Flag(nil,name))
 elseif kind=="OVERRIDE" then row.aggregate=typed(db:Override(nil,name))
 elseif kind=="MAX" then row.aggregate=typed(db:Max(nil,name)) else error("unexpected channel")end
 if name=="ActionSpeed" or name=="TemporalChainsActionSpeed" then row.positive_sum=typed(db:SumPositiveValues(kind,nil,name))end
 result.channels[#result.channels+1]=row
end
-- A direct condition value is a separate source path from the Condition:* FLAG inventory.
result.ignore_movement_penalties_condition=typed(db:GetCondition("IgnoreMovementPenalties",nil))
result.temporal_chains_effect_cap=typed(data.misc.TemporalChainsEffectCap)
result.action_speed_recomputed=typed(actionSpeed(actor))
for _,name in ipairs({"MovementSpeedMod","ActionSpeedMod","EffectiveMovementSpeedMod"})do result.output[name]=typed(output[name])end
result.party_movement_output=typed(actor.partyMembers and actor.partyMembers.output and actor.partyMembers.output.MovementSpeedMod)
local itemSlots={};for slot in pairs(actor.itemList)do assert(type(slot)=="string");itemSlots[#itemSlots+1]=slot end;table.sort(itemSlots)
local itemSnapshots={}
for _,slot in ipairs(itemSlots)do
 local item=actor.itemList[slot];assert(type(item)=="table");itemSnapshots[#itemSnapshots+1]={slot=slot,item=item,id=item.id,raw=item.raw,source=item.modSource}
 result.selected_items[#result.selected_items+1]={slot=slot,id=item.id,base=item.baseName,mod_source=item.modSource,raw=item.raw}
end
local gemSources={}
for groupIndex,group in ipairs(build.skillsTab.socketGroupList)do
 local row={group=groupIndex,source=typed(group.source),enabled=typed(group.enabled),slot_enabled=typed(group.slotEnabled),slot=typed(group.slot),gems={}}
 for gemIndex,gem in ipairs(group.gemList)do
  assert(not gemSources[gem],"one physical source appears in multiple saved group positions")
  gemSources[gem]={group=groupIndex,gem=gemIndex}
  row.gems[#row.gems+1]={gem=gemIndex,enabled=typed(gem.enabled),gem_id=typed(gem.gemId),skill_id=typed(gem.skillId),from_item=typed(gem.fromItem),from_node=typed(gem.fromNode),loaded_source_level=typed(gem.level),loaded_source_quality=typed(gem.quality),enable_global1=typed(gem.enableGlobal1),enable_global2=typed(gem.enableGlobal2)}
 end
 result.selected_groups[#result.selected_groups+1]=row
end
for index,skill in ipairs(actor.activeSkillList)do
 local effect=assert(skill.activeEffect);local physical=gemSources[effect.srcInstance]
 result.active_skills[#result.active_skills+1]={index=index,effect_id=typed(effect.grantedEffect and effect.grantedEffect.id),source_resolved=physical~=nil,source=physical and plain(physical),computed_level=typed(effect.level),computed_quality=typed(effect.quality),enabled=typed(skill.enabled),skill_flags=plain(skill.skillFlags)}
end
local nodeIds={};for id in pairs(env.allocNodes)do assert(type(id)=="number");nodeIds[#nodeIds+1]=id end;table.sort(nodeIds)
for _,id in ipairs(nodeIds)do local node=env.allocNodes[id];result.allocated_nodes[#result.allocated_nodes+1]={id=id,name=node.name,stats=plain(node.stats),allocation_mode=typed(node.allocMode)}end
checkOutput();result.main_output_graph_preserved=true
checkRecords()
for _,s in ipairs(layerSnapshots)do
 assert(s.value.parent==s.parent and s.value.mods==s.mods and s.value.conditions==s.conditions and s.value.multipliers==s.multipliers)
 for _,b in ipairs(s.buckets)do assert(s.value.mods[b.name]==b.value)end
end
result.source_database_records_preserved=true
for _,s in ipairs(itemSnapshots)do assert(actor.itemList[s.slot]==s.item and s.item.id==s.id and s.item.raw==s.raw and s.item.modSource==s.source)end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==savedGroup);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output and actor.modDB==db and env.modDB==db)
for name,f in pairs(methods)do assert(store[name]==f)end;for name,f in pairs(internals)do assert(dbclass[name]==f)end;assert(calcs.actionSpeedMod==actionSpeed);result.original_functions_preserved=true
return result
"##;
