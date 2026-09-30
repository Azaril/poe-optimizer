//! Release packaging commits evaluator data; it never certifies real-build parity.
#[allow(dead_code)]
#[path = "../../../tests/support/owned_evaluation_release_fixture.rs"]
mod fixture;
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey, owned_schema::SchemaState,
    owned_supports::SupportTypePredicate,
};
use poe_optimizer_import::{
    owned_release::*, owned_release_migration::*, owned_release_revision::*,
};
use std::{collections::BTreeMap, sync::OnceLock};

fn fixture() -> &'static fixture::Fixture {
    static FIXTURE: OnceLock<fixture::Fixture> = OnceLock::new();
    FIXTURE.get_or_init(fixture::fixture)
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn files(release: &StagedOwnedRelease) -> BTreeMap<String, Vec<u8>> {
    release
        .artifacts()
        .map(|(name, bytes)| (name.to_owned(), bytes.to_vec()))
        .collect()
}
fn stage(input: OwnedReleaseInput) -> StagedOwnedRelease {
    assemble_owned_release(input, Default::default()).unwrap()
}

#[test]
fn all_typed_evaluation_artifacts_and_exact_digest_group_roundtrip() {
    let release = stage(fixture().input.clone());
    let evaluation = release.evaluation().unwrap();
    let support = evaluation.support().unwrap();
    let receipt = release.receipt().evaluation.as_ref().unwrap();
    assert_eq!(receipt, &evaluation.receipt());
    assert_eq!(
        receipt.support.as_ref().unwrap().outputs,
        Some(*support.outputs().unwrap().identity())
    );
    assert_eq!(
        support.inputs().input().rules,
        *release.assembled().rules().identity()
    );
    assert_eq!(
        support.stages().input().routing,
        *release.assembled().routing().identity()
    );
    let emitted = files(&release);
    for (name, bytes) in [
        (
            "metrics.json",
            serde_json::to_vec(evaluation.metrics().input()).unwrap(),
        ),
        (
            "support-stages.json",
            serde_json::to_vec(support.stages().input()).unwrap(),
        ),
        (
            "support-preparation.json",
            serde_json::to_vec(support.preparation().input()).unwrap(),
        ),
        (
            "support-inputs.json",
            serde_json::to_vec(support.inputs().input()).unwrap(),
        ),
        (
            "support-receiving.json",
            serde_json::to_vec(support.receiving().input()).unwrap(),
        ),
        (
            "support-outputs.json",
            serde_json::to_vec(support.outputs().unwrap().input()).unwrap(),
        ),
    ] {
        assert_eq!(emitted[name], bytes);
        assert!(release.receipt().artifacts.iter().any(|a| a.file == name));
    }
    let restored = decode_owned_release(
        &serde_json::to_vec(release.input()).unwrap(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(restored.receipt(), release.receipt());
    assert_eq!(files(&restored), emitted);
}

#[test]
fn canonical_set_order_changes_do_not_rebind_or_change_the_release() {
    let original = stage(fixture().input.clone());
    let mut input = fixture().input.clone();
    let support = input.evaluation.as_mut().unwrap().support.as_mut().unwrap();
    support.stages.stages.reverse();
    support.stages.programs.members.reverse();
    support.stages.frozen_channels.reverse();
    support.preparation.types.reverse();
    support.preparation.supports.reverse();
    support.inputs.target.skill_types.reverse();
    support.inputs.target.summoner.skill_types.reverse();
    support.receiving.roles.reverse();
    support.receiving.targets.reverse();
    support.receiving.supports.reverse();
    support
        .outputs
        .as_mut()
        .unwrap()
        .final_skill_types
        .reverse();
    let canonical = stage(input);
    assert_eq!(canonical.input(), original.input());
    assert_eq!(canonical.receipt(), original.receipt());
    assert_eq!(files(&canonical), files(&original));
}

#[test]
fn every_package_binding_is_checked_without_automatic_repair() {
    let different = digest_owned("release-evaluation-mismatch", &1, 100).unwrap();
    for case in 0..8 {
        let mut input = fixture().input.clone();
        let evaluation = input.evaluation.as_mut().unwrap();
        let s = evaluation.support.as_mut().unwrap();
        match case {
            0 => evaluation.metrics.definitions.content_sha256 = "0".repeat(64),
            1 => s.stages.rules = different,
            2 => s.stages.routing = different,
            3 => s.preparation.rules = different,
            4 => s.inputs.preparation = different,
            5 => s.receiving.inputs = different,
            6 => s.outputs.as_mut().unwrap().receiving = different,
            _ => s.outputs.as_mut().unwrap().stages = different,
        }
        assert!(matches!(
            assemble_owned_release(input, Default::default()),
            Err(OwnedReleaseError::Evaluation(_))
        ));
    }
}

#[test]
fn optional_outputs_stay_optional_but_the_support_group_cannot_be_partial() {
    let mut input = fixture().input.clone();
    input
        .evaluation
        .as_mut()
        .unwrap()
        .support
        .as_mut()
        .unwrap()
        .outputs = None;
    let release = stage(input);
    assert!(
        release
            .evaluation()
            .unwrap()
            .support()
            .unwrap()
            .outputs()
            .is_none()
    );
    assert!(
        release
            .receipt()
            .evaluation
            .as_ref()
            .unwrap()
            .support
            .as_ref()
            .unwrap()
            .outputs
            .is_none()
    );
    assert!(!files(&release).contains_key("support-outputs.json"));
    assert!(
        serde_json::to_value(release.input()).unwrap()["evaluation"]["support"]
            .get("outputs")
            .is_none()
    );
    for field in ["stages", "preparation", "inputs", "receiving"] {
        let mut wire = serde_json::to_value(&fixture().input).unwrap();
        wire["evaluation"]["support"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            decode_owned_release(&serde_json::to_vec(&wire).unwrap(), Default::default()).is_err()
        );
    }
    let mut wire = serde_json::to_value(&fixture().input).unwrap();
    wire["evaluation"]
        .as_object_mut()
        .unwrap()
        .remove("metrics");
    assert!(decode_owned_release(&serde_json::to_vec(&wire).unwrap(), Default::default()).is_err());
}

#[test]
fn aggregate_nested_semantics_depth_constituent_and_output_limits_apply() {
    let input = fixture().input.clone();
    let release = stage(input.clone());
    let wire = serde_json::to_vec(&input).unwrap();
    let output_size: usize = release.artifacts().map(|(_, bytes)| bytes.len()).sum();
    for limits in [
        OwnedReleaseLimits {
            max_input_bytes: wire.len() - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_output_bytes: output_size - 1,
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
    ] {
        assert!(assemble_owned_release(input.clone(), limits).is_err());
    }
    let mut limits = OwnedReleaseLimits::default();
    limits.evaluation.metrics.max_wire_bytes = 1;
    assert!(matches!(
        assemble_owned_release(input.clone(), limits),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    let bounded = OwnedReleaseLimits {
        max_validation_entries: 10_000,
        ..Default::default()
    };
    assemble_owned_release(input.clone(), bounded).unwrap();
    let mut wide = input.clone();
    let support = wide.evaluation.as_mut().unwrap().support.as_mut().unwrap();
    let SchemaState::Known(row) = &mut support.preparation.supports[0].preparation else {
        unreachable!()
    };
    row.requires = Some(SupportTypePredicate::Any(vec![
        SupportTypePredicate::Type(
            support.preparation.types[0].clone()
        );
        10_000
    ]));
    assert!(matches!(
        assemble_owned_release(wide, bounded),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));
    let mut deep = input;
    let support = deep.evaluation.as_mut().unwrap().support.as_mut().unwrap();
    let SchemaState::Known(row) = &mut support.preparation.supports[0].preparation else {
        unreachable!()
    };
    let mut predicate = SupportTypePredicate::Type(support.preparation.types[0].clone());
    for _ in 0..33 {
        predicate = SupportTypePredicate::Not(Box::new(predicate));
    }
    row.requires = Some(predicate);
    assert!(matches!(
        assemble_owned_release(deep, Default::default()),
        Err(OwnedReleaseError::Limit("support predicate depth"))
    ));
}

#[test]
fn legacy_revision_and_migration_cannot_silently_rebind_evaluation_artifacts() {
    let prior = stage(fixture().input.clone());
    let revision = OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key("changed-release"),
        reason: key("test"),
        definitions: vec![],
        slots: vec![],
    };
    assert!(matches!(
        compile_owned_release_revision(&prior, revision, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "schema revision needs an explicit evaluation-artifact migration"
        ))
    ));
    let migration = OwnedReleaseMigrationInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key("changed-release"),
        reason: key("test"),
        contract: OwnedReleaseContractMigration {
            schema_version: 3,
            schema_semantics_version: key("test"),
            operations_version: key("owned-domain-operations-v11"),
            rule_semantics_version: key("test"),
        },
        schema: vec![],
        tables: vec![],
        owners: vec![],
        receivers: vec![],
        query_targets: vec![],
        evaluation: None,
    };
    assert!(matches!(
        compile_owned_release_migration(&prior, migration, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "contract migration needs an explicit evaluation-artifact migration"
        ))
    ));
}

fn endpoint() -> &'static fixture::Fixture {
    static FIXTURE: OnceLock<fixture::Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| fixture::fixture_with_contract("migrated-evaluation", "migrated-rules"))
}

fn evaluation_migration(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    let target = &endpoint().input;
    OwnedReleaseMigrationInput {
        schema_version: 2,
        before: prior.receipt().input,
        release: target.recipe.schema.release.clone(),
        reason: key("explicit-evaluation-migration"),
        contract: OwnedReleaseContractMigration {
            schema_version: target.recipe.schema.schema_version,
            schema_semantics_version: target.recipe.schema.semantics_version.clone(),
            operations_version: target.recipe.rules.operations_version.clone(),
            rule_semantics_version: target.recipe.rules.semantics_version.clone(),
        },
        schema: vec![],
        tables: vec![],
        owners: vec![],
        receivers: vec![],
        query_targets: vec![],
        evaluation: target.evaluation.clone(),
    }
}

#[test]
fn explicit_v2_replacement_preserves_prior_and_uses_only_authored_endpoint_packages() {
    let prior = stage(fixture().input.clone());
    let old_files = files(&prior);
    let old_receipt = prior.receipt().clone();
    let migration = evaluation_migration(&prior);
    let expected = migration.evaluation.clone().unwrap();
    let v2_authoring =
        digest_owned("owned-release-contract-migration-v2", &migration, 1_000_000).unwrap();
    assert_ne!(
        v2_authoring,
        digest_owned("owned-release-contract-migration-v1", &migration, 1_000_000).unwrap()
    );
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    assert_eq!(result.input().evaluation.as_ref(), Some(&expected));
    assert_eq!(result.evaluation().unwrap().input(), &expected);
    assert_eq!(
        result.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(
        result.input().recipe.schema.definitions,
        prior.input().recipe.schema.definitions
    );
    assert_eq!(
        result.input().recipe.schema.slots,
        prior.input().recipe.schema.slots
    );
    assert_eq!(
        result.input().recipe.rules.owners,
        prior.input().recipe.rules.owners
    );
    assert_eq!(
        result.input().recipe.rules.tables,
        prior.input().recipe.rules.tables
    );
    assert_eq!(
        result.input().recipe.rules.receivers,
        prior.input().recipe.rules.receivers
    );
    assert_eq!(result.query_sets(), prior.query_sets());
    assert_eq!(result.receipt().source, prior.receipt().source);
    assert_eq!(
        &result.input().provenance[..prior.input().provenance.len()],
        &prior.input().provenance
    );
    assert_eq!(
        result.input().provenance.last().unwrap(),
        &OwnedReleaseProvenance {
            kind: migration.reason,
            prior_input: old_receipt.input,
            authoring_input: v2_authoring,
        }
    );
    assert_eq!(prior.receipt(), &old_receipt);
    assert_eq!(files(&prior), old_files);
    let rebuilt = stage(result.input().clone());
    assert_eq!(files(&rebuilt), files(&result));
}

#[test]
fn v2_attachment_is_a_semantic_change_even_without_changing_schema_or_rule_contracts() {
    // Use a different release header, but retain the same actual contracts.
    let mut original = fixture().input.clone();
    original.schema_version = 1;
    original.evaluation = None;
    let prior = stage(original);
    let mut migration = evaluation_migration(&prior);
    migration.contract.rule_semantics_version =
        prior.input().recipe.rules.semantics_version.clone();
    let no_semantic_change = fixture::fixture_with_contract(
        migration.release.as_str(),
        prior.input().recipe.rules.semantics_version.as_str(),
    );
    migration.evaluation = no_semantic_change.input.evaluation;
    assert_eq!(
        migration.contract.schema_version,
        prior.input().recipe.schema.schema_version
    );
    assert_eq!(
        migration.contract.schema_semantics_version,
        prior.input().recipe.schema.semantics_version
    );
    assert_eq!(
        migration.contract.operations_version,
        prior.input().recipe.rules.operations_version
    );
    let result = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert_eq!(result.input().schema_version, 2);
    assert!(
        result
            .evaluation()
            .unwrap()
            .support()
            .unwrap()
            .outputs()
            .is_some()
    );
    assert!(prior.evaluation().is_none());
}

#[test]
fn migration_rejects_missing_or_stale_evaluation_and_unreviewed_contracts() {
    let prior = stage(fixture().input.clone());
    let valid = evaluation_migration(&prior);
    let mut cases = vec![];
    let mut bad = valid.clone();
    bad.evaluation = None;
    cases.push(bad);
    let mut bad = valid.clone();
    bad.evaluation = prior.input().evaluation.clone();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.schema_version = 1;
    cases.push(bad);
    let mut bad = valid.clone();
    bad.schema_version = 3;
    cases.push(bad);
    for schema_version in [2, 3, 5] {
        let mut bad = valid.clone();
        bad.contract.schema_version = schema_version;
        cases.push(bad);
    }
    for version in [
        "owned-domain-operations-v11",
        "owned-domain-operations-v12",
        "owned-domain-operations-v14",
    ] {
        let mut bad = valid.clone();
        bad.contract.operations_version = key(version);
        cases.push(bad);
    }
    let mut bad = valid.clone();
    bad.evaluation
        .as_mut()
        .unwrap()
        .support
        .as_mut()
        .unwrap()
        .outputs
        .as_mut()
        .unwrap()
        .receiving = digest_owned("stale-receiving", &1, 100).unwrap();
    cases.push(bad);
    for (case, bad) in cases.into_iter().enumerate() {
        assert!(
            compile_owned_release_migration(&prior, bad, Default::default()).is_err(),
            "case {case}"
        );
    }
    for field in ["stages", "preparation", "inputs", "receiving"] {
        let mut wire = serde_json::to_value(&valid).unwrap();
        wire["evaluation"]["support"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<OwnedReleaseMigrationInput>(wire).is_err());
    }
    // A v1 predecessor does not make a v1 document containing new artifacts legal.
    let mut input = prior.input().clone();
    input.schema_version = 1;
    input.evaluation = None;
    let legacy = stage(input);
    let mut bad = evaluation_migration(&legacy);
    bad.schema_version = 1;
    assert!(matches!(
        compile_owned_release_migration(&legacy, bad, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration version or evaluation group"
        ))
    ));
}

#[test]
fn migration_preflights_both_evaluation_groups_and_depth_before_digest_or_copy() {
    let prior = stage(fixture().input.clone());
    let valid = evaluation_migration(&prior);
    let mut legacy_input = prior.input().clone();
    legacy_input.schema_version = 1;
    legacy_input.evaluation = None;
    let legacy = stage(legacy_input);
    let legacy_migration = evaluation_migration(&legacy);
    // Find the full public-boundary budget for attaching to the same predecessor
    // without an old evaluation group. Replacing one must charge that group too.
    let (mut low, mut high) = (1, 10_000);
    compile_owned_release_migration(
        &legacy,
        legacy_migration.clone(),
        OwnedReleaseLimits {
            max_validation_entries: high,
            ..Default::default()
        },
    )
    .unwrap();
    while low < high {
        let middle = low + (high - low) / 2;
        match compile_owned_release_migration(
            &legacy,
            legacy_migration.clone(),
            OwnedReleaseLimits {
                max_validation_entries: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(OwnedReleaseError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected migration error: {error}"),
        }
    }
    assert!(matches!(
        compile_owned_release_migration(
            &prior,
            valid.clone(),
            OwnedReleaseLimits {
                max_validation_entries: low,
                ..Default::default()
            }
        ),
        Err(OwnedReleaseError::Limit(
            "migration prior and authoring entries"
        ))
    ));
    let mut deep = valid.clone();
    let support = deep.evaluation.as_mut().unwrap().support.as_mut().unwrap();
    let SchemaState::Known(row) = &mut support.preparation.supports[0].preparation else {
        unreachable!()
    };
    let mut predicate = SupportTypePredicate::Type(support.preparation.types[0].clone());
    for _ in 0..33 {
        predicate = SupportTypePredicate::Not(Box::new(predicate));
    }
    row.requires = Some(predicate);
    // The deliberately tiny digest limit must not mask recursive preflight.
    assert!(matches!(
        compile_owned_release_migration(
            &prior,
            deep,
            OwnedReleaseLimits {
                max_artifact_bytes: 1,
                ..Default::default()
            }
        ),
        Err(OwnedReleaseError::Limit("support predicate depth"))
    ));
    let mut wide = valid.clone();
    let support = wide.evaluation.as_mut().unwrap().support.as_mut().unwrap();
    let SchemaState::Known(row) = &mut support.preparation.supports[0].preparation else {
        unreachable!()
    };
    row.requires = Some(SupportTypePredicate::Any(vec![
        SupportTypePredicate::Type(
            support.preparation.types[0].clone()
        );
        10_000
    ]));
    assert!(matches!(
        compile_owned_release_migration(
            &prior,
            wide,
            OwnedReleaseLimits {
                max_validation_entries: 10_000,
                max_artifact_bytes: 1,
                ..Default::default()
            }
        ),
        Err(OwnedReleaseError::Limit("validation entries"))
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
            max_output_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_release_migration(&prior, valid.clone(), limits).is_err());
    }
}
