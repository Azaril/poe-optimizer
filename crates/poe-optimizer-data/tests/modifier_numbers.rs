use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use std::{collections::BTreeSet, sync::OnceLock};
fn data() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap().modifier_parser().data().clone())
        .clone()
}
fn converts(e: &ParserFactoryExpr) -> bool {
    match e {
        ParserFactoryExpr::ToNumber { .. } => true,
        ParserFactoryExpr::Negate(v) | ParserFactoryExpr::FirstToUpper { value: v, .. } => {
            converts(v)
        }
        ParserFactoryExpr::Concat { left, right } => converts(left) || converts(right),
        ParserFactoryExpr::Table(fields) => fields.iter().any(|f| match f {
            ParserFactoryField::Named { value, .. } | ParserFactoryField::List(value) => {
                converts(value)
            }
        }),
        ParserFactoryExpr::CreateMod { args } | ParserFactoryExpr::Flag { args, .. } => {
            args.iter().any(converts)
        }
        _ => false,
    }
}
fn id(data: &ModifierParserData) -> ParserCallbackId {
    *data
        .factories
        .iter()
        .find(|(_, f)| matches!(f, ParserFactoryDisposition::Pure(f) if converts(&f.body)))
        .unwrap()
        .0
}
fn replace(data: &mut ModifierParserData, child: ParserFactoryExpr) -> ParserCallbackId {
    let id = id(data);
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    f.body = ParserFactoryExpr::Table(vec![ParserFactoryField::List(
        ParserFactoryExpr::ToNumber {
            value: Box::new(child),
        },
    )]);
    f.provenance.constructor = None;
    id
}
#[test]
fn number_factories_cover_all_three_existing_caller_dictionaries_without_helper_fiction() {
    let data = data();
    data.validate().unwrap();
    let ids: BTreeSet<_> = data
        .factories
        .iter()
        .filter_map(|(id, f)| match f {
            ParserFactoryDisposition::Pure(f) if converts(&f.body) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 81);
    for (dict, expected) in [
        (ParserDictionary::Special, 58),
        (ParserDictionary::PreAnchorSpecial, 58),
        (ParserDictionary::ModTag, 22),
        (ParserDictionary::PreFlag, 1),
    ] {
        let found: BTreeSet<_> = data.tables[data.dictionaries[&dict].0 as usize - 1]
            .fields
            .values()
            .filter_map(|v| match v {
                ParserValue::Callback(id) if ids.contains(id) => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(found.len(), expected);
    }
    assert!(!data.helpers.contains_key("tonumber"));
    for id in ids {
        assert!(
            !data.callbacks[id.0 as usize - 1]
                .upvalues
                .iter()
                .any(|u| u.name == "tonumber")
        );
    }
}
#[test]
fn number_nodes_preserve_lazy_child_kinds_and_injected_literal_bytes() {
    let original = data();
    for child in [
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Boolean(false)),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Number(-0.0)),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::NonFinite(ParserNonFinite::Nan)),
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Text("12\0é".into())),
        ParserFactoryExpr::Table(vec![]),
        ParserFactoryExpr::ConstantField {
            table: original.policy.mod_flags,
            key: "CallerSupplied".into(),
        },
    ] {
        let mut data = original.clone();
        let id = replace(&mut data, child);
        data.validate().unwrap();
        let json = serde_json::to_string(&data.factories[&id]).unwrap();
        assert_eq!(
            serde_json::from_str::<ParserFactoryDisposition>(&json).unwrap(),
            data.factories[&id]
        );
    }
}
#[test]
fn captured_shadow_is_rejected_only_when_the_recipe_requires_the_global_operation() {
    let mut data = data();
    let id = replace(
        &mut data,
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
    );
    data.callbacks[id.0 as usize - 1]
        .upvalues
        .push(ParserUpvalue {
            name: "tonumber".into(),
            value: ParserValue::Nil,
        });
    assert!(data.validate().is_err());
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    f.body = ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil);
    data.validate().unwrap();
}
#[test]
fn number_operation_has_one_required_child_no_base_or_spread_and_no_scalar_root() {
    let node = ParserFactoryExpr::ToNumber {
        value: Box::new(ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)),
    };
    for case in 0..3 {
        let mut json = serde_json::to_value(&node).unwrap();
        let fields = json["value"].as_object_mut().unwrap();
        match case {
            0 => {
                fields.remove("value");
            }
            1 => {
                fields.insert("base".into(), serde_json::json!(16));
            }
            2 => {
                fields.insert("extra_arguments".into(), serde_json::json!([]));
            }
            _ => unreachable!(),
        }
        assert!(serde_json::from_value::<ParserFactoryExpr>(json).is_err());
    }
    let mut data = data();
    let id = id(&data);
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    f.body = node;
    f.provenance.constructor = None;
    assert!(data.validate().is_err());
}
#[test]
fn number_nodes_charge_nested_depth_and_literal_bytes_to_existing_bounds() {
    let mut data = data();
    let mut child = ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil);
    for _ in 0..34 {
        child = ParserFactoryExpr::ToNumber {
            value: Box::new(child),
        };
    }
    replace(&mut data, child);
    assert!(data.validate().is_err());
    replace(
        &mut data,
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Text("x".repeat(4097))),
    );
    assert!(data.validate().is_err());
}
