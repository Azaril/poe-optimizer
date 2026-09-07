//! Native calculation kernels translated from pinned Path of Building PoE2 source.
//!
//! This partial port is not a build evaluator. Production code has no Lua, I/O,
//! threading, or external-crate dependencies. Inputs are already resolved numeric
//! values, validated numeric modifiers and explicit condition contexts; build legality lives outside
//! this slice. See `docs/native-engine.md` for scope and parity requirements.

#![forbid(unsafe_code)]

pub mod conditions;
pub mod defence;
pub mod modifiers;

/// PoB revision from which the currently implemented kernels were translated.
pub const UPSTREAM_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
