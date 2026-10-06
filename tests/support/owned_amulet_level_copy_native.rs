//! One finite ordinary-rule plan using the existing Minion numeric fixture.
//!
//! Exact authored copy/placement/Passive bodies and exact equipment placement
//! inventories are retained. Unrelated item parameters, catalyst inputs, passive
//! adjacency and producer inventories are explicit test boundaries. This is not
//! a complete real release or a V18 source-property relation evaluation.
#[allow(dead_code)]
#[path = "owned_global_minion_level_native.rs"]
mod component;

use component::Fixture as Component;
pub use component::occurrence;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    fs,
    ops::{Deref, DerefMut},
    path::Path,
};

// Numerical fixtures consume the exact authored bytes. They do not need to
// compile the publication/migration helper to read these three data fragments.
fn read<T: DeserializeOwned>(name: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/amulet-level-copy")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn empty() -> DeclaredSlots {
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
#[derive(Deserialize)]
pub struct Bindings {
    pub modifier: ModifierDefId,
    pub effective: StatDefId,
    pub minion_level: StatDefId,
    pub unscalable: DeclaredSlot<ParameterSlotDefId>,
    pub units: Units,
    pub channels: Channels,
    pub programs: Programs,
    pub templates: Vec<Template>,
    pub passive: Passive,
    pub placement_slots: Placement,
}
#[derive(Deserialize)]
pub struct Units {
    pub level: UnitDefId,
    pub percent: UnitDefId,
}
#[derive(Deserialize)]
pub struct Channels {
    pub eligibility: StatDefId,
    pub pre_amulet_percent: StatDefId,
}
#[derive(Deserialize)]
pub struct Programs {
    pub copy: OwnedDefinitionKey,
    pub eligibility: OwnedDefinitionKey,
    pub passive: OwnedDefinitionKey,
}
#[derive(Deserialize)]
pub struct Template {
    pub template: ItemTemplateDefId,
    pub eligible: bool,
    pub complete_placement: bool,
}
#[derive(Deserialize)]
pub struct Passive {
    pub definition: PassiveNodeDefId,
    pub percent: f64,
}
#[derive(Deserialize)]
pub struct Placement {
    pub helmet: EquipmentSlotDefId,
    pub amulet: EquipmentSlotDefId,
}
#[derive(Deserialize)]
struct Dependencies {
    definitions: Vec<DefinitionDescriptor>,
}

pub struct Fixture {
    pub native: Component,
    pub bindings: Bindings,
    pub authored_passive: RuleProgram,
    pub authored_copy: RuleProgram,
    partial_family: SchemaClosure,
    passive_pool: PointPoolDefId,
}
impl Deref for Fixture {
    type Target = Component;
    fn deref(&self) -> &Component {
        &self.native
    }
}
impl DerefMut for Fixture {
    fn deref_mut(&mut self) -> &mut Component {
        &mut self.native
    }
}

impl Fixture {
    pub fn new() -> Self {
        let b: Bindings = read("bindings.json");
        let m: OwnedReleaseMigrationInput = read("migration.json");
        let d: Dependencies = read("dependencies.json");
        let mut native = Component::new();
        assert_eq!(native.family.modifier, b.modifier);
        assert_eq!(native.family.effective, b.effective);
        assert_eq!(native.family.contribution, b.minion_level);
        assert_eq!(native.family.unit, b.units.level);
        let copy_owner = m
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.modifier.address()))
            .unwrap();
        assert!(!copy_owner.programs.is_complete());
        let copy = copy_owner
            .programs
            .members
            .iter()
            .find(|p| p.id == b.programs.copy)
            .unwrap()
            .clone();
        let passive_owner = m
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.passive.definition.address()))
            .unwrap();
        assert!(!passive_owner.programs.is_complete());
        let passive = passive_owner
            .programs
            .members
            .iter()
            .find(|p| p.id == b.programs.passive)
            .unwrap()
            .clone();

        for wanted in [
            b.channels.eligibility.address(),
            b.channels.pre_amulet_percent.address(),
        ] {
            let definition = m
                .schema
                .iter()
                .find_map(|e| match e {
                    SchemaExtensionEntry::Definition(d) if d.address() == wanted => Some(d.clone()),
                    _ => None,
                })
                .unwrap();
            assert!(
                !native
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .any(|d| d.address() == wanted)
            );
            native.recipe.schema.definitions.push(definition);
        }
        native
            .family_owner_mut()
            .programs
            .members
            .push(copy.clone());

        // Preserve the existing fixture's catalyst boundary as literal test
        // facts, with zero catalyst magnitude. They are not physical defaults.
        let old_template = native.build.items[0].template.clone();
        let catalyst = native
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(old_template.address()))
            .unwrap()
            .programs
            .members
            .clone();
        let boundary_parameters = native.build.items[0].parameters.clone();
        let catalyst: Vec<_> = catalyst
            .into_iter()
            .map(|mut p| {
                let RuleReadSource::Parameter { slot } = &p.reads[0].source else {
                    panic!()
                };
                let mut value = boundary_parameters
                    .iter()
                    .find(|v| &v.slot == slot)
                    .unwrap()
                    .value
                    .clone();
                if let ParameterValue::Quantity(q) = &value {
                    value = quantity(0.0, q.unit());
                }
                assert_eq!(p.reads.len(), 1);
                assert_eq!(p.nodes.len(), 1);
                p.reads.clear();
                p.nodes[0].expression = RuleExpression::Literal { value };
                p
            })
            .collect();

        // Four published complete placement rows are copied exactly. Their
        // unrelated declarations are closed only in this finite test universe.
        for row in b.templates.iter().filter(|t| t.complete_placement) {
            let mut definition = m
                .schema
                .iter()
                .find_map(|e| match e {
                    SchemaExtensionEntry::Definition(d)
                        if d.address() == row.template.address() =>
                    {
                        Some(d.clone())
                    }
                    _ => None,
                })
                .expect("the authored placement refinement must exist");
            let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) = &mut definition
            else {
                panic!()
            };
            assert!(s.equipment_slots.is_complete());
            assert!(
                s.socket_destinations.is_complete() && s.socket_destinations.members.is_empty()
            );
            assert_eq!(
                s.equipment_slots.members,
                vec![if row.eligible {
                    b.placement_slots.amulet.clone()
                } else {
                    b.placement_slots.helmet.clone()
                }]
            );
            s.modifiers = DeclaredSet::complete(vec![b.modifier.clone()]);
            s.quality = QualityUseSchema {
                presence: QualityPresence::Forbidden,
                allowed_kinds: DeclaredSet::complete(vec![]),
            };
            s.declarations = empty();
            native.recipe.schema.definitions.push(definition);
            let actual = m
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(row.template.address()))
                .unwrap();
            assert!(!actual.programs.is_complete());
            let program = actual
                .programs
                .members
                .iter()
                .find(|p| p.id == b.programs.eligibility)
                .unwrap()
                .clone();
            let mut programs = catalyst.clone();
            programs.push(program);
            native.recipe.rules.owners.push(DefinitionRules {
                owner: actual.owner.clone(),
                programs: DeclaredSet::complete(programs),
            });
        }
        for slot in [&b.placement_slots.helmet, &b.placement_slots.amulet] {
            let definition = d
                .definitions
                .iter()
                .find(|d| d.address() == slot.address())
                .unwrap()
                .clone();
            let DefinitionDescriptor::EquipmentSlot(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) = &definition
            else {
                panic!()
            };
            assert_eq!(s.scope, ScopePolicy::Shared);
            native.recipe.schema.definitions.push(definition);
        }
        let mut passive_definition = d
            .definitions
            .iter()
            .find(|d| d.address() == b.passive.definition.address())
            .unwrap()
            .clone();
        let DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) = &mut passive_definition
        else {
            panic!()
        };
        assert!(s.pools.is_complete());
        assert_eq!(s.pools.members.len(), 1);
        let pool = s.pools.members[0].clone();
        s.adjacent = DeclaredSet::complete(vec![]);
        s.declarations = empty();
        native.recipe.schema.definitions.push(passive_definition);
        native.recipe.schema.definitions.push(
            d.definitions
                .iter()
                .find(|d| d.address() == pool.address())
                .unwrap()
                .clone(),
        );
        native.recipe.rules.owners.push(DefinitionRules {
            owner: passive_owner.owner.clone(),
            programs: DeclaredSet::complete(vec![passive.clone()]),
        });

        let solar: ItemTemplateDefId =
            DefId::parse(b.modifier.namespace().clone(), "def.0000000000002343").unwrap();
        let crown: ItemTemplateDefId =
            DefId::parse(b.modifier.namespace().clone(), "def.0000000000001f1c").unwrap();
        native.build.items[0].template = solar;
        native.build.items[1].template = crown;
        for item in &mut native.build.items {
            item.parameters.clear();
        }
        native.build.equipment.retain(|u| u.id != occurrence(7));
        native.build.equipment[0].destination =
            EquipmentDestination::CharacterSlot(b.placement_slots.amulet.clone());
        native.build.equipment[1].destination =
            EquipmentDestination::CharacterSlot(b.placement_slots.helmet.clone());
        native.set_raw(0, 0, 1.0);
        native.set_raw(0, 1, 3.0);
        native.set_raw(1, 0, 1.0);
        native.complete_domain();
        Self {
            native,
            bindings: b,
            authored_passive: passive,
            authored_copy: copy,
            partial_family: copy_owner.programs.closure.clone(),
            passive_pool: pool,
        }
    }

    fn class_programs(&mut self) -> &mut Vec<RuleProgram> {
        let owner = SchemaSubject::Definition(self.native.build.character.class.address());
        &mut self
            .native
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner)
            .unwrap()
            .programs
            .members
    }
    pub fn boundary_factor(&mut self, percent: f64) {
        self.class_programs()
            .retain(|p| p.id.as_str() != "fixture-pre-amulet-snapshot");
        let program = RuleProgram {
            id: key("fixture-pre-amulet-snapshot"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("snapshot"),
                expression: RuleExpression::Literal {
                    value: quantity(percent, &self.bindings.units.percent),
                },
            }],
            effects: vec![RuleEffect {
                id: key("snapshot"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Player,
                    stat: self.bindings.channels.pre_amulet_percent.clone(),
                    value: key("snapshot"),
                },
            }],
        };
        self.class_programs().push(program);
    }
    pub fn actual_passive(&mut self) {
        self.boundary_factor(0.0);
        let stat = self.bindings.channels.pre_amulet_percent.clone();
        let unit = self.bindings.units.percent.clone();
        let p = self
            .class_programs()
            .iter_mut()
            .find(|p| p.id.as_str() == "fixture-pre-amulet-snapshot")
            .unwrap();
        p.reads = vec![RuleRead {
            id: key("incoming"),
            value_type: ComputedValueType::Quantity { unit: unit.clone() },
            source: RuleReadSource::Contributions {
                entity: RuleEntity::Player,
                stat,
                contribution: ContributionKind::Add,
                reduction: ContributionReduction::Sum,
                empty: quantity(0.0, &unit),
            },
        }];
        p.nodes[0].expression = RuleExpression::Read {
            input: key("incoming"),
        };
        self.native.build.allocations = vec![Allocation {
            id: occurrence(50),
            node: self.bindings.passive.definition.clone(),
            pool: self.passive_pool.clone(),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        }];
    }
    pub fn set_unscalable_boundary(&mut self, value: bool) {
        let slot = self.bindings.unscalable.clone();
        self.native.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|p| p.slot == slot)
            .unwrap()
            .value = ParameterValue::Boolean(value);
    }
    pub fn original_pair(&mut self) {
        self.native.build.items[0].modifiers.truncate(1);
        self.native.build.items[0].modifier_order.truncate(1);
    }
    pub fn restore_actual_family_gap(&mut self) {
        self.native.family_owner_mut().programs.closure = self.partial_family.clone();
    }
    pub fn incomplete_incoming(&mut self) {
        self.native.recipe.rules.receivers.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(
                    self.bindings.channels.pre_amulet_percent.address(),
                ),
                facet: SchemaFacet::GameRules,
                code: key("fixture-unreviewed-pre-amulet-contributors"),
            }],
        };
    }
    /// Active records include Known zero. Explicitly inactive copies are checked
    /// separately and do not enter this finite numerical contribution census.
    pub fn contributions<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        copy: bool,
    ) -> Vec<&'a BoundEffectResult> {
        report
            .effects
            .iter()
            .filter(|e| {
                e.value != EffectValue::Inactive
                    && e.key.invocation.program
                        == if copy {
                            self.bindings.programs.copy.clone()
                        } else {
                            key("contribute-player-minion-gem-level")
                        }
            })
            .collect()
    }
    pub fn factor<'a>(&self, report: &'a OwnedEffectsReport) -> &'a EffectValue {
        let key = PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: self.bindings.channels.pre_amulet_percent.clone(),
        };
        &report.values.iter().find(|v| v.key == key).unwrap().value
    }
    pub fn expected_percent(&self, n: f64) -> EffectValue {
        EffectValue::Known {
            value: quantity(n, &self.bindings.units.percent),
        }
    }
}
