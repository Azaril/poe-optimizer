//! Cold-plan binding evidence remains exact and does not change numerical authority.
#[allow(dead_code)]
#[path = "support/owned_metric_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_binding::*, owned_build::*, owned_content::digest_owned, owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use std::sync::Arc;
use support::*;

fn independently_bound(
    plan: &OwnedMetricPlan<poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage>,
    fixture: &Fixture,
) -> DefinitionBindingReport {
    bind_owned_request(
        plan.effect_plan().definitions(),
        &fixture.request(),
        BindingLimits::default(),
    )
    .unwrap()
}
fn query_site(id: &str, facet: BindingFacet) -> BindingSite {
    BindingSite {
        location: BindingLocation::Query(QueryId::new(id).unwrap()),
        facet,
    }
}
fn partial_modifier_inputs(fixture: &mut Fixture) {
    for row in &mut fixture.schema.definitions {
        if let DefinitionDescriptor::Modifier(row) = row
            && row.id == def("modifier")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.declarations.parameters.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: modifier_owner(),
                    facet: SchemaFacet::InputSchema,
                    code: key("remaining-modifier-inputs"),
                }],
            };
        }
    }
}

#[test]
fn exact_report_is_shared_read_only_and_preserves_plan_identity_and_evaluation() {
    let fixture = fixture();
    let plan = Arc::new(compile(&fixture));
    let independently_bound = independently_bound(&plan, &fixture);
    let effects = plan.effect_plan();
    let retained = effects.binding_report();
    assert_eq!(retained, &independently_bound);
    assert_eq!(retained.schema(), SchemaBindingStatus::Valid);
    assert!(retained.issues().is_empty());
    assert_eq!(retained.request_digest(), effects.bindings().request);
    assert_eq!(retained.data_identity(), &effects.bindings().definitions);
    assert_eq!(
        retained.queries().iter().map(|q| &q.id).collect::<Vec<_>>(),
        fixture
            .queries
            .requests
            .iter()
            .map(|q| &q.id)
            .collect::<Vec<_>>()
    );
    // Retaining diagnostics adds no digest input and does not change the existing
    // operations-v10 effect domain or metric-plan domain.
    assert_eq!(
        effects.identity(),
        digest_owned(
            "owned-effect-plan-v7",
            effects.bindings(),
            PlanLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    assert_eq!(
        plan.identity(),
        digest_owned(
            "owned-metric-plan-v1",
            plan.bindings(),
            PlanLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    let report_address = retained as *const DefinitionBindingReport;
    let mut scratch = plan.new_scratch();
    let first = plan.evaluate(&mut scratch).unwrap();
    assert_eq!(
        first.results.iter().map(number).collect::<Vec<_>>(),
        [11.0, 16.0, 18.0, 20.0, 11.0]
    );
    assert!(first.gaps.is_empty());
    let mut changed = support::fixture();
    changed.build.items[0].modifiers[0].rolls[0].value = integer(7);
    let changed = compile(&changed);
    assert_ne!(changed.identity(), plan.identity());
    assert_eq!(
        changed
            .evaluate(&mut scratch)
            .unwrap()
            .results
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        [11.0, 24.0, 22.0, 20.0, 11.0]
    );
    assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
    assert_eq!(
        report_address,
        plan.effect_plan().binding_report() as *const DefinitionBindingReport
    );
    assert_eq!(plan.effect_plan().binding_report(), &independently_bound);
    let workers: Vec<_> = (0..3)
        .map(|_| {
            let plan = Arc::clone(&plan);
            std::thread::spawn(move || {
                let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
                let binding = serde_json::to_vec(plan.effect_plan().binding_report()).unwrap();
                (report, binding)
            })
        })
        .collect();
    let expected_binding = serde_json::to_vec(&independently_bound).unwrap();
    for worker in workers {
        let (report, binding) = worker.join().unwrap();
        assert_eq!(report, first);
        assert_eq!(binding, expected_binding);
    }
}

#[test]
fn unresolved_report_retains_exact_occurrence_subjects_issue_classes_and_query_order() {
    let mut fixture = fixture();
    partial_modifier_inputs(&mut fixture);
    fixture
        .schema
        .definitions
        .push(DefinitionDescriptor::Metric(DefinitionEntry {
            id: def("unmapped-metric"),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: subject(def::<MetricDefinition>("unmapped-metric")),
                    facet: SchemaFacet::InputSchema,
                    code: key("metric-schema-pending"),
                }],
            },
        }));
    for (id, metric) in [
        ("missing-last", "missing-metric"),
        ("unmapped-first", "unmapped-metric"),
    ] {
        fixture.queries.requests.push(MetricRequest {
            id: QueryId::new(id).unwrap(),
            metric: def(metric),
            target: MetricTarget::Actor(ActorKey::Player),
        });
    }
    let plan = compile(&fixture);
    let retained = plan.effect_plan().binding_report();
    assert_eq!(retained, &independently_bound(&plan, &fixture));
    assert_eq!(retained.schema(), SchemaBindingStatus::Unresolved);
    let mut expected_issues: Vec<_> = [4, 5]
        .into_iter()
        .map(|modifier| BindingIssue {
            site: BindingSite {
                location: BindingLocation::Modifier {
                    item: occurrence(3),
                    modifier: occurrence(modifier),
                },
                facet: BindingFacet::RequiredValues,
            },
            class: IssueClass::Unresolved,
            code: BindingIssueCode::PartialMembership,
            subject: Some(modifier_owner()),
        })
        .collect();
    for (id, metric, code) in [
        (
            "missing-last",
            "missing-metric",
            BindingIssueCode::MissingDefinition,
        ),
        (
            "unmapped-first",
            "unmapped-metric",
            BindingIssueCode::UnmappedSchema,
        ),
    ] {
        expected_issues.push(BindingIssue {
            site: query_site(id, BindingFacet::Definition),
            class: IssueClass::Unresolved,
            code,
            subject: Some(subject(def::<MetricDefinition>(metric))),
        });
    }
    assert_eq!(retained.issues(), expected_issues);
    assert_eq!(
        retained.queries().iter().map(|q| &q.id).collect::<Vec<_>>(),
        fixture
            .queries
            .requests
            .iter()
            .map(|q| &q.id)
            .collect::<Vec<_>>()
    );
    assert!(
        retained.queries()[..5]
            .iter()
            .all(|q| q.schema == SchemaBindingStatus::Valid)
    );
    assert!(
        retained.queries()[5..]
            .iter()
            .all(|q| q.schema == SchemaBindingStatus::Unresolved
                && q.selector == SelectorBindingStatus::Unresolved)
    );
    assert!(plan.effect_plan().gaps().contains(&PlanGap {
        provider: None,
        subject: None,
        reason: PlanGapReason::SchemaUnresolved,
    }));
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.results[..5].iter().all(|row| matches!(
        row.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    )));
    assert!(report.results[5..].iter().all(|row| matches!(
        row.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingMetricBinding,
            ..
        }
    )));
    assert_eq!(retained, &independently_bound(&plan, &fixture));
}

#[test]
fn unavailable_queries_remain_distinct_from_unresolved_schema_and_keep_other_values() {
    let mut fixture = fixture();
    fixture
        .build
        .skills
        .iter_mut()
        .find(|skill| skill.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let plan = compile(&fixture);
    let retained = plan.effect_plan().binding_report();
    assert_eq!(retained, &independently_bound(&plan, &fixture));
    assert_eq!(retained.schema(), SchemaBindingStatus::Valid);
    let expected: Vec<_> = ["child-a", "child-a-again"]
        .into_iter()
        .map(|id| BindingIssue {
            site: query_site(id, BindingFacet::Target),
            class: IssueClass::Unavailable,
            code: BindingIssueCode::DisabledProvider,
            subject: None,
        })
        .collect();
    assert_eq!(retained.issues(), expected);
    for index in [0, 4] {
        assert_eq!(retained.queries()[index].schema, SchemaBindingStatus::Valid);
        assert_eq!(
            retained.queries()[index].selector,
            SelectorBindingStatus::Unavailable
        );
    }
    assert!(
        !plan
            .effect_plan()
            .gaps()
            .iter()
            .any(|gap| gap.reason == PlanGapReason::SchemaUnresolved)
    );
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(report.results[0].value, EffectValue::Inactive);
    assert_eq!(report.results[4].value, EffectValue::Inactive);
    assert_eq!(number(&report.results[1]), 16.0);
    assert_eq!(number(&report.results[2]), 18.0);
    assert_eq!(number(&report.results[3]), 20.0);
}

#[test]
fn retaining_evidence_does_not_bypass_binding_limits_or_invalid_inputs() {
    let mut fixture = fixture();
    partial_modifier_inputs(&mut fixture);
    let limits = PlanLimits {
        binding: BindingLimits {
            max_issues: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        fixture.compile_with(limits),
        Err(PlanError::Binding(BindingError::IssueLimit))
    ));
    let limits = PlanLimits {
        binding: BindingLimits {
            max_work: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        fixture.compile_with(limits),
        Err(PlanError::Binding(BindingError::WorkLimit))
    ));
    fixture.build.items[0].modifiers[0].rolls[0].value = integer(101);
    assert!(
        matches!(fixture.compile(), Err(PlanError::Invalid(message)) if message == "owned request has invalid schema bindings")
    );
}
