//! Exact raw input normalization; numerical input preparation remains separate.
#[path = "support/owned_effective_gem_raw_release.rs"]
mod release;
use poe_optimizer_core::{build_identity::BuildLineage, owned_build::ParameterValue};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{GemInputRule, NormalizationPolicy},
    owned_source::SourceAttributeRef,
    owned_value_policy::*,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/effective-gem-raw-inputs")
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn rule<'a>(p: &'a NormalizationPolicy, key: &str) -> &'a GemInputRule {
    p.gem_inputs
        .as_ref()
        .unwrap()
        .gems
        .iter()
        .find(|v| v.gem.key().as_str() == key)
        .unwrap()
}
fn command(name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg(name);
    c
}

#[test]
fn authored_two_delta_recipes_reuse_existing_codec_and_keep_corruption_guard() {
    let p: NormalizationPolicy = read(data().join("normalization.json"));
    let cleric = rule(&p, "def.0000000000000916");
    let template = cleric
        .parameters
        .iter()
        .find(|v| v.slot.slot.key().as_str() == "def.0000000000003072")
        .unwrap();
    for (gem, slot) in [
        ("def.000000000000000a", "def.00000000000030a9"),
        ("def.0000000000000011", "def.00000000000030aa"),
    ] {
        let row = rule(&p, gem);
        assert_eq!(row.parameters.len(), 1);
        assert_eq!(row.parameters[0].slot.slot.key().as_str(), slot);
        assert_eq!(row.parameters[0].value, template.value);
        assert_eq!(row.guards.len(), 1);
        assert_eq!(row.guards[0].attribute, "corrupted");
        assert_eq!(
            serde_json::to_value(&row.guards[0].allowed).unwrap(),
            serde_json::json!([{"kind":"text","value":"false"},{"kind":"text","value":"nil"}])
        );
    }
}

#[test]
fn injected_count_codec_preserves_fractional_values_and_rejects_implicit_defaults() {
    let p: NormalizationPolicy = read(data().join("normalization.json"));
    let raw = &rule(&p, "def.000000000000000a").parameters[0].value;
    let recipe = ValueRecipe::new(raw.clone(), Default::default()).unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(b"<PathOfBuilding2><Gem corruptLevel=\"0\"/></PathOfBuilding2>").unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let row = source
        .occurrences()
        .iter()
        .find(|v| v.name() == "Gem")
        .unwrap();
    let origin = SourceAttributeRef {
        occurrence: row.id(),
        index: 0,
    };
    let selector = &raw.tiers[0].selectors[0];
    for (token, expected) in [
        ("0", 0.),
        ("nil", 0.),
        ("2", 2.),
        ("-3", -3.),
        ("0.25", 0.25),
        ("2.5e1", 25.),
        ("1.7976931348623157e308", f64::MAX),
    ] {
        let decision = recipe
            .decide(&[ValueCandidate {
                selector,
                origin,
                value: CandidateValue::Decoded(token),
            }])
            .unwrap();
        let ValueOutcome::Selected {
            origin: actual,
            value: ParameterValue::Quantity(q),
        } = decision.outcome
        else {
            panic!("must select exact {token}");
        };
        assert_eq!(actual, origin);
        assert_eq!(q.value(), expected);
        assert_eq!(q.unit().key().as_str(), "def.000000000000295a");
    }
    for token in ["1x", "NaN", "1e309", "-1e309", " 1", "nil "] {
        assert!(
            matches!(
                recipe
                    .decide(&[ValueCandidate {
                        selector,
                        origin,
                        value: CandidateValue::Decoded(token)
                    }])
                    .unwrap()
                    .outcome,
                ValueOutcome::Pending { .. }
            ),
            "{token}"
        );
    }
    assert!(matches!(
        recipe.decide(&[]).unwrap().outcome,
        ValueOutcome::Pending { .. }
    ));
}

#[test]
#[ignore = "requires the explicit checked real effective-input predecessor"]
fn real_raw_input_successor_preserves_release_and_normalizes_originals_and_probes() {
    let prior = std::env::var_os("POE_OPTIMIZER_TEST_EFFECTIVE_GEM_RAW_PRIOR")
        .expect("explicit predecessor required");
    release::check(&PathBuf::from(prior));
}
