//! Persisted catalog and normalization artifacts through the public host CLI.
//! Every input path is explicit; the working directory contains no source checkout.
use poe_optimizer_core::{
    owned_definitions::{GemDefId, QualityDefId, UnitDefId},
    owned_draft::{
        DraftFinalization, DraftLimits, DraftListCompletion, EvaluationSelection, decode_draft,
    },
    owned_project::VariantSelection,
    owned_schema::{DefinitionSchemaIndex, SchemaLookup},
};
use poe_optimizer_data::owned_schema::{OwnedSchemaLimits, decode_schema_package};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::ImportQueryTemplate,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const FILES: [&str; 9] = [
    "registry.json",
    "schema.json",
    "rules.json",
    "routing.json",
    "manifest.json",
    "recipe.json",
    "mapping.json",
    "roles.json",
    "transition.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn inputs() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/import")
}
fn json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn extend(cwd: &Path, recipe: &Path, mapping: &Path, output: &Path) -> Output {
    let input = inputs();
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-skill-catalog")
        .arg(recipe)
        .arg("--catalog")
        .arg(input.join("skill-identities.json"))
        .arg("--mapping")
        .arg(mapping)
        .arg("--source")
        .arg(input.join("source-pin.json"))
        .arg("--policy")
        .arg(input.join("skill-catalog-policy.json"))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn normalize(cwd: &Path, case: usize, output: &Path) -> Output {
    let input = inputs();
    let compiled = input.join("compiled");
    let policies = input.join("policies");
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("normalize-owned")
        .arg(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .arg("--policy")
        .arg(policies.join("normalization.json"))
        .arg("--registry")
        .arg(compiled.join("registry.json"))
        .arg("--definitions")
        .arg(compiled.join("schema.json"))
        .arg("--mapping")
        .arg(compiled.join("mapping.json"))
        .arg("--roles")
        .arg(compiled.join("roles.json"))
        .arg("--rewards")
        .arg(policies.join("rewards.json"))
        .arg("--items")
        .arg(policies.join("items.json"))
        .arg("--item-source")
        .arg(policies.join("item-source.json"))
        .arg("--queries")
        .arg(input.join(format!("queries/original-{case:02}.json")))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn bundle(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
#[test]
fn real_catalog_cli_reproduces_nine_artifacts_and_second_pass_reuses_every_identity() {
    let temp = tempfile::tempdir().unwrap();
    let input = inputs();
    let first = temp.path().join("first");
    let transition = success(&extend(
        temp.path(),
        &input.join("recipe-seed.json"),
        &input.join("mapping-seed.json"),
        &first,
    ));
    assert_eq!(
        transition["counts"],
        serde_json::json!({
            "reused_gems":2,"reused_skills":4,"allocated_gems":964,"allocated_skills":1432
        })
    );
    assert_eq!(transition["calculation"], "not_run");
    assert_eq!(transition["whole_build_parity"], "not_established");
    let actual = bundle(&first);
    let expected = bundle(&input.join("compiled"));
    assert_eq!(actual.len(), FILES.len());
    assert_eq!(expected.len(), FILES.len());
    for name in FILES {
        assert!(
            actual[name] == expected[name],
            "emitted artifact differs: {name}"
        );
    }
    assert_eq!(json(first.join("transition.json")), transition);
    let second = temp.path().join("second");
    let repeated = success(&extend(
        temp.path(),
        &first.join("recipe.json"),
        &first.join("mapping.json"),
        &second,
    ));
    assert_eq!(
        repeated["counts"],
        serde_json::json!({
            "reused_gems":966,"reused_skills":1436,"allocated_gems":0,"allocated_skills":0
        })
    );
    for field in ["registry", "definitions", "mapping"] {
        assert_eq!(
            repeated[format!("before_{field}")],
            repeated[format!("after_{field}")]
        );
    }
    // Roles/transition carry the new operation's predecessor provenance. Runtime
    // packages and all owned inputs remain byte-identical after zero allocation.
    for name in [
        "registry.json",
        "schema.json",
        "rules.json",
        "routing.json",
        "manifest.json",
        "recipe.json",
        "mapping.json",
    ] {
        assert!(
            fs::read(first.join(name)).unwrap() == fs::read(second.join(name)).unwrap(),
            "second-pass artifact changed: {name}"
        );
    }
    assert_eq!(fs::read_dir(&second).unwrap().count(), 9);
    let a = json(first.join("manifest.json"));
    let b = json(second.join("manifest.json"));
    for key in [
        "registry",
        "definitions",
        "rules",
        "compiled_rules",
        "routing",
    ] {
        assert_eq!(a[key], b[key]);
    }
    assert!(a["partial_rule_owners"].as_u64().unwrap() > 0);
    assert!(a["partial_route_outputs"].as_u64().unwrap() > 0);
}
#[test]
fn stale_mapping_and_existing_destinations_never_publish_or_clobber() {
    let temp = tempfile::tempdir().unwrap();
    let input = inputs();
    let mut stale = json(input.join("mapping-seed.json"));
    stale["registry"] = Value::String("00".repeat(32));
    let path = temp.path().join("stale.json");
    fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let output = temp.path().join("unpublished");
    assert!(
        !extend(temp.path(), &input.join("recipe-seed.json"), &path, &output)
            .status
            .success()
    );
    assert!(!output.exists());
    assert_eq!(
        fs::read_dir(temp.path()).unwrap().count(),
        1,
        "failure leaked staging output"
    );
    let existing = temp.path().join("existing");
    fs::create_dir(&existing).unwrap();
    assert!(
        !extend(
            temp.path(),
            &input.join("recipe-seed.json"),
            &input.join("mapping-seed.json"),
            &existing
        )
        .status
        .success()
    );
    assert_eq!(
        fs::read_dir(&existing).unwrap().count(),
        0,
        "existing empty directory was replaced"
    );
    fs::write(existing.join("sentinel"), b"caller-owned").unwrap();
    let before = bundle(&existing);
    assert!(
        !extend(
            temp.path(),
            &input.join("recipe-seed.json"),
            &input.join("mapping-seed.json"),
            &existing
        )
        .status
        .success()
    );
    assert_eq!(bundle(&existing), before);
    assert_eq!(
        fs::read_dir(temp.path()).unwrap().count(),
        2,
        "failure leaked staging output"
    );
}
#[test]
fn all_five_normalize_with_persisted_ids_quality_rewards_and_every_query_still_pending() {
    let temp = tempfile::tempdir().unwrap();
    let input = inputs();
    let schema = decode_schema_package(
        &fs::read(input.join("compiled/schema.json")).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mut parameter_closure_totals = [0usize; 2]; // complete, pending
    let ids = json(root().join("data/owned/poe2/3887ae68/ids.json"));
    let known_gems: [GemDefId; 2] = ["twister-gem", "sniper-gem"]
        .map(|name| serde_json::from_value(ids["allocations"][name].clone()).unwrap());
    let quality_kind: QualityDefId =
        serde_json::from_value(ids["allocations"]["standard-quality"].clone()).unwrap();
    let quality_unit: UnitDefId =
        serde_json::from_value(ids["allocations"]["percentage-points"].clone()).unwrap();
    let mut reused = [0usize; 2];
    let mut totals = [0usize; 5]; // physical, zero quality, queries, source Gems, unresolved origins
    let expected_gems = [52, 153, 57, 59, 157];
    let expected_rewards = [16, 17, 15, 16, 17];
    let expected_zeros = [50, 146, 51, 52, 149];
    for case in 1..=5 {
        let output = temp.path().join(format!("original-{case:02}"));
        let report = success(&normalize(temp.path(), case, &output));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["structure"], "valid");
        assert_eq!(report["verification"]["artifact_bindings"], "checked");
        assert_eq!(report["verification"]["definitions"], "not_bound");
        assert_eq!(report["verification"]["legality"], "not_checked");
        assert_eq!(report["verification"]["calculation"], "not_run");
        assert_eq!(report["counts"]["gems"], expected_gems[case - 1]);
        assert_eq!(report["counts"]["rewards"], expected_rewards[case - 1]);
        assert_eq!(json(output.join("report.json")), report);
        assert_eq!(fs::read_dir(&output).unwrap().count(), 3);
        let limits = DraftLimits::default();
        let session = decode_draft(&fs::read(output.join("draft.json")).unwrap(), limits).unwrap();
        let draft = session.input();
        let sidecar = json(output.join("sidecar.json"));
        let validation = session.validate_limits(limits).unwrap();
        assert!(!validation.issues.is_empty());
        assert_eq!(report["issue_count"], validation.issues.len());
        assert_eq!(
            sidecar["draft"],
            serde_json::to_value(session.digest(limits.input.max_wire_bytes).unwrap()).unwrap()
        );
        let transition = json(input.join("compiled/transition.json"));
        assert_eq!(sidecar["registry"], transition["after_registry"]);
        assert_eq!(sidecar["mapping"], transition["after_mapping"]);
        assert_eq!(sidecar["definitions"], transition["after_definitions"]);
        assert_eq!(sidecar["skill_roles"], transition["roles"]);
        assert_eq!(draft.gems.members.len(), expected_gems[case - 1]);
        assert_eq!(
            draft
                .rewards
                .members
                .iter()
                .filter(|r| r.to_resolved().is_some())
                .count(),
            expected_rewards[case - 1]
        );
        assert!(draft.skill_presets.members.iter().all(|preset| matches!(
            preset.skills.completion,
            DraftListCompletion::Pending { .. }
        ) && matches!(
            preset.supports.completion,
            DraftListCompletion::Pending { .. }
        )));
        assert!(draft.choice_presets.members.iter().all(|preset| matches!(
            preset.rewards.completion,
            DraftListCompletion::Pending { .. }
        )));
        let gems: BTreeMap<_, _> = draft
            .gems
            .members
            .iter()
            .map(|gem| (serde_json::to_string(&gem.id).unwrap(), gem))
            .collect();
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            draft.allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        assert_eq!(sidecar["source_sha256"], source.source_sha256());
        assert_eq!(report["source"]["sha256"], source.source_sha256());
        let origins = sidecar["origins"].as_array().unwrap();
        assert_eq!(origins.len(), evidence.rows().len());
        assert_eq!(report["source"]["origin_rows"], origins.len());
        let mut linked_gems = BTreeSet::new();
        let mut zeros = 0;
        let mut known_empty_parameters = 0;
        let issue_ids: BTreeSet<_> = validation
            .issues
            .iter()
            .map(|issue| serde_json::to_string(&issue.id).unwrap())
            .collect();
        for (row, origin) in evidence.rows().iter().zip(origins) {
            assert_eq!(
                origin["source"],
                serde_json::to_value(row.occurrence().id()).unwrap()
            );
            let links = origin["links"].as_array().unwrap();
            for link in links.iter().filter(|link| link["kind"] == "issue") {
                assert!(issue_ids.contains(&serde_json::to_string(&link["value"]).unwrap()));
            }
            if !matches!(
                row.authored_instance(),
                Some(AuthoredInstanceId::SkillEntry(_))
            ) {
                continue;
            }
            totals[3] += 1;
            let physical: Vec<_> = links.iter().filter(|link| link["kind"] == "gem").collect();
            if physical.is_empty() {
                totals[4] += 1;
                assert!(links.iter().any(|link| link["kind"] == "issue"));
                continue;
            }
            assert_eq!(physical.len(), 1);
            let id = serde_json::to_string(&physical[0]["value"]).unwrap();
            assert!(linked_gems.insert(id.clone()));
            let gem = gems[&id];
            let quality = gem
                .quality
                .to_resolved()
                .flatten()
                .expect("explicit physical quality remains known");
            assert_eq!(quality.kind, quality_kind);
            assert_eq!(quality.amount.unit(), &quality_unit);
            let raw_quality: f64 = row
                .attribute("quality")
                .unwrap()
                .decoded()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(quality.amount.value(), raw_quality);
            zeros += usize::from(raw_quality == 0.0);
            for (index, definition) in known_gems.iter().enumerate() {
                reused[index] +=
                    usize::from(gem.definition.to_resolved().as_ref() == Some(definition));
            }
            let known_empty = gem.definition.to_resolved().is_some_and(|definition| {
                matches!(schema.definition(&definition), SchemaLookup::Known(schema)
                    if schema.declarations.parameters.is_complete()
                        && schema.declarations.parameters.members.is_empty())
            });
            assert!(gem.parameters.members.is_empty());
            known_empty_parameters += usize::from(known_empty);
            // Historical policies do not author the neutral-input proof.
            assert!(matches!(
                gem.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            parameter_closure_totals[1] += 1;
        }
        assert_eq!(known_empty_parameters, [1, 6, 0, 0, 5][case - 1]);

        assert_eq!(linked_gems.len(), draft.gems.members.len());
        assert_eq!(zeros, expected_zeros[case - 1]);
        let templates: Vec<ImportQueryTemplate> = serde_json::from_slice(
            &fs::read(input.join(format!("queries/original-{case:02}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(draft.query_presets.members.len(), 1);
        let queries = &draft.query_presets.members[0].queries;
        assert_eq!(queries.requests.members.len(), 22);
        assert!(matches!(
            queries.requests.completion,
            DraftListCompletion::Complete
        ));
        assert_eq!(
            queries
                .requests
                .members
                .iter()
                .map(|r| &r.id)
                .collect::<Vec<_>>(),
            templates.iter().map(|r| &r.id).collect::<Vec<_>>()
        );
        // The caller explicitly chooses proven preset/loadout members; this does
        // not infer saved active state or close any unconverted collection.
        let selection = EvaluationSelection {
            build: VariantSelection {
                character: draft.character_presets.members[0].id,
                equipment: draft.equipment_presets.members[0].id,
                allocations: draft.allocation_presets.members[0].id,
                skills: draft.skill_presets.members[0].id,
                choices: draft.choice_presets.members[0].id,
                active_weapon_loadout: draft.weapon_loadouts.members[0],
            },
            scenario: draft.scenario_presets.members[0].id,
            queries: draft.query_presets.members[0].id,
        };
        match session.finalize_selection(selection, limits).unwrap() {
            DraftFinalization::Pending {
                issues,
                queries: retained,
                ..
            } => {
                assert!(!issues.is_empty());
                assert_eq!(&retained, queries);
            }
            DraftFinalization::Ready(_) => panic!("partial import was incorrectly finalized"),
        }
        totals[0] += draft.gems.members.len();
        totals[1] += zeros;
        totals[2] += queries.requests.members.len();
    }
    assert_eq!(totals, [478, 448, 110, 541, 63]);
    assert_eq!(parameter_closure_totals, [0, 478]);
    assert!(
        reused.iter().all(|count| *count > 0),
        "both original recipe Gem IDs must survive normalization"
    );
}
