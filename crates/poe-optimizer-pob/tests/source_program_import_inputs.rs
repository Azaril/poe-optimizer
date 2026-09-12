//! Helper-only synthetic-host tests. Full original item processing is covered by
//! the separate all-five integration; these cases test observer mechanics only.
use mlua::{Function, Lua, MultiValue, Table, Value};
const OBSERVER: &str = include_str!("support/source_program_import_inputs.lua");
const FIXTURE: &str = r#"
local cache, calls, equality_calls = {}, 0, 0
modLib = { parseModCache = cache }
modLib.parseMod = function(line, isComb)
    calls = calls + 1
    cache[line] = cache[line] or { line, isComb }
    return line, isComb
end
local Item = {}
common = { classes = { Item = Item } }
local function getRangedModList(item, modLine)
    local line, combined = modLib.parseMod(modLine.line, true)
    return line, combined
end
function Item:BuildModList()
    for _, modLine in ipairs(self.explicitModLines) do
        getRangedModList(self, modLine)
    end
end
function Item:ParseRaw(text)
    local l = 1
    local modLine = { line = text }
    self.explicitModLines = { modLine }
    modLib.parseMod(text)
    modLib.parseMod(text, true)
    self:BuildModList()
    return l
end
function Item:Outer(other, text)
    local l = 91
    local modLine = { line = 'outer-only' }
    self.explicitModLines = { modLine }
    other:Inner(text)
    return l, modLine
end
function Item:Inner(text)
    modLib.parseMod(text)
end
function Item:OuterRanged(other, text)
    local l = 92
    local modLine = { line = 'another-outer-line' }
    local actual = { line = text }
    other.explicitModLines = { actual }
    getRangedModList(other, actual)
    return l, modLine
end
local function nested_import(text)
    local identity_trap = { __index = Item, __eq = function()
        equality_calls = equality_calls + 1
        return true
    end }
    local first = setmetatable({ id = 7 }, identity_trap)
    local second = setmetatable({ id = 8 }, identity_trap)
    first:Outer(second, text)
    first:OuterRanged(second, text)
    build = { itemsTab = { items = { [7] = first, [8] = second }, itemSets = {} } }
end
local function import_fixture(text)
    local first = setmetatable({ id = 7 }, { __index = Item })
    local second = setmetatable({ id = 8 }, { __index = Item })
    first:ParseRaw(text)
    second:ParseRaw(text)
    local selected = { id = 1, Ring1 = { selItemId = 7, active = true },
        Ring2 = { selItemId = 7, active = false } }
    local other = { id = 2, Helmet = { selItemId = 8, active = false } }
    local identity_trap = { __eq = function()
        equality_calls = equality_calls + 1
        return true
    end }
    setmetatable(selected, identity_trap)
    setmetatable(other, identity_trap)
    build = { itemsTab = { items = { [7] = first, [8] = second },
        itemSets = { selected, other }, activeItemSet = selected }, mainOutput = { marker = true } }
    modLib.parseMod('outside-item', false)
end
local function transient_import(text)
    local item = setmetatable({ id = 9 }, { __index = Item })
    item:ParseRaw(text)
    build = { itemsTab = { items = {}, itemSets = {} } }
end
return { import = import_fixture, nested_import = nested_import, transient_import = transient_import,
    equality_calls = function() return equality_calls end, result = function()
    return calls, build.itemsTab.items[7].explicitModLines[1].line, build.mainOutput.marker
end }
"#;
struct Fixture {
    lua: Lua,
    observer: Table,
    source: Table,
}
impl Fixture {
    fn new() -> Self {
        // Only the static observer/unit fixture executes in this isolated host.
        let lua = unsafe { Lua::unsafe_new() };
        let observer = lua
            .load(OBSERVER)
            .set_name("@source_program_import_inputs.lua")
            .eval()
            .unwrap();
        let source = lua
            .load(FIXTURE)
            .set_name("@import-input-unit-source.lua")
            .eval()
            .unwrap();
        Self {
            lua,
            observer,
            source,
        }
    }
    fn start(&self) -> Table {
        self.observer
            .raw_get::<Function>("start")
            .unwrap()
            .call(())
            .unwrap()
    }
    fn import(&self, bytes: &[u8]) {
        self.source
            .raw_get::<Function>("import")
            .unwrap()
            .call::<()>(self.lua.create_string(bytes).unwrap())
            .unwrap();
    }
    fn result(&self) -> (i64, Vec<u8>, bool) {
        let (calls, text, marker): (i64, mlua::LuaString, bool) = self
            .source
            .raw_get::<Function>("result")
            .unwrap()
            .call(())
            .unwrap();
        (calls, text.as_bytes().to_vec(), marker)
    }
    fn no_identity_callbacks(&self) {
        assert_eq!(
            self.source
                .raw_get::<Function>("equality_calls")
                .unwrap()
                .call::<usize>(())
                .unwrap(),
            0
        );
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
fn original_call_order_raw_bytes_duplicate_items_and_all_slot_memberships_are_preserved() {
    let baseline = Fixture::new();
    let original = b"same\0\xff input";
    baseline.import(original);
    let fixture = Fixture::new();
    let parser: Function = fixture
        .lua
        .globals()
        .raw_get::<Table>("modLib")
        .unwrap()
        .raw_get("parseMod")
        .unwrap();
    let capture = fixture.start();
    fixture.import(original);
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    fixture.no_hook();
    assert_eq!(fixture.result(), baseline.result());
    assert_eq!(
        capture
            .raw_get::<Function>("parser")
            .unwrap()
            .call::<Function>(())
            .unwrap(),
        parser
    );
    let events: Table = report.raw_get("events").unwrap();
    assert_eq!(events.raw_len(), 7);
    for i in 1..=7 {
        let event: Table = events.raw_get(i).unwrap();
        assert_eq!(event.raw_get::<usize>("ordinal").unwrap(), i);
        let line: Table = event.raw_get("line").unwrap();
        assert_eq!(line.raw_get::<String>("kind").unwrap(), "string");
        let expected: &[u8] = if i == 7 { b"outside-item" } else { original };
        assert_eq!(
            line.raw_get::<mlua::LuaString>("value")
                .unwrap()
                .as_bytes()
                .as_ref(),
            expected
        );
        let combined: Table = event.raw_get("combined").unwrap();
        assert_eq!(
            combined.raw_get::<String>("kind").unwrap(),
            if i == 1 || i == 4 { "nil" } else { "boolean" }
        );
        if i != 1 && i != 4 {
            assert_eq!(combined.raw_get::<bool>("value").unwrap(), i != 7);
        }
        if i == 7 {
            assert!(matches!(
                event.raw_get::<Value>("token").unwrap(),
                Value::Nil
            ));
            let caller: Table = event.raw_get("caller").unwrap();
            let actual: Function = capture
                .raw_get::<Function>("function_at")
                .unwrap()
                .call(caller.raw_get::<usize>("function_token").unwrap())
                .unwrap();
            assert_eq!(
                actual,
                fixture.source.raw_get::<Function>("import").unwrap()
            );
        } else {
            assert_eq!(
                event.raw_get::<usize>("token").unwrap(),
                if i <= 3 { 1 } else { 2 }
            );
            assert!(event.raw_get::<usize>("mod_line_token").unwrap() > 0);
        }
    }
    let items: Table = report.raw_get("items").unwrap();
    assert_eq!(items.raw_len(), 2);
    let first: Table = items.raw_get(1).unwrap();
    let memberships: Table = first.raw_get("memberships").unwrap();
    assert_eq!(memberships.raw_len(), 2);
    let mut active = Vec::new();
    for membership in memberships.sequence_values::<Table>() {
        let membership = membership.unwrap();
        assert_eq!(membership.raw_get::<i64>("item_id").unwrap(), 7);
        assert!(membership.raw_get::<bool>("selected_set").unwrap());
        active.push(
            membership
                .raw_get::<Table>("active")
                .unwrap()
                .raw_get::<bool>("value")
                .unwrap(),
        );
    }
    active.sort();
    assert_eq!(active, vec![false, true]);
    let second: Table = items.raw_get(2).unwrap();
    assert_eq!(second.raw_get::<Table>("memberships").unwrap().raw_len(), 1);
    let second_membership: Table = second
        .raw_get::<Table>("memberships")
        .unwrap()
        .raw_get(1)
        .unwrap();
    assert!(!second_membership.raw_get::<bool>("selected_set").unwrap());
    fixture.no_identity_callbacks();
    let first_handle: Table = capture
        .raw_get::<Function>("item")
        .unwrap()
        .call(1)
        .unwrap();
    let second_handle: Table = capture
        .raw_get::<Function>("item")
        .unwrap()
        .call(2)
        .unwrap();
    assert_ne!(first_handle, second_handle);
    assert_eq!(first_handle.raw_get::<i64>("id").unwrap(), 7);
    let again: Table = capture
        .raw_get::<Function>("report")
        .unwrap()
        .call(())
        .unwrap();
    assert_eq!(again, report);
    capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call::<()>(())
        .unwrap();
}
#[test]
fn preexisting_hook_is_rejected_without_removing_it() {
    let fixture = Fixture::new();
    let prior: Function = fixture
        .lua
        .load("local f=function() end; debug.sethook(f,'c'); return f")
        .eval()
        .unwrap();
    let error = fixture
        .observer
        .raw_get::<Function>("start")
        .unwrap()
        .call::<Table>(())
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
#[test]
fn caught_limit_failure_is_sticky_and_finish_removes_observer() {
    let fixture = Fixture::new();
    let capture = fixture.start();
    let result: MultiValue = fixture
        .lua
        .load("return pcall(modLib.parseMod, string.rep('x',65537))")
        .eval()
        .unwrap();
    assert!(matches!(result.front(), Some(Value::Boolean(false))));
    let error = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call::<Table>(())
        .unwrap_err();
    assert!(error.to_string().contains("byte bound"), "{error}");
    fixture.no_hook();
    assert!(
        capture
            .raw_get::<Function>("report")
            .unwrap()
            .call::<Table>(())
            .is_err()
    );
}
#[test]
fn changed_original_parser_is_rejected_after_safe_cleanup() {
    let fixture = Fixture::new();
    let capture = fixture.start();
    fixture.import(b"valid");
    fixture
        .lua
        .load("modLib.parseMod=function(line,isComb) return line,isComb end")
        .exec()
        .unwrap();
    let error = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call::<Table>(())
        .unwrap_err();
    assert!(error.to_string().contains("binding changed"), "{error}");
    fixture.no_hook();
}

#[test]
fn nested_other_item_frames_cannot_lend_their_line_locals_to_the_callee() {
    let fixture = Fixture::new();
    let capture = fixture.start();
    fixture
        .source
        .raw_get::<Function>("nested_import")
        .unwrap()
        .call::<()>("inner")
        .unwrap();
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    fixture.no_hook();
    fixture.no_identity_callbacks();
    let events: Table = report.raw_get("events").unwrap();
    assert_eq!(events.raw_len(), 2);
    for index in 1..=2 {
        let event: Table = events.raw_get(index).unwrap();
        let item: Table = capture
            .raw_get::<Function>("item")
            .unwrap()
            .call(event.raw_get::<usize>("token").unwrap())
            .unwrap();
        assert_eq!(item.raw_get::<i64>("id").unwrap(), 8);
        assert!(matches!(
            event.raw_get::<Value>("raw_line_index").unwrap(),
            Value::Nil
        ));
        if index == 1 {
            assert!(matches!(
                event.raw_get::<Value>("mod_line_token").unwrap(),
                Value::Nil
            ));
        } else {
            assert!(event.raw_get::<usize>("mod_line_token").unwrap() > 0);
            assert_eq!(event.raw_get::<usize>("mod_line_join_count").unwrap(), 1);
            assert_eq!(
                event
                    .raw_get::<Table>("item_context")
                    .unwrap()
                    .raw_get::<String>("kind")
                    .unwrap(),
                "ranged"
            );
        }
    }
}

#[test]
fn transient_item_occurrences_remain_visible_without_final_line_joins() {
    let fixture = Fixture::new();
    let capture = fixture.start();
    fixture
        .source
        .raw_get::<Function>("transient_import")
        .unwrap()
        .call::<()>("transient")
        .unwrap();
    let report: Table = capture
        .raw_get::<Function>("finish")
        .unwrap()
        .call(())
        .unwrap();
    fixture.no_hook();
    let items: Table = report.raw_get("items").unwrap();
    assert_eq!(items.raw_len(), 1);
    let item: Table = items.raw_get(1).unwrap();
    assert_eq!(
        item.raw_get::<Table>("final_item_ids").unwrap().raw_len(),
        0
    );
    assert_eq!(item.raw_get::<Table>("mod_lines").unwrap().raw_len(), 0);
    let original: Table = capture
        .raw_get::<Function>("item")
        .unwrap()
        .call(1)
        .unwrap();
    assert_eq!(original.raw_get::<i64>("id").unwrap(), 9);
    let events: Table = report.raw_get("events").unwrap();
    assert_eq!(events.raw_len(), 3);
    for event in events.sequence_values::<Table>() {
        let event = event.unwrap();
        assert_eq!(event.raw_get::<usize>("token").unwrap(), 1);
        assert!(event.raw_get::<usize>("mod_line_token").unwrap() > 0);
        assert_eq!(event.raw_get::<usize>("mod_line_join_count").unwrap(), 0);
        assert!(
            event
                .raw_get::<String>("mod_line_unavailable_reason")
                .unwrap()
                .contains("final Item")
        );
    }
}
