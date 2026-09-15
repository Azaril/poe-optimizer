//! Explicit injected artifacts, from an empty directory, with either CLI feature set.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{ParameterValue, QueryId},
    owned_content::digest_owned,
    owned_definitions::{GameVersionNamespace, OwnedDefinitionKey},
    owned_draft::{DraftField, DraftLimits, DraftListCompletion, decode_draft},
};
use poe_optimizer_data::owned_schema::{
    OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageInput, decode_schema_package,
    encode_schema_package,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_item_lines::*,
    owned_item_source::*,
    owned_mapping::*,
    owned_normalize::{
        GemQualityPolicy, ImportQueryTarget, ImportQueryTemplate, NormalizationPolicy,
    },
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_value::*,
    owned_value_policy::*,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const INPUT: &str = r#"<PathOfBuilding2><Build level="33"/><Tree><Spec classInternalId="caller-class" ascendancyInternalId="caller-ascendancy" treeVersion="caller-tree" nodes=""/></Tree><Items><Item id="1">unconverted item text</Item><ItemSet id="1"><Slot name="caller-slot" itemId="1"/></ItemSet></Items><Skills><SkillSet id="1"><Skill enabled="true"><Gem gemId="caller-gem" variantId="" level="7" enabled="true"/></Skill></SkillSet></Skills><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("caller-cli", "v4").unwrap()
}
fn recipe(name: &str, boolean: bool) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(if boolean {
            "caller-bool"
        } else {
            "caller-level"
        }),
        codec: ValueCodecInput {
            namespace: namespace(),
            whitespace: WhitespacePolicy::Exact,
            codec: if boolean {
                ValueCodecKind::Boolean {
                    tokens: vec![
                        BooleanToken {
                            token: "true".into(),
                            value: true,
                        },
                        BooleanToken {
                            token: "false".into(),
                            value: false,
                        },
                    ],
                }
            } else {
                ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                }
            },
        },
        tiers: vec![ValueTier {
            selectors: vec![ValueSelector {
                lane: ValueLane::Attribute,
                name: name.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: if boolean {
            MissingValuePolicy::Explicit {
                value: ParameterValue::Boolean(true),
            }
        } else {
            MissingValuePolicy::Pending
        },
    }
}
fn policy() -> NormalizationPolicy {
    NormalizationPolicy {
        version: key("caller-normalization"),
        namespace: namespace(),
        character_level: recipe("level", false),
        gem_level: recipe("level", false),
        gem_enabled: recipe("enabled", true),
        group_enabled: recipe("enabled", true),
        manual_skill_sources: vec![SourceComponent::Missing],
        empty_item_keys: vec![SourceComponent::Text("0".into())],
        generated_support_prefixes: vec![],
        allocation_attribute: "nodes".into(),
        single_active_support_target: false,
        equipment_loadouts: vec![],
        gem_quality: GemQualityPolicy::Unconverted,
    }
}
fn save(directory: &Path) {
    let limits = OwnedMappingLimits::default();
    let registry = OwnedIdRegistry::empty(namespace(), limits).unwrap();
    let definitions = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 1,
            namespace: namespace(),
            release: key("caller-release"),
            semantics_version: key("caller-schema"),
            definitions: vec![],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let source = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "caller-source-revision".into(),
        files: vec![SourceFilePin {
            path: "caller/definitions.lua".into(),
            sha256: "ab".repeat(32),
        }],
    };
    let mappings = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: 1,
            namespace: namespace(),
            registry: registry.identity().unwrap(),
            definitions: definitions.identity().clone(),
            source: source.clone(),
            policy_version: key("caller-mapping"),
            entries: vec![],
        },
        &registry,
        &definitions,
        limits,
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: 1,
            namespace: namespace(),
            definitions: definitions.identity().clone(),
            mapping: *mappings.identity(),
            roles: vec![],
            compilation: SkillCatalogReceipt {
                source,
                catalog_digest: digest_owned(
                    "caller-empty-catalog-v1",
                    &Vec::<u8>::new(),
                    limits.max_wire_bytes,
                )
                .unwrap(),
                policy: SkillCatalogPolicy {
                    version: key("caller-mapping"),
                    absent_support: AbsentSupportPolicy::Pending,
                    absent_from_tree: AbsentFromTreePolicy::Physical,
                },
                base_registry: registry.identity().unwrap(),
                staged_registry: registry.identity().unwrap(),
                gem_count: 0,
                skill_count: 0,
            },
        },
        &mappings,
        &definitions,
        SkillCatalogLimits { mapping: limits },
    )
    .unwrap();
    let rewards = OwnedRewardPolicy::new(
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: namespace(),
            version: key("caller-rewards"),
            definitions: definitions.identity().clone(),
            mapping: *mappings.identity(),
            rules: vec![],
        },
        &mappings,
        &definitions,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    let items = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: namespace(),
            version: key("caller-items"),
            definitions: definitions.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules: vec![],
        },
        &definitions,
        ItemLineLimits::default(),
    )
    .unwrap();
    let item_source = ItemSourceLayoutPolicy::new(
        ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
            namespace: namespace(),
            version: key("caller-item-source"),
            source: mappings.input().source.clone(),
            item_lines: *items.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextV1,
            rule_layouts: vec![],
            template_layouts: vec![],
        },
        &items,
        &definitions,
        ItemSourceLimits::default(),
    )
    .unwrap();
    let metric = ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Metric,
        key: SourceComponent::Text("caller-metric".into()),
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    };
    let queries = vec![
        ImportQueryTemplate {
            id: QueryId::new("z-first").unwrap(),
            metric: metric.clone(),
            target: ImportQueryTarget::Player,
        },
        ImportQueryTemplate {
            id: QueryId::new("a-second").unwrap(),
            metric,
            target: ImportQueryTarget::Unresolved(key("caller-target-pending")),
        },
    ];
    for (name, bytes) in [
        ("source.xml", INPUT.as_bytes().to_vec()),
        ("policy.json", serde_json::to_vec(&policy()).unwrap()),
        ("registry.json", encode_registry(&registry, limits).unwrap()),
        (
            "definitions.json",
            encode_schema_package(&definitions, OwnedSchemaLimits::default()).unwrap(),
        ),
        (
            "mapping.json",
            encode_mapping_package(&mappings, limits).unwrap(),
        ),
        ("roles.json", serde_json::to_vec(roles.input()).unwrap()),
        (
            "rewards.json",
            encode_reward_policy(&rewards, RewardPolicyLimits::default()).unwrap(),
        ),
        (
            "items.json",
            encode_item_line_policy(&items, ItemLineLimits::default()).unwrap(),
        ),
        (
            "item-source.json",
            encode_item_source_policy(&item_source, ItemSourceLimits::default()).unwrap(),
        ),
        ("queries.json", serde_json::to_vec(&queries).unwrap()),
    ] {
        fs::write(directory.join(name), bytes).unwrap();
    }
}
fn arguments(output: &str) -> Vec<&str> {
    vec![
        "normalize-owned",
        "source.xml",
        "--policy",
        "policy.json",
        "--registry",
        "registry.json",
        "--definitions",
        "definitions.json",
        "--mapping",
        "mapping.json",
        "--roles",
        "roles.json",
        "--rewards",
        "rewards.json",
        "--items",
        "items.json",
        "--item-source",
        "item-source.json",
        "--queries",
        "queries.json",
        "--output",
        output,
    ]
}
fn run(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap()
}
fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["document_kind"], "owned_normalization_report");
    assert_eq!(report["verification"]["structure"], "valid");
    assert_eq!(report["verification"]["artifact_bindings"], "checked");
    assert_eq!(report["verification"]["definitions"], "not_bound");
    assert_eq!(report["verification"]["legality"], "not_checked");
    assert_eq!(report["verification"]["calculation"], "not_run");
    report
}

#[test]
fn help_and_missing_required_artifacts_do_not_need_runtime_data() {
    let temp = tempfile::tempdir().unwrap();
    let help = run(temp.path(), &["normalize-owned", "--help"]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    for flag in [
        "--policy",
        "--registry",
        "--definitions",
        "--mapping",
        "--roles",
        "--rewards",
        "--items",
        "--item-source",
        "--queries",
        "--output",
    ] {
        assert!(text.contains(flag));
    }
    let output = run(
        temp.path(),
        &["normalize-owned", "source.xml", "--output", "result"],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--policy"));
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn explicit_artifacts_produce_a_checked_pending_draft_sidecar_and_summary() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let report = successful(run(temp.path(), &arguments("result")));
    assert_eq!(report["normalization_status"], "pending");
    assert!(report["issue_count"].as_u64().unwrap() > 0);
    assert_eq!(report["counts"]["items"], 1);
    assert_eq!(report["counts"]["equipment"], 1);
    // The injected role catalog is empty. Source gemId alone cannot establish
    // physical ownership, so this source remains pending without an instance.
    assert_eq!(report["counts"]["gems"], 0);
    assert_eq!(report["counts"]["skills"], 0);
    assert_eq!(report["counts"]["supports"], 0);
    assert_eq!(report["counts"]["character_presets"], 1);
    assert_eq!(
        report["source"]["occurrences"],
        report["source"]["origin_rows"]
    );
    let directory = temp.path().join("result");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 3);
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(draft.input().game_version, namespace());
    assert!(matches!(
        draft.input().character_presets.members[0].level,
        DraftField::Known { value: 33 }
    ));
    let queries = &draft.input().query_presets.members[0]
        .queries
        .requests
        .members;
    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].id, QueryId::new("z-first").unwrap());
    assert_eq!(queries[1].id, QueryId::new("a-second").unwrap());
    let sidecar: Value =
        serde_json::from_slice(&fs::read(directory.join("sidecar.json")).unwrap()).unwrap();
    assert_eq!(sidecar["schema_version"], 7);
    let definitions = decode_schema_package(
        &fs::read(temp.path().join("definitions.json")).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let items = decode_item_line_policy(
        &fs::read(temp.path().join("items.json")).unwrap(),
        &definitions,
        ItemLineLimits::default(),
    )
    .unwrap();
    assert_eq!(
        sidecar["item_policy"],
        serde_json::to_value(items.identity()).unwrap()
    );
    let item_source = decode_item_source_policy(
        &fs::read(temp.path().join("item-source.json")).unwrap(),
        &items,
        &definitions,
        ItemSourceLimits::default(),
    )
    .unwrap();
    assert_eq!(
        sidecar["item_source_policy"],
        serde_json::to_value(item_source.identity()).unwrap()
    );
    assert_eq!(sidecar["draft"], report["draft_digest"]);
    assert_eq!(sidecar["allocator_after"], report["allocator_after"]);
    assert_eq!(sidecar["source_sha256"], report["source"]["sha256"]);
    assert!(draft.input().gems.members.is_empty());
    assert!(draft.input().skills.members.is_empty());
    assert!(draft.input().supports.members.is_empty());
    let skill_preset = &draft.input().skill_presets.members[0];
    assert!(matches!(
        skill_preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        skill_preset.supports.completion,
        DraftListCompletion::Pending { .. }
    ));
    // Exact source hash/ordinal linkage survives the CLI boundary. A separate
    // source snapshot can inspect that content-addressed location without sharing
    // the CLI's newly allocated owned occurrence identities.
    let source = ImportedBuildInstance::from_decoded(
        decode_build(INPUT.as_bytes()).unwrap(),
        BuildLineage::from_bytes([1; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let source_gems: Vec<_> = source
        .occurrences()
        .iter()
        .filter(|row| row.name() == "Gem")
        .collect();
    assert_eq!(source_gems.len(), 1);
    let gem = source_gems[0];
    let origin = &sidecar["origins"][gem.id().ordinal() as usize];
    assert_eq!(origin["source"], serde_json::to_value(gem.id()).unwrap());
    assert!(
        source
            .source_fragment(gem.id())
            .unwrap()
            .contains("gemId=\"caller-gem\"")
    );
    let links = origin["links"].as_array().unwrap();
    assert!(
        links
            .iter()
            .all(|link| !matches!(link["kind"].as_str(), Some("gem" | "skill" | "support")))
    );
    let issues = draft
        .validate_limits(DraftLimits::default())
        .unwrap()
        .issues;
    assert!(links.iter().any(|link| {
        link["kind"] == "issue"
            && issues
                .iter()
                .any(|issue| link["value"] == serde_json::to_value(issue.id).unwrap())
    }));

    let persisted: Value =
        serde_json::from_slice(&fs::read(directory.join("report.json")).unwrap()).unwrap();
    assert_eq!(persisted, report);
    assert_eq!(
        fs::read_to_string(temp.path().join("source.xml")).unwrap(),
        INPUT
    );
}

#[test]
fn repeated_fresh_imports_have_distinct_lineages_and_preserve_existing_output() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let first = successful(run(temp.path(), &arguments("first")));
    let before = fs::read(temp.path().join("first/draft.json")).unwrap();
    let second = successful(run(temp.path(), &arguments("second")));
    assert_eq!(first["source"]["sha256"], second["source"]["sha256"]);
    assert_ne!(first["allocator_before"], second["allocator_before"]);
    let repeated = run(temp.path(), &arguments("first"));
    assert!(!repeated.status.success());
    assert_eq!(
        fs::read(temp.path().join("first/draft.json")).unwrap(),
        before
    );
    fs::create_dir(temp.path().join("existing-empty")).unwrap();
    assert!(
        !run(temp.path(), &arguments("existing-empty"))
            .status
            .success()
    );
    assert_eq!(
        fs::read_dir(temp.path().join("existing-empty"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn malformed_missing_or_oversized_policy_never_publishes_a_directory() {
    let original = serde_json::to_string(&policy()).unwrap();
    let mut unknown = json!(policy());
    unknown["unexpected"] = json!(true);
    let duplicate = format!("{{\"version\":\"caller-normalization\",{}", &original[1..]);
    let mut missing = json!(policy());
    missing
        .as_object_mut()
        .unwrap()
        .remove("generated_support_prefixes");
    for invalid in [
        unknown.to_string(),
        duplicate,
        missing.to_string(),
        "{".into(),
    ] {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path());
        fs::write(temp.path().join("policy.json"), invalid).unwrap();
        assert!(!run(temp.path(), &arguments("result")).status.success());
        assert!(!temp.path().join("result").exists());
    }
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    fs::remove_file(temp.path().join("policy.json")).unwrap();
    assert!(!run(temp.path(), &arguments("missing")).status.success());
    assert!(!temp.path().join("missing").exists());
    fs::File::create(temp.path().join("policy.json"))
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    let output = run(temp.path(), &arguments("oversized"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds"));
    assert!(!temp.path().join("oversized").exists());
}

#[test]
fn strict_roles_queries_and_wrong_bindings_fail_before_publication() {
    for file in [
        "roles.json",
        "queries.json",
        "mapping.json",
        "rewards.json",
        "items.json",
        "item-source.json",
    ] {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path());
        let path = temp.path().join(file);
        let mut input: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match file {
            "queries.json" => input[0]["unexpected"] = json!(true),
            "mapping.json" => input["policy_version"] = json!("foreign-mapping"),
            _ => input["unexpected"] = json!(true),
        }
        fs::write(path, serde_json::to_vec(&input).unwrap()).unwrap();
        assert!(!run(temp.path(), &arguments("result")).status.success());
        assert!(!temp.path().join("result").exists());
    }
}

#[test]
fn missing_or_stale_reward_policy_rejects_before_output_publication() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let path = temp.path().join("rewards.json");
    let mut input: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    input["mapping"] = json!("cd".repeat(32));
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(!run(temp.path(), &arguments("stale")).status.success());
    assert!(!temp.path().join("stale").exists());
    fs::remove_file(path).unwrap();
    assert!(!run(temp.path(), &arguments("missing")).status.success());
    assert!(!temp.path().join("missing").exists());
}

#[test]
fn item_policy_path_is_required_even_when_other_artifacts_are_present() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let mut args = arguments("result");
    let position = args.iter().position(|arg| *arg == "--items").unwrap();
    drop(args.drain(position..position + 2));
    let output = run(temp.path(), &args);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--items"));
    assert!(!temp.path().join("result").exists());
}

#[test]
fn malformed_stale_missing_or_oversized_item_policy_never_publishes_output() {
    for case in [
        "duplicate",
        "missing-field",
        "malformed",
        "version",
        "namespace",
        "definitions",
    ] {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path());
        let path = temp.path().join("items.json");
        let original = fs::read_to_string(&path).unwrap();
        let mut input: Value = serde_json::from_str(&original).unwrap();
        let invalid = match case {
            "duplicate" => format!("{{\"schema_version\":1,{}", &original[1..]),
            "malformed" => "{".to_owned(),
            _ => {
                match case {
                    "missing-field" => {
                        input.as_object_mut().unwrap().remove("whitespace");
                    }
                    "version" => {
                        input["schema_version"] = json!(OWNED_ITEM_LINE_POLICY_VERSION + 1)
                    }
                    "namespace" => input["namespace"]["version"] = json!("foreign"),
                    "definitions" => {
                        input["definitions"]["content_sha256"] = json!("cd".repeat(32))
                    }
                    _ => unreachable!(),
                }
                input.to_string()
            }
        };
        fs::write(&path, invalid).unwrap();
        let output = run(temp.path(), &arguments("result"));
        assert!(!output.status.success(), "{case}");
        assert!(!temp.path().join("result").exists(), "{case}");
    }
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let path = temp.path().join("items.json");
    fs::remove_file(&path).unwrap();
    assert!(!run(temp.path(), &arguments("missing")).status.success());
    assert!(!temp.path().join("missing").exists());
    fs::File::create(&path)
        .unwrap()
        .set_len(ItemLineLimits::default().max_wire_bytes as u64 + 1)
        .unwrap();
    let output = run(temp.path(), &arguments("oversized"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds"));
    assert!(!temp.path().join("oversized").exists());
}

#[test]
fn source_layout_policy_is_required_and_must_bind_the_exact_item_line_artifact() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let mut args = arguments("missing-argument");
    let position = args.iter().position(|arg| *arg == "--item-source").unwrap();
    drop(args.drain(position..position + 2));
    let output = run(temp.path(), &args);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--item-source"));
    assert!(!temp.path().join("missing-argument").exists());
    let path = temp.path().join("item-source.json");
    let original = fs::read(&path).unwrap();
    let mut input: Value = serde_json::from_slice(&original).unwrap();
    input["item_lines"] = json!("cd".repeat(32));
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(!run(temp.path(), &arguments("stale")).status.success());
    assert!(!temp.path().join("stale").exists());
    fs::write(&path, &original).unwrap();
    successful(run(temp.path(), &arguments("valid")));
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn source_layout_malformed_missing_and_oversized_inputs_never_publish_output() {
    for case in [
        "missing-field",
        "duplicate",
        "version",
        "namespace",
        "dialect",
    ] {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path());
        let path = temp.path().join("item-source.json");
        let original = fs::read_to_string(&path).unwrap();
        let mut input: Value = serde_json::from_str(&original).unwrap();
        let invalid = if case == "duplicate" {
            format!("{{\"schema_version\":1,{}", &original[1..])
        } else {
            match case {
                "missing-field" => {
                    input.as_object_mut().unwrap().remove("rule_layouts");
                }
                "version" => input["schema_version"] = json!(OWNED_ITEM_SOURCE_POLICY_VERSION + 1),
                "namespace" => input["namespace"]["version"] = json!("foreign"),
                "dialect" => input["dialect"] = json!("unreviewed-source"),
                _ => unreachable!(),
            }
            input.to_string()
        };
        fs::write(&path, invalid).unwrap();
        assert!(
            !run(temp.path(), &arguments("result")).status.success(),
            "{case}"
        );
        assert!(!temp.path().join("result").exists());
    }
    let temp = tempfile::tempdir().unwrap();
    save(temp.path());
    let path = temp.path().join("item-source.json");
    fs::remove_file(&path).unwrap();
    assert!(!run(temp.path(), &arguments("missing")).status.success());
    assert!(!temp.path().join("missing").exists());
    fs::File::create(&path)
        .unwrap()
        .set_len(ItemSourceLimits::default().max_wire_bytes as u64 + 1)
        .unwrap();
    let output = run(temp.path(), &arguments("oversized"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds"));
    assert!(!temp.path().join("oversized").exists());
}
