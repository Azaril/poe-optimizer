//! Complete source observations for flat Life; no native whole-Life coverage claim.
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
const TEST: &str = "original_flat_life_keeps_source_inputs_and_actual_player_delivery_distinct";
const CHILD: &str = "POE_FLAT_LIFE_CHILD";

fn expected(build: usize) -> Vec<(u64, usize, u64)> {
    match build {
        1 => vec![(321, 16, 61)],
        2 => vec![
            (482, 12, 21),
            (501, 19, 69),
            (506, 21, 25),
            (519, 16, 15),
            (553, 11, 50),
            (562, 17, 213),
            (598, 16, 63),
            (605, 17, 26),
        ],
        3 => vec![(326, 20, 44)],
        4 => vec![
            (192, 14, 91),
            (201, 19, 65),
            (201, 27, 44),
            (228, 15, 128),
            (247, 10, 93),
            (257, 11, 68),
            (274, 14, 165),
        ],
        5 => vec![
            (480, 18, 130),
            (534, 21, 26),
            (550, 17, 148),
            (572, 20, 17),
            (574, 20, 16),
            (587, 14, 10),
            (590, 16, 10),
        ],
        _ => unreachable!(),
    }
}

#[test]
fn original_flat_life_keeps_source_inputs_and_actual_player_delivery_distinct() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-flat-life-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let manifest = read(&fixtures.join("index.json"));
        let mut cases = Vec::new();
        let mut source_hash = None;
        for build in 1..=5 {
            let file = format!("build-{build:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&file)).unwrap();
            let entry = rows(&manifest["builds"])
                .iter()
                .find(|r| r["xml"] == file)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            let imported = ImportedBuildInstance::from_decoded(
                decode_build(xml.as_bytes()).unwrap(),
                BuildLineage::from_bytes([49; 16]),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let evidence =
                SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
            let expected:Vec<_>=expected(build).into_iter().map(|(ordinal,line,amount)| {
                let found:Vec<_>=evidence.rows().iter().filter(|r|r.occurrence().name()=="Item"&&u64::from(r.occurrence().id().ordinal())==ordinal).collect();
                assert_eq!(found.len(),1);let item:u64=found[0].attribute("id").unwrap().decoded().unwrap().parse().unwrap();
                json!({"item":item,"source":ordinal,"line":line,"amount":amount,"raw":format!("+{amount} to maximum Life")})
            }).collect();
            let mut variants = vec![("original", xml.clone())];
            if build == 5 {
                for (name, fraction) in [("overlay-zero", "0"), ("overlay-one", "1")] {
                    variants.push((name, overlay(&overlay(&xml, 19, fraction), 20, fraction)));
                }
            }
            for (variant, text) in variants {
                let before = |lua: &Lua| {
                    lua.globals().set("lifeXml", text.as_str())?;
                    lua.globals()
                        .set("lifeExpected", lua.to_value(&expected)?)?;
                    lua.globals()
                        .set("lifeControls", build == 5 && variant == "original")?;
                    lua.globals().set("lifeJit", enabled)?;
                    lua.load("if lifeJit then jit.on() else jit.off();jit.flush() end")
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
                if let Some(hash) = &source_hash {
                    assert_eq!(&result["source_hash"], hash);
                } else {
                    source_hash = Some(result["source_hash"].clone());
                }
                cases.push(json!({"build":build,"variant":variant,"xml_sha256":digest(text.as_bytes()),"state":result["additional_observation"]}));
            }
            assert_eq!(fs::read_to_string(fixtures.join(file)).unwrap(), xml);
        }
        let result = json!({"source_hash":source_hash,"evidence":{"manifest_sha256":pinned::manifest_sha256(),"native_parity":false,"whole_life_metric_coverage":false,"files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Classes/ModDB.lua","src/Modules/CalcSetup.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/ModScalability.lua","src/Data/ModItem.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))},"cases":cases});
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
            if start.elapsed() > Duration::from_secs(240) {
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
fn overlay(xml: &str, id: usize, fraction: &str) -> String {
    let opening = format!("<Item id=\"{id}\">");
    assert_eq!(xml.matches(&opening).count(), 1);
    let begin = xml.find(&opening).unwrap() + opening.len();
    let end = begin + xml[begin..].find("</Item>").unwrap();
    let raw = &xml[begin..end];
    assert_eq!(raw.matches("range=\"0.5\"").count(), 1);
    format!(
        "{}{}{}",
        &xml[..begin],
        raw.replace("range=\"0.5\"", &format!("range=\"{fraction}\"")),
        &xml[end..]
    )
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-flat-life-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 7);
    let mut original_count = 0;
    for case in cases {
        let state = &case["state"];
        for flag in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(state[flag], true);
        }
        assert_eq!(state["scalability"], json!([{"isScalable":true}]));
        let expected = expected(case["build"].as_u64().unwrap() as usize);
        assert_eq!(rows(&state["physical_lines"]).len(), expected.len());
        for row in rows(&state["physical_lines"]) {
            if case["variant"] == "original" {
                original_count += 1;
            }
            let source = row["source"].as_u64().unwrap();
            let line = row["line"].as_u64().unwrap() as usize;
            let amount = expected
                .iter()
                .find(|r| r.0 == source && r.1 == line)
                .unwrap()
                .2;
            assert_eq!(row["raw"], format!("+{amount} to maximum Life"));
            let parsed = &row["parsed"];
            assert_eq!(
                parsed["category"],
                if case["build"] == 4 && source == 201 && line == 19 {
                    "implicit"
                } else {
                    "explicit"
                }
            );
            assert_eq!(parsed["field_types"]["extra"], "nil");
            assert_eq!(parsed["field_types"]["corrupted_range"], "nil");
            assert_eq!(parsed["catalyst_factor"], 1);
            assert_eq!(parsed["value_scalar"], 1);
            assert!(rows(&parsed["mod_tags"]).is_empty());
            let records = rows(&parsed["records"]);
            assert_eq!(records.len(), 1);
            assert_eq!(records[0]["name"], "Life");
            assert_eq!(records[0]["type"], "BASE");
            assert_eq!(records[0]["value"], amount);
            assert_eq!(records[0]["flags"], 0);
            assert_eq!(records[0]["keyword_flags"], 0);
            assert!(rows(&records[0]["tags"]).is_empty());
            assert!(
                records[0]["source"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("Item:{}:", row["item"].as_u64().unwrap()))
            );
        }
        if case["build"] == 2 {
            for (id, amount) in [(28, 69), (29, 25)] {
                let item = rows(&state["items"])
                    .iter()
                    .find(|r| r["id"] == id)
                    .unwrap();
                assert_eq!(rows(&item["active"]).len(), 1);
                assert_eq!(item["active"][0]["value"], amount);
                assert!(rows(&item["selected_slots"]).is_empty());
                assert!(rows(&item["item_database"]).is_empty());
                assert!(rows(&item["player_database"]).is_empty());
            }
        }
        if case["build"] == 5 {
            assert_eq!(state["selected_items"], 2);
            assert_eq!(state["selected_spec"], 3);
            assert_eq!(state["selected_skills"], 4);
            assert_eq!(state["selected_config"], 1);
            for (id, amount) in [(19, 17), (20, 16), (26, 10)] {
                let item = rows(&state["items"])
                    .iter()
                    .find(|r| r["id"] == id)
                    .unwrap();
                assert!(!rows(&item["selected_slots"]).is_empty());
                let uses = if id == 26 { 2 } else { 1 };
                assert_eq!(rows(&item["selected_slots"]).len(), uses);
                assert_eq!(rows(&item["active"]).len(), 1);
                assert_eq!(item["active"][0]["value"], amount);
                for path in ["item_database", "player_database"] {
                    let records = rows(&item[path]);
                    assert_eq!(records.len(), uses, "item{id}/{path}");
                    for record in records {
                        assert_eq!(record, &item["active"][0]);
                        assert_eq!(record["value"], amount);
                        assert_eq!(record["name"], "Life");
                        assert_eq!(record["type"], "BASE");
                        assert!(rows(&record["tags"]).is_empty());
                    }
                }
                assert_eq!(item["item_database"], item["player_database"]);
                if id != 26 {
                    assert_eq!(item["catalogue_tags"], json!(["resource", "life"]));
                }
            }
            for ordinal in [572, 574] {
                let row = rows(&state["physical_lines"])
                    .iter()
                    .find(|r| r["source"] == ordinal)
                    .unwrap();
                assert_eq!(row["parsed"]["category"], "explicit");
                assert_eq!(
                    row["parsed"]["range"],
                    match case["variant"].as_str().unwrap() {
                        "original" => 0.5,
                        "overlay-zero" => 0.0,
                        "overlay-one" => 1.0,
                        _ => unreachable!(),
                    }
                );
            }
        }
    }
    assert_eq!(original_count, 24);
    let state = &cases
        .iter()
        .find(|r| r["build"] == 5 && r["variant"] == "original")
        .unwrap()["state"];
    let probes = rows(&state["probes"]);
    assert_eq!(probes.len(), 23);
    for (name, amount) in [
        ("fixed", 10),
        ("zero", 0),
        ("fraction-down", 10),
        ("fraction-half", 11),
        ("negative-half", -11),
        ("untagged-flesh", 10),
        ("tagged-flesh", 12),
        ("wrong-catalyst", 10),
        ("explicit-magnitude", 15),
        ("implicit-mismatch", 10),
        ("untagged-life-magnitude", 10),
        ("tagged-life-magnitude", 15),
        ("add-then-double", 30),
        ("double-then-add", 25),
        ("corrupted-fixed", 13),
        ("corrupted-fixed-magnitude", 15),
        ("corrupted-ranged-magnitude", 19),
        ("range-midpoint", 15),
        ("unscalable", 10),
        ("implicit-magnitude", 15),
        ("enchant-magnitude", 15),
    ] {
        let p = probes.iter().find(|r| r["name"] == name).unwrap();
        assert_eq!(rows(&p["active"]).len(), 1, "{name}");
        assert_eq!(p["active"][0]["value"], amount, "{name}");
    }
    assert!(rows(&probes.iter().find(|r| r["name"] == "disabled").unwrap()["active"]).is_empty());
    let zero = probes.iter().find(|r| r["name"] == "zero").unwrap();
    assert_eq!(rows(&zero["after_build"]["lines"]).len(), 1);
    assert_eq!(
        zero["after_build"]["lines"][0]["field_types"]["extra"],
        "nil"
    );
    let bare = probes
        .iter()
        .find(|r| r["name"] == "bare-unsigned")
        .unwrap();
    assert!(rows(&bare["active"]).is_empty());
    assert_eq!(bare["after_build"]["lines"][0]["extra"], " to  ");
    assert_eq!(bare["after_build"]["lines"][0]["records"][0]["value"], 17);
}
const OBSERVE: &str = r##"
local function original(f,path,first,last)local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last)end;return f end
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
local function plain(v,depth)if type(v)~="table"then assert(type(v)~="function"and type(v)~="userdata");return v end;depth=(depth or 0)+1;assert(depth<9);local out={};local n=0;for k,x in pairs(v)do n=n+1;assert(n<=256);out[k]=plain(x,depth)end;return out end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function records(list,source)local out={};for _,m in ipairs(list or {})do if m.name=="Life"and m.type=="BASE"and(not source or m.source==source)then local tags={};for _,v in ipairs(m)do tags[#tags+1]=plain(v)end;out[#out+1]={name=m.name,type=m.type,value=m.value,flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,tags=tags}end end;return out end
local function lines(item)local out={};for _,category in ipairs({"enchant","rune","implicit","explicit"})do for _,line in ipairs(item[category.."ModLines"])do
 if #records(line.modList)>0 or line.line:find("to maximum Life",1,true) then out[#out+1]={line=line.line,category=category,mod_tags=plain(line.modTags),custom=line.custom,prefix=line.prefix,suffix=line.suffix,unscalable=line.unscalable,disabled=line.disabled,bonded=line.bonded,range=line.range,corrupted_range=line.corruptedRange,value_scalar=line.valueScalar,extra=line.extra,catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality),records=records(line.modList),field_types={extra=type(line.extra),corrupted_range=type(line.corruptedRange)}}end
end end;return out end
local function snapshot(item,list)return{base=item.baseName,kind=item.type,raw=item.raw,item_level=item.itemLevel,quality=item.quality,catalyst=item.catalyst,catalyst_quality=item.catalystQuality,field_types={item_level=type(item.itemLevel),catalyst=type(item.catalyst),catalyst_quality=type(item.catalystQuality)},lines=lines(item),active=records(list)}end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);return item end
local doc,err=common.xml.ParseXML(lifeXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local text={};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s end end;rawItems[assert(tonumber(v.attrib.id))]=table.concat(text,"\n")end end
local wanted,ids={},{};for _,row in ipairs(lifeExpected)do if not wanted[row.item]then wanted[row.item]=true;ids[#ids+1]=row.item end end;table.sort(ids)
local physicalCount=0;for _,raw in pairs(rawItems)do for line in (raw.."\n"):gmatch("(.-)\n")do if line:match("^%+%d+ to maximum Life$")then physicalCount=physicalCount+1 end end end;assert(physicalCount==#lifeExpected)
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},physical_lines={},probes={},scalability=plain(assert(data.modScalability["# to maximum Life"]))}
local originals,snapshots={},{}
for _,id in ipairs(ids)do
 local item=assert(build.itemsTab.items[id]);originals[id]=item;local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local fresh=construct(rawItems[id],id);mods(fresh);local again=snapshot(fresh,active(fresh,1,false))
 local selected={};for slot,v in pairs(env.player.itemList)do if v==item then selected[#selected+1]=slot end end;table.sort(selected)
 result.items[#result.items+1]={id=id,raw=rawItems[id],loaded=loaded,fresh=again,active=loaded.active,selected_slots=selected,item_database=records(env.itemModDB.mods.Life,item.modSource),player_database=records(env.player.modDB.mods.Life,item.modSource),catalogue_tags=item.affixes.IncreasedLife1 and plain(item.affixes.IncreasedLife1.modTags)}
 for _,expected in ipairs(lifeExpected)do if expected.item==id then
  local n=0;local rawLine;for line in (rawItems[id].."\n"):gmatch("(.-)\n")do n=n+1;if n==expected.line then rawLine=line end end;assert(rawLine==expected.raw)
  local matches={};for _,line in ipairs(loaded.lines)do if line.line==rawLine then matches[#matches+1]=line end end;assert(#matches==1)
  local freshMatches={};for _,line in ipairs(again.lines)do if line.line==rawLine then freshMatches[#freshMatches+1]=line end end;assert(#freshMatches==1)
  local a,b=plain(matches[1]),plain(freshMatches[1]);a.range=nil;b.range=nil;assert(equal(a,b))
  result.physical_lines[#result.physical_lines+1]={item=id,source=expected.source,line=expected.line,raw=rawLine,parsed=matches[1],fresh=freshMatches[1]}
 end end
end
table.sort(result.physical_lines,function(a,b)if a.source~=b.source then return a.source<b.source end;return a.line<b.line end)
if lifeControls then
 local raw=assert(rawItems[19]);local needle="+17 to maximum Life";local at=assert(raw:find(needle,1,true));assert(not raw:find(needle,at+#needle,true))
 local function changed(line,headers,after)local value=raw:sub(1,at-1)..line..raw:sub(at+#needle);if headers then local pos=assert(value:find("Crafted: true",1,true));value=value:sub(1,pos-1)..headers.."\n"..value:sub(pos)end;if after then value=value.."\n"..after end;return value end
 local cases={
 {"fixed","+10 to maximum Life"},{"zero","+0 to maximum Life"},{"bare-unsigned","17 to maximum Life"},{"fraction-down","+10.49 to maximum Life"},{"fraction-half","+10.5 to maximum Life"},{"negative-half","-10.5 to maximum Life"},
 {"untagged-flesh","+10 to maximum Life","Catalyst: Flesh\nCatalystQuality: 20"},{"tagged-flesh","{tags:life}+10 to maximum Life","Catalyst: Flesh\nCatalystQuality: 20"},{"wrong-catalyst","{tags:life}+10 to maximum Life","Catalyst: Neural\nCatalystQuality: 20"},
 {"explicit-magnitude","+10 to maximum Life",nil,"50% increased explicit modifier magnitudes"},{"implicit-mismatch","+10 to maximum Life",nil,"50% increased implicit modifier magnitudes"},
 {"untagged-life-magnitude","+10 to maximum Life",nil,"50% increased life modifier magnitudes"},{"tagged-life-magnitude","{tags:life}+10 to maximum Life",nil,"50% increased life modifier magnitudes"},
 {"add-then-double","+10 to maximum Life",nil,"50% increased explicit modifier magnitudes\nExplicit Modifier magnitudes are doubled"},{"double-then-add","+10 to maximum Life",nil,"Explicit Modifier magnitudes are doubled\n50% increased explicit modifier magnitudes"},
 {"corrupted-fixed","{corruptedRange:1.25}+10 to maximum Life"},{"corrupted-fixed-magnitude","{corruptedRange:1.25}+10 to maximum Life",nil,"50% increased explicit modifier magnitudes"},{"corrupted-ranged-magnitude","{range:0.5}{corruptedRange:1.25}+(10-10) to maximum Life",nil,"50% increased explicit modifier magnitudes"},
 {"range-midpoint","{range:0.5}+(10-20) to maximum Life"},{"unscalable","{unscalable}{tags:life}+10 to maximum Life","Catalyst: Flesh\nCatalystQuality: 20","50% increased explicit modifier magnitudes"},{"disabled","{disabled}+10 to maximum Life"},
 {"implicit-magnitude","{implicit}+10 to maximum Life",nil,"50% increased implicit modifier magnitudes"},{"enchant-magnitude","{enchant}+10 to maximum Life",nil,"50% increased enchantment modifier magnitudes"}}
 for _,case in ipairs(cases)do local text=changed(case[2],case[3],case[4]);local item=construct(text,9001);local before=snapshot(item,nil);mods(item);local list=active(item,1,false);local after=snapshot(item,list);assert(equal(after,snapshot(item,active(item,1,false))));result.probes[#result.probes+1]={name=case[1],raw=text,input_line=case[2],before_build=before,after_build=after,active=records(list)}end
end
for _,id in ipairs(ids)do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and itemLib.formatValue==formatter and itemLib.applyRange==range and modLib.parseMod==parser and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
return result
"##;
