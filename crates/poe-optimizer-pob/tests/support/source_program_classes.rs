//! Full-runtime test wiring for the production constructed-class observer.
//! Only instrumentation unwrapping and source-inventory selection live here.
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_pob::{
    runtime::RuntimeError,
    source_programs::capture::{
        SourceClassCaptureRequest, SourceClassSelection, SourceClosureObserver,
    },
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
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
pub struct Observed {
    pub owner: SourceProgramOwner,
    pub texts: BTreeMap<String, String>,
    pub callbacks: BTreeMap<String, SourceCallbackId>,
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
pub fn observe(lua: &Lua, primitives: &Primitives, root: &Path, probes: &Table) -> Observed {
    let mut texts = BTreeMap::new();
    for path in [
        "src/Modules/Common.lua",
        "src/Modules/ModTools.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
    ] {
        texts.insert(
            path.into(),
            poe_optimizer_pob::source::read_verified_text(root, path).unwrap(),
        );
    }
    texts.insert(
        "tests/support/source_program_methods.lua".into(),
        include_str!("source_program_methods.lua").into(),
    );
    let classes: Table = lua
        .globals()
        .get::<Table>("common")
        .unwrap()
        .get("classes")
        .unwrap();
    // Explicit source bootstrap, outside the production capture operation.
    let _: Table = lua.load("return new('ModList')").eval().unwrap();
    let selection = vec![
        SourceClassSelection {
            table: classes.get("ModStore").unwrap(),
            methods: BTreeSet::from(["NewMod".into(), "ReplaceMod".into()]),
        },
        SourceClassSelection {
            table: classes.get("ModList").unwrap(),
            methods: BTreeSet::from([
                "NewMod".into(),
                "ReplaceMod".into(),
                "AddMod".into(),
                "ReplaceModInternal".into(),
            ]),
        },
    ];
    let callbacks = probes
        .clone()
        .pairs::<String, Function>()
        .map(|entry| {
            let (name, function) = entry.unwrap();
            (format!("probe.{name}"), function)
        })
        .collect();
    let wrapped: Function = lua.globals().get("new").unwrap();
    assert_eq!(
        wrapped.info().source.as_deref(),
        Some("@configuration-source-observation.lua")
    );
    let allocation = primitives.unwrap(&wrapped, "originalNew");
    let (source, source_names) = inventory(lua, root, &texts);
    let observed = primitives
        .observer
        .observe_classes(
            lua,
            &texts,
            source,
            SourceClassCaptureRequest {
                classes: selection,
                callbacks,
                definition_roots: BTreeMap::new(),
                allocation,
                source_names,
            },
        )
        .unwrap();
    Observed {
        owner: observed.owner().clone(),
        callbacks: observed.callbacks().clone(),
        texts,
    }
}
