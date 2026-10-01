//! A new release corrects data without changing monotonic migration semantics.
#[path = "support/owned_character_reward_policy.rs"]
mod character_reward_inventory;
#[path = "support/owned_configuration_reward_policy.rs"]
mod configuration_rewards;
#[path = "support/owned_enemy_level_policy.rs"]
mod enemy_level;
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey, owned_schema::*,
};
use poe_optimizer_import::{owned_release::*, owned_release_revision::*, owned_successor::*};
use std::path::PathBuf;

fn prior() -> StagedOwnedRelease {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let initial = transition_owned_bundle(fixture::input(&root), Default::default()).unwrap();
    let next = fixture::next(&initial);
    let bundle = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        fixture::tree(&next),
        Default::default(),
    )
    .unwrap();
    assemble_owned_release(
        OwnedReleaseInput {
            schema_version: 1,
            recipe: bundle.recipe().clone(),
            mapping: bundle.mapping().input().clone(),
            roles: bundle.roles().input().clone(),
            normalization: bundle.normalization().clone(),
            rewards: bundle.rewards().input().clone(),
            items: bundle.items().input().clone(),
            item_source: bundle.item_source().input().clone(),
            tree: bundle.tree().map(|tree| tree.input().clone()),
            evaluation: None,
            query_sets: bundle.query_sets().to_vec(),
            provenance: vec![],
        },
        Default::default(),
    )
    .unwrap()
}
fn correction(prior: &StagedOwnedRelease) -> OwnedReleaseRevisionInput {
    let mut gem = prior.input().recipe.schema.definitions.iter().find(|row| {
        matches!(row, DefinitionDescriptor::Gem(entry) if matches!(&entry.schema, SchemaState::Known(schema)
            if schema.declarations.parameters.is_complete()))
    }).unwrap().clone();
    let subject = SchemaSubject::Definition(gem.address());
    let DefinitionDescriptor::Gem(entry) = &mut gem else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let gaps = vec![SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: OwnedDefinitionKey::new("intrinsic-input-review-incomplete").unwrap(),
    }];
    schema.declarations.parameters.closure = SchemaClosure::Partial { gaps: gaps.clone() };
    schema.declarations.choices.closure = SchemaClosure::Partial { gaps };
    OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: OwnedDefinitionKey::new("explicit-corrected-release").unwrap(),
        reason: OwnedDefinitionKey::new("correct-premature-input-closure").unwrap(),
        definitions: vec![gem],
        slots: vec![],
    }
}

#[test]
fn checked_revision_retains_reward_authority_and_standalone_rejects_stale_commitments() {
    use poe_optimizer_import::{
        owned_normalize::ConfigurationRewardInventoryPolicy, owned_reward_policy::RewardTemplate,
        owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let original = prior();
    let mut input = original.input().clone();
    input.normalization.configuration_reward_inventory = Some(configuration_rewards::policy(
        original.mapping(),
        original.rewards(),
    ));
    input.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            input.tree.take().unwrap().content,
            original.assembled().registry(),
            original.assembled().schema(),
            original.mapping(),
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    let checked = assemble_owned_release(input, Default::default()).unwrap();
    let before = serde_json::to_vec(checked.input()).unwrap();
    let revision = correction(&checked);
    let revised =
        compile_owned_release_revision(&checked, revision.clone(), Default::default()).unwrap();
    assert_ne!(revised.rewards().identity(), checked.rewards().identity());
    configuration_rewards::assert_rebound(
        checked
            .normalization()
            .configuration_reward_inventory
            .as_ref()
            .unwrap(),
        revised
            .normalization()
            .configuration_reward_inventory
            .as_ref()
            .unwrap(),
        revised.rewards(),
    );
    assert_eq!(
        revised.rewards().input().rules,
        checked.rewards().input().rules
    );
    assert_eq!(
        revised.mapping().source_identity(),
        checked.mapping().source_identity()
    );
    assert_eq!(revised.query_sets(), checked.query_sets());
    assert_eq!(
        revised.tree().unwrap().input().content,
        checked.tree().unwrap().input().content
    );
    assert_eq!(serde_json::to_vec(checked.input()).unwrap(), before);
    assert!(
        compile_owned_release_revision(&checked, revision, Default::default())
            .unwrap()
            .artifacts()
            .eq(revised.artifacts())
    );

    for case in 0..5 {
        let mut stale = revised.input().clone();
        match case {
            0 => {
                stale.normalization.configuration_reward_inventory = checked
                    .normalization()
                    .configuration_reward_inventory
                    .clone()
            }
            1 => {
                let ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
                    mapping_source,
                    ..
                } = stale
                    .normalization
                    .configuration_reward_inventory
                    .as_mut()
                    .unwrap();
                *mapping_source = digest_owned("wrong-reward-source", &0, 100).unwrap();
            }
            2 => {
                stale.rewards.version = OwnedDefinitionKey::new("different-reward-policy").unwrap()
            }
            3 => {
                stale.rewards.rules[0]
                    .outcomes
                    .iter_mut()
                    .find(|case| matches!(case.outcome, RewardTemplate::Reward { .. }))
                    .unwrap()
                    .outcome = RewardTemplate::None;
            }
            4 => {
                let ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
                    controls,
                    ..
                } = stale
                    .normalization
                    .configuration_reward_inventory
                    .as_mut()
                    .unwrap();
                controls.pop();
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                assemble_owned_release(stale, Default::default()),
                Err(OwnedReleaseError::Normalization(_))
            ),
            "standalone stale case {case}"
        );
    }
}

#[test]
fn enemy_level_source_policy_survives_checked_revision_and_stale_release_is_rejected() {
    use poe_optimizer_import::{
        owned_normalize::EnemyLevelPolicy, owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let original = prior();
    let mut input = original.input().clone();
    input.normalization.enemy_level = Some(enemy_level::policy(original.mapping()));
    input.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            input.tree.take().unwrap().content,
            original.assembled().registry(),
            original.assembled().schema(),
            original.mapping(),
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    let checked = assemble_owned_release(input, Default::default()).unwrap();
    let before = serde_json::to_vec(checked.input()).unwrap();
    let policy_bytes = serde_json::to_vec(&checked.normalization().enemy_level).unwrap();
    let revised =
        compile_owned_release_revision(&checked, correction(&checked), Default::default()).unwrap();
    assert_ne!(revised.receipt().definitions, checked.receipt().definitions);
    assert_eq!(
        serde_json::to_vec(&revised.normalization().enemy_level).unwrap(),
        policy_bytes
    );
    assert_eq!(
        revised.mapping().source_identity(),
        checked.mapping().source_identity()
    );
    assert_eq!(revised.query_sets(), checked.query_sets());
    assert_eq!(
        revised.tree().unwrap().input().content,
        checked.tree().unwrap().input().content
    );
    assert_eq!(serde_json::to_vec(checked.input()).unwrap(), before);
    let mut stale = revised.input().clone();
    let EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 { mapping_source, .. } =
        stale.normalization.enemy_level.as_mut().unwrap();
    *mapping_source = digest_owned("stale-enemy-level-source", &0, 100).unwrap();
    assert!(matches!(
        assemble_owned_release(stale, Default::default()),
        Err(OwnedReleaseError::Normalization(_))
    ));
}

#[test]
fn schema_correction_preserves_equipment_facts_and_rebinds_their_identity() {
    use poe_optimizer_import::{
        owned_normalize::{EquipmentAugmentBase, EquipmentMembershipPolicy},
        owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let original = prior();
    let mut input = original.input().clone();
    let template = input
        .recipe
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::ItemTemplate(entry)
                if matches!(&entry.schema, SchemaState::Known(_)) =>
            {
                Some(entry.id.clone())
            }
            _ => None,
        })
        .expect("fixture supplies a known item template");
    input.normalization.equipment_membership =
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
            definitions: original.receipt().definitions.clone(),
            templates: vec![EquipmentAugmentBase {
                template,
                base_name: "Source inventory fixture".into(),
                weapon: false,
                armour: true,
                wand: false,
                staff: false,
                sceptre: false,
            }],
            source_base_names: vec!["Source inventory fixture".into()],
            loader_jewel_fallback_titles: vec!["Loader inventory fixture".into()],
        });
    input.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            input.tree.take().unwrap().content,
            original.assembled().registry(),
            original.assembled().schema(),
            original.mapping(),
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    let prior = assemble_owned_release(input, Default::default()).unwrap();
    let prior_bytes = serde_json::to_vec(prior.input()).unwrap();
    let revised =
        compile_owned_release_revision(&prior, correction(&prior), Default::default()).unwrap();
    let mut expected = prior.normalization().equipment_membership.clone().unwrap();
    let EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. } = &mut expected else {
        panic!("expected legacy policy")
    };
    *definitions = revised.receipt().definitions.clone();
    assert_ne!(
        Some(&expected),
        prior.normalization().equipment_membership.as_ref()
    );
    assert_eq!(
        revised.normalization().equipment_membership.as_ref(),
        Some(&expected)
    );
    assert_eq!(revised.query_sets(), prior.query_sets());
    assert_eq!(
        revised.tree().unwrap().input().content,
        prior.tree().unwrap().input().content
    );
    assert_eq!(serde_json::to_vec(prior.input()).unwrap(), prior_bytes);
}

#[test]
fn imported_equipment_revision_rebinds_all_dependencies_without_member_or_input_policies() {
    use poe_optimizer_import::{
        owned_normalize::EquipmentMembershipPolicy, owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let original = prior();
    let mut input = original.input().clone();
    input.normalization.item_modifier_membership = None;
    input.normalization.item_parameter_inputs = None;
    input.normalization.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            definitions: original.receipt().definitions.clone(),
            templates: vec![],
            source_base_names: vec!["Reviewed empty imported domain".into()],
            loader_jewel_fallback_titles: vec!["Reviewed loader title".into()],
            item_lines: *original.items().identity(),
            item_source: *original.item_source().identity(),
            imported_profiles: vec![],
        },
    );
    input.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            input.tree.take().unwrap().content,
            original.assembled().registry(),
            original.assembled().schema(),
            original.mapping(),
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    let checked = assemble_owned_release(input, Default::default()).unwrap();
    let before = serde_json::to_vec(checked.input()).unwrap();
    let revised =
        compile_owned_release_revision(&checked, correction(&checked), Default::default()).unwrap();
    assert!(revised.normalization().item_modifier_membership.is_none());
    assert!(revised.normalization().item_parameter_inputs.is_none());
    let mut expected = checked
        .normalization()
        .equipment_membership
        .clone()
        .unwrap();
    let EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        definitions,
        item_lines,
        item_source,
        ..
    } = &mut expected
    else {
        panic!()
    };
    assert_ne!(&*definitions, &revised.receipt().definitions);
    assert_ne!(*item_lines, *revised.items().identity());
    assert_ne!(*item_source, *revised.item_source().identity());
    *definitions = revised.receipt().definitions.clone();
    *item_lines = *revised.items().identity();
    *item_source = *revised.item_source().identity();
    assert_eq!(
        revised.normalization().equipment_membership.as_ref(),
        Some(&expected)
    );
    assert_eq!(revised.query_sets(), checked.query_sets());
    assert_eq!(
        revised.tree().unwrap().input().content,
        checked.tree().unwrap().input().content
    );
    assert_eq!(serde_json::to_vec(checked.input()).unwrap(), before);
    for field in 0..2 {
        let mut stale = revised.input().clone();
        let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            item_lines,
            item_source,
            ..
        }) = &mut stale.normalization.equipment_membership
        else {
            panic!()
        };
        if field == 0 {
            *item_lines = *checked.items().identity();
        } else {
            *item_source = *checked.item_source().identity();
        }
        assert!(
            matches!(
                assemble_owned_release(stale, Default::default()),
                Err(OwnedReleaseError::Normalization(_))
            ),
            "stale imported dependency {field}"
        );
    }
}

fn prior_with_inert_gem_inventory() -> StagedOwnedRelease {
    use poe_optimizer_import::{
        owned_normalize::{GemInventoryPolicy, gem_inventory_scalar_inputs_identity},
        owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let original = prior();
    let mut input = original.input().clone();
    input.normalization.gem_inventory = Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
        definitions: original.receipt().definitions.clone(),
        roles: *original.roles().identity(),
        catalog: original.roles().input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(
            &input.normalization,
            Default::default(),
        )
        .unwrap(),
        // An empty reviewed domain grants no source inventory authority. It
        // isolates publication binding behavior from the runtime proof tests.
        gems: vec![],
    });
    input.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            input.tree.take().unwrap().content,
            original.assembled().registry(),
            original.assembled().schema(),
            original.mapping(),
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    assemble_owned_release(input, Default::default()).unwrap()
}

#[test]
fn gem_inventory_publication_rejects_every_stale_commitment_even_for_empty_domain() {
    use poe_optimizer_import::{
        owned_mapping::SourceComponent,
        owned_normalize::{GemInventoryPolicy, NormalizationError},
    };
    let prior = prior_with_inert_gem_inventory();
    let before = serde_json::to_vec(prior.input()).unwrap();
    let wrong = digest_owned("stale-gem-inventory-test", &1, 100).unwrap();
    for kind in 0..5 {
        let mut input = prior.input().clone();
        let GemInventoryPolicy::PobFreshSingleSupportV1 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            ..
        } = input.normalization.gem_inventory.as_mut().unwrap();
        match kind {
            0 => definitions.release = "stale-gem-inventory-release".into(),
            1 => *roles = wrong,
            2 => *catalog = wrong,
            3 => *scalar_inputs = wrong,
            4 => input
                .normalization
                .manual_skill_sources
                .push(SourceComponent::Text("unreviewed-source".into())),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                assemble_owned_release(input, Default::default()),
                Err(OwnedReleaseError::Normalization(
                    NormalizationError::Binding
                ))
            ),
            "stale inventory commitment case {kind}"
        );
    }
    assert_eq!(serde_json::to_vec(prior.input()).unwrap(), before);
}

#[test]
fn checked_schema_revision_rebinds_gem_inventory_without_changing_reviewed_content() {
    use poe_optimizer_import::owned_normalize::{
        GemInventoryPolicy, NormalizationError, gem_inventory_scalar_inputs_identity,
    };
    let prior = prior_with_inert_gem_inventory();
    let before = serde_json::to_vec(prior.input()).unwrap();
    let revision = correction(&prior);
    let revised =
        compile_owned_release_revision(&prior, revision.clone(), Default::default()).unwrap();
    let mut expected = prior.normalization().gem_inventory.clone().unwrap();
    let GemInventoryPolicy::PobFreshSingleSupportV1 {
        definitions,
        roles,
        scalar_inputs,
        ..
    } = &mut expected;
    *definitions = revised.receipt().definitions.clone();
    *roles = *revised.roles().identity();
    *scalar_inputs =
        gem_inventory_scalar_inputs_identity(revised.normalization(), Default::default()).unwrap();
    assert_ne!(
        prior.normalization().gem_inventory.as_ref(),
        Some(&expected)
    );
    assert_eq!(
        revised.normalization().gem_inventory.as_ref(),
        Some(&expected)
    );
    assert_eq!(revised.query_sets(), prior.query_sets());
    assert_eq!(
        revised.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(
        revised.tree().unwrap().input().content,
        prior.tree().unwrap().input().content
    );
    assert_eq!(serde_json::to_vec(prior.input()).unwrap(), before);
    assert!(
        compile_owned_release_revision(&prior, revision.clone(), Default::default())
            .unwrap()
            .artifacts()
            .eq(revised.artifacts())
    );

    // Staged releases cannot be mutated or fabricated by callers. An unbound
    // previous policy must fail ordinary assembly before a revision can receive
    // it, and a revision for the old endpoint cannot target the new release.
    let mut stale = revised.input().clone();
    stale.normalization.gem_inventory = prior.normalization().gem_inventory.clone();
    assert!(matches!(
        assemble_owned_release(stale, Default::default()),
        Err(OwnedReleaseError::Normalization(
            NormalizationError::Binding
        ))
    ));
    assert!(matches!(
        compile_owned_release_revision(&revised, revision, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "release revision version or endpoint"
        ))
    ));
}

#[test]
fn explicit_release_correction_preserves_ids_rules_policies_queries_and_prior_bytes() {
    let prior = prior();
    let prior_bytes = serde_json::to_vec(prior.input()).unwrap();
    let revision = correction(&prior);
    let target = revision.definitions[0].address();
    let correction_hash =
        digest_owned("owned-release-schema-revision-v1", &revision, 1024 * 1024).unwrap();
    let revised =
        compile_owned_release_revision(&prior, revision.clone(), Default::default()).unwrap();
    assert_ne!(revised.receipt().input, prior.receipt().input);
    assert_ne!(revised.receipt().definitions, prior.receipt().definitions);
    assert_eq!(
        revised.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(revised.query_sets(), prior.query_sets());
    assert_eq!(revised.receipt().query_rows, 110);
    assert_eq!(revised.receipt().source, prior.receipt().source);
    let provenance = revised.input().provenance.last().unwrap();
    assert_eq!(provenance.prior_input, prior.receipt().input);
    assert_eq!(provenance.authoring_input, correction_hash);
    assert_eq!(provenance.kind, revision.reason);
    assert_eq!(serde_json::to_vec(prior.input()).unwrap(), prior_bytes);

    let mut restored = revised.input().clone();
    restored.recipe.schema.release = prior.input().recipe.schema.release.clone();
    let original = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == target)
        .unwrap();
    *restored
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|row| row.address() == target)
        .unwrap() = original.clone();
    restored.recipe.rules.definitions = prior.receipt().definitions.clone();
    restored.recipe.routing.definitions = prior.receipt().definitions.clone();
    restored.mapping.definitions = prior.receipt().definitions.clone();
    restored.roles.definitions = prior.receipt().definitions.clone();
    restored.roles.mapping = prior.receipt().mapping;
    if let poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(quality) =
        &mut restored.normalization.gem_quality
    {
        quality.definitions = prior.receipt().definitions.clone();
    }
    if let Some(gems) = &mut restored.normalization.gem_inputs {
        gems.definitions = prior.receipt().definitions.clone();
    }
    restored.rewards.definitions = prior.receipt().definitions.clone();
    restored.rewards.mapping = prior.receipt().mapping;
    restored.items.definitions = prior.receipt().definitions.clone();
    restored.item_source.item_lines = prior.receipt().items;
    // Every tree content field must stay equal; only its verified binding fields differ.
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    restored.tree = prior.input().tree.clone();
    restored.provenance = prior.input().provenance.clone();
    assert!(
        restored == *prior.input(),
        "undeclared semantic change in release correction"
    );
    let repeated = compile_owned_release_revision(&prior, revision, Default::default()).unwrap();
    assert!(
        repeated.artifacts().eq(revised.artifacts()),
        "release build must be reproducible"
    );
    assert!(
        compile_owned_release_revision(&revised, correction(&prior), Default::default()).is_err()
    );
}

#[test]
fn revision_rejects_stale_unsupported_duplicate_unchanged_and_noncanonical_targets() {
    let prior = prior();
    let valid = correction(&prior);
    let mut inputs = vec![];
    let mut input = valid.clone();
    input.schema_version = 2;
    inputs.push(input);
    let mut input = valid.clone();
    input.before = digest_owned("wrong-endpoint", &1, 100).unwrap();
    inputs.push(input);
    let mut input = valid.clone();
    input.release = prior.input().recipe.schema.release.clone();
    inputs.push(input);
    let mut input = valid.clone();
    input.definitions.clear();
    inputs.push(input);
    let mut input = valid.clone();
    input.definitions.push(input.definitions[0].clone());
    inputs.push(input);
    let mut input = valid.clone();
    let target = input.definitions[0].address();
    input.definitions[0] = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == target)
        .unwrap()
        .clone();
    inputs.push(input);
    for (index, input) in inputs.into_iter().enumerate() {
        assert!(
            compile_owned_release_revision(&prior, input, Default::default()).is_err(),
            "invalid case {index}"
        );
    }
    let mut wire = serde_json::to_value(valid).unwrap();
    wire["skip_preservation"] = true.into();
    assert!(serde_json::from_value::<OwnedReleaseRevisionInput>(wire).is_err());
}

#[test]
fn revision_compiles_changed_schema_and_bounds_work_before_expansion() {
    let prior = prior();
    let mut invalid = correction(&prior);
    let DefinitionDescriptor::Gem(entry) = &mut invalid.definitions[0] else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema.level.minimum = schema.level.maximum;
    schema.level.maximum = poe_optimizer_core::owned_definitions::BoundedInteger::new(0).unwrap();
    assert!(compile_owned_release_revision(&prior, invalid, Default::default()).is_err());
    let valid = correction(&prior);
    for limits in [
        OwnedReleaseLimits {
            max_artifact_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_input_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 0,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_release_revision(&prior, valid.clone(), limits).is_err());
    }
}

#[test]
fn existing_slot_revision_rebinds_without_allocating_or_changing_its_owner() {
    use poe_optimizer_core::owned_definitions::FiniteQuantity;
    let prior = prior();
    let mut row = prior
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| {
            matches!(row, SlotDescriptor::Parameter(entry) if matches!(&entry.schema,
            SchemaState::Known(ParameterSlotSchema { value: ValueSchema::Quantity(range), .. })
            if range.maximum.value().abs() < 1e12))
        })
        .expect("fixture has bounded numeric input slots")
        .clone();
    let target = row.address();
    let SlotDescriptor::Parameter(entry) = &mut row else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let ValueSchema::Quantity(range) = &mut schema.value else {
        unreachable!()
    };
    range.maximum =
        FiniteQuantity::new(range.maximum.value() + 1.0, range.maximum.unit().clone()).unwrap();
    let mut policy = correction(&prior);
    policy.definitions.clear();
    policy.slots = vec![row.clone()];
    let result =
        compile_owned_release_revision(&prior, policy.clone(), Default::default()).unwrap();
    assert_eq!(
        result.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(
        result
            .input()
            .recipe
            .schema
            .slots
            .iter()
            .find(|slot| slot.address() == target),
        Some(&row)
    );
    assert_eq!(
        result.input().recipe.schema.definitions,
        prior.input().recipe.schema.definitions
    );
    assert_eq!(result.query_sets(), prior.query_sets());
    policy.slots.push(row);
    assert!(compile_owned_release_revision(&prior, policy, Default::default()).is_err());
    let mut with_provenance = prior.input().clone();
    with_provenance.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("reviewed-parent").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned("test-prior", &1, 100).unwrap(),
    });
    let prior = assemble_owned_release(with_provenance, Default::default()).unwrap();
    assert!(
        compile_owned_release_revision(
            &prior,
            correction(&prior),
            OwnedReleaseLimits {
                max_provenance_entries: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn character_reward_inventory_source_policy_survives_checked_revision_and_stale_release_is_rejected()
 {
    use poe_optimizer_import::{
        owned_normalize::CharacterRewardInventoryPolicy,
        owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let original = prior();
    let mut input = original.input().clone();
    input.normalization.character_reward_inventory =
        Some(character_reward_inventory::policy(original.mapping()));
    input.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            input.tree.take().unwrap().content,
            original.assembled().registry(),
            original.assembled().schema(),
            original.mapping(),
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    let checked = assemble_owned_release(input, Default::default()).unwrap();
    let before = serde_json::to_vec(checked.input()).unwrap();
    let policy_bytes =
        serde_json::to_vec(&checked.normalization().character_reward_inventory).unwrap();
    let revised =
        compile_owned_release_revision(&checked, correction(&checked), Default::default()).unwrap();
    assert_ne!(revised.receipt().definitions, checked.receipt().definitions);
    assert_eq!(
        serde_json::to_vec(&revised.normalization().character_reward_inventory).unwrap(),
        policy_bytes
    );
    assert_eq!(
        revised.mapping().source_identity(),
        checked.mapping().source_identity()
    );
    assert_eq!(revised.query_sets(), checked.query_sets());
    assert_eq!(
        revised.tree().unwrap().input().content,
        checked.tree().unwrap().input().content
    );
    assert_eq!(serde_json::to_vec(checked.input()).unwrap(), before);
    let mut stale = revised.input().clone();
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { mapping_source, .. } = stale
        .normalization
        .character_reward_inventory
        .as_mut()
        .unwrap();
    *mapping_source = digest_owned("stale-character-reward-source", &0, 100).unwrap();
    assert!(matches!(
        assemble_owned_release(stale, Default::default()),
        Err(OwnedReleaseError::Normalization(_))
    ));
}
