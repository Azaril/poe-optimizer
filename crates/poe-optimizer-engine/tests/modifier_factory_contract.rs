//! Caller-data and resource contracts for pure special callback execution.
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::lua_pattern::{MatchBudget, MatchLimits};
use poe_optimizer_engine::modifier_parser::{
    CompiledModifierParser, ModifierValue as V, ParserError,
};
use std::sync::{Arc, OnceLock};
type E = ParserFactoryExpr;
type F = ParserFactoryField;
type L = ParserFactoryLiteral;
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn text(value: &str) -> E {
    E::Literal(L::Text(value.into()))
}
fn program(data: &mut ModifierParserData, id: ParserCallbackId) -> &mut ParserPureFactory {
    let ParserFactoryDisposition::Pure(factory) = data.factories.get_mut(&id).unwrap() else {
        panic!("pure factory")
    };
    factory
}
fn fixture(pattern: &str) -> (ModifierParserData, ParserCallbackId) {
    let mut data = snapshot().modifier_parser().data().clone();
    // Authored legacy fixture changes do not retain original program admissions.
    data.programs = Default::default();
    for id in data.dictionaries.values() {
        data.tables[id.0 as usize - 1] = ParserTable::default();
    }
    let id = *data
        .factories
        .iter()
        .find(|(_, disposition)| {
            matches!(disposition,
        ParserFactoryDisposition::Pure(factory) if factory.provenance.constructor.is_some())
        })
        .unwrap()
        .0;
    let special = data.dictionaries[&ParserDictionary::Special];
    data.tables[special.0 as usize - 1]
        .fields
        .insert(pattern.into(), ParserValue::Callback(id));
    let factory = program(&mut data, id);
    factory.parameter_count = 6;
    factory.body = E::Table(vec![F::List(E::CreateMod {
        args: vec![text("CallerValue"), text("BASE"), E::Argument(0)],
    })]);
    (data, id)
}
fn compile(data: ModifierParserData) -> CompiledModifierParser {
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn parse(
    parser: &CompiledModifierParser,
    input: &[u8],
) -> poe_optimizer_engine::modifier_parser::ParseOutcome {
    parser.parse(input, &mut MatchBudget::default()).unwrap()
}
#[test]
fn special_arguments_keep_converted_and_raw_bytes_separate() {
    let (mut data, id) = fixture("^probe (%S+) (.*)$");
    let factory = program(&mut data, id);
    factory.provenance.constructor = None;
    factory.body = E::Table(vec![F::List(E::Table(vec![
        F::Named {
            key: "converted".into(),
            value: E::Argument(0),
        },
        F::Named {
            key: "raw".into(),
            value: E::Argument(1),
        },
        F::Named {
            key: "tail".into(),
            value: E::Argument(2),
        },
        F::Named {
            key: "missing".into(),
            value: E::Argument(5),
        },
    ]))]);
    let parser = compile(data);
    for (input, converted, raw, tail) in [
        (
            b"probe 12 ALPHA".as_slice(),
            Some(12.0),
            b"12".as_slice(),
            b"alpha".as_slice(),
        ),
        (b"probe nope tail", None, b"nope", b"tail"),
        (b"probe \xff tail", None, b"\xff", b"tail"),
    ] {
        let result = parse(&parser, input);
        assert!(result.extra.is_none());
        let table = result.modifiers.unwrap();
        let row = table.indexed_value(1).as_table().unwrap();
        assert_eq!(
            row.field("converted"),
            &converted.map(V::Number).unwrap_or(V::Nil)
        );
        assert_eq!(row.field("raw").as_bytes(), Some(raw));
        assert_eq!(row.field("tail").as_bytes(), Some(tail));
        assert!(!row.fields.contains_key("missing"));
    }
}
#[test]
fn captured_non_scalar_is_deferred_only_when_selected_and_reached() {
    let (mut data, id) = fixture("^probe (%d+)$");
    let callback = &mut data.callbacks[id.0 as usize - 1];
    let slot = callback.upvalues.len() as u16;
    callback.upvalues.push(ParserUpvalue {
        name: "caller_capture".into(),
        value: ParserValue::Table(data.policy.mod_flags),
    });
    program(&mut data, id).body = E::Table(vec![F::List(E::CreateMod {
        args: vec![
            text("CallerValue"),
            text("LIST"),
            E::CapturedScalar { upvalue: slot },
        ],
    })]);
    let parser = compile(data.clone());
    assert!(
        parser
            .parse(b"unrelated", &mut MatchBudget::default())
            .is_ok()
    );
    assert!(
        matches!(parser.parse(b"probe 7", &mut MatchBudget::default()), Err(ParserError::Deferred { stage: "special factory captured value shape", callback: Some(actual) }) if actual == id)
    );
    data.callbacks[id.0 as usize - 1].upvalues[slot as usize].value =
        ParserValue::Text("injected".into());
    let result = parse(&compile(data), b"probe 7");
    assert_eq!(
        result
            .modifiers
            .unwrap()
            .indexed_value(1)
            .as_table()
            .unwrap()
            .field("value")
            .as_bytes(),
        Some(b"injected".as_slice())
    );
}
#[test]
fn definitions_are_shareable_and_separate_datasets_do_not_leak() {
    let (mut data, id) = fixture("^probe (%d+)$");
    let first = Arc::new(compile(data.clone()));
    program(&mut data, id).body = E::Table(vec![F::List(E::CreateMod {
        args: vec![
            text("OtherDataset"),
            text("BASE"),
            E::Negate(Box::new(E::Argument(0))),
        ],
    })]);
    let other = compile(data);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let parser = first.clone();
            scope.spawn(move || {
                for number in 1..=16 {
                    let output = parse(&parser, format!("probe {number}").as_bytes())
                        .modifiers
                        .unwrap();
                    let row = output.indexed_value(1).as_table().unwrap();
                    assert_eq!(
                        row.field("name").as_bytes(),
                        Some(b"CallerValue".as_slice())
                    );
                    assert_eq!(row.field("value"), &V::Number(number as f64));
                }
            });
        }
    });
    let output = parse(&other, b"probe 7").modifiers.unwrap();
    let row = output.indexed_value(1).as_table().unwrap();
    assert_eq!(
        row.field("name").as_bytes(),
        Some(b"OtherDataset".as_slice())
    );
    assert_eq!(row.field("value"), &V::Number(-7.0));
}
#[test]
fn factory_expansion_consumes_the_request_work_budget() {
    let (mut data, id) = fixture("^probe (%d+)$");
    let factory = program(&mut data, id);
    factory.provenance.constructor = None;
    factory.body = E::Table((0..4096).map(|_| F::List(E::Argument(1))).collect());
    let parser = compile(data);
    let mut budget = MatchBudget::new(MatchLimits {
        max_steps: 300,
        ..Default::default()
    });
    assert!(matches!(
        parser.parse(b"probe 7", &mut budget),
        Err(ParserError::Scan(_)) | Err(ParserError::ResourceBound(_))
    ));
    let output = parse(&parser, b"probe 7").modifiers.unwrap();
    assert_eq!(output.indexed.len(), 4096);
}

#[test]
fn repeated_capture_bytes_hit_output_bound_before_later_arithmetic_error() {
    let (mut data, id) = fixture("^probe (.+)$");
    let factory = program(&mut data, id);
    factory.provenance.constructor = None;
    let mut fields: Vec<_> = (0..2048).map(|_| F::List(E::Argument(1))).collect();
    fields.push(F::List(E::Negate(Box::new(E::Literal(L::Boolean(false))))));
    factory.body = E::Table(fields);
    let parser = compile(data);
    let input = format!("probe {}", "x".repeat(8192));
    assert!(matches!(
        parser.parse(input.as_bytes(), &mut MatchBudget::default()),
        Err(ParserError::ResourceBound(_))
    ));
}
