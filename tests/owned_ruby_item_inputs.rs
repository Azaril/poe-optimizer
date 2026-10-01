//! Imported construction closes physical inventories, not static rule coverage.
#[path = "support/owned_ruby_item_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::owned_schema::{SchemaClosure, SchemaState};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::assemble_owned_release,
    owned_value::ValueCodecKind,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap()
}
fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
#[test]
fn authored_imported_inputs_preserve_static_coverage_and_existing_semantics() {
    let extension = family::extension();
    assert_eq!(extension.schema.len(), 7);
    assert_eq!(extension.owners.len(), 1);
    assert!(extension.tables.is_empty() && extension.receivers.is_empty());
    assert_eq!(
        extension.operations_version.as_ref().unwrap().as_str(),
        "owned-domain-operations-v14"
    );
    assert!(matches!(
        extension.owners[0].programs.closure,
        SchemaClosure::Partial { .. }
    ));
    let mut programs: Vec<_> = extension.owners[0]
        .programs
        .members
        .iter()
        .map(|p| p.id.as_str())
        .collect();
    programs.sort_unstable();
    assert_eq!(
        programs,
        ["catalyst-inputs", "template-supplies-base-attack-profile"]
    );
    let bindings = family::bindings();
    assert_eq!(bindings.len(), 1);
    let binding = &bindings[0];
    assert_eq!(binding.template.key().as_str(), "def.000000000000200b");
    assert_eq!(
        serde_json::to_value(&binding.construction).unwrap(),
        "fresh_imported_category_census_v4"
    );
    assert_eq!(binding.header_inputs.len(), 2);
    let slot_keys: Vec<_> = extension
        .schema
        .iter()
        .filter_map(|e| match e {
            SchemaExtensionEntry::Slot(s) => Some(s.address().key().as_str().to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(
        slot_keys,
        [
            "def.00000000000031fd",
            "def.00000000000031fe",
            "def.00000000000031ff",
            "def.0000000000003200",
            "def.0000000000003201",
            "def.0000000000003202"
        ]
    );
    for entry in &extension.schema {
        if let SchemaExtensionEntry::Definition(
            poe_optimizer_core::owned_schema::DefinitionDescriptor::ItemTemplate(template),
        ) = entry
        {
            let SchemaState::Known(schema) = &template.schema else {
                panic!()
            };
            assert_eq!(template.id, binding.template);
            assert_eq!(schema.declarations.parameters.members.len(), 6);
            assert!(matches!(
                schema.declarations.parameters.closure,
                SchemaClosure::Partial { .. }
            ));
        }
    }
    let membership = serde_json::to_value(family::membership()).unwrap();
    assert_eq!(
        membership["modifier_rules"],
        json!(["fixed-increased-fire-damage"])
    );
    assert_eq!(membership["census_templates"][0]["implicit_members"], 0);
    assert_eq!(membership["census_templates"][0]["explicit_members"], 1);
    let profile = serde_json::to_value(family::construction_profile()).unwrap();
    assert_eq!(profile["template"], json!(binding.template));
    assert_eq!(profile["headers"].as_array().unwrap().len(), 5);
    assert!(
        profile["headers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["cardinality"] == "required_once")
    );
    assert_eq!(family::defaults()[0].parameters.len(), 2);
    let authoring = family::authoring();
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest_bytes)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
}
fn source_item(sidecar: &Value, ordinal: u64) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    let links: Vec<_> = rows[0]["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["kind"] == "item")
        .collect();
    assert_eq!(links.len(), 1);
    &links[0]["value"]
}
fn item_index(draft: &Value, id: &Value) -> usize {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .position(|i| &i["id"] == id)
        .unwrap()
}
fn row_mut(sidecar: &mut Value, ordinal: u64) -> &mut Value {
    sidecar["item_texts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn raw_assignments(requirement: u64) -> Vec<Value> {
    let b = family::bindings().remove(0);
    let ValueCodecKind::Option { tokens } = &b.header_inputs[0].codec.codec else {
        panic!()
    };
    let rare = &tokens.iter().find(|v| v.token == "RARE").unwrap().value;
    [(&b.header_inputs[0].slot, json!({"kind":"option","value":rare})),
     (&b.header_inputs[1].slot, json!({"kind":"integer","value":requirement})),
     (&b.corruption_slot, json!({"kind":"boolean","value":false})),
     (&b.capacity_slot, json!({"kind":"integer","value":0}))]
        .into_iter().map(|(slot,value)|json!({"slot":{"kind":"known","value":slot},"value":{"kind":"known","value":value}})).collect()
}
fn expected_parameters(requirement: u64) -> Vec<Value> {
    let mut parameters = raw_assignments(requirement);
    parameters.extend(family::defaults()[0].parameters.iter().map(|p|json!({"slot":{"kind":"known","value":p.assignment.slot},"value":{"kind":"known","value":p.assignment.value}})));
    parameters
}
fn check_complete(item: &Value, requirement: u64) {
    assert_eq!(item["parameters"]["completion"], json!({"kind":"complete"}));
    assert_eq!(item["modifiers"]["completion"], json!({"kind":"complete"}));
    assert_eq!(item["modifiers"]["members"].as_array().unwrap().len(), 1);
    let modifier = &item["modifiers"]["members"][0];
    assert_eq!(
        modifier["definition"]["value"]["key"],
        "def.00000000000031e4"
    );
    assert_eq!(modifier["rolls"]["completion"], json!({"kind":"complete"}));
    assert_eq!(modifier["rolls"]["members"].as_array().unwrap().len(), 24);
    assert_eq!(
        item["modifier_order"],
        json!({"kind":"known","value":[modifier["id"]]})
    );
    assert_eq!(item["quality"], json!({"kind":"known","value":null}));
    assert_eq!(item["item_level"], json!({"kind":"known","value":55}));
    let mut parameters = item["parameters"]["members"].as_array().unwrap().clone();
    assert_eq!(parameters.len(), 6);
    for expected in expected_parameters(requirement) {
        let index = parameters
            .iter()
            .position(|p| p == &expected)
            .expect("exact physical assignment");
        parameters.remove(index);
    }
    assert!(parameters.is_empty());
}
fn source_lineage(value: &mut Value, current: &Value, prior: &Value) {
    match value {
        Value::Object(o) => {
            if !o.contains_key("local")
                && let Some(lineage) = o.get_mut("lineage")
            {
                assert_eq!(lineage, current);
                *lineage = prior.clone();
            }
            for v in o.values_mut() {
                source_lineage(v, current, prior);
            }
        }
        Value::Array(a) => {
            for v in a {
                source_lineage(v, current, prior);
            }
        }
        _ => (),
    }
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], sa["schema_version"]);
    assert_eq!(a["schema_version"], b["schema_version"]);
    let mut expected = a["draft"].clone();
    let mut actual = b["draft"].clone();
    let mut expected_side = sa.clone();
    let mut actual_side = sb.clone();
    source_lineage(
        &mut actual_side,
        &b["draft"]["allocator"]["lineage"],
        &a["draft"]["allocator"]["lineage"],
    );
    let mut retired = vec![];
    if case == 3 {
        // The other Ruby has unconverted physical members, so the new required
        // template inputs correctly add a diagnostic without admitting facts.
        let old_row = row_mut(&mut expected_side, 251);
        let new_row = row_mut(&mut actual_side, 251);
        assert_eq!(
            old_row["issues"],
            json!([{"problem":"schema_partial","lines":[]}])
        );
        assert_eq!(
            new_row["issues"],
            json!([
                {"problem":"required_parameter_missing","lines":[]},
                {"problem":"schema_partial","lines":[]}
            ])
        );
        assert_eq!(old_row["defaults"]["parameters"], json!([]));
        assert_eq!(new_row["defaults"], old_row["defaults"]);
        assert_eq!(
            new_row["attribution"]["default_scope"],
            json!({"kind":"unproved"})
        );
        assert!(new_row.get("parameter_inputs").is_none());
        old_row["issues"] = new_row["issues"].clone();
    }
    if case == 4 {
        let old_id = source_item(&sa, 171);
        let new_id = source_item(&sb, 171);
        let index = item_index(&a, old_id);
        assert_eq!(index, item_index(&b, new_id));
        let old_item = &a["draft"]["items"]["members"][index];
        let new_item = &b["draft"]["items"]["members"][index];
        assert_eq!(old_id["local"], "0000000000000094");
        assert_eq!(old_item["parameters"]["members"], json!([]));
        for (field, code) in [
            (
                &old_item["parameters"]["completion"],
                "item-parameters-not-converted",
            ),
            (
                &old_item["modifiers"]["completion"],
                "item-modifiers-not-converted",
            ),
            (
                &old_item["modifier_order"],
                "item-modifier-order-not-converted",
            ),
        ] {
            assert_eq!(field["kind"], "pending");
            assert_eq!(field["code"], code);
            retired.push(field["id"].clone());
        }
        check_complete(new_item, 0);
        expected["items"]["members"][index]["parameters"]["completion"] =
            json!({"kind":"complete"});
        expected["items"]["members"][index]["modifiers"]["completion"] = json!({"kind":"complete"});
        expected["items"]["members"][index]["modifier_order"] =
            json!({"kind":"known","value":[old_item["modifiers"]["members"][0]["id"]]});
        actual["items"]["members"][index]["parameters"]["members"] = json!([]);
        let old_row = row_mut(&mut expected_side, 171);
        let new_row = row_mut(&mut actual_side, 171);
        assert!(old_row.get("parameter_inputs").is_none());
        let mut proof = Vec::new();
        let assignments = raw_assignments(0);
        let binding = family::bindings().remove(0);
        for (i, assignment) in assignments.iter().enumerate() {
            let origin = match i {
                0 | 1 => {
                    let header = &binding.header_inputs[i];
                    let line = if i == 0 { 1 } else { 6 };
                    assert_eq!(
                        old_row["lines"][line - 1]["text"],
                        if i == 0 {
                            "Rarity: RARE"
                        } else {
                            "LevelReq: 0"
                        }
                    );
                    json!({"kind":"header","line":line,"rule":header.rule,"capture":header.capture})
                }
                2 => json!({"kind":"fresh_uncorrupted"}),
                3 => json!({"kind":"absent_socket_header"}),
                _ => unreachable!(),
            };
            proof.push(json!({"slot":assignment["slot"]["value"],"value":assignment["value"]["value"],"origin":origin}));
        }
        assert_eq!(new_row["parameter_inputs"], json!(proof));
        old_row["parameter_inputs"] = json!(proof);
        assert_eq!(old_row["defaults"]["parameters"], json!([]));
        let defaults: Vec<_> = family::defaults()[0]
            .parameters
            .iter()
            .map(|p| p.assignment.clone())
            .collect();
        assert_eq!(new_row["defaults"]["parameters"], json!(defaults));
        old_row["defaults"]["parameters"] = json!(defaults);
    }
    let issued = |d: &Value| {
        u64::from_str_radix(d["allocator"]["last_issued"].as_str().unwrap(), 16).unwrap()
    };
    assert_eq!(issued(&expected) - issued(&actual), retired.len() as u64);
    actual["allocator"] = expected["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(
        &expected,
        &mut actual,
        &mut ids,
        "all other canonical inputs",
    );
    let mut removed = 0;
    for origin in expected_side["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue" && retired.contains(&link["value"]);
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
    assert_eq!(expected_side["allocator_after"], a["draft"]["allocator"]);
    assert_eq!(
        actual_side["allocator_after"]["last_issued"],
        b["draft"]["allocator"]["last_issued"]
    );
    for field in [
        "policy",
        "mapping",
        "registry",
        "definitions",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
        "draft",
        "allocator_after",
    ] {
        actual_side[field] = expected_side[field].clone();
    }
    assert_eq!(
        expected_side["item_texts"].as_array().unwrap().len(),
        actual_side["item_texts"].as_array().unwrap().len()
    );
    for (p, q) in expected_side["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(actual_side["item_texts"].as_array_mut().unwrap())
    {
        for field in ["policy", "item_lines"] {
            q["attribution"][field] = p["attribution"][field].clone();
        }
    }
    identity::correspond(
        &expected_side,
        &mut actual_side,
        &mut ids,
        "all other source evidence",
    );
    let before = selected::finalize(
        xml,
        old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        new,
        &out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        out.join(format!("original-{case:02}-prior-selected-report.json")),
        &before,
    );
    write(
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut expected_issues = before["finalization"]["issues"].clone();
    expected_issues
        .as_array_mut()
        .unwrap()
        .retain(|i| !retired.contains(&i["id"]));
    let mut actual_issues = after["finalization"]["issues"].clone();
    identity::relocate(&mut actual_issues, &ids);
    assert_eq!(
        actual_issues, expected_issues,
        "only the three Ruby physical obligations retire"
    );
    let mut selection = selected::selection(xml, new);
    identity::relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    let count = before["finalization"]["issues"].as_array().unwrap().len();
    let next_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(count, [123, 124, 116, 153, 20][case - 1]);
    assert_eq!(next_count, [123, 124, 116, 150, 20][case - 1]);
    json!({"original":case,"selected_before":count,"selected_after":next_count,"retired_issues":retired,"raw_assignments":if case==4 {6}else{0},"allocator_before":a["draft"]["allocator"],"allocator_after":b["draft"]["allocator"],"calculation":"not_run"})
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-04.xml"))
            .unwrap();
    let start = original.find("<Item id=\"1\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let identity = raw.lines().find(|l| l.starts_with("Unique ID: ")).unwrap();
    let body = "14% increased Fire Damage";
    let cases = vec![
        ("missing-id", raw.replace(identity, "")),
        ("empty-id", raw.replace(identity, "Unique ID: ")),
        (
            "duplicate-id",
            raw.replace(identity, &format!("{identity}\n{identity}")),
        ),
        ("malformed-id", raw.replace(identity, "Unique ID: not-hex")),
        ("id-alias", raw.replace("Unique ID:", "Unique Id:")),
        ("missing-requirement", raw.replace("LevelReq: 0", "")),
        (
            "duplicate-requirement",
            raw.replace("LevelReq: 0", "LevelReq: 0\nLevelReq: 1"),
        ),
        (
            "negative-requirement",
            raw.replace("LevelReq: 0", "LevelReq: -1"),
        ),
        (
            "fractional-requirement",
            raw.replace("LevelReq: 0", "LevelReq: 1.5"),
        ),
        (
            "aliased-requirement",
            raw.replace("LevelReq: 0", "LevelReq: 00"),
        ),
        (
            "high-requirement",
            raw.replace("LevelReq: 0", "LevelReq: 1000001"),
        ),
        ("missing-level", raw.replace("Item Level: 55", "")),
        (
            "duplicate-level",
            raw.replace("Item Level: 55", "Item Level: 55\nItem Level: 54"),
        ),
        (
            "implicit-count",
            raw.replace("Implicits: 0", "Implicits: 1"),
        ),
        (
            "crafted-false",
            raw.replace(body, &format!("Crafted: false\n{body}")),
        ),
        (
            "prefix",
            raw.replace(body, &format!("Prefix: None\n{body}")),
        ),
        (
            "corrupted",
            raw.replace(body, &format!("Corrupted\n{body}")),
        ),
        ("mirrored", raw.replace(body, &format!("Mirrored\n{body}"))),
        (
            "unidentified",
            raw.replace(body, &format!("Unidentified\n{body}")),
        ),
        (
            "empty-sockets",
            raw.replace(body, &format!("Sockets: S\nRune: None\n{body}")),
        ),
        (
            "jewel-socket",
            raw.replace(body, &format!("Sockets: J\n{body}")),
        ),
        (
            "unknown-header",
            raw.replace(body, &format!("Unreviewed: 1\n{body}")),
        ),
        (
            "unknown-member",
            raw.replace(body, &format!("Unreviewed mechanic\n{body}")),
        ),
        (
            "advanced-range",
            raw.replace(body, &format!("{{range:0.5}}{body}")),
        ),
        (
            "overlay-outside",
            raw.replace("range=\"0.5\"", "range=\"1.5\""),
        ),
        (
            "overlay-duplicate",
            raw.replace(
                "<ModRange range=\"0.5\" id=\"1\"/>",
                "<ModRange range=\"0.5\" id=\"1\"/><ModRange range=\"0.5\" id=\"1\"/>",
            ),
        ),
        (
            "overlay-target",
            raw.replace(
                "<ModRange range=\"0.5\" id=\"1\"/>",
                "<ModRange range=\"0.5\" id=\"2\"/>",
            ),
        ),
    ];
    for (label, changed) in &cases {
        assert_ne!(changed, raw, "probe must change its source: {label}");
        let file = out.join(format!("probe-{label}.xml"));
        fs::write(
            &file,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let directory = out.join(format!("probe-{label}"));
        release::normalize(package, &file, 4, &directory);
        let draft: Value = read(directory.join("draft.json"));
        let sidecar: Value = read(directory.join("sidecar.json"));
        let index = item_index(&draft, source_item(&sidecar, 171));
        let item = &draft["draft"]["items"]["members"][index];
        for field in [
            &item["parameters"]["completion"],
            &item["modifiers"]["completion"],
            &item["modifier_order"],
        ] {
            assert_eq!(
                field["kind"], "pending",
                "unsupported construction: {label}"
            );
        }
    }
    let positives = [
        (
            "different-id",
            raw.replace(identity, &format!("Unique ID: {}", "a".repeat(64))),
            0,
        ),
        (
            "different-title",
            raw.replace("Viper Wound", "Independent Fixture"),
            0,
        ),
        (
            "requirement-one",
            raw.replace("LevelReq: 0", "LevelReq: 1"),
            1,
        ),
        (
            "requirement-bound",
            raw.replace("LevelReq: 0", "LevelReq: 1000000"),
            1_000_000,
        ),
    ];
    for (label, changed, requirement) in &positives {
        assert_ne!(changed, raw, "positive source change: {label}");
        let file = out.join(format!("probe-{label}.xml"));
        fs::write(
            &file,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let directory = out.join(format!("probe-{label}"));
        release::normalize(package, &file, 4, &directory);
        let draft: Value = read(directory.join("draft.json"));
        let sidecar: Value = read(directory.join("sidecar.json"));
        let index = item_index(&draft, source_item(&sidecar, 171));
        check_complete(&draft["draft"]["items"]["members"][index], *requirement);
    }
    cases.len() + positives.len()
}
#[test]
#[ignore = "requires the checked Fire Damage predecessor and complete source evidence"]
fn real_imported_ruby_preserves_every_original_request_and_retires_only_physical_gates() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RUBY_ITEM_INPUTS_PRIOR")
            .expect("explicit checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RUBY_ITEM_INPUTS_OUTPUT")
            .expect("explicit fresh output"),
    );
    assert!(!out.exists());
    let authoring = family::authoring();
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let bytes = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        bytes.len() as u64,
        proof["evidence_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        proof["evidence_sha256"]
    );
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    family::assert_cross_owner_rejected(&next);
    assert!(next.input().evaluation.is_none());
    assert_eq!(next.receipt().query_rows, 110);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (source, target) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(source, target),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    for policy in [
        "equipment_membership",
        "item_modifier_membership",
        "item_parameter_inputs",
    ] {
        for field in ["definitions", "item_lines", "item_source"] {
            let mut bad = serde_json::to_value(next.input()).unwrap();
            bad["normalization"][policy][field] = match field {
                "definitions" => serde_json::to_value(&prior.receipt().definitions).unwrap(),
                "item_lines" => json!(prior.receipt().items),
                _ => json!(prior.receipt().item_source),
            };
            assert!(
                assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                    .is_err(),
                "stale {policy}.{field}"
            );
        }
    }
    let mut reports = Vec::new();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior_path.join(&query)).unwrap(),
            fs::read(package.join(query)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    let probe_count = probes(&package, &out);
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":110,"originals":reports,"probes":probe_count,"stale_binding_rejections":9,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
