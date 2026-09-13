//! Scalar DOUBLED creates owned output; shared dictionary writes stay deferred.
use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use poe_optimizer_engine::{
    lua_pattern::MatchBudget,
    modifier_parser::{
        CompiledModifierParser, ModifierTable, ModifierValue as V, ParseOutcome, ParserError,
    },
};
use std::sync::Arc;

fn text(s: &str) -> V {
    V::Bytes(s.as_bytes().to_vec())
}
fn row(mods: &ModifierTable, index: i64) -> &ModifierTable {
    mods.indexed_value(index).as_table().unwrap()
}
fn parse(parser: &CompiledModifierParser, line: &[u8]) -> ParseOutcome {
    parser.parse(line, &mut MatchBudget::default()).unwrap()
}
fn table(data: &mut ModifierParserData, value: ParserTable) -> ParserValue {
    data.tables.push(value);
    ParserValue::Table(ParserTableId(data.tables.len() as u32))
}
fn fields(entries: impl IntoIterator<Item = (&'static str, ParserValue)>) -> ParserTable {
    ParserTable {
        fields: entries.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        ..Default::default()
    }
}
fn fixture(
    name: ParserValue,
    alter: impl FnOnce(&mut ModifierParserData),
) -> CompiledModifierParser {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    data.programs = Default::default();
    for id in data.dictionaries.values() {
        data.tables[id.0 as usize - 1] = ParserTable::default();
    }
    data.tables[data.dictionaries[&ParserDictionary::Form].0 as usize - 1]
        .fields
        .insert("echo".into(), ParserValue::Text("DOUBLED".into()));
    if !matches!(name, ParserValue::Nil) {
        data.tables[data.dictionaries[&ParserDictionary::ModName].0 as usize - 1]
            .fields
            .insert("caller".into(), name);
    }
    alter(&mut data);
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn assert_pair(
    mods: &ModifierTable,
    name: &str,
    prefix: &str,
    suffix: &str,
    limit_suffix: &str,
    values: [f64; 3],
) {
    assert_eq!(mods.indexed.len(), 2);
    assert!(mods.fields.is_empty());
    let first = row(mods, 1);
    let second = row(mods, 2);
    assert_eq!(first.field("name"), &text(name));
    assert_eq!(
        second.field("name"),
        &text(&format!("{prefix}{name}{suffix}"))
    );
    assert_eq!(first.field("type"), &text("MORE"));
    assert_eq!(second.field("type"), &text("OVERRIDE"));
    assert_eq!(first.field("value"), &V::Number(values[0]));
    assert_eq!(second.field("value"), &V::Number(values[1]));
    for item in [first, second] {
        assert_eq!(item.fields.len(), 5);
        assert_eq!(item.field("flags"), &V::Number(0.0));
        assert_eq!(item.field("keywordFlags"), &V::Number(0.0));
    }
    assert_eq!(first.indexed.len(), 1);
    assert!(second.indexed.is_empty());
    let tag = first.indexed_value(1).as_table().unwrap();
    assert_eq!(tag.fields.len(), 4);
    assert!(tag.indexed.is_empty());
    assert_eq!(tag.field("type"), &text("Multiplier"));
    assert_eq!(tag.field("var"), &text(&format!("{name}{suffix}")));
    assert_eq!(tag.field("globalLimit"), &V::Number(values[2]));
    assert_eq!(
        tag.field("globalLimitKey"),
        &text(&format!("{name}{limit_suffix}"))
    );
}
#[test]
fn four_real_rune_lines_keep_two_rows_and_the_actual_unparsed_remainder() {
    let snapshot = bundled_snapshot().unwrap();
    let parser = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    for (line, name, extra) in [
        ("Causes Double Stun Buildup", "EnemyHeavyStunBuildup", None),
        (
            "Flammability Magnitude is doubled",
            "EnemyIgniteChance",
            None,
        ),
        (
            "Double Stun Threshold while Shield is Raised",
            "StunThreshold",
            Some("  while Shield is Raised "),
        ),
        ("Runic Ward Regeneration Rate is doubled", "WardRegen", None),
    ] {
        let result = parse(&parser, line.as_bytes());
        assert_eq!(result.extra.as_deref(), extra.map(str::as_bytes), "{line}");
        assert_pair(
            result.modifiers.as_ref().unwrap(),
            name,
            "Multiplier:",
            "Doubled",
            "DoubledLimit",
            [100.0, 1.0, 100.0],
        );
    }
}
#[test]
fn arbitrary_scalar_names_and_all_six_policy_operands_are_consumed() {
    for (name, rendered) in [
        (ParserValue::Text("CallerλStat".into()), "CallerλStat"),
        (ParserValue::Number(42.5), "42.5"),
        (ParserValue::Number(-0.0), "-0"),
        (ParserValue::Text("".into()), ""),
    ] {
        let parser = fixture(name, |data| {
            data.policy.doubled_multiplier_prefix = "P\0:".into();
            data.policy.doubled_name_suffix = "_Changed".into();
            data.policy.doubled_limit_suffix = "_Ceiling".into();
            data.policy.doubled_more = -3.5;
            data.policy.doubled_override = 7.0;
            data.policy.doubled_global_limit = 0.0;
        });
        let result = parse(&parser, b"caller echo");
        assert!(result.extra.is_none());
        assert_pair(
            result.modifiers.as_ref().unwrap(),
            rendered,
            "P\0:",
            "_Changed",
            "_Ceiling",
            [-3.5, 7.0, 0.0],
        );
    }
}
#[test]
fn nil_and_false_names_short_circuit_while_true_and_functions_error_at_concat() {
    for value in [ParserValue::Nil, ParserValue::Boolean(false)] {
        let parser = fixture(value, |_| {});
        let result = parse(&parser, b"caller echo");
        assert_eq!(result.modifiers, Some(ModifierTable::default()));
        assert_eq!(result.extra, Some(b"caller  ".to_vec()));
    }
    let callback = bundled_snapshot().unwrap().modifier_parser().data().helpers["flag"];
    for value in [ParserValue::Boolean(true), ParserValue::Callback(callback)] {
        let parser = fixture(value, |_| {});
        assert_eq!(
            parser.parse(b"caller echo", &mut MatchBudget::default()),
            Err(ParserError::SourceError(
                "concatenation of a non-string value".into()
            ))
        );
    }
}
fn tagged_parser(wrapped: bool) -> CompiledModifierParser {
    fixture(ParserValue::Text("Caller".into()), |data| {
        let condition = table(
            data,
            fields([
                ("type", ParserValue::Text("Condition".into())),
                ("var", ParserValue::Text("CallerCondition".into())),
            ]),
        );
        let mut contribution = fields([
            ("tag", condition),
            ("flags", ParserValue::Number(8.0)),
            ("keywordFlags", ParserValue::Number(16.0)),
            ("modSuffix", ParserValue::Text(":display".into())),
        ]);
        if wrapped {
            contribution
                .fields
                .insert("addToMinion".into(), ParserValue::Boolean(true));
        }
        let contribution = table(data, contribution);
        data.tables[data.dictionaries[&ParserDictionary::ModTag].0 as usize - 1]
            .fields
            .insert("when called".into(), contribution);
    })
}
#[test]
fn common_tags_flags_and_suffix_remain_on_both_rows_with_only_one_limit_tag() {
    let parser = tagged_parser(false);
    let result = parse(&parser, b"caller echo when called");
    assert!(result.extra.is_none());
    let mods = result.modifiers.unwrap();
    for (index, name, tags) in [
        (1, "Caller:display", 2),
        (2, "Multiplier:CallerDoubled:display", 1),
    ] {
        let item = row(&mods, index);
        assert_eq!(item.field("name"), &text(name));
        assert_eq!(item.field("flags"), &V::Number(8.0));
        assert_eq!(item.field("keywordFlags"), &V::Number(16.0));
        assert_eq!(item.indexed.len(), tags);
        let tag = item.indexed_value(1).as_table().unwrap();
        assert_eq!(tag.field("var"), &text("CallerCondition"));
    }
    let tag = row(&mods, 1).indexed_value(2).as_table().unwrap();
    assert_eq!(tag.field("var"), &text("CallerDoubled"));
    assert_eq!(tag.field("globalLimitKey"), &text("CallerDoubledLimit"));
}
#[test]
fn existing_minion_wrapper_keeps_the_first_effects_extra_tag() {
    let parser = tagged_parser(true);
    let result = parse(&parser, b"caller echo when called");
    assert!(result.extra.is_none());
    let mods = result.modifiers.unwrap();
    assert_eq!(mods.indexed.len(), 2);
    for (index, kind, count) in [(1, "MORE", 2), (2, "OVERRIDE", 1)] {
        let wrapper = row(&mods, index);
        assert_eq!(wrapper.field("name"), &text("MinionModifier"));
        assert_eq!(wrapper.field("type"), &text("LIST"));
        let effect = wrapper
            .field("value")
            .as_table()
            .unwrap()
            .field("mod")
            .as_table()
            .unwrap();
        assert_eq!(effect.field("type"), &text(kind));
        assert_eq!(effect.indexed.len(), count);
    }
}
#[test]
fn table_names_remain_explicitly_deferred_without_modifying_later_parses() {
    let parser = fixture(ParserValue::Nil, |data| {
        let names = table(
            data,
            ParserTable {
                indexed: [
                    (1, ParserValue::Text("First".into())),
                    (2, ParserValue::Text("Second".into())),
                    (3, ParserValue::Text("Third".into())),
                ]
                .into(),
                ..Default::default()
            },
        );
        data.tables[data.dictionaries[&ParserDictionary::ModName].0 as usize - 1]
            .fields
            .insert("caller".into(), names);
        data.tables[data.dictionaries[&ParserDictionary::Form].0 as usize - 1]
            .fields
            .insert("(%d+) increased".into(), ParserValue::Text("INC".into()));
    });
    let baseline = parse(&parser, b"caller 5 increased");
    for _ in 0..2 {
        assert_eq!(
            parser.parse(b"caller echo", &mut MatchBudget::default()),
            Err(ParserError::Deferred {
                stage: "shared dictionary mutation in doubled form",
                callback: None,
            })
        );
        assert_eq!(parse(&parser, b"caller 5 increased"), baseline);
    }
    let mods = baseline.modifiers.unwrap();
    assert_eq!(mods.indexed.len(), 3);
    for (index, name) in [(1, "First"), (2, "Second"), (3, "Third")] {
        assert_eq!(row(&mods, index).field("name"), &text(name));
    }
}
#[test]
fn returned_rows_and_nested_tags_are_fresh_on_every_parse() {
    let parser = tagged_parser(false);
    let mut first = parse(&parser, b"caller echo when called");
    let baseline = parse(&parser, b"caller echo when called");
    assert_eq!(first, baseline);
    let mods = first.modifiers.as_mut().unwrap();
    let V::Table(first_row) = mods.indexed.get_mut(&1).unwrap() else {
        panic!()
    };
    let V::Table(other_row) = baseline.modifiers.as_ref().unwrap().indexed_value(1) else {
        panic!()
    };
    assert!(!Arc::ptr_eq(first_row, other_row));
    let row = Arc::get_mut(first_row).unwrap();
    let V::Table(first_tag) = row.indexed.get_mut(&1).unwrap() else {
        panic!()
    };
    let V::Table(other_tag) = other_row.indexed_value(1) else {
        panic!()
    };
    assert!(!Arc::ptr_eq(first_tag, other_tag));
    Arc::get_mut(first_tag)
        .unwrap()
        .fields
        .insert("var".into(), text("CallerChanged"));
    assert_eq!(parse(&parser, b"caller echo when called"), baseline);
}
