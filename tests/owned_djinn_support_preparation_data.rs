//! Reviewed preparation data binds the real release without claiming a support bundle.
#[allow(dead_code)]
#[path = "support/owned_djinn_support_preparation.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_data::owned_supports::{
    OwnedSupportPreparation, SupportStorageError, decode_support_preparation,
    encode_support_preparation,
};
use serde_json::json;
use std::{fs, path::PathBuf};

#[test]
fn preparation_packet_authenticates_its_bounded_source_facts() {
    family::check_authored();
}

#[test]
#[ignore = "requires the exact Ice intrinsic release and authenticated Djinn source reports"]
fn checked_preparation_preserves_all_five_originals_and_release_artifacts() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DJINN_SUPPORT_PREPARATION_PRIOR")
            .expect("exact checked release directory"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DJINN_SUPPORT_PREPARATION_OUTPUT")
            .expect("fresh validation output directory"),
    );
    assert!(!out.exists());
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let preparation = family::load(&prior);
    assert!(prior.evaluation().is_none());
    assert_eq!(preparation.input().supports.len(), 10);
    assert_eq!(prior.receipt().query_rows, 110);

    let canonical = encode_support_preparation(&preparation, Default::default()).unwrap();
    let decoded = decode_support_preparation(
        &canonical,
        prior.assembled().schema(),
        prior.assembled().rules(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(decoded.identity(), preparation.identity());
    assert_eq!(
        encode_support_preparation(&decoded, Default::default()).unwrap(),
        canonical
    );
    // Similar Gem IDs do not authorize mixing this artifact with another rule snapshot.
    let mut stale = preparation.input().clone();
    stale.rules = *prior.assembled().routing().identity();
    assert!(matches!(
        OwnedSupportPreparation::new(
            stale,
            prior.assembled().schema(),
            prior.assembled().rules(),
            Default::default()
        ),
        Err(SupportStorageError::Binding)
    ));

    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("preparation.json"), &canonical).unwrap();
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &prior,
        prior_path: &prior_path,
        package: &prior_path,
        out: &out,
        families: &[],
        selected_before: [113, 116, 108, 121, 11],
        selected_after: [113, 116, 108, 121, 11],
        rebind_definitions: false,
    };
    let mut originals = Vec::new();
    for case in 1..=5 {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        for prefix in ["prior-original", "original"] {
            release::normalize(
                &prior_path,
                &source,
                case,
                &out.join(format!("{prefix}-{case:02}")),
            );
        }
        originals.push(preservation::compare_original(case, &bytes, &comparison));
        assert_eq!(fs::read(&source).unwrap(), bytes);
    }
    assert_eq!(release::inventory(&prior_path), before);
    fs::write(
        out.join("validation.json"),
        serde_json::to_vec(&json!({
            "release":prior.receipt().input,
            "preparation":preparation.identity(),
            "supports":10,
            "canonical_roundtrip":true,
            "release_artifacts_unchanged":true,
            "evaluation_bundle_added":false,
            "queries":110,
            "complete_original_builds":0,
            "originals":originals
        }))
        .unwrap(),
    )
    .unwrap();
}
