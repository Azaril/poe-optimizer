//! Original-source selection observations for the R1b prerequisite.
//!
//! No native selection implementation is paired here. Complete authenticated
//! ConfigTab Load/Create/SetActive, TreeTab Load/SetActive and PassiveSpec.Load
//! bodies execute with JIT disabled. Config varList is empty: these tests isolate
//! authored selection, not definition defaults. The PassiveSpec constructor is a
//! minimal test host; allocation/URL methods reject unexpected calls. UI, undo,
//! radius updates and notifications record calls. No full passive allocation,
//! item preparation, effective graph or warmed compilation claim is made.
#![cfg(not(target_arch = "wasm32"))]

use mlua::{Function, HookTriggers, Lua, Table, Value, VmState};
use poe_optimizer_pob::source;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn original(path: &str) -> &'static str {
    static TEXT: OnceLock<BTreeMap<&str, String>> = OnceLock::new();
    TEXT.get_or_init(|| {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        [
            "runtime/lua/xml.lua",
            "src/GameVersions.lua",
            "src/Classes/ConfigTab.lua",
            "src/Classes/TreeTab.lua",
            "src/Classes/PassiveSpec.lua",
        ]
        .into_iter()
        .map(|path| (path, source::read_verified_text(&root, path).unwrap()))
        .collect()
    })[path]
        .as_str()
}

fn install(lua: &Lua, path: &str, start: &str, end: Option<&str>, prelude: &str) {
    let text = original(path);
    assert_eq!(
        text.matches(start).count(),
        1,
        "unique source start {start}"
    );
    let from = text.find(start).unwrap();
    let to = end.map_or(text.len(), |end| from + text[from..].find(end).unwrap());
    let line = text[..from].bytes().filter(|&b| b == b'\n').count();
    // One-line context binding plus padding retains the original function lines.
    assert!(!prelude.contains('\n'));
    let body = format!("{prelude}\n{}{}", "\n".repeat(line - 1), &text[from..to]);
    lua.load(body).set_name(format!("@{path}")).exec().unwrap();
}

struct Oracle {
    lua: Lua,
    helpers: Table,
}
impl Oracle {
    fn new() -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(64 * 1024 * 1024).unwrap();
        let started = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(10_000),
            move |_, _| {
                if started.elapsed() > Duration::from_secs(15) {
                    Err(mlua::Error::RuntimeError(
                        "selector source oracle deadline exceeded".into(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load("jit.off();ConfigTabClass={};TreeTabClass={};PassiveSpecClass={};m_min=math.min;legacyClassIdMap={}").exec().unwrap();
        lua.globals()
            .set(
                "originalXml",
                lua.load(original("runtime/lua/xml.lua"))
                    .eval::<Table>()
                    .unwrap(),
            )
            .unwrap();
        lua.load(original("src/GameVersions.lua"))
            .set_name("@src/GameVersions.lua")
            .exec()
            .unwrap();
        install(
            &lua,
            "src/Classes/ConfigTab.lua",
            "function ConfigTabClass:Load(xml, fileName)",
            Some("function ConfigTabClass:GetDefaultState(var, varType)"),
            "local t_insert=table.insert;local s_upper=string.upper",
        );
        install(
            &lua,
            "src/Classes/ConfigTab.lua",
            "function ConfigTabClass:CreateConfigSet(configSetId, title)",
            Some("-- Creates a new config set, adds it"),
            "local varList={}",
        );
        install(
            &lua,
            "src/Classes/ConfigTab.lua",
            "function ConfigTabClass:SetActiveConfigSet(configSetId, init, deferSync)",
            None,
            "",
        );
        install(
            &lua,
            "src/Classes/TreeTab.lua",
            "function TreeTabClass:Load(xml, dbFileName)",
            Some("function TreeTabClass:PostLoad()"),
            "local t_insert=table.insert",
        );
        install(
            &lua,
            "src/Classes/TreeTab.lua",
            "function TreeTabClass:SetActiveSpec(specId, deferSync)",
            Some("function TreeTabClass:SetCompareSpec(specId)"),
            "",
        );
        install(
            &lua,
            "src/Classes/PassiveSpec.lua",
            "function PassiveSpecClass:Load(xml, dbFileName)",
            Some("function PassiveSpecClass:Save(xml)"),
            "local t_insert=table.insert",
        );
        let helpers = lua.load(r#"
local events,messages={},{}
local function event(text)table.insert(events,text)end
local function pack(...)return {n=select('#',...),...}end
local function parse(text)
 local roots,err=originalXml.ParseXML(text);assert(roots,err);return roots[1]
end
launch={ShowErrMsg=function(_,fmt,...)table.insert(messages,string.format(fmt,...))end}
main={OpenMessagePopup=function(_,title,message)table.insert(messages,title..':'..message)end}
data={setJewelRadiiGlobally=function(version)event('radius:'..version)end}
ConPrintf=function(fmt,...)table.insert(messages,string.format(fmt,...))end
local function reject()error('unexpected allocation/URL boundary in selector fixture')end
new=function(kind)
 assert(kind=='PassiveSpec')
 return {PassiveSpec=function(_,build,version)
  event('construct:'..version)
  return setmetatable({build=build,treeVersion=version,jewels={},nodeNotes={},hashOverrides={},
   tree={classIntegerIdMap={},internalAscendNameMap={}},
   ResetUndo=function(self)event('spec-undo:'..(self.title or 'nil'))end,
   SetWindowTitleWithBuildClass=function(self)event('window:'..(self.title or 'nil'))end,
   ImportFromNodeList=reject,DecodeURL=reject,SwitchAttributeNode=reject}, {__index=PassiveSpecClass})
 end}
end
local function config()
 return setmetatable({defaultState={},
  build={buildFlag=false,SyncLoadouts=function()event('sync')end},
  UpdateControls=function()event('controls')end,
  BuildModList=function()event('mod-list')end,
  ResetUndo=function()event('config-undo')end}, {__index=ConfigTabClass})
end
local function tree()
 local build={buildFlag=false,
  SyncLoadouts=function()event('sync')end,
  UpdateClassDropdowns=function(_,version)event('classes:'..version)end,
  itemsTab={items={[1]={},[2]={},[26]={}},itemOrderList={},
   slots={jewel={nodeId=9,selItemId=77},ring={selItemId=26}},
   controls={specSelect={selIndex=-99}},PopulateSlots=function()event('populate')end}}
 return setmetatable({build=build,controls={versionSelect={
  SelByValue=function(_,version,key)assert(key=='value');event('version:'..version)end}}}, {__index=TreeTabClass})
end
local function observe(tab,method,...)
 local result=pack(pcall(tab[method],tab,...))
 return {tab=tab,result=result,events=events,messages=messages}
end
return {
 config=function(text)return observe(config(),'Load',parse(text),'selector.xml')end,
 tree=function(text)return observe(tree(),'Load',parse(text),'selector.xml')end,
 select_config=function(tab,id,init,defer)return observe(tab,'SetActiveConfigSet',id,init,defer)end,
 select_tree=function(tab,id,defer)return observe(tab,'SetActiveSpec',id,defer)end,
 reset=function()events={};messages={}end,
 new_config=function()return config()end,
 create=function(tab,id,title)return tab:CreateConfigSet(id,title)end
}
"#).set_name("@shared_build_selection_test_host").eval().unwrap();
        Self { lua, helpers }
    }
    fn run(&self, kind: &str, xml: &str) -> Table {
        self.helpers
            .get::<Function>("reset")
            .unwrap()
            .call::<()>(())
            .unwrap();
        self.helpers
            .get::<Function>(kind)
            .unwrap()
            .call(xml)
            .unwrap()
    }
    fn reset(&self) {
        self.helpers
            .get::<Function>("reset")
            .unwrap()
            .call::<()>(())
            .unwrap();
    }
    fn select_tree(&self, tab: Table, id: f64, defer: bool) -> Table {
        self.helpers
            .get::<Function>("select_tree")
            .unwrap()
            .call((tab, id, defer))
            .unwrap()
    }
}
fn table(t: &Table, key: &str) -> Table {
    t.get(key).unwrap()
}
fn text(t: &Table, key: &str) -> String {
    t.get(key).unwrap()
}
fn strings(t: Table) -> Vec<String> {
    t.sequence_values().map(Result::unwrap).collect()
}
fn numbers(t: Table) -> Vec<f64> {
    t.sequence_values().map(Result::unwrap).collect()
}
fn nil(t: &Table, key: impl mlua::IntoLua) {
    assert!(matches!(t.get::<Value>(key).unwrap(), Value::Nil));
}
fn success(observation: &Table, returns: usize) {
    let result = table(observation, "result");
    assert!(
        result.get::<bool>(1).unwrap(),
        "{:?}",
        result.get::<Value>(2).unwrap()
    );
    assert_eq!(result.get::<usize>("n").unwrap(), returns + 1);
}
fn failure(observation: &Table, source: &str) -> String {
    let result = table(observation, "result");
    assert!(!result.get::<bool>(1).unwrap());
    assert_eq!(result.get::<usize>("n").unwrap(), 2);
    let error = result.get::<String>(2).unwrap();
    assert!(error.contains(source), "{error}");
    assert!(
        !error.contains("unexpected allocation/URL boundary"),
        "{error}"
    );
    error
}

#[test]
fn original_config_valid_selection_preserves_duplicate_order_and_input_aliases() {
    let o = Oracle::new();
    for (request, selected) in [
        ("", 7.0),
        (" activeConfigSet='99'", 7.0),
        (" activeConfigSet='2'", 2.0),
        (" activeConfigSet='bad'", 7.0),
        (" activeConfigSet='nan'", 7.0),
    ] {
        let out=o.run("config",&format!("<Config{request}><ConfigSet id='7' title='old'/><ConfigSet id='7.0' title='winner'><Input name='x' number='13'/></ConfigSet><ConfigSet id='2' title='two'/></Config>"));
        success(&out, 0);
        let tab = table(&out, "tab");
        assert_eq!(numbers(table(&tab, "configSetOrderList")), [7.0, 7.0, 2.0]);
        assert_eq!(tab.get::<f64>("activeConfigSetId").unwrap(), selected);
        let sets = table(&tab, "configSets");
        assert_eq!(text(&sets.get::<Table>(7).unwrap(), "title"), "winner");
        assert_eq!(
            table(&tab, "input"),
            table(&sets.get::<Table>(selected).unwrap(), "input")
        );
        assert_eq!(
            strings(table(&out, "events")),
            ["controls", "mod-list", "sync", "config-undo"]
        );
        assert!(strings(table(&out, "messages")).is_empty());
    }
    for xml in ["<Config/>", "<Config><Input name='x' number='1'/></Config>"] {
        let out = o.run("config", xml);
        success(&out, 0);
        assert_eq!(
            table(&out, "tab").get::<f64>("activeConfigSetId").unwrap(),
            1.0
        );
    }
}

#[test]
fn original_config_malformed_ids_preserve_created_set_and_fail_before_selection() {
    let o = Oracle::new();
    for id in ["", " id='bad'", " id=''", " id='nan'"] {
        let out = o.run(
            "config",
            &format!("<Config><ConfigSet{id} title='malformed'/></Config>"),
        );
        let error = failure(&out, "src/Classes/ConfigTab.lua:");
        let tab = table(&out, "tab");
        assert_eq!(tab.get::<f64>("activeConfigSetId").unwrap(), 1.0);
        nil(&tab, "input");
        assert!(strings(table(&out, "events")).is_empty());
        assert!(!table(&tab, "build").get::<bool>("buildFlag").unwrap());
        let sets = table(&tab, "configSets");
        if id.contains("nan") {
            assert!(error.contains("NaN"), "{error}");
            assert_eq!(sets.pairs::<Value, Value>().count(), 0);
        } else {
            assert!(error.contains("nil"), "{error}");
            assert_eq!(text(&sets.get::<Table>(1).unwrap(), "title"), "malformed");
            nil(&table(&tab, "configSetOrderList"), 1);
        }
    }
    let out=o.run("config","<Config><ConfigSet id='1' title='retained'/><ConfigSet title='created-before-error'/></Config>");
    failure(&out, "src/Classes/ConfigTab.lua:");
    let tab = table(&out, "tab");
    let sets = table(&tab, "configSets");
    assert_eq!(text(&sets.get::<Table>(1).unwrap(), "title"), "retained");
    assert_eq!(
        text(&sets.get::<Table>(2).unwrap(), "title"),
        "created-before-error"
    );
    assert_eq!(numbers(table(&tab, "configSetOrderList")), [1.0]);
}

#[test]
fn original_config_mixed_legacy_rows_use_absolute_order_positions_and_messages_do_not_throw() {
    let o = Oracle::new();
    let out=o.run("config","<Config><ConfigSet id='7' title='seven'/><Input name='legacy' number='4'/><ConfigSet id='2' title='two'/></Config>");
    success(&out, 0);
    let tab = table(&out, "tab");
    let order = table(&tab, "configSetOrderList");
    assert_eq!(order.get::<f64>(1).unwrap(), 7.0);
    nil(&order, 2);
    assert_eq!(order.get::<f64>(3).unwrap(), 2.0);
    let sets = table(&tab, "configSets");
    assert_eq!(
        table(&sets.get::<Table>(1).unwrap(), "input")
            .get::<f64>("legacy")
            .unwrap(),
        4.0
    );
    // The legacy Input created ID 1, so the absent selector uses that existing
    // ID rather than falling back to order[1] (7).
    assert_eq!(tab.get::<f64>("activeConfigSetId").unwrap(), 1.0);
    // Missing Input name is reported by the complete original helper. Its true
    // return is ignored by Load; a following valid record and selection run.
    let out = o.run(
        "config",
        "<Config><Input number='1'/><Input name='later' number='8'/></Config>",
    );
    success(&out, 0);
    assert_eq!(strings(table(&out, "messages")).len(), 1);
    assert!(strings(table(&out, "messages"))[0].contains("missing name attribute"));
    assert_eq!(
        table(&table(&out, "tab"), "input")
            .get::<f64>("later")
            .unwrap(),
        8.0
    );
}

#[test]
fn original_config_sparse_allocation_is_lua_length_and_numeric_keys_are_not_positions() {
    let o = Oracle::new();
    let create = o.helpers.get::<Function>("create").unwrap();
    for existing in [
        vec![7.0],
        vec![1.0, 3.0],
        vec![2.0, 8.0],
        vec![-2.5, 0.0],
        vec![1.0, 2.0, 4.0],
    ] {
        let tab = o
            .helpers
            .get::<Function>("new_config")
            .unwrap()
            .call::<Table>(())
            .unwrap();
        tab.set("configSets", o.lua.create_table().unwrap())
            .unwrap();
        for id in &existing {
            create
                .call::<Table>((tab.clone(), *id, "existing"))
                .unwrap();
        }
        let sets = table(&tab, "configSets");
        // Observe the VM length at the same state. This does not declare a
        // portable sparse-table length algorithm or substitute first-free.
        let source_len = o
            .lua
            .load("return function(t)return #t end")
            .eval::<Function>()
            .unwrap()
            .call::<usize>(sets.clone())
            .unwrap();
        let added = create
            .call::<Table>((tab.clone(), Value::Nil, "allocated"))
            .unwrap();
        assert_eq!(added.get::<usize>("id").unwrap(), source_len + 1);
        assert_eq!(sets.get::<Table>(source_len + 1).unwrap(), added);
        println!(
            "config sparse allocation: existing={existing:?}, original_length={source_len}, allocated={}",
            source_len + 1
        );
    }
    for id in ["-2.5", "0", "-0", "inf"] {
        let out = o.run(
            "config",
            &format!(
                "<Config activeConfigSet='{id}'><ConfigSet id='{id}' title='numeric'/></Config>"
            ),
        );
        success(&out, 0);
        assert_eq!(
            text(
                &table(&out, "tab")
                    .get::<Table>("configSets")
                    .unwrap()
                    .get::<Table>(table(&out, "tab").get::<f64>("activeConfigSetId").unwrap())
                    .unwrap(),
                "title"
            ),
            "numeric"
        );
    }
}

#[test]
fn original_tree_positions_upper_clamp_and_version_defaults_are_independent() {
    let o = Oracle::new();
    for (request, selected, dropdown) in [
        ("", 1.0, 1.0),
        (" activeSpec='2'", 2.0, 2.0),
        (" activeSpec='99'", 2.0, 99.0),
        (" activeSpec='bad'", 1.0, 1.0),
    ] {
        let out=o.run("tree",&format!("<Tree{request}><Spec id='90' title='first'/><Spec id='90' title='second' treeVersion='0_5'/></Tree>"));
        success(&out, 0);
        let tab = table(&out, "tab");
        let build = table(&tab, "build");
        assert_eq!(tab.get::<f64>("activeSpec").unwrap(), selected);
        assert_eq!(
            table(&build, "spec"),
            table(&tab, "specList").get::<Table>(selected).unwrap()
        );
        assert_eq!(
            table(&table(&table(&build, "itemsTab"), "controls"), "specSelect")
                .get::<f64>("selIndex")
                .unwrap(),
            dropdown
        );
        assert_eq!(
            text(
                &table(&tab, "specList").get::<Table>(1).unwrap(),
                "treeVersion"
            ),
            "0_1"
        );
        assert_eq!(strings(table(&out, "messages")), Vec::<String>::new());
    }
    let empty = o.run("tree", "<Tree/>");
    success(&empty, 0);
    assert_eq!(
        text(
            &table(&table(&empty, "tab"), "build")
                .get::<Table>("spec")
                .unwrap(),
            "treeVersion"
        ),
        "0_5"
    );
    let legacy = o.run("tree", "<Spec title='legacy' treeVersion='not-a-version'/>");
    success(&legacy, 0);
    let tab = table(&legacy, "tab");
    assert_eq!(tab.get::<f64>("activeSpec").unwrap(), 1.0);
    assert_eq!(
        text(&table(&table(&tab, "build"), "spec"), "treeVersion"),
        "0_1"
    );
    assert_eq!(
        strings(table(&legacy, "events")),
        ["construct:0_1", "spec-undo:legacy"]
    );
    assert!(!table(&tab, "build").get::<bool>("buildFlag").unwrap());
}

#[test]
fn original_tree_zero_negative_and_fractional_requests_fail_after_recording_active_index() {
    let o = Oracle::new();
    for (request, expected) in [
        ("0", 0.0),
        ("-0", -0.0),
        ("-2", -2.0),
        ("1.5", 1.5),
        ("-inf", f64::NEG_INFINITY),
    ] {
        let out = o.run(
            "tree",
            &format!("<Tree activeSpec='{request}'><Spec title='one'/><Spec title='two'/></Tree>"),
        );
        let error = failure(&out, "src/Classes/TreeTab.lua:");
        assert!(error.contains("nil"), "{error}");
        let tab = table(&out, "tab");
        assert_eq!(
            tab.get::<f64>("activeSpec").unwrap().to_bits(),
            expected.to_bits()
        );
        nil(&table(&tab, "build"), "spec");
        assert_eq!(
            strings(table(&out, "events")),
            [
                "construct:0_1",
                "spec-undo:one",
                "construct:0_1",
                "spec-undo:two"
            ]
        );
    }
    let out = o.run(
        "tree",
        "<Tree activeSpec='inf'><Spec title='one'/><Spec title='two'/></Tree>",
    );
    success(&out, 0);
    assert_eq!(table(&out, "tab").get::<f64>("activeSpec").unwrap(), 2.0);
}

#[test]
fn original_tree_unknown_version_returns_failure_but_passive_load_failure_is_ignored() {
    let o = Oracle::new();
    let out=o.run("tree","<Tree><Spec title='retained'/><Spec title='unknown' treeVersion='missing'/><Spec title='unreached'/></Tree>");
    success(&out, 1);
    assert!(table(&out, "result").get::<bool>(2).unwrap());
    let tab = table(&out, "tab");
    assert_eq!(table(&tab, "specList").raw_len(), 1);
    nil(&tab, "activeSpec");
    nil(&table(&tab, "build"), "spec");
    assert_eq!(
        strings(table(&out, "events")),
        ["construct:0_1", "spec-undo:retained"]
    );
    assert!(strings(table(&out, "messages"))[0].starts_with("Unknown Passive Tree Version:"));
    // This is the complete original PassiveSpec.Load, not a synthetic true return.
    let out=o.run("tree","<Tree><Spec title='partial'><Sockets><Socket nodeId='9' itemId='1'/><Socket nodeId='10'/></Sockets></Spec></Tree>");
    success(&out, 0);
    let tab = table(&out, "tab");
    let spec = table(&table(&tab, "build"), "spec");
    assert_eq!(table(&spec, "jewels").get::<f64>(9).unwrap(), 1.0);
    assert!(strings(table(&out, "messages"))[0].contains("missing 'itemId'"));
    let events = strings(table(&out, "events"));
    assert!(!events.iter().any(|e| e.starts_with("spec-undo:")));
    assert!(events.contains(&"window:partial".to_owned()));
    assert_eq!(tab.get::<f64>("activeSpec").unwrap(), 1.0);
}

#[test]
fn original_spec_switch_preserves_previous_jewel_writeback_and_error_prefix() {
    let o = Oracle::new();
    let out=o.run("tree","<Tree><Spec title='one'><Sockets><Socket nodeId='9' itemId='1'/></Sockets></Spec><Spec title='two'><Sockets><Socket nodeId='9' itemId='2'/></Sockets></Spec></Tree>");
    success(&out, 0);
    let tab = table(&out, "tab");
    let build = table(&tab, "build");
    let items = table(&build, "itemsTab");
    let slot = table(&table(&items, "slots"), "jewel");
    assert_eq!(slot.get::<f64>("selItemId").unwrap(), 1.0);
    slot.set("selItemId", 26).unwrap();
    items
        .set("itemOrderList", o.lua.create_sequence_from([26]).unwrap())
        .unwrap();
    o.reset();
    let selected = o.select_tree(tab.clone(), 2.0, true);
    success(&selected, 0);
    assert_eq!(
        table(&table(&tab, "specList").get::<Table>(1).unwrap(), "jewels")
            .get::<f64>(9)
            .unwrap(),
        26.0
    );
    assert_eq!(slot.get::<f64>("selItemId").unwrap(), 2.0);
    assert_eq!(
        strings(table(&selected, "events")),
        [
            "radius:0_1",
            "window:two",
            "classes:0_1",
            "populate",
            "version:0_1"
        ]
    );
    o.reset();
    let before = table(&build, "spec");
    let failed = o.select_tree(tab.clone(), 0.0, false);
    failure(&failed, "src/Classes/TreeTab.lua:");
    assert_eq!(table(&build, "spec"), before);
    assert_eq!(slot.get::<f64>("selItemId").unwrap(), 2.0);
    assert_eq!(tab.get::<f64>("activeSpec").unwrap(), 0.0);
    assert!(strings(table(&failed, "events")).is_empty());
}

#[test]
fn original_direct_config_selection_observes_init_defer_and_missing_order_reset() {
    let o = Oracle::new();
    let out = o.run(
        "config",
        "<Config activeConfigSet='2'><ConfigSet id='7'/><ConfigSet id='2'/></Config>",
    );
    success(&out, 0);
    let tab = table(&out, "tab");
    let select = o.helpers.get::<Function>("select_config").unwrap();
    o.reset();
    let kept = select
        .call::<Table>((tab.clone(), Value::Nil, true, true))
        .unwrap();
    success(&kept, 0);
    assert_eq!(tab.get::<f64>("activeConfigSetId").unwrap(), 2.0);
    assert!(strings(table(&kept, "events")).is_empty());
    o.reset();
    let fallback = select
        .call::<Table>((tab.clone(), 99, false, false))
        .unwrap();
    success(&fallback, 0);
    assert_eq!(tab.get::<f64>("activeConfigSetId").unwrap(), 7.0);
    assert_eq!(
        strings(table(&fallback, "events")),
        ["controls", "mod-list", "sync"]
    );
    tab.set("configSets", o.lua.create_table().unwrap())
        .unwrap();
    tab.set("configSetOrderList", o.lua.create_table().unwrap())
        .unwrap();
    o.reset();
    let reset = select
        .call::<Table>((tab.clone(), Value::Nil, true, true))
        .unwrap();
    success(&reset, 0);
    assert_eq!(tab.get::<f64>("activeConfigSetId").unwrap(), 1.0);
    assert_eq!(numbers(table(&tab, "configSetOrderList")), [1.0]);
    assert_eq!(
        table(&tab, "input"),
        table(&table(&tab, "configSets").get::<Table>(1).unwrap(), "input")
    );
}

#[test]
fn original_tree_nan_request_uses_source_min_but_retains_requested_dropdown_value() {
    let o = Oracle::new();
    let out = o.run(
        "tree",
        "<Tree activeSpec='nan'><Spec title='one'/><Spec title='two'/></Tree>",
    );
    success(&out, 0);
    let tab = table(&out, "tab");
    assert_eq!(tab.get::<f64>("activeSpec").unwrap(), 2.0);
    assert_eq!(text(&table(&table(&tab, "build"), "spec"), "title"), "two");
    let dropdown = table(
        &table(&table(&table(&tab, "build"), "itemsTab"), "controls"),
        "specSelect",
    );
    let request = dropdown.get::<f64>("selIndex").unwrap();
    assert!(request.is_nan());
    println!(
        "original Tree nan: active=2, requested_dropdown_bits={:016x}",
        request.to_bits()
    );
}
