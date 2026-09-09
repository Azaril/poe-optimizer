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
    assert_eq!(report["schema_version"], 3);
    assert_eq!(report["scope"], "build_source_projection_v3");
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

fn inspect_definitions(path: &Path, cwd: &Path, data: Option<&Path>) -> Value {
    let mut command = cli();
    command.current_dir(cwd).arg("inspect-build").arg(path);
    if let Some(data) = data {
        command.arg("--data").arg(data);
    } else {
        command.arg("--with-definitions");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn caller_catalog_changes_item_resolution_and_keeps_unavailable_operations_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let data_path = temp.path().join("caller-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let mut base = package
        .item_loading
        .bases
        .iter()
        .find(|base| base.name == "Rusted Greathelm")
        .unwrap()
        .clone();
    base.name = "Caller Supplied Helmet Base".into();
    package.item_loading.bases.push(base);
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test changes item loading construction inputs",
        );
    package.refresh_section_digests().unwrap();
    let data_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &data_bytes).unwrap();
    let xml = "<PathOfBuilding2><Items><Item id='007'>Rarity: Normal\nCaller Supplied Helmet Base\nItem Level: 1\nQuality: 0\n</Item></Items></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let bundled = inspect_definitions(&input, temp.path(), None);
    assert_eq!(
        bundled["definition_lookup"]["items"]["report"]["items"][0]["state"]["base_present"],
        false
    );
    let selected = inspect_definitions(&input, temp.path(), Some(&data_path));
    let loaded = &selected["definition_lookup"]["items"]["report"];
    let item = &loaded["items"][0];
    assert_eq!(item["authored_id"], "007");
    assert_eq!(
        item["source_occurrence"],
        serde_json::json!({"container_index":0,"item_index":0})
    );
    assert_eq!(item["state"]["base_present"], true);
    assert_eq!(item["state"]["base_name"], "Caller Supplied Helmet Base");
    assert_eq!(item["status"], "pending");
    assert!(
        item["pending"]["message"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(item["instructions"][0]["kind"], "constructor");
    assert_eq!(item["instructions"][0]["status"], "executed");
    assert_eq!(
        loaded["source_sha256"],
        format!("{:x}", Sha256::digest(xml.as_bytes()))
    );
    assert_eq!(
        loaded["data_identity"],
        selected["definition_lookup"]["data"]
    );
    assert_eq!(
        loaded["data_identity"]["content_sha256"],
        format!("{:x}", Sha256::digest(&data_bytes))
    );
    assert_eq!(
        loaded["implementation_sha256"],
        selected["item_loading_implementation_sha256"]
    );
    assert_eq!(
        loaded["implementation_sha256"],
        poe_optimizer_import::item_loading::implementation_fingerprint()
    );
    assert_eq!(
        selected["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(selected["verification"]["item_loading"], "reported");
    assert_eq!(selected["verification"]["native_admission"], "not_checked");
    assert_eq!(selected["verification"]["reference_calculation"], "not_run");
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
}
#[test]
fn complete_corpus_inventory_receives_loading_evidence_without_changing_admission() {
    let temp = tempfile::tempdir().unwrap();
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let mut count = 0;
    let mut pending = 0;
    for i in 1..=5 {
        let input = corpus.join(format!("build-{i:02}.xml"));
        let before = fs::read(&input).unwrap();
        let report = inspect_definitions(&input, temp.path(), None);
        let loaded = &report["definition_lookup"]["items"]["report"];
        let items = loaded["items"].as_array().unwrap();
        assert_eq!(items.len(), projection_count(&report, "inventory_item"));
        assert_eq!(
            loaded["source_sha256"],
            format!("{:x}", Sha256::digest(&before))
        );
        assert_eq!(loaded["data_identity"], report["definition_lookup"]["data"]);
        for item in items {
            assert_eq!(
                item["instructions"][0]["status"], "executed",
                "empty constructor must advance"
            );
            assert_eq!(
                item["instructions"].as_array().unwrap().last().unwrap()["kind"],
                "final_assembly"
            );
            if item["status"] == "pending" {
                pending += 1;
                assert!(item["pending"]["message"].is_string());
                let instructions = item["instructions"].as_array().unwrap();
                let stop = instructions
                    .iter()
                    .position(|entry| entry["status"] == "pending")
                    .unwrap();
                assert!(
                    instructions[stop + 1..]
                        .iter()
                        .all(|entry| entry["status"] == "not_executed")
                );
            }
        }
        count += items.len();
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
        assert_eq!(fs::read(&input).unwrap(), before);
    }
    assert_eq!(count, 116);
    assert!(
        pending > 0,
        "unimplemented parser callbacks and item assembly must remain observable"
    );
}
#[test]
fn item_loading_source_error_keeps_other_definition_sections_and_original_input() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("deep.xml");
    let depth = poe_optimizer_import::item_source::MAX_ITEM_SOURCE_DEPTH + 1;
    let xml = format!(
        "<PathOfBuilding2><Items>{}{}</Items><Skills><SkillSet id='1'/></Skills></PathOfBuilding2>",
        "<Future>".repeat(depth),
        "</Future>".repeat(depth)
    );
    fs::write(&input, &xml).unwrap();
    let report = inspect_definitions(&input, temp.path(), None);
    assert_eq!(
        report["definition_lookup"]["items"]["status"],
        "not_reported"
    );
    assert_eq!(
        report["definition_lookup"]["items"]["source_error"],
        report["items"]["error"]
    );
    assert_eq!(report["definition_lookup"]["skills"]["status"], "looked_up");
    assert_eq!(report["verification"]["item_loading"], "not_reported");
    assert_eq!(fs::read_to_string(&input).unwrap(), xml);
}

#[test]
fn caller_scalability_data_controls_formatting_and_preserves_unknown_lines_before_assembly() {
    use poe_optimizer_data::item_scalability::{ItemFormatAssignments, ItemScalabilityValue};
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let data_path = temp.path().join("caller-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    assert!(
        !package
            .item_scalability
            .entries
            .contains_key("Caller roll #")
    );
    package.item_scalability.entries.insert(
        "Caller roll #".into(),
        vec![ItemScalabilityValue {
            is_scalable: true,
            formats: Some(vec!["caller_precision".into()]),
        }],
    );
    package.item_scalability.format_assignments.insert(
        "caller_precision".into(),
        ItemFormatAssignments {
            precision: Some(10.0),
            display_precision: Some(1),
            if_required: Some(false),
        },
    );
    package.refresh_section_digests().unwrap();
    let data_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &data_bytes).unwrap();
    let xml = "<PathOfBuilding2><Items><Item id='caller'>Rarity: Normal\nRusted Greathelm\nItem Level: 1\nQuality: 0\nCaller roll 10.049\n</Item></Items></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let bundled = inspect_definitions(&input, temp.path(), None);
    let selected = inspect_definitions(&input, temp.path(), Some(&data_path));
    for (report, expected) in [
        (&bundled, "Caller roll 10.049"),
        (&selected, "Caller roll 10.0"),
    ] {
        let loaded = &report["definition_lookup"]["items"]["report"];
        let item = &loaded["items"][0];
        assert_eq!(item["status"], "pending", "{item}");
        assert_eq!(item["pending"]["kind"], "assembly", "{item}");
        let lines = item["state"]["explicit_mod_lines"].as_array().unwrap();
        assert_eq!(lines.len(), 1);
        // Item.lua retains the authored display line when parsing returns nil,
        // even when the formatted parser request has a rounded number.
        assert_eq!(lines[0]["line"], "Caller roll 10.049");
        assert_eq!(lines[0]["extra"], "Caller roll 10.049");
        assert!(lines[0]["modifiers"].as_array().unwrap().is_empty());
        let calls = item["state"]["parser_calls"].as_array().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["text"], expected);
        assert_eq!(loaded["data_identity"], report["definition_lookup"]["data"]);
        assert_eq!(
            loaded["implementation_sha256"],
            report["item_loading_implementation_sha256"]
        );
        assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
    }
    assert_ne!(
        bundled["definition_lookup"]["data"],
        selected["definition_lookup"]["data"]
    );
    assert_eq!(bundled["items"], selected["items"]);
    assert_eq!(
        selected["definition_lookup"]["data"]["content_sha256"],
        format!("{:x}", Sha256::digest(&data_bytes))
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
}

#[test]
fn injected_defence_headers_reach_cli_state_without_granting_assembly_or_admission() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-defence.xml");
    let data_path = temp.path().join("caller-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    package
        .item_loading
        .policy
        .header_names
        .insert("Caller Guard".into());
    package
        .item_loading
        .policy
        .defence_header_keys
        .insert("Caller Guard".into(), "CallerGuardValue".into());
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test changes item loading construction inputs",
        );
    package.refresh_section_digests().unwrap();
    let data_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &data_bytes).unwrap();
    let xml = "<PathOfBuilding2><Items><Item id='header'>Rarity: Normal\nRusted Greathelm\nArmour: 42\nArmour: invalid\nCaller Guard: -3.5\n</Item></Items></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let bundled = inspect_definitions(&input, temp.path(), None);
    let selected = inspect_definitions(&input, temp.path(), Some(&data_path));
    for (report, expected) in [
        (&bundled, serde_json::json!({})),
        (
            &selected,
            serde_json::json!({"CallerGuardValue":{"kind":"finite","value":-3.5}}),
        ),
    ] {
        let item = &report["definition_lookup"]["items"]["report"]["items"][0];
        assert_eq!(item["state"]["armour_data"], expected);
        assert!(
            item["state"]["retained_fields"]
                .get("hidden_specs")
                .is_none()
        );
        assert_eq!(item["status"], "pending");
        assert_eq!(item["pending"]["kind"], "assembly");
        assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
    }
    assert_eq!(bundled["items"], selected["items"]);
    assert_ne!(
        bundled["definition_lookup"]["data"],
        selected["definition_lookup"]["data"]
    );
    assert_eq!(
        selected["definition_lookup"]["data"]["content_sha256"],
        format!("{:x}", Sha256::digest(&data_bytes))
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
}

#[test]
fn injected_base_buffs_reach_cli_loading_state_without_numerical_admission() {
    use poe_optimizer_data::item_loading::ItemMetadataValue;
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-buffs.xml");
    let data_path = temp.path().join("caller-buff-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let base = package
        .item_loading
        .bases
        .iter_mut()
        .find(|base| base.name == "Amethyst Charm")
        .unwrap();
    let ItemMetadataValue::Table(charm) = base.fields.fields.get_mut("charm").unwrap() else {
        panic!("fixture charm shape");
    };
    let text = "+123 to maximum Life";
    charm.fields.insert(
        "buff".into(),
        ItemMetadataValue::Array(
            [text, text, ""]
                .into_iter()
                .map(|line| ItemMetadataValue::Text(line.into()))
                .collect(),
        ),
    );
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test changes base buff construction inputs",
        );
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &bytes).unwrap();
    let xml = format!(
        "<PathOfBuilding2><Items><Item id='caller-buff'>Rarity: Normal\nAmethyst Charm\n{text}\n{text}\n</Item></Items></PathOfBuilding2>"
    );
    fs::write(&input, &xml).unwrap();
    let report = inspect_definitions(&input, temp.path(), Some(&data_path));
    let state = &report["definition_lookup"]["items"]["report"]["items"][0]["state"];
    let rows = state["buff_mod_lines"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for (row, expected) in rows.iter().zip([text, text, ""]) {
        assert_eq!(row["line"], expected);
        assert_eq!(row["range"]["kind"], "nil");
        assert_eq!(row["value_scalar"]["kind"], "nil");
    }
    assert_eq!(state["explicit_mod_lines"].as_array().unwrap().len(), 1);
    assert_eq!(state["explicit_mod_lines"][0]["line"], text);
    assert_eq!(state["parser_calls"].as_array().unwrap().len(), 4);
    assert_eq!(state["format_calls"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["definition_lookup"]["items"]["report"]["items"][0]["pending"]["kind"],
        "assembly"
    );
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(
        report["definition_lookup"]["data"]["content_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), bytes);
}

#[test]
fn injected_unique_requirements_distinguish_hit_miss_unavailable_and_stale_inputs() {
    use poe_optimizer_data::unique_requirements::UniqueRequirementData;
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-unique.xml");
    let data_path = temp.path().join("caller-unique-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    // Select fixture identity from injected data; no production item/level mapping.
    // Omit base buffs so this test reaches the requirement seam directly.
    let (key, title, base_name, base_level) = package
        .unique_requirements
        .complete()
        .unwrap()
        .entries
        .iter()
        .find_map(|entry| {
            let base = package
                .item_loading
                .bases
                .iter()
                .find(|base| base.name == entry.base_name)?;
            if base.fields.fields.contains_key("flask") || base.fields.fields.contains_key("charm")
            {
                return None;
            }
            let title = entry
                .canonical_key
                .strip_suffix(&format!(", {}", entry.base_name))?;
            let level = base.requirements()?.fields.get("level")?.as_f64()?;
            Some((
                entry.canonical_key.clone(),
                title.to_owned(),
                entry.base_name.clone(),
                level,
            ))
        })
        .expect("completed catalog contains an ordinary base fixture");
    let missing_title = format!("{title} Caller Missing");
    let missing_key = format!("{missing_title}, {base_name}");
    assert!(
        package
            .unique_requirements
            .complete()
            .unwrap()
            .entries
            .iter()
            .all(|entry| entry.canonical_key != missing_key)
    );
    let xml = format!(
        "<PathOfBuilding2><Items><Item id='hit'><![CDATA[Rarity: UNIQUE\n{title}\n{base_name}\nImplicits: 0]]></Item><Item id='miss'><![CDATA[Rarity: UNIQUE\n{missing_title}\n{base_name}\nImplicits: 0]]></Item></Items></PathOfBuilding2>"
    );
    fs::write(&input, &xml).unwrap();
    let natural = base_level + 13.5;
    let entry = package
        .unique_requirements
        .complete_mut()
        .unwrap()
        .entries
        .iter_mut()
        .find(|entry| entry.canonical_key == key)
        .unwrap();
    entry.natural_level = Some(natural);
    // A present natural requirement takes precedence over this larger DB level.
    entry.level = Some(natural + 20.0);
    package.refresh_section_digests().unwrap();
    let complete_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &complete_bytes).unwrap();
    let complete = inspect_definitions(&input, temp.path(), Some(&data_path));
    let items = complete["definition_lookup"]["items"]["report"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(items.len(), 2);
    for (item, name, level) in [
        (&items[0], &key, natural),
        (&items[1], &missing_key, base_level),
    ] {
        assert_eq!(item["state"]["name"], *name);
        assert_eq!(item["status"], "pending");
        assert_eq!(item["pending"]["kind"], "assembly", "{item}");
        for requirement in ["naturalLevel", "level"] {
            assert_eq!(
                item["state"]["requirements"][requirement],
                serde_json::json!({"kind":"finite","value":level})
            );
        }
    }
    assert_eq!(
        complete["definition_lookup"]["data"]["content_sha256"],
        format!("{:x}", Sha256::digest(&complete_bytes))
    );
    assert_eq!(
        complete["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(complete["verification"]["native_admission"], "not_checked");
    assert_eq!(complete["verification"]["reference_calculation"], "not_run");
    assert_eq!(fs::read(&data_path).unwrap(), complete_bytes);

    // Reusing completed facts after changing a construction dependency is rejected.
    package.item_loading.policy.default_item_quality += 1.0;
    package.refresh_section_digests().unwrap();
    let stale_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &stale_bytes).unwrap();
    let failed = cli()
        .current_dir(temp.path())
        .arg("inspect-build")
        .arg(&input)
        .arg("--data")
        .arg(&data_path)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("stale construction input identity"),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    assert_eq!(fs::read(&data_path).unwrap(), stale_bytes);

    // An explicitly unavailable catalog is distinct from a complete negative lookup.
    package.unique_requirements =
        UniqueRequirementData::unavailable("caller requires new construction");
    package.refresh_section_digests().unwrap();
    let unavailable_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &unavailable_bytes).unwrap();
    let unavailable = inspect_definitions(&input, temp.path(), Some(&data_path));
    assert_eq!(complete["items"], unavailable["items"]);
    assert_ne!(
        complete["definition_lookup"]["data"],
        unavailable["definition_lookup"]["data"]
    );
    for item in unavailable["definition_lookup"]["items"]["report"]["items"]
        .as_array()
        .unwrap()
    {
        assert_eq!(item["status"], "pending");
        assert_eq!(item["pending"]["kind"], "unique_database");
        assert_eq!(
            item["pending"]["message"],
            "caller requires new construction"
        );
        assert!(item["state"]["requirements"].get("naturalLevel").is_none());
    }
    assert_eq!(
        unavailable["verification"]["native_admission"],
        "not_checked"
    );
    assert_eq!(
        unavailable["verification"]["reference_calculation"],
        "not_run"
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), unavailable_bytes);
}

#[test]
fn injected_affix_records_limits_and_legacy_names_reach_cli_without_admission() {
    use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
    use std::collections::BTreeMap;

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-affixes.xml");
    let data_path = temp.path().join("caller-affix-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let base = package
        .item_loading
        .bases
        .iter()
        .find(|base| {
            base.item_type
                != package
                    .item_loading
                    .policy
                    .affix_loading
                    .reconcile
                    .jewel_type
                && base.sub_type().is_none()
                && base.field("flask").is_none()
                && base.field("charm").is_none()
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap();
    let base_name = base.name.clone();
    let table_name = base.item_type.clone();
    let mod_id = "CallerExactAffix";
    let legacy = "Caller legacy affix label";
    let label_field = "callerLegacyLabel";
    let none = package
        .item_loading
        .policy
        .affix_loading
        .none_mod_id
        .clone();
    package.item_loading.policy.default_affix_quality = 0.375;
    package.item_loading.policy.affix_loading.legacy_label_field = label_field.into();
    let reconcile = &mut package.item_loading.policy.affix_loading.reconcile;
    reconcile.magic_limit = 4.0;
    reconcile.magic_side_base = 2.0;
    reconcile.magic_side_max = 4.0;
    package.item_loading.modifier_tables.insert(
        table_name,
        ItemMetadataTable {
            fields: BTreeMap::from([(
                mod_id.into(),
                ItemMetadataValue::Table(ItemMetadataTable {
                    fields: BTreeMap::from([(
                        label_field.into(),
                        ItemMetadataValue::Text(legacy.into()),
                    )]),
                    indexed: BTreeMap::new(),
                }),
            )]),
            indexed: BTreeMap::new(),
        },
    );
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "caller changes affix construction inputs",
        );
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &bytes).unwrap();
    let xml = format!(
        "<PathOfBuilding2><Items><Item id='caller-affixes'>Rarity: Magic\n{base_name}\nCrafted: true\nPrefix: {{fractured}}{{range:0.25,invalid,0.75}}{mod_id}\nPrefix: {legacy}\nPrefix: {none}\nSuffix: CallerMissingAffix\nSuffix: {{range:,}}{none}\n+1 prefix modifier allowed\n-1 suffix modifier allowed\n</Item></Items></PathOfBuilding2>"
    );
    fs::write(&input, &xml).unwrap();
    let report = inspect_definitions(&input, temp.path(), Some(&data_path));
    let item = &report["definition_lookup"]["items"]["report"]["items"][0];
    let state = &item["state"];
    let finite = |value| serde_json::json!({"kind": "finite", "value": value});
    let scalar = |value| serde_json::json!({"kind": "scalar", "value": finite(value)});
    assert_eq!(state["base_name"], base_name);
    assert_eq!(state["prefixes"]["limit"], finite(3.0));
    assert_eq!(state["suffixes"]["limit"], finite(1.0));
    assert_eq!(
        state["retained_fields"]["affixLimit"],
        serde_json::json!({"kind": "number", "value": finite(4.0)})
    );
    assert_eq!(
        state["prefixes"]["entries"],
        serde_json::json!([
            {"mod_id": mod_id, "range": {"kind": "independent", "value": [finite(0.25), finite(0.75)]}, "fractured": true},
            {"mod_id": mod_id, "range": scalar(0.375), "fractured": null},
            {"mod_id": none, "range": null, "fractured": null}
        ])
    );
    assert_eq!(
        state["suffixes"]["entries"],
        serde_json::json!([
            {"mod_id": none, "range": scalar(0.375), "fractured": null},
            {"mod_id": none, "range": {"kind": "independent", "value": []}, "fractured": null}
        ]),
        "authored rows beyond the active limit remain diagnostic state"
    );
    assert_eq!(item["status"], "pending");
    assert_eq!(item["pending"]["kind"], "assembly");
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(
        report["definition_lookup"]["data"]["content_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(
        report["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), bytes);
}
