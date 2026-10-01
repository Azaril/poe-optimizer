#[allow(dead_code)]
#[path = "configuration_resistance_fixture.rs"]
mod component;
pub use component::Fixture;
use poe_optimizer_core::owned_schema::DefinitionDescriptor;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn asset(family: &str, file: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68")
        .join(family)
        .join(file);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

pub fn fixture(finite: bool) -> Fixture {
    let prior = asset("configuration-resistance-inputs", "extension.json");
    let prior_definitions: Vec<DefinitionDescriptor> = prior["schema"]
        .as_array()
        .unwrap()
        .iter()
        .skip(1)
        .map(|row| serde_json::from_value(row["value"].clone()).unwrap())
        .collect();
    let inputs =
        serde_json::from_value(asset("configuration-rating-inputs", "native-inputs.json")).unwrap();
    Fixture::with_assets(
        inputs,
        asset("configuration-rating-inputs", "extension.json"),
        prior_definitions,
        finite,
    )
}
