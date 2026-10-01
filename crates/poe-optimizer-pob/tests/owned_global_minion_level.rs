//! Complete-item evidence for the original global minion-level occurrences.
//! Native arithmetic comparisons live with the authored numeric recipe; this
//! witness grants neither source admission nor whole-contributor coverage.
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
const TEST: &str = "original_minion_level_lines_keep_item_scaling_and_copy_occurrences_separate";
const CHILD: &str = "POE_GLOBAL_MINION_LEVEL_CHILD";

#[test]
fn original_minion_level_lines_keep_item_scaling_and_copy_occurrences_separate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-global-minion-level-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let manifest = read(&fixtures.join("index.json"));
        let mut builds = Vec::new();
        let mut source_hash = None;
        for (file, skill) in [
            ("build-01.xml", "SummonSkeletalClericsPlayer"),
            ("build-05.xml", "SummonSkeletalSnipersPlayer"),
        ] {
            let xml = fs::read_to_string(fixtures.join(file)).unwrap();
            let entry = manifest["builds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["xml"] == file)
                .unwrap();
            let hash = digest(xml.as_bytes());
            assert_eq!(entry["xml_sha256"], hash);
            let before = |lua: &Lua| {
                lua.globals().set("minionLevelXml", xml.as_str())?;
                lua.globals().set("minionLevelSkill", skill)?;
                lua.globals()
                    .set("minionLevelComponents", file == "build-05.xml")?;
                lua.globals().set("minionLevelJit", enabled)?;
                lua.load("if minionLevelJit then jit.on() else jit.off();jit.flush() end")
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
            if let Some(expected) = &source_hash {
                assert_eq!(&result["source_hash"], expected);
            } else {
                source_hash = Some(result["source_hash"].clone());
            }
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            assert_eq!(fs::read_to_string(fixtures.join(file)).unwrap(), xml);
            let state = result["additional_observation"].clone();
            // Existing immutable full-build delivery evidence is reused when
            // available; the current complete bootstrap independently observes
            // the same actual matching records without replacing Gem inputs.
            let prior = root
                .join("runs/owned-effective-gem-inputs-01")
                .join(file.trim_end_matches(".xml"))
                .join(format!(
                    "source-jit-{}.json",
                    if enabled { "on" } else { "off" }
                ));
            let prior_binding = if prior.exists() {
                let bytes = fs::read(&prior).unwrap();
                let old: Json = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(old["source_hash"], result["source_hash"]);
                assert_eq!(old["evidence"]["xml_sha256"], hash);
                assert_eq!(
                    state["external_properties"],
                    old["additional_observation"]["untouched_original"]["value"]["external_properties"]
                );
                Some(json!({"path":prior.to_string_lossy(),"sha256":digest(&bytes)}))
            } else {
                None
            };
            builds.push(json!({"fixture":file,"xml_sha256":hash,"state":state,"prior_delivery_evidence":prior_binding}));
        }
        let result = json!({"source_hash":source_hash,"evidence":{
            "manifest_sha256":pinned::manifest_sha256(),"native_parity":false,"whole_contributor_coverage":false,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Classes/ModStore.lua","src/Classes/ModList.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Modules/CalcSetup.lua","src/Data/ModScalability.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
        },"builds":builds});
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
    let off = read(&out.join("source-jit-off.json"));
    let on = read(&out.join("source-jit-on.json"));
    assert_eq!(off["source_hash"], on["source_hash"]);
    assert_eq!(off["evidence"], on["evidence"]);
    for (a, b) in rows(&off["builds"]).iter().zip(rows(&on["builds"])) {
        assert_eq!(a["fixture"], b["fixture"]);
        assert_eq!(a["xml_sha256"], b["xml_sha256"]);
        assert_eq!(a["state"], b["state"]);
    }
}
fn read(p: &Path) -> Json {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@global-minion-level-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let builds = rows(&result["builds"]);
    assert_eq!(builds.len(), 2);
    for (i, build) in builds.iter().enumerate() {
        let s = &build["state"];
        for key in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(s[key], true);
        }
        assert_eq!(s["selected_items"], if i == 0 { 1 } else { 2 });
        assert_eq!(s["selected_spec"], if i == 0 { 1 } else { 3 });
        assert_eq!(s["selected_skills"], if i == 0 { 1 } else { 4 });
        assert_eq!(s["selected_config"], 1);
        assert_eq!(rows(&s["items"]).len(), if i == 0 { 4 } else { 7 });
        let records = rows(&s["external_properties"]);
        assert_eq!(records.len(), if i == 0 { 5 } else { 3 });
        assert_eq!(
            records.iter().filter(|r| r["value"]["value"] == 0).count(),
            1
        );
        for r in records {
            assert_eq!(r["mod"]["name"], "GemProperty");
            assert_eq!(r["value"]["keyword"], "minion");
            assert_eq!(r["value"]["key"], "level");
            assert_eq!(r["mod"]["flags"], 0);
            assert_eq!(r["mod"]["keyword_flags"], 0);
            assert!(rows(&r["mod"]["tags"]).is_empty());
        }
        for item in rows(&s["items"]) {
            assert_eq!(item["reparsed_equal"], true);
            assert_eq!(rows(&item["snapshot"]["lines"]).len(), 1);
            assert_eq!(rows(&item["snapshot"]["active"]).len(), 1);
            let line = &item["snapshot"]["lines"][0];
            assert!(rows(&line["mod_tags"]).is_empty());
            assert_eq!(line["value_scalar"], 1);
            assert_eq!(line["catalyst_factor"], 1);
            assert_eq!(line["field_types"]["corrupted_range"], "nil");
            assert_eq!(line["field_types"]["extra"], "nil");
        }
    }
    let probes = rows(&builds[1]["state"]["probes"]);
    assert_eq!(probes.len(), 25);
    let find = |name: &str| probes.iter().find(|r| r["name"] == name).unwrap();
    let amount = |name: &str| {
        find(name)["after_build"]["active"][0]["value"]["value"]
            .as_f64()
            .unwrap()
    };
    assert_eq!(amount("amount-five"), 5.0);
    assert_eq!(amount("untagged-necrotic"), 5.0);
    assert_eq!(amount("tagged-necrotic"), 6.0);
    assert_eq!(amount("tagged-default-quality"), 6.0);
    assert_eq!(amount("tagged-wrong-catalyst"), 5.0);
    assert_eq!(amount("tagged-quality-fifty"), 7.0);
    assert_eq!(amount("desecrated"), 5.0);
    assert_eq!(amount("untagged-minion-magnitude"), 5.0);
    assert_eq!(amount("tagged-minion-magnitude"), 7.0);
    assert_eq!(amount("desecrated-magnitude"), 7.0);
    assert_eq!(amount("corrupted-fixed"), 8.0);
    // Complete item cache history is stricter than standalone numeric parity.
    // This counterexample must remain excluded by initial source admission.
    assert_eq!(amount("corrupted-fixed-magnitude"), 7.0);
    assert_eq!(amount("corrupted-ranged-magnitude"), 12.0);
    assert_eq!(amount("add-then-double"), 15.0);
    assert_eq!(amount("double-then-add"), 12.0);
    assert_eq!(amount("unscalable"), 5.0);
    assert!(rows(&find("disabled")["after_build"]["active"]).is_empty());
    assert_eq!(amount("unknown-predecessor"), 5.0);
    for category in ["explicit", "implicit", "enchant"] {
        let matched = format!("category-{category}-matched");
        let mismatched = format!("category-{category}-mismatched");
        assert_eq!(amount(&matched), 7.0);
        assert_eq!(amount(&mismatched), 5.0);
        for name in [matched, mismatched] {
            let line = &find(&name)["after_build"]["lines"][0];
            assert_eq!(line["category"], category);
            assert!(rows(&line["mod_tags"]).is_empty());
        }
    }
    let levels = &builds[1]["state"]["item_level_probes"];
    assert_eq!(levels["fresh_original"]["field_type"], "nil");
    assert_eq!(levels["header_present"]["value"], 17);
    assert_eq!(levels["reused_after_header_removal"]["value"], 17);
    assert_eq!(levels["fresh_header_absent"]["field_type"], "nil");
    assert_eq!(rows(&find("repeated")["after_build"]["active"]).len(), 2);
    let copies = rows(&builds[1]["state"]["copy_probes"]);
    assert_eq!(copies.len(), 4);
    for (copy, (scale, expected)) in copies.iter().zip([
        (0.0, vec![0, 0]),
        (0.25, vec![0, 0]),
        (0.5, vec![0, 1]),
        (1.0, vec![1, 3]),
    ]) {
        assert_eq!(copy["scale"], scale);
        assert_eq!(
            rows(&copy["records"])
                .iter()
                .map(|m| m["value"]["value"].as_i64().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
const OBSERVE: &str = r#"
local function original(f,path,first,last)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first)
 if last then assert(i.lastlinedefined==last,path..":"..tostring(i.linedefined).."-"..tostring(i.lastlinedefined)) end;return f
end
local function upvalue(f,wanted)
 for i=1,100 do local name,value=debug.getupvalue(f,i);if not name then break end;if name==wanted then return value end end
 error("missing original upvalue "..wanted)
end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local buildMods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local formatter=original(itemLib.formatValue,"Modules/ItemTools.lua",45,58)
local applyRange=original(itemLib.applyRange,"Modules/ItemTools.lua",130)
local parser=original(modLib.parseMod,"Modules/ModParser.lua",7404)
local scaleAdd=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82,119)
local doc,err=common.xml.ParseXML(minionLevelXml);assert(doc and not err)
local itemsNode;for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Items" then assert(not itemsNode);itemsNode=n end end;assert(itemsNode)
local rawItems,ids={},{}
for _,n in ipairs(itemsNode) do if type(n)=="table" and n.elem=="Item" then
 local text={};for _,s in ipairs(n) do if type(s)=="string" then text[#text+1]=s end end
 local raw=table.concat(text,"\n");local id=assert(tonumber(n.attrib.id));rawItems[id]=raw
 if raw:find(" to Level of all Minion Skills",1,true) then ids[#ids+1]=id end
end end;table.sort(ids)
local function plain(value,depth)
 if type(value)~="table" then assert(type(value)~="function" and type(value)~="userdata");return value end
 depth=(depth or 0)+1;assert(depth<9);local result={};local count=0
 for k,v in pairs(value) do count=count+1;assert(count<=256);result[k]=plain(v,depth) end;return result
end
local function equal(a,b)
 if type(a)~=type(b) then return false end;if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function record(mod)
 local tags={};for _,tag in ipairs(mod) do tags[#tags+1]=plain(tag) end
 return {name=mod.name,type=mod.type,source=mod.source,flags=mod.flags,keyword_flags=mod.keywordFlags,value=plain(mod.value),tags=tags}
end
local function properties(list)
 local out={};for _,mod in ipairs(list or {}) do
 if mod.name=="GemProperty" and type(mod.value)=="table" and mod.value.keyword=="minion" and mod.value.key=="level" then out[#out+1]=record(mod) end
 end;return out
end
local function snapshot(item,activeList)
 local lines={};for _,category in ipairs({"enchant","rune","implicit","explicit"}) do
  for _,line in ipairs(item[category.."ModLines"]) do if line.line:find(" to Level of all Minion Skills",1,true) then
   lines[#lines+1]={line=line.line,category=category,mod_tags=plain(line.modTags),desecrated=line.desecrated,unscalable=line.unscalable,disabled=line.disabled,
    prefix=line.prefix,suffix=line.suffix,extra=line.extra,range=line.range,corrupted_range=line.corruptedRange,value_scalar=line.valueScalar,
    field_types={corrupted_range=type(line.corruptedRange),value_scalar=type(line.valueScalar),extra=type(line.extra)},
    catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality),records=properties(line.modList)}
  end end
 end
 return {name=item.name,base=item.baseName,kind=item.type,crafted=item.crafted,advanced_copy=item.advancedCopy,
  catalyst=item.catalyst,catalyst_quality=item.catalystQuality,quality=item.quality,item_level=item.itemLevel,
  field_types={catalyst=type(item.catalyst),catalyst_quality=type(item.catalystQuality),item_level=type(item.itemLevel)},lines=lines,active=properties(activeList)}
end
local function construct(raw,id)
 local item=new("Item"):Item("");item.id=id;parse(item,raw);return item
end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},external_properties={},probes={},copy_probes={}}
local observedSkill
for _,skill in ipairs(env.player.activeSkillList) do if skill.activeEffect.grantedEffect.id==minionLevelSkill then assert(not observedSkill);observedSkill=skill end end
assert(observedSkill)
for _,row in ipairs(observedSkill.activeEffect.gemPropertyInfo or {}) do result.external_properties[#result.external_properties+1]={value=plain(row.value),mod=record(row.mod)} end
local originals,snapshots={},{}
for _,id in ipairs(ids) do
 local originalItem=assert(build.itemsTab.items[id]);originals[id]=originalItem
 -- Reading the already-built base list avoids mutating source item caches.
 local before=snapshot(originalItem,originalItem.baseModList);snapshots[id]=before
 local fresh=construct(rawItems[id],id);buildMods(fresh)
 local current=snapshot(fresh,active(fresh,1,false));assert(equal(before.lines,current.lines));assert(equal(before.active,current.active))
 local rawLines={};for line in rawItems[id]:gmatch("[^\r\n]+") do if line:find(" to Level of all Minion Skills",1,true) then rawLines[#rawLines+1]=line end end
 result.items[#result.items+1]={item_id=id,raw_lines=rawLines,snapshot=before,reparsed_equal=true}
end
if minionLevelComponents then
 local crown=assert(rawItems[21]);assert(not crown:find("Item Level:",1,true))
 local at=assert(crown:find("Crafted: true",1,true));local withLevel=crown:sub(1,at-1).."Item Level: 17\n"..crown:sub(at)
 local function level(item)return {field_type=type(item.itemLevel),value=item.itemLevel}end
 local reused=construct(withLevel,9003);buildMods(reused)
 result.item_level_probes={fresh_original=level(assert(build.itemsTab.items[21])),header_present=level(reused)}
 parse(reused,crown);buildMods(reused);result.item_level_probes.reused_after_header_removal=level(reused)
 local fresh=construct(crown,9004);buildMods(fresh);result.item_level_probes.fresh_header_absent=level(fresh)
 local base=assert(rawItems[23]);local needle="+1 to Level of all Minion Skills"
 local function altered(line,headers,after)
  local at=assert(base:find(needle,1,true));assert(not base:find(needle,at+#needle,true))
  local raw=base:sub(1,at-1)..line..base:sub(at+#needle)
  if headers then local pos=assert(raw:find("Crafted: true",1,true));raw=raw:sub(1,pos-1)..headers.."\n"..raw:sub(pos) end
  if after then raw=raw.."\n"..after end;return raw
 end
 local cases={
  {"amount-five","+5 to Level of all Minion Skills"},
  {"untagged-necrotic","+5 to Level of all Minion Skills","Catalyst: Necrotic\nCatalystQuality: 20"},
  {"tagged-necrotic","{tags:minion}+5 to Level of all Minion Skills","Catalyst: Necrotic\nCatalystQuality: 20"},
  {"tagged-default-quality","{tags:minion}+5 to Level of all Minion Skills","Catalyst: Necrotic"},
  {"tagged-wrong-catalyst","{tags:minion}+5 to Level of all Minion Skills","Catalyst: Flesh\nCatalystQuality: 20"},
  {"tagged-quality-fifty","{tags:minion}+5 to Level of all Minion Skills","Catalyst: Necrotic\nCatalystQuality: 50"},
  {"desecrated","{desecrated}+5 to Level of all Minion Skills"},
  {"untagged-minion-magnitude","+5 to Level of all Minion Skills",nil,"50% increased minion modifier magnitudes"},
  {"tagged-minion-magnitude","{tags:minion}+5 to Level of all Minion Skills",nil,"50% increased minion modifier magnitudes"},
  {"desecrated-magnitude","{desecrated}+5 to Level of all Minion Skills",nil,"50% increased desecrated modifier magnitudes"},
  {"add-then-double","+5 to Level of all Minion Skills",nil,"50% increased explicit modifier magnitudes\nExplicit Modifier magnitudes are doubled"},
  {"double-then-add","+5 to Level of all Minion Skills",nil,"Explicit Modifier magnitudes are doubled\n50% increased explicit modifier magnitudes"},
  {"corrupted-fixed","{corruptedRange:1.5}+5 to Level of all Minion Skills"},
  {"corrupted-fixed-magnitude","{corruptedRange:1.5}+5 to Level of all Minion Skills",nil,"50% increased explicit modifier magnitudes"},
  {"corrupted-ranged-magnitude","{range:0.5}{corruptedRange:1.5}+(5-5) to Level of all Minion Skills",nil,"50% increased explicit modifier magnitudes"},
  {"unscalable","{unscalable}{tags:minion}+5 to Level of all Minion Skills","Catalyst: Necrotic\nCatalystQuality: 20","50% increased explicit modifier magnitudes"},
  {"disabled","{disabled}+5 to Level of all Minion Skills"},
  {"repeated","+1 to Level of all Minion Skills\n+3 to Level of all Minion Skills"},
  {"unknown-predecessor","Owned witness unknown modifier\n+5 to Level of all Minion Skills"}
 }
 -- Same physical value and empty modifier tags; only the source category and
 -- category-selected magnitude differ. This supplies no new import authority.
 for _,category in ipairs({"explicit","implicit","enchant"}) do
  local line=(category=="explicit" and "" or "{"..category.."}").."+5 to Level of all Minion Skills"
  cases[#cases+1]={"category-"..category.."-matched",line,nil,"50% increased "..category.." modifier magnitudes"}
  local other=category=="explicit" and "implicit" or "explicit"
  cases[#cases+1]={"category-"..category.."-mismatched",line,nil,"50% increased "..other.." modifier magnitudes"}
 end
 for _,case in ipairs(cases) do
  local raw=altered(case[2],case[3],case[4]);local item=construct(raw,9001)
  local before=snapshot(item,nil);buildMods(item);local after=snapshot(item,active(item,1,false))
  local again=snapshot(item,active(item,1,false));assert(equal(after,again))
  result.probes[#result.probes+1]={name=case[1],raw=raw,input_line=case[2],before_build=before,after_build=after}
 end
 local repeated=construct(altered("+1 to Level of all Minion Skills\n+3 to Level of all Minion Skills"),9002);buildMods(repeated)
 local list=active(repeated,1,false);local originals=properties(list)
 for _,factor in ipairs({0,0.25,0.5,1}) do
  local target=new("ModList"):ModList()
  for _,mod in ipairs(list) do if mod.name=="GemProperty" and mod.value.keyword=="minion" and mod.value.key=="level" then scaleAdd(target,mod,factor) end end
  result.copy_probes[#result.copy_probes+1]={scale=factor,records=properties(target)}
 end
 assert(equal(originals,properties(list)))
end
for _,id in ipairs(ids) do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].baseModList),snapshots[id])) end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==buildMods and class.GetActiveModListForSlotNum==active and itemLib.formatValue==formatter and itemLib.applyRange==applyRange and modLib.parseMod==parser and common.classes.ModStore.ScaleAddMod==scaleAdd and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
return result
"#;
