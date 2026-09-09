use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use std::{collections::BTreeSet, sync::OnceLock};
fn data() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap().modifier_parser().data().clone())
        .clone()
}
fn flag_owners(data: &ModifierParserData) -> BTreeSet<ParserCallbackId> {
    data.factories
        .iter()
        .filter(|(id, f)| {
            matches!(f, ParserFactoryDisposition::Pure(_))
                && data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|u| u.name == "flag")
        })
        .map(|(id, _)| *id)
        .collect()
}
fn owner(data: &ModifierParserData) -> ParserCallbackId {
    flag_owners(data).into_iter().find(|id| matches!(&data.factories[id], ParserFactoryDisposition::Pure(f) if f.provenance.constructor.is_none())).unwrap()
}
fn replace(data: &mut ModifierParserData, args: Vec<ParserFactoryExpr>) -> ParserCallbackId {
    let id = owner(data);
    let helper = data.helpers["flag"];
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    f.body = ParserFactoryExpr::Flag { helper, args };
    id
}
#[test]
fn complete_flag_breadth_preserves_the_indirect_constructor_path() {
    let data = data();
    data.validate().unwrap();
    let owners = flag_owners(&data);
    assert_eq!(owners.len(), 68);
    assert_eq!(owners.iter().filter(|id| matches!(&data.factories[id], ParserFactoryDisposition::Pure(f) if f.provenance.constructor.is_none())).count(), 23);
    for dict in [
        ParserDictionary::Special,
        ParserDictionary::PreAnchorSpecial,
    ] {
        let rows = &data.tables[data.dictionaries[&dict].0 as usize - 1];
        let found: BTreeSet<_> = rows
            .fields
            .values()
            .filter_map(|v| match v {
                ParserValue::Callback(id) if owners.contains(id) => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(found, owners);
    }
    let flag = &data.callbacks[data.helpers["flag"].0 as usize - 1];
    assert_eq!(flag.upvalues.len(), 1);
    assert_eq!(flag.upvalues[0].name, "mod");
    let ParserValue::Callback(constructor) = flag.upvalues[0].value else {
        panic!()
    };
    assert_eq!(
        data.callbacks[constructor.0 as usize - 1].kind,
        ParserCallbackKind::Lua {
            source: data.source.construction_spans["create_mod"].clone()
        }
    );
    assert_eq!(
        flag.kind,
        ParserCallbackKind::Lua {
            source: data.source.construction_spans["flag_primitive"].clone()
        }
    );
    assert_eq!(data.policy.flag_mod_type, "FLAG");
    assert!(data.policy.flag_mod_value);
}
#[test]
fn injected_prefix_is_required_typed_bounded_literal_payload() {
    let mut data = data();
    for text in [String::new(), "a\0é".into(), "x".repeat(4096)] {
        for value in [false, true] {
            data.policy.flag_mod_type = text.clone();
            data.policy.flag_mod_value = value;
            data.validate().unwrap();
        }
    }
    data.policy.flag_mod_type = "x".repeat(4097);
    assert!(data.validate().is_err());
    data.policy.flag_mod_type.clear();
    for field in ["flag_mod_type", "flag_mod_value"] {
        let mut policy = serde_json::to_value(&data.policy).unwrap();
        policy.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ParserPolicy>(policy).is_err());
    }
    let mut policy = serde_json::to_value(&data.policy).unwrap();
    policy["flag_mod_value"] = serde_json::json!(1);
    assert!(serde_json::from_value::<ParserPolicy>(policy).is_err());
    let serialized = serde_json::to_string(&data.policy).unwrap();
    let duplicate = serialized.replacen("{", "{\"flag_mod_value\":false,", 1);
    assert!(serde_json::from_str::<ParserPolicy>(&duplicate).is_err());
}
#[test]
fn flag_root_preserves_nil_holes_lazy_values_and_has_no_direct_capture() {
    let mut data = data();
    let id = replace(
        &mut data,
        vec![
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Boolean(false)),
            ParserFactoryExpr::Table(vec![]),
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
        ],
    );
    data.validate().unwrap();
    let ParserFactoryDisposition::Pure(f) = &data.factories[&id] else {
        panic!()
    };
    assert!(f.provenance.constructor.is_none());
    let serialized = serde_json::to_string(&f.body).unwrap();
    assert_eq!(
        serde_json::from_str::<ParserFactoryExpr>(&serialized).unwrap(),
        f.body
    );
    replace(&mut data, vec![]);
    data.validate().unwrap();
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    let ParserValue::Callback(constructor) =
        data.callbacks[data.helpers["flag"].0 as usize - 1].upvalues[0].value
    else {
        panic!()
    };
    f.provenance.constructor = Some(constructor);
    assert!(data.validate().is_err());
}
#[test]
fn flag_rejects_rebound_owner_helper_constructor_and_missing_source_anchors() {
    let original = data();
    let helper = original.helpers["flag"];
    let ParserValue::Callback(constructor) =
        original.callbacks[helper.0 as usize - 1].upvalues[0].value
    else {
        panic!()
    };
    for case in 0..10 {
        let mut data = original.clone();
        let id = replace(&mut data, vec![]);
        match case {
            0 => {
                data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .retain(|u| u.name != "flag");
            }
            1 => {
                data.helpers.insert("flag".into(), constructor);
            }
            2 => {
                data.callbacks[helper.0 as usize - 1].upvalues[0].name = "other".into();
            }
            3 => {
                data.callbacks[helper.0 as usize - 1].upvalues[0].value =
                    ParserValue::Callback(helper);
            }
            4 => {
                data.callbacks[helper.0 as usize - 1]
                    .upvalues
                    .push(ParserUpvalue {
                        name: "extra".into(),
                        value: ParserValue::Nil,
                    });
            }
            5 => {
                data.callbacks[constructor.0 as usize - 1]
                    .upvalues
                    .push(ParserUpvalue {
                        name: "type".into(),
                        value: ParserValue::Nil,
                    });
            }
            6 => {
                data.source.construction_spans.remove("create_mod");
            }
            7 => {
                data.source.construction_spans.remove("flag_primitive");
            }
            8 => {
                data.source
                    .construction_spans
                    .get_mut("create_mod")
                    .unwrap()
                    .line += 1;
            }
            9 => {
                data.source
                    .construction_spans
                    .get_mut("flag_primitive")
                    .unwrap()
                    .end_line += 1;
            }
            _ => unreachable!(),
        }
        assert!(data.validate().is_err(), "case {case}");
    }
}
#[test]
fn flag_nodes_bound_arguments_depth_and_reject_undeclared_spread_fields() {
    let original = data();
    let nil = ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil);
    let mut data = original.clone();
    replace(&mut data, vec![nil.clone(); 4097]);
    assert!(data.validate().is_err());
    let mut value = nil;
    let helper = data.helpers["flag"];
    for _ in 0..34 {
        value = ParserFactoryExpr::Flag {
            helper,
            args: vec![value],
        };
    }
    let mut data = original;
    replace(&mut data, vec![value]);
    assert!(data.validate().is_err());
    let mut json = serde_json::to_value(ParserFactoryExpr::Flag {
        helper,
        args: vec![],
    })
    .unwrap();
    json["value"]["spread"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ParserFactoryExpr>(json).is_err());
}
