//! Support-aware quantity metrics over the same complete native delivery fixture.
#[allow(dead_code)]
#[path = "owned_support_delivery_fixture.rs"]
pub mod delivery;
pub use delivery::{Fixture, child_actor, def, fixture, integer, key, occurrence};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_rules::*, owned_schema::*,
    owned_support_receiving::*, owned_supports::SupportPreparationInput,
};
use poe_optimizer_data::{owned_metrics::*, owned_schema::OwnedDefinitionSchemaPackage};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use std::sync::Arc;

pub type Plan = OwnedSupportMetricPlan<OwnedDefinitionSchemaPackage>;
pub fn quantity(value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def("count")).unwrap())
}
pub fn action(use_id: u64, name: &str, part: &str) -> ActionSelection {
    let SkillTarget::Generated(target) = fixture::target(use_id, name) else {
        unreachable!()
    };
    let mut provider = target.provider;
    provider.grant_path.push(DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    });
    ActionSelection {
        action: ActionKey {
            actor: child_actor(use_id),
            provider,
            output: delivery::output(),
        },
        part: def(part),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
pub fn query(id: &str, target: MetricTarget) -> MetricRequest {
    MetricRequest {
        id: QueryId::new(id).unwrap(),
        metric: def("requested"),
        target,
    }
}
pub fn fixture() -> Fixture {
    let mut f = delivery::source_fixture();
    for row in &mut f.schema.definitions {
        if let DefinitionDescriptor::Metric(row) = row {
            row.schema = SchemaState::Known(MetricSchema {
                targets: vec![MetricTargetKind::Actor, MetricTargetKind::Action],
                unit: def("count"),
                actor_roles: vec![MetricActorRole::Player, MetricActorRole::Owned],
                provider_roles: vec![ProviderRole::Character, ProviderRole::SkillUse],
            });
        }
    }
    for name in ["support-quantity", "dynamic-quantity", "absent-quantity"] {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: def(name),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Quantity { unit: def("count") },
                    targets: vec![RuleEntityKind::Actor, RuleEntityKind::Action],
                }),
            }));
    }
    for owner in &mut f.owners {
        for program in &mut owner.programs.members {
            if [key("actor-final"), key("action-final")].contains(&program.id) {
                program.nodes.extend([
                    fixture::node(
                        "one-unit",
                        RuleExpression::Literal {
                            value: quantity(1.0),
                        },
                    ),
                    fixture::node(
                        "quantity",
                        RuleExpression::ScaleInteger {
                            value: key("one-unit"),
                            count: key("value"),
                        },
                    ),
                ]);
                program.effects.push(fixture::derive(
                    "quantity",
                    RuleEntity::Current,
                    "support-quantity",
                    "quantity",
                ));
            }
        }
    }
    f.queries.requests = vec![
        query("actor-a", MetricTarget::Actor(child_actor(30))),
        query(
            "first-action",
            MetricTarget::Action(Box::new(action(30, "first", "part"))),
        ),
        query(
            "second-action",
            MetricTarget::Action(Box::new(action(30, "second", "part-two"))),
        ),
        query("actor-b", MetricTarget::Actor(child_actor(31))),
        query("actor-a-repeat", MetricTarget::Actor(child_actor(30))),
        query(
            "other-action",
            MetricTarget::Action(Box::new(action(31, "first", "part"))),
        ),
    ];
    f
}
pub fn mapping_input(definitions: &impl DefinitionSchemaIndex) -> MetricMappingInput {
    MetricMappingInput {
        schema_version: OWNED_METRIC_MAPPING_VERSION,
        namespace: fixture::ns(),
        release: key("mapping"),
        definitions: definitions.identity().clone(),
        bindings: vec![
            MetricStatBinding {
                metric: def("requested"),
                role: MetricBindingRole::OwnedActor,
                stat: def("support-quantity"),
            },
            MetricStatBinding {
                metric: def("requested"),
                role: MetricBindingRole::Action,
                stat: def("support-quantity"),
            },
        ],
    }
}
pub fn compile(f: &Fixture) -> Plan {
    compile_with(f, |_| {}, |_| {}, |_| {}, |_| {})
}
pub fn compile_with(
    f: &Fixture,
    rules: impl FnOnce(&mut RulePackageInput),
    preparation: impl FnOnce(&mut SupportPreparationInput),
    receiving: impl FnOnce(&mut SupportReceivingInput),
    mapping: impl FnOnce(&mut MetricMappingInput),
) -> Plan {
    let args = delivery::inputs_with_packages(f, rules, preparation, receiving);
    let mut input = mapping_input(args.definitions.as_ref());
    mapping(&mut input);
    let mapping = Arc::new(
        OwnedMetricMapping::new(
            input,
            args.definitions.as_ref(),
            MetricMappingLimits::default(),
        )
        .unwrap(),
    );
    let effects = Arc::new(
        OwnedSupportEffectPlan::compile(
            args,
            PlanLimits::default(),
            SupportPreparationLimits::default(),
        )
        .unwrap(),
    );
    OwnedSupportMetricPlan::compile(effects, mapping).unwrap()
}
pub fn number(row: &OwnedMetricResult) -> f64 {
    let EffectValue::Known {
        value: ParameterValue::Quantity(value),
    } = &row.value
    else {
        panic!("expected quantity: {row:?}")
    };
    assert_eq!(value.unit(), &def("count"));
    value.value()
}
