//! Semantic contribution ordering over candidate-owned occurrences, not opaque IDs.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_core::{
    owned_readiness::*, owned_stages::*, owned_support_inputs::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_data::{
    owned_rules::OwnedRulePackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::sync::Arc;
use support::*;
fn q(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def("count")).unwrap())
}
fn qt() -> ComputedValueType {
    ComputedValueType::Quantity { unit: def("count") }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn passive() -> SchemaSubject {
    subject(def::<PassiveNodeDefinition>("passive"))
}
fn empty_ports() -> DeclaredSlots {
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
fn member(
    owner: SchemaSubject,
    program: &str,
    source_rank: u32,
    origin: OrderedContributionOrigin,
) -> OrderedContributionMember {
    OrderedContributionMember {
        owner,
        program: key(program),
        effect: key("add"),
        order: OrderedContributionOrder {
            source_rank,
            program_rank: 0,
            effect_rank: 0,
            origin,
        },
    }
}
fn slots() -> Vec<OrderedEquipmentSlot> {
    vec![
        OrderedEquipmentSlot {
            slot: def("weapon"),
            rank: 0,
        },
        OrderedEquipmentSlot {
            slot: def("other"),
            rank: 1,
        },
    ]
}
fn producer(id: &str, context: RuleEntityKind, n: f64) -> RuleProgram {
    RuleProgram {
        id: key(id),
        context,
        reads: vec![],
        nodes: vec![node("value", RuleExpression::Literal { value: q(n) })],
        effects: vec![effect(
            "add",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: def("ordered"),
                contribution: ContributionKind::Add,
                value: key("value"),
            },
        )],
    }
}
struct World {
    f: Fixture,
    registry: DeclaredSet<OrderedContributionQuery>,
}
impl World {
    fn new() -> Self {
        let mut f = Fixture::new();
        // The generic fixture carries an unused Skill/output. This finite domain
        // contains no Skill, Gem or support occurrence; remove that unused schema.
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
        for owner in [class_owner(), item_owner(), modifier_owner()] {
            f.owner_mut(&owner).programs.members.clear();
        }
        for name in ["ordered", "result", "other-result"] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: qt(),
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
        }
        f.schema
            .definitions
            .push(DefinitionDescriptor::PointPool(known(
                def("pool"),
                PointPoolSchema {
                    scope: PointPoolScope::Shared,
                },
            )));
        f.schema
            .definitions
            .push(DefinitionDescriptor::PassiveNode(known(
                def("passive"),
                PassiveNodeSchema {
                    pools: DeclaredSet::complete(vec![def("pool")]),
                    adjacent: DeclaredSet::complete(vec![]),
                    declarations: empty_ports(),
                },
            )));
        f.build.allocations.push(Allocation {
            id: occurrence(30),
            node: def("passive"),
            pool: def("pool"),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        });
        f.owners.push(DefinitionRules {
            owner: passive(),
            programs: DeclaredSet::complete(vec![producer(
                "passive",
                RuleEntityKind::Actor,
                -1e16,
            )]),
        });
        f.owner_mut(&class_owner()).programs.members.push(producer(
            "class",
            RuleEntityKind::Actor,
            1e16,
        ));
        f.owner_mut(&item_owner()).programs.members.push(producer(
            "equipment",
            RuleEntityKind::EquipmentUse,
            0.25,
        ));
        let roll = parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll");
        let row = f
            .schema
            .slots
            .iter_mut()
            .find_map(|s| match s {
                SlotDescriptor::Parameter(r) if r.id == roll => Some(r),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(s) = &mut row.schema else {
            panic!()
        };
        s.value = ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(-1e16, def("count")).unwrap(),
            maximum: FiniteQuantity::new(1e16, def("count")).unwrap(),
        });
        for (m, n) in f.build.items[0].modifiers.iter_mut().zip([0.5, 0.25]) {
            m.rolls[0].value = q(n);
        }
        let mut p = producer("modifier", RuleEntityKind::EquipmentUse, 0.);
        p.reads = vec![RuleRead {
            id: key("roll"),
            value_type: qt(),
            source: RuleReadSource::Parameter { slot: roll },
        }];
        p.nodes = vec![read_node("value", "roll")];
        f.owner_mut(&modifier_owner()).programs.members.push(p);
        let registry = DeclaredSet::complete(vec![OrderedContributionQuery {
            id: key("ordered-query"),
            stat: def("ordered"),
            contribution: ContributionKind::Add,
            groups: vec![
                OrderedContributionGroup {
                    id: key("main"),
                    reduction: ContributionReduction::Sum,
                    empty: q(0.),
                    members: DeclaredSet::complete(vec![
                        member(
                            class_owner(),
                            "class",
                            10,
                            OrderedContributionOrigin::Character,
                        ),
                        member(
                            passive(),
                            "passive",
                            20,
                            OrderedContributionOrigin::Allocation,
                        ),
                        member(
                            item_owner(),
                            "equipment",
                            30,
                            OrderedContributionOrigin::EquipmentUse { slots: slots() },
                        ),
                        member(
                            modifier_owner(),
                            "modifier",
                            40,
                            OrderedContributionOrigin::ItemModifier { slots: slots() },
                        ),
                    ]),
                },
                OrderedContributionGroup {
                    id: key("empty"),
                    reduction: ContributionReduction::Sum,
                    empty: q(0.),
                    members: DeclaredSet::complete(vec![]),
                },
            ],
        }]);
        let mut w = Self { f, registry };
        w.receiver("result", "main");
        w.receiver("other-result", "empty");
        w
    }
    fn receiver(&mut self, name: &str, group: &str) {
        self.f.receivers.members.push(StatReceiver {
            id: key(name),
            stat: def(name),
            program: key(name),
            targets: vec![StatReceiverTarget::Player],
        });
        self.f.owners.push(DefinitionRules {
            owner: subject(def::<StatDefinition>(name)),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key(name),
                context: RuleEntityKind::Actor,
                reads: vec![RuleRead {
                    id: key("incoming"),
                    value_type: qt(),
                    source: RuleReadSource::OrderedContributions {
                        entity: RuleEntity::Current,
                        query: key("ordered-query"),
                        group: key(group),
                    },
                }],
                nodes: vec![read_node("value", "incoming")],
                effects: vec![derive("final", RuleEntity::Current, name, "value")],
            }]),
        });
    }
    fn input(&self, schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
        RulePackageInput {
            existing_actor_rules: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("ordered-rules"),
            semantics_version: key("test-v1"),
            operations_version: key("owned-domain-operations-v21"),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: self.f.owners.clone(),
            receivers: self.f.receivers.clone(),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            ordered_contributions: Some(self.registry.clone()),
        }
    }
    fn plan(&self) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), OwnedSchemaLimits::default())
                .unwrap();
        checked_plan(
            &self.f,
            self.input(&schema),
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("route"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
        )
    }
    fn report(&self) -> SupportEffectsReport {
        let p = self.plan().unwrap();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        evaluated(&self.report()).clone()
    }
    fn members(&mut self) -> &mut Vec<OrderedContributionMember> {
        &mut self.registry.members[0].groups[0].members.members
    }
}
fn evaluated(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("closed execution expected: {:?}", report.outcome)
    };
    effects
}
fn assert_incomplete(report: &SupportEffectsReport, expected_gap: PlanGapReason) {
    assert!(matches!(
        &report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(report.gaps.iter().any(|g| g.reason == expected_gap));
}
fn value<'a>(r: &'a OwnedEffectsReport, name: &str) -> &'a EffectValue {
    &r.values
        .iter()
        .find(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(name),
                }
        })
        .unwrap()
        .value
}
fn number(r: &OwnedEffectsReport, name: &str, n: f64) {
    assert_eq!(value(r, name), &EffectValue::Known { value: q(n) });
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::GameRules,
            code: key("not-closed"),
        }],
    }
}
#[test]
fn actual_class_passive_equipment_and_repeated_modifiers_follow_authored_order() {
    let w = World::new();
    let r = w.evaluate();
    assert!(r.gaps.is_empty());
    number(&r, "result", 2.);
    number(&r, "other-result", 0.);
    let actual:Vec<_>=r.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.stat==def("ordered"))).collect();
    assert_eq!(actual.len(), 8);
    let mut edited = World::new();
    edited.members()[0].order.source_rank = 50;
    number(&edited.evaluate(), "result", 0.);
}
#[test]
fn storage_ids_and_rebased_lineages_cannot_replace_semantic_modifier_order() {
    let mut w = World::new();
    let expected = w.evaluate();
    w.f.build.equipment.reverse();
    w.f.build.items[0].modifiers.reverse();
    w.f.owners.reverse();
    w.members().reverse();
    number(&w.evaluate(), "result", 2.);
    // Rebase every occurrence ID while retaining all owned relationships/order.
    let mut v = serde_json::to_value(&w.f.build).unwrap();
    fn rebase(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                if o.contains_key("lineage") && o.contains_key("local") {
                    o.insert(
                        "lineage".into(),
                        serde_json::json!("abababababababababababababababab"),
                    );
                    let n = u64::from_str_radix(o["local"].as_str().unwrap(), 16).unwrap();
                    o.insert(
                        "local".into(),
                        serde_json::json!(format!("{:016x}", 200 - n)),
                    );
                } else {
                    for v in o.values_mut() {
                        rebase(v)
                    }
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    rebase(v)
                }
            }
            _ => {}
        }
    }
    rebase(&mut v);
    v["allocator"]["lineage"] = serde_json::json!("abababababababababababababababab");
    v["allocator"]["last_issued"] = serde_json::json!("00000000000000c8");
    // Allocator's concrete format is preserved except its lineage; a larger ceiling
    // also keeps the newly permuted IDs valid.
    w.f.build = serde_json::from_value(v).unwrap();
    w.f.build.allocator =
        InstanceAllocatorState::from_parts(BuildLineage::from_bytes([0xab; 16]), 201);
    number(&w.evaluate(), "result", 2.);
    assert_eq!(value(&expected, "result"), value(&w.evaluate(), "result"));
    let mut w = World::new();
    w.f.build.equipment.truncate(1);
    w.f.owner_mut(&class_owner()).programs.members[0].nodes[0] =
        node("value", RuleExpression::Literal { value: q(0.) });
    w.f.owner_mut(&passive()).programs.members[0].nodes[0] =
        node("value", RuleExpression::Literal { value: q(0.) });
    w.f.owner_mut(&item_owner()).programs.members[0].nodes[0] =
        node("value", RuleExpression::Literal { value: q(0.) });
    w.f.build.items[0].modifiers[0].rolls[0].value = q(1e16);
    w.f.build.items[0].modifiers[1].rolls[0].value = q(-1e16);
    let mut third = w.f.build.items[0].modifiers[0].clone();
    third.id = occurrence(31);
    third.rolls[0].value = q(1.);
    w.f.build.items[0].modifiers.push(third);
    w.f.build.items[0].modifier_order.push(occurrence(31));
    number(&w.evaluate(), "result", 1.);
    w.f.build.items[0].modifier_order.swap(1, 2);
    number(&w.evaluate(), "result", 0.);
}
#[test]
fn exact_membership_rejects_unmapped_duplicate_tied_and_missing_group_policies() {
    let mut w = World::new();
    w.members().pop();
    assert!(w.plan().is_err());
    let mut w = World::new();
    let duplicate = w.members()[0].clone();
    w.registry.members[0].groups[1]
        .members
        .members
        .push(duplicate);
    assert!(w.plan().is_err());
    let mut w = World::new();
    w.members()[1].order.source_rank = 10;
    assert!(w.plan().is_err());
    let mut w = World::new();
    w.registry.members[0].groups.pop();
    assert!(w.plan().is_err());
    let mut w = World::new();
    let OrderedContributionOrigin::ItemModifier { slots } = &mut w.members()[3].order.origin else {
        panic!()
    };
    slots.pop();
    assert!(w.plan().is_err());
    let mut w = World::new();
    w.members()[3].order.origin = OrderedContributionOrigin::Allocation;
    assert!(w.plan().is_err());
}
#[test]
fn inactive_effects_skip_but_unknown_inputs_and_partial_membership_never_default() {
    let mut w = World::new();
    let p = &mut w.f.owner_mut(&modifier_owner()).programs.members[0];
    p.nodes.push(node(
        "enabled",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    ));
    p.effects[0].when = Some(key("enabled"));
    number(&w.evaluate(), "result", 0.5);
    let p = &mut w.f.owner_mut(&modifier_owner()).programs.members[0];
    p.reads.push(RuleRead {
        id: key("unknown"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: def("unknown"),
        },
    });
    p.nodes.pop();
    p.nodes.push(read_node("enabled", "unknown"));
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(known(
            def("unknown"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Actor],
            },
        )));
    assert!(matches!(
        value(&w.evaluate(), "result"),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new();
    w.registry.members[0].groups[1].members.closure =
        partial(subject(def::<StatDefinition>("ordered")));
    assert_incomplete(&w.report(), PlanGapReason::IncompleteContributors);
    let mut w = World::new();
    w.f.owner_mut(&modifier_owner()).programs.closure = partial(modifier_owner());
    let report = w.report();
    assert_incomplete(&report, PlanGapReason::PartialPrograms);
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                && gap.subject.as_ref() == Some(&modifier_owner()))
    );
    let mut w = World::new();
    w.f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    number(&w.evaluate(), "result", 1.);
}
#[test]
fn independent_groups_and_lazy_reads_remain_explicit() {
    let mut w = World::new();
    let modifiers = w.members().pop().unwrap();
    w.registry.members[0].groups[1]
        .members
        .members
        .push(modifiers);
    number(&w.evaluate(), "result", 0.5);
    number(&w.evaluate(), "other-result", 1.5);
    // Existing lazy Select gates an entire group read; the selected literal
    // does not evaluate the unused reduction branch.
    let p = &mut w
        .f
        .owner_mut(&subject(def::<StatDefinition>("result")))
        .programs
        .members[0];
    p.nodes.extend([
        node(
            "off",
            RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        ),
        node("zero", RuleExpression::Literal { value: q(0.) }),
        node(
            "selected",
            RuleExpression::Select {
                condition: key("off"),
                when_true: key("value"),
                when_false: key("zero"),
            },
        ),
    ]);
    p.effects[0] = derive("final", RuleEntity::Current, "result", "selected");
    number(&w.evaluate(), "result", 0.);
}
#[test]
fn fresh_reused_and_four_worker_candidate_reports_are_deterministic() {
    let a = World::new();
    let mut b = World::new();
    b.f.build.items[0].modifiers[0].rolls[0].value = q(2.);
    let pa = a.plan().unwrap();
    let pb = b.plan().unwrap();
    let ra = a.report();
    let rb = b.report();
    number(evaluated(&ra), "result", 2.);
    number(evaluated(&rb), "result", 5.);
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    assert_eq!(pb.evaluate(&mut scratch).unwrap(), rb);
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let actual: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                p.evaluate(&mut p.new_scratch()).unwrap()
            })
            .collect()
    });
    for (i, r) in actual.iter().enumerate() {
        assert_eq!(r, if i % 2 == 0 { &ra } else { &rb });
    }
}

#[test]
fn finite_stage_conditional_contribution_rebinds_without_mutable_snapshots() {
    let mut w = World::new();
    for name in ["stage-two-input", "stage-two"] {
        w.f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: qt(),
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
    }
    let mut producer = producer("stage-two-producer", RuleEntityKind::Actor, 3.);
    producer.reads.push(RuleRead {
        id: key("previous"),
        value_type: qt(),
        source: RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: def("result"),
        },
    });
    producer.nodes.extend([
        read_node("previous", "previous"),
        node("threshold", RuleExpression::Literal { value: q(1.) }),
        node(
            "active",
            RuleExpression::Compare {
                operation: RuleComparison::Greater,
                left: key("previous"),
                right: key("threshold"),
            },
        ),
    ]);
    producer.effects[0].when = Some(key("active"));
    let RuleEffectKind::Contribute { stat, .. } = &mut producer.effects[0].effect else {
        panic!()
    };
    *stat = def("stage-two-input");
    w.f.owner_mut(&class_owner())
        .programs
        .members
        .push(producer);
    w.registry.members.push(OrderedContributionQuery {
        id: key("stage-two-query"),
        stat: def("stage-two-input"),
        contribution: ContributionKind::Add,
        groups: vec![OrderedContributionGroup {
            id: key("main"),
            reduction: ContributionReduction::Sum,
            empty: q(0.),
            members: DeclaredSet::complete(vec![member(
                class_owner(),
                "stage-two-producer",
                0,
                OrderedContributionOrigin::Character,
            )]),
        }],
    });
    w.receiver("stage-two", "main");
    let p = &mut w
        .f
        .owner_mut(&subject(def::<StatDefinition>("stage-two")))
        .programs
        .members[0];
    let RuleReadSource::OrderedContributions { query, .. } = &mut p.reads[0].source else {
        panic!()
    };
    *query = key("stage-two-query");
    number(&w.evaluate(), "stage-two", 3.);
    w.members()[0].order.source_rank = 50;
    number(&w.evaluate(), "stage-two", 0.);
}

#[test]
fn partial_registry_and_unread_groups_cannot_become_complete_empty_identities() {
    for registry in [false, true] {
        let mut w = World::new();
        let gap = partial(subject(def::<StatDefinition>("ordered")));
        if registry {
            w.registry.closure = gap;
        } else {
            w.registry.members[0].groups[1].members.closure = gap;
        }
        assert_incomplete(&w.report(), PlanGapReason::IncompleteContributors);
        // No selected ordered read remains, but open registry evidence remains a
        // whole-plan gap, exactly like the other global authored inventories.
        w.f.receivers.members.clear();
        assert_incomplete(&w.report(), PlanGapReason::IncompleteContributors);
    }
}

// Same checked empty-support construction used by the publication component
// fixtures: all real programs execute, with no support/input values invented.
fn checked_plan(
    f: &Fixture,
    mut rules_input: RulePackageInput,
    mut routing_input: ActionRoutingInput,
) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
    let namespace = f.schema.namespace.clone();
    let build = &f.build;
    let scenario = &f.scenario;
    let quality_unit = def("percent");
    let unused_input = |name: &str| -> StatDefId {
        DefId::parse(
            namespace.clone(),
            format!("fixture.empty-support-domain.{name}"),
        )
        .unwrap()
    };
    assert_eq!(
        rules_input.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V21
    );
    assert!(build.gems.is_empty());
    assert!(build.skills.is_empty());
    assert!(build.supports.is_empty());
    assert!(
        build
            .generated_inputs
            .as_ref()
            .is_none_or(|v| v.schema_version == 1 && v.bindings.is_empty())
    );
    assert!(build.support_origins.as_ref().is_none_or(Vec::is_empty));
    assert_eq!(
        rules_input.effect_applications,
        Some(DeclaredSet::complete(vec![]))
    );
    let unused = [
        (
            unused_input("support-level"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Integer,
        ),
        (
            unused_input("support-quality"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Quantity {
                unit: quality_unit.clone(),
            },
        ),
        (
            unused_input("target-presence"),
            RuleEntityKind::Skill,
            ComputedValueType::Boolean,
        ),
    ];
    let mut schema_input = f.schema.clone();
    assert!(!schema_input.definitions.iter().any(|d| matches!(
        d,
        DefinitionDescriptor::Gem(_) | DefinitionDescriptor::Skill(_)
    )));
    for (id, scope, value) in &unused {
        assert!(
            !schema_input
                .definitions
                .iter()
                .any(|d| d.address() == id.address())
        );
        schema_input
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: value.clone(),
                    targets: vec![*scope],
                }),
            }));
    }
    let schema =
        Arc::new(OwnedDefinitionSchemaPackage::new(schema_input, Default::default()).unwrap());

    rules_input.definitions = schema.identity().clone();
    let stored = OwnedRulePackage::new(rules_input, schema.as_ref(), Default::default())
        .map_err(|e| PlanError::Invalid(e.to_string()))?;
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, schema.as_ref(), Default::default())
            .map_err(|e| PlanError::Invalid(e.to_string()))?,
    );

    routing_input.definitions = schema.identity().clone();
    let routing = Arc::new(
        OwnedActionRouting::new(routing_input, schema.as_ref(), Default::default()).unwrap(),
    );
    let programs: Vec<_> = stored
        .input()
        .owners
        .iter()
        .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
        .collect();
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            EvaluationStagesInput {
                schema_version: OWNED_EVALUATION_STAGES_V3,
                namespace: namespace.clone(),
                release: key("finite-empty-support-stages"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                routing: *routing.identity(),
                stages: vec![
                    EvaluationStage {
                        id: key("prepare"),
                        predecessors: vec![],
                    },
                    EvaluationStage {
                        id: key("execute"),
                        predecessors: vec![key("prepare")],
                    },
                ],
                programs: DeclaredSet::complete(
                    programs
                        .iter()
                        .map(|(o, p)| StagedRuleProgram {
                            owner: o.owner.clone(),
                            program: p.id.clone(),
                            stage: key("execute"),
                        })
                        .collect(),
                ),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                routing_stage: key("execute"),
                frozen_channels: unused
                    .iter()
                    .map(|(id, scope, _)| FrozenStageChannel {
                        channel: StageChannel::Stat {
                            scope: *scope,
                            stat: id.clone(),
                        },
                        stage: key("prepare"),
                    })
                    .collect(),
                readiness: Some(ReadinessInput {
                    skills: vec![],
                    programs: DeclaredSet::complete(
                        programs
                            .iter()
                            .map(|(o, p)| ReadinessProgram {
                                owner: o.owner.clone(),
                                program: p.id.clone(),
                                phase: ReadinessPhase::Execution,
                                role: ReadinessProgramRole::Execution,
                                outputs: vec![],
                            })
                            .collect(),
                    ),
                }),
            },
            schema.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )
        .unwrap(),
    );
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: namespace.clone(),
                release: key("finite-empty-support-no-supports"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                quality_unit: quality_unit.clone(),
                types: vec![],
                effects: vec![],
                families: vec![],
                supports: vec![],
            },
            schema.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap(),
    );
    let presence = unused_input("target-presence");
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            SupportInputBindingsInput {
                schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
                namespace: namespace.clone(),
                release: key("finite-empty-support-no-support-inputs"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: key("prepare"),
                effective_level: unused_input("support-level"),
                effective_quality: unused_input("support-quality"),
                target: SupportTargetInputBindings {
                    skill_types: vec![],
                    minion_types: OptionalTypeInputs {
                        present: presence.clone(),
                        members: vec![],
                    },
                    summoner: OptionalTypeContextInputs {
                        present: presence.clone(),
                        skill_types: vec![],
                        minion_types: OptionalTypeInputs {
                            present: presence.clone(),
                            members: vec![],
                        },
                    },
                    cannot_be_supported: presence.clone(),
                    has_gem: presence.clone(),
                    from_item: presence.clone(),
                    is_player_actor: presence,
                },
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            SupportReceivingInput {
                schema_version: OWNED_SUPPORT_RECEIVING_V2,
                namespace: namespace.clone(),
                release: key("finite-empty-support-no-support-receivers"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                inputs: *inputs.identity(),
                stages: *stages.identity(),
                roles: vec![],
                targets: vec![],
                supports: vec![],
                source_properties: None,
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let mut build = build.clone();
    build.support_origins = Some(vec![]);
    build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: vec![],
    });
    let limits = OwnedInputLimits::default();
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(build, limits).unwrap(),
        ScenarioSpec::new(scenario.clone(), limits).unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: namespace.clone(),
                requests: vec![],
            },
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap();
    OwnedSupportEffectPlan::compile(
        SupportEffectPlanInputs {
            request: Arc::new(request),
            definitions: schema,
            rules,
            routing,
            stages,
            preparation,
            inputs,
            receiving,
        },
        Default::default(),
        Default::default(),
    )
}
