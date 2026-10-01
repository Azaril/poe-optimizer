//! Complete source item inventories; no native input/owner closure authority.
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
const TEST: &str = "simple_saved_items_keep_complete_source_lists_separate_from_input_closure";
const CHILD: &str = "POE_SIMPLE_ITEM_INPUTS_CHILD";

#[test]
fn simple_saved_items_keep_complete_source_lists_separate_from_input_closure() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-simple-item-inputs-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let manifest = read(&fixtures.join("index.json"));
        let entry = rows(&manifest["builds"])
            .iter()
            .find(|r| r["xml"] == "build-05.xml")
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([53; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        for (id, ordinal) in [("19", 572), ("20", 574)] {
            let matches: Vec<_> = evidence
                .rows()
                .iter()
                .filter(|r| {
                    r.occurrence().name() == "Item"
                        && r.attribute("id").and_then(|a| a.decoded().ok()) == Some(id)
                })
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].occurrence().id().ordinal(), ordinal);
        }
        let changed_headers = edit_item(&edit_item(&xml, 19, change_headers), 20, change_headers);
        let changed_variants = edit_item(&xml, 19, |body| {
            body.replace(
                "Crafted: true",
                "Variant: First\nVariant: Second\nCrafted: true",
            )
            .replace(
                "+17 to maximum Life",
                "+17 to maximum Life\n{variant:1}+3 to maximum Life\n{variant:2}+5 to maximum Life",
            )
        })
        .replace("<Item id=\"19\">", "<Item id=\"19\" variant=\"2\">");
        let mut cases = Vec::new();
        let mut source_hash = None;
        for (name, text) in [
            ("original", &xml),
            ("display-and-affix-headers", &changed_headers),
            ("xml-variant-two", &changed_variants),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("simpleItemXml", text.as_str())?;
                lua.globals()
                    .set("simpleItemControls", name == "original")?;
                lua.globals().set("simpleItemJit", enabled)?;
                lua.load("if simpleItemJit then jit.on() else jit.off();jit.flush() end")
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
            if let Some(hash) = &source_hash {
                assert_eq!(&result["source_hash"], hash);
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
            "manifest_sha256":pinned::manifest_sha256(),"native_input_closure":false,"native_owner_coverage":false,
            "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/Build.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/Bases/body.lua","src/Data/Bases/gloves.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
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
fn edit_item(xml: &str, id: usize, edit: impl FnOnce(&str) -> String) -> String {
    let opening = format!("<Item id=\"{id}\">");
    assert_eq!(xml.matches(&opening).count(), 1);
    let begin = xml.find(&opening).unwrap() + opening.len();
    let end = begin + xml[begin..].find("</Item>").unwrap();
    let mut result = xml.to_owned();
    result.replace_range(begin..end, &edit(&xml[begin..end]));
    result
}
fn change_headers(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            if line.starts_with("Energy Shield:") {
                "Energy Shield: 9999"
            } else if line.starts_with("Armour:") {
                "Armour: 9998"
            } else if line.starts_with("Prefix:") {
                "Prefix: None"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@owned-simple-item-inputs-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn life_values(snapshot: &Json) -> Vec<f64> {
    rows(&snapshot["active"])
        .iter()
        .filter(|r| r["name"] == "Life" && r["type"] == "BASE")
        .map(|r| r["value"].as_f64().unwrap())
        .collect()
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 3);
    for case in cases {
        let s = &case["state"];
        for field in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(s[field], true, "{field}");
        }
        assert_eq!(s["selected_items"], 2);
        assert_eq!(s["selected_spec"], 3);
        assert_eq!(s["selected_skills"], 4);
        assert_eq!(s["selected_config"], 1);
    }
    let original = &cases[0]["state"];
    let baseline = rows(&original["items"]);
    assert_eq!(baseline.len(), 2);
    for (i, (base, life, sockets)) in [("Tattered Robe", 17.0, 4), ("Rope Cuffs", 16.0, 3)]
        .into_iter()
        .enumerate()
    {
        let item = &baseline[i];
        let s = &item["loaded"];
        assert_eq!(s["base"], base);
        assert_eq!(s["rarity"], "RARE");
        assert_eq!(s["quality"], 20);
        assert_eq!(s["crafted"], true);
        assert_eq!(s["field_types"]["itemLevel"], "nil");
        assert_eq!(s["field_types"]["catalyst"], "nil");
        for absent in [
            "corrupted",
            "variant",
            "variantAlt",
            "hasAltVariant",
            "allowDuplicateVariants",
            "classRestriction",
        ] {
            assert_eq!(s["field_types"][absent], "nil", "{base}/{absent}");
        }
        assert_eq!(s["itemSocketCount"], sockets);
        assert_eq!(s["jewelSocketCount"], 0);
        assert_eq!(rows(&s["runes"]).len(), sockets);
        assert!(rows(&s["runes"]).iter().all(|r| r == "None"));
        for category in ["buff", "enchant", "rune", "classRequirement", "implicit"] {
            assert!(rows(&s["lists"][category]).is_empty(), "{base}/{category}");
        }
        assert_eq!(rows(&s["lists"]["explicit"]).len(), 1);
        assert_eq!(rows(&s["base_mods"]).len(), 1);
        assert_eq!(life_values(s), vec![life]);
        assert_eq!(s["lists"]["explicit"][0]["records"][0]["name"], "Life");
        assert_eq!(s["lists"]["explicit"][0]["records"][0]["value"], life);
        assert_eq!(item["fresh"]["base_mods"], s["base_mods"]);
        assert_eq!(item["fresh"]["active"], s["active"]);
        assert_eq!(item["base_facts"]["implicit"], Json::Null);
        assert!(rows(&item["base_facts"]["implicit_mod_types"]).is_empty());
        assert_eq!(rows(&item["requirements_rows"]).len(), 1);
        assert_eq!(
            item["requirements_rows"][0]["Str"],
            if i == 0 { 0 } else { 6 }
        );
        assert_eq!(
            item["requirements_rows"][0]["Int"],
            if i == 0 { 0 } else { 6 }
        );
        assert_eq!(item["rare_slot_conditions"][0], true);
        let changed = &cases[1]["state"]["items"][i]["loaded"];
        assert_eq!(changed["armourData"], s["armourData"]);
        assert_eq!(changed["active"], s["active"]);
        assert_eq!(changed["base_mods"], s["base_mods"]);
        assert_eq!(changed["prefixes"][0]["modId"], "None");
    }
    assert_eq!(baseline[0]["loaded"]["armourData"]["EnergyShield"], 34);
    assert_eq!(baseline[1]["loaded"]["armourData"]["Armour"], 16);
    assert_eq!(baseline[1]["loaded"]["armourData"]["EnergyShield"], 7);
    assert_eq!(rows(&baseline[0]["loaded"]["active"]).len(), 3);
    assert_eq!(rows(&baseline[1]["loaded"]["active"]).len(), 2);
    assert_eq!(
        baseline[0]["loaded"]["active"][1]["name"],
        "Multiplier:QualityOnBody Armour"
    );
    assert_eq!(baseline[0]["loaded"]["active"][1]["value"], 20);
    assert_eq!(baseline[0]["loaded"]["active"][2]["name"], "MovementSpeed");
    assert_eq!(baseline[0]["loaded"]["active"][2]["value"], -0.03);
    assert_eq!(
        baseline[0]["loaded"]["active"][2]["tags"],
        json!([{"type":"Condition","var":"IgnoreMovementPenalties","neg":true}])
    );
    let variant = &cases[2]["state"]["items"][0]["loaded"];
    assert_eq!(variant["variant"], 2);
    assert_eq!(rows(&variant["lists"]["explicit"]).len(), 3);
    assert_eq!(life_values(variant), vec![17.0, 5.0]);
    let probes = rows(&original["probes"]);
    let p = |name: &str| probes.iter().find(|r| r["name"] == name).unwrap();
    assert_eq!(life_values(&p("second-member")["after"]), vec![17.0, 3.0]);
    assert!(
        rows(&p("unknown-member")["after"]["lists"]["explicit"])
            .iter()
            .any(|r| r["extra"].is_string())
    );
    assert_eq!(p("required-level")["after"]["requirements"]["level"], 77);
    assert_eq!(p("quality-zero")["after"]["quality"], 0);
    assert_eq!(p("quality-zero")["after"]["armourData"]["EnergyShield"], 28);
    assert_eq!(p("flesh-catalyst")["after"]["catalyst"], 1);
    assert_eq!(life_values(&p("flesh-catalyst")["after"]), vec![17.0]);
    assert_eq!(p("corrupted")["after"]["corrupted"], true);
    assert_eq!(
        life_values(&p("disabled-member")["after"]),
        Vec::<f64>::new()
    );
    assert!(!rows(&p("occupied-rune")["after"]["lists"]["rune"]).is_empty());
    assert!(
        rows(&p("unknown-rune")["after"]["runes"])
            .iter()
            .any(|r| r == "OwnedWitnessUnknownRune")
    );
    let ordered = &p("source-category-order")["after"];
    assert_eq!(rows(&ordered["lists"]["enchant"]).len(), 1);
    assert_eq!(rows(&ordered["lists"]["implicit"]).len(), 1);
    let names: Vec<_> = rows(&ordered["base_mods"])
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Mana", "Str", "Life"]);
}

const OBSERVE: &str = r##"
local function original(f,path,first,last)local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last)end;return f end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local slotLists=original(class.BuildModListsForSlots,"Classes/Item.lua",2671,2680)
local slotList=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414)
local variantCheck=original(class.CheckModLineVariant,"Classes/Item.lua",2289,2318)
local variantCount=original(class.GetModLineVariantCount,"Classes/Item.lua",2320,2336)
local function plain(v,d)if type(v)~="table"then assert(type(v)~="function"and type(v)~="userdata");return v end;d=(d or 0)+1;assert(d<10);local o={};local n=0;for k,x in pairs(v)do n=n+1;assert(n<512);o[k]=plain(x,d)end;return o end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function records(list)local out={};for _,m in ipairs(list or {})do local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end;out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}end;return out end
local scalarFields={"title","name","baseName","type","rarity","crafted","itemLevel","quality","corrupted","mirrored","split","catalyst","catalystQuality","itemSocketCount","jewelSocketCount","variant","variantAlt","hasAltVariant","allowDuplicateVariants","classRestriction","socketedAugmentTypeOverride","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier"}
local function snapshot(item,list)
 local out={base=item.baseName,field_types={},lists={},base_mods=records(item.baseModList),active=records(list)}
 for _,k in ipairs(scalarFields)do out[k]=item[k];out.field_types[k]=type(item[k])end
 for _,k in ipairs({"prefixes","suffixes","requirements","armourData","sockets","runes","variantList","socketedSoulCoreTypes"})do out[k]=plain(item[k]);out.field_types[k]=type(item[k])end
 for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"})do local lines={};for _,line in ipairs(item[category.."ModLines"] or {})do
  local row={line=line.line,records=records(line.modList),field_types={extra=type(line.extra)}}
  for _,k in ipairs({"extra","modTags","variantList","range","corruptedRange","valueScalar","custom","unscalable","disabled","bonded","prefix","suffix","rune","enchant","augmentType","socketedAugmentTypeOverride","socketedSoulCoreType"})do row[k]=plain(line[k])end
  lines[#lines+1]=row
 end;out.lists[category]=lines end
 return out
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);return item end
local doc,err=common.xml.ParseXML(simpleItemXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};local xmlRows={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local text={};local childNames={};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s else childNames[#childNames+1]={name=s.elem,attributes=plain(s.attrib)}end end;local id=assert(tonumber(v.attrib.id));rawItems[id]=table.concat(text,"\n");xmlRows[id]={attributes=plain(v.attrib),text_nodes=#text,children=childNames}end end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},probes={}}
local originals,snapshots={},{}
for _,id in ipairs({19,20})do local item=assert(build.itemsTab.items[id]);originals[id]=item;local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local fresh=construct(rawItems[id],id);local before=snapshot(fresh,nil);mods(fresh);local after=snapshot(fresh,active(fresh,1,false));local selected={};for slot,v in pairs(env.player.itemList)do if v==item then selected[#selected+1]=slot end end;table.sort(selected)
 local requirements={};for _,row in ipairs(env.requirementsTableItems)do if row.sourceItem==item then requirements[#requirements+1]={source=row.source,slot=row.sourceSlot,Str=row.Str,Dex=row.Dex,Int=row.Int}end end
 local rare={};for _,slot in ipairs(selected)do rare[#rare+1]=env.itemModDB.conditions["RareItemIn"..slot]end
 result.items[#result.items+1]={id=id,raw=rawItems[id],xml=xmlRows[id],loaded=loaded,fresh_before_build=before,fresh=after,selected_slots=selected,requirements_rows=requirements,rare_slot_conditions=rare,base_facts={name=item.baseName,type=item.base.type,subtype=item.base.subType,implicit=item.base.implicit,implicit_mod_types=plain(item.base.implicitModTypes),armour=plain(item.base.armour),requirements=plain(item.base.req),quality=item.base.quality,socket_limit=item.base.socketLimit}}
end
if simpleItemControls then
 local raw=assert(rawItems[19]);local function replace(text,old,new)local start=assert(text:find(old,1,true));assert(not text:find(old,start+#old,true));return text:sub(1,start-1)..new..text:sub(start+#old)end
 local function header(text,value)return replace(text,"Crafted: true",value.."\nCrafted: true")end
 local cases={
 {"second-member",raw.."\n+3 to maximum Life"},{"unknown-member",raw.."\nOwned witness unknown semantic line"},
 {"required-level",replace(raw,"LevelReq: 0","LevelReq: 77")},{"quality-zero",replace(raw,"Quality: 20","Quality: 0")},
 {"flesh-catalyst",header(raw,"Catalyst: Flesh\nCatalystQuality: 20")},{"corrupted",raw.."\nCorrupted"},
 {"disabled-member",replace(raw,"+17 to maximum Life","{disabled}+17 to maximum Life")},
 {"source-category-order",raw.."\n{implicit}+2 to Strength\n{enchant}+3 to maximum Mana"},
 {"missing-rune-declarations",raw:gsub("Rune: None[\r\n]+","")},
 {"unknown-rune",raw:gsub("Rune: None","Rune: OwnedWitnessUnknownRune",1)}}
 local ordinary=construct(raw,19);local broad,specific=ordinary:GetSocketedAugmentTypes();local names={};for name,rune in pairs(data.itemMods.Runes)do if(rune[broad]and #rune[broad]>0)or(rune[specific]and #rune[specific]>0)then names[#names+1]=name end end;table.sort(names);local rune=assert(names[1]);cases[#cases+1]={"occupied-rune",raw:gsub("Rune: None","Rune: "..rune,1)}
 for _,case in ipairs(cases)do local item=construct(case[2],9001);local before=snapshot(item,nil);mods(item);local after=snapshot(item,active(item,1,false));assert(equal(after,snapshot(item,active(item,1,false))));result.probes[#result.probes+1]={name=case[1],raw=case[2],before=before,after=after}end
end
for _,id in ipairs({19,20})do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and class.BuildModListsForSlots==slotLists and class.BuildModListForSlotNum==slotList and class.CheckModLineVariant==variantCheck and class.GetModLineVariantCount==variantCount);result.original_functions_preserved=true
return result
"##;
