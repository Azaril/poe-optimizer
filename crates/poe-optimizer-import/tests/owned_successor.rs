//! Exact offline succession using shipped inputs; no source checkout or VM.
#[path = "support/owned_character_reward_policy.rs"]
mod character_reward_inventory;
#[path = "support/owned_configuration_reward_policy.rs"]
mod configuration_rewards;
#[path = "support/owned_empty_character_rune_policy.rs"]
mod empty_character_rune_policy;
#[path = "support/owned_enemy_level_policy.rs"]
mod enemy_level;
#[path = "support/owned_passive_socket_policy.rs"]
mod passive_socket_policy;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{FiniteQuantity, OwnedDefinitionKey},
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{
    owned_normalize::GemQualityPolicy, owned_skill_catalog::OwnedGemMaterialization,
    owned_successor::*,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/owned/poe2/3887ae68")
}
fn load<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn input() -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: load("import/compiled/recipe.json"),
        successor: load("resistance/recipe.json"),
        mapping: load("import/compiled/mapping.json"),
        roles: load("import/compiled/roles.json"),
        normalization: load("import/policies/normalization.json"),
        rewards: load("import/policies/rewards.json"),
        query_sets: (1..=5)
            .map(|i| NamedQuerySet {
                name: OwnedDefinitionKey::new(format!("original-{i:02}")).unwrap(),
                queries: load(&format!("import/queries/original-{i:02}.json")),
            })
            .collect(),
        items: load("resistance/items.json"),
        item_source: load("resistance/item-source.json"),
    }
}
fn stage(input: SuccessorBundleInput) -> StagedSuccessorBundle {
    transition_owned_bundle(input, SuccessorBundleLimits::default()).unwrap()
}
fn schema_rebind(input: &mut SuccessorBundleInput) {
    let schema = OwnedDefinitionSchemaPackage::new(
        input.successor.schema.clone(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    input.successor.rules.definitions = schema.identity().clone();
    input.successor.routing.definitions = schema.identity().clone();
}

#[test]
fn usage_inputs_successor_rebinds_checked_dependencies_without_changing_domain() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::{
        owned_mapping::OwnedMappingIndex,
        owned_normalize::{UsageInputPolicy, gem_inventory_scalar_inputs_identity},
        owned_recipe::assemble_owned_recipe,
        owned_skill_catalog::OwnedSkillRoleIndex,
    };
    let mut input = input();
    let prior = assemble_owned_recipe(input.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        prior.registry(),
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        input.roles.clone(),
        &mapping,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    input.normalization.usage_inputs = Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions: prior.schema().identity().clone(),
        roles: *roles.identity(),
        catalog: roles.input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(
            &input.normalization,
            Default::default(),
        )
        .unwrap(),
        gems: vec![],
    });
    let before = serde_json::to_vec(&input).unwrap();
    let next = stage(input.clone());
    let mut expected = input.normalization.usage_inputs.clone().unwrap();
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions,
        roles,
        scalar_inputs,
        ..
    } = &mut expected;
    *definitions = next.roles().input().definitions.clone();
    *roles = *next.roles().identity();
    *scalar_inputs =
        gem_inventory_scalar_inputs_identity(next.normalization(), Default::default()).unwrap();
    assert_ne!(input.normalization.usage_inputs.as_ref(), Some(&expected));
    assert_eq!(next.normalization().usage_inputs.as_ref(), Some(&expected));
    assert_eq!(serde_json::to_vec(&input).unwrap(), before);
    let wrong = digest_owned("stale-usage-input-successor", &1, 100).unwrap();
    for case in 0..4 {
        let mut stale = input.clone();
        let UsageInputPolicy::PobPhysicalPrimarySkillV1 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            ..
        } = stale.normalization.usage_inputs.as_mut().unwrap();
        match case {
            0 => definitions.release = "stale".into(),
            1 => *roles = wrong,
            2 => *catalog = wrong,
            3 => *scalar_inputs = wrong,
            _ => unreachable!(),
        }
        assert!(
            transition_owned_bundle(stale, Default::default()).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn support_inventory_rebinds_reviewed_roles_without_repairing_stale_source_authority() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::{
        owned_mapping::OwnedMappingIndex, owned_normalize::SupportOriginOrderPolicy,
        owned_recipe::assemble_owned_recipe, owned_skill_catalog::OwnedSkillRoleIndex,
    };
    let (mut input, changed_source) = catalog_input();
    let prior = assemble_owned_recipe(input.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        prior.registry(),
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        input.roles.clone(),
        &mapping,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    assert!(input.normalization.gem_inventory.is_none());
    input.normalization.support_origin_order = Some(
        SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
            mapping_source: *mapping.source_identity(),
            roles: *roles.identity(),
        },
    );
    let before = serde_json::to_vec(&input).unwrap();
    let mut append = changed_source.clone();
    append.source = input.mapping.source.clone();
    let next = transition_owned_catalog(input.clone(), append.clone(), Default::default()).unwrap();
    assert_ne!(next.roles().identity(), roles.identity());
    assert_eq!(next.mapping().source_identity(), mapping.source_identity());
    assert_eq!(
        next.normalization().support_origin_order,
        Some(
            SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
                mapping_source: *mapping.source_identity(),
                roles: *next.roles().identity(),
            }
        )
    );
    assert_eq!(next.query_sets(), input.query_sets);
    assert!(next.normalization().gem_inventory.is_none());
    for field in 0..2 {
        let mut stale = input.clone();
        let Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
            mapping_source,
            roles,
        }) = &mut stale.normalization.support_origin_order
        else {
            unreachable!()
        };
        let invalid = digest_owned("stale-physical-support-inventory", &field, 100).unwrap();
        if field == 0 {
            *mapping_source = invalid;
        } else {
            *roles = invalid;
        }
        assert!(
            matches!(
                transition_owned_catalog(stale, append.clone(), Default::default()),
                Err(SuccessorBundleError::Normalization(_))
            ),
            "stale prior dependency {field}"
        );
    }
    assert!(
        matches!(
            transition_owned_catalog(input.clone(), changed_source, Default::default()),
            Err(SuccessorBundleError::Normalization(_))
        ),
        "changed source needs explicit renewed inventory authority"
    );
    assert_eq!(serde_json::to_vec(&input).unwrap(), before);
}

#[test]
fn reward_inventory_tracks_checked_catalog_bindings_without_repairing_stale_authority() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::{
        owned_mapping::OwnedMappingIndex,
        owned_normalize::ConfigurationRewardInventoryPolicy,
        owned_recipe::assemble_owned_recipe,
        owned_reward_policy::{OwnedRewardPolicy, RewardTemplate},
    };
    let mut ordinary = input();
    let prior = assemble_owned_recipe(ordinary.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        ordinary.mapping.clone(),
        prior.registry(),
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let rewards = OwnedRewardPolicy::new(
        ordinary.rewards.clone(),
        &mapping,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let policy = configuration_rewards::policy(&mapping, &rewards);
    ordinary.normalization.configuration_reward_inventory = Some(policy.clone());
    let before = serde_json::to_vec(&ordinary).unwrap();
    let next = stage(ordinary.clone());
    assert_ne!(next.rewards().identity(), rewards.identity());
    configuration_rewards::assert_rebound(
        &policy,
        next.normalization()
            .configuration_reward_inventory
            .as_ref()
            .unwrap(),
        next.rewards(),
    );
    assert_eq!(next.rewards().input().rules, ordinary.rewards.rules);
    assert_eq!(next.query_sets(), ordinary.query_sets);

    for case in 0..3 {
        let mut stale = ordinary.clone();
        match case {
            0 => {
                let ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
                    reward_policy,
                    ..
                } = stale
                    .normalization
                    .configuration_reward_inventory
                    .as_mut()
                    .unwrap();
                *reward_policy = digest_owned("stale-reward-prior", &0, 100).unwrap();
            }
            1 => {
                stale.rewards.version =
                    OwnedDefinitionKey::new("different-reward-authoring").unwrap()
            }
            2 => {
                stale.rewards.rules[0]
                    .outcomes
                    .iter_mut()
                    .find(|case| matches!(case.outcome, RewardTemplate::Reward { .. }))
                    .unwrap()
                    .outcome = RewardTemplate::None;
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                transition_owned_bundle(stale, Default::default()),
                Err(SuccessorBundleError::Normalization(_))
            ),
            "stale prior case {case}"
        );
    }

    let (mut catalog, changed_source) = catalog_input();
    let catalog_policy = configuration_rewards::policy(next.mapping(), next.rewards());
    catalog.normalization.configuration_reward_inventory = Some(catalog_policy.clone());
    let mut same_source = changed_source.clone();
    same_source.source = catalog.mapping.source.clone();
    let result =
        transition_owned_catalog(catalog.clone(), same_source, Default::default()).unwrap();
    configuration_rewards::assert_rebound(
        &catalog_policy,
        result
            .normalization()
            .configuration_reward_inventory
            .as_ref()
            .unwrap(),
        result.rewards(),
    );
    assert_eq!(result.rewards().input().rules, catalog.rewards.rules);
    assert_eq!(
        result.mapping().input().entries.len(),
        catalog.mapping.entries.len() + 1
    );
    assert_eq!(
        result.mapping().source_identity(),
        mapping.source_identity()
    );
    assert_eq!(result.query_sets(), catalog.query_sets);
    assert!(
        matches!(
            transition_owned_catalog(catalog, changed_source, Default::default()),
            Err(SuccessorBundleError::Normalization(_))
        ),
        "new source needs newly authored authority"
    );
    assert_eq!(serde_json::to_vec(&ordinary).unwrap(), before);
}
#[test]
fn equipment_membership_rebinds_only_after_validating_the_prior_policy() {
    use poe_optimizer_import::owned_normalize::EquipmentMembershipPolicy;
    let mut input = input();
    input.normalization.equipment_membership =
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
            definitions: input.prior.rules.definitions.clone(),
            templates: vec![],
            source_base_names: vec!["Source inventory fixture".into()],
            loader_jewel_fallback_titles: vec!["Loader inventory fixture".into()],
        });
    let original = input.normalization.equipment_membership.clone();
    let staged = stage(input.clone());
    let mut expected = original.clone().unwrap();
    let EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. } = &mut expected else {
        panic!("expected legacy policy")
    };
    *definitions = staged.assembled().schema().identity().clone();
    assert_ne!(Some(&expected), original.as_ref());
    assert_eq!(
        staged.normalization().equipment_membership.as_ref(),
        Some(&expected)
    );
    assert_eq!(staged.query_sets(), &input.query_sets);
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. }) =
        &mut input.normalization.equipment_membership
    else {
        unreachable!()
    };
    *definitions = input.successor.rules.definitions.clone();
    assert!(
        transition_owned_bundle(input, Default::default()).is_err(),
        "a stale prior must not be silently repaired"
    );
}

#[test]
fn enemy_level_policy_survives_schema_and_catalog_changes_but_not_new_source_pins() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::owned_normalize::EnemyLevelPolicy;
    let checked = stage(input());
    let policy = enemy_level::policy(checked.mapping());
    let expected = serde_json::to_vec(&policy).unwrap();
    let mut ordinary = input();
    ordinary.normalization.enemy_level = Some(policy.clone());
    let next = stage(ordinary.clone());
    assert_eq!(
        serde_json::to_vec(next.normalization().enemy_level.as_ref().unwrap()).unwrap(),
        expected
    );
    assert_eq!(next.query_sets(), &ordinary.query_sets);
    let EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 { mapping_source, .. } =
        ordinary.normalization.enemy_level.as_mut().unwrap();
    *mapping_source = digest_owned("stale-enemy-level-source", &0, 100).unwrap();
    assert!(matches!(
        transition_owned_bundle(ordinary, Default::default()),
        Err(SuccessorBundleError::Normalization(_))
    ));

    let (mut input, changed_source) = catalog_input();
    input.normalization.enemy_level = Some(policy);
    let mut same_source = changed_source.clone();
    same_source.source = input.mapping.source.clone();
    let result = transition_owned_catalog(input.clone(), same_source, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_vec(result.normalization().enemy_level.as_ref().unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        result.mapping().source_identity(),
        checked.mapping().source_identity()
    );
    assert_eq!(
        result.mapping().input().entries.len(),
        input.mapping.entries.len() + 1
    );
    assert_eq!(result.query_sets(), input.query_sets);
    // The same catalog addition with new source bytes cannot inherit the old
    // reviewed callback branch merely because all definition IDs still match.
    assert!(matches!(
        transition_owned_catalog(input, changed_source, Default::default()),
        Err(SuccessorBundleError::Normalization(_))
    ));
}

#[test]
fn imported_equipment_catalog_rebind_checks_both_digests_without_other_item_policies() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::{
        owned_item_lines::OwnedItemLinePolicy, owned_item_source::ItemSourceLayoutPolicy,
        owned_normalize::EquipmentMembershipPolicy, owned_recipe::assemble_owned_recipe,
    };
    let (mut input, append) = catalog_input();
    let prior = assemble_owned_recipe(input.prior.clone(), Default::default()).unwrap();
    let items =
        OwnedItemLinePolicy::new(input.items.clone(), prior.schema(), Default::default()).unwrap();
    let source = ItemSourceLayoutPolicy::new(
        input.item_source.clone(),
        &items,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    input.normalization.item_modifier_membership = None;
    input.normalization.item_parameter_inputs = None;
    input.normalization.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            definitions: prior.schema().identity().clone(),
            templates: vec![],
            source_base_names: vec!["Reviewed empty imported domain".into()],
            loader_jewel_fallback_titles: vec!["Reviewed loader title".into()],
            item_lines: *items.identity(),
            item_source: *source.identity(),
            imported_profiles: vec![],
        },
    );
    let before = serde_json::to_vec(&input).unwrap();
    let result =
        transition_owned_catalog(input.clone(), append.clone(), Default::default()).unwrap();
    assert!(result.normalization().item_modifier_membership.is_none());
    assert!(result.normalization().item_parameter_inputs.is_none());
    let mut expected = input.normalization.equipment_membership.clone().unwrap();
    let EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        definitions,
        item_lines,
        item_source,
        ..
    } = &mut expected
    else {
        panic!()
    };
    assert_ne!(&*definitions, result.assembled().schema().identity());
    assert_ne!(*item_lines, *result.items().identity());
    assert_ne!(*item_source, *result.item_source().identity());
    *definitions = result.assembled().schema().identity().clone();
    *item_lines = *result.items().identity();
    *item_source = *result.item_source().identity();
    assert_eq!(
        result.normalization().equipment_membership.as_ref(),
        Some(&expected)
    );
    assert_eq!(result.query_sets(), &input.query_sets);
    assert_eq!(serde_json::to_vec(&input).unwrap(), before);
    for field in 0..2 {
        let mut stale = input.clone();
        let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            item_lines,
            item_source,
            ..
        }) = &mut stale.normalization.equipment_membership
        else {
            panic!()
        };
        let bad = digest_owned("stale-imported-equipment-policy", &field, 128).unwrap();
        if field == 0 {
            *item_lines = bad;
        } else {
            *item_source = bad;
        }
        assert!(
            matches!(
                transition_owned_catalog(stale, append.clone(), Default::default()),
                Err(SuccessorBundleError::Normalization(_))
            ),
            "stale imported dependency {field}"
        );
    }
    // Valid supplied item artifacts must not silently repair the independently
    // authored equipment bindings. Supplying those exact bindings is accepted.
    let mut supplied = input;
    supplied.items = result.items().input().clone();
    supplied.item_source = result.item_source().input().clone();
    let mut supplied_append = append;
    supplied_append.item_policies = CatalogItemPolicyMode::SuppliedSuccessor;
    assert!(matches!(
        transition_owned_catalog(
            supplied.clone(),
            supplied_append.clone(),
            Default::default()
        ),
        Err(SuccessorBundleError::Normalization(_))
    ));
    let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        item_lines,
        item_source,
        ..
    }) = &mut supplied.normalization.equipment_membership
    else {
        panic!()
    };
    *item_lines = *result.items().identity();
    *item_source = *result.item_source().identity();
    let supplied_result =
        transition_owned_catalog(supplied, supplied_append, Default::default()).unwrap();
    assert_eq!(
        supplied_result
            .normalization()
            .equipment_membership
            .as_ref(),
        Some(&expected)
    );
}

#[test]
fn encounter_and_reward_authority_survive_checked_schema_and_catalog_appends() {
    use poe_optimizer_core::{
        owned_content::digest_owned,
        owned_definitions::{
            BoundedInteger, EncounterDefId, EncounterDefinition, OptionDefinition,
        },
        owned_rules::DefinitionRules,
        owned_schema::{
            DeclaredSet, DefinitionEntry, EncounterSchema, IntegerRange, OptionSchema, SchemaFacet,
            SchemaGap, SchemaSubject,
        },
    };
    use poe_optimizer_import::{owned_mapping::*, owned_normalize::EncounterPolicy};

    fn carry(previous: &StagedSuccessorBundle) -> SuccessorBundleInput {
        SuccessorBundleInput {
            schema_version: OWNED_SUCCESSOR_VERSION,
            prior: previous.recipe().clone(),
            successor: previous.recipe().clone(),
            mapping: previous.mapping().input().clone(),
            roles: previous.roles().input().clone(),
            normalization: previous.normalization().clone(),
            rewards: previous.rewards().input().clone(),
            query_sets: previous.query_sets().to_vec(),
            items: previous.items().input().clone(),
            item_source: previous.item_source().input().clone(),
        }
    }
    fn append(input: &SuccessorBundleInput, mappings: Vec<MappingEntry>) -> CatalogAppend {
        CatalogAppend {
            mappings,
            source: input.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        }
    }
    let mut seed = current_input();
    let mut registry =
        OwnedIdRegistry::new(seed.prior.registry.clone(), Default::default()).unwrap();
    let encounter = registry
        .allocate_definition::<EncounterDefinition>()
        .unwrap();
    let owner = SchemaSubject::Definition(encounter.address());
    seed.successor.registry = registry.input().clone();
    seed.successor
        .schema
        .definitions
        .push(DefinitionDescriptor::Encounter(DefinitionEntry {
            id: encounter.clone(),
            schema: SchemaState::Known(EncounterSchema {
                enemy_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(85).unwrap(),
                },
                external_inputs: DeclaredSet::partial(
                    vec![],
                    vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::StaticLinks,
                        code: OwnedDefinitionKey::new("encounter-externals-unconverted").unwrap(),
                    }],
                ),
            }),
        }));
    seed.successor.rules.owners.push(DefinitionRules {
        owner: owner.clone(),
        programs: DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject: owner.clone(),
                facet: SchemaFacet::GameRules,
                code: OwnedDefinitionKey::new("encounter-programs-unconverted").unwrap(),
            }],
        ),
    });
    schema_rebind(&mut seed);
    let selector = ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Encounter,
        key: SourceComponent::Text("pinnacle-default".into()),
        version: SourceComponent::Text(seed.mapping.source.revision.clone()),
        variant: SourceComponent::Text("no-boss-skill-medium".into()),
    };
    let catalog = append(
        &seed,
        vec![MappingEntry {
            source: selector.clone(),
            outcome: MappingOutcome::Mapped {
                target: owner,
                basis: MappingBasis::Exact,
            },
        }],
    );
    let checked = transition_owned_catalog(seed, catalog, Default::default()).unwrap();
    let policy = EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        mapping_source: *checked.mapping().source_identity(),
        selector,
        target: encounter,
        absent_input_names: vec![
            "enemyIsBoss".into(),
            "presetBossSkills".into(),
            "enemySizePreset".into(),
        ],
    };
    let mut schema_only = carry(&checked);
    schema_only.normalization.encounter = Some(policy.clone());
    let rewards = configuration_rewards::policy(checked.mapping(), checked.rewards());
    schema_only.normalization.configuration_reward_inventory = Some(rewards.clone());
    let mut registry =
        OwnedIdRegistry::new(schema_only.prior.registry.clone(), Default::default()).unwrap();
    let option = registry.allocate_definition::<OptionDefinition>().unwrap();
    schema_only.successor.registry = registry.input().clone();
    schema_only
        .successor
        .schema
        .definitions
        .push(DefinitionDescriptor::Option(DefinitionEntry {
            id: option.clone(),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    schema_rebind(&mut schema_only);
    let before = serde_json::to_vec(&schema_only).unwrap();
    let catalog = append(&schema_only, vec![]);
    let next =
        transition_owned_catalog(schema_only.clone(), catalog.clone(), Default::default()).unwrap();
    assert_eq!(next.normalization().encounter, Some(policy.clone()));
    assert_eq!(next.mapping().input().entries, schema_only.mapping.entries);
    assert_ne!(next.mapping().identity(), checked.mapping().identity());
    configuration_rewards::assert_rebound(
        &rewards,
        next.normalization()
            .configuration_reward_inventory
            .as_ref()
            .unwrap(),
        next.rewards(),
    );
    assert_eq!(next.rewards().input().rules, schema_only.rewards.rules);
    assert_eq!(next.query_sets(), schema_only.query_sets);
    assert_eq!(next.recipe().rules.owners, schema_only.prior.rules.owners);

    for case in 0..3 {
        let mut invalid = schema_only.clone();
        let EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
            mapping_source,
            selector,
            target,
            ..
        } = invalid.normalization.encounter.as_mut().unwrap();
        match case {
            0 => *mapping_source = digest_owned("stale-encounter-source", &0, 100).unwrap(),
            1 => {
                let ExternalSelector::Catalog { key, .. } = selector else {
                    unreachable!()
                };
                *key = SourceComponent::Text("missing-encounter-selector".into());
            }
            2 => {
                *target =
                    EncounterDefId::parse(target.namespace().clone(), "missing-encounter").unwrap()
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                transition_owned_catalog(invalid, catalog.clone(), Default::default()),
                Err(SuccessorBundleError::Normalization(_))
            ),
            "invalid encounter authority {case}"
        );
    }
    let mut changed_source = catalog;
    changed_source.source.files.push(SourceFilePin {
        path: "test/new-encounter-source.lua".into(),
        sha256: "ab".repeat(32),
    });
    assert!(matches!(
        transition_owned_catalog(schema_only.clone(), changed_source, Default::default()),
        Err(SuccessorBundleError::Normalization(_))
    ));
    assert_eq!(serde_json::to_vec(&schema_only).unwrap(), before);

    let catalog_only = carry(&next);
    let prior_rewards = catalog_only
        .normalization
        .configuration_reward_inventory
        .clone()
        .unwrap();
    let addition = append(
        &catalog_only,
        vec![MappingEntry {
            source: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::Option,
                key: SourceComponent::Text("unrelated-encounter-regression-option".into()),
                version: SourceComponent::Missing,
                variant: SourceComponent::Missing,
            },
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(option.address()),
                basis: MappingBasis::Exact,
            },
        }],
    );
    let result =
        transition_owned_catalog(catalog_only.clone(), addition, Default::default()).unwrap();
    assert_eq!(result.normalization().encounter, Some(policy));
    configuration_rewards::assert_rebound(
        &prior_rewards,
        result
            .normalization()
            .configuration_reward_inventory
            .as_ref()
            .unwrap(),
        result.rewards(),
    );
    assert_eq!(result.recipe(), &catalog_only.prior);
    assert_eq!(result.rewards().input().rules, catalog_only.rewards.rules);
    assert_eq!(result.query_sets(), catalog_only.query_sets);
    assert_eq!(
        result.mapping().source_identity(),
        checked.mapping().source_identity()
    );
    assert_eq!(result.mapping().input().source, catalog_only.mapping.source);
    assert_eq!(
        result.mapping().input().entries.len(),
        catalog_only.mapping.entries.len() + 1
    );
}
#[test]
fn existing_mechanics_and_all_five_import_inputs_form_one_checked_successor() {
    let source = input();
    let previous = source.clone();
    let staged = stage(source);
    assert_eq!(staged.recipe(), &previous.successor);
    assert_eq!(staged.mapping().input().entries, previous.mapping.entries);
    assert_eq!(staged.mapping().input().source, previous.mapping.source);
    assert_eq!(
        staged.mapping().input().policy_version,
        previous.mapping.policy_version
    );
    assert_eq!(staged.roles().input().roles, previous.roles.roles);
    assert_eq!(
        staged.roles().input().compilation,
        previous.roles.compilation
    );
    assert_eq!(staged.rewards().input().rules, previous.rewards.rules);
    let mut expected_policy = previous.normalization;
    if let GemQualityPolicy::Attributes(quality) = &mut expected_policy.gem_quality {
        quality.definitions = staged.assembled().schema().identity().clone();
    }
    assert_eq!(staged.normalization(), &expected_policy);
    assert_eq!(staged.items().input(), &previous.items);
    assert_eq!(staged.item_source().input(), &previous.item_source);
    assert_eq!(staged.query_sets(), &previous.query_sets);
    assert_eq!(staged.transition().query_rows, 110);
    assert_eq!(
        staged.transition().preserved_registry_entries,
        previous.prior.registry.entries.len()
    );
    assert_eq!(
        staged.transition().preserved_definitions,
        previous.prior.schema.definitions.len()
    );
    assert_eq!(
        staged.transition().preserved_slots,
        previous.prior.schema.slots.len()
    );
    assert_ne!(
        staged.transition().before.rules,
        staged.transition().after.rules
    );
    assert_ne!(
        staged.transition().before.mapping,
        staged.transition().after.mapping
    );
    assert_eq!(staged.transition().calculation, "not_run");
    assert_eq!(staged.transition().whole_build_parity, "not_established");
    let artifacts: BTreeMap<_, _> = staged.artifacts().collect();
    assert_eq!(artifacts.len(), 18);
    for artifact in &staged.transition().artifacts {
        let bytes = artifacts[artifact.file.as_str()];
        assert_eq!(bytes.len(), artifact.bytes);
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), artifact.sha256);
    }
    for queries in staged.query_sets() {
        let bytes = artifacts[format!("queries-{}.json", queries.name.as_str()).as_str()];
        assert_eq!(serde_json::from_slice::<Vec<poe_optimizer_import::owned_normalize::ImportQueryTemplate>>(bytes).unwrap(), queries.queries);
    }
}
#[test]
fn already_bound_endpoint_is_a_valid_no_allocation_transition() {
    let first = stage(input());
    let next = SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: first.recipe().clone(),
        successor: first.recipe().clone(),
        mapping: first.mapping().input().clone(),
        roles: first.roles().input().clone(),
        normalization: first.normalization().clone(),
        rewards: first.rewards().input().clone(),
        query_sets: first.query_sets().to_vec(),
        items: first.items().input().clone(),
        item_source: first.item_source().input().clone(),
    };
    let second = stage(next);
    assert_eq!(second.recipe(), first.recipe());
    assert_eq!(second.transition().before, second.transition().after);
    assert_eq!(second.mapping().identity(), first.mapping().identity());
    assert_eq!(second.roles().identity(), first.roles().identity());
}
#[test]
fn new_rule_values_require_compilation_but_are_not_forced_to_equal_prior_rules() {
    let mut changed = input();
    let value = changed
        .successor
        .rules
        .tables
        .iter_mut()
        .flat_map(|t| &mut t.rows)
        .find_map(|v| match v {
            ParameterValue::Quantity(q) => Some(q),
            _ => None,
        })
        .unwrap();
    *value = FiniteQuantity::new(value.value() + 0.125, value.unit().clone()).unwrap();
    let expected = changed.successor.rules.clone();
    let result = stage(changed);
    assert_eq!(result.assembled().rules().input(), &expected);
    assert_ne!(
        result.transition().before.rules,
        result.transition().after.rules
    );
    let mut invalid = input();
    invalid.successor.rules.operations_version =
        OwnedDefinitionKey::new("unknown-native-operations").unwrap();
    assert!(transition_owned_bundle(invalid, SuccessorBundleLimits::default()).is_err());
}
#[test]
fn stale_prior_bindings_are_rejected_before_any_repair() {
    let base = input();
    let mut mapping = base.clone();
    mapping.mapping.definitions = mapping.successor.rules.definitions.clone();
    let mut roles = base.clone();
    roles.roles.definitions = roles.successor.rules.definitions.clone();
    let mut rewards = base.clone();
    rewards.rewards.definitions = rewards.successor.rules.definitions.clone();
    let mut quality = base;
    if let GemQualityPolicy::Attributes(value) = &mut quality.normalization.gem_quality {
        value.definitions = quality.successor.rules.definitions.clone();
    }
    for bad in [mapping, roles, rewards, quality] {
        assert!(transition_owned_bundle(bad, SuccessorBundleLimits::default()).is_err());
    }
}
#[test]
fn registry_conflicts_and_changed_old_declarations_cannot_be_rebound() {
    let mut conflicting = input();
    conflicting.successor.registry.entries.swap(0, 1);
    conflicting.successor.registry.entries[0].target =
        conflicting.successor.registry.entries[1].target.clone();
    assert!(transition_owned_bundle(conflicting, SuccessorBundleLimits::default()).is_err());
    let mut changed = input();
    let gem = changed
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(entry) => match &mut entry.schema {
                SchemaState::Known(schema) => Some(schema),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    gem.level.maximum =
        poe_optimizer_core::owned_definitions::BoundedInteger::new(gem.level.maximum.get() - 1)
            .unwrap();
    schema_rebind(&mut changed);
    assert!(matches!(
        transition_owned_bundle(changed, SuccessorBundleLimits::default()),
        Err(SuccessorBundleError::ChangedDeclaration)
    ));
}
#[test]
fn contradictory_roles_and_stale_successor_item_policies_fail() {
    let mut role = input();
    let known_gem = role
        .prior
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(entry) if matches!(entry.schema, SchemaState::Known(_)) => {
                Some(entry.id.clone())
            }
            _ => None,
        })
        .unwrap();
    role.roles
        .roles
        .iter_mut()
        .find(|r| r.gem == known_gem)
        .unwrap()
        .materialization = OwnedGemMaterialization::ProviderOnly;
    assert!(transition_owned_bundle(role, SuccessorBundleLimits::default()).is_err());
    let mut items = input();
    items.items.definitions = items.prior.rules.definitions.clone();
    assert!(transition_owned_bundle(items, SuccessorBundleLimits::default()).is_err());
    let mut layout = input();
    layout.item_source.item_lines =
        poe_optimizer_core::owned_content::digest_owned("different-policy", &true, 1024).unwrap();
    assert!(transition_owned_bundle(layout, SuccessorBundleLimits::default()).is_err());
}
#[test]
fn names_duplicate_rows_and_aggregate_limits_reject_without_mutation() {
    let base = input();
    let snapshot = serde_json::to_vec(&base).unwrap();
    for limits in [
        SuccessorBundleLimits {
            max_input_bytes: 64,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_output_bytes: 64,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_query_sets: 1,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_queries: 30,
            ..Default::default()
        },
    ] {
        assert!(transition_owned_bundle(base.clone(), limits).is_err());
    }
    assert_eq!(serde_json::to_vec(&base).unwrap(), snapshot);
    let mut duplicate = base.clone();
    duplicate.query_sets[1].name = duplicate.query_sets[0].name.clone();
    assert!(matches!(
        transition_owned_bundle(duplicate, Default::default()),
        Err(SuccessorBundleError::QuerySetName)
    ));
    let mut unsafe_name = base.clone();
    unsafe_name.query_sets[0].name = OwnedDefinitionKey::new("parent.child").unwrap();
    assert!(matches!(
        transition_owned_bundle(unsafe_name, Default::default()),
        Err(SuccessorBundleError::QuerySetName)
    ));
    let mut duplicate_row = base;
    let row = duplicate_row.query_sets[0].queries[0].clone();
    duplicate_row.query_sets[0].queries.push(row);
    assert!(transition_owned_bundle(duplicate_row, Default::default()).is_err());
}
#[test]
fn strict_bundle_wire_rejects_unknown_duplicate_and_missing_fields() {
    let raw = serde_json::to_string(&input()).unwrap();
    let duplicate = raw.replacen("{", "{\"schema_version\":1,", 1);
    assert!(decode_successor_bundle(duplicate.as_bytes(), Default::default()).is_err());
    let unknown = raw.replacen("{", "{\"hidden_rebind\":true,", 1);
    assert!(decode_successor_bundle(unknown.as_bytes(), Default::default()).is_err());
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value.as_object_mut().unwrap().remove("item_source");
    assert!(
        decode_successor_bundle(&serde_json::to_vec(&value).unwrap(), Default::default()).is_err()
    );
}

#[test]
fn normalization_mapping_and_combined_successor_query_budgets_are_honored() {
    let base = input();
    let mut limits = SuccessorBundleLimits::default();
    limits.normalization.mapping.max_entries = 1;
    assert!(transition_owned_bundle(base, limits).is_err());

    let mut changed = input();
    changed.query_sets.truncate(1);
    let before_bytes =
        serde_json::to_vec(&(&changed.normalization, &changed.query_sets[0].queries))
            .unwrap()
            .len();
    changed.successor.schema.release =
        OwnedDefinitionKey::new("expanded-release-".to_owned() + &"x".repeat(100)).unwrap();
    schema_rebind(&mut changed);
    let mut rebound = changed.normalization.clone();
    if let GemQualityPolicy::Attributes(quality) = &mut rebound.gem_quality {
        quality.definitions = changed.successor.rules.definitions.clone();
    }
    let after_bytes = serde_json::to_vec(&(&rebound, &changed.query_sets[0].queries))
        .unwrap()
        .len();
    assert!(after_bytes > before_bytes);
    let mut limits = SuccessorBundleLimits::default();
    limits.normalization.max_policy_bytes = before_bytes;
    assert!(
        matches!(transition_owned_bundle(changed, limits), Err(SuccessorBundleError::Digest(poe_optimizer_core::owned_content::ContentDigestError::TooLarge { maximum })) if maximum == before_bytes)
    );
}
#[test]
fn rule_programs_and_table_cells_consume_the_aggregate_entry_budget() {
    let base = input();
    let declarations_only: usize = [&base.prior, &base.successor]
        .iter()
        .map(|r| r.registry.entries.len() + r.schema.definitions.len() + r.schema.slots.len())
        .sum::<usize>()
        + base.mapping.entries.len()
        + base.roles.roles.len()
        + base.rewards.rules.len()
        + base
            .rewards
            .rules
            .iter()
            .map(|r| r.outcomes.len())
            .sum::<usize>()
        + base.items.rules.len()
        + base.item_source.rule_layouts.len()
        + base.item_source.template_layouts.len()
        + base
            .query_sets
            .iter()
            .map(|q| q.queries.len())
            .sum::<usize>();
    assert!(!base.successor.rules.tables.is_empty());
    assert!(matches!(
        transition_owned_bundle(
            base,
            SuccessorBundleLimits {
                max_validation_entries: declarations_only,
                ..Default::default()
            }
        ),
        Err(SuccessorBundleError::Limit("validation entries"))
    ));
}

// Reconstruct the immutable pre-tree stage through its real publisher. The shipped
// current bundle continues forward and must never become a second allocation base.
fn current_input() -> SuccessorBundleInput {
    let previous = stage(input());
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: previous.recipe().clone(),
        successor: previous.recipe().clone(),
        mapping: previous.mapping().input().clone(),
        roles: previous.roles().input().clone(),
        normalization: previous.normalization().clone(),
        rewards: previous.rewards().input().clone(),
        query_sets: previous.query_sets().to_vec(),
        items: previous.items().input().clone(),
        item_source: previous.item_source().input().clone(),
    }
}

fn catalog_input() -> (SuccessorBundleInput, CatalogAppend) {
    use poe_optimizer_core::{
        owned_definitions::OptionDefinition,
        owned_schema::{DefinitionAddress, DefinitionEntry, OptionSchema, SchemaSubject},
    };
    use poe_optimizer_import::owned_mapping::*;
    let mut input = current_input();
    let mut registry =
        OwnedIdRegistry::new(input.prior.registry.clone(), OwnedMappingLimits::default()).unwrap();
    let option = registry.allocate_definition::<OptionDefinition>().unwrap();
    input.successor.registry = registry.input().clone();
    input
        .successor
        .schema
        .definitions
        .push(DefinitionDescriptor::Option(DefinitionEntry {
            id: option.clone(),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    schema_rebind(&mut input);
    let append = CatalogAppend {
        mappings: vec![MappingEntry {
            source: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::Option,
                key: SourceComponent::Text("new-independent-catalog-option".into()),
                version: SourceComponent::Text("v1".into()),
                variant: SourceComponent::Missing,
            },
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Option(option)),
                basis: MappingBasis::Exact,
            },
        }],
        source: SourcePin {
            system: input.mapping.source.system,
            revision: input.mapping.source.revision.clone(),
            files: vec![SourceFilePin {
                path: "test/independent-catalog.json".into(),
                sha256: "ab".repeat(32),
            }],
        },
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    (input, append)
}
#[test]
fn catalog_additions_share_checked_finalization_and_preserve_existing_facts() {
    let (input, append) = catalog_input();
    let previous = input.clone();
    let expected_append = append.clone();
    let result = transition_owned_catalog(input, append, Default::default()).unwrap();
    assert_eq!(
        result.recipe().registry.last_issued.get(),
        previous.prior.registry.last_issued.get() + 1
    );
    assert_eq!(
        &result.recipe().registry.entries[..previous.prior.registry.entries.len()],
        &previous.prior.registry.entries
    );
    assert_eq!(
        result.mapping().input().entries.len(),
        previous.mapping.entries.len() + 1
    );
    for entry in &previous.mapping.entries {
        assert!(result.mapping().input().entries.contains(entry));
    }
    assert!(
        result
            .mapping()
            .input()
            .entries
            .contains(&expected_append.mappings[0])
    );
    assert_eq!(
        result.mapping().input().source.files.len(),
        previous.mapping.source.files.len() + 1
    );
    assert_eq!(
        result.roles().input().compilation,
        previous.roles.compilation
    );
    assert_eq!(result.roles().input().roles, previous.roles.roles);
    assert_eq!(result.roles().input().mapping, *result.mapping().identity());
    assert_eq!(result.query_sets(), previous.query_sets);
    assert_eq!(result.rewards().input().rules, previous.rewards.rules);
    assert_eq!(result.items().input().rules, previous.items.rules);
    assert_eq!(
        result.item_source().input().source,
        previous.item_source.source
    );
    let mut expected_source = previous.item_source;
    expected_source.item_lines = *result.items().identity();
    assert_eq!(result.item_source().input(), &expected_source);
    assert_eq!(
        result.transition().item_policy_mode,
        "validated_prior_binding_rebind"
    );
    assert_eq!(result.recipe().rules.owners, previous.prior.rules.owners);
    assert_eq!(result.recipe().rules.tables, previous.prior.rules.tables);
    let raw = result
        .artifacts()
        .find(|(name, _)| *name == "catalog-append.json")
        .unwrap()
        .1;
    assert_eq!(
        serde_json::from_slice::<CatalogAppend>(raw).unwrap(),
        expected_append
    );
    assert!(
        result
            .transition()
            .artifacts
            .iter()
            .any(|a| a.file == "catalog-append.json")
    );
}
#[test]
fn catalog_repeat_accepts_matching_overlaps_but_no_existing_selector_append() {
    let input = current_input();
    let append = CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let before = input.clone();
    let result = transition_owned_catalog(input, append.clone(), Default::default()).unwrap();
    assert_eq!(result.recipe(), &before.prior);
    assert_eq!(result.mapping().input(), &before.mapping);
    assert_eq!(result.roles().input(), &before.roles);
    assert_eq!(result.transition().before, result.transition().after);
    let mut bad = append;
    bad.mappings.push(before.mapping.entries[0].clone());
    assert!(matches!(
        transition_owned_catalog(before, bad, Default::default()),
        Err(SuccessorBundleError::CatalogConflict(
            "existing or duplicate mapping selector"
        ))
    ));
}
#[test]
fn catalog_conflicts_fail_before_publication_and_leave_inputs_unchanged() {
    use poe_optimizer_import::owned_mapping::ExternalSourceSystem;
    let (input, append) = catalog_input();
    let snapshot = serde_json::to_vec(&input).unwrap();
    let mut duplicate_mapping = append.clone();
    let duplicate_entry = duplicate_mapping.mappings[0].clone();
    duplicate_mapping.mappings.push(duplicate_entry);
    let mut duplicate_pin = append.clone();
    let duplicate_file = duplicate_pin.source.files[0].clone();
    duplicate_pin.source.files.push(duplicate_file);
    let mut wrong_hash = append.clone();
    wrong_hash.source.files = vec![input.mapping.source.files[0].clone()];
    wrong_hash.source.files[0].sha256 = "cd".repeat(32);
    let mut revision = append.clone();
    revision.source.revision.push_str("-different");
    let mut system = append;
    system.source.system = ExternalSourceSystem::PathOfBuilding1;
    for bad in [
        duplicate_mapping,
        duplicate_pin,
        wrong_hash,
        revision,
        system,
    ] {
        assert!(matches!(
            transition_owned_catalog(input.clone(), bad, Default::default()),
            Err(SuccessorBundleError::CatalogConflict(_))
        ));
    }
    assert_eq!(serde_json::to_vec(&input).unwrap(), snapshot);
}
#[test]
fn catalog_item_rebind_never_repairs_stale_or_implicitly_supplied_bindings() {
    let (input, append) = catalog_input();
    let mut stale = input.clone();
    stale.items.definitions = stale.successor.rules.definitions.clone();
    assert!(transition_owned_catalog(stale, append.clone(), Default::default()).is_err());
    let mut supplied = append;
    supplied.item_policies = CatalogItemPolicyMode::SuppliedSuccessor;
    assert!(transition_owned_catalog(input, supplied, Default::default()).is_err());
}
#[test]
fn catalog_append_uses_combined_bytes_and_combined_mapping_bounds() {
    let (input, append) = catalog_input();
    let limits = SuccessorBundleLimits {
        max_input_bytes: serde_json::to_vec(&input).unwrap().len(),
        ..Default::default()
    };
    assert!(transition_owned_catalog(input.clone(), append.clone(), limits).is_err());
    let mut limits = SuccessorBundleLimits::default();
    limits.catalog.mapping.max_collection_entries = input.mapping.entries.len();
    assert!(transition_owned_catalog(input, append, limits).is_err());
}

fn empty_tree_content(
    input: &SuccessorBundleInput,
) -> poe_optimizer_import::owned_tree_policy::TreeNormalizationContent {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::owned_tree_policy::*;
    TreeNormalizationContent {
        access: None,
        version: OwnedDefinitionKey::new("test-tree-policy").unwrap(),
        source: input.mapping.source.clone(),
        catalog: digest_owned("test-catalog", &1, 100).unwrap(),
        policy: digest_owned("test-policy", &2, 100).unwrap(),
        tree_version: "test-tree".into(),
        classes: vec![],
        ascendancies: vec![],
        tokens: vec![TreeTokenRow {
            token: "unknown-node".into(),
            role: TreeTokenRole::Unresolved {
                code: OwnedDefinitionKey::new("not-converted").unwrap(),
            },
        }],
        attributes: vec![],
        syntax: TreeNormalizationSyntax {
            tree_version_attribute: "treeVersion".into(),
            class_attribute: "classInternalId".into(),
            ascendancy_attribute: "ascendancyInternalId".into(),
            class_consistency_attribute: Some("classId".into()),
            ascendancy_consistency_attribute: Some("ascendClassId".into()),
            overrides_element: "Overrides".into(),
            attribute_override_element: "AttributeOverride".into(),
            weapon_overlays: vec![],
            ignored_spec_children: vec!["URL".into()],
        },
    }
}
#[test]
fn tree_installation_uses_final_bindings_and_one_fixed_manifest_artifact() {
    let (input, append) = catalog_input();
    let content = empty_tree_content(&input);
    let result = transition_owned_catalog_with_tree(
        input,
        append,
        TreePolicyTransitionInput::Install {
            content: Box::new(content.clone()),
        },
        Default::default(),
    )
    .unwrap();
    let tree = result.tree().unwrap();
    assert_eq!(tree.input().content, content);
    assert_eq!(
        tree.input().definitions,
        *result.assembled().schema().identity()
    );
    assert_eq!(tree.input().mapping, *result.mapping().identity());
    assert_eq!(
        tree.input().normalization,
        result.transition().after.normalization
    );
    assert_eq!(result.transition().tree, Some(*tree.identity()));
    let files: BTreeMap<_, _> = result.artifacts().collect();
    assert_eq!(
        serde_json::from_slice::<
            poe_optimizer_import::owned_tree_policy::TreeNormalizationPackageInput,
        >(files["tree-normalization.json"])
        .unwrap(),
        *tree.input()
    );
    assert!(
        result
            .transition()
            .artifacts
            .iter()
            .any(|r| r.file == "tree-normalization.json")
    );
    assert_eq!(files.len(), 20); // Original18 + catalog append + one typed tree artifact.
}
#[test]
fn tree_rebinding_checks_prior_package_and_preserves_content() {
    let input = current_input();
    let content = empty_tree_content(&input);
    let append = CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let installed = transition_owned_catalog_with_tree(
        input,
        append.clone(),
        TreePolicyTransitionInput::Install {
            content: Box::new(content),
        },
        Default::default(),
    )
    .unwrap();
    let input = SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: installed.recipe().clone(),
        successor: installed.recipe().clone(),
        mapping: installed.mapping().input().clone(),
        roles: installed.roles().input().clone(),
        normalization: installed.normalization().clone(),
        rewards: installed.rewards().input().clone(),
        query_sets: installed.query_sets().to_vec(),
        items: installed.items().input().clone(),
        item_source: installed.item_source().input().clone(),
    };
    let tree = installed.tree().unwrap().input().clone();
    let rebound = transition_owned_catalog_with_tree(
        input.clone(),
        append.clone(),
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(tree.clone()),
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(rebound.tree().unwrap().input(), &tree);
    let mut stale = tree;
    stale.mapping = poe_optimizer_core::owned_content::digest_owned("wrong", &1, 100).unwrap();
    assert!(
        transition_owned_catalog_with_tree(
            input,
            append,
            TreePolicyTransitionInput::RebindPrior {
                prior: Box::new(stale)
            },
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn tree_install_never_bypasses_prior_policy_checks_or_shared_output_limits() {
    let (input, append) = catalog_input();
    let content = empty_tree_content(&input);
    let mut stale = input.clone();
    if let GemQualityPolicy::Attributes(q) = &mut stale.normalization.gem_quality {
        q.definitions = stale.successor.rules.definitions.clone();
    }
    assert!(
        transition_owned_catalog_with_tree(
            stale,
            append.clone(),
            TreePolicyTransitionInput::Install {
                content: Box::new(content.clone())
            },
            Default::default()
        )
        .is_err()
    );
    let baseline =
        transition_owned_catalog(input.clone(), append.clone(), Default::default()).unwrap();
    let old_bytes = baseline.artifacts().map(|(_, bytes)| bytes.len()).sum();
    let limits = SuccessorBundleLimits {
        max_output_bytes: old_bytes,
        ..Default::default()
    };
    assert!(
        transition_owned_catalog_with_tree(
            input,
            append,
            TreePolicyTransitionInput::Install {
                content: Box::new(content)
            },
            limits
        )
        .is_err()
    );
}

#[test]
fn passive_socket_catalog_rebind_preserves_authority_and_checks_empty_domain_dependencies() {
    use poe_optimizer_import::{
        owned_item_lines::OwnedItemLinePolicy, owned_item_source::ItemSourceLayoutPolicy,
        owned_mapping::OwnedMappingIndex, owned_recipe::assemble_owned_recipe,
        owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let (mut input, changed_source) = catalog_input();
    let mut append = changed_source.clone();
    append.source = input.mapping.source.clone();
    let prior = assemble_owned_recipe(input.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        prior.registry(),
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let items =
        OwnedItemLinePolicy::new(input.items.clone(), prior.schema(), Default::default()).unwrap();
    let source = ItemSourceLayoutPolicy::new(
        input.item_source.clone(),
        &items,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let content = empty_tree_content(&input);
    passive_socket_policy::attach(
        &mut input.normalization,
        prior.schema(),
        &mapping,
        &items,
        &source,
        &content,
    );
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        content.clone(),
        prior.registry(),
        prior.schema(),
        &mapping,
        &input.normalization,
        Default::default(),
    )
    .unwrap();
    let before = serde_json::to_vec(&input).unwrap();
    let result = transition_owned_catalog_with_tree(
        input.clone(),
        append.clone(),
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(tree.input().clone()),
        },
        Default::default(),
    )
    .unwrap();
    assert_ne!(
        result.assembled().schema().identity(),
        prior.schema().identity()
    );
    assert_ne!(result.items().identity(), items.identity());
    assert_ne!(result.item_source().identity(), source.identity());
    let mut expected = input.normalization.clone();
    passive_socket_policy::attach(
        &mut expected,
        result.assembled().schema(),
        result.mapping(),
        result.items(),
        result.item_source(),
        &content,
    );
    assert_eq!(
        result.normalization().passive_socket_membership,
        expected.passive_socket_membership
    );
    assert_eq!(
        result.normalization().equipment_membership,
        expected.equipment_membership
    );
    assert!(result.normalization().item_modifier_membership.is_none());
    assert!(result.normalization().item_parameter_inputs.is_none());
    assert_eq!(result.tree().unwrap().input().content, content);
    assert_eq!(
        result.mapping().source_identity(),
        mapping.source_identity()
    );
    assert_eq!(result.query_sets(), input.query_sets);
    assert_eq!(serde_json::to_vec(&input).unwrap(), before);
    for field in 0..7 {
        let mut stale = input.clone();
        passive_socket_policy::corrupt(
            &mut stale.normalization,
            field,
            result.assembled().schema().identity(),
        );
        // Rebind the full tree commitment where possible so a stale complete-tree
        // digest cannot mask item/source/content validation of an empty domain.
        if let Ok(tree) = OwnedTreeNormalizationPolicy::bind_new(
            content.clone(),
            prior.registry(),
            prior.schema(),
            &mapping,
            &stale.normalization,
            Default::default(),
        ) {
            assert!(
                transition_owned_catalog_with_tree(
                    stale,
                    append.clone(),
                    TreePolicyTransitionInput::RebindPrior {
                        prior: Box::new(tree.input().clone())
                    },
                    Default::default()
                )
                .is_err(),
                "dependency {field}"
            );
        }
    }
    assert!(
        transition_owned_catalog_with_tree(
            input.clone(),
            changed_source,
            TreePolicyTransitionInput::RebindPrior {
                prior: Box::new(tree.input().clone())
            },
            Default::default()
        )
        .is_err()
    );
    assert!(
        transition_owned_catalog_with_tree(
            input,
            append,
            TreePolicyTransitionInput::Install {
                content: Box::new(content)
            },
            Default::default()
        )
        .is_err(),
        "inherited placement needs its checked prior tree, not a replacement content assertion"
    );
}

#[test]
fn character_reward_inventory_policy_survives_schema_and_catalog_changes_but_not_new_source_pins() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::owned_normalize::CharacterRewardInventoryPolicy;
    let checked = stage(input());
    let policy = character_reward_inventory::policy(checked.mapping());
    let expected = serde_json::to_vec(&policy).unwrap();
    let mut ordinary = input();
    ordinary.normalization.character_reward_inventory = Some(policy.clone());
    let next = stage(ordinary.clone());
    assert_eq!(
        serde_json::to_vec(
            next.normalization()
                .character_reward_inventory
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        expected
    );
    assert_eq!(next.query_sets(), &ordinary.query_sets);
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { mapping_source, .. } =
        ordinary
            .normalization
            .character_reward_inventory
            .as_mut()
            .unwrap();
    *mapping_source = digest_owned("stale-character-reward-source", &0, 100).unwrap();
    assert!(matches!(
        transition_owned_bundle(ordinary, Default::default()),
        Err(SuccessorBundleError::Normalization(_))
    ));

    let (mut input, changed_source) = catalog_input();
    input.normalization.character_reward_inventory = Some(policy);
    let mut same_source = changed_source.clone();
    same_source.source = input.mapping.source.clone();
    let result = transition_owned_catalog(input.clone(), same_source, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_vec(
            result
                .normalization()
                .character_reward_inventory
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        expected
    );
    assert_eq!(
        result.mapping().source_identity(),
        checked.mapping().source_identity()
    );
    assert_eq!(
        result.mapping().input().entries.len(),
        input.mapping.entries.len() + 1
    );
    assert_eq!(result.query_sets(), input.query_sets);
    // The same catalog addition with new source bytes cannot inherit the old
    // reviewed callback branch merely because all definition IDs still match.
    assert!(matches!(
        transition_owned_catalog(input, changed_source, Default::default()),
        Err(SuccessorBundleError::Normalization(_))
    ));
}

#[test]
fn empty_character_runes_catalog_rebind_preserves_source_authority_and_passive_tree_commitments() {
    use poe_optimizer_import::{
        owned_item_lines::OwnedItemLinePolicy, owned_item_source::ItemSourceLayoutPolicy,
        owned_mapping::OwnedMappingIndex, owned_recipe::assemble_owned_recipe,
        owned_tree_policy::OwnedTreeNormalizationPolicy,
    };
    let (mut input, changed_source) = catalog_input();
    let mut append = changed_source.clone();
    append.source = input.mapping.source.clone();
    let prior = assemble_owned_recipe(input.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        prior.registry(),
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let items =
        OwnedItemLinePolicy::new(input.items.clone(), prior.schema(), Default::default()).unwrap();
    let source = ItemSourceLayoutPolicy::new(
        input.item_source.clone(),
        &items,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let content = empty_tree_content(&input);
    passive_socket_policy::attach(
        &mut input.normalization,
        prior.schema(),
        &mapping,
        &items,
        &source,
        &content,
    );
    empty_character_rune_policy::upgrade(&mut input.normalization, &mapping, &items, &source);
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        content.clone(),
        prior.registry(),
        prior.schema(),
        &mapping,
        &input.normalization,
        Default::default(),
    )
    .unwrap();
    let before = serde_json::to_vec(&input).unwrap();
    let result = transition_owned_catalog_with_tree(
        input.clone(),
        append.clone(),
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(tree.input().clone()),
        },
        Default::default(),
    )
    .unwrap();
    assert_ne!(
        result.assembled().schema().identity(),
        prior.schema().identity()
    );
    assert_ne!(result.items().identity(), items.identity());
    assert_ne!(result.item_source().identity(), source.identity());
    let mut expected = input.normalization.clone();
    passive_socket_policy::attach(
        &mut expected,
        result.assembled().schema(),
        result.mapping(),
        result.items(),
        result.item_source(),
        &content,
    );
    empty_character_rune_policy::upgrade(
        &mut expected,
        result.mapping(),
        result.items(),
        result.item_source(),
    );
    assert_eq!(
        result.normalization().passive_socket_membership,
        expected.passive_socket_membership
    );
    assert_eq!(
        result.normalization().equipment_membership,
        expected.equipment_membership
    );
    assert!(result.normalization().item_modifier_membership.is_none());
    assert!(result.normalization().item_parameter_inputs.is_none());
    assert_eq!(result.tree().unwrap().input().content, content);
    assert_eq!(
        result.mapping().source_identity(),
        mapping.source_identity()
    );
    assert_eq!(result.query_sets(), input.query_sets);
    assert_eq!(serde_json::to_vec(&input).unwrap(), before);
    for field in 0..4 {
        let mut stale = input.clone();
        empty_character_rune_policy::corrupt(&mut stale.normalization, field);
        let mut bad_tree = tree.input().clone();
        // Keep the outer commitment current without validating/repairing the
        // deliberately stale inner V3 source or artifact authority first.
        bad_tree.normalization = poe_optimizer_core::owned_content::digest_owned(
            "owned-normalization-policy-v3",
            &stale.normalization,
            poe_optimizer_import::owned_tree_policy::TreePolicyLimits::default()
                .max_base_policy_bytes,
        )
        .unwrap();
        assert!(
            transition_owned_catalog_with_tree(
                stale,
                append.clone(),
                TreePolicyTransitionInput::RebindPrior {
                    prior: Box::new(bad_tree)
                },
                Default::default()
            )
            .is_err(),
            "dependency {field}"
        );
    }
    assert!(
        transition_owned_catalog_with_tree(
            input.clone(),
            changed_source,
            TreePolicyTransitionInput::RebindPrior {
                prior: Box::new(tree.input().clone())
            },
            Default::default()
        )
        .is_err()
    );
    assert!(
        transition_owned_catalog_with_tree(
            input,
            append,
            TreePolicyTransitionInput::Install {
                content: Box::new(content)
            },
            Default::default()
        )
        .is_err(),
        "inherited placement needs its checked prior tree, not a replacement content assertion"
    );
}

#[test]
fn nonphysical_support_inventory_rebinds_roles_and_preserves_exact_source_declarations() {
    use poe_optimizer_core::owned_content::digest_owned;
    use poe_optimizer_import::{
        owned_mapping::OwnedMappingIndex, owned_normalize::SupportOriginOrderPolicy,
        owned_recipe::assemble_owned_recipe, owned_skill_catalog::OwnedSkillRoleIndex,
    };
    let (mut input, changed_source) = catalog_input();
    let prior = assemble_owned_recipe(input.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        prior.registry(),
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        input.roles.clone(),
        &mapping,
        prior.schema(),
        Default::default(),
    )
    .unwrap();
    assert!(input.normalization.gem_inventory.is_none());
    input.normalization.support_origin_order = Some(
        SupportOriginOrderPolicy::SavedManualGroupOrderWithNonphysicalSkillInventoryV3 {
            mapping_source: *mapping.source_identity(),
            roles: *roles.identity(),
            nonphysical_skill_ids: vec!["InjectedEffect".into()],
        },
    );
    let before = serde_json::to_vec(&input).unwrap();
    let mut append = changed_source.clone();
    append.source = input.mapping.source.clone();
    let next = transition_owned_catalog(input.clone(), append.clone(), Default::default()).unwrap();
    assert_ne!(next.roles().identity(), roles.identity());
    assert_eq!(next.mapping().source_identity(), mapping.source_identity());
    assert_eq!(
        next.normalization().support_origin_order,
        Some(
            SupportOriginOrderPolicy::SavedManualGroupOrderWithNonphysicalSkillInventoryV3 {
                mapping_source: *mapping.source_identity(),
                roles: *next.roles().identity(),
                nonphysical_skill_ids: vec!["InjectedEffect".into()],
            }
        )
    );
    assert_eq!(next.query_sets(), input.query_sets);
    assert!(next.normalization().gem_inventory.is_none());
    for field in 0..2 {
        let mut stale = input.clone();
        let Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithNonphysicalSkillInventoryV3 {
            mapping_source,
            roles,
            ..
        }) = &mut stale.normalization.support_origin_order
        else {
            unreachable!()
        };
        let invalid = digest_owned("stale-physical-support-inventory", &field, 100).unwrap();
        if field == 0 {
            *mapping_source = invalid;
        } else {
            *roles = invalid;
        }
        assert!(
            matches!(
                transition_owned_catalog(stale, append.clone(), Default::default()),
                Err(SuccessorBundleError::Normalization(_))
            ),
            "stale prior dependency {field}"
        );
    }
    assert!(
        matches!(
            transition_owned_catalog(input.clone(), changed_source, Default::default()),
            Err(SuccessorBundleError::Normalization(_))
        ),
        "changed source needs explicit renewed inventory authority"
    );
    assert_eq!(serde_json::to_vec(&input).unwrap(), before);
}
