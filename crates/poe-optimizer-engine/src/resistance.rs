//! Shared resistance arithmetic over explicit numeric inputs.
//! Source-shaped preparation remains behind the transitional legacy feature.
#[cfg(feature = "legacy")]
mod legacy;
pub mod ordinary;
#[cfg(feature = "legacy")]
pub use legacy::PlayerResistances;
#[cfg(feature = "legacy")]
pub(crate) use legacy::calculate;
