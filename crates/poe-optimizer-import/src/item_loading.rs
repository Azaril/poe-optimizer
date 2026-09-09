//! Ordered native item loading, with explicit parser and assembly dependencies.
//!
//! A completed source-loading trace is not item legality, native calculation
//! admission, or an equipment-selection certificate. Missing dependencies stop
//! execution before any dependent state is invented.
mod machine;
mod parser;
mod provider;
mod report;
mod syntax;
mod variants;
pub use machine::*;
pub use parser::*;
pub use provider::*;
pub use report::*;
use sha2::{Digest, Sha256};
pub use syntax::{ItemNumber, spec_to_number};
pub use variants::{LineSelection, VariantState};
/// Identity of this diagnostic implementation, independent of injected data.
/// Normalize checkout newlines, matching the other source implementation hashes.
pub fn implementation_fingerprint() -> String {
    let mut hash = Sha256::new();
    for source in [
        include_str!("item_loading.rs"),
        include_str!("item_loading/syntax.rs"),
        include_str!("item_loading/variants.rs"),
        include_str!("item_loading/machine.rs"),
        include_str!("item_loading/report.rs"),
        include_str!("item_loading/provider.rs"),
        include_str!("item_loading/parser.rs"),
    ] {
        hash.update(source.replace("\r\n", "\n").as_bytes());
    }
    for source in poe_optimizer_engine::item_tools::implementation_sources() {
        hash.update(source.replace("\r\n", "\n").as_bytes());
    }
    for source in poe_optimizer_engine::modifier_parser::implementation_sources() {
        hash.update(source.replace("\r\n", "\n").as_bytes());
    }
    format!("{:x}", hash.finalize())
}
