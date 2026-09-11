//! Independent G4 public-wrapper observations and explicit authored test catalogs.
use super::{factory_source, native_observer, public_source, raw_source};
use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::{
    lua_pattern::MatchBudget,
    modifier_parser::{CompiledModifierParser, ParseOutcome, ParserError},
};
use public_source::Graph;
use raw_source::Target;
use std::collections::BTreeMap;

pub struct Original {
    pub raw: raw_source::Source,
    pub scan: Function,
    pub by_pointer: BTreeMap<usize, ParserCallbackId>,
}
impl Original {
    pub fn new(owner: &ModifierParserCatalog) -> Self {
        let raw = raw_source::Source::new();
        let scan = factory_source::upvalue(
            &raw.factory.public.source.lua,
            &raw.factory.internal,
            "scan",
        )
        .as_function()
        .unwrap()
        .clone();
        assert_eq!(scan.info().line_defined, Some(6592));
        assert_eq!(scan.info().last_line_defined, Some(6614));
        let mut by_pointer = BTreeMap::new();
        for (pattern, value) in &owner.dictionary(ParserDictionary::Special).fields {
            if let ParserValue::Callback(id) = value
                && let Value::Function(function) =
                    raw.factory.special.get::<Value>(pattern.as_str()).unwrap()
            {
                by_pointer.insert(function.to_pointer() as usize, *id);
            }
        }
        Self {
            raw,
            scan,
            by_pointer,
        }
    }
    pub fn selection(&self, line: &[u8]) -> (Option<ParserCallbackId>, Vec<u8>, Graph) {
        let (selected, remainder, captures): (Value, mlua::LuaString, Value) = self
            .scan
            .call((self.raw.text(line), self.raw.factory.special.clone()))
            .unwrap();
        let selected = selected
            .as_function()
            .and_then(|f| self.by_pointer.get(&(f.to_pointer() as usize)))
            .copied();
        let captures = self
            .raw
            .factory
            .public
            .capture(MultiValue::from_vec(vec![captures]))
            .unwrap();
        (selected, remainder.as_bytes().to_vec(), captures)
    }
    pub fn alias(&mut self, pattern: &str, function: Function, id: ParserCallbackId) {
        self.by_pointer.insert(function.to_pointer() as usize, id);
        self.raw.factory.add(pattern, function);
    }
    pub fn pair(&self, native: &CompiledModifierParser, line: &[u8]) -> (Graph, ParseOutcome) {
        let expected = self
            .raw
            .factory
            .public
            .raw(line)
            .unwrap_or_else(|e| panic!("original {line:?}: {e}"));
        let expected = self.raw.factory.public.capture(expected).unwrap();
        let actual = native
            .parse(line, &mut MatchBudget::default())
            .unwrap_or_else(|e| panic!("native {line:?}: {e:?}"));
        assert_eq!(
            expected,
            native_observer::capture(&actual, native.catalog()).unwrap(),
            "complete public output {line:?}"
        );
        (expected, actual)
    }
    pub fn source_error(&self, native: &CompiledModifierParser, line: &[u8], message: &str) {
        let source = self.raw.factory.public.raw(line).unwrap_err().to_string();
        assert!(
            source.contains(message),
            "source {source:?}; wanted {message:?}"
        );
        match native.parse(line, &mut MatchBudget::default()) {
            Err(ParserError::Program(error)) => {
                assert_eq!(
                    error.kind,
                    poe_optimizer_engine::parser_program::ProgramRuntimeErrorKind::Source
                );
                assert!(error.callback.is_some() && error.location.is_some());
            }
            Err(ParserError::SourceError(_)) => {}
            other => panic!("native source classification {line:?}: {other:?}"),
        }
    }
    pub fn counters(&mut self, owner: &ModifierParserCatalog) -> Table {
        let lua = &self.raw.factory.public.source.lua;
        let counts = lua.create_table().unwrap();
        let make:Function=lua.load("return function(original,counts,id) return function(...) counts[id]=(counts[id] or 0)+1; return original(...) end end").set_name("@test-only-public-body-call-observer").eval().unwrap();
        let mut wrappers = BTreeMap::new();
        for target in [Target::Property, Target::Plain, Target::Level] {
            let id = target.id(owner);
            let wrapper: Function = make
                .call((self.raw.function(target).clone(), counts.clone(), id.0))
                .unwrap();
            wrappers.insert(id, wrapper);
        }
        for (pattern, value) in &owner.dictionary(ParserDictionary::Special).fields {
            if let ParserValue::Callback(id) = value
                && let Some(wrapper) = wrappers.get(id)
            {
                self.raw
                    .factory
                    .special
                    .raw_set(pattern.as_str(), wrapper.clone())
                    .unwrap();
                self.by_pointer.insert(wrapper.to_pointer() as usize, *id);
            }
        }
        counts
    }
}

pub fn original_programs() -> ParserProgramData {
    raw_source::extraction().catalog().data().clone()
}
pub fn bare_data() -> ModifierParserData {
    let mut data = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .modifier_parser()
        .data()
        .clone();
    data.programs = ParserProgramPayload::default();
    data
}
pub fn rebind(mut data: ModifierParserData, programs: ParserProgramData) -> ModifierParserCatalog {
    data.programs = ParserProgramPayload::default();
    let bare = ModifierParserCatalog::new(data.clone()).unwrap();
    let mut admissions = BTreeMap::new();
    for target in Target::ALL {
        let id = target.id(&bare);
        let program = programs.programs.iter().find(|p| p.callback == id).unwrap();
        let role = if matches!(target, Target::Grant) {
            ParserProgramRole::Helper
        } else {
            ParserProgramRole::Special
        };
        let admission = ParserProgramAdmission::bind(
            &data,
            program,
            role,
            "explicit-authored-G4-source-test; not packaged parity attestation",
        )
        .unwrap();
        admissions.insert(id, admission);
    }
    data.programs = ParserProgramPayload {
        data: programs,
        admissions,
    };
    ModifierParserCatalog::new(data).unwrap()
}
pub fn parser(data: ModifierParserData) -> CompiledModifierParser {
    CompiledModifierParser::new(&rebind(data, original_programs())).unwrap()
}
pub fn add_alias(data: &mut ModifierParserData, pattern: &str, id: ParserCallbackId) {
    let table = data.dictionaries[&ParserDictionary::Special];
    data.tables[table.0 as usize - 1]
        .fields
        .insert(pattern.into(), ParserValue::Callback(id));
}
pub fn set_lookup(data: &mut ModifierParserData, key: &str, value: ParserValue) {
    let table = data.dictionaries[&ParserDictionary::GemIdLookup];
    if matches!(value, ParserValue::Nil) {
        data.tables[table.0 as usize - 1].fields.remove(key);
    } else {
        data.tables[table.0 as usize - 1]
            .fields
            .insert(key.into(), value);
    }
}

pub fn expr(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
pub fn nil() -> ParserProgramExpr {
    expr(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Nil,
    })
}
pub fn bytes(value: &[u8]) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Bytes {
        value: value.to_vec(),
    })
}
pub fn empty_table() -> ParserProgramExpr {
    expr(ParserProgramExprKind::Table { fields: vec![] })
}
pub fn returning(
    mut programs: ParserProgramData,
    id: ParserCallbackId,
    values: Vec<ParserProgramExpr>,
) -> ParserProgramData {
    let program = programs
        .programs
        .iter_mut()
        .find(|p| p.callback == id)
        .unwrap();
    program.body = vec![ParserProgramStatement {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation: ParserProgramStatementKind::Return {
            values: ParserProgramValueList { values, tail: None },
        },
    }];
    programs
}
