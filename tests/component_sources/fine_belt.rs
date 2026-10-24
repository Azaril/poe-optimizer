//! Actual Fine Belt modifier programs in an explicitly finite numerical domain.
//! Local charm capacity and player flask recovery remain separate consumers.
use super::{fine_belt_family as family, native as component, release};

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
                support_source_domains: vec![],
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
            .modifier_orders
            .contributions
            .retain(|m| retained.contains(m));
        f.build.items[0]
            .modifier_orders
            .numeric_transforms
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

// The existing numerical-only controls above stay unchanged. This separate,
// finite topology adds the actual authored delivery to the same four programs.
// Reuse the checked stage/support fixture without changing any authored IDs.
#[allow(dead_code)]
#[path = "../../crates/poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod flask_delivery_base;
mod support {
    pub use super::flask_delivery_base::Fixture;
    use poe_optimizer_core::owned_definitions::*;
    pub fn key(value: &str) -> OwnedDefinitionKey {
        OwnedDefinitionKey::new(value).unwrap()
    }
    pub fn def(name: &str) -> UnitDefId {
        assert_eq!(
            name, "percent",
            "the empty-support helper requests only its quality unit"
        );
        UnitDefId::parse(
            GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
            "def.0000000000000002",
        )
        .unwrap()
    }
}
#[allow(dead_code)]
#[path = "../../crates/poe-optimizer-engine/tests/support/owned_empty_support_contribution_fixture.rs"]
mod checked_empty_support;

mod flask_delivery {
    use super::*;
    use poe_optimizer_core::owned_readiness::{ReadinessPhase, ReadinessProgramRole};
    use poe_optimizer_core::owned_rules::*;
    use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
    use poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput;
    use sha2::{Digest, Sha256};

    const PROGRAM: &str = "contribute-player-flask-charge-generation";
    const EFFECT: &str = "direct-flask-charge-generation";
    const APPLICABILITY: &str = "ordinary-item-direct-applicability";

    fn packet() -> Value {
        let bytes = fs::read(family::data().join("flask-charge-delivery.json")).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            "f6ad7fc98631cc9803a5e68f178f8c4e658b7f06c4be17867330661811e8f9b1"
        );
        serde_json::from_slice(&bytes).unwrap()
    }

    fn fixture() -> Fixture {
        let p = packet();
        let b = family::bindings().flask;
        let owner: DefinitionRules = typed(&p["owner_before"]);
        assert_eq!(owner.programs.members.len(), 4);
        let descriptor: DefinitionDescriptor = typed(&p["definition"]);
        let DefinitionDescriptor::Stat(channel) = &descriptor else {
            panic!("one Actor rate channel")
        };
        let channel = channel.id.clone();
        let mut dependencies: Vec<DefinitionDescriptor> = typed(&p["dependencies"]);
        dependencies.push(descriptor);
        let mut schema = vec![SchemaExtensionEntry::Definition(typed(
            &p["modifier_descriptor"],
        ))];
        schema.extend(
            typed::<Vec<SlotDescriptor>>(&p["slots"])
                .into_iter()
                .map(SchemaExtensionEntry::Slot),
        );
        let categories = component::categories::bindings();
        let mut f = Fixture::from_compiled_effects(AuthoredComponent {
            bindings: ComponentBindings {
                modifier: b.modifier,
                amount: b.amount,
                properties: b.properties,
                corrupted_base: b.corrupted_base,
                unit: b.unit.clone(),
                contribution_unit: b.unit,
                factor_unit: b.factor_unit,
                effective: b.effective,
                contribution: channel,
            },
            extension: OwnedRecipeExtension {
                support_source_domains: vec![],
                schema_version: 1,
                version: key("finite-flask-delivery-component"),
                schema,
                operations_version: Some(typed(&p["operations_version"])),
                tables: vec![],
                owners: vec![owner.clone()],
                receivers: vec![],
            },
            dependencies,
            numeric_policy: |_| panic!("use the actual four compiled programs"),
            category: Some(CategoryBindings {
                slot: b.category,
                explicit: categories.explicit,
                implicit: categories.implicit,
                enchant: categories.enchant,
            }),
            category_target: None,
            catalyst_property: "life",
            catalyst_amount: 0.0,
            parameter_count: 24,
            parameters_complete: true,
            last_authored: 0x33d0,
            release: "finite-flask-delivery-component",
        });
        f.build.items.truncate(1);
        f.build.equipment.truncate(1);
        f.build.items[0].modifiers.truncate(1);
        f.build.items[0].modifier_orders.contributions.truncate(1);
        f.build.items[0]
            .modifier_orders
            .numeric_transforms
            .truncate(1);
        f.build.items[0].modifiers[0].rolls = assignments(&p["actual_rolls"]);
        assert_eq!(f.family_owner_mut().programs, owner.programs);
        f.family_owner_mut()
            .programs
            .members
            .push(typed(&p["program"]));
        let template = SchemaSubject::Definition(f.build.items[0].template.address());
        f.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == template)
            .unwrap()
            .programs
            .members
            .push(typed(&p["applicability_program"]));
        // The existing finite fixture owns this complete consumer census. This
        // diagnostic fold is not a production aggregate or metric declaration.
        let sum: RuleProgram = typed(&json!({
            "id":"fixture-flask-rate-sum","context":"actor",
            "reads":[{"id":"incoming","value_type":{"kind":"quantity","value":{"unit":f.family.unit}},
                "source":{"kind":"contributions","value":{"entity":"player","stat":f.family.contribution,
                    "contribution":"add","reduction":"sum","empty":{"kind":"quantity","value":{"value":0.0,"unit":f.family.unit}}}}}],
            "nodes":[{"id":"sum","expression":{"kind":"read","input":"incoming"}}],
            "effects":[{"id":"total","when":null,"effect":{"kind":"derive","entity":"player","stat":f.family.contribution,"value":"sum"}}]
        }));
        let class = SchemaSubject::Definition(f.build.character.class.address());
        f.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == class)
            .unwrap()
            .programs
            .members
            .push(sum);
        f.complete_domain();
        f
    }

    fn staged_plan(
        f: &Fixture,
        retain_readiness: bool,
    ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        assert_eq!(
            f.recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V28
        );
        assert!(
            f.build.gems.is_empty() && f.build.skills.is_empty() && f.build.supports.is_empty()
        );
        assert!(f.recipe.routing.outputs.is_empty());
        let domain = flask_delivery_base::Fixture {
            schema: f.recipe.schema.clone(),
            build: f.build.clone(),
            scenario: f.scenario.clone(),
            queries: QueryInput {
                game_version: f.build.game_version.clone(),
                requests: vec![],
            },
            owners: f.recipe.rules.owners.clone(),
            tables: f.recipe.rules.tables.clone(),
            receivers: f.recipe.rules.receivers.clone(),
            routes: f.recipe.routing.outputs.clone(),
        };
        // The helper's three support-input schema placeholders are unused in
        // this explicitly empty support domain. All actual data/program IDs,
        // source occurrences, orders and coverage reach the compiler unchanged.
        checked_empty_support::checked_plan_with_stages(
            &domain,
            f.recipe.rules.clone(),
            f.recipe.routing.clone(),
            Default::default(),
            |stages| {
                let readiness = stages.readiness.as_ref().unwrap();
                assert_eq!(
                    readiness.programs.members.len(),
                    f.recipe
                        .rules
                        .owners
                        .iter()
                        .map(|o| o.programs.members.len())
                        .sum::<usize>()
                );
                assert!(readiness.programs.is_complete());
                for owner in &f.recipe.rules.owners {
                    for program in &owner.programs.members {
                        let declarations: Vec<_> = readiness
                            .programs
                            .members
                            .iter()
                            .filter(|p| p.owner == owner.owner && p.program == program.id)
                            .collect();
                        assert_eq!(declarations.len(), 1);
                        assert_eq!(declarations[0].phase, ReadinessPhase::Execution);
                        assert_eq!(declarations[0].role, ReadinessProgramRole::Execution);
                    }
                }
                if !retain_readiness {
                    stages.readiness = None;
                }
            },
        )
    }

    fn report(f: &Fixture) -> OwnedEffectsReport {
        let plan = staged_plan(f, true).unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
            panic!(
                "the explicit empty support domain must evaluate: {:?}",
                report.outcome
            );
        };
        effects
    }

    #[test]
    fn flask_delivery_requires_checked_v28_readiness() {
        let f = fixture();
        assert!(
            matches!(staged_plan(&f, false), Err(PlanError::Invalid(message))
            if message.contains("readiness"))
        );
        assert!(matches!(f.plan(), Err(PlanError::Invalid(message))
            if message == "operation v16 requires checked readiness stages"));
    }

    fn deliveries(report: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
        report
            .effects
            .iter()
            .filter(|effect| {
                effect.key.invocation.program == key(PROGRAM) && effect.key.effect == key(EFFECT)
            })
            .collect()
    }

    fn assert_delivery(f: &Fixture, effect: &BoundEffectResult, modifier: u64, expected: f64) {
        assert_eq!(
            effect.key.invocation.origin,
            RuleOrigin::Provider {
                provider: ProviderKey {
                    root: ProviderRoot::ItemModifier {
                        equipment_use: occurrence(6),
                        modifier: occurrence(modifier)
                    },
                    grant_path: vec![],
                },
            }
        );
        assert_eq!(
            effect.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: f.family.contribution.clone(),
                    kind: ContributionKind::Add
                },
            }
        );
        assert_eq!(effect.value, f.expected(expected));
    }

    #[test]
    fn flask_delivery_payload_preserves_current_programs_and_all_five_gaps() {
        let p = packet();
        let migration_bytes =
            fs::read(family::data().join("flask-charge-delivery-migration.json")).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&migration_bytes)),
            "e16a0a61309e91c766947bb92ead1f446f5e1a4c8200baad06bf0f06006f4812"
        );
        let migration: OwnedReleaseMigrationInput =
            serde_json::from_slice(&migration_bytes).unwrap();
        let before: DefinitionRules = typed(&p["owner_before"]);
        let SchemaClosure::Partial { gaps } = &before.programs.closure else {
            panic!()
        };
        assert_eq!(gaps.len(), 5);
        assert!(
            gaps.iter()
                .any(|g| g.code == key("flask-charge-rate-delivery-unconverted"))
        );
        assert_eq!(
            migration.before.to_string(),
            p["predecessor_input"].as_str().unwrap()
        );
        assert_eq!(migration.schema_version, 5);
        assert_eq!(
            migration.contract.operations_version,
            typed::<OwnedDefinitionKey>(&p["operations_version"])
        );
        assert_eq!(
            migration.schema,
            vec![SchemaExtensionEntry::Definition(typed(&p["definition"]))]
        );
        assert_eq!(migration.owners.len(), 1);
        assert_eq!(migration.owners[0].owner, before.owner);
        assert_eq!(
            migration.owners[0].programs.closure,
            before.programs.closure
        );
        assert_eq!(
            migration.owners[0].programs.members,
            vec![typed::<RuleProgram>(&p["program"])]
        );
        assert!(
            migration.tables.is_empty()
                && migration.receivers.is_empty()
                && migration.query_targets.is_empty()
        );
        assert!(migration.evaluation.is_none());
        let mut f = fixture();
        assert_eq!(
            &f.family_owner_mut().programs.members[..4],
            before.programs.members
        );
        assert_eq!(f.family_owner_mut().programs.members.len(), 5);
        f.restore_partial_rules();
        assert_eq!(
            f.family_owner_mut().programs.closure,
            before.programs.closure
        );
    }

    #[test]
    fn flask_delivery_preserves_present_zero_and_actual_non_neutral_numeric_chain() {
        for (raw, corruption, expected) in [
            (0.0, 1.0, 0.0),
            (0.17, 1.0, 0.17),
            (0.175, 1.0, 0.18),
            (0.001, 1.0, 0.0),
            (0.125, 1.0, 0.13),
            (0.17, 1.5, 0.25),
        ] {
            let mut f = fixture();
            f.set_raw(0, 0, raw);
            factor(&mut f, corruption);
            let r = report(&f);
            assert!(r.gaps.is_empty(), "{:?}", r.gaps);
            assert_eq!(f.effective(&r, 6, 4), &f.expected(expected));
            let rows = deliveries(&r);
            assert_eq!(
                rows.len(),
                1,
                "zero is a present contribution, not dropped membership"
            );
            assert_delivery(&f, rows[0], 4, expected);
            assert_eq!(f.total(&r), &f.expected_total(expected));
        }
    }

    #[test]
    fn flask_delivery_keeps_duplicate_occurrences_and_ignores_storage_order() {
        let mut f = fixture();
        f.set_raw(0, 0, 0.125);
        let mut second = f.build.items[0].modifiers[0].clone();
        second.id = occurrence(5);
        f.build.items[0].modifiers.push(second);
        f.build.items[0]
            .modifier_orders
            .contributions
            .push(occurrence(5));
        f.build.items[0]
            .modifier_orders
            .numeric_transforms
            .push(occurrence(5));
        let r = report(&f);
        let rows = deliveries(&r);
        assert_eq!(rows.len(), 2);
        for modifier in [4, 5] {
            let row = rows.iter().find(|e| matches!(&e.key.invocation.origin,
                RuleOrigin::Provider { provider } if matches!(provider.root,
                    ProviderRoot::ItemModifier { modifier: actual, .. } if actual == occurrence(modifier)))).unwrap();
            assert_delivery(&f, row, modifier, 0.13);
        }
        assert_eq!(f.total(&r), &f.expected_total(0.26));
        f.build.items[0].modifiers.reverse();
        let reordered = report(&f);
        assert_eq!(deliveries(&reordered), rows);
        assert_eq!(f.total(&reordered), f.total(&r));
    }

    #[test]
    fn flask_delivery_uses_selected_equipment_and_preserves_proven_absence() {
        let mut f = fixture();
        f.build.equipment[0].scope = LoadoutScope::Selected {
            loadouts: vec![occurrence(2)],
        };
        let inactive = report(&f);
        assert!(deliveries(&inactive).is_empty());
        assert_eq!(f.total(&inactive), &f.expected_total(0.0));
        f.build.active_weapon_loadout = occurrence(2);
        let active = report(&f);
        assert_eq!(deliveries(&active).len(), 1);
        assert_delivery(&f, deliveries(&active)[0], 4, 0.17);
        f.build.equipment.clear();
        let unequipped = report(&f);
        assert!(deliveries(&unequipped).is_empty());
        assert_eq!(f.total(&unequipped), &f.expected_total(0.0));
        let mut empty = fixture();
        empty.build.items[0].modifiers.clear();
        empty.build.items[0].modifier_orders.contributions.clear();
        empty.build.items[0]
            .modifier_orders
            .numeric_transforms
            .clear();
        let absent = report(&empty);
        assert!(deliveries(&absent).is_empty());
        assert_eq!(empty.total(&absent), &empty.expected_total(0.0));
    }

    #[test]
    fn flask_delivery_false_routing_skips_missing_strength_but_unknown_is_not_false() {
        let mut f = fixture();
        for o in &mut f.recipe.rules.owners {
            if let Some(p) = o
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key(APPLICABILITY))
            {
                p.nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Boolean(false),
                };
            }
            o.programs
                .members
                .retain(|p| p.id != key("effective-amount"));
        }
        let inactive = report(&f);
        assert_eq!(deliveries(&inactive).len(), 1);
        assert_eq!(deliveries(&inactive)[0].value, EffectValue::Inactive);
        assert_eq!(f.total(&inactive), &f.expected_total(0.0));
        for missing in [APPLICABILITY, "effective-amount"] {
            let mut f = fixture();
            for o in &mut f.recipe.rules.owners {
                o.programs.members.retain(|p| p.id != key(missing));
            }
            let r = report(&f);
            assert_eq!(deliveries(&r).len(), 1);
            assert!(
                matches!(
                    deliveries(&r)[0].value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ),
                "{missing}: {:?}",
                deliveries(&r)[0].value
            );
            assert!(matches!(
                f.total(&r),
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ));
        }
    }

    #[test]
    fn flask_delivery_missing_raw_input_and_actual_partial_owner_never_become_zero() {
        let mut f = fixture();
        let amount = f.family.amount.clone();
        f.build.items[0].modifiers[0]
            .rolls
            .retain(|r| r.slot != amount);
        assert!(
            matches!(staged_plan(&f, true), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
        let mut f = fixture();
        f.restore_partial_rules();
        let p = packet();
        let mut expected: DefinitionRules = typed(&p["owner_before"]);
        let SchemaClosure::Partial { gaps } = &expected.programs.closure else {
            panic!("the actual Flask owner remains Partial")
        };
        assert_eq!(gaps.len(), 5);
        expected.programs.members.push(typed(&p["program"]));
        assert_eq!(&*f.family_owner_mut(), &expected);
        // The checked support boundary refuses the incomplete selected owner
        // before execution, so there is no numerical effect report to inspect.
        let plan = staged_plan(&f, true).unwrap();
        let r = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(
            r.gaps
                .iter()
                .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                    && gap.subject == Some(SchemaSubject::Definition(f.family.modifier.address()))
                    && gap.provider
                        == Some(ProviderKey {
                            root: ProviderRoot::ItemModifier {
                                equipment_use: occurrence(6),
                                modifier: occurrence(4)
                            },
                            grant_path: vec![]
                        }))
        );
        assert_eq!(
            r.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None
            }
        );
    }
}
