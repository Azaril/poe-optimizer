//! Same authored ingredient, finite explicit contributor closure, no final metric claim.
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod base;
use base as support;
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_empty_support_contribution_fixture.rs"]
mod checked_empty_support;
#[allow(dead_code)]
#[path = "support/owned_movement_speed_penalty.rs"]
mod packet;
use base::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::Arc;
fn rebound<T: Serialize + DeserializeOwned>(input: &T) -> T {
    fn visit(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if o.contains_key("namespace") && o.contains_key("key") {
                    o.insert("namespace".into(), serde_json::to_value(ns()).unwrap());
                    if o["kind"] == "equipment_slot" && o["key"] == "def.0000000000000067" {
                        o.insert("key".into(), Value::String("weapon".into()));
                    }
                }
                for v in o.values_mut() {
                    visit(v)
                }
            }
            Value::Array(a) => {
                for v in a {
                    visit(v)
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(input).unwrap();
    visit(&mut v);
    serde_json::from_value(v).unwrap()
}
struct World {
    f: Fixture,
    query: ContributionQuery,
    application: ExistingActorRuleApplication,
}
impl World {
    fn new() -> Self {
        packet::check_authored();
        let p = packet::consumer();
        let d = packet::dependencies();
        let mut f = Fixture::new();
        // This equipment-only component has no Skill or support occurrences.
        // Remove the base fixture's unused Action scaffold before using the
        // existing checked empty-support stage/readiness construction.
        f.schema
            .definitions
            .retain(|row| !matches!(row, DefinitionDescriptor::Skill(_)));
        f.schema
            .slots
            .retain(|row| !matches!(row, SlotDescriptor::ActionOutput(_)));
        f.owners.retain(|row| {
            !matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::Skill(_))
                    | SchemaSubject::Slot(SlotAddress::ActionOutput(_))
            )
        });
        f.schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: def("percent"),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                }),
            }));
        for owner in &mut f.owners {
            owner.programs = DeclaredSet::complete(vec![])
        }
        f.receivers = DeclaredSet::complete(vec![]);
        for mut definition in d.definitions.into_iter().chain(p.definitions) {
            if let DefinitionDescriptor::EquipmentSlot(_) = definition {
                continue;
            }
            if let DefinitionDescriptor::Stat(e) = &mut definition {
                if e.id.key().as_str() == packet::RAW {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.value = ComputedValueType::Quantity {
                        unit: UnitDefId::parse(e.id.namespace().clone(), "def.0000000000000001")
                            .unwrap(),
                    };
                }
            }
            f.schema.definitions.push(rebound(&definition));
        }
        let raw = rebound(&d.raw_profiles[0].program);
        f.owner_mut(&item_owner()).programs.members.push(raw);
        f.owners.push(DefinitionRules {
            owner: rebound(&p.owner),
            programs: DeclaredSet::complete(vec![rebound(&p.program)]),
        });
        f.schema
            .definitions
            .push(DefinitionDescriptor::Metric(DefinitionEntry {
                id: def("fixture-player-query"),
                schema: SchemaState::Known(MetricSchema {
                    targets: vec![MetricTargetKind::Actor],
                    unit: def("def.0000000000000001"),
                    actor_roles: vec![MetricActorRole::Player],
                    provider_roles: vec![ProviderRole::Character],
                }),
            }));
        f.queries.requests.push(MetricRequest {
            id: QueryId::new("fixture-player-query").unwrap(),
            metric: def("fixture-player-query"),
            target: MetricTarget::Actor(ActorKey::Player),
        });
        let mut query: ContributionQuery = rebound(&p.query);
        query.groups[0].members = DeclaredSet::complete(vec![]);
        Self {
            f,
            query,
            application: rebound(&d.existing_actor_application),
        }
    }
    fn raw_mut(&mut self) -> &mut RuleProgram {
        self.f
            .owner_mut(&item_owner())
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key("raw-base-defence-profile"))
            .unwrap()
    }
    fn raw(&mut self, amount: f64) {
        let p = self.raw_mut();
        let n = p.nodes.iter_mut().find(|n| n.id == key("value-4")).unwrap();
        n.expression = RuleExpression::Literal {
            value: ParameterValue::Quantity(
                FiniteQuantity::new(amount, def("def.0000000000000001")).unwrap(),
            ),
        };
    }
    fn ignore(&mut self, value: Option<bool>) {
        let expression = if let Some(value) = value {
            RuleExpression::Literal {
                value: ParameterValue::Boolean(value),
            }
        } else {
            self.f
                .schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: def("unknown-immunity"),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Boolean,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
            RuleExpression::Read {
                input: key("unknown"),
            }
        };
        let p = RuleProgram {
            id: key("fixture-immunity"),
            context: RuleEntityKind::Actor,
            reads: if value.is_some() {
                vec![]
            } else {
                vec![RuleRead {
                    id: key("unknown"),
                    value_type: ComputedValueType::Boolean,
                    source: RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("unknown-immunity"),
                    },
                }]
            },
            nodes: vec![node("value", expression)],
            effects: vec![effect(
                "flag",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def(packet::IGNORE),
                    contribution: ContributionKind::Flag,
                    value: key("value"),
                },
            )],
        };
        self.f.owner_mut(&class_owner()).programs.members.push(p);
        self.query.groups[0]
            .members
            .members
            .push(ContributionMember {
                producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    owner: class_owner(),
                    program: key("fixture-immunity"),
                    effect: key("flag"),
                    origin: ContributionOrigin::Character,
                }),
                order: None,
            });
    }
    fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
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
            existing_actor_rules: Some(DeclaredSet::complete(vec![self.application.clone()])),
            support_discovery: Some(assignment_only_domains(&f.owners)),
        };
        let routing = ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("routing"),
            definitions: schema.identity().clone(),
            outputs: f.routes.clone(),
        };
        checked_empty_support::checked_plan(f, rules, routing, Default::default()).unwrap_or_else(
            |error| {
                let bindings = poe_optimizer_core::owned_binding::bind_owned_request(
                    &schema,
                    &f.request(),
                    Default::default(),
                )
                .unwrap();
                panic!("{error:?}; request bindings: {:?}", bindings.issues())
            },
        )
    }
    fn report(&self) -> OwnedEffectsReport {
        let p = self.plan();
        let report = p.evaluate(&mut p.new_scratch()).unwrap();
        let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
            panic!("explicitly empty support domain must finish this component");
        };
        effects
    }
}
fn result(r: &OwnedEffectsReport) -> &EffectValue {
    let e: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(packet::PROGRAM))
        .collect();
    assert_eq!(e.len(), 1);
    assert_eq!(
        e[0].key.invocation.entity,
        ConcreteEntity::Actor(ActorKey::Player)
    );
    &e[0].value
}
fn known(n: f64) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Quantity(
            FiniteQuantity::new(n, def("def.0000000000000001")).unwrap(),
        ),
    }
}
#[test]
fn source_fraction_is_negated_without_percent_rescaling_and_immunity_is_checked() {
    let w = World::new();
    assert_eq!(result(&w.report()), &known(-0.03));
    for (amount, ignore, expected) in [
        (0.05, false, -0.05),
        (0.0, false, 0.0),
        (0.03, true, 0.0),
        (0.03, false, -0.03),
    ] {
        let mut w = World::new();
        w.raw(amount);
        w.ignore(Some(ignore));
        assert_eq!(result(&w.report()), &known(expected));
    }
}
#[test]
fn unknown_condition_and_missing_profile_are_not_neutral_and_empty_slots_are_lazy() {
    let mut w = World::new();
    w.ignore(None);
    assert!(matches!(
        result(&w.report()),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new();
    w.raw_mut().nodes.retain(|n| n.id != key("value-4"));
    w.raw_mut().effects.retain(|e| e.id != key("raw-4"));
    assert!(matches!(
        result(&w.report()),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new();
    w.f.owner_mut(&item_owner()).programs.members.clear();
    assert!(matches!(
        result(&w.report()),
        EffectValue::Unresolved { .. }
    ));
    w.f.build.equipment.retain(|e| e.id != occurrence(6));
    w.ignore(None);
    assert_eq!(result(&w.report()), &known(0.0));
    let mut w = World::new();
    let d = packet::dependencies();
    w.f.owner_mut(&item_owner()).programs.members = vec![rebound(&d.raw_profiles[1].program)];
    w.ignore(None);
    assert_eq!(result(&w.report()), &known(0.0));
}
#[test]
fn authored_partial_supplier_inventory_stays_unavailable_instead_of_assuming_false() {
    let mut w = World::new();
    w.query = rebound(&packet::consumer().query);
    let plan = w.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                },
                ..
            }
        ),
        "incomplete membership must refuse execution: {:?}",
        report.outcome
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::IncompleteContributors)
    );
}
#[test]
fn only_exact_active_body_occupancy_contributes_and_ambiguity_is_unavailable() {
    let mut w = World::new();
    // A second physical item/template has a different independently computed raw value.
    let mut other_template =
        w.f.schema
            .definitions
            .iter()
            .find_map(|d| {
                if let DefinitionDescriptor::ItemTemplate(e) = d {
                    Some(e.clone())
                } else {
                    None
                }
            })
            .unwrap();
    other_template.id = def("alternate-item");
    let SchemaState::Known(template_schema) = &mut other_template.schema else {
        panic!()
    };
    template_schema.declarations.parameters = DeclaredSet::complete(vec![]);
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::ItemTemplate(other_template));
    let mut other_owner = w.f.owner_mut(&item_owner()).clone();
    other_owner.owner = subject(def::<ItemTemplateDefinition>("alternate-item"));
    let amount = other_owner.programs.members[0]
        .nodes
        .iter_mut()
        .find(|n| n.id == key("value-4"))
        .unwrap();
    amount.expression = RuleExpression::Literal {
        value: ParameterValue::Quantity(
            FiniteQuantity::new(0.07, def("def.0000000000000001")).unwrap(),
        ),
    };
    w.f.owners.push(other_owner);
    let mut other = w.f.build.items[0].clone();
    other.id = occurrence(83);
    other.template = def("alternate-item");
    other.parameters.clear();
    other.modifiers.clear();
    other.modifier_order.clear();
    w.f.build.items.push(other);
    let mut alternative = w.f.build.equipment[0].clone();
    alternative.id = occurrence(84);
    alternative.item = occurrence(83);
    alternative.scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    w.f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(1)],
    };
    w.f.build.equipment.push(alternative);
    assert_eq!(result(&w.report()), &known(-0.03));
    w.f.build.active_weapon_loadout = occurrence(2);
    assert_eq!(result(&w.report()), &known(-0.07));
    w.f.build.equipment.reverse();
    assert_eq!(result(&w.report()), &known(-0.07));
    w.f.build
        .equipment
        .iter_mut()
        .find(|e| e.id == occurrence(84))
        .unwrap()
        .scope = LoadoutScope::Shared;
    w.f.build.active_weapon_loadout = occurrence(1);
    assert!(matches!(
        result(&w.report()),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new();
    w.f.build.equipment.retain(|e| e.id != occurrence(6));
    assert_eq!(
        result(&w.report()),
        &known(0.0),
        "other occupied slot cannot donate penalty"
    );
}
#[test]
fn query_choice_fresh_reused_and_parallel_scratch_preserve_exact_component() {
    let a = World::new();
    let expected = a.report();
    let mut b = World::new();
    assert_eq!(a.f.queries.requests.len(), 1);
    b.f.queries.requests.clear();
    assert!(b.f.queries.requests.is_empty());
    assert_eq!(result(&b.report()), result(&expected));
    let plans = [Arc::new(a.plan()), Arc::new(b.plan())];
    assert_ne!(
        plans[0].identity(),
        plans[1].identity(),
        "nonempty and empty requests remain distinct"
    );
    let reports: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for i in [0, 1, 0] {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), reports[i]);
    }
    let out: Vec<_> = (0..12)
        .into_par_iter()
        .map_init(
            || plans[0].new_scratch(),
            |s, i| (i % 2, plans[i % 2].evaluate(s).unwrap()),
        )
        .collect();
    for (i, r) in out {
        assert_eq!(r, reports[i]);
    }
}
