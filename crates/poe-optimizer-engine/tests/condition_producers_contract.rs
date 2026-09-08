//! Native boundary, error propagation, immutable sharing and allocation contracts.
use poe_optimizer_engine::{
    conditions::*,
    modifiers::*,
    multipliers::{ScalarSource, StatThreshold, StatThresholdValue, StatVariables},
    stats::ResolvedStatEnvironment,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    collections::BTreeMap,
    hint::black_box,
};
// Count only the measured test thread; delegate all allocation unchanged.
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
fn record_allocation() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: Exact caller layout is forwarded to System.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: Exact caller layout is forwarded to System.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        // SAFETY: All pointer/layout arguments are passed unchanged to System.
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer was allocated by System with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn allocations<T>(run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNTING.with(|flag| flag.set(false));
        }
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|flag| flag.set(true));
    let reset = Reset;
    let result = run();
    drop(reset);
    (result, ALLOCATIONS.with(Cell::get))
}
fn flag(name: &str, tags: Vec<FlagTag>) -> FlagModifierInput {
    FlagModifierInput {
        name: name.into(),
        value: ConditionValue::Boolean(true),
        flags: 0,
        keyword_flags: 0,
        source: Some("Config:test".into()),
        tags,
    }
}
fn condition(name: &str) -> FlagTag {
    FlagTag::Predicate(ModifierTag::Condition {
        variables: ConditionVariables::One(name.into()),
        negated: false,
    })
}
fn program(flags: Vec<FlagModifierInput>) -> ConditionProgram {
    ConditionProgram::try_new(ConditionProgramInput {
        stores: vec![ConditionStoreInput {
            flags,
            ..Default::default()
        }],
        actors: vec![ConditionProgramActor::default()],
        ..Default::default()
    })
    .unwrap()
}
fn numeric(kind: NumericKind, value: f64) -> TaggedModifierInput {
    TaggedModifierInput {
        modifier: ModifierInput {
            name: "Power".into(),
            kind: ModifierKind::Numeric(kind),
            value: ModifierValue::Number(value),
            flags: 0,
            keyword_flags: 0,
            source: Some("Config:test".into()),
            tag_kinds: vec![],
        },
        tags: vec![ModifierTag::Condition {
            variables: ConditionVariables::One("Ready".into()),
            negated: false,
        }],
    }
}

#[test]
fn reached_cycles_fail_but_overrides_and_earlier_source_short_circuits_avoid_them() {
    let program = program(vec![
        flag("Condition:A", vec![condition("B")]),
        flag("Condition:B", vec![condition("A")]),
    ]);
    let context = QueryContext::default();
    let tables = [ScalarConditions::new()];
    let query = ConditionQuery::new(&context, &tables);
    let bound = program.bind(0, &query).unwrap();
    assert!(
        bound
            .get_condition("A", false)
            .unwrap_err()
            .to_string()
            .contains("Recursive")
    );
    assert_eq!(
        bound.get_condition("A", true).unwrap(),
        ConditionResult::Boolean(false)
    );
    let overrides = BTreeMap::from([("B".into(), ConditionValue::Boolean(false))]);
    let query = ConditionQuery {
        overrides: &overrides,
        ..query
    };
    assert_eq!(
        program
            .bind(0, &query)
            .unwrap()
            .get_condition("A", false)
            .unwrap(),
        ConditionResult::Nil
    );
    let overrides = BTreeMap::from([("B".into(), ConditionValue::Number(0.0))]);
    let query = ConditionQuery {
        overrides: &overrides,
        ..query
    };
    assert_eq!(
        program
            .bind(0, &query)
            .unwrap()
            .get_condition("A", false)
            .unwrap(),
        ConditionResult::Boolean(true)
    );
    let mut inputs = program.input().clone();
    inputs.stores[0]
        .flags
        .insert(0, flag("Condition:A", vec![]));
    let program = ConditionProgram::try_new(inputs).unwrap();
    let query = ConditionQuery::new(&context, &tables);
    assert_eq!(
        program
            .bind(0, &query)
            .unwrap()
            .get_condition("A", false)
            .unwrap(),
        ConditionResult::Boolean(true)
    );
}

#[test]
fn every_numeric_query_consumes_the_same_flag_resolver_and_propagates_errors() {
    let program = program(vec![flag("Condition:Ready", vec![condition("Switch")])]);
    let context = QueryContext::default();
    let database = ModifierDatabase::try_new_tagged(vec![vec![
        numeric(NumericKind::Base, 11.0),
        numeric(NumericKind::Increased, 25.0),
        numeric(NumericKind::More, 30.0),
        numeric(NumericKind::Override, 0.0),
        numeric(NumericKind::Max, 7.0),
    ]])
    .unwrap();
    let precision = MorePrecision::default();
    for enabled in [false, true] {
        let tables = [BTreeMap::from([(
            "Switch".into(),
            ConditionValue::Boolean(enabled),
        )])];
        let query = ConditionQuery::new(&context, &tables);
        let bound = program.bind(0, &query).unwrap();
        assert_eq!(
            database
                .sum_with_conditions(SumKind::Base, &context, &["Power"], &bound)
                .unwrap(),
            if enabled { 11.0 } else { 0.0 }
        );
        assert_eq!(
            database
                .more_with_conditions(&context, &["Power"], &precision, &bound)
                .unwrap(),
            if enabled { 1.3 } else { 1.0 }
        );
        assert_eq!(
            database
                .max_with_conditions(&context, &["Power"], &bound)
                .unwrap(),
            enabled.then_some(7.0)
        );
        assert_eq!(
            database
                .override_with_conditions(&context, &["Power"], &bound)
                .unwrap(),
            enabled.then_some(0.0)
        );
        assert_eq!(
            database
                .sum_positive_with_conditions(SumKind::Increased, &context, "Power", &bound)
                .unwrap(),
            if enabled { 25.0 } else { 0.0 }
        );
        let other_context = QueryContext {
            source: Some("Config".into()),
            ..Default::default()
        };
        assert!(
            database
                .sum_with_conditions(SumKind::Base, &other_context, &["Power"], &bound)
                .is_err()
        );
        assert!(
            database
                .more_with_conditions(&other_context, &["Power"], &precision, &bound)
                .is_err()
        );
        assert!(
            database
                .max_with_conditions(&other_context, &["Power"], &bound)
                .is_err()
        );
        assert!(
            database
                .override_with_conditions(&other_context, &["Power"], &bound)
                .is_err()
        );
        assert!(
            database
                .sum_positive_with_conditions(SumKind::Increased, &other_context, "Power", &bound)
                .is_err()
        );
    }
    let program = self::program(vec![flag("Condition:Ready", vec![condition("Ready")])]);
    let tables = [ScalarConditions::new()];
    let query = ConditionQuery::new(&context, &tables);
    let bound = program.bind(0, &query).unwrap();
    assert!(
        database
            .sum_with_conditions(SumKind::Base, &context, &["Power"], &bound)
            .is_err()
    );
    assert!(
        database
            .more_with_conditions(&context, &["Power"], &precision, &bound)
            .is_err()
    );
    assert!(
        database
            .max_with_conditions(&context, &["Power"], &bound)
            .is_err()
    );
    assert!(
        database
            .override_with_conditions(&context, &["Power"], &bound)
            .is_err()
    );
    assert!(
        database
            .sum_positive_with_conditions(SumKind::Increased, &context, "Power", &bound)
            .is_err()
    );
}

#[test]
fn unsupported_entries_fail_before_any_false_gate_and_tables_are_validated_at_binding() {
    let base = program(vec![flag("Condition:A", vec![])]).input().clone();
    for tag in [
        FlagTag::Unsupported("PercentStat".into()),
        FlagTag::StatThreshold(StatThreshold {
            stats: StatVariables::One("Life".into()),
            threshold: StatThresholdValue::Constant(1.0),
            percent: Some(ScalarSource::Multiplier("Percent".into())),
            upper: false,
        }),
        FlagTag::StatThreshold(StatThreshold {
            stats: StatVariables::One("ManaUnreserved".into()),
            threshold: StatThresholdValue::Constant(1.0),
            percent: None,
            upper: false,
        }),
        FlagTag::Predicate(ModifierTag::GlobalEffect {
            effect_type: "Buff".into(),
            unscalable: true,
        }),
    ] {
        let mut input = base.clone();
        input.stores[0].flags[0].tags = vec![condition("Disabled"), tag];
        assert!(ConditionProgram::try_new(input).is_err());
    }
    let mut input = base.clone();
    input.stores[0].flags[0].value = ConditionValue::Unsupported("table".into());
    assert!(ConditionProgram::try_new(input).is_err());
    let mut input = base.clone();
    input.stores[0].parent = Some(0);
    assert!(ConditionProgram::try_new(input).is_err());
    let mut input = base.clone();
    input.actors[0].links.insert("parent".into(), 1);
    assert!(ConditionProgram::try_new(input).is_err());
    let program = ConditionProgram::try_new(base).unwrap();
    let context = QueryContext::default();
    let tables = [BTreeMap::from([(
        "Unused".into(),
        ConditionValue::Unsupported("function".into()),
    )])];
    assert!(
        program
            .bind(0, &ConditionQuery::new(&context, &tables))
            .is_err()
    );
    assert!(
        program
            .bind(0, &ConditionQuery::new(&context, &[]))
            .is_err()
    );
}

#[test]
fn temporary_query_binding_and_absent_nil_false_zero_text_results_are_distinct() {
    let program = program(vec![]);
    let context = QueryContext::default();
    let tables = [BTreeMap::from([
        ("Nil".into(), ConditionValue::Nil),
        ("False".into(), ConditionValue::Boolean(false)),
        ("Zero".into(), ConditionValue::Number(-0.0)),
        ("Text".into(), ConditionValue::Text("".into())),
    ])];
    let bound = program
        .bind(0, &ConditionQuery::new(&context, &tables))
        .unwrap();
    for name in ["Absent", "Nil", "False"] {
        assert_eq!(
            bound.get_condition(name, false).unwrap(),
            ConditionResult::Nil
        );
        assert_eq!(
            bound.get_condition(name, true).unwrap(),
            ConditionResult::Boolean(false)
        );
    }
    assert!(
        matches!(bound.get_condition("Zero",false).unwrap(),ConditionResult::Number(value) if value.to_bits()==(-0.0_f64).to_bits())
    );
    assert_eq!(
        bound.get_condition("Text", false).unwrap(),
        ConditionResult::Text("")
    );
    assert!(bound.get_condition("Text", false).unwrap().truthy());
    assert_eq!(bound.flag(&["Absent"]).unwrap(), None);
}

#[test]
fn varying_conditions_stats_and_parent_sources_have_zero_successful_query_allocations() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ConditionProgram>();
    let mut input = program(vec![flag(
        "Condition:Ready",
        vec![
            condition("Gate"),
            FlagTag::StatThreshold(StatThreshold {
                stats: StatVariables::One("Life".into()),
                threshold: StatThresholdValue::Constant(2.0),
                percent: None,
                upper: false,
            }),
        ],
    )])
    .input()
    .clone();
    input.stores.insert(
        0,
        ConditionStoreInput {
            parent: Some(1),
            ..Default::default()
        },
    );
    let program = ConditionProgram::try_new(input).unwrap();
    let context = QueryContext {
        source: Some("Config".into()),
        ..Default::default()
    };
    let database =
        ModifierDatabase::try_new_tagged(vec![vec![numeric(NumericKind::Base, 9.0)], vec![]])
            .unwrap();
    let mut tables = [
        BTreeMap::from([("Gate".into(), ConditionValue::Boolean(false))]),
        ScalarConditions::new(),
    ];
    let stats_low = [
        ResolvedStatEnvironment::try_new(
            Some(BTreeMap::from([("Life".into(), 1.0)])),
            BTreeMap::new(),
            vec![],
        )
        .unwrap(),
        ResolvedStatEnvironment::try_new(
            Some(BTreeMap::from([("Life".into(), 1000.0)])),
            BTreeMap::new(),
            vec![],
        )
        .unwrap(),
    ];
    let stats_high = [
        ResolvedStatEnvironment::try_new(
            Some(BTreeMap::from([("Life".into(), 2.0)])),
            BTreeMap::new(),
            vec![],
        )
        .unwrap(),
        stats_low[1].clone(),
    ];
    let (checksum, count) = allocations(|| {
        let mut checksum = 0.0;
        for index in 0..4096 {
            *tables[0].get_mut("Gate").unwrap() = ConditionValue::Boolean(index % 2 == 0);
            let stats = if index % 3 == 0 {
                &stats_high
            } else {
                &stats_low
            };
            let query = ConditionQuery {
                stats,
                ..ConditionQuery::new(&context, &tables)
            };
            let bound = program.bind(0, &query).unwrap();
            let ready = bound.get_condition("Ready", false).unwrap().truthy();
            assert_eq!(ready, index % 6 == 0);
            assert_eq!(bound.flag(&["Condition:Ready"]).unwrap().is_some(), ready);
            checksum += black_box(
                database
                    .sum_with_conditions(SumKind::Base, &context, &["Power"], &bound)
                    .unwrap(),
            );
        }
        checksum
    });
    assert_eq!(checksum, 683.0 * 9.0);
    assert_eq!(
        count, 0,
        "bind, FLAG, GetCondition and producer-aware numeric SUM allocate nothing"
    );
}

#[test]
fn compiled_name_buckets_keep_name_record_and_parent_order_amid_unrelated_flags() {
    let mut input = program(vec![]).input().clone();
    input.stores[0].parent = Some(1);
    input.stores.push(ConditionStoreInput::default());
    for layer in &mut input.stores {
        for index in 0..2048 {
            layer.flags.push(flag(
                &format!("Condition:Irrelevant{index}"),
                vec![condition("Cycle")],
            ));
        }
    }
    input.stores[0].flags.push(flag("Condition:Ready", vec![]));
    input.stores[0]
        .flags
        .push(flag("Condition:Ready", vec![condition("Cycle")]));
    input.stores[0]
        .flags
        .push(flag("Condition:Cycle", vec![condition("Cycle")]));
    input.stores[1]
        .flags
        .push(flag("Condition:Ready", vec![condition("Cycle")]));
    let program = ConditionProgram::try_new(input).unwrap();
    let context = QueryContext::default();
    let tables = [ScalarConditions::new(), ScalarConditions::new()];
    let bound = program
        .bind(0, &ConditionQuery::new(&context, &tables))
        .unwrap();
    let (_, count) = allocations(|| {
        for _ in 0..1024 {
            assert_eq!(
                bound.get_condition("Ready", false).unwrap(),
                ConditionResult::Boolean(true)
            );
            assert_eq!(
                bound.flag(&["Condition:Ready", "Condition:Cycle"]).unwrap(),
                Some(true)
            );
        }
    });
    assert_eq!(count, 0);
    assert!(bound.flag(&["Condition:Cycle", "Condition:Ready"]).is_err());
    assert!(
        program
            .bind(1, &ConditionQuery::new(&context, &tables))
            .unwrap()
            .get_condition("Ready", false)
            .is_ok()
    );
}
