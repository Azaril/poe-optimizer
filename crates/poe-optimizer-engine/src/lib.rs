//! Native calculation kernels translated from pinned Path of Building PoE2 source.
//!
//! This partial port includes closed Spark/Mace profiles, not a general build evaluator.
//! Production code has no Lua, I/O,
//! threading, or host-runtime dependencies. Immutable game data is injected explicitly. Inputs are already resolved numeric
//! values, validated numeric modifiers and explicit condition contexts; build legality lives outside
//! this slice. See `docs/native-engine.md` for scope and parity requirements.

#![forbid(unsafe_code)]

pub mod character;
pub mod conditions;
pub mod data;
pub mod defence;
pub use data::CompiledGameData;
pub mod mace;
pub mod modifiers;
pub mod multipliers;
pub mod spark;
pub mod stats;

/// PoB revision from which the currently implemented kernels were translated.
pub const UPSTREAM_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
