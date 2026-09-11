//! Portable, authenticated versioned game data. No Lua, I/O or host runtime.
#![forbid(unsafe_code)]
mod action_speed;
mod actor;
pub mod bundled;
pub mod class_tree;
pub mod configuration;
pub mod game_data;
pub mod item_assembly;
mod item_formatting;
pub mod item_loading;
mod item_rules;
pub mod item_scalability;
pub mod modifier_parser;
mod movement;
pub mod passive_allocation;
pub mod skill_identities;
pub mod tree_data;
pub mod tree_projection;
pub mod unique_requirements;
use sha2::{Digest, Sha256};
/// Fingerprint the portable implementation independently of bundled content provenance.
pub fn implementation_fingerprint() -> String {
    let mut digest = Sha256::new();
    for source in [
        include_str!("lib.rs"),
        include_str!("actor.rs"),
        include_str!("action_speed.rs"),
        include_str!("class_tree.rs"),
        include_str!("configuration.rs"),
        include_str!("skill_identities.rs"),
        include_str!("tree_data.rs"),
        include_str!("tree_projection.rs"),
        include_str!("bundled.rs"),
        include_str!("game_data.rs"),
        include_str!("item_rules.rs"),
        include_str!("item_assembly.rs"),
        include_str!("item_formatting.rs"),
        include_str!("item_loading.rs"),
        include_str!("item_loading/runes.rs"),
        include_str!("item_scalability.rs"),
        include_str!("modifier_parser.rs"),
        include_str!("modifier_parser/factories.rs"),
        include_str!("modifier_parser/programs.rs"),
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
