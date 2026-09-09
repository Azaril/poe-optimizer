use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use std::sync::OnceLock;
fn data() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap().modifier_parser().data().clone())
        .clone()
}
fn pure_id(data: &ModifierParserData) -> ParserCallbackId {
    *data.factories.iter().find(|(_, f)| matches!(f, ParserFactoryDisposition::Pure(p) if p.provenance.constructor.is_some())).unwrap().0
}
fn pure(data: &mut ModifierParserData, id: ParserCallbackId) -> &mut ParserPureFactory {
    let ParserFactoryDisposition::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!()
    };
    f
}

#[test]
fn complete_factory_inventory_and_generated_capture_identity_are_preserved() {
    let data = data();
    data.validate().unwrap();
    assert_eq!(data.factories.len(), data.callbacks.len());
    assert_eq!(
        data.factories
            .values()
            .filter(|f| matches!(f, ParserFactoryDisposition::Pure(_)))
            .count(),
        1073
    );
    assert_eq!(
        data.factories
            .values()
            .filter(|f| matches!(f, ParserFactoryDisposition::Unsupported { .. }))
            .count(),
        578
    );
    let generated = data
        .factories
        .iter()
        .filter(|(_, f)| matches!(f, ParserFactoryDisposition::Pure(_)))
        .filter(|(id, _)| {
            data.callbacks[id.0 as usize - 1]
                .upvalues
                .iter()
                .any(|u| u.name == "skillName")
        })
        .count();
    assert_eq!(generated, 244);
}

#[test]
fn injected_recipes_and_dictionary_aliases_are_new_data_with_borrowed_lookup() {
    let mut data = data();
    let id = pure_id(&data);
    let f = pure(&mut data, id);
    f.body = ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil);
    f.provenance.constructor = None;
    let special = data.dictionaries[&ParserDictionary::Special];
    data.tables[special.0 as usize - 1]
        .fields
        .insert("^injected alias$".into(), ParserValue::Callback(id));
    let catalog = ModifierParserCatalog::new(data).unwrap();
    assert!(
        matches!(catalog.factory(id), Some(ParserFactoryDisposition::Pure(p)) if matches!(p.body, ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)))
    );
    assert_eq!(
        catalog.exact(ParserDictionary::Special, "^injected alias$"),
        Some(&ParserValue::Callback(id))
    );
    assert!(catalog.factory(ParserCallbackId(0)).is_none());
}

#[test]
fn disposition_and_provenance_references_are_complete_and_bound() {
    let base = data();
    let id = pure_id(&base);
    let json = serde_json::to_string(&base).unwrap();
    let disposition = serde_json::to_string(&base.factories[&id]).unwrap();
    let duplicate = json.replacen(
        "\"factories\":{",
        &format!("\"factories\":{{\"{}\":{},", id.0, disposition),
        1,
    );
    assert!(serde_json::from_str::<ModifierParserData>(&duplicate).is_err());
    let mut changed = base.clone();
    changed.factories.remove(&id);
    assert!(changed.validate().is_err());
    let mut changed = base.clone();
    pure(&mut changed, id).provenance.source.line += 1;
    assert!(changed.validate().is_err());
    let mut changed = base.clone();
    pure(&mut changed, id).provenance.function_end = 0;
    assert!(changed.validate().is_err());
    let mut changed = base.clone();
    pure(&mut changed, id).provenance.function_sha256 = "x".repeat(64);
    assert!(changed.validate().is_err());
    let mut changed = base.clone();
    pure(&mut changed, id).provenance.constructor = Some(ParserCallbackId(0));
    assert!(changed.validate().is_err());
    let mut changed = base;
    pure(&mut changed, id).provenance.constructor = None;
    assert!(changed.validate().is_err());
}

#[test]
fn unsupported_runtime_capture_kinds_and_missing_constant_fields_remain_lazy() {
    let mut data = data();
    let (&id, _) = data
        .factories
        .iter()
        .find(|(id, f)| {
            matches!(f, ParserFactoryDisposition::Pure(_))
                && data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|u| u.name == "skillName")
        })
        .unwrap();
    let callback = &mut data.callbacks[id.0 as usize - 1];
    let capture = callback
        .upvalues
        .iter_mut()
        .find(|u| u.name == "skillName")
        .unwrap();
    capture.value = ParserValue::Table(data.policy.mod_flags);
    data.validate().unwrap();
    let f = pure(&mut data, id);
    f.provenance.constructor = None;
    let table = data.policy.mod_flags;
    pure(&mut data, id).body = ParserFactoryExpr::Table(vec![ParserFactoryField::List(
        ParserFactoryExpr::ConstantField {
            table,
            key: "UnprovidedField".into(),
        },
    )]);
    data.validate().unwrap();
}

#[test]
fn invalid_expression_structure_is_rejected_without_global_runtime_type_checks() {
    let base = data();
    let id = pure_id(&base);
    let bad = [
        ParserFactoryExpr::Argument(0),
        ParserFactoryExpr::Table(vec![ParserFactoryField::List(ParserFactoryExpr::Argument(
            128,
        ))]),
        ParserFactoryExpr::Table(vec![ParserFactoryField::List(
            ParserFactoryExpr::CapturedScalar { upvalue: 128 },
        )]),
        ParserFactoryExpr::Table(vec![ParserFactoryField::List(
            ParserFactoryExpr::ConstantField {
                table: ParserTableId(0),
                key: "X".into(),
            },
        )]),
        ParserFactoryExpr::Table(vec![ParserFactoryField::List(ParserFactoryExpr::Literal(
            ParserFactoryLiteral::Number(f64::NAN),
        ))]),
        ParserFactoryExpr::Table(vec![
            ParserFactoryField::Named {
                key: "x".into(),
                value: ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
            },
            ParserFactoryField::Named {
                key: "x".into(),
                value: ParserFactoryExpr::Literal(ParserFactoryLiteral::Boolean(false)),
            },
        ]),
    ];
    for body in bad {
        let mut changed = base.clone();
        let f = pure(&mut changed, id);
        f.body = body;
        f.provenance.constructor = None;
        assert!(changed.validate().is_err());
    }
}

#[test]
fn factory_depth_and_scalar_bytes_are_bounded_before_native_compilation() {
    let mut data = data();
    let id = pure_id(&data);
    let mut body = ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil);
    for _ in 0..34 {
        body = ParserFactoryExpr::Table(vec![ParserFactoryField::List(body)]);
    }
    let f = pure(&mut data, id);
    f.body = body;
    f.provenance.constructor = None;
    assert!(data.validate().is_err());
    pure(&mut data, id).body = ParserFactoryExpr::Table(vec![ParserFactoryField::List(
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Text("x".repeat(4097))),
    )]);
    assert!(data.validate().is_err());
}
