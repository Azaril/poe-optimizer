//! Actual flat-Life programs in an unpublished finite contributor domain.
//! Raw/effective modifier counts and contributed Life points have distinct units.
#[allow(dead_code)]
#[path = "owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "owned_flat_life.rs"]
pub mod family;

use component::{AuthoredComponent, CategoryBindings, ComponentBindings};
pub use component::{Fixture, categories, occurrence};
use poe_optimizer_core::owned_definitions::OptionDefId;

pub fn fixture() -> Fixture {
    domain(None)
}
pub fn with_categories(target: OptionDefId) -> Fixture {
    domain(Some(target))
}
fn domain(target: Option<OptionDefId>) -> Fixture {
    let binding = family::bindings();
    let category = categories::bindings();
    assert_ne!(binding.unit, binding.contribution_unit);
    Fixture::from_authored(AuthoredComponent {
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
        extension: family::extension(),
        dependencies: family::dependency_definitions(),
        numeric_policy: family::numeric_policy,
        category: Some(CategoryBindings {
            slot: binding.category,
            explicit: category.explicit,
            implicit: category.implicit,
            enchant: category.enchant,
        }),
        category_target: target,
        catalyst_property: "life",
        catalyst_amount: 20.0,
        parameter_count: 24,
        parameters_complete: true,
        last_authored: 0x311a,
        release: "synthetic-flat-life-component",
    })
}
