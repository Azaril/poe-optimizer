//! Test-only capture of exact method closures and an explicitly partial class view.
//! Every omitted class field is declared unsupported. This is not a production
//! class extractor and does not claim a complete graph of all PoB classes.
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    source_program::*,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub struct Primitives {
    functions: BTreeMap<usize, (String, Option<SourceProgramIntrinsic>)>,
    getupvalue: Function,
    getfenv: Function,
    globals: usize,
}
impl Primitives {
    pub fn before_source(lua: &Lua) -> mlua::Result<Self> {
        let mut functions = BTreeMap::new();
        for (symbol, intrinsic) in [
            ("type", Some(SourceProgramIntrinsic::Type)),
            ("select", Some(SourceProgramIntrinsic::Select)),
            ("ipairs", Some(SourceProgramIntrinsic::Ipairs)),
            ("pairs", None),
            ("table.insert", Some(SourceProgramIntrinsic::TableInsert)),
            ("string.format", None),
        ] {
            let mut value = Value::Table(lua.globals());
            for key in symbol.split('.') {
                let Value::Table(table) = value else {
                    panic!("original primitive path")
                };
                value = table.raw_get(key)?;
            }
            let Value::Function(function) = value else {
                panic!("original primitive")
            };
            assert_eq!(function.info().what, "C");
            functions.insert(function.to_pointer() as usize, (symbol.into(), intrinsic));
        }
        Ok(Self {
            functions,
            getupvalue: lua.globals().get::<Table>("debug")?.get("getupvalue")?,
            getfenv: lua.globals().get("getfenv")?,
            globals: lua.globals().to_pointer() as usize,
        })
    }
}
struct Capture<'a> {
    primitives: &'a Primitives,
    texts: BTreeMap<String, String>,
    tables: Vec<SourceTable>,
    callbacks: Vec<SourceCallback>,
    table_ids: BTreeMap<usize, SourceTableId>,
    function_ids: BTreeMap<usize, SourceCallbackId>,
    intrinsics: BTreeMap<SourceCallbackId, SourceProgramIntrinsic>,
}
impl Capture<'_> {
    fn span(&self, function: &Function) -> ItemSourceSpan {
        let info = function.info();
        let source = info
            .source
            .as_deref()
            .expect("source info")
            .trim_start_matches('@')
            .replace('\\', "/");
        let path = self
            .texts
            .keys()
            .find(|path| {
                source.ends_with(path.as_str())
                    || source.ends_with(path.strip_prefix("src/").unwrap_or(path))
            })
            .expect("captured callback has known full source")
            .clone();
        let line = info.line_defined.unwrap() as u32;
        let end_line = info.last_line_defined.unwrap() as u32;
        assert!(line > 0 && end_line >= line);
        let bytes = self.texts[&path]
            .split_inclusive('\n')
            .skip(line as usize - 1)
            .take((end_line - line + 1) as usize)
            .collect::<String>();
        ItemSourceSpan {
            path,
            line,
            end_line,
            sha256: format!("{:x}", Sha256::digest(bytes.as_bytes())),
        }
    }
    fn upvalues(&self, function: &Function) -> Vec<(String, Value)> {
        let mut values = Vec::new();
        for slot in 1..=256 {
            let raw: MultiValue = self
                .primitives
                .getupvalue
                .call((function.clone(), slot))
                .unwrap();
            if raw.is_empty() {
                return values;
            }
            assert_eq!(raw.len(), 2);
            let Value::String(name) = &raw[0] else {
                panic!("capture name")
            };
            values.push((name.to_str().unwrap().to_string(), raw[1].clone()));
        }
        panic!("test capture closure exceeds limit")
    }
    fn function(&mut self, function: &Function) -> SourceCallbackId {
        let pointer = function.to_pointer() as usize;
        if let Some(id) = self.function_ids.get(&pointer) {
            return *id;
        }
        assert!(self.callbacks.len() < 256);
        let id = SourceCallbackId(self.callbacks.len() as u32 + 1);
        self.function_ids.insert(pointer, id);
        let kind = if function.info().what == "C" {
            let (symbol, intrinsic) = self
                .primitives
                .functions
                .get(&pointer)
                .expect("all C captures matched to before-source original pointers");
            if let Some(intrinsic) = intrinsic {
                self.intrinsics.insert(id, *intrinsic);
            }
            SourceCallbackKind::Builtin {
                symbol: symbol.clone(),
            }
        } else {
            let environment: Table = self.primitives.getfenv.call(function.clone()).unwrap();
            assert_eq!(environment.to_pointer() as usize, self.primitives.globals);
            SourceCallbackKind::Lua {
                source: self.span(function),
            }
        };
        self.callbacks.push(SourceCallback {
            kind,
            upvalues: vec![],
            environment: SourceEnvironment::OriginalGlobals,
        });
        // Builtins are authenticated primitive bindings, not Lua closures; their
        // private VM implementation upvalues are outside this public contract.
        if function.info().what == "C" {
            return id;
        }
        let upvalues = self
            .upvalues(function)
            .into_iter()
            .map(|(name, value)| SourceUpvalue {
                name,
                value: self.value(&value),
            })
            .collect();
        self.callbacks[id.0 as usize - 1].upvalues = upvalues;
        id
    }
    fn value(&mut self, value: &Value) -> SourceValue {
        match value {
            Value::Nil => SourceValue::Nil,
            Value::Boolean(value) => SourceValue::Boolean(*value),
            Value::Integer(value) => SourceValue::Number(*value as f64),
            Value::Number(value) if value.is_finite() => SourceValue::Number(*value),
            Value::String(value) => SourceValue::Text(value.to_str().unwrap().to_string()),
            Value::Function(function) => SourceValue::Callback(self.function(function)),
            Value::Table(table) => {
                let pointer = table.to_pointer() as usize;
                if let Some(id) = self.table_ids.get(&pointer) {
                    return SourceValue::Table(*id);
                }
                assert!(
                    table.metatable().is_none(),
                    "unrepresented nonclass metatable capture"
                );
                assert!(self.tables.len() < 256);
                let id = SourceTableId(self.tables.len() as u32 + 1);
                self.table_ids.insert(pointer, id);
                self.tables.push(SourceTable::default());
                let mut captured = SourceTable::default();
                for pair in table.clone().pairs::<Value, Value>() {
                    let (key, value) = pair.unwrap();
                    let value = self.value(&value);
                    match key {
                        Value::String(key) => {
                            captured
                                .fields
                                .insert(key.to_str().unwrap().to_string(), value);
                        }
                        Value::Integer(key) => {
                            captured.indexed.insert(key, value);
                        }
                        Value::Number(key)
                            if key.fract() == 0.0 && key.abs() < (1u64 << 53) as f64 =>
                        {
                            captured.indexed.insert(key as i64, value);
                        }
                        _ => panic!("unrepresented nonclass capture key"),
                    }
                }
                self.tables[id.0 as usize - 1] = captured;
                SourceValue::Table(id)
            }
            _ => panic!("unrepresented source capture value"),
        }
    }
}
pub struct Observed {
    pub owner: SourceProgramOwner,
    pub texts: BTreeMap<String, String>,
    pub callbacks: BTreeMap<String, SourceCallbackId>,
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
    let source_classes = [
        classes.get::<Table>("ModStore").unwrap(),
        classes.get::<Table>("ModList").unwrap(),
    ];
    let mut capture = Capture {
        primitives,
        texts,
        tables: vec![SourceTable::default(); 2],
        callbacks: vec![],
        table_ids: BTreeMap::new(),
        function_ids: BTreeMap::new(),
        intrinsics: BTreeMap::new(),
    };
    for (index, table) in source_classes.iter().enumerate() {
        assert!(table.metatable().is_none());
        capture
            .table_ids
            .insert(table.to_pointer() as usize, SourceTableId(index as u32 + 1));
    }
    let allocated: Table = lua.load("return new('ModList')").eval().unwrap();
    let proxy: Table = allocated.raw_get("ModStore").unwrap();
    let parent_call: Function = proxy.raw_get("__call").unwrap();
    let parent_index: Function = proxy.raw_get("__index").unwrap();
    let observed_new: Function = lua.globals().get("new").unwrap();
    assert_eq!(
        observed_new.info().source.as_deref(),
        Some("@configuration-source-observation.lua")
    );
    let Value::Function(new) = capture
        .upvalues(&observed_new)
        .into_iter()
        .find(|(name, _)| name == "originalNew")
        .expect("observation delegates unchanged new")
        .1
    else {
        panic!("original new function")
    };
    let wrap = capture
        .upvalues(&new)
        .into_iter()
        .find(|(name, _)| name == "wrapConstructor")
        .expect("original new captures wrapper helper")
        .1;
    let Value::Function(wrap) = wrap else {
        panic!("wrapper helper capture")
    };
    let policy = SourceClassConstructionPolicy {
        allocation: capture.span(&new),
        parent_call: capture.span(&parent_call),
        parent_index: capture.span(&parent_index),
        wrap_constructor: capture.span(&wrap),
        parent_call_callback: capture.function(&parent_call),
        parent_index_callback: capture.function(&parent_index),
        object_alias: "Object".into(),
        parent_init: "_parentInit".into(),
        proxy_parent: "_parent".into(),
        proxy_object: "_object".into(),
        proxy_class_name: "_className".into(),
        class_name_field: "_className".into(),
        parent_classes_field: "_parents".into(),
        super_parents_field: "_superParents".into(),
        unconstructed_meta_field: "_unconstructedMeta".into(),
        constructor_initialized_field: "_constructorInitialised".into(),
        parent_call_format_upvalue: capture
            .upvalues(&parent_call)
            .iter()
            .position(|(name, _)| name == "s_format")
            .unwrap() as u16,
    };
    let mut descriptors = Vec::new();
    let mut callbacks = BTreeMap::new();
    for (index, table) in source_classes.iter().enumerate() {
        let name: String = table.raw_get("_className").unwrap();
        let mut descriptor = SourceClassDefinition {
            name: name.clone(),
            table: SourceTableId(index as u32 + 1),
            parents: if index == 1 {
                vec![SourceClassId(1)]
            } else {
                vec![]
            },
            super_parents: match table.raw_get::<Value>("_superParents").unwrap() {
                Value::Nil => None,
                Value::Table(set) => Some(
                    set.pairs::<Table, bool>()
                        .map(|pair| {
                            let (parent, present) = pair.unwrap();
                            assert!(present);
                            SourceClassId(
                                source_classes
                                    .iter()
                                    .position(|class| class.to_pointer() == parent.to_pointer())
                                    .expect("observed superclass identity")
                                    as u32
                                    + 1,
                            )
                        })
                        .collect(),
                ),
                _ => panic!("source superclass set"),
            },
            unsupported_fields: BTreeSet::new(),
            methods: BTreeMap::new(),
            constructor: None,
        };
        let selected = [
            "ModStore",
            "ModList",
            "NewMod",
            "ReplaceMod",
            "AddMod",
            "ReplaceModInternal",
        ];
        let mut fields = BTreeMap::new();
        let mut entries = table
            .clone()
            .pairs::<String, Value>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (key, value) in entries {
            let represented = selected.contains(&key.as_str())
                || matches!(key.as_str(), "__index" | "_unconstructedMeta" | "_parents")
                || matches!(
                    value,
                    Value::Nil
                        | Value::Boolean(_)
                        | Value::Number(_)
                        | Value::Integer(_)
                        | Value::String(_)
                );
            if !represented {
                descriptor.unsupported_fields.insert(key);
                continue;
            }
            let captured = capture.value(&value);
            if selected.contains(&key.as_str()) {
                let SourceValue::Callback(callback) = captured else {
                    panic!("method callback")
                };
                callbacks.insert(format!("{name}.{key}"), callback);
                if key == name {
                    let closure = &capture.callbacks[callback.0 as usize - 1];
                    let original = closure
                        .upvalues
                        .iter()
                        .position(|v| v.name == "originalFunc");
                    descriptor.constructor = Some(if let Some(original_upvalue) = original {
                        let SourceValue::Callback(original) =
                            closure.upvalues[original_upvalue].value
                        else {
                            panic!("original constructor")
                        };
                        SourceClassConstructor {
                            callback: original,
                            wrapper: Some(SourceClassConstructorWrapper {
                                callback,
                                original_upvalue: original_upvalue as u16,
                                class_upvalue: closure
                                    .upvalues
                                    .iter()
                                    .position(|v| v.name == "class")
                                    .unwrap() as u16,
                                pairs_upvalue: closure
                                    .upvalues
                                    .iter()
                                    .position(|v| v.name == "pairs")
                                    .unwrap() as u16,
                                class_name_upvalue: closure
                                    .upvalues
                                    .iter()
                                    .position(|v| v.name == "className")
                                    .unwrap()
                                    as u16,
                            }),
                        }
                    } else {
                        SourceClassConstructor {
                            callback,
                            wrapper: None,
                        }
                    });
                }
                let declared_by =
                    if index == 1 && matches!(key.as_str(), "ModStore" | "NewMod" | "ReplaceMod") {
                        SourceClassId(1)
                    } else {
                        SourceClassId(index as u32 + 1)
                    };
                descriptor.methods.insert(
                    key.clone(),
                    SourceClassMethod {
                        callback,
                        declared_by,
                    },
                );
            }
            fields.insert(key, captured);
        }
        capture.tables[index].fields = fields;
        descriptors.push(descriptor);
    }
    let mut probe_functions = probes
        .clone()
        .pairs::<String, Function>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    probe_functions.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, function) in probe_functions {
        callbacks.insert(format!("probe.{name}"), capture.function(&function));
    }
    let source = ItemLoadingSource {
        upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
        files: capture
            .texts
            .iter()
            .map(|(path, text)| {
                (
                    path.clone(),
                    format!("{:x}", Sha256::digest(text.as_bytes())),
                )
            })
            .collect(),
        construction_spans: BTreeMap::new(),
        module_order: capture.texts.keys().cloned().collect(),
    };
    let owner = SourceProgramOwner::new_with_classes(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source,
            tables: capture.tables,
            callbacks: capture.callbacks,
            roots: vec![],
            intrinsics: capture.intrinsics,
        },
        SourceClassDefinitions {
            schema_version: SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION,
            source: policy,
            classes: descriptors,
        },
    )
    .unwrap();
    Observed {
        owner,
        texts: capture.texts,
        callbacks,
    }
}
