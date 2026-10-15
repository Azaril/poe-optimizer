//! One checked unused observation, with no build or numerical coverage closure.
#[path = "support/owned_configuration_accounting.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{build_identity::BuildLineage, owned_build::ParameterValue};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_normalize::ConfigurationDestination,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceAttributeRef, SourceProjectEvidence},
    owned_value_policy::{CandidateValue, ValueCandidate, ValueLane, ValueOutcome, ValueRecipe},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

#[test]
fn retained_both_jit_witness_authenticates_the_exact_inactive_consumer_branch() {
    family::check_authored();
}

#[test]
fn evidence_rejects_changed_callbacks_outputs_scope_or_positive_controls() {
    let original: Value = family::read("evidence/source-jit-off.json");
    family::validate_observations(&original).unwrap();
    for mutation in 0..7 {
        let mut changed = original.clone();
        match mutation {
            0 => changed["cases"]
                .as_array_mut()
                .unwrap()
                .pop()
                .map(|_| ())
                .unwrap(),
            1 => changed["cases"][0]["state"]["callbacks"][0]["roll_read"] = json!(true),
            2 => changed["cases"][0]["state"]["callbacks"][0]["roll"] = json!(0),
            3 => changed["cases"][1]["state"]["main"]["player"]["scalar_sha256"] = json!("changed"),
            4 => changed["cases"][11]["state"]["main"]["placeholder_damage"]["Cold"] = json!(4127),
            5 => changed["evidence"]["native_coverage"] = json!(true),
            6 => changed["cases"][0]["state"]["hook_restored"] = json!(false),
            _ => unreachable!(),
        }
        assert!(
            family::validate_observations(&changed).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn injected_recipe_admits_only_finite_numeric_placeholders_without_a_default() {
    let rules = family::rules();
    assert_eq!(rules.len(), 1);
    let row = &rules[0];
    assert_eq!(row.selector.lane, ValueLane::PlaceholderNumber);
    assert_eq!(row.selector.name, "enemyDamageRollRange");
    let [
        ConfigurationDestination::InapplicableObservation {
            recipe,
            absent_input_names,
            reason,
        },
    ] = row.destinations.as_slice()
    else {
        panic!("one branch-only destination")
    };
    assert_eq!(
        absent_input_names,
        &["enemyIsBoss", "presetBossSkills", "enemySizePreset"]
    );
    assert_eq!(
        reason.as_str(),
        "inactive-boss-skill-damage-roll-placeholder"
    );
    let recipe = ValueRecipe::new(recipe.as_ref().clone(), Default::default()).unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(b"<PathOfBuilding2/>").unwrap(),
        BuildLineage::from_bytes([142; 16]),
        Default::default(),
    )
    .unwrap();
    let origin = SourceAttributeRef {
        occurrence: source.occurrences()[0].id(),
        index: 0,
    };
    let candidate = |text| ValueCandidate {
        selector: &row.selector,
        origin,
        value: CandidateValue::Decoded(text),
    };
    for text in ["0", "70", "-19.75", "1e99"] {
        assert!(matches!(
            recipe.decide(&[candidate(text)]).unwrap().outcome,
            ValueOutcome::Selected {
                value: ParameterValue::Quantity(_),
                ..
            }
        ));
    }
    for text in ["NaN", "inf", " 70", "70%", "1e999", "None", ""] {
        assert!(matches!(
            recipe.decide(&[candidate(text)]).unwrap().outcome,
            ValueOutcome::Pending { .. }
        ));
    }
    assert!(matches!(
        recipe.decide(&[]).unwrap().outcome,
        ValueOutcome::Pending { .. }
    ));
    let duplicate = ValueCandidate {
        origin: SourceAttributeRef { index: 1, ..origin },
        ..candidate("70")
    };
    assert!(recipe.decide(&[candidate("0"), duplicate]).is_err());
    let mut other_lane = row.selector.clone();
    other_lane.lane = ValueLane::InputNumber;
    assert!(matches!(
        recipe
            .decide(&[ValueCandidate {
                selector: &other_lane,
                ..candidate("70")
            }])
            .unwrap()
            .outcome,
        ValueOutcome::Pending { .. }
    ));
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

fn compare(
    case: usize,
    xml: &[u8],
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    prior_path: &Path,
    package: &Path,
    out: &Path,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let mut a = read(old.join("draft.json"));
    let mut b = read(new.join("draft.json"));
    let mut sa = read(old.join("sidecar.json"));
    let mut sb = read(new.join("sidecar.json"));
    preservation::authenticate(&sa, &old, prior_path, case, prior);
    preservation::authenticate(&sb, &new, package, case, next);
    for (directory, sidecar) in [(&old, &sa), (&new, &sb)] {
        let report = read(directory.join("report.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(
            report["sidecar_sha256"],
            family::sha(&fs::read(directory.join("sidecar.json")).unwrap())
        );
        assert_eq!(sidecar["source_sha256"], family::sha(xml));
        assert_eq!(sidecar["source_bytes"], xml.len());
    }
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    assert_eq!(
        a, b,
        "every draft value, local ID, list, inventory and query is unchanged"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([143; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let matching: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().name() == "Placeholder"
                && row
                    .attribute("name")
                    .is_some_and(|a| a.decoded().ok() == Some("enemyDamageRollRange"))
        })
        .collect();
    assert_eq!(matching.len(), 1);
    let leaf = matching[0].occurrence().id().ordinal() as usize;
    let scope = matching[0].occurrence().parent().unwrap().ordinal() as usize;
    assert_eq!(evidence.rows()[scope].occurrence().name(), "ConfigSet");
    let issue = a["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"].clone();
    let scenario = a["draft"]["scenario_presets"]["members"][0]["id"].clone();
    let issue_link = json!({"kind":"issue","value":issue});
    let before = &sa["origins"][leaf];
    assert_eq!(before["disposition"], json!({"kind":"contributes"}));
    assert_eq!(before["links"], json!([issue_link]));
    let expected = json!({"source":before["source"],"disposition":{"kind":"source_only","value":"inactive-boss-skill-damage-roll-placeholder"},"links":[{"kind":"scenario_preset","value":scenario}]});
    assert_eq!(sb["origins"][leaf], expected);
    let census = |sidecar: &Value| {
        let mut total = 0usize;
        let mut inside = 0usize;
        for row in sidecar["origins"].as_array().unwrap() {
            if !row["links"].as_array().unwrap().contains(&issue_link) {
                continue;
            }
            total += 1;
            let mut source = Some(
                evidence.rows()[row["source"]["ordinal"].as_u64().unwrap() as usize]
                    .occurrence()
                    .id(),
            );
            while let Some(id) = source {
                let current = evidence.rows()[id.ordinal() as usize].occurrence();
                if current.name() == "Config" {
                    inside += 1;
                    break;
                }
                source = current.parent();
            }
        }
        (total, inside, total - inside)
    };
    let before_counts = census(&sa);
    let after_counts = census(&sb);
    assert_eq!(
        after_counts,
        (before_counts.0 - 1, before_counts.1 - 1, before_counts.2)
    );
    if case == 5 {
        assert_eq!(before_counts, (36, 21, 15));
        assert_eq!(after_counts, (35, 20, 15));
    }
    sb["origins"][leaf] = sa["origins"][leaf].clone();
    for field in ["policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(
        sa, sb,
        "exact sidecar inverse: no other origin, link, decision, ordering or commitment changed"
    );
    let mut before = selected::finalize_with_definitions(
        xml,
        &old,
        &out.join(format!("prior-selection-{case:02}.json")),
        &prior_path.join("schema.json"),
    );
    let mut after = selected::finalize_with_definitions(
        xml,
        &new,
        &out.join(format!("selection-{case:02}.json")),
        &package.join("schema.json"),
    );
    write(
        out.join(format!("prior-selected-report-{case:02}.json")),
        &before,
    );
    write(out.join(format!("selected-report-{case:02}.json")), &after);
    selected::canonical(&mut before);
    selected::canonical(&mut after);
    assert_eq!(
        before["finalization"]["issues"],
        after["finalization"]["issues"]
    );
    let issues = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(issues, [106, 117, 109, 123, 4][case - 1]);
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    json!({"original":case,"changed_leaf":leaf,"configuration_before":before_counts,"configuration_after":after_counts,"selected_issues":issues,"draft_inverse_exact":true,"sidecar_inverse_exact":true,"inventory_completed":false})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original = fs::read_to_string(
        family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let mut cases = Vec::new();
    for (name, text, allowed) in [
        ("zero", "0", true),
        ("fraction", "37.5", true),
        ("finite-outside-ui", "-19.75", true),
        ("malformed", "NaN", false),
    ] {
        cases.push((
            name,
            family::field(
                &original,
                "Placeholder",
                "enemyDamageRollRange",
                Some(("number", text)),
            ),
            allowed,
        ));
    }
    for (name, element, key, attribute, value) in [
        (
            "explicit-none",
            "Input",
            "presetBossSkills",
            "string",
            "None",
        ),
        (
            "boss-preset",
            "Input",
            "presetBossSkills",
            "string",
            "Shaper Ball",
        ),
        (
            "placeholder-boss-write",
            "Placeholder",
            "presetBossSkills",
            "string",
            "None",
        ),
        (
            "authored-roll",
            "Input",
            "enemyDamageRollRange",
            "number",
            "0",
        ),
    ] {
        cases.push((
            name,
            family::field(&original, element, key, Some((attribute, value))),
            false,
        ));
    }
    let count = cases.len();
    for (name, xml, allowed) in cases {
        let path = out.join(format!("probe-{name}.xml"));
        fs::write(&path, &xml).unwrap();
        let directory = out.join(format!("probe-{name}"));
        release::normalize(package, &path, 5, &directory);
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([144; 16]),
            Default::default(),
        )
        .unwrap();
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let row = evidence
            .rows()
            .iter()
            .find(|r| {
                r.occurrence().name() == "Placeholder"
                    && r.attribute("name")
                        .is_some_and(|a| a.decoded().ok() == Some("enemyDamageRollRange"))
            })
            .unwrap();
        let sidecar = read(directory.join("sidecar.json"));
        let origin = &sidecar["origins"][row.occurrence().id().ordinal() as usize];
        assert_eq!(
            origin["disposition"]["kind"] == "source_only",
            allowed,
            "{name}"
        );
        if !allowed {
            assert!(
                origin["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|link| link["kind"] == "issue")
            );
        }
    }
    count
}

#[test]
#[ignore = "requires current CONFIGURATION_ACCOUNTING_PRIOR and a fresh CONFIGURATION_ACCOUNTING_OUTPUT; no PoB runtime"]
fn publish_one_configuration_observation_preserving_all_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CONFIGURATION_ACCOUNTING_PRIOR")
            .expect("checked current package"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CONFIGURATION_ACCOUNTING_OUTPUT")
            .expect("fresh output"),
    );
    assert!(!out.exists());
    let old_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let files = release::inventory(&package);
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    assert_eq!(files.len(), 18);
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        old_inventory.keys().collect::<Vec<_>>()
    );
    let changed: Vec<_> = files
        .iter()
        .filter(|(name, hash)| old_inventory.get(*name) != Some(*hash))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        changed,
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    assert_eq!(release::load(&package).receipt(), next.receipt());
    let mut originals = Vec::new();
    for case in 1..=5 {
        let xml = family::root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &xml,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(compare(
            case,
            &fs::read(xml).unwrap(),
            &prior,
            &next,
            &prior_path,
            &package,
            &out,
        ));
        let file = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior_path.join(&file)).unwrap(),
            fs::read(package.join(&file)).unwrap()
        );
    }
    let controls = probes(&package, &out);
    assert_eq!(old_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"mutation_controls":controls,"queries":110,"prior_unchanged":true,"rebuild_byte_identical":true,"new_definitions":0,"new_programs":0,"inventory_completed":false,"complete_original_builds":0}),
    );
}
