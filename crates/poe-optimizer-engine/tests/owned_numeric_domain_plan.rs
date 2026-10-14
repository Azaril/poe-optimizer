//! Numeric-domain availability through the current readiness/support/metric graph.
#[allow(unused_imports)]
#[path = "support/owned_skill_participation_fixture.rs"]
mod support;
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use support::key;

fn range(maximum: i64) -> NumericDomain {
    NumericDomain::Integer(IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(maximum).unwrap(),
    })
}
fn inputs(f: &support::Fixture, maximum: i64, early: bool) -> support::Checked<support::Inputs> {
    support::inputs_with(
        f,
        |r| {
            r.operations_version = key(OWNED_RULE_OPERATIONS_V27);
            let p = support::readiness::package_program_mut(
                r,
                &support::delivery::support_owner(),
                "origin-facts",
            );
            let n = p
                .nodes
                .iter_mut()
                .find(|n| n.id == key("effective"))
                .unwrap();
            n.expression = RuleExpression::CheckNumericDomain {
                value: key("level"),
                domain: range(maximum),
            };
            if early {
                let p = support::readiness::package_program_mut(
                    r,
                    &support::readiness::child_owner(),
                    "target-facts",
                );
                p.reads.push(support::base::read(
                    "late-domain-input",
                    RuleReadSource::Parameter {
                        slot: support::readiness::ability_parameter("quality"),
                    },
                ));
                p.nodes.push(support::base::read_node(
                    "late-domain-input",
                    "late-domain-input",
                ));
                p.nodes
                    .iter_mut()
                    .find(|n| n.id == key("base"))
                    .unwrap()
                    .expression = RuleExpression::CheckNumericDomain {
                    value: key("late-domain-input"),
                    domain: range(100),
                };
            }
        },
        |_| {},
        |_| {},
    )
}
fn plan(maximum: i64) -> support::readiness::Metrics {
    support::readiness::metrics(
        support::readiness::compile_inputs(inputs(&support::fixture(), maximum, false).unwrap())
            .unwrap(),
    )
}

#[test]
fn demanded_range_failure_survives_preparation_into_every_requested_metric() {
    let plan = plan(1);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let failure = EffectValue::UnsupportedDomain {
        node: key("effective"),
        domain: UnsupportedDomainCause::NumericRange {
            value: support::base::integer(2),
            domain: Box::new(range(1)),
        },
    };
    assert!(
        matches!(&report.support, SupportMetricStatus::Unavailable { cause, input: Some(_) } if cause == &failure)
    );
    assert_eq!(report.evaluation.results.len(), 3);
    assert!(report.evaluation.results.iter().all(|r| r.value == failure));
    let wire = serde_json::to_value(&failure).unwrap();
    assert_eq!(wire["status"], "unsupported_domain");
    assert_eq!(wire["domain"]["kind"], "numeric_range");
    assert_eq!(wire["domain"]["domain"]["kind"], "integer");
}

#[test]
fn range_changes_and_failure_recovery_match_fresh_reused_and_rayon_workers() {
    let bad = plan(1);
    let good = plan(10);
    let a = bad.evaluate(&mut bad.new_scratch()).unwrap();
    let b = good.evaluate(&mut good.new_scratch()).unwrap();
    assert!(matches!(b.support, SupportMetricStatus::Evaluated));
    assert!(
        b.evaluation
            .results
            .iter()
            .all(|r| matches!(r.value, EffectValue::Known { .. }))
    );
    assert_ne!(a.evaluation.identity, b.evaluation.identity);
    let mut scratch = bad.new_scratch();
    for p in [&bad, &good, &bad, &good] {
        assert_eq!(
            p.evaluate(&mut scratch).unwrap(),
            p.evaluate(&mut p.new_scratch()).unwrap()
        );
    }
    let rows = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..16)
                .into_par_iter()
                .map_init(
                    || bad.new_scratch(),
                    |s, n| {
                        if n % 2 == 0 {
                            bad.evaluate(s).unwrap()
                        } else {
                            good.evaluate(s).unwrap()
                        }
                    },
                )
                .collect::<Vec<_>>()
        });
    for (n, row) in rows.into_iter().enumerate() {
        assert_eq!(row, if n % 2 == 0 { a.clone() } else { b.clone() });
    }
}

#[test]
fn numeric_domain_does_not_erase_preparation_stage_dependencies() {
    let error = inputs(&support::fixture(), 10, true)
        .and_then(support::readiness::compile_inputs)
        .err()
        .expect("early assembly must fail");
    assert!(
        error.contains("early readiness depends on a later input or producer"),
        "{error}"
    );
}
