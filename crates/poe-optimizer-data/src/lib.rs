//! Portable, authenticated versioned game data. No Lua, I/O or host runtime.
#![forbid(unsafe_code)]
pub mod bundled;
pub mod class_tree;
pub mod game_data;
mod item_rules;
pub mod tree_data;
pub mod tree_projection;
use sha2::{Digest, Sha256};
/// Fingerprint the portable implementation independently of bundled content provenance.
pub fn implementation_fingerprint() -> String {
    let mut digest = Sha256::new();
    for source in [
        include_str!("lib.rs"),
        include_str!("class_tree.rs"),
        include_str!("tree_data.rs"),
        include_str!("tree_projection.rs"),
        include_str!("bundled.rs"),
        include_str!("game_data.rs"),
        include_str!("item_rules.rs"),
        include_str!("../Cargo.toml"),
        include_str!("../data/tree-source-identity.json"),
        include_str!("../data/class-tree.json"),
        include_str!("../data/class-tree.sha256"),
        include_str!("../data/class-tree-policy.json"),
    ] {
        digest.update(source.replace("\r\n", "\n"));
    }
    format!("{:x}", digest.finalize())
}
