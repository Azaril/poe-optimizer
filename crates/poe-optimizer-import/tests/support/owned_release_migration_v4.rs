//! The permission transition reuses the checked release and provider fixtures.
use super::*;

fn header(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    let mut input = v3_header(prior);
    input.schema_version = 4;
    input.release = key("explicit-preset-input-contract");
    input.reason = key("reviewed-preset-input-migration");
    input.contract.schema_version = 6;
    input.contract.operations_version = key(OWNED_RULE_OPERATIONS_V19);
    input
}

#[test]
fn reviewed_predecessors_preserve_rules_source_inputs_and_distinct_receipts() {
    let original = prior();
    for (schema, operations) in [
        (5, OWNED_RULE_OPERATIONS_V17),
        (5, OWNED_RULE_OPERATIONS_V18),
        (6, OWNED_RULE_OPERATIONS_V19),
    ] {
        let prior = v3_fixture::contract(
            original.input().clone(),
            schema,
            operations,
            "reviewed-preset-prior",
        );
        let mut migration = header(&prior);
        if schema == 6 {
            assert!(matches!(
                compile_owned_release_migration(&prior, migration.clone(), Default::default()),
                Err(OwnedReleaseError::Invalid(
                    "migration contains no semantic changes"
                ))
            ));
            migration.contract.rule_semantics_version = key("reviewed-preset-refinement");
        }
        let authoring =
            digest_owned("owned-release-contract-migration-v4", &migration, 1_000_000).unwrap();
        let result =
            compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
        assert_eq!(result.input().recipe.schema.schema_version, 6);
        assert_eq!(
            result.input().recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V19
        );
        assert!(result.evaluation().is_none());
        assert!(
            result.input().recipe.schema.definitions == prior.input().recipe.schema.definitions
        );
        assert!(result.input().recipe.schema.slots == prior.input().recipe.schema.slots);
        assert!(result.input().recipe.registry == prior.input().recipe.registry);
        assert!(result.input().recipe.rules.owners == prior.input().recipe.rules.owners);
        assert!(result.input().recipe.rules.tables == prior.input().recipe.rules.tables);
        assert!(result.input().recipe.rules.receivers == prior.input().recipe.rules.receivers);
        let mut routing = result.input().recipe.routing.clone();
        routing.definitions = prior.receipt().definitions.clone();
        assert!(routing == prior.input().recipe.routing);
        assert_eq!(
            result.input().provenance.last().unwrap().authoring_input,
            authoring
        );
        assert_source_inputs_preserved(&prior, &result);
        let repeated =
            compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
        assert!(result.artifacts().eq(repeated.artifacts()));
        let rebuilt = assemble_owned_release(result.input().clone(), Default::default()).unwrap();
        assert!(result.artifacts().eq(rebuilt.artifacts()));
    }
}

#[test]
fn explicit_permission_refinement_rebinds_without_replacing_provider_programs() {
    let original = prior();
    let before = v3_fixture::contract(
        original.input().clone(),
        5,
        OWNED_RULE_OPERATIONS_V17,
        "provider-prior",
    );
    let mut provider = actor_migration(&before);
    provider.schema_version = 3;
    provider.contract = v3_header(&before).contract;
    for row in &mut provider.schema {
        if let SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry)) = row {
            let SchemaState::Known(schema) = &mut entry.schema else {
                unreachable!()
            };
            schema.skill_input = Some(SkillInputAuthority::Projected);
        }
    }
    let prior =
        compile_owned_release_migration(&before, provider.clone(), Default::default()).unwrap();
    let parameter = provider
        .schema
        .iter()
        .find_map(|row| match row {
            SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry)) => Some(entry.id.clone()),
            _ => None,
        })
        .unwrap();
    let mut supply = provider
        .schema
        .iter()
        .find_map(|row| match row {
            SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(entry)) => Some(entry.clone()),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(schema) = &mut supply.schema else {
        unreachable!()
    };
    assert!(schema.preset_inputs.is_none());
    schema.preset_inputs = Some(PresetSkillInputPermission {
        schema_version: PRESET_SKILL_INPUT_PERMISSION_V1,
        parameters: DeclaredSet::complete(vec![parameter]),
    });
    let mut migration = header(&prior);
    migration.schema = vec![SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(
        supply.clone(),
    ))];
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    let mut restored = result.input().recipe.schema.slots.clone();
    let changed = restored
        .iter_mut()
        .find(|row| row.address() == SkillGrantSlotDefId::address(&supply.id))
        .unwrap();
    assert!(*changed == SlotDescriptor::SkillGrant(supply.clone()));
    *changed = prior
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| row.address() == SkillGrantSlotDefId::address(&supply.id))
        .unwrap()
        .clone();
    assert!(restored == prior.input().recipe.schema.slots);
    assert!(result.input().recipe.rules.owners == prior.input().recipe.rules.owners);
    assert!(result.input().recipe.registry == prior.input().recipe.registry);
    assert_source_inputs_preserved(&prior, &result);
    let mut historical = migration;
    historical.schema_version = 3;
    assert!(compile_owned_release_migration(&prior, historical, Default::default()).is_err());
    let mut old_schema = result.input().recipe.schema.clone();
    old_schema.schema_version = 5;
    assert!(OwnedDefinitionSchemaPackage::new(old_schema, Default::default()).is_err());

    // Authorization changes are explicit same-contract schema replacements;
    // removing permission does not remove the provider's existing program.
    let mut next = header(&result);
    next.release = key("reviewed-preset-permission-removal");
    let SchemaState::Known(schema) = &mut supply.schema else {
        unreachable!()
    };
    schema.preset_inputs.as_mut().unwrap().parameters = DeclaredSet::complete(vec![]);
    next.schema = vec![SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(
        supply.clone(),
    ))];
    let updated = compile_owned_release_migration(&result, next, Default::default()).unwrap();
    assert!(
        updated
            .input()
            .recipe
            .schema
            .slots
            .contains(&SlotDescriptor::SkillGrant(supply))
    );
    assert!(updated.input().recipe.rules.owners == result.input().recipe.rules.owners);
    assert_source_inputs_preserved(&result, &updated);
}

#[test]
fn old_versions_stale_bindings_and_unreviewed_pairs_cannot_enter_or_downgrade() {
    let original = prior();
    let prior = v3_fixture::contract(
        original.input().clone(),
        5,
        OWNED_RULE_OPERATIONS_V18,
        "v18-prior",
    );
    let valid = header(&prior);
    for version in [1, 2, 3, 5] {
        let mut bad = valid.clone();
        bad.schema_version = version;
        assert!(compile_owned_release_migration(&prior, bad, Default::default()).is_err());
    }
    for (schema, operations) in [
        (5, OWNED_RULE_OPERATIONS_V18),
        (6, OWNED_RULE_OPERATIONS_V18),
        (5, OWNED_RULE_OPERATIONS_V19),
        (6, "owned-domain-operations-v20"),
    ] {
        let mut bad = valid.clone();
        bad.contract.schema_version = schema;
        bad.contract.operations_version = key(operations);
        assert!(matches!(
            compile_owned_release_migration(&prior, bad, Default::default()),
            Err(OwnedReleaseError::Invalid(
                "preset-input migration requires schema v6 and operations v19"
            ))
        ));
    }
    let mut stale = valid.clone();
    stale.before = digest_owned("stale-v4-before", &1, 100).unwrap();
    assert!(matches!(
        compile_owned_release_migration(&prior, stale, Default::default()),
        Err(OwnedReleaseError::Invalid("migration version or endpoint"))
    ));
    for (schema, operations) in [
        (4, OWNED_RULE_OPERATIONS_V17),
        (5, OWNED_RULE_OPERATIONS_V16),
        (5, OWNED_RULE_OPERATIONS_V19),
        (6, OWNED_RULE_OPERATIONS_V18),
    ] {
        let unsupported = v3_fixture::contract(
            original.input().clone(),
            schema,
            operations,
            "unreviewed-preset-prior",
        );
        assert!(matches!(
            compile_owned_release_migration(&unsupported, header(&unsupported), Default::default()),
            Err(OwnedReleaseError::Invalid(
                "migration prior contract is unsupported"
            ))
        ));
    }
    let result =
        compile_owned_release_migration(&prior, valid.clone(), Default::default()).unwrap();
    let mut downgrade = header(&result);
    downgrade.release = key("forbidden-preset-downgrade");
    downgrade.contract.schema_version = 5;
    downgrade.contract.operations_version = key(OWNED_RULE_OPERATIONS_V18);
    assert!(matches!(
        compile_owned_release_migration(&result, downgrade.clone(), Default::default()),
        Err(OwnedReleaseError::Invalid(
            "preset-input migration requires schema v6 and operations v19"
        ))
    ));
    downgrade.schema_version = 3;
    assert!(matches!(
        compile_owned_release_migration(&result, downgrade, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration prior contract is unsupported"
        ))
    ));
    for limits in [
        OwnedReleaseLimits {
            max_input_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_artifact_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_provenance_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_release_migration(&prior, valid.clone(), limits).is_err());
    }
    // A failed attempt does not consume registry identities or change the prior.
    let retry = compile_owned_release_migration(&prior, valid, Default::default()).unwrap();
    assert!(result.artifacts().eq(retry.artifacts()));
}

#[test]
fn prior_evaluation_is_preserved_and_endpoint_bindings_are_never_repaired() {
    let original = prior();
    let prior = v3_fixture::contract(
        original.input().clone(),
        5,
        OWNED_RULE_OPERATIONS_V17,
        "metrics-prior",
    );
    let mut migration = header(&prior);
    migration.evaluation = Some(endpoint_metrics(&prior, &migration));
    let result = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert!(result.evaluation().is_some());
    let mut next = header(&result);
    next.release = key("preset-next-metrics");
    next.contract.rule_semantics_version = key("explicit-next-preset-semantics");
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot drop prior evaluation artifacts"
        ))
    ));
    next.evaluation = result.input().evaluation.clone();
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    next.evaluation = Some(endpoint_metrics(&result, &next));
    let updated = compile_owned_release_migration(&result, next, Default::default()).unwrap();
    assert_source_inputs_preserved(&result, &updated);
}
