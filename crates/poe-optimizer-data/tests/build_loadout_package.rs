//! Package binding is separate from standalone loadout policy semantics.
use poe_optimizer_data::game_data::{
    DataTrust, GameDataError, GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits,
    SCHEMA_VERSION, SEMANTICS_VERSION, TrustPolicy, bundled_snapshot,
};
use poe_optimizer_data::loadouts::{BUILD_LOADOUT_POLICY_SCHEMA_VERSION, BuildLoadoutPolicy};
use std::{collections::BTreeMap, sync::OnceLock};

fn reviewed() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn caller_policy(latest: &str) -> BuildLoadoutPolicy {
    BuildLoadoutPolicy {
        schema_version: BUILD_LOADOUT_POLICY_SCHEMA_VERSION,
        default_title: "Caller \u{03bb}\0title".into(),
        latest_tree_version: latest.into(),
        tree_version_display: BTreeMap::new(),
        version_prefix: "<".into(),
        version_suffix: ">\0 ".into(),
        // A malformed pattern is valid metadata until a consumer reaches it.
        single_link_pattern: "[".into(),
    }
}
fn custom(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    package.refresh_section_digests()?;
    load(&package, &LoadLimits::default())
}
fn load(package: &GameDataPackage, limits: &LoadLimits) -> Result<GameDataSnapshot, GameDataError> {
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        limits,
    )
}

#[test]
fn reviewed_package_exposes_required_versioned_loadout_operands() {
    let snapshot = reviewed();
    let package = snapshot.package();
    assert_eq!(snapshot.identity().schema_version, SCHEMA_VERSION);
    assert_eq!(snapshot.identity().semantics_version, SEMANTICS_VERSION);
    assert_eq!(snapshot.build_loadouts(), &package.build_loadouts);
    assert_eq!(
        snapshot.build_loadouts().schema_version,
        BUILD_LOADOUT_POLICY_SCHEMA_VERSION
    );
    assert_eq!(
        snapshot.build_loadouts().latest_tree_version,
        package.item_loading.policy.jewel_radius.latest_tree_version
    );
    assert!(
        package
            .manifest
            .section_sha256
            .contains_key("build_loadouts")
    );

    let mut value = serde_json::to_value(package).unwrap();
    value.as_object_mut().unwrap().remove("build_loadouts");
    let error = GameDataLoader::from_bytes(
        &serde_json::to_vec(&value).unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap_err();
    assert!(
        error.0.contains("missing field") && error.0.contains("build_loadouts"),
        "{error}"
    );
}

#[test]
fn loadout_edits_require_their_own_digest_and_change_only_that_section() {
    let original = reviewed();
    let mut package = original.package().clone();
    package
        .build_loadouts
        .default_title
        .push_str(" caller edit");
    assert_eq!(
        load(&package, &LoadLimits::default()).unwrap_err().0,
        "section SHA-256 manifest does not match actual records"
    );
    package.refresh_section_digests().unwrap();
    let changed = load(&package, &LoadLimits::default()).unwrap();
    assert_ne!(changed.identity(), original.identity());
    assert_eq!(changed.trust(), &DataTrust::CustomUnreviewed);
    assert_eq!(changed.build_loadouts(), &package.build_loadouts);
    for (section, digest) in &original.package().manifest.section_sha256 {
        if section == "build_loadouts" {
            assert_ne!(&package.manifest.section_sha256[section], digest);
        } else {
            assert_eq!(
                &package.manifest.section_sha256[section], digest,
                "{section}"
            );
        }
    }
}

#[test]
fn custom_operands_roundtrip_without_eager_display_or_pattern_lookup() {
    let mut package = reviewed().package().clone();
    let policy = caller_policy(&package.item_loading.policy.jewel_radius.latest_tree_version);
    policy.validate().unwrap();
    assert!(
        !policy
            .tree_version_display
            .contains_key(&policy.latest_tree_version)
    );
    package.build_loadouts = policy.clone();
    let snapshot = custom(package).unwrap();
    assert_eq!(snapshot.build_loadouts(), &policy);
    assert_eq!(snapshot.trust(), &DataTrust::CustomUnreviewed);
    let decoded = GameDataPackage::decode_for_authoring(
        &snapshot.package().canonical_bytes().unwrap(),
        &LoadLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded.build_loadouts, policy);
}

#[test]
fn valid_standalone_latest_version_cannot_disagree_with_packaged_startup() {
    let mut package = reviewed().package().clone();
    package.build_loadouts.latest_tree_version = "caller-different-startup".into();
    package.build_loadouts.validate().unwrap();
    assert_ne!(
        package.build_loadouts.latest_tree_version,
        package.item_loading.policy.jewel_radius.latest_tree_version
    );
    // Refreshing the digest cannot turn an inconsistent startup relationship
    // into valid data; the other tree/radius owners remain unchanged.
    assert_eq!(
        custom(package).unwrap_err().0,
        "build loadout policy belongs to a different startup tree"
    );
}

#[test]
fn package_enforces_policy_schema_and_text_bounds_even_with_relaxed_loader_limits() {
    let limits = LoadLimits {
        max_string_bytes: 8192,
        ..LoadLimits::default()
    };
    for (case, expected) in [
        (0, "unsupported build loadout policy schema 2"),
        (
            1,
            "build loadout policy text exceeds the per-string byte limit",
        ),
        (
            2,
            "build loadout policy exceeds the aggregate text byte limit",
        ),
    ] {
        let mut package = reviewed().package().clone();
        match case {
            0 => package.build_loadouts.schema_version = 2,
            1 => package.build_loadouts.default_title = "x".repeat(4097),
            2 => {
                package.build_loadouts.tree_version_display = (0..257)
                    .map(|index| (format!("caller-{index}"), "x".repeat(4096)))
                    .collect();
            }
            _ => unreachable!(),
        }
        package.refresh_section_digests().unwrap();
        assert_eq!(
            load(&package, &limits).unwrap_err().0,
            expected,
            "case {case}"
        );
    }
}
