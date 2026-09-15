//! Portable owned game packages; optional legacy source packages are feature-gated.
#![forbid(unsafe_code)]
#[cfg(feature = "legacy")]
mod action_speed;
#[cfg(feature = "legacy")]
mod actor;
#[cfg(feature = "legacy")]
pub mod bundled;
#[cfg(feature = "legacy")]
pub mod class_tree;
#[cfg(feature = "legacy")]
pub mod configuration;
#[cfg(feature = "legacy")]
pub mod game_data;
#[cfg(feature = "legacy")]
pub mod item_assembly;
#[cfg(feature = "legacy")]
mod item_formatting;
#[cfg(feature = "legacy")]
pub mod item_loading;
#[cfg(feature = "legacy")]
mod item_rules;
#[cfg(feature = "legacy")]
pub mod item_scalability;
#[cfg(feature = "legacy")]
pub mod loadouts;
#[cfg(feature = "legacy")]
pub mod modifier_parser;
#[cfg(feature = "legacy")]
mod movement;
pub mod owned_allocations;
pub mod owned_metrics;
pub mod owned_routing;
pub mod owned_rules;
pub mod owned_schema;
#[cfg(feature = "legacy")]
pub mod passive_allocation;
#[cfg(feature = "legacy")]
pub mod skill_identities;
#[cfg(feature = "legacy")]
pub mod skill_preparation;
#[cfg(feature = "legacy")]
pub mod source_program;
#[cfg(feature = "legacy")]
pub mod tree_data;
#[cfg(feature = "legacy")]
pub mod tree_projection;
#[cfg(feature = "legacy")]
pub mod unique_requirements;
#[cfg(feature = "legacy")]
use sha2::{Digest, Sha256};
/// Fingerprint the portable implementation independently of bundled content provenance.
#[cfg(feature = "legacy")]
pub fn implementation_fingerprint() -> String {
    let mut digest = Sha256::new();
    for source in [
        include_str!("lib.rs"),
        include_str!("actor.rs"),
        include_str!("action_speed.rs"),
        include_str!("class_tree.rs"),
        include_str!("configuration.rs"),
        include_str!("loadouts.rs"),
        include_str!("skill_identities.rs"),
        include_str!("skill_preparation.rs"),
        include_str!("tree_data.rs"),
        include_str!("tree_projection.rs"),
        include_str!("bundled.rs"),
        include_str!("game_data.rs"),
        include_str!("item_rules.rs"),
        include_str!("item_assembly.rs"),
        include_str!("item_assembly/local.rs"),
        include_str!("item_assembly/weapon.rs"),
        include_str!("item_assembly/jewel.rs"),
        include_str!("item_assembly/slot_validity.rs"),
        include_str!("item_assembly/inventory.rs"),
        include_str!("item_assembly/inventory_activation.rs"),
        include_str!("item_assembly/inventory_nodes.rs"),
        include_str!("item_formatting.rs"),
        include_str!("item_loading.rs"),
        include_str!("item_loading/runes.rs"),
        include_str!("item_loading/radius.rs"),
        include_str!("item_loading/stat_ordering.rs"),
        include_str!("item_scalability.rs"),
        include_str!("modifier_parser.rs"),
        include_str!("source_program.rs"),
        include_str!("source_program/graph.rs"),
        include_str!("source_program/classes.rs"),
        include_str!("source_program/closures.rs"),
        include_str!("source_program/closure_creations.rs"),
        include_str!("source_program/constructors.rs"),
        include_str!("source_program/constructors/walk.rs"),
        include_str!("source_program/context.rs"),
        include_str!("source_program/iteration.rs"),
        include_str!("source_program/session.rs"),
        include_str!("source_program/session/traversal.rs"),
        include_str!("modifier_parser/factories.rs"),
        include_str!("modifier_parser/programs.rs"),
        include_str!("modifier_parser/programs/payload.rs"),
        include_str!("modifier_parser/programs/validate.rs"),
        include_str!("unique_requirements.rs"),
        include_str!("movement.rs"),
        include_str!("passive_allocation.rs"),
        include_str!("../Cargo.toml"),
        include_str!("../../../Cargo.toml"),
        include_str!("../data/tree-source-identity.json"),
        include_str!("../data/class-tree.json"),
        include_str!("../data/class-tree.sha256"),
        include_str!("../data/class-tree-policy.json"),
    ] {
        digest.update(source.replace("\r\n", "\n"));
    }
    format!("{:x}", digest.finalize())
}
