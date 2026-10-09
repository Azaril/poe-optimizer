//! Optional numeric replacements retain exact sources and share one cached result.
#[allow(dead_code)]
#[path = "support/owned_empty_support_contribution_fixture.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, decode_rule_package, encode_rule_package},
    owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use support::*;

fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
struct World {
    f: Fixture,
    registry: DeclaredSet<ContributionQuery>,
}
impl World {
    fn new() -> Self {
        let mut f = Fixture::new();
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
        for o in &mut f.owners {
            o.programs.members.clear();
        }
        f.schema.definitions.push(DefinitionDescriptor::Unit(known(
            def("percent"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )));
        for (name, value) in [
            ("override", ComputedValueType::Integer),
            ("result", ComputedValueType::Integer),
            ("value-only", ComputedValueType::Integer),
            ("present", ComputedValueType::Boolean),
        ] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
        }
        let roll = parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll");
        f.owner_mut(&modifier_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("override-producer"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![read("roll", RuleReadSource::Parameter { slot: roll })],
                nodes: vec![
                    read_node("amount", "roll"),
                    node(
                        "active",
                        RuleExpression::Literal {
                            value: ParameterValue::Boolean(true),
                        },
                    ),
                ],
                effects: vec![RuleEffect {
                    id: key("override"),
                    when: Some(key("active")),
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Player,
                        stat: def("override"),
                        contribution: ContributionKind::Override,
                        value: key("amount"),
                    },
                }],
            });
        let registry = DeclaredSet::complete(vec![ContributionQuery {
            id: key("overrides"),
            stat: def("override"),
            contribution: ContributionKind::Override,
            groups: vec![ContributionGroup {
                id: key("all"),
                reduction: ContributionReduction::RequireAgreement,
                ordering: ContributionOrdering::Unordered,
                empty: None,
                members: DeclaredSet::complete(vec![ContributionMember {
                    producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                        owner: modifier_owner(),
                        program: key("override-producer"),
                        effect: key("override"),
                        origin: ContributionOrigin::ItemModifier {
                            slots: vec![def("weapon"), def("other")],
                        },
                    }),
                    order: None,
                }]),
            }],
        }]);
        let mut world = Self { f, registry };
        world.rolls(5, 5);
        world.consumer("result", None);
        world.consumer("present", Some(ContributionSelectionProjection::Present));
        world.consumer("value-only", Some(ContributionSelectionProjection::Value));
        world
    }
    fn consumer(&mut self, name: &str, projection: Option<ContributionSelectionProjection>) {
        let mut p = RuleProgram {
            id: key(name),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![],
            effects: vec![derive("final", RuleEntity::Current, name, "result")],
        };
        for (name, projection, value_type) in [
            (
                "present",
                ContributionSelectionProjection::Present,
                ComputedValueType::Boolean,
            ),
            (
                "selected",
                ContributionSelectionProjection::Value,
                ComputedValueType::Integer,
            ),
        ] {
            p.reads.push(RuleRead {
                id: key(name),
                value_type,
                source: RuleReadSource::ContributionSelection {
                    entity: RuleEntity::Current,
                    query: key("overrides"),
                    group: key("all"),
                    projection,
                },
            });
            p.nodes.push(read_node(name, name));
        }
        p.nodes.push(literal("fallback", 91));
        p.nodes.push(match projection {
            None => node(
                "result",
                RuleExpression::Select {
                    condition: key("present"),
                    when_true: key("selected"),
                    when_false: key("fallback"),
                },
            ),
            Some(ContributionSelectionProjection::Present) => read_node("result", "present"),
            Some(ContributionSelectionProjection::Value) => read_node("result", "selected"),
        });
        self.f.receivers.members.push(StatReceiver {
            id: key(name),
            stat: def(name),
            program: key(name),
            targets: vec![StatReceiverTarget::Player],
        });
        self.f.owners.push(DefinitionRules {
            owner: subject(def::<StatDefinition>(name)),
            programs: DeclaredSet::complete(vec![p]),
        });
    }
    fn rolls(&mut self, a: i64, b: i64) {
        for (m, n) in self.f.build.items[0].modifiers.iter_mut().zip([a, b]) {
            m.rolls[0].value = integer(n);
        }
    }
    fn producer(&mut self) -> &mut RuleProgram {
        &mut self.f.owner_mut(&modifier_owner()).programs.members[0]
    }
    fn active(&mut self, active: bool) {
        self.producer().nodes[1].expression = RuleExpression::Literal {
            value: ParameterValue::Boolean(active),
        };
    }
    fn input(&self) -> (OwnedDefinitionSchemaPackage, RulePackageInput) {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap();
        let input = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("selection-rules"),
            semantics_version: key("test-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V27),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: self.f.owners.clone(),
            receivers: self.f.receivers.clone(),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            contribution_queries: Some(self.registry.clone()),
            existing_actor_rules: None,
            support_discovery: Some(assignment_only_domains(&self.f.owners)),
        };
        (schema, input)
    }
    fn plan(&self) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        self.plan_with(|_| {})
    }
    fn plan_with(
        &self,
        edit: impl FnOnce(&mut EvaluationStagesInput),
    ) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        let (schema, input) = self.input();
        empty_support::checked_plan_with_stages(
            &self.f,
            input,
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("route"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            Default::default(),
            |s| {
                s.frozen_channels.push(FrozenStageChannel {
                    channel: StageChannel::Contributions {
                        scope: RuleEntityKind::Actor,
                        stat: def("override"),
                        contribution: ContributionKind::Override,
                    },
                    stage: key("execute"),
                });
                edit(s);
            },
        )
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        let p = self.plan().unwrap();
        evaluated(p.evaluate(&mut p.new_scratch()).unwrap())
    }
}
fn evaluated(report: SupportEffectsReport) -> OwnedEffectsReport {
    match report.outcome {
        SupportEffectsOutcome::Evaluated { effects } => effects,
        other => panic!("expected executed graph: {other:?}"),
    }
}
fn value<'a>(report: &'a OwnedEffectsReport, name: &str) -> &'a EffectValue {
    &report.values.iter().find(|v| matches!(&v.key, PlanValueKey::Stat { entity: ConcreteEntity::Actor(ActorKey::Player), stat } if stat == &def::<StatDefinition>(name))).unwrap().value
}
fn selected(report: &OwnedEffectsReport) -> &BoundEffectResult {
    let results: Vec<_> = report
        .effects
        .iter()
        .filter(|v| matches!(v.target, BoundEffectTarget::ContributionSelection { .. }))
        .collect();
    assert_eq!(
        results.len(),
        1,
        "separate programs and both projections share a single result"
    );
    results[0]
}
fn known_value(v: ParameterValue) -> EffectValue {
    EffectValue::Known { value: v }
}
#[test]
fn agreeing_repeated_occurrences_select_once_including_zero() {
    for n in [-10, 0, 5, 97] {
        let mut w = World::new();
        w.rolls(n, n);
        let report = w.evaluate();
        assert_eq!(value(&report, "result"), &known_value(integer(n)));
        assert_eq!(
            value(&report, "present"),
            &known_value(ParameterValue::Boolean(true))
        );
        assert_eq!(selected(&report).value, known_value(integer(n)));
        assert_eq!(
            report
                .effects
                .iter()
                .filter(|r| matches!(r.target, BoundEffectTarget::Contribution { .. }))
                .count(),
            4,
            "two rolled modifiers on two equipment occurrences remain distinct"
        );
    }
}
#[test]
fn empty_or_inactive_is_absent_and_lazy_default_does_not_demand_value() {
    for empty in [false, true] {
        let mut w = World::new();
        if empty {
            w.f.build.equipment.clear();
        } else {
            w.active(false);
        }
        let report = w.evaluate();
        assert_eq!(selected(&report).value, EffectValue::Inactive);
        assert_eq!(
            value(&report, "present"),
            &known_value(ParameterValue::Boolean(false))
        );
        assert_eq!(value(&report, "result"), &known_value(integer(91)));
        assert!(matches!(
            value(&report, "value-only"),
            EffectValue::Unresolved {
                reason: PlanGapReason::AbsentSelection,
                ..
            }
        ));
    }
}
#[test]
fn conflicts_block_both_projections_and_do_not_choose_first_source() {
    for amounts in [(0, 5), (5, 0)] {
        let mut w = World::new();
        w.rolls(amounts.0, amounts.1);
        let report = w.evaluate();
        for name in ["present", "value-only", "result"] {
            assert!(matches!(
                value(&report, name),
                EffectValue::Unresolved {
                    reason: PlanGapReason::ConflictingContributors,
                    ..
                }
            ));
        }
        assert!(matches!(
            selected(&report).value,
            EffectValue::Unresolved {
                reason: PlanGapReason::ConflictingContributors,
                ..
            }
        ));
    }
}
#[test]
fn unknown_member_cannot_be_hidden_by_known_zero_and_scratch_resets() {
    let mut w = World::new();
    w.rolls(0, 0);
    let a = w.plan().unwrap();
    w.f.owner_mut(&class_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("unknown-source"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "unknown",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("override"),
                },
            )],
            nodes: vec![read_node("unknown", "unknown")],
            effects: vec![effect(
                "override",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def("override"),
                    contribution: ContributionKind::Override,
                    value: key("unknown"),
                },
            )],
        });
    w.registry.members[0].groups[0]
        .members
        .members
        .push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: class_owner(),
                program: key("unknown-source"),
                effect: key("override"),
                origin: ContributionOrigin::Character,
            }),
            order: None,
        });
    let unknown = w.plan().unwrap();
    w.f.owner_mut(&class_owner()).programs.members.clear();
    w.registry.members[0].groups[0].members.members.pop();
    w.rolls(7, 7);
    let b = w.plan().unwrap();
    let mut scratch = a.new_scratch();
    let first = evaluated(a.evaluate(&mut scratch).unwrap());
    let missing = evaluated(unknown.evaluate(&mut scratch).unwrap());
    assert!(matches!(
        value(&missing, "present"),
        EffectValue::Unresolved { .. }
    ));
    assert!(matches!(
        selected(&missing).value,
        EffectValue::Unresolved { .. }
    ));
    assert_eq!(
        value(&evaluated(b.evaluate(&mut scratch).unwrap()), "result"),
        &known_value(integer(7))
    );
    assert_eq!(evaluated(a.evaluate(&mut scratch).unwrap()), first);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..24).into_par_iter().for_each_init(
            || a.new_scratch(),
            |scratch, _| {
                for p in [&a, &unknown, &b, &a] {
                    assert_eq!(
                        p.evaluate(scratch).unwrap(),
                        p.evaluate(&mut p.new_scratch()).unwrap()
                    );
                }
            },
        )
    });
}
#[test]
fn exact_membership_is_required_for_inactive_zero_and_unread_groups() {
    for active in [true, false] {
        let mut w = World::new();
        w.rolls(0, 0);
        w.active(active);
        w.registry.members[0].groups[0].members.members.clear();
        // Even deleting all consumers cannot make the unlisted writer disappear.
        w.f.receivers.members.clear();
        w.f.owners.retain(|o| {
            !matches!(
                o.owner,
                SchemaSubject::Definition(DefinitionAddress::Stat(_))
            )
        });
        let error = w
            .plan()
            .err()
            .expect("unlisted writer must fail")
            .to_string();
        assert!(error.contains("no declared membership"), "{error}");
    }
}
#[test]
fn selection_storage_round_trip_and_contract_rejections() {
    let w = World::new();
    let (schema, raw) = w.input();
    let stored = OwnedRulePackage::new(raw.clone(), &schema, Default::default()).unwrap();
    let bytes = encode_rule_package(&stored, Default::default()).unwrap();
    let rebuilt = decode_rule_package(&bytes, &schema, Default::default()).unwrap();
    assert_eq!(stored.identity(), rebuilt.identity());
    for case in 0..7 {
        let mut bad = raw.clone();
        let group = &mut bad.contribution_queries.as_mut().unwrap().members[0].groups[0];
        match case {
            0 => group.empty = Some(integer(0)),
            1 => group.ordering = ContributionOrdering::Ordered,
            2 => group.reduction = ContributionReduction::Sum,
            3 => bad.operations_version = key(OWNED_RULE_OPERATIONS_V26),
            4 => {
                bad.owners
                    .iter_mut()
                    .find(|o| o.owner == subject(def::<StatDefinition>("present")))
                    .unwrap()
                    .programs
                    .members[0]
                    .reads[0]
                    .value_type = ComputedValueType::Integer
            }
            5 => {
                bad.owners
                    .iter_mut()
                    .find(|o| o.owner == subject(def::<StatDefinition>("result")))
                    .unwrap()
                    .programs
                    .members[0]
                    .reads[1]
                    .source = RuleReadSource::ContributionQuery {
                    entity: RuleEntity::Current,
                    query: key("overrides"),
                    group: key("all"),
                }
            }
            6 => {
                bad.owners
                    .iter_mut()
                    .find(|o| o.owner == subject(def::<StatDefinition>("result")))
                    .unwrap()
                    .programs
                    .members[0]
                    .reads[1]
                    .source = RuleReadSource::Contributions {
                    entity: RuleEntity::Current,
                    stat: def("override"),
                    contribution: ContributionKind::Override,
                    reduction: ContributionReduction::RequireAgreement,
                    empty: integer(0),
                }
            }
            _ => unreachable!(),
        }
        assert!(
            OwnedRulePackage::new(bad, &schema, Default::default()).is_err(),
            "invalid case {case}"
        );
    }
}
#[test]
fn selection_stage_and_cycles_remain_checked() {
    let w = World::new();
    assert!(
        w.plan_with(|s| s
            .frozen_channels
            .retain(|r| !matches!(r.channel, StageChannel::Contributions { .. })))
            .is_err()
    );
    assert!(
        w.plan_with(|s| s.frozen_channels.last_mut().unwrap().stage = key("prepare"))
            .is_err()
    );
    let mut w = World::new();
    w.producer().reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Player,
        stat: def("result"),
    };
    let error = w.plan().err().expect("cycle must fail").to_string();
    assert!(error.contains("cycle"), "{error}");
}

#[test]
fn quantities_keep_units_negative_values_and_canonical_positive_zero() {
    for (a, b) in [(-7.5, -7.5), (-0.0, 0.0), (0.0, -0.0)] {
        let mut w = World::new();
        let quantity = |n| ParameterValue::Quantity(FiniteQuantity::new(n, def("count")).unwrap());
        let ty = ComputedValueType::Quantity { unit: def("count") };
        for d in &mut w.f.schema.definitions {
            if let DefinitionDescriptor::Stat(row) = d
                && [
                    def::<StatDefinition>("override"),
                    def("result"),
                    def("value-only"),
                ]
                .contains(&row.id)
                && let SchemaState::Known(schema) = &mut row.schema
            {
                schema.value = ty.clone();
            }
        }
        for slot in &mut w.f.schema.slots {
            if let SlotDescriptor::Parameter(row) = slot
                && row.id == parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll")
                && let SchemaState::Known(schema) = &mut row.schema
            {
                schema.value = ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-100.0, def("count")).unwrap(),
                    maximum: FiniteQuantity::new(100.0, def("count")).unwrap(),
                });
            }
        }
        for (m, n) in w.f.build.items[0].modifiers.iter_mut().zip([a, b]) {
            m.rolls[0].value = quantity(n);
        }
        w.producer().reads[0].value_type = ty.clone();
        for owner in &mut w.f.owners {
            for program in &mut owner.programs.members {
                for read in &mut program.reads {
                    if matches!(
                        read.source,
                        RuleReadSource::ContributionSelection {
                            projection: ContributionSelectionProjection::Value,
                            ..
                        }
                    ) {
                        read.value_type = ty.clone();
                    }
                }
                for node in &mut program.nodes {
                    if node.id == key("fallback") {
                        node.expression = RuleExpression::Literal {
                            value: quantity(91.0),
                        };
                    }
                }
            }
        }
        let report = w.evaluate();
        assert_eq!(value(&report, "result"), &known_value(quantity(a)));
        if a == 0.0 {
            let EffectValue::Known {
                value: ParameterValue::Quantity(q),
            } = &selected(&report).value
            else {
                panic!()
            };
            assert_eq!(q.value().to_bits(), 0.0f64.to_bits());
        }
        w.producer().nodes[0].expression = RuleExpression::Literal {
            value: ParameterValue::Quantity(FiniteQuantity::new(a, def("percent")).unwrap()),
        };
        assert!(
            w.plan().is_err(),
            "contribution with wrong units cannot be compiled"
        );
    }
}

#[test]
fn partial_membership_does_not_become_an_absent_override() {
    let mut w = World::new();
    w.active(false);
    w.registry.members[0].groups[0].members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(def::<StatDefinition>("override")),
            facet: SchemaFacet::GameRules,
            code: key("unknown-source"),
        }],
    };
    let p = w.plan().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}

#[test]
fn unknown_activation_is_unavailable_but_false_skips_an_unknown_value() {
    let mut w = World::new();
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(known(
            def("missing-active"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Actor],
            },
        )));
    w.producer().reads.push(RuleRead {
        id: key("active"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: def("missing-active"),
        },
    });
    w.producer().nodes[1] = read_node("active", "active");
    let report = w.evaluate();
    assert!(matches!(
        value(&report, "present"),
        EffectValue::Unresolved { .. }
    ));
    assert!(matches!(
        selected(&report).value,
        EffectValue::Unresolved { .. }
    ));
    w.active(false);
    w.producer().reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Player,
        stat: def("override"),
    };
    let report = w.evaluate();
    assert_eq!(value(&report, "result"), &known_value(integer(91)));
    assert_eq!(selected(&report).value, EffectValue::Inactive);
}

#[test]
fn a_selection_frozen_in_preparation_survives_suffix_binding() {
    let mut w = World::new();
    w.f.build.equipment.clear();
    let p = w
        .plan_with(|s| {
            s.frozen_channels.last_mut().unwrap().stage = key("prepare");
            for program in &mut s.programs.members {
                if program.owner == modifier_owner() {
                    program.stage = key("prepare");
                }
            }
        })
        .unwrap();
    let mut scratch = p.new_scratch();
    let report = evaluated(p.evaluate(&mut scratch).unwrap());
    assert_eq!(selected(&report).value, EffectValue::Inactive);
    assert_eq!(value(&report, "result"), &known_value(integer(91)));
    assert_eq!(evaluated(p.evaluate(&mut scratch).unwrap()), report);
}
