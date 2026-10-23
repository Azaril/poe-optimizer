//! Content construction uses checked dependencies; sealed inputs remain strict.
use super::tracked_release;
use poe_optimizer_authoring::{
    NamedIdRegistry, StatDeclaration, StatDeclarations, StatValueDeclaration,
    stage_stat_declarations,
};
use poe_optimizer_core::owned_schema::RuleEntityKind;
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_item_lines::{ItemLineLimits, ItemLinePolicyContent, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutContent, ItemSourceLayoutPolicy, ItemSourceLimits},
    owned_mapping::{MappingContent, OwnedIdRegistry, OwnedIdRegistrySnapshot, OwnedMappingIndex},
    owned_reward_policy::{OwnedRewardPolicy, RewardPolicyContent, RewardPolicyLimits},
    owned_skill_roles::{OwnedSkillRoleIndex, SkillRoleContent},
};

fn assert_content_binding(
    registry: &OwnedIdRegistrySnapshot,
    schema: &OwnedDefinitionSchemaPackage,
    changed: bool,
) {
    let release = tracked_release().release();
    let mapping = OwnedMappingIndex::bind_new(
        release.mapping().clone().into_authored_content(),
        registry,
        schema,
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::bind_new(
        release.roles().clone().into_authored_content(),
        &mapping,
        schema,
        Default::default(),
    )
    .unwrap();
    let rewards = OwnedRewardPolicy::bind_new(
        release.rewards().clone().into_authored_content(),
        &mapping,
        schema,
        Default::default(),
    )
    .unwrap();
    let items = OwnedItemLinePolicy::bind_new(
        release.items().clone().into_authored_content().unwrap(),
        schema,
        Default::default(),
    )
    .unwrap();
    let source = ItemSourceLayoutPolicy::bind_new(
        release
            .item_source()
            .clone()
            .into_authored_content()
            .unwrap(),
        &items,
        schema,
        Default::default(),
    )
    .unwrap();

    for (before, after) in [
        (release.mapping().identity(), mapping.identity()),
        (release.roles().identity(), roles.identity()),
        (release.rewards().identity(), rewards.identity()),
        (release.items().identity(), items.identity()),
        (release.item_source().identity(), source.identity()),
    ] {
        assert_eq!(before != after, changed);
    }
    if !changed {
        assert_eq!(mapping.input(), release.mapping().input());
        assert_eq!(roles.input(), release.roles().input());
        assert_eq!(rewards.input(), release.rewards().input());
        assert_eq!(items.input(), release.items().input());
        assert_eq!(source.input(), release.item_source().input());
    }
    assert_eq!(mapping.input().registry, registry.identity().unwrap());
    assert_eq!(&mapping.input().definitions, schema.identity());
    assert_eq!(roles.input().mapping, *mapping.identity());
    assert_eq!(&roles.input().definitions, schema.identity());
    assert_eq!(rewards.input().mapping, *mapping.identity());
    assert_eq!(&rewards.input().definitions, schema.identity());
    assert_eq!(&items.input().definitions, schema.identity());
    assert_eq!(source.input().item_lines, *items.identity());
    assert_eq!(
        mapping.into_authored_content(),
        release.mapping().clone().into_authored_content()
    );
    assert_eq!(
        roles.into_authored_content(),
        release.roles().clone().into_authored_content()
    );
    assert_eq!(
        rewards.into_authored_content(),
        release.rewards().clone().into_authored_content()
    );
    assert_eq!(
        items.into_authored_content().unwrap(),
        release.items().clone().into_authored_content().unwrap()
    );
    assert_eq!(
        source.into_authored_content().unwrap(),
        release
            .item_source()
            .clone()
            .into_authored_content()
            .unwrap()
    );
}

#[test]
fn current_import_content_preserves_identity_and_rebinds_to_declared_stat_dependencies() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    assert_content_binding(recipe.registry(), recipe.schema().as_ref(), false);

    // This temporary ledger is a test of immutable staging, not the production
    // persistence workflow. No identifier is predicted or placed in source data.
    let registry =
        OwnedIdRegistry::new(recipe.registry().input().clone(), Default::default()).unwrap();
    let names = NamedIdRegistry::empty(registry, Default::default()).unwrap();
    let symbols = names
        .symbols(recipe.schema().as_ref(), Default::default())
        .unwrap();
    let declarations = StatDeclarations {
        namespace: recipe.schema().input().namespace.clone(),
        stats: vec![StatDeclaration {
            name: "authoring_binding_probe".into(),
            value: StatValueDeclaration::Boolean,
            targets: vec![RuleEntityKind::Skill],
        }],
    };
    let staged = stage_stat_declarations(
        &names,
        names.identity().unwrap(),
        recipe.schema().as_ref(),
        &symbols,
        &declarations,
        Default::default(),
    )
    .unwrap();
    assert_content_binding(staged.names().registry(), staged.schema(), true);
    // Reconstructing individual components does not produce a complete release
    // or close any existing coverage. Normalization/tree/evaluation still need
    // their own construction and final release validation.
    assert_content_binding(recipe.registry(), recipe.schema().as_ref(), false);
}

#[test]
fn authored_content_never_accepts_a_sealed_runtime_artifact() {
    let release = tracked_release().release();
    assert!(
        serde_json::from_value::<MappingContent>(
            serde_json::to_value(release.mapping().input()).unwrap()
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<SkillRoleContent>(
            serde_json::to_value(release.roles().input()).unwrap()
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<RewardPolicyContent>(
            serde_json::to_value(release.rewards().input()).unwrap()
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ItemLinePolicyContent>(
            serde_json::to_value(release.items().input()).unwrap()
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ItemSourceLayoutContent>(
            serde_json::to_value(release.item_source().input()).unwrap()
        )
        .is_err()
    );
}

#[test]
fn authored_reward_and_item_content_retains_semantic_and_resource_rejections() {
    let release = tracked_release().release();
    let schema = release.assembled().schema();
    let rewards = release.rewards().clone().into_authored_content();
    assert!(
        OwnedRewardPolicy::bind_new(
            rewards.clone(),
            release.mapping(),
            schema,
            RewardPolicyLimits {
                max_wire_bytes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut bad_rewards = rewards;
    bad_rewards.rules.push(bad_rewards.rules[0].clone());
    assert!(
        OwnedRewardPolicy::bind_new(bad_rewards, release.mapping(), schema, Default::default())
            .is_err()
    );

    let items = release.items().clone().into_authored_content().unwrap();
    assert!(
        OwnedItemLinePolicy::bind_new(
            items.clone(),
            schema,
            ItemLineLimits {
                max_wire_bytes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut bad_items = items;
    bad_items.rules.push(bad_items.rules[0].clone());
    assert!(OwnedItemLinePolicy::bind_new(bad_items, schema, Default::default()).is_err());

    let source = release
        .item_source()
        .clone()
        .into_authored_content()
        .unwrap();
    assert!(
        ItemSourceLayoutPolicy::bind_new(
            source.clone(),
            release.items(),
            schema,
            ItemSourceLimits {
                max_wire_bytes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut bad_source = source;
    bad_source.source.files[0].sha256 = "g".repeat(64);
    assert!(
        ItemSourceLayoutPolicy::bind_new(bad_source, release.items(), schema, Default::default())
            .is_err()
    );
}
