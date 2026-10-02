//! Legacy parser-reference observer wiring and source inventory.
//! This helper does not construct or capture PoB UI/class instances.
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_pob::{runtime::RuntimeError, source_programs::capture::SourceClosureObserver};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
pub struct Primitives {
    pub observer: SourceClosureObserver,
    getupvalue: Function,
}
impl Primitives {
    pub fn before_source(lua: &Lua) -> Result<Self, RuntimeError> {
        Self::before_source_inner(lua, false)
    }
    #[allow(dead_code)]
    pub fn before_source_with_constructors(lua: &Lua) -> Result<Self, RuntimeError> {
        Self::before_source_inner(lua, true)
    }
    #[allow(dead_code)]
    pub fn before_source_with_closures(lua: &Lua) -> Result<Self, RuntimeError> {
        let observer = SourceClosureObserver::capture_before_source_with_closures(
            lua,
            SourceTableRuntimeProfile::luajit21_x64_single(),
        )
        .map_err(|e| RuntimeError::Setup(e.to_string()))?;
        Ok(Self {
            observer,
            getupvalue: lua.globals().get::<Table>("debug")?.get("getupvalue")?,
        })
    }
    fn before_source_inner(lua: &Lua, constructors: bool) -> Result<Self, RuntimeError> {
        // This host uses the locked vendored LuaJIT build. Non-reflectable build
        // flags are an explicit adapter attestation; source names prove none.
        let observer = if constructors {
            SourceClosureObserver::capture_before_source_with_constructors(
                lua,
                SourceTableRuntimeProfile::luajit21_x64_single(),
            )
        } else {
            SourceClosureObserver::capture_before_source(lua)
        }
        .map_err(|e| RuntimeError::Setup(e.to_string()))?;
        Ok(Self {
            observer,
            getupvalue: lua.globals().get::<Table>("debug")?.get("getupvalue")?,
        })
    }
    #[allow(dead_code)]
    pub fn captured_value(&self, function: &Function, name: &str) -> Value {
        for slot in 1..=256 {
            let raw: MultiValue = self.getupvalue.call((function.clone(), slot)).unwrap();
            if raw.is_empty() {
                break;
            }
            if let Value::String(key) = &raw[0]
                && key.to_str().unwrap() == name
            {
                return raw[1].clone();
            }
        }
        panic!("original closure missing capture {name}")
    }
    pub fn unwrap(&self, function: &Function, name: &str) -> Function {
        for slot in 1..=256 {
            let raw: MultiValue = self.getupvalue.call((function.clone(), slot)).unwrap();
            if raw.is_empty() {
                break;
            }
            let Value::String(key) = &raw[0] else {
                panic!("upvalue name")
            };
            if key.to_str().unwrap() == name {
                let Value::Function(value) = &raw[1] else {
                    panic!("instrumentation original is callable")
                };
                return value.clone();
            }
        }
        panic!("instrumentation must capture original {name}")
    }
}
pub fn inventory(
    lua: &Lua,
    root: &Path,
    texts: &BTreeMap<String, String>,
) -> (ItemLoadingSource, BTreeMap<String, String>) {
    let mut aliases = BTreeMap::new();
    let mut order = Vec::new();
    let modules: Table = lua.globals().get("_configuration_source_modules").unwrap();
    for name in modules.sequence_values::<String>() {
        let name = name.unwrap();
        let name = name.strip_prefix('@').expect("loaded file source");
        let normalized = name.replace('\\', "/");
        let resolved = if Path::new(&normalized).is_absolute() {
            Path::new(&normalized).canonicalize().unwrap()
        } else {
            root.join("src").join(&normalized).canonicalize().unwrap()
        };
        let matched = texts.keys().find(|path| {
            root.join(path)
                .canonicalize()
                .is_ok_and(|expected| expected == resolved)
        });
        if let Some(path) = matched {
            aliases.insert(name.into(), path.clone());
            order.push(path.clone());
        }
    }
    for path in texts.keys().filter(|path| path.starts_with("tests/")) {
        order.push(path.clone());
    }
    assert!(
        texts.keys().all(|path| order.contains(path)),
        "all supplied source modules observed entering: {order:?}"
    );
    let source = ItemLoadingSource {
        upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
        files: texts
            .iter()
            .map(|(p, t)| (p.clone(), format!("{:x}", Sha256::digest(t.as_bytes()))))
            .collect(),
        construction_spans: BTreeMap::new(),
        module_order: order,
    };
    (source, aliases)
}
