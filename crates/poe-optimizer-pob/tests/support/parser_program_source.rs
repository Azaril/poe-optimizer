//! Independent unchanged-source invocation and raw graph comparison for G3.
use super::{factory_source, public_source, runtime};
use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::parser_program::*;
use poe_optimizer_pob::parser_programs::{ParserProgramExtraction, extract_pinned};
use public_source::{Atom, Graph};
use std::{collections::BTreeMap, sync::OnceLock};

pub fn extraction() -> &'static ParserProgramExtraction {
    static EXTRACTION: OnceLock<ParserProgramExtraction> = OnceLock::new();
    EXTRACTION.get_or_init(|| {
        extract_pinned(
            &runtime::repository().join("vendor/path-of-building-poe2"),
            poe_optimizer_data::game_data::bundled_snapshot()
                .unwrap()
                .modifier_parser(),
        )
        .unwrap()
    })
}

#[derive(Clone, Copy, Debug)]
pub enum Target {
    Property,
    Grant,
    Plain,
    Level,
}
impl Target {
    pub const ALL: [Self; 4] = [Self::Property, Self::Grant, Self::Plain, Self::Level];
    pub fn lines(self) -> (u32, u32) {
        match self {
            Self::Property => (3526, 3554),
            Self::Grant => (2193, 2200),
            Self::Plain => (3589, 3589),
            Self::Level => (3590, 3590),
        }
    }
    pub fn id(self, owner: &ModifierParserCatalog) -> ParserCallbackId {
        let (first, last) = self.lines();
        let ids = owner
            .data()
            .callbacks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let ParserCallbackKind::Lua { source } = &c.kind else {
                    return None;
                };
                (source.path == "src/Modules/ModParser.lua"
                    && source.line == first
                    && source.end_line == last)
                    .then_some(ParserCallbackId(i as u32 + 1))
            })
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 1, "unique authenticated closure for {self:?}");
        ids[0]
    }
}

pub struct Source {
    pub factory: factory_source::FactorySource,
    pub lookup: Table,
    pub property: Function,
    pub grant: Function,
    pub plain: Function,
    pub level: Function,
}
impl Source {
    pub fn new() -> Self {
        let factory = factory_source::FactorySource::new();
        let find = |line: usize| {
            let functions = factory
                .special
                .clone()
                .pairs::<Value, Value>()
                .filter_map(|row| {
                    let (_, value) = row.unwrap();
                    let Value::Function(f) = value else {
                        return None;
                    };
                    (f.info().line_defined == Some(line)).then_some(f)
                })
                .collect::<Vec<_>>();
            let first = functions
                .first()
                .expect("actual original special callback")
                .clone();
            assert!(
                functions
                    .iter()
                    .all(|f| f.to_pointer() == first.to_pointer())
            );
            first
        };
        let property = find(3526);
        let plain = find(3589);
        let level = find(3590);
        let lua = &factory.public.source.lua;
        let grant = factory_source::upvalue(lua, &plain, "grantedExtraSkill")
            .as_function()
            .unwrap()
            .clone();
        assert_eq!(grant.info().line_defined, Some(2193));
        assert_eq!(grant.info().last_line_defined, Some(2200));
        assert_eq!(
            factory_source::upvalue(lua, &level, "grantedExtraSkill")
                .as_function()
                .unwrap()
                .to_pointer(),
            grant.to_pointer()
        );
        let lookup = factory_source::upvalue(lua, &grant, "gemIdLookup")
            .as_table()
            .unwrap()
            .clone();
        assert_eq!(
            factory_source::upvalue(lua, &property, "gemIdLookup")
                .as_table()
                .unwrap()
                .to_pointer(),
            lookup.to_pointer()
        );
        lua.load("jit.off(); jit.flush(); assert(not jit.status())")
            .exec()
            .unwrap();
        Self {
            factory,
            lookup,
            property,
            grant,
            plain,
            level,
        }
    }
    pub fn function(&self, target: Target) -> &Function {
        match target {
            Target::Property => &self.property,
            Target::Grant => &self.grant,
            Target::Plain => &self.plain,
            Target::Level => &self.level,
        }
    }
    pub fn text(&self, bytes: impl AsRef<[u8]>) -> Value {
        self.factory.text(bytes.as_ref())
    }
    pub fn table(&self) -> Table {
        self.factory.public.source.lua.create_table().unwrap()
    }
    pub fn args(&self, source: &str) -> MultiValue {
        self.factory
            .public
            .source
            .lua
            .load(source)
            .set_name("@test-only-raw-program-arguments")
            .eval()
            .unwrap()
    }
    pub fn pair(
        &self,
        plan: &CompiledParserPrograms,
        target: Target,
        args: MultiValue,
        label: &str,
    ) -> Graph {
        let input = from_source(&self.factory.public.capture(args.clone()).unwrap());
        let original: MultiValue = self
            .function(target)
            .call(args)
            .unwrap_or_else(|e| panic!("{label}: original {e}"));
        let expected = self.factory.public.capture(original).unwrap();
        let result = plan
            .execute(
                target.id(plan.catalog().owner()),
                &input,
                ProgramLimits::default(),
            )
            .unwrap_or_else(|e| panic!("{label}: native {e:?}"));
        let actual = native_graph(result.graph());
        assert_eq!(actual, expected, "{label}: {target:?} full raw graph");
        expected
    }
    pub fn source_error(
        &self,
        plan: &CompiledParserPrograms,
        target: Target,
        args: MultiValue,
        expected: &str,
    ) {
        let input = from_source(&self.factory.public.capture(args.clone()).unwrap());
        let error = self
            .function(target)
            .call::<MultiValue>(args)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(expected),
            "original source error {error:?}, expected {expected:?}"
        );
        let native = plan
            .execute(
                target.id(plan.catalog().owner()),
                &input,
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_eq!(native.kind, ProgramRuntimeErrorKind::Source, "{native:?}");
        let origin = match target {
            Target::Plain | Target::Level => Target::Grant,
            other => other,
        };
        assert_eq!(
            native.callback,
            Some(origin.id(plan.catalog().owner())),
            "error belongs to the reached original callee"
        );
        let location = native
            .location
            .expect("source error carries authenticated function-relative location");
        assert!(location.start < location.end);
    }
}
pub fn plan() -> CompiledParserPrograms {
    let extraction = extraction();
    for target in Target::ALL {
        let id = target.id(extraction.catalog().owner());
        assert!(
            extraction.catalog().program_id(id).is_some(),
            "{target:?} not lowered: {:?}",
            extraction.unsupported().get(&id)
        );
    }
    CompiledParserPrograms::new(extraction.catalog()).unwrap()
}

pub fn injected(source: &Source, rows: &[(&str, ParserValue)]) -> CompiledParserPrograms {
    let extraction = extraction();
    let mut data = extraction.catalog().owner().data().clone();
    // Raw injected-definition proof is independent of package dispatch permission.
    data.programs.admissions.clear();
    let table_id = data.dictionaries[&ParserDictionary::GemIdLookup];
    let table = &mut data.tables[table_id.0 as usize - 1];
    for (key, value) in rows {
        if matches!(value, ParserValue::Nil) {
            table.fields.remove(*key);
        } else {
            table.fields.insert((*key).into(), value.clone());
        }
        let lua_value = match value {
            ParserValue::Nil => Value::Nil,
            ParserValue::Boolean(v) => Value::Boolean(*v),
            ParserValue::Number(v) => Value::Number(*v),
            ParserValue::Text(v) => source.text(v),
            _ => panic!("use explicit graph fixture for reference/nonfinite definitions"),
        };
        source.lookup.raw_set(*key, lua_value).unwrap();
    }
    let owner = ModifierParserCatalog::new(data).unwrap();
    let catalog = ParserProgramCatalog::new(extraction.catalog().data().clone(), owner).unwrap();
    CompiledParserPrograms::new(&catalog).unwrap()
}

fn from_atom(value: &Atom) -> ProgramValue {
    match value {
        Atom::Nil => ProgramValue::Nil,
        Atom::Boolean(v) => ProgramValue::Boolean(*v),
        Atom::Number(v) => ProgramValue::Number(f64::from_bits(*v)),
        Atom::Bytes(v) => ProgramValue::Bytes(v.clone()),
        Atom::Table(v) => ProgramValue::Table(ProgramTableId(*v as u32 + 1)),
        Atom::Function(_) => {
            panic!("opaque function input requires a separate identity-bound control")
        }
    }
}
pub fn from_source(graph: &Graph) -> ProgramValueGraph {
    assert!(
        graph.callbacks.is_empty(),
        "no opaque function graph normalization"
    );
    ProgramValueGraph {
        values: graph.roots.iter().map(from_atom).collect(),
        tables: graph
            .tables
            .iter()
            .map(|entries| ProgramTable {
                entries: entries
                    .iter()
                    .map(|(k, v)| (from_atom(k), from_atom(v)))
                    .collect(),
            })
            .collect(),
    }
}
struct Capture<'a> {
    input: &'a ProgramValueGraph,
    output: Graph,
    ids: BTreeMap<ProgramTableId, usize>,
}
impl Capture<'_> {
    fn value(&mut self, value: &ProgramValue) -> Atom {
        match value {
            ProgramValue::Nil => Atom::Nil,
            ProgramValue::Boolean(v) => Atom::Boolean(*v),
            ProgramValue::Number(v) => Atom::Number(v.to_bits()),
            ProgramValue::Bytes(v) => Atom::Bytes(v.clone()),
            ProgramValue::Callback(_)
            | ProgramValue::Closure(_)
            | ProgramValue::DefinitionTable(_) => {
                panic!("opaque callback values are a separate boundary, not graph parity")
            }
            ProgramValue::Table(id) => {
                if let Some(index) = self.ids.get(id) {
                    return Atom::Table(*index);
                }
                let index = self.output.tables.len();
                self.ids.insert(*id, index);
                self.output.tables.push(vec![]);
                let raw = self.input.tables[id.0 as usize - 1].entries.clone();
                let mut entries = raw
                    .iter()
                    .map(|(key, value)| {
                        assert!(
                            !matches!(key, ProgramValue::Table(_) | ProgramValue::Callback(_)),
                            "unsupported source observer reference key"
                        );
                        (self.value(key), value)
                    })
                    .collect::<Vec<_>>();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let fields = entries
                    .into_iter()
                    .map(|(key, value)| (key, self.value(value)))
                    .collect();
                self.output.tables[index] = fields;
                Atom::Table(index)
            }
        }
    }
}
pub fn native_graph(input: &ProgramValueGraph) -> Graph {
    let mut capture = Capture {
        input,
        output: Graph::default(),
        ids: BTreeMap::new(),
    };
    capture.output.roots = input.values.iter().map(|v| capture.value(v)).collect();
    capture.output
}
