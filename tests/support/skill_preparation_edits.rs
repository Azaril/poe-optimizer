// Test-only authoring of coherent identity/preparation packages. Canonical
// ordering preserves candidate sets; it never chooses a winner from a collision.
use poe_optimizer_data::game_data::GameDataPackage;
use poe_optimizer_data::skill_preparation::ExternalGemVariants;
use std::collections::BTreeMap;

#[allow(dead_code)]
pub fn rename_gem(package: &mut GameDataPackage, old: &str, new: &str) {
    for gem in &mut package.skill_preparation.gems {
        if gem.key == old {
            gem.key = new.into();
        }
    }
    for key in &mut package.skill_preparation.source.canonical_gem_order {
        if key == old {
            *key = new.into();
        }
    }
    package.skill_preparation.source.canonical_gem_order.sort();
    for key in package.skill_preparation.string_gem_for_skill.values_mut() {
        if key == old {
            *key = new.into();
        }
    }
}

pub fn refresh_lookups(package: &mut GameDataPackage) {
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut variants: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for gem in &package.skill_identities.gems {
        owners
            .entry(gem.primary_effect_id.clone())
            .or_default()
            .push(gem.key.clone());
        variants
            .entry(gem.game_id.clone())
            .or_default()
            .entry(gem.variant_id.clone())
            .or_default()
            .push(gem.key.clone());
    }
    let mut unique_owners = BTreeMap::new();
    let mut ambiguous_owners = BTreeMap::new();
    for (effect, mut keys) in owners {
        keys.sort();
        if keys.len() == 1 {
            unique_owners.insert(effect, keys.remove(0));
        } else {
            ambiguous_owners.insert(effect, keys);
        }
    }
    package.skill_preparation.table_gem_for_skill = unique_owners;
    package.skill_preparation.ambiguous_table_gem_for_skill = ambiguous_owners;
    package.skill_preparation.external_variants = variants
        .into_iter()
        .map(|(game_id, variants)| {
            let canonical_variant_order = variants.keys().cloned().collect();
            let mut unique = BTreeMap::new();
            let mut ambiguous = BTreeMap::new();
            for (variant, mut keys) in variants {
                keys.sort();
                if keys.len() == 1 {
                    unique.insert(variant, keys.remove(0));
                } else {
                    ambiguous.insert(variant, keys);
                }
            }
            ExternalGemVariants {
                game_id,
                canonical_variant_order,
                variants: unique,
                ambiguous_variants: ambiguous,
            }
        })
        .collect();
}
