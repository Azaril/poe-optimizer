//! Original full Item methods and retained helper identities; no source replacement.
use super::{reference, source};
use mlua::{Function, Lua, Table, Value, ffi};
use poe_optimizer_data::item_loading::ItemLoadingCatalog;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};

fn reflected(lua: &Lua, function: &Function, slot: i32) -> (String, Value) {
    let mut failed = false;
    // SAFETY: rooted same-host Function, reserved stack space, checked capture
    // name. No bytecode/upvalue mutation; failure returns no fabricated values.
    let result: mlua::Result<(String, Value)> = unsafe {
        lua.exec_raw(function.clone(), |state| {
            if ffi::lua_checkstack(state, 4) == 0 {
                failed = true;
                ffi::lua_settop(state, 0);
                return;
            }
            let name = ffi::lua_getupvalue(state, 1, slot);
            if name.is_null() {
                failed = true;
                ffi::lua_settop(state, 0);
                return;
            }
            ffi::lua_pushstring(state, name);
            ffi::lua_insert(state, -2);
            ffi::lua_remove(state, 1);
        })
    };
    assert!(!failed, "bounded original capture reflection failed");
    result.unwrap()
}
pub fn upvalue(lua: &Lua, function: &Function, wanted: &str) -> Value {
    let mut found = None;
    for slot in 1..=i32::from(function.info().num_upvalues) {
        let (name, value) = reflected(lua, function, slot);
        if name == wanted {
            assert!(found.is_none());
            found = Some(value);
        }
    }
    found.unwrap_or_else(|| panic!("missing original capture {wanted}"))
}
fn original(lua: &Lua, entry: &Function) -> Function {
    let mut current = entry.clone();
    for _ in 0..8 {
        let info = current.info();
        if info.source.as_deref() == Some("@src/Classes/Item.lua") {
            assert_eq!(
                (info.line_defined, info.last_line_defined),
                (Some(468), Some(1803))
            );
            return current;
        }
        let mut next = None;
        for name in ["parseRaw", "originalParse"] {
            // Delegating wrappers have one of these exact captured names.
            for slot in 1..=i32::from(current.info().num_upvalues) {
                let (actual, value) = reflected(lua, &current, slot);
                if actual == name {
                    assert!(next.is_none());
                    next = Some(value.as_function().unwrap().clone());
                }
            }
        }
        current = next.expect("delegating original capture");
    }
    panic!("original capture bound")
}
pub struct Oracle {
    pub source: source::Source,
    class: Table,
    entry: Function,
    pub parse: Function,
    pub sort: Function,
    pub normalize: Function,
}
impl Oracle {
    pub fn new() -> Self {
        let source = source::Source::new();
        let lua = &source.oracle.lua;
        let class: Table = lua
            .globals()
            .get::<Table>("common")
            .unwrap()
            .get::<Table>("classes")
            .unwrap()
            .get("Item")
            .unwrap();
        let entry: Function = class.get("ParseRaw").unwrap();
        let parse = original(lua, &entry);
        let sort = upvalue(lua, &parse, "sortCraftedModLines")
            .as_function()
            .unwrap()
            .clone();
        let normalize = upvalue(lua, &parse, "normaliseModLine")
            .as_function()
            .unwrap()
            .clone();
        for (f, first, last) in [(&sort, 73, 88), (&normalize, 65, 69)] {
            let info = f.info();
            assert_eq!(info.source.as_deref(), Some("@src/Classes/Item.lua"));
            assert_eq!(
                (info.line_defined, info.last_line_defined),
                (Some(first), Some(last))
            );
        }
        Self {
            source,
            class,
            entry,
            parse,
            sort,
            normalize,
        }
    }
    pub fn verify(&self) {
        let lua = &self.source.oracle.lua;
        assert_eq!(self.class.get::<Function>("ParseRaw").unwrap(), self.entry);
        assert_eq!(original(lua, &self.entry), self.parse);
        assert_eq!(
            upvalue(lua, &self.parse, "sortCraftedModLines"),
            Value::Function(self.sort.clone())
        );
        assert_eq!(
            upvalue(lua, &self.parse, "normaliseModLine"),
            Value::Function(self.normalize.clone())
        );
    }
    pub fn cache(&self) -> Value {
        upvalue(&self.source.oracle.lua, &self.parse, "uniqueModStatOrder")
    }
    pub fn install(&self, base: &ItemLoadingCatalog, script: &str) -> ItemLoadingCatalog {
        assert!(
            self.cache().is_nil(),
            "custom input must precede original lazy cache creation"
        );
        let table: Table = self
            .source
            .oracle
            .lua
            .load(script)
            .set_name("@test-owned-exclusive-catalog")
            .eval()
            .unwrap();
        let metadata = reference::metadata(table.clone());
        self.source
            .oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemMods")
            .unwrap()
            .set("Exclusive", table)
            .unwrap();
        let mut data = base.data().clone();
        data.modifier_tables.insert("Exclusive".into(), metadata);
        ItemLoadingCatalog::new(data).unwrap()
    }
    pub fn provenance(&self) -> Json {
        self.verify();
        let text = super::runtime::verified("src/Classes/Item.lua").unwrap();
        json!({"path":"src/Classes/Item.lua","crlf_normalized_sha256":format!("{:x}",Sha256::digest(text.as_bytes())),"ParseRaw":[468,1803],"normaliseModLine":[65,69],"sortCraftedModLines":[73,88],"identity":"actual retained full methods and captures, rechecked"})
    }
}
// Test snapshots are finite bounded trees, explicitly not an Item alias graph.
pub fn canonical(value: Value) -> Json {
    fn visit(value: Value, depth: usize, work: &mut usize, bytes: &mut usize) -> Json {
        assert!(depth <= 32);
        *work += 1;
        assert!(*work <= 65536);
        match value {
            Value::Nil => json!(["nil"]),
            Value::Boolean(v) => json!(["boolean", v]),
            Value::Integer(v) => json!(["number", format!("{:016x}", (v as f64).to_bits())]),
            Value::Number(v) => json!(["number", format!("{:016x}", v.to_bits())]),
            Value::String(v) => {
                let data = v.as_bytes();
                *bytes += data.len();
                assert!(*bytes <= 4 * 1024 * 1024);
                json!(["bytes", data.as_ref()])
            }
            Value::Table(table) => {
                let mut rows = Vec::new();
                for row in table.pairs::<Value, Value>() {
                    let (k, v) = row.unwrap();
                    assert!(rows.len() < 4096);
                    rows.push((
                        visit(k, depth + 1, work, bytes),
                        visit(v, depth + 1, work, bytes),
                    ));
                }
                rows.sort_by_key(|r| r.0.to_string());
                json!(["table", rows])
            }
            other => panic!("unrepresented finite snapshot {other:?}"),
        }
    }
    visit(value, 0, &mut 0, &mut 0)
}
pub fn rows(table: &Table) -> Vec<(String, Option<f64>)> {
    let lines: Table = table.get("explicitModLines").unwrap();
    assert!(lines.raw_len() <= 128);
    lines
        .sequence_values::<Table>()
        .map(|row| {
            let row = row.unwrap();
            (row.get("line").unwrap(), row.get("order").unwrap())
        })
        .collect()
}
pub fn native_rows(
    rows: &[poe_optimizer_import::item_loading::LoadedModLine],
) -> Vec<(String, Option<f64>)> {
    rows.iter()
        .map(|r| (r.line.clone(), r.order.and_then(|v| v.value())))
        .collect()
}

/// Existing runtime observer is inactive in this control; no buff-stage wrappers
/// or new hooks are installed. The declared finite projection still has its
/// historical callback/alias sentinels and is not a whole Item graph claim.
pub struct Control {
    pub oracle: super::runtime::Oracle,
    snapshot: Function,
    parse: Function,
    item: std::cell::RefCell<Option<Table>>,
}
impl Control {
    pub fn new(script: Option<&str>) -> Self {
        let oracle = super::runtime::Oracle::new();
        let lua = &oracle.lua;
        let class: Table = lua
            .globals()
            .get::<Table>("common")
            .unwrap()
            .get::<Table>("classes")
            .unwrap()
            .get("Item")
            .unwrap();
        let parse = original(lua, &class.get::<Function>("ParseRaw").unwrap());
        let snapshot = upvalue(
            lua,
            &class.get::<Function>("BuildModList").unwrap(),
            "snapshot",
        )
        .as_function()
        .unwrap()
        .clone();
        if let Some(script) = script {
            assert!(upvalue(lua, &parse, "uniqueModStatOrder").is_nil());
            let table: Table = lua.load(script).eval().unwrap();
            lua.globals()
                .get::<Table>("data")
                .unwrap()
                .get::<Table>("itemMods")
                .unwrap()
                .set("Exclusive", table)
                .unwrap();
        }
        Self {
            oracle,
            snapshot,
            parse,
            item: std::cell::RefCell::new(None),
        }
    }
    pub fn snapshot(&self, raw: &str, reuse: bool) -> Json {
        assert_eq!(self.parse.info().line_defined, Some(468));
        let item = if reuse {
            let item = self
                .item
                .borrow()
                .as_ref()
                .expect("control must have the same prior Item history")
                .clone();
            // Call the retained complete original on the actual retained receiver.
            // No source function or retained state is replaced or normalized.
            self.parse.call::<()>((item.clone(), raw)).unwrap();
            item
        } else {
            self.oracle.parse(raw)
        };
        *self.item.borrow_mut() = Some(item.clone());
        canonical(Value::Table(self.snapshot.call(item).unwrap()))
    }
    pub fn field(&self, key: &str) -> Value {
        self.item
            .borrow()
            .as_ref()
            .expect("control Item")
            .raw_get(key)
            .unwrap()
    }
}

/// Diagnostics only: equality remains exact over the original canonical value.
/// Report one bounded semantic path instead of dumping the entire source Item.
pub fn first_difference(left: &Json, right: &Json) -> String {
    fn label(key: &Json) -> String {
        if key[0] == "bytes" {
            let bytes = key[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
                .collect::<Vec<_>>();
            String::from_utf8_lossy(&bytes).chars().take(96).collect()
        } else {
            key.to_string().chars().take(96).collect()
        }
    }
    fn walk(left: &Json, right: &Json, path: String, depth: usize) -> String {
        if depth > 32 {
            return format!("{path}: deeper canonical value differs");
        }
        if left[0] == "table" && right[0] == "table" {
            fn rows(value: &Json) -> std::collections::BTreeMap<String, (&Json, &Json)> {
                value[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| (row[0].to_string(), (&row[0], &row[1])))
                    .collect()
            }
            let a = rows(left);
            let b = rows(right);
            for (key, (original, value)) in &a {
                let child = format!("{path}.{}", label(original));
                match b.get(key) {
                    None => return format!("{child}: absent from right"),
                    Some((_, other)) if value != other => {
                        return walk(value, other, child, depth + 1);
                    }
                    _ => {}
                }
            }
            for (key, (original, _)) in &b {
                if !a.contains_key(key) {
                    return format!("{path}.{}: absent from left", label(original));
                }
            }
        }
        let a = left.to_string().chars().take(160).collect::<String>();
        let b = right.to_string().chars().take(160).collect::<String>();
        format!("{path}: left={a}; right={b}")
    }
    walk(left, right, "$".into(), 0)
}
#[test]
fn bounded_difference_diagnostic_preserves_presence_and_value_changes() {
    let key = json!(["bytes", [102, 114, 97, 99, 116, 117, 114, 101, 100]]);
    let left = json!(["table", [[key, ["boolean", true]]]]);
    assert_eq!(
        first_difference(&left, &json!(["table", []])),
        "$.fractured: absent from right"
    );
    let right = json!(["table", [[key, ["boolean", false]]]]);
    let message = first_difference(&left, &right);
    assert!(message.starts_with("$.fractured:"));
    assert!(message.len() < 400);
}
