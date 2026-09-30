//! The same private kernel serves whole-plan and sealed-stage execution.
use super::*;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("graph-execution", "v1").unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn quantity(value: f64) -> ParameterValue {
    ParameterValue::Quantity(
        FiniteQuantity::new(value, UnitDefId::parse(namespace(), "unit").unwrap()).unwrap(),
    )
}
fn literal(value: ParameterValue) -> ReadBinding {
    ReadBinding::Constant(Some(value))
}
fn final_read(index: usize) -> ReadBinding {
    ReadBinding::Final {
        effect: Some(index),
        complete: true,
    }
}
fn node(index: usize, source: ReadBinding, dependencies: Vec<usize>) -> EffectNode {
    let id = key(&format!("effect-{index}"));
    let stat = StatDefId::parse(namespace(), format!("value-{index}")).unwrap();
    EffectNode {
        key: EffectOccurrenceKey {
            invocation: ProgramOccurrenceKey {
                origin: RuleOrigin::Encounter,
                owner: SchemaSubject::Definition(stat.address()),
                program: key("program"),
                entity: ConcreteEntity::Actor(ActorKey::Player),
            },
            effect: id,
        },
        target: BoundEffectTarget::Value {
            key: PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat,
            },
        },
        operation: EffectOperation::Route { source },
        gates: vec![],
        dependencies,
    }
}
fn graph(effects: &[EffectNode]) -> ExecutionGraph<'_> {
    ExecutionGraph {
        effects,
        invocations: &[],
    }
}
fn chain(value: i64) -> Vec<EffectNode> {
    vec![
        node(0, literal(integer(value)), vec![]),
        node(1, final_read(0), vec![0]),
        node(2, final_read(1), vec![1]),
    ]
}

#[test]
fn split_predecessor_schedules_match_full_values_fold_order_and_work() {
    let effects = vec![
        node(0, literal(quantity(1e16)), vec![]),
        node(1, literal(quantity(-1e16)), vec![]),
        node(2, literal(quantity(1.0)), vec![]),
        node(
            3,
            ReadBinding::Reduction {
                effects: vec![0, 1, 2],
                reduction: ContributionReduction::Sum,
                empty: quantity(0.0),
                complete: true,
            },
            vec![0, 1, 2],
        ),
    ];
    let mut full = OwnedPlanScratch::default();
    let mut staged = OwnedPlanScratch::default();
    let mut full_work = 100;
    let mut staged_work = full_work;
    begin_graph(effects.len(), &mut full, &mut full_work).unwrap();
    execute_graph(graph(&effects), &mut full, &[0, 1, 2, 3], &mut full_work).unwrap();
    begin_graph(effects.len(), &mut staged, &mut staged_work).unwrap();
    execute_graph(graph(&effects), &mut staged, &[0, 1], &mut staged_work).unwrap();
    assert_eq!(staged.values[2..], [None, None]);
    execute_graph(graph(&effects), &mut staged, &[2], &mut staged_work).unwrap();
    execute_graph(graph(&effects), &mut staged, &[3], &mut staged_work).unwrap();
    assert_eq!(full.values[3], Some(known(quantity(1.0))));
    assert_eq!(staged.values, full.values);
    assert_eq!(staged_work, full_work);
}

#[test]
fn out_of_bounds_duplicate_and_missing_predecessor_schedules_clear_partial_state() {
    let effects = chain(17);
    for indices in [vec![3], vec![1], vec![1, 0], vec![0, 0], vec![0, 2]] {
        let mut scratch = OwnedPlanScratch::default();
        let mut work = 100;
        begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
        let before = work;
        scratch.facts.push(Some(integer(99)));
        assert!(matches!(
            execute_graph(graph(&effects), &mut scratch, &indices, &mut work),
            Err(PlanError::Invalid(_))
        ));
        assert!(work < before);
        assert!(scratch.values.is_empty());
        assert!(scratch.facts.is_empty());
    }
    // Completed nodes also cannot be repeated across separate stage calls.
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[0], &mut work).unwrap();
    assert!(execute_graph(graph(&effects), &mut scratch, &[0], &mut work).is_err());
    assert!(scratch.values.is_empty());
    assert!(execute_graph(graph(&effects), &mut scratch, &[1], &mut work).is_err());
}

#[test]
fn false_gates_do_not_waive_declared_schedule_dependencies() {
    let mut effects = chain(7);
    effects[1].gates = vec![literal(ParameterValue::Boolean(false))];
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    assert!(execute_graph(graph(&effects), &mut scratch, &[1], &mut work).is_err());
    assert!(scratch.values.is_empty());
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[0], &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[1, 2], &mut work).unwrap();
    assert_eq!(scratch.values[1], Some(EffectValue::Inactive));
    assert_eq!(scratch.values[2], Some(EffectValue::Inactive));
}

#[test]
fn staged_execution_preserves_false_lazy_and_partial_read_semantics() {
    let mut masked = node(1, ReadBinding::Missing(PlanGapReason::MissingInput), vec![]);
    masked.gates = vec![
        ReadBinding::Missing(PlanGapReason::MissingProducer),
        literal(ParameterValue::Boolean(false)),
    ];
    let effects = vec![
        node(0, literal(ParameterValue::Boolean(true)), vec![]),
        masked,
        node(
            2,
            ReadBinding::Select {
                decision: 0,
                when_true: Box::new(literal(integer(23))),
                when_false: Box::new(ReadBinding::Missing(PlanGapReason::MissingInput)),
            },
            vec![0],
        ),
        node(
            3,
            ReadBinding::Final {
                effect: Some(2),
                complete: false,
            },
            vec![2],
        ),
        node(
            4,
            ReadBinding::Reduction {
                effects: vec![2],
                reduction: ContributionReduction::Sum,
                empty: integer(0),
                complete: false,
            },
            vec![2],
        ),
    ];
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[0, 1], &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[2, 3, 4], &mut work).unwrap();
    assert_eq!(scratch.values[1], Some(EffectValue::Inactive));
    assert_eq!(scratch.values[2], Some(known(integer(23))));
    for index in [3, 4] {
        assert_eq!(
            scratch.values[index],
            Some(EffectValue::unresolved(
                PlanGapReason::IncompleteContributors
            ))
        );
    }
}

#[test]
fn exhausted_shared_budget_clears_stages_and_retry_starts_without_old_values() {
    let effects = chain(11);
    let mut scratch = OwnedPlanScratch::default();
    // Initialization and the first literal consume the entire allowance.
    let mut work = 5;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[0], &mut work).unwrap();
    assert_eq!(work, 0);
    scratch.facts.push(Some(integer(123)));
    assert!(matches!(
        execute_graph(graph(&effects), &mut scratch, &[1, 2], &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(work, 0);
    assert!(scratch.values.is_empty());
    assert!(scratch.facts.is_empty());
    work = 100;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    assert!(execute_graph(graph(&effects), &mut scratch, &[1], &mut work).is_err());
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(graph(&effects), &mut scratch, &[0, 1, 2], &mut work).unwrap();
    assert_eq!(scratch.values[2], Some(known(integer(11))));
}

#[test]
fn failed_begin_clears_previous_attempt_before_allocation() {
    let mut scratch = OwnedPlanScratch::default();
    scratch.values.push(Some(known(integer(17))));
    scratch.facts.push(Some(integer(19)));
    let capacity = scratch.values.capacity();
    let mut work = 1;
    assert!(matches!(
        begin_graph(capacity + 10, &mut scratch, &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert!(scratch.values.is_empty());
    assert!(scratch.facts.is_empty());
    assert_eq!(scratch.values.capacity(), capacity);
    assert_eq!(
        work, 0,
        "an insufficient reservation exhausts its allowance"
    );
}

#[test]
fn reused_scratch_a_b_a_matches_fresh_attempts_with_different_graph_sizes() {
    let a = chain(17);
    let b = vec![node(0, literal(integer(91)), vec![])];
    let mut reused = OwnedPlanScratch::default();
    for effects in [&a, &b, &a] {
        let mut fresh = OwnedPlanScratch::default();
        let mut fresh_work = 100;
        let mut reused_work = fresh_work;
        begin_graph(effects.len(), &mut fresh, &mut fresh_work).unwrap();
        begin_graph(effects.len(), &mut reused, &mut reused_work).unwrap();
        let indices: Vec<_> = (0..effects.len()).collect();
        execute_graph(graph(effects), &mut fresh, &indices, &mut fresh_work).unwrap();
        execute_graph(graph(effects), &mut reused, &indices[..1], &mut reused_work).unwrap();
        execute_graph(graph(effects), &mut reused, &indices[1..], &mut reused_work).unwrap();
        assert_eq!(reused.values, fresh.values);
        assert_eq!(reused_work, fresh_work);
    }
}

#[test]
fn private_attempt_seal_rejects_foreign_or_absent_plans_and_clears_every_stage() {
    let first = digest_owned("graph-attempt-test", &"first", 100).unwrap();
    let second = digest_owned("graph-attempt-test", &"second", 100).unwrap();
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    begin_graph(1, &mut scratch, &mut work).unwrap();
    assert!(require_attempt_plan(&mut scratch, first).is_err());
    begin_graph(1, &mut scratch, &mut work).unwrap();
    scratch.attempt_plan = Some(first);
    scratch.values[0] = Some(known(integer(17)));
    scratch.facts.push(Some(integer(19)));
    require_attempt_plan(&mut scratch, first).unwrap();
    assert!(require_attempt_plan(&mut scratch, second).is_err());
    assert!(scratch.values.is_empty());
    assert!(scratch.facts.is_empty());
    assert!(scratch.attempt_plan.is_none());

    scratch.attempt_plan = Some(first);
    begin_graph(1, &mut scratch, &mut work).unwrap();
    assert!(
        scratch.attempt_plan.is_none(),
        "begin invalidates the old seal"
    );
    scratch.attempt_plan = Some(first);
    assert!(begin_graph(1, &mut scratch, &mut 0).is_err());
    assert!(scratch.attempt_plan.is_none());
}

#[test]
fn attempt_budget_above_plan_limit_invalidates_previous_state_without_replenishing() {
    let identity = digest_owned("graph-attempt-test", &"bounded", 100).unwrap();
    let mut scratch = OwnedPlanScratch {
        attempt_plan: Some(identity),
        ..Default::default()
    };
    scratch.values.push(Some(known(integer(17))));
    scratch.facts.push(Some(integer(19)));
    let allowance = 101;
    assert!(check_attempt_budget(&mut scratch, allowance, 100).is_err());
    assert!(scratch.attempt_plan.is_none());
    assert!(scratch.values.is_empty());
    assert!(scratch.facts.is_empty());
    check_attempt_budget(&mut scratch, 100, 100).unwrap();
    check_attempt_budget(&mut scratch, 0, 100).unwrap();
}
