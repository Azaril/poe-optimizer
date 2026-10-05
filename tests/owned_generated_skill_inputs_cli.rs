//! Real-source generated input joins preserve independent saved alternatives.
#[path = "support/owned_generated_skill_inputs.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{FiniteQuantity, OwnedDefinitionKey},
    owned_draft::{DraftLimits, ResolveDraft, decode_draft},
    owned_schema::{SchemaState, SlotDescriptor, ValueSchema},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{GeneratedSkillInputPolicy, NormalizationLimits},
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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
fn finalize(xml: &[u8], directory: &Path, package: &Path, selection_path: &Path) -> Value {
    write(selection_path, &selected::selection(xml, directory));
    let report = run(
        "check-owned-draft",
        &[
            &directory.join("draft.json"),
            Path::new("--definitions"),
            &package.join("schema.json"),
            Path::new("--selection"),
            selection_path,
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
fn commitments(package: &StagedOwnedRelease, case: usize, path: &Path, sidecar: &Value) {
    let draft = decode_draft(
        &fs::read(path.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(
        sidecar["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let queries = &package
        .input()
        .query_sets
        .iter()
        .find(|set| set.name.as_str() == format!("original-{case:02}"))
        .unwrap()
        .queries;
    assert_eq!(
        sidecar["policy"],
        json!(
            digest_owned(
                "owned-normalization-policy-v3",
                &(package.normalization(), queries),
                NormalizationLimits::default().max_policy_bytes
            )
            .unwrap()
        )
    );
    let receipt = json!(package.receipt());
    for (field, binding) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(sidecar[field], receipt[binding], "{field}");
    }
    assert_eq!(
        sidecar["mapping_source"],
        json!(package.mapping().source_identity())
    );
    for item in sidecar["item_texts"].as_array().unwrap() {
        assert_eq!(item["attribution"]["item_lines"], sidecar["item_policy"]);
        assert_eq!(item["attribution"]["policy"], sidecar["item_source_policy"]);
    }
}
fn local(id: &Value) -> String {
    id["local"].as_str().unwrap().to_owned()
}
fn issued(allocator: &Value) -> u64 {
    u64::from_str_radix(allocator["last_issued"].as_str().unwrap(), 16).unwrap()
}
fn active_inputs<'a>(draft: &'a Value, selection: &Value) -> &'a Value {
    &draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == selection["build"]["skills"])
        .unwrap()["intent"]["generated_inputs"]
}

fn compare(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
    let old_dir = out.join(format!("prior-original-{case:02}"));
    let new_dir = out.join(format!("original-{case:02}"));
    let mut before: Value = read(old_dir.join("draft.json"));
    let mut after: Value = read(new_dir.join("draft.json"));
    let mut old_sidecar: Value = read(old_dir.join("sidecar.json"));
    let mut new_sidecar: Value = read(new_dir.join("sidecar.json"));
    commitments(prior, case, &old_dir, &old_sidecar);
    commitments(next, case, &new_dir, &new_sidecar);
    assert_eq!(old_sidecar["schema_version"], 15);
    assert_eq!(new_sidecar["schema_version"], 16);
    let selection = selected::selection(xml, &new_dir);
    let inputs = active_inputs(&after, &selection);
    assert_eq!(
        inputs["completion"]["kind"],
        if case == 5 { "complete" } else { "pending" }
    );
    assert_eq!(
        inputs["members"].as_array().unwrap().len(),
        [2, 0, 0, 0, 3][case - 1]
    );
    let mut issues = BTreeSet::new();
    let mut input_links = BTreeSet::new();
    let mut archived_pending = 0;
    let old_presets = before["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap();
    for (old, new) in old_presets.iter().zip(
        after["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap(),
    ) {
        let intent = new.as_object_mut().unwrap().remove("intent").unwrap();
        assert_eq!(intent["schema_version"], 1);
        assert!(new.get("usage_preferences").is_none());
        let expected = match old.get("usage_preferences") {
            Some(usage) => {
                let mut expected = usage.clone();
                for member in expected["members"].as_array_mut().unwrap() {
                    *member = json!({"selection":member.clone(),"applicability":"required"});
                }
                new["usage_preferences"] = usage.clone();
                expected
            }
            None => json!({"members":[],"completion":{"kind":"complete"}}),
        };
        let mut actual = intent["usage"].clone();
        let mut expected = expected;
        selected::canonical(&mut actual);
        selected::canonical(&mut expected);
        assert_eq!(
            actual, expected,
            "existing usage obligations and IDs survive explicit migration"
        );
        let generated = &intent["generated_inputs"];
        if generated["completion"]["kind"] == "pending" {
            if new["id"] == selection["build"]["skills"] {
                assert_ne!(
                    case, 5,
                    "Original05 selected generated sources are fully reviewed"
                );
                assert_eq!(
                    generated["completion"]["code"],
                    "generated-skill-inputs-not-converted"
                );
            } else {
                archived_pending += 1;
            }
            assert!(issues.insert(local(&generated["completion"]["id"])));
        }
        for input in generated["members"].as_array().unwrap() {
            assert_eq!(
                new["id"], selection["build"]["skills"],
                "no inferred cross-preset pairing"
            );
            assert_eq!(input["applicability"], "when_exact_source_selected");
            assert_eq!(input["parameters"]["completion"]["kind"], "complete");
            let parameters = input["parameters"]["members"].as_array().unwrap();
            assert_eq!(
                parameters.len(),
                1,
                "raw quality only, never provider level"
            );
            assert_eq!(parameters[0]["value"]["kind"], "known");
            assert_eq!(parameters[0]["value"]["value"]["value"]["value"], 0.0);
            // The importer links complete generated keys, not draft wrappers.
            let resolved = serde_json::from_value::<
                poe_optimizer_core::owned_draft::GeneratedSkillInputBindingDraft,
            >(input.clone())
            .unwrap()
            .to_resolved()
            .unwrap();
            input_links.insert(serde_json::to_string(&json!({"kind":"generated_skill_input","value":{"skill_preset":new["id"],"target":resolved.target}})).unwrap());
        }
    }
    assert_eq!(
        issued(&after["draft"]["allocator"]) - issued(&before["draft"]["allocator"]),
        issues.len() as u64,
        "only new Pending obligations allocate IDs"
    );
    after["draft"]["allocator"] = before["draft"]["allocator"].clone();
    selected::canonical(&mut before);
    selected::canonical(&mut after);
    assert!(
        before == after,
        "every preexisting occurrence, declaration, saved alternative and query survives"
    );
    let mut linked = BTreeSet::new();
    for row in new_sidecar["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            if link["kind"] == "generated_skill_input" {
                let text = serde_json::to_string(link).unwrap();
                assert!(
                    input_links.contains(&text),
                    "provenance must address an actual exact binding"
                );
                linked.insert(text);
                return false;
            }
            !(link["kind"] == "issue" && issues.contains(&local(&link["value"])))
        });
    }
    assert_eq!(linked, input_links);
    // Each commitment was independently authenticated above before restoring its
    // exact prior counterpart. Source provenance and all other fields compare.
    for field in [
        "schema_version",
        "allocator_after",
        "draft",
        "policy",
        "mapping",
        "registry",
        "definitions",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
    ] {
        new_sidecar[field] = old_sidecar[field].clone();
    }
    for (old, new) in old_sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(new_sidecar["item_texts"].as_array_mut().unwrap())
    {
        for field in ["item_lines", "policy"] {
            new["attribution"][field] = old["attribution"][field].clone();
        }
    }
    selected::canonical(&mut old_sidecar);
    selected::canonical(&mut new_sidecar);
    assert!(
        old_sidecar == new_sidecar,
        "exact source correspondence and existing obligations remain"
    );
    let old_report = selected::finalize(
        xml,
        &old_dir,
        &out.join(format!("prior-selection-{case:02}.json")),
    );
    let new_report = finalize(
        xml,
        &new_dir,
        &out.join("package"),
        &out.join(format!("selection-{case:02}.json")),
    );
    write(out.join(format!("report-{case:02}.json")), &new_report);
    let mut old_issues = old_report["finalization"]["issues"].clone();
    let mut new_issues = new_report["finalization"]["issues"].clone();
    let count = new_issues.as_array().unwrap().len();
    let mut surfaced = 0;
    new_issues.as_array_mut().unwrap().retain(|issue| {
        if issues.contains(&local(&issue["id"])) {
            assert_eq!(issue["code"], "generated-skill-inputs-not-converted");
            surfaced += 1;
            false
        } else {
            true
        }
    });
    assert_eq!(surfaced, usize::from(case != 5));
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    assert_eq!(
        old_issues.as_array().unwrap().len(),
        new_issues.as_array().unwrap().len()
    );
    for (old, new) in old_issues
        .as_array()
        .unwrap()
        .iter()
        .zip(new_issues.as_array_mut().unwrap())
    {
        let path = new["path"].as_str().unwrap();
        if path.starts_with("skill_presets.members[") && path.contains(".intent.usage.") {
            let mut restored = path.replace(".intent.usage.", ".usage_preferences.");
            if restored.contains(".usage_preferences.members[") {
                restored = restored.replace(".selection.", ".");
            }
            new["path"] = json!(restored);
        }
        assert_eq!(
            old, new,
            "existing obligation keeps its ID, code and owner; only the explicit usage envelope changes its path"
        );
    }
    assert_eq!(count, [114, 117, 109, 122, 11][case - 1]);
    let mut old_selection = selected::selection(xml, &old_dir);
    let mut new_selection = selection;
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    json!({"original":case,"source_sha256":format!("{:x}",Sha256::digest(xml)),"active_generated_inputs":([2,0,0,0,3][case-1]),"archived_input_obligations":archived_pending,"new_selected_obligations":surfaced,"selected_unresolved":count,"calculation":"not_run"})
}

#[test]
fn generated_input_authoring_preserves_level_authority_and_partial_coverage() {
    family::check_authored();
}

fn inherited_policy_rebind(next: &StagedOwnedRelease) {
    let policy = next
        .normalization()
        .generated_skill_inputs
        .as_ref()
        .unwrap();
    let GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { rows, .. } = policy;
    let slot = &rows[0].parameters[0].slot;
    let mut descriptor = next
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| matches!(row,SlotDescriptor::Parameter(p) if &p.id==slot))
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
            release: OwnedDefinitionKey::new("test-generated-raw-bound-revision").unwrap(),
            reason: OwnedDefinitionKey::new("test-inherited-generated-policy-rebind").unwrap(),
            definitions: vec![],
            slots: vec![descriptor],
        },
        Default::default(),
    )
    .unwrap();
    let mut expected = policy.clone();
    let GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
        definitions, roles, ..
    } = &mut expected;
    *definitions = revised.receipt().definitions.clone();
    *roles = *revised.roles().identity();
    assert_eq!(
        revised.normalization().generated_skill_inputs.as_ref(),
        Some(&expected)
    );
    assert!(revised.input().query_sets == next.input().query_sets);
    let mut stale = revised.input().clone();
    stale.normalization.generated_skill_inputs = Some(policy.clone());
    stale.tree.as_mut().unwrap().normalization = digest_owned(
        "owned-normalization-policy-v3",
        &stale.normalization,
        TreePolicyLimits::default().max_base_policy_bytes,
    )
    .unwrap();
    assert!(
        assemble_owned_release(stale, Default::default()).is_err(),
        "explicit stale replacement cannot inherit rebind authority"
    );
}

#[test]
#[ignore = "requires exact Command publication and authenticated generated-input source reports"]
fn generated_input_publication_preserves_five_originals_and_exact_provider_joins() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_INPUT_PRIOR")
            .expect("explicit checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_INPUT_OUTPUT")
            .expect("new output directory"),
    );
    assert!(!out.exists());
    let old_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    inherited_policy_rebind(&next);
    for field in ["definitions", "roles", "catalog", "source", "tree"] {
        let mut bad = next.input().clone();
        if field == "tree" {
            bad.tree = prior.input().tree.clone();
        } else {
            let GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
                definitions,
                roles,
                catalog,
                source,
                ..
            } = bad.normalization.generated_skill_inputs.as_mut().unwrap();
            match field {
                "definitions" => *definitions = prior.receipt().definitions.clone(),
                "roles" => *roles = *prior.roles().identity(),
                "catalog" => *catalog = digest_owned("stale-generated-catalog", &0, 100).unwrap(),
                "source" => source.revision.push_str("-stale"),
                _ => unreachable!(),
            }
            bad.tree.as_mut().unwrap().normalization = digest_owned(
                "owned-normalization-policy-v3",
                &bad.normalization,
                TreePolicyLimits::default().max_base_policy_bytes,
            )
            .unwrap();
        }
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale {field}"
        );
    }
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
    let inventory = release::inventory(&package);
    assert_eq!(inventory, release::inventory(&out.join("rebuilt")));
    assert_eq!(inventory.len(), 18);
    let mut reports = vec![];
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(inventory[&name], old_inventory[&name]);
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
        reports.push(compare(case, &fs::read(xml).unwrap(), &out, &prior, &next));
    }
    source_controls(&out, &package);
    assert_eq!(old_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"queries":110,"artifacts":18,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}

fn source_controls(out: &Path, package: &Path) {
    let original =
        fs::read(family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let draft = decode_draft(
        &fs::read(out.join("original-05/draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(&original).unwrap(),
        draft.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let rows = evidence.rows();
    let skills = rows
        .iter()
        .find(|row| row.occurrence().name() == "Skills")
        .unwrap();
    let active = skills
        .attribute("activeSkillSet")
        .unwrap()
        .decoded()
        .unwrap();
    let preset = rows
        .iter()
        .find(|row| {
            row.occurrence().name() == "SkillSet"
                && row.occurrence().parent() == Some(skills.occurrence().id())
                && row.attribute("id").unwrap().decoded().unwrap() == active
        })
        .unwrap();
    let groups: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.occurrence().name() == "Skill"
                && row.occurrence().parent() == Some(preset.occurrence().id())
                && row.attribute("source").is_some_and(|a| {
                    a.decoded().unwrap().starts_with("Tree:")
                        || a.decoded().unwrap().starts_with("Item:")
                })
        })
        .collect();
    assert_eq!(groups.len(), 3);
    let mut completed = 0;
    for (group_index, group) in groups.iter().enumerate() {
        let gem = rows
            .iter()
            .find(|row| {
                row.occurrence().name() == "Gem"
                    && row.occurrence().parent() == Some(group.occurrence().id())
            })
            .unwrap();
        for (name, attribute, replacement, admitted) in [
            ("quality", gem.attribute("quality").unwrap(), "12.5", true),
            ("stale-level", gem.attribute("level").unwrap(), "999", false),
            (
                "invalid-quality",
                gem.attribute("quality").unwrap(),
                "unknown",
                false,
            ),
            (
                "stale-source",
                group.attribute("source").unwrap(),
                "Tree:999999",
                false,
            ),
        ] {
            let mut xml = String::from_utf8(original.clone()).unwrap();
            xml.replace_range(attribute.range(), replacement);
            let stem = format!("control-{group_index}-{name}");
            let path = out.join(format!("{stem}.xml"));
            fs::write(&path, &xml).unwrap();
            let directory = out.join(&stem);
            release::normalize(package, &path, 5, &directory);
            let changed: Value = read(directory.join("draft.json"));
            let selection = selected::selection(xml.as_bytes(), &directory);
            let inputs = active_inputs(&changed, &selection);
            assert_eq!(
                inputs["completion"]["kind"],
                if admitted || name == "invalid-quality" {
                    "complete"
                } else {
                    "pending"
                },
                "{stem}"
            );
            if admitted {
                let values: Vec<_> = inputs["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| {
                        row["parameters"]["members"][0]["value"]["value"]["value"]["value"]
                            .as_f64()
                            .unwrap()
                    })
                    .collect();
                assert_eq!(values.len(), 3);
                assert_eq!(values.iter().filter(|v| **v == 12.5).count(), 1);
                assert_eq!(values.iter().filter(|v| **v == 0.0).count(), 2);
            } else if name == "invalid-quality" {
                assert_eq!(inputs["members"].as_array().unwrap().len(), 3);
                assert_eq!(
                    inputs["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|row| row["parameters"]["members"][0]["value"]["kind"] == "pending")
                        .count(),
                    1
                );
            }
            finalize(
                xml.as_bytes(),
                &directory,
                package,
                &out.join(format!("{stem}-selection.json")),
            );
            completed += 1;
        }
    }
    assert_eq!(completed, 12);
    // Saved axes are independent. Reimporting an explicitly activated archived
    // SkillSet proves its own join; the initial import cannot infer that pairing.
    let archived: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.occurrence().name() == "SkillSet"
                && row.occurrence().parent() == Some(skills.occurrence().id())
                && row.attribute("id").unwrap().decoded().unwrap() != active
        })
        .collect();
    assert_eq!(archived.len(), 5);
    for set in archived {
        let id = set.attribute("id").unwrap().decoded().unwrap();
        let generated_groups: Vec<_> = rows
            .iter()
            .filter(|row| {
                row.occurrence().name() == "Skill"
                    && row.occurrence().parent() == Some(set.occurrence().id())
                    && row.attribute("source").is_some()
            })
            .collect();
        let reviewed_groups: Vec<_> = generated_groups
            .iter()
            .copied()
            .filter(|group| {
                rows.iter().any(|row| {
                    row.occurrence().parent() == Some(group.occurrence().id())
                        && row.attribute("skillId").is_some_and(|id| {
                            matches!(
                                id.decoded().unwrap(),
                                "SummonSandDjinnPlayer"
                                    | "SummonWaterDjinnPlayer"
                                    | "FireboltPlayer"
                            )
                        })
                })
            })
            .collect();
        let group = reviewed_groups[0];
        let gem = rows
            .iter()
            .find(|row| {
                row.occurrence().name() == "Gem"
                    && row.occurrence().parent() == Some(group.occurrence().id())
            })
            .unwrap();
        let mut changes = vec![
            (skills.attribute("activeSkillSet").unwrap().range(), id),
            (gem.attribute("quality").unwrap().range(), "12.5"),
        ];
        changes.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut xml = String::from_utf8(original.clone()).unwrap();
        for (range, value) in changes {
            xml.replace_range(range, value);
        }
        let stem = format!("activate-archived-{id}");
        let path = out.join(format!("{stem}.xml"));
        fs::write(&path, &xml).unwrap();
        let directory = out.join(&stem);
        release::normalize(package, &path, 5, &directory);
        let changed: Value = read(directory.join("draft.json"));
        let selection = selected::selection(xml.as_bytes(), &directory);
        let inputs = active_inputs(&changed, &selection);
        assert_eq!(
            inputs["completion"]["kind"],
            if reviewed_groups.len() == generated_groups.len() {
                "complete"
            } else {
                "pending"
            }
        );
        assert_eq!(
            inputs["members"].as_array().unwrap().len(),
            reviewed_groups.len()
        );
        assert_eq!(
            inputs["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(
                    |row| row["parameters"]["members"][0]["value"]["value"]["value"]["value"]
                        == 12.5
                )
                .count(),
            1
        );
        finalize(
            xml.as_bytes(),
            &directory,
            package,
            &out.join(format!("{stem}-selection.json")),
        );
    }
    // A changed item name or slot must not inherit the saved representation's
    // quality merely because the modifier definition still matches.
    let item_group = groups
        .iter()
        .find(|g| {
            g.attribute("source")
                .unwrap()
                .decoded()
                .unwrap()
                .starts_with("Item:")
        })
        .unwrap();
    let source_name = item_group.attribute("source").unwrap().decoded().unwrap();
    let item_key = source_name
        .strip_prefix("Item:")
        .unwrap()
        .split_once(':')
        .unwrap()
        .0;
    let item = rows
        .iter()
        .find(|row| {
            row.occurrence().name() == "Item"
                && row
                    .attribute("id")
                    .is_some_and(|a| a.decoded().unwrap() == item_key)
        })
        .unwrap();
    let range = item.occurrence().range();
    let text = std::str::from_utf8(&original).unwrap();
    let name_start = range.start + text[range.clone()].find("New Item").unwrap();
    for (stem, range, value) in [
        (
            "stale-item-name",
            name_start..name_start + "New Item".len(),
            "Renamed Item",
        ),
        (
            "stale-item-slot",
            item_group.attribute("slot").unwrap().range(),
            "Weapon 2",
        ),
    ] {
        let mut xml = text.to_owned();
        xml.replace_range(range, value);
        let path = out.join(format!("{stem}.xml"));
        fs::write(&path, &xml).unwrap();
        let directory = out.join(stem);
        release::normalize(package, &path, 5, &directory);
        let changed: Value = read(directory.join("draft.json"));
        let selection = selected::selection(xml.as_bytes(), &directory);
        assert_eq!(
            active_inputs(&changed, &selection)["completion"]["kind"],
            "pending"
        );
        finalize(
            xml.as_bytes(),
            &directory,
            package,
            &out.join(format!("{stem}-selection.json")),
        );
    }
}
