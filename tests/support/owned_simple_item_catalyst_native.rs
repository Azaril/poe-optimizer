//! Actual template catalyst transport feeding actual Life programs in a finite,
//! unpublished contributor domain. Import defaults are explicit input values.
#[allow(dead_code)]
#[path = "owned_simple_item_catalyst.rs"]
pub mod catalyst;
#[allow(dead_code)]
#[path = "owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "owned_flat_life.rs"]
mod life;
use component::{AuthoredComponent, AuthoredTemplateInputs, CategoryBindings, ComponentBindings};
pub use component::{Fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
use std::ops::{Deref, DerefMut};

pub struct CatalystFixture {
    pub native: Fixture,
    pub bindings: Vec<catalyst::CatalystBinding>,
}
impl Deref for CatalystFixture {
    type Target = Fixture;
    fn deref(&self) -> &Fixture {
        &self.native
    }
}
impl DerefMut for CatalystFixture {
    fn deref_mut(&mut self) -> &mut Fixture {
        &mut self.native
    }
}

pub fn fixture() -> CatalystFixture {
    let binding = life::bindings();
    let category = component::categories::bindings();
    let mut bindings = catalyst::bindings();
    bindings.reverse(); // Tattered then Rope, matching the source witness's two rows.
    assert_eq!(bindings[0].template.key().as_str(), "def.000000000000238c");
    assert_eq!(bindings[1].template.key().as_str(), "def.0000000000002007");
    let extension = catalyst::extension();
    let defaults = catalyst::defaults();
    let inputs = bindings
        .iter()
        .map(|binding| {
            let owner = extension
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(binding.template.address()))
                .unwrap()
                .clone();
            assert!(matches!(
                owner.programs.closure,
                SchemaClosure::Partial { .. }
            ));
            assert_eq!(owner.programs.members.len(), 1);
            assert_eq!(owner.programs.members[0].id.as_str(), "catalyst-inputs");
            let slots: Vec<_> = extension
                .schema
                .iter()
                .filter_map(|entry| match entry {
                    SchemaExtensionEntry::Slot(slot)
                        if slot.address().declaration()
                            == &SlotOwnerDefId::ItemTemplate(binding.template.clone()) =>
                    {
                        Some(slot.clone())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(slots.len(), 2);
            let defaults = defaults
                .iter()
                .find(|d| d.template == binding.template)
                .unwrap();
            assert_eq!(defaults.parameters.len(), 2);
            let assignments = defaults
                .parameters
                .iter()
                .map(|d| {
                    assert_eq!(
                        d.headers,
                        if d.assignment.slot == binding.selection {
                            vec!["Catalyst"]
                        } else {
                            assert_eq!(d.assignment.slot, binding.amount);
                            vec!["CatalystQuality"]
                        }
                    );
                    d.assignment.clone()
                })
                .collect();
            AuthoredTemplateInputs {
                template: binding.template.clone(),
                slots,
                owner,
                assignments,
            }
        })
        .collect();
    let mut dependencies = life::dependency_definitions();
    for row in catalyst::dependency_definitions() {
        if let Some(previous) = dependencies.iter().find(|d| d.address() == row.address()) {
            assert_eq!(previous, &row, "shared immutable descriptor must agree");
        } else {
            assert!(
                !matches!(row, DefinitionDescriptor::ItemTemplate(_)),
                "finite topology is declared only by this fixture"
            );
            dependencies.push(row);
        }
    }
    let mut native = Fixture::from_authored_with_template_inputs(
        AuthoredComponent {
            bindings: ComponentBindings {
                modifier: binding.modifier,
                amount: binding.amount,
                properties: binding.properties,
                corrupted_base: binding.corrupted_base,
                unit: binding.unit,
                contribution_unit: binding.contribution_unit,
                factor_unit: binding.factor_unit,
                effective: binding.effective,
                contribution: binding.contribution,
            },
            extension: life::extension(),
            dependencies,
            numeric_policy: life::numeric_policy,
            category: Some(CategoryBindings {
                slot: binding.category,
                explicit: category.explicit,
                implicit: category.implicit,
                enchant: category.enchant,
            }),
            category_target: None,
            catalyst_property: "life",
            catalyst_amount: 20.0,
            parameter_count: 24,
            parameters_complete: true,
            last_authored: 0x311a,
            release: "synthetic-ordinary-catalyst-transport",
        },
        inputs,
    );
    native.build.items[0].modifiers.truncate(1);
    native.build.items[0].modifier_order.truncate(1);
    for index in 0..2 {
        native.set_raw(index, 0, 10.0);
        for parameter in &mut native.build.items[index].modifiers[0].rolls {
            if let ParameterValue::Boolean(value) = &mut parameter.value {
                *value = false;
            }
        }
    }
    CatalystFixture { native, bindings }
}
impl CatalystFixture {
    pub fn option(&self, property: &str) -> OptionDefId {
        let e = life::extension();
        let p = e.owners[0]
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "catalyst-scalar")
            .unwrap();
        let RuleExpression::Literal {
            value: ParameterValue::Option(option),
        } = &p
            .nodes
            .iter()
            .find(|n| n.id.as_str() == format!("option-{property}"))
            .unwrap()
            .expression
        else {
            panic!()
        };
        option.clone()
    }
    pub fn set_kind(&mut self, item: usize, value: OptionDefId) {
        let slot = self.bindings[item].selection.clone();
        self.build.items[item]
            .parameters
            .iter_mut()
            .find(|p| p.slot == slot)
            .unwrap()
            .value = ParameterValue::Option(value);
    }
    pub fn set_amount(&mut self, item: usize, value: f64) {
        let slot = self.bindings[item].amount.clone();
        let p = self.build.items[item]
            .parameters
            .iter_mut()
            .find(|p| p.slot == slot)
            .unwrap();
        let ParameterValue::Quantity(old) = &p.value else {
            panic!()
        };
        p.value = ParameterValue::Quantity(FiniteQuantity::new(value, old.unit().clone()).unwrap());
    }
    pub fn set_life_tag(&mut self, item: usize, value: bool) {
        let slot = self.family.properties["life"].clone();
        self.build.items[item].modifiers[0]
            .rolls
            .iter_mut()
            .find(|p| p.slot == slot)
            .unwrap()
            .value = ParameterValue::Boolean(value);
    }
    pub fn parameter(&self, item: usize, amount: bool) -> &ParameterValue {
        let slot = if amount {
            &self.bindings[item].amount
        } else {
            &self.bindings[item].selection
        };
        &self.build.items[item]
            .parameters
            .iter()
            .find(|p| &p.slot == slot)
            .unwrap()
            .value
    }
    pub fn input<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        equipment: u64,
        amount: bool,
    ) -> &'a EffectValue {
        self.input_if_produced(report, equipment, amount)
            .expect("missing catalyst producer output")
    }
    pub fn input_if_produced<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        equipment: u64,
        amount: bool,
    ) -> Option<&'a EffectValue> {
        let key = PlanValueKey::Stat {
            entity: ConcreteEntity::EquipmentUse(occurrence(equipment)),
            stat: self.stat(if amount {
                "def.0000000000000a1a"
            } else {
                "def.0000000000000a19"
            }),
        };
        report
            .values
            .iter()
            .find(|v| v.key == key)
            .map(|v| &v.value)
    }
    pub fn factor<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        equipment: u64,
        modifier: u64,
    ) -> &'a EffectValue {
        self.value(
            report,
            PlanValueKey::Stat {
                entity: ConcreteEntity::Modifier(ProviderKey {
                    root: ProviderRoot::ItemModifier {
                        equipment_use: occurrence(equipment),
                        modifier: occurrence(modifier),
                    },
                    grant_path: vec![],
                }),
                stat: self.stat("def.0000000000000a1b"),
            },
        )
    }
    fn stat(&self, id: &str) -> StatDefId {
        StatDefId::parse(self.family.modifier.namespace().clone(), id).unwrap()
    }
    fn value<'a>(&self, report: &'a OwnedEffectsReport, key: PlanValueKey) -> &'a EffectValue {
        &report
            .values
            .iter()
            .find(|v| v.key == key)
            .unwrap_or_else(|| panic!("missing {key:?}"))
            .value
    }
    pub fn expected_factor(&self, value: f64) -> EffectValue {
        EffectValue::Known {
            value: ParameterValue::Quantity(
                FiniteQuantity::new(value, self.family.factor_unit.clone()).unwrap(),
            ),
        }
    }
}
