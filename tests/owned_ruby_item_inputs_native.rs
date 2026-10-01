//! Actual imported Ruby inputs and compiled programs in a finite numerical fixture.
//! This does not supply a damage receiver, aggregate, or whole-build parity.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "support/owned_fire_damage_modifier.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_ruby_item_inputs.rs"]
mod physical;
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
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn assignments(collection: &Value) -> Vec<ParameterAssignment> {
    assert_eq!(collection["completion"], json!({"kind":"complete"}));
    collection["members"]
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
fn only(rows: &Value, predicate: impl Fn(&Value) -> bool) -> &Value {
    let found: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| predicate(v))
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn source_amount(snapshot: &Value) -> f64 {
    let records = snapshot["active"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record["name"], "FireDamage");
    assert_eq!(record["type"], "INC");
    assert_eq!(record["flags"], 0);
    assert_eq!(record["keyword_flags"], 0);
    record["value"].as_f64().unwrap()
}

struct Inputs {
    endpoint: StagedOwnedRelease,
    item: Value,
    source: Value,
}
impl Inputs {
    fn load() -> Self {
        let source_bytes = fs::read(
            std::env::var_os("POE_OPTIMIZER_TEST_RUBY_ITEM_INPUTS_SOURCE")
                .expect("set to the verified complete imported-item source evidence"),
        )
        .unwrap();
        let authoring = physical::authoring();
        assert_eq!(authoring["source_validation"]["status"], "passed");
        assert_eq!(
            format!("{:x}", Sha256::digest(&source_bytes)),
            authoring["source_validation"]["evidence_sha256"]
                .as_str()
                .unwrap()
        );
        let reference: Value = serde_json::from_slice(&source_bytes).unwrap();
        assert_eq!(reference["source_revision"], authoring["source_revision"]);
        assert_eq!(
            reference["source_hash"],
            authoring["source_manifest_sha256"]
        );
        assert_eq!(reference["evidence"]["source_ordinal"], 171);
        assert_eq!(reference["evidence"]["item_id"], 1);
        assert_eq!(reference["evidence"]["selected_jewel_node"], 46882);
        let original = only(&reference["cases"], |c| c["name"] == "original-04");
        let xml = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/builds/breadth-20260908/build-04.xml"),
        )
        .unwrap();
        assert_eq!(original["xml_sha256"], format!("{:x}", Sha256::digest(xml)));
        for field in [
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "calcs_output_preserved",
            "original_functions_preserved",
            "fresh_loaded_items",
            "loaded_fresh_equivalent_except_legacy_range",
        ] {
            assert_eq!(original["state"][field], true, "{field}");
        }
        assert_eq!(original["state"]["selected"]["spec"], 1);
        let source = only(&original["state"]["items"], |i| i["id"] == 1).clone();
        assert_eq!(source["node"], 46882);
        assert_eq!(source["saved_socket"]["attrib"]["itemId"], "1");
        assert_eq!(source["saved_socket"]["attrib"]["nodeId"], "46882");
        assert_eq!(source["selected_slots"], json!(["Jewel 46882"]));
        assert_eq!(source["loaded"]["rarity"], "RARE");
        assert_eq!(source["loaded"]["requirements"]["level"], 0);
        assert_eq!(source["loaded"]["itemSocketCount"], 0);
        assert_eq!(source["loaded"]["advancedCopy"], false);
        for field in [
            "crafted",
            "corrupted",
            "quality",
            "catalyst",
            "catalystQuality",
        ] {
            assert_eq!(source["loaded"]["field_types"][field], "nil", "{field}");
        }
        assert_eq!(source_amount(&source["loaded"]), 14.0);
        assert_eq!(source_amount(&source["fresh"]), 14.0);

        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_RUBY_ITEM_INPUTS_OUTPUT")
                .expect("set to the verified Ruby physical-input publication output"),
        );
        let endpoint = release::load(&output.join("package"));
        let read = |name| -> Value {
            serde_json::from_slice(&fs::read(output.join(name)).unwrap()).unwrap()
        };
        let draft = read("original-04/draft.json");
        let sidecar = read("original-04/sidecar.json");
        let origin = only(&sidecar["origins"], |o| o["source"]["ordinal"] == 171);
        let link = only(&origin["links"], |l| l["kind"] == "item");
        let item = only(&draft["draft"]["items"]["members"], |i| {
            i["id"] == link["value"]
        })
        .clone();
        let b = family::bindings();
        assert_eq!(
            typed::<ItemTemplateDefId>(&item["template"]["value"]),
            b.templates[0]
        );
        assert_eq!(item["item_level"], json!({"kind":"known","value":55}));
        assert_eq!(item["quality"], json!({"kind":"known","value":null}));
        assert_eq!(item["modifiers"]["completion"], json!({"kind":"complete"}));
        assert_eq!(item["modifiers"]["members"].as_array().unwrap().len(), 1);
        let modifier = &item["modifiers"]["members"][0];
        assert_eq!(
            typed::<ModifierDefId>(&modifier["definition"]["value"]),
            b.modifier
        );
        let rolls = assignments(&modifier["rolls"]);
        assert_eq!(rolls.len(), 24);
        assert_eq!(
            rolls.iter().find(|r| r.slot == b.amount).unwrap().value,
            ParameterValue::Quantity(
                FiniteQuantity::new(source_amount(&source["loaded"]), b.unit).unwrap()
            )
        );
        assert!(
            rolls
                .iter()
                .filter_map(|r| match r.value {
                    ParameterValue::Boolean(value) => Some(value),
                    _ => None,
                })
                .all(|value| !value)
        );
        assert_eq!(assignments(&item["parameters"]).len(), 6);
        let recipe = &endpoint.input().recipe;
        for entry in physical::extension().schema {
            match entry {
                SchemaExtensionEntry::Definition(d) => {
                    assert!(recipe.schema.definitions.contains(&d))
                }
                SchemaExtensionEntry::Slot(s) => assert!(recipe.schema.slots.contains(&s)),
            }
        }
        for dependency in physical::dependency_definitions()
            .into_iter()
            .chain(family::dependency_definitions())
        {
            assert!(recipe.schema.definitions.contains(&dependency));
        }
        Self {
            endpoint,
            item,
            source,
        }
    }
    fn probe(&self, name: &str) -> &Value {
        let probe = only(&self.source["probes"], |p| p["name"] == name);
        assert_eq!(probe["available"], true, "{name}");
        &probe["after"]
    }
    fn expected(&self, name: &str) -> f64 {
        source_amount(self.probe(name))
    }
    fn fixture(&self, two_items: bool) -> Fixture {
        let b = family::bindings();
        let recipe = &self.endpoint.input().recipe;
        let owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.modifier.address()))
            .unwrap()
            .clone();
        assert_eq!(owner.programs.members.len(), 4);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        // The numerical compiler adds effective-amount to the three authored
        // source programs. Copy all four from the actual published endpoint.
        for program in &family::extension().owners[0].programs.members {
            assert!(owner.programs.members.contains(program));
        }
        assert!(
            owner
                .programs
                .members
                .iter()
                .any(|p| p.id.as_str() == "effective-amount")
        );
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
        assert_eq!(slots.len(), 24);
        let template = b.templates[0].clone();
        let template_owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(template.address()))
            .unwrap()
            .clone();
        assert_eq!(template_owner, physical::extension().owners[0]);
        assert!(matches!(
            template_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let mut names: Vec<_> = template_owner
            .programs
            .members
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["catalyst-inputs", "template-supplies-base-attack-profile"]
        );
        let template_slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|s| {
                s.address().declaration() == &SlotOwnerDefId::ItemTemplate(template.clone())
            })
            .cloned()
            .collect();
        assert_eq!(template_slots.len(), 6);
        let mut needed = BTreeSet::new();
        for value in [
            serde_json::to_value(&owner).unwrap(),
            serde_json::to_value(&definition).unwrap(),
            serde_json::to_value(&slots).unwrap(),
            serde_json::to_value(&template_owner).unwrap(),
            serde_json::to_value(&template_slots).unwrap(),
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
                        && d.address() != b.modifier.address()
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
        let mut schema = vec![SchemaExtensionEntry::Definition(definition)];
        schema.extend(slots.into_iter().map(SchemaExtensionEntry::Slot));
        let c = component::categories::bindings();
        let mut f = Fixture::from_compiled_effects_with_template_inputs(
            AuthoredComponent {
                bindings: ComponentBindings {
                    modifier: b.modifier,
                    amount: b.amount,
                    properties: b.properties,
                    corrupted_base: b.corrupted_base,
                    unit: b.unit.clone(),
                    factor_unit: b.factor_unit,
                    effective: b.effective.clone(),
                    // The finite constructor supplies no contribution or aggregate.
                    contribution: b.effective,
                    contribution_unit: b.unit,
                },
                extension: OwnedRecipeExtension {
                    schema_version: 1,
                    version: OwnedDefinitionKey::new("actual-ruby-input-numeric-component")
                        .unwrap(),
                    schema,
                    operations_version: Some(recipe.rules.operations_version.clone()),
                    tables: vec![],
                    owners: vec![owner.clone()],
                    receivers: vec![],
                },
                dependencies,
                numeric_policy: |_| panic!("retain published programs"),
                category: Some(CategoryBindings {
                    slot: b.category,
                    explicit: c.explicit,
                    implicit: c.implicit,
                    enchant: c.enchant,
                }),
                category_target: None,
                catalyst_property: "fire",
                catalyst_amount: 20.0,
                parameter_count: 24,
                parameters_complete: true,
                last_authored: 0x31fc,
                release: "synthetic-ruby-item-input-component",
            },
            AuthoredTemplateInputs {
                template,
                slots: template_slots,
                owner: template_owner.clone(),
                assignments: assignments(&self.item["parameters"]),
            },
        );
        if !two_items {
            f.build.items.truncate(1);
            f.build.equipment.truncate(2);
        }
        let rolls = assignments(&self.item["modifiers"]["members"][0]["rolls"]);
        for item in &mut f.build.items {
            item.item_level = Some(55);
            item.modifiers.retain(|m| m.id != occurrence(5));
            item.modifier_order.retain(|m| *m != occurrence(5));
            assert_eq!(item.modifiers.len(), 1);
            item.modifiers[0].rolls = rolls.clone();
        }
        // Only this unpublished, explicitly finite topology is closed. Every
        // published program body, slot and dependency remains identical.
        f.complete_domain();
        assert_eq!(
            f.family_owner_mut().programs.members,
            owner.programs.members
        );
        let compiled_template = f
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == template_owner.owner)
            .unwrap();
        assert_eq!(
            compiled_template.programs.members,
            template_owner.programs.members
        );
        f
    }
}

fn evaluate(f: &Fixture, expected: &[(u64, u64, f64)]) -> OwnedEffectsReport {
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for &(equipment, modifier, number) in expected {
        assert_eq!(
            f.effective(&report, equipment, modifier),
            &f.expected(number)
        );
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
fn check(f: &Fixture, expected: f64) -> OwnedEffectsReport {
    evaluate(f, &[(6, 4, expected), (7, 4, expected)])
}
fn fire(f: &mut Fixture, index: usize, enabled: bool) {
    let slot = f.family.properties["fire"].clone();
    f.build.items[index].modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot == slot)
        .unwrap()
        .value = ParameterValue::Boolean(enabled);
}
fn matching_catalyst(f: &mut Fixture, index: usize) {
    let program = f
        .family_owner_mut()
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "catalyst-scalar")
        .unwrap();
    let RuleExpression::Literal { value } = &program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == "option-fire")
        .unwrap()
        .expression
    else {
        panic!("published fire catalyst option")
    };
    let value = value.clone();
    let binding = physical::catalyst_bindings().remove(0);
    f.build.items[index]
        .parameters
        .iter_mut()
        .find(|p| p.slot == binding.selection)
        .unwrap()
        .value = value;
}
fn catalyst_amount(f: &mut Fixture, index: usize, number: f64) {
    let binding = physical::catalyst_bindings().remove(0);
    let parameter = f.build.items[index]
        .parameters
        .iter_mut()
        .find(|p| p.slot == binding.amount)
        .unwrap();
    let ParameterValue::Quantity(old) = &parameter.value else {
        panic!("published quantity")
    };
    parameter.value =
        ParameterValue::Quantity(FiniteQuantity::new(number, old.unit().clone()).unwrap());
}
fn invalid(f: &Fixture) {
    assert!(
        matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
}

#[test]
#[ignore = "requires verified Ruby physical-input publication and complete source evidence"]
fn actual_six_inputs_and_source_catalyst_contrasts_use_published_programs() {
    let inputs = Inputs::load();
    let mut f = inputs.fixture(false);
    let bindings = physical::bindings().remove(0);
    let raw = assignments(&inputs.item["parameters"]);
    let parameter = |slot: &DeclaredSlot<ParameterSlotDefId>| {
        &raw.iter().find(|p| &p.slot == slot).unwrap().value
    };
    assert_eq!(
        parameter(&bindings.corruption_slot),
        &ParameterValue::Boolean(false)
    );
    assert_eq!(
        parameter(&bindings.capacity_slot),
        &integer(inputs.source["loaded"]["itemSocketCount"].as_i64().unwrap())
    );
    let level = bindings
        .header_inputs
        .iter()
        .find(|h| h.rule.as_str() == "level-requirement")
        .unwrap();
    assert_eq!(
        parameter(&level.slot),
        &integer(
            inputs.source["loaded"]["requirements"]["level"]
                .as_i64()
                .unwrap()
        )
    );
    let rarity = bindings
        .header_inputs
        .iter()
        .find(|h| h.rule.as_str() == "rarity")
        .unwrap();
    let codec = serde_json::to_value(&rarity.codec).unwrap();
    let token = only(&codec["codec"]["value"]["tokens"], |t| {
        t["token"] == inputs.source["loaded"]["rarity"]
    });
    assert_eq!(
        parameter(&rarity.slot),
        &ParameterValue::Option(typed(&token["value"]))
    );
    for default in physical::defaults().remove(0).parameters {
        assert!(raw.contains(&default.assignment));
    }
    let catalyst = physical::catalyst_bindings().remove(0);
    let ParameterValue::Quantity(effective_default) = parameter(&catalyst.amount) else {
        panic!()
    };
    assert_eq!(effective_default.value(), 20.0);
    // The canonical effective input is twenty; the original raw source is nil.
    assert_eq!(
        inputs.source["loaded"]["field_types"]["catalystQuality"],
        "nil"
    );
    check(&f, source_amount(&inputs.source["loaded"]));
    fire(&mut f, 0, true);
    check(&f, inputs.expected("tagged-fire"));
    matching_catalyst(&mut f, 0);
    assert_eq!(
        inputs.probe("tagged-catalyst-absent-amount")["field_types"]["catalystQuality"],
        "nil"
    );
    check(&f, inputs.expected("tagged-catalyst-absent-amount"));
    assert_eq!(
        inputs.probe("tagged-catalyst-explicit-twenty")["catalystQuality"],
        20
    );
    check(&f, inputs.expected("tagged-catalyst-explicit-twenty"));
    catalyst_amount(
        &mut f,
        0,
        inputs.probe("tagged-catalyst-explicit-zero")["catalystQuality"]
            .as_f64()
            .unwrap(),
    );
    check(&f, inputs.expected("tagged-catalyst-explicit-zero"));
    fire(&mut f, 0, false);
    catalyst_amount(
        &mut f,
        0,
        inputs.probe("catalyst-both")["catalystQuality"]
            .as_f64()
            .unwrap(),
    );
    check(&f, inputs.expected("catalyst-both"));
}

#[test]
#[ignore = "requires verified Ruby physical-input publication and complete source evidence"]
fn required_inputs_ownership_units_optional_raw_level_and_partial_coverage_are_preserved() {
    let inputs = Inputs::load();
    let binding = physical::bindings().remove(0);
    let level = binding
        .header_inputs
        .iter()
        .find(|h| h.rule.as_str() == "level-requirement")
        .unwrap()
        .slot
        .clone();
    for parameter in assignments(&inputs.item["parameters"]) {
        let mut f = inputs.fixture(false);
        f.build.items[0]
            .parameters
            .retain(|p| p.slot != parameter.slot);
        if parameter.slot == level {
            // This represents absence, not a synthesized required level of zero.
            check(&f, source_amount(&inputs.source["loaded"]));
        } else {
            invalid(&f);
        }
        let mut wrong_type = inputs.fixture(false);
        wrong_type.build.items[0]
            .parameters
            .iter_mut()
            .find(|p| p.slot == parameter.slot)
            .unwrap()
            .value = if matches!(parameter.value, ParameterValue::Boolean(_)) {
            integer(0)
        } else {
            ParameterValue::Boolean(false)
        };
        invalid(&wrong_type);
    }
    let mut wrong_owner = inputs.fixture(false);
    wrong_owner.build.items[0].parameters[0].slot.declaration =
        SlotOwnerDefId::Modifier(wrong_owner.family.modifier.clone());
    assert!(matches!(
        BuildSpec::new(wrong_owner.build.clone(), OwnedInputLimits::default()),
        Err(StructuralError {
            kind: StructuralErrorKind::WrongDeclaration,
            ..
        })
    ));
    let mut wrong_unit = inputs.fixture(false);
    let catalyst = physical::catalyst_bindings().remove(0);
    wrong_unit.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot == catalyst.amount)
        .unwrap()
        .value = ParameterValue::Quantity(
        FiniteQuantity::new(20.0, wrong_unit.family.factor_unit.clone()).unwrap(),
    );
    invalid(&wrong_unit);
    let mut missing_transport = inputs.fixture(false);
    fire(&mut missing_transport, 0, true);
    matching_catalyst(&mut missing_transport, 0);
    for owner in &mut missing_transport.recipe.rules.owners {
        owner
            .programs
            .members
            .retain(|p| p.id.as_str() != "catalyst-inputs");
    }
    let plan = missing_transport.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        missing_transport.effective(&report, 6, 4),
        EffectValue::Unresolved { .. }
    ));
    for template in [true, false] {
        let mut f = inputs.fixture(false);
        if template {
            f.restore_template_rules();
        } else {
            f.restore_partial_rules();
        }
        let plan = f.plan().unwrap();
        assert!(
            !plan
                .evaluate(&mut plan.new_scratch())
                .unwrap()
                .gaps
                .is_empty()
        );
    }
}

#[test]
#[ignore = "requires verified Ruby physical-input publication and complete source evidence"]
fn physical_transport_is_occurrence_scoped_parallel_and_scratch_isolated() {
    let inputs = Inputs::load();
    let mut f = inputs.fixture(true);
    let plain = source_amount(&inputs.source["loaded"]);
    let scaled = inputs.expected("tagged-catalyst-absent-amount");
    for index in [0, 1] {
        fire(&mut f, index, true);
        matching_catalyst(&mut f, index);
    }
    catalyst_amount(&mut f, 1, 0.0);
    let before = evaluate(&f, &[(6, 4, scaled), (7, 4, scaled), (32, 31, plain)]);
    let first = f.plan().unwrap();
    catalyst_amount(&mut f, 0, 0.0);
    catalyst_amount(&mut f, 1, 20.0);
    let after = evaluate(&f, &[(6, 4, plain), (7, 4, plain), (32, 31, scaled)]);
    let second = f.plan().unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_ne!(before, after);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let (first, second, before, after) = (&first, &second, &before, &after);
            scope.spawn(move || {
                let mut scratch = first.new_scratch();
                for _ in 0..3 {
                    assert_eq!(first.evaluate(&mut scratch).unwrap(), *before);
                    assert_eq!(second.evaluate(&mut scratch).unwrap(), *after);
                }
            });
        }
    });
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let report = evaluate(&f, &[(6, 4, plain), (32, 31, scaled)]);
    assert!(!report.values.iter().any(|value| matches!(&value.key,
        PlanValueKey::Stat { entity: ConcreteEntity::Modifier(ProviderKey {
            root: ProviderRoot::ItemModifier { equipment_use, .. }, .. }), .. }
        if *equipment_use == occurrence(7))));
    f.build.active_weapon_loadout = occurrence(2);
    evaluate(&f, &[(6, 4, plain), (7, 4, plain), (32, 31, scaled)]);
}
