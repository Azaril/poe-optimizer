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

#[test]
fn default_mode_keeps_sync_calls_outside_load_unobserved() {
    let f = Fixture::new();
    let options = f.lua.create_table().unwrap();
    options.raw_set("loadouts", false).unwrap();
    let capture: Table = f
        .module
        .raw_get::<Function>("start")
        .unwrap()
        .call((true, options))
        .unwrap();
    f.lua.load("build:SyncLoadouts(true)").exec().unwrap();
    let report = f.finish(&capture).unwrap();
    f.no_hook();
    assert_eq!(report.raw_get::<Table>("events").unwrap().raw_len(), 0);
    assert!(matches!(
        report.raw_get::<Value>("loadout_states").unwrap(),
        Value::Nil
    ));
    assert!(matches!(
        report.raw_get::<Value>("finite_post_loadouts").unwrap(),
        Value::Nil
    ));
}

#[test]
fn malformed_loadout_observation_options_do_not_install_a_hook() {
    for use_number in [true, false] {
        let f = Fixture::new();
        let options = f.lua.create_table().unwrap();
        let value = if use_number {
            Value::Integer(1)
        } else {
            Value::String(f.lua.create_string("yes").unwrap())
        };
        options.raw_set("loadouts", value).unwrap();
        let error = f
            .module
            .raw_get::<Function>("start")
            .unwrap()
            .call::<Table>((true, options))
            .unwrap_err();
        assert!(error.to_string().contains("loadouts"));
        f.no_hook();
    }
}

#[test]
fn all_five_original_loadout_sync_and_lookup_histories() {
    original::run_loadouts();
}

// Synthetic mechanics fixture only. Complete game behavior is checked through
// retained original Functions in the separate all-five source lane.
const LOADOUT_FIXTURE: &str = r#"
local Import=common.classes.ImportTab
Import.__index=Import;setmetatable(build.importTab,Import)
local Tree,Skills,Config={},{},{}
Tree.__index=Tree;Skills.__index=Skills;Config.__index=Config
common.classes.TreeTab=Tree;common.classes.SkillsTab=Skills;common.classes.ConfigTab=Config
function Tree:GetSpecList()
    local out={};for i,spec in ipairs(self.specList) do out[i]=spec.title end;return out
end
function Tree:SetActiveSpec(specId,deferSync)
    self.activeSpec=specId;self.build.spec=self.specList[specId]
end
function Skills:SetActiveSkillSet(skillSetId,deferSync)
    self.activeSkillSetId=skillSetId;self.socketGroupList=self.skillSets[skillSetId].socketGroupList
end
function Config:SetActiveConfigSet(configSetId,init,deferSync)
    self.activeConfigSetId=configSetId;self.input=self.configSets[configSetId].input;self.placeholder=self.configSets[configSetId].placeholder
end
latestTreeVersion='fixture'
treeVersions={fixture={display='Fixture'}}
local spec={title='Default',treeVersion='fixture',jewels={}}
build.treeTab=setmetatable({build=build,activeSpec=1,specList={spec}},Tree)
build.spec=spec
local groups={}
build.skillsTab=setmetatable({build=build,activeSkillSetId=1,skillSetOrderList={1},skillSets={[1]={title='Default',socketGroupList=groups}},socketGroupList=groups},Skills)
local input,placeholder={},{}
build.configTab=setmetatable({build=build,activeConfigSetId=1,configSetOrderList={1},configSets={[1]={title='Default',input=input,placeholder=placeholder}},input=input,placeholder=placeholder},Config)
build.itemsTab.itemSets[1].title='Default'
build.loadoutsList={spec};build.treeListSpecialLinks={};build.itemListSpecialLinks={};build.skillListSpecialLinks={};build.configListSpecialLinks={}
build.controls.buildLoadouts.list={'Header','Default'}
build.controls.buildLoadouts.searchTerm=''
function build:GetLoadoutByName(loadoutName)
    self.treeTab:GetSpecList()
    if loadoutName=='failure' then error('synthetic loadout source failure') end
    if loadoutName=='missing' then return nil end
    return {specId=1,itemSetId=1,skillSetId=1,configSetId=1}
end
function build:SetActiveLoadout(loadout)
    if not loadout or not loadout.specId then return end
    self.treeTab:SetActiveSpec(loadout.specId,true)
    self.skillsTab:SetActiveSkillSet(loadout.skillSetId,true)
    self.configTab:SetActiveConfigSet(loadout.configSetId,false,true)
    self:SyncLoadouts(true)
end
function build:SyncLoadouts(skipBuildPlannerSync)
    self.loadoutsList={self.treeTab.specList[1]}
    self.treeListSpecialLinks={};self.itemListSpecialLinks={};self.skillListSpecialLinks={};self.configListSpecialLinks={}
    if not skipBuildPlannerSync then self.importTab:RefreshBuildPlannerSets() end
    self.controls.buildLoadouts:SetSel(2)
    self.activeLoadout=1
    return {'Default'},{Default=true},{Default=true},{Default=true}
end
local self=build
build.controls.buildLoadouts.selFunc=function(index,value)
    self:SetActiveLoadout(self:GetLoadoutByName('Default'))
end
"#;

impl Fixture {
    fn loadouts() -> Self {
        let fixture = Self::new();
        fixture
            .lua
            .load(LOADOUT_FIXTURE)
            .set_name("@loadout-mechanics.lua")
            .exec()
            .unwrap();
        fixture
    }
    fn start_loadouts(&self, observed: bool) -> Table {
        let options = self.lua.create_table().unwrap();
        options.raw_set("loadouts", true).unwrap();
        self.module
            .raw_get::<Function>("start")
            .unwrap()
            .call((observed, options))
            .unwrap()
    }
}

#[test]
fn opt_in_records_sync_and_lookup_roots_without_inventing_a_load_parent() {
    let f = Fixture::loadouts();
    let capture = f.start_loadouts(true);
    f.lua
        .load("build:SyncLoadouts(true);build:GetLoadoutByName('missing')")
        .exec()
        .unwrap();
    let report = f.finish(&capture).unwrap();
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
    let mut roots = Vec::new();
    let mut saw_activation = false;
    for event in events {
        assert!(matches!(
            event.raw_get::<Value>("load_call_ordinal").unwrap(),
            Value::Nil
        ));
        if event.raw_get::<String>("event").unwrap() != "call" {
            continue;
        }
        let name = event.raw_get::<String>("name").unwrap();
        saw_activation |= name == "activate_loadout";
        if matches!(
            event.raw_get::<Value>("parent_call_ordinal").unwrap(),
            Value::Nil
        ) {
            roots.push(name);
        }
    }
    assert_eq!(roots, vec!["sync_loadouts", "lookup_loadout"]);
    assert!(
        saw_activation,
        "changed-index callback must remain a nested source call"
    );
    assert!(report.raw_get::<Table>("loadout_states").unwrap().raw_len() >= 4);
}

#[test]
fn loadout_source_failure_retains_incomplete_root_and_cleans_up() {
    let f = Fixture::loadouts();
    let capture = f.start_loadouts(true);
    let error = f
        .lua
        .load("build:GetLoadoutByName('failure')")
        .exec()
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("synthetic loadout source failure")
    );
    let report = f.finish(&capture).unwrap();
    f.no_hook();
    let incomplete = report.raw_get::<Table>("incomplete_calls").unwrap();
    assert_eq!(incomplete.raw_len(), 1);
    assert_eq!(
        incomplete
            .raw_get::<Table>(1)
            .unwrap()
            .raw_get::<String>("name")
            .unwrap(),
        "lookup_loadout"
    );
}

#[test]
fn direct_snapshot_preserves_nil_pack_arity_and_shared_spec_aliases() {
    let f = Fixture::loadouts();
    let capture = f.start_loadouts(false);
    let values: mlua::MultiValue = f
        .lua
        .load("return build:GetLoadoutByName('missing')")
        .eval()
        .unwrap();
    assert_eq!(values.len(), 1);
    let pack = f.lua.create_table().unwrap();
    pack.raw_set("n", values.len()).unwrap();
    for (index, value) in values.into_iter().enumerate() {
        pack.raw_set(index + 1, value).unwrap();
    }
    let snapshot: Table = capture
        .raw_get::<Function>("loadout_snapshot")
        .unwrap()
        .call(pack)
        .unwrap();
    let result: Table = snapshot.raw_get("result_pack").unwrap();
    assert_eq!(result.raw_get::<usize>("n").unwrap(), 1);
    assert!(matches!(result.raw_get::<Value>(1).unwrap(), Value::Nil));

    let pack: Table = f
        .lua
        .load("return {n=3,[1]=build.spec,[2]=build.spec}")
        .eval()
        .unwrap();
    let snapshot: Table = capture
        .raw_get::<Function>("loadout_snapshot")
        .unwrap()
        .call(pack)
        .unwrap();
    let result: Table = snapshot.raw_get("result_pack").unwrap();
    assert_eq!(result.raw_get::<usize>("n").unwrap(), 3);
    let first: Table = result.raw_get(1).unwrap();
    assert_eq!(first, result.raw_get::<Table>(2).unwrap());
    assert_eq!(
        first,
        snapshot
            .raw_get::<Table>("tree")
            .unwrap()
            .raw_get::<Table>("specList")
            .unwrap()
            .raw_get::<Table>(1)
            .unwrap()
    );
    assert_eq!(
        first,
        snapshot
            .raw_get::<Table>("build")
            .unwrap()
            .raw_get::<Table>("loadoutsList")
            .unwrap()
            .raw_get::<Table>(1)
            .unwrap()
    );
    f.finish(&capture).unwrap();
    f.no_hook();
}

#[test]
fn loadout_control_replacement_between_completed_roots_is_allowed() {
    let f = Fixture::loadouts();
    f.lua.load(r#"
        function fixtureNewLoadoutControl()
            local self=build
            local control=setmetatable({selIndex=1,list={'Header','Default'},searchTerm=''},common.classes.DropDownControl)
            control.selFunc=function(index,value)self:SetActiveLoadout(self:GetLoadoutByName('Default'))end
            return control
        end
        build.controls.buildLoadouts=fixtureNewLoadoutControl()
    "#).exec().unwrap();
    let capture = f.start_loadouts(true);
    f.lua.load("build:SyncLoadouts(true)").exec().unwrap();
    f.lua
        .load("build.controls.buildLoadouts=fixtureNewLoadoutControl()")
        .exec()
        .unwrap();
    f.lua.load("build:SyncLoadouts(true)").exec().unwrap();
    let report = f.finish(&capture).unwrap();
    f.no_hook();
    assert_eq!(
        report
            .raw_get::<Table>("incomplete_calls")
            .unwrap()
            .raw_len(),
        0
    );
    let root_count = report
        .raw_get::<Table>("events")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|row| {
            row.raw_get::<String>("event").unwrap() == "call"
                && row.raw_get::<String>("name").unwrap() == "sync_loadouts"
                && matches!(
                    row.raw_get::<Value>("parent_call_ordinal").unwrap(),
                    Value::Nil
                )
        })
        .count();
    assert_eq!(root_count, 2);
}

#[test]
fn loadout_callback_replacement_during_a_root_is_rejected_and_cleaned_up() {
    let f = Fixture::loadouts();
    f.lua
        .load(
            r#"
        local self=build
        self.controls.buildLoadouts.selFunc=function(index,value)
            self.controls.buildLoadouts.selFunc=function()end
        end
    "#,
        )
        .exec()
        .unwrap();
    let capture = f.start_loadouts(true);
    let called = f.lua.load("build:SyncLoadouts(true)").exec();
    let finished = f.finish(&capture);
    f.no_hook();
    assert!(called.is_err() || finished.is_err());
    let error = finished.err().or_else(|| called.err()).unwrap();
    assert!(
        error.to_string().contains("callback"),
        "unexpected observer failure: {error}"
    );
}

#[test]
fn missing_startup_domains_are_explicit_without_a_fabricated_item_snapshot() {
    let f = Fixture::loadouts();
    f.lua.load("build.treeTab=nil;build.itemsTab=nil;build.skillsTab=nil;build.configTab=nil;build.importTab=nil;build.spec=nil;build.loadoutsList={}")
        .exec().unwrap();
    let capture = f.start_loadouts(false);
    let snapshot: Table = capture
        .raw_get::<Function>("loadout_snapshot")
        .unwrap()
        .call(())
        .unwrap();
    let presence: Table = snapshot
        .raw_get::<Table>("identity")
        .unwrap()
        .raw_get("domain_presence")
        .unwrap();
    for key in ["tree", "items", "skills", "config", "export"] {
        assert!(!presence.raw_get::<bool>(key).unwrap());
        assert!(matches!(
            snapshot.raw_get::<Value>(key).unwrap(),
            Value::Nil
        ));
    }
    let report = f.finish(&capture).unwrap();
    f.no_hook();
    assert!(
        !report
            .raw_get::<Table>("scope")
            .unwrap()
            .raw_get::<bool>("finite_post_import_available")
            .unwrap()
    );
    assert!(matches!(
        report.raw_get::<Value>("finite_post_import").unwrap(),
        Value::Nil
    ));
}

#[test]
fn loadout_build_preserves_the_original_control_host_class_boundary() {
    for (mutation, expected) in [
        (
            "setmetatable(build,{__index=common.classes.ControlHost})",
            "loadout projected class lookup for loadoutsList",
        ),
        (
            "common.classes.ControlHost={}",
            "original loadout classes changed",
        ),
        (
            "common.classes.ControlHost.loadoutsList={};build.loadoutsList=nil",
            "inherited loadout data field",
        ),
    ] {
        let f = Fixture::loadouts();
        f.lua.load("local class={};class.__index=class;common.classes.ControlHost=class;setmetatable(build,class)").exec().unwrap();
        let capture = f.start_loadouts(true);
        let snapshot = capture.raw_get::<Function>("loadout_snapshot").unwrap();
        let before: Table = snapshot.call(()).unwrap();
        assert_eq!(
            before
                .raw_get::<Table>("build")
                .unwrap()
                .raw_get::<Table>("loadoutsList")
                .unwrap()
                .raw_len(),
            1
        );
        f.lua.load(mutation).exec().unwrap();
        let error = snapshot.call::<Table>(()).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert!(f.finish(&capture).is_err());
        f.no_hook();
    }
}

#[test]
fn loadout_snapshot_preserves_replaced_specs_and_rejects_foreign_owners() {
    for mutation in [
        "setmetatable(build.loadoutsList[1],{__index=common.classes.PassiveSpec})",
        "build.loadoutsList[1].build={}",
    ] {
        let f = Fixture::loadouts();
        f.lua.load(r#"
local class={_className='PassiveSpec'};class.__index=class
common.classes.PassiveSpec=class
local old=build.spec;old.build=build;setmetatable(old,class)
local current=setmetatable({build=build,title=old.title,treeVersion=old.treeVersion,jewels={}},class)
build.spec=current;build.treeTab.specList={current};build.loadoutsList={old,old,current}
"#).exec().unwrap();
        let capture = f.start_loadouts(true);
        let snapshot = capture.raw_get::<Function>("loadout_snapshot").unwrap();
        let before: Table = snapshot.call(()).unwrap();
        let projected_build: Table = before.raw_get("build").unwrap();
        let rows: Table = projected_build.raw_get("loadoutsList").unwrap();
        let old: Table = rows.raw_get(1).unwrap();
        let current: Table = rows.raw_get(3).unwrap();
        assert_eq!(old, rows.raw_get::<Table>(2).unwrap());
        assert_ne!(
            old, current,
            "equal-valued old and current specs stay distinct"
        );
        assert_eq!(current, projected_build.raw_get::<Table>("spec").unwrap());
        assert_eq!(
            current,
            before
                .raw_get::<Table>("tree")
                .unwrap()
                .raw_get::<Table>("specList")
                .unwrap()
                .raw_get::<Table>(1)
                .unwrap()
        );
        let title: String = old.raw_get("title").unwrap();
        f.lua
            .load("build.loadoutsList[1].title='Retained old spec'")
            .exec()
            .unwrap();
        let after: Table = snapshot.call(()).unwrap();
        assert_eq!(old.raw_get::<String>("title").unwrap(), title);
        assert_eq!(
            after
                .raw_get::<Table>("build")
                .unwrap()
                .raw_get::<Table>("loadoutsList")
                .unwrap()
                .raw_get::<Table>(1)
                .unwrap()
                .raw_get::<String>("title")
                .unwrap(),
            "Retained old spec"
        );
        f.lua.load(mutation).exec().unwrap();
        let error = snapshot.call::<Table>(()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("actual PassiveSpec class and owner"),
            "{error}"
        );
        assert!(f.finish(&capture).is_err());
        f.no_hook();
    }
}
