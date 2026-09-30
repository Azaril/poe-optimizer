//! Metric projection consumes the sealed support-aware execution, never reports.
#[allow(dead_code)]
#[path = "support/owned_support_metric_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_metrics::*;
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use rayon::prelude::*;
use std::sync::Arc;
use support::*;

#[test]
fn supported_final_metrics_preserve_query_order_exact_occurrences_and_units() {
    let f = fixture();
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(report.support, SupportMetricStatus::Evaluated));
    assert!(report.evaluation.gaps.is_empty());
    assert_eq!(report.evaluation.identity, plan.identity());
    assert_eq!(
        report
            .evaluation
            .results
            .iter()
            .map(|r| r.request.clone())
            .collect::<Vec<_>>(),
        f.queries.requests
    );
    assert_eq!(
        report
            .evaluation
            .results
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        [32.0, 16.0, 16.0, 25.0, 32.0, 25.0]
    );
    assert!(
        report
            .evaluation
            .results
            .iter()
            .all(|r| r.stat == Some(def("support-quantity")))
    );
}

#[test]
fn metric_can_bind_a_final_producer_created_only_by_retained_delivery() {
    let f = fixture();
    let plan = compile_with(
        &f,
        |rules| {
            let program = rules
                .owners
                .iter_mut()
                .find(|o| o.owner == delivery::support_owner())
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("action-deliver"))
                .unwrap();
            program.nodes.extend([
                fixture::node(
                    "one-unit",
                    RuleExpression::Literal {
                        value: quantity(1.0),
                    },
                ),
                fixture::node(
                    "quantity",
                    RuleExpression::ScaleInteger {
                        value: key("one-unit"),
                        count: key("value"),
                    },
                ),
            ]);
            program.effects.push(fixture::derive(
                "dynamic",
                RuleEntity::Current,
                "dynamic-quantity",
                "quantity",
            ));
        },
        |_| {},
        |_| {},
        |mapping| {
            mapping
                .bindings
                .iter_mut()
                .find(|row| row.role == MetricBindingRole::Action)
                .unwrap()
                .stat = def("dynamic-quantity");
        },
    );
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(report.support, SupportMetricStatus::Evaluated));
    for index in [1, 2, 5] {
        assert_eq!(
            report.evaluation.results[index].stat,
            Some(def("dynamic-quantity"))
        );
        assert_eq!(
            number(&report.evaluation.results[index]),
            if index == 5 { 25.0 } else { 16.0 }
        );
    }
    let missing = compile_with(
        &f,
        |_| {},
        |_| {},
        |_| {},
        |mapping| {
            mapping
                .bindings
                .iter_mut()
                .find(|row| row.role == MetricBindingRole::Action)
                .unwrap()
                .stat = def("dynamic-quantity");
        },
    );
    assert_ne!(plan.identity(), missing.identity());
    let report = missing.evaluate(&mut missing.new_scratch()).unwrap();
    assert!(matches!(
        report.evaluation.results[1].value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
}

#[test]
fn target_activation_precedes_missing_mapping_without_aliasing_sibling_queries() {
    let mut f = fixture();
    // Keep exact generated query topology declared while its activation grant
    // evaluates false. Removing the authored parent instead leaves unresolved
    // output-owner coverage for its explicitly requested generated actions.
    f.build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let plan = compile_with(
        &f,
        |_| {},
        |_| {},
        |_| {},
        |mapping| mapping.bindings.clear(),
    );
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        matches!(report.support, SupportMetricStatus::Evaluated),
        "{report:?}"
    );
    for index in [0, 1, 2, 4] {
        assert_eq!(
            report.evaluation.results[index].value,
            EffectValue::Inactive
        );
        assert_eq!(report.evaluation.results[index].stat, None);
    }
    for index in [3, 5] {
        assert!(matches!(
            report.evaluation.results[index].value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingMetricBinding,
                ..
            }
        ));
    }
}

#[test]
fn unresolved_order_and_incomplete_inventory_preserve_every_query_and_mapping() {
    let mut f = fixture();
    f.build.support_origins = None;
    let plan = compile(&f);
    let mut scratch = plan.new_scratch();
    let allowance = PlanLimits::default().max_work;
    let mut work = allowance;
    let report = plan.evaluate_with_budget(&mut scratch, &mut work).unwrap();
    assert!(matches!(
        report.support,
        SupportMetricStatus::PreparationUnresolved {
            reason: SupportPreparationGap::OriginOrder,
            ..
        }
    ));
    assert_eq!(report.evaluation.results.len(), f.queries.requests.len());
    for (row, request) in report.evaluation.results.iter().zip(&f.queries.requests) {
        assert_eq!(&row.request, request);
        assert_eq!(row.stat, Some(def("support-quantity")));
        assert_eq!(
            row.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::UpstreamUnavailable,
                read: None
            }
        );
    }
    // Preparation itself fits. The final common-failure row projection still
    // consumes this attempt's allowance before allocating its result rows.
    let mut one_short = allowance - work - 1;
    assert!(matches!(
        plan.evaluate_with_budget(&mut scratch, &mut one_short),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(one_short, 0);
    let mut retry_work = allowance;
    assert_eq!(
        report,
        plan.evaluate_with_budget(&mut scratch, &mut retry_work)
            .unwrap()
    );
    assert_eq!(work, retry_work);
    let complete = fixture();
    let partial = compile_with(
        &complete,
        |_| {},
        |_| {},
        |receiving| receiving.supports.clear(),
        |mapping| {
            mapping
                .bindings
                .retain(|row| row.role == MetricBindingRole::OwnedActor)
        },
    );
    let report = partial.evaluate(&mut partial.new_scratch()).unwrap();
    assert!(matches!(
        report.support,
        SupportMetricStatus::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert_eq!(
        report.evaluation.results.len(),
        complete.queries.requests.len()
    );
    assert!(report.evaluation.results.iter().all(|r| matches!(
        r.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    )));
    for row in &report.evaluation.results {
        assert_eq!(
            row.stat,
            match row.request.target {
                MetricTarget::Actor(_) => Some(def("support-quantity")),
                MetricTarget::Action(_) => None,
            }
        );
    }
}

#[test]
fn preparation_failure_keeps_exact_scalar_and_domain_provenance_for_all_rows() {
    let mut f = fixture();
    let one = BoundedInteger::new(1).unwrap();
    f.tables.push(IntegerRuleTable {
        id: key("effective-level-domain"),
        minimum: one,
        maximum: one,
        value_type: ComputedValueType::Integer,
        rows: vec![integer(1)],
    });
    f.owner_mut(&delivery::support_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("origin-facts"))
        .unwrap()
        .nodes
        .iter_mut()
        .find(|n| n.id == key("effective"))
        .unwrap()
        .expression = RuleExpression::LookupIntegerTable {
        table: key("effective-level-domain"),
        key: key("level"),
    };
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let expected = EffectValue::UnsupportedDomain {
        node: key("effective"),
        table: key("effective-level-domain"),
        key: BoundedInteger::new(2).unwrap(),
        minimum: one,
        maximum: one,
    };
    assert_eq!(
        report.support,
        SupportMetricStatus::Unavailable {
            cause: expected.clone(),
            input: Some(Box::new(PlanValueKey::Stat {
                entity: ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(occurrence(61))),
                stat: def("effective-level")
            })),
        }
    );
    assert!(
        report
            .evaluation
            .results
            .iter()
            .all(|r| r.value == expected)
    );
}

#[test]
fn mapping_requires_exact_units_and_exact_definition_identity() {
    let mut f = fixture();
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("wrong-unit"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity {
                    unit: def("quality"),
                },
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    let args = delivery::inputs(&f, |_| {});
    let mut mapping = mapping_input(args.definitions.as_ref());
    mapping.bindings[0].stat = def("wrong-unit");
    assert!(
        OwnedMetricMapping::new(
            mapping,
            args.definitions.as_ref(),
            MetricMappingLimits::default()
        )
        .is_err()
    );
    let mapping = Arc::new(
        OwnedMetricMapping::new(
            mapping_input(args.definitions.as_ref()),
            args.definitions.as_ref(),
            MetricMappingLimits::default(),
        )
        .unwrap(),
    );
    f.schema.release = key("another-schema");
    let effects = Arc::new(
        OwnedSupportEffectPlan::compile(
            delivery::inputs(&f, |_| {}),
            PlanLimits::default(),
            SupportPreparationLimits::default(),
        )
        .unwrap(),
    );
    assert!(OwnedSupportMetricPlan::compile(effects, mapping).is_err());
}

#[test]
fn support_metric_budget_failure_and_a_b_a_reuse_match_fresh_execution() {
    let a = compile(&fixture());
    let mut changed = fixture();
    changed
        .build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(29))
        .unwrap()
        .level = 30;
    let b = compile(&changed);
    let mut scratch = a.new_scratch();
    let allowance = PlanLimits::default().max_work;
    let mut measured_work = allowance;
    let initial = a
        .evaluate_with_budget(&mut scratch, &mut measured_work)
        .unwrap();
    let middle = b.evaluate(&mut scratch).unwrap();
    assert_ne!(initial.evaluation.identity, middle.evaluation.identity);
    assert_eq!(number(&middle.evaluation.results[3]), 35.0);
    // Every graph and preparation step fits; the last metric collection read
    // must still charge the same budget rather than start a fresh allowance.
    let mut exhausted = allowance - measured_work - 1;
    assert!(matches!(
        a.evaluate_with_budget(&mut scratch, &mut exhausted),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(exhausted, 0);
    let mut work = PlanLimits::default().max_work;
    let again = a.evaluate_with_budget(&mut scratch, &mut work).unwrap();
    assert_eq!(initial, again);
    let mut fresh_work = PlanLimits::default().max_work;
    assert_eq!(
        again,
        a.evaluate_with_budget(&mut a.new_scratch(), &mut fresh_work)
            .unwrap()
    );
    assert_eq!(work, fresh_work);
}

#[test]
fn rayon_workers_share_one_metric_plan_with_independent_scratch_and_work() {
    let plan = Arc::new(compile(&fixture()));
    let mut scratch = plan.new_scratch();
    let expected: Vec<_> = (0..24)
        .map(|_| {
            let mut work = PlanLimits::default().max_work;
            (
                plan.evaluate_with_budget(&mut scratch, &mut work).unwrap(),
                work,
            )
        })
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let actual: Vec<_> = pool.install(|| {
        (0..24)
            .into_par_iter()
            .map_init(
                || plan.new_scratch(),
                |scratch, _| {
                    let mut work = PlanLimits::default().max_work;
                    (plan.evaluate_with_budget(scratch, &mut work).unwrap(), work)
                },
            )
            .collect()
    });
    assert_eq!(actual, expected);
}
