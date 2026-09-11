//! Test-only explicit IR fixtures and independent result-graph canonicalization.
//! These authored programs reuse catalog descriptors solely to exercise the raw
//! runtime boundary. No original callback lowering/admission is asserted.
use super::{Atom, Graph, Key};
use poe_optimizer_data::modifier_parser::*;
pub(super) use poe_optimizer_data::modifier_parser::{
    ParserProgramBinary as B, ParserProgramExprKind as E, ParserProgramField as F,
    ParserProgramIntrinsic as I, ParserProgramStatementKind as S, ParserProgramUnary as U,
};
use poe_optimizer_engine::parser_program::*;
use std::{collections::BTreeMap, sync::OnceLock};

struct Fixture {
    owner: ModifierParserCatalog,
    callback: ParserCallbackId,
    constructor: (u16, ParserCallbackId),
    provenance: ParserProgramProvenance,
}
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let mut data = poe_optimizer_data::game_data::bundled_snapshot()
            .unwrap()
            .modifier_parser()
            .data()
            .clone();
        // Authored raw-runtime fixtures do not inherit packaged public permission.
        data.programs = ParserProgramPayload::default();
        let (callback, constructor, provenance) =
            data.factories
                .iter()
                .find_map(|(&id, body)| {
                    let ParserFactoryDisposition::Pure(body) = body else {
                        return None;
                    };
                    let callback = &data.callbacks[id.0 as usize - 1];
                    if callback.upvalues.iter().any(|u| {
                        ["tonumber", "table", "string", "ipairs"].contains(&u.name.as_str())
                    }) {
                        return None;
                    }
                    let constructor = body.provenance.constructor?;
                    let upvalue = callback
                        .upvalues
                        .iter()
                        .position(|u| u.value == ParserValue::Callback(constructor))?
                        as u16;
                    Some((id, (upvalue, constructor), body.provenance.clone()))
                })
                .unwrap();
        data.factories.insert(
            callback,
            ParserFactoryDisposition::Unsupported {
                reason: "synthetic generic program fixture".into(),
            },
        );
        Fixture {
            owner: ModifierParserCatalog::new(data).unwrap(),
            callback,
            constructor,
            provenance: ParserProgramProvenance {
                source: provenance.source,
                function_start: provenance.function_start,
                function_end: provenance.function_end,
                function_sha256: provenance.function_sha256,
            },
        }
    })
}
pub(super) fn loc() -> ParserProgramLocation {
    ParserProgramLocation { start: 0, end: 1 }
}
pub(super) fn e(operation: E) -> ParserProgramExpr {
    ParserProgramExpr {
        location: loc(),
        operation,
    }
}
pub(super) fn s(operation: S) -> ParserProgramStatement {
    ParserProgramStatement {
        location: loc(),
        operation,
    }
}
pub(super) fn l(local: u16) -> ParserProgramExpr {
    e(E::Local { local })
}
pub(super) fn n(value: f64) -> ParserProgramExpr {
    e(E::Literal {
        value: ParserFactoryLiteral::Number(value),
    })
}
pub(super) fn nil() -> ParserProgramExpr {
    e(E::Literal {
        value: ParserFactoryLiteral::Nil,
    })
}
pub(super) fn text(value: &[u8]) -> ParserProgramExpr {
    e(E::Bytes {
        value: value.to_vec(),
    })
}
pub(super) fn boolean(value: bool) -> ParserProgramExpr {
    e(E::Literal {
        value: ParserFactoryLiteral::Boolean(value),
    })
}
pub(super) fn table(fields: Vec<F>) -> ParserProgramExpr {
    e(E::Table { fields })
}
pub(super) fn list(value: ParserProgramExpr) -> F {
    F::List { value }
}
pub(super) fn values(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
pub(super) fn declare(local: u16, value: ParserProgramExpr) -> ParserProgramStatement {
    s(S::Declare {
        locals: vec![local],
        values: values(vec![value]),
    })
}
pub(super) fn ret(value: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(S::Return {
        values: values(value),
    })
}
pub(super) fn binary(
    operation: B,
    left: ParserProgramExpr,
    right: ParserProgramExpr,
) -> ParserProgramExpr {
    e(E::Binary {
        operation,
        left: Box::new(left),
        right: Box::new(right),
    })
}
pub(super) fn unary(operation: U, value: ParserProgramExpr) -> ParserProgramExpr {
    e(E::Unary {
        operation,
        value: Box::new(value),
    })
}
pub(super) fn set(
    table: ParserProgramExpr,
    key: ParserProgramExpr,
    value: ParserProgramExpr,
) -> ParserProgramStatement {
    s(S::TableSet { table, key, value })
}
pub(super) fn append(table: ParserProgramExpr, value: ParserProgramExpr) -> ParserProgramStatement {
    set(
        table.clone(),
        binary(B::Add, unary(U::Length, table), n(1.0)),
        value,
    )
}
pub(super) fn branch(
    condition: ParserProgramExpr,
    body: Vec<ParserProgramStatement>,
) -> ParserProgramStatement {
    s(S::If {
        branches: vec![ParserProgramBranch { condition, body }],
        otherwise: vec![],
    })
}
pub(super) fn call(binding: u16, args: Vec<ParserProgramExpr>) -> ParserProgramCall {
    ParserProgramCall {
        binding,
        receiver: None,
        arguments: values(args),
    }
}
pub(super) fn binding(operation: I) -> ParserProgramBinding {
    ParserProgramBinding::Intrinsic {
        operation,
        source: ParserProgramIntrinsicSource::OriginalGlobal,
    }
}
pub(super) fn compile(
    parameters: u16,
    variadic: bool,
    bindings: Vec<ParserProgramBinding>,
    body: Vec<ParserProgramStatement>,
) -> CompiledParserPrograms {
    let f = fixture();
    let data = ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        programs: vec![ParserProgram {
            callback: f.callback,
            parameter_count: parameters,
            local_count: 32,
            variadic,
            bindings,
            body,
            provenance: f.provenance.clone(),
        }],
        callbacks: BTreeMap::from([(f.callback, ParserProgramId(1))]),
    };
    CompiledParserPrograms::new(&ParserProgramCatalog::new(data, f.owner.clone()).unwrap()).unwrap()
}
pub(super) fn execute(
    plan: &CompiledParserPrograms,
    input: &ProgramValueGraph,
) -> Result<ProgramOutput, ProgramRuntimeError> {
    plan.execute(fixture().callback, input, ProgramLimits::default())
}
pub(super) fn input(values: Vec<ProgramValue>) -> ProgramValueGraph {
    ProgramValueGraph {
        values,
        tables: vec![],
    }
}
fn intern(
    v: &ProgramValue,
    ids: &mut BTreeMap<ProgramTableId, usize>,
    pending: &mut Vec<ProgramTableId>,
) -> Atom {
    match v {
        ProgramValue::Nil => Atom::Nil,
        ProgramValue::Boolean(v) => Atom::Boolean(*v),
        ProgramValue::Number(v) => Atom::Number(v.to_bits()),
        ProgramValue::Bytes(v) => Atom::Bytes(v.clone()),
        ProgramValue::Table(id) => {
            let index = if let Some(index) = ids.get(id) {
                *index
            } else {
                let index = pending.len();
                ids.insert(*id, index);
                pending.push(*id);
                index
            };
            Atom::Table(index)
        }
        ProgramValue::Callback(_) => {
            panic!("callback graph not part of this generic control matrix")
        }
        ProgramValue::Closure(_) | ProgramValue::DefinitionTable(_) => {
            panic!("live session reference escaped the plain graph snapshot boundary")
        }
    }
}
fn key(v: &ProgramValue) -> Key {
    match v {
        ProgramValue::Boolean(v) => Key::Boolean(*v),
        ProgramValue::Number(v) => Key::Number(if *v == 0.0 { 0 } else { v.to_bits() }),
        ProgramValue::Bytes(v) => Key::Bytes(v.clone()),
        v => panic!("unrepresented paired key {v:?}"),
    }
}
pub(super) fn graph(g: &ProgramValueGraph) -> Graph {
    let mut ids = BTreeMap::new();
    let mut pending = Vec::new();
    let results = g
        .values
        .iter()
        .map(|v| intern(v, &mut ids, &mut pending))
        .collect();
    let mut tables = Vec::new();
    while tables.len() < pending.len() {
        let id = pending[tables.len()];
        let mut entries = g.tables[id.0 as usize - 1]
            .entries
            .iter()
            .map(|(k, v)| (key(k), v))
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        tables.push(
            entries
                .into_iter()
                .map(|(k, v)| (k, intern(v, &mut ids, &mut pending)))
                .collect(),
        );
    }
    Graph { results, tables }
}

pub(super) fn constructor() -> ParserProgramBinding {
    let (upvalue, callback) = fixture().constructor;
    ParserProgramBinding::Intrinsic {
        operation: I::CreateMod,
        source: ParserProgramIntrinsicSource::Captured { upvalue, callback },
    }
}
pub(super) fn from_source(g: &Graph) -> ProgramValueGraph {
    fn atom(a: &Atom) -> ProgramValue {
        match a {
            Atom::Nil => ProgramValue::Nil,
            Atom::Boolean(v) => ProgramValue::Boolean(*v),
            Atom::Number(v) => ProgramValue::Number(f64::from_bits(*v)),
            Atom::Bytes(v) => ProgramValue::Bytes(v.clone()),
            Atom::Table(id) => ProgramValue::Table(ProgramTableId(*id as u32 + 1)),
        }
    }
    fn key(k: &Key) -> ProgramValue {
        match k {
            Key::Boolean(v) => ProgramValue::Boolean(*v),
            Key::Number(v) => ProgramValue::Number(f64::from_bits(*v)),
            Key::Bytes(v) => ProgramValue::Bytes(v.clone()),
        }
    }
    ProgramValueGraph {
        values: g.results.iter().map(atom).collect(),
        tables: g
            .tables
            .iter()
            .map(|t| ProgramTable {
                entries: t.iter().map(|(k, v)| (key(k), atom(v))).collect(),
            })
            .collect(),
    }
}

pub(super) fn opaque_method_input() -> ProgramValueGraph {
    ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable {
            entries: vec![(
                ProgramValue::Bytes(b"gsub".to_vec()),
                ProgramValue::Callback(fixture().constructor.1),
            )],
        }],
    }
}
