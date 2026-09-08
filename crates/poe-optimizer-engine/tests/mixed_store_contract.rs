//! Mixed local stores: explicit kind, source order, bounded scratch and compatibility.
use poe_optimizer_engine::{conditions::*, modifiers::*};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
    collections::BTreeMap,
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
fn row(name: &str, kind: NumericKind, value: f64, source: Option<&str>) -> TaggedModifierInput {
    ModifierInput {
        name: name.into(),
        kind: ModifierKind::Numeric(kind),
        value: ModifierValue::Number(value),
        flags: 0,
        keyword_flags: 0,
        source: source.map(str::to_owned),
        tag_kinds: vec![],
    }
    .into()
}
fn layer(kind: ModifierStoreKind, modifiers: Vec<TaggedModifierInput>) -> ModifierLayerInput {
    ModifierLayerInput { kind, modifiers }
}
fn tag(mut row: TaggedModifierInput, name: &str) -> TaggedModifierInput {
    row.tags.push(ModifierTag::Condition {
        variables: ConditionVariables::One(name.into()),
        negated: false,
    });
    row
}
struct Probe {
    kinds: Vec<ModifierStoreKind>,
    seen: RefCell<Vec<String>>,
    fail: Option<&'static str>,
    enabled: bool,
}
impl ConditionResolver for Probe {
    fn store_layer_count(&self) -> usize {
        self.kinds.len()
    }
    fn store_kind(&self, index: usize) -> Option<ModifierStoreKind> {
        self.kinds.get(index).copied()
    }
    fn matches(&self, tags: &[ModifierTag]) -> Result<bool, ModifierError> {
        for tag in tags {
            if let ModifierTag::Condition {
                variables: ConditionVariables::One(name),
                ..
            } = tag
            {
                self.seen.borrow_mut().push(name.clone());
                if self.fail == Some(name.as_str()) {
                    return Err(ModifierError::InvalidConditionContext {
                        reason: name.clone(),
                    });
                }
            }
        }
        Ok(self.enabled)
    }
}

#[test]
fn old_constructors_default_to_moddb_and_all_constructors_share_the_layer_bound() {
    let input = row("A", NumericKind::Base, 1.0, None);
    for database in [
        ModifierDatabase::try_new(vec![vec![input.modifier.clone()]]).unwrap(),
        ModifierDatabase::try_new_tagged(vec![vec![input.clone()]]).unwrap(),
    ] {
        assert_eq!(database.store_kind(0), Some(ModifierStoreKind::ModDb));
        assert_eq!(database.store_kind(1), None);
    }
    for count in [0, MAX_MODIFIER_LAYERS, MAX_MODIFIER_LAYERS + 1] {
        let raw = ModifierDatabase::try_new(vec![vec![]; count]);
        let tagged = ModifierDatabase::try_new_tagged(vec![vec![]; count]);
        let mixed = ModifierDatabase::try_new_layers(vec![ModifierLayerInput::default(); count]);
        for result in [raw, tagged, mixed] {
            if count <= MAX_MODIFIER_LAYERS {
                let database = result.unwrap();
                assert_eq!(database.layer_count(), count);
                assert_eq!(
                    database
                        .sum(SumKind::Base, &QueryContext::default(), &["A"])
                        .unwrap(),
                    0.0
                );
            } else {
                assert_eq!(result.unwrap_err(), ModifierError::TooManyLayers { count });
            }
        }
    }
}

#[test]
fn source_matching_is_decided_by_each_layer_and_missing_list_source_is_an_error() {
    use ModifierStoreKind::*;
    let query = QueryContext {
        source: Some("Item:local".into()),
        ..Default::default()
    };
    for (kinds, expected) in [([ModList, ModDb], 11.0), ([ModDb, ModList], 7.0)] {
        let database = ModifierDatabase::try_new_layers(vec![
            layer(
                kinds[0],
                vec![row("A", NumericKind::Base, 7.0, Some("Item:local"))],
            ),
            layer(
                kinds[1],
                vec![row("A", NumericKind::Base, 11.0, Some("Item:local"))],
            ),
        ])
        .unwrap();
        assert_eq!(
            database.sum(SumKind::Base, &query, &["A"]).unwrap(),
            expected
        );
        let prefix = QueryContext {
            source: Some("Item".into()),
            ..Default::default()
        };
        assert_eq!(database.sum(SumKind::Base, &prefix, &["A"]).unwrap(), 18.0);
    }
    let database = ModifierDatabase::try_new_layers(vec![
        layer(ModDb, vec![row("A", NumericKind::Base, 5.0, None)]),
        layer(ModList, vec![row("A", NumericKind::Base, 3.0, None)]),
    ])
    .unwrap();
    assert_eq!(
        database.sum(SumKind::Base, &query, &["A"]).unwrap_err(),
        ModifierError::MissingSource {
            layer: 1,
            modifier: 0
        }
    );
    let mut hidden = row("A", NumericKind::Base, 3.0, None);
    hidden.modifier.flags = 1;
    let database = ModifierDatabase::try_new_layers(vec![layer(ModList, vec![hidden])]).unwrap();
    assert_eq!(database.sum(SumKind::Base, &query, &["A"]).unwrap(), 0.0);
}

#[test]
fn child_first_evaluation_preserves_parent_grouping_and_source_error_order() {
    use ModifierStoreKind::*;
    let kinds = vec![ModList, ModDb, ModList];
    let database = ModifierDatabase::try_new_layers(
        kinds
            .iter()
            .enumerate()
            .map(|(index, &kind)| {
                layer(
                    kind,
                    vec![
                        tag(
                            row("B", NumericKind::Base, 0.0, Some("Config:x")),
                            &format!("B{index}"),
                        ),
                        tag(
                            row(
                                "A",
                                NumericKind::Base,
                                [1e16, -1e16, 1.0][index],
                                Some("Config:x"),
                            ),
                            &format!("A{index}"),
                        ),
                    ],
                )
            })
            .collect(),
    )
    .unwrap();
    let probe = Probe {
        kinds: kinds.clone(),
        seen: RefCell::default(),
        fail: None,
        enabled: true,
    };
    assert_eq!(
        database
            .sum_with_conditions(SumKind::Base, &QueryContext::default(), &["A", "B"], &probe)
            .unwrap(),
        0.0
    );
    assert_eq!(*probe.seen.borrow(), ["A0", "B0", "A1", "B1", "A2", "B2"]);
    let mut inputs = vec![
        layer(
            ModDb,
            vec![tag(
                row("A", NumericKind::Base, 1.0, Some("Config:x")),
                "ChildError",
            )],
        ),
        layer(ModList, vec![row("A", NumericKind::Base, 1.0, None)]),
    ];
    let context = QueryContext {
        source: Some("Config".into()),
        ..Default::default()
    };
    let probe = Probe {
        kinds: vec![ModDb, ModList],
        seen: RefCell::default(),
        fail: Some("ChildError"),
        enabled: true,
    };
    let database = ModifierDatabase::try_new_layers(inputs.clone()).unwrap();
    assert_eq!(
        database
            .sum_with_conditions(SumKind::Base, &context, &["A"], &probe)
            .unwrap_err(),
        ModifierError::InvalidConditionContext {
            reason: "ChildError".into()
        }
    );
    inputs[0].modifiers[0].modifier.source = None;
    let database = ModifierDatabase::try_new_layers(inputs).unwrap();
    assert_eq!(
        database
            .sum_with_conditions(SumKind::Base, &context, &["A"], &probe)
            .unwrap_err(),
        ModifierError::MissingSource {
            layer: 1,
            modifier: 0
        }
    );
}

#[test]
fn every_numeric_consumer_rejects_kind_or_query_context_mismatch() {
    use ModifierStoreKind::*;
    let database = ModifierDatabase::try_new_layers(vec![
        layer(ModList, vec![row("A", NumericKind::Base, 1.0, None)]),
        layer(ModDb, vec![]),
    ])
    .unwrap();
    let program = ConditionProgram::try_new(ConditionProgramInput {
        stores: vec![
            ConditionStoreInput {
                kind: ModList,
                parent: Some(1),
                ..Default::default()
            },
            ConditionStoreInput::default(),
        ],
        actors: vec![ConditionProgramActor::default()],
        ..Default::default()
    })
    .unwrap();
    let context = QueryContext::default();
    let tables = vec![ScalarConditions::new(); 2];
    let bound = program
        .bind(0, &ConditionQuery::new(&context, &tables))
        .unwrap();
    assert_eq!(
        database
            .sum_with_conditions(SumKind::Base, &context, &["A"], &bound)
            .unwrap(),
        1.0
    );
    let wrong = Probe {
        kinds: vec![ModList, ModList],
        seen: RefCell::default(),
        fail: None,
        enabled: true,
    };
    let precision = MorePrecision::default();
    let other = QueryContext {
        flags: 1,
        ..Default::default()
    };
    for (query, resolver) in [
        (&context, &wrong as &dyn ConditionResolver),
        (&other, &bound as &dyn ConditionResolver),
    ] {
        assert!(
            database
                .sum_with_conditions(SumKind::Base, query, &["A"], resolver)
                .is_err()
        );
        assert!(
            database
                .more_with_conditions(query, &["A"], &precision, resolver)
                .is_err()
        );
        assert!(
            database
                .override_with_conditions(query, &["A"], resolver)
                .is_err()
        );
        assert!(
            database
                .max_with_conditions(query, &["A"], resolver)
                .is_err()
        );
        assert!(
            database
                .sum_positive_with_conditions(SumKind::Base, query, "A", resolver)
                .is_err()
        );
    }
    let legacy = ConditionEnvironment::try_new(ConditionEnvironmentInput {
        store_conditions: vec![BTreeMap::new(); 2],
        actors: vec![ConditionActor::default()],
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        database
            .sum_with_conditions(SumKind::Base, &context, &["A"], &legacy)
            .unwrap_err(),
        ModifierError::StoreKindMismatch {
            layer: 0,
            numeric: ModList,
            conditions: Some(ModDb)
        }
    );
}

#[test]
fn mixed_layer_rounding_overrides_max_and_positive_rows_keep_their_existing_semantics() {
    use ModifierStoreKind::*;
    let precision = MorePrecision::try_new(BTreeMap::from([("Fine".into(), 4)])).unwrap();
    let context = QueryContext::default();
    for mask in 0..8 {
        let kinds: Vec<_> = (0..3)
            .map(|index| {
                if mask & (1 << index) == 0 {
                    ModDb
                } else {
                    ModList
                }
            })
            .collect();
        let layers = kinds
            .iter()
            .enumerate()
            .map(|(index, &kind)| {
                layer(
                    kind,
                    vec![
                        row(
                            "A",
                            NumericKind::Override,
                            if index == 0 { 0.0 } else { 99.0 },
                            Some("Config:x"),
                        ),
                        row(
                            "A",
                            NumericKind::Max,
                            [0.0, -1.0, 7.0][index],
                            Some("Config:x"),
                        ),
                        row(
                            "A",
                            NumericKind::Increased,
                            [3.0, -10.0, 4.0][index],
                            Some("Config:x"),
                        ),
                        row("Fine", NumericKind::More, 0.015, Some("Config:x")),
                        row("Coarse", NumericKind::More, 0.015, Some("Config:x")),
                    ],
                )
            })
            .collect();
        let database = ModifierDatabase::try_new_layers(layers).unwrap();
        assert_eq!(
            database.override_value(&context, &["A"]).unwrap(),
            Some(0.0)
        );
        assert_eq!(database.max(&context, &["A"]).unwrap(), Some(7.0));
        assert_eq!(
            database
                .sum_positive_values(SumKind::Increased, &context, "A")
                .unwrap(),
            7.0
        );
        let local = ((1.0001_f64 * (1.0 + 0.015 / 100.0) * 10000.0).floor()) / 10000.0;
        assert_eq!(
            database
                .more(&context, &["Fine", "Coarse"], &precision)
                .unwrap(),
            local * (local * local)
        );
        let exact = QueryContext {
            source: Some("Config:x".into()),
            ..Default::default()
        };
        assert_eq!(database.override_value(&exact, &["A"]).unwrap(), None);
        assert_eq!(database.max(&exact, &["A"]).unwrap(), None);
        assert_eq!(
            database
                .sum_positive_values(SumKind::Increased, &exact, "A")
                .unwrap(),
            0.0
        );
        assert_eq!(
            database
                .more(&exact, &["Fine", "Coarse"], &precision)
                .unwrap(),
            1.0
        );
    }
}

#[test]
fn maximum_depth_mixed_sums_use_stack_scratch_and_dynamic_conditions_without_allocating() {
    use ModifierStoreKind::*;
    let kinds: Vec<_> = (0..MAX_MODIFIER_LAYERS)
        .map(|index| if index % 2 == 0 { ModList } else { ModDb })
        .collect();
    let database = ModifierDatabase::try_new_layers(
        kinds
            .iter()
            .enumerate()
            .map(|(index, &kind)| {
                layer(
                    kind,
                    vec![tag(row("A", NumericKind::Base, index as f64, None), "Gate")],
                )
            })
            .collect(),
    )
    .unwrap();
    let program = ConditionProgram::try_new(ConditionProgramInput {
        stores: kinds
            .iter()
            .enumerate()
            .map(|(index, &kind)| ConditionStoreInput {
                kind,
                parent: (index + 1 < kinds.len()).then_some(index + 1),
                ..Default::default()
            })
            .collect(),
        actors: vec![ConditionProgramActor::default()],
        ..Default::default()
    })
    .unwrap();
    let context = QueryContext::default();
    let mut tables = vec![ScalarConditions::new(); kinds.len()];
    tables[0].insert("Gate".into(), ConditionValue::Boolean(false));
    let (checksum, count) = allocations(|| {
        let mut total = 0.0;
        for index in 0..128 {
            *tables[0].get_mut("Gate").unwrap() = ConditionValue::Boolean(index % 2 == 0);
            let bound = program
                .bind(0, &ConditionQuery::new(&context, &tables))
                .unwrap();
            let result = database
                .sum_with_conditions(SumKind::Base, &context, &["A"], &bound)
                .unwrap();
            assert_eq!(result, if index % 2 == 0 { 32640.0 } else { 0.0 });
            total += result;
        }
        total
    });
    assert_eq!(checksum, 64.0 * 32640.0);
    assert_eq!(count, 0);
}

#[test]
fn mixed_constructor_rejects_unrepresented_numeric_tags_and_values_in_all_layers() {
    use ModifierStoreKind::*;
    for kind in [ModDb, ModList] {
        let mut unknown = row("A", NumericKind::Base, 1.0, None);
        unknown
            .tags
            .push(ModifierTag::Unsupported("GlobalLimit".into()));
        assert!(
            ModifierDatabase::try_new_layers(vec![layer(kind, vec![]), layer(kind, vec![unknown])])
                .is_err()
        );
        let mut unknown = row("A", NumericKind::Base, 1.0, None);
        unknown.modifier.value = ModifierValue::Unsupported {
            kind: "function".into(),
        };
        assert!(ModifierDatabase::try_new_layers(vec![layer(kind, vec![unknown])]).is_err());
    }
}
