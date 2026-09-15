//! Native root ownership: injected class/ascendancy data, no source tree or UI.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod owned_plan_fixture;
use owned_plan_fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;

fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn root_owner(name: &str) -> SchemaSubject {
    subject(def::<PassiveNodeDefinition>(name))
}
fn add_root(f: &mut Fixture, name: &str, amount: i64) {
    f.schema
        .definitions
        .push(DefinitionDescriptor::PassiveNode(DefinitionEntry {
            id: def(name),
            schema: SchemaState::Known(PassiveNodeSchema {
                pools: empty(),
                adjacent: empty(),
                declarations: ports(),
            }),
        }));
    f.owners.push(DefinitionRules {
        owner: root_owner(name),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("implicit-bonus"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![literal("amount", amount)],
            effects: vec![effect(
                "bonus",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: def("actor-total"),
                    contribution: ContributionKind::Add,
                    value: key("amount"),
                },
            )],
        }]),
    });
}
fn class(f: &mut Fixture) -> &mut ClassSchema {
    f.schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Class(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => Some(s),
            _ => None,
        })
        .unwrap()
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::GameRules,
            code: key("pending-root-membership"),
        }],
    }
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn actor_total(r: &OwnedEffectsReport) -> &EffectValue {
    &r.values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def("actor-total"),
                }
        })
        .unwrap()
        .value
}

#[test]
fn shared_class_and_ascendancy_root_contributes_once_without_paid_allocation() {
    let mut f = Fixture::new();
    add_root(&mut f, "shared-root", 3);
    class(&mut f).implicit_passives = DeclaredSet::complete(vec![def("shared-root")]);
    class(&mut f).ascendancies = DeclaredSet::complete(vec![def("ascendancy")]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Ascendancy(DefinitionEntry {
            id: def("ascendancy"),
            schema: SchemaState::Known(AscendancySchema {
                classes: DeclaredSet::complete(vec![def("class")]),
                implicit_passives: DeclaredSet::complete(vec![def("shared-root")]),
                declarations: ports(),
            }),
        }));
    f.owners.push(DefinitionRules {
        owner: subject(def::<AscendancyDefinition>("ascendancy")),
        programs: empty(),
    });
    f.build.character.ascendancy = Some(def("ascendancy"));
    assert!(f.build.allocations.is_empty());
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{report:?}");
    assert_eq!(
        actor_total(&report),
        &EffectValue::Known { value: integer(19) }
    );
    let effects: Vec<_> = report
        .effects
        .iter()
        .filter(|row| row.key.invocation.owner == root_owner("shared-root"))
        .collect();
    assert_eq!(effects.len(), 1);
    assert!(
        matches!(&effects[0].key.invocation.origin, RuleOrigin::Provider {provider}
        if provider.root == ProviderRoot::Character && provider.grant_path.is_empty())
    );
}

#[test]
fn changing_selected_class_replaces_implicit_root_effects() {
    let mut f = Fixture::new();
    add_root(&mut f, "first-root", 3);
    add_root(&mut f, "second-root", 5);
    class(&mut f).implicit_passives = DeclaredSet::complete(vec![def("first-root")]);
    let mut second_class = class(&mut f).clone();
    second_class.implicit_passives = DeclaredSet::complete(vec![def("second-root")]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Class(DefinitionEntry {
            id: def("second-class"),
            schema: SchemaState::Known(second_class),
        }));
    let mut second_rules = f.owner_mut(&class_owner()).clone();
    second_rules.owner = subject(def::<ClassDefinition>("second-class"));
    f.owners.push(second_rules);
    assert_eq!(
        actor_total(&evaluate(&f)),
        &EffectValue::Known { value: integer(19) }
    );
    f.build.character.class = def("second-class");
    let report = evaluate(&f);
    assert_eq!(
        actor_total(&report),
        &EffectValue::Known { value: integer(21) }
    );
    assert!(
        report
            .effects
            .iter()
            .all(|row| row.key.invocation.owner != root_owner("first-root"))
    );
}

#[test]
fn open_root_or_pool_membership_preserves_known_effects_but_blocks_reduction() {
    for partial_pool in [false, true] {
        let mut f = Fixture::new();
        add_root(&mut f, "root", 3);
        class(&mut f).implicit_passives = DeclaredSet::complete(vec![def("root")]);
        if partial_pool {
            let node = f
                .schema
                .definitions
                .iter_mut()
                .find_map(|row| match row {
                    DefinitionDescriptor::PassiveNode(DefinitionEntry {
                        schema: SchemaState::Known(s),
                        ..
                    }) => Some(s),
                    _ => None,
                })
                .unwrap();
            node.pools.closure = partial(root_owner("root"));
        } else {
            class(&mut f).implicit_passives.closure = partial(class_owner());
        }
        let report = evaluate(&f);
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialDeclarations),
            "{report:?}"
        );
        assert!(matches!(
            actor_total(&report),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
        let row = report
            .effects
            .iter()
            .find(|row| row.key.invocation.owner == root_owner("root"))
            .expect("known root owner survives partial membership");
        assert_eq!(row.value, EffectValue::Known { value: integer(3) });
    }
}

#[test]
fn unmapped_implicit_root_does_not_turn_into_empty_contributions() {
    let mut f = Fixture::new();
    class(&mut f).implicit_passives = DeclaredSet::complete(vec![def("unknown-root")]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::PassiveNode(DefinitionEntry {
            id: def("unknown-root"),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: root_owner("unknown-root"),
                    facet: SchemaFacet::GameRules,
                    code: key("unconverted-root"),
                }],
            },
        }));
    let report = evaluate(&f);
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::UnresolvedTopology)
    );
    assert!(matches!(
        actor_total(&report),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}
