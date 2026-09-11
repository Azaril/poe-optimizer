//! Flag policy is selected per immutable parser, including concurrent callers.
use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::lua_pattern::MatchBudget;
use poe_optimizer_engine::modifier_parser::{CompiledModifierParser, ModifierValue as V};
use std::sync::Arc;

#[test]
fn independently_injected_flag_prefixes_remain_isolated_across_workers() {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    // Authored legacy fixture changes do not retain original program admissions.
    data.programs = Default::default();
    let helper = data.helpers["flag"];
    let owner = data
        .factories
        .iter()
        .find_map(|(id, disposition)| {
            let ParserFactoryDisposition::Pure(factory) = disposition else {
                return None;
            };
            (factory.provenance.constructor.is_none()
                && data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|u| u.name == "flag" && u.value == ParserValue::Callback(helper)))
            .then_some(*id)
        })
        .expect("source-linked Flag-only factory");
    for table in data.dictionaries.values() {
        data.tables[table.0 as usize - 1] = ParserTable::default();
    }
    data.tables[data.dictionaries[&ParserDictionary::Special].0 as usize - 1]
        .fields
        .insert("^flag (.+)$".into(), ParserValue::Callback(owner));
    let ParserFactoryDisposition::Pure(factory) = data.factories.get_mut(&owner).unwrap() else {
        unreachable!()
    };
    factory.parameter_count = 2;
    factory.body =
        ParserFactoryExpr::Table(vec![ParserFactoryField::List(ParserFactoryExpr::Flag {
            helper,
            args: vec![
                ParserFactoryExpr::Argument(1),
                ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
                ParserFactoryExpr::Literal(ParserFactoryLiteral::Text("7".into())),
                ParserFactoryExpr::Literal(ParserFactoryLiteral::Number(9.0)),
                ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
                ParserFactoryExpr::Table(vec![ParserFactoryField::Named {
                    key: "type".into(),
                    value: ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(
                        "CallerTag".into(),
                    )),
                }]),
                ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
            ],
        })]);
    let parsers: Vec<_> = [("", true), ("Caller\0é", false)]
        .into_iter()
        .map(|(kind, value)| {
            let mut selected = data.clone();
            selected.policy.flag_mod_type = kind.into();
            selected.policy.flag_mod_value = value;
            let catalog = ModifierParserCatalog::new(selected).unwrap();
            (
                Arc::new(CompiledModifierParser::new(&catalog).unwrap()),
                kind,
                value,
            )
        })
        .collect();
    std::thread::scope(|scope| {
        for (parser, kind, value) in parsers {
            scope.spawn(move || {
                for _ in 0..8 {
                    let output = parser
                        .parse(b"flag caller", &mut MatchBudget::default())
                        .unwrap();
                    assert!(output.extra.is_none());
                    let row = output
                        .modifiers
                        .as_ref()
                        .unwrap()
                        .indexed_value(1)
                        .as_table()
                        .unwrap();
                    assert_eq!(row.field("name"), &V::Bytes(b"caller".to_vec()));
                    assert_eq!(row.field("type"), &V::Bytes(kind.as_bytes().to_vec()));
                    assert_eq!(row.field("value"), &V::Boolean(value));
                    assert_eq!(row.field("flags"), &V::Number(0.0));
                    assert_eq!(row.field("keywordFlags"), &V::Number(9.0));
                    assert!(!row.fields.contains_key("source"));
                    assert_eq!(row.indexed.len(), 1);
                    assert!(!row.indexed.contains_key(&1));
                    assert_eq!(
                        row.indexed_value(2).as_table().unwrap().field("type"),
                        &V::Bytes(b"CallerTag".to_vec())
                    );
                }
            });
        }
    });
}
