//! Exact fresh-item absence evidence, not authority for incomplete source layouts.
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

const TEST: &str = "selected_sniper_item_levels_remain_absent_after_fresh_complete_loads";
const CHILD: &str = "POE_ITEM_LEVEL_ABSENCE_CHILD";

#[test]
fn selected_sniper_item_levels_remain_absent_after_fresh_complete_loads() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-item-level-absence-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let manifest = read(&fixtures.join("index.json"));
        let row = manifest["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["xml"] == "build-05.xml")
            .unwrap();
        assert_eq!(row["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([41; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        for (id, ordinal) in [("19", 572), ("20", 574)] {
            let rows: Vec<_> = evidence
                .rows()
                .iter()
                .filter(|row| {
                    row.occurrence().name() == "Item"
                        && row.attribute("id").and_then(|a| a.decoded().ok()) == Some(id)
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].occurrence().id().ordinal(), ordinal);
        }
        let with_headers = header(&header(&xml, 19, 37), 20, 38);
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text, headers) in [
            ("original", &xml, false),
            ("explicit-headers", &with_headers, true),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("itemLevelXml", text.as_str())?;
                lua.globals().set("itemLevelHeaders", headers)?;
                lua.globals().set("itemLevelJit", enabled)?;
                lua.load("if itemLevelJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                text,
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
            cases.push(json!({"name":name,"xml_sha256":digest(text.as_bytes()),"state":result["additional_observation"]}));
        }
        assert_eq!(
            fs::read_to_string(fixtures.join("build-05.xml")).unwrap(),
            xml
        );
        let result = json!({"source_hash":source_hash,"evidence":{
            "fixture":"build-05.xml","xml_sha256":digest(xml.as_bytes()),
            "manifest_sha256":pinned::manifest_sha256(),"native_parity":false,"source_layout_authority":false,
            "items":[{"item_id":19,"source_ordinal":572,"base":"Tattered Robe"},{"item_id":20,"source_ordinal":574,"base":"Rope Cuffs"}],
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
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
            if started.elapsed() > Duration::from_secs(180) {
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
fn header(xml: &str, id: u32, level: u32) -> String {
    let opening = format!("<Item id=\"{id}\">");
    assert_eq!(xml.matches(&opening).count(), 1);
    let begin = xml.find(&opening).unwrap() + opening.len();
    let end = begin + xml[begin..].find("</Item>").unwrap();
    let raw = &xml[begin..end];
    assert!(!raw.contains("Item Level:"));
    assert_eq!(raw.matches("Crafted: true").count(), 1);
    let position = begin + raw.find("Crafted: true").unwrap();
    let mut changed = xml.to_owned();
    changed.insert_str(position, &format!("Item Level: {level}\n"));
    changed
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-item-level-absence-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let cases = result["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    for (case, headers) in cases.iter().zip([false, true]) {
        let state = &case["state"];
        assert_eq!(state["selected_items"], 2);
        assert_eq!(state["selected_spec"], 3);
        assert_eq!(state["selected_skills"], 4);
        assert_eq!(state["selected_config"], 1);
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[flag], true);
        }
        let items = state["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        for (row, (id, base, required, level)) in items
            .iter()
            .zip([(19, "Tattered Robe", 0, 37), (20, "Rope Cuffs", 5, 38)])
        {
            assert_eq!(row["id"], id);
            assert_eq!(row["loaded"]["base"], base);
            assert_eq!(row["raw_required_level"], required);
            assert_eq!(
                row["loaded"]["item_level_type"],
                if headers { "number" } else { "nil" }
            );
            if headers {
                assert_eq!(row["loaded"]["item_level"], level);
            } else {
                assert_eq!(row["fresh"]["item_level_type"], "nil");
                assert_eq!(row["header_present"]["item_level"], level);
                assert_eq!(row["reused_after_header_removal"]["item_level"], level);
                assert_eq!(row["fresh_after_header_removal"]["item_level_type"], "nil");
                assert_eq!(row["unknown_layout"]["item_level_type"], "nil");
                assert!(row["unknown_layout"]["unparsed_lines"].as_u64().unwrap() > 0);
            }
        }
    }
}
const OBSERVE: &str = r#"
local function original(f,path,first,last)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first and i.lastlinedefined==last);return f
end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local doc,err=common.xml.ParseXML(itemLevelXml);assert(doc and not err)
local itemsNode;for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Items" then assert(not itemsNode);itemsNode=n end end;assert(itemsNode)
local rawItems={};local selected
for _,n in ipairs(itemsNode) do if type(n)=="table" then
 if n.elem=="Item" then
  local text={};for _,s in ipairs(n) do if type(s)=="string" then text[#text+1]=s end end
  local id=assert(tonumber(n.attrib.id));assert(not rawItems[id]);rawItems[id]=table.concat(text,"\n")
 elseif n.elem=="ItemSet" and tonumber(n.attrib.id)==2 then assert(not selected);selected=n end
end end
assert(selected)
local selectedIds={};for _,slot in ipairs(selected) do if type(slot)=="table" and slot.elem=="Slot" then selectedIds[tonumber(slot.attrib.itemId)]=true end end
assert(selectedIds[19] and selectedIds[20])
local function snapshot(item)
 local unparsed=0;for _,category in ipairs({"implicit","explicit","enchant","rune"}) do
  for _,line in ipairs(item[category.."ModLines"]) do if line.extra then unparsed=unparsed+1 end end
 end
 return {base=item.baseName,raw=item.raw,item_level_type=type(item.itemLevel),item_level=item.itemLevel,
  required_level=item.requirements.level,unparsed_lines=unparsed}
end
local function equal(a,b)
 for k,v in pairs(a) do if b[k]~=v then return false end end
 for k,v in pairs(b) do if a[k]~=v then return false end end;return true
end
local function construct(raw)
 local item=new("Item"):Item("");parse(item,raw);mods(item);return item
end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={}}
local originals,snapshots={},{}
for _,id in ipairs({19,20}) do
 local item=assert(build.itemsTab.items[id]);local raw=assert(rawItems[id]);originals[id]=item;snapshots[id]=snapshot(item)
 local row={id=id,raw=raw,raw_required_level=assert(tonumber(raw:match("LevelReq: (%d+)"))),loaded=snapshots[id]}
 if not itemLevelHeaders then
  assert(not raw:find("Item Level:",1,true))
  local fresh=construct(raw);assert(fresh~=item);row.fresh=snapshot(fresh)
  local at=assert(raw:find("Crafted: true",1,true));local level=id+18
  local withHeader=raw:sub(1,at-1).."Item Level: "..level.."\n"..raw:sub(at)
  local reused=construct(withHeader);row.header_present=snapshot(reused)
  parse(reused,raw);mods(reused);row.reused_after_header_removal=snapshot(reused)
  local reset=construct(raw);assert(reset~=reused and reset~=fresh);row.fresh_after_header_removal=snapshot(reset)
  local unknown=construct(raw.."\nOwned witness unknown semantic line");row.unknown_layout=snapshot(unknown)
 end
 result.items[#result.items+1]=row
end
for _,id in ipairs({19,20}) do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id]),snapshots[id])) end
result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and build.itemsTab.Load==load);result.original_functions_preserved=true
return result
"#;
