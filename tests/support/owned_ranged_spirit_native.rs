//! Shared finite item-Spirit fixture; no published scalar or maximum-Spirit claim.
#[allow(dead_code)]
#[path = "owned_global_minion_level_native.rs"]
pub mod component;
#[allow(dead_code)]
#[path = "owned_ranged_spirit.rs"]
pub mod family;
use component::{AuthoredComponent, CategoryBindings, ComponentBindings, Fixture};
use poe_optimizer_core::{owned_build::ParameterValue, owned_definitions::OptionDefId};

pub fn fixture(target: Option<OptionDefId>) -> Fixture {
    let b = family::bindings();
    let c = component::categories::bindings();
    Fixture::from_authored(AuthoredComponent {
        bindings: ComponentBindings {
            modifier: b.modifier,
            amount: b.amount,
            properties: b.properties,
            corrupted_base: b.corrupted_base,
            unit: b.unit,
            contribution_unit: b.contribution_unit,
            factor_unit: b.factor_unit,
            effective: b.effective,
            contribution: b.contribution,
        },
        extension: family::extension(),
        dependencies: family::dependency_definitions(),
        numeric_policy: family::numeric_policy,
        category: Some(CategoryBindings {
            slot: b.category,
            explicit: c.explicit,
            implicit: c.implicit,
            enchant: c.enchant,
        }),
        category_target: target,
        catalyst_property: "mana",
        catalyst_amount: 20.0,
        parameter_count: 24,
        parameters_complete: true,
        last_authored: 0x3166,
        release: "synthetic-ranged-spirit-component",
    })
}
pub fn clear_properties(f: &mut Fixture) {
    for item in &mut f.build.items {
        for modifier in &mut item.modifiers {
            for roll in &mut modifier.rolls {
                if let ParameterValue::Boolean(value) = &mut roll.value {
                    *value = false;
                }
            }
        }
    }
}
