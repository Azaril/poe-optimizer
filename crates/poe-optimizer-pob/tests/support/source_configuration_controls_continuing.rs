//! Test-only lifecycle changes and dependency removals, paired with complete
//! original methods. These producer operations do not admit SetActiveConfigSet.
use super::*;
use mlua::MultiValue;
use poe_optimizer_data::source_program::{SourceSessionValue, SourceValue};

#[derive(Clone)]
enum Arg {
    Root(&'static str),
    Scalar(Value),
}
fn scalar(value: impl Into<Value>) -> Arg {
    Arg::Scalar(value.into())
}
struct Restore(Vec<(Table, Value, Value)>);
impl Restore {
    fn new() -> Self {
        Self(vec![])
    }
    fn save(&mut self, table: &Table, key: Value) {
        self.0
            .push((table.clone(), key.clone(), table.raw_get(key).unwrap()));
    }
    fn field(&mut self, lua: &Lua, table: &Table, key: &str) {
        self.save(table, Value::String(lua.create_string(key).unwrap()));
    }
    fn restore(&mut self) {
        for (table, key, value) in self.0.drain(..).rev() {
            table.raw_set(key, value).unwrap();
        }
    }
}
impl Drop for Restore {
    fn drop(&mut self) {
        self.restore();
    }
}
struct Pair {
    observed: ObservedSourceSession,
    native: ProgramSession,
    roots: Vec<SessionValue>,
    lua_roots: BTreeMap<String, Value>,
    lua_callbacks: BTreeMap<String, Function>,
    rows: Vec<Json>,
}
impl Pair {
    fn new(
        lua: &Lua,
        primitives: &Primitives,
        config: &Table,
        controls: &[Table; 2],
        vars: &[String; 2],
        fields: &[&str],
    ) -> Self {
        let extras = BTreeMap::from([
            (
                "va".into(),
                Value::String(lua.create_string(&vars[0]).unwrap()),
            ),
            (
                "vb".into(),
                Value::String(lua.create_string(&vars[1]).unwrap()),
            ),
        ]);
        let selected = BTreeMap::from([
            ("a".into(), controls[0].clone()),
            ("b".into(), controls[1].clone()),
        ]);
        let observed =
            capture_with_fields(lua, primitives, config, selected, extras.clone(), fields);
        let library = compile(&observed);
        let (native, roots) = library
            .session_from_input(observed.input(), ProgramLimits::default())
            .unwrap();
        let mut lua_roots = extras;
        lua_roots.insert("config".into(), Value::Table(config.clone()));
        lua_roots.insert("a".into(), Value::Table(controls[0].clone()));
        lua_roots.insert("b".into(), Value::Table(controls[1].clone()));
        let probes: Table = lua
            .load(PROBES)
            .set_name("@tests/support/source_configuration_controls.lua")
            .eval()
            .unwrap();
        let mut lua_callbacks = probes
            .pairs::<String, Function>()
            .map(|row| {
                let (name, function) = row.unwrap();
                (format!("probe.{name}"), function)
            })
            .collect::<BTreeMap<_, _>>();
        let class: Table = lua
            .globals()
            .get::<Table>("common")
            .unwrap()
            .get::<Table>("classes")
            .unwrap()
            .get("EditControl")
            .unwrap();
        lua_callbacks.insert(
            "set_placeholder".into(),
            primitives.unwrap(&class.get("SetPlaceholder").unwrap(), "original"),
        );
        Self {
            observed,
            native,
            roots,
            lua_roots,
            lua_callbacks,
            rows: vec![],
        }
    }
    fn arguments(&mut self, args: &[Arg]) -> (Vec<SessionValue>, MultiValue) {
        let mut native = vec![];
        let mut lua = MultiValue::new();
        for arg in args {
            let (n, l) = match arg {
                Arg::Root(name) => (
                    root(&self.observed, &self.roots, name),
                    self.lua_roots[*name].clone(),
                ),
                Arg::Scalar(value) => {
                    let input = match value {
                        Value::Nil => ProgramValue::Nil,
                        Value::Boolean(value) => ProgramValue::Boolean(*value),
                        Value::Number(value) => ProgramValue::Number(*value),
                        Value::Integer(value) => ProgramValue::Number(*value as f64),
                        Value::String(value) => ProgramValue::Bytes(value.as_bytes().to_vec()),
                        _ => panic!("scalar test argument"),
                    };
                    (
                        self.native
                            .borrow(&ProgramValueGraph {
                                values: vec![input],
                                tables: vec![],
                            })
                            .unwrap()
                            .remove(0),
                        value.clone(),
                    )
                }
            };
            native.push(n);
            lua.push_back(l);
        }
        (native, lua)
    }
    fn execute(
        &mut self,
        name: &str,
        args: &[Arg],
        expected: Option<ProgramRuntimeErrorKind>,
    ) -> Json {
        let (native_args, lua_args) = self.arguments(args);
        let native = self
            .native
            .invoke_callable(&root(&self.observed, &self.roots, name), &native_args);
        let original = self.lua_callbacks[name].call::<MultiValue>(lua_args);
        match expected {
            None => {
                let native = native.unwrap();
                let original = original.unwrap().into_vec();
                let native = observation::canonical(self.native.snapshot(&native).unwrap().graph());
                let original = observation::canonical(&observation::capture(&original));
                assert_eq!(native, original, "continuing {name}");
                json!({"native":"success","original":"success","result":native})
            }
            Some(kind) => {
                let native = native.unwrap_err();
                let original = original.unwrap_err();
                assert_eq!(native.kind, kind, "{native}");
                json!({"native_kind":format!("{:?}",native.kind),"native_error":native.to_string(),"original_kind":"source_call_error","original_error":original.to_string()})
            }
        }
    }
    fn state(&mut self) -> Json {
        self.execute(
            "probe.continuing_result",
            &[
                Arg::Root("a"),
                Arg::Root("b"),
                Arg::Root("config"),
                Arg::Root("va"),
                Arg::Root("vb"),
            ],
            None,
        )["result"]
            .clone()
    }
    fn case(
        &mut self,
        label: &str,
        name: &str,
        args: &[Arg],
        expected: Option<ProgramRuntimeErrorKind>,
    ) -> (Json, Json) {
        let before = self.state();
        let outcome = self.execute(name, args, expected);
        let after = self.state();
        self.rows
            .push(json!({"case":label,"before":before,"outcome":outcome,"after":after}));
        (before, after)
    }
    fn flag(&mut self) {
        self.execute(
            "probe.set_flag",
            &[Arg::Root("config"), scalar(Value::Boolean(false))],
            None,
        );
    }
    fn verify_cells(&self) {
        let input = self.observed.input();
        let closure = |name| {
            let SourceSessionValue::Table(id) =
                input.state.values[self.observed.root_index(name).unwrap()]
            else {
                panic!("control table")
            };
            let (_, SourceSessionValue::Closure(id)) = input.state.tables[id.0 as usize - 1]
                .entries
                .iter()
                .find(|(key, _)| *key == SourceSessionValue::Bytes(b"changeFunc".to_vec()))
                .unwrap()
            else {
                panic!("actual control closure")
            };
            &input.closures[id.0 as usize - 1]
        };
        let a = closure("a");
        let b = closure("b");
        assert_eq!(a.prototype.id(), b.prototype.id());
        let callback = self
            .observed
            .owner()
            .callback(a.prototype.definition().callback)
            .unwrap();
        let slot = |name| {
            callback
                .upvalues
                .iter()
                .position(|capture| capture.name == name)
                .unwrap()
        };
        let self_slot = slot("self");
        let option_slot = slot("varData");
        assert_eq!(
            a.captures[self_slot], b.captures[self_slot],
            "actual controls share ConfigTab capture cell"
        );
        assert_ne!(
            a.captures[option_slot], b.captures[option_slot],
            "loop option cells remain distinct"
        );
        for c in [a, b] {
            assert!(matches!(
                input.cells[c.captures[option_slot].0 as usize - 1],
                SourceSessionValue::DefinitionTable(_)
            ));
            assert!(matches!(
                callback.upvalues[self_slot].value,
                SourceValue::LiveCapture {}
            ));
        }
    }
}
pub(super) fn run(lua: &Lua, primitives: &Primitives, reached: &[Reached]) -> Json {
    let first = reached.first().unwrap();
    let second = reached
        .iter()
        .find(|row| row.config.to_pointer() == first.config.to_pointer() && row.var != first.var)
        .unwrap();
    let config = &first.config;
    let controls = [first.control.clone(), second.control.clone()];
    let vars = [first.var.clone(), second.var.clone()];
    let build: Table = config.raw_get("build").unwrap();
    let sets: Table = config.raw_get("configSets").unwrap();
    let active: Value = config.raw_get("activeConfigSetId").unwrap();
    let active_set: Table = sets.raw_get(active.clone()).unwrap();
    let placeholders: Table = active_set.raw_get("placeholder").unwrap();
    let inputs: Table = active_set.raw_get("input").unwrap();
    let new_id = Value::String(lua.create_string("__r2g_continuing_set").unwrap());
    assert!(matches!(
        sets.raw_get::<Value>(new_id.clone()).unwrap(),
        Value::Nil
    ));
    let mut restore = Restore::new();
    for name in ["activeConfigSetId", "input", "placeholder", "AddUndoState"] {
        restore.field(lua, config, name);
    }
    restore.field(lua, &build, "buildFlag");
    restore.save(&sets, new_id.clone());
    for control in &controls {
        for name in ["placeholder", "changeFunc"] {
            restore.field(lua, control, name);
        }
    }
    for var in &vars {
        restore.field(lua, &placeholders, var);
        restore.field(lua, &inputs, var);
    }
    let initial = [
        raw_result(&controls[0], config, &vars[0]),
        raw_result(&controls[1], config, &vars[1]),
    ];
    let mut pair = Pair::new(
        lua,
        primitives,
        config,
        &controls,
        &vars,
        &["placeholder", "changeFunc"],
    );
    pair.verify_cells();
    pair.case(
        "same and distinct actual closure identities",
        "probe.identities",
        &[Arg::Root("a"), Arg::Root("a"), Arg::Root("b")],
        None,
    );
    for (label, notify) in [
        ("notify false", Value::Boolean(false)),
        ("notify nil", Value::Nil),
    ] {
        pair.flag();
        let (before, after) = pair.case(
            label,
            "set_placeholder",
            &[
                Arg::Root("a"),
                scalar(Value::String(lua.create_string("42").unwrap())),
                scalar(notify),
            ],
            None,
        );
        assert_eq!(before["values"][2], after["values"][2]);
        assert_eq!(after["values"][6], json!({"boolean":false}));
    }
    for text in ["0", "-7", "2.5", "", "invalid"] {
        pair.flag();
        pair.case(
            &format!("numeric string {text:?}"),
            "set_placeholder",
            &[
                Arg::Root("a"),
                scalar(Value::String(lua.create_string(text).unwrap())),
                scalar(Value::Boolean(true)),
            ],
            None,
        );
    }
    pair.flag();
    pair.case(
        "nil placeholder conversion",
        "probe.call_change",
        &[
            Arg::Root("a"),
            scalar(Value::Nil),
            scalar(Value::Boolean(true)),
        ],
        None,
    );
    pair.case(
        "create test-only configuration set",
        "probe.add_set",
        &[Arg::Root("config"), scalar(new_id.clone())],
        None,
    );
    pair.case(
        "switch test-only active set aliases",
        "probe.switch",
        &[Arg::Root("config"), scalar(new_id.clone())],
        None,
    );
    pair.flag();
    pair.case(
        "reuse original control after active-set switch",
        "set_placeholder",
        &[
            Arg::Root("a"),
            scalar(Value::String(lua.create_string("19.5").unwrap())),
            scalar(Value::Boolean(true)),
        ],
        None,
    );
    pair.flag();
    pair.case(
        "second actual control shares active ConfigTab",
        "set_placeholder",
        &[
            Arg::Root("b"),
            scalar(Value::String(lua.create_string("-3").unwrap())),
            scalar(Value::Boolean(true)),
        ],
        None,
    );
    let mut rows = pair.rows;
    // Fault observations use explicitly removed source dependencies. They retain
    // complete original method bodies and compare only the reached state prefix.
    let mut faults = vec![];
    let globals = lua.globals();
    let mut global_restore = Restore::new();
    global_restore.field(lua, &globals, "tostring");
    globals.raw_set("tostring", false).unwrap();
    let mut failure = Pair::new(
        lua,
        primitives,
        config,
        &controls,
        &vars,
        &["placeholder", "changeFunc"],
    );
    failure.flag();
    let (before, after) = failure.case(
        "tostring failure before placeholder write",
        "set_placeholder",
        &[
            Arg::Root("a"),
            scalar(Value::Integer(7)),
            scalar(Value::Boolean(true)),
        ],
        Some(ProgramRuntimeErrorKind::Source),
    );
    assert_eq!(before, after);
    faults.extend(failure.rows);
    global_restore.restore();
    let original_change: Value = controls[0].raw_get("changeFunc").unwrap();
    controls[0].raw_set("changeFunc", true).unwrap();
    let mut failure = Pair::new(lua, primitives, config, &controls, &vars, &["placeholder"]);
    for (label, notify) in [
        (
            "unavailable changeFunc bypassed by false",
            Value::Boolean(false),
        ),
        ("unavailable changeFunc bypassed by nil", Value::Nil),
    ] {
        failure.flag();
        let (before, after) = failure.case(
            label,
            "set_placeholder",
            &[
                Arg::Root("a"),
                scalar(Value::String(lua.create_string(label).unwrap())),
                scalar(notify),
            ],
            None,
        );
        assert_ne!(before["values"][0], after["values"][0]);
        assert_eq!(before["values"][2], after["values"][2]);
        assert_eq!(after["values"][6], json!({"boolean":false}));
    }
    rows.append(&mut failure.rows);
    failure.flag();
    let (before, after) = failure.case(
        "unavailable changeFunc after placeholder write",
        "set_placeholder",
        &[
            Arg::Root("a"),
            scalar(Value::String(lua.create_string("81").unwrap())),
            scalar(Value::Boolean(true)),
        ],
        Some(ProgramRuntimeErrorKind::UnsupportedCapability),
    );
    assert_ne!(before["values"][0], after["values"][0]);
    assert_eq!(before["values"][2], after["values"][2]);
    assert_eq!(after["values"][6], json!({"boolean":false}));
    faults.extend(failure.rows);
    controls[0].raw_set("changeFunc", original_change).unwrap();
    config.raw_set("AddUndoState", false).unwrap();
    let mut failure = Pair::new(
        lua,
        primitives,
        config,
        &controls,
        &vars,
        &["placeholder", "changeFunc"],
    );
    failure.flag();
    let (before, after) = failure.case(
        "input write before unavailable AddUndoState",
        "probe.call_change",
        &[
            Arg::Root("a"),
            scalar(Value::String(lua.create_string("33.5").unwrap())),
            scalar(Value::Boolean(false)),
        ],
        Some(ProgramRuntimeErrorKind::UnsupportedCapability),
    );
    assert_ne!(before["values"][4], after["values"][4]);
    assert_eq!(
        after["values"][4],
        json!({"number_bits":format!("{:016x}",33.5f64.to_bits())})
    );
    assert_eq!(after["values"][6], json!({"boolean":false}));
    faults.extend(failure.rows);
    restore.restore();
    assert_eq!(
        config.raw_get::<Value>("activeConfigSetId").unwrap(),
        active
    );
    assert!(matches!(sets.raw_get::<Value>(new_id).unwrap(), Value::Nil));
    assert_eq!(
        initial,
        [
            raw_result(&controls[0], config, &vars[0]),
            raw_result(&controls[1], config, &vars[1])
        ]
    );
    json!({"cases":rows,"faults":faults,"source_state_restored":true,"full_set_active_admitted":false,"BuildModList":"unentered behind unavailable AddUndoState"})
}
