//! Injected numeric expressions remain immutable and isolated across parallel parsers.
use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::lua_pattern::MatchBudget;
use poe_optimizer_engine::modifier_parser::{CompiledModifierParser, ModifierValue as V};
use std::sync::Arc;

#[test]
fn parallel_parsers_use_selected_expressions_and_preserve_raw_captures() {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    let owner = data
        .factories
        .iter()
        .find_map(|(id, disposition)| {
            let ParserFactoryDisposition::Pure(factory) = disposition else {
                return None;
            };
            factory.provenance.constructor.is_none().then_some(*id)
        })
        .expect("source-linked constructor-free factory");
    for table in data.dictionaries.values() {
        data.tables[table.0 as usize - 1] = ParserTable::default();
    }
    data.tables[data.dictionaries[&ParserDictionary::Special].0 as usize - 1]
        .fields
        .insert("^number (.+)$".into(), ParserValue::Callback(owner));
    let parsers: Vec<_> = [("3.5", 3.5), ("12.25", 12.25)]
        .into_iter()
        .map(|(text, expected)| {
            let mut selected = data.clone();
            let ParserFactoryDisposition::Pure(factory) =
                selected.factories.get_mut(&owner).unwrap()
            else {
                unreachable!()
            };
            factory.parameter_count = 3;
            let convert = |value| ParserFactoryExpr::ToNumber {
                value: Box::new(value),
            };
            factory.body = ParserFactoryExpr::Table(vec![ParserFactoryField::List(
                ParserFactoryExpr::Table(vec![
                    ParserFactoryField::Named {
                        key: "converted".into(),
                        value: convert(ParserFactoryExpr::Argument(1)),
                    },
                    ParserFactoryField::Named {
                        key: "raw".into(),
                        value: ParserFactoryExpr::Argument(1),
                    },
                    ParserFactoryField::Named {
                        key: "injected".into(),
                        value: convert(ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(
                            text.into(),
                        ))),
                    },
                    ParserFactoryField::Named {
                        key: "missing".into(),
                        value: convert(ParserFactoryExpr::Argument(2)),
                    },
                ]),
            )]);
            let catalog = ModifierParserCatalog::new(selected).unwrap();
            (
                Arc::new(CompiledModifierParser::new(&catalog).unwrap()),
                expected,
            )
        })
        .collect();
    std::thread::scope(|scope| {
        for (parser, expected) in parsers {
            scope.spawn(move || {
                for (input, raw, number) in [
                    (&b"number 0x1p-2"[..], &b"0x1p-2"[..], Some(0.25_f64)),
                    (&b"number -0"[..], &b"-0"[..], Some(-0.0_f64)),
                    (&b"number 12\0"[..], &b"12\0"[..], None),
                ] {
                    for _ in 0..8 {
                        let output = parser.parse(input, &mut MatchBudget::default()).unwrap();
                        assert!(output.extra.is_none());
                        let row = output
                            .modifiers
                            .as_ref()
                            .unwrap()
                            .indexed_value(1)
                            .as_table()
                            .unwrap();
                        assert_eq!(row.field("injected"), &V::Number(expected));
                        assert_eq!(row.field("raw"), &V::Bytes(raw.to_vec()));
                        assert!(!row.fields.contains_key("missing"));
                        match number {
                            Some(number) => {
                                let V::Number(actual) = row.field("converted") else {
                                    panic!("number result")
                                };
                                assert_eq!(actual.to_bits(), number.to_bits());
                            }
                            None => assert!(!row.fields.contains_key("converted")),
                        }
                    }
                }
            });
        }
    });
}
