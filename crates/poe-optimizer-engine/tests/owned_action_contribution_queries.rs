//! Exact Action reductions use the same native graph as other recipients.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_source_properties::*,
};
use poe_optimizer_data::owned_rules::{OwnedRulePackage, decode_rule_package, encode_rule_package};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::collections::BTreeMap;
use support::delivery::fixture as base;
use support::*;

fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn output_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::ActionOutput(delivery::output()))
}
fn member(
    owner: SchemaSubject,
    program: &str,
    effect: &str,
    source: u32,
    rank: u32,
) -> ContributionMember {
    ContributionMember {
        producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner,
            program: key(program),
            effect: key(effect),
            origin: ContributionOrigin::Action {
                authored: false,
                supplies: vec![ability_supply("first"), ability_supply("second")],
            },
        }),
        order: Some(ContributionOrder {
            source_rank: source,
            program_rank: 0,
            effect_rank: rank,
            slot_ranks: vec![],
        }),
    }
}
fn contribute(name: &str) -> RuleEffect {
    effect(
        name,
        RuleEffectKind::Contribute {
            entity: RuleEntity::Current,
            stat: def("action-channel"),
            contribution: ContributionKind::Add,
            value: key(name),
        },
    )
}
struct World {
    f: Fixture,
    queries: DeclaredSet<ContributionQuery>,
}
impl World {
    fn new() -> Self {
        let mut f = fixture();
        for name in ["action-channel", "action-total"] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Action],
                },
            )));
        }
        f.schema.definitions.extend([
            DefinitionDescriptor::ActionMode(known(def("mode-two"), ActionModeSchema {})),
            DefinitionDescriptor::ActionStatSet(known(def("set-two"), ActionStatSetSchema {})),
        ]);
        for slot in &mut f.schema.slots {
            if let SlotDescriptor::ActionOutput(row) = slot
                && row.id == delivery::output()
                && let SchemaState::Known(schema) = &mut row.schema
            {
                schema.modes.members.push(def("mode-two"));
                schema.stat_sets.members.push(def("set-two"));
            }
        }
        let axes = [
            (
                "part",
                RuleReadSource::ActionPartIs { part: def("part") },
                10,
            ),
            (
                "mode",
                RuleReadSource::ActionModeIs { mode: def("mode") },
                100,
            ),
            (
                "set",
                RuleReadSource::ActionStatSetIs {
                    stat_set: def("set"),
                },
                1000,
            ),
        ];
        let mut producer = RuleProgram {
            id: key("action-producer"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![base::literal("zero", 0)],
            effects: vec![],
        };
        for (name, source, amount) in axes {
            let condition = format!("{name}-is");
            let positive = format!("{name}-positive");
            producer.reads.push(RuleRead {
                id: key(&condition),
                value_type: ComputedValueType::Boolean,
                source,
            });
            producer.nodes.extend([
                base::read_node(&condition, &condition),
                base::literal(&positive, amount),
                base::node(
                    name,
                    RuleExpression::Select {
                        condition: key(&condition),
                        when_true: key(&positive),
                        when_false: key("zero"),
                    },
                ),
            ]);
            producer.effects.push(contribute(name));
        }
        f.owner_mut(&output_owner()).programs.members.extend([
            producer,
            RuleProgram {
                id: key("action-consumer"),
                context: RuleEntityKind::Action,
                reads: vec![base::read(
                    "sum",
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Current,
                        query: key("action-query"),
                        group: key("self"),
                    },
                )],
                nodes: vec![base::read_node("sum", "sum")],
                effects: vec![base::derive(
                    "total",
                    RuleEntity::Current,
                    "action-total",
                    "sum",
                )],
            },
        ]);
        // Definition-owned Action programs and output-owned programs may share
        // a channel, while retaining separate declared semantic positions.
        f.owner_mut(&child_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("definition-producer"),
                context: RuleEntityKind::Action,
                reads: vec![],
                nodes: vec![base::literal("base", 7)],
                effects: vec![contribute("base")],
            });
        let mut members = vec![member(child_owner(), "definition-producer", "base", 0, 0)];
        members.extend(
            ["part", "mode", "set"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| member(output_owner(), "action-producer", name, 1, i as u32)),
        );
        Self {
            f,
            queries: DeclaredSet::complete(vec![ContributionQuery {
                id: key("action-query"),
                stat: def("action-channel"),
                contribution: ContributionKind::Add,
                groups: vec![ContributionGroup {
                    id: key("self"),
                    reduction: ContributionReduction::Sum,
                    ordering: ContributionOrdering::Ordered,
                    empty: base::integer(0),
                    members: DeclaredSet::complete(members),
                }],
            }]),
        }
    }
    fn inputs(&self, edit: impl FnOnce(&mut RulePackageInput)) -> Checked<Inputs> {
        inputs_with_operations(
            &self.f,
            false,
            OWNED_RULE_OPERATIONS_V26,
            |r| {
                r.existing_actor_rules = Some(DeclaredSet::complete(vec![]));
                r.contribution_queries = Some(self.queries.clone());
                edit(r);
            },
            |s| {
                s.schema_version = 3;
                for row in &mut s.readiness.as_mut().unwrap().programs.members {
                    if row.program == key("authored-supply") {
                        row.phase = ReadinessPhase::Structural;
                        row.role = ReadinessProgramRole::PreparationFacts;
                        row.outputs = vec![poe_optimizer_core::owned_stages::StageChannel::Grant {
                            slot: authored_grant(),
                        }];
                        s.programs
                            .members
                            .iter_mut()
                            .find(|p| p.owner == row.owner && p.program == row.program)
                            .unwrap()
                            .stage = key("prepare");
                    }
                }
                if self.f.schema.definitions.iter().any(|d| matches!(d, DefinitionDescriptor::Skill(row) if row.id == def::<SkillDefinition>("authored"))) {
                    s.readiness.as_mut().unwrap().skills.push(SkillReadiness {
                        skill: def("authored"), parameters: DeclaredSet::complete(vec![]), participation: None,
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
        program_mut(&mut self.f, output_owner(), "action-producer")
    }
    fn consumer(&mut self) -> &mut RuleProgram {
        program_mut(&mut self.f, output_owner(), "action-consumer")
    }
    fn clear_producers(&mut self) {
        self.producer().effects.clear();
        program_mut(&mut self.f, child_owner(), "definition-producer")
            .effects
            .clear();
        self.group().members.members.clear();
    }
}
fn rejected<T>(result: Checked<T>, needle: &str) {
    let Err(error) = result else {
        panic!("expected {needle}")
    };
    assert!(error.contains(needle), "expected {needle:?}, got {error:?}");
}
fn projection(report: &SupportEffectsReport) -> BTreeMap<ActionSelection, EffectValue> {
    delivery::evaluated(report)
        .values
        .iter()
        .filter_map(|row| {
            let PlanValueKey::Stat {
                entity: ConcreteEntity::Action(action),
                stat,
            } = &row.key
            else {
                return None;
            };
            (stat == &def::<StatDefinition>("action-total"))
                .then(|| (action.as_ref().clone(), row.value.clone()))
        })
        .collect()
}
fn expected(action: &ActionSelection) -> i64 {
    7 + 10 * i64::from(action.part == def("part"))
        + 100 * i64::from(action.mode == def("mode"))
        + 1000 * i64::from(action.stat_set == def("set"))
}

#[test]
fn generated_occurrences_and_every_selection_axis_keep_their_own_contributions() {
    let w = World::new();
    let p = w.plan().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    let values = projection(&report);
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(values.len(), 3 * 2 * 2 * 2);
    for (action, value) in &values {
        assert_eq!(
            *value,
            EffectValue::Known {
                value: base::integer(expected(action))
            }
        );
        assert_eq!(action.action.provider.grant_path.len(), 3);
    }
    let mut subset = World::new();
    subset.f.queries.requests.truncate(1);
    let p = subset.plan().unwrap();
    assert_eq!(
        projection(&p.evaluate(&mut p.new_scratch()).unwrap()),
        values
    );
}

#[test]
fn fresh_reused_unknown_changed_restored_and_four_workers_agree() {
    let a = World::new().plan().unwrap();
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    let mut changed = World::new();
    changed.producer().nodes[0] = base::literal("zero", 4);
    let b = changed.plan().unwrap();
    let mut unknown = World::new();
    unknown.group().members.closure = partial(
        subject(def::<StatDefinition>("action-channel")),
        SchemaFacet::GameRules,
    );
    let unknown = unknown.plan().unwrap();
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    assert!(matches!(
        unknown.evaluate(&mut scratch).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
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
        let ContributionOrigin::Action { supplies, .. } =
            &mut m.producer.as_program_effect_mut().unwrap().origin
        else {
            panic!()
        };
        supplies.reverse();
    }
    let p = reversed.plan().unwrap();
    assert_eq!(
        projection(&p.evaluate(&mut p.new_scratch()).unwrap()),
        projection(&expected)
    );
}

#[test]
fn storage_roundtrip_and_capability_gate_are_checked() {
    let w = World::new();
    let input = w.inputs(|_| {}).unwrap();
    let stored = OwnedRulePackage::new(
        input.rules.input().clone(),
        input.definitions.as_ref(),
        Default::default(),
    )
    .unwrap();
    let bytes = encode_rule_package(&stored, Default::default()).unwrap();
    let decoded =
        decode_rule_package(&bytes, input.definitions.as_ref(), Default::default()).unwrap();
    assert_eq!(decoded.input(), stored.input());
    assert_eq!(decoded.identity(), stored.identity());
    rejected(
        w.inputs(|r| r.operations_version = key(OWNED_RULE_OPERATIONS_V25)),
        "Action contribution origins require",
    );
    let mut empty = World::new();
    empty.clear_producers();
    rejected(
        empty.inputs(|r| r.operations_version = key(OWNED_RULE_OPERATIONS_V25)),
        "unsupported relative recipient scope",
    );
}

#[test]
fn potential_membership_is_required_even_for_unread_inactive_zero_and_unselected_writers() {
    for case in 0..4 {
        let mut w = World::new();
        w.f.owner_mut(&output_owner())
            .programs
            .members
            .retain(|p| p.id != key("action-consumer"));
        if case == 1 {
            w.producer().nodes.push(base::bool_node("disabled", false));
            w.producer().effects[2].when = Some(key("disabled"));
        } else if case == 2 {
            w.producer().effects[2].effect = RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def("action-channel"),
                contribution: ContributionKind::Add,
                value: key("zero"),
            };
        } else if case == 3 {
            w.f.queries.requests.clear();
            w.f.build.supports.clear();
            for order in w.f.build.authored_support_order.as_mut().unwrap() {
                order.assignments.clear();
            }
        }
        w.group().members.members.pop();
        rejected(
            w.inputs(|_| {}),
            "potential Action contribution has no declared membership",
        );
    }
    let mut w = World::new();
    let duplicate = w.group().members.members[0].clone();
    w.group().members.members.push(duplicate);
    rejected(w.inputs(|_| {}), "occurs more than once");
}

#[test]
fn source_authority_rejects_invalid_supply_context_recipient_and_order() {
    for case in 0..8 {
        let mut w = World::new();
        let expected = match case {
            0 => {
                w.group().members.members[1]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .origin = ContributionOrigin::Action {
                    authored: false,
                    supplies: vec![],
                };
                "explicit supply membership"
            }
            1 => {
                w.group().members.members[1]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .origin = ContributionOrigin::Action {
                    authored: false,
                    supplies: vec![ability_supply("first"), ability_supply("first")],
                };
                "duplicate supplied Action"
            }
            2 => {
                w.group().members.members[1]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .origin = ContributionOrigin::Action {
                    authored: false,
                    supplies: vec![summon_supply()],
                };
                "owner differs from its Skill slot"
            }
            3 => {
                w.group().members.members[1]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .origin = ContributionOrigin::Action {
                    authored: false,
                    supplies: vec![ability_supply("missing")],
                };
                "slot must be known"
            }
            4 => {
                w.producer().context = RuleEntityKind::Actor;
                "action selection reads require Action context"
            }
            5 => {
                w.group().members.members[1]
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
                "only the exact current Action recipient"
            }
            _ => {
                w.group().members.members[2]
                    .order
                    .as_mut()
                    .unwrap()
                    .effect_rank = 0;
                "semantic positions are tied"
            }
        };
        rejected(w.plan(), expected);
    }
    let mut w = World::new();
    for member in &mut w.group().members.members {
        member.producer.as_program_effect_mut().unwrap().origin = ContributionOrigin::Action {
            authored: false,
            supplies: vec![ability_supply("first")],
        };
    }
    rejected(w.plan(), "validated Skill supply membership");
}

#[test]
fn complete_empty_is_zero_but_missing_and_partial_values_remain_unavailable() {
    let mut w = World::new();
    w.clear_producers();
    let p = w.plan().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(projection(&report).len(), 24);
    assert!(projection(&report).values().all(|v| *v
        == EffectValue::Known {
            value: base::integer(0)
        }));
    w.queries.closure = partial(
        subject(def::<StatDefinition>("action-channel")),
        SchemaFacet::GameRules,
    );
    let p = w.plan().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut w = World::new();
    w.producer().reads.push(base::read(
        "missing",
        RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("action-channel"),
        },
    ));
    w.producer().nodes[0] = base::read_node("zero", "missing");
    let p = w.plan().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    match &report.outcome {
        SupportEffectsOutcome::Unavailable { .. } => {}
        SupportEffectsOutcome::Evaluated { .. } => assert!(
            projection(&report)
                .values()
                .any(|v| !matches!(v, EffectValue::Known { .. }))
        ),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn raw_compilation_and_bounded_validation_cannot_bypass_membership() {
    let w = World::new();
    let input = w.inputs(|_| {}).unwrap();
    let mut raw = input.rules.input().clone();
    raw.contribution_queries.as_mut().unwrap().members[0].groups[0]
        .members
        .members
        .pop();
    let error = poe_optimizer_engine::owned_rules::CompiledRulePackage::compile(
        &raw,
        input.definitions.as_ref(),
        Default::default(),
    )
    .err()
    .unwrap();
    assert!(
        error
            .to_string()
            .contains("potential Action contribution has no declared membership")
    );
    for limits in [
        poe_optimizer_data::owned_rules::RuleStorageLimits {
            max_ordered_slots: 1,
            ..Default::default()
        },
        poe_optimizer_data::owned_rules::RuleStorageLimits {
            max_ordered_work: 1,
            ..Default::default()
        },
    ] {
        assert!(
            OwnedRulePackage::new(
                input.rules.input().clone(),
                input.definitions.as_ref(),
                limits
            )
            .is_err()
        );
    }
}

fn authored_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("authored-gem")),
        slot: def("authored-grant"),
    }
}
fn authored_supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("authored-gem")),
        slot: def("authored-supply"),
    }
}
fn authored_world(gem: bool) -> World {
    let mut w = World::new();
    let skill = def::<SkillDefinition>("authored");
    let outputs: Vec<_> = ["one", "two"]
        .map(|name| DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(skill.clone()),
            slot: def(name),
        })
        .into();
    let ports = DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    };
    let mut skill_ports = ports.clone();
    skill_ports.outputs.members = outputs.clone();
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Skill(known(
            skill.clone(),
            SkillSchema {
                directly_selectable: !gem,
                declarations: skill_ports,
            },
        )));
    w.f.owners.push(DefinitionRules {
        owner: subject(skill.clone()),
        programs: DeclaredSet::complete(vec![]),
    });
    if gem {
        let mut gem_ports = ports;
        gem_ports.grants.members.push(authored_grant());
        gem_ports.skill_grants.members.push(authored_supply());
        w.f.schema.definitions.push(DefinitionDescriptor::Gem(known(
            def("authored-gem"),
            GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(20).unwrap(),
                },
                roles: vec![AuthoredGemRole::SkillUse],
                skills: DeclaredSet::complete(vec![skill.clone()]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: gem_ports,
            },
        )));
        w.f.owners.push(DefinitionRules {
            owner: subject(def::<GemDefinition>("authored-gem")),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("authored-supply"),
                context: RuleEntityKind::Skill,
                reads: vec![],
                nodes: vec![base::bool_node("enabled", true)],
                effects: vec![effect(
                    "grant",
                    RuleEffectKind::ActivateGrant {
                        slot: authored_grant(),
                        enabled: key("enabled"),
                    },
                )],
            }]),
        });
        w.f.schema.slots.extend([
            SlotDescriptor::SkillGrant(known(
                authored_supply(),
                SkillGrantSlotSchema {
                    skill: skill.clone(),
                    outputs: DeclaredSet::complete(outputs.clone()),
                    preset_inputs: None,
                },
            )),
            SlotDescriptor::Grant(known(
                authored_grant(),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(authored_supply()),
                },
            )),
        ]);
        w.f.owners.extend([
            DefinitionRules {
                owner: SchemaSubject::Slot(SlotAddress::SkillGrant(authored_supply())),
                programs: DeclaredSet::complete(vec![]),
            },
            DefinitionRules {
                owner: SchemaSubject::Slot(SlotAddress::Grant(authored_grant())),
                programs: DeclaredSet::complete(vec![]),
            },
        ]);
    }
    for row in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::Metric(row) = row
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.actor_roles.push(MetricActorRole::Player);
        }
    }
    for (i, output) in outputs.iter().enumerate() {
        w.f.schema.slots.push(SlotDescriptor::ActionOutput(known(
            output.clone(),
            ActionOutputSchema {
                actor_role: DeclaredActorRole::ProviderActor,
                parts: DeclaredSet::complete(vec![def("part")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("set")]),
                choices: DeclaredSet::complete(vec![]),
            },
        )));
        let owner = SchemaSubject::Slot(SlotAddress::ActionOutput(output.clone()));
        let consumer = w.consumer().clone();
        w.f.owners.push(DefinitionRules {
            owner: owner.clone(),
            programs: DeclaredSet::complete(vec![
                RuleProgram {
                    id: key("authored-producer"),
                    context: RuleEntityKind::Action,
                    reads: vec![],
                    nodes: vec![base::literal("amount", 11 + i as i64)],
                    effects: vec![contribute("amount")],
                },
                consumer,
            ]),
        });
        w.f.routes
            .push(poe_optimizer_core::owned_routing::ActionOutputRoutes {
                output: output.clone(),
                routes: DeclaredSet::complete(vec![]),
                source_selectors: Some(DeclaredSet::complete(vec![])),
            });
        let mut m = member(owner, "authored-producer", "amount", 2 + i as u32, 0);
        m.producer.as_program_effect_mut().unwrap().origin = ContributionOrigin::Action {
            authored: !gem,
            supplies: if gem { vec![authored_supply()] } else { vec![] },
        };
        w.group().members.members.push(m);
    }
    for id in [90, 91] {
        let source = if gem {
            w.f.build.gems.push(GemInstance {
                id: occurrence(id - 10),
                definition: def("authored-gem"),
                parameters: vec![],
                level: 1,
                quality: None,
            });
            AuthoredSkillSource::Gem(occurrence(id - 10))
        } else {
            AuthoredSkillSource::Direct(skill.clone())
        };
        w.f.build.skills.push(SkillUse {
            id: occurrence(id),
            source,
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
        for output in &outputs {
            w.f.queries.requests.push(MetricRequest {
                id: QueryId::new(format!("{id}-{}", output.slot.key().as_str())).unwrap(),
                metric: def("requested"),
                target: MetricTarget::Action(Box::new(ActionSelection {
                    action: ActionKey {
                        actor: ActorKey::Player,
                        provider: ProviderKey {
                            root: ProviderRoot::SkillUse(occurrence(id)),
                            grant_path: if gem { vec![authored_grant()] } else { vec![] },
                        },
                        output: output.clone(),
                    },
                    part: def("part"),
                    mode: def("mode"),
                    stat_set: def("set"),
                })),
            });
        }
    }
    w
}

#[test]
fn authored_direct_and_gem_supplied_skills_preserve_copies_and_separate_outputs() {
    for gem in [false, true] {
        let w = authored_world(gem);
        let p = w.plan().unwrap();
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        let values = projection(&r);
        assert_eq!(values.len(), 28);
        for (action, value) in values {
            if action.action.output.declaration == SlotOwnerDefId::Skill(def("authored")) {
                let expected = match action.action.output.slot.key().as_str() {
                    "one" => 11,
                    "two" => 12,
                    _ => panic!(),
                };
                assert_eq!(
                    value,
                    EffectValue::Known {
                        value: base::integer(expected)
                    }
                );
            }
        }
    }
    let mut w = authored_world(true);
    for row in &mut w.f.queries.requests {
        if let MetricTarget::Action(action) = &mut row.target
            && action.action.output.declaration == SlotOwnerDefId::Skill(def("authored"))
        {
            action.action.provider.grant_path.clear();
        }
    }
    // A symbolically bindable Gem root never substitutes for its supplied child.
    match w.plan() {
        Err(_) => {}
        Ok(p) => assert!(matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Unavailable { .. }
        )),
    }
}

#[test]
fn cycles_and_reads_before_producer_stage_are_rejected() {
    let mut w = World::new();
    let read = w.consumer().reads[0].clone();
    w.producer().reads.push(read);
    w.producer().nodes[0] = base::read_node("zero", "sum");
    rejected(w.plan(), "cycle");
    let w = World::new();
    let result = inputs_with_operations(
        &w.f,
        false,
        OWNED_RULE_OPERATIONS_V26,
        |r| {
            r.existing_actor_rules = Some(DeclaredSet::complete(vec![]));
            r.contribution_queries = Some(w.queries.clone());
        },
        |s| {
            s.schema_version = 3;
            s.stages
                .push(poe_optimizer_core::owned_stages::EvaluationStage {
                    id: key("late"),
                    predecessors: vec![key("execute")],
                });
            s.programs
                .members
                .iter_mut()
                .find(|p| p.program == key("action-producer"))
                .unwrap()
                .stage = key("late");
        },
        |r| {
            r.schema_version = 3;
            r.source_properties = Some(SourcePropertyPreparationInput {
                relations: DeclaredSet::complete(vec![]),
            });
        },
    )
    .and_then(compile_inputs);
    rejected(result, "stage");
}

#[test]
fn support_delivery_is_not_authorized_as_an_action_self_contribution() {
    let w = World::new();
    let edited = |r: &mut RulePackageInput| {
        let p = package_program_mut(r, &delivery::support_owner(), "action-deliver");
        p.nodes.push(base::literal("unreviewed", 0));
        p.effects.push(effect(
            "unreviewed",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def("action-channel"),
                contribution: ContributionKind::Add,
                value: key("unreviewed"),
            },
        ));
    };
    rejected(
        w.inputs(edited),
        "potential Action contribution has no declared membership",
    );
    rejected(
        w.inputs(|r| {
            edited(r);
            let m = member(
                delivery::support_owner(),
                "action-deliver",
                "unreviewed",
                2,
                0,
            );
            r.contribution_queries.as_mut().unwrap().members[0].groups[0]
                .members
                .members
                .push(m);
        }),
        "requires a Skill or Action-output owner",
    );
}
