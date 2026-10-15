//! One publication with disjoint per-source expectations and exact inverses.
use crate::{activity as family, publication};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_source::SourceProjectEvidence,
};
use publication::{
    Control, Expectations,
    LeafRole::{Contributes as C, UnselectedProjection as U},
    Output, SourceExpectations,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};

fn expectations() -> Expectations {
    let rows = [
        (
            "changed-placeholder",
            None,
            Some("80"),
            Some(80.),
            [None, Some(C)],
        ),
        ("zero", Some("0"), Some("80"), Some(80.), [Some(U), Some(C)]),
        (
            "negative-zero",
            Some("-0"),
            Some("80"),
            Some(80.),
            [Some(U), Some(C)],
        ),
        (
            "fraction",
            Some("-12.5"),
            Some("1"),
            Some(-12.5),
            [Some(C), Some(U)],
        ),
        (
            "above-callback-clamp",
            Some("150"),
            Some("1"),
            Some(150.),
            [Some(C), Some(U)],
        ),
        (
            "fractional-placeholder",
            None,
            Some("0.5"),
            Some(0.5),
            [None, Some(C)],
        ),
        (
            "zero-placeholder",
            None,
            Some("0"),
            Some(0.),
            [None, Some(C)],
        ),
        (
            "both-zero",
            Some("0"),
            Some("0"),
            Some(0.),
            [Some(U), Some(C)],
        ),
        (
            "zero-constructor",
            Some("0"),
            None,
            Some(1.),
            [Some(U), None],
        ),
        ("absent", None, None, Some(1.), [None, None]),
        (
            "boundary",
            Some("1000000"),
            Some("-1000000"),
            Some(1000000.),
            [Some(C), Some(U)],
        ),
        (
            "negative-boundary",
            None,
            Some("-1000000"),
            Some(-1000000.),
            [None, Some(C)],
        ),
        (
            "bad-placeholder",
            Some("0"),
            Some("NaN"),
            None,
            [None, None],
        ),
        ("bad-input", Some("NaN"), Some("1"), None, [None, None]),
        (
            "losing-outside-range",
            Some("50"),
            Some("1000001"),
            None,
            [None, None],
        ),
        (
            "losing-malformed-placeholder",
            Some("50"),
            Some("NaN"),
            None,
            [None, None],
        ),
        (
            "input-outside-range",
            Some("-1000001"),
            Some("1"),
            None,
            [None, None],
        ),
    ];
    Expectations {
        outputs: vec![Output {
            policy_target: "player",
            draft_target: json!({"kind":"actor","value":{"kind":"player"}}),
            selection: Some("nonzero_input_then_placeholder"),
            ignore_placeholder: false,
        }],
        original_placeholder: C,
        default: 1.,
        controls: rows
            .into_iter()
            .map(|(name, input, placeholder, value, roles)| Control {
                name,
                input,
                placeholder,
                values: value.map(|v| vec![v]),
                roles,
            })
            .collect(),
    }
}
fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn source_expectations() -> Vec<SourceExpectations> {
    family::NAMES
        .into_iter()
        .map(|source_name| SourceExpectations {
            source_name: source_name.into(),
            expected: expectations(),
        })
        .collect()
}
pub fn run() {
    let prior: PathBuf = std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_ACTIVITY_PRIOR")
        .expect("prior package")
        .into();
    let output: PathBuf = std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_ACTIVITY_OUTPUT")
        .expect("fresh output")
        .into();
    publication::run_sources(publication::Publication {
        directory: family::directory(),
        stage: family::stage,
        prior,
        output: output.clone(),
        original05_before: (25, 11, 14),
        new_definitions: 4,
        expectations: source_expectations(),
    });
    let original = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let mut controls = vec![];
    for source_name in family::NAMES {
        for (name, extra, count) in [
            (
                "duplicate-placeholder",
                format!("<Placeholder name=\"{source_name}\" number=\"1\"/>"),
                2,
            ),
            (
                "duplicate-input",
                format!(
                    "<Input name=\"{source_name}\" number=\"2\"/><Input name=\"{source_name}\" number=\"3\"/>"
                ),
                3,
            ),
        ] {
            assert_eq!(original.matches("</ConfigSet>").count(), 1);
            let xml = original.replacen("</ConfigSet>", &format!("{extra}</ConfigSet>"), 1);
            let path = output.join(format!("probe-{source_name}-{name}.xml"));
            fs::write(&path, &xml).unwrap();
            let destination = output.join(format!("probe-{source_name}-{name}"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
            command.arg("normalize-owned").arg(&path);
            for (flag, file) in [
                ("--policy", "normalization.json"),
                ("--registry", "registry.json"),
                ("--definitions", "schema.json"),
                ("--mapping", "mapping.json"),
                ("--roles", "roles.json"),
                ("--rewards", "rewards.json"),
                ("--items", "items.json"),
                ("--item-source", "item-source.json"),
                ("--tree-policy", "tree-normalization.json"),
                ("--queries", "queries-original-05.json"),
            ] {
                command.arg(flag).arg(output.join("package").join(file));
            }
            let result = command.arg("--output").arg(&destination).output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let draft = read(destination.join("draft.json"));
            for preset in draft["draft"]["scenario_presets"]["members"]
                .as_array()
                .unwrap()
            {
                assert!(
                    !preset["scenario"]["assumptions"]["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| family::INPUTS
                            .iter()
                            .any(|id| v["input"]["value"]["key"] == *id))
                );
            }
            let imported = ImportedBuildInstance::from_decoded(
                decode_build(xml.as_bytes()).unwrap(),
                BuildLineage::from_bytes([153; 16]),
                Default::default(),
            )
            .unwrap();
            let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
            let sidecar = read(destination.join("sidecar.json"));
            let mut counts = [0; 4];
            for row in evidence
                .rows()
                .iter()
                .filter(|r| matches!(r.occurrence().name(), "Input" | "Placeholder"))
            {
                let Some(index) = family::NAMES.iter().position(|name| {
                    row.attribute("name")
                        .is_some_and(|a| a.decoded().ok() == Some(*name))
                }) else {
                    continue;
                };
                counts[index] += 1;
                let origin = &sidecar["origins"][row.occurrence().id().ordinal() as usize];
                let links = origin["links"].as_array().unwrap();
                assert!(links.iter().any(|l| l["kind"] == "issue"));
                assert!(!links.iter().any(|l| l["kind"] == "scenario_preset"));
            }
            let mut expected = [1; 4];
            expected[family::NAMES
                .iter()
                .position(|name| *name == source_name)
                .unwrap()] = count;
            assert_eq!(counts, expected);
            controls.push(json!({"source":source_name,"case":name,"saved_lanes":counts,"inputs_emitted":0,"all_saved_lanes_pending":true}));
        }
    }
    let path = output.join("validation.json");
    let mut validation = read(&path);
    validation["duplicate_controls"] = json!(controls);
    fs::write(path, serde_json::to_vec_pretty(&validation).unwrap()).unwrap();
}
