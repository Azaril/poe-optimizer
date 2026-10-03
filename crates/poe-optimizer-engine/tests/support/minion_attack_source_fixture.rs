//! Component fixture only. Actual owned population, actor baseline and ability
//! programs retain their IDs and arithmetic. The sole synthetic producer projects
//! an explicitly supplied final summoning level; it is never a production default.
#![allow(dead_code)]
#[path = "owned_plan_fixture.rs"]
mod base;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::Arc};

pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
pub fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn named<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), s).unwrap()
}
pub fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, n: u64) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: def(n),
    }
}
pub fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
pub fn asset<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/minion-attack-source")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn transcode<T: Serialize + DeserializeOwned>(v: &T, finite: bool) -> T {
    fn walk(v: &mut Value, finite: bool) {
        match v {
            Value::Object(o) => {
                if o.get("game").and_then(Value::as_str) == Some("owned-plan-test") {
                    *v = json!({"game":"poe2", "version":"owned-mechanics-v1"});
                    return;
                }
                if finite && o.contains_key("members") && o.contains_key("closure") {
                    o.insert("closure".into(), json!({"kind":"complete"}));
                }
                for v in o.values_mut() {
                    walk(v, finite);
                }
            }
            Value::Array(a) => {
                for v in a {
                    walk(v, finite);
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(v).unwrap();
    walk(&mut v, finite);
    serde_json::from_value(v).unwrap()
}
pub fn actor_slot() -> DeclaredSlot<ActorSlotDefId> {
    slot(SlotOwnerDefId::Skill(def(0x12)), 0x1f)
}
fn primary_grant() -> DeclaredSlot<GrantSlotDefId> {
    slot(SlotOwnerDefId::Gem(def(0x11)), 0x17)
}
pub fn actor(index: usize) -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(base::occurrence(30 + index as u64)),
            grant_path: vec![primary_grant()],
        },
        slot: actor_slot(),
    }))
}
pub fn selected(index: usize) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: actor(index),
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(base::occurrence(30 + index as u64)),
                grant_path: vec![
                    primary_grant(),
                    slot(SlotOwnerDefId::Skill(def(0x12)), 0x20),
                    slot(SlotOwnerDefId::Actor(def(0x3091)), 0x3093),
                ],
            },
            output: slot(SlotOwnerDefId::Skill(def(0x21)), 0x22),
        },
        part: def(7),
        mode: def(8),
        stat_set: def(9),
    }
}
pub struct World {
    pub f: base::Fixture,
}
impl World {
    pub fn new(levels: [u16; 2]) -> Self {
        let mut f = base::Fixture::new();
        f.schema = transcode(&f.schema, false);
        f.build = transcode(&f.build, false);
        f.scenario = transcode(&f.scenario, false);
        f.queries = transcode(&f.queries, false);
        f.owners.clear();
        f.tables.clear();
        f.routes.clear();
        f.receivers = DeclaredSet::complete(vec![]);
        f.build.items.clear();
        f.build.equipment.clear();
        f.build.character.level = 100;
        let snapshot: Value =
            serde_json::from_str(include_str!("minion_attack_source_snapshot.json")).unwrap();
        let descriptors: Vec<DefinitionDescriptor> =
            serde_json::from_value(snapshot["schema_definitions"].clone()).unwrap();
        let slots: Vec<SlotDescriptor> =
            serde_json::from_value(snapshot["schema_slots"].clone()).unwrap();
        f.schema.definitions.extend(transcode(&descriptors, true));
        f.schema.slots.extend(transcode(&slots, true));
        let extension: Value = asset("extension.json");
        for row in extension["schema"].as_array().unwrap() {
            assert_eq!(row["kind"], "definition");
            f.schema
                .definitions
                .push(serde_json::from_value(row["value"].clone()).unwrap());
        }
        let owners: Vec<DefinitionRules> =
            serde_json::from_value(snapshot["owners"].clone()).unwrap();
        f.owners = transcode(&owners, true);
        f.tables = serde_json::from_value(snapshot["tables"].clone()).unwrap();
        let added: Vec<DefinitionRules> =
            serde_json::from_value(extension["owners"].clone()).unwrap();
        for row in added {
            f.owner_mut(&row.owner)
                .programs
                .members
                .extend(row.programs.members);
        }
        // Physical finalization is outside this component. The finite world makes
        // final summoning level explicit per physical occurrence instead of
        // claiming the real unsupported producer is implemented.
        let gem = SchemaSubject::Definition(DefinitionAddress::Gem(def(0x11)));
        let parent = f.owner_mut(&gem);
        parent
            .programs
            .members
            .retain(|p| p.id.as_str() == "primary-supply");
        parent.programs.members.push(RuleProgram {
            id: key("fixture-explicit-final-level"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("level"),
                value_type: ComputedValueType::Integer,
                source: RuleReadSource::GemLevel,
            }],
            nodes: vec![
                RuleNode {
                    id: key("level"),
                    expression: RuleExpression::Read {
                        input: key("level"),
                    },
                },
                RuleNode {
                    id: key("quality"),
                    expression: RuleExpression::Literal {
                        value: quantity(0., 2),
                    },
                },
            ],
            effects: vec![
                RuleEffect {
                    id: key("level"),
                    when: None,
                    effect: RuleEffectKind::ProjectSkillParameter {
                        skill: slot(SlotOwnerDefId::Gem(def(0x11)), 0x16),
                        parameter: slot(SlotOwnerDefId::Skill(def(0x12)), 0x13),
                        value: key("level"),
                    },
                },
                RuleEffect {
                    id: key("quality"),
                    when: None,
                    effect: RuleEffectKind::ProjectSkillParameter {
                        skill: slot(SlotOwnerDefId::Gem(def(0x11)), 0x16),
                        parameter: slot(SlotOwnerDefId::Skill(def(0x12)), 0x14),
                        value: key("quality"),
                    },
                },
            ],
        });
        // Missing unrelated behaviors are certified empty only in this fixture.
        let mut subjects: Vec<_> = f
            .schema
            .definitions
            .iter()
            .filter_map(|d| match d {
                DefinitionDescriptor::Class(_)
                | DefinitionDescriptor::Encounter(_)
                | DefinitionDescriptor::Gem(_)
                | DefinitionDescriptor::Skill(_)
                | DefinitionDescriptor::Actor(_) => Some(SchemaSubject::Definition(d.address())),
                _ => None,
            })
            .collect();
        subjects.extend(f.schema.slots.iter().filter_map(|s| match s {
            SlotDescriptor::Actor(_)
            | SlotDescriptor::Grant(_)
            | SlotDescriptor::SkillGrant(_)
            | SlotDescriptor::ActionOutput(_) => Some(SchemaSubject::Slot(s.address())),
            _ => None,
        }));
        for owner in subjects {
            if !f.owners.iter().any(|r| r.owner == owner) {
                f.owners.push(DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
        for (index, level) in levels.into_iter().enumerate() {
            f.build.gems.push(GemInstance {
                id: base::occurrence(28 + index as u64),
                definition: def(0x11),
                level,
                quality: None,
                parameters: vec![
                    ParameterAssignment {
                        slot: slot(SlotOwnerDefId::Gem(def(0x11)), 0x30aa),
                        value: quantity(0., 0x295a),
                    },
                    ParameterAssignment {
                        slot: slot(SlotOwnerDefId::Gem(def(0x11)), 0x30b1),
                        value: ParameterValue::Boolean(true),
                    },
                ],
            });
            f.build.skills.push(SkillUse {
                parameters: None,
                id: base::occurrence(30 + index as u64),
                source: AuthoredSkillSource::Gem(base::occurrence(28 + index as u64)),
                enabled: true,
                scope: LoadoutScope::Shared,
            });
            f.queries.requests.push(MetricRequest {
                id: QueryId::new(format!("intrinsic-{index}")).unwrap(),
                metric: named("requested"),
                target: MetricTarget::Action(Box::new(selected(index))),
            });
            // The actual parent owner also contains an Action-context reservation
            // program. Explicitly request that context in this finite component
            // world; do not delete its program or rewrite real build requests.
            f.queries.requests.push(MetricRequest {
                id: QueryId::new(format!("parent-context-{index}")).unwrap(),
                metric: named("requested"),
                target: MetricTarget::Action(Box::new(ActionSelection {
                    action: ActionKey {
                        actor: ActorKey::Player,
                        provider: ProviderKey {
                            root: ProviderRoot::SkillUse(base::occurrence(30 + index as u64)),
                            grant_path: vec![primary_grant()],
                        },
                        output: slot(SlotOwnerDefId::Skill(def(0x12)), 0x15),
                    },
                    part: def(7),
                    mode: def(8),
                    stat_set: def(9),
                })),
            });
        }
        for d in &mut f.schema.definitions {
            if let DefinitionDescriptor::Metric(e) = d
                && let SchemaState::Known(s) = &mut e.schema
            {
                s.actor_roles.push(MetricActorRole::Owned);
            }
        }
        let routes: Vec<ActionOutputRoutes> = asset("routes.json");
        f.routes = transcode(&routes, true);
        for route in &mut f.routes {
            route.source_selectors = Some(DeclaredSet::complete(vec![]));
        }
        for s in &f.schema.slots {
            if let SlotDescriptor::ActionOutput(e) = s
                && !f.routes.iter().any(|r| r.output == e.id)
            {
                f.routes.push(ActionOutputRoutes {
                    output: e.id.clone(),
                    routes: DeclaredSet::complete(vec![]),
                    source_selectors: Some(DeclaredSet::complete(vec![])),
                });
            }
        }
        Self { f }
    }
    pub fn compile(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = RulePackageInput {
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("minion-component"),
            semantics_version: key("finite-test"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            owners: self.f.owners.clone(),
            tables: self.f.tables.clone(),
            receivers: self.f.receivers.clone(),
        };
        let rules = Arc::new(
            CompiledRulePackage::compile(&rules, schema.as_ref(), Default::default()).unwrap(),
        );
        let routes = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("finite-routes"),
                    definitions: schema.identity().clone(),
                    outputs: self.f.routes.clone(),
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(self.f.request()),
            schema,
            rules,
            routes,
            Default::default(),
        )
        .unwrap()
    }
    pub fn evaluate(&self) -> OwnedEffectsReport {
        let p = self.compile();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    pub fn baseline_mut(&mut self) -> &mut RuleProgram {
        self.f
            .owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(actor_slot())))
            .programs
            .members
            .iter_mut()
            .find(|p| p.id.as_str() == "finite-actor-baseline")
            .unwrap()
    }
    pub fn set_baseline(&mut self, node: &str, value: ParameterValue) {
        self.baseline_mut()
            .nodes
            .iter_mut()
            .find(|n| n.id.as_str() == node)
            .unwrap()
            .expression = RuleExpression::Literal { value };
    }
    pub fn missing_final_input(&mut self) {
        self.f
            .owner_mut(&SchemaSubject::Definition(DefinitionAddress::Gem(def(
                0x11,
            ))))
            .programs
            .members
            .retain(|p| p.id.as_str() != "fixture-explicit-final-level");
    }
}
pub fn value(report: &OwnedEffectsReport, entity: ConcreteEntity, stat: u64) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: entity.clone(),
                    stat: def(stat),
                }
        })
        .unwrap_or_else(|| panic!("missing stat {stat:x} for {entity:?}"))
        .value
}
pub fn actor_value(report: &OwnedEffectsReport, index: usize, stat: u64) -> &EffectValue {
    value(report, ConcreteEntity::Actor(actor(index)), stat)
}
pub fn action_value(report: &OwnedEffectsReport, index: usize, stat: u64) -> &EffectValue {
    value(
        report,
        ConcreteEntity::Action(Box::new(selected(index))),
        stat,
    )
}
pub fn known(value: &EffectValue) -> f64 {
    match value {
        EffectValue::Known {
            value: ParameterValue::Quantity(q),
        } => q.value(),
        _ => panic!("expected known quantity: {value:?}"),
    }
}
