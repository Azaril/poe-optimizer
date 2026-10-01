//! Full-source physical input observations, not native item or calculation closure.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "fresh_saved_armour_physical_inputs_distinguish_authored_state_and_requirements";
const CHILD: &str = "POE_SIMPLE_ITEM_PHYSICAL_INPUTS_CHILD";
const XML_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";

#[test]
fn fresh_saved_armour_physical_inputs_distinguish_authored_state_and_requirements() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-simple-item-physical-inputs-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
        let xml = fs::read_to_string(&path).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(xml.as_bytes())), XML_HASH);
        let before = |lua: &Lua| {
            lua.globals().set("physicalItemXml", xml.as_str())?;
            lua.globals().set("physicalItemJit", enabled)?;
            lua.load("if physicalItemJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let temp = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
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
        assert_eq!(observed["configuration_method_wrappers"], false);
        assert_eq!(observed["original_build_output_available"], true);
        assert_eq!(fs::read_to_string(path).unwrap(), xml);
        let result = json!({
            "source_hash":observed["source_hash"],"manifest_sha256":pinned::manifest_sha256(),
            "original_xml_sha256":XML_HASH,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/Bases/body.lua","src/Data/Bases/gloves.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
            "native_input_closure":false,"native_owner_coverage":false,
            "state":observed["additional_observation"],
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
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-simple-item-physical-inputs-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let state = &result["state"];
    for field in [
        "saved_items_preserved",
        "saved_selections_preserved",
        "main_output_preserved",
        "original_functions_preserved",
    ] {
        assert_eq!(state[field], true, "{field}");
    }
    assert_eq!(state["selected_items"], 2);
    assert_eq!(state["selected_spec"], 3);
    let items = rows(&state["items"]);
    assert_eq!(items.len(), 2);
    for (item, (id, base, requirement, capacity, life, slot)) in items.iter().zip([
        (19, "Tattered Robe", 0, 4, 17, "Body Armour"),
        (20, "Rope Cuffs", 5, 3, 16, "Gloves"),
    ]) {
        assert_eq!(item["id"], id);
        assert_eq!(item["selected_slots"], json!([slot]));
        let loaded = &item["loaded"];
        assert_eq!(loaded["base"], base);
        assert_eq!(loaded["rarity"], "RARE");
        assert_eq!(loaded["crafted"], true);
        assert_eq!(loaded["field_types"]["corrupted"], "nil");
        assert_eq!(loaded["item_level"], Json::Null);
        assert_eq!(loaded["quality"], 20);
        assert_eq!(
            loaded["requirement_headers"],
            json!([format!("LevelReq: {requirement}")])
        );
        assert_eq!(loaded["requirements"]["level"], requirement);
        assert_eq!(loaded["socket_capacity"], capacity);
        assert_eq!(loaded["jewel_capacity"], 0);
        assert_eq!(loaded["installed_augments"], 0);
        assert_eq!(loaded["rune_rows"], capacity);
        assert_eq!(loaded["active_life"], json!([life]));
        assert_eq!(item["fresh"], *loaded);
        let probes = rows(&item["probes"]);
        assert_eq!(probes.len(), 23);
        let p = |name| &probes.iter().find(|p| p["name"] == name).unwrap()["snapshot"];
        for (name, rarity) in [
            ("rarity-magic", "MAGIC"),
            ("rarity-normal", "NORMAL"),
            ("rarity-unique", "UNIQUE"),
            ("rarity-relic", "RELIC"),
            ("rarity-unknown", "UNIQUE"),
        ] {
            assert_eq!(p(name)["rarity"], rarity, "{base}/{name}");
            assert_eq!(p(name)["base"], base);
        }
        // These controls are explicitly excluded source shapes, not permission
        // to map unknown/missing source tokens to the fallback UNIQUE option.
        assert_eq!(p("rarity-absent")["rarity"], "UNIQUE");
        for (name, expected) in [
            ("level-zero", 0),
            ("level-high", 77),
            ("level-alias", 77),
            ("level-duplicate-high-last", 77),
            ("level-duplicate-zero-last", 0),
        ] {
            assert_eq!(p(name)["requirements"]["level"], expected, "{base}/{name}");
        }
        for name in ["level-absent", "level-imported", "level-malformed-last"] {
            assert_eq!(
                p(name)["requirements"]["level"],
                requirement,
                "{base}/{name}"
            );
        }
        assert!(rows(&p("level-absent")["requirement_headers"]).is_empty());
        assert_eq!(
            p("level-alias")["requirement_headers"],
            json!(["Requires Level: 77"])
        );
        assert_eq!(
            p("level-imported")["requirement_headers"],
            json!(["Level: 77"])
        );
        assert_eq!(p("corrupted")["corrupted"], true);
        assert_eq!(p("twice-corrupted")["corrupted"], true);
        assert_eq!(p("twice-corrupted")["double_corrupted"], true);
        assert_eq!(p("mirrored")["mirrored"], true);
        assert_eq!(p("sanctified")["sanctified"], true);
        assert_eq!(p("crafted-false")["crafted"], true);
        assert_eq!(p("crafted-absent")["crafted"], Json::Null);
        assert_eq!(p("socket-one-fewer")["socket_capacity"], capacity - 1);
        assert_eq!(p("socket-one-fewer")["rune_rows"], capacity - 1);
        assert_eq!(p("socket-one-fewer")["installed_augments"], 0);
        assert_eq!(p("socket-zero")["socket_capacity"], 0);
        assert_eq!(p("socket-zero")["installed_augments"], 0);
        assert_eq!(p("socket-zero-imported-level")["requirements"]["level"], 77);
        for probe in probes {
            assert_eq!(
                probe["snapshot"]["active_life"],
                json!([life]),
                "{base}/{}",
                probe["name"]
            );
        }
        assert_eq!(item["reparsed_corrupted"]["corrupted"], true);
        assert_eq!(
            item["reparsed_requirement"]["requirements"]["level"],
            requirement
        );
        assert_eq!(item["fresh_after_reparse"], *loaded);
    }
}

const OBSERVE: &str = r##"
local function original(f,path,first,last)local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last)end;return f end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function snapshot(item,list)
 local headers={};for _,line in ipairs(item.rawLines)do if line:match("^LevelReq:")or line:match("^Requires Level:")or line:match("^Level:")then headers[#headers+1]=line end end
 local req={};for k,v in pairs(item.requirements)do assert(type(v)=="number");req[k]=v end
 local installed=0;for _,rune in ipairs(item.runes)do if rune~="None"then installed=installed+1 end end
 local values={};for _,mod in ipairs(list or {})do if mod.name=="Life"then assert(mod.type=="BASE");values[#values+1]=mod.value end end
 local counts={};for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"})do counts[category]=#item[category.."ModLines"]end
 return{base=item.baseName,rarity=item.rarity,crafted=item.crafted,corrupted=item.corrupted,double_corrupted=item.doubleCorrupted,mirrored=item.mirrored,sanctified=item.sanctified,item_level=item.itemLevel,quality=item.quality,requirements=req,requirement_headers=headers,socket_capacity=item.itemSocketCount,jewel_capacity=item.jewelSocketCount,rune_rows=#item.runes,installed_augments=installed,active_life=values,list_counts=counts,field_types={corrupted=type(item.corrupted),crafted=type(item.crafted),item_level=type(item.itemLevel)}}
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);mods(item);return item end
local function replace(text,old,new)local pos=assert(text:find(old,1,true));assert(not text:find(old,pos+#old,true));return text:sub(1,pos-1)..new..text:sub(pos+#old)end
local doc,err=common.xml.ParseXML(physicalItemXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local texts={};for _,s in ipairs(v)do if type(s)=="string"then texts[#texts+1]=s end end;rawItems[assert(tonumber(v.attrib.id))]=table.concat(texts,"\n")end end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,items={}};local originals,snapshots={},{}
for _,id in ipairs({19,20})do
 local item=assert(build.itemsTab.items[id]);originals[id]=item;local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local raw=assert(rawItems[id]);local req="LevelReq: "..(id==19 and "0"or"5");local sockets="Sockets:"..string.rep(" S",id==19 and 4 or 3)
 local fresh=construct(raw,id);local row={id=id,raw=raw,loaded=loaded,fresh=snapshot(fresh,active(fresh,1,false)),selected_slots={},probes={}}
 for slot,v in pairs(env.player.itemList)do if v==item then row.selected_slots[#row.selected_slots+1]=slot end end;table.sort(row.selected_slots)
 local zero=replace(raw,sockets,"Sockets:"):gsub("Rune: None[\r\n]+","")
 local fewer=replace(raw,sockets,"Sockets:"..string.rep(" S",id==19 and 3 or 2)):gsub("Rune: None[\r\n]+","",1)
 local cases={
 {"rarity-magic",replace(replace(raw,"Rarity: RARE","Rarity: MAGIC"),"New Item\n","")},{"rarity-normal",replace(replace(raw,"Rarity: RARE","Rarity: NORMAL"),"New Item\n","")},{"rarity-unique",replace(raw,"Rarity: RARE","Rarity: UNIQUE")},{"rarity-relic",replace(raw,"Rarity: RARE","Rarity: RELIC")},{"rarity-unknown",replace(raw,"Rarity: RARE","Rarity: OwnedWitnessUnknown")},{"rarity-absent",replace(raw,"Rarity: RARE\n","")},
 {"level-zero",replace(raw,req,"LevelReq: 0")},{"level-high",replace(raw,req,"LevelReq: 77")},{"level-alias",replace(raw,req,"Requires Level: 77")},{"level-imported",replace(raw,req,"Level: 77")},{"level-absent",replace(raw,req.."\n","")},{"level-duplicate-high-last",replace(raw,req,"LevelReq: 0\nLevelReq: 77")},{"level-duplicate-zero-last",replace(raw,req,"LevelReq: 77\nLevelReq: 0")},{"level-malformed-last",replace(raw,req,"LevelReq: 77\nLevelReq: malformed")},
 {"corrupted",raw.."\nCorrupted"},{"twice-corrupted",raw.."\nTwice Corrupted"},{"mirrored",raw.."\nMirrored"},{"sanctified",raw.."\nSanctified"},
 {"crafted-false",replace(raw,"Crafted: true","Crafted: false")},{"crafted-absent",replace(raw,"Crafted: true\n","")},
 {"socket-one-fewer",fewer},{"socket-zero",zero},{"socket-zero-imported-level",replace(zero,req,"Level: 77")}}
 for _,case in ipairs(cases)do local probe=construct(case[2],9001);local after=snapshot(probe,active(probe,1,false));assert(equal(after,snapshot(probe,active(probe,1,false))));row.probes[#row.probes+1]={name=case[1],raw=case[2],snapshot=after}end
 local reused=construct(raw.."\nCorrupted",9002);parse(reused,raw);mods(reused);row.reparsed_corrupted=snapshot(reused,active(reused,1,false))
 local reqReused=construct(replace(raw,req,"LevelReq: 77"),9003);parse(reqReused,replace(raw,req.."\n",""));mods(reqReused);row.reparsed_requirement=snapshot(reqReused,active(reqReused,1,false))
 local again=construct(raw,9004);row.fresh_after_reparse=snapshot(again,active(again,1,false));result.items[#result.items+1]=row
end
for _,id in ipairs({19,20})do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load);result.original_functions_preserved=true
return result
"##;
