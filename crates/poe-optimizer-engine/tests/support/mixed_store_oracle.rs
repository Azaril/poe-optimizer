//! Authenticated original Lua query oracle; no native calculation is imported here.
use mlua::{Function, HookTriggers, Lua, Table, Value as LuaValue, VmState};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum Observed {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(String),
    Other(&'static str),
    Rows(Vec<(String, Observed)>),
}

pub struct Oracle {
    lua: Lua,
    warm: bool,
}

fn section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let at = source
        .find(start)
        .unwrap_or_else(|| panic!("missing source anchor {start}"));
    &source[at..at + source[at..].find(end).unwrap()]
}

pub(super) fn source(path: &str) -> String {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../poe-optimizer-pob/data/pob-source-manifest.json"
    ))
    .unwrap();
    assert_eq!(
        manifest["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let value = std::fs::read_to_string(repo.join("vendor/path-of-building-poe2").join(path))
        .unwrap()
        .replace("\r\n", "\n");
    let row = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == path)
        .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(value.as_bytes())),
        row["sha256"].as_str().unwrap(),
        "source changed: {path}"
    );
    value
}

fn lua_value(lua: &Lua, value: &Value) -> mlua::Result<LuaValue> {
    Ok(match value {
        Value::Null => LuaValue::Nil,
        Value::Bool(v) => LuaValue::Boolean(*v),
        Value::Number(v) => LuaValue::Number(v.as_f64().unwrap()),
        Value::String(v) => LuaValue::String(lua.create_string(v)?),
        Value::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.raw_set(index + 1, lua_value(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
        Value::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                table.raw_set(key.as_str(), lua_value(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
    })
}

impl Oracle {
    pub fn new(warm: bool) -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(64 * 1024 * 1024).unwrap();
        let common = source("src/Modules/Common.lua");
        lua.load(format!(
            "local s_format=string.format; local m_floor=math.floor; common={{}}\n{}\n{}\n{}",
            section(&common, "-- Class library\n", "function codePointToUTF8"),
            section(
                &common,
                "function round(val, dec)\n",
                "\n--- Rounds down a number"
            ),
            section(&common, "function copyTable(tbl, noRecurse)\n", "\ndo\n")
        ))
        .set_name("@source-class-and-standard-helpers")
        .exec()
        .unwrap();
        lua.load(source("src/Data/Global.lua"))
            .set_name("@src/Data/Global.lua")
            .exec()
            .unwrap();
        lua.load("modLib={};data={}").exec().unwrap();
        let tools = source("src/Modules/ModTools.lua");
        lua.load(section(
            &tools,
            "function modLib.createMod(",
            "\nmodLib.parseMod,",
        ))
        .set_name("@source-ModTools-createMod")
        .exec()
        .unwrap();
        for path in [
            "src/Classes/ModStore.lua",
            "src/Classes/ModDB.lua",
            "src/Classes/ModList.lua",
        ] {
            lua.load(source(path))
                .set_name(format!("@{path}"))
                .exec()
                .unwrap();
        }
        lua.load(include_str!("mixed_store_oracle.lua"))
            .set_name("@condition-producer-test-driver")
            .exec()
            .unwrap();
        lua.globals()
            .get::<Function>("condition_oracle_jit")
            .unwrap()
            .call::<()>(warm)
            .unwrap();
        Self { lua, warm }
    }

    pub fn query(&self, fixture: &Value) -> mlua::Result<Observed> {
        let query: Function = self
            .lua
            .globals()
            .get::<Function>("condition_oracle_prepare")?
            .call(lua_value(&self.lua, fixture)?)?;
        let value = self
            .lua
            .globals()
            .get::<Function>("condition_oracle_run")?
            .call::<LuaValue>((query, if self.warm { 2048 } else { 1 }))?;
        Ok(match value {
            LuaValue::Nil => Observed::Nil,
            LuaValue::Boolean(value) => Observed::Boolean(value),
            LuaValue::Integer(value) => Observed::Number(value as f64),
            LuaValue::Number(value) => Observed::Number(value),
            LuaValue::String(value) => Observed::Text(value.to_str()?.to_owned()),
            LuaValue::Table(table) if table.get::<bool>("__mixed_rows").unwrap_or(false) => {
                let mut rows = Vec::new();
                for row in table.sequence_values::<Table>() {
                    let row = row?;
                    let observed = match row.get::<LuaValue>("value")? {
                        LuaValue::Number(v) => Observed::Number(v),
                        LuaValue::Integer(v) => Observed::Number(v as f64),
                        LuaValue::Boolean(v) => Observed::Boolean(v),
                        other => panic!("unsupported observed row value {other:?}"),
                    };
                    rows.push((row.get::<String>("id")?, observed));
                }
                Observed::Rows(rows)
            }
            LuaValue::Table(_) => Observed::Other("table"),
            LuaValue::Function(_) => Observed::Other("function"),
            _ => Observed::Other("other"),
        })
    }

    pub fn completed_trace_functions(&self) -> BTreeSet<String> {
        let table: Table = self
            .lua
            .globals()
            .get::<Function>("condition_oracle_traces")
            .unwrap()
            .call(())
            .unwrap();
        table
            .sequence_values::<String>()
            .map(Result::unwrap)
            .collect()
    }

    /// An instruction bound for deliberately cyclic/error cases. Those execute
    /// interpreted; a VM hook is not presented as evidence of warmed traces.
    pub fn bound_instructions(&self) {
        assert!(!self.warm);
        let calls = Arc::new(AtomicUsize::new(0));
        self.lua
            .set_hook(
                HookTriggers::new().every_nth_instruction(1000),
                move |_, _| {
                    if calls.fetch_add(1, Ordering::Relaxed) >= 20 {
                        return Err(mlua::Error::RuntimeError(
                            "condition oracle instruction budget exhausted".into(),
                        ));
                    }
                    Ok(VmState::Continue)
                },
            )
            .unwrap();
    }
}
