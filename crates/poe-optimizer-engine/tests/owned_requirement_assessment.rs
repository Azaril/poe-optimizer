//! Requirement assessment accompanies measurements and does not certify game legality.
//! Synthetic rules exercise exact ordinary occurrences.
#[allow(dead_code)]
#[path = "support/owned_metric_fixture.rs"]
mod ordinary;

use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;

fn requirement(id: &str, value: &str, when: Option<&str>) -> RuleEffect {
    RuleEffect {
        id: ordinary::key(id),
        when: when.map(ordinary::key),
        effect: RuleEffectKind::Requirement {
            satisfied: ordinary::key(value),
            code: ordinary::key(id),
        },
    }
}

fn ordinary_fixture(requirements: &[(&str, Option<bool>, bool)]) -> ordinary::Fixture {
    let mut f = ordinary::fixture();
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: ordinary::def("unavailable-condition"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    let mut program = RuleProgram {
        id: ordinary::key("requirements"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![ordinary::node(
            "inactive-guard",
            RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        )],
        effects: vec![],
    };
    for &(name, value, active) in requirements {
        let expression = if let Some(value) = value {
            RuleExpression::Literal {
                value: ParameterValue::Boolean(value),
            }
        } else {
            program.reads.push(RuleRead {
                id: ordinary::key(name),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: ordinary::def("unavailable-condition"),
                },
            });
            RuleExpression::Read {
                input: ordinary::key(name),
            }
        };
        program.nodes.push(ordinary::node(name, expression));
        program.effects.push(requirement(
            name,
            name,
            (!active).then_some("inactive-guard"),
        ));
    }
    f.owner_mut(&ordinary::class_owner())
        .programs
        .members
        .push(program);
    f
}

fn outcome<'a>(assessment: &'a OwnedRequirementAssessment, code: &str) -> &'a RequirementOutcome {
    let rows: Vec<_> = assessment
        .results
        .iter()
        .filter(|row| row.code == ordinary::key(code))
        .collect();
    assert_eq!(rows.len(), 1, "expected one exact requirement {code}");
    &rows[0].outcome
}

fn assert_complete(assessment: &OwnedRequirementAssessment) {
    assert_eq!(assessment.coverage, RequirementCoverage::Complete);
}

fn assert_ordinary_metrics(report: &OwnedMetricReport) {
    assert_eq!(
        report
            .results
            .iter()
            .map(ordinary::number)
            .collect::<Vec<_>>(),
        [11.0, 16.0, 18.0, 20.0, 11.0]
    );
}

fn query_independent_fixture() -> ordinary::Fixture {
    let mut f = ordinary_fixture(&[("selected", Some(true), true)]);
    let owner = ordinary::subject(ordinary::def::<SkillDefinition>("skill"));
    let programs = &mut f.owner_mut(&owner).programs.members;
    let index = programs
        .iter()
        .position(|program| program.id == ordinary::key("quantity"))
        .unwrap();
    let program = programs.remove(index);
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
        ordinary::output(),
    )))
    .programs
    .members
    .push(program);
    f
}

fn add_unselected_skill_requirement(f: &mut ordinary::Fixture) -> SchemaSubject {
    let mut schema = f
        .schema
        .definitions
        .iter()
        .find_map(|definition| match definition {
            DefinitionDescriptor::Skill(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) if *id == ordinary::def::<SkillDefinition>("skill") => Some(schema.clone()),
            _ => None,
        })
        .unwrap();
    schema.declarations.outputs = DeclaredSet::complete(vec![]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: ordinary::def("unselected-skill"),
            schema: SchemaState::Known(schema),
        }));
    let owner = ordinary::subject(ordinary::def::<SkillDefinition>("unselected-skill"));
    f.owners.push(DefinitionRules {
        owner: owner.clone(),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: ordinary::key("unselected-action-requirement"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![ordinary::node(
                "satisfied",
                RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                },
            )],
            effects: vec![requirement(
                "unselected-action-requirement",
                "satisfied",
                None,
            )],
        }]),
    });
    owner
}

fn select_added_skill(f: &mut ordinary::Fixture, enabled: bool) {
    f.build.skills.push(SkillUse {
        id: ordinary::occurrence(90),
        source: AuthoredSkillSource::Direct(ordinary::def("unselected-skill")),
        enabled,
        scope: LoadoutScope::Shared,
        parameters: None,
    });
}

fn added_skill_schema(f: &mut ordinary::Fixture) -> &mut SkillSchema {
    f.schema
        .definitions
        .iter_mut()
        .find_map(|definition| match definition {
            DefinitionDescriptor::Skill(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) if *id == ordinary::def::<SkillDefinition>("unselected-skill") => Some(schema),
            _ => None,
        })
        .unwrap()
}

fn add_disabled_potential_gem(f: &mut ordinary::Fixture, supplied: bool) -> ProviderKey {
    let declaration = SlotOwnerDefId::Gem(ordinary::def("potential-gem"));
    let grant = DeclaredSlot {
        declaration: declaration.clone(),
        slot: ordinary::def("potential-skill-grant"),
    };
    let supply = DeclaredSlot {
        declaration,
        slot: ordinary::def("potential-skill-supply"),
    };
    // Reuse a complete empty port set, not the potential Skill's child slots.
    let mut ports = added_skill_schema(f).declarations.clone();
    ports.outputs.members.clear();
    ports.grants.members.clear();
    ports.actors.members.clear();
    ports.skill_grants.members.clear();
    if supplied {
        ports.grants.members.push(grant.clone());
        ports.skill_grants.members.push(supply.clone());
        f.schema.slots.extend([
            SlotDescriptor::Grant(DefinitionEntry {
                id: grant,
                schema: SchemaState::Known(GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(supply.clone()),
                }),
            }),
            SlotDescriptor::SkillGrant(DefinitionEntry {
                id: supply,
                schema: SchemaState::Known(SkillGrantSlotSchema {
                    skill: ordinary::def("unselected-skill"),
                    outputs: DeclaredSet::complete(vec![]),
                    preset_inputs: None,
                }),
            }),
        ]);
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::Gem(DefinitionEntry {
            id: ordinary::def("potential-gem"),
            schema: SchemaState::Known(GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                roles: vec![AuthoredGemRole::SkillUse],
                skills: DeclaredSet::complete(vec![ordinary::def("unselected-skill")]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: ports,
            }),
        }));
    f.owners.push(DefinitionRules {
        owner: ordinary::subject(ordinary::def::<GemDefinition>("potential-gem")),
        programs: DeclaredSet::complete(vec![]),
    });
    f.build.gems.push(GemInstance {
        id: ordinary::occurrence(91),
        definition: ordinary::def("potential-gem"),
        parameters: vec![],
        level: 1,
        quality: None,
    });
    f.build.skills.push(SkillUse {
        id: ordinary::occurrence(92),
        source: AuthoredSkillSource::Gem(ordinary::occurrence(91)),
        enabled: false,
        scope: LoadoutScope::Shared,
        parameters: None,
    });
    ProviderKey {
        root: ProviderRoot::SkillUse(ordinary::occurrence(92)),
        grant_path: vec![],
    }
}

fn move_added_requirement_to(f: &mut ordinary::Fixture, owner: SchemaSubject) {
    let skill_owner = ordinary::subject(ordinary::def::<SkillDefinition>("unselected-skill"));
    let programs = std::mem::take(&mut f.owner_mut(&skill_owner).programs.members);
    f.owners.push(DefinitionRules {
        owner,
        programs: DeclaredSet::complete(programs),
    });
}

#[test]
fn unrelated_packaged_action_requirements_do_not_change_selected_assessment() {
    for query_scope in 0..3 {
        let mut f = query_independent_fixture();
        match query_scope {
            0 => f.queries.requests.clear(),
            1 => f.queries.requests.reverse(),
            _ => {}
        }
        let baseline = ordinary::compile(&f);
        let baseline = baseline.evaluate(&mut baseline.new_scratch()).unwrap();
        assert!(baseline.requirements.is_satisfied());
        let owner = add_unselected_skill_requirement(&mut f);
        for partial in [false, true] {
            if partial {
                f.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: ordinary::key("unreviewed-unselected-family"),
                    }],
                };
            }
            let plan = ordinary::compile(&f);
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            assert_eq!(report.requirements, baseline.requirements);
            assert_eq!(report.results, baseline.results);
            assert_eq!(report.gaps, baseline.gaps);
        }
    }
}

#[test]
fn selecting_action_requirement_owner_restores_gap_without_requested_action() {
    for enabled in [true, false] {
        for empty_queries in [true, false] {
            let mut f = query_independent_fixture();
            let owner = add_unselected_skill_requirement(&mut f);
            select_added_skill(&mut f, enabled);
            if empty_queries {
                f.queries.requests.clear();
            } else {
                f.queries.requests.reverse();
            }
            let plan = ordinary::compile(&f);
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
                panic!("selected potential Action family must remain incomplete");
            };
            assert!(gaps.iter().any(|gap| {
                gap.subject.as_ref() == Some(&owner)
                    && gap.reason == PlanGapReason::IncompleteRequirementContexts
            }));
            assert!(!report.requirements.is_satisfied());
            assert!(
                report
                    .requirements
                    .results
                    .iter()
                    .all(|row| row.key.invocation.owner != owner)
            );
        }
    }
}

#[test]
fn missing_or_partial_selected_program_inventory_is_not_an_empty_requirement_census() {
    for enabled in [true, false] {
        for partial in [true, false] {
            let mut f = query_independent_fixture();
            let owner = add_unselected_skill_requirement(&mut f);
            let reason = if partial {
                f.owner_mut(&owner).programs = DeclaredSet::partial(
                    vec![],
                    vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: ordinary::key("unknown-selected-requirements"),
                    }],
                );
                PlanGapReason::PartialPrograms
            } else {
                f.owners.retain(|row| row.owner != owner);
                PlanGapReason::MissingPrograms
            };
            select_added_skill(&mut f, enabled);
            f.queries.requests.clear();
            let plan = ordinary::compile(&f);
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
                panic!("unknown selected programs cannot be a complete empty inventory");
            };
            assert!(
                gaps.iter()
                    .any(|gap| gap.subject.as_ref() == Some(&owner) && gap.reason == reason)
            );
            assert!(!report.requirements.is_satisfied());
        }
    }
}

#[test]
fn disabled_potential_skill_keeps_output_requirement_family_without_queries_or_intents() {
    for supplied in [false, true] {
        let mut f = query_independent_fixture();
        let skill_owner = add_unselected_skill_requirement(&mut f);
        let output = DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(ordinary::def("unselected-skill")),
            slot: ordinary::def("unselected-output"),
        };
        added_skill_schema(&mut f)
            .declarations
            .outputs
            .members
            .push(output.clone());
        f.schema
            .slots
            .push(SlotDescriptor::ActionOutput(DefinitionEntry {
                id: output.clone(),
                schema: SchemaState::Known(ActionOutputSchema {
                    actor_role: DeclaredActorRole::ProviderActor,
                    parts: DeclaredSet::complete(vec![ordinary::def("part")]),
                    modes: DeclaredSet::complete(vec![ordinary::def("mode")]),
                    stat_sets: DeclaredSet::complete(vec![ordinary::def("set")]),
                    choices: DeclaredSet::complete(vec![]),
                }),
            }));
        let output_owner = SchemaSubject::Slot(SlotAddress::ActionOutput(output));
        move_added_requirement_to(&mut f, output_owner.clone());
        let provider = add_disabled_potential_gem(&mut f, supplied);
        f.queries.requests.clear();
        assert!(f.scenario.usage_intents.is_empty());
        let plan = ordinary::compile(&f);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
            panic!("a disabled source cannot hide the potential output inventory");
        };
        assert!(gaps.iter().any(|gap| {
            gap.subject.as_ref() == Some(&output_owner)
                && gap.reason == PlanGapReason::IncompleteRequirementContexts
        }));
        assert_eq!(
            gaps.iter().any(|gap| {
                gap.subject.as_ref() == Some(&skill_owner)
                    && gap.provider.as_ref() == Some(&provider)
                    && gap.reason == PlanGapReason::UnresolvedTopology
            }),
            !supplied
        );
        assert!(report.results.is_empty());
        assert!(
            report
                .requirements
                .results
                .iter()
                .all(|row| row.key.invocation.owner != output_owner)
        );
    }
}

#[test]
fn untraversed_potential_skill_descendants_remain_unknown_until_exact_supply_is_seen() {
    for supplied in [false, true] {
        let mut f = query_independent_fixture();
        let skill_owner = add_unselected_skill_requirement(&mut f);
        let declaration = SlotOwnerDefId::Skill(ordinary::def("unselected-skill"));
        let actor = DeclaredSlot {
            declaration: declaration.clone(),
            slot: ordinary::def("unselected-child"),
        };
        let grant = DeclaredSlot {
            declaration,
            slot: ordinary::def("unselected-child-grant"),
        };
        let ports = &mut added_skill_schema(&mut f).declarations;
        ports.actors.members.push(actor.clone());
        ports.grants.members.push(grant.clone());
        f.schema.slots.extend([
            SlotDescriptor::Actor(DefinitionEntry {
                id: actor.clone(),
                schema: SchemaState::Known(ActorSlotSchema {
                    provider_definition: None,
                    skills: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                }),
            }),
            SlotDescriptor::Grant(DefinitionEntry {
                id: grant,
                schema: SchemaState::Known(GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Actor(actor.clone()),
                }),
            }),
        ]);
        let child_owner = SchemaSubject::Slot(SlotAddress::Actor(actor));
        move_added_requirement_to(&mut f, child_owner.clone());
        let provider = add_disabled_potential_gem(&mut f, supplied);
        f.queries.requests.clear();
        let plan = ordinary::compile(&f);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
            panic!("untraversed descendants cannot become a complete empty census");
        };
        assert_eq!(
            gaps.iter().any(|gap| {
                gap.subject.as_ref() == Some(&skill_owner)
                    && gap.provider.as_ref() == Some(&provider)
                    && gap.reason == PlanGapReason::UnresolvedTopology
            }),
            !supplied
        );
        assert_eq!(
            gaps.iter().any(|gap| {
                gap.subject.as_ref() == Some(&child_owner)
                    && gap.reason == PlanGapReason::IncompleteRequirementContexts
            }),
            supplied
        );
        assert!(!report.requirements.is_satisfied());
    }
}

#[test]
fn inactive_loadout_partial_grants_keep_requirement_gap_without_scenario_intents() {
    let mut f = query_independent_fixture();
    let owner = add_unselected_skill_requirement(&mut f);
    f.owner_mut(&owner).programs.members.clear();
    added_skill_schema(&mut f).declarations.grants.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::StaticLinks,
            code: ordinary::key("unknown-inactive-descendants"),
        }],
    };
    select_added_skill(&mut f, true);
    f.build.skills.last_mut().unwrap().scope = LoadoutScope::Selected {
        loadouts: vec![ordinary::occurrence(2)],
    };
    f.queries.requests.clear();
    assert!(f.scenario.usage_intents.is_empty());
    let plan = ordinary::compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let provider = ProviderKey {
        root: ProviderRoot::SkillUse(ordinary::occurrence(90)),
        grant_path: vec![],
    };
    let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
        panic!("unknown inactive descendants cannot become an empty requirement inventory");
    };
    assert!(gaps.iter().any(|gap| {
        gap.provider.as_ref() == Some(&provider) && gap.reason == PlanGapReason::PartialDeclarations
    }));
    // The structural unknown is a requirement coverage diagnostic. It does not
    // create an executable provider, usage intent or numerical-plan gap.
    assert!(!report.gaps.iter().any(|gap| {
        gap.provider.as_ref() == Some(&provider) && gap.reason == PlanGapReason::PartialDeclarations
    }));
    assert!(plan.effect_plan().usage_resolution().intents.is_empty());
    assert!(!report.requirements.is_satisfied());
}

#[test]
fn ordinary_assessment_preserves_satisfied_violated_and_inapplicable_exact_effects() {
    let f = ordinary_fixture(&[
        ("satisfied", Some(true), true),
        ("violated", Some(false), true),
        ("inactive", None, false),
    ]);
    let plan = ordinary::compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_ordinary_metrics(&report);
    assert!(report.gaps.is_empty());
    assert_complete(&report.requirements);
    assert_eq!(
        outcome(&report.requirements, "satisfied"),
        &RequirementOutcome::Satisfied
    );
    assert_eq!(
        outcome(&report.requirements, "violated"),
        &RequirementOutcome::Violated
    );
    assert_eq!(
        outcome(&report.requirements, "inactive"),
        &RequirementOutcome::Inapplicable
    );
    assert!(report.requirements.has_violations());
    assert!(!report.requirements.is_satisfied());
    assert_eq!(report.requirements.results.len(), 3);
    for row in &report.requirements.results {
        assert_eq!(row.key.invocation.owner, ordinary::class_owner());
        assert_eq!(row.key.invocation.program, ordinary::key("requirements"));
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert_eq!(row.key.effect, row.code);
    }
}

#[test]
fn ordinary_requirement_census_is_independent_of_requested_metric_rows() {
    let mut f = ordinary_fixture(&[("violation", Some(false), true)]);
    // This test isolates Actor requirements while retaining all queried values.
    // In this fixture's V14 domain, a Skill-owned Action program with no selected
    // Action has UnsupportedContext. Put the unrelated measurement on its exact
    // output owner, whose complete inventory can be unused when no Action is queried.
    let skill_owner = ordinary::subject(ordinary::def::<SkillDefinition>("skill"));
    let programs = &mut f.owner_mut(&skill_owner).programs.members;
    let quantity = programs
        .iter()
        .position(|program| program.id == ordinary::key("quantity"))
        .unwrap();
    let quantity = programs.remove(quantity);
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
        ordinary::output(),
    )))
    .programs
    .members
    .push(quantity);
    let plan = ordinary::compile(&f);
    let full = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_ordinary_metrics(&full);
    assert_complete(&full.requirements);
    f.queries.requests.reverse();
    let plan = ordinary::compile(&f);
    let reversed = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(reversed.requirements, full.requirements);
    assert_eq!(
        reversed
            .results
            .iter()
            .map(ordinary::number)
            .collect::<Vec<_>>(),
        [11.0, 20.0, 18.0, 16.0, 11.0]
    );
    f.queries
        .requests
        .retain(|row| row.target == MetricTarget::Actor(ActorKey::Player));
    let plan = ordinary::compile(&f);
    let unrelated = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(unrelated.results.len(), 1);
    assert_eq!(ordinary::number(&unrelated.results[0]), 16.0);
    assert_eq!(unrelated.requirements, full.requirements);
    f.queries.requests.clear();
    let plan = ordinary::compile(&f);
    let empty = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(empty.results.is_empty());
    assert_eq!(empty.requirements, full.requirements);
    assert!(empty.requirements.has_violations());
}

#[test]
fn unavailable_requirement_and_incomplete_inventory_never_claim_satisfaction() {
    let f = ordinary_fixture(&[("missing", None, true)]);
    let plan = ordinary::compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_ordinary_metrics(&report);
    assert!(matches!(
        outcome(&report.requirements, "missing"),
        RequirementOutcome::Unavailable {
            cause: EffectValue::Unresolved { .. }
        }
    ));
    assert!(matches!(
        report.requirements.coverage,
        RequirementCoverage::Incomplete { .. }
    ));
    assert!(!report.requirements.has_violations());
    assert!(!report.requirements.is_satisfied());

    let mut f = ordinary_fixture(&[("known-true", Some(true), true)]);
    let owner = ordinary::class_owner();
    f.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: ordinary::key("unreviewed-requirements"),
        }],
    };
    let plan = ordinary::compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
        panic!("partial owner must remain incomplete")
    };
    assert!(!gaps.is_empty());
    assert!(!report.requirements.is_satisfied());
}

#[test]
fn query_created_action_requirements_never_upgrade_the_global_census() {
    let mut f = ordinary_fixture(&[("actor", Some(true), true)]);
    let owner = ordinary::subject(ordinary::def::<SkillDefinition>("skill"));
    let program = f
        .owner_mut(&owner)
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == ordinary::key("quantity"))
        .unwrap();
    program.nodes.push(ordinary::node(
        "action-valid",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(true),
        },
    ));
    program
        .effects
        .push(requirement("action-requirement", "action-valid", None));
    for queried in [true, false] {
        if !queried {
            f.queries
                .requests
                .retain(|r| !matches!(r.target, MetricTarget::Action(_)));
        }
        let plan = ordinary::compile(&f);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage else {
            panic!("requested Actions do not certify an independent Action census")
        };
        assert!(
            gaps.iter()
                .any(|gap| gap.reason == PlanGapReason::IncompleteRequirementContexts)
        );
        assert_eq!(
            report
                .requirements
                .results
                .iter()
                .any(|row| row.code == ordinary::key("action-requirement")),
            queried
        );
        assert!(!report.requirements.is_satisfied());
        if queried {
            assert_ordinary_metrics(&report);
        } else {
            // V14's unselected Skill Action program already blocks the whole
            // numerical plan. Requirement assessment must retain that existing
            // gap as well as its independent mechanical-context census gap.
            assert!(report.gaps.iter().any(|gap| {
                gap.reason == PlanGapReason::UnsupportedContext
                    && gap.subject.as_ref() == Some(&owner)
            }));
            assert!(report.results.iter().all(|row| matches!(
                row.value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                }
            )));
        }
    }
}

#[test]
fn unknown_selected_output_and_actor_slot_owners_cannot_be_hidden_by_queries() {
    for (owner, providers) in [
        (
            SchemaSubject::Slot(SlotAddress::ActionOutput(ordinary::output())),
            vec![ordinary::action().action.provider],
        ),
        (
            SchemaSubject::Slot(SlotAddress::Actor(ordinary::child_slot())),
            [30, 31]
                .into_iter()
                .map(|id| ProviderKey {
                    root: ProviderRoot::SkillUse(ordinary::occurrence(id)),
                    grant_path: vec![ordinary::child_grant()],
                })
                .collect(),
        ),
    ] {
        for partial in [false, true] {
            let mut f = ordinary::fixture();
            let reason = if partial {
                f.owner_mut(&owner).programs = DeclaredSet::partial(
                    vec![],
                    vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: ordinary::key("unknown-potential-requirements"),
                    }],
                );
                PlanGapReason::PartialPrograms
            } else {
                f.owners.retain(|row| row.owner != owner);
                PlanGapReason::MissingPrograms
            };
            for query_scope in [2, 1, 0] {
                if query_scope == 1 {
                    f.queries
                        .requests
                        .retain(|row| matches!(row.target, MetricTarget::Actor(_)));
                } else if query_scope == 0 {
                    f.queries.requests.clear();
                }
                let plan = ordinary::compile(&f);
                let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
                let RequirementCoverage::Incomplete { gaps, .. } = &report.requirements.coverage
                else {
                    panic!("unknown selected owner cannot become a complete empty census")
                };
                for provider in &providers {
                    assert!(
                        gaps.iter().any(|gap| {
                            gap.reason == reason
                                && gap.subject.as_ref() == Some(&owner)
                                && gap.provider.as_ref() == Some(provider)
                        }),
                        "missing exact owner gap for {owner:?}, query scope {query_scope}: {gaps:?}"
                    );
                }
                assert!(report.requirements.results.is_empty());
                assert!(!report.requirements.is_satisfied());
                assert_eq!(report.results.len(), f.queries.requests.len());
            }
        }
    }
}

#[test]
fn ordinary_assessment_is_stable_across_fresh_reused_and_parallel_attempts() {
    let failed = ordinary::compile(&ordinary_fixture(&[("condition", Some(false), true)]));
    let passed = ordinary::compile(&ordinary_fixture(&[("condition", Some(true), true)]));
    let expected_failed = failed.evaluate(&mut failed.new_scratch()).unwrap();
    let expected_passed = passed.evaluate(&mut passed.new_scratch()).unwrap();
    assert!(expected_failed.requirements.has_violations());
    assert!(expected_passed.requirements.is_satisfied());
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
