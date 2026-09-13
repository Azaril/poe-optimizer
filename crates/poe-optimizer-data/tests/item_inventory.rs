use poe_optimizer_data::game_data::{
    GameDataError, GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits, TrustPolicy,
    bundled_snapshot,
};
use std::sync::OnceLock;

const TREE_BINDING_ERROR: &str = "inventory socket projection belongs to a different full tree";

fn reviewed() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}

fn load_custom(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    // Refresh the changed section so the negative cases exercise the tree
    // binding, rather than a stale section digest or reviewed-byte trust check.
    package.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}

#[test]
fn inventory_socket_projection_matches_the_reviewed_and_custom_package_tree() {
    let package = reviewed().package();
    let nodes = &package.item_assembly.policy.inventory.layout.passive.nodes;
    assert_eq!(nodes.tree_version, package.tree.source.tree_version);
    assert_eq!(
        nodes.full_snapshot_sha256,
        package.tree.full_snapshot_sha256
    );

    let loaded = load_custom(package.clone()).unwrap();
    assert_eq!(
        loaded
            .item_assembly()
            .policy()
            .inventory
            .layout
            .passive
            .nodes,
        *nodes
    );
}

#[test]
fn inventory_socket_projection_rejects_a_different_full_tree_version() {
    let mut package = reviewed().package().clone();
    let version = &mut package
        .item_assembly
        .policy
        .inventory
        .layout
        .passive
        .nodes
        .tree_version;
    version.push_str("-different");
    assert_ne!(*version, package.tree.source.tree_version);
    // The standalone injected policy remains well formed; only its relation to
    // the enclosing package tree is wrong.
    package.item_assembly.policy.inventory.validate().unwrap();

    assert_eq!(load_custom(package).unwrap_err().0, TREE_BINDING_ERROR);
}

#[test]
fn inventory_socket_projection_rejects_a_different_full_tree_digest() {
    let mut package = reviewed().package().clone();
    let digest = &mut package
        .item_assembly
        .policy
        .inventory
        .layout
        .passive
        .nodes
        .full_snapshot_sha256;
    // Keep the digest syntactically valid and change no IDs or tree fields.
    let replacement = if digest.starts_with('0') { "1" } else { "0" };
    digest.replace_range(..1, replacement);
    assert_ne!(*digest, package.tree.full_snapshot_sha256);
    package.item_assembly.policy.inventory.validate().unwrap();

    assert_eq!(load_custom(package).unwrap_err().0, TREE_BINDING_ERROR);
}

#[test]
fn inventory_socket_projection_rejects_a_different_startup_tree() {
    let mut package = reviewed().package().clone();
    assert_ne!(
        package.item_loading.policy.jewel_radius.latest_tree_version,
        "0_1"
    );
    package.item_loading.policy.jewel_radius.latest_tree_version = "0_1".into();
    // An independent item-loading catalog can use this valid startup version.
    // The complete package must also agree with its constructor's tree.
    package.item_loading.validate().unwrap();

    assert_eq!(
        load_custom(package).unwrap_err().0,
        "inventory socket projection belongs to a different startup tree"
    );
}
