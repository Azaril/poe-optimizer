//! Optional full-source catalyst observations; no native input closure or crafting legality.
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
const TEST: &str = "fresh_saved_armour_catalyst_inputs_preserve_absence_zero_and_source_tags";
const CHILD: &str = "POE_SIMPLE_ITEM_CATALYSTS_CHILD";
const XML_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";

#[test]
fn fresh_saved_armour_catalyst_inputs_preserve_absence_zero_and_source_tags() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-simple-item-catalysts-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
        let xml = fs::read_to_string(&path).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(xml.as_bytes())), XML_HASH);
        let before = |lua: &Lua| {
            lua.globals().set("armourCatalystXml", xml.as_str())?;
            lua.globals().set("armourCatalystJit", enabled)?;
            lua.load("if armourCatalystJit then jit.on() else jit.off();jit.flush() end")
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
            "source_hash":observed["source_hash"],
            "manifest_sha256":pinned::manifest_sha256(),
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
        .set_name("@owned-simple-item-catalysts-observer")
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
    for (item, (id, base, life, slot)) in items.iter().zip([
        (19, "Tattered Robe", 17, "Body Armour"),
        (20, "Rope Cuffs", 16, "Gloves"),
    ]) {
        assert_eq!(item["id"], id);
        assert_eq!(item["selected_slots"], json!([slot]));
        let loaded = &item["loaded"];
        assert_eq!(loaded["base"], base);
        assert_eq!(loaded["field_types"]["catalyst"], "nil");
        assert_eq!(loaded["field_types"]["catalyst_quality"], "nil");
        assert_eq!(loaded["quality"], 20);
        assert_eq!(loaded["catalyst"], Json::Null);
        assert_eq!(loaded["catalyst_quality"], Json::Null);
        assert_eq!(loaded["helper_factor"], 1);
        assert_eq!(loaded["active_life"], json!([life]));
        assert!(rows(&loaded["mod_tags"]).is_empty());
        assert_eq!(item["fresh"], *loaded);
        assert_eq!(item["reparsed"]["catalyst"], "Flesh");
        assert_eq!(item["reparsed"]["catalyst_quality"], 37);
        assert_eq!(item["fresh_after_reparse"], *loaded);
        let probes = rows(&item["probes"]);
        assert_eq!(probes.len(), 15);
        let p = |name| &probes.iter().find(|p| p["name"] == name).unwrap()["snapshot"];
        for name in [
            "absent",
            "selected-missing-amount",
            "explicit-zero",
            "explicit-twenty",
            "amount-only",
            "unknown-kind",
            "malformed-amount",
            "duplicate-amount",
            "duplicate-kind",
        ] {
            assert!(rows(&p(name)["mod_tags"]).is_empty());
            assert_eq!(p(name)["helper_factor"], 1, "{base}/{name}");
            assert_eq!(p(name)["active_life"], json!([life]), "{base}/{name}");
        }
        assert_eq!(p("absent")["catalyst"], Json::Null);
        assert_eq!(p("absent")["catalyst_quality"], Json::Null);
        assert_eq!(p("selected-missing-amount")["catalyst"], "Flesh");
        assert_eq!(p("selected-missing-amount")["catalyst_quality"], Json::Null);
        assert_eq!(p("explicit-zero")["catalyst_quality"], 0);
        assert_eq!(p("explicit-twenty")["catalyst_quality"], 20);
        assert_eq!(p("amount-only")["catalyst"], Json::Null);
        assert_eq!(p("amount-only")["catalyst_quality"], 37);
        assert_eq!(p("unknown-kind")["catalyst"], Json::Null);
        assert_eq!(p("malformed-amount")["catalyst_quality"], Json::Null);
        assert_eq!(p("duplicate-amount")["catalyst_quality"], Json::Null);
        assert_eq!(p("duplicate-kind")["catalyst"], "Neural");
        for (name, factor, value, tag) in [
            ("matching-missing-amount", 1.2, 12, "life"),
            ("matching-explicit-twenty", 1.2, 12, "life"),
            ("matching-zero", 1.0, 10, "life"),
            ("matching-amount-only", 1.0, 10, "life"),
            ("wrong-tag", 1.0, 10, "mana"),
            ("wrong-kind", 1.0, 10, "life"),
        ] {
            assert_eq!(p(name)["mod_tags"], json!([tag]));
            assert_eq!(p(name)["helper_factor"], factor);
            assert_eq!(p(name)["active_life"], json!([value]));
            assert_eq!(p(name)["quality"], 20);
        }
        assert_eq!(p("matching-missing-amount")["catalyst_quality"], Json::Null);
        assert_eq!(p("matching-explicit-twenty")["catalyst_quality"], 20);
    }
}

const OBSERVE: &str = r##"
local function original(f,path,first,last)local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last)end;return f end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local names=upvalue(parse,"catalystList");assert(#names==13 and names[1]=="Flesh" and names[2]=="Neural")
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function snapshot(item,list)
 assert(#item.explicitModLines==1);local line=item.explicitModLines[1];assert(not line.extra and #line.modList==1)
 local m=line.modList[1];assert(m.name=="Life" and m.type=="BASE" and m.flags==0 and m.keywordFlags==0 and #m==0)
 local tags={};for _,tag in ipairs(line.modTags)do tags[#tags+1]=tag end
 local values={};for _,mod in ipairs(list or {})do if mod.name=="Life"then assert(mod.type=="BASE");values[#values+1]=mod.value end end
 return{base=item.baseName,catalyst=item.catalyst and names[item.catalyst],catalyst_quality=item.catalystQuality,quality=item.quality,field_types={catalyst=type(item.catalyst),catalyst_quality=type(item.catalystQuality)},mod_tags=tags,helper_factor=catalyst(item.catalyst,line,item.catalystQuality),canonical_life=m.value,active_life=values}
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);mods(item);return item end
local function replace(text,old,new)local pos=assert(text:find(old,1,true));assert(not text:find(old,pos+#old,true));return text:sub(1,pos-1)..new..text:sub(pos+#old)end
local function header(raw,value)if value==""then return raw end;return replace(raw,"Crafted: true",value.."\nCrafted: true")end
local doc,err=common.xml.ParseXML(armourCatalystXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local texts={};for _,s in ipairs(v)do if type(s)=="string"then texts[#texts+1]=s end end;rawItems[assert(tonumber(v.attrib.id))]=table.concat(texts,"\n")end end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,items={}};local originals,snapshots={},{}
for _,id in ipairs({19,20})do
 local item=assert(build.itemsTab.items[id]);originals[id]=item;local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local raw=assert(rawItems[id]);assert(not raw:find("Catalyst",1,true));local needle="+"..(id==19 and "17"or"16").." to maximum Life"
 local fresh=construct(raw,id);local row={id=id,raw=raw,loaded=loaded,fresh=snapshot(fresh,active(fresh,1,false)),selected_slots={},probes={}}
 for slot,v in pairs(env.player.itemList)do if v==item then row.selected_slots[#row.selected_slots+1]=slot end end;table.sort(row.selected_slots)
 local cases={
 {"absent",""},{"selected-missing-amount","Catalyst: Flesh"},{"explicit-zero","Catalyst: Flesh\nCatalystQuality: 0"},{"explicit-twenty","Catalyst: Flesh\nCatalystQuality: 20"},{"amount-only","CatalystQuality: 37"},
 {"unknown-kind","Catalyst: OwnedWitnessUnknown"},{"malformed-amount","Catalyst: Flesh\nCatalystQuality: bad"},{"duplicate-amount","Catalyst: Flesh\nCatalystQuality: 20\nCatalystQuality: bad"},{"duplicate-kind","Catalyst: Flesh\nCatalyst: Neural"},
 {"matching-missing-amount","Catalyst: Flesh","life"},{"matching-explicit-twenty","Catalyst: Flesh\nCatalystQuality: 20","life"},{"matching-zero","Catalyst: Flesh\nCatalystQuality: 0","life"},{"matching-amount-only","CatalystQuality: 37","life"},{"wrong-tag","Catalyst: Flesh\nCatalystQuality: 20","mana"},{"wrong-kind","Catalyst: Neural\nCatalystQuality: 20","life"}}
 for _,case in ipairs(cases)do local text=header(raw,case[2]);if case[3]then text=replace(text,needle,"{tags:"..case[3].."}+10 to maximum Life")end;local probe=construct(text,9001);local after=snapshot(probe,active(probe,1,false));assert(equal(after,snapshot(probe,active(probe,1,false))));row.probes[#row.probes+1]={name=case[1],raw=text,snapshot=after}end
 local reused=construct(header(raw,"Catalyst: Flesh\nCatalystQuality: 37"),9002);parse(reused,raw);mods(reused);row.reparsed=snapshot(reused,active(reused,1,false));local again=construct(raw,9003);row.fresh_after_reparse=snapshot(again,active(again,1,false))
 result.items[#result.items+1]=row
end
for _,id in ipairs({19,20})do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
return result
"##;
