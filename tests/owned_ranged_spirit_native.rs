//! Authored Spirit component programs; final maximum Spirit remains unbound.
// The shared fixture also exposes constructors for other authored families.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "support/owned_ranged_spirit.rs"]
mod family;
use component::{AuthoredComponent, CategoryBindings, ComponentBindings, Fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{owned_item_lines::*, owned_value::WhitespacePolicy};
use std::collections::BTreeMap;
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}

fn fixture(target: Option<OptionDefId>) -> Fixture {
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
fn clear_properties(f: &mut Fixture) {
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

#[test]
fn authored_range_transport_and_resource_projection_preserve_source_values() {
    let mut f = fixture(None);
    let schema =
        OwnedDefinitionSchemaPackage::new(f.recipe.schema.clone(), Default::default()).unwrap();
    let policy = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_V7,
            namespace: f.family.modifier.namespace().clone(),
            version: key("ranged-spirit-component"),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules: vec![family::item_rule()],
        },
        &schema,
        Default::default(),
    )
    .unwrap();
    let properties = f
        .family
        .properties
        .keys()
        .map(|name| (key(name), false))
        .collect();
    let options = BTreeMap::from([(
        key("modifier-source-category"),
        component::categories::bindings().implicit,
    )]);
    // Independent complete-source witness: raw range .1/.5/.9 formats as11/13/15.
    // The importer transports 10.5/12.5/14.5; only the native numeric recipe rounds.
    for (fraction, unrounded, expected) in [
        (0.0, 10.0, 10.0),
        (0.1, 10.5, 11.0),
        (0.5, 12.5, 13.0),
        (0.9, 14.5, 15.0),
        (1.0, 15.0, 15.0),
    ] {
        let result = policy
            .convert_lines([ItemLineInput {
                index: 1,
                text: "+(10-15) to Spirit",
                range_fraction: Some(fraction),
                properties: Some(&properties),
                option_inputs: Some(&options),
            }])
            .unwrap();
        let ItemLineOutcome::Known { emissions, .. } = &result.lines[0].outcome else {
            panic!("actual range recipe must convert")
        };
        let [
            ConvertedItemEmission::Modifier {
                definition, rolls, ..
            },
        ] = emissions.as_slice()
        else {
            panic!()
        };
        assert_eq!(definition, &f.family.modifier);
        assert_eq!(
            rolls
                .iter()
                .find(|v| v.slot == f.family.amount)
                .unwrap()
                .value,
            ParameterValue::Quantity(
                FiniteQuantity::new(unrounded, f.family.unit.clone()).unwrap()
            )
        );
        // Use the converted canonical inputs directly, not a duplicate formula.
        for item in &mut f.build.items {
            for modifier in &mut item.modifiers {
                modifier.rolls = rolls.clone();
            }
        }
        f.complete_domain();
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        for (equipment, modifier) in [(6, 4), (6, 5), (7, 4), (7, 5), (32, 31)] {
            assert_eq!(
                f.effective(&report, equipment, modifier),
                &f.expected(expected)
            );
            let effects:Vec<_>=report.effects.iter().filter(|e|matches!((&e.key.invocation.origin,&e.target),
                (RuleOrigin::Provider{provider},BoundEffectTarget::Contribution{key}) if provider.root==ProviderRoot::ItemModifier{equipment_use:occurrence(equipment),modifier:occurrence(modifier)} && provider.grant_path.is_empty() && key.entity==ConcreteEntity::Actor(ActorKey::Player) && key.stat==f.family.contribution && key.kind==ContributionKind::Add)).collect();
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0].value, f.expected_total(expected));
        }
        assert_eq!(f.total(&report), &f.expected_total(5.0 * expected));
        assert_ne!(f.expected(expected), f.expected_total(expected));
    }
}

#[test]
fn source_properties_category_and_occurrence_activation_control_spirit() {
    let mut f = fixture(None);
    f.set_raw(0, 0, 12.5);
    f.complete_domain();
    let tagged = f.plan().unwrap();
    let tagged_report = tagged.evaluate(&mut tagged.new_scratch()).unwrap();
    assert_eq!(f.effective(&tagged_report, 6, 4), &f.expected(15.0));
    clear_properties(&mut f);
    let plain = f.plan().unwrap();
    let before = plain.evaluate(&mut plain.new_scratch()).unwrap();
    assert_eq!(f.effective(&before, 6, 4), &f.expected(13.0));
    assert_eq!(f.effective(&before, 7, 4), &f.expected(13.0));
    assert_eq!(f.effective(&before, 32, 31), &f.expected(7.0));
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    f.set_raw(0, 0, 15.0);
    let changed = f.plan().unwrap();
    let after = changed.evaluate(&mut changed.new_scratch()).unwrap();
    assert_eq!(f.effective(&after, 6, 4), &f.expected(15.0));
    assert_eq!(f.effective(&after, 32, 31), &f.expected(7.0));
    assert!(!after.effects.iter().any(|e| matches!(&e.key.invocation.origin,RuleOrigin::Provider{provider} if matches!(provider.root,ProviderRoot::EquipmentUse(id)|ProviderRoot::ItemModifier{equipment_use:id,..} if id==occurrence(7)))));
    let mut scratch = plain.new_scratch();
    for _ in 0..3 {
        assert_eq!(plain.evaluate(&mut scratch).unwrap(), before);
        assert_eq!(changed.evaluate(&mut scratch).unwrap(), after);
    }
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let (plain, changed, before, after) = (&plain, &changed, &before, &after);
            scope.spawn(move || {
                let mut scratch = plain.new_scratch();
                assert_eq!(plain.evaluate(&mut scratch).unwrap(), *before);
                assert_eq!(changed.evaluate(&mut scratch).unwrap(), *after);
            });
        }
    });
    let c = component::categories::bindings();
    let mut f = fixture(Some(c.implicit));
    clear_properties(&mut f);
    for (item, modifier) in [(0, 0), (0, 1), (1, 0)] {
        f.set_raw(item, modifier, 12.5);
    }
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(f.effective(&report, 6, 4), &f.expected(13.0));
    assert_eq!(f.effective(&report, 6, 5), &f.expected(19.0));
    assert_eq!(f.effective(&report, 32, 31), &f.expected(13.0));
}

#[test]
fn missing_inputs_scalars_and_original_partial_owner_never_become_zero() {
    for slot in [family::bindings().amount, family::bindings().category] {
        let mut f = fixture(None);
        f.complete_domain();
        f.build.items[0].modifiers[0]
            .rolls
            .retain(|v| v.slot != slot);
        assert!(
            matches!(f.plan(),Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    let mut wrong_unit = fixture(None);
    wrong_unit.complete_domain();
    wrong_unit.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|roll| roll.slot == wrong_unit.family.amount)
        .unwrap()
        .value = ParameterValue::Quantity(
        FiniteQuantity::new(12.5, wrong_unit.family.contribution_unit.clone()).unwrap(),
    );
    assert!(
        matches!(wrong_unit.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    for missing in [
        "fixture-catalyst-amount",
        "corrupted-base-factor",
        "ordered-magnitude-scalar",
    ] {
        let mut f = fixture(None);
        f.complete_domain();
        for owner in &mut f.recipe.rules.owners {
            owner.programs.members.retain(|p| p.id.as_str() != missing);
        }
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(
            f.effective(&report, 6, 4),
            EffectValue::Unresolved { .. }
        ));
        assert!(matches!(f.total(&report), EffectValue::Unresolved { .. }));
    }
    let mut f = fixture(None);
    f.complete_domain();
    f.restore_partial_rules();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert!(matches!(
        f.total(&report),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}
