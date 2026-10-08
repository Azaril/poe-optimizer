//! Checked Skill reductions reuse real generated supply, readiness and support execution.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;
use poe_optimizer_core::owned_stages::StageChannel;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_source_properties::*,
};
use poe_optimizer_data::owned_rules::{OwnedRulePackage, decode_rule_package, encode_rule_package};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use support::delivery::fixture as base;
use support::*;

fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn member(effect: &str, rank: u32) -> ContributionMember {
    ContributionMember {
        owner: child_owner(),
        program: key("query-producer"),
        effect: key(effect),
        origin: ContributionOrigin::Skill {
            authored: false,
            supplies: vec![ability_supply("first"), ability_supply("second")],
        },
        order: Some(ContributionOrder {
            source_rank: 0,
            program_rank: 0,
            effect_rank: rank,
            slot_ranks: vec![],
        }),
    }
}
struct World {
    f: Fixture,
    queries: DeclaredSet<ContributionQuery>,
}
impl World {
    fn new() -> Self {
        let mut f = fixture();
        for name in ["skill-channel", "skill-total"] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Skill, RuleEntityKind::Actor],
                },
            )));
        }
        f.owner_mut(&child_owner()).programs.members.extend([
            RuleProgram {
                id: key("query-producer"),
                context: RuleEntityKind::Skill,
                reads: vec![base::read(
                    "level",
                    RuleReadSource::Parameter {
                        slot: ability_parameter("level"),
                    },
                )],
                nodes: vec![base::read_node("level", "level"), base::literal("bonus", 3)],
                effects: ["level", "bonus"]
                    .into_iter()
                    .map(|name| {
                        effect(
                            name,
                            RuleEffectKind::Contribute {
                                entity: RuleEntity::Current,
                                stat: def("skill-channel"),
                                contribution: ContributionKind::Add,
                                value: key(name),
                            },
                        )
                    })
                    .collect(),
            },
            RuleProgram {
                id: key("query-consumer"),
                context: RuleEntityKind::Skill,
                reads: vec![base::read(
                    "sum",
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Current,
                        query: key("skill-query"),
                        group: key("self"),
                    },
                )],
                nodes: vec![base::read_node("sum", "sum")],
                effects: vec![base::derive(
                    "total",
                    RuleEntity::Current,
                    "skill-total",
                    "sum",
                )],
            },
        ]);
        Self {
            f,
            queries: DeclaredSet::complete(vec![ContributionQuery {
                id: key("skill-query"),
                stat: def("skill-channel"),
                contribution: ContributionKind::Add,
                groups: vec![ContributionGroup {
                    id: key("self"),
                    reduction: ContributionReduction::Sum,
                    ordering: ContributionOrdering::Ordered,
                    empty: base::integer(0),
                    members: DeclaredSet::complete(vec![member("level", 0), member("bonus", 1)]),
                }],
            }]),
        }
    }
    fn inputs(&self, edit: impl FnOnce(&mut RulePackageInput)) -> Checked<Inputs> {
        inputs_with_operations(
            &self.f,
            false,
            OWNED_RULE_OPERATIONS_V24,
            |r| {
                r.existing_actor_rules = Some(DeclaredSet::complete(vec![]));
                r.contribution_queries = Some(self.queries.clone());
                edit(r);
            },
            |s| {
                s.schema_version = 3;
                for row in &mut s.readiness.as_mut().unwrap().programs.members {
                    if row.program == key("direct-supply") {
                        row.phase = ReadinessPhase::Structural;
                        row.role = ReadinessProgramRole::PreparationFacts;
                        row.outputs = vec![StageChannel::Grant {
                            slot: DeclaredSlot {
                                declaration: SlotOwnerDefId::ItemTemplate(def("item")),
                                slot: def("direct-grant"),
                            },
                        }];
                        s.programs
                            .members
                            .iter_mut()
                            .find(|p| p.owner == row.owner && p.program == row.program)
                            .unwrap()
                            .stage = key("prepare");
                    }
                }
                if self
                    .f
                    .build
                    .skills
                    .iter()
                    .any(|s| s.source == AuthoredSkillSource::Direct(def("direct")))
                {
                    s.readiness.as_mut().unwrap().skills.push(SkillReadiness {
                        skill: def("direct"),
                        parameters: DeclaredSet::complete(vec![]),
                        participation: None,
                    });
                }
            },
            |r| {
                r.schema_version = 3;
                r.source_properties = Some(SourcePropertyPreparationInput {
                    relations: DeclaredSet::complete(vec![]),
                });
            },
        )
    }
    fn plan(&self) -> Checked<Effects> {
        self.inputs(|_| {}).and_then(compile_inputs)
    }
    fn group(&mut self) -> &mut ContributionGroup {
        &mut self.queries.members[0].groups[0]
    }
    fn producer(&mut self) -> &mut RuleProgram {
        program_mut(&mut self.f, child_owner(), "query-producer")
    }
    fn consumer(&mut self) -> &mut RuleProgram {
        program_mut(&mut self.f, child_owner(), "query-consumer")
    }
}
fn value(report: &OwnedEffectsReport, target: SkillTarget) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    stat: def("skill-total"),
                }
        })
        .unwrap()
        .value
}
fn evaluate(p: &Effects) -> OwnedEffectsReport {
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    delivery::evaluated(&report).clone()
}
fn rejected<T>(r: Checked<T>, needle: &str) {
    let Err(error) = r else {
        panic!("expected {needle}")
    };
    assert!(error.contains(needle), "expected {needle:?}, got {error:?}");
}

#[test]
fn generated_copies_and_sibling_slots_keep_independent_values_and_sources() {
    let w = World::new();
    let p = w.plan().unwrap();
    let r = evaluate(&p);
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    for (root, expected) in [(30, 43), (31, 47)] {
        for name in ["first", "second"] {
            assert_eq!(
                *value(&r, target(root, name)),
                EffectValue::Known {
                    value: base::integer(expected)
                }
            );
        }
    }
    let effects: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("query-producer"))
        .collect();
    assert_eq!(
        effects.len(),
        8,
        "two effects for each of four distinct Skill occurrences"
    );
    for row in effects {
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!()
        };
        assert_eq!(
            provider.grant_path.len(),
            3,
            "entered provider includes the final ability grant"
        );
    }
}

#[test]
fn reused_parallel_and_permuted_storage_agree() {
    let w = World::new();
    let a = w.plan().unwrap();
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    let mut b = World::new();
    b.producer().nodes[1] = base::literal("bonus", 9);
    let b = b.plan().unwrap();
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    assert_ne!(b.evaluate(&mut scratch).unwrap(), expected);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..16)
                .into_par_iter()
                .for_each(|_| assert_eq!(a.evaluate(&mut a.new_scratch()).unwrap(), expected));
        });
    let mut reversed = World::new();
    reversed.f.build.skills.reverse();
    reversed.f.build.gems.reverse();
    reversed.f.owners.reverse();
    reversed.group().members.members.reverse();
    for m in &mut reversed.group().members.members {
        let ContributionOrigin::Skill { supplies, .. } = &mut m.origin else {
            panic!()
        };
        supplies.reverse();
    }
    let r = evaluate(&reversed.plan().unwrap());
    let original = delivery::evaluated(&expected);
    for root in [30, 31] {
        for name in ["first", "second"] {
            assert_eq!(
                value(&r, target(root, name)),
                value(original, target(root, name))
            );
        }
    }
}

#[test]
fn storage_roundtrip_and_new_authority_are_explicit() {
    let w = World::new();
    let inputs = w.inputs(|_| {}).unwrap();
    let raw = inputs.rules.input();
    let stored =
        OwnedRulePackage::new(raw.clone(), inputs.definitions.as_ref(), Default::default())
            .unwrap();
    let bytes = encode_rule_package(&stored, Default::default()).unwrap();
    let decoded =
        decode_rule_package(&bytes, inputs.definitions.as_ref(), Default::default()).unwrap();
    assert_eq!(decoded.input(), raw);
    assert_eq!(decoded.identity(), stored.identity());
    rejected(
        w.inputs(|r| r.operations_version = key(OWNED_RULE_OPERATIONS_V23)),
        "Skill contribution origins require",
    );
    let mut empty = World::new();
    empty.producer().effects.clear();
    empty.group().members.members.clear();
    rejected(
        empty.inputs(|r| r.operations_version = key(OWNED_RULE_OPERATIONS_V23)),
        "unsupported relative recipient scope",
    );
    for entity in [RuleEntity::PropertyOwner, RuleEntity::Skill] {
        let mut bad = World::new();
        let RuleReadSource::ContributionQuery { entity: actual, .. } =
            &mut bad.consumer().reads[0].source
        else {
            panic!()
        };
        *actual = entity;
        rejected(bad.inputs(|_| {}), "unsupported relative recipient scope");
    }
}

#[test]
fn membership_rejects_wrong_supply_owner_context_ranks_and_recipient() {
    for case in 0..8 {
        let mut w = World::new();
        let expected = match case {
            0 => {
                w.group().members.members[0].origin = ContributionOrigin::Skill {
                    authored: false,
                    supplies: vec![],
                };
                "explicit supply membership"
            }
            1 => {
                w.group().members.members[0].origin = ContributionOrigin::Skill {
                    authored: false,
                    supplies: vec![ability_supply("first"), ability_supply("first")],
                };
                "duplicate supplied Skill"
            }
            2 => {
                w.group().members.members[0].origin = ContributionOrigin::Skill {
                    authored: false,
                    supplies: vec![summon_supply()],
                };
                "owner differs from its slot"
            }
            3 => {
                w.group().members.members[0].origin = ContributionOrigin::Skill {
                    authored: false,
                    supplies: vec![ability_supply("missing")],
                };
                "slot must be known"
            }
            4 => {
                w.producer().context = RuleEntityKind::Actor;
                "require Skill context"
            }
            5 => {
                w.group().members.members[0]
                    .order
                    .as_mut()
                    .unwrap()
                    .slot_ranks
                    .push(ContributionSlotRank {
                        slot: def("weapon"),
                        rank: 0,
                    });
                "no equipment ranks"
            }
            6 => {
                let RuleEffectKind::Contribute { entity, .. } = &mut w.producer().effects[0].effect
                else {
                    panic!()
                };
                *entity = RuleEntity::Player;
                "only the exact current Skill recipient"
            }
            _ => {
                w.group().members.members[0].origin = ContributionOrigin::Skill {
                    authored: true,
                    supplies: vec![],
                };
                "directly selectable Skill"
            }
        };
        rejected(w.inputs(|_| {}), expected);
    }
}

#[test]
fn unread_inactive_and_neutral_effects_still_require_exact_membership() {
    for case in 0..3 {
        let mut w = World::new();
        w.f.owner_mut(&child_owner())
            .programs
            .members
            .retain(|p| p.id != key("query-consumer"));
        if case == 1 {
            w.producer().nodes.push(base::bool_node("disabled", false));
            w.producer().effects[1].when = Some(key("disabled"));
        } else if case == 2 {
            w.producer().nodes[1] = base::literal("bonus", 0);
        }
        w.group().members.members.pop();
        rejected(w.plan(), "actual contribution has no declared membership");
    }
    let mut w = World::new();
    for m in &mut w.group().members.members {
        m.origin = ContributionOrigin::Skill {
            authored: false,
            supplies: vec![ability_supply("first")],
        };
    }
    rejected(w.plan(), "differs from the validated skill supply");
    let mut w = World::new();
    w.group().members.members[1]
        .order
        .as_mut()
        .unwrap()
        .effect_rank = 0;
    rejected(w.plan(), "semantic positions are tied");
}

#[test]
fn empty_identity_requires_complete_coverage_and_never_masks_missing_values() {
    let mut w = World::new();
    w.producer().effects.clear();
    w.group().members.members.clear();
    let r = evaluate(&w.plan().unwrap());
    assert!(r.gaps.is_empty());
    assert_eq!(
        *value(&r, target(30, "first")),
        EffectValue::Known {
            value: base::integer(0)
        }
    );
    w.group().members.closure = partial(
        subject(def::<StatDefinition>("skill-channel")),
        SchemaFacet::GameRules,
    );
    let p = w.plan().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(!r.gaps.is_empty());
    assert!(matches!(
        r.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut w = World::new();
    remove_projection(&mut w.f, actor_owner(), "assemble-first", "final-level");
    let p = w.plan().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    match r.outcome {
        SupportEffectsOutcome::Unavailable { .. } => {}
        SupportEffectsOutcome::Evaluated { effects } => assert!(!matches!(
            value(&effects, target(30, "first")),
            EffectValue::Known { .. }
        )),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn late_support_delivery_cannot_enter_a_skill_query_by_provider_ancestry() {
    let w = World::new();
    let input = w
        .inputs(|r| {
            let owner = r
                .owners
                .iter_mut()
                .find(|o| o.owner == delivery::support_owner())
                .unwrap();
            let program = owner
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("actor-deliver"))
                .unwrap();
            program.nodes.push(base::literal("unregistered", 0));
            program.effects.push(effect(
                "unregistered",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def("skill-channel"),
                    contribution: ContributionKind::Add,
                    value: key("unregistered"),
                },
            ));
        })
        .unwrap();
    let plan = compile_inputs(input).unwrap();
    let result = plan.evaluate(&mut plan.new_scratch());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("actual contribution has no declared membership")
    );
}

#[test]
fn direct_authored_copies_use_explicit_permission_and_preserve_each_occurrence() {
    let mut w = World::new();
    let direct = def::<SkillDefinition>("direct");
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Skill(known(
            direct.clone(),
            SkillSchema {
                directly_selectable: true,
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            },
        )));
    let mut producer = w.producer().clone();
    producer.reads.clear();
    producer.nodes[0] = base::literal("level", 7);
    let consumer = w.consumer().clone();
    w.f.owners.push(DefinitionRules {
        owner: subject(direct.clone()),
        programs: DeclaredSet::complete(vec![producer, consumer]),
    });
    for id in [90, 91] {
        w.f.build.skills.push(SkillUse {
            id: occurrence(id),
            source: AuthoredSkillSource::Direct(direct.clone()),
            enabled: true,
            scope: LoadoutScope::Shared,
            parameters: None,
        });
        w.f.build
            .authored_support_order
            .as_mut()
            .unwrap()
            .push(AuthoredSupportOrder {
                target: SkillTarget::Authored(occurrence(id)),
                assignments: vec![],
            });
    }
    for (name, rank) in [("level", 0), ("bonus", 1)] {
        let mut row = member(name, rank);
        row.owner = subject(direct.clone());
        row.origin = ContributionOrigin::Skill {
            authored: true,
            supplies: vec![],
        };
        row.order.as_mut().unwrap().source_rank = 1;
        w.group().members.members.push(row);
    }
    let r = evaluate(&w.plan().unwrap());
    for id in [90, 91] {
        assert_eq!(
            *value(&r, SkillTarget::Authored(occurrence(id))),
            EffectValue::Known {
                value: base::integer(10)
            }
        );
    }
    assert_eq!(
        *value(&r, target(30, "first")),
        EffectValue::Known {
            value: base::integer(43)
        }
    );
    // The same definition/program/effects can also serve item-generated copies.
    // This is one membership row with explicit permissions, not duplicated rules.
    let supply = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def("direct-supply"),
    };
    let grant = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def("direct-grant"),
    };
    for row in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(row) = row
            && row.id == def::<ItemTemplateDefinition>("item")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema
                .declarations
                .skill_grants
                .members
                .push(supply.clone());
            schema.declarations.grants.members.push(grant.clone());
        }
    }
    w.f.schema.slots.extend([
        SlotDescriptor::SkillGrant(known(
            supply.clone(),
            SkillGrantSlotSchema {
                skill: direct.clone(),
                outputs: DeclaredSet::complete(vec![]),
                preset_inputs: None,
            },
        )),
        SlotDescriptor::Grant(known(
            grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::EquipmentUse],
                target: GrantTarget::Skill(supply.clone()),
            },
        )),
    ]);
    w.f.owner_mut(&subject(def::<ItemTemplateDefinition>("item")))
        .programs
        .members
        .push(RuleProgram {
            id: key("direct-supply"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![base::bool_node("enabled", true)],
            effects: vec![effect(
                "activate",
                RuleEffectKind::ActivateGrant {
                    slot: grant,
                    enabled: key("enabled"),
                },
            )],
        });
    let original = Fixture::new();
    w.f.build.items = original.build.items;
    w.f.build.equipment = original.build.equipment;
    let supplied: Vec<_> =
        w.f.build
            .equipment
            .iter()
            .map(|usage| {
                SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::EquipmentUse(usage.id),
                        grant_path: vec![],
                    },
                    slot: supply.clone(),
                }))
            })
            .collect();
    assert_eq!(supplied.len(), 2);
    for target in &supplied {
        w.f.build
            .authored_support_order
            .as_mut()
            .unwrap()
            .push(AuthoredSupportOrder {
                target: target.clone(),
                assignments: vec![],
            });
    }
    for row in w
        .group()
        .members
        .members
        .iter_mut()
        .filter(|m| m.owner == subject(direct.clone()))
    {
        row.origin = ContributionOrigin::Skill {
            authored: true,
            supplies: vec![supply.clone()],
        };
    }
    let r = evaluate(&w.plan().unwrap());
    for target in supplied
        .iter()
        .cloned()
        .chain([90, 91].map(|id| SkillTarget::Authored(occurrence(id))))
    {
        assert_eq!(
            *value(&r, target),
            EffectValue::Known {
                value: base::integer(10)
            }
        );
    }
    // An admitted supply never silently permits the direct uses of that definition.
    for row in w
        .group()
        .members
        .members
        .iter_mut()
        .filter(|m| m.owner == subject(direct.clone()))
    {
        row.origin = ContributionOrigin::Skill {
            authored: false,
            supplies: vec![supply.clone()],
        };
    }
    rejected(w.plan(), "authored Skill contribution requires permission");
}

#[test]
fn boolean_any_keeps_skill_identity_and_does_not_turn_unknown_into_false() {
    let mut w = World::new();
    for row in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::Stat(row) = row
            && [def::<StatDefinition>("skill-channel"), def("skill-total")].contains(&row.id)
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.value = ComputedValueType::Boolean;
        }
    }
    let p = w.producer();
    p.nodes[1] = base::bool_node("bonus", false);
    p.nodes.push(base::literal("zero", 0));
    p.nodes.push(base::node(
        "positive",
        RuleExpression::Compare {
            operation: RuleComparison::Greater,
            left: key("level"),
            right: key("zero"),
        },
    ));
    for (i, effect) in p.effects.iter_mut().enumerate() {
        let RuleEffectKind::Contribute {
            contribution,
            value,
            ..
        } = &mut effect.effect
        else {
            panic!()
        };
        *contribution = ContributionKind::Flag;
        if i == 0 {
            *value = key("positive");
        }
    }
    w.consumer().reads[0].value_type = ComputedValueType::Boolean;
    w.queries.members[0].contribution = ContributionKind::Flag;
    let group = w.group();
    group.reduction = ContributionReduction::Any;
    group.ordering = ContributionOrdering::Unordered;
    group.empty = ParameterValue::Boolean(false);
    for member in &mut group.members.members {
        member.order = None;
    }
    let r = evaluate(&w.plan().unwrap());
    for root in [30, 31] {
        for name in ["first", "second"] {
            assert_eq!(
                *value(&r, target(root, name)),
                EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                }
            );
        }
    }
    w.group().members.closure = partial(
        subject(def::<StatDefinition>("skill-channel")),
        SchemaFacet::GameRules,
    );
    let p = w.plan().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}

#[test]
fn raw_compilation_target_types_and_limits_preserve_the_checked_boundary() {
    let w = World::new();
    let input = w.inputs(|_| {}).unwrap();
    let mut raw = input.rules.input().clone();
    let ContributionOrigin::Skill { supplies, .. } =
        &mut raw.contribution_queries.as_mut().unwrap().members[0].groups[0]
            .members
            .members[0]
            .origin
    else {
        panic!()
    };
    *supplies = vec![summon_supply()];
    let result = poe_optimizer_engine::owned_rules::CompiledRulePackage::compile(
        &raw,
        input.definitions.as_ref(),
        Default::default(),
    );
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("owner differs from its slot")
    );
    let limits = poe_optimizer_data::owned_rules::RuleStorageLimits {
        max_ordered_slots: 1,
        ..Default::default()
    };
    assert!(
        OwnedRulePackage::new(
            input.rules.input().clone(),
            input.definitions.as_ref(),
            limits
        )
        .is_err()
    );
    let mut w = World::new();
    w.producer().effects.clear();
    w.group().members.members.clear();
    for row in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::Stat(row) = row
            && row.id == def::<StatDefinition>("skill-channel")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.targets = vec![RuleEntityKind::Actor];
        }
    }
    rejected(
        w.inputs(|_| {}),
        "read recipient is not admitted by its stat",
    );
}
