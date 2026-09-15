//! Shared direct-authored metric fixture: changed build facts flow through rules.
#[path = "owned_plan_fixture.rs"]
#[allow(dead_code)]
mod plan_fixture;
pub use plan_fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_metrics::*, owned_schema::OwnedDefinitionSchemaPackage};
use poe_optimizer_engine::owned_plan::*;
use std::sync::Arc;

fn quantity(value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def("count")).unwrap())
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn quantity_nodes(input: &str) -> Vec<RuleNode> {
    vec![
        node(
            "one-unit",
            RuleExpression::Literal {
                value: quantity(1.0),
            },
        ),
        node(
            "measurement",
            RuleExpression::ScaleInteger {
                value: key("one-unit"),
                count: key(input),
            },
        ),
    ]
}
pub fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.add_action_route();
    f.add_generated_actors();
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
    for (name, kind) in [
        ("player-quantity", RuleEntityKind::Actor),
        ("child-quantity", RuleEntityKind::Actor),
        ("action-quantity", RuleEntityKind::Action),
        ("absent-quantity", RuleEntityKind::Actor),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![kind],
            },
        )));
    }
    for (owner, context, source, target) in [
        (
            class_owner(),
            RuleEntityKind::Actor,
            "actor-total",
            "player-quantity",
        ),
        (
            subject(def::<SkillDefinition>("skill")),
            RuleEntityKind::Action,
            "routed",
            "action-quantity",
        ),
    ] {
        let mut nodes = vec![read_node("incoming", "incoming")];
        nodes.extend(quantity_nodes("incoming"));
        f.owner_mut(&owner).programs.members.push(RuleProgram {
            id: key("quantity"),
            context,
            reads: vec![read(
                "incoming",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def(source),
                },
            )],
            nodes,
            effects: vec![derive(
                "measure",
                RuleEntity::Current,
                target,
                "measurement",
            )],
        });
    }
    let parent = &mut f.owner_mut(&summoner_owner()).programs.members[0];
    parent.nodes.extend(quantity_nodes("level"));
    parent.effects.push(effect(
        "project-quantity",
        RuleEffectKind::ProjectActorStat {
            actor: child_slot(),
            stat: def("child-quantity"),
            value: key("measurement"),
        },
    ));
    f.queries.requests = vec![
        query("child-a", MetricTarget::Actor(child_actor(30))),
        query("player", MetricTarget::Actor(ActorKey::Player)),
        query("action", MetricTarget::Action(Box::new(action()))),
        query("child-b", MetricTarget::Actor(child_actor(31))),
        query("child-a-again", MetricTarget::Actor(child_actor(30))),
    ];
    f
}
pub fn query(id: &str, target: MetricTarget) -> MetricRequest {
    MetricRequest {
        id: QueryId::new(id).unwrap(),
        metric: def("requested"),
        target,
    }
}
pub fn mapping_input(index: &impl DefinitionSchemaIndex) -> MetricMappingInput {
    MetricMappingInput {
        schema_version: OWNED_METRIC_MAPPING_VERSION,
        namespace: ns(),
        release: key("mapping"),
        definitions: index.identity().clone(),
        bindings: vec![
            MetricStatBinding {
                metric: def("requested"),
                role: MetricBindingRole::PlayerActor,
                stat: def("player-quantity"),
            },
            MetricStatBinding {
                metric: def("requested"),
                role: MetricBindingRole::OwnedActor,
                stat: def("child-quantity"),
            },
            MetricStatBinding {
                metric: def("requested"),
                role: MetricBindingRole::Action,
                stat: def("action-quantity"),
            },
        ],
    }
}
pub fn compile(f: &Fixture) -> OwnedMetricPlan<OwnedDefinitionSchemaPackage> {
    let effects = Arc::new(f.compile().unwrap());
    let mapping = Arc::new(
        OwnedMetricMapping::new(
            mapping_input(effects.definitions()),
            effects.definitions(),
            MetricMappingLimits::default(),
        )
        .unwrap(),
    );
    OwnedMetricPlan::compile(effects, mapping).unwrap()
}
pub fn number(result: &OwnedMetricResult) -> f64 {
    let EffectValue::Known {
        value: ParameterValue::Quantity(q),
    } = &result.value
    else {
        panic!("{result:?}")
    };
    assert_eq!(q.unit(), &def("count"));
    q.value()
}
