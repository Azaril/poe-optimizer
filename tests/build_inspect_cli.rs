//! Caller input and source diagnostics never imply mechanic admission.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
#[test]
fn caller_structure_is_preserved_without_a_runtime_or_build_default() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let xml = "<PathOfBuilding2><Future owner='a\r\n\tb &amp; c'><Nested/></Future><Party/><Future/><Calcs><Input name='odd' number='NaN'/></Calcs><Config><Input name='bad' boolean='maybe'/></Config></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let output = cli()
        .current_dir(temp.path())
        .arg("inspect-build")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["scope"], "build_source_projection_v2");
    assert_eq!(report["items"]["status"], "source_projected");
    assert_eq!(report["verification"]["item_loading"], "not_run");
    assert_eq!(
        report["verification"]["equipment_resolution"],
        "not_resolved"
    );
    assert_eq!(report["verification"]["passive_allocation"], "not_checked");
    assert_eq!(
        report["build"]["source_sha256"],
        format!("{:x}", Sha256::digest(xml.as_bytes()))
    );
    let sections = report["build"]["sections"].as_array().unwrap();
    assert_eq!(
        sections
            .iter()
            .map(|s| s["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["unknown", "party", "unknown", "calcs", "config"]
    );
    assert_eq!(
        sections[0]["element"]["attributes"][0]["value"]["decoded"],
        "a\r\n\tb & c"
    );
    assert!(
        sections[3]["records"][0]["scalar_error"]["reason"]
            .as_str()
            .unwrap()
            .contains("finite")
    );
    assert_eq!(report["configuration"]["status"], "not_projected");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
}
#[test]
fn all_five_share_codes_and_xmls_inspect_to_identical_source_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let imports = fs::read_to_string(corpus.join("imports.txt")).unwrap();
    let index: Value =
        serde_json::from_slice(&fs::read(corpus.join("index.json")).unwrap()).unwrap();
    for (line, entry) in imports.lines().zip(index["builds"].as_array().unwrap()) {
        let input = temp.path().join("caller.import.txt");
        fs::write(&input, line).unwrap();
        let mut evidence = Vec::new();
        for path in [input, corpus.join(entry["xml"].as_str().unwrap())] {
            let output = cli()
                .current_dir(temp.path())
                .arg("inspect-build")
                .arg(path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["input"]["xml_sha256"], entry["xml_sha256"]);
            assert_eq!(report["configuration"]["status"], "source_projected");
            evidence.push(report["build"].clone());
        }
        assert_eq!(evidence[0], evidence[1]);
    }
}
#[test]
fn requires_input_and_never_overwrites_or_publishes_invalid_xml() {
    assert!(
        !cli()
            .arg("inspect-build")
            .output()
            .unwrap()
            .status
            .success()
    );
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.xml");
    let report = temp.path().join("report.json");
    fs::write(&input, "<PathOfBuilding2><Party/>").unwrap();
    let output = cli()
        .arg("inspect-build")
        .arg(&input)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!report.exists());
    let xml = "<PathOfBuilding2><Caller/></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let output = cli()
        .arg("inspect-build")
        .arg(&input)
        .arg("--output")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(&input).unwrap(), xml);
    let output = cli()
        .arg("inspect-build")
        .arg(&input)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&report).unwrap()).unwrap()["status"],
        "source_projected"
    );
}
