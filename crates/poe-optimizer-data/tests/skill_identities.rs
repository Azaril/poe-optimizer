use poe_optimizer_data::{game_data, skill_identities::*};
use std::sync::OnceLock;
fn source() -> &'static SkillIdentityData {
    static DATA: OnceLock<SkillIdentityData> = OnceLock::new();
    DATA.get_or_init(|| {
        game_data::bundled_snapshot()
            .unwrap()
            .skill_identities()
            .data()
            .clone()
    })
}
fn add(data: &mut SkillIdentityData, key: &str, game: &str, variant: &str, effect: &str) {
    let mut gem = data.gems[0].clone();
    gem.key = key.into();
    gem.game_id = game.into();
    gem.variant_id = variant.into();
    gem.primary_effect_id = effect.into();
    gem.declared_additional_effects.clear();
    gem.declared_additional_stat_sets.clear();
    gem.constructed_additional_effects.clear();
    gem.additional_effects.clear();
    gem.effect_list = vec![effect.into()];
    gem.display_order = None;
    gem.winning_declaration = data.gem_declarations.len() as u32 + 1;
    let mut declaration = data.gem_declarations[0].clone();
    declaration.index = gem.winning_declaration;
    declaration.key = gem.key.clone();
    declaration.identity = DeclaredGemIdentity {
        game_id: gem.game_id.clone(),
        variant_id: gem.variant_id.clone(),
        name: gem.name.clone(),
        name_spec: gem.name_spec.clone(),
        base_type_name: gem.base_type_name.clone(),
        primary_effect_id: effect.into(),
        additional_effects: vec![],
        additional_stat_sets: vec![],
        display_order: None,
    };
    data.gem_declarations.push(declaration);
    data.gems.push(gem);
    data.missing_references = data.expected_missing_references();
}
#[test]
fn bundled_catalog_retains_complete_declarations_and_constructed_rows() {
    let data = source();
    assert_eq!(data.gem_declarations.len(), 967);
    assert_eq!(data.gems.len(), 966);
    assert_eq!(data.skill_declarations.len(), 1439);
    assert_eq!(data.skills.len(), 1436);
    assert_eq!(data.missing_references.len(), 16);
    assert_eq!(data.capability, SkillIdentityCapability::IdentityOnly);
    assert_eq!(
        game_data::bundled_snapshot()
            .unwrap()
            .identity()
            .schema_version,
        23
    );
    let catalog = SkillIdentityCatalog::new(data.clone()).unwrap();
    for gem in &data.gems {
        let result = catalog.resolve_external(&gem.game_id, Some(&gem.variant_id));
        assert!(result.candidates.iter().any(|g| g.key == gem.key));
        assert_eq!(catalog.gem_by_key(&gem.key), Some(gem));
    }
    for skill in &data.skills {
        assert_eq!(catalog.skill_by_id(&skill.id), Some(skill));
    }
}
#[test]
fn exact_external_identity_is_distinct_from_internal_key_effect_and_label() {
    let catalog = SkillIdentityCatalog::new(source().clone()).unwrap();
    let gem = catalog
        .data()
        .gems
        .iter()
        .find(|g| g.key != g.game_id)
        .unwrap();
    let result = catalog.resolve_external(&gem.game_id, Some(&gem.variant_id));
    assert_eq!(result.status, GemIdentityResolutionStatus::Exact);
    assert_eq!(result.candidates[0].key, gem.key);
    assert!(catalog.skill_by_id(&gem.primary_effect_id).is_some());
    assert_eq!(
        catalog.resolve_external("", Some(&gem.variant_id)).status,
        GemIdentityResolutionStatus::Missing
    );
    assert_eq!(
        catalog.resolve_external(&gem.name, None).status,
        GemIdentityResolutionStatus::Missing
    );
}
#[test]
fn missing_and_invalid_single_variant_fallbacks_are_explicit() {
    let mut data = source().clone();
    let effect = data.skills[0].id.clone();
    add(
        &mut data,
        "custom-key",
        "custom-game",
        "custom-variant",
        &effect,
    );
    let catalog = SkillIdentityCatalog::new(data).unwrap();
    for (variant, reason) in [
        (
            None,
            GemIdentityResolutionReason::MissingVariantSingleCandidate,
        ),
        (
            Some("invalid"),
            GemIdentityResolutionReason::UnknownVariantSingleCandidate,
        ),
        (
            Some(""),
            GemIdentityResolutionReason::UnknownVariantSingleCandidate,
        ),
    ] {
        let result = catalog.resolve_external("custom-game", variant);
        assert_eq!(result.status, GemIdentityResolutionStatus::SingleFallback);
        assert_eq!(result.reason, reason);
        assert_eq!(result.candidates[0].key, "custom-key");
    }
}
#[test]
fn ambiguous_variants_and_colliding_exact_pairs_never_choose_a_hidden_winner() {
    let mut data = source().clone();
    let effect = data.skills[0].id.clone();
    add(&mut data, "custom-one", "custom-game", "one", &effect);
    add(&mut data, "custom-two", "custom-game", "two", &effect);
    let catalog = SkillIdentityCatalog::new(data.clone()).unwrap();
    assert_eq!(
        catalog.resolve_external("custom-game", Some("one")).status,
        GemIdentityResolutionStatus::Exact
    );
    for (variant, reason) in [
        (
            None,
            GemIdentityResolutionReason::MissingVariantMultipleCandidates,
        ),
        (
            Some("bad"),
            GemIdentityResolutionReason::UnknownVariantMultipleCandidates,
        ),
    ] {
        let result = catalog.resolve_external("custom-game", variant);
        assert_eq!(result.status, GemIdentityResolutionStatus::Ambiguous);
        assert_eq!(result.reason, reason);
        assert_eq!(result.candidates.len(), 2);
    }
    add(&mut data, "custom-three", "custom-game", "one", &effect);
    let catalog = SkillIdentityCatalog::new(data).unwrap();
    let result = catalog.resolve_external("custom-game", Some("one"));
    assert_eq!(result.status, GemIdentityResolutionStatus::Ambiguous);
    assert_eq!(
        result.reason,
        GemIdentityResolutionReason::CollidingExternalVariant
    );
    assert_eq!(result.candidates.len(), 2);
    let owners: Vec<_> = catalog
        .gems_for_effect(&effect)
        .map(|g| g.key.as_str())
        .collect();
    for key in ["custom-one", "custom-two", "custom-three"] {
        assert!(owners.contains(&key));
    }
}
#[test]
fn cloned_catalog_shares_immutable_owned_data_and_indices() {
    let catalog = SkillIdentityCatalog::new(source().clone()).unwrap();
    let cloned = catalog.clone();
    assert!(std::ptr::eq(catalog.data(), cloned.data()));
    let key = &source().gems[0].key;
    assert!(std::ptr::eq(
        catalog.gem_by_key(key).unwrap(),
        cloned.gem_by_key(key).unwrap()
    ));
}
#[test]
fn duplicate_declarations_are_not_erased_by_unique_constructed_maps() {
    let data = source();
    let gem = data
        .gems
        .iter()
        .find(|g| {
            data.gem_declarations
                .iter()
                .filter(|d| d.key == g.key)
                .count()
                > 1
        })
        .unwrap();
    let declarations: Vec<_> = data
        .gem_declarations
        .iter()
        .filter(|d| d.key == gem.key)
        .collect();
    assert_eq!(gem.winning_declaration, declarations.last().unwrap().index);
    assert_ne!(declarations[0].source.sha256, declarations[1].source.sha256);
    for skill in &data.skills {
        let declarations: Vec<_> = data
            .skill_declarations
            .iter()
            .filter(|d| d.id == skill.id)
            .collect();
        assert_eq!(
            skill.winning_declaration,
            declarations.last().unwrap().index
        );
    }
}
#[test]
fn stat_sets_missing_effects_generated_effects_and_display_order_remain_distinct() {
    let data = source();
    assert!(
        data.gems
            .iter()
            .any(|g| !g.declared_additional_stat_sets.is_empty())
    );
    assert!(
        data.gems
            .iter()
            .any(|g| g.constructed_additional_effects != g.declared_additional_effects)
    );
    assert!(data.gems.iter().any(|g| g.display_order.is_some()));
    let known: std::collections::BTreeSet<_> = data.skills.iter().map(|s| s.id.as_str()).collect();
    for row in &data.missing_references {
        assert!(!known.contains(row.effect_id.as_str()));
        assert!(data.gems.iter().any(|g| g.key == row.gem_key));
    }
    let mut changed = data.clone();
    changed.missing_references.pop();
    assert!(changed.validate().is_err());
}
#[test]
fn unknown_fields_missing_required_options_and_invalid_types_reject() {
    let value = serde_json::to_value(source()).unwrap();
    let mut unknown = value.clone();
    unknown["gems"][0]["native_supported"] = true.into();
    assert!(serde_json::from_value::<SkillIdentityData>(unknown).is_err());
    let mut missing = value.clone();
    missing["gems"][0]
        .as_object_mut()
        .unwrap()
        .remove("name_spec");
    assert!(serde_json::from_value::<SkillIdentityData>(missing).is_err());
    let mut wrong = value;
    wrong["gems"][0]["game_id"] = false.into();
    assert!(serde_json::from_value::<SkillIdentityData>(wrong).is_err());
}
#[test]
fn invalid_winners_partitions_source_spans_and_reference_lists_reject() {
    let mut data = source().clone();
    data.gems[0].winning_declaration = 0;
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.gems.pop();
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.skill_declarations[0].source.line = 0;
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.source
        .skill_module_order
        .push("unlisted-source".into());
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.gems[0].effect_list.push("absent-effect".into());
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.gems[0].constructed_additional_effects = vec![IndexedIdentityReference {
        index: 0,
        id: "known".into(),
    }];
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.gems[0].name = "x".repeat(4097);
    assert!(data.validate().is_err());
}

#[test]
fn portable_winner_is_explicit_not_an_invented_duplicate_order_rule() {
    let mut data = source().clone();
    let gem = data.gems[0].clone();
    let original = data.gem_declarations[gem.winning_declaration as usize - 1].clone();
    let mut duplicate = original.clone();
    duplicate.index = data.gem_declarations.len() as u32 + 1;
    data.gem_declarations.push(duplicate);
    let catalog = SkillIdentityCatalog::new(data).unwrap();
    assert_eq!(
        catalog.gem_by_key(&gem.key).unwrap().winning_declaration,
        original.index
    );
}
#[test]
fn declared_references_cannot_disappear_and_source_paths_remain_relative() {
    let mut data = source().clone();
    let gem = data
        .gems
        .iter_mut()
        .find(|g| !g.declared_additional_effects.is_empty())
        .unwrap();
    gem.constructed_additional_effects.clear();
    assert!(data.validate().is_err());
    let mut data = source().clone();
    data.source
        .files
        .insert("C:/absolute.lua".into(), "a".repeat(64));
    assert!(data.validate().is_err());
}
