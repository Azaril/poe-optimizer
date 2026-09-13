//! Source-fed read-only slot component oracle; not original Load/native preparation parity.
#[path = "item_slot_validity_directed.rs"]
mod directed;
#[path = "item_assembly_graph.rs"]
mod graph;
#[allow(dead_code)]
#[path = "configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, MultiValue, Table, Value as LuaValue};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    item_loading::{ItemMetadataTable as Metadata, ItemMetadataValue as Field},
};
use poe_optimizer_engine::source_program::{
    ProgramTable, ProgramTableId, ProgramValue, ProgramValueGraph,
};
use poe_optimizer_import::{
    item_loading::assembly::{AssemblyError, AssemblyErrorKind},
    item_slot_validity::{
        self as native, SlotValidityContext, SlotValidityLimits, SlotValidityProgram,
        SlotValidityRequest, SlotValidityResult, Value,
    },
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const HELPER: &str = include_str!("item_slot_validity_source.lua");
const TEST: &str = "all_five_original_item_slot_validity_matches_native_component";
const CHILD: &str = "POE_ITEM_SLOT_VALIDITY_CHILD";
const OUTPUT: &str = "POE_ITEM_SLOT_VALIDITY_OUTPUT";
// Only Lua execution errors count as source outcomes. Host, resource and
// conversion failures are observation failures even inside callback wrappers.
pub(super) fn is_source_runtime_error(mut error: &mlua::Error) -> bool {
    loop {
        match error {
            mlua::Error::RuntimeError(_) => return true,
            mlua::Error::CallbackError { cause, .. } => error = cause.as_ref(),
            _ => return false,
        }
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn metadata(table: &Table) -> Metadata {
    struct Budget {
        rows: usize,
        bytes: usize,
        active: BTreeSet<usize>,
    }
    fn value(v: LuaValue, b: &mut Budget, depth: usize) -> Field {
        assert!(depth <= 64, "finite context depth bound");
        match v {
            LuaValue::Boolean(v) => Field::Boolean(v),
            LuaValue::Integer(v) => Field::Number(v as f64),
            LuaValue::Number(v) => {
                assert!(v.is_finite());
                Field::Number(v)
            }
            LuaValue::String(v) => {
                let v = v.to_str().unwrap().to_string();
                b.bytes = b.bytes.checked_add(v.len()).unwrap();
                assert!(b.bytes <= 16 * 1024 * 1024);
                Field::Text(v)
            }
            LuaValue::Table(t) => Field::Table(table_value(&t, b, depth + 1)),
            _ => panic!("explicit finite context cannot erase {}", v.type_name()),
        }
    }
    fn table_value(t: &Table, b: &mut Budget, depth: usize) -> Metadata {
        assert!(depth <= 64 && t.metatable().is_none());
        let pointer = t.to_pointer() as usize;
        assert!(
            b.active.insert(pointer),
            "cyclic context outside metadata ingress"
        );
        let mut out = Metadata::default();
        for row in t.clone().pairs::<LuaValue, LuaValue>() {
            b.rows += 1;
            assert!(b.rows <= 262144);
            let (k, v) = row.unwrap();
            let v = value(v, b, depth);
            match k {
                LuaValue::String(k) => {
                    let k = k.to_str().unwrap().to_string();
                    b.bytes = b.bytes.checked_add(k.len()).unwrap();
                    assert!(b.bytes <= 16 * 1024 * 1024);
                    out.fields.insert(k, v);
                }
                LuaValue::Integer(k) => {
                    out.indexed.insert(k, v);
                }
                LuaValue::Number(k)
                    if k.is_finite()
                        && k.fract() == 0.0
                        && k >= i64::MIN as f64
                        && k < i64::MAX as f64 =>
                {
                    out.indexed.insert(k as i64, v);
                }
                _ => panic!("unrepresented context key"),
            }
        }
        assert!(b.active.remove(&pointer));
        out
    }
    table_value(
        table,
        &mut Budget {
            rows: 0,
            bytes: 0,
            active: BTreeSet::new(),
        },
        0,
    )
}
fn lua_value(lua: &Lua, v: &Field) -> LuaValue {
    match v {
        Field::Boolean(v) => LuaValue::Boolean(*v),
        Field::Number(v) => LuaValue::Number(*v),
        Field::Text(v) => LuaValue::String(lua.create_string(v).unwrap()),
        Field::Table(t) => LuaValue::Table(lua_table(lua, t)),
        Field::Array(v) => LuaValue::Table(
            lua.create_sequence_from(v.iter().map(|v| lua_value(lua, v)))
                .unwrap(),
        ),
        Field::Callback(_) => panic!("callback outside finite fixture"),
    }
}
fn lua_table(lua: &Lua, t: &Metadata) -> Table {
    let out = lua.create_table().unwrap();
    for (k, v) in &t.fields {
        out.raw_set(k.as_str(), lua_value(lua, v)).unwrap()
    }
    for (k, v) in &t.indexed {
        out.raw_set(*k, lua_value(lua, v)).unwrap()
    }
    out
}
fn raw<'a>(t: &'a Metadata, k: &str) -> Value<'a> {
    t.fields
        .get(k)
        .map_or(Ok(Value::Nil), Value::metadata)
        .unwrap()
}
struct Context<'a> {
    root: &'a Metadata,
    flags: &'a Metadata,
    calcs: bool,
    calls: Vec<String>,
    failure: Option<&'a str>,
}
impl<'a> SlotValidityContext<'a> for Context<'a> {
    fn active_item_set(&mut self) -> Result<Value<'a>, AssemblyError> {
        Ok(raw(self.root, "activeItemSet"))
    }
    fn tree_node(&mut self, key: Value<'_>) -> Result<Value<'a>, AssemblyError> {
        raw(self.root, "treeNodes").index(key)
    }
    fn effective_node(&mut self, key: Value<'_>) -> Result<Value<'a>, AssemblyError> {
        raw(self.root, "specNodes").index(key)
    }
    fn inventory_item(&mut self, key: Value<'_>) -> Result<Value<'a>, AssemblyError> {
        raw(self.root, "items").index(key)
    }
    fn has_calculation_environment(&mut self) -> Result<bool, AssemblyError> {
        Ok(self.calcs)
    }
    fn flag(&mut self, name: &str) -> Result<Value<'a>, AssemblyError> {
        self.calls.push(name.into());
        if self.failure == Some(name) {
            return Err(AssemblyError::source("directed original Flag failure"));
        }
        Ok(raw(self.flags, name))
    }
}
fn source_graph(values: &[LuaValue]) -> Json {
    graph::canonical(&graph::capture(values).unwrap()).unwrap()
}
fn native_graph(values: &[Value<'_>]) -> Json {
    fn add<'a>(
        v: Value<'a>,
        g: &mut ProgramValueGraph,
        seen: &mut Vec<Value<'a>>,
        depth: usize,
        work: &mut usize,
        bytes: &mut usize,
    ) -> ProgramValue {
        *work += 1;
        assert!(*work <= 262144 && depth <= 64);
        match v {
            Value::Nil => ProgramValue::Nil,
            Value::Boolean(v) => ProgramValue::Boolean(v),
            Value::Number(v) => ProgramValue::Number(v),
            Value::Text(v) => {
                *bytes = bytes
                    .checked_sub(v.len())
                    .expect("native output text bound");
                ProgramValue::Bytes(v.as_bytes().to_vec())
            }
            Value::Table(t) => {
                let identity = Value::Table(t);
                if let Some(i) = seen.iter().position(|old| old.same_identity(identity)) {
                    return ProgramValue::Table(ProgramTableId(i as u32 + 1));
                }
                assert!(seen.len() < 32768);
                seen.push(identity);
                let id = ProgramTableId(seen.len() as u32);
                g.tables.push(ProgramTable::default());
                let mut entries = Vec::new();
                for row in t.entries().unwrap() {
                    let (k, v) = row.unwrap();
                    let k = match k {
                        native::Key::Text(k) => {
                            *bytes = bytes
                                .checked_sub(k.len())
                                .expect("native output key byte bound");
                            ProgramValue::Bytes(k.as_bytes().to_vec())
                        }
                        native::Key::Index(k) => ProgramValue::Number(k as f64),
                    };
                    entries.push((k, add(v, g, seen, depth + 1, work, bytes)));
                }
                g.tables[id.0 as usize - 1].entries = entries;
                ProgramValue::Table(id)
            }
        }
    }
    let mut g = ProgramValueGraph::default();
    let mut seen = Vec::new();
    let mut work = 0;
    let mut bytes = 16 * 1024 * 1024;
    g.values = values
        .iter()
        .map(|v| add(*v, &mut g, &mut seen, 0, &mut work, &mut bytes))
        .collect();
    graph::canonical(&g).unwrap()
}
#[allow(clippy::too_many_arguments)]
fn compare<'a>(
    program: &SlotValidityProgram,
    method: &Function,
    receiver: Table,
    item: LuaValue,
    slot: &str,
    set: LuaValue,
    flags: LuaValue,
    request: SlotValidityRequest<'a>,
    ctx: &mut Context<'a>,
    label: &str,
) -> Json {
    ctx.calls.clear();
    let original = method.call::<MultiValue>((receiver, item, slot, set, flags));
    let result = program.check(request, ctx);
    match (original, result) {
        (Ok(source), Ok(result)) => {
            let native = match result {
                SlotValidityResult::NoValues => vec![],
                SlotValidityResult::Value(v) => vec![v],
            };
            let source = source.into_iter().collect::<Vec<_>>();
            assert_eq!(
                source.len(),
                native.len(),
                "{label}: exact original result count"
            );
            let sg = source_graph(&source);
            assert_eq!(sg, native_graph(&native), "{label}: exact finite result");
            json!({"return_count":source.len(),"result":sg,"native_flag_queries":ctx.calls,"status":"matched"})
        }
        (Err(source), Err(native)) => {
            assert!(
                is_source_runtime_error(&source),
                "{label}: original host/observation failure: {source:?}"
            );
            assert_eq!(native.kind, AssemblyErrorKind::Source, "{label}: {source}");
            json!({"status":"matching_source_error","source_error":source.to_string(),"native_error":native.message,"native_flag_queries":ctx.calls})
        }
        (source, native) => panic!("{label}: original {source:?}; native {native:?}"),
    }
}
fn run_host(lua: &Lua, module: &Table, execute: bool) -> Json {
    let bound: Table = module
        .raw_get::<Function>("bind")
        .unwrap()
        .call(())
        .unwrap();
    let declaration: Table = bound.raw_get("declaration").unwrap();
    assert_eq!(declaration.raw_get::<usize>("first").unwrap(), 2603);
    assert_eq!(declaration.raw_get::<usize>("last").unwrap(), 2687);
    assert!(
        declaration
            .raw_get::<String>("source")
            .unwrap()
            .replace('\\', "/")
            .ends_with("Classes/ItemsTab.lua")
    );
    let project: Function = bound.raw_get("projection").unwrap();
    let before: Table = project.call(()).unwrap();
    let before_graph = source_graph(&[LuaValue::Table(before.clone())]);
    if !execute {
        return json!({"initial_read_set":before_graph,"source_only_control":true});
    }
    let root = metadata(&before);
    let snapshot = bundled_snapshot().unwrap();
    let program = SlotValidityProgram::new(
        &snapshot.item_assembly().policy().slot_validity,
        SlotValidityLimits::default(),
    )
    .unwrap();
    let policy = program.policy();
    let method: Function = bound.raw_get("target").unwrap();
    let receiver: Table = bound.raw_get("receiver").unwrap();
    let items: Table = bound.raw_get("items").unwrap();
    let sets: Table = bound.raw_get("sets").unwrap();
    let mut ids = bound
        .raw_get::<Table>("item_ids")
        .unwrap()
        .sequence_values::<i64>()
        .collect::<mlua::Result<Vec<_>>>()
        .unwrap();
    ids.sort();
    let mut set_ids = bound
        .raw_get::<Table>("set_ids")
        .unwrap()
        .sequence_values::<i64>()
        .collect::<mlua::Result<Vec<_>>>()
        .unwrap();
    set_ids.sort();
    let mut slots = bound
        .raw_get::<Table>("slots")
        .unwrap()
        .sequence_values::<String>()
        .collect::<mlua::Result<Vec<_>>>()
        .unwrap();
    slots.sort();
    assert!(!ids.is_empty() && !set_ids.is_empty() && !slots.is_empty());
    assert!(ids.len() * set_ids.len() * slots.len() <= 262144);
    let build: Table = lua.globals().raw_get("build").unwrap();
    let mod_db: Table = build
        .raw_get::<Table>("calcsTab")
        .unwrap()
        .raw_get::<Table>("mainEnv")
        .unwrap()
        .raw_get("modDB")
        .unwrap();
    let flag: Function = mod_db.get("Flag").unwrap();
    let mut observed_flags = Metadata::default();
    for name in [
        &policy.weapon.giants_blood.query_name,
        &policy.weapon.instruments_of_power.query_name,
        &policy.weapon.lord_of_the_wilds.query_name,
    ] {
        let v: LuaValue = flag
            .call((mod_db.clone(), LuaValue::Nil, name.as_str()))
            .unwrap();
        let t = lua.create_table().unwrap();
        t.raw_set("value", v).unwrap();
        if let Some(v) = metadata(&t).fields.remove("value") {
            observed_flags.fields.insert(name.clone(), v);
        }
    }
    let mut ctx = Context {
        root: &root,
        flags: &observed_flags,
        calcs: true,
        calls: vec![],
        failure: None,
    };
    let mut results = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    for id in &ids {
        for set_id in &set_ids {
            for slot in &slots {
                let item = items.raw_get::<Table>(*id).unwrap();
                let set = sets.raw_get::<Table>(*set_id).unwrap();
                let request = SlotValidityRequest {
                    item: raw(&root, "items")
                        .index(Value::Number(*id as f64))
                        .unwrap(),
                    slot_name: slot,
                    item_set: raw(&root, "itemSets")
                        .index(Value::Number(*set_id as f64))
                        .unwrap(),
                    flag_state: Value::Nil,
                };
                let out = compare(
                    &program,
                    &method,
                    receiver.clone(),
                    LuaValue::Table(item),
                    slot,
                    LuaValue::Table(set),
                    LuaValue::Nil,
                    request,
                    &mut ctx,
                    &format!("actual item {id}, set {set_id}, slot {slot}"),
                );
                *counts
                    .entry(out["status"].as_str().unwrap().into())
                    .or_default() += 1;
                results.push(json!({"item_id":id,"set_id":set_id,"slot":slot,"outcome":out}));
            }
        }
    }
    bound
        .raw_get::<Function>("verify")
        .unwrap()
        .call::<()>(())
        .unwrap();
    let after: Table = project.call(()).unwrap();
    assert_eq!(
        before_graph,
        source_graph(&[LuaValue::Table(after)]),
        "complete original method mutated declared read set"
    );
    let derived = directed::run(lua, &program, &method);
    json!({"initial_read_set":before_graph,"item_count":ids.len(),"set_count":set_ids.len(),"slot_count":slots.len(),"calls":results.len(),"counts":counts,"results":results,"directed":derived,"scope":{"complete_method":true,"source_fed_context":true,"native_preparation":false,"whole_load":false,"all_original_items_retained":true,"raw_return_pack":true,"post_import_context":true,"enumeration_order":"sorted identifiers and slot labels; no activation replay","initial_load_flag_context_proven":false,"native_flag_order_observed_source":false,"read_set_unchanged":true,"arbitrary_alias_recovery":false}})
}
fn host(repo: &Path, directory: &Path, xml: &str, execute: bool) -> Json {
    fs::create_dir_all(directory).unwrap();
    let module = Rc::new(RefCell::new(None::<Table>));
    let before = |lua: &Lua| {
        *module.borrow_mut() = Some(
            lua.load(HELPER)
                .set_name("@item_slot_validity_source.lua")
                .eval()?,
        );
        Ok(())
    };
    let after = |lua: &Lua| Ok(run_host(lua, module.borrow().as_ref().unwrap(), execute));
    source::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        directory,
        xml,
        None,
        false,
        Some(&before),
        None,
        Some(&after),
    )
    .unwrap()
}
fn child(repo: &Path, output: &Path, entry: &Json) {
    let name = entry["xml"].as_str().unwrap();
    let xml = fs::read_to_string(
        repo.join("tests/fixtures/builds/breadth-20260908")
            .join(name),
    )
    .unwrap();
    assert_eq!(hash(xml.as_bytes()), entry["xml_sha256"].as_str().unwrap());
    let directory = output.join(name);
    fs::create_dir_all(&directory).unwrap();
    let control = host(repo, &directory.join("control-host"), &xml, false);
    let report = host(repo, &directory.join("observed-host"), &xml, true);
    assert_eq!(
        control["additional_observation"]["initial_read_set"],
        report["additional_observation"]["initial_read_set"]
    );
    assert_eq!(control["selected"], report["selected"]);
    let expected = roxmltree::Document::parse(&xml)
        .unwrap()
        .descendants()
        .filter(|n| n.has_tag_name("Item") && n.parent().is_some_and(|p| p.has_tag_name("Items")))
        .count();
    assert_eq!(
        report["additional_observation"]["item_count"]
            .as_u64()
            .unwrap() as usize,
        expected
    );
    fs::write(
        directory.join("control.json"),
        serde_json::to_vec_pretty(&control).unwrap(),
    )
    .unwrap();
    fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
pub fn run() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(repo.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    let output = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2ac-slot-validity-01/source"));
    fs::create_dir_all(&output).unwrap();
    if let Ok(name) = std::env::var(CHILD) {
        child(
            &repo,
            &output,
            index["builds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["xml"] == name)
                .unwrap(),
        );
        return;
    }
    let mut children = Vec::new();
    for entry in index["builds"].as_array().unwrap() {
        let name = entry["xml"].as_str().unwrap();
        let mut p = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, name)
            .env(OUTPUT, &output)
            .current_dir(repo.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(
                fs::File::create(output.join(format!("{name}.stdout.log"))).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(output.join(format!("{name}.stderr.log"))).unwrap(),
            ))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(s) = p.try_wait().unwrap() {
                break s;
            }
            if start.elapsed() > Duration::from_secs(300) {
                p.kill().unwrap();
                let _ = p.wait();
                panic!("slot source child timeout {name}")
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "slot source child failed {name}: {}",
            output.display()
        );
        children.push(json!({"xml":name,"exit":status.code()}));
    }
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({"children":children,"original_items":116,"fresh_hosts":10,"whole_native_builds":0,"complete_load_parity":false})).unwrap()).unwrap();
}
