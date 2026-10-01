//! Exact original05 tree socket lifecycle; source evidence, not native closure.
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
const TEST: &str = "original05_explicit_empty_tree_sockets_are_fresh_and_preset_local";
const CHILD: &str = "POE_EMPTY_TREE_SOCKETS_CHILD";

#[test]
fn original05_explicit_empty_tree_sockets_are_fresh_and_preset_local() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-empty-tree-sockets-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let fixture = fixtures.join("build-05.xml");
        let xml = fs::read_to_string(&fixture).unwrap();
        let index = read(&fixtures.join("index.json"));
        let row = index["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["xml"] == "build-05.xml")
            .unwrap();
        let hash = format!("{:x}", Sha256::digest(xml.as_bytes()));
        assert_eq!(row["xml_sha256"], hash);
        let occupied = replace_selected_sockets(
            &xml,
            r#"<Sockets><Socket nodeId="7960" itemId="16"/></Sockets>"#,
        );
        let missing = replace_selected_sockets(&xml, "");
        let nested = replace_selected_sockets(
            &xml,
            r#"<Sockets><Hidden><Socket nodeId="7960" itemId="16"/></Hidden></Sockets>"#,
        );
        let mut observations = Vec::new();
        let mut source_hash = None;
        for (label, input, warm) in [
            ("original", xml.as_str(), None),
            ("occupied", occupied.as_str(), None),
            ("missing", missing.as_str(), None),
            ("nested", nested.as_str(), None),
            (
                "original_after_occupied",
                xml.as_str(),
                Some(occupied.as_str()),
            ),
        ] {
            let before = |lua: &Lua| {
                lua.globals().set("emptyTreeInput", input)?;
                lua.globals().set("emptyTreeJit", enabled)?;
                lua.load("if emptyTreeJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                input,
                warm,
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
            observations.push(json!({"case":label,"input_sha256":format!("{:x}",Sha256::digest(input.as_bytes())),"state":result["additional_observation"]}));
        }
        assert_eq!(fs::read_to_string(&fixture).unwrap(), xml);
        let result = json!({"source_hash":source_hash,"evidence":{
            "fixture":"build-05.xml","xml_sha256":hash,"manifest_sha256":pinned::manifest_sha256(),
            "native_parity":false,"membership_closure_granted":false,
            "scope":"complete original05 tree socket loading and preset isolation",
            "files": (["src/Classes/TreeTab.lua","src/Classes/PassiveSpec.lua","src/Modules/Common.lua","src/Modules/Build.lua","src/Classes/ItemsTab.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
        },"observations":observations});
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
fn read(p: &Path) -> Json {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn replace_selected_sockets(xml: &str, replacement: &str) -> String {
    let start = xml.match_indices("<Spec ").nth(2).unwrap().0;
    let end = start + xml[start..].find("</Spec>").unwrap();
    let spec = &xml[start..end];
    assert!(spec.contains("title=\"Act 3\""));
    assert_eq!(spec.matches("<Sockets/>").count(), 1);
    let offset = start + spec.find("<Sockets/>").unwrap();
    let mut result = xml.to_owned();
    result.replace_range(offset..offset + "<Sockets/>".len(), replacement);
    result
}
fn install(lua: &Lua) -> Result<Function, RuntimeError> {
    Ok(lua
        .load(INSTALL)
        .set_name("@empty-tree-call-observer")
        .eval()?)
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@empty-tree-state-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn check(result: &Json) {
    let observations = rows(&result["observations"]);
    assert_eq!(observations.len(), 5);
    for observation in observations {
        let state = &observation["state"];
        assert_eq!(state["active_spec"], 3);
        assert_eq!(state["tree_loads"], 1);
        assert_eq!(state["tree_postloads"], 1);
        assert_eq!(state["fresh_spec_objects"], 7);
        assert_eq!(state["original_functions_preserved"], true);
        assert_eq!(state["saved_state_preserved"], true);
        assert_eq!(state["hook_removed"], true);
        let specs = rows(&state["specs"]);
        assert_eq!(specs.len(), 7);
        for (i, spec) in specs.iter().enumerate() {
            assert_eq!(spec["index"], i + 1);
            assert_eq!(spec["constructor_calls"], 1);
            assert_eq!(spec["load_calls"], 1);
            assert_eq!(spec["postload_calls"], 1);
            assert_eq!(spec["jewels_type"], "table");
            assert!(rows(&spec["zero_socket_nodes"]).iter().all(Json::is_u64));
            let occupied = observation["case"] == "occupied" && i == 2;
            let expected = if i == 4 || i == 5 {
                json!([{"node":7960,"item":16},{"node":21984,"item":17},{"node":61419,"item":18}])
            } else if occupied {
                json!([{"node":7960,"item":16}])
            } else {
                json!([])
            };
            assert_eq!(rows(&spec["jewels"]), rows(&expected));
        }
        let selected = &specs[2];
        assert_eq!(selected["title"], "Act 3");
        assert_eq!(
            selected["source_socket_containers"],
            if observation["case"] == "missing" {
                0
            } else {
                1
            }
        );
        assert_eq!(
            selected["source_direct_socket_rows"],
            usize::from(observation["case"] == "occupied")
        );
        assert_eq!(
            selected["source_nested_socket_rows"],
            usize::from(observation["case"] == "nested")
        );
    }
    assert_eq!(
        observations[0]["state"], observations[4]["state"],
        "fresh and occupied-warmed original loads must agree"
    );
    assert_eq!(
        observations[0]["input_sha256"],
        observations[4]["input_sha256"]
    );
}
const INSTALL: &str = r#"
local function original(f,path,first,last)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first and i.lastlinedefined==last)
 return f
end
assert(debug.gethook()==nil)
local spec=common.classes.PassiveSpec;local tree=common.classes.TreeTab
-- The original class system wraps constructors after their first invocation.
-- Observe its authenticated originalFunc upvalue without replacing either.
local constructor_wrapper=original(spec.PassiveSpec,"Modules/Common.lua",167,183)
local constructor
for i=1,20 do local name,value=debug.getupvalue(constructor_wrapper,i);if not name then break end
 if name=="originalFunc" then assert(not constructor);constructor=value end
end
local functions={
 constructor=original(assert(constructor),"Classes/PassiveSpec.lua",33,43),
 constructor_wrapper=constructor_wrapper,
 init=original(spec.Init,"Classes/PassiveSpec.lua",45,115),
 load=original(spec.Load,"Classes/PassiveSpec.lua",117,250),
 postload=original(spec.PostLoad,"Classes/PassiveSpec.lua",353,355),
 tree_load=original(tree.Load,"Classes/TreeTab.lua",492,519),
 tree_postload=original(tree.PostLoad,"Classes/TreeTab.lua",521,525)
}
local trace={functions=functions,constructors={},loads={},postloads={},prior={},tree_loads=0,tree_postloads=0}
for _,s in ipairs(build.treeTab.specList) do trace.prior[s]=true end
local function hook(event)
 if event~="call" then return end
 local f=debug.getinfo(2,"f").func
 if f==functions.tree_load then trace.tree_loads=trace.tree_loads+1
 elseif f==functions.tree_postload then trace.tree_postloads=trace.tree_postloads+1
 elseif f==functions.constructor or f==functions.load or f==functions.postload then
  local name,self=debug.getlocal(2,1);assert(name=="self" and type(self)=="table")
  local counts=f==functions.constructor and trace.constructors or f==functions.load and trace.loads or trace.postloads
  counts[self]=(counts[self] or 0)+1
 end
end
_emptyTreeTrace=trace;trace.hook=hook;debug.sethook(hook,"c")
return function() assert(debug.gethook()==hook);debug.sethook();trace.removed=true end
"#;
const OBSERVE: &str = r#"
local t=assert(_emptyTreeTrace);assert(t.removed and debug.gethook()==nil)
local doc,err=common.xml.ParseXML(emptyTreeInput);assert(doc and not err)
local sourceTree;for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Tree" then assert(not sourceTree);sourceTree=n end end
assert(sourceTree and sourceTree.attrib.activeSpec=="3")
local sourceSpecs={};for _,n in ipairs(sourceTree) do if type(n)=="table" and n.elem=="Spec" then sourceSpecs[#sourceSpecs+1]=n end end
local tab=build.treeTab;local list=tab.specList;local active=build.spec;assert(tab.activeSpec==3 and active==list[3] and #list==#sourceSpecs)
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local itemSet=build.itemsTab.activeItemSet;local skills=build.skillsTab.activeSkillSetId;local config=build.configTab.activeConfigSetId;local group=build.mainSocketGroup
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local function jewels(spec)
 assert(type(spec.jewels)=="table");local rows,zeros={},{}
 -- Full UI/loadout synchronization may write numeric zero placeholders through
 -- TreeTab.SetActiveSpec. Preserve them separately from equipped membership.
 for node,item in pairs(spec.jewels) do
  assert(type(node)=="number" and node>0 and node==math.floor(node) and type(item)=="number" and item>=0 and item==math.floor(item))
  if item==0 then zeros[#zeros+1]=node
  else assert(build.itemsTab.items[item]);rows[#rows+1]={node=node,item=item} end
 end
 table.sort(rows,function(a,b)return a.node<b.node end);table.sort(zeros);return rows,zeros
end
local function equal(a,b)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local result={active_spec=tab.activeSpec,tree_loads=t.tree_loads,tree_postloads=t.tree_postloads,fresh_spec_objects=0,specs={},hook_removed=true}
local seen={};local saved={}
for i,spec in ipairs(list) do
 assert(not seen[spec] and not t.prior[spec]);seen[spec]=true;result.fresh_spec_objects=result.fresh_spec_objects+1
 local source=sourceSpecs[i];local containers,direct,nested=0,0,0
 for _,n in ipairs(source) do if type(n)=="table" and n.elem=="Sockets" then
  containers=containers+1
  for _,child in ipairs(n) do if type(child)=="table" then
   if child.elem=="Socket" then direct=direct+1
   else for _,grandchild in ipairs(child) do if type(grandchild)=="table" and grandchild.elem=="Socket" then nested=nested+1 end end end
  end end
 end end
 local actual,zeros=jewels(spec);saved[i]={object=spec,table=spec.jewels,jewels=actual,zeros=zeros,title=spec.title}
 result.specs[i]={index=i,title=spec.title,constructor_calls=t.constructors[spec] or 0,load_calls=t.loads[spec] or 0,postload_calls=t.postloads[spec] or 0,jewels_type=type(spec.jewels),jewels=actual,zero_socket_nodes=zeros,source_socket_containers=containers,source_direct_socket_rows=direct,source_nested_socket_rows=nested}
 assert(spec.title==source.attrib.title)
end
assert(build.treeTab==tab and tab.specList==list and tab.activeSpec==3 and build.spec==active)
for i,row in ipairs(saved) do local actual,zeros=jewels(list[i]);assert(list[i]==row.object and list[i].jewels==row.table and list[i].title==row.title and equal(actual,row.jewels) and equal(zeros,row.zeros)) end
assert(build.itemsTab.activeItemSet==itemSet and build.skillsTab.activeSkillSetId==skills and build.configTab.activeConfigSetId==config and build.mainSocketGroup==group)
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end
result.saved_state_preserved=true
local spec=common.classes.PassiveSpec;local tree=common.classes.TreeTab;local f=t.functions
assert(spec.PassiveSpec==f.constructor_wrapper and spec.Init==f.init and spec.Load==f.load and spec.PostLoad==f.postload and tree.Load==f.tree_load and tree.PostLoad==f.tree_postload)
result.original_functions_preserved=true
return result
"#;
