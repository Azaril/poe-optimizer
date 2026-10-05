//! Three explicit physical-Gem schemas and scalar inputs, preserving all originals.
#[path = "support/owned_minion_physical_inputs.rs"]
mod family;
#[path = "support/owned_canonical_instances.rs"]
mod instances;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    build_identity::*, owned_binding::*, owned_build::*, owned_content::digest_owned,
    owned_definitions::*, owned_draft::*, owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_gem_schema::GemSchemaMigrationInput,
    owned_normalize::{
        NormalizationArtifacts, NormalizationLimits, NormalizedImport, normalize_fresh,
    },
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct Bindings {
    gems: Vec<Binding>,
    sniper: Value,
}
#[derive(Deserialize)]
struct Binding {
    gem: GemDefId,
    game_id: String,
    variant_id: String,
    corrupted: DeclaredSlot<ParameterSlotDefId>,
    corruption_level: DeclaredSlot<ParameterSlotDefId>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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
fn minion_authoring_preserves_unknown_mechanics_and_missing_commands() {
    family::check_authored();
}

fn commitments(package: &StagedOwnedRelease, case: usize, draft: &DraftSession, sidecar: &Value) {
    assert_eq!(
        sidecar["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let queries = &package
        .query_sets()
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
    for (source, target) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(sidecar[source], receipt[target], "{source}");
    }
    assert_eq!(
        sidecar["mapping_source"],
        json!(package.mapping().source_identity())
    );
}
fn compare_original(
    case: usize,
    xml: &[u8],
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    bindings: &Bindings,
    out: &Path,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let old_session = decode_draft(
        &fs::read(old.join("draft.json")).unwrap(),
        Default::default(),
    )
    .unwrap();
    let new_session = decode_draft(
        &fs::read(new.join("draft.json")).unwrap(),
        Default::default(),
    )
    .unwrap();
    let old_lineage = old_session.input().allocator.lineage();
    let new_lineage = new_session.input().allocator.lineage();
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    commitments(prior, case, &old_session, &sa);
    commitments(next, case, &new_session, &sb);
    assert_eq!(
        old_session.input().allocator.last_issued(),
        new_session.input().allocator.last_issued()
    );
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        new_lineage,
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let mut counts = BTreeMap::new();
    let mut selected_occurrences = 0;
    let selection = selected::selection(xml, &new);
    let selected_id: SkillPresetId =
        serde_json::from_value(selection["build"]["skills"].clone()).unwrap();
    let selected_preset = new_session
        .input()
        .skill_presets
        .members
        .iter()
        .find(|row| row.id == selected_id)
        .unwrap();
    for (index, (before, gem)) in old_session
        .input()
        .gems
        .members
        .iter()
        .zip(&new_session.input().gems.members)
        .enumerate()
    {
        let definition = gem.definition.to_resolved().unwrap();
        let Some(binding) = bindings.gems.iter().find(|row| row.gem == definition) else {
            continue;
        };
        assert!(before.parameters.members.is_empty());
        assert!(
            matches!(&gem.parameters.completion, DraftListCompletion::Pending { code, .. } if code.as_str() == "gem-parameters-not-converted")
        );
        assert!(matches!(
            before.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(gem.parameters.members.len(), 2);
        let origins: Vec<_> = sb["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|origin| {
                origin["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|link| link["kind"] == "gem" && link["value"] == json!(gem.id))
            })
            .collect();
        assert_eq!(origins.len(), 1);
        let row = evidence
            .rows()
            .iter()
            .find(|row| json!(row.occurrence().id()) == origins[0]["source"])
            .unwrap();
        assert_eq!(
            row.attribute("gemId").unwrap().decoded().unwrap(),
            binding.game_id
        );
        assert_eq!(
            row.attribute("variantId").unwrap().decoded().unwrap(),
            binding.variant_id
        );
        let flag = match row.attribute("corrupted").unwrap().decoded().unwrap() {
            "true" => true,
            "false" | "nil" => false,
            _ => panic!("outside authenticated original corruption tokens"),
        };
        let delta = match row.attribute("corruptLevel").unwrap().decoded().unwrap() {
            "nil" => 0.,
            value => value.parse::<f64>().unwrap(),
        };
        let SchemaLookup::Known(schema) = next.assembled().schema().slot(&binding.corruption_level)
        else {
            panic!("known delta schema")
        };
        let ValueSchema::Quantity(range) = &schema.value else {
            panic!("typed delta")
        };
        let expected = [
            ParameterAssignment {
                slot: binding.corrupted.clone(),
                value: ParameterValue::Boolean(flag),
            },
            ParameterAssignment {
                slot: binding.corruption_level.clone(),
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(delta, range.minimum.unit().clone()).unwrap(),
                ),
            },
        ];
        let actual: Vec<_> = gem
            .parameters
            .members
            .iter()
            .map(|row| row.to_resolved().unwrap())
            .collect();
        assert_eq!(actual, expected);
        b["draft"]["gems"]["members"][index]["parameters"]["members"] = json!([]);
        *counts
            .entry(binding.gem.key().as_str().to_string())
            .or_insert(0usize) += 1;
        for link in origins[0]["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "skill")
        {
            let skill: SkillUseId = serde_json::from_value(link["value"].clone()).unwrap();
            selected_occurrences += usize::from(selected_preset.skills.members.contains(&skill));
        }
    }
    instances::canonical_instances(&mut a, old_lineage, &[]);
    instances::canonical_instances(&mut b, new_lineage, &[]);
    assert!(
        a == b,
        "original {case}: every prior value, preference, pending inventory, selection, and allocator survives exactly"
    );
    let current_item = sb["item_policy"].clone();
    let current_source = sb["item_source_policy"].clone();
    for (previous, current) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array_mut().unwrap())
    {
        assert_eq!(previous["attribution"]["item_lines"], sa["item_policy"]);
        assert_eq!(previous["attribution"]["policy"], sa["item_source_policy"]);
        assert_eq!(current["attribution"]["item_lines"], current_item);
        assert_eq!(current["attribution"]["policy"], current_source);
        current["attribution"]["item_lines"] = previous["attribution"]["item_lines"].clone();
        current["attribution"]["policy"] = previous["attribution"]["policy"].clone();
    }
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
    ] {
        sb[field] = sa[field].clone();
    }
    instances::canonical_instances(&mut sa, old_lineage, &[]);
    instances::canonical_instances(&mut sb, new_lineage, &[]);
    assert!(
        sa == sb,
        "original {case}: source sidecar retains all content and exact links apart from checked dependency identities"
    );
    let before = selected::finalize(
        xml,
        &old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        &new,
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
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    let issue_count = old_issues.as_array().unwrap().len();
    assert_eq!(issue_count, [116, 116, 108, 121, 19][case - 1]);
    assert_eq!(new_issues.as_array().unwrap().len(), issue_count);
    instances::canonical_instances(&mut old_issues, old_lineage, &[]);
    instances::canonical_instances(&mut new_issues, new_lineage, &[]);
    assert!(
        old_issues == new_issues,
        "every selected obligation remains exact"
    );
    let mut before_selection = selected::selection(xml, &old);
    let mut after_selection = selection;
    instances::canonical_instances(&mut before_selection, old_lineage, &[]);
    instances::canonical_instances(&mut after_selection, new_lineage, &[]);
    assert!(
        before_selection == after_selection,
        "all saved source selections preserved"
    );
    let count: usize = counts.values().sum();
    json!({"original":case,"gem_occurrences":count,"by_definition":counts,"scalar_assignments":count*2,"selected_occurrences":selected_occurrences,
        "selected_before":issue_count,"selected_after":issue_count,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn normalize_probe(
    package: &StagedOwnedRelease,
    binding: &Binding,
    attributes: &str,
) -> NormalizedImport {
    let xml = format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="1"><Skill enabled="true"><Gem gemId="{}" variantId="{}" enabled="true" {attributes}/></Skill></SkillSet></Skills></PathOfBuilding2>"#,
        binding.game_id, binding.variant_id
    );
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            mappings: package.mapping(),
            registry: package.assembled().registry(),
            definitions: package.assembled().schema(),
            roles: package.roles(),
            rewards: package.rewards(),
            items: package.items(),
            item_source: package.item_source(),
            tree: package.tree(),
        },
        package.normalization(),
        &[],
        Default::default(),
    )
    .unwrap()
}
fn rejects_level(package: &StagedOwnedRelease, gem: &GemDraft) -> bool {
    // A focused owned binding fixture, not finalization of an incomplete imported build.
    let schema = package.assembled().schema();
    let class = schema
        .input()
        .definitions
        .iter()
        .find_map(|row| {
            if let DefinitionDescriptor::Class(row) = row {
                Some(row.id.clone())
            } else {
                None
            }
        })
        .unwrap();
    let encounter = schema
        .input()
        .definitions
        .iter()
        .find_map(|row| {
            if let DefinitionDescriptor::Encounter(row) = row {
                Some(row.id.clone())
            } else {
                None
            }
        })
        .unwrap();
    let lineage = BuildLineage::from_bytes([91; 16]);
    let loadout = WeaponLoadoutId::from_instance_id(InstanceId::from_parts(lineage, 1).unwrap());
    let id = GemInstanceId::from_instance_id(InstanceId::from_parts(lineage, 2).unwrap());
    let namespace = schema.namespace().clone();
    let limits = OwnedInputLimits::default();
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(
            BuildInput {
                generated_inputs: None,
                allocator: InstanceAllocatorState::from_parts(lineage, 2),
                revision: BuildRevision::from_u64(1),
                game_version: namespace.clone(),
                character: CharacterSpec {
                    class,
                    ascendancy: None,
                    level: 1,
                    rewards: vec![],
                },
                weapon_loadouts: vec![loadout],
                active_weapon_loadout: loadout,
                items: vec![],
                gems: vec![GemInstance {
                    id,
                    definition: gem.definition.to_resolved().unwrap(),
                    level: gem.level.to_resolved().unwrap(),
                    quality: gem.quality.to_resolved().unwrap(),
                    parameters: gem
                        .parameters
                        .members
                        .iter()
                        .map(|row| row.to_resolved().unwrap())
                        .collect(),
                }],
                equipment: vec![],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                support_origins: None,
                payload_links: vec![],
                choices: vec![],
            },
            limits,
        )
        .unwrap(),
        ScenarioSpec::new(
            ScenarioInput {
                game_version: namespace.clone(),
                enemy: EnemySpec {
                    encounter,
                    level: 1,
                },
                assumptions: vec![],
                usage: vec![],
            },
            limits,
        )
        .unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: namespace,
                requests: vec![],
            },
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap();
    let report = bind_owned_request(schema, &request, Default::default()).unwrap();
    report.issues().iter().any(|issue| {
        issue.site.location == BindingLocation::Gem(id)
            && issue.site.facet == BindingFacet::Level
            && issue.class == IssueClass::Invalid
            && issue.code == BindingIssueCode::OutOfRange
    })
}
fn malformed_controls(package: &StagedOwnedRelease, bindings: &Bindings) -> usize {
    let base = r#"level="20" quality="0.5" corrupted="false" corruptLevel="1""#;
    let mut count = 0;
    for binding in &bindings.gems {
        for level in ["1", "20", "21", "40", "0", "41"] {
            let result = normalize_probe(
                package,
                binding,
                &base.replace("level=\"20\"", &format!("level=\"{level}\"")),
            );
            let gem = &result.draft().input().gems.members[0];
            assert_eq!(gem.level.to_resolved(), Some(level.parse().unwrap()));
            assert_eq!(
                rejects_level(package, gem),
                matches!(level, "0" | "41"),
                "schema binding excludes out-of-range parsed integers without clamping"
            );
            assert!(matches!(
                gem.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            count += 1;
        }
        for (needle, replacement, expected_members, pending_level, pending_quality) in [
            ("level=\"20\"", "", 2, true, false),
            ("level=\"20\"", "level=\"1.5\"", 2, true, false),
            ("quality=\"0.5\"", "", 2, false, true),
            ("quality=\"0.5\"", "quality=\"bad\"", 2, false, true),
            ("corrupted=\"false\"", "", 1, false, false),
            ("corrupted=\"false\"", "corrupted=\"bad\"", 1, false, false),
            ("corruptLevel=\"1\"", "", 1, false, false),
            (
                "corruptLevel=\"1\"",
                "corruptLevel=\"bad\"",
                1,
                false,
                false,
            ),
            ("corrupted=\"false\"", "corrupted=\"nil\"", 2, false, false),
            (
                "corruptLevel=\"1\"",
                "corruptLevel=\"nil\"",
                2,
                false,
                false,
            ),
        ] {
            let result = normalize_probe(package, binding, &base.replace(needle, replacement));
            let gem = &result.draft().input().gems.members[0];
            assert_eq!(gem.parameters.members.len(), expected_members);
            assert_eq!(gem.level.to_resolved().is_none(), pending_level);
            assert_eq!(gem.quality.to_resolved().is_none(), pending_quality);
            assert!(matches!(
                gem.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            for parameter in &gem.parameters.members {
                let parameter = parameter.to_resolved().unwrap();
                match parameter.value {
                    ParameterValue::Boolean(value) => assert!(!value),
                    ParameterValue::Quantity(value) => assert_eq!(
                        value.value(),
                        if replacement == "corruptLevel=\"nil\"" {
                            0.
                        } else {
                            1.
                        }
                    ),
                    _ => panic!("only authored scalar kinds"),
                }
            }
            count += 1;
        }
    }
    count
}

#[test]
#[ignore = "requires checked usage-input predecessor and authenticated full-source minion witness"]
fn minion_physical_inputs_publication_preserves_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_INPUTS_PRIOR").expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_INPUTS_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    let bindings: Bindings = family::read("bindings.json");
    assert_eq!(bindings.gems.len(), 3);
    let sniper: GemDefId = serde_json::from_value(bindings.sniper["gem"].clone()).unwrap();
    assert_eq!(
        next.assembled()
            .schema()
            .lookup_definition(&sniper.address()),
        prior
            .assembled()
            .schema()
            .lookup_definition(&sniper.address())
    );
    let migration: GemSchemaMigrationInput = family::read("migration.json");
    assert!(migration.gems.iter().all(|row| row.id != sniper));
    for field in ["usage_scalar", "gem_scalar", "tree"] {
        let mut stale = next.input().clone();
        if field == "tree" {
            stale.tree = prior.input().tree.clone();
        } else {
            let policy = if field == "usage_scalar" {
                "usage_inputs"
            } else {
                "gem_inventory"
            };
            let mut value = serde_json::to_value(&stale.normalization).unwrap();
            value[policy]["scalar_inputs"] = json!(prior.receipt().input);
            stale.normalization = serde_json::from_value(value).unwrap();
            stale.tree.as_mut().unwrap().normalization = digest_owned(
                "owned-normalization-policy-v3",
                &stale.normalization,
                TreePolicyLimits::default().max_base_policy_bytes,
            )
            .unwrap();
        }
        assert!(
            assemble_owned_release(stale, Default::default()).is_err(),
            "stale {field} rejects"
        );
    }
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(publish(&package, &rebuilt), json!(next.receipt()));
    let published = release::inventory(&package);
    assert_eq!(published, release::inventory(&rebuilt));
    assert_eq!(published.len(), inventory.len());
    let mut reports = vec![];
    for case in 1..=5 {
        let file = format!("queries-original-{case:02}.json");
        assert_eq!(published.get(&file), inventory.get(&file));
        let path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&path).unwrap();
        release::normalize(
            &prior_path,
            &path,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &path,
            case,
            &out.join(format!("original-{case:02}")),
        );
        let mut report = compare_original(case, &xml, &prior, &next, &bindings, &out);
        report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&xml)));
        reports.push(report);
    }
    assert_eq!(
        reports
            .iter()
            .map(|row| row["gem_occurrences"].as_u64().unwrap())
            .sum::<u64>(),
        15
    );
    assert_eq!(
        reports
            .iter()
            .map(|row| row["scalar_assignments"].as_u64().unwrap())
            .sum::<u64>(),
        30
    );
    assert_eq!(
        reports
            .iter()
            .map(|row| row["selected_occurrences"].as_u64().unwrap())
            .sum::<u64>(),
        6
    );
    let controls = malformed_controls(&next, &bindings);
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,
        "queries":110,"promoted_gems":3,"allocated_parameters":6,"gem_occurrences":15,"scalar_assignments":30,"selected_occurrences":6,
        "malformed_and_boundary_controls":controls,"stale_bindings":3,"prior_unchanged":true,"rebuild_byte_identical":true,"sniper_unchanged":true,"complete_original_builds":0}),
    );
}
