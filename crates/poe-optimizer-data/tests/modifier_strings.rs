use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use std::sync::OnceLock;
fn data() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap().modifier_parser().data().clone())
        .clone()
}
fn id(data: &ModifierParserData) -> ParserCallbackId {
    *data
        .factories
        .iter()
        .find(|(id, f)| {
            matches!(f, ParserFactoryDisposition::Pure(_))
                && data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|u| u.name == "firstToUpper")
        })
        .unwrap()
        .0
}
fn replace(data: &mut ModifierParserData, helper: ParserCallbackId, value: ParserFactoryExpr) {
    let id = id(data);
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    f.body = ParserFactoryExpr::Table(vec![ParserFactoryField::List(
        ParserFactoryExpr::FirstToUpper {
            helper,
            value: Box::new(value),
        },
    )]);
    f.provenance.constructor = None;
}
#[test]
fn source_string_factories_have_complete_dictionary_breadth_and_binding() {
    let data = data();
    let candidates = data
        .factories
        .iter()
        .filter(|(id, f)| {
            matches!(f, ParserFactoryDisposition::Pure(_))
                && data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|u| u.name == "firstToUpper")
        })
        .map(|(id, _)| *id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(candidates.len(), 54);
    for (dictionary, count) in [
        (ParserDictionary::Special, 44),
        (ParserDictionary::ModTag, 10),
    ] {
        let table = &data.tables[data.dictionaries[&dictionary].0 as usize - 1];
        let found = table
            .fields
            .values()
            .filter_map(|v| {
                if let ParserValue::Callback(id) = v {
                    candidates.contains(id).then_some(*id)
                } else {
                    None
                }
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(found.len(), count);
    }
    assert_eq!(data.policy.first_to_upper_pattern, "^%l");
    let helper = &data.callbacks[data.helpers["firstToUpper"].0 as usize - 1];
    let ParserCallbackKind::Lua { source } = &helper.kind else {
        panic!()
    };
    assert_eq!(
        data.source.construction_spans["first_to_upper_primitive"],
        *source
    );
    assert!(helper.upvalues.is_empty());
}
#[test]
fn string_patterns_are_required_bounded_and_lazy_and_values_keep_their_kinds() {
    let mut data = data();
    for pattern in [String::new(), "[".into(), "()(.)".into(), "x".repeat(4096)] {
        data.policy.first_to_upper_pattern = pattern;
        data.validate().unwrap();
    }
    for pattern in ["x".repeat(4097), "\0".into()] {
        data.policy.first_to_upper_pattern = pattern;
        assert!(data.validate().is_err());
    }
    data.policy.first_to_upper_pattern = "^%l".into();
    let helper = data.helpers["firstToUpper"];
    for value in [
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Boolean(false)),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Number(13.0)),
        ParserFactoryExpr::Table(vec![]),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Text("a\0é".into())),
    ] {
        replace(&mut data, helper, value);
        data.validate().unwrap();
    }
    let mut policy = serde_json::to_value(&data.policy).unwrap();
    policy
        .as_object_mut()
        .unwrap()
        .remove("first_to_upper_pattern");
    assert!(serde_json::from_value::<ParserPolicy>(policy).is_err());
}
#[test]
fn helper_node_requires_its_declared_closed_captured_source_binding() {
    let original = data();
    let helper = original.helpers["firstToUpper"];
    for case in 0..5 {
        let mut data = original.clone();
        let id = id(&data);
        replace(
            &mut data,
            if case == 0 {
                ParserCallbackId(0)
            } else {
                helper
            },
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
        );
        match case {
            1 => {
                data.helpers
                    .insert("firstToUpper".into(), ParserCallbackId(0));
            }
            2 => {
                data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .retain(|u| u.name != "firstToUpper");
            }
            3 => {
                data.callbacks[helper.0 as usize - 1].kind = ParserCallbackKind::Builtin {
                    symbol: "string.upper".into(),
                };
            }
            4 => {
                data.callbacks[helper.0 as usize - 1]
                    .upvalues
                    .push(ParserUpvalue {
                        name: "string".into(),
                        value: ParserValue::Nil,
                    });
            }
            _ => {}
        }
        assert!(data.validate().is_err(), "case {case}");
    }
}
#[test]
fn string_nodes_have_closed_single_value_shape_and_bounded_expression_depth() {
    let original = data();
    let helper = original.helpers["firstToUpper"];
    let expr = ParserFactoryExpr::FirstToUpper {
        helper,
        value: Box::new(ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)),
    };
    for case in 0..2 {
        let mut value = serde_json::to_value(&expr).unwrap();
        let fields = value["value"].as_object_mut().unwrap();
        if case == 0 {
            fields.remove("value");
        } else {
            fields.insert("extra_arguments".into(), serde_json::json!([1]));
        }
        assert!(serde_json::from_value::<ParserFactoryExpr>(value).is_err());
    }
    for upper in [false, true] {
        let mut data = original.clone();
        let mut value = ParserFactoryExpr::Literal(ParserFactoryLiteral::Text("x".into()));
        for _ in 0..34 {
            value = if upper {
                ParserFactoryExpr::FirstToUpper {
                    helper,
                    value: Box::new(value),
                }
            } else {
                ParserFactoryExpr::Concat {
                    left: Box::new(ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(
                        "x".into(),
                    ))),
                    right: Box::new(value),
                }
            };
        }
        replace(&mut data, helper, value);
        assert!(data.validate().is_err());
    }
}
