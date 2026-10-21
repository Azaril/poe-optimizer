//! Actual scalar and delivery programs in an explicitly finite component.
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod base;
use base as support;
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_empty_support_contribution_fixture.rs"]
mod checked_empty_support;
#[allow(dead_code)]
#[path = "support/owned_charm_capacity.rs"]
mod packet;
use base::*;
use poe_optimizer_core::{
    build_identity::{ItemSlotUseId, ModifierInstanceId},
    owned_build::*,
    owned_definitions::*,
    owned_routing::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

fn rebound<T: Serialize + DeserializeOwned>(input: &T) -> T {
    fn visit(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if o.contains_key("namespace") && o.contains_key("key") {
                    o.insert("namespace".into(), serde_json::to_value(ns()).unwrap());
                    let replacement = match o["key"].as_str().unwrap() {
                        "def.0000000000003172" => Some("modifier"),
                        "def.0000000000001e84" => Some("item"),
                        "def.000000000000006e" => Some("weapon"),
                        _ => None,
                    };
                    if let Some(k) = replacement {
                        o.insert("key".into(), Value::String(k.into()));
                    }
                }
                for x in o.values_mut() {
                    visit(x)
                }
            }
            Value::Array(a) => {
                for x in a {
                    visit(x)
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(input).unwrap();
    visit(&mut v);
    serde_json::from_value(v).unwrap()
}
fn quantity(value: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def(unit)).unwrap())
}
fn known(value: f64) -> EffectValue {
    EffectValue::Known {
        value: quantity(value, "def.000000000000295a"),
    }
}
struct World {
    f: Fixture,
    query: ContributionQuery,
}
impl World {
    fn new() -> Self {
        packet::check_authored();
        let c = packet::consumer();
        let d = packet::dependencies();
        let mut f = Fixture::new();
        f.schema.definitions.retain(|x| {
            !matches!(
                x,
                DefinitionDescriptor::Skill(_) | DefinitionDescriptor::Modifier(_)
            )
        });
        f.schema.slots.retain(|x| {
            !matches!(x, SlotDescriptor::ActionOutput(_))
                && x.address().declaration() != &SlotOwnerDefId::Modifier(def("modifier"))
        });
        f.owners.retain(|x| {
            !matches!(
                x.owner,
                SchemaSubject::Definition(DefinitionAddress::Skill(_))
                    | SchemaSubject::Slot(SlotAddress::ActionOutput(_))
            )
        });
        for o in &mut f.owners {
            o.programs = DeclaredSet::complete(vec![])
        }
        f.schema
            .definitions
            .extend(d.definitions.iter().map(rebound));
        f.schema.definitions.push(rebound(&d.modifier_descriptor));
        f.schema
            .definitions
            .extend(c.definitions.iter().map(rebound));
        f.schema.slots.extend(d.slots.iter().map(rebound));
        f.schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: def("percent"),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                }),
            }));
        f.owner_mut(&modifier_owner()).programs =
            DeclaredSet::complete(d.owners[0].programs.members.iter().map(rebound).collect());
        f.owner_mut(&modifier_owner())
            .programs
            .members
            .push(rebound(&c.modifier_program));
        let boundary = RuleProgram {
            id: key("fixture-catalyst-boundary"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![
                node(
                    "kind",
                    RuleExpression::Literal {
                        value: ParameterValue::Option(def("def.00000000000009ec")),
                    },
                ),
                node(
                    "amount",
                    RuleExpression::Literal {
                        value: quantity(0.0, "def.0000000000000002"),
                    },
                ),
            ],
            effects: vec![
                effect(
                    "kind",
                    RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: def("def.0000000000000a19"),
                        value: key("kind"),
                    },
                ),
                effect(
                    "amount",
                    RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: def("def.0000000000000a1a"),
                        value: key("amount"),
                    },
                ),
            ],
        };
        f.owner_mut(&item_owner()).programs =
            DeclaredSet::complete(vec![boundary, rebound(&c.template_program)]);
        let rolls: Vec<ParameterAssignment> = d.actual_rolls["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                assert_eq!(r["slot"]["kind"], "known");
                assert_eq!(r["value"]["kind"], "known");
                rebound(&ParameterAssignment {
                    slot: serde_json::from_value(r["slot"]["value"].clone()).unwrap(),
                    value: serde_json::from_value(r["value"]["value"].clone()).unwrap(),
                })
            })
            .collect();
        f.build.items[0].modifiers.truncate(1);
        f.build.items[0].modifier_orders.contributions.truncate(1);
        f.build.items[0]
            .modifier_orders
            .numeric_transforms
            .truncate(1);
        f.build.equipment.truncate(1);
        f.build.items[0].modifiers[0].rolls = rolls;
        let mut query: ContributionQuery = rebound(&c.query);
        query.groups[0].members.closure = SchemaClosure::Complete;
        Self { f, query }
    }
    fn raw(&mut self, value: f64) {
        self.roll(
            "def.0000000000003173",
            quantity(value, "def.000000000000295a"),
        );
    }
    fn roll(&mut self, key: &str, value: ParameterValue) {
        self.f.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot.slot.key().as_str() == key)
            .unwrap()
            .value = value;
    }
    fn plan(&self) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        let f = &self.f;
        let schema =
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
        let rules = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("test"),
            operations_version: key(OWNED_RULE_OPERATIONS_V28),
            definitions: schema.identity().clone(),
            tables: f.tables.clone(),
            owners: f.owners.clone(),
            receivers: f.receivers.clone(),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            contribution_queries: Some(DeclaredSet::complete(vec![self.query.clone()])),
            existing_actor_rules: Some(DeclaredSet::complete(vec![])),
            support_discovery: Some(assignment_only_domains(&f.owners)),
        };
        let routing = ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("routing"),
            definitions: schema.identity().clone(),
            outputs: vec![],
        };
        checked_empty_support::checked_plan(f, rules, routing, Default::default())
    }
    fn report(&self) -> OwnedEffectsReport {
        let p = self.plan().unwrap();
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        let SupportEffectsOutcome::Evaluated { effects } = r.outcome else {
            panic!("{:?}", r.outcome)
        };
        effects
    }
}
fn deliveries(r: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
    r.effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("item-charm-capacity-delivery")
                && e.key.effect == key("player-capacity")
        })
        .collect()
}
#[test]
fn selected_raw_capacity_is_consumed_locally_and_delivered_once_without_final_cap() {
    let mut w = World::new();
    assert_eq!(deliveries(&w.report())[0].value, known(2.0));
    for value in [0.0, 1.0, 3.0, 7.0] {
        w.raw(value);
        let r = w.report();
        let e = deliveries(&r);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].value, known(value));
        assert_eq!(r.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Actor(ActorKey::Player))).count(),1,"local modifiers must not double-deliver");
    }
}
#[test]
fn duplicate_modifiers_sum_in_semantic_item_order_before_one_capacity_delivery() {
    let mut w = World::new();
    let mut second = w.f.build.items[0].modifiers[0].clone();
    second.id = occurrence(5);
    second
        .rolls
        .iter_mut()
        .find(|r| r.slot.slot.key().as_str() == "def.0000000000003173")
        .unwrap()
        .value = quantity(3.0, "def.000000000000295a");
    w.f.build.items[0].modifiers.push(second);
    w.f.build.items[0]
        .modifier_orders
        .contributions
        .push(occurrence(5));
    w.f.build.items[0]
        .modifier_orders
        .numeric_transforms
        .push(occurrence(5));
    let r = w.report();
    assert_eq!(deliveries(&r).len(), 1);
    assert_eq!(deliveries(&r)[0].value, known(5.0));
    let local: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("local-charm-capacity-contribution"))
        .collect();
    assert_eq!(local.len(), 2);
    assert_ne!(
        local[0].key.invocation.origin,
        local[1].key.invocation.origin
    );
    w.f.build.items[0].modifiers.reverse();
    let reordered = w.report();
    assert_eq!(
        deliveries(&reordered),
        deliveries(&r),
        "storage order is not modifier order"
    );
}
#[test]
fn actual_scalar_programs_precede_local_consumption_and_missing_inputs_do_not_default() {
    let mut w = World::new();
    w.roll(
        "def.0000000000003189",
        quantity(1.5, "def.0000000000000001"),
    );
    assert_eq!(deliveries(&w.report())[0].value, known(3.0));
    w.f.build.items[0].modifiers[0]
        .rolls
        .retain(|r| r.slot.slot.key().as_str() != "def.0000000000003173");
    assert!(w.plan().is_err(), "required raw input cannot become zero");
}
#[test]
fn signed_effective_boundary_is_not_clamped_and_unknown_local_values_block_delivery() {
    let mut w = World::new();
    let p =
        w.f.owner_mut(&modifier_owner())
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key("effective-amount"))
            .unwrap();
    *p = RuleProgram {
        id: key("effective-amount"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes: vec![node(
            "diagnostic",
            RuleExpression::Literal {
                value: quantity(-2.0, "def.000000000000295a"),
            },
        )],
        effects: vec![effect(
            "effective",
            RuleEffectKind::Derive {
                entity: RuleEntity::Modifier,
                stat: def("def.000000000000295b"),
                value: key("diagnostic"),
            },
        )],
    };
    assert_eq!(
        deliveries(&w.report())[0].value,
        known(-2.0),
        "diagnostic effective-value boundary, not source admission"
    );
    w.f.owner_mut(&modifier_owner())
        .programs
        .members
        .retain(|p| p.id != key("effective-amount"));
    let p = w.plan().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    match r.outcome {
        SupportEffectsOutcome::Evaluated { effects } => {
            let delivered = deliveries(&effects);
            assert_eq!(delivered.len(), 1);
            assert!(
                matches!(
                    delivered[0].value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ),
                "missing effective producer must remain missing through local reduction and delivery"
            );
        }
        SupportEffectsOutcome::Unavailable { cause, .. } => {
            assert!(
                matches!(
                    cause,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ),
                "only the removed effective producer may make this component unavailable"
            );
        }
        SupportEffectsOutcome::PreparationUnresolved { target, reason, .. } => {
            panic!(
                "unrelated support preparation cannot satisfy missing-producer control: {target:?}: {reason:?}"
            );
        }
    }
}
#[test]
fn selected_receiving_use_and_partial_membership_remain_independent() {
    let mut w = World::new();
    w.f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    assert!(deliveries(&w.report()).is_empty());
    w.f.build.active_weapon_loadout = occurrence(2);
    assert_eq!(deliveries(&w.report())[0].value, known(2.0));
    w.query = rebound(&packet::consumer().query);
    let p = w.plan().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        matches!(r.outcome, SupportEffectsOutcome::Unavailable { .. }),
        "authored Partial membership cannot be an empty proof"
    );
}
#[test]
fn fresh_reused_and_rayon_workers_observe_identical_exact_source_results() {
    let w = World::new();
    let p = w.plan().unwrap();
    let expected = p.evaluate(&mut p.new_scratch()).unwrap();
    let mut scratch = p.new_scratch();
    for _ in 0..3 {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    }
    (0..8)
        .into_par_iter()
        .for_each(|_| assert_eq!(p.evaluate(&mut p.new_scratch()).unwrap(), expected));
}
#[test]
fn query_requests_do_not_create_capacity_and_partial_owner_still_blocks_coverage() {
    let mut w = World::new();
    let before = w.report();
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Metric(DefinitionEntry {
            id: def("fixture-capacity-query"),
            schema: SchemaState::Known(MetricSchema {
                targets: vec![MetricTargetKind::Actor],
                unit: def("def.000000000000295a"),
                actor_roles: vec![MetricActorRole::Player],
                provider_roles: vec![ProviderRole::Character],
            }),
        }));
    w.f.queries.requests.push(MetricRequest {
        id: QueryId::new("fixture-capacity-query").unwrap(),
        metric: def("fixture-capacity-query"),
        target: MetricTarget::Actor(ActorKey::Player),
    });
    assert_eq!(deliveries(&w.report()), deliveries(&before));
    w.f.owner_mut(&modifier_owner()).programs.closure =
        rebound(&packet::dependencies().owners[0].programs.closure);
    let plan = w.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}
#[test]
fn wrong_source_membership_rejects_and_known_empty_local_list_keeps_only_base() {
    let mut w = World::new();
    w.f.build.items[0].modifiers.clear();
    w.f.build.items[0].modifier_orders.contributions.clear();
    w.f.build.items[0]
        .modifier_orders
        .numeric_transforms
        .clear();
    assert_eq!(deliveries(&w.report())[0].value, known(0.0));
    let mut w = World::new();
    let ContributionProducer::ProgramEffect(p) = &mut w.query.groups[0].members.members[0].producer
    else {
        panic!()
    };
    p.origin = ContributionOrigin::EquipmentUse {
        slots: vec![def("weapon")],
    };
    assert!(
        w.plan().is_err(),
        "item modifier cannot impersonate equipment owner"
    );
}
#[test]
fn alternative_equipment_uses_keep_local_capacity_and_player_delivery_isolated() {
    let mut w = World::new();
    w.f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(1)],
    };
    let mut alternate_item = w.f.build.items[0].clone();
    alternate_item.id = occurrence(20);
    alternate_item.modifiers[0].id = occurrence(21);
    alternate_item.modifier_orders.contributions = vec![occurrence(21)];
    alternate_item.modifier_orders.numeric_transforms = vec![occurrence(21)];
    alternate_item.modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot.slot.key().as_str() == "def.0000000000003173")
        .unwrap()
        .value = quantity(5.0, "def.000000000000295a");
    w.f.build.items.push(alternate_item);
    let mut alternate_use = w.f.build.equipment[0].clone();
    alternate_use.id = occurrence(22);
    alternate_use.item = occurrence(20);
    alternate_use.scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    w.f.build.equipment.push(alternate_use);

    let check = |world: &World,
                 equipment_use: ItemSlotUseId,
                 modifier: ModifierInstanceId,
                 expected: f64| {
        let report = world.report();
        let local: Vec<_> = report
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key("local-charm-capacity-contribution"))
            .collect();
        assert_eq!(local.len(), 1, "inactive alternative must not contribute");
        assert_eq!(local[0].value, known(expected));
        assert_eq!(
            local[0].key.invocation.origin,
            RuleOrigin::Provider {
                provider: ProviderKey {
                    root: ProviderRoot::ItemModifier {
                        equipment_use,
                        modifier,
                    },
                    grant_path: vec![],
                },
            }
        );
        assert_eq!(
            local[0].target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::EquipmentUse(equipment_use),
                    stat: def(packet::LOCAL),
                    kind: ContributionKind::Add,
                },
            }
        );
        let capacities: Vec<_> = report
            .effects
            .iter()
            .filter(|e| {
                e.key.invocation.program == key("item-charm-capacity-delivery")
                    && e.key.effect == key("derived-capacity")
            })
            .collect();
        assert_eq!(capacities.len(), 1);
        assert_eq!(capacities[0].value, known(expected));
        assert_eq!(
            capacities[0].target,
            BoundEffectTarget::Value {
                key: PlanValueKey::Stat {
                    entity: ConcreteEntity::EquipmentUse(equipment_use),
                    stat: def(packet::CAPACITY),
                },
            }
        );
        let delivered = deliveries(&report);
        assert_eq!(
            delivered.len(),
            1,
            "one Player delivery from the active use only"
        );
        assert_eq!(delivered[0].value, known(expected));
        assert_eq!(
            delivered[0].key.invocation.entity,
            ConcreteEntity::EquipmentUse(equipment_use)
        );
        assert_eq!(
            delivered[0].target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(packet::PLAYER),
                    kind: ContributionKind::Add,
                },
            }
        );
    };
    check(&w, occurrence(6), occurrence(4), 2.0);
    w.f.build.active_weapon_loadout = occurrence(2);
    check(&w, occurrence(22), occurrence(21), 5.0);
    w.f.build.items.reverse();
    w.f.build.equipment.reverse();
    check(&w, occurrence(22), occurrence(21), 5.0);
    w.f.build.active_weapon_loadout = occurrence(1);
    check(&w, occurrence(6), occurrence(4), 2.0);
}

// Deliberately finite transform suppliers. These are fixture boundaries, not
// claims that a particular game item/source is imported or fully implemented.
// The four released 3172 scalar programs and both delivery programs stay exact.
fn add_charm_transform(
    w: &mut World,
    definition: &str,
    id: u64,
    operation: ModifierTransformOperation,
    amount: f64,
) {
    let DefinitionDescriptor::Modifier(mut descriptor) =
        rebound(&packet::dependencies().modifier_descriptor)
    else {
        panic!("released modifier descriptor")
    };
    descriptor.id = def(definition);
    let SchemaState::Known(schema) = &mut descriptor.schema else {
        panic!()
    };
    schema.declarations.parameters.members.clear();
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Modifier(descriptor));
    for row in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(row) = row
            && row.id == def("item")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.modifiers.members.push(def(definition));
        }
    }
    w.f.owners.push(DefinitionRules {
        owner: subject(def::<ModifierDefinition>(definition)),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("fixture-charm-transform"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![node(
                "amount",
                RuleExpression::Literal {
                    value: quantity(amount, "def.0000000000000001"),
                },
            )],
            effects: vec![effect(
                "project",
                RuleEffectKind::ProjectModifierTransform {
                    stat: def("def.000000000000253d"),
                    targets: vec![ModifierTransformTarget {
                        definition: def("modifier"),
                        when: None,
                    }],
                    order: BoundedInteger::new(0).unwrap(),
                    operation,
                    value: key("amount"),
                },
            )],
        }]),
    });
    w.f.build.items[0].modifiers.push(RolledModifier {
        id: occurrence(id),
        definition: def(definition),
        rolls: vec![],
    });
    w.f.build.items[0]
        .modifier_orders
        .contributions
        .push(occurrence(id));
    w.f.build.items[0]
        .modifier_orders
        .numeric_transforms
        .push(occurrence(id));
}
fn charm_modifier_value(report: &OwnedEffectsReport, stat: &str) -> EffectValue {
    let wanted = PlanValueKey::Stat {
        entity: ConcreteEntity::Modifier(ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: occurrence(6),
                modifier: occurrence(4),
            },
            grant_path: vec![],
        }),
        stat: def(stat),
    };
    let rows: Vec<_> = report.values.iter().filter(|r| r.key == wanted).collect();
    assert_eq!(rows.len(), 1, "exact released modifier occurrence/stat");
    rows[0].value.clone()
}
fn assert_charm_scalar_chain(
    report: &OwnedEffectsReport,
    corruption: f64,
    magnitude: f64,
    amount: f64,
) {
    for (stat, value) in [
        ("def.0000000000000a1b", 1.0),
        ("def.0000000000002541", corruption),
        ("def.000000000000253d", magnitude),
    ] {
        assert_eq!(
            charm_modifier_value(report, stat),
            EffectValue::Known {
                value: quantity(value, "def.0000000000000001"),
            }
        );
    }
    assert_eq!(
        charm_modifier_value(report, "def.000000000000295b"),
        known(amount)
    );
    let local: Vec<_> = report
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("local-charm-capacity-contribution"))
        .collect();
    assert_eq!(local.len(), 1);
    assert_eq!(local[0].value, known(amount));
    assert_eq!(
        local[0].target,
        BoundEffectTarget::Contribution {
            key: ContributionKey {
                entity: ConcreteEntity::EquipmentUse(occurrence(6)),
                stat: def(packet::LOCAL),
                kind: ContributionKind::Add
            },
        }
    );
    let delivered = deliveries(report);
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].value, known(amount));
}

#[test]
fn released_charm_scalar_rounding_precedes_corruption_then_actual_magnitude_transform() {
    // Exact arithmetic examples, not a source-text admission claim. In
    // particular, zero/signed effective values do not establish parser coverage.
    for (raw, corruption, magnitude, expected) in [
        (2.0, 1.5, 1.0, 3.0),
        (2.0, 1.5, 1.5, 4.0),
        (1.49, 1.5, 1.5, 3.0),
        (1.5, 1.5, 1.5, 4.0),
        (2.0, 1.0, 0.5, 1.0),
        (2.0, 1.5, -1.0, -3.0),
        (2.0, 0.0, 1.5, 0.0),
    ] {
        let mut w = World::new();
        w.raw(raw);
        w.roll(
            "def.0000000000003189",
            quantity(corruption, "def.0000000000000001"),
        );
        add_charm_transform(
            &mut w,
            "fixture-charm-multiply",
            90,
            ModifierTransformOperation::Multiply,
            magnitude,
        );
        assert_charm_scalar_chain(&w.report(), corruption, magnitude, expected);
    }
}

#[test]
fn released_charm_delivery_observes_modifier_transform_order_and_ignores_storage_order() {
    let mut w = World::new();
    w.roll(
        "def.0000000000003189",
        quantity(1.5, "def.0000000000000001"),
    );
    add_charm_transform(
        &mut w,
        "fixture-charm-add",
        90,
        ModifierTransformOperation::Add,
        0.5,
    );
    add_charm_transform(
        &mut w,
        "fixture-charm-multiply",
        91,
        ModifierTransformOperation::Multiply,
        2.0,
    );
    // raw 2 -> corruption 3 -> magnitude ((1 + .5) * 2) -> delivered 9.
    let first = w.report();
    assert_charm_scalar_chain(&first, 1.5, 3.0, 9.0);
    w.f.build.items[0].modifiers.reverse();
    w.f.owners.reverse();
    assert_charm_scalar_chain(&w.report(), 1.5, 3.0, 9.0);
    // The actual semantic order changes the scalar and the final truncation:
    // magnitude ((1 * 2) + .5) -> trunc(3 * 2.5) -> delivered 7.
    w.f.build.items[0].modifier_orders.numeric_transforms =
        vec![occurrence(4), occurrence(91), occurrence(90)];
    assert_charm_scalar_chain(&w.report(), 1.5, 2.5, 7.0);
}

#[test]
fn released_charm_transform_dependencies_cannot_default_missing_or_partial_suppliers() {
    for partial in [false, true] {
        let mut w = World::new();
        w.roll(
            "def.0000000000003189",
            quantity(1.5, "def.0000000000000001"),
        );
        add_charm_transform(
            &mut w,
            "fixture-charm-multiply",
            90,
            ModifierTransformOperation::Multiply,
            1.5,
        );
        let owner = subject(def::<ModifierDefinition>("fixture-charm-multiply"));
        if partial {
            w.f.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: owner.clone(),
                    facet: SchemaFacet::GameRules,
                    code: key("fixture-transform-suppliers-unproved"),
                }],
            };
        } else {
            w.f.schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: def("fixture-missing-transform-input"),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Quantity {
                            unit: def("def.0000000000000001"),
                        },
                        targets: vec![RuleEntityKind::Modifier],
                    }),
                }));
            let producer = &mut w.f.owner_mut(&owner).programs.members[0];
            producer.reads.push(RuleRead {
                id: key("missing"),
                value_type: ComputedValueType::Quantity {
                    unit: def("def.0000000000000001"),
                },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Modifier,
                    stat: def("fixture-missing-transform-input"),
                },
            });
            producer.nodes[0].expression = RuleExpression::Read {
                input: key("missing"),
            };
        }
        let plan = w.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        if partial {
            assert!(report.gaps.contains(&PlanGap {
                provider: Some(ProviderKey {
                    root: ProviderRoot::ItemModifier {
                        equipment_use: occurrence(6),
                        modifier: occurrence(90),
                    },
                    grant_path: vec![],
                }),
                subject: Some(owner),
                reason: PlanGapReason::PartialPrograms,
            }));
        }
        match report.outcome {
            SupportEffectsOutcome::Evaluated { effects } => {
                assert!(
                    !partial,
                    "Partial transform suppliers must keep whole coverage unavailable"
                );
                assert!(matches!(
                    charm_modifier_value(&effects, "def.000000000000253d"),
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ));
                let delivered = deliveries(&effects);
                assert_eq!(delivered.len(), 1);
                assert!(matches!(
                    delivered[0].value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ));
            }
            SupportEffectsOutcome::Unavailable { cause, .. } => {
                if partial {
                    assert!(matches!(
                        cause,
                        EffectValue::Unresolved {
                            reason: PlanGapReason::IncompleteContributors,
                            ..
                        }
                    ));
                } else {
                    assert!(
                        matches!(
                            cause,
                            EffectValue::Unresolved {
                                reason: PlanGapReason::MissingProducer,
                                ..
                            }
                        ),
                        "only the deliberately missing transform dependency may block this finite chain"
                    );
                }
            }
            SupportEffectsOutcome::PreparationUnresolved { target, reason, .. } => {
                panic!(
                    "unrelated preparation gap cannot satisfy transform refusal: {target:?}: {reason:?}"
                );
            }
        }
    }
}
