//! Actual checked ring inputs and compiled cold programs in a finite topology.
//! This does not supply a final resistance contribution or a complete build.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

use component::{
    AuthoredComponent, AuthoredTemplateInputs, CategoryBindings, ComponentBindings, Fixture,
    occurrence,
};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{owned_recipe_extension::*, owned_release::StagedOwnedRelease};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn id<K: DefinitionDomain>(namespace: &GameVersionNamespace, suffix: u64) -> DefId<K> {
    DefId::parse(namespace.clone(), format!("def.{suffix:016x}")).unwrap()
}
fn references(value: &Value, found: &mut BTreeSet<DefinitionAddress>) {
    if let Ok(address) = serde_json::from_value::<DefinitionAddress>(value.clone()) {
        found.insert(address);
    }
    // Programs and schemas also embed typed DefIds directly, whereas owner
    // addresses use the tagged DefinitionAddress wrapper. Ignore slot IDs here;
    // the exact family/template slot descriptors are supplied separately.
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
fn assignments(values: &Value) -> Vec<ParameterAssignment> {
    values["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            assert_eq!(row["slot"]["kind"], "known");
            assert_eq!(row["value"]["kind"], "known");
            ParameterAssignment {
                slot: typed(&row["slot"]["value"]),
                value: typed(&row["value"]["value"]),
            }
        })
        .collect()
}
struct Inputs {
    endpoint: StagedOwnedRelease,
    ring: Value,
}
impl Inputs {
    fn load() -> Self {
        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SAPPHIRE_NATIVE_OUTPUT")
                .expect("set to the verified Sapphire publication output directory"),
        );
        let endpoint = release::load(&output.join("package"));
        let draft: Value = read(output.join("original-05/draft.json"));
        let rings: Vec<_> = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["template"]["value"]["key"] == "def.00000000000009dc")
            .collect();
        assert_eq!(rings.len(), 1);
        let ring = rings[0].clone();
        assert_eq!(ring["parameters"]["completion"], json!({"kind":"complete"}));
        assert_eq!(ring["modifiers"]["completion"], json!({"kind":"complete"}));
        Self { endpoint, ring }
    }
    fn fixture(&self, target: Option<OptionDefId>) -> Fixture {
        let recipe = &self.endpoint.input().recipe;
        let namespace = &recipe.schema.namespace;
        let modifier: ModifierDefId = id(namespace, 0x2542);
        let template: ItemTemplateDefId = id(namespace, 0x09dc);
        let owner = |address| {
            let subject = SchemaSubject::Definition(address);
            recipe
                .rules
                .owners
                .iter()
                .find(|owner| owner.owner == subject)
                .unwrap()
                .clone()
        };
        let cold_owner = owner(modifier.address());
        let template_owner = owner(template.address());
        assert_eq!(cold_owner.programs.members.len(), 4);
        assert_eq!(template_owner.programs.members.len(), 2);
        assert_eq!(
            template_owner
                .programs
                .members
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            ["catalyst-inputs", "template-supplies-base-attack-profile"]
        );
        assert!(matches!(
            cold_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(matches!(
            template_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let cold_definition = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == modifier.address())
            .unwrap()
            .clone();
        let cold_slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|s| s.address().declaration() == &SlotOwnerDefId::Modifier(modifier.clone()))
            .cloned()
            .collect();
        let item_slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|s| {
                s.address().declaration() == &SlotOwnerDefId::ItemTemplate(template.clone())
            })
            .cloned()
            .collect();
        assert_eq!(cold_slots.len(), 24);
        assert_eq!(item_slots.len(), 6);
        let mut needed = BTreeSet::new();
        for value in [
            serde_json::to_value(&cold_owner).unwrap(),
            serde_json::to_value(&template_owner).unwrap(),
            serde_json::to_value(&cold_definition).unwrap(),
            serde_json::to_value(&cold_slots).unwrap(),
            serde_json::to_value(&item_slots).unwrap(),
        ] {
            references(&value, &mut needed);
        }
        let dependencies = loop {
            let before = needed.len();
            let rows: Vec<_> = recipe
                .schema
                .definitions
                .iter()
                .filter(|d| {
                    needed.contains(&d.address())
                        && d.address() != modifier.address()
                        && d.address() != template.address()
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
        let legacy: Value = read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68/modifier-value-inputs/bindings.json"),
        );
        let bindings = legacy["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["canonical"] == serde_json::to_value(&modifier).unwrap())
            .unwrap();
        let category = component::categories::bindings();
        let mut schema = vec![SchemaExtensionEntry::Definition(cold_definition)];
        schema.extend(cold_slots.into_iter().map(SchemaExtensionEntry::Slot));
        let mut f = Fixture::from_compiled_effects_with_template_inputs(
            AuthoredComponent {
                bindings: ComponentBindings {
                    modifier: modifier.clone(),
                    amount: typed(&bindings["canonical_inputs"]["amount"]),
                    properties: typed(&bindings["property_inputs"]),
                    corrupted_base: typed(&bindings["corrupted_base_input"]),
                    unit: id(namespace, 2),
                    factor_unit: id(namespace, 1),
                    effective: id(namespace, 0x253e),
                    // Unused in this numerical-only fixture: there is no aggregate.
                    contribution: id(namespace, 0x253e),
                    contribution_unit: id(namespace, 2),
                },
                extension: OwnedRecipeExtension {
                    schema_version: 1,
                    version: key("actual-cold-numeric-component"),
                    schema,
                    operations_version: Some(recipe.rules.operations_version.clone()),
                    tables: vec![],
                    owners: vec![cold_owner.clone()],
                    receivers: vec![],
                },
                dependencies,
                numeric_policy: |_| panic!("already compiled programs must not be recompiled"),
                category: Some(CategoryBindings {
                    slot: DeclaredSlot {
                        declaration: SlotOwnerDefId::Modifier(modifier.clone()),
                        slot: id(namespace, 0x316d),
                    },
                    explicit: category.explicit,
                    implicit: category.implicit,
                    enchant: category.enchant,
                }),
                category_target: target,
                catalyst_property: "cold",
                catalyst_amount: 20.0,
                parameter_count: 24,
                parameters_complete: true,
                last_authored: 0x316d,
                release: "synthetic-sapphire-cold-component",
            },
            AuthoredTemplateInputs {
                template,
                slots: item_slots,
                owner: template_owner.clone(),
                assignments: assignments(&self.ring["parameters"]),
            },
        );
        f.build.items.truncate(1);
        f.build.equipment.truncate(2);
        f.build.items[0].modifiers.retain(|m| m.id != occurrence(5));
        let retained: BTreeSet<_> = f.build.items[0].modifiers.iter().map(|m| m.id).collect();
        f.build.items[0]
            .modifier_order
            .retain(|m| retained.contains(m));
        f.build.items[0].item_level = None;
        let cold = self.ring["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["definition"]["value"] == serde_json::to_value(&modifier).unwrap())
            .unwrap();
        assert_eq!(cold["rolls"]["completion"], json!({"kind":"complete"}));
        f.build.items[0].modifiers[0].rolls = assignments(&cold["rolls"]);
        f.complete_domain();
        for original in [&cold_owner, &template_owner] {
            let actual = f
                .recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == original.owner)
                .unwrap();
            for program in &original.programs.members {
                assert!(actual.programs.members.contains(program));
            }
        }
        f
    }
}
fn set_parameter(f: &mut Fixture, suffix: u64, value: ParameterValue) {
    f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot.slot.key().as_str() == format!("def.{suffix:016x}"))
        .unwrap()
        .value = value;
}
fn amount(f: &mut Fixture, value: f64) {
    let unit = f.family.unit.clone();
    set_parameter(
        f,
        0x09fa,
        ParameterValue::Quantity(FiniteQuantity::new(value, unit).unwrap()),
    );
}
fn kind(f: &mut Fixture, property: &str) {
    let owner = f.family_owner_mut();
    let program = owner
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "catalyst-scalar")
        .unwrap();
    let RuleExpression::Literal { value } = &program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == format!("option-{property}"))
        .unwrap()
        .expression
    else {
        panic!()
    };
    let value = value.clone();
    set_parameter(f, 0x09f9, value);
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
            .any(|e| matches!(e.target, BoundEffectTarget::Contribution { .. })),
        "no final resistance contribution is authored"
    );
    assert!(
        !report.values.iter().any(|v| matches!(
            v.key,
            PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(_),
                ..
            }
        )),
        "no final resistance aggregate is supplied"
    );
    report
}

#[test]
#[ignore = "requires verified Sapphire endpoint and original-05 normalized inputs"]
fn exact_ring_inputs_feed_actual_cold_programs_for_two_receiving_uses() {
    let inputs = Inputs::load();
    let f = inputs.fixture(None);
    evaluate(&f, 25.0);
    for (property, percentage, expected) in [
        ("cold", 20.0, 30.0),
        ("cold", 0.0, 25.0),
        ("cold", 2.0, 25.0),
        ("cold", 4.0, 26.0),
        ("fire", 20.0, 25.0),
    ] {
        let mut f = inputs.fixture(None);
        kind(&mut f, property);
        amount(&mut f, percentage);
        evaluate(&f, expected);
    }
    for (raw, expected) in [(20.0, 20.0), (25.0, 25.0), (25.5, 26.0), (30.0, 30.0)] {
        let mut f = inputs.fixture(None);
        f.set_raw(0, 0, raw);
        evaluate(&f, expected);
    }
    let mut f = inputs.fixture(None);
    let factor = f.family.corrupted_base.clone();
    let unit = f.family.factor_unit.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot == factor)
        .unwrap()
        .value = ParameterValue::Quantity(FiniteQuantity::new(1.5, unit).unwrap());
    // The numeric factor is supported independently of the Import profile,
    // which still rejects the source's corruptedRange construction.
    evaluate(&f, 38.0);
}

#[test]
#[ignore = "requires verified Sapphire endpoint and original-05 normalized inputs"]
fn shared_physical_edits_category_targeting_parallel_and_reused_scratch_are_isolated() {
    let inputs = Inputs::load();
    let mut f = inputs.fixture(None);
    let first = f.plan().unwrap();
    let before = first.evaluate(&mut first.new_scratch()).unwrap();
    kind(&mut f, "cold");
    let second = f.plan().unwrap();
    let after = second.evaluate(&mut second.new_scratch()).unwrap();
    assert_ne!(first.identity(), second.identity());
    for equipment in [6, 7] {
        assert_eq!(f.effective(&before, equipment, 4), &f.expected(25.0));
        assert_eq!(f.effective(&after, equipment, 4), &f.expected(30.0));
    }
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
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let third = f.plan().unwrap();
    let report = third.evaluate(&mut third.new_scratch()).unwrap();
    assert_eq!(f.effective(&report, 6, 4), &f.expected(30.0));
    assert!(!report.effects.iter().any(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::EquipmentUse(use_id) | ProviderRoot::ItemModifier { equipment_use: use_id, .. } if use_id == occurrence(7)))));
    let categories = component::categories::bindings();
    for (target, expected) in [(categories.implicit, 37.0), (categories.explicit, 25.0)] {
        let f = inputs.fixture(Some(target));
        // The category producer is explicitly synthetic; the numeric consumer
        // is the unchanged authored ordered-magnitude-scalar program.
        evaluate(&f, expected);
    }
}

#[test]
#[ignore = "requires verified Sapphire endpoint and original-05 normalized inputs"]
fn missing_inputs_and_producers_remain_errors_or_unknown_values() {
    let inputs = Inputs::load();
    for slot in [0x2543, 0x2674, 0x316d] {
        let mut f = inputs.fixture(None);
        f.build.items[0].modifiers[0]
            .rolls
            .retain(|p| p.slot.slot.key().as_str() != format!("def.{slot:016x}"));
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    for slot in [0x09f9, 0x09fa] {
        let mut f = inputs.fixture(None);
        f.build.items[0]
            .parameters
            .retain(|p| p.slot.slot.key().as_str() != format!("def.{slot:016x}"));
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    let mut f = inputs.fixture(None);
    let raw = f.family.amount.clone();
    let wrong_unit = f.family.factor_unit.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot == raw)
        .unwrap()
        .value = ParameterValue::Quantity(FiniteQuantity::new(25.0, wrong_unit).unwrap());
    assert!(
        matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    for program in [
        "catalyst-inputs",
        "corrupted-base-factor",
        "ordered-magnitude-scalar",
    ] {
        let mut f = inputs.fixture(None);
        for owner in &mut f.recipe.rules.owners {
            owner.programs.members.retain(|p| p.id.as_str() != program);
        }
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        for equipment in [6, 7] {
            assert!(matches!(
                f.effective(&report, equipment, 4),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    let mut f = inputs.fixture(None);
    f.restore_partial_rules();
    f.restore_template_rules();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        !report.gaps.is_empty(),
        "actual owner incompleteness is not a complete build"
    );
}
