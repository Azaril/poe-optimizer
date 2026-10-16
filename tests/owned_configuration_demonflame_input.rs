//! Source-bound Player transformation count, with no activation or buff-effect admission.
#[allow(dead_code)]
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[allow(dead_code)]
#[path = "support/owned_configuration_demonflame_input.rs"]
mod demonflame;
#[path = "support/owned_configuration_demonflame_publication.rs"]
mod demonflame_publication;
#[path = "support/owned_configuration_default_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;

#[test]
fn demonflame_is_a_player_raw_count_with_both_lane_receipts() {
    demonflame::check_authored();
}
#[test]
fn larger_offline_catalogue_allowance_is_path_scoped_and_bounded() {
    let proof = demonflame::read::<serde_json::Value>("source-evidence.json");
    let mut archive = proof["archives"][2].clone();
    let ordinary = 1024 * 1024;
    let catalogue = 2 * ordinary;
    assert_eq!(
        configuration_sources::archive_limit("src/Data/Skills/other.lua"),
        catalogue
    );
    for path in [
        "src/Modules/ConfigOptions.lua",
        "src/Data/Skills/unknown.lua",
    ] {
        assert_eq!(configuration_sources::archive_limit(path), ordinary);
    }
    // The authentic full catalogue is too large for the ordinary allowance.
    assert!(archive["bytes"].as_u64().unwrap() > ordinary as u64);
    assert!(
        std::panic::catch_unwind(|| configuration_sources::retained_gzip(&archive, ordinary))
            .is_err()
    );
    // A coherently identified archive cannot request an unbounded expansion.
    archive["bytes"] = json!(catalogue + 1);
    assert!(
        std::panic::catch_unwind(|| configuration_sources::retained_gzip(&archive, catalogue))
            .is_err()
    );
}
#[test]
fn retained_archive_locators_are_portable_without_changing_evidence() {
    let proof = demonflame::read::<serde_json::Value>("source-evidence.json");
    let original = &proof["archives"][2];
    let locator = original["file"].as_str().unwrap();
    assert!(locator.contains('\\'));
    let portable = locator.replace('\\', "/");
    assert_eq!(
        configuration_sources::repository_source_file(locator),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&portable)
    );
    let mut comparison = original.clone();
    comparison["file"] = json!(portable);
    let limit = configuration_sources::archive_limit("src/Data/Skills/other.lua");
    let original_bytes = configuration_sources::retained_gzip(original, limit);
    let portable_bytes = configuration_sources::retained_gzip(&comparison, limit);
    assert!(
        original_bytes == portable_bytes,
        "same independently sealed complete source bytes"
    );
    assert_eq!(original_bytes.len(), 1_172_527);
    comparison["file"] = original["file"].clone();
    assert_eq!(
        comparison, *original,
        "locator handling changes no source commitment"
    );
    for invalid in [
        "",
        "/archive.gz",
        "C:\\archive.gz",
        "../archive.gz",
        "data//archive.gz",
        "data\\..\\archive.gz",
    ] {
        assert!(
            std::panic::catch_unwind(|| configuration_sources::repository_source_file(invalid))
                .is_err()
        );
    }
}
#[test]
fn source_identity_and_effect_claim_mutations_are_rejected() {
    let original = demonflame::read::<serde_json::Value>("source-evidence.json");
    for mutation in 0..6 {
        let mut e = original.clone();
        match mutation {
            0 => e["spans"][0]["body"] = json!("wrong"),
            1 => e["archives"][0]["path"] = json!("src/Modules/CalcSetup.lua"),
            2 => e["constructor_dependencies"][0]["sha256"] = json!("wrong"),
            3 => e["scope"]["target"] = json!("enemy"),
            4 => e["scope"]["native_effect"] = json!(true),
            _ => e["scope"]["zero_candidate_is_emission"] = json!(true),
        }
        assert!(std::panic::catch_unwind(|| demonflame::authenticate(&e)).is_err());
    }
}
#[test]
fn constructor_callback_and_player_record_are_independently_checked() {
    let original = demonflame::observation();
    demonflame::validate_observation(&original);
    for mutation in 0..9 {
        let mut o = original.clone();
        match mutation {
            0 => o["constructors"][0]["default_state"] = json!(20),
            1 => o["constructors"][0]["placeholder"] = json!(0),
            2 => o["calls"][0]["body"]["first_line"] = json!(1908),
            3 => o["calls"][0]["argument_fields"]["val"] = json!(0),
            4 => {
                o["calls"].as_array_mut().unwrap().pop();
            }
            5 => {
                o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            6 => {
                let record = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop()
                    .unwrap();
                o["calls"][0]["after"]["enemy_records"] = json!([record]);
            }
            7 => {
                o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap()["body"]["value"] = json!(20)
            }
            _ => {
                o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap()["body"]["_positions"] = json!([])
            }
        }
        assert!(std::panic::catch_unwind(|| demonflame::validate_observation(&o)).is_err());
    }
}
#[test]
#[ignore = "requires authenticated prior and fresh output; no source VM"]
fn publish_demonflame_input_preserving_all_five_originals() {
    demonflame_publication::run();
}
