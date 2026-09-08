use poe_optimizer_engine::{
    conditions::{
        ConditionProgram, ConditionProgramActor, ConditionProgramInput, ConditionQuery,
        ConditionResolver, ConditionStoreInput, ConditionValue, FlagModifierInput,
        ScalarConditions,
    },
    modifiers::{ModifierError, ModifierStoreKind, QueryContext},
};

fn flag(name: &str, source: Option<&str>, value: bool) -> FlagModifierInput {
    FlagModifierInput {
        name: name.into(),
        value: ConditionValue::Boolean(value),
        flags: 0,
        keyword_flags: 0,
        source: source.map(str::to_owned),
        tags: vec![],
    }
}
fn program(stores: Vec<ConditionStoreInput>) -> ConditionProgram {
    ConditionProgram::try_new(ConditionProgramInput {
        stores,
        actors: vec![ConditionProgramActor::default()],
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn flag_source_bypass_belongs_to_each_producer_store_kind() {
    for kind in [ModifierStoreKind::ModDb, ModifierStoreKind::ModList] {
        let input = program(vec![ConditionStoreInput {
            kind,
            flags: vec![flag("Condition:Ready", Some("Item:7"), true)],
            ..Default::default()
        }]);
        let values = [ScalarConditions::new()];
        for (source, without_bypass) in [
            ("Item", true),
            ("Item:7", false),
            ("Other", false),
            ("", false),
        ] {
            let context = QueryContext {
                source: Some(source.into()),
                ..Default::default()
            };
            for bypass in [false, true] {
                let mut query = ConditionQuery::new(&context, &values);
                query.ignore_source_in_check_conditions = bypass;
                let bound = input.bind(0, &query).unwrap();
                let expected = without_bypass || (bypass && kind == ModifierStoreKind::ModDb);
                assert_eq!(
                    bound.flag(&["Condition:Ready"]).unwrap(),
                    expected.then_some(true)
                );
                assert_eq!(
                    bound.get_condition("Ready", false).unwrap().truthy(),
                    expected
                );
            }
        }
    }
}

#[test]
fn mixed_parent_flag_queries_keep_child_context_and_local_source_rules() {
    let values = [ScalarConditions::new(), ScalarConditions::new()];
    let context = QueryContext {
        source: Some("Selected".into()),
        ..Default::default()
    };
    let mut query = ConditionQuery::new(&context, &values);
    query.ignore_source_in_check_conditions = true;
    for (child, parent, expected) in [
        (
            ModifierStoreKind::ModList,
            ModifierStoreKind::ModDb,
            Some(true),
        ),
        (ModifierStoreKind::ModDb, ModifierStoreKind::ModList, None),
    ] {
        let input = program(vec![
            ConditionStoreInput {
                kind: child,
                parent: Some(1),
                ..Default::default()
            },
            ConditionStoreInput {
                kind: parent,
                flags: vec![flag("Ready", Some("Other:1"), true)],
                ..Default::default()
            },
        ]);
        assert_eq!(
            input.bind(0, &query).unwrap().flag(&["Ready"]).unwrap(),
            expected
        );
    }
}

#[test]
fn reached_list_missing_source_errors_before_false_value_or_bypass() {
    let values = [ScalarConditions::new()];
    let context = QueryContext {
        source: Some("Selected".into()),
        ..Default::default()
    };
    let mut query = ConditionQuery::new(&context, &values);
    query.ignore_source_in_check_conditions = true;
    for value in [false, true] {
        let input = program(vec![ConditionStoreInput {
            kind: ModifierStoreKind::ModList,
            flags: vec![flag("Ready", None, value)],
            ..Default::default()
        }]);
        assert_eq!(
            input.bind(0, &query).unwrap().flag(&["Ready"]),
            Err(ModifierError::MissingSource {
                layer: 0,
                modifier: 0
            })
        );
        // A missing-source record with a different name is never reached.
        assert_eq!(
            input.bind(0, &query).unwrap().flag(&["Absent"]).unwrap(),
            None
        );
    }
}

#[test]
fn bound_store_kinds_follow_parent_references_not_storage_positions() {
    let input = program(vec![
        ConditionStoreInput::default(),
        ConditionStoreInput {
            kind: ModifierStoreKind::ModList,
            parent: Some(0),
            ..Default::default()
        },
        ConditionStoreInput {
            kind: ModifierStoreKind::ModList,
            parent: Some(1),
            ..Default::default()
        },
    ]);
    let values = [
        ScalarConditions::new(),
        ScalarConditions::new(),
        ScalarConditions::new(),
    ];
    let context = QueryContext::default();
    let query = ConditionQuery::new(&context, &values);
    let bound = input.bind(2, &query).unwrap();
    assert_eq!(bound.store_layer_count(), 3);
    assert_eq!(bound.store_kind(0), Some(ModifierStoreKind::ModList));
    assert_eq!(bound.store_kind(1), Some(ModifierStoreKind::ModList));
    assert_eq!(bound.store_kind(2), Some(ModifierStoreKind::ModDb));
    assert_eq!(bound.store_kind(3), None);
    assert_eq!(bound.store_kind(usize::MAX), None);
}
