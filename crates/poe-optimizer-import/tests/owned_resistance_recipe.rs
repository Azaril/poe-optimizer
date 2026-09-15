//! Validate the persisted producer component through production constructors.
//! No PoB checkout, source VM, legacy profile, final metric or build-parity claim.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{ModifierDefId, OwnedDefinitionKey, RewardDefId, StatDefId},
    owned_rules::{ContributionKind, RuleEffectKind, RuleEntity},
    owned_schema::{SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits,
};
use poe_optimizer_import::{
    owned_item_lines::{
        ConvertedItemEmission, ItemField, ItemLineLimits, ItemLineOutcome, OwnedItemLinePolicy,
        decode_item_line_policy,
    },
    owned_item_source::{ItemSourceLimits, decode_item_source_policy},
    owned_recipe::{OwnedRecipeLimits, StagedOwnedRecipe, decode_owned_recipe},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn data(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/owned/poe2/3887ae68/resistance")
            .join(name),
    )
    .unwrap()
}
struct Component {
    staged: StagedOwnedRecipe,
    rules: CompiledRulePackage,
    items: OwnedItemLinePolicy,
    ids: Value,
}
impl Component {
    fn load() -> Self {
        let raw = data("recipe.json");
        let facts: Value = serde_json::from_slice(&data("source-facts.json")).unwrap();
        assert_eq!(
            facts["recipe_sha256"].as_str().unwrap(),
            format!("{:x}", Sha256::digest(&raw))
        );
        assert_eq!(facts["metric_producer"], false);
        assert_eq!(facts["range_conversion"]["implemented"], false);
        assert_eq!(facts["whole_original_native_completion"], "0/5");
        // This path deserializes every exact DTO, checks registry/schema closure,
        // compiles all programs, and validates the injected routing package.
        let staged = decode_owned_recipe(&raw, OwnedRecipeLimits::default()).unwrap();
        let rules = CompiledRulePackage::compile(
            staged.rules().input(),
            staged.schema(),
            RuleLimits::default(),
        )
        .unwrap();
        let items = decode_item_line_policy(
            &data("items-fixed.json"),
            staged.schema(),
            ItemLineLimits::default(),
        )
        .unwrap();
        let layout = decode_item_source_policy(
            &data("item-source-fixed.json"),
            &items,
            staged.schema(),
            ItemSourceLimits::default(),
        )
        .unwrap();
        assert_eq!(&layout.input().item_lines, items.identity());
        assert_eq!(items.input().definitions, *staged.schema().identity());
        Self {
            staged,
            rules,
            items,
            ids: serde_json::from_slice(&data("ids.json")).unwrap(),
        }
    }
    fn id<T: DeserializeOwned>(&self, label: &str) -> T {
        serde_json::from_value(self.ids["allocations"][label].clone()).unwrap()
    }
}

#[test]
fn persisted_recipe_and_exact_item_policies_pass_production_constructors() {
    let c = Component::load();
    assert_eq!(c.staged.registry().input().entries.len(), 2524);
    assert_eq!(c.staged.registry().input().last_issued.get(), 2524);
    assert!(c.staged.manifest().partial_rule_owners > 0);
    assert_eq!(c.staged.manifest().calculation, "not_run");
    let base: Value = serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/import/compiled/registry.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let successor = serde_json::to_value(c.staged.registry().input()).unwrap();
    assert_eq!(
        &successor["entries"].as_array().unwrap()[..2515],
        base["entries"].as_array().unwrap()
    );
}

#[test]
fn generic_signed_fixed_lines_supply_typed_rolls_to_real_owned_programs() {
    let c = Component::load();
    let mut scratch = c.rules.new_scratch();
    for (text, expected, kind) in [
        ("+17% to Cold Resistance", 17.0, "cold"),
        ("-3% to Cold Resistance", -3.0, "cold"),
        ("0% to Cold Resistance", 0.0, "cold"),
        ("349% to Cold Resistance", 349.0, "cold"),
        ("+16% to all Elemental Resistances", 16.0, "elemental"),
        ("-5% to all Elemental Resistances", -5.0, "elemental"),
    ] {
        let line = c.items.convert_line(1, text, None).unwrap();
        let ItemLineOutcome::Known { emissions, .. } = line.outcome else {
            panic!("fixed line should decode: {text}");
        };
        let [ConvertedItemEmission::Modifier { definition, rolls }] = emissions.as_slice() else {
            panic!("one modifier per fixed line");
        };
        let wanted: ModifierDefId = c.id(&format!("flat-{kind}-modifier"));
        let stat: StatDefId = c.id(&format!("{kind}-resistance-base-contributions"));
        assert_eq!(definition, &wanted);
        assert_eq!(rolls.len(), 1);
        let ParameterValue::Quantity(amount) = &rolls[0].value else {
            panic!("exact quantity roll");
        };
        assert_eq!(amount.value(), expected);
        let result = c
            .rules
            .evaluate(
                &SchemaSubject::Definition(definition.address()),
                &key("resistance-contribution"),
                &[RuleFact {
                    read: key("amount"),
                    value: rolls[0].value.clone(),
                }],
                c.staged.schema(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert!(matches!(
            &result.effects[0].effect,
            RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: actual,
                contribution: ContributionKind::Add,
                ..
            } if actual == &stat
        ));
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: rolls[0].value.clone()
            }
        );
    }
}

#[test]
fn repeated_equal_definition_lines_remain_separate_modifiers_and_missing_roll_is_unresolved() {
    let c = Component::load();
    let converted = c
        .items
        .convert_text("Sapphire Ring\n+17% to Cold Resistance\n+19% to Cold Resistance")
        .unwrap();
    assert_eq!(converted.modifiers.len(), 2);
    assert_eq!(
        converted.modifiers[0].definition,
        converted.modifiers[1].definition
    );
    assert_ne!(converted.modifiers[0].line, converted.modifiers[1].line);
    assert_ne!(converted.modifiers[0].rolls, converted.modifiers[1].rolls);
    let owner: ModifierDefId = c.id("flat-cold-modifier");
    let result = c
        .rules
        .evaluate(
            &SchemaSubject::Definition(owner.address()),
            &key("resistance-contribution"),
            &[],
            c.staged.schema(),
            &mut c.rules.new_scratch(),
        )
        .unwrap();
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Unresolved {
            input: key("amount")
        }
    );
}

#[test]
fn ranges_unknown_lines_and_metadata_do_not_become_numeric_facts() {
    let c = Component::load();
    for text in [
        "+(20-30)% to Cold Resistance",
        "+(-3--2)% to Cold Resistance",
        "{range:0.5}+(20-30)% to Cold Resistance",
        "unknown nearby effect",
    ] {
        for fraction in [None, Some(0.0), Some(0.5), Some(1.0)] {
            assert!(matches!(
                c.items.convert_line(1, text, fraction).unwrap().outcome,
                ItemLineOutcome::Pending { .. }
            ));
        }
    }
    let converted = c
        .items
        .convert_text("Sapphire Ring\nItem Level: 77\nQuality: 20\n+17% to Cold Resistance")
        .unwrap();
    assert!(matches!(converted.item_level, ItemField::Absent));
    assert!(matches!(converted.quality, ItemField::Absent));
    assert_eq!(converted.modifiers.len(), 1);
}

#[test]
fn source_reviewed_reward_programs_contribute_without_hiding_partial_effects() {
    let c = Component::load();
    let facts: Value = serde_json::from_slice(&data("source-facts.json")).unwrap();
    let mut scratch = c.rules.new_scratch();
    for row in facts["reward_outcomes"].as_array().unwrap() {
        let reward: RewardDefId = serde_json::from_value(row["target"].clone()).unwrap();
        let expected = row["values"][0]["amount"].as_f64().unwrap();
        let result = c
            .rules
            .evaluate(
                &SchemaSubject::Definition(reward.address()),
                &key("resistance-contribution"),
                &[],
                c.staged.schema(),
                &mut scratch,
            )
            .unwrap();
        let EffectDisposition::Applied {
            value: ParameterValue::Quantity(amount),
        } = &result.effects[0].disposition
        else {
            panic!("known finite reward contribution");
        };
        assert_eq!(amount.value(), expected);
        assert_eq!(
            matches!(result.owner_programs_closure, SchemaClosure::Complete),
            row["complete_effect_text"].as_bool().unwrap()
        );
    }
}

#[test]
fn obsolete_read_field_and_stale_line_binding_are_rejected() {
    let mut recipe: Value = serde_json::from_slice(&data("recipe.json")).unwrap();
    let node = recipe["rules"]["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"]["value"]["value"]["key"] == "def.00000000000009d6")
        .unwrap()["programs"]["members"][0]["nodes"][0]["expression"]
        .as_object_mut()
        .unwrap();
    let input = node.remove("input").unwrap();
    node.insert("read".into(), input);
    assert!(
        decode_owned_recipe(
            &serde_json::to_vec(&recipe).unwrap(),
            OwnedRecipeLimits::default()
        )
        .is_err()
    );
    let c = Component::load();
    let mut changed = c.items.input().clone();
    changed.version = key("changed-policy-binding");
    let changed =
        OwnedItemLinePolicy::new(changed, c.staged.schema(), ItemLineLimits::default()).unwrap();
    assert!(
        decode_item_source_policy(
            &data("item-source-fixed.json"),
            &changed,
            c.staged.schema(),
            ItemSourceLimits::default(),
        )
        .is_err()
    );
}
