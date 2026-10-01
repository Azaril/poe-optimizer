//! Complete pinned-source allocation lifecycle evidence, not native legality.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_source_allocation_access_distinguishes_paid_paths_and_providers";
const CHILD: &str = "POE_ALLOCATION_ACCESS_INPUTS_CHILD";
const HASH01: &str = "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194";
const HASH05: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const HASH03: &str = "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490";

#[test]
fn complete_source_allocation_access_distinguishes_paid_paths_and_providers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-allocation-access-source-01");
    fs::create_dir_all(&out).unwrap();
    let tree_json =
        fs::read_to_string(root.join("vendor/path-of-building-poe2/src/TreeData/0_5/tree.json"))
            .unwrap();
    let tree_hash = sha(&tree_json.replace("\r\n", "\n"));
    assert_eq!(
        tree_hash,
        "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95"
    );
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let path01 = root.join("tests/fixtures/builds/breadth-20260908/build-01.xml");
        let path05 = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
        let path03 = root.join("tests/fixtures/builds/breadth-20260908/build-03.xml");
        let original01 = fs::read_to_string(&path01).unwrap();
        let original05 = fs::read_to_string(&path05).unwrap();
        let original03 = fs::read_to_string(&path03).unwrap();
        assert_eq!(sha(&original01), HASH01);
        assert_eq!(sha(&original05), HASH05);
        assert_eq!(sha(&original03), HASH03);
        let cases = vec![
            ("original05", original05.clone()),
            (
                "missing_saved_class_root",
                edit_spec(&original05, 2, |s| remove_node(s, 54447)),
            ),
            (
                "removed_connector",
                edit_spec(&original05, 2, |s| remove_node(s, 4739)),
            ),
            (
                "changed_class_with_old_saved_roots",
                edit_spec(&original05, 2, |s| {
                    let s = set_attr(s, "classInternalId", "2");
                    let s = set_attr(&s, "classId", "2");
                    let s = set_attr(&s, "ascendancyInternalId", "");
                    set_attr(&s, "ascendClassId", "0")
                }),
            ),
            (
                "changed_ascendancy_with_old_saved_root",
                edit_spec(&original05, 2, |s| {
                    let s = set_attr(s, "ascendancyInternalId", "Sorceress1");
                    set_attr(&s, "ascendClassId", "1")
                }),
            ),
            (
                "scoped_connector",
                edit_spec(&original05, 2, |s| {
                    assert!(!s.contains("<WeaponSet"));
                    s.replace("</Spec>", "<WeaponSet1 nodes=\"4739\"/></Spec>")
                }),
            ),
            (
                "scoped_class_root",
                edit_spec(&original05, 2, |s| scoped_node(s, "54447", false)),
            ),
            (
                "numeric_alias_scoped_class_root",
                edit_spec(&original05, 2, |s| scoped_node(s, "054447", true)),
            ),
            (
                "numeric_alias_scoped_connector",
                edit_spec(&original05, 2, |s| scoped_node(s, "04739", true)),
            ),
            ("original01", original01.clone()),
            (
                "removed_from_nothing",
                edit_spec(&original01, 0, |s| {
                    let start = s.find("<Socket nodeId=\"7960\" itemId=\"16\"").unwrap();
                    let end = start + s[start..].find("/>").unwrap() + 2;
                    let mut result = s.to_owned();
                    result.replace_range(start..end, "");
                    result
                }),
            ),
            ("original03_partial_root", original03.clone()),
        ];
        let mut observations = Vec::new();
        let mut source_hash = None;
        for (label, xml) in &cases {
            let before = |lua: &Lua| {
                lua.globals().set("accessXml", xml.as_str())?;
                lua.globals().set("accessCase", *label)?;
                lua.globals().set("accessJit", enabled)?;
                lua.load("if accessJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                xml,
                None,
                false,
                Some(&before),
                Some(&install),
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
            observations.push(json!({"case":label,"input_sha256":sha(xml),"state":result["additional_observation"]}));
        }
        assert_eq!(fs::read_to_string(&path01).unwrap(), original01);
        assert_eq!(fs::read_to_string(&path05).unwrap(), original05);
        assert_eq!(fs::read_to_string(&path03).unwrap(), original03);
        let result = json!({
            "source_hash":source_hash,"manifest_sha256":pinned::manifest_sha256(),
            "files":(["src/Classes/PassiveSpec.lua","src/Classes/PassiveTree.lua",
                "src/Classes/TreeTab.lua","src/Modules/CalcSetup.lua","src/Modules/Common.lua"].map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()}))),
            "tree_json":{"path":"src/TreeData/0_5/tree.json","lf_sha256":tree_hash},
            "partial_root_source":serde_json::from_str::<Json>(&tree_json).unwrap()["nodes"]["44683"],
            "native_access_authority":false,"native_build_parity":false,
            "original_sha256":[HASH01,HASH03,HASH05],"observations":observations,
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

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn edit_spec(xml: &str, index: usize, edit: impl FnOnce(&str) -> String) -> String {
    let start = xml.match_indices("<Spec ").nth(index).unwrap().0;
    let end = start + xml[start..].find("</Spec>").unwrap() + "</Spec>".len();
    let mut result = xml.to_owned();
    result.replace_range(start..end, &edit(&xml[start..end]));
    result
}
fn attr<'a>(spec: &'a str, name: &str) -> &'a str {
    let key = format!(" {name}=\"");
    let start = spec.find(&key).unwrap() + key.len();
    &spec[start..start + spec[start..].find('"').unwrap()]
}
fn set_attr(spec: &str, name: &str, value: &str) -> String {
    let key = format!(" {name}=\"");
    let start = spec.find(&key).unwrap() + key.len();
    let end = start + spec[start..].find('"').unwrap();
    let mut result = spec.to_owned();
    result.replace_range(start..end, value);
    result
}
fn remove_node(spec: &str, node: u32) -> String {
    let mut nodes = attr(spec, "nodes")
        .split(',')
        .map(|s| s.parse::<u32>().unwrap())
        .collect::<Vec<_>>();
    let before = nodes.len();
    nodes.retain(|n| *n != node);
    assert_eq!(nodes.len() + 1, before);
    set_attr(
        spec,
        "nodes",
        &nodes
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(","),
    )
}
fn scoped_node(spec: &str, token: &str, append_alias: bool) -> String {
    assert!(!spec.contains("<WeaponSet"));
    let spec = if append_alias {
        set_attr(spec, "nodes", &format!("{},{token}", attr(spec, "nodes")))
    } else {
        assert!(attr(spec, "nodes").split(',').any(|n| n == token));
        spec.to_owned()
    };
    spec.replace(
        "</Spec>",
        &format!("<WeaponSet1 nodes=\"{token}\"/></Spec>"),
    )
}
fn install(lua: &Lua) -> Result<Function, RuntimeError> {
    Ok(lua
        .load(INSTALL)
        .set_name("@allocation-access-authentication")
        .eval()?)
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let v: Value = lua
        .load(OBSERVE)
        .set_name("@allocation-access-observation")
        .eval()?;
    Ok(lua.from_value(v)?)
}
fn rows(v: &Json) -> &[Json] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn case<'a>(v: &'a Json, name: &str) -> &'a Json {
    &rows(&v["observations"])
        .iter()
        .find(|r| r["case"] == name)
        .unwrap()["state"]
}
fn node(v: &Json, id: u64) -> &Json {
    rows(&v["nodes"]).iter().find(|n| n["id"] == id).unwrap()
}
fn check(result: &Json) {
    assert_eq!(rows(&result["observations"]).len(), 12);
    for observation in rows(&result["observations"]) {
        let s = &observation["state"];
        for key in [
            "fresh_specs",
            "original_functions_preserved",
            "saved_state_preserved",
            "main_output_preserved",
            "main_spec_matches_selected",
        ] {
            assert_eq!(s[key], true, "{} {key}", observation["case"]);
        }
    }
    let original = case(result, "original05");
    assert_eq!(original["selected"]["spec"], 3);
    assert_eq!(original["retained"]["counts"], json!([51, 4, 0, 1, 0, 0]));
    assert_eq!(rows(&original["requested_ids"]).len(), 57);
    assert_eq!(original["requested_ids"], original["retained"]["ids"]);
    assert_eq!(original["retained"]["ids"], original["main"]["ids"]);
    assert!(rows(&original["main_granted_ids"]).is_empty());
    assert!(rows(&original["retained"]["jewels"]).is_empty());
    for n in rows(&original["retained"]["nodes"]) {
        assert_eq!(n["alloc_mode"], 0);
        assert_eq!(n["free"]["type"], "nil");
        assert_eq!(n["granted"]["type"], "nil");
        assert!(rows(&n["radius_providers"]).is_empty());
        if n["type"] != "ClassStart" && n["type"] != "AscendClassStart" {
            assert_eq!(n["connected"]["value"], true);
        }
    }
    let root_missing = case(result, "missing_saved_class_root");
    assert_eq!(root_missing["retained"], original["retained"]);
    assert_eq!(root_missing["added_ids"], json!([54447]));
    for label in ["removed_connector", "scoped_connector"] {
        assert!(
            !rows(&case(result, label)["removed_ids"]).is_empty(),
            "{label} must expose pruning"
        );
    }
    let scoped_root = case(result, "scoped_class_root");
    assert_eq!(node(&scoped_root["retained"], 54447)["alloc_mode"], 1);
    assert!(rows(&scoped_root["removed_ids"]).contains(&json!(4739)));
    let alias_root = case(result, "numeric_alias_scoped_class_root");
    assert_eq!(alias_root["retained"], scoped_root["retained"]);
    let alias_connector = case(result, "numeric_alias_scoped_connector");
    assert_eq!(
        alias_connector["retained"],
        case(result, "scoped_connector")["retained"]
    );
    for (s, aliased_id) in [(alias_root, 54447), (alias_connector, 4739)] {
        assert_eq!(
            rows(&s["requested_ids"])
                .iter()
                .filter(|id| **id == json!(aliased_id))
                .count(),
            2,
            "distinct raw spellings collide through source tonumber"
        );
    }
    let exceptional = case(result, "original01");
    assert_eq!(
        exceptional["retained"]["counts"],
        json!([120, 8, 0, 3, 0, 0])
    );
    assert_eq!(exceptional["main_granted_ids"], json!([26214]));
    assert_eq!(rows(&exceptional["main"]["ids"]).len(), 131);
    assert!(!rows(&exceptional["retained"]["ids"]).contains(&json!(26214)));
    assert_eq!(node(&exceptional["main"], 26214)["granted"]["type"], "nil");
    assert!(!rows(&exceptional["main_grant_records"]).is_empty());
    for id in [338, 8483, 47441, 49088, 55180, 59387] {
        let n = node(&exceptional["retained"], id);
        assert_eq!(n["free"]["type"], "nil");
        assert_eq!(n["granted"]["type"], "nil");
        assert_eq!(n["connected"]["value"], false);
        assert_eq!(n["radius_providers"], json!([7960]));
        assert!(rows(&case(result, "removed_from_nothing")["removed_ids"]).contains(&json!(id)));
    }
    let grant = &original["grant_control"];
    assert_eq!(grant["new_grant"]["free"]["value"], true);
    assert_eq!(grant["new_grant"]["granted"]["value"], true);
    assert_eq!(grant["paid_overlap"]["free"]["type"], "nil");
    assert_eq!(grant["paid_overlap"]["granted"]["type"], "nil");
    assert!(!rows(&grant["saved_ids_with_grant"]).contains(&json!(44871)));
    assert_eq!(grant["paid_counts_before"], grant["paid_counts_after"]);
    assert_eq!(grant["restored"], true);
    let partial = case(result, "original03_partial_root");
    assert_eq!(partial["retained"]["class_root"], 44683);
    assert_eq!(partial["requested_ids"], partial["retained"]["ids"]);
    for absent in [5162, 45406, 50198] {
        assert!(!rows(&partial["requested_ids"]).contains(&json!(absent)));
    }
    assert!(
        rows(&partial["retained"]["nodes"])
            .iter()
            .any(|n| n["type"] != "ClassStart" && n["connected"]["value"] == true)
    );
}

const INSTALL: &str = r#"
local function original(f,path,first)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first,path..":"..tostring(first).." actual "..s..":"..tostring(i.linedefined))
 return f
end
local c=common.classes.PassiveSpec;local t=common.classes.TreeTab;local calcs=require("Modules.CalcBase")
local wrapper=original(c.PassiveSpec,"Modules/Common.lua",167);local constructor
for i=1,20 do local name,value=debug.getupvalue(wrapper,i);if not name then break end;if name=="originalFunc"then assert(not constructor);constructor=value end end
original(assert(constructor),"Classes/PassiveSpec.lua",33)
local methods={PassiveSpec=wrapper,Init=original(c.Init,"Classes/PassiveSpec.lua",45),Load=original(c.Load,"Classes/PassiveSpec.lua",117),
 PostLoad=original(c.PostLoad,"Classes/PassiveSpec.lua",353),ImportFromNodeList=original(c.ImportFromNodeList,"Classes/PassiveSpec.lua",358),
 Save=original(c.Save,"Classes/PassiveSpec.lua",252),CountAllocNodes=original(c.CountAllocNodes,"Classes/PassiveSpec.lua",1029),
 SetGrantedPassiveNodes=original(c.SetGrantedPassiveNodes,"Classes/PassiveSpec.lua",1190),BuildAllDependsAndPaths=original(c.BuildAllDependsAndPaths,"Classes/PassiveSpec.lua",1442),
 CanPathThroughAllocMode=original(c.CanPathThroughAllocMode,"Classes/PassiveSpec.lua",863),FindStartFromNode=original(c.FindStartFromNode,"Classes/PassiveSpec.lua",1060),
 ReplaceNode=original(c.ReplaceNode,"Classes/PassiveSpec.lua",2047),SwitchAttributeNode=original(c.SwitchAttributeNode,"Classes/PassiveSpec.lua",2718)}
local prior={};for _,s in ipairs(build.treeTab.specList)do prior[s]=true end
_allocationAccess={methods=methods,prior=prior,treeLoad=original(t.Load,"Classes/TreeTab.lua",492),treePostLoad=original(t.PostLoad,"Classes/TreeTab.lua",521),initEnv=original(calcs.initEnv,"Modules/CalcSetup.lua",717)}
return function()_allocationAccess.finished=true end
"#;

const OBSERVE: &str = r#"
local auth=assert(_allocationAccess);assert(auth.finished);local calcs=require("Modules.CalcBase")
local doc,err=common.xml.ParseXML(accessXml);assert(doc and not err)
local tree;for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Tree"then assert(not tree);tree=n end end;assert(tree)
local specs={};for _,n in ipairs(tree)do if type(n)=="table"and n.elem=="Spec"then specs[#specs+1]=n end end
local spec=build.spec;local tab=build.treeTab;local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local selected={spec=tab.activeSpec,items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local raw=assert(specs[selected.spec]);assert(spec==tab.specList[selected.spec] and env.spec==spec)
local function field(v)return {type=type(v),value=v}end
local function ids(map)local r={};for id in pairs(map)do assert(type(id)=="number");r[#r+1]=id end;table.sort(r);return r end
local function csv(value)local r={};for token in (value or ""):gmatch("%d+")do r[#r+1]=tonumber(token)end;table.sort(r);return r end
local function plain(v,depth)
 depth=depth or 0;assert(depth<12);local ty=type(v)
 if ty=="string"or ty=="number"or ty=="boolean"or ty=="nil"then return v end
 assert(ty=="table");local out={};for k,x in pairs(v)do assert(type(k)=="string"or type(k)=="number");out[k]=plain(x,depth+1)end;return out
end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function noderow(id,n)
 local providers={};for _,p in ipairs(n.intuitiveLeapLikesAffecting or {})do providers[#providers+1]=p.id end;table.sort(providers)
 local linked,alllinked={},{};for _,p in ipairs(n.linked or {})do alllinked[#alllinked+1]=p.id;if p.alloc then linked[#linked+1]=p.id end end;table.sort(linked);table.sort(alllinked)
 return {id=id,effective_id=n.id,type=n.type,name=n.name,display_name=n.dn,ascendancy=field(n.ascendancyName),alloc=field(n.alloc),alloc_mode=n.allocMode,
  connected=field(n.connectedToStart),free=field(n.isFreeAllocate),granted=field(n.isGrantedPassive),radius_providers=providers,
  unlock=plain(n.unlockConstraint),linked_allocated=linked,linked_ids=alllinked,multiple_choice=field(n.isMultipleChoice),choice_option=field(n.isMultipleChoiceOption)}
end
local function snapshot(s,nodes,withspec)
 local out={ids=ids(nodes),nodes={}};for _,id in ipairs(out.ids)do out.nodes[#out.nodes+1]=noderow(id,nodes[id])end
 if withspec then
  out.counts={s:CountAllocNodes()};out.class=s.curClassId;out.class_internal=s.curClass.integerId;out.ascendancy=s.curAscendClassId;
  out.class_root=s.curClass.startNodeId;out.ascendancy_root=s.curAscendClass.startNodeId;out.jewels={};out.zero_jewel_nodes={}
  for node,itemid in pairs(s.jewels)do if itemid>0 then
   local item=assert(build.itemsTab.items[itemid]);out.jewels[#out.jewels+1]={node=node,item=itemid,name=item.name,title=item.title,radius=item.jewelRadiusIndex,
    alternate_class_start=field(item.jewelData.alternateClassStart),intuitive_leap=field(item.jewelData.intuitiveLeapLike),
    from_nothing=field(item.jewelData.fromNothingKeystone),from_nothing_keystones=plain(item.jewelData.fromNothingKeystones),limit_disabled=field(item.jewelData.limitDisabled)}
   else assert(itemid==0);out.zero_jewel_nodes[#out.zero_jewel_nodes+1]=node end end
  table.sort(out.jewels,function(a,b)return a.node<b.node end);table.sort(out.zero_jewel_nodes)
  out.node_radius_rules=plain(s.intuitiveLeapLikeNodes);out.subgraph_ids=ids(s.subGraphs);out.alloc_subgraph=plain(s.allocSubgraphNodes)
 end;return out
end
local saved={};local seen={};for i,s in ipairs(tab.specList)do assert(not auth.prior[s]and not seen[s]);seen[s]=true;saved[i]={object=s,allocation_table=s.allocNodes,state=snapshot(s,s.allocNodes,true)}end
local primitives={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then primitives[k]=v end end
local result={selected=selected,source_attributes=plain(raw.attrib),source_children={},requested_ids=csv(raw.attrib.nodes),retained=snapshot(spec,spec.allocNodes,true),main=snapshot(spec,env.allocNodes,false),main_granted_ids=ids(env.grantedPassives),main_grant_records=plain(env.modDB.mods.GrantedPassive or {}),fresh_specs=true,main_spec_matches_selected=true}
for _,n in ipairs(raw)do if type(n)=="table"then result.source_children[#result.source_children+1]=plain(n)end end
local requested={};for _,id in ipairs(result.requested_ids)do requested[id]=true end
result.removed_ids={};for _,id in ipairs(result.requested_ids)do if not spec.allocNodes[id]then result.removed_ids[#result.removed_ids+1]=id end end
result.added_ids={};for _,id in ipairs(result.retained.ids)do if not requested[id]then result.added_ids[#result.added_ids+1]=id end end
local savedxml={elem="Spec"};spec:Save(savedxml);result.resaved_ids=csv(savedxml.attrib.nodes)
if accessCase=="original05"then
 -- This isolated source object shares the authenticated static tree, never the
 -- original spec's mutable nodes or MAIN environment. No provider is invented
 -- in the original build; this is a control of the complete grant lifecycle.
 local clone=new("PassiveSpec"):PassiveSpec(build,spec.treeVersion);assert(clone~=spec and clone.nodes~=spec.nodes)
 assert(not clone:Load(copyTable(raw,true),"isolated-grant-control"));clone:PostLoad()
 local before=snapshot(clone,clone.allocNodes,true);assert(not clone.allocNodes[44871]and clone.allocNodes[4739])
 assert(clone:SetGrantedPassiveNodes({[44871]=clone.nodes[44871],[4739]=clone.nodes[4739]}))
 local grant={paid_counts_before=before.counts,paid_counts_after={clone:CountAllocNodes()},new_grant=noderow(44871,clone.nodes[44871]),paid_overlap=noderow(4739,clone.nodes[4739])}
 local sx={elem="Spec"};clone:Save(sx);grant.saved_ids_with_grant=csv(sx.attrib.nodes)
 assert(clone:SetGrantedPassiveNodes({}));grant.restored=equal(before,snapshot(clone,clone.allocNodes,true));assert(grant.restored)
 result.grant_control=grant
 local function structural(n)
  local links={};for _,other in ipairs(n.linked)do links[#links+1]=other.id end;table.sort(links)
  return {id=n.id,type=n.type,ascendancy=field(n.ascendancyName),links=links,unlock=plain(n.unlockConstraint),alloc_mode=n.allocMode,connected=field(n.connectedToStart)}
 end
 local entrance=clone.nodes[4739];local base=clone.tree.nodes[4739];local witch=assert(base.options.Witch)
 assert(base.isSwitchable and witch.id~=base.id)
 local switch={physical_id=4739,option_id=witch.id,before=structural(entrance),before_stats=plain(entrance.sd)}
 clone:ReplaceNode(entrance,witch);switch.witch=structural(entrance);switch.witch_stats=plain(entrance.sd)
 assert(equal(switch.before,switch.witch)and not equal(switch.before_stats,switch.witch_stats))
 clone:ReplaceNode(entrance,base);assert(equal(switch.before,structural(entrance)));switch.restored=true
 result.switch_control=switch
 local attribute=clone.nodes[15782];assert(attribute.isAttribute)
 local attr={physical_id=15782,before=structural(attribute),before_stats=plain(attribute.sd)}
 clone:SwitchAttributeNode(15782,3);clone:BuildAllDependsAndPaths()
 attr.after=structural(attribute);attr.after_stats=plain(attribute.sd)
 assert(equal(attr.before,attr.after)and not equal(attr.before_stats,attr.after_stats));result.attribute_control=attr
end
assert(build.spec==spec and tab.activeSpec==selected.spec and build.itemsTab.activeItemSetId==selected.items and build.skillsTab.activeSkillSetId==selected.skills and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group)
for i,s in ipairs(saved)do assert(tab.specList[i]==s.object and s.object.allocNodes==s.allocation_table and equal(s.state,snapshot(s.object,s.object.allocNodes,true)))end
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(primitives)do assert(output[k]==v)end
for name,f in pairs(auth.methods)do assert(common.classes.PassiveSpec[name]==f)end
assert(common.classes.TreeTab.Load==auth.treeLoad and common.classes.TreeTab.PostLoad==auth.treePostLoad and calcs.initEnv==auth.initEnv)
result.saved_state_preserved=true;result.main_output_preserved=true;result.original_functions_preserved=true
return result
"#;
