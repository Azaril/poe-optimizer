//! Published Spell Damage programs in an unpublished finite numerical domain.
//! This supplies no receiving damage contribution, aggregate, or build parity.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "support/owned_spell_damage_modifier.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

use component::{AuthoredComponent, CategoryBindings, ComponentBindings, Fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
struct Inputs {
    extension: OwnedRecipeExtension,
    rolls: Vec<ParameterAssignment>,
}
impl Inputs {
    fn load() -> Self {
        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ASHEN_STAFF_OUTPUT")
                .expect("set to the verified Ashen Staff publication output"),
        );
        let endpoint = release::load(&output.join("package"));
        let recipe = &endpoint.input().recipe;
        let b = family::bindings();
        let mut extension = family::extension();
        for entry in &extension.schema {
            match entry {
                SchemaExtensionEntry::Definition(d) => {
                    assert!(recipe.schema.definitions.contains(d))
                }
                SchemaExtensionEntry::Slot(s) => assert!(recipe.schema.slots.contains(s)),
            }
        }
        for d in family::dependency_definitions() {
            assert!(recipe.schema.definitions.contains(&d));
        }
        let owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.modifier.address()))
            .unwrap();
        assert_eq!(owner.programs.members.len(), 4);
        assert_eq!(owner.programs.closure, extension.owners[0].programs.closure);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        for program in &extension.owners[0].programs.members {
            assert!(owner.programs.members.contains(program));
        }
        assert!(
            owner
                .programs
                .members
                .iter()
                .any(|p| p.id.as_str() == "effective-amount")
        );
        extension.owners = vec![owner.clone()];
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(output.join(name)).unwrap()).unwrap()
        };
        let draft = read("original-05/draft.json");
        let sidecar = read("original-05/sidecar.json");
        let origins: Vec<_> = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["source"]["ordinal"] == 594)
            .collect();
        assert_eq!(origins.len(), 1);
        let links: Vec<_> = origins[0]["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["kind"] == "item")
            .collect();
        assert_eq!(links.len(), 1);
        let item = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == links[0]["value"])
            .unwrap();
        assert_eq!(
            typed::<ItemTemplateDefId>(&item["template"]["value"]),
            b.templates[0]
        );
        let modifiers: Vec<_> = item["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| typed::<ModifierDefId>(&m["definition"]["value"]) == b.modifier)
            .collect();
        assert_eq!(modifiers.len(), 1);
        assert_eq!(
            modifiers[0]["rolls"]["completion"],
            json!({"kind":"complete"})
        );
        let rolls: Vec<ParameterAssignment> = modifiers[0]["rolls"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                assert_eq!(r["slot"]["kind"], "known");
                assert_eq!(r["value"]["kind"], "known");
                ParameterAssignment {
                    slot: typed(&r["slot"]["value"]),
                    value: typed(&r["value"]["value"]),
                }
            })
            .collect();
        assert_eq!(rolls.len(), 24);
        assert_eq!(
            rolls.iter().find(|r| r.slot == b.amount).unwrap().value,
            ParameterValue::Quantity(FiniteQuantity::new(128.0, b.unit).unwrap())
        );
        assert!(
            rolls
                .iter()
                .filter_map(|r| match r.value {
                    ParameterValue::Boolean(v) => Some(v),
                    _ => None,
                })
                .all(|v| !v)
        );
        Self { extension, rolls }
    }
    fn fixture(&self, target: Option<OptionDefId>) -> Fixture {
        let b = family::bindings();
        let c = component::categories::bindings();
        let mut f = Fixture::from_compiled_effects(AuthoredComponent {
            bindings: ComponentBindings {
                modifier: b.modifier,
                amount: b.amount,
                properties: b.properties,
                corrupted_base: b.corrupted_base,
                unit: b.unit.clone(),
                factor_unit: b.factor_unit,
                effective: b.effective.clone(),
                // The numerical-only constructor creates no contribution/sum.
                contribution: b.effective,
                contribution_unit: b.unit,
            },
            extension: self.extension.clone(),
            dependencies: family::dependency_definitions(),
            numeric_policy: |_| panic!("retain the published compiled programs"),
            category: Some(CategoryBindings {
                slot: b.category,
                explicit: c.explicit,
                implicit: c.implicit,
                enchant: c.enchant,
            }),
            category_target: target,
            catalyst_property: "caster",
            catalyst_amount: 20.0,
            parameter_count: 24,
            parameters_complete: true,
            last_authored: 0x31c5,
            release: "synthetic-spell-damage-component",
        });
        // Keep one physical roll at two synthetic equipment occurrences.
        // The optional category transform is a separate test-only modifier.
        f.build.items.truncate(1);
        f.build.equipment.truncate(2);
        f.build.items[0].modifiers.retain(|m| m.id != occurrence(5));
        f.build.items[0]
            .modifier_order
            .retain(|id| *id != occurrence(5));
        f.build.items[0].modifiers[0].rolls = self.rolls.clone();
        f.complete_domain();
        for program in &self.extension.owners[0].programs.members {
            assert!(f.family_owner_mut().programs.members.contains(program));
        }
        f
    }
}
fn evaluate(f: &Fixture, expected: f64) -> OwnedEffectsReport {
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for equipment in [6, 7] {
        assert_eq!(f.effective(&report, equipment, 4), &f.expected(expected));
    }
    assert!(
        !report
            .effects
            .iter()
            .any(|e| matches!(e.target, BoundEffectTarget::Contribution { .. }))
    );
    assert!(!report.values.iter().any(|v| matches!(
        v.key,
        PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(_),
            ..
        }
    )));
    report
}
fn caster(f: &mut Fixture, enabled: bool) {
    let slot = f.family.properties["caster"].clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot == slot)
        .unwrap()
        .value = ParameterValue::Boolean(enabled);
}
fn catalyst_amount(f: &mut Fixture, value: f64) {
    let assignment = f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| matches!(p.value, ParameterValue::Quantity(_)))
        .unwrap();
    let ParameterValue::Quantity(old) = &assignment.value else {
        unreachable!()
    };
    assignment.value =
        ParameterValue::Quantity(FiniteQuantity::new(value, old.unit().clone()).unwrap());
}

#[test]
#[ignore = "requires verified Ashen Staff publication output"]
fn actual_rolls_zero_and_source_catalyst_contrasts_use_published_programs() {
    let inputs = Inputs::load();
    let mut f = inputs.fixture(None);
    evaluate(&f, 128.0);
    f.set_raw(0, 0, 0.0);
    evaluate(&f, 0.0);
    f.set_raw(0, 0, 128.0);
    caster(&mut f, true);
    evaluate(&f, 153.0);
    catalyst_amount(&mut f, 0.0);
    evaluate(&f, 128.0);
    catalyst_amount(&mut f, 20.0);
    let program = f
        .family_owner_mut()
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "catalyst-scalar")
        .unwrap();
    let RuleExpression::Literal { value: wrong_kind } = &program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == "option-life")
        .unwrap()
        .expression
    else {
        panic!("authored different catalyst kind")
    };
    let wrong_kind = wrong_kind.clone();
    f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| matches!(p.value, ParameterValue::Option(_)))
        .unwrap()
        .value = wrong_kind;
    evaluate(&f, 128.0);
}

#[test]
#[ignore = "requires verified Ashen Staff publication output"]
fn source_scaling_vectors_and_reused_scratch_preserve_occurrence_boundaries() {
    let inputs = Inputs::load();
    let categories = component::categories::bindings();
    evaluate(&inputs.fixture(Some(categories.explicit)), 192.0);
    evaluate(&inputs.fixture(Some(categories.implicit)), 128.0);
    let mut f = inputs.fixture(None);
    let factor = f.family.corrupted_base.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot == factor)
        .unwrap()
        .value =
        ParameterValue::Quantity(FiniteQuantity::new(1.5, f.family.factor_unit.clone()).unwrap());
    evaluate(&f, 192.0);
    let first = f.plan().unwrap();
    let before = first.evaluate(&mut first.new_scratch()).unwrap();
    f.set_raw(0, 0, 0.0);
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let second = f.plan().unwrap();
    let after = second.evaluate(&mut second.new_scratch()).unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_eq!(f.effective(&after, 6, 4), &f.expected(0.0));
    assert!(!after.effects.iter().any(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::ItemModifier { equipment_use, .. } if equipment_use == occurrence(7)))));
    let mut scratch = first.new_scratch();
    assert_eq!(first.evaluate(&mut scratch).unwrap(), before);
    assert_eq!(second.evaluate(&mut scratch).unwrap(), after);
    assert_eq!(first.evaluate(&mut scratch).unwrap(), before);
}

#[test]
#[ignore = "requires verified Ashen Staff publication output"]
fn missing_inputs_demanded_producers_and_real_owner_gaps_remain_unavailable() {
    let inputs = Inputs::load();
    let b = family::bindings();
    for slot in [b.amount, b.category, b.corrupted_base] {
        let mut f = inputs.fixture(None);
        f.build.items[0].modifiers[0]
            .rolls
            .retain(|r| r.slot != slot);
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    let mut f = inputs.fixture(None);
    let amount = f.family.amount.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot == amount)
        .unwrap()
        .value =
        ParameterValue::Quantity(FiniteQuantity::new(128.0, f.family.factor_unit.clone()).unwrap());
    assert!(
        matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    for missing in ["corrupted-base-factor", "ordered-magnitude-scalar"] {
        let mut f = inputs.fixture(None);
        f.family_owner_mut()
            .programs
            .members
            .retain(|p| p.id.as_str() != missing);
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(
            f.effective(&report, 6, 4),
            EffectValue::Unresolved { .. }
        ));
    }
    let mut f = inputs.fixture(None);
    for owner in &mut f.recipe.rules.owners {
        owner
            .programs
            .members
            .retain(|p| p.id.as_str() != "fixture-catalyst-amount");
    }
    // Actual untagged input has no dependency on an inapplicable catalyst amount.
    evaluate(&f, 128.0);
    caster(&mut f, true);
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        f.effective(&report, 6, 4),
        EffectValue::Unresolved { .. }
    ));
    let mut f = inputs.fixture(None);
    f.restore_partial_rules();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert!(matches!(
        f.family_owner_mut().programs.closure,
        SchemaClosure::Partial { .. }
    ));
}
