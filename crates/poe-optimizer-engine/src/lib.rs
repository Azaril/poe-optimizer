//! Native calculation kernels translated from pinned Path of Building PoE2 source.
//!
//! This partial port includes closed Spark/Mace profiles, not a general build evaluator.
//! Production code has no Lua, I/O,
//! threading, or host-runtime dependencies. Immutable game data is injected explicitly. Inputs are already resolved numeric
//! values, validated numeric modifiers and explicit condition contexts; build legality lives outside
//! this slice. See `docs/native-engine.md` for scope and parity requirements.

#![forbid(unsafe_code)]

pub mod action_speed;
pub mod actor;
pub mod armour;
pub mod character;
pub mod conditions;
pub mod data;
pub mod defence;
pub mod item_format;
pub mod item_runes;
pub mod item_tools;
pub mod lua_bits;
pub mod lua_number;
pub mod lua_pattern;
pub mod modifier_parser;
pub mod modifier_scan;
pub mod parser_program;
pub mod source_program;
pub mod timing;
pub use data::CompiledGameData;
pub mod mace;
pub mod mace_supports;
pub mod modifiers;
pub mod movement;
pub mod multipliers;
mod offence;
pub mod resistance;
pub mod selection_keys;
pub mod spark;
pub mod stats;
pub mod weapon;

/// PoB revision from which the currently implemented kernels were translated.
pub const UPSTREAM_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
