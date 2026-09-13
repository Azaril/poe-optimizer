//! Complete finite accessory/armour/flask/charm/weapon assembly on actual original inputs, with separate
//! source-observer mechanics tests. No complete native-build claim.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table, Value};
const OBSERVER: &str = include_str!("support/item_assembly_source.lua");
const MECHANICS: &str = r#"
local Item, ItemsTab, ModList = {}, {}, {}
common = { classes = { Item = Item, ItemsTab = ItemsTab, ModList = ModList } }
function Item:ParseRaw() self:BuildModList() end
function Item:BuildModList()
    if self.empty then return end
    -- Like original BuildModList, a later local closure captures self even
    -- when the early return is taken before that closure is instantiated.
    local function payload() return self.payload end
    self.baseModList = setmetatable({ payload(), self.payload }, ModList)
    if self.fail then error('original assembly failure') end
    self.slotModList = { { self.payload }, { self.payload }, { self.payload } }
end
function ItemsTab:Load(xml)
    for _, node in ipairs(xml) do
        local item = node.item
        item:BuildModList()
        self.items[item.id] = item
    end
end
local function item(fail)
    return setmetatable({ id = 7, payload = { name = 'same', value = 13 }, fail = fail }, { __index = Item })
end
return {
    item = item,
    load = function(value)
        local node = { elem = 'Item', item = value }
        local tab = setmetatable({ items = {} }, { __index = ItemsTab })
        tab:Load({ node })
        return tab, node
    end,
    build = Item.BuildModList,
}
"#;
struct Mechanics {
    lua: Lua,
    observer: Table,
    source: Table,
}
impl Mechanics {
    fn new() -> Self {
        // Only static local mechanics fixtures execute in this isolated test host.
        let lua = unsafe { Lua::unsafe_new() };
        let observer = lua
            .load(OBSERVER)
            .set_name("@item_assembly_source.lua")
            .eval()
            .unwrap();
        let source = lua
            .load(MECHANICS)
            .set_name("@item-assembly-observer-mechanics.lua")
            .eval()
            .unwrap();
        Self {
            lua,
            observer,
            source,
        }
    }
    fn start(&self) -> Table {
        let fields = self
            .lua
            .create_sequence_from(["payload", "baseModList", "slotModList", "fail"])
            .unwrap();
        self.observer
            .raw_get::<Function>("start")
            .unwrap()
            .call(fields)
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
fn source_assembly_observer_keeps_joint_aliases_exact_receiver_and_load_node() {
    let fixture = Mechanics::new();
    let capture = fixture.start();
    let item: Table = fixture
        .source
        .raw_get::<Function>("item")
        .unwrap()
        .call(false)
        .unwrap();
    let (_tab, node): (Table, Table) = fixture
        .source
        .raw_get::<Function>("load")
        .unwrap()
        .call(item.clone())
        .unwrap();
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    fixture.no_hook();
    let events: Table = report.raw_get("events").unwrap();
    assert_eq!(events.raw_len(), 1);
    let event: Table = events.raw_get(1).unwrap();
    assert!(event.raw_get::<bool>("completed").unwrap());
    assert_eq!(event.raw_get::<String>("phase").unwrap(), "final_load");
    assert!(event.raw_get::<bool>("load_receiver_matches").unwrap());
    assert_eq!(
        capture
            .raw_get::<Function>("item")
            .unwrap()
            .call::<Table>(event.raw_get::<u32>("item_token").unwrap())
            .unwrap(),
        item
    );
    assert_eq!(
        capture
            .raw_get::<Function>("node")
            .unwrap()
            .call::<Table>(event.raw_get::<u32>("parent_node_token").unwrap())
            .unwrap(),
        node
    );
    let before: Table = event
        .raw_get::<Table>("before")
        .unwrap()
        .raw_get("root")
        .unwrap();
    let after: Table = event
        .raw_get::<Table>("after")
        .unwrap()
        .raw_get("root")
        .unwrap();
    assert!(matches!(
        before.raw_get::<Value>("baseModList").unwrap(),
        Value::Nil
    ));
    let payload: Table = after.raw_get("payload").unwrap();
    let list: Table = after.raw_get("baseModList").unwrap();
    assert_eq!(list.raw_get::<Table>(1).unwrap(), payload);
    assert_eq!(list.raw_get::<Table>(2).unwrap(), payload);
    assert_ne!(payload, item.raw_get::<Table>("payload").unwrap());
    assert_eq!(after.raw_get::<Table>("slotModList").unwrap().raw_len(), 3);
    assert_eq!(
        capture
            .raw_get::<Table>("methods")
            .unwrap()
            .raw_get::<Function>("build")
            .unwrap(),
        fixture.source.raw_get::<Function>("build").unwrap()
    );
}
#[test]
fn incomplete_original_assembly_keeps_prefix_without_claiming_a_completed_return() {
    let fixture = Mechanics::new();
    let capture = fixture.start();
    let item: Table = fixture
        .source
        .raw_get::<Function>("item")
        .unwrap()
        .call(true)
        .unwrap();
    let error = fixture
        .source
        .raw_get::<Function>("load")
        .unwrap()
        .call::<Table>(item)
        .unwrap_err();
    assert!(error.to_string().contains("original assembly failure"));
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    fixture.no_hook();
    let event: Table = report
        .raw_get::<Table>("events")
        .unwrap()
        .raw_get(1)
        .unwrap();
    assert!(!event.raw_get::<bool>("completed").unwrap());
    assert_eq!(
        event.raw_get::<String>("after_scope").unwrap(),
        "observation_end_after_incomplete_call"
    );
    let after: Table = event
        .raw_get::<Table>("after")
        .unwrap()
        .raw_get("root")
        .unwrap();
    assert_eq!(after.raw_get::<Table>("baseModList").unwrap().raw_len(), 2);
    assert!(matches!(
        after.raw_get::<Value>("slotModList").unwrap(),
        Value::Nil
    ));
}
#[test]
fn source_observer_rejects_existing_hook_without_removing_it() {
    let fixture = Mechanics::new();
    let prior: Function = fixture
        .lua
        .load("local hook=function() end; debug.sethook(hook,'c'); return hook")
        .eval()
        .unwrap();
    let fields = fixture.lua.create_sequence_from(["payload"]).unwrap();
    let error = fixture
        .observer
        .raw_get::<Function>("start")
        .unwrap()
        .call::<Table>(fields)
        .unwrap_err();
    assert!(error.to_string().contains("pre-existing hook"));
    assert_eq!(
        fixture
            .lua
            .load("return debug.gethook()")
            .eval::<Function>()
            .unwrap(),
        prior
    );
    fixture.lua.load("debug.sethook()").exec().unwrap();
}

#[path = "support/item_assembly_corpus.rs"]
mod corpus;
#[test]
fn all_five_actual_finite_items_match_owned_native_assembly() {
    corpus::run();
}

#[test]
fn control_mode_installs_no_hook_and_keeps_original_method_output() {
    let fixture = Mechanics::new();
    let fields = fixture
        .lua
        .create_sequence_from(["payload", "baseModList", "slotModList"])
        .unwrap();
    let capture: Table = fixture
        .observer
        .raw_get::<Function>("start")
        .unwrap()
        .call((fields, false))
        .unwrap();
    fixture.no_hook();
    let item: Table = fixture
        .source
        .raw_get::<Function>("item")
        .unwrap()
        .call(false)
        .unwrap();
    fixture
        .source
        .raw_get::<Function>("load")
        .unwrap()
        .call::<(Table, Table)>(item.clone())
        .unwrap();
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    assert_eq!(report.raw_get::<Table>("events").unwrap().raw_len(), 0);
    assert!(
        !report
            .raw_get::<Table>("scope")
            .unwrap()
            .raw_get::<bool>("call_hook_installed")
            .unwrap()
    );
    let snapshot: Table = capture
        .raw_get::<Function>("snapshot")
        .unwrap()
        .call(item)
        .unwrap();
    let root: Table = snapshot.raw_get("root").unwrap();
    let payload: Table = root.raw_get("payload").unwrap();
    let mods: Table = root.raw_get("baseModList").unwrap();
    assert_eq!(mods.raw_get::<Table>(1).unwrap(), payload);
    assert_eq!(mods.raw_get::<Table>(2).unwrap(), payload);
    assert_eq!(root.raw_get::<Table>("slotModList").unwrap().raw_len(), 3);
    fixture.no_hook();
}

#[test]
fn early_original_return_matches_retained_receiver_when_local_name_is_temporary() {
    let fixture = Mechanics::new();
    let item: Table = fixture
        .source
        .raw_get::<Function>("item")
        .unwrap()
        .call(false)
        .unwrap();
    item.raw_set("empty", true).unwrap();
    let capture = fixture.start();
    fixture
        .source
        .raw_get::<Function>("build")
        .unwrap()
        .call::<()>(item.clone())
        .unwrap();
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    fixture.no_hook();
    let events: Table = report.raw_get("events").unwrap();
    assert_eq!(events.raw_len(), 1);
    let event: Table = events.raw_get(1).unwrap();
    assert!(event.raw_get::<bool>("completed").unwrap());
    assert_eq!(
        event
            .raw_get::<String>("return_receiver_declaration")
            .unwrap(),
        "(*temporary)"
    );
    assert_eq!(
        capture
            .raw_get::<Function>("item")
            .unwrap()
            .call::<Table>(event.raw_get::<u32>("item_token").unwrap())
            .unwrap(),
        item
    );
    let after: Table = event
        .raw_get::<Table>("after")
        .unwrap()
        .raw_get("root")
        .unwrap();
    assert!(matches!(
        after.raw_get::<Value>("baseModList").unwrap(),
        Value::Nil
    ));
    assert!(matches!(
        after.raw_get::<Value>("slotModList").unwrap(),
        Value::Nil
    ));
}
