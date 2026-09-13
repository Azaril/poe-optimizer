//! Observer mechanics and complete original Load lifecycle receipts.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/item_set_lifecycle.rs"]
mod original;
use mlua::{Function, Lua, Table, Value};
const OBSERVER: &str = include_str!("support/item_set_lifecycle.lua");
const FIXTURE: &str = r#"
local ItemsTab,Slot,DropDown,Undo,Import={},{},{},{},{}
ItemsTab.__index=ItemsTab;Slot.__index=Slot;DropDown.__index=DropDown
common={classes={ItemsTab=ItemsTab,ItemSlotControl=Slot,DropDownControl=DropDown,UndoHandler=Undo,ImportTab=Import}}
function ItemsTab:CreateItemSet(itemSetId,name)
    local set={id=itemSetId,title=name}
    for name in pairs(self.slots) do set[name]={selItemId=0} end
    for name in pairs(self.runeSlots) do set[name]={runeName='None'} end
    self.itemSets[itemSetId]=set;return set
end
function ItemsTab:IsItemValidForSlot(item,slotName,itemSet,flagState) return item.type==slotName end
function Slot:SetSelItemId(selItemId) self.itemsTab.activeItemSet[self.slotName].selItemId=selItemId;self.selItemId=selItemId end
function Slot:Populate()
    self.items={0};self.list={'None'};self.selIndex=1
    for _,item in pairs(self.itemsTab.items) do
        if self.itemsTab:IsItemValidForSlot(item,self.slotName) then
            self.items[#self.items+1]=item.id;self.list[#self.list+1]=item.name
            if self.selItemId==item.id then self.selIndex=#self.items end
        end
    end
    if self.selIndex==1 then self:SetSelItemId(0) end
end
function ItemsTab:PopulateSlots() for _,slot in pairs(self.slots) do slot:Populate() end end
function DropDown:SelByValue(value,key)
    for index,row in ipairs(self.list) do if row[key]==value then self.selIndex=index;return end end
end
function DropDown:SetSel(newSel,noCallSelFunc)
    if self.selIndex~=newSel then self.selIndex=newSel;if not noCallSelFunc and self.selFunc then self.selFunc() end end
end
function ItemsTab:SetActiveItemSet(itemSetId,deferSync)
    local previous=self.activeItemSet;self.activeItemSet=self.itemSets[itemSetId];self.activeItemSetId=itemSetId
    for name,slot in pairs(self.slots) do
        if previous then previous[name].selItemId=slot.selItemId;previous[name].note=slot.note end
        slot.selItemId=self.activeItemSet[name].selItemId;slot.note=self.activeItemSet[name].note
    end
    for name,rune in pairs(self.runeSlots) do rune:SelByValue(self.activeItemSet[name].runeName,'name') end
    self:PopulateSlots();if not deferSync then self.build:SyncLoadouts() end
end
function ItemsTab:CreateUndoState() return {activeItemSetId=self.activeItemSetId,items=self.items,itemOrderList=self.itemOrderList,itemSets=self.itemSets,itemSetOrderList=self.itemSetOrderList,slotSelItemId={}} end
function Undo:ResetUndo() self.undo={self:CreateUndoState()};self.redo={} end
ItemsTab.ResetUndo=Undo.ResetUndo
function ItemsTab:Load(xml,dbFileName)
    self.itemSets={};self.itemSetOrderList={2};self.tradeQuery.statSortSelectionList={}
    local set=self:CreateItemSet(2,'Incoming');set.Ring.selItemId=1;set.Ring.note='incoming';set.Rune.runeName='unknown'
    self:SetActiveItemSet(2)
    if xml.attrib.fail then error('original fixture failure') end
    self:ResetUndo()
end
build={controls={}}
function Import:RefreshBuildPlannerSets() end
function build:SyncLoadouts(skipBuildPlannerSync)
    if not skipBuildPlannerSync then self.importTab:RefreshBuildPlannerSets() end
    self.controls.buildLoadouts:SetSel(2)
end
function build:SetActiveLoadout(loadout) self.activeLoadout=loadout;self:SyncLoadouts() end
build.importTab=setmetatable({build=build,controls={}},{__index=Import})
build.controls.buildLoadouts=setmetatable({selIndex=1,list={'None','Default'},selFunc=function()build:SetActiveLoadout(2)end},DropDown)
local previous={id=1,title='Previous',Ring={selItemId=0,note='stale'},Rune={runeName='None'}}
local tab=setmetatable({build=build,items={[1]={id=1,type='Ring',name='Ring',rarity='NORMAL'}},itemOrderList={1},
    itemSets={[1]=previous},activeItemSet=previous,activeItemSetId=1,itemSetOrderList={1},slots={},runeSlots={},tradeQuery={},undo={},redo={}},ItemsTab)
build.itemsTab=tab
local slot=setmetatable({itemsTab=tab,slotName='Ring',selItemId=1,note='live previous',items={0},list={'None'},selIndex=1,controls={},jewelSocketList={}},Slot)
tab.slots.Ring=slot
tab.runeSlots.Rune=setmetatable({selIndex=2,list={{name='None'},{name='Retained'}}},DropDown)
return {run=function(fail)tab:Load({elem='Items',attrib={activeItemSet='2',fail=fail}})end,previous=previous}
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
            .set_name("@item_set_lifecycle.lua")
            .eval()
            .unwrap();
        let source = lua
            .load(FIXTURE)
            .set_name("@item-set-mechanics.lua")
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
    fn run(&self, fail: bool) -> mlua::Result<()> {
        self.source.raw_get::<Function>("run").unwrap().call(fail)
    }
    fn finish(&self, capture: &Table) -> mlua::Result<Table> {
        capture.raw_get::<Function>("finish").unwrap().call(())
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
fn whole_load_records_order_context_reentry_and_previous_set_after_return() {
    let f = Fixture::new();
    let c = f.start(true);
    f.run(false).unwrap();
    let report = f.finish(&c).unwrap();
    f.no_hook();
    assert_eq!(
        report
            .raw_get::<Table>("incomplete_calls")
            .unwrap()
            .raw_len(),
        0
    );
    let events = report
        .raw_get::<Table>("events")
        .unwrap()
        .sequence_values::<Table>()
        .collect::<mlua::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        events.first().unwrap().raw_get::<String>("name").unwrap(),
        "items_load"
    );
    assert_eq!(
        events.last().unwrap().raw_get::<String>("name").unwrap(),
        "items_load"
    );
    assert_eq!(
        events.last().unwrap().raw_get::<String>("event").unwrap(),
        "return"
    );
    let mut order = Vec::new();
    let mut sync_calls = 0;
    let mut loadout_calls = 0;
    for row in &events {
        if row.raw_get::<String>("event").unwrap() != "call" {
            continue;
        }
        match row.raw_get::<String>("name").unwrap().as_str() {
            "populate_slot" => {
                assert!(row.raw_get::<bool>("direct_populate_slots_caller").unwrap());
                order.push(row.raw_get::<String>("slot").unwrap());
            }
            "validity" => {
                let context: Table = row.raw_get("context").unwrap();
                assert!(!context.raw_get::<bool>("calcs_tab_present").unwrap());
                assert!(!context.raw_get::<bool>("main_env_present").unwrap());
            }
            "sync_loadouts" => sync_calls += 1,
            "activate_loadout" => loadout_calls += 1,
            _ => {}
        }
    }
    assert_eq!(order, vec!["Ring"]);
    assert_eq!(sync_calls, 2);
    assert_eq!(loadout_calls, 1);
    let states: Table = report.raw_get("states").unwrap();
    let after: Table = states.raw_get(states.raw_len()).unwrap();
    let state: Table = after.raw_get("value").unwrap();
    let previous: Table = state.raw_get("previousActiveItemSet").unwrap();
    assert_eq!(
        previous
            .raw_get::<Table>("Ring")
            .unwrap()
            .raw_get::<String>("note")
            .unwrap(),
        "live previous"
    );
    assert_ne!(previous, state.raw_get::<Table>("activeItemSet").unwrap());
    let rune: Table = state
        .raw_get::<Table>("runeSlots")
        .unwrap()
        .raw_get("Rune")
        .unwrap();
    assert_eq!(
        rune.raw_get::<usize>("selIndex").unwrap(),
        2,
        "unknown rune name preserves selection"
    );
}
#[test]
fn source_error_retains_incomplete_load_and_removes_hook() {
    let f = Fixture::new();
    let c = f.start(true);
    assert!(f.run(true).is_err());
    let report = f.finish(&c).unwrap();
    assert!(
        report
            .raw_get::<Table>("incomplete_calls")
            .unwrap()
            .raw_len()
            > 0
    );
    f.no_hook();
}
#[test]
fn rejects_foreign_hooks_and_changed_original_functions() {
    let f = Fixture::new();
    f.lua
        .load("foreign=function()end;debug.sethook(foreign,'c')")
        .exec()
        .unwrap();
    assert!(
        f.module
            .raw_get::<Function>("start")
            .unwrap()
            .call::<Table>(true)
            .is_err()
    );
    f.lua.load("debug.sethook()").exec().unwrap();
    let c = f.start(true);
    f.lua.load("debug.sethook(foreign,'c')").exec().unwrap();
    assert!(f.finish(&c).is_err());
    assert_eq!(
        f.lua
            .load("return debug.gethook()")
            .eval::<Function>()
            .unwrap(),
        f.lua.globals().raw_get::<Function>("foreign").unwrap()
    );
    f.lua.load("debug.sethook()").exec().unwrap();
    let f = Fixture::new();
    let c = f.start(true);
    f.lua
        .load("common.classes.ItemsTab.Load=function()end")
        .exec()
        .unwrap();
    assert!(f.finish(&c).is_err());
    f.no_hook();
}
#[test]
fn snapshot_limits_fail_without_reporting_complete_load() {
    let f = Fixture::new();
    let c = f.start(true);
    f.lua
        .load("build.itemsTab.items[1].name=string.rep('x',262145)")
        .exec()
        .unwrap();
    assert!(f.run(false).is_err());
    assert!(f.finish(&c).is_err());
    f.no_hook();
}
#[test]
fn no_hook_control_preserves_fixture_outputs() {
    let f = Fixture::new();
    let c = f.start(false);
    f.run(false).unwrap();
    let report = f.finish(&c).unwrap();
    f.no_hook();
    assert_eq!(report.raw_get::<Table>("events").unwrap().raw_len(), 0);
    let state: Table = report.raw_get("finite_post_import").unwrap();
    assert_eq!(state.raw_get::<i64>("activeItemSetId").unwrap(), 2);
    assert_eq!(state.raw_get::<Table>("items").unwrap().raw_len(), 1);
}
#[test]
fn rune_key_compares_selected_names_and_duplicate_counts_without_order() {
    let lua = Lua::new();
    let choices = |names: &[&str], selected: usize| {
        let list = lua.create_table().unwrap();
        for (index, name) in names.iter().enumerate() {
            let row = lua.create_table().unwrap();
            row.raw_set("name", *name).unwrap();
            list.raw_set(index + 1, row).unwrap();
        }
        let rune = lua.create_table().unwrap();
        rune.raw_set("list", list).unwrap();
        rune.raw_set("selIndex", selected).unwrap();
        let runes = lua.create_table().unwrap();
        runes.raw_set("Helmet Rune #1", rune).unwrap();
        runes
    };
    let initial = choices(&["None", "Wings", "Wings", "Horns"], 2);
    let reordered = choices(&["Wings", "Horns", "None", "Wings"], 4);
    let expected = original::rune_control_key(&initial);
    assert_eq!(expected, original::rune_control_key(&reordered));
    assert_eq!(expected["Helmet Rune #1"]["name_occurrences"]["Wings"], 2);
    // The source arrays/indices remain distinct; equality is only this key.
    assert_eq!(
        initial
            .raw_get::<Table>("Helmet Rune #1")
            .unwrap()
            .raw_get::<usize>("selIndex")
            .unwrap(),
        2
    );
    assert_eq!(
        reordered
            .raw_get::<Table>("Helmet Rune #1")
            .unwrap()
            .raw_get::<usize>("selIndex")
            .unwrap(),
        4
    );
    assert_ne!(
        expected,
        original::rune_control_key(&choices(&["Wings", "Horns", "None", "Wings"], 2))
    );
    assert_ne!(
        expected,
        original::rune_control_key(&choices(&["Wings", "Horns", "None", "Horns"], 1))
    );
}

#[test]
fn all_five_complete_original_item_set_load_lifecycles() {
    original::run();
}
