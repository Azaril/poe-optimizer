//! Original-build occurrence usage transport with unchanged numerical coverage.
#[path = "support/owned_occurrence_counts.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_content::digest_owned,
    owned_definitions::{FiniteQuantity, OwnedDefinitionKey},
    owned_schema::{SchemaState, SlotDescriptor, ValueSchema},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        GemInventoryPolicy, GeneratedSkillInputPolicy, UsageInputPolicy,
        gem_inventory_scalar_inputs_identity, usage_inputs_identity,
    },
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(p, serde_json::to_vec(value).unwrap()).unwrap();
}
fn run(name: &str, args: &[&Path]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg(name)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn publish(input: &Path, output: &Path) -> Value {
    run(
        "assemble-owned-release",
        &[input, Path::new("--output"), output],
    )
}
fn finalize(xml: &[u8], dir: &Path, package: &Path, selection: &Path) -> Value {
    write(selection, &selected::selection(xml, dir));
    let report = run(
        "check-owned-draft",
        &[
            &dir.join("draft.json"),
            Path::new("--definitions"),
            &package.join("schema.json"),
            Path::new("--selection"),
            selection,
        ],
    );
    assert_eq!(report["finalization"]["status"], "pending");
    assert_eq!(report["verification"]["calculation"], "not_run");
    assert!(
        report["intent_validation"]["schema_issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    report
}
#[test]
fn occurrence_count_authoring_reuses_generic_native_policy() {
    family::check_authored();
}

fn inherited_policy_rebind(next: &StagedOwnedRelease) {
    let policy = next.normalization().usage_inputs.as_ref().unwrap();
    assert!(matches!(
        policy,
        UsageInputPolicy::PobOccurrenceUsageV3 { .. }
    ));
    let GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { rows, .. } = next
        .normalization()
        .generated_skill_inputs
        .as_ref()
        .unwrap();
    let slot = &rows[0].parameters[0].slot;
    let mut descriptor = next
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| matches!(row, SlotDescriptor::Parameter(p) if &p.id == slot))
        .unwrap()
        .clone();
    let SlotDescriptor::Parameter(parameter) = &mut descriptor else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut parameter.schema else {
        unreachable!()
    };
    let ValueSchema::Quantity(range) = &mut schema.value else {
        unreachable!()
    };
    range.minimum = FiniteQuantity::new(-1000.0, range.minimum.unit().clone()).unwrap();
    let revised = compile_owned_release_revision(
        next,
        OwnedReleaseRevisionInput {
            schema_version: 1,
            before: next.receipt().input,
            release: OwnedDefinitionKey::new("test-occurrence-usage-bound-revision").unwrap(),
            reason: OwnedDefinitionKey::new("test-inherited-occurrence-usage-rebind").unwrap(),
            definitions: vec![],
            slots: vec![descriptor],
        },
        Default::default(),
    )
    .unwrap();
    assert_ne!(next.receipt().definitions, revised.receipt().definitions);
    let mut expected = policy.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        roles,
        scalar_inputs,
        ..
    } = &mut expected
    else {
        unreachable!()
    };
    *definitions = revised.receipt().definitions.clone();
    *roles = *revised.roles().identity();
    *scalar_inputs =
        gem_inventory_scalar_inputs_identity(revised.normalization(), Default::default()).unwrap();
    assert_eq!(
        revised.normalization().usage_inputs.as_ref(),
        Some(&expected)
    );
    assert!(revised.input().query_sets == next.input().query_sets);

    let mut stale = revised.input().clone();
    stale.normalization.usage_inputs = Some(policy.clone());
    // Refresh dependent commitments so rejection tests the stale policy itself.
    let usage_identity = usage_inputs_identity(&stale.normalization, Default::default()).unwrap();
    match stale.normalization.gem_inventory.as_mut().unwrap() {
        GemInventoryPolicy::PobFreshPhysicalV2 { usage_inputs, .. }
        | GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. } => {
            *usage_inputs = usage_identity
        }
        _ => panic!("expected usage-bound physical inventory"),
    }
    stale.tree.as_mut().unwrap().normalization = digest_owned(
        "owned-normalization-policy-v3",
        &stale.normalization,
        TreePolicyLimits::default().max_base_policy_bytes,
    )
    .unwrap();
    assert!(
        assemble_owned_release(stale, Default::default()).is_err(),
        "an explicit stale V3 policy cannot inherit schema rebind authority"
    );
}

fn compare(case: usize, xml: &[u8], out: &Path, prior: &Path, package: &Path) -> Value {
    let old_dir = out.join(format!("prior-original-{case:02}"));
    let new_dir = out.join(format!("original-{case:02}"));
    let mut before: Value = read(old_dir.join("draft.json"));
    let mut after: Value = read(new_dir.join("draft.json"));
    let mut selection = selected::selection(xml, &new_dir);
    let mut provenance: Value = read(new_dir.join("sidecar.json"));
    let count_rules: Value = family::read("usage.json");
    let instance = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([118; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let attr = |r: &poe_optimizer_import::owned_source::SourceEvidenceRow<'_>, name: &str| {
        r.attribute(name).map(|a| a.decoded().unwrap().to_owned())
    };
    // Resolve the existing bindings before canonicalization replaces real lineages.
    for preset in before["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
    {
        for binding in preset["intent"]["generated_inputs"]["members"]
            .as_array()
            .unwrap()
        {
            let target: poe_optimizer_core::owned_draft::GeneratedSkillKeyDraft =
                serde_json::from_value(binding["target"].clone()).unwrap();
            assert!(
                target.to_resolved().is_some(),
                "existing raw binding is exact"
            );
        }
    }
    selected::canonical(&mut before);
    selected::canonical(&mut after);
    selected::canonical(&mut selection);
    selected::canonical(&mut provenance);
    let has_link = |source, kind: &str, value: &Value| {
        provenance["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|origin| {
                origin["source"] == json!(source)
                    && origin["links"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|link| link["kind"] == kind && &link["value"] == value)
            })
    };
    let mut total = 0;
    let mut active = 0;
    for (old, new) in before["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .zip(
            after["draft"]["skill_presets"]["members"]
                .as_array_mut()
                .unwrap(),
        )
    {
        let old_usage = &old["intent"]["usage"];
        assert_eq!(
            new["intent"]["usage"]["completion"],
            old_usage["completion"]
        );
        let selected = new["id"] == selection["build"]["skills"];
        let rows = new["intent"]["usage"]["members"].as_array_mut().unwrap();
        let added = rows.len() - old_usage["members"].as_array().unwrap().len();
        total += added;
        if selected {
            active += added;
        }
        let mut targets = std::collections::BTreeSet::new();
        for row in rows.iter().skip(rows.len() - added) {
            let policy = &row["selection"];
            assert_eq!(policy["policy"]["value"]["key"], "def.000000000000326a");
            assert_eq!(policy["parameters"]["completion"]["kind"], "complete");
            let parameters = policy["parameters"]["members"].as_array().unwrap();
            assert_eq!(parameters.len(), 1);
            assert_eq!(
                parameters[0]["value"],
                json!({"kind":"known","value":{"kind":"integer","value":1}})
            );
            assert_eq!(policy["target"]["kind"], "skill");
            let target = &policy["target"]["value"];
            assert!(
                targets.insert(target.to_string()),
                "one count per exact target"
            );
            match target["kind"].as_str().unwrap() {
                "generated" => {
                    let bindings: Vec<_> = old["intent"]["generated_inputs"]["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|binding| binding["target"] == target["value"])
                        .collect();
                    assert_eq!(
                        bindings.len(),
                        1,
                        "count uses the existing exact raw-input target"
                    );
                    assert_eq!(bindings[0]["parameters"]["completion"]["kind"], "complete");
                    assert_eq!(bindings[0]["applicability"], "when_exact_source_selected");
                }
                "authored" => {
                    assert_eq!(target["value"]["kind"], "known");
                    let id = &target["value"]["value"];
                    let skills: Vec<_> = before["draft"]["skills"]["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|skill| &skill["id"] == id)
                        .collect();
                    assert_eq!(skills.len(), 1);
                    assert_eq!(skills[0]["source"]["kind"], "direct");
                    assert_eq!(skills[0]["source"]["value"]["kind"], "known");
                    let sources: Vec<_> = evidence
                        .rows()
                        .iter()
                        .filter(|gem| {
                            if gem.occurrence().name() != "Gem"
                                || !has_link(gem.occurrence().id(), "skill", id)
                            {
                                return false;
                            }
                            let group = &evidence.rows()
                                [gem.occurrence().parent().unwrap().ordinal() as usize];
                            if group.occurrence().name() != "Skill"
                                || group.attribute("source").is_some()
                            {
                                return false;
                            }
                            let set = &evidence.rows()
                                [group.occurrence().parent().unwrap().ordinal() as usize];
                            set.occurrence().name() == "SkillSet"
                                && has_link(set.occurrence().id(), "skill_preset", &old["id"])
                                && count_rules.as_array().unwrap().iter().any(|rule| {
                                    let expected = &rule["target"];
                                    expected["kind"] == "authored_direct"
                                        && expected["skill"]
                                            == skills[0]["source"]["value"]["value"]
                                        && [
                                            ("gemId", "game_id"),
                                            ("variantId", "variant_id"),
                                            ("skillId", "skill_id"),
                                            ("nameSpec", "name_spec"),
                                        ]
                                        .iter()
                                        .all(
                                            |(source, field)| {
                                                attr(gem, source).as_deref()
                                                    == expected[*field].as_str()
                                            },
                                        )
                                })
                        })
                        .collect();
                    assert_eq!(
                        sources.len(),
                        1,
                        "Direct count retains its exact source-linked SkillUse and preset"
                    );
                }
                kind => panic!("unexpected occurrence count target {kind}"),
            }
            assert_eq!(
                row["applicability"],
                if target["kind"] == "generated" {
                    "when_exact_source_selected"
                } else {
                    "required"
                }
            );
        }
        rows.truncate(rows.len() - added);
    }
    selected::canonical(&mut before);
    selected::canonical(&mut after);
    assert_eq!(
        before, after,
        "every prior value, ID, closure and original query survives inverse delta"
    );
    assert_eq!(total, [4, 0, 0, 0, 12][case - 1]);
    assert_eq!(active, [4, 0, 0, 0, 5][case - 1]);
    let a: Value = read(old_dir.join("sidecar.json"));
    let b: Value = read(new_dir.join("sidecar.json"));
    assert_eq!(a["schema_version"], 16);
    assert_eq!(b["schema_version"], 17);
    let old_report = finalize(
        xml,
        &old_dir,
        prior,
        &out.join(format!("prior-selection-{case:02}.json")),
    );
    let new_report = finalize(
        xml,
        &new_dir,
        package,
        &out.join(format!("selection-{case:02}.json")),
    );
    let mut old_issues = old_report["finalization"]["issues"].clone();
    let mut new_issues = new_report["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    assert_eq!(old_issues, new_issues, "no incomplete inventory is retired");
    assert_eq!(
        new_issues.as_array().unwrap().len(),
        [114, 117, 109, 122, 11][case - 1]
    );
    write(out.join(format!("report-{case:02}.json")), &new_report);
    json!({"original":case,"selected_added_counts":active,"all_preset_added_counts":total,"selected_unresolved":new_issues.as_array().unwrap().len(),"calculation":"not_run"})
}

fn edit(
    xml: &[u8],
    generated: bool,
    group_field: bool,
    field: &str,
    token: Option<&str>,
) -> (Vec<u8>, u32) {
    let instance = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([118; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let attr = |r: &poe_optimizer_import::owned_source::SourceEvidenceRow<'_>, name: &str| {
        r.attribute(name).map(|a| a.decoded().unwrap().to_owned())
    };
    let set = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "SkillSet" && attr(r, "id").as_deref() == Some("4"))
        .unwrap();
    let gem = evidence
        .rows()
        .iter()
        .find(|r| {
            if r.occurrence().name() != "Gem"
                || attr(r, "skillId").as_deref() != Some("SummonSandDjinnPlayer")
            {
                return false;
            }
            let group = &evidence.rows()[r.occurrence().parent().unwrap().ordinal() as usize];
            group.occurrence().parent() == Some(set.occurrence().id())
                && group.attribute("source").is_some() == generated
        })
        .unwrap();
    let row = if group_field {
        &evidence.rows()[gem.occurrence().parent().unwrap().ordinal() as usize]
    } else {
        gem
    };
    let range = row.occurrence().range();
    let text = std::str::from_utf8(&xml[range.clone()]).unwrap();
    let tag_end = text.find('>').unwrap();
    let head = &text[..tag_end];
    let needle = format!(" {field}=\"");
    let modified = if let Some(start) = head.find(&needle) {
        let end = start + needle.len() + head[start + needle.len()..].find('"').unwrap() + 1;
        format!(
            "{}{}{}",
            &text[..start],
            token
                .map(|v| format!(" {field}=\"{v}\""))
                .unwrap_or_default(),
            &text[end..]
        )
    } else {
        let insert = if head.ends_with('/') {
            tag_end - 1
        } else {
            tag_end
        };
        format!(
            "{}{}{}",
            &text[..insert],
            token
                .map(|v| format!(" {field}=\"{v}\""))
                .unwrap_or_default(),
            &text[insert..]
        )
    };
    (
        [
            xml[..range.start].to_vec(),
            modified.into_bytes(),
            xml[range.end..].to_vec(),
        ]
        .concat(),
        gem.occurrence().id().ordinal(),
    )
}
fn controls(out: &Path, package: &Path) {
    let original =
        fs::read(family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    for (name, generated, group, field, token, expected) in [
        (
            "generated-count-three",
            true,
            false,
            "count",
            Some("3"),
            Some(3),
        ),
        (
            "direct-count-three",
            false,
            false,
            "count",
            Some("3"),
            Some(3),
        ),
        (
            "generated-group-zero",
            true,
            true,
            "groupCount",
            Some("0"),
            Some(0),
        ),
        (
            "direct-group-four",
            false,
            true,
            "groupCount",
            Some("4"),
            Some(4),
        ),
        (
            "generated-count-five",
            true,
            false,
            "count",
            Some("5"),
            None,
        ),
        ("generated-count-missing", true, false, "count", None, None),
        (
            "generated-count-malformed",
            true,
            false,
            "count",
            Some("broken"),
            None,
        ),
        ("direct-count-nil", false, false, "count", Some("nil"), None),
        (
            "generated-group-malformed",
            true,
            true,
            "groupCount",
            Some("broken"),
            None,
        ),
        (
            "generated-quality-malformed",
            true,
            false,
            "quality",
            Some("broken"),
            Some(1),
        ),
    ] {
        let (xml, ordinal) = edit(&original, generated, group, field, token);
        let input = out.join(format!("{name}.xml"));
        fs::write(&input, &xml).unwrap();
        let dir = out.join(name);
        release::normalize(package, &input, 5, &dir);
        let draft: Value = read(dir.join("draft.json"));
        let selected = selected::selection(&xml, &dir);
        let preset = draft["draft"]["skill_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == selected["build"]["skills"])
            .unwrap();
        let new_rows: Vec<_> = preset["intent"]["usage"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .skip(6)
            .collect();
        assert_eq!(
            new_rows.len(),
            5,
            "{name}: exact occurrences survive scalar failure"
        );
        let sidecar: Value = read(dir.join("sidecar.json"));
        let source = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["source"]["ordinal"] == ordinal)
            .unwrap();
        let direct = source["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["kind"] == "skill")
            .map(|l| &l["value"]);
        let row = new_rows
            .iter()
            .find(|r| {
                let target = &r["selection"]["target"]["value"];
                if generated {
                    target["kind"] == "generated"
                        && target["value"]["slot"]["value"]["slot"]["key"] == "def.00000000000032d2"
                } else {
                    target["kind"] == "authored" && direct == Some(&target["value"]["value"])
                }
            })
            .unwrap();
        let parameters = &row["selection"]["parameters"];
        match expected {
            Some(value) => {
                assert_eq!(parameters["completion"]["kind"], "complete", "{name}");
                assert_eq!(
                    parameters["members"][0]["value"]["value"]["value"], value,
                    "{name}"
                );
            }
            None => assert_eq!(parameters["completion"]["kind"], "pending", "{name}"),
        }
        assert_eq!(preset["intent"]["usage"]["completion"]["kind"], "pending");
    }
}
#[test]
#[ignore = "requires exact generated-input publication and authenticated source reports"]
fn occurrence_count_publication_preserves_originals_and_exact_targets() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_OCCURRENCE_COUNT_PRIOR")
            .expect("explicit prior package"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_OCCURRENCE_COUNT_OUTPUT")
            .expect("new output directory"),
    );
    assert!(!out.exists());
    let old_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    inherited_policy_rebind(&next);
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
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
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
        let historical = prior_path
            .parent()
            .unwrap()
            .join(format!("original-{case:02}"));
        let replay = out.join(format!("prior-original-{case:02}"));
        let mut old_draft: Value = read(historical.join("draft.json"));
        let mut replay_draft: Value = read(replay.join("draft.json"));
        selected::canonical(&mut old_draft);
        selected::canonical(&mut replay_draft);
        assert_eq!(
            old_draft, replay_draft,
            "shared resolver preserves the preceding checked draft"
        );
        let old_side: Value = read(historical.join("sidecar.json"));
        let replay_side: Value = read(replay.join("sidecar.json"));
        let mut old_origins = old_side["origins"].clone();
        let mut replay_origins = replay_side["origins"].clone();
        selected::canonical(&mut old_origins);
        selected::canonical(&mut replay_origins);
        assert_eq!(
            old_origins, replay_origins,
            "historical source correspondence is preserved"
        );
        originals.push(compare(
            case,
            &fs::read(xml).unwrap(),
            &out,
            &prior_path,
            &package,
        ));
    }
    controls(&out, &package);
    assert_eq!(old_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"queries":110,"artifacts":18,"controls":10,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
