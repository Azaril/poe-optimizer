//! Exact source membership over shared actors, rewards and independently supplied actors.
#[path = "support/owned_empty_support_contribution_fixture.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, decode_rule_package, encode_rule_package},
    owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::sync::Arc;
use support::*;

fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn shared() -> SchemaSubject {
    subject(def::<ActorDefinition>("shared"))
}
fn supplied() -> SchemaSubject {
    subject(def::<ActorDefinition>("supplied"))
}
fn slot_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(child_slot()))
}
fn member(owner: SchemaSubject, origin: ContributionOrigin, rank: u32) -> ContributionMember {
    ContributionMember {
        owner,
        program: key("produce"),
        effect: key("add"),
        origin,
        order: Some(ContributionOrder {
            source_rank: rank,
            slot_ranks: vec![],
            program_rank: 0,
            effect_rank: 0,
        }),
    }
}
fn producer(source: RuleReadSource) -> RuleProgram {
    RuleProgram {
        id: key("produce"),
        context: RuleEntityKind::Actor,
        reads: vec![read("value", source)],
        nodes: vec![read_node("value", "value")],
        effects: vec![effect(
            "add",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def("channel"),
                contribution: ContributionKind::Add,
                value: key("value"),
            },
        )],
    }
}
fn fixed(n: i64) -> RuleProgram {
    let mut p = producer(RuleReadSource::CharacterLevel);
    p.reads.clear();
    p.nodes = vec![literal("value", n)];
    p
}
struct World {
    f: Fixture,
    queries: DeclaredSet<ContributionQuery>,
    applications: DeclaredSet<ExistingActorRuleApplication>,
}
impl World {
    fn new() -> Self {
        let mut f = Fixture::new();
        f.build.items.clear();
        f.build.equipment.clear();
        f.schema
            .definitions
            .retain(|d| !matches!(d, DefinitionDescriptor::Skill(_)));
        f.schema
            .slots
            .retain(|d| !matches!(d, SlotDescriptor::ActionOutput(_)));
        f.owners.retain(|o| {
            !matches!(
                o.owner,
                SchemaSubject::Definition(DefinitionAddress::Skill(_)) | SchemaSubject::Slot(_)
            )
        });
        f.schema.definitions.push(DefinitionDescriptor::Unit(known(
            def("percent"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )));
        f.owner_mut(&class_owner()).programs.members.clear();
        f.add_generated_actors();
        for name in ["shared", "supplied"] {
            f.schema.definitions.push(DefinitionDescriptor::Actor(known(
                def(name),
                ActorSchema {
                    declarations: ports(),
                },
            )));
        }
        for row in &mut f.schema.slots {
            if let SlotDescriptor::Actor(row) = row
                && row.id == child_slot()
            {
                let SchemaState::Known(s) = &mut row.schema else {
                    panic!()
                };
                s.provider_definition = Some(def("supplied"));
            }
        }
        for name in ["channel", "total"] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
        }
        f.owners.push(DefinitionRules {
            owner: shared(),
            programs: DeclaredSet::complete(vec![producer(RuleReadSource::CharacterLevel)]),
        });
        f.owners.push(DefinitionRules {
            owner: supplied(),
            programs: DeclaredSet::complete(vec![fixed(3)]),
        });
        f.owner_mut(&slot_owner()).programs.members = vec![producer(RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("child-level"),
        })];
        let supplied_origin = ContributionOrigin::SuppliedActor {
            slots: vec![child_slot()],
        };
        let mut members = vec![
            member(
                shared(),
                ContributionOrigin::ExistingActor {
                    application: key("shared-player"),
                },
                0,
            ),
            member(slot_owner(), supplied_origin.clone(), 0),
            member(supplied(), supplied_origin, 1),
        ];
        for (n, name, value) in [(40, "reward-a", 5), (41, "reward-b", 7)] {
            let reward = def::<RewardDefinition>(name);
            f.schema
                .definitions
                .push(DefinitionDescriptor::Reward(known(
                    reward.clone(),
                    RewardSchema {
                        declarations: ports(),
                    },
                )));
            f.build.character.rewards.push(RewardSelection {
                id: occurrence(n),
                definition: reward.clone(),
                parameters: vec![],
            });
            f.owners.push(DefinitionRules {
                owner: subject(reward.clone()),
                programs: DeclaredSet::complete(vec![fixed(value)]),
            });
            members.push(member(
                subject(reward),
                ContributionOrigin::Reward,
                n as u32,
            ));
        }
        let queries = DeclaredSet::complete(vec![ContributionQuery {
            id: key("query"),
            stat: def("channel"),
            contribution: ContributionKind::Add,
            groups: vec![ContributionGroup {
                id: key("base"),
                reduction: ContributionReduction::Sum,
                ordering: ContributionOrdering::Ordered,
                empty: integer(0),
                members: DeclaredSet::complete(members),
            }],
        }]);
        f.owners.push(DefinitionRules {
            owner: subject(def::<StatDefinition>("total")),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("total"),
                context: RuleEntityKind::Actor,
                reads: vec![read(
                    "sum",
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Current,
                        query: key("query"),
                        group: key("base"),
                    },
                )],
                nodes: vec![read_node("sum", "sum")],
                effects: vec![derive("total", RuleEntity::Current, "total", "sum")],
            }]),
        });
        f.receivers.members.push(StatReceiver {
            id: key("total"),
            stat: def("total"),
            program: key("total"),
            targets: vec![
                StatReceiverTarget::Player,
                StatReceiverTarget::OwnedSlot { slot: child_slot() },
            ],
        });
        Self {
            f,
            queries,
            applications: DeclaredSet::complete(vec![ExistingActorRuleApplication {
                id: key("shared-player"),
                owner: def("shared"),
                targets: vec![ExistingActorRuleTarget::Player],
            }]),
        }
    }
    fn schema(&self) -> Arc<OwnedDefinitionSchemaPackage> {
        Arc::new(
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap(),
        )
    }
    fn input(&self, s: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
        RulePackageInput {
            support_discovery: Some(SupportDiscoveryInput {
                providers: self
                    .f
                    .owners
                    .iter()
                    .map(|row| SupportSourceDomainDeclaration {
                        owner: row.owner.clone(),
                        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
                    })
                    .collect(),
            }),
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("test"),
            operations_version: key(OWNED_RULE_OPERATIONS_V23),
            definitions: s.identity().clone(),
            tables: self.f.tables.clone(),
            owners: self.f.owners.clone(),
            receivers: self.f.receivers.clone(),
            existing_actor_rules: Some(self.applications.clone()),
            contribution_queries: Some(self.queries.clone()),
            effect_applications: Some(DeclaredSet::complete(vec![])),
        }
    }
    fn plan(
        &self,
        limits: PlanLimits,
    ) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        let schema = self.schema();
        let input = self.input(&schema);
        empty_support::checked_plan(
            &self.f,
            input,
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routes"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            limits,
        )
    }
    fn report(&self) -> OwnedEffectsReport {
        let p = self.plan(Default::default()).unwrap();
        let report = p.evaluate(&mut p.new_scratch()).unwrap();
        let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
            panic!("{:?}", report.outcome)
        };
        effects
    }
    fn members(&mut self) -> &mut Vec<ContributionMember> {
        &mut self.queries.members[0].groups[0].members.members
    }
}
fn value(r: &OwnedEffectsReport, actor: ActorKey) -> &EffectValue {
    &r.values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: def("total"),
                }
        })
        .unwrap()
        .value
}
fn assert_values(r: &OwnedEffectsReport, player: i64, first: i64, second: i64) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    for (actor, expected) in [
        (ActorKey::Player, player),
        (child_actor(30), first),
        (child_actor(31), second),
    ] {
        assert_eq!(
            value(r, actor),
            &EffectValue::Known {
                value: integer(expected)
            }
        );
    }
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    }
}

fn boolean_world() -> World {
    let mut w = World::new();
    for d in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::Stat(row) = d
            && [def("channel"), def("total")].contains(&row.id)
        {
            let SchemaState::Known(s) = &mut row.schema else {
                panic!()
            };
            s.value = ComputedValueType::Boolean;
        }
    }
    for m in w.members().clone() {
        let p = &mut w.f.owner_mut(&m.owner).programs.members[0];
        p.reads.clear();
        p.nodes = vec![node(
            "value",
            RuleExpression::Literal {
                value: ParameterValue::Boolean(m.owner == shared()),
            },
        )];
        let RuleEffectKind::Contribute { contribution, .. } = &mut p.effects[0].effect else {
            panic!()
        };
        *contribution = ContributionKind::Flag;
    }
    w.f.owner_mut(&subject(def::<StatDefinition>("total")))
        .programs
        .members[0]
        .reads[0]
        .value_type = ComputedValueType::Boolean;
    let query = &mut w.queries.members[0];
    query.contribution = ContributionKind::Flag;
    query.groups[0].reduction = ContributionReduction::Any;
    query.groups[0].ordering = ContributionOrdering::Unordered;
    query.groups[0].empty = ParameterValue::Boolean(false);
    for m in &mut query.groups[0].members.members {
        m.order = None;
    }
    w
}

#[test]
fn boolean_duplicates_keep_occurrences_and_player_flags_do_not_flow_to_children() {
    let mut w = boolean_world();
    let mut duplicate = w.f.build.character.rewards[0].clone();
    duplicate.id = occurrence(42);
    w.f.build.character.rewards.push(duplicate);
    let r = w.report();
    assert!(r.gaps.is_empty());
    assert_eq!(
        value(&r, ActorKey::Player),
        &EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    for id in [30, 31] {
        assert_eq!(
            value(&r, child_actor(id)),
            &EffectValue::Known {
                value: ParameterValue::Boolean(false)
            }
        );
    }
    let rewards: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.owner == subject(def::<RewardDefinition>("reward-a")))
        .collect();
    assert_eq!(rewards.len(), 2);
    assert_ne!(
        rewards[0].key.invocation.origin,
        rewards[1].key.invocation.origin
    );
    w.f.build.character.rewards.reverse();
    w.members().reverse();
    let next = w.report();
    for actor in [ActorKey::Player, child_actor(30), child_actor(31)] {
        assert_eq!(value(&r, actor.clone()), value(&next, actor));
    }
}

#[test]
fn true_shared_flag_cannot_hide_unknown_reward_and_inactive_unlisted_sources() {
    let mut w = boolean_world();
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(known(
            def("missing"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Actor],
            },
        )));
    let p = &mut w
        .f
        .owner_mut(&subject(def::<RewardDefinition>("reward-a")))
        .programs
        .members[0];
    p.reads.push(RuleRead {
        id: key("missing"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("missing"),
        },
    });
    p.nodes = vec![read_node("value", "missing")];
    assert!(matches!(
        value(&w.report(), ActorKey::Player),
        EffectValue::Unresolved { .. }
    ));
    let p = &mut w
        .f
        .owner_mut(&subject(def::<RewardDefinition>("reward-a")))
        .programs
        .members[0];
    p.nodes.push(node(
        "disabled",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    ));
    p.effects[0].when = Some(key("disabled"));
    assert_eq!(
        value(&w.report(), ActorKey::Player),
        &EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    w.members()
        .retain(|m| m.owner != subject(def::<RewardDefinition>("reward-a")));
    assert!(
        w.plan(Default::default())
            .is_err_and(|e| e.to_string().contains("no declared membership"))
    );
}

#[test]
fn shared_rewards_and_supplied_slot_and_definition_owners_keep_exact_recipients() {
    let w = World::new();
    let r = w.report();
    assert_values(&r, 32, 14, 23);
    let contributions: Vec<_> = r.effects.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.stat == def("channel"))).collect();
    assert_eq!(contributions.len(), 7);
    assert_eq!(
        contributions
            .iter()
            .filter(|e| matches!(&e.key.invocation.origin, RuleOrigin::ExistingActor { .. }))
            .count(),
        1
    );
    // The child's identity stores the parent provider, whereas its rule origin
    // contains the entered grant. This must not be admitted by raw path equality.
    for c in contributions
        .iter()
        .filter(|e| e.key.invocation.owner == slot_owner())
    {
        let RuleOrigin::Provider { provider } = &c.key.invocation.origin else {
            panic!()
        };
        let ConcreteEntity::Actor(ActorKey::Owned(actor)) = &c.key.invocation.entity else {
            panic!()
        };
        assert_ne!(&actor.provider, provider);
        assert_eq!(provider.grant_path, [child_grant()]);
    }
}
#[test]
fn format_roundtrip_and_raw_compilation_authenticate_owner_origin_and_capability() {
    let w = World::new();
    let s = w.schema();
    let input = w.input(&s);
    let stored = OwnedRulePackage::new(input.clone(), s.as_ref(), Default::default()).unwrap();
    let encoded = encode_rule_package(&stored, Default::default()).unwrap();
    assert_eq!(
        decode_rule_package(&encoded, s.as_ref(), Default::default())
            .unwrap()
            .identity(),
        stored.identity()
    );
    for n in 0..8 {
        let mut bad = input.clone();
        let members = &mut bad.contribution_queries.as_mut().unwrap().members[0].groups[0]
            .members
            .members;
        match n {
            0 => bad.operations_version = key(OWNED_RULE_OPERATIONS_V22),
            1 => {
                members[0].origin = ContributionOrigin::ExistingActor {
                    application: key("absent"),
                }
            }
            2 => members[1].origin = ContributionOrigin::SuppliedActor { slots: vec![] },
            3 => members[2].origin = ContributionOrigin::Reward,
            4 => members[3].origin = ContributionOrigin::Character,
            5 => {
                members[1].origin = ContributionOrigin::SuppliedActor {
                    slots: vec![child_slot(), child_slot()],
                }
            }
            6 => members[0].origin = members[1].origin.clone(),
            _ => {
                bad.existing_actor_rules.as_mut().unwrap().members[0].owner = def("supplied");
            }
        }
        assert!(
            OwnedRulePackage::new(bad.clone(), s.as_ref(), Default::default()).is_err(),
            "case {n}"
        );
        assert!(
            CompiledRulePackage::compile(&bad, s.as_ref(), Default::default()).is_err(),
            "case {n}"
        );
    }
    assert_ne!(
        RuleOperationsVersion::V22.effect_plan_domain(),
        RuleOperationsVersion::V23.effect_plan_domain()
    );
}
#[test]
fn unread_inactive_and_tied_potential_sources_cannot_escape_membership() {
    let mut w = World::new();
    w.f.receivers.members.clear();
    w.members().remove(1);
    assert!(w.plan(Default::default()).is_err());
    let mut w = World::new();
    let mut duplicate = w.f.build.character.rewards[0].clone();
    duplicate.id = occurrence(42);
    w.f.build.character.rewards.push(duplicate);
    assert!(
        w.plan(Default::default())
            .is_err_and(|e| e.to_string().contains("positions are tied"))
    );
    let p = &mut w
        .f
        .owner_mut(&subject(def::<RewardDefinition>("reward-a")))
        .programs
        .members[0];
    p.nodes.push(node(
        "inactive",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    ));
    p.effects[0].when = Some(key("inactive"));
    assert!(
        w.plan(Default::default())
            .is_err_and(|e| e.to_string().contains("positions are tied"))
    );
    // Two different supplied occurrences contributing to Player need their own
    // semantic ordering law; they may not be ranked by their opaque IDs.
    let mut w = World::new();
    if let RuleEffectKind::Contribute { entity, .. } =
        &mut w.f.owner_mut(&supplied()).programs.members[0].effects[0].effect
    {
        *entity = RuleEntity::Player;
    }
    assert!(
        w.plan(Default::default())
            .is_err_and(|e| e.to_string().contains("positions are tied"))
    );
}
#[test]
fn partial_applicability_owners_and_query_membership_remain_unknown() {
    for n in 0..3 {
        let mut w = World::new();
        match n {
            0 => w.applications.closure = partial(shared()),
            1 => w.f.owner_mut(&supplied()).programs.closure = partial(supplied()),
            _ => {
                w.queries.members[0].groups[0].members.closure =
                    partial(subject(def::<StatDefinition>("channel")))
            }
        }
        let p = w.plan(Default::default()).unwrap();
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(!r.gaps.is_empty());
        assert!(matches!(
            r.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved { .. },
                ..
            }
        ));
    }
}
#[test]
fn independent_input_changes_storage_order_reuse_and_parallel_workers_are_deterministic() {
    let a = World::new();
    let pa = a.plan(Default::default()).unwrap();
    let ra = pa.evaluate(&mut pa.new_scratch()).unwrap();
    let mut b = World::new();
    b.f.build.character.level = 21;
    b.f.build.gems[0].level = 12;
    b.f.build.character.rewards.reverse();
    b.f.build.gems.reverse();
    b.f.build.skills.reverse();
    b.f.owners.reverse();
    b.members().reverse();
    let pb = b.plan(Default::default()).unwrap();
    let rb = pb.evaluate(&mut pb.new_scratch()).unwrap();
    assert_values(&b.report(), 33, 15, 23);
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    for (p, r) in [(&pa, &ra), (&pb, &rb), (&pa, &ra)] {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *r);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                p.evaluate(&mut p.new_scratch()).unwrap()
            })
            .collect()
    });
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r, if i % 2 == 0 { &ra } else { &rb });
    }
    assert!(matches!(
        a.plan(PlanLimits {
            max_work: 1,
            ..Default::default()
        }),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
}
