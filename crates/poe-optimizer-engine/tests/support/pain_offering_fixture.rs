//! Finite component only: real Offering supply, level table and application.
//! Final supported inputs and usage/scaling are explicitly supplied fixture
//! boundaries, not implementations of the pending source-preference contracts.
#![allow(dead_code)]
#[path = "minion_attack_source_fixture.rs"]
pub mod intrinsic;
pub use intrinsic::{actor, actor_slot, def, key, known, ns, quantity, slot};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf, sync::Arc};

pub fn asset<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/pain-offering")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn finite<T: Serialize + DeserializeOwned>(value: &T) -> T {
    fn walk(value: &mut Value) {
        match value {
            Value::Object(map) => {
                if map.contains_key("members") && map.contains_key("closure") {
                    map.insert("closure".into(), serde_json::json!({"kind":"complete"}));
                }
                for child in map.values_mut() {
                    walk(child);
                }
            }
            Value::Array(rows) => rows.iter_mut().for_each(walk),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    walk(&mut value);
    serde_json::from_value(value).unwrap()
}
pub fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([61; 16]), n).unwrap())
}
pub fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
fn entry<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn named<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(ns(), format!("fixture.pain-offering.{name}")).unwrap()
}
fn boundary_parameter(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def(0x86b)),
        slot: named(name),
    }
}
fn boundary_choice(skill: u64, name: &str) -> DeclaredSlot<ChoiceSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def(skill)),
        slot: named(name),
    }
}
fn quantity_schema(unit: u64) -> ValueSchema {
    ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(-1_000_000., def(unit)).unwrap(),
        maximum: FiniteQuantity::new(1_000_000., def(unit)).unwrap(),
    })
}
fn skill_target(use_id: u64, gem: u64, supply: u64) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(use_id)),
            grant_path: vec![],
        },
        slot: slot(SlotOwnerDefId::Gem(def(gem)), supply),
    }))
}
pub fn offering_target(index: usize) -> SkillTarget {
    skill_target(1_100 + index as u64, 0x86b, 0x3221)
}
pub fn recipient_target(index: usize) -> SkillTarget {
    skill_target(30 + index as u64, 0x11, 0x16)
}

#[derive(Clone, Debug)]
pub struct Offering {
    pub raw_level: u16,
    pub final_level: i64,
    pub final_quality: f64,
    pub active: Option<bool>,
    pub increased: f64,
    pub more: f64,
    pub magnitude: f64,
}
impl Offering {
    pub fn at_final_level(final_level: i64) -> Self {
        Self {
            raw_level: 20,
            final_level,
            final_quality: 0.,
            active: Some(true),
            increased: 0.,
            more: 1.,
            magnitude: 1.,
        }
    }
}

pub struct World {
    pub intrinsic: intrinsic::World,
    pub applications: DeclaredSet<EffectApplicationRule>,
}
impl World {
    pub fn new(offerings: &[Offering], recipients: [(f64, f64); 2]) -> Self {
        assert!(offerings.len() <= 100, "finite fixture occurrence ranges");
        let mut result = Self {
            intrinsic: intrinsic::World::new([22, 22]),
            applications: finite(&asset::<DeclaredSet<EffectApplicationRule>>(
                "applications.json",
            )),
        };
        let dependencies: Vec<DefinitionDescriptor> = asset("dependencies.json");
        for definition in finite(&dependencies) {
            result.put_definition(definition, false);
        }
        let dependencies: Vec<SlotDescriptor> = asset("dependency-slots.json");
        for descriptor in finite(&dependencies) {
            result.put_slot(descriptor);
        }
        let declaration: DefinitionDescriptor = asset("skill-declaration.json");
        result.put_definition(finite(&declaration), true);
        let extension: Value = asset("extension.json");
        for row in extension["schema"].as_array().unwrap() {
            match row["kind"].as_str().unwrap() {
                "definition" => result.put_definition(
                    finite(
                        &serde_json::from_value::<DefinitionDescriptor>(row["value"].clone())
                            .unwrap(),
                    ),
                    true,
                ),
                "slot" => result.put_slot(finite(
                    &serde_json::from_value::<SlotDescriptor>(row["value"].clone()).unwrap(),
                )),
                other => panic!("unexpected schema mutation {other}"),
            }
        }
        let owners: Vec<DefinitionRules> =
            serde_json::from_value(extension["owners"].clone()).unwrap();
        for owner in finite(&owners) {
            result.intrinsic.f.owners.push(owner);
        }
        let tables: Vec<IntegerRuleTable> =
            serde_json::from_value(extension["tables"].clone()).unwrap();
        result.intrinsic.f.tables.extend(tables);
        assert!(extension["receivers"].as_array().unwrap().is_empty());
        result.add_final_boundary();
        result.add_choice_boundary(0x2a2, "active", 0x3227, ComputedValueType::Boolean, None);
        for (name, stat, unit) in [
            ("source-increased", 0x3228, 2),
            ("source-more", 0x3229, 1),
            ("source-magnitude", 0x322a, 1),
        ] {
            result.add_choice_boundary(
                0x2a2,
                name,
                stat,
                ComputedValueType::Quantity { unit: def(unit) },
                None,
            );
        }
        for (name, stat, unit) in [
            ("recipient-increased", 0x322b, 2),
            ("recipient-more", 0x322c, 1),
        ] {
            result.add_choice_boundary(
                0x12,
                name,
                stat,
                ComputedValueType::Quantity { unit: def(unit) },
                Some(actor_slot()),
            );
        }
        for (index, offering) in offerings.iter().enumerate() {
            result.add_offering(index, offering);
        }
        for (index, (increased, more)) in recipients.into_iter().enumerate() {
            result.choice(
                recipient_target(index),
                0x12,
                "recipient-increased",
                quantity(increased, 2),
            );
            result.choice(
                recipient_target(index),
                0x12,
                "recipient-more",
                quantity(more, 1),
            );
        }
        result.add_empty_structural_owners();
        result
    }
    fn put_definition(&mut self, definition: DefinitionDescriptor, replace: bool) {
        let rows = &mut self.intrinsic.f.schema.definitions;
        if let Some(old) = rows
            .iter_mut()
            .find(|row| row.address() == definition.address())
        {
            if replace {
                *old = definition;
            }
        } else {
            rows.push(definition);
        }
    }
    fn put_slot(&mut self, slot: SlotDescriptor) {
        let rows = &mut self.intrinsic.f.schema.slots;
        if let Some(old) = rows.iter_mut().find(|row| row.address() == slot.address()) {
            *old = slot;
        } else {
            rows.push(slot);
        }
    }
    fn add_final_boundary(&mut self) {
        let inputs = [
            (
                "final-level",
                0x3225,
                ComputedValueType::Integer,
                ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
            ),
            (
                "final-quality",
                0x3226,
                ComputedValueType::Quantity { unit: def(2) },
                quantity_schema(2),
            ),
        ];
        for (name, stat, value_type, schema) in inputs {
            let parameter = boundary_parameter(name);
            self.put_slot(SlotDescriptor::Parameter(entry(
                parameter.clone(),
                ParameterSlotSchema {
                    value: schema,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
            )));
            for row in &mut self.intrinsic.f.schema.definitions {
                if let DefinitionDescriptor::Gem(row) = row
                    && row.id == def(0x86b)
                    && let SchemaState::Known(schema) = &mut row.schema
                {
                    schema
                        .declarations
                        .parameters
                        .members
                        .push(parameter.clone());
                }
            }
            self.intrinsic
                .f
                .owner_mut(&SchemaSubject::Definition(DefinitionAddress::Gem(def(
                    0x86b,
                ))))
                .programs
                .members
                .push(RuleProgram {
                    id: key(&format!("fixture-explicit-supported-{name}")),
                    context: RuleEntityKind::Skill,
                    reads: vec![RuleRead {
                        id: key("input"),
                        value_type,
                        source: RuleReadSource::Parameter { slot: parameter },
                    }],
                    nodes: vec![RuleNode {
                        id: key("input"),
                        expression: RuleExpression::Read {
                            input: key("input"),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("boundary"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat: def(stat),
                            value: key("input"),
                        },
                    }],
                });
        }
    }
    fn add_choice_boundary(
        &mut self,
        skill: u64,
        name: &str,
        stat: u64,
        value_type: ComputedValueType,
        actor: Option<DeclaredSlot<ActorSlotDefId>>,
    ) {
        let choice = boundary_choice(skill, name);
        let value = match &value_type {
            ComputedValueType::Boolean => ValueSchema::Boolean,
            ComputedValueType::Quantity { unit } => {
                quantity_schema(if *unit == def(2) { 2 } else { 1 })
            }
            other => panic!("unsupported fixture choice {other:?}"),
        };
        self.put_slot(SlotDescriptor::Choice(entry(
            choice.clone(),
            ChoiceSlotSchema {
                value,
                presence: SlotPresence::OptionalOnce,
                owners: vec![ChoiceOwnerScope::Skill],
            },
        )));
        for row in &mut self.intrinsic.f.schema.definitions {
            if let DefinitionDescriptor::Skill(row) = row
                && row.id == def(skill)
                && let SchemaState::Known(schema) = &mut row.schema
            {
                schema.declarations.choices.members.push(choice.clone());
            }
        }
        let owner = SchemaSubject::Definition(DefinitionAddress::Skill(def(skill)));
        if !self.intrinsic.f.owners.iter().any(|row| row.owner == owner) {
            self.intrinsic.f.owners.push(DefinitionRules {
                owner: owner.clone(),
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let effect = match actor {
            Some(actor) => RuleEffectKind::ProjectActorStat {
                actor,
                stat: def(stat),
                value: key("value"),
            },
            None => RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: def(stat),
                value: key("value"),
            },
        };
        self.intrinsic
            .f
            .owner_mut(&owner)
            .programs
            .members
            .push(RuleProgram {
                id: key(&format!("fixture-explicit-{name}")),
                context: RuleEntityKind::Skill,
                reads: vec![RuleRead {
                    id: key("value"),
                    value_type,
                    source: RuleReadSource::Choice { slot: choice },
                }],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Read {
                        input: key("value"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("boundary"),
                    when: None,
                    effect,
                }],
            });
    }
    fn choice(&mut self, target: SkillTarget, skill: u64, name: &str, value: ParameterValue) {
        self.intrinsic.f.build.choices.push(MechanicChoice {
            owner: ChoiceOwner::Skill(target),
            choice: ChoiceSelection {
                slot: boundary_choice(skill, name),
                value,
            },
        });
    }
    fn add_offering(&mut self, index: usize, offering: &Offering) {
        // These fixture-local ranges are disjoint from the inherited Sniper
        // occurrences. Preserve the shared lineage and record every issued ID.
        let last = u64::try_from(index).unwrap().checked_add(1_100).unwrap();
        let allocator = self.intrinsic.f.build.allocator;
        self.intrinsic.f.build.allocator = InstanceAllocatorState::from_parts(
            allocator.lineage(),
            allocator.last_issued().max(last),
        );
        let gem_id = occurrence(1_000 + index as u64);
        self.intrinsic.f.build.gems.push(GemInstance {
            id: gem_id,
            definition: def(0x86b),
            level: offering.raw_level,
            quality: Some(QualitySelection {
                kind: def(6),
                amount: FiniteQuantity::new(0., def(2)).unwrap(),
            }),
            parameters: vec![
                ParameterAssignment {
                    slot: slot(SlotOwnerDefId::Gem(def(0x86b)), 0x3063),
                    value: ParameterValue::Boolean(false),
                },
                ParameterAssignment {
                    slot: slot(SlotOwnerDefId::Gem(def(0x86b)), 0x3064),
                    value: quantity(0., 0x295a),
                },
                ParameterAssignment {
                    slot: boundary_parameter("final-level"),
                    value: integer(offering.final_level),
                },
                ParameterAssignment {
                    slot: boundary_parameter("final-quality"),
                    value: quantity(offering.final_quality, 2),
                },
            ],
        });
        self.intrinsic.f.build.skills.push(SkillUse {
            id: occurrence(1_100 + index as u64),
            source: AuthoredSkillSource::Gem(gem_id),
            enabled: true,
            scope: LoadoutScope::Shared,
        });
        if let Some(active) = offering.active {
            self.choice(
                offering_target(index),
                0x2a2,
                "active",
                ParameterValue::Boolean(active),
            );
        }
        for (name, value) in [
            ("source-increased", quantity(offering.increased, 2)),
            ("source-more", quantity(offering.more, 1)),
            ("source-magnitude", quantity(offering.magnitude, 1)),
        ] {
            self.choice(offering_target(index), 0x2a2, name, value);
        }
    }
    fn add_empty_structural_owners(&mut self) {
        let mut owners: Vec<_> = self
            .intrinsic
            .f
            .schema
            .slots
            .iter()
            .filter_map(|row| match row {
                SlotDescriptor::Grant(_) | SlotDescriptor::SkillGrant(_) => {
                    Some(SchemaSubject::Slot(row.address()))
                }
                _ => None,
            })
            .collect();
        owners.extend(
            self.intrinsic
                .f
                .schema
                .definitions
                .iter()
                .filter_map(|row| match row {
                    DefinitionDescriptor::Gem(_) | DefinitionDescriptor::Skill(_) => {
                        Some(SchemaSubject::Definition(row.address()))
                    }
                    _ => None,
                }),
        );
        for owner in owners {
            if !self.intrinsic.f.owners.iter().any(|row| row.owner == owner) {
                self.intrinsic.f.owners.push(DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
    }
    pub fn compile(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
        let f = &self.intrinsic.f;
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = Arc::new(
            CompiledRulePackage::compile(
                &RulePackageInput {
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: ns(),
                    release: key("finite-pain-offering"),
                    semantics_version: key("finite-explicit-boundaries"),
                    operations_version: key(OWNED_RULE_OPERATIONS_V15),
                    definitions: schema.identity().clone(),
                    owners: f.owners.clone(),
                    tables: f.tables.clone(),
                    receivers: f.receivers.clone(),
                    effect_applications: Some(self.applications.clone()),
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("finite-routes"),
                    definitions: schema.identity().clone(),
                    outputs: f.routes.clone(),
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(f.request()),
            schema,
            rules,
            routing,
            Default::default(),
        )
        .unwrap()
    }
    pub fn evaluate(&self) -> OwnedEffectsReport {
        let plan = self.compile();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
    pub fn missing_final_producer(&mut self) {
        self.intrinsic
            .f
            .owner_mut(&SchemaSubject::Definition(DefinitionAddress::Gem(def(
                0x86b,
            ))))
            .programs
            .members
            .retain(|row| !row.id.as_str().starts_with("fixture-explicit-supported-"));
    }
    pub fn restore_partial_registry(&mut self) {
        self.applications.closure =
            asset::<DeclaredSet<EffectApplicationRule>>("applications.json").closure;
    }
}
pub fn group(report: &OwnedEffectsReport, recipient: usize) -> &EffectApplicationGroupResult {
    report.application_groups.iter().find(|row| matches!(&row.key.invocation.origin,
        RuleOrigin::EffectApplicationGroup { recipient: ConcreteEntity::Actor(target), .. } if *target == actor(recipient)
    )).expect("exact Sniper recipient group")
}
