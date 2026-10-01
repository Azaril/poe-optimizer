//! Actual original05 equipment membership witness; source evidence, not closure authority.
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
const TEST: &str = "selected_original05_items_have_source_proven_empty_augments";
const CHILD: &str = "POE_EMPTY_EQUIPMENT_AUGMENTS_CHILD";
#[test]
fn selected_original05_items_have_source_proven_empty_augments() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-empty-equipment-augments-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let index = read(&fixtures.join("index.json"));
        let row = index["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["xml"] == "build-05.xml")
            .unwrap();
        let hash = format!("{:x}", Sha256::digest(xml.as_bytes()));
        assert_eq!(row["xml_sha256"], hash);
        let before = |lua: &Lua| {
            lua.globals()
                .set("emptyEquipmentFixtureXml", xml.as_str())?;
            lua.globals().set("emptyEquipmentJit", enabled)?;
            lua.load("if emptyEquipmentJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let temp = tempfile::tempdir().unwrap();
        let mut result = source::observe_with_build_hook_unwrapped(
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
        // Exercise the complete original loader, including its post-ParseRaw
        // fallback. The immutable fixture is changed only at two rare titles.
        let fallback_xml = replace_item_title(&xml, "19", "Tabula Rasa");
        let fallback_xml = replace_item_title(&fallback_xml, "23", "Sekhema's Resolve");
        let fallback_hash = format!("{:x}", Sha256::digest(fallback_xml.as_bytes()));
        let before_fallback = |lua: &Lua| {
            lua.globals()
                .set("emptyEquipmentFixtureXml", fallback_xml.as_str())?;
            lua.globals().set("emptyEquipmentJit", enabled)?;
            lua.load("if emptyEquipmentJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let fallback_temp = tempfile::tempdir().unwrap();
        let fallback = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            fallback_temp.path(),
            &fallback_xml,
            None,
            false,
            Some(&before_fallback),
            None,
            Some(&observe_loader_fallback),
        )
        .unwrap();
        assert_eq!(result["source_hash"], fallback["source_hash"]);
        assert_eq!(
            fs::read_to_string(fixtures.join("build-05.xml")).unwrap(),
            xml
        );
        result["additional_observation"]["loader_fallback"] =
            fallback["additional_observation"].clone();
        result["evidence"] = json!({"scope":"original05_selected_items_empty_augments","fixture":"build-05.xml","xml_sha256":hash,"manifest_sha256":pinned::manifest_sha256(),"native_parity":false,"membership_closure_granted":false,"files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))});
        result["evidence"]["loader_fallback_xml_sha256"] = json!(fallback_hash);
        result["evidence"]["loader_fallback_mutations"] = json!([{"item_id":19,"title":"Tabula Rasa"},{"item_id":23,"title":"Sekhema's Resolve"}]);
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        check(&result["additional_observation"]);
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
            if started.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = read(&out.join("source-jit-off.json"));
    let on = read(&out.join("source-jit-on.json"));
    for field in ["source_hash", "evidence", "additional_observation"] {
        assert_eq!(off[field], on[field]);
    }
}
fn read(p: &Path) -> Json {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn replace_item_title(xml: &str, id: &str, title: &str) -> String {
    let opening = format!("<Item id=\"{id}\">");
    assert_eq!(xml.matches(&opening).count(), 1);
    let begin = xml.find(&opening).unwrap() + opening.len();
    let end = begin + xml[begin..].find("</Item>").unwrap();
    let raw = &xml[begin..end];
    assert_eq!(raw.matches("New Item").count(), 1);
    assert_eq!(raw.lines().nth(1).unwrap().trim(), "Rarity: RARE");
    assert_eq!(raw.lines().nth(2).unwrap(), "New Item");
    let mut changed = xml.to_owned();
    let offset = begin + raw.find("New Item").unwrap();
    changed.replace_range(offset..offset + "New Item".len(), title);
    changed
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let v: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-empty-equipment-augments-observation")
        .eval()?;
    Ok(lua.from_value(v)?)
}
fn observe_loader_fallback(lua: &Lua) -> Result<Json, RuntimeError> {
    let v: Value = lua
        .load(LOADER_FALLBACK_OBSERVATION)
        .set_name("@owned-equipment-loader-fallback-observation")
        .eval()?;
    Ok(lua.from_value(v)?)
}
fn rows(v: &Json) -> &[Json] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn check(r: &Json) {
    assert_eq!(r["selected_set"], 2);
    assert_eq!(r["slot_rows"], 22);
    assert_eq!(r["explicit_empty_slots"], 13);
    assert_eq!(r["url_rows"], 1);
    assert_eq!(r["rune_slot_rows"], 0);
    assert_eq!(r["raw_none_count"], 17);
    let uses = rows(&r["uses"]);
    assert_eq!(uses.len(), 9);
    let actual: std::collections::BTreeMap<_, _> = uses
        .iter()
        .map(|v| (v["slot"].as_str().unwrap(), v["item_id"].as_u64().unwrap()))
        .collect();
    assert_eq!(
        actual,
        std::collections::BTreeMap::from([
            ("Amulet", 23),
            ("Gloves", 20),
            ("Boots", 22),
            ("Ring 1", 26),
            ("Belt", 27),
            ("Ring 2", 26),
            ("Body Armour", 19),
            ("Helmet", 21),
            ("Weapon 1", 28)
        ])
    );
    let items = rows(&r["items"]);
    assert_eq!(items.len(), 8);
    let mut sockets = 0;
    for item in items {
        assert_eq!(item["base_known"], true);
        assert_eq!(
            item["field_types"],
            json!({"base":"table","item_socket_count":"number","jewel_socket_count":"number","sockets":"table","runes":"table","rune_mod_lines":"table"})
        );
        assert_eq!(item["jewel_sockets"], 0);
        assert_eq!(item["nonempty_runes"], 0);
        assert_eq!(item["rune_mod_lines"], 0);
        assert_eq!(item["reparsed_equal"], true);
        let n = item["item_sockets"].as_u64().unwrap();
        sockets += n;
        assert_eq!(item["raw_none_count"], n);
        assert_eq!(rows(&item["runes"]).len() as u64, n);
        for rune in rows(&item["runes"]) {
            assert_eq!(rune, "None");
        }
    }
    assert_eq!(sockets, 17);
    let p = &r["probes"];
    assert_eq!(p["occupied"]["nonempty_runes"], 1);
    assert!(p["occupied"]["rune_mod_lines"].as_u64().unwrap() > 0);
    assert_eq!(p["unknown_rune"]["nonempty_runes"], 1);
    assert_eq!(p["unknown_rune"]["known_nonempty_runes"], 0);
    assert_eq!(p["removed_declarations"]["raw_none_count"], 0);
    assert_eq!(p["removed_declarations"]["item_sockets"], 4);
    assert!(
        p["legacy_reconstruction"]["nonempty_runes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        p["legacy_reconstruction"]["rune_mod_lines"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(p["unknown_base"]["base_known"], false);
    let fallback = &r["loader_fallback"];
    let observed = rows(&fallback["items"]);
    assert_eq!(observed.len(), 2);
    for (row, id, title, base, sockets, jewels) in [
        (&observed[0], 19, "Tabula Rasa", "Tattered Robe", 4, 6),
        (&observed[1], 23, "Sekhema's Resolve", "Solar Amulet", 0, 1),
    ] {
        assert_eq!(row["id"], id);
        assert_eq!(row["title"], title);
        assert_eq!(row["rarity"], "RARE");
        assert_eq!(row["base_name"], base);
        assert_eq!(row["parsed_jewel_sockets"], 0);
        assert_eq!(row["loaded_jewel_sockets"], jewels);
        assert_eq!(row["item_sockets"], sockets);
        assert_eq!(row["raw_none_count"], sockets);
        assert_eq!(row["rune_count"], sockets);
        assert_eq!(
            row["explicit_socket_headers"],
            if sockets == 0 { 0 } else { 1 }
        );
    }
    assert_eq!(fallback["loaded_state_preserved"], true);
    assert_eq!(fallback["original_functions_preserved"], true);
    for key in [
        "saved_selections_preserved",
        "saved_items_preserved",
        "main_output_preserved",
        "original_functions_preserved",
    ] {
        assert_eq!(r[key], true);
    }
}
const OBSERVATION: &str = r#"
local function original(f,path,first,last)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==first)
 if last then assert(info.lastlinedefined==last) end
 return f
end
local itemClass=common.classes.Item
local parse=original(itemClass.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(itemClass.BuildModList,"Classes/Item.lua",2694,2863)
local update=original(itemClass.UpdateRunes,"Classes/Item.lua",2106)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local create=original(build.itemsTab.CreateItemSet,"Classes/ItemsTab.lua",1571)
local doc,err=common.xml.ParseXML(emptyEquipmentFixtureXml);assert(doc and not err)
local itemsNode;for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Items" then assert(not itemsNode);itemsNode=n end end
assert(itemsNode and tonumber(itemsNode.attrib.activeItemSet)==2)
local rawItems={};local selected
for _,n in ipairs(itemsNode) do
 if type(n)=="table" and n.elem=="Item" then
  local id=assert(tonumber(n.attrib.id));assert(not rawItems[id]);local text={}
  for _,child in ipairs(n) do if type(child)=="string" then text[#text+1]=child end end
  rawItems[id]=table.concat(text,"\n")
 elseif type(n)=="table" and n.elem=="ItemSet" and tonumber(n.attrib.id)==2 then assert(not selected);selected=n end
end
assert(selected and build.itemsTab.activeItemSetId==2)
local function equal(a,b)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end
 return true
end
local set=build.itemsTab.itemSets[2];local savedSet=copyTable(set)
local selectedSkills=build.skillsTab.activeSkillSetId;local selectedConfig=build.configTab.activeConfigSetId
local selectedSpec=build.treeTab.activeSpec;local selectedGroup=build.mainSocketGroup
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local function snapshot(item,raw)
 local runes={};local occupied,known=0,0
 for _,rune in ipairs(item.runes or {}) do runes[#runes+1]=rune;if rune~="None" then occupied=occupied+1;if data.itemMods.Runes[rune] then known=known+1 end end end
 local none=0;for line in raw:gmatch("[^\r\n]+") do if line:match("^%s*Rune: None%s*$") then none=none+1 end end
 local lines={};for _,line in ipairs(item.runeModLines or {}) do lines[#lines+1]={line=line.line,augment_type=line.augmentType,rune=line.rune,enchant=line.enchant} end
 return {field_types={base=type(item.base),item_socket_count=type(item.itemSocketCount),jewel_socket_count=type(item.jewelSocketCount),sockets=type(item.sockets),runes=type(item.runes),rune_mod_lines=type(item.runeModLines)},base_known=item.base~=nil,base_name=item.baseName,item_type=item.type,item_sockets=item.itemSocketCount or 0,jewel_sockets=item.jewelSocketCount or 0,socket_records=#(item.sockets or {}),runes=runes,nonempty_runes=occupied,known_nonempty_runes=known,rune_mod_lines=#lines,mod_lines=lines,raw_none_count=none}
end
local function constructed(raw)
 local item=new("Item"):Item("");parse(item,raw);mods(item);return item
end
local result={selected_set=2,slot_rows=0,explicit_empty_slots=0,url_rows=0,rune_slot_rows=0,raw_none_count=0,uses={},items={},probes={}}
local ids={};local originals={};local snapshots={}
for _,n in ipairs(selected) do
 assert(type(n)=="table" and #n==0)
 if n.elem=="Slot" then
  result.slot_rows=result.slot_rows+1;local id=assert(tonumber(n.attrib.itemId));local name=assert(n.attrib.name);assert(set[name] and set[name].selItemId==id)
  if id==0 then result.explicit_empty_slots=result.explicit_empty_slots+1 else
   result.uses[#result.uses+1]={slot=name,item_id=id};ids[id]=true
  end
 elseif n.elem=="SocketIdURL" then
  result.url_rows=result.url_rows+1;assert(n.attrib.nodeId=="7960" and n.attrib.itemPbURL=="")
  assert(set[7960] and set[7960].pbURL=="" and set[7960].selItemId==nil)
 elseif n.elem=="RuneSlot" then result.rune_slot_rows=result.rune_slot_rows+1
 else error("unclassified ItemSet child") end
end
local order={};for id in pairs(ids) do order[#order+1]=id end;table.sort(order)
for _,id in ipairs(order) do
 local item=assert(build.itemsTab.items[id]);local raw=assert(rawItems[id]);local observed=snapshot(item,raw)
 local again=snapshot(constructed(raw),raw);assert(equal(observed,again))
 originals[id]=item;snapshots[id]=copyTable(observed);observed.item_id=id;observed.reparsed_equal=true
 result.items[#result.items+1]=observed;result.raw_none_count=result.raw_none_count+observed.raw_none_count
end
-- Isolated item objects execute the same complete ParseRaw/BuildModList methods.
-- The saved item records, selected set and calculation environment are untouched.
local raw=assert(rawItems[19]);local sample=constructed(raw);local broad,specific=sample:GetSocketedAugmentTypes()
local names={};for name,rune in pairs(data.itemMods.Runes) do if (rune[broad] and #rune[broad]>0) or (rune[specific] and #rune[specific]>0) then names[#names+1]=name end end;table.sort(names);local occupiedName=assert(names[1])
local occupiedRaw,replacements=raw:gsub("Rune: None","Rune: "..occupiedName,1);assert(replacements==1)
local occupied=constructed(occupiedRaw);result.probes.occupied=snapshot(occupied,occupiedRaw);result.probes.occupied.rune_name=occupiedName
local unknownRaw=raw:gsub("Rune: None","Rune: OwnedObserverUnknownRune",1);result.probes.unknown_rune=snapshot(constructed(unknownRaw),unknownRaw)
local removed=raw:gsub("Rune: None[\r\n]+","");result.probes.removed_declarations=snapshot(constructed(removed),removed)
local line;for _,row in ipairs(occupied.runeModLines) do if not row.bonded then line=row.line;break end end;assert(line);local legacy=removed.."\n{rune}"..line;result.probes.legacy_reconstruction=snapshot(constructed(legacy),legacy)
local unknownBase,n=raw:gsub("Tattered Robe","OwnedObserverUnknownBase",1);assert(n==1);result.probes.unknown_base=snapshot(constructed(unknownBase),unknownBase)
for _,id in ipairs(order) do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],rawItems[id]),snapshots[id])) end
result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==2 and build.itemsTab.itemSets[2]==set and equal(savedSet,set))
assert(build.skillsTab.activeSkillSetId==selectedSkills and build.configTab.activeConfigSetId==selectedConfig and build.treeTab.activeSpec==selectedSpec and build.mainSocketGroup==selectedGroup)
result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end;result.main_output_preserved=true
assert(itemClass.ParseRaw==parse and itemClass.BuildModList==mods and itemClass.UpdateRunes==update and build.itemsTab.Load==load and build.itemsTab.CreateItemSet==create);result.original_functions_preserved=true
return result
"#;

const LOADER_FALLBACK_OBSERVATION: &str = r#"
local function original(f,path,first,last)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==first and info.lastlinedefined==last)
 return f
end
local itemClass=common.classes.Item
local parse=original(itemClass.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(itemClass.BuildModList,"Classes/Item.lua",2694,2863)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local doc,err=common.xml.ParseXML(emptyEquipmentFixtureXml);assert(doc and not err)
local itemsNode;for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Items" then assert(not itemsNode);itemsNode=n end end
assert(itemsNode and build.itemsTab.activeItemSetId==2)
local rawItems={}
for _,n in ipairs(itemsNode) do
 if type(n)=="table" and n.elem=="Item" then
  local id=assert(tonumber(n.attrib.id));assert(not rawItems[id]);local text={}
  for _,child in ipairs(n) do if type(child)=="string" then text[#text+1]=child end end
  rawItems[id]=table.concat(text,"\n")
 end
end
local tab=build.itemsTab;local itemTable=tab.items;local itemOrder=tab.itemOrderList
local selectedSet=tab.activeItemSet;local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local selectedSkills=build.skillsTab.activeSkillSetId;local selectedConfig=build.configTab.activeConfigSetId
local selectedSpec=build.treeTab.activeSpec;local selectedGroup=build.mainSocketGroup
local saved={};for id,item in pairs(itemTable) do saved[id]={item=item,raw=item.raw,title=item.title,jewels=item.jewelSocketCount} end
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local result={items={}}
for _,id in ipairs({19,23}) do
 local loaded=assert(itemTable[id]);local raw=assert(rawItems[id])
 -- Only the controlled full bootstrap's unchanged ItemsTab.Load applied its
 -- title fallback. A complete fresh Item parse is the contrasting lower layer.
 local parsed=new("Item"):Item("");parse(parsed,raw);mods(parsed)
 assert(type(loaded.jewelSocketCount)=="number" and type(parsed.jewelSocketCount)=="number")
 assert(type(loaded.itemSocketCount)=="number" and type(loaded.runes)=="table")
 assert(loaded.title==parsed.title and loaded.rarity==parsed.rarity and loaded.baseName==parsed.baseName)
 assert(loaded.itemSocketCount==parsed.itemSocketCount and #loaded.runes==#parsed.runes)
 for i,rune in ipairs(loaded.runes) do assert(rune=="None" and parsed.runes[i]==rune) end
 local none,headers=0,0
 for line in raw:gmatch("[^\r\n]+") do
  if line:match("^%s*Rune: None%s*$") then none=none+1 end
  if line:match("^%s*Sockets:") then headers=headers+1 end
 end
 result.items[#result.items+1]={id=id,title=loaded.title,rarity=loaded.rarity,base_name=loaded.baseName,item_sockets=loaded.itemSocketCount,rune_count=#loaded.runes,raw_none_count=none,explicit_socket_headers=headers,parsed_jewel_sockets=parsed.jewelSocketCount,loaded_jewel_sockets=loaded.jewelSocketCount}
end
assert(build.itemsTab==tab and tab.items==itemTable and tab.itemOrderList==itemOrder and tab.activeItemSet==selectedSet and tab.activeItemSetId==2)
for id,row in pairs(saved) do local item=assert(itemTable[id]);assert(item==row.item and item.raw==row.raw and item.title==row.title and item.jewelSocketCount==row.jewels) end
for id in pairs(itemTable) do assert(saved[id]) end
assert(build.skillsTab.activeSkillSetId==selectedSkills and build.configTab.activeConfigSetId==selectedConfig and build.treeTab.activeSpec==selectedSpec and build.mainSocketGroup==selectedGroup)
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end
result.loaded_state_preserved=true
assert(itemClass.ParseRaw==parse and itemClass.BuildModList==mods and build.itemsTab.Load==load)
result.original_functions_preserved=true
return result
"#;
