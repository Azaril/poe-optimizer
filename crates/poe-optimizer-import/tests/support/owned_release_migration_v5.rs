//! Selection reads opt in without reopening historical migration contracts.
use super::*;

fn header(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    let mut migration = v3_header(prior);
    migration.schema_version = 5;
    migration.release = key("action-selection-contract");
    migration.reason = key("typed-action-selection-reads");
    migration.contract.schema_version = 6;
    migration.contract.operations_version = key(OWNED_RULE_OPERATIONS_V20);
    migration
}

#[test]
fn exact_v19_and_v20_predecessors_preserve_source_queries_and_rebuilds() {
    for operations in [OWNED_RULE_OPERATIONS_V19, OWNED_RULE_OPERATIONS_V20] {
        let prior = v3_fixture::contract(prior().input().clone(), 6, operations, "selection-prior");
        let original = prior.receipt().input;
        let mut migration = header(&prior);
        if operations == OWNED_RULE_OPERATIONS_V20 {
            assert!(matches!(
                compile_owned_release_migration(&prior, migration.clone(), Default::default()),
                Err(OwnedReleaseError::Invalid(
                    "migration contains no semantic changes"
                ))
            ));
            migration.contract.rule_semantics_version = key("explicit-selection-refinement");
        }
        let authoring =
            digest_owned("owned-release-contract-migration-v5", &migration, 1_000_000).unwrap();
        let result =
            compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
        assert_eq!(result.input().recipe.schema.schema_version, 6);
        assert_eq!(
            result.input().recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V20
        );
        assert!(
            result.input().recipe.schema.definitions == prior.input().recipe.schema.definitions
        );
        assert!(result.input().recipe.schema.slots == prior.input().recipe.schema.slots);
        assert!(result.input().recipe.registry == prior.input().recipe.registry);
        assert!(result.input().recipe.rules.owners == prior.input().recipe.rules.owners);
        assert!(result.input().recipe.rules.tables == prior.input().recipe.rules.tables);
        assert!(result.input().recipe.rules.receivers == prior.input().recipe.rules.receivers);
        assert_eq!(
            result.input().provenance.last().unwrap().authoring_input,
            authoring
        );
        assert_source_inputs_preserved(&prior, &result);
        let repeated =
            compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
        let rebuilt = assemble_owned_release(result.input().clone(), Default::default()).unwrap();
        assert!(result.artifacts().eq(repeated.artifacts()));
        assert!(result.artifacts().eq(rebuilt.artifacts()));
        assert_eq!(prior.receipt().input, original);
    }
}

#[test]
fn v5_rejects_wrong_contracts_downgrades_stale_inputs_and_exhausted_budgets() {
    let original = prior();
    let prior = v3_fixture::contract(
        original.input().clone(),
        6,
        OWNED_RULE_OPERATIONS_V19,
        "selection-prior",
    );
    let valid = header(&prior);
    for version in [1, 2, 3, 4, 6] {
        let mut bad = valid.clone();
        bad.schema_version = version;
        assert!(compile_owned_release_migration(&prior, bad, Default::default()).is_err());
    }
    for (schema, operations) in [
        (5, OWNED_RULE_OPERATIONS_V20),
        (6, OWNED_RULE_OPERATIONS_V19),
        (6, "owned-domain-operations-v21"),
    ] {
        let mut bad = valid.clone();
        bad.contract.schema_version = schema;
        bad.contract.operations_version = key(operations);
        assert!(matches!(
            compile_owned_release_migration(&prior, bad, Default::default()),
            Err(OwnedReleaseError::Invalid(
                "action-selection migration requires schema v6 and operations v20"
            ))
        ));
    }
    for (schema, operations) in [
        (5, OWNED_RULE_OPERATIONS_V19),
        (6, OWNED_RULE_OPERATIONS_V18),
    ] {
        let unsupported = v3_fixture::contract(
            original.input().clone(),
            schema,
            operations,
            "unsupported-selection-prior",
        );
        assert!(matches!(
            compile_owned_release_migration(&unsupported, header(&unsupported), Default::default()),
            Err(OwnedReleaseError::Invalid(
                "migration prior contract is unsupported"
            ))
        ));
    }
    let mut stale = valid.clone();
    stale.before = digest_owned("stale-selection-prior", &1, 100).unwrap();
    assert!(compile_owned_release_migration(&prior, stale, Default::default()).is_err());
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
    let result =
        compile_owned_release_migration(&prior, valid.clone(), Default::default()).unwrap();
    let retry = compile_owned_release_migration(&prior, valid, Default::default()).unwrap();
    assert!(result.artifacts().eq(retry.artifacts()));
    let mut downgrade = header(&result);
    downgrade.release = key("invalid-selection-downgrade");
    downgrade.contract.operations_version = key(OWNED_RULE_OPERATIONS_V19);
    assert!(
        compile_owned_release_migration(&result, downgrade.clone(), Default::default()).is_err()
    );
    downgrade.schema_version = 4;
    assert!(compile_owned_release_migration(&result, downgrade, Default::default()).is_err());
}

#[test]
fn v5_cannot_drop_or_silently_rebind_existing_evaluation_artifacts() {
    let prior = v3_fixture::contract(
        prior().input().clone(),
        6,
        OWNED_RULE_OPERATIONS_V19,
        "selection-metrics-prior",
    );
    let mut migration = header(&prior);
    migration.evaluation = Some(endpoint_metrics(&prior, &migration));
    let result = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert!(result.evaluation().is_some());
    let mut next = header(&result);
    next.release = key("next-selection-metrics");
    next.contract.rule_semantics_version = key("explicit-next-selection-semantics");
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
