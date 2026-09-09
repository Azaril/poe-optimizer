//! Adversarial caller-data contracts for the native structural parser.
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ModifierParserData, ParserDictionary as D, ParserNonFinite, ParserTable,
    ParserTableId, ParserValue as P,
};
use poe_optimizer_engine::lua_pattern::{MatchBudget, MatchLimits, PatternError, ResourceKind};
use poe_optimizer_engine::modifier_parser::{
    CompiledModifierParser, MAX_PARSER_TEXT_BYTES, ModifierTable, ModifierValue as V, ParseOutcome,
    ParserError,
};
use poe_optimizer_engine::modifier_scan::ScanError;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn data() -> ModifierParserData {
    let mut data = snapshot().modifier_parser().data().clone();
    for id in data.dictionaries.values() {
        data.tables[id.0 as usize - 1] = ParserTable::default();
    }
    data
}
fn dictionary(data: &mut ModifierParserData, dictionary: D, rows: &[(&str, P)]) {
    let id = data.dictionaries[&dictionary];
    data.tables[id.0 as usize - 1] = ParserTable {
        fields: rows
            .iter()
            .map(|(key, value)| ((*key).into(), value.clone()))
            .collect(),
        indexed: BTreeMap::new(),
    };
}
fn table(data: &mut ModifierParserData, fields: &[(&str, P)], values: Vec<P>) -> P {
    data.tables.push(ParserTable {
        fields: fields
            .iter()
            .map(|(key, value)| ((*key).into(), value.clone()))
            .collect(),
        indexed: values
            .into_iter()
            .enumerate()
            .map(|(i, value)| (i as i64 + 1, value))
            .collect(),
    });
    P::Table(ParserTableId(data.tables.len() as u32))
}
fn compiled(data: ModifierParserData) -> CompiledModifierParser {
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn ordinary(data: &mut ModifierParserData, names: P) {
    dictionary(data, D::Form, &[("(%d+) ", P::Text("BASE".into()))]);
    dictionary(data, D::ModName, &[("life ", names)]);
}
fn parse(parser: &CompiledModifierParser, input: &[u8]) -> Result<ParseOutcome, ParserError> {
    parser.parse(input, &mut MatchBudget::default())
}
fn first(outcome: &ParseOutcome) -> &ModifierTable {
    outcome
        .modifiers
        .as_ref()
        .unwrap()
        .indexed_value(1)
        .as_table()
        .unwrap()
}

#[test]
fn equal_payload_ties_with_distinct_captures_are_ambiguous() {
    let mut data = data();
    dictionary(
        &mut data,
        D::Form,
        &[
            ("(%d)2", P::Text("BASE".into())),
            ("1(%d)", P::Text("BASE".into())),
        ],
    );
    dictionary(
        &mut data,
        D::ModName,
        &[(" to maximum life ", P::Text("Life".into()))],
    );
    let parser = compiled(data);
    let result = parse(&parser, b"12 to maximum life");
    assert!(
        matches!(result,Err(ParserError::Ambiguous { dictionary: D::Form, ref patterns }) if patterns.len()==2),
        "{result:?}"
    );
}

fn twin_prefix(data: &mut ModifierParserData, left: P, right: P) {
    dictionary(data, D::PreFlag, &[("^a[b]", left), ("^[a]b", right)]);
}
fn dag(data: &mut ModifierParserData, depth: usize, leaf: P) -> P {
    let mut tail = table(data, &[("marker", leaf)], vec![]);
    for _ in 0..depth {
        tail = table(data, &[("left", tail.clone()), ("right", tail)], vec![]);
    }
    tail
}
#[test]
fn shared_graph_ties_fit_a_small_request_work_budget() {
    let mut data = data();
    let left = dag(&mut data, 40, P::Text("same tail".into()));
    let right = dag(&mut data, 40, P::Text("same tail".into()));
    twin_prefix(&mut data, left, right);
    ordinary(&mut data, P::Text("Life".into()));
    let parser = compiled(data);
    let mut budget = MatchBudget::new(MatchLimits {
        max_steps: 2_000,
        ..MatchLimits::default()
    });
    let result = parser.parse(b"ab1 life", &mut budget).unwrap();
    assert_eq!(first(&result).field("value"), &V::Number(1.0));
    assert!(budget.steps_used() < 2_000);
    let mut exhausted = MatchBudget::new(MatchLimits {
        max_steps: 1,
        ..MatchLimits::default()
    });
    assert!(matches!(
        parser.parse(b"ab1 life", &mut exhausted),
        Err(ParserError::Scan(ScanError::Pattern(
            PatternError::Resource(ResourceKind::MatchSteps)
        )))
    ));
}
#[test]
fn graph_tie_equivalence_preserves_negative_zero_and_nan_bits() {
    for (left, right, ambiguous) in [
        (P::Number(0.0), P::Number(-0.0), true),
        (
            P::NonFinite(ParserNonFinite::Nan),
            P::NonFinite(ParserNonFinite::Nan),
            false,
        ),
    ] {
        let mut data = data();
        let left = table(&mut data, &[("unused", left)], vec![]);
        let right = table(&mut data, &[("unused", right)], vec![]);
        twin_prefix(&mut data, left, right);
        ordinary(&mut data, P::Text("Life".into()));
        let result = parse(&compiled(data), b"ab1 life");
        if ambiguous {
            assert!(
                matches!(
                    result,
                    Err(ParserError::Ambiguous {
                        dictionary: D::PreFlag,
                        ..
                    })
                ),
                "{result:?}"
            );
        } else {
            assert_eq!(first(&result.unwrap()).field("value"), &V::Number(1.0));
        }
    }
}

#[test]
fn cartesian_output_hits_resource_limit_before_later_bad_name() {
    let mut data = data();
    let empty_tag = table(&mut data, &[], vec![]);
    let tags = table(&mut data, &[], vec![empty_tag; 500]);
    let prefix = table(&mut data, &[("tagList", tags)], vec![]);
    dictionary(&mut data, D::PreFlag, &[("^ab", prefix)]);
    let mut names = vec![P::Text("Life".into()); 200];
    // If expansion were deferred until final public copying, this source error
    // would occur only after allocating 100,000 tag entries across prior rows.
    names.push(P::Boolean(false));
    let names = table(&mut data, &[], names);
    ordinary(&mut data, names);
    assert!(matches!(
        parse(&compiled(data), b"ab1 life"),
        Err(ParserError::ResourceBound(_))
    ));
}

#[test]
fn raw_row_errors_precede_wrapper_errors() {
    let mut data = data();
    let names = table(
        &mut data,
        &[
            ("addToMinion", P::Boolean(true)),
            ("playerTagList", P::Boolean(true)),
        ],
        vec![P::Text("Life".into()), P::Boolean(false)],
    );
    ordinary(&mut data, names);
    let result = parse(&compiled(data), b"1 life");
    assert!(
        matches!(result,Err(ParserError::SourceError(ref message)) if message.contains("concatenation")),
        "{result:?}"
    );
}

#[test]
fn compilation_limits_apply_across_dictionaries() {
    let mut data = data();
    for dictionary in [D::Conqueror, D::Form] {
        let id = data.dictionaries[&dictionary];
        data.tables[id.0 as usize - 1].fields = (0..32_769)
            .map(|i| (format!("key{i:05}"), P::Boolean(true)))
            .collect();
    }
    let catalog = ModifierParserCatalog::new(data).unwrap();
    assert!(matches!(
        CompiledModifierParser::new(&catalog),
        Err(ParserError::ResourceBound("compiled dictionary rows"))
    ));
}

#[test]
fn pending_callbacks_source_errors_and_input_bounds_remain_distinct() {
    let mut deferred = data();
    let callback = *deferred.helpers.values().next().unwrap();
    dictionary(
        &mut deferred,
        D::Special,
        &[("^1 life$", P::Callback(callback))],
    );
    ordinary(&mut deferred, P::Text("Life".into()));
    let parser = compiled(deferred);
    assert!(
        matches!(parse(&parser,b"1 life"),Err(ParserError::Deferred { stage:"special callback",callback:Some(id) }) if id==callback)
    );
    assert!(matches!(
        parse(&parser, &vec![b'x'; MAX_PARSER_TEXT_BYTES + 1]),
        Err(ParserError::ResourceBound("input bytes"))
    ));
    let mut malformed = data();
    let prefix = table(&mut malformed, &[("tag", P::Boolean(true))], vec![]);
    dictionary(&mut malformed, D::PreFlag, &[("^ab", prefix)]);
    ordinary(&mut malformed, P::Text("Life".into()));
    assert!(matches!(
        parse(&compiled(malformed), b"ab1 life"),
        Err(ParserError::SourceError(_))
    ));
}

#[test]
fn caller_data_and_request_values_are_independent_and_shareable() {
    let mut data = data();
    dictionary(
        &mut data,
        D::Form,
        &[("^custom (%d+) ", P::Text("MORE".into()))],
    );
    dictionary(
        &mut data,
        D::ModName,
        &[("caller resource ", P::Text("CallerResource".into()))],
    );
    let parser = Arc::new(compiled(data));
    let workers: Vec<_> = (1..=8)
        .map(|n| {
            let parser = parser.clone();
            std::thread::spawn(move || {
                let result =
                    parse(&parser, format!("custom {n} caller resource").as_bytes()).unwrap();
                assert_eq!(
                    first(&result).field("name").as_bytes(),
                    Some(b"CallerResource".as_slice())
                );
                assert_eq!(
                    first(&result).field("type").as_bytes(),
                    Some(b"MORE".as_slice())
                );
                assert_eq!(first(&result).field("value"), &V::Number(f64::from(n)));
                assert!(result.extra.is_none());
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let absent = parse(&parser, b"custom 9 unregistered resource").unwrap();
    assert!(absent.modifiers.unwrap().indexed.is_empty());
    assert!(absent.extra.is_some());
}

#[test]
fn direct_numeric_flag_ties_preserve_signed_zero() {
    let mut data = data();
    dictionary(&mut data, D::Form, &[("^flag ", P::Text("FLAG".into()))]);
    dictionary(
        &mut data,
        D::Flag,
        &[("^a[b]", P::Number(0.0)), ("^[a]b", P::Number(-0.0))],
    );
    let result = parse(&compiled(data), b"flag ab");
    assert!(
        matches!(
            result,
            Err(ParserError::Ambiguous {
                dictionary: D::Flag,
                ..
            })
        ),
        "{result:?}"
    );
}
