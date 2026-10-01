//! Actual Fine Belt modifier programs in an explicitly finite numerical domain.
//! Local charm capacity and player flask recovery remain separate consumers.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "support/owned_fine_belt_modifiers.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

use component::{
    AuthoredComponent, AuthoredTemplateInputs, CategoryBindings, ComponentBindings, Fixture,
    occurrence,
};
use poe_optimizer_core::owned_rules::RuleExpression;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{owned_recipe_extension::*, owned_release::StagedOwnedRelease};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn assignments(values: &Value) -> Vec<ParameterAssignment> {
    values["members"]
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
        .collect()
}
fn references(value: &Value, found: &mut BTreeSet<DefinitionAddress>) {
    if let Ok(address) = serde_json::from_value::<DefinitionAddress>(value.clone()) {
        found.insert(address);
    }
    if value.get("namespace").is_some()
        && value.get("key").is_some()
        && let Ok(address) =
            serde_json::from_value::<DefinitionAddress>(json!({"kind":value["kind"],"value":value}))
    {
        found.insert(address);
    }
    match value {
        Value::Object(fields) => fields.values().for_each(|v| references(v, found)),
        Value::Array(values) => values.iter().for_each(|v| references(v, found)),
        _ => (),
    }
}
struct Inputs {
    endpoint: StagedOwnedRelease,
    belt: Value,
    physical: bool,
}
impl Inputs {
    fn load() -> Self {
        Self::load_from("POE_OPTIMIZER_TEST_FINE_BELT_NATIVE_OUTPUT", false)
    }
    fn load_physical() -> Self {
        Self::load_from("POE_OPTIMIZER_TEST_FINE_BELT_ITEM_NATIVE_OUTPUT", true)
    }
    fn load_from(variable: &str, physical: bool) -> Self {
        let output = PathBuf::from(
            std::env::var_os(variable)
                .expect("set to the verified Fine Belt publication output directory"),
        );
        let endpoint = release::load(&output.join("package"));
        let draft: Value =
            serde_json::from_slice(&fs::read(output.join("original-05/draft.json")).unwrap())
                .unwrap();
        let belts: Vec<_> = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["template"]["value"]["key"] == "def.0000000000001e84")
            .collect();
        assert_eq!(belts.len(), 1);
        let belt = belts[0].clone();
        for collection in ["parameters", "modifiers"] {
            assert_eq!(
                belt[collection]["completion"] == json!({"kind":"complete"}),
                physical
            );
        }
        Self {
            endpoint,
            belt,
            physical,
        }
    }
    fn fixture(&self, charm: bool, target: Option<OptionDefId>) -> Fixture {
        let bindings = family::bindings();
        let b = if charm {
            bindings.charm
        } else {
            bindings.flask
        };
        let recipe = &self.endpoint.input().recipe;
        let subject = SchemaSubject::Definition(b.modifier.address());
        let owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject)
            .unwrap()
            .clone();
        assert_eq!(owner.programs.members.len(), 4);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let definition = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == b.modifier.address())
            .unwrap()
            .clone();
        let slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|s| s.address().declaration() == &SlotOwnerDefId::Modifier(b.modifier.clone()))
            .cloned()
            .collect();
        assert_eq!(slots.len(), if charm { 25 } else { 24 });
        let template_inputs = self.physical.then(|| {
            let template: ItemTemplateDefId = typed(&self.belt["template"]["value"]);
            let subject = SchemaSubject::Definition(template.address());
            let owner = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == subject)
                .unwrap()
                .clone();
            assert!(matches!(
                owner.programs.closure,
                SchemaClosure::Partial { .. }
            ));
            let mut programs: Vec<_> = owner
                .programs
                .members
                .iter()
                .map(|p| p.id.as_str())
                .collect();
            programs.sort_unstable();
            assert_eq!(
                programs,
                ["catalyst-inputs", "template-supplies-base-attack-profile"]
            );
            let slots: Vec<_> = recipe
                .schema
                .slots
                .iter()
                .filter(|s| {
                    s.address().declaration() == &SlotOwnerDefId::ItemTemplate(template.clone())
                })
                .cloned()
                .collect();
            assert_eq!(slots.len(), 6);
            let assignments = assignments(&self.belt["parameters"]);
            assert_eq!(assignments.len(), 6);
            AuthoredTemplateInputs {
                template,
                slots,
                owner,
                assignments,
            }
        });
        let mut needed = BTreeSet::new();
        for v in [
            serde_json::to_value(&owner).unwrap(),
            serde_json::to_value(&definition).unwrap(),
            serde_json::to_value(&slots).unwrap(),
        ] {
            references(&v, &mut needed);
        }
        if let Some(t) = &template_inputs {
            references(&serde_json::to_value(&t.owner).unwrap(), &mut needed);
            references(&serde_json::to_value(&t.slots).unwrap(), &mut needed);
        }
        let dependencies = loop {
            let before = needed.len();
            let rows: Vec<_> = recipe
                .schema
                .definitions
                .iter()
                .filter(|d| {
                    needed.contains(&d.address())
                        && d.address() != b.modifier.address()
                        && template_inputs
                            .as_ref()
                            .is_none_or(|t| d.address() != t.template.address())
                })
                .cloned()
                .collect();
            for row in &rows {
                references(&serde_json::to_value(row).unwrap(), &mut needed);
            }
            if before == needed.len() {
                break rows;
            }
        };
        assert!(dependencies.iter().all(|d| matches!(
            d,
            DefinitionDescriptor::Unit(_)
                | DefinitionDescriptor::Option(_)
                | DefinitionDescriptor::Stat(_)
                | DefinitionDescriptor::Capability(_)
        )));
        let mut schema = vec![SchemaExtensionEntry::Definition(definition)];
        schema.extend(slots.into_iter().map(SchemaExtensionEntry::Slot));
        let categories = component::categories::bindings();
        let modifier = b.modifier.clone();
        let component = AuthoredComponent {
            bindings: ComponentBindings {
                modifier: b.modifier,
                amount: b.amount,
                properties: b.properties,
                corrupted_base: b.corrupted_base,
                unit: b.unit.clone(),
                factor_unit: b.factor_unit,
                effective: b.effective.clone(),
                // No contribution or aggregate uses this fixture field.
                contribution: b.effective,
                contribution_unit: b.unit,
            },
            extension: OwnedRecipeExtension {
                schema_version: 1,
                version: key("actual-fine-belt-numeric-component"),
                schema,
                operations_version: Some(recipe.rules.operations_version.clone()),
                tables: vec![],
                owners: vec![owner.clone()],
                receivers: vec![],
            },
            dependencies,
            numeric_policy: |_| panic!("retain compiled endpoint programs"),
            category: Some(CategoryBindings {
                slot: b.category,
                explicit: categories.explicit,
                implicit: categories.implicit,
                enchant: categories.enchant,
            }),
            category_target: target,
            catalyst_property: "life",
            catalyst_amount: 0.0,
            parameter_count: if charm { 25 } else { 24 },
            parameters_complete: true,
            last_authored: if charm { 0x318b } else { 0x31a6 },
            release: "synthetic-fine-belt-numeric-component",
        };
        let mut f = if let Some(template) = template_inputs {
            Fixture::from_compiled_effects_with_template_inputs(component, template)
        } else {
            Fixture::from_compiled_effects(component)
        };
        f.build.items.truncate(1);
        f.build.equipment.truncate(1);
        if self.physical {
            assert_eq!(
                self.belt["item_level"],
                json!({"kind":"known","value":null})
            );
            assert_eq!(self.belt["quality"], json!({"kind":"known","value":null}));
            f.build.items[0].item_level = None;
        }
        f.build.items[0].modifiers.retain(|m| m.id != occurrence(5));
        let retained: BTreeSet<_> = f.build.items[0].modifiers.iter().map(|m| m.id).collect();
        f.build.items[0]
            .modifier_order
            .retain(|m| retained.contains(m));
        let actual = self.belt["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["definition"]["value"] == serde_json::to_value(&modifier).unwrap())
            .unwrap();
        assert_eq!(actual["rolls"]["completion"], json!({"kind":"complete"}));
        f.build.items[0].modifiers[0].rolls = assignments(&actual["rolls"]);
        f.complete_domain();
        let actual_owner = f.family_owner_mut();
        for p in &owner.programs.members {
            assert!(actual_owner.programs.members.contains(p));
        }
        f
    }
}
fn evaluate(f: &Fixture, expected: f64) -> OwnedEffectsReport {
    let p = f.plan().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(f.effective(&report, 6, 4), &f.expected(expected));
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
fn factor(f: &mut Fixture, value: f64) {
    let slot = f.family.corrupted_base.clone();
    let unit = f.family.factor_unit.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot == slot)
        .unwrap()
        .value = ParameterValue::Quantity(FiniteQuantity::new(value, unit).unwrap());
}

fn physical_life_catalyst(f: &mut Fixture, amount: f64) {
    let program = f
        .family_owner_mut()
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "catalyst-scalar")
        .unwrap();
    let RuleExpression::Literal { value: kind } = &program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == "option-life")
        .unwrap()
        .expression
    else {
        panic!()
    };
    let kind = kind.clone();
    let raw_amount = f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot.slot.key().as_str() == "def.00000000000031ac")
        .unwrap();
    let ParameterValue::Quantity(current) = &raw_amount.value else {
        panic!()
    };
    raw_amount.value =
        ParameterValue::Quantity(FiniteQuantity::new(amount, current.unit().clone()).unwrap());
    f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot.slot.key().as_str() == "def.00000000000031ab")
        .unwrap()
        .value = kind;
}

#[test]
#[ignore = "requires verified Fine Belt physical-input endpoint"]
fn actual_physical_inputs_feed_unchanged_numeric_programs_and_keep_owner_gaps() {
    let inputs = Inputs::load_physical();
    for charm in [true, false] {
        let mut f = inputs.fixture(charm, None);
        evaluate(&f, if charm { 2.0 } else { 0.17 });
        physical_life_catalyst(&mut f, 50.0);
        // Changing catalyst inputs remains inert for the actual source flags.
        evaluate(&f, if charm { 2.0 } else { 0.17 });
        // This explicit synthetic flag tests transport through the real template
        // program. It is not a claim that the original item had a Life tag.
        let property = f.family.properties["life"].clone();
        f.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot == property)
            .unwrap()
            .value = ParameterValue::Boolean(true);
        evaluate(&f, if charm { 3.0 } else { 0.25 });
        let scaled = f.plan().unwrap();
        let before = scaled.evaluate(&mut scaled.new_scratch()).unwrap();
        physical_life_catalyst(&mut f, 0.0);
        let zero = f.plan().unwrap();
        let after = evaluate(&f, if charm { 2.0 } else { 0.17 });
        assert_ne!(scaled.identity(), zero.identity());
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let (scaled, zero, before, after) = (&scaled, &zero, &before, &after);
                scope.spawn(move || {
                    let mut scratch = scaled.new_scratch();
                    assert_eq!(scaled.evaluate(&mut scratch).unwrap(), *before);
                    assert_eq!(zero.evaluate(&mut scratch).unwrap(), *after);
                    assert_eq!(scaled.evaluate(&mut scratch).unwrap(), *before);
                });
            }
        });
        for suffix in [0x31a7, 0x31a8, 0x31aa, 0x31ab, 0x31ac] {
            let mut missing = inputs.fixture(charm, None);
            missing.build.items[0]
                .parameters
                .retain(|p| p.slot.slot.key().as_str() != format!("def.{suffix:016x}"));
            assert!(
                matches!(missing.plan(),Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
            );
        }
        f.restore_template_rules();
        let p = f.plan().unwrap();
        assert!(!p.evaluate(&mut p.new_scratch()).unwrap().gaps.is_empty());
    }
}

#[test]
#[ignore = "requires verified Fine Belt endpoint and actual original-05 inputs"]
fn actual_modifiers_preserve_capacity_and_per_second_numeric_units() {
    let inputs = Inputs::load();
    evaluate(&inputs.fixture(true, None), 2.0);
    evaluate(&inputs.fixture(false, None), 0.17);
    for (charm, vectors) in [
        (true, vec![(1.0, 1.0), (1.5, 2.0), (2.0, 2.0), (3.0, 3.0)]),
        (
            false,
            vec![
                (0.0, 0.0),
                (0.001, 0.0),
                (0.125, 0.13),
                (0.17, 0.17),
                (0.175, 0.18),
                (0.5, 0.5),
            ],
        ),
    ] {
        for (raw, expected) in vectors {
            let mut f = inputs.fixture(charm, None);
            f.set_raw(0, 0, raw);
            evaluate(&f, expected);
        }
        let mut f = inputs.fixture(charm, None);
        factor(&mut f, 1.5);
        evaluate(&f, if charm { 3.0 } else { 0.25 });
        let c = component::categories::bindings();
        evaluate(
            &inputs.fixture(charm, Some(c.implicit)),
            if charm { 3.0 } else { 0.25 },
        );
        evaluate(
            &inputs.fixture(charm, Some(c.explicit)),
            if charm { 2.0 } else { 0.17 },
        );
    }
}

#[test]
#[ignore = "requires verified Fine Belt endpoint and actual original-05 inputs"]
fn compiled_numeric_plans_are_parallel_and_reused_scratch_isolated() {
    let inputs = Inputs::load();
    for charm in [true, false] {
        let mut f = inputs.fixture(charm, None);
        let first = f.plan().unwrap();
        let before = first.evaluate(&mut first.new_scratch()).unwrap();
        f.set_raw(0, 0, if charm { 3.0 } else { 0.5 });
        let second = f.plan().unwrap();
        let after = second.evaluate(&mut second.new_scratch()).unwrap();
        assert_ne!(first.identity(), second.identity());
        assert_ne!(before, after);
        let mut scratch = first.new_scratch();
        for _ in 0..3 {
            assert_eq!(first.evaluate(&mut scratch).unwrap(), before);
            assert_eq!(second.evaluate(&mut scratch).unwrap(), after);
        }
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let (first, second, before, after) = (&first, &second, &before, &after);
                scope.spawn(move || {
                    let mut scratch = first.new_scratch();
                    assert_eq!(first.evaluate(&mut scratch).unwrap(), *before);
                    assert_eq!(second.evaluate(&mut scratch).unwrap(), *after);
                });
            }
        });
        f.build.equipment[0].scope = LoadoutScope::Selected {
            loadouts: vec![occurrence(2)],
        };
        let inactive = f.plan().unwrap();
        let report = inactive.evaluate(&mut inactive.new_scratch()).unwrap();
        assert!(!report.effects.iter().any(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider {provider} if matches!(provider.root, ProviderRoot::ItemModifier {equipment_use, ..} if equipment_use == occurrence(6)))));
    }
}

#[test]
#[ignore = "requires verified Fine Belt endpoint and actual original-05 inputs"]
fn missing_inputs_and_partial_owner_coverage_remain_visible() {
    let inputs = Inputs::load();
    for charm in [true, false] {
        let b = family::bindings();
        let b = if charm { b.charm } else { b.flask };
        for slot in [b.amount, b.corrupted_base, b.category] {
            let mut f = inputs.fixture(charm, None);
            f.build.items[0].modifiers[0]
                .rolls
                .retain(|r| r.slot != slot);
            assert!(
                matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
            );
        }
        let mut wrong_unit = inputs.fixture(charm, None);
        let amount = wrong_unit.family.amount.clone();
        let unit = wrong_unit.family.factor_unit.clone();
        wrong_unit.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot == amount)
            .unwrap()
            .value = ParameterValue::Quantity(FiniteQuantity::new(2.0, unit).unwrap());
        assert!(
            matches!(wrong_unit.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
        for missing in ["corrupted-base-factor", "ordered-magnitude-scalar"] {
            let mut f = inputs.fixture(charm, None);
            for owner in &mut f.recipe.rules.owners {
                owner.programs.members.retain(|p| p.id.as_str() != missing);
            }
            let plan = f.plan().unwrap();
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            assert!(matches!(
                f.effective(&report, 6, 4),
                EffectValue::Unresolved { .. }
            ));
        }
        // The actual source supplies no catalyst-matching property. A lazy
        // literal-one branch has no dependency on the inapplicable amount.
        let mut inert = inputs.fixture(charm, None);
        for owner in &mut inert.recipe.rules.owners {
            owner
                .programs
                .members
                .retain(|p| p.id.as_str() != "fixture-catalyst-amount");
        }
        evaluate(&inert, if charm { 2.0 } else { 0.17 });
        // In a separate synthetic control, making the matching property true
        // requires that same producer. Never infer this flag from English text.
        let matching = inert.family.properties["life"].clone();
        inert.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot == matching)
            .unwrap()
            .value = ParameterValue::Boolean(true);
        let plan = inert.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(
            inert.effective(&report, 6, 4),
            EffectValue::Unresolved { .. }
        ));
        let mut f = inputs.fixture(charm, None);
        f.restore_partial_rules();
        let p = f.plan().unwrap();
        assert!(!p.evaluate(&mut p.new_scratch()).unwrap().gaps.is_empty());
    }
}
