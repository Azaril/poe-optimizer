//! Item input evidence is portable and distinct from item/equipment admission.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn inspect(path: &Path, cwd: &Path) -> Value {
    let output = cli()
        .current_dir(cwd)
        .arg("inspect-build")
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn count_role(node: &Value, role: &str) -> usize {
    usize::from(node["source_use"] == role)
        + node["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| count_role(child, role))
            .sum::<usize>()
}
fn projection_count(report: &Value, role: &str) -> usize {
    ["containers", "trees"]
        .into_iter()
        .flat_map(|field| report["items"]["projection"][field].as_array().unwrap())
        .map(|node| count_role(node, role))
        .sum()
}
#[test]
fn caller_mixed_item_content_preserves_distinct_loads_without_blocking_other_sections() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let xml = "<PathOfBuilding2><Skills><SkillSet id='8'/></Skills><Config><Input name='enemyLevel' number='90'/></Config><Items><Item id='007'>first<!--keep-->second<ModRange id='1' range='.25'/><![CDATA[ next &literal; ]]></Item><ItemSet id='9'><SocketIdURL nodeId='55' itemPbURL='caller metadata'/></ItemSet></Items><Spec><Sockets><Socket nodeId='55' itemId='007'/></Sockets></Spec></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let report = inspect(&input, temp.path());
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["scope"], "build_source_projection_v2");
    for section in ["items", "skills", "configuration"] {
        assert_eq!(report[section]["status"], "source_projected", "{section}");
    }
    assert!(report.get("definition_lookup").is_none());
    assert_eq!(report["verification"]["item_loading"], "not_run");
    assert_eq!(
        report["verification"]["equipment_resolution"],
        "not_resolved"
    );
    assert_eq!(report["verification"]["passive_allocation"], "not_checked");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    let item = &report["items"]["projection"]["containers"][0]["children"][0];
    assert_eq!(item["element"]["attributes"][0]["value"]["decoded"], "007");
    let consumed = item["ordered_content"]["consumed"].as_array().unwrap();
    assert_eq!(consumed.len(), 3);
    assert_eq!(consumed[0]["text"], "firstsecond");
    assert_eq!(consumed[1]["kind"], "element");
    assert_eq!(consumed[1]["child_index"], 0);
    assert_eq!(consumed[2]["text_kind"], "cdata");
    assert_eq!(consumed[2]["text"], " next &literal; ");
    assert_eq!(projection_count(&report, "socket_url_metadata"), 1);
    assert_eq!(projection_count(&report, "jewel_assignment"), 1);
    assert_eq!(
        report["items"]["projection"]["trees"][0]["source_use"],
        "passive_spec"
    );
    assert_eq!(
        report["input"]["xml_sha256"],
        format!("{:x}", Sha256::digest(xml.as_bytes()))
    );
    assert_eq!(fs::read(input).unwrap(), xml.as_bytes());
}
#[test]
fn every_saved_corpus_item_set_and_passive_jewel_assignment_is_inspected() {
    let temp = tempfile::tempdir().unwrap();
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let mut totals = [0usize; 5];
    for i in 1..=5 {
        let path = corpus.join(format!("build-{i:02}.xml"));
        let before = fs::read(&path).unwrap();
        let report = inspect(&path, temp.path());
        assert_eq!(report["items"]["status"], "source_projected");
        assert_eq!(
            report["items"]["projection"]["source_sha256"],
            format!("{:x}", Sha256::digest(&before))
        );
        for (sum, role) in totals.iter_mut().zip([
            "inventory_item",
            "saved_set",
            "modifier_range_instruction",
            "passive_spec",
            "jewel_assignment",
        ]) {
            *sum += projection_count(&report, role);
        }
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    // Independently observed complete authored corpus, including inactive alternatives.
    assert_eq!(totals, [116, 15, 486, 16, 21]);
}
#[test]
fn item_projection_failure_does_not_erase_skills_or_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("deep.xml");
    let depth = poe_optimizer_import::item_source::MAX_ITEM_SOURCE_DEPTH + 1;
    let xml = format!(
        "<PathOfBuilding2><Items>{}{}</Items><Skills><SkillSet id='1'/></Skills><Config><Input name='enemyLevel' number='90'/></Config></PathOfBuilding2>",
        "<Future>".repeat(depth),
        "</Future>".repeat(depth)
    );
    fs::write(&input, &xml).unwrap();
    let report = inspect(&input, temp.path());
    assert_eq!(report["items"]["status"], "not_projected");
    assert!(
        report["items"]["error"]["reason"]
            .as_str()
            .unwrap()
            .contains("depth")
    );
    for section in ["skills", "configuration"] {
        assert_eq!(report[section]["status"], "source_projected");
    }
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
}
#[test]
fn item_source_inspection_does_not_bypass_existing_native_lexical_admission() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("mixed-item.xml");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/calibration/mace-wooden.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    let xml = original.replacen("</Item>", "<![CDATA[later item text]]></Item>", 1);
    assert_ne!(xml, original);
    fs::write(&input, &xml).unwrap();
    let report = inspect(&input, temp.path());
    assert_eq!(report["items"]["status"], "source_projected");
    let output = cli()
        .current_dir(temp.path())
        .arg("evaluate")
        .arg(&input)
        .args(["--backend", "native"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("CDATA"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
}
