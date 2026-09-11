//! Complete unchanged ModList.AddMod paired with the shared native session.
//! This gate covers ordered insertion/aliasing, not configuration callback completion.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    source_program::{
        SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION, SourceProgramDefinitions, SourceProgramOwner,
    },
};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn original_modlist_addmod_uses_shared_persistent_native_session() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/r2c-modstore-source");
    fs::create_dir_all(&destination).unwrap();
    if std::env::var_os("POE_MODSTORE_SOURCE_CHILD").is_some() {
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_hook(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&pair_original),
        )
        .unwrap();
        fs::write(
            destination.join("result.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    let log = fs::File::create(destination.join("child.log")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "original_modlist_addmod_uses_shared_persistent_native_session",
            "--nocapture",
        ])
        .env("POE_MODSTORE_SOURCE_CHILD", "1")
        .current_dir(root.join("vendor/path-of-building-poe2/src"))
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "original ModList child failed; see {}",
                destination.join("child.log").display()
            );
            break;
        }
        if started.elapsed() > Duration::from_secs(120) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("original ModList source comparison exceeded 120 seconds");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Test observation walks raw keys only. Plain receiver arguments isolate this
/// complete method; Common.new parent-proxy construction remains a separate gate.
/// The cases exercise finite scalars and string/integer keys, retaining table aliases.
fn capture(values: &[Value]) -> ProgramValueGraph {
    struct Capture {
        graph: ProgramValueGraph,
        ids: BTreeMap<usize, ProgramTableId>,
    }
    impl Capture {
        fn value(&mut self, value: &Value) -> ProgramValue {
            match value {
                Value::Nil => ProgramValue::Nil,
                Value::Boolean(v) => ProgramValue::Boolean(*v),
                Value::Integer(v) => ProgramValue::Number(*v as f64),
                Value::Number(v) => ProgramValue::Number(*v),
                Value::String(v) => ProgramValue::Bytes(v.as_bytes().to_vec()),
                Value::Table(table) => {
                    let pointer = table.to_pointer() as usize;
                    if let Some(id) = self.ids.get(&pointer) {
                        return ProgramValue::Table(*id);
                    }
                    let id = ProgramTableId(self.graph.tables.len() as u32 + 1);
                    self.ids.insert(pointer, id);
                    self.graph.tables.push(ProgramTable::default());
                    let mut raw = table
                        .clone()
                        .pairs::<Value, Value>()
                        .map(Result::unwrap)
                        .collect::<Vec<_>>();
                    raw.sort_by_key(|(key, _)| match key {
                        Value::Integer(n) => format!("0:{n:020}"),
                        Value::Number(n) => format!("0:{n:020}"),
                        Value::String(s) => format!("1:{}", s.to_str().unwrap()),
                        _ => panic!("unrepresented test observation key"),
                    });
                    let entries = raw
                        .into_iter()
                        .map(|(k, v)| (self.value(&k), self.value(&v)))
                        .collect();
                    self.graph.tables[id.0 as usize - 1].entries = entries;
                    ProgramValue::Table(id)
                }
                _ => panic!("unrepresented test observation value"),
            }
        }
    }
    let mut capture = Capture {
        graph: ProgramValueGraph::default(),
        ids: BTreeMap::new(),
    };
    capture.graph.values = values.iter().map(|v| capture.value(v)).collect();
    capture.graph
}

/// Normalize transport table numbering only; retain scalar types, order, aliases and cycles.
fn canonical(graph: &ProgramValueGraph) -> Json {
    fn scalar(value: &ProgramValue) -> String {
        match value {
            ProgramValue::Bytes(v) => format!("b:{v:?}"),
            ProgramValue::Number(v) => format!("n:{:016x}", v.to_bits()),
            _ => panic!("unrepresented test key"),
        }
    }
    fn visit(
        value: &ProgramValue,
        graph: &ProgramValueGraph,
        ids: &mut BTreeMap<ProgramTableId, usize>,
        tables: &mut Vec<Json>,
    ) -> Json {
        match value {
            ProgramValue::Nil => json!({"nil":true}),
            ProgramValue::Boolean(v) => json!({"boolean":v}),
            ProgramValue::Number(v) => json!({"number_bits":format!("{:016x}",v.to_bits())}),
            ProgramValue::Bytes(v) => json!({"bytes":v}),
            ProgramValue::Callback(_) => panic!("callback escaped observation"),
            ProgramValue::Table(id) => {
                if let Some(index) = ids.get(id) {
                    return json!({"table":index});
                }
                let index = tables.len();
                ids.insert(*id, index);
                tables.push(Json::Null);
                let mut rows = graph.tables[id.0 as usize - 1]
                    .entries
                    .iter()
                    .collect::<Vec<_>>();
                rows.sort_by_key(|(key, _)| scalar(key));
                tables[index] = Json::Array(
                    rows.into_iter()
                        .map(|(k, v)| {
                            json!([visit(k, graph, ids, tables), visit(v, graph, ids, tables)])
                        })
                        .collect(),
                );
                json!({"table":index})
            }
        }
    }
    let mut ids = BTreeMap::new();
    let mut tables = Vec::new();
    let values = graph
        .values
        .iter()
        .map(|v| visit(v, graph, &mut ids, &mut tables))
        .collect::<Vec<_>>();
    json!({"values":values,"tables":tables})
}

fn pair_original(lua: &Lua) -> Result<Json, RuntimeError> {
    let common: Table = lua.globals().get("common")?;
    let classes: Table = common.get("classes")?;
    let class: Table = classes.get("ModList")?;
    let original: Function = class.get("AddMod")?;
    let info = original.info();
    assert!(
        info.source
            .as_deref()
            .unwrap()
            .ends_with("Classes/ModList.lua")
    );
    let first = info.line_defined.unwrap() as u32;
    let last = info.last_line_defined.unwrap() as u32;
    let path = "src/Classes/ModList.lua".to_owned();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let text = poe_optimizer_pob::source::read_verified_text(&root, &path).unwrap();
    let span_text = text
        .split_inclusive('\n')
        .skip(first as usize - 1)
        .take((last - first + 1) as usize)
        .collect::<String>();
    let getupvalue: Function = lua.globals().get::<Table>("debug")?.get("getupvalue")?;
    let (name, value): (String, Function) = getupvalue.call((original.clone(), 1))?;
    assert_eq!(name, "t_insert");
    let no_more: MultiValue = getupvalue.call((original.clone(), 2))?;
    assert!(no_more.is_empty());
    let insert: Function = lua.globals().get::<Table>("table")?.get("insert")?;
    assert_eq!(
        value.to_pointer(),
        insert.to_pointer(),
        "capture must be the actual original primitive"
    );
    assert_eq!(value.info().what, "C");
    let environment: Function = lua
        .load("return function(f) return getfenv(f) == _G end")
        .eval()?;
    assert!(environment.call::<bool>(original.clone())?);
    let owner = SourceProgramOwner::new(SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: BTreeMap::from([(
                path.clone(),
                format!("{:x}", Sha256::digest(text.as_bytes())),
            )]),
            construction_spans: BTreeMap::new(),
            module_order: vec![path.clone()],
        },
        tables: vec![],
        roots: vec![],
        callbacks: vec![
            ParserCallback {
                kind: ParserCallbackKind::Lua {
                    source: ItemSourceSpan {
                        path: path.clone(),
                        line: first,
                        end_line: last,
                        sha256: format!("{:x}", Sha256::digest(span_text.as_bytes())),
                    },
                },
                upvalues: vec![ParserUpvalue {
                    name,
                    value: ParserValue::Callback(ParserCallbackId(2)),
                }],
                environment: ParserEnvironment::OriginalGlobals,
            },
            ParserCallback {
                kind: ParserCallbackKind::Builtin {
                    symbol: "table.insert".into(),
                },
                upvalues: vec![],
                environment: ParserEnvironment::OriginalGlobals,
            },
        ],
        intrinsics: BTreeMap::from([(ParserCallbackId(2), ParserProgramIntrinsic::TableInsert)]),
    })
    .unwrap();
    assert!(owner.parser().is_none());
    let extraction = poe_optimizer_pob::source_programs::lower_from_sources(
        &BTreeMap::from([(path, text)]),
        &owner,
    )
    .unwrap();
    assert!(
        extraction.unsupported().is_empty(),
        "{:?}",
        extraction.unsupported()
    );
    assert_eq!(extraction.catalog().data().programs.len(), 1);
    assert_eq!(
        extraction.catalog().data().programs[0].parameter_count,
        2,
        "implicit self is a real source parameter"
    );
    let plan = CompiledSourcePrograms::new(extraction.catalog()).unwrap();
    let args: MultiValue = lua
        .load(
            r#"
        local tag = {type="Condition", var="SharedSourceTag"}
        local first = modLib.createMod("Strength", "BASE", 10, "SourceSession", 0, 0, tag)
        local second = modLib.createMod("Strength", "MORE", 20, "SourceSession", 0, 0, tag)
        return {}, first, second, tag
    "#,
        )
        .set_name("@test-only-modstore-inputs")
        .eval()?;
    let args = args.into_vec();
    let initial = capture(&args[..1]);
    let (mut session, state) = plan.session(&initial, ProgramLimits::default()).unwrap();
    let borrowed = session.borrow(&capture(&args[1..])).unwrap();
    let mut observed = vec![state[0].clone()];
    observed.extend(borrowed.iter().cloned());
    let compare = |session: &mut ProgramSession| {
        let output = session.snapshot(&observed).unwrap();
        assert!(output.owner().is_same_owner(&owner));
        assert_eq!(
            canonical(output.graph()),
            canonical(&capture(&args)),
            "complete raw state and aliases"
        );
        output
    };
    compare(&mut session);
    let mut steps = vec![session.steps()];
    for index in [0, 1, 0] {
        let source_returns: MultiValue =
            original.call((args[0].clone(), args[index + 1].clone()))?;
        assert!(source_returns.is_empty());
        let native_returns = session
            .invoke(
                ParserCallbackId(1),
                &[state[0].clone(), borrowed[index].clone()],
            )
            .unwrap();
        assert!(native_returns.is_empty());
        compare(&mut session);
        assert!(session.steps() > *steps.last().unwrap());
        steps.push(session.steps());
    }
    let nil = session
        .borrow(&ProgramValueGraph {
            values: vec![ProgramValue::Nil],
            tables: vec![],
        })
        .unwrap();
    assert!(
        original
            .call::<MultiValue>((args[0].clone(), Value::Nil))?
            .is_empty()
    );
    assert!(
        session
            .invoke(ParserCallbackId(1), &[state[0].clone(), nil[0].clone()])
            .unwrap()
            .is_empty()
    );
    compare(&mut session);
    let invalid = session
        .borrow(&ProgramValueGraph {
            values: vec![ProgramValue::Boolean(false)],
            tables: vec![],
        })
        .unwrap();
    assert!(
        original
            .call::<MultiValue>((false, args[1].clone()))
            .is_err()
    );
    let before_failure = session.steps();
    let failure = session
        .invoke(
            ParserCallbackId(1),
            &[invalid[0].clone(), borrowed[0].clone()],
        )
        .unwrap_err();
    assert_eq!(failure.kind, ProgramRuntimeErrorKind::Source);
    assert!(session.steps() > before_failure);
    let final_state = compare(&mut session);
    // Observe the next class-dispatch gate without pretending these objects fit
    // the plain-table admission above. All methods/constructors remain original.
    let constructed: Value = lua
        .load(
            r#"
        local list = new("ModList"):ModList()
        local parent = common.classes.ModStore
        assert(list.Object == list and list._parentInit[parent] == true)
        local proxy = rawget(list, "ModStore")
        assert(proxy._object == list and proxy._parent == parent)
        assert(type(proxy.__index) == "function" and type(proxy.__call) == "function")
        assert(proxy.__newindex == list)
        list:NewMod("Strength", "BASE", 10, "SourceClass")
        list:ReplaceMod("Strength", "BASE", 20, "SourceClass")
        assert(#list == 1 and list[1].value == 20)
        return {source_only=true, native_class_admission=false, parent_proxy_alias=true,
            parent_initialization_table_key=true, original_newmod_replace_value=list[1].value}
    "#,
        )
        .set_name("@test-only-original-class-observation")
        .eval()?;
    let constructed: Json = lua.from_value(constructed)?;
    Ok(json!({
        "source_method":"ModList.AddMod", "source_line":first, "source_end_line":last,
        "programs":1, "paired_insertions":4, "paired_source_failures":1,
        "snapshots_compared":6, "parser_catalog_present":false,
        "capture_authenticated_by_original_pointer":true,
        "steps_after_insertions":steps,"final_steps":session.steps(),
        "final_state":canonical(final_state.graph()),
        "constructed_class_observation":constructed,
        "extractor_sha256":extraction.implementation_sha256(),
        "scope":"Complete AddMod with explicit plain-table receiver arguments. Common.new parent proxies, NewMod method lookup, configuration activation and complete build evaluation remain pending."
    }))
}
