//! Shared timing arithmetic over explicit numeric inputs.
//! Source-shaped preparation remains behind the transitional legacy feature.
#[cfg(feature = "legacy")]
mod legacy;
pub mod ordinary;
#[cfg(feature = "legacy")]
pub use legacy::{DirectActionTimingInput, DirectActionTimingOutput, calculate};
