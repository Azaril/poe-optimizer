//! Portable, authenticated versioned game data. No Lua, I/O or host runtime.
#![forbid(unsafe_code)]
pub mod bundled;
pub mod tree_data;
pub mod tree_projection;
use sha2::{Digest, Sha256};
/// Fingerprint the portable implementation independently of bundled content provenance.
pub fn implementation_fingerprint() -> String {
    let mut digest = Sha256::new();
    for source in [
        include_str!("lib.rs"),
        include_str!("tree_data.rs"),
        include_str!("tree_projection.rs"),
        include_str!("bundled.rs"),
        include_str!("../Cargo.toml"),
        include_str!("../data/tree-source-identity.json"),
        include_str!("../data/class-tree.json"),
        include_str!("../data/class-tree.sha256"),
    ] {
        digest.update(source.replace("\r\n", "\n"));
    }
    format!("{:x}", digest.finalize())
}
