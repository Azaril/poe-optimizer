//! Exact-function observer mechanics plus supervised complete original-source imports.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/jewel_radius_lifecycle.rs"]
mod original;
use mlua::{Function, Lua, Table, Value};
const OBSERVER: &str = include_str!("support/jewel_radius_lifecycle.lua");
const FIXTURE: &str = r#"
local Item,ItemsTab,TreeTab={},{},{}
latestTreeVersion="new"
common={classes={Item=Item,ItemsTab=ItemsTab,TreeTab=TreeTab}}
data={jewelRadii={old={{label='Small',inner=0,outer=1,innerSquared=0,outerSquared=1}},new={{label='Small',inner=0,outer=2,innerSquared=0,outerSquared=4}}}}
function data.setJewelRadiiGlobally(treeVersion)
    if treeVersion=='fail' then error('original context failure') end
    data.jewelRadius=data.jewelRadii[treeVersion]
    data.maxJewelRadius=treeVersion=='new' and 2 or 1
end
main={}
function main:LoadTree(treeVersion)
    data.setJewelRadiiGlobally(treeVersion)
    return {treeVersion=treeVersion}
end
function Item:ParseRaw(raw)
    self.name=raw;self.jewelRadiusLabel='Small'
    self.jewelRadiusIndex=data.maxJewelRadius
end
function ItemsTab:Load(xml)
    for _,node in ipairs(xml) do
        local item=setmetatable({id=tonumber(node.attrib.id)},{__index=Item})
        item:ParseRaw(node[1])
        self.items[item.id]=item
        self.itemOrderList[#self.itemOrderList+1]=item.id
    end
end
function TreeTab:Load(xml)
    self:SetActiveSpec(xml.attrib.activeSpec)
end
function TreeTab:SetActiveSpec(specId)
    self.activeSpec=specId
    build.spec=self.specList[specId]
    data.setJewelRadiiGlobally(build.spec.treeVersion)
end
local items=setmetatable({items={},itemOrderList={}},{__index=ItemsTab})
local tree=setmetatable({activeSpec=1,specList={{treeVersion='old'}}},{__index=TreeTab})
build={itemsTab=items,treeTab=tree,spec=tree.specList[1]}
data.setJewelRadiiGlobally('old')
local xml={elem='Items',{elem='Item',attrib={id='1'},'first'},{elem='Item',attrib={id='2'},'second'}}
return {
    run=function()
        main:LoadTree('new')
        items:Load(xml)
        tree:Load({elem='Tree',attrib={activeSpec=1}})
    end,
    fail=function()data.setJewelRadiiGlobally('fail')end,
    overflow=function()
        local item=setmetatable({id=9},{__index=Item})
        for i=1,5000 do item:ParseRaw('bounded') end
    end,
    result=function()return build.itemsTab.items[1].jewelRadiusIndex,data.maxJewelRadius end
}
"#;
struct Fixture {
    lua: Lua,
    module: Table,
    source: Table,
}
impl Fixture {
    fn new() -> Self {
        let lua = unsafe { Lua::unsafe_new() };
        let module = lua
            .load(OBSERVER)
            .set_name("@jewel_radius_lifecycle.lua")
            .eval()
            .unwrap();
        let source = lua
            .load(FIXTURE)
            .set_name("@jewel-lifecycle-mechanics.lua")
            .eval()
            .unwrap();
        Self {
            lua,
            module,
            source,
        }
    }
    fn start(&self, observed: bool) -> Table {
        self.module
            .raw_get::<Function>("start")
            .unwrap()
            .call(observed)
            .unwrap()
    }
    fn run(&self) {
        self.source
            .raw_get::<Function>("run")
            .unwrap()
            .call::<()>(())
            .unwrap()
    }
    fn finish(&self, capture: &Table) -> Table {
        capture
            .raw_get::<Function>("finish")
            .unwrap()
            .call(())
            .unwrap()
    }
    fn no_hook(&self) {
        assert!(matches!(
            self.lua
                .load("return debug.gethook()")
                .eval::<Value>()
                .unwrap(),
            Value::Nil
        ));
    }
}
#[test]
fn exact_function_hook_records_requested_context_and_item_xml_identity() {
    let fixture = Fixture::new();
    let capture = fixture.start(true);
    fixture.run();
    let report = fixture.finish(&capture);
    fixture.no_hook();
    assert_eq!(
        report
            .raw_get::<Table>("incomplete_calls")
            .unwrap()
            .raw_len(),
        0
    );
    let events: Table = report.raw_get("events").unwrap();
    let mut calls = Vec::new();
    for i in 1..=events.raw_len() {
        let row: Table = events.raw_get(i).unwrap();
        if row.raw_get::<String>("name").unwrap() == "parse_raw"
            && row.raw_get::<String>("event").unwrap() == "call"
        {
            calls.push(row);
        }
    }
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].raw_get::<u32>("items_xml_token").unwrap(),
        calls[1].raw_get::<u32>("items_xml_token").unwrap()
    );
    assert_ne!(
        calls[0].raw_get::<u32>("item_xml_token").unwrap(),
        calls[1].raw_get::<u32>("item_xml_token").unwrap()
    );
    assert_ne!(
        calls[0].raw_get::<u32>("receiver_token").unwrap(),
        calls[1].raw_get::<u32>("receiver_token").unwrap()
    );
    assert_eq!(
        fixture
            .source
            .raw_get::<Function>("result")
            .unwrap()
            .call::<(i64, i64)>(())
            .unwrap(),
        (2, 1)
    );
    let control = Fixture::new();
    let capture = control.start(false);
    control.run();
    let control_report = control.finish(&capture);
    control.no_hook();
    use mlua::LuaSerdeExt;
    let actual: serde_json::Value = fixture
        .lua
        .from_value(Value::Table(report.raw_get("finite_post_import").unwrap()))
        .unwrap();
    let expected: serde_json::Value = control
        .lua
        .from_value(Value::Table(
            control_report.raw_get("finite_post_import").unwrap(),
        ))
        .unwrap();
    // Maps with integer IDs may traverse differently; the fixed array is canonicalized in full-host driver.
    assert_eq!(actual["item_order"], expected["item_order"]);
    assert_eq!(actual["radii"], expected["radii"]);
}
#[test]
fn source_error_keeps_incomplete_calls_and_removes_hook() {
    let fixture = Fixture::new();
    let capture = fixture.start(true);
    assert!(
        fixture
            .source
            .raw_get::<Function>("fail")
            .unwrap()
            .call::<()>(())
            .is_err()
    );
    let report = fixture.finish(&capture);
    assert!(
        report
            .raw_get::<Table>("incomplete_calls")
            .unwrap()
            .raw_len()
            > 0
    );
    fixture.no_hook();
}
#[test]
fn preexisting_hook_and_rebound_exact_function_are_rejected() {
    let fixture = Fixture::new();
    fixture
        .lua
        .load("debug.sethook(function() end,'c')")
        .exec()
        .unwrap();
    assert!(
        fixture
            .module
            .raw_get::<Function>("start")
            .unwrap()
            .call::<Table>(true)
            .is_err()
    );
    assert!(matches!(
        fixture
            .lua
            .load("return debug.gethook()")
            .eval::<Value>()
            .unwrap(),
        Value::Function(_)
    ));
    fixture.lua.load("debug.sethook() ").exec().unwrap();
    let capture = fixture.start(true);
    fixture
        .lua
        .load("data.setJewelRadiiGlobally=function(treeVersion) return treeVersion end")
        .exec()
        .unwrap();
    assert!(
        capture
            .raw_get::<Function>("finish")
            .unwrap()
            .call::<Table>(())
            .is_err()
    );
    fixture.no_hook();
}
#[test]
fn event_limit_is_sticky_explicit_and_cleanup_is_safe() {
    let fixture = Fixture::new();
    let capture = fixture.start(true);
    let error = fixture
        .source
        .raw_get::<Function>("overflow")
        .unwrap()
        .call::<()>(())
        .unwrap_err()
        .to_string();
    assert!(error.contains("event count bound"), "{error}");
    assert!(
        capture
            .raw_get::<Function>("finish")
            .unwrap()
            .call::<Table>(())
            .is_err()
    );
    fixture.no_hook();
}
#[test]
fn foreign_replacement_hook_is_preserved_when_cleanup_rejects_tampering() {
    for observed in [true, false] {
        let fixture = Fixture::new();
        let capture = fixture.start(observed);
        let foreign: Function = fixture.lua.load("return function() end").eval().unwrap();
        fixture
            .lua
            .globals()
            .raw_get::<Table>("debug")
            .unwrap()
            .raw_get::<Function>("sethook")
            .unwrap()
            .call::<()>((foreign.clone(), "c"))
            .unwrap();
        assert!(
            capture
                .raw_get::<Function>("finish")
                .unwrap()
                .call::<Table>(())
                .is_err()
        );
        assert_eq!(
            fixture
                .lua
                .load("return debug.gethook()")
                .eval::<Function>()
                .unwrap(),
            foreign
        );
        fixture.lua.load("debug.sethook()").exec().unwrap();
        fixture.no_hook();
    }
}
#[test]
fn all_five_original_jewel_radius_import_lifecycles() {
    original::run()
}
