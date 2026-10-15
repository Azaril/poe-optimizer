//! Exact publication and negative source-frame probes for one Player input.
use crate::{demonflame as family, publication};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_source::SourceProjectEvidence,
};
use publication::{
    Control, Expectations,
    LeafRole::{Contributes as C, UnselectedProjection as U},
    Output,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};

fn expectations() -> Expectations {
    let cases = [
        (
            "changed-placeholder",
            None,
            Some("80"),
            Some(80.),
            [None, Some(C)],
        ),
        (
            "zero-fallback",
            Some("0"),
            Some("80"),
            Some(80.),
            [Some(U), Some(C)],
        ),
        (
            "negative-zero-fallback",
            Some("-0"),
            Some("80"),
            Some(80.),
            [Some(U), Some(C)],
        ),
        (
            "fraction",
            Some("-12.5"),
            Some("20"),
            Some(-12.5),
            [Some(C), Some(U)],
        ),
        (
            "above-effective-cap",
            Some("25"),
            Some("20"),
            Some(25.),
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
            Some(10.),
            [Some(U), None],
        ),
        ("absent", None, None, Some(10.), [None, None]),
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
        ("bad-input", Some("NaN"), Some("20"), None, [None, None]),
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
            Some("20"),
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
        default: 10.,
        controls: cases
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
pub fn run() {
    let prior: PathBuf = std::env::var_os("POE_OPTIMIZER_TEST_DEMONFLAME_PRIOR")
        .expect("prior package")
        .into();
    let output: PathBuf = std::env::var_os("POE_OPTIMIZER_TEST_DEMONFLAME_OUTPUT")
        .expect("fresh output")
        .into();
    publication::run(publication::Publication {
        directory: family::directory(),
        stage: family::stage,
        prior,
        output: output.clone(),
        original05_before: (21, 7, 14),
        new_definitions: 1,
        expectations: expectations(),
    });
    let original = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let mut rows = vec![];
    for (name, extra, expected_count) in [
        (
            "duplicate-placeholder",
            "<Placeholder name=\"demonFormStacks\" number=\"20\"/>",
            2,
        ),
        (
            "duplicate-input",
            "<Input name=\"demonFormStacks\" number=\"30\"/><Input name=\"demonFormStacks\" number=\"40\"/>",
            3,
        ),
    ] {
        assert_eq!(original.matches("</ConfigSet>").count(), 1);
        let xml = original.replacen("</ConfigSet>", &format!("{extra}</ConfigSet>"), 1);
        let xml_path = output.join(format!("probe-{name}.xml"));
        fs::write(&xml_path, &xml).unwrap();
        let destination = output.join(format!("probe-{name}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
        command.arg("normalize-owned").arg(&xml_path);
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
                    .any(|v| v["input"]["value"]["key"] == family::INPUT)
            );
        }
        let source = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([152; 16]),
            Default::default(),
        )
        .unwrap();
        let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
        let sidecar = read(destination.join("sidecar.json"));
        let mut count = 0;
        for row in evidence.rows().iter().filter(|r| {
            matches!(r.occurrence().name(), "Input" | "Placeholder")
                && r.attribute("name")
                    .is_some_and(|v| v.decoded().ok() == Some(family::SOURCE))
        }) {
            count += 1;
            let origin = &sidecar["origins"][row.occurrence().id().ordinal() as usize];
            assert!(
                origin["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["kind"] == "issue")
            );
            assert!(
                !origin["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["kind"] == "scenario_preset")
            );
        }
        assert_eq!(count, expected_count);
        rows.push(json!({"case":name,"saved_lanes":count,"input_emitted":false,"all_saved_lanes_pending":true}));
    }
    let path = output.join("validation.json");
    let mut validation = read(&path);
    validation["duplicate_controls"] = json!(rows);
    fs::write(path, serde_json::to_vec_pretty(&validation).unwrap()).unwrap();
}
