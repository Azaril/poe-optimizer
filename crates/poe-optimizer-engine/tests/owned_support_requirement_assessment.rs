//! Requirement assessment over retained support, including preparation failures.
#[allow(dead_code, unused_imports)]
use crate::engine_fixtures::support_metrics as supported;

use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::ActionOutputRoutes, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;

fn requirement(id: &str, value: &str, when: Option<&str>) -> RuleEffect {
    RuleEffect {
        id: supported::key(id),
        when: when.map(supported::key),
        effect: RuleEffectKind::Requirement {
            satisfied: supported::key(value),
            code: supported::key(id),
        },
    }
}

fn assert_complete(assessment: &OwnedRequirementAssessment) {
    assert_eq!(assessment.coverage, RequirementCoverage::Complete);
}

fn support_fixture() -> supported::Fixture {
    let mut f = supported::fixture();
    let owner = supported::delivery::subject(supported::def::<ActorDefinition>("family"));
    let program = f
        .owner_mut(&owner)
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == supported::key("actor-final"))
        .unwrap();
    program.nodes.extend([
        supported::delivery::fixture::literal("zero", 0),
        supported::delivery::fixture::node(
            "received-support",
            RuleExpression::Compare {
                operation: RuleComparison::Greater,
                left: supported::key("value"),
                right: supported::key("zero"),
            },
        ),
    ]);
    program.effects.push(requirement(
        "has-retained-support",
        "received-support",
        None,
    ));
    f
}

fn support_plan(f: &supported::Fixture, accepted: bool) -> supported::Plan {
    support_plan_for(f, accepted, "actor-deliver")
}

fn support_plan_for(f: &supported::Fixture, accepted: bool, program_name: &str) -> supported::Plan {
    supported::compile_with(
        f,
        |rules| {
            let program = rules
                .owners
                .iter_mut()
                .find(|owner| owner.owner == supported::delivery::support_owner())
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|program| program.id == supported::key(program_name))
                .unwrap();
            program.nodes.push(supported::delivery::fixture::node(
                "accepted",
                RuleExpression::Literal {
                    value: ParameterValue::Boolean(accepted),
                },
            ));
            program
                .effects
                .push(requirement("delivery-requirement", "accepted", None));
        },
        |_| {},
        |_| {},
        |_| {},
    )
}

#[test]
fn support_actor_requirements_remain_present_with_no_requested_metrics() {
    let mut f = support_fixture();
    let plan = support_plan(&f, false);
    let full = plan.evaluate(&mut plan.new_scratch()).unwrap();
    f.queries.requests.clear();
    let plan = support_plan(&f, false);
    let empty = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(empty.evaluation.results.is_empty());
    assert_eq!(empty.evaluation.requirements, full.evaluation.requirements);
    assert!(empty.evaluation.requirements.has_violations());
}

#[test]
fn action_support_delivery_requires_an_independent_context_census() {
    let mut f = support_fixture();
    for retained_queries in [2, 1, 0] {
        if retained_queries == 1 {
            f.queries
                .requests
                .retain(|row| matches!(row.target, MetricTarget::Actor(_)));
        } else if retained_queries == 0 {
            f.queries.requests.clear();
        }
        let plan = support_plan_for(&f, true, "action-deliver");
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(report.support, SupportMetricStatus::Evaluated));
        let RequirementCoverage::Incomplete { gaps, .. } = &report.evaluation.requirements.coverage
        else {
            panic!("Action support delivery requires independent context coverage")
        };
        assert!(gaps.iter().any(|gap| {
            gap.reason == PlanGapReason::IncompleteRequirementContexts
                && gap.subject == Some(supported::delivery::support_owner())
        }));
        assert!(!report.evaluation.requirements.is_satisfied());
        assert_eq!(report.evaluation.results.len(), f.queries.requests.len());
        assert!(
            report
                .evaluation
                .results
                .iter()
                .all(|row| matches!(row.value, EffectValue::Known { .. }))
        );
    }
}

#[test]
fn unselected_support_action_requirements_do_not_change_the_selected_assessment() {
    for empty_queries in [false, true] {
        let mut f = support_fixture();
        if empty_queries {
            f.queries.requests.clear();
        }
        let baseline = support_plan(&f, true);
        let expected = baseline.evaluate(&mut baseline.new_scratch()).unwrap();
        assert_complete(&expected.evaluation.requirements);
        assert!(expected.evaluation.requirements.is_satisfied());

        let foreign = supported::def::<GemDefinition>("unselected-support");
        let mut descriptor = f
            .schema
            .definitions
            .iter()
            .find_map(|row| match row {
                DefinitionDescriptor::Gem(row) if row.id == supported::def("support") => {
                    Some(row.clone())
                }
                _ => None,
            })
            .unwrap();
        descriptor.id = foreign.clone();
        f.schema
            .definitions
            .push(DefinitionDescriptor::Gem(descriptor));
        f.owners.push(DefinitionRules {
            owner: supported::delivery::subject(foreign),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: supported::key("unselected-action-requirement"),
                context: RuleEntityKind::Action,
                reads: vec![],
                nodes: vec![supported::delivery::fixture::bool_node("false", false)],
                effects: vec![requirement("unselected-violation", "false", None)],
            }]),
        });

        let plan = support_plan(&f, true);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(report.support, expected.support);
        assert_eq!(report.evaluation.results, expected.evaluation.results);
        assert_eq!(
            report.evaluation.requirements,
            expected.evaluation.requirements
        );
    }
}

fn separately_declared_output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(supported::def("output-declaration")),
        slot: supported::def("exposed-output"),
    }
}

/// The supplying ability exposes this exact output through its SkillGrant slot.
/// Its own declaration does not contain it, and no receiving endpoint or query
/// registers an Action for it.
fn with_separately_declared_output() -> supported::Fixture {
    let mut f = support_fixture();
    let output = separately_declared_output();
    let mut declaration = f
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::Skill(row) if row.id == supported::def("ability") => {
                Some(row.clone())
            }
            _ => None,
        })
        .unwrap();
    declaration.id = supported::def("output-declaration");
    let SchemaState::Known(schema) = &mut declaration.schema else {
        unreachable!()
    };
    schema.declarations.parameters = DeclaredSet::complete(vec![]);
    schema.declarations.outputs = DeclaredSet::complete(vec![output.clone()]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Skill(declaration));
    let mut descriptor = f
        .schema
        .slots
        .iter()
        .find_map(|row| match row {
            SlotDescriptor::ActionOutput(row) if row.id == supported::delivery::output() => {
                Some(row.clone())
            }
            _ => None,
        })
        .unwrap();
    descriptor.id = output.clone();
    f.schema
        .slots
        .push(SlotDescriptor::ActionOutput(descriptor));
    let mut supplied = 0;
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = slot
            && schema.skill == supported::def::<SkillDefinition>("ability")
        {
            schema.outputs.members.push(output.clone());
            supplied += 1;
        }
    }
    assert_eq!(supplied, 2);
    f.routes.push(ActionOutputRoutes {
        output: output.clone(),
        routes: DeclaredSet::complete(vec![]),
        source_selectors: Some(DeclaredSet::complete(vec![])),
    });
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(ActionOutputDefId::address(&output)),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: supported::key("exposed-output-requirement"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![supported::delivery::fixture::bool_node("false", false)],
            effects: vec![requirement("exposed-output-violation", "false", None)],
        }]),
    });
    f
}

#[test]
fn supplied_skill_output_requirement_is_censused_outside_its_own_declaration() {
    for empty_queries in [false, true] {
        let mut f = with_separately_declared_output();
        if empty_queries {
            f.queries.requests.clear();
        }
        let plan = support_plan(&f, true);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(report.support, SupportMetricStatus::Evaluated));
        assert!(
            report
                .evaluation
                .results
                .iter()
                .all(|row| matches!(row.value, EffectValue::Known { .. }))
        );
        let RequirementCoverage::Incomplete { gaps, .. } = &report.evaluation.requirements.coverage
        else {
            panic!("an exposed output remains a potential requirement owner without queries")
        };
        assert!(gaps.iter().any(|gap| {
            gap.reason == PlanGapReason::IncompleteRequirementContexts
                && gap.subject
                    == Some(SchemaSubject::Slot(ActionOutputDefId::address(
                        &separately_declared_output(),
                    )))
        }));
        assert!(!report.evaluation.requirements.is_satisfied());
        assert!(
            report
                .evaluation
                .requirements
                .results
                .iter()
                .all(|row| row.code != supported::key("exposed-output-violation")),
            "census must not create an Action or run its requirement"
        );
    }
}

#[test]
fn missing_or_partial_supplied_output_programs_remain_incomplete_without_queries() {
    let subject = SchemaSubject::Slot(ActionOutputDefId::address(&separately_declared_output()));
    for missing in [false, true] {
        for empty_queries in [false, true] {
            let mut f = with_separately_declared_output();
            if empty_queries {
                f.queries.requests.clear();
            }
            let reason = if missing {
                f.owners.retain(|row| row.owner != subject);
                PlanGapReason::MissingPrograms
            } else {
                let owner = f.owner_mut(&subject);
                owner.programs.members.clear();
                owner.programs.closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: subject.clone(),
                        facet: SchemaFacet::GameRules,
                        code: supported::key("unreviewed-exposed-output"),
                    }],
                };
                PlanGapReason::PartialPrograms
            };
            let plan = support_plan(&f, true);
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            assert!(matches!(report.support, SupportMetricStatus::Evaluated));
            assert!(
                report
                    .evaluation
                    .results
                    .iter()
                    .all(|row| matches!(row.value, EffectValue::Known { .. }))
            );
            let RequirementCoverage::Incomplete { gaps, .. } =
                &report.evaluation.requirements.coverage
            else {
                panic!("unknown potential output programs cannot prove no requirements")
            };
            for root in [30, 31] {
                for skill in ["first", "second"] {
                    let provider = supported::action(root, skill, "part").action.provider;
                    assert!(gaps.iter().any(|gap| {
                        gap.reason == reason
                            && gap.subject.as_ref() == Some(&subject)
                            && gap.provider.as_ref() == Some(&provider)
                    }));
                }
            }
            assert!(!report.evaluation.requirements.is_satisfied());
        }
    }
}

#[test]
fn support_assessment_uses_final_rebound_reads_and_appended_delivery_effects() {
    let f = support_fixture();
    let plan = support_plan(&f, false);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(report.support, SupportMetricStatus::Evaluated));
    assert_eq!(
        report
            .evaluation
            .results
            .iter()
            .map(supported::number)
            .collect::<Vec<_>>(),
        [32.0, 16.0, 16.0, 25.0, 32.0, 25.0]
    );
    let assessment = &report.evaluation.requirements;
    assert_complete(assessment);
    assert!(assessment.has_violations());
    assert!(!assessment.is_satisfied());
    let rebound: Vec<_> = assessment
        .results
        .iter()
        .filter(|row| row.code == supported::key("has-retained-support"))
        .collect();
    assert_eq!(rebound.len(), 2);
    for row in rebound {
        assert_eq!(
            row.outcome,
            RequirementOutcome::Satisfied,
            "final total must include retained support, not the empty base contribution set"
        );
        assert_eq!(row.key.invocation.program, supported::key("actor-final"));
    }
    let appended: Vec<_> = assessment
        .results
        .iter()
        .filter(|row| row.code == supported::key("delivery-requirement"))
        .collect();
    assert!(!appended.is_empty());
    for row in appended {
        assert_eq!(row.outcome, RequirementOutcome::Violated);
        assert!(matches!(
            row.key.invocation.origin,
            RuleOrigin::SupportApplication { .. }
        ));
        assert_eq!(
            row.key.invocation.owner,
            supported::delivery::support_owner()
        );
        assert!(matches!(
            row.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Owned(_))
        ));
    }
}

#[test]
fn failed_support_preparation_has_incomplete_assessment_and_keeps_query_rows() {
    let mut f = support_fixture();
    let prefix = f
        .owner_mut(&supported::delivery::support_owner())
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == supported::key("origin-facts"))
        .unwrap();
    prefix.nodes.push(supported::delivery::fixture::node(
        "prefix-condition",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    ));
    prefix
        .effects
        .push(requirement("prefix-violation", "prefix-condition", None));
    f.build.authored_support_order = None;
    let plan = support_plan(&f, false);
    let mut scratch = plan.new_scratch();
    let report = plan.evaluate(&mut scratch).unwrap();
    assert!(matches!(
        report.support,
        SupportMetricStatus::PreparationUnresolved { .. }
    ));
    assert!(matches!(
        report.evaluation.requirements.coverage,
        RequirementCoverage::Incomplete { .. }
    ));
    assert!(!report.evaluation.requirements.is_satisfied());
    assert!(report.evaluation.requirements.has_violations());
    let prefix_results: Vec<_> = report
        .evaluation
        .requirements
        .results
        .iter()
        .filter(|row| row.code == supported::key("prefix-violation"))
        .collect();
    assert!(!prefix_results.is_empty());
    for row in prefix_results {
        assert_eq!(row.outcome, RequirementOutcome::Violated);
        assert_eq!(row.key.invocation.program, supported::key("origin-facts"));
        assert!(matches!(
            row.key.invocation.entity,
            ConcreteEntity::SupportOrigin(_)
        ));
    }
    assert_eq!(report.evaluation.results.len(), f.queries.requests.len());
    assert!(
        report
            .evaluation
            .results
            .iter()
            .all(|row| matches!(row.value, EffectValue::Unresolved { .. }))
    );
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), report);
    assert_eq!(plan.evaluate(&mut plan.new_scratch()).unwrap(), report);
}

#[test]
fn support_assessment_is_stable_across_fresh_reused_and_parallel_attempts() {
    let f = support_fixture();
    let failed = support_plan(&f, false);
    let passed = support_plan(&f, true);
    let expected_failed = failed.evaluate(&mut failed.new_scratch()).unwrap();
    let expected_passed = passed.evaluate(&mut passed.new_scratch()).unwrap();
    assert!(expected_failed.evaluation.requirements.has_violations());
    assert!(expected_passed.evaluation.requirements.is_satisfied());
    let mut scratch = failed.new_scratch();
    assert_eq!(failed.evaluate(&mut scratch).unwrap(), expected_failed);
    assert_eq!(passed.evaluate(&mut scratch).unwrap(), expected_passed);
    assert_eq!(failed.evaluate(&mut scratch).unwrap(), expected_failed);
    assert!(
        (0..16)
            .into_par_iter()
            .map(|_| failed.evaluate(&mut failed.new_scratch()).unwrap())
            .all(|report| report == expected_failed)
    );
}
