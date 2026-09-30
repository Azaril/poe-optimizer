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
    };
    assert!(matches!(
        compile_owned_release_migration(&prior, migration, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "contract migration needs an explicit evaluation-artifact migration"
        ))
    ));
}
