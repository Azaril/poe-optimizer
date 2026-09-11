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
    assert_eq!(report["schema_version"], 3);
    assert!(report.get("instances").is_none());
    assert_eq!(report["scope"], "build_source_projection_v3");
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

// Authored occurrences intentionally disagree with uniqueness assumptions. No
// selected/effective interpretation is needed to preserve their identities.
const INSTANCE_CALLER_XML: &str = r#"<PathOfBuilding2>
<Skills activeSkillSet='2'>
  <SkillSet id='2' title='First'><Skill label='Repeated'><Gem skillId='caller_effect'/><FutureEntry enabled='maybe'/></Skill></SkillSet>
  <SkillSet id='2' title='Second'><Skill label='Repeated'><Gem nameSpec='Caller Skill'/></Skill></SkillSet>
</Skills>
<Items activeItemSet='4'>
  <Item id='7'>Rarity: RARE
Caller Jewel
Sapphire
Implicits: 0
+1 to maximum Mana</Item>
  <Item id='7'>Rarity: RARE
Other Caller Jewel
Sapphire
Implicits: 0
+2 to maximum Mana</Item>
  <ItemSet id='4'><Slot name='Ring 1' itemId='7'/><Slot name='Ring 2' itemId='7'/></ItemSet>
  <ItemSet id='4' title='Stored'><Slot name='Weapon 1' itemId='7'/></ItemSet>
</Items>
<Tree activeSpec='2'><Spec title='First'/><Spec title='Second'><Sockets><Socket nodeId='11' itemId='7'/></Sockets></Spec></Tree>
<Config activeConfigSet='3'><ConfigSet id='3'><Input name='caller' boolean='true'/></ConfigSet><ConfigSet id='3'><Input name='caller' boolean='false'/></ConfigSet></Config>
<Future payload='a &amp; b'><Child/></Future>
</PathOfBuilding2>"#;

fn inspect_report(path: &Path, flags: &[&str]) -> Value {
    let output = cli()
        .arg("inspect-build")
        .arg(path)
        .args(flags)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn source_slice<'a>(xml: &'a str, range: &Value) -> &'a str {
    &xml[range["start"].as_u64().unwrap() as usize..range["end"].as_u64().unwrap() as usize]
}

#[test]
fn instance_report_preserves_duplicates_and_opaque_source_with_fresh_host_lineages() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("instance-caller.xml");
    fs::write(&input, INSTANCE_CALLER_XML).unwrap();
    let default = inspect_report(&input, &[]);
    let first = inspect_report(&input, &["--with-instances"]);
    let second = inspect_report(&input, &["--with-instances"]);
    assert!(default.get("instances").is_none());
    for report in [&first, &second] {
        let mut legacy = report.clone();
        legacy.as_object_mut().unwrap().remove("instances");
        assert_eq!(legacy, default);
    }
    let first = &first["instances"];
    let second = &second["instances"];
    assert_eq!(first["schema_version"], 1);
    assert_eq!(first["source_sha256"], default["input"]["xml_sha256"]);
    assert_eq!(first["source_sha256"], second["source_sha256"]);
    assert_eq!(first["source_bytes"], INSTANCE_CALLER_XML.len());
    assert_eq!(first["occurrences"], second["occurrences"]);
    assert_ne!(first["lineage"], second["lineage"]);
    assert_eq!(first["revision"], "0000000000000000");
    let lineage = first["lineage"].as_str().unwrap();
    assert_eq!(lineage.len(), 32);
    assert!(
        lineage
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    let occurrences = first["occurrences"].as_array().unwrap();
    let bindings = first["instances"].as_array().unwrap();
    let mut counts = std::collections::BTreeMap::new();
    let mut ids = std::collections::BTreeSet::new();
    for binding in bindings {
        let kind = binding["instance"]["kind"].as_str().unwrap();
        *counts.entry(kind).or_insert(0) += 1;
        let id = &binding["instance"]["id"];
        assert_eq!(id["lineage"], first["lineage"]);
        assert!(ids.insert(id["local"].as_str().unwrap()));
        let source = &binding["source"];
        assert_eq!(source["source_sha256"], first["source_sha256"]);
        let ordinal = source["ordinal"].as_u64().unwrap() as usize;
        assert_eq!(occurrences[ordinal]["id"], *source);
    }
    assert_eq!(
        counts,
        std::collections::BTreeMap::from([
            ("skill_set", 2),
            ("skill_group", 2),
            ("skill_entry", 3),
            ("item_record", 2),
            ("item_set", 2),
            ("item_slot_use", 4),
            ("passive_spec", 2),
            ("config_set", 2),
        ])
    );
    // Two different equipment uses retain the same unresolved authored item ID.
    let ring_slots = occurrences
        .iter()
        .filter(|row| row["name"] == "Slot")
        .filter(|row| source_slice(INSTANCE_CALLER_XML, &row["source_range"]).contains("Ring "))
        .collect::<Vec<_>>();
    assert_eq!(ring_slots.len(), 2);
    assert_ne!(ring_slots[0]["id"], ring_slots[1]["id"]);
    for slot in ring_slots {
        assert_eq!(slot["role"]["usage"], "equipment_slot");
        let item_ref = slot["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|attribute| attribute["name"] == "itemId")
            .unwrap();
        assert_eq!(
            source_slice(INSTANCE_CALLER_XML, &item_ref["value_range"]),
            "7"
        );
    }
    let future = occurrences
        .iter()
        .find(|row| row["name"] == "Future")
        .unwrap();
    assert_eq!(future["role"]["domain"], "opaque");
    assert_eq!(
        source_slice(INSTANCE_CALLER_XML, &future["source_range"]),
        "<Future payload='a &amp; b'><Child/></Future>"
    );
    let unknown_gem = occurrences
        .iter()
        .find(|row| row["name"] == "FutureEntry")
        .unwrap();
    assert_eq!(unknown_gem["role"]["kind"], "unknown");
    assert_eq!(unknown_gem["role"]["usage"], "gem_instance");
    let config = first["projections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|projection| projection["projection"] == "configuration")
        .unwrap();
    assert_eq!(config["status"], "unavailable");
    assert!(config.get("error").is_some());
    assert_eq!(default["configuration"]["status"], "not_projected");
    assert_eq!(default["verification"]["native_admission"], "not_checked");
    assert_eq!(
        default["verification"]["equipment_resolution"],
        "not_resolved"
    );
    assert_eq!(fs::read(&input).unwrap(), INSTANCE_CALLER_XML.as_bytes());
}

#[test]
fn instance_report_composes_with_explicit_definition_inspection() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("instance-caller.xml");
    fs::write(&input, INSTANCE_CALLER_XML).unwrap();
    let definitions = inspect_report(&input, &["--with-definitions"]);
    let mut combined = inspect_report(&input, &["--with-definitions", "--with-instances"]);
    let instances = combined
        .as_object_mut()
        .unwrap()
        .remove("instances")
        .unwrap();
    assert!(!instances["instances"].as_array().unwrap().is_empty());
    assert_eq!(
        instances["source_sha256"],
        definitions["input"]["xml_sha256"]
    );
    assert_eq!(combined, definitions);
    assert_eq!(combined["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(combined["verification"]["native_admission"], "not_checked");
    assert_eq!(
        combined["definition_lookup"]["configuration"]["status"],
        "not_looked_up"
    );
    assert_eq!(fs::read(&input).unwrap(), INSTANCE_CALLER_XML.as_bytes());
}
