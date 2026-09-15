//! Portable owned rule compilation, resolution and metric evaluation.
//!
//! Disable default features to exclude the legacy Spark/Mace, source interpreter
//! and source-shaped data paths. Shared numerical primitives remain available.
//! Existing legacy consumers are transitional and are not a general evaluator.

#![forbid(unsafe_code)]

#[cfg(feature = "legacy")]
pub mod action_speed;
#[cfg(feature = "legacy")]
pub mod actor;
#[cfg(feature = "legacy")]
pub mod armour;
#[cfg(feature = "legacy")]
pub mod character;
#[cfg(feature = "legacy")]
pub mod conditions;
#[cfg(feature = "legacy")]
pub mod data;
#[cfg(feature = "legacy")]
pub mod defence;
#[cfg(feature = "legacy")]
pub mod item_format;
#[cfg(feature = "legacy")]
pub mod item_runes;
#[cfg(feature = "legacy")]
pub mod item_tools;
#[cfg(feature = "legacy")]
pub mod lua_bits;
#[cfg(feature = "legacy")]
pub mod lua_number;
#[cfg(feature = "legacy")]
pub mod lua_pattern;
#[cfg(feature = "legacy")]
pub mod modifier_parser;
#[cfg(feature = "legacy")]
pub mod modifier_scan;
#[cfg(feature = "legacy")]
pub mod parser_program;
#[cfg(feature = "legacy")]
pub mod source_program;
pub mod timing;
#[cfg(feature = "legacy")]
pub use data::CompiledGameData;
#[cfg(feature = "legacy")]
pub mod mace;
#[cfg(feature = "legacy")]
pub mod mace_supports;
#[cfg(feature = "legacy")]
pub mod modifiers;
#[cfg(feature = "legacy")]
pub mod movement;
#[cfg(feature = "legacy")]
pub mod multipliers;
#[cfg(feature = "legacy")]
mod offence;
pub mod owned_rules;
pub mod resistance;
#[cfg(feature = "legacy")]
pub mod selection_keys;
#[cfg(feature = "legacy")]
pub mod spark;
#[cfg(feature = "legacy")]
pub mod stats;
#[cfg(feature = "legacy")]
pub mod weapon;

/// PoB revision from which the currently implemented kernels were translated.
#[cfg(feature = "legacy")]
pub const UPSTREAM_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";

pub mod owned_plan;
