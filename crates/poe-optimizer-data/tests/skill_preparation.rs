use poe_optimizer_data::game_data::*;
use std::{collections::BTreeMap, sync::OnceLock};
fn package() -> &'static GameDataPackage {
    static DATA: OnceLock<GameDataPackage> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap().package().clone())
}
#[test]
fn complete_preparation_definitions_preserve_source_semantics_and_identity_seam() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.skill_preparation();
    let data = catalog.data();
    assert_eq!(snapshot.identity().schema_version, 31);
    assert_eq!(data.schema_version, SKILL_PREPARATION_SCHEMA_VERSION);
    assert_eq!(data.gems.len(), 966);
    assert_eq!(data.effects.len(), 1436);
    assert_eq!(
        data.effects.iter().map(|s| s.levels.len()).sum::<usize>(),
        22004
    );
    assert!(data.string_gem_for_skill.is_empty());
    assert!(data.ambiguous_table_gem_for_skill.is_empty());
    assert_eq!(catalog.gems_ordered().count(), data.gems.len());
    for gem in &package().skill_identities.gems {
        assert_eq!(
            catalog.table_gem_for_skill(&gem.primary_effect_id),
            Some(gem.key.as_str())
        );
        assert!(
            catalog
                .string_gem_for_skill(&gem.primary_effect_id)
                .is_none()
        );
        assert!(catalog.gem(&gem.key).is_some());
    }
    let effect = catalog.effect("TwisterPlayer").unwrap();
    assert_eq!(effect.level(19.0).unwrap().key, 19.0);
    assert_eq!(effect.level(effect.levels_length as f64 + 1.0), None);
    assert!(data.source.operations.contains_key("stat_requirement"));
    assert_eq!(
        data.default_gem_level_options,
        ["normalMaximum", "corruptedMaximum", "characterLevel"]
    );
    assert_eq!(data.default_gem_quality, 0.0);
    assert_eq!(data.maximum_gem_quality, 23.0);
}
#[test]
fn injected_formula_level_rows_and_colors_change_content_identity() {
    let mut modified = package().clone();
    modified.skill_preparation.requirement.level_multiplier += 0.2;
    modified.skill_preparation.colors.strength = "^xABCDEF".into();
    let row_id = modified
        .skill_preparation
        .effects
        .iter()
        .find(|s| s.id == "TwisterPlayer")
        .unwrap()
        .level(19.0)
        .unwrap()
        .row_id
        .clone();
    for row in modified
        .skill_preparation
        .effects
        .iter_mut()
        .flat_map(|s| &mut s.levels)
        .filter(|r| r.row_id == row_id)
    {
        row.level_requirement = Some(row.level_requirement.unwrap() + 7.0);
    }
    modified.refresh_section_digests().unwrap();
    let loaded = GameDataLoader::from_bytes(
        &modified.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    assert_ne!(loaded.identity(), bundled_snapshot().unwrap().identity());
    assert_eq!(
        loaded.skill_preparation().data().colors.strength,
        "^xABCDEF"
    );
    assert_ne!(
        loaded
            .skill_preparation()
            .effect("TwisterPlayer")
            .unwrap()
            .level(19.0),
        package()
            .skill_preparation
            .effects
            .iter()
            .find(|s| s.id == "TwisterPlayer")
            .unwrap()
            .level(19.0)
    );
}
#[test]
fn alias_inconsistency_invalid_boundaries_and_nonfinite_policy_are_rejected() {
    let mut data = package().skill_preparation.clone();
    let index = data
        .effects
        .iter()
        .position(|s| s.levels.len() >= 2)
        .unwrap();
    let row = data.effects[index].levels[0].clone();
    data.effects[index].levels[1].row_id = row.row_id.clone();
    data.effects[index].levels[1].cost = row.cost.clone();
    data.effects[index].levels[1].level_requirement = row.level_requirement;
    data.validate(&package().skill_identities).unwrap();
    data.effects[index].levels[1].cost = Some(BTreeMap::from([("synthetic".into(), 1.0)]));
    assert!(
        data.validate(&package().skill_identities)
            .unwrap_err()
            .0
            .contains("aliased")
    );
    let mut data = package().skill_preparation.clone();
    data.effects[index].levels_length = 500001;
    assert!(data.validate(&package().skill_identities).is_err());
    let mut data = package().skill_preparation.clone();
    data.requirement.attribute_divisor = f64::NAN;
    assert!(data.validate(&package().skill_identities).is_err());
    let mut data = package().skill_preparation.clone();
    data.source.canonical_gem_order.swap(0, 1);
    assert!(data.validate(&package().skill_identities).is_err());
}
#[test]
fn duplicate_effect_and_external_variant_owners_remain_explicitly_ambiguous() {
    let mut identities = package().skill_identities.clone();
    let mut data = package().skill_preparation.clone();
    let original = identities.gems[0].clone();
    let mut gem = original.clone();
    gem.key = "custom-colliding-gem".into();
    let mut declaration =
        identities.gem_declarations[(original.winning_declaration - 1) as usize].clone();
    declaration.index = identities.gem_declarations.len() as u32 + 1;
    declaration.key = gem.key.clone();
    gem.winning_declaration = declaration.index;
    identities.gem_declarations.push(declaration);
    identities.gems.push(gem.clone());
    identities.gems.sort_by(|a, b| a.key.cmp(&b.key));
    identities.missing_references = identities.expected_missing_references();
    identities.validate().unwrap();
    let mut prepared = data
        .gems
        .iter()
        .find(|g| g.key == original.key)
        .unwrap()
        .clone();
    prepared.key = gem.key.clone();
    data.gems.push(prepared);
    data.source.canonical_gem_order.push(gem.key.clone());
    data.source.canonical_gem_order.sort();
    assert!(data.validate(&identities).is_err());
    let mut candidates = vec![original.key.clone(), gem.key.clone()];
    candidates.sort();
    data.table_gem_for_skill.remove(&original.primary_effect_id);
    data.ambiguous_table_gem_for_skill
        .insert(original.primary_effect_id.clone(), candidates.clone());
    let external = data
        .external_variants
        .iter_mut()
        .find(|e| e.game_id == original.game_id)
        .unwrap();
    external.variants.remove(&original.variant_id);
    external
        .ambiguous_variants
        .insert(original.variant_id.clone(), candidates.clone());
    let catalog = SkillPreparationCatalog::new(data.clone(), &identities).unwrap();
    assert_eq!(
        catalog.table_gem_for_skill(&original.primary_effect_id),
        None
    );
    assert_eq!(
        catalog.ambiguous_table_gem_for_skill(&original.primary_effect_id),
        Some(candidates.as_slice())
    );
    assert!(
        !catalog
            .external_variants(&original.game_id)
            .unwrap()
            .variants
            .contains_key(&original.variant_id)
    );
    data.table_gem_for_skill
        .insert(original.primary_effect_id.clone(), candidates[0].clone());
    assert!(
        data.validate(&identities).is_err(),
        "lexical order cannot fabricate a Lua winner"
    );
}
#[test]
fn required_fields_and_source_fidelity_are_validated() {
    let mut value = serde_json::to_value(&package().skill_preparation).unwrap();
    value["effects"][0].as_object_mut().unwrap().remove("color");
    assert!(serde_json::from_value::<SkillPreparationData>(value).is_err());
    let mut data = package().skill_preparation.clone();
    let source = data.source.files.keys().next().unwrap().clone();
    data.source.files.insert(source, "0".repeat(64));
    assert!(data.validate(&package().skill_identities).is_err());
    let mut data = package().skill_preparation.clone();
    data.effects[0].id = "invented-effect".into();
    assert!(data.validate(&package().skill_identities).is_err());
}
