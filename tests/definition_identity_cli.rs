//! Selected definition identities survive interchange without choosing numeric behavior by name.
use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy, bundled_snapshot,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden-brutality.xml");

fn evaluate(directory: &Path, input: &Path, data: Option<&Path>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(directory)
        .arg("evaluate")
        .arg(input)
        .args(["--backend", "native", "--metric", "player.selected_hit_dps"]);
    if let Some(data) = data {
        command.arg("--data").arg(data);
    }
    command
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn write_package(directory: &Path, mut package: GameDataPackage) -> (PathBuf, Value) {
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let snapshot =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    let identity = serde_json::to_value(snapshot.identity()).unwrap();
    let path = directory.join("custom.json");
    fs::write(&path, bytes).unwrap();
    (path, identity)
}

fn attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn with_disabled_quests(xml: &str, keys: &[String]) -> String {
    let inputs = keys
        .iter()
        .map(|key| {
            format!(
                "      <Input name=\"{}\" boolean=\"false\"/>\n",
                attribute(key)
            )
        })
        .collect::<String>();
    assert_eq!(xml.matches("    </ConfigSet>").count(), 1);
    xml.replace("    </ConfigSet>", &format!("{inputs}    </ConfigSet>"))
}

// This compatibility fixture renames definitions and their current import
// references together; source provenance is evidence and keeps its original names.
fn rename_identity_references(value: &mut Value, renames: &BTreeMap<String, String>) {
    match value {
        Value::String(text) => {
            if let Some(replacement) = renames.get(text) {
                *text = replacement.clone();
            }
        }
        Value::Array(values) => {
            for value in values {
                rename_identity_references(value, renames);
            }
        }
        Value::Object(values) => {
            for (key, mut value) in std::mem::take(values) {
                if key != "source" {
                    rename_identity_references(&mut value, renames);
                }
                let key = renames.get(&key).cloned().unwrap_or(key);
                assert!(values.insert(key, value).is_none(), "fixture key collision");
            }
        }
        _ => {}
    }
}

#[test]
fn escaped_definition_ids_names_and_quest_keys_preserve_evaluation_and_fresh_export() {
    let temp = tempfile::tempdir().unwrap();
    let mut package = bundled_snapshot().unwrap().package().clone();
    let baseline_path = temp.path().join("baseline.xml");
    fs::write(
        &baseline_path,
        with_disabled_quests(TEMPLATE, &package.quests.config_keys),
    )
    .unwrap();
    let baseline = success(
        evaluate(temp.path(), &baseline_path, None)
            .output()
            .unwrap(),
    );

    let mut xml = TEMPLATE.to_string();
    let mut renames = BTreeMap::new();
    for (field, xml_attribute) in [
        ("skill_id", "skillId"),
        ("game_id", "gemId"),
        ("variant_id", "variantId"),
        ("name", "nameSpec"),
    ] {
        let mut data = serde_json::to_value(&package).unwrap();
        let mut renamed = Vec::new();
        for definition in ["mace", "brutality_i"] {
            let record = if definition == "mace" {
                &mut data["mace"]
            } else {
                data["supports"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|support| support["id"] == definition)
                    .unwrap()
            };
            let old = record[field].as_str().unwrap().to_owned();
            let replacement = format!("{old} &'\"<> caf\u{e9}");
            record[field] = json!(replacement);
            renamed.push((old, replacement));
        }
        for (old, replacement) in renamed {
            let old_attribute = format!("{xml_attribute}=\"{}\"", attribute(&old));
            assert_eq!(xml.matches(&old_attribute).count(), 1);
            xml = xml.replace(
                &old_attribute,
                &format!("{xml_attribute}=\"{}\"", attribute(&replacement)),
            );
            renames.insert(old, replacement);
        }
        package = serde_json::from_value(data).unwrap();
    }
    let mut identities = serde_json::to_value(&package.skill_identities).unwrap();
    rename_identity_references(&mut identities, &renames);
    package.skill_identities = serde_json::from_value(identities).unwrap();
    let mut preparation = serde_json::to_value(&package.skill_preparation).unwrap();
    rename_identity_references(&mut preparation, &renames);
    // This ordered import input is the non-provenance field under `source`.
    rename_identity_references(&mut preparation["source"]["canonical_gem_order"], &renames);
    package.skill_preparation = serde_json::from_value(preparation).unwrap();
    for key in &mut package.quests.config_keys {
        key.push_str(" &'\"<> quest");
    }
    let selected_skill = package.mace.skill_id.clone();
    let selected_support = package.support("brutality_i").unwrap().skill_id.clone();
    let quest_keys = package.quests.config_keys.clone();
    xml = with_disabled_quests(&xml, &quest_keys);
    let input = temp.path().join("renamed.xml");
    fs::write(&input, &xml).unwrap();
    let (data, identity) = write_package(temp.path(), package);
    let export = temp.path().join("export.xml");
    let renamed = success(
        evaluate(temp.path(), &input, Some(&data))
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap(),
    );
    assert_eq!(renamed["evaluation"]["backend"]["data"], identity);
    assert_eq!(
        renamed["evaluation"]["coverage"]["selected_player"]["skill_id"],
        selected_skill
    );
    assert_eq!(
        renamed["evaluation"]["measurements"],
        baseline["evaluation"]["measurements"]
    );
    let exported = fs::read_to_string(&export).unwrap();
    assert!(exported.contains(&attribute(&selected_skill)));
    assert!(exported.contains(&attribute(&selected_support)));
    for key in quest_keys {
        assert!(exported.contains(&attribute(&key)));
    }
    let metadata: Value =
        serde_json::from_slice(&fs::read(export.with_extension("xml.data.json")).unwrap()).unwrap();
    assert_eq!(metadata["backend"], renamed["evaluation"]["backend"]);
    assert_eq!(metadata["uses_packaged_default"], false);
    assert_eq!(metadata["warnings"], renamed["evaluation"]["warnings"]);
    assert!(
        metadata["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("CustomUnreviewed"))
    );

    let digest = format!("{:x}", Sha256::digest(fs::read(&data).unwrap()));
    let fresh = success(
        evaluate(temp.path(), &export, Some(&data))
            .arg("--data-sha256")
            .arg(&digest)
            .output()
            .unwrap(),
    );
    assert_eq!(
        fresh["evaluation"]["backend"],
        renamed["evaluation"]["backend"]
    );
    assert_eq!(
        fresh["evaluation"]["measurements"],
        renamed["evaluation"]["measurements"]
    );
    assert_eq!(
        fresh["evaluation"]["coverage"]["selected_player"]["skill_id"],
        selected_skill
    );
    assert!(
        fresh["evaluation"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|warning| !warning.as_str().unwrap().contains("CustomUnreviewed"))
    );
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
}

#[test]
fn inconsistent_definition_identities_fail_before_output_or_export() {
    let temp = tempfile::tempdir().unwrap();
    let mut package = bundled_snapshot().unwrap().package().clone();
    let original = package.mace.skill_id.clone();
    package.mace.skill_id.push_str(" caller-only");
    let xml = TEMPLATE.replace(&original, &package.mace.skill_id);
    let input = temp.path().join("inconsistent.xml");
    fs::write(&input, &xml).unwrap();
    let (data, _) = write_package(temp.path(), package);
    let output = temp.path().join("result.json");
    let export = temp.path().join("export.xml");
    let failure = evaluate(temp.path(), &input, Some(&data))
        .arg("--output")
        .arg(&output)
        .arg("--export")
        .arg(&export)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(
        String::from_utf8_lossy(&failure.stderr)
            .contains("Authored skill resolution differs from the closed numerical adapter"),
        "{}",
        String::from_utf8_lossy(&failure.stderr)
    );
    assert!(!output.exists());
    assert!(!export.exists());
    assert!(!export.with_extension("xml.data.json").exists());
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
}
