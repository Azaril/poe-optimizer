//! Closed substitutions stay source/data driven and catalog scoped.
use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use poe_optimizer_engine::{
    lua_pattern::MatchBudget,
    modifier_parser::{CompiledModifierParser, ModifierValue as V},
};

fn literal(text: &str) -> ParserFactoryExpr {
    ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(text.into()))
}
fn substitute(
    value: ParserFactoryExpr,
    pattern: &str,
    replacement: ParserFactoryReplacement,
) -> ParserFactoryExpr {
    ParserFactoryExpr::Gsub {
        value: Box::new(value),
        pattern: pattern.into(),
        replacement,
    }
}
fn parser(expression: ParserFactoryExpr) -> CompiledModifierParser {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    data.programs = Default::default();
    let helper = data.helpers["flag"];
    let owner = data
        .factories
        .iter()
        .find_map(|(id, f)| {
            (matches!(f, ParserFactoryDisposition::Pure(f) if f.provenance.constructor.is_none())
                && data.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|v| v.name == "flag" && v.value == ParserValue::Callback(helper)))
            .then_some(*id)
        })
        .unwrap();
    for id in data.dictionaries.values() {
        data.tables[id.0 as usize - 1] = ParserTable::default();
    }
    data.tables[data.dictionaries[&ParserDictionary::Special].0 as usize - 1]
        .fields
        .insert("^factory (.*)$".into(), ParserValue::Callback(owner));
    let ParserFactoryDisposition::Pure(factory) = data.factories.get_mut(&owner).unwrap() else {
        panic!()
    };
    factory.parameter_count = 2;
    factory.body =
        ParserFactoryExpr::Table(vec![ParserFactoryField::List(ParserFactoryExpr::Flag {
            helper,
            args: vec![
                expression,
                ParserFactoryExpr::Table(vec![
                    ParserFactoryField::Named {
                        key: "type".into(),
                        value: literal("Condition"),
                    },
                    ParserFactoryField::Named {
                        key: "var".into(),
                        value: literal("CallerCondition"),
                    },
                ]),
            ],
        })]);
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn name(parser: &CompiledModifierParser, line: &[u8]) -> Vec<u8> {
    let result = parser.parse(line, &mut MatchBudget::default()).unwrap();
    assert!(result.extra.is_none());
    let row = result
        .modifiers
        .as_ref()
        .unwrap()
        .indexed_value(1)
        .as_table()
        .unwrap();
    assert_eq!(row.field("type"), &V::Bytes(b"FLAG".to_vec()));
    assert_eq!(row.field("value"), &V::Boolean(true));
    assert_eq!(row.field("flags"), &V::Number(0.0));
    assert_eq!(row.field("keywordFlags"), &V::Number(0.0));
    row.field("name").as_bytes().unwrap().to_vec()
}
#[test]
fn actual_buildup_factory_emits_the_complete_local_condition_flag() {
    let snapshot = bundled_snapshot().unwrap();
    let parser = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    let result = parser
        .parse(
            b"All damage with this Weapon causes Electrocution buildup",
            &mut MatchBudget::default(),
        )
        .unwrap();
    assert!(result.extra.is_none());
    let modifiers = result.modifiers.as_ref().unwrap();
    assert_eq!(modifiers.indexed.len(), 1);
    let row = modifiers.indexed_value(1).as_table().unwrap();
    assert_eq!(row.field("name"), &V::Bytes(b"CanElectrocution".to_vec()));
    assert_eq!(row.field("type"), &V::Bytes(b"FLAG".to_vec()));
    assert_eq!(row.field("value"), &V::Boolean(true));
    assert_eq!(row.field("flags"), &V::Number(0.0));
    assert_eq!(row.field("keywordFlags"), &V::Number(0.0));
    assert_eq!(row.indexed.len(), 1);
    let condition = row.indexed_value(1).as_table().unwrap();
    assert_eq!(condition.field("type"), &V::Bytes(b"Condition".to_vec()));
    assert_eq!(condition.field("var"), &V::Bytes(b"{Hand}Attack".to_vec()));
}
#[test]
fn chained_substitutions_use_injected_patterns_and_keep_tabs_and_raw_bytes() {
    let value = substitute(
        ParserFactoryExpr::Argument(1),
        "^%l",
        ParserFactoryReplacement::StringUpper,
    );
    let value = substitute(value, " %l", ParserFactoryReplacement::StringUpper);
    let value = substitute(value, " ", ParserFactoryReplacement::Text(String::new()));
    let parser = parser(ParserFactoryExpr::Concat {
        left: Box::new(literal("Caller")),
        right: Box::new(value),
    });
    assert_eq!(
        name(&parser, b"factory freeze  shock"),
        b"CallerFreezeShock"
    );
    assert_eq!(
        name(&parser, b"factory freeze\tshock"),
        b"CallerFreeze\tshock"
    );
    assert_eq!(name(&parser, b"factory a\0\xff"), b"CallerA\0\xff");
}
#[test]
fn compiled_substitutions_are_reused_without_cross_catalog_replacement_state() {
    let first = parser(substitute(
        ParserFactoryExpr::Argument(1),
        "_",
        ParserFactoryReplacement::Text("X".into()),
    ));
    let second = parser(substitute(
        ParserFactoryExpr::Argument(1),
        "_",
        ParserFactoryReplacement::Text("Y".into()),
    ));
    for _ in 0..3 {
        assert_eq!(name(&first, b"factory a_b"), b"aXb");
        assert_eq!(name(&second, b"factory a_b"), b"aYb");
    }
}
#[test]
fn invalid_substitution_is_lazy_until_the_selected_factory_reaches_it() {
    let parser = parser(substitute(
        ParserFactoryExpr::Argument(1),
        "[",
        ParserFactoryReplacement::Text(String::new()),
    ));
    let unmatched = parser
        .parse(b"unrelated input", &mut MatchBudget::default())
        .unwrap();
    assert!(unmatched.modifiers.is_none());
    assert!(
        parser
            .parse(b"factory abc", &mut MatchBudget::default())
            .is_err()
    );
}
