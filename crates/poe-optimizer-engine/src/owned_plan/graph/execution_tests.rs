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
fn graph(effects: &[EffectNode]) -> SliceGraph<'_> {
    SliceGraph {
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
    execute_graph(&graph(&effects), &mut full, &[0, 1, 2, 3], &mut full_work).unwrap();
    begin_graph(effects.len(), &mut staged, &mut staged_work).unwrap();
    execute_graph(&graph(&effects), &mut staged, &[0, 1], &mut staged_work).unwrap();
    assert_eq!(staged.values[2..], [None, None]);
    execute_graph(&graph(&effects), &mut staged, &[2], &mut staged_work).unwrap();
    execute_graph(&graph(&effects), &mut staged, &[3], &mut staged_work).unwrap();
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
            execute_graph(&graph(&effects), &mut scratch, &indices, &mut work),
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
    execute_graph(&graph(&effects), &mut scratch, &[0], &mut work).unwrap();
    assert!(execute_graph(&graph(&effects), &mut scratch, &[0], &mut work).is_err());
    assert!(scratch.values.is_empty());
    assert!(execute_graph(&graph(&effects), &mut scratch, &[1], &mut work).is_err());
}

#[test]
fn false_gates_do_not_waive_declared_schedule_dependencies() {
    let mut effects = chain(7);
    effects[1].gates = vec![literal(ParameterValue::Boolean(false))];
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    assert!(execute_graph(&graph(&effects), &mut scratch, &[1], &mut work).is_err());
    assert!(scratch.values.is_empty());
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(&graph(&effects), &mut scratch, &[0], &mut work).unwrap();
    execute_graph(&graph(&effects), &mut scratch, &[1, 2], &mut work).unwrap();
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
    execute_graph(&graph(&effects), &mut scratch, &[0, 1], &mut work).unwrap();
    execute_graph(&graph(&effects), &mut scratch, &[2, 3, 4], &mut work).unwrap();
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
    execute_graph(&graph(&effects), &mut scratch, &[0], &mut work).unwrap();
    assert_eq!(work, 0);
    scratch.facts.push(Some(integer(123)));
    assert!(matches!(
        execute_graph(&graph(&effects), &mut scratch, &[1, 2], &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(work, 0);
    assert!(scratch.values.is_empty());
    assert!(scratch.facts.is_empty());
    work = 100;
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    assert!(execute_graph(&graph(&effects), &mut scratch, &[1], &mut work).is_err());
    begin_graph(effects.len(), &mut scratch, &mut work).unwrap();
    execute_graph(&graph(&effects), &mut scratch, &[0, 1, 2], &mut work).unwrap();
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
        execute_graph(&graph(effects), &mut fresh, &indices, &mut fresh_work).unwrap();
        execute_graph(
            &graph(effects),
            &mut reused,
            &indices[..1],
            &mut reused_work,
        )
        .unwrap();
        execute_graph(
            &graph(effects),
            &mut reused,
            &indices[1..],
            &mut reused_work,
        )
        .unwrap();
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

/// Test-only storage owner: immutable base slices plus sparse suffix changes and
/// appended rows. The production driver supplies its own sealed implementation.
struct Overlay<'a> {
    base: SliceGraph<'a>,
    effects: &'a [EffectNode],
    invocations: &'a [Invocation],
    changed_effects: BTreeMap<usize, EffectNode>,
    changed_invocations: BTreeMap<usize, Invocation>,
}
impl ExecutionGraphView for Overlay<'_> {
    fn effect_count(&self) -> usize {
        self.base.effects.len() + self.effects.len()
    }
    fn invocation_count(&self) -> usize {
        self.base.invocations.len() + self.invocations.len()
    }
    fn effect(&self, index: usize) -> Option<&EffectNode> {
        if index < self.base.effects.len() {
            self.changed_effects
                .get(&index)
                .or_else(|| self.base.effects.get(index))
        } else {
            self.effects.get(index - self.base.effects.len())
        }
    }
    fn invocation(&self, index: usize) -> Option<&Invocation> {
        if index < self.base.invocations.len() {
            self.changed_invocations
                .get(&index)
                .or_else(|| self.base.invocations.get(index))
        } else {
            self.invocations.get(index - self.base.invocations.len())
        }
    }
}
fn attempt_identity(name: &str) -> OwnedContentDigest {
    digest_owned("graph-view-attempt", &name, 100).unwrap()
}
fn start_view(
    identity: OwnedContentDigest,
    count: usize,
    scratch: &mut OwnedPlanScratch,
    work: &mut usize,
) {
    begin_graph(count, scratch, work).unwrap();
    scratch.attempt_plan = Some(identity);
}
fn sum(effects: Vec<usize>) -> ReadBinding {
    ReadBinding::Reduction {
        effects,
        reduction: ContributionReduction::Sum,
        empty: integer(0),
        complete: true,
    }
}
fn contribution(index: usize, value: i64) -> EffectNode {
    let mut effect = node(index, literal(integer(value)), vec![]);
    effect.target = BoundEffectTarget::Contribution {
        key: ContributionKey {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: StatDefId::parse(namespace(), "total").unwrap(),
            kind: ContributionKind::Add,
        },
    };
    effect
}

#[test]
fn borrowed_append_and_suffix_reduction_execute_without_replaying_prefix() {
    let identity = attempt_identity("append");
    let limits = PlanLimits::default();
    let base = vec![contribution(0, 2), node(1, sum(vec![0]), vec![0])];
    let appended = [contribution(2, 7)];
    let view = Overlay {
        base: graph(&base),
        effects: &appended,
        invocations: &[],
        changed_effects: BTreeMap::from([(1, node(1, sum(vec![0, 2]), vec![0, 2]))]),
        changed_invocations: BTreeMap::new(),
    };
    assert!(std::ptr::eq(view.effect(0).unwrap(), &base[0]));
    let mut staged = OwnedPlanScratch::default();
    let mut staged_work = 100;
    start_view(identity, base.len(), &mut staged, &mut staged_work);
    execute_view(
        identity,
        &graph(&base),
        &mut staged,
        &[0],
        &mut staged_work,
        limits,
    )
    .unwrap();
    let prefix = staged.values[0].clone();
    extend_attempt(
        identity,
        base.len(),
        view.effect_count(),
        &mut staged,
        &mut staged_work,
        limits,
    )
    .unwrap();
    assert_eq!(staged.values, vec![prefix.clone(), None, None]);
    execute_view(
        identity,
        &view,
        &mut staged,
        &[2, 1],
        &mut staged_work,
        limits,
    )
    .unwrap();
    assert_eq!(staged.values[0], prefix);
    assert_eq!(staged.values[1], Some(known(integer(9))));
    // A one-shot materialized control computes exactly the same values and work;
    // materialization exists only in this test, never in the execution kernel.
    let flat = vec![
        base[0].clone(),
        view.effect(1).unwrap().clone(),
        appended[0].clone(),
    ];
    let mut full = OwnedPlanScratch::default();
    let mut full_work = 100;
    start_view(identity, flat.len(), &mut full, &mut full_work);
    execute_view(
        identity,
        &graph(&flat),
        &mut full,
        &[0, 2, 1],
        &mut full_work,
        limits,
    )
    .unwrap();
    assert_eq!(staged.values, full.values);
    assert_eq!(staged_work, full_work);
}

fn compiled_invocation() -> Invocation {
    typed_invocation(ComputedValueType::Integer)
}

fn typed_invocation(value_type: ComputedValueType) -> Invocation {
    use crate::owned_rules::RuleLimits;
    use poe_optimizer_data::owned_schema::{
        OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
    };
    let encounter = EncounterDefId::parse(namespace(), "encounter").unwrap();
    let stat = StatDefId::parse(namespace(), "value").unwrap();
    let owner = SchemaSubject::Definition(encounter.address());
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("schema"),
            semantics_version: key("v1"),
            definitions: vec![
                DefinitionDescriptor::Encounter(DefinitionEntry {
                    id: encounter,
                    schema: SchemaState::Known(EncounterSchema {
                        enemy_level: IntegerRange {
                            minimum: BoundedInteger::new(1).unwrap(),
                            maximum: BoundedInteger::new(100).unwrap(),
                        },
                        external_inputs: DeclaredSet::complete(vec![]),
                    }),
                }),
                DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: value_type.clone(),
                        targets: vec![RuleEntityKind::Enemy],
                    }),
                }),
            ],
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    let package = CompiledRulePackage::compile(
        &RulePackageInput {
            existing_actor_rules: None,
            ordered_contributions: None,
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("rules"),
            semantics_version: key("v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![],
            receivers: DeclaredSet::complete(vec![]),
            owners: vec![DefinitionRules {
                owner: owner.clone(),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key("copy"),
                    context: RuleEntityKind::Enemy,
                    reads: vec![RuleRead {
                        id: key("input"),
                        value_type: value_type.clone(),
                        source: RuleReadSource::Stat {
                            entity: RuleEntity::Current,
                            stat: stat.clone(),
                        },
                    }],
                    nodes: vec![RuleNode {
                        id: key("read"),
                        expression: RuleExpression::Read {
                            input: key("input"),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("copy-value"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat,
                            value: key("read"),
                        },
                    }],
                }]),
            }],
        },
        &schema,
        RuleLimits::default(),
    )
    .unwrap();
    Invocation {
        key: ProgramOccurrenceKey {
            origin: RuleOrigin::Encounter,
            owner: owner.clone(),
            program: key("copy"),
            entity: ConcreteEntity::Enemy,
        },
        program: package.prepare_program(&owner, &key("copy")).unwrap(),
        reads: vec![literal(integer(-1))],
        read_ids: vec![key("input")],
    }
}

#[test]
fn sparse_invocation_gates_and_dependency_overrides_use_the_same_program_interpreter() {
    let identity = attempt_identity("invocations");
    let limits = PlanLimits::default();
    let base_invocations = [compiled_invocation()];
    let mut consumer = node(1, literal(integer(-1)), vec![]);
    consumer.operation = EffectOperation::Program {
        invocation: 0,
        effect: 0,
    };
    consumer.gates = vec![literal(ParameterValue::Boolean(false))];
    let base = vec![contribution(0, 17), consumer];
    let mut producer = contribution(2, 0);
    producer.operation = EffectOperation::Program {
        invocation: 1,
        effect: 0,
    };
    let appended = [producer];
    let mut added_invocation = base_invocations[0].clone();
    added_invocation.reads = vec![literal(integer(41))];
    let appended_invocations = [added_invocation];
    let mut changed_invocation = base_invocations[0].clone();
    changed_invocation.reads = vec![sum(vec![0, 2])];
    let mut changed_effect = base[1].clone();
    changed_effect.gates.clear();
    changed_effect.dependencies = vec![0, 2];
    let view = Overlay {
        base: SliceGraph {
            effects: &base,
            invocations: &base_invocations,
        },
        effects: &appended,
        invocations: &appended_invocations,
        changed_effects: BTreeMap::from([(1, changed_effect)]),
        changed_invocations: BTreeMap::from([(0, changed_invocation)]),
    };
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 1000;
    start_view(identity, base.len(), &mut scratch, &mut work);
    execute_view(identity, &view.base, &mut scratch, &[0], &mut work, limits).unwrap();
    extend_attempt(
        identity,
        base.len(),
        view.effect_count(),
        &mut scratch,
        &mut work,
        limits,
    )
    .unwrap();
    execute_view(identity, &view, &mut scratch, &[2, 1], &mut work, limits).unwrap();
    assert_eq!(scratch.values[1], Some(known(integer(58))));
    assert_eq!(scratch.values[2], Some(known(integer(41))));
    assert!(
        matches!(base_invocations[0].reads[0], ReadBinding::Constant(Some(ParameterValue::Integer(v))) if v.get() == -1)
    );
    assert_eq!(base[1].gates.len(), 1);
}

#[test]
fn extended_views_reject_duplicate_missing_dependency_and_foreign_attempts() {
    let identity = attempt_identity("extended");
    let limits = PlanLimits::default();
    let base = chain(17);
    let appended = [node(3, final_read(2), vec![2])];
    let view = Overlay {
        base: graph(&base),
        effects: &appended,
        invocations: &[],
        changed_effects: BTreeMap::new(),
        changed_invocations: BTreeMap::new(),
    };
    for indices in [vec![0], vec![3], vec![4], vec![1, 1]] {
        let mut scratch = OwnedPlanScratch::default();
        let mut work = 100;
        start_view(identity, base.len(), &mut scratch, &mut work);
        execute_view(identity, &view.base, &mut scratch, &[0], &mut work, limits).unwrap();
        extend_attempt(
            identity,
            base.len(),
            view.effect_count(),
            &mut scratch,
            &mut work,
            limits,
        )
        .unwrap();
        scratch.facts.push(Some(integer(99)));
        assert!(execute_view(identity, &view, &mut scratch, &indices, &mut work, limits).is_err());
        assert!(
            scratch.values.is_empty() && scratch.facts.is_empty() && scratch.attempt_plan.is_none()
        );
    }
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    start_view(identity, base.len(), &mut scratch, &mut work);
    assert!(
        execute_view(
            attempt_identity("other"),
            &view.base,
            &mut scratch,
            &[],
            &mut work,
            limits
        )
        .is_err()
    );
    assert!(scratch.values.is_empty() && scratch.attempt_plan.is_none());
    assert!(extend_attempt(identity, 0, 1, &mut scratch, &mut work, limits).is_err());
}

#[test]
fn extension_limits_and_work_failure_clear_without_allocation_or_replenishment() {
    let identity = attempt_identity("limits");
    let limits = PlanLimits::default();
    for (old, new, custom) in [
        (1, 3, limits),
        (2, 1, limits),
        (
            2,
            3,
            PlanLimits {
                max_effects: 2,
                ..limits
            },
        ),
        (
            2,
            3,
            PlanLimits {
                max_effects: 0,
                ..limits
            },
        ),
    ] {
        let mut scratch = OwnedPlanScratch::default();
        let mut work = 100;
        start_view(identity, 2, &mut scratch, &mut work);
        scratch.values[0] = Some(known(integer(9)));
        scratch.facts.push(Some(integer(11)));
        let capacity = scratch.values.capacity();
        let before = work;
        assert!(extend_attempt(identity, old, new, &mut scratch, &mut work, custom).is_err());
        assert!(
            scratch.values.is_empty() && scratch.facts.is_empty() && scratch.attempt_plan.is_none()
        );
        assert_eq!(scratch.values.capacity(), capacity);
        assert_eq!(work, before);
    }
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 100;
    start_view(identity, 2, &mut scratch, &mut work);
    let capacity = scratch.values.capacity();
    work = 1;
    assert!(matches!(
        extend_attempt(identity, 2, capacity + 10, &mut scratch, &mut work, limits),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(work, 0);
    assert_eq!(scratch.values.capacity(), capacity);
    assert!(scratch.values.is_empty() && scratch.attempt_plan.is_none());
    work = limits.max_work + 1;
    start_view(identity, 2, &mut scratch, &mut 10);
    assert!(extend_attempt(identity, 2, 3, &mut scratch, &mut work, limits).is_err());
    assert_eq!(work, limits.max_work + 1);
    assert!(scratch.values.is_empty() && scratch.attempt_plan.is_none());
}

#[test]
fn borrowed_views_recheck_aggregate_counts_and_attempt_size_even_for_empty_schedules() {
    let identity = attempt_identity("counts");
    let base = chain(17);
    let invocation = [compiled_invocation(), compiled_invocation()];
    for (count, custom) in [
        (2, PlanLimits::default()),
        (
            3,
            PlanLimits {
                max_effects: 2,
                ..Default::default()
            },
        ),
        (
            3,
            PlanLimits {
                max_invocations: 1,
                ..Default::default()
            },
        ),
    ] {
        let view = SliceGraph {
            effects: &base,
            invocations: &invocation,
        };
        let mut scratch = OwnedPlanScratch::default();
        let mut work = 100;
        start_view(identity, count, &mut scratch, &mut work);
        scratch.facts.push(Some(integer(99)));
        assert!(execute_view(identity, &view, &mut scratch, &[], &mut work, custom).is_err());
        assert!(
            scratch.values.is_empty() && scratch.facts.is_empty() && scratch.attempt_plan.is_none()
        );
    }
}

#[test]
fn reused_dynamic_attempts_a_b_a_match_fresh_values_and_budgets() {
    let a = vec![contribution(0, 11), node(1, sum(vec![0]), vec![0])];
    let b = vec![contribution(0, 23), node(1, sum(vec![0]), vec![0])];
    let appended_a = vec![contribution(2, 7)];
    let appended_b = vec![contribution(2, 3), contribution(3, 5)];
    let mut reused = OwnedPlanScratch::default();
    for (name, base, appended) in [
        ("a", &a, &appended_a),
        ("b", &b, &appended_b),
        ("a", &a, &appended_a),
    ] {
        let identity = attempt_identity(name);
        let contributors: Vec<_> = std::iter::once(0)
            .chain(2..base.len() + appended.len())
            .collect();
        let schedule: Vec<_> = (2..base.len() + appended.len())
            .chain(std::iter::once(1))
            .collect();
        let view = Overlay {
            base: graph(base),
            effects: appended,
            invocations: &[],
            changed_effects: BTreeMap::from([(
                1,
                node(1, sum(contributors.clone()), contributors),
            )]),
            changed_invocations: BTreeMap::new(),
        };
        let evaluate = |scratch: &mut OwnedPlanScratch| {
            let mut work = 100;
            start_view(identity, base.len(), scratch, &mut work);
            execute_view(
                identity,
                &view.base,
                scratch,
                &[0],
                &mut work,
                PlanLimits::default(),
            )
            .unwrap();
            extend_attempt(
                identity,
                base.len(),
                view.effect_count(),
                scratch,
                &mut work,
                PlanLimits::default(),
            )
            .unwrap();
            execute_view(
                identity,
                &view,
                scratch,
                &schedule,
                &mut work,
                PlanLimits::default(),
            )
            .unwrap();
            work
        };
        let mut fresh = OwnedPlanScratch::default();
        let fresh_work = evaluate(&mut fresh);
        let reused_work = evaluate(&mut reused);
        assert_eq!(reused.values, fresh.values);
        assert_eq!(reused_work, fresh_work);
        if name == "b" {
            // The next A must also survive a failed dynamic growth on B.
            reused.facts.push(Some(integer(99)));
            let mut exhausted = 0;
            assert!(matches!(
                extend_attempt(
                    identity,
                    view.effect_count(),
                    view.effect_count() + 1,
                    &mut reused,
                    &mut exhausted,
                    PlanLimits::default()
                ),
                Err(PlanError::Limit("work"))
            ));
            assert!(reused.values.is_empty() && reused.facts.is_empty());
            assert!(reused.attempt_plan.is_none());
        }
    }
}

#[test]
fn native_applicability_false_is_explicit_and_true_requires_boolean_program_results() {
    let identity = attempt_identity("applicability");
    let limits = PlanLimits::default();
    for (eligible, input, expected) in [
        (
            false,
            ReadBinding::Missing(PlanGapReason::MissingInput),
            known(ParameterValue::Boolean(false)),
        ),
        (
            true,
            literal(ParameterValue::Boolean(true)),
            known(ParameterValue::Boolean(true)),
        ),
        (
            true,
            literal(ParameterValue::Boolean(false)),
            known(ParameterValue::Boolean(false)),
        ),
        (
            true,
            ReadBinding::Missing(PlanGapReason::MissingInput),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                read: Some(key("input")),
            },
        ),
    ] {
        let mut invocation = typed_invocation(ComputedValueType::Boolean);
        invocation.reads = vec![input];
        let invocations = [invocation];
        let mut applicability = node(0, literal(integer(0)), vec![]);
        applicability.operation = EffectOperation::SupportApplicability {
            invocation: 0,
            effect: 0,
            eligible,
        };
        let mut delivery = node(1, literal(integer(37)), vec![0]);
        delivery.gates = vec![final_read(0)];
        let effects = [applicability, delivery];
        let view = SliceGraph {
            effects: &effects,
            invocations: &invocations,
        };
        let mut scratch = OwnedPlanScratch::default();
        let mut work = 1000;
        start_view(identity, 2, &mut scratch, &mut work);
        execute_view(identity, &view, &mut scratch, &[0, 1], &mut work, limits).unwrap();
        assert_eq!(scratch.values[0], Some(expected.clone()));
        if expected == known(ParameterValue::Boolean(false)) {
            assert_eq!(scratch.values[1], Some(EffectValue::Inactive));
        }
    }
    let invocations = [compiled_invocation()];
    let mut applicability = node(0, literal(integer(0)), vec![]);
    applicability.operation = EffectOperation::SupportApplicability {
        invocation: 0,
        effect: 0,
        eligible: true,
    };
    let effects = [applicability];
    let view = SliceGraph {
        effects: &effects,
        invocations: &invocations,
    };
    let mut scratch = OwnedPlanScratch::default();
    let mut work = 1000;
    start_view(identity, 1, &mut scratch, &mut work);
    assert!(
        matches!(execute_view(identity, &view, &mut scratch, &[0], &mut work, limits), Err(PlanError::Invalid(message)) if message.contains("must be boolean"))
    );
    assert!(scratch.values.is_empty() && scratch.attempt_plan.is_none());
}
