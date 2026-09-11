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

#[test]
fn injected_rune_names_slots_and_numeric_grammar_rebuild_with_generated_origins() {
    use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
    use std::collections::BTreeMap;

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-runes.xml");
    let data_path = temp.path().join("caller-rune-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let base_name = package
        .item_loading
        .bases
        .iter()
        .find(|base| {
            base.field("weapon")
                .is_some_and(|value| !matches!(value, ItemMetadataValue::Boolean(false)))
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let family = "Caller Rune Definitions";
    let names = ["Caller Azure Fragment", "Caller Ochre Fragment"];
    let slot = "caller weapon slot";
    let rune_header = "CallerRune";
    let socket_header = "CallerSockets";
    package
        .item_loading
        .policy
        .header_names
        .extend([rune_header.into(), socket_header.into()]);
    let policy = &mut package.item_loading.policy.rune_loading;
    policy.rune_header = rune_header.into();
    policy.socket_header = socket_header.into();
    policy.rune_table = family.into();
    policy.broad_weapon_type = slot.into();
    policy.socket_character_pattern = "q".into();
    policy.item_socket_pattern = "^q$".into();
    // This caller grammar treats a multi-digit decimal as one capture. The
    // shipped rune grammar would split 12.34 into separate 12 and 34 captures.
    policy.numeric_pattern = "(%d+%.?%d*)".into();
    policy.stripped_marker = "@".into();
    policy.order_default = 7.5;
    let augment_type = policy.rune_augment_type.clone();
    let definitions = names
        .into_iter()
        .zip(["+12.34 to maximum Life", "+5.67 to maximum Life"])
        .map(|(name, line)| {
            (
                name.into(),
                ItemMetadataValue::Table(ItemMetadataTable {
                    fields: BTreeMap::from([(
                        slot.into(),
                        ItemMetadataValue::Table(ItemMetadataTable {
                            fields: BTreeMap::from([
                                ("type".into(), ItemMetadataValue::Text(augment_type.clone())),
                                ("levelReq".into(), ItemMetadataValue::Number(1.0)),
                            ]),
                            indexed: BTreeMap::from([(1, ItemMetadataValue::Text(line.into()))]),
                        }),
                    )]),
                    indexed: BTreeMap::new(),
                }),
            )
        })
        .collect();
    package.item_loading.modifier_tables.insert(
        family.into(),
        ItemMetadataTable {
            fields: definitions,
            indexed: BTreeMap::new(),
        },
    );
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "caller changes rune construction inputs",
        );
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &bytes).unwrap();
    let xml = format!(
        "<PathOfBuilding2><Items><Item id='caller-runes'>Rarity: Normal\n{base_name}\n{socket_header}: qq\n{rune_header}: {}\n{rune_header}: {}\n</Item></Items></PathOfBuilding2>",
        names[0], names[1]
    );
    fs::write(&input, &xml).unwrap();
    let report = inspect_definitions(&input, temp.path(), Some(&data_path));
    let item = &report["definition_lookup"]["items"]["report"]["items"][0];
    let state = &item["state"];
    assert_eq!(state["base_name"], base_name);
    assert_eq!(state["runes"], serde_json::json!(names));
    assert_eq!(state["sockets"], serde_json::json!([0, 1]));
    assert_eq!(state["item_socket_count"], 2);
    let lines = state["rune_mod_lines"].as_array().unwrap();
    assert_eq!(lines.len(), 1, "{item:#}");
    let line = &lines[0];
    assert_eq!(line["line"], "+18.01 to maximum Life");
    assert!(line["source_line"].is_null());
    assert_eq!(line["augment_type"], augment_type);
    assert_eq!(
        line["order"],
        serde_json::json!({"kind":"finite","value":7.5})
    );
    assert_eq!(
        line["rune_origins"],
        serde_json::json!([
            {"socket_index":1,"slot_key":slot,"bonded":false,"definition_line_index":1,"combined":false},
            {"socket_index":2,"slot_key":slot,"bonded":false,"definition_line_index":1,"combined":true}
        ])
    );
    let calls = state["parser_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2, "{item:#}");
    for (index, text) in ["+12.34 to maximum Life", "+18.01 to maximum Life"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(calls[index]["text"], text);
        assert!(calls[index]["line_index"].is_null());
        assert_eq!(calls[index]["origin"], line["rune_origins"][index]);
        assert_eq!(
            calls[index]["combined"], false,
            "parser argument differs from contribution provenance"
        );
    }
    assert_eq!(item["status"], "pending", "{item:#}");
    assert_eq!(item["pending"]["kind"], "assembly", "{item:#}");
    assert_eq!(
        line["rune_count"],
        serde_json::json!({"kind":"finite","value":2.0})
    );
    assert_eq!(line["socketed_rune_effect_already_applied"], false);
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(
        report["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(
        report["definition_lookup"]["data"]["content_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), bytes);
}

#[test]
fn injected_special_factories_preserve_nil_results_and_generated_rune_origins() {
    use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
    use poe_optimizer_data::modifier_parser::{
        ParserDictionary, ParserFactoryDisposition, ParserFactoryExpr as Expr,
        ParserFactoryField as Field, ParserFactoryLiteral as Literal, ParserValue,
    };
    use std::collections::BTreeMap;

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-factories.xml");
    let data_path = temp.path().join("caller-factory-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    // This test authors legacy recipes and does not inherit program permissions.
    package.modifier_parser.programs = Default::default();
    let base_name = package
        .item_loading
        .bases
        .iter()
        .find(|base| {
            base.field("weapon")
                .is_some_and(|value| !matches!(value, ItemMetadataValue::Boolean(false)))
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let modifier_name = "CallerCallbackMagnitude";
    let modifier_source = "Caller callback source";
    let text = |value: &str| Expr::Literal(Literal::Text(value.into()));
    // Select source-linked recipe slots by capability, never by a game callback
    // identity. The custom package digest binds the caller-authored new bodies.
    let table_id = package
        .modifier_parser
        .factories
        .iter()
        .find_map(|(id, disposition)| match disposition {
            ParserFactoryDisposition::Pure(factory) if factory.provenance.constructor.is_some() => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let nil_id = package
        .modifier_parser
        .factories
        .iter()
        .find_map(|(id, disposition)| {
            (*id != table_id && matches!(disposition, ParserFactoryDisposition::Pure(_)))
                .then_some(*id)
        })
        .unwrap();
    let ParserFactoryDisposition::Pure(factory) = package
        .modifier_parser
        .factories
        .get_mut(&table_id)
        .unwrap()
    else {
        unreachable!()
    };
    factory.parameter_count = 2;
    factory.body = Expr::Table(vec![Field::List(Expr::CreateMod {
        args: vec![
            text(modifier_name),
            text("BASE"),
            Expr::Argument(0),
            text(modifier_source),
            Expr::Literal(Literal::Number(0.0)),
            Expr::Literal(Literal::Number(0.0)),
            Expr::Table(vec![
                Field::Named {
                    key: "type".into(),
                    value: text("CallerCallbackTag"),
                },
                Field::Named {
                    key: "capture".into(),
                    value: Expr::Argument(1),
                },
                Field::Named {
                    key: "enabled".into(),
                    value: Expr::Literal(Literal::Boolean(true)),
                },
            ]),
        ],
    })]);
    let ParserFactoryDisposition::Pure(factory) =
        package.modifier_parser.factories.get_mut(&nil_id).unwrap()
    else {
        unreachable!()
    };
    factory.parameter_count = 0;
    factory.body = Expr::Literal(Literal::Nil);
    factory.provenance.constructor = None;
    let special = package.modifier_parser.dictionaries[&ParserDictionary::Special];
    let fields = &mut package.modifier_parser.tables[special.0 as usize - 1].fields;
    assert!(
        fields
            .insert(
                "^(%d+%.?%d*) caller potency$".into(),
                ParserValue::Callback(table_id),
            )
            .is_none()
    );
    assert!(
        fields
            .insert("^caller omitted$".into(), ParserValue::Callback(nil_id))
            .is_none()
    );

    let family = "Caller Callback Rune Definitions";
    let names = ["Caller Silver Shard", "Caller Copper Shard"];
    let slot = "caller callback weapon slot";
    let rune_header = "CallerFactoryRune";
    let socket_header = "CallerFactorySockets";
    package
        .item_loading
        .policy
        .header_names
        .extend([rune_header.into(), socket_header.into()]);
    let policy = &mut package.item_loading.policy.rune_loading;
    policy.rune_header = rune_header.into();
    policy.socket_header = socket_header.into();
    policy.rune_table = family.into();
    policy.broad_weapon_type = slot.into();
    policy.socket_character_pattern = "q".into();
    policy.item_socket_pattern = "^q$".into();
    policy.numeric_pattern = "(%d+%.?%d*)".into();
    let augment_type = policy.rune_augment_type.clone();
    let definitions = names
        .into_iter()
        .zip(["12.5 caller potency", "5.5 caller potency"])
        .map(|(name, line)| {
            (
                name.into(),
                ItemMetadataValue::Table(ItemMetadataTable {
                    fields: BTreeMap::from([(
                        slot.into(),
                        ItemMetadataValue::Table(ItemMetadataTable {
                            fields: BTreeMap::from([
                                ("type".into(), ItemMetadataValue::Text(augment_type.clone())),
                                ("levelReq".into(), ItemMetadataValue::Number(1.0)),
                            ]),
                            indexed: BTreeMap::from([(1, ItemMetadataValue::Text(line.into()))]),
                        }),
                    )]),
                    indexed: BTreeMap::new(),
                }),
            )
        })
        .collect();
    package.item_loading.modifier_tables.insert(
        family.into(),
        ItemMetadataTable {
            fields: definitions,
            indexed: BTreeMap::new(),
        },
    );
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "caller changes rune construction inputs",
        );
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &bytes).unwrap();
    let xml = format!(
        "<PathOfBuilding2><Items><Item id='caller-table'>Rarity: Normal\n{base_name}\n7 caller potency\n</Item><Item id='caller-nil'>Rarity: Normal\n{base_name}\ncaller omitted\n</Item><Item id='caller-generated'>Rarity: Normal\n{base_name}\n{socket_header}: qq\n{rune_header}: {}\n{rune_header}: {}\n</Item></Items></PathOfBuilding2>",
        names[0], names[1]
    );
    fs::write(&input, &xml).unwrap();
    let report = inspect_definitions(&input, temp.path(), Some(&data_path));
    let loaded = &report["definition_lookup"]["items"]["report"];
    let items = loaded["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    let expected_modifiers = |value: f64, capture: &str| {
        serde_json::json!([{
            "fields": {
                "name": modifier_name, "type": "BASE", "value": value,
                "flags": 0.0, "keywordFlags": 0.0, "source": modifier_source
            },
            "indexed": {"1": {"fields": {
                "type": "CallerCallbackTag", "capture": capture, "enabled": true
            }, "indexed": {}}}
        }])
    };
    let table_state = &items[0]["state"];
    assert_eq!(items[0]["authored_id"], "caller-table");
    let rows = table_state["explicit_mod_lines"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["line"], "7 caller potency");
    assert_eq!(rows[0]["source_line"], 3);
    assert_eq!(rows[0]["modifiers"], expected_modifiers(7.0, "7"));
    assert!(rows[0]["extra"].is_null());
    assert_eq!(table_state["parser_calls"].as_array().unwrap().len(), 1);
    assert_eq!(table_state["parser_calls"][0]["line_index"], 3);
    assert_eq!(table_state["parser_calls"][0]["combined"], false);
    assert!(table_state["parser_calls"][0].get("origin").is_none());

    let nil_state = &items[1]["state"];
    assert_eq!(items[1]["authored_id"], "caller-nil");
    let rows = nil_state["explicit_mod_lines"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["line"], "caller omitted");
    assert_eq!(rows[0]["source_line"], 3);
    assert_eq!(rows[0]["modifiers"], serde_json::json!([]));
    assert_eq!(rows[0]["extra"], "caller omitted");
    assert_eq!(nil_state["parser_calls"].as_array().unwrap().len(), 1);
    assert_eq!(nil_state["parser_calls"][0]["text"], "caller omitted");
    assert_eq!(nil_state["parser_calls"][0]["combined"], false);

    let rune_state = &items[2]["state"];
    assert_eq!(items[2]["authored_id"], "caller-generated");
    assert_eq!(rune_state["runes"], serde_json::json!(names));
    let rows = rune_state["rune_mod_lines"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["line"], "18 caller potency");
    assert_eq!(row["modifiers"], expected_modifiers(18.0, "18"));
    assert!(row["extra"].is_null());
    assert!(row["source_line"].is_null());
    assert_eq!(
        row["rune_origins"],
        serde_json::json!([
            {"socket_index":1,"slot_key":slot,"bonded":false,"definition_line_index":1,"combined":false},
            {"socket_index":2,"slot_key":slot,"bonded":false,"definition_line_index":1,"combined":true}
        ])
    );
    let calls = rune_state["parser_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    for (index, text) in ["12.5 caller potency", "18 caller potency"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(calls[index]["text"], text);
        assert!(calls[index]["line_index"].is_null());
        assert_eq!(calls[index]["origin"], row["rune_origins"][index]);
        assert_eq!(calls[index]["combined"], false);
    }
    for item in items {
        assert_eq!(item["status"], "pending", "{item:#}");
        assert_eq!(item["pending"]["kind"], "assembly", "{item:#}");
    }
    assert_eq!(report["verification"]["item_loading"], "reported");
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(
        report["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(
        loaded["data_identity"]["content_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(
        loaded["implementation_sha256"],
        poe_optimizer_import::item_loading::implementation_fingerprint()
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), bytes);
}

#[test]
fn injected_ordinary_factories_preserve_capture_types_and_conditional_error_stages() {
    use poe_optimizer_data::modifier_parser::{
        ParserCallbackKind, ParserDictionary as Dict, ParserFactoryDisposition,
        ParserFactoryExpr as Expr, ParserFactoryField as Field, ParserFactoryLiteral as Literal,
        ParserValue,
    };

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-ordinary.xml");
    let data_path = temp.path().join("caller-ordinary-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    // This test authors legacy recipes and does not inherit program permissions.
    package.modifier_parser.programs = Default::default();
    let base_name = package
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
                && base.field("flask").is_none()
                && base.field("charm").is_none()
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let (prefix_id, tag_id) = {
        let pure_in = |dictionary, excluded| {
            let table = package.modifier_parser.dictionaries[&dictionary];
            package.modifier_parser.tables[table.0 as usize - 1]
                .fields
                .values()
                .find_map(|value| {
                    let ParserValue::Callback(id) = value else {
                        return None;
                    };
                    (excluded != Some(*id)
                        && matches!(
                            package.modifier_parser.factories.get(id),
                            Some(ParserFactoryDisposition::Pure(_))
                        ))
                    .then_some(*id)
                })
                .unwrap()
        };
        let prefix = pure_in(Dict::PreFlag, None);
        (prefix, pure_in(Dict::ModTag, Some(prefix)))
    };
    let unsupported_id = package
        .modifier_parser
        .factories
        .iter()
        .find_map(|(id, disposition)| {
            (matches!(disposition, ParserFactoryDisposition::Unsupported { .. })
                && matches!(
                    package.modifier_parser.callbacks[id.0 as usize - 1].kind,
                    ParserCallbackKind::Lua { .. }
                ))
            .then_some(*id)
        })
        .unwrap();
    let text = |value: &str| Expr::Literal(Literal::Text(value.into()));
    for (id, parameter_count, fields) in [
        (
            prefix_id,
            1,
            vec![
                Field::Named {
                    key: "type".into(),
                    value: text("CallerPrefixTag"),
                },
                Field::Named {
                    key: "capture".into(),
                    value: Expr::Argument(0),
                },
            ],
        ),
        (
            tag_id,
            2,
            vec![
                Field::Named {
                    key: "type".into(),
                    value: text("CallerOrdinaryTag"),
                },
                Field::Named {
                    key: "first".into(),
                    value: Expr::Argument(0),
                },
                Field::Named {
                    key: "raw".into(),
                    value: Expr::Argument(1),
                },
            ],
        ),
    ] {
        let ParserFactoryDisposition::Pure(factory) =
            package.modifier_parser.factories.get_mut(&id).unwrap()
        else {
            unreachable!()
        };
        factory.parameter_count = parameter_count;
        factory.provenance.constructor = None;
        factory.body = Expr::Table(vec![Field::Named {
            key: "tag".into(),
            value: Expr::Table(fields),
        }]);
    }
    package.modifier_parser.policy.tag_capture_numeric_pattern = "^%d+$".into();
    for (dictionary, pattern, value) in [
        (
            Dict::PreFlag,
            "^caller prefix (%d+) ",
            ParserValue::Callback(prefix_id),
        ),
        (
            Dict::PreFlag,
            "^caller unknown ",
            ParserValue::Callback(unsupported_id),
        ),
        (
            Dict::ModTag,
            "caller mark (%w+)",
            ParserValue::Callback(tag_id),
        ),
        (
            Dict::ModTag,
            "caller amount (%d+)",
            ParserValue::Callback(tag_id),
        ),
        (
            Dict::ModTag,
            "caller pending (%d+)",
            ParserValue::Callback(unsupported_id),
        ),
        (
            Dict::ModTag,
            "caller empty",
            ParserValue::Callback(unsupported_id),
        ),
        (
            Dict::ModName,
            "caller stat",
            ParserValue::Text("CallerOrdinaryStat".into()),
        ),
    ] {
        let table = package.modifier_parser.dictionaries[&dictionary];
        assert!(
            package.modifier_parser.tables[table.0 as usize - 1]
                .fields
                .insert(pattern.into(), value)
                .is_none()
        );
    }
    package.refresh_section_digests().unwrap();
    let data_bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &data_bytes).unwrap();
    let lines = [
        "caller prefix 007 +2 to caller stat caller mark v13 caller amount 13",
        "caller unknown +2 to caller stat",
        "+2 to caller stat caller pending 7",
        "+2 to caller stat caller amount 3 caller pending 7",
        "+2 to caller stat caller empty",
    ];
    let items = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            format!("<Item id='caller-{index}'>Rarity: Normal\n{base_name}\n{line}\n</Item>")
        })
        .collect::<String>();
    let xml = format!("<PathOfBuilding2><Items>{items}</Items></PathOfBuilding2>");
    fs::write(&input, &xml).unwrap();
    let report = inspect_definitions(&input, temp.path(), Some(&data_path));
    let loaded = &report["definition_lookup"]["items"]["report"];
    let items = loaded["items"].as_array().unwrap();
    assert_eq!(items.len(), lines.len());
    let success = &items[0];
    assert_eq!(success["status"], "pending", "{success:#}");
    assert_eq!(success["pending"]["kind"], "assembly", "{success:#}");
    let rows = success["state"]["explicit_mod_lines"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["line"], lines[0]);
    assert_eq!(rows[0]["source_line"], 3);
    assert!(rows[0]["extra"].is_null());
    assert_eq!(
        rows[0]["modifiers"],
        serde_json::json!([{
            "fields": {"name":"CallerOrdinaryStat","type":"BASE","value":2.0,"flags":0.0,"keywordFlags":0.0},
            "indexed": {
                "1":{"fields":{"type":"CallerPrefixTag","capture":"007"},"indexed":{}},
                "2":{"fields":{"type":"CallerOrdinaryTag","first":"v13","raw":"v13"},"indexed":{}},
                "3":{"fields":{"type":"CallerOrdinaryTag","first":13.0,"raw":"13"},"indexed":{}}
            }
        }])
    );
    for (item, stage) in items[1..4].iter().zip([
        "prefix callback",
        "modifier tag callback",
        "second modifier tag callback",
    ]) {
        assert_eq!(item["status"], "pending", "{item:#}");
        assert_eq!(item["pending"]["kind"], "modifier_parser", "{item:#}");
        assert!(
            item["pending"]["message"].as_str().unwrap().contains(stage),
            "{item:#}"
        );
    }
    let error = &items[4];
    assert_eq!(error["status"], "source_error", "{error:#}");
    assert!(error["pending"].is_null());
    assert!(
        error["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|instruction| instruction["error"].is_string())
    );
    assert_eq!(
        error["instructions"].as_array().unwrap().last().unwrap()["status"],
        "not_executed"
    );
    for (index, item) in items.iter().enumerate() {
        assert_eq!(item["authored_id"], format!("caller-{index}"));
        let calls = item["state"]["parser_calls"].as_array().unwrap();
        assert_eq!(calls.len(), 1, "{item:#}");
        assert_eq!(calls[0]["text"], lines[index]);
        assert_eq!(calls[0]["line_index"], 3);
        assert_eq!(calls[0]["combined"], false);
        assert!(calls[0].get("origin").is_none());
    }
    assert_eq!(report["verification"]["item_loading"], "reported");
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(
        report["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(
        loaded["data_identity"]["content_sha256"],
        format!("{:x}", Sha256::digest(&data_bytes))
    );
    assert_eq!(
        loaded["implementation_sha256"],
        poe_optimizer_import::item_loading::implementation_fingerprint()
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
}

#[test]
fn injected_string_factories_preserve_bytes_and_distinguish_opaque_methods() {
    use poe_optimizer_data::modifier_parser::{
        ParserDictionary as Dict, ParserFactoryDisposition, ParserFactoryExpr as Expr,
        ParserFactoryField as Field, ParserFactoryLiteral as Literal, ParserValue,
    };

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-strings.xml");
    let data_path = temp.path().join("caller-string-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    // This test authors legacy recipes and does not inherit program permissions.
    package.modifier_parser.programs = Default::default();
    let base_name = package
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
                && base.field("flask").is_none()
                && base.field("charm").is_none()
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let helper = package.modifier_parser.helpers["firstToUpper"];
    // Keep the real captured helper binding while the caller's package identity
    // supplies these expression bodies. Selection is by capability, not game ID.
    let pure_helpers = |dictionary| {
        let table = package.modifier_parser.dictionaries[&dictionary];
        package.modifier_parser.tables[table.0 as usize - 1]
            .fields
            .values()
            .filter_map(|value| {
                let ParserValue::Callback(id) = value else {
                    return None;
                };
                let Some(ParserFactoryDisposition::Pure(factory)) =
                    package.modifier_parser.factories.get(id)
                else {
                    return None;
                };
                (factory.provenance.constructor.is_some()
                    && package.modifier_parser.callbacks[id.0 as usize - 1]
                        .upvalues
                        .iter()
                        .any(|upvalue| {
                            upvalue.name == "firstToUpper"
                                && upvalue.value == ParserValue::Callback(helper)
                        }))
                .then_some(*id)
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    let special_ids = pure_helpers(Dict::Special)
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();
    assert_eq!(special_ids.len(), 3);
    let tag_table = package.modifier_parser.dictionaries[&Dict::ModTag];
    let tag_id = package.modifier_parser.tables[tag_table.0 as usize - 1]
        .fields
        .values()
        .find_map(|value| {
            let ParserValue::Callback(id) = value else {
                return None;
            };
            (!special_ids.contains(id)
                && matches!(
                    package.modifier_parser.factories.get(id),
                    Some(ParserFactoryDisposition::Pure(_))
                )
                && package.modifier_parser.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|upvalue| {
                        upvalue.name == "firstToUpper"
                            && upvalue.value == ParserValue::Callback(helper)
                    }))
            .then_some(*id)
        })
        .unwrap();
    let text = |value: &str| Expr::Literal(Literal::Text(value.into()));
    let concat = |left, right| Expr::Concat {
        left: Box::new(left),
        right: Box::new(right),
    };
    let upper = |value| Expr::FirstToUpper {
        helper,
        value: Box::new(value),
    };
    let named = |key: &str, value| Field::Named {
        key: key.into(),
        value,
    };
    let constants = package.modifier_parser.policy.mod_flags;
    assert!(
        package.modifier_parser.tables[constants.0 as usize - 1]
            .fields
            .insert("CallerOpaqueMethod".into(), ParserValue::Callback(helper))
            .is_none()
    );
    let success_body = Expr::Table(vec![Field::List(Expr::CreateMod {
        args: vec![
            concat(text("Caller"), upper(Expr::Argument(2))),
            text("BASE"),
            Expr::Argument(0),
            text("Caller string source"),
            Expr::Literal(Literal::Number(0.0)),
            Expr::Literal(Literal::Number(0.0)),
            Expr::Table(vec![
                named("type", text("CallerStringBytes")),
                named("raw", concat(text("nul\0é/"), Expr::Argument(1))),
                named("numeric", concat(text("n="), Expr::Argument(0))),
                named("byte_positions", upper(text("a\0éz"))),
            ]),
        ],
    })]);
    let error_body = Expr::Table(vec![named(
        "failure",
        upper(Expr::Literal(Literal::Boolean(false))),
    )]);
    let opaque_body = Expr::Table(vec![named(
        "opaque",
        upper(Expr::Table(vec![named(
            "gsub",
            Expr::ConstantField {
                table: constants,
                key: "CallerOpaqueMethod".into(),
            },
        )])),
    )]);
    let tag_body = Expr::Table(vec![named(
        "tag",
        Expr::Table(vec![
            named("type", text("CallerStringTag")),
            named("label", upper(Expr::Argument(0))),
            named("raw", concat(text("raw:"), Expr::Argument(1))),
        ]),
    )]);
    for (id, parameters, body, constructor) in [
        (special_ids[0], 3, success_body, true),
        (special_ids[1], 0, error_body, false),
        (special_ids[2], 0, opaque_body, false),
        (tag_id, 2, tag_body, false),
    ] {
        let ParserFactoryDisposition::Pure(factory) =
            package.modifier_parser.factories.get_mut(&id).unwrap()
        else {
            unreachable!()
        };
        factory.parameter_count = parameters;
        factory.body = body;
        if !constructor {
            factory.provenance.constructor = None;
        }
    }
    for (dictionary, pattern, value) in [
        (
            Dict::Special,
            "^caller (%d+) bytes (.+)$",
            ParserValue::Callback(special_ids[0]),
        ),
        (
            Dict::Special,
            "^caller bad helper$",
            ParserValue::Callback(special_ids[1]),
        ),
        (
            Dict::Special,
            "^caller opaque helper$",
            ParserValue::Callback(special_ids[2]),
        ),
        (
            Dict::ModTag,
            "caller marker (%w+)",
            ParserValue::Callback(tag_id),
        ),
        (
            Dict::ModName,
            "caller string stat",
            ParserValue::Text("CallerStringAmount".into()),
        ),
    ] {
        let table = package.modifier_parser.dictionaries[&dictionary];
        assert!(
            package.modifier_parser.tables[table.0 as usize - 1]
                .fields
                .insert(pattern.into(), value)
                .is_none()
        );
    }
    let lines = [
        "caller 007 bytes aéz",
        "+2 to caller string stat caller marker ab",
        "caller bad helper",
        "caller opaque helper",
    ];
    let items = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            format!("<Item id='caller-{index}'>Rarity: Normal\n{base_name}\n{line}\n</Item>")
        })
        .collect::<String>();
    let xml = format!("<PathOfBuilding2><Items>{items}</Items></PathOfBuilding2>");
    fs::write(&input, &xml).unwrap();
    let mut digests = Vec::new();
    for (pattern, expected_name, expected_bytes, expected_tag) in [
        ("(.)", "CallerAéZ", "A\0éZ", "AB"),
        ("()(.)", "Caller1234", "12345", "12"),
    ] {
        package.modifier_parser.policy.first_to_upper_pattern = pattern.into();
        package.refresh_section_digests().unwrap();
        let data_bytes = package.canonical_bytes().unwrap();
        let digest = format!("{:x}", Sha256::digest(&data_bytes));
        fs::write(&data_path, &data_bytes).unwrap();
        let report = inspect_definitions(&input, temp.path(), Some(&data_path));
        let loaded = &report["definition_lookup"]["items"]["report"];
        let items = loaded["items"].as_array().unwrap();
        assert_eq!(items.len(), lines.len());
        let special_rows = items[0]["state"]["explicit_mod_lines"].as_array().unwrap();
        assert_eq!(special_rows.len(), 1, "{items:#?}");
        assert_eq!(
            special_rows[0]["modifiers"],
            serde_json::json!([{
                "fields": {"name":expected_name,"type":"BASE","value":7.0,"flags":0.0,"keywordFlags":0.0,"source":"Caller string source"},
                "indexed": {"1":{"fields":{"type":"CallerStringBytes","raw":"nul\0é/007","numeric":"n=7","byte_positions":expected_bytes},"indexed":{}}}
            }]),
            "configured pattern {pattern}"
        );
        let tag_rows = items[1]["state"]["explicit_mod_lines"].as_array().unwrap();
        assert_eq!(tag_rows.len(), 1);
        assert_eq!(
            tag_rows[0]["modifiers"],
            serde_json::json!([{
                "fields": {"name":"CallerStringAmount","type":"BASE","value":2.0,"flags":0.0,"keywordFlags":0.0},
                "indexed": {"1":{"fields":{"type":"CallerStringTag","label":expected_tag,"raw":"raw:ab"},"indexed":{}}}
            }])
        );
        for (index, rows) in [special_rows, tag_rows].into_iter().enumerate() {
            assert_eq!(items[index]["status"], "pending", "{}", items[index]);
            assert_eq!(items[index]["pending"]["kind"], "assembly");
            assert_eq!(rows[0]["line"], lines[index]);
            assert_eq!(rows[0]["source_line"], 3);
            assert!(rows[0]["extra"].is_null());
        }
        let error = &items[2];
        assert_eq!(error["status"], "source_error", "{error:#}");
        assert!(error["pending"].is_null());
        assert!(
            error["instructions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|instruction| instruction["error"].is_string())
        );
        assert_eq!(
            error["instructions"].as_array().unwrap().last().unwrap()["status"],
            "not_executed"
        );
        let opaque = &items[3];
        assert_eq!(opaque["status"], "pending", "{opaque:#}");
        assert_eq!(opaque["pending"]["kind"], "modifier_parser");
        assert!(
            opaque["pending"]["message"]
                .as_str()
                .unwrap()
                .contains("firstToUpper receiver method"),
            "{opaque:#}"
        );
        for (index, item) in items.iter().enumerate() {
            assert_eq!(item["authored_id"], format!("caller-{index}"));
            let calls = item["state"]["parser_calls"].as_array().unwrap();
            assert_eq!(calls.len(), 1, "{item:#}");
            assert_eq!(calls[0]["text"], lines[index]);
            assert_eq!(calls[0]["line_index"], 3);
            assert_eq!(calls[0]["combined"], false);
            assert!(calls[0].get("origin").is_none());
        }
        assert_eq!(report["verification"]["item_loading"], "reported");
        assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
        assert_eq!(
            report["definition_lookup"]["data_trust"]["status"],
            "custom_unreviewed"
        );
        assert_eq!(loaded["data_identity"]["content_sha256"], digest);
        assert_eq!(
            loaded["implementation_sha256"],
            poe_optimizer_import::item_loading::implementation_fingerprint()
        );
        assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
        assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
        digests.push(digest);
    }
    assert_ne!(
        digests[0], digests[1],
        "injected pattern has its own identity"
    );
}

#[test]
fn injected_flag_factories_preserve_prefixes_and_variadic_slots() {
    use poe_optimizer_data::modifier_parser::{
        ParserDictionary as Dict, ParserFactoryDisposition, ParserFactoryExpr as Expr,
        ParserFactoryField as Field, ParserFactoryLiteral as Literal, ParserValue,
    };

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-flags.xml");
    let data_path = temp.path().join("caller-flag-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    // This test authors legacy recipes and does not inherit program permissions.
    package.modifier_parser.programs = Default::default();
    let base_name = package
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
                && base.field("flask").is_none()
                && base.field("charm").is_none()
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let helper = package.modifier_parser.helpers["flag"];
    let special = package.modifier_parser.dictionaries[&Dict::Special];
    // A Flag-only owner has no direct constructor provenance. Its captured
    // helper supplies the separate authenticated constructor path.
    let ids = package.modifier_parser.tables[special.0 as usize - 1]
        .fields
        .values()
        .filter_map(|value| {
            let ParserValue::Callback(id) = value else {
                return None;
            };
            let Some(ParserFactoryDisposition::Pure(factory)) =
                package.modifier_parser.factories.get(id)
            else {
                return None;
            };
            (factory.provenance.constructor.is_none()
                && package.modifier_parser.callbacks[id.0 as usize - 1]
                    .upvalues
                    .iter()
                    .any(|upvalue| {
                        upvalue.name == "flag" && upvalue.value == ParserValue::Callback(helper)
                    }))
            .then_some(*id)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .take(4)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 4);
    let text = |value: &str| Expr::Literal(Literal::Text(value.into()));
    let number = |value| Expr::Literal(Literal::Number(value));
    let nil = || Expr::Literal(Literal::Nil);
    let named = |key: &str, value| Field::Named {
        key: key.into(),
        value,
    };
    let flag = |args| Expr::Flag { helper, args };
    let constants = package.modifier_parser.policy.mod_flags;
    assert!(
        package.modifier_parser.tables[constants.0 as usize - 1]
            .fields
            .insert("CallerFlagOpaque".into(), ParserValue::Callback(helper))
            .is_none()
    );
    let success = Expr::Table(vec![
        Field::List(flag(vec![
            Expr::Concat {
                left: Box::new(text("Caller/")),
                right: Box::new(Expr::Argument(2)),
            },
            nil(),
            number(16.0),
            number(32.0),
            Expr::Table(vec![
                named("type", text("CallerFlagTag")),
                named("raw", Expr::Argument(1)),
                named("numeric", Expr::Argument(0)),
            ]),
            nil(),
            Expr::Table(vec![named("type", text("CallerAfterHoleTag"))]),
            nil(),
        ])),
        Field::List(flag(vec![
            text("Caller/TextFlags"),
            text("Caller flag source"),
            text("64"),
            number(128.0),
            Expr::Table(vec![named("type", text("CallerTextFlagTag"))]),
        ])),
    ]);
    let empty = Expr::Table(vec![Field::List(flag(vec![]))]);
    let opaque = Expr::Table(vec![Field::List(flag(vec![
        text("Caller/Opaque"),
        nil(),
        number(0.0),
        number(0.0),
        Expr::Table(vec![named(
            "opaque",
            Expr::ConstantField {
                table: constants,
                key: "CallerFlagOpaque".into(),
            },
        )]),
    ]))]);
    let error = Expr::Table(vec![Field::List(flag(vec![
        text("Caller/Bad"),
        Expr::Negate(Box::new(Expr::Literal(Literal::Boolean(false)))),
    ]))]);
    for (id, parameters, body) in [
        (ids[0], 3, success),
        (ids[1], 0, empty),
        (ids[2], 0, opaque),
        (ids[3], 0, error),
    ] {
        let ParserFactoryDisposition::Pure(factory) =
            package.modifier_parser.factories.get_mut(&id).unwrap()
        else {
            unreachable!()
        };
        assert!(factory.provenance.constructor.is_none());
        factory.parameter_count = parameters;
        factory.body = body;
    }
    for (pattern, id) in [
        ("^caller flag (%d+) (.+)$", ids[0]),
        ("^caller empty flag$", ids[1]),
        ("^caller opaque flag$", ids[2]),
        ("^caller bad flag$", ids[3]),
    ] {
        assert!(
            package.modifier_parser.tables[special.0 as usize - 1]
                .fields
                .insert(pattern.into(), ParserValue::Callback(id))
                .is_none()
        );
    }
    let lines = [
        "caller flag 007 alpha",
        "caller empty flag",
        "caller opaque flag",
        "caller bad flag",
    ];
    let items = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            format!("<Item id='caller-{index}'>Rarity: Normal\n{base_name}\n{line}\n</Item>")
        })
        .collect::<String>();
    let xml = format!("<PathOfBuilding2><Items>{items}</Items></PathOfBuilding2>");
    fs::write(&input, &xml).unwrap();
    let mut digests = Vec::new();
    for (mod_type, mod_value) in [("CALLER_SWITCH", false), ("CALLER_TOGGLE", true)] {
        package.modifier_parser.policy.flag_mod_type = mod_type.into();
        package.modifier_parser.policy.flag_mod_value = mod_value;
        package.refresh_section_digests().unwrap();
        let data_bytes = package.canonical_bytes().unwrap();
        let digest = format!("{:x}", Sha256::digest(&data_bytes));
        fs::write(&data_path, &data_bytes).unwrap();
        let report = inspect_definitions(&input, temp.path(), Some(&data_path));
        let loaded = &report["definition_lookup"]["items"]["report"];
        let items = loaded["items"].as_array().unwrap();
        assert_eq!(items.len(), lines.len());
        let rows = items[0]["state"]["explicit_mod_lines"].as_array().unwrap();
        assert_eq!(rows.len(), 1, "{items:#?}");
        assert_eq!(
            rows[0]["modifiers"],
            serde_json::json!([
                {
                    "fields":{"name":"Caller/alpha","type":mod_type,"value":mod_value,"flags":16.0,"keywordFlags":32.0},
                    "indexed":{
                        "1":{"fields":{"type":"CallerFlagTag","raw":"007","numeric":7.0},"indexed":{}},
                        "3":{"fields":{"type":"CallerAfterHoleTag"},"indexed":{}}
                    }
                },
                {
                    "fields":{"name":"Caller/TextFlags","type":mod_type,"value":mod_value,"source":"Caller flag source","flags":0.0,"keywordFlags":128.0},
                    "indexed":{"1":{"fields":{"type":"CallerTextFlagTag"},"indexed":{}}}
                }
            ])
        );
        let empty_rows = items[1]["state"]["explicit_mod_lines"].as_array().unwrap();
        assert_eq!(empty_rows.len(), 1);
        assert_eq!(
            empty_rows[0]["modifiers"],
            serde_json::json!([{
                "fields":{"type":mod_type,"value":mod_value,"flags":0.0,"keywordFlags":0.0},
                "indexed":{}
            }])
        );
        for (index, rows) in [rows, empty_rows].into_iter().enumerate() {
            assert_eq!(items[index]["status"], "pending", "{}", items[index]);
            assert_eq!(items[index]["pending"]["kind"], "assembly");
            assert_eq!(rows[0]["line"], lines[index]);
            assert_eq!(rows[0]["source_line"], 3);
            assert!(rows[0]["extra"].is_null());
        }
        let opaque = &items[2];
        assert_eq!(opaque["status"], "pending", "{opaque:#}");
        assert_eq!(opaque["pending"]["kind"], "modifier_parser");
        assert!(
            opaque["pending"]["message"]
                .as_str()
                .unwrap()
                .contains("parser callback output requires its captured-state item assembly model"),
            "{opaque:#}"
        );
        let error = &items[3];
        assert_eq!(error["status"], "source_error", "{error:#}");
        assert!(error["pending"].is_null());
        assert!(
            error["instructions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|instruction| instruction["error"].is_string())
        );
        assert_eq!(
            error["instructions"].as_array().unwrap().last().unwrap()["status"],
            "not_executed"
        );
        for (index, item) in items.iter().enumerate() {
            assert_eq!(item["authored_id"], format!("caller-{index}"));
            let calls = item["state"]["parser_calls"].as_array().unwrap();
            assert_eq!(calls.len(), 1, "{item:#}");
            assert_eq!(calls[0]["text"], lines[index]);
            assert_eq!(calls[0]["line_index"], 3);
            assert_eq!(calls[0]["combined"], false);
            assert!(calls[0].get("origin").is_none());
        }
        assert_eq!(report["verification"]["item_loading"], "reported");
        assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
        assert_eq!(
            report["definition_lookup"]["data_trust"]["status"],
            "custom_unreviewed"
        );
        assert_eq!(loaded["data_identity"]["content_sha256"], digest);
        assert_eq!(
            loaded["implementation_sha256"],
            poe_optimizer_import::item_loading::implementation_fingerprint()
        );
        assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
        assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
        digests.push(digest);
    }
    assert_ne!(
        digests[0], digests[1],
        "injected prefix has its own identity"
    );
}
#[test]
fn injected_number_factories_preserve_raw_inputs_nil_and_error_order() {
    use poe_optimizer_data::modifier_parser::{
        ParserDictionary as Dict, ParserFactoryDisposition, ParserFactoryExpr as Expr,
        ParserFactoryField as Field, ParserFactoryLiteral as Literal, ParserValue,
    };

    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-numbers.xml");
    let data_path = temp.path().join("caller-number-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    // This test authors legacy recipes and does not inherit program permissions.
    package.modifier_parser.programs = Default::default();
    let base_name = package
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
                && base.field("flask").is_none()
                && base.field("charm").is_none()
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let special = package.modifier_parser.dictionaries[&Dict::Special];
    // Choose source-linked constructor owners by capability. Authored recipe
    // changes receive a custom package identity, not an invented game identity.
    let ids = package.modifier_parser.tables[special.0 as usize - 1]
        .fields
        .values()
        .filter_map(|value| {
            let ParserValue::Callback(id) = value else {
                return None;
            };
            matches!(
                package.modifier_parser.factories.get(id),
                Some(ParserFactoryDisposition::Pure(factory))
                    if factory.provenance.constructor.is_some()
            )
            .then_some(*id)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    let ParserFactoryDisposition::Pure(error_factory) = &package.modifier_parser.factories[&ids[1]]
    else {
        unreachable!()
    };
    let constructor = error_factory.provenance.constructor.unwrap();
    let opaque_upvalue = package.modifier_parser.callbacks[ids[1].0 as usize - 1]
        .upvalues
        .iter()
        .position(|upvalue| upvalue.value == ParserValue::Callback(constructor))
        .unwrap();
    let constants = package.modifier_parser.policy.mod_flags;
    assert!(
        package.modifier_parser.tables[constants.0 as usize - 1]
            .fields
            .insert(
                "CallerNumericOpaque".into(),
                ParserValue::Callback(constructor)
            )
            .is_none()
    );
    let text = |value: &str| Expr::Literal(Literal::Text(value.into()));
    let number = |value| Expr::Literal(Literal::Number(value));
    let nil = || Expr::Literal(Literal::Nil);
    let convert = |value| Expr::ToNumber {
        value: Box::new(value),
    };
    let named = |key: &str, value| Field::Named {
        key: key.into(),
        value,
    };
    let ParserFactoryDisposition::Pure(error_factory) =
        package.modifier_parser.factories.get_mut(&ids[1]).unwrap()
    else {
        unreachable!()
    };
    error_factory.parameter_count = 0;
    error_factory.body = Expr::Table(vec![Field::List(Expr::CreateMod {
        args: vec![
            text("CallerNumericError"),
            text("CALLER_NUMBER"),
            convert(Expr::Negate(Box::new(Expr::Literal(Literal::Boolean(
                false,
            ))))),
            // This later captured function would be deferred if evaluated.
            // The earlier child arithmetic error must win before conversion.
            Expr::CapturedScalar {
                upvalue: u16::try_from(opaque_upvalue).unwrap(),
            },
        ],
    })]);
    for (pattern, id) in [
        ("^caller number (.+)$", ids[0]),
        ("^caller bad number$", ids[1]),
    ] {
        assert!(
            package.modifier_parser.tables[special.0 as usize - 1]
                .fields
                .insert(pattern.into(), ParserValue::Callback(id))
                .is_none()
        );
    }
    let cases = [
        ("007", Some(7.0)),
        ("0x1.8p1", Some(3.0)),
        ("0b101", Some(5.0)),
        ("-2.5e1", Some(-25.0)),
        ("not-a-number", None),
    ];
    let mut lines = cases
        .iter()
        .map(|(raw, _)| format!("caller number {raw}"))
        .collect::<Vec<_>>();
    lines.extend(["caller number 1e999".into(), "caller bad number".into()]);
    let items = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            format!("<Item id='caller-{index}'>Rarity: Normal\n{base_name}\n{line}\n</Item>")
        })
        .collect::<String>();
    let xml = format!("<PathOfBuilding2><Items>{items}</Items></PathOfBuilding2>");
    fs::write(&input, &xml).unwrap();
    let mut digests = Vec::new();
    for (literal, expected_literal) in [("0x1.8p1", 3.0), ("0b101", 5.0)] {
        let ParserFactoryDisposition::Pure(factory) =
            package.modifier_parser.factories.get_mut(&ids[0]).unwrap()
        else {
            unreachable!()
        };
        factory.parameter_count = 3;
        factory.body = Expr::Table(vec![Field::List(Expr::CreateMod {
            args: vec![
                text("CallerNumericValue"),
                text("CALLER_NUMBER"),
                convert(Expr::Argument(1)),
                text("Caller number source"),
                number(0.0),
                number(0.0),
                Expr::Table(vec![
                    named("type", text("CallerNumberTag")),
                    named("raw", Expr::Argument(1)),
                    named("convertedTwice", convert(convert(Expr::Argument(1)))),
                    named("literalRaw", text(literal)),
                    named("literalConverted", convert(text(literal))),
                    named("nulRaw", text("12\0tail")),
                    named("nulConverted", convert(text("12\0tail"))),
                    named("utf8Raw", text("é")),
                    named("utf8Converted", convert(text("é"))),
                    named("nilConverted", convert(nil())),
                    named(
                        "falseConverted",
                        convert(Expr::Literal(Literal::Boolean(false))),
                    ),
                    named("tableConverted", convert(Expr::Table(vec![]))),
                    named(
                        "functionConverted",
                        convert(Expr::ConstantField {
                            table: constants,
                            key: "CallerNumericOpaque".into(),
                        }),
                    ),
                    named("missingConverted", convert(Expr::Argument(2))),
                    named("negativeZero", convert(number(-0.0))),
                    Field::List(number(1.0)),
                    Field::List(convert(nil())),
                    Field::List(number(3.0)),
                ]),
            ],
        })]);
        package.refresh_section_digests().unwrap();
        let data_bytes = package.canonical_bytes().unwrap();
        let digest = format!("{:x}", Sha256::digest(&data_bytes));
        fs::write(&data_path, &data_bytes).unwrap();
        let report = inspect_definitions(&input, temp.path(), Some(&data_path));
        let loaded = &report["definition_lookup"]["items"]["report"];
        let items = loaded["items"].as_array().unwrap();
        assert_eq!(items.len(), lines.len());
        for (index, (raw, expected)) in cases.iter().enumerate() {
            let item = &items[index];
            assert_eq!(item["status"], "pending", "{item:#}");
            assert_eq!(item["pending"]["kind"], "assembly", "{item:#}");
            let rows = item["state"]["explicit_mod_lines"].as_array().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["line"], lines[index]);
            assert_eq!(rows[0]["source_line"], 3);
            assert!(rows[0]["extra"].is_null());
            let mut modifier = serde_json::json!({
                "fields":{"name":"CallerNumericValue","type":"CALLER_NUMBER","source":"Caller number source","flags":0.0,"keywordFlags":0.0},
                "indexed":{"1":{
                    "fields":{"type":"CallerNumberTag","raw":raw,"literalRaw":literal,"literalConverted":expected_literal,"nulRaw":"12\0tail","utf8Raw":"é","negativeZero":-0.0},
                    "indexed":{"1":1.0,"3":3.0}
                }}
            });
            if let Some(value) = expected {
                modifier["fields"]["value"] = serde_json::json!(value);
                modifier["indexed"]["1"]["fields"]["convertedTwice"] = serde_json::json!(value);
            }
            assert_eq!(rows[0]["modifiers"], serde_json::json!([modifier]));
            assert_eq!(
                rows[0]["modifiers"][0]["indexed"]["1"]["fields"]["negativeZero"]
                    .as_f64()
                    .unwrap()
                    .to_bits(),
                (-0.0_f64).to_bits()
            );
        }
        let nonfinite = &items[cases.len()];
        assert_eq!(nonfinite["status"], "pending", "{nonfinite:#}");
        assert_eq!(nonfinite["pending"]["kind"], "modifier_parser");
        assert!(
            nonfinite["pending"]["message"]
                .as_str()
                .unwrap()
                .contains("non-finite parser output requires a richer item assembly model")
        );
        let error = items.last().unwrap();
        assert_eq!(error["status"], "source_error", "{error:#}");
        assert!(error["pending"].is_null());
        assert!(
            error["instructions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|instruction| instruction["error"]
                    .as_str()
                    .is_some_and(|message| message.contains("arithmetic on a non-number"))),
            "{error:#}"
        );
        for (index, item) in items.iter().enumerate() {
            assert_eq!(item["authored_id"], format!("caller-{index}"));
            let calls = item["state"]["parser_calls"].as_array().unwrap();
            assert_eq!(calls.len(), 1, "{item:#}");
            assert_eq!(calls[0]["text"], lines[index]);
            assert_eq!(calls[0]["line_index"], 3);
            assert_eq!(calls[0]["combined"], false);
            assert!(calls[0].get("origin").is_none());
        }
        assert_eq!(report["verification"]["item_loading"], "reported");
        assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
        assert_eq!(
            report["definition_lookup"]["data_trust"]["status"],
            "custom_unreviewed"
        );
        assert_eq!(loaded["data_identity"]["content_sha256"], digest);
        assert_eq!(
            loaded["implementation_sha256"],
            poe_optimizer_import::item_loading::implementation_fingerprint()
        );
        assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
        assert_eq!(fs::read(&data_path).unwrap(), data_bytes);
        digests.push(digest);
    }
    assert_ne!(
        digests[0], digests[1],
        "injected numeric source has its own identity"
    );
}

#[test]
fn injected_typed_programs_change_cli_results_and_require_explicit_permission() {
    use poe_optimizer_data::item_loading::ItemMetadataValue;
    use poe_optimizer_data::modifier_parser::*;
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller-typed.xml");
    let data_path = temp.path().join("caller-program-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let base = package
        .item_loading
        .bases
        .iter()
        .find(|base| {
            base.field("weapon")
                .is_some_and(|v| !matches!(v, ItemMetadataValue::Boolean(false)))
                && !base.name.contains(['&', '<', '>'])
        })
        .unwrap()
        .name
        .clone();
    let xml = format!(
        "<PathOfBuilding2><Items><Item id='caller-typed'>Rarity: Rare\nCaller Item\n{base}\n7 caller typed\n</Item></Items></PathOfBuilding2>"
    );
    fs::write(&input, &xml).unwrap();
    let id = package
        .modifier_parser
        .programs
        .admissions
        .iter()
        .find(|(_, a)| a.role == ParserProgramRole::Special)
        .map(|(id, _)| *id)
        .unwrap();
    let dictionary = package.modifier_parser.dictionaries[&ParserDictionary::Special];
    package.modifier_parser.tables[dictionary.0 as usize - 1]
        .fields
        .insert("^(%d+) caller typed$".into(), ParserValue::Callback(id));
    let roles = package
        .modifier_parser
        .programs
        .admissions
        .iter()
        .map(|(id, a)| (*id, a.role))
        .collect::<Vec<_>>();
    let mut identities = Vec::new();
    for label in ["Caller First", "Caller Second"] {
        let program = package
            .modifier_parser
            .programs
            .data
            .programs
            .iter_mut()
            .find(|p| p.callback == id)
            .unwrap();
        let location = ParserProgramLocation {
            start: program.provenance.function_start,
            end: program.provenance.function_start + 1,
        };
        let expression = |operation| ParserProgramExpr {
            location,
            operation,
        };
        let text = |value: &str| {
            expression(ParserProgramExprKind::Bytes {
                value: value.as_bytes().to_vec(),
            })
        };
        program.bindings.clear();
        program.body = vec![ParserProgramStatement {
            location,
            operation: ParserProgramStatementKind::Return {
                values: ParserProgramValueList {
                    values: vec![expression(ParserProgramExprKind::Table {
                        fields: vec![ParserProgramField::List {
                            value: expression(ParserProgramExprKind::Table {
                                fields: vec![
                                    ParserProgramField::Named {
                                        key: "name".into(),
                                        value: text(label),
                                    },
                                    ParserProgramField::Named {
                                        key: "type".into(),
                                        value: text("BASE"),
                                    },
                                    ParserProgramField::Named {
                                        key: "value".into(),
                                        value: expression(ParserProgramExprKind::Local {
                                            local: 0,
                                        }),
                                    },
                                ],
                            }),
                        }],
                    })],
                    tail: None,
                },
            },
        }];
        package.modifier_parser.programs.admissions = roles
            .iter()
            .map(|(id, role)| {
                let program = package
                    .modifier_parser
                    .programs
                    .data
                    .programs
                    .iter()
                    .find(|p| p.callback == *id)
                    .unwrap();
                (
                    *id,
                    ParserProgramAdmission::bind(
                        &package.modifier_parser,
                        program,
                        *role,
                        "caller-authored CLI experiment",
                    )
                    .unwrap(),
                )
            })
            .collect();
        package.refresh_section_digests().unwrap();
        fs::write(&data_path, package.canonical_bytes().unwrap()).unwrap();
        let report = inspect_definitions(&input, temp.path(), Some(&data_path));
        let item = &report["definition_lookup"]["items"]["report"]["items"][0];
        assert_eq!(item["pending"]["kind"], "assembly", "{item:#}");
        let fields = &item["state"]["explicit_mod_lines"][0]["modifiers"][0]["fields"];
        assert_eq!(fields["name"], label);
        assert_eq!(fields["value"].as_f64(), Some(7.0));
        assert_eq!(
            report["definition_lookup"]["data_trust"]["status"],
            "custom_unreviewed"
        );
        identities.push(
            report["definition_lookup"]["items"]["report"]["data_identity"]["content_sha256"]
                .clone(),
        );
    }
    assert_ne!(identities[0], identities[1]);
    package.modifier_parser.programs.admissions.clear();
    package.refresh_section_digests().unwrap();
    fs::write(&data_path, package.canonical_bytes().unwrap()).unwrap();
    let report = inspect_definitions(&input, temp.path(), Some(&data_path));
    assert_eq!(
        report["definition_lookup"]["items"]["report"]["items"][0]["pending"]["kind"],
        "modifier_parser"
    );
    assert_eq!(fs::read(&input).unwrap(), xml.as_bytes());
}
