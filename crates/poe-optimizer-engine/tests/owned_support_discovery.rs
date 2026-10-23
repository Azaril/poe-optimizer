//! Composed request coverage, including empty/disabled authored assignments.
#[allow(dead_code)]
use crate::engine_fixtures::computed_support as support;
use poe_optimizer_core::{
    owned_build::*,
    owned_definitions::{ActionOutputDefId, SlotOwnerDefId},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::sync::Arc;
use support::*;

fn direct(
    f: &Fixture,
    domains: Option<SupportDiscoveryInput>,
) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
    let schema =
        Arc::new(OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap());
    let mut input = raw_rules(f, &schema, OWNED_RULE_OPERATIONS_V14);
    input.support_discovery = domains;
    let rules = Arc::new(
        CompiledRulePackage::compile(&input, schema.as_ref(), Default::default()).unwrap(),
    );
    let routing = Arc::new(
        OwnedActionRouting::new(raw_routing(f, &schema), schema.as_ref(), Default::default())
            .unwrap(),
    );
    OwnedEffectPlan::compile(
        Arc::new(f.request()),
        schema,
        rules,
        routing,
        Default::default(),
    )
    .unwrap()
}
fn prepared_plan(
    f: &Fixture,
    domains: Option<SupportDiscoveryInput>,
) -> OwnedSupportPreparationPlan<OwnedDefinitionSchemaPackage> {
    OwnedSupportPreparationPlan::compile(
        compile_inputs_with_domains(f, target(30, "first"), domains),
        Default::default(),
        Default::default(),
    )
    .unwrap()
}
fn unmapped(owner: SchemaSubject) -> SupportSourceDomainDeclaration {
    SupportSourceDomainDeclaration {
        domain: SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject: owner.clone(),
                facet: SchemaFacet::GameRules,
                code: key("unsupported-support-origin"),
            }],
        },
        owner,
    }
}
fn source_gaps(gaps: &[PlanGap]) -> Vec<PlanGap> {
    gaps.iter()
        .filter(|gap| {
            matches!(
                gap.reason,
                PlanGapReason::MissingSupportSources | PlanGapReason::UnmappedSupportSources
            )
        })
        .cloned()
        .collect()
}

#[test]
fn potential_output_source_coverage_is_independent_of_action_queries() {
    let mut f = Fixture::new();
    f.add_action_route();
    let queries = std::mem::take(&mut f.queries.requests);
    let owner = SchemaSubject::Slot(ActionOutputDefId::address(&output()));
    let provider = action().action.provider;
    for reason in [
        None,
        Some(PlanGapReason::MissingSupportSources),
        Some(PlanGapReason::UnmappedSupportSources),
    ] {
        let mut domains = assignment_only_domains(&f.owners);
        match reason {
            None => {}
            Some(PlanGapReason::MissingSupportSources) => {
                domains.providers.retain(|row| row.owner != owner);
            }
            Some(PlanGapReason::UnmappedSupportSources) => {
                *domains
                    .providers
                    .iter_mut()
                    .find(|row| row.owner == owner)
                    .unwrap() = unmapped(owner.clone());
            }
            _ => unreachable!(),
        }
        let expected = reason
            .map(|reason| PlanGap {
                provider: Some(provider.clone()),
                subject: Some(owner.clone()),
                reason,
            })
            .into_iter()
            .collect::<Vec<_>>();
        for queried in [false, true, false] {
            f.queries.requests = if queried { queries.clone() } else { vec![] };
            let plan = direct(&f, Some(domains.clone()));
            assert_eq!(source_gaps(plan.gaps()), expected, "queried={queried}");
            assert_eq!(plan.gaps().is_empty(), reason.is_none());
            let mut scratch = plan.new_scratch();
            let first = plan.evaluate(&mut scratch).unwrap();
            assert_eq!(source_gaps(&first.gaps), expected);
            assert_eq!(plan.evaluate(&mut scratch).unwrap(), first);
        }
    }
}

#[test]
fn known_output_source_domain_does_not_complete_partial_numerical_programs() {
    let mut f = Fixture::new();
    f.add_action_route();
    let owner = SchemaSubject::Slot(ActionOutputDefId::address(&output()));
    f.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unreviewed-output-programs"),
        }],
    };
    let plan = direct(&f, Some(assignment_only_domains(&f.owners)));
    assert!(source_gaps(plan.gaps()).is_empty());
    assert!(plan.gaps().contains(&PlanGap {
        provider: Some(action().action.provider),
        subject: Some(owner.clone()),
        reason: PlanGapReason::PartialPrograms,
    }));
    assert!(!f.owner_mut(&owner).programs.is_complete());
}

#[test]
fn absent_output_relation_is_not_an_inaccessible_output_inventory() {
    let mut f = Fixture::new();
    f.add_action_route();
    f.queries.requests.clear();
    f.build.skills[0].enabled = false;
    f.routes.clear();
    let output_owner = SchemaSubject::Slot(ActionOutputDefId::address(&output()));
    f.owners.retain(|row| row.owner != output_owner);
    f.schema
        .slots
        .retain(|row| !matches!(row, SlotDescriptor::ActionOutput(entry) if entry.id == output()));
    let skill = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Skill(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) if *id == def("skill") => Some(schema),
            _ => None,
        })
        .unwrap();
    skill.declarations.outputs = DeclaredSet::complete(vec![]);
    let known = direct(&f, Some(assignment_only_domains(&f.owners)));
    assert!(known.gaps().is_empty(), "{:?}", known.gaps());

    let skill = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Skill(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) if *id == def("skill") => Some(schema),
            _ => None,
        })
        .unwrap();
    skill.declarations.outputs = DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject: subject(
                def::<poe_optimizer_core::owned_definitions::SkillDefinition>("skill"),
            ),
            facet: SchemaFacet::InputSchema,
            code: key("unreviewed-output-inventory"),
        }],
    );
    let partial = direct(&f, Some(assignment_only_domains(&f.owners)));
    assert!(partial.gaps().contains(&PlanGap {
        provider: Some(action().action.provider),
        subject: None,
        reason: PlanGapReason::PartialDeclarations,
    }));
    assert!(source_gaps(partial.gaps()).is_empty());
    assert!(partial.gaps().iter().all(|gap| !matches!(
        gap.subject,
        Some(SchemaSubject::Slot(SlotAddress::ActionOutput(_)))
    )));

    // An inaccessible declaration cannot be treated as a known-empty list or
    // expanded into an invented output address.
    let owner = subject(def::<poe_optimizer_core::owned_definitions::SkillDefinition>("skill"));
    f.owners.retain(|row| row.owner != owner);
    let skill = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Skill(entry) if entry.id == def("skill") => Some(entry),
            _ => None,
        })
        .unwrap();
    skill.schema = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::InputSchema,
            code: key("unreviewed-skill-declarations"),
        }],
    };
    let inaccessible = direct(&f, Some(assignment_only_domains(&f.owners)));
    assert!(inaccessible.gaps().contains(&PlanGap {
        provider: Some(action().action.provider),
        subject: None,
        reason: PlanGapReason::UnresolvedTopology,
    }));
    assert!(source_gaps(inaccessible.gaps()).is_empty());
    assert!(inaccessible.gaps().iter().all(|gap| !matches!(
        gap.subject,
        Some(SchemaSubject::Slot(SlotAddress::ActionOutput(_)))
    )));
}

#[test]
fn disabled_descendant_output_domains_keep_the_exact_provider_path() {
    let mut f = generated_fixture();
    f.build
        .skills
        .iter_mut()
        .find(|skill| skill.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let child_output = DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def::<poe_optimizer_core::owned_definitions::ActionOutputDefinition>("child-output"),
    };
    let owner = SchemaSubject::Slot(ActionOutputDefId::address(&child_output));
    let actor = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Actor(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) if *id == def("family") => Some(schema),
            _ => None,
        })
        .unwrap();
    actor
        .declarations
        .outputs
        .members
        .push(child_output.clone());
    let child = f
        .schema
        .slots
        .iter_mut()
        .find_map(|row| match row {
            SlotDescriptor::Actor(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) if *id == child_slot() => Some(schema),
            _ => None,
        })
        .unwrap();
    child.outputs.members.push(child_output.clone());
    let mut descriptor = f
        .schema
        .slots
        .iter()
        .find(|row| matches!(row, SlotDescriptor::ActionOutput(entry) if entry.id == output()))
        .unwrap()
        .clone();
    let SlotDescriptor::ActionOutput(entry) = &mut descriptor else {
        unreachable!()
    };
    entry.id = child_output;
    f.schema.slots.push(descriptor);
    f.owners.push(DefinitionRules {
        owner: owner.clone(),
        programs: DeclaredSet::complete(vec![]),
    });
    assert!(f.queries.requests.is_empty());
    let domains = assignment_only_domains(&f.owners);
    let known = prepared_plan(&f, Some(domains.clone()));
    assert!(known.gaps().is_empty(), "{:?}", known.gaps());
    for reason in [
        PlanGapReason::MissingSupportSources,
        PlanGapReason::UnmappedSupportSources,
    ] {
        let mut domains = domains.clone();
        domains.providers.retain(|row| row.owner != owner);
        if reason == PlanGapReason::UnmappedSupportSources {
            domains.providers.push(unmapped(owner.clone()));
        }
        let unknown = prepared_plan(&f, Some(domains));
        let expected = [30, 31]
            .map(|id| {
                let mut provider = summoner_provider(id);
                provider.grant_path.push(child_grant());
                PlanGap {
                    provider: Some(provider),
                    subject: Some(owner.clone()),
                    reason,
                }
            })
            .to_vec();
        assert_eq!(source_gaps(unknown.gaps()), expected);
        assert!(matches!(
            unknown
                .evaluate(&mut unknown.new_scratch())
                .unwrap()
                .outcome,
            ComputedSupportOutcome::Unavailable { .. }
        ));
    }
}

#[test]
fn empty_authored_inventory_does_not_prove_complete_sources() {
    let f = Fixture::new();
    assert!(f.build.supports.is_empty());
    let known = direct(&f, Some(assignment_only_domains(&f.owners)));
    assert!(known.gaps().is_empty(), "{:?}", known.gaps());
    for domain in [None, Some(SupportDiscoveryInput { providers: vec![] })] {
        let unknown = direct(&f, domain);
        assert_ne!(known.identity(), unknown.identity());
        assert!(
            unknown
                .gaps()
                .iter()
                .any(|gap| gap.reason == PlanGapReason::MissingSupportSources)
        );
    }
}

#[test]
fn exact_duplicate_occurrences_are_checked_even_with_no_programs() {
    let mut f = Fixture::new();
    f.owner_mut(&item_owner()).programs.members.clear();
    let mut domains = assignment_only_domains(&f.owners);
    *domains
        .providers
        .iter_mut()
        .find(|row| row.owner == item_owner())
        .unwrap() = unmapped(item_owner());
    let plan = direct(&f, Some(domains.clone()));
    let providers: Vec<_> = plan
        .gaps()
        .iter()
        .filter(|g| g.reason == PlanGapReason::UnmappedSupportSources)
        .map(|g| g.provider.clone().unwrap())
        .collect();
    assert_eq!(
        providers,
        vec![
            ProviderKey {
                root: ProviderRoot::EquipmentUse(occurrence(6)),
                grant_path: vec![]
            },
            ProviderKey {
                root: ProviderRoot::EquipmentUse(occurrence(7)),
                grant_path: vec![]
            },
        ]
    );
    // The same injected catalog is sufficient after removing the unknown
    // supplying occurrences; unselected definitions do not poison every build.
    f.build.equipment.clear();
    let changed = direct(&f, Some(domains));
    assert!(changed.gaps().is_empty(), "{:?}", changed.gaps());
    assert_ne!(plan.identity(), changed.identity());
}

#[test]
fn switching_loadouts_rechecks_the_same_catalog_against_new_providers() {
    let mut f = Fixture::new();
    f.build.equipment.truncate(1);
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let mut domains = assignment_only_domains(&f.owners);
    domains.providers.retain(|row| row.owner != item_owner());
    let first = direct(&f, Some(domains.clone()));
    assert!(first.gaps().is_empty(), "{:?}", first.gaps());
    f.build.active_weapon_loadout = occurrence(2);
    let second = direct(&f, Some(domains));
    assert!(
        second
            .gaps()
            .iter()
            .any(|gap| gap.subject.as_ref() == Some(&item_owner())
                && gap.reason == PlanGapReason::MissingSupportSources)
    );
    assert_ne!(first.identity(), second.identity());
}

#[test]
fn unselected_loadout_outputs_do_not_poison_selected_support_sources() {
    let mut f = Fixture::new();
    f.add_action_route();
    f.queries.requests.clear();
    assert!(f.build.skills[0].enabled);
    f.build.skills[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let owner = SchemaSubject::Slot(ActionOutputDefId::address(&output()));
    for reason in [
        PlanGapReason::MissingSupportSources,
        PlanGapReason::UnmappedSupportSources,
    ] {
        let mut domains = assignment_only_domains(&f.owners);
        domains.providers.retain(|row| row.owner != owner);
        if reason == PlanGapReason::UnmappedSupportSources {
            domains.providers.push(unmapped(owner.clone()));
        }
        for selected in [false, true, false] {
            f.build.active_weapon_loadout = occurrence(if selected { 2 } else { 1 });
            let plan = direct(&f, Some(domains.clone()));
            let expected = if selected {
                vec![PlanGap {
                    provider: Some(action().action.provider),
                    subject: Some(owner.clone()),
                    reason,
                }]
            } else {
                vec![]
            };
            assert_eq!(source_gaps(plan.gaps()), expected);
            assert_eq!(plan.gaps().is_empty(), !selected);
        }
    }
}

#[test]
fn disabled_assignments_and_staged_empty_orders_cannot_bypass_discovery() {
    let mut f = generated_fixture();
    for support in &mut f.build.supports {
        support.enabled = false;
    }
    let known = prepared_plan(&f, Some(assignment_only_domains(&f.owners)));
    assert!(matches!(
        known.evaluate(&mut known.new_scratch()).unwrap().outcome,
        ComputedSupportOutcome::Prepared { .. }
    ));
    let mut domains = assignment_only_domains(&f.owners);
    domains.providers.retain(|row| {
        row.owner
            != subject(def::<poe_optimizer_core::owned_definitions::GemDefinition>(
                "support",
            ))
    });
    let unknown = prepared_plan(&f, Some(domains));
    assert!(unknown.gaps().iter().any(|gap| matches!(
        &gap.provider,
        Some(ProviderKey {
            root: ProviderRoot::SupportAssignment(_),
            ..
        })
    ) && gap.reason
        == PlanGapReason::MissingSupportSources));
    assert!(matches!(
        unknown
            .evaluate(&mut unknown.new_scratch())
            .unwrap()
            .outcome,
        ComputedSupportOutcome::Unavailable { .. }
    ));
    f.build.supports.clear();
    for sequence in f.build.authored_support_order.as_mut().unwrap() {
        sequence.assignments.clear();
    }
    let empty = prepared_plan(&f, None);
    assert!(matches!(
        empty.evaluate(&mut empty.new_scratch()).unwrap().outcome,
        ComputedSupportOutcome::Unavailable { .. }
    ));
}

#[test]
fn disabled_summoners_still_require_their_exact_descendant_source_domains() {
    let mut f = generated_fixture();
    f.build
        .skills
        .iter_mut()
        .find(|skill| skill.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let known = prepared_plan(&f, Some(assignment_only_domains(&f.owners)));
    assert!(known.gaps().is_empty(), "{:?}", known.gaps());
    let mut domains = assignment_only_domains(&f.owners);
    let ability = subject(def::<poe_optimizer_core::owned_definitions::SkillDefinition>("ability"));
    domains.providers.retain(|row| row.owner != ability);
    let unknown = prepared_plan(&f, Some(domains));
    assert!(unknown.gaps().iter().any(|gap| {
        gap.reason == PlanGapReason::MissingSupportSources
            && gap.subject.as_ref() == Some(&ability)
            && gap.provider.as_ref().is_some_and(|provider| {
                provider.root == ProviderRoot::SkillUse(occurrence(30))
                    && !provider.grant_path.is_empty()
            })
    }));
}

#[test]
fn coverage_failures_are_deterministic_with_reused_scratch_and_parallel_workers() {
    let f = generated_fixture();
    let known = prepared_plan(&f, Some(assignment_only_domains(&f.owners)));
    let unknown = prepared_plan(&f, None);
    let mut changed = generated_fixture();
    changed.build.supports[0].enabled = false;
    let other = prepared_plan(&changed, Some(assignment_only_domains(&changed.owners)));
    let plans = [&known, &unknown, &other, &known];
    let fresh: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = known.new_scratch();
    for (p, expected) in plans.iter().zip(&fresh) {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel = pool.install(|| {
        plans
            .par_iter()
            .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(fresh, parallel);
    assert_eq!(fresh[0], fresh[3]);
    assert_ne!(fresh[0].identity, fresh[1].identity);
}

#[test]
fn raw_and_stored_compilers_reject_the_same_invalid_source_declarations() {
    let f = Fixture::new();
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut input = raw_rules(&f, &schema, OWNED_RULE_OPERATIONS_V14);
    let original = input.support_discovery.clone().unwrap();
    let invalid = [
        SupportDiscoveryInput {
            providers: vec![original.providers[0].clone(), original.providers[0].clone()],
        },
        SupportDiscoveryInput {
            providers: vec![unmapped(subject(def::<
                poe_optimizer_core::owned_definitions::SkillDefinition,
            >("absent")))],
        },
        SupportDiscoveryInput {
            providers: vec![SupportSourceDomainDeclaration {
                owner: item_owner(),
                domain: SchemaState::Unmapped { gaps: vec![] },
            }],
        },
    ];
    for domain in invalid {
        input.support_discovery = Some(domain);
        assert!(OwnedRulePackage::new(input.clone(), &schema, Default::default()).is_err());
        assert!(CompiledRulePackage::compile(&input, &schema, Default::default()).is_err());
    }
}
