//! Reference routing laws use exact original reports and separately authored
//! synthetic malformed evidence. No evaluator, VM or copied parity result.
#[path = "support/empty_owned_items.rs"]
mod empty_owned_items;
use empty_owned_items::{empty_item_source, empty_items};

use poe_optimizer_core::{
    build_identity::*,
    data::DataIdentity,
    metrics::{ActorScope, MetricQuery},
    owned_build::{ActorKey, QueryId},
    owned_content::OwnedContentDigest,
    owned_definitions::*,
    owned_draft::*,
};
use poe_optimizer_import::owned_reference_projection::*;
use serde_json::{Value, json};

#[path = "support/owned_reference_fixture.rs"]
mod owned_reference_fixture;

fn limits() -> ProjectionLimits {
    ProjectionLimits::default()
}
fn hash(c: char) -> OwnedContentDigest {
    c.to_string().repeat(64).parse().unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("reference-routing-test", "v1").unwrap()
}
fn def<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), s).unwrap()
}
fn row_id(i: usize) -> QueryId {
    QueryId::new(format!("row-{:02}", 100 - i)).unwrap()
}
fn policy() -> ProjectionPolicyBinding {
    ProjectionPolicyBinding {
        version: key("mapping-policy"),
        game_version: ns(),
        normalization_policy: hash('a'),
        reward_policy: hash('b'),
        item_policy: hash('1'),
        item_source_policy: hash('2'),
        mapping: hash('c'),
        mapping_source: hash('d'),
        registry: hash('e'),
        skill_roles: hash('f'),
        definitions: DataIdentity {
            game: "custom".into(),
            release: "release".into(),
            schema_version: 1,
            content_sha256: "a".repeat(64),
            semantics_version: "identity-only".into(),
        },
    }
}
fn mapped(i: usize, actor: ActorScope) -> MetricRequestDraft {
    MetricRequestDraft {
        id: row_id(i),
        metric: def::<MetricDefinition>("declared-metric").into(),
        target: if actor == ActorScope::Player {
            DraftMetricTarget::Actor(DraftActorKey::Player)
        } else {
            DraftMetricTarget::Pending(PendingValue {
                id: DraftIssueId::from_instance_id(
                    InstanceId::from_parts(BuildLineage::from_bytes([0x73; 16]), 500 + i as u64)
                        .unwrap(),
                ),
                code: key("existing-minion-action-unmapped"),
                candidates: vec![],
            })
        },
    }
}
fn requested(bytes: &[u8]) -> Vec<MetricQuery> {
    let value: Value = serde_json::from_slice(bytes).unwrap();
    value["evaluation"]["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| serde_json::from_value(m["query"].clone()).unwrap())
        .collect()
}
fn plan(reference: &RecordedReference, queries: &[MetricQuery]) -> ProjectionPlan {
    ProjectionPlan::new(
        reference,
        policy(),
        queries
            .iter()
            .enumerate()
            .map(|(i, q)| ProjectionRowInput {
                id: row_id(i),
                reference: q.clone(),
                request: reference
                    .requires_evaluation(q)
                    .unwrap()
                    .then(|| mapped(i, q.actor)),
            })
            .collect(),
        limits(),
    )
    .unwrap()
}
fn selected_minion() -> Value {
    json!({"actor":"minion","skill_id":"minion-action","skill_name":"Observed action","group_index":1,"gem_index":1,"actor_skill_index":1,"minion_id":"observed-minion","part_index":1,"part_name":null,"stat_set_index":1,"stat_set_label":null,"show_average":false,"synthesized_default_attack":false})
}
fn synthetic(present: bool) -> Value {
    let mut value = json!({"schema_version":3,"source":{"format":"raw_xml","xml_sha256":hash('a')},"status":"experimental_evaluation","initialization":{"backend_and_data_ms":0.0},"evaluation":{
        "backend":{"id":"pob-poe2-mlua","implementation_version":"test","rules_revision":"rules","source_fingerprint":"source","adapter_fingerprint":"adapter"},
        "build":{"level":80,"class_name":"Synthetic","ascendancy_name":"None","tree_version":"test","main_socket_group":1,"allocated_nodes":[],"skill_groups":0},
        "context":{"requested":{"selection":null,"encounter":null},"calculation_mode":"MAIN","enemy_level":80,"config_inputs":{},"config_placeholders":{},"player_conditions":{},"enemy_conditions":{}},
        "coverage":{"schema_version":1,"active_skill_set_id":1,"groups":[],"selected_player":null,"selected_minion":if present{selected_minion()}else{Value::Null},"full_dps":{"included_group_count":0,"selected_group_included":false,"active_skills":[],"reported_contributions":[]},"unresolved_entry_count":0,"tree_connections":[]},
        "measurements":[
            {"query":{"actor":"player","id":"first"},"unit":"damage","schema_version":1,"value":{"status":"finite","value":10.0}},
            {"query":{"actor":"selected_minion","id":"first"},"unit":"damage","schema_version":1,"value":{"status":"unavailable","reason":"arbitrary diagnostic, not evidence"}},
            {"query":{"actor":"player","id":"second"},"unit":"damage","schema_version":1,"value":{"status":"unavailable","reason":"no numeric output"}},
            {"query":{"actor":"selected_minion","id":"second"},"unit":"damage","schema_version":1,"value":{"status":"unavailable","reason":"no numeric output"}}
        ],"exports":[],"warnings":[],"elapsed_ms":1.0,"diagnostic_only":true,"attachments":[]
    }});
    let evaluation = &value["evaluation"];
    let actor = json!({"skill_id":"minion-action","skill_name":"Observed action","has_hit_damage":true,"metrics":{},"non_finite_metrics":[],"non_finite_values":{}});
    let snapshot = json!({"runtime":{"upstream_revision":"rules","source_hash":"source","adapter_hash":"adapter","mlua_version":"test","lua_version":"test","lua_arch":"test","operating_system":"test","luajit_source":"test","utf8_version":"test"},"build":evaluation["build"],"coverage":evaluation["coverage"],"context":evaluation["context"],"player":actor,"minion":if present{actor}else{Value::Null},"warnings":[],"export_xml":"<PathOfBuilding2/>","elapsed_ms":1.0,"diagnostics":"","diagnostics_truncated":false});
    value["evaluation"]["attachments"] = json!([{"media_type":"application/vnd.poe-optimizer.pob-snapshot+json;version=2","content":serde_json::to_string(&snapshot).unwrap()}]);
    value
}
fn decode(value: &Value) -> Result<RecordedReference, ProjectionError> {
    RecordedReference::from_cli_report(&serde_json::to_vec(value).unwrap(), hash('a'), limits())
}
fn change_snapshot(value: &mut Value, edit: impl FnOnce(&mut Value)) {
    let mut snapshot: Value = serde_json::from_str(
        value["evaluation"]["attachments"][0]["content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    edit(&mut snapshot);
    value["evaluation"]["attachments"][0]["content"] =
        json!(serde_json::to_string(&snapshot).unwrap());
}

#[test]
fn exact_five_original_reports_keep_all_110_rows_and_only_six_reference_absences() {
    let mut total = 0;
    let mut absent = 0;
    let mut evaluate = 0;
    for fixture in owned_reference_fixture::load_references() {
        let reference = RecordedReference::from_cli_report(
            &fixture.bytes,
            fixture.source_xml_sha256.parse().unwrap(),
            limits(),
        )
        .unwrap();
        let queries = requested(&fixture.bytes);
        assert_eq!(queries.len(), 22);
        let plan = plan(&reference, &queries);
        let expected_absent = if [2, 3, 4].contains(&fixture.source_line) {
            vec![14, 16]
        } else {
            vec![]
        };
        let actual_absent: Vec<_> = plan
            .rows()
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                matches!(row.route, ProjectionRoute::KnownUnavailable { .. }).then_some(i)
            })
            .collect();
        assert_eq!(actual_absent, expected_absent);
        let core = plan.query_draft();
        assert_eq!(core.requests.members.len(), 22 - expected_absent.len());
        let mut result_ids: Vec<_> = core.requests.members.iter().map(|r| r.id.clone()).collect();
        result_ids.reverse();
        let joined = plan.join_ids(&result_ids, limits()).unwrap();
        assert_eq!(joined.rows.len(), 22);
        for (i, row) in joined.rows.iter().enumerate() {
            assert_eq!(row.id, row_id(i));
            assert_eq!(row.reference, queries[i]);
            match row.association {
                JoinedAssociation::Evaluate { result_index } => {
                    assert_eq!(result_ids[result_index], row.id)
                }
                JoinedAssociation::ReferenceKnownUnavailable { reason } => {
                    assert_eq!(reason, ReferenceAbsenceReason::NoSelectedMinion)
                }
            }
        }
        if fixture.source_line == 5 {
            let wire: Value = serde_json::from_slice(&fixture.bytes).unwrap();
            assert_eq!(
                wire["evaluation"]["measurements"][16]["value"]["status"],
                "unavailable"
            );
            assert!(reference.requires_evaluation(&queries[16]).unwrap());
            assert!(matches!(
                plan.rows()[16].route,
                ProjectionRoute::Evaluate { .. }
            ));
        }
        total += joined.rows.len();
        absent += actual_absent.len();
        evaluate += core.requests.members.len();
    }
    assert_eq!((total, evaluate, absent), (110, 104, 6));
}

#[test]
fn unavailable_metric_reason_and_unmapped_existing_actor_do_not_establish_absence() {
    let value = synthetic(true);
    let bytes = serde_json::to_vec(&value).unwrap();
    let reference = decode(&value).unwrap();
    assert_eq!(
        reference.binding().selected_minion,
        RecordedMinionState::Present
    );
    let queries = requested(&bytes);
    let plan = plan(&reference, &queries);
    let core = plan.query_draft();
    assert_eq!(core.requests.members.len(), 4);
    for i in [1, 3] {
        assert!(
            matches!(&core.requests.members[i].target,DraftMetricTarget::Pending(p) if p.code==key("existing-minion-action-unmapped"))
        );
    }
    assert!(
        plan.rows()
            .iter()
            .all(|row| matches!(row.route, ProjectionRoute::Evaluate { .. }))
    );
    assert_eq!(
        core.requests.members[0].target.to_resolved(),
        Some(poe_optimizer_core::owned_build::MetricTarget::Actor(
            ActorKey::Player
        ))
    );
}

#[test]
fn absent_optional_snapshot_requires_evaluation_instead_of_inventing_absence() {
    let mut value = synthetic(false);
    value["evaluation"]["attachments"] = json!([]);
    let reference = decode(&value).unwrap();
    assert_eq!(
        reference.binding().selected_minion,
        RecordedMinionState::Unknown
    );
    for query in requested(&serde_json::to_vec(&value).unwrap()) {
        assert!(reference.requires_evaluation(&query).unwrap());
    }
}

#[test]
fn contradictory_and_omitted_structured_absence_fields_reject() {
    for mutation in 0..8 {
        let mut value = synthetic(false);
        match mutation {
            0 => change_snapshot(&mut value, |snapshot| {
                snapshot["minion"] = json!({"skill_id":"other","skill_name":"Other"})
            }),
            1 => change_snapshot(&mut value, |snapshot| {
                snapshot.as_object_mut().unwrap().remove("minion");
            }),
            2 => {
                value["evaluation"]["coverage"]
                    .as_object_mut()
                    .unwrap()
                    .remove("selected_minion");
            }
            3 => change_snapshot(&mut value, |snapshot| {
                snapshot["coverage"]
                    .as_object_mut()
                    .unwrap()
                    .remove("selected_minion");
            }),
            4 => change_snapshot(&mut value, |snapshot| {
                snapshot["coverage"]["selected_minion"] = selected_minion()
            }),
            5 => change_snapshot(&mut value, |snapshot| {
                snapshot["context"]["enemy_level"] = json!(81)
            }),
            6 => change_snapshot(&mut value, |snapshot| {
                snapshot["runtime"]["adapter_hash"] = json!("different")
            }),
            _ => {
                value["evaluation"]["measurements"][1]["value"] =
                    json!({"status":"finite","value":1.0})
            }
        }
        assert!(decode(&value).is_err(), "mutation {mutation} accepted");
    }
    let mut value = synthetic(true);
    change_snapshot(&mut value, |snapshot| {
        snapshot["minion"]["skill_id"] = json!("different-action")
    });
    assert!(decode(&value).is_err());
}

#[test]
fn recorded_validation_covers_every_measurement_and_exact_source_backend_and_attachment() {
    let value = synthetic(false);
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(matches!(
        RecordedReference::from_cli_report(&bytes, hash('b'), limits()),
        Err(ProjectionError::SourceBinding)
    ));
    for mutation in 0..5 {
        let mut value = value.clone();
        match mutation {
            0 => value["evaluation"]["measurements"][2]["schema_version"] = json!(0),
            1 => {
                let duplicate = value["evaluation"]["measurements"][0].clone();
                value["evaluation"]["measurements"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            2 => value["evaluation"]["elapsed_ms"] = json!(-1),
            3 => {
                let duplicate = value["evaluation"]["attachments"][0].clone();
                value["evaluation"]["attachments"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            _ => {
                value["evaluation"]["attachments"][0]["media_type"] =
                    json!("application/vnd.poe-optimizer.pob-snapshot+json;version=3")
            }
        }
        assert!(decode(&value).is_err());
    }
    let mut changed = value;
    changed["evaluation"]["backend"]["implementation_version"] = json!("updated");
    let after = decode(&changed).unwrap();
    let before = RecordedReference::from_cli_report(&bytes, hash('a'), limits()).unwrap();
    assert_ne!(before.binding(), after.binding());
}

#[test]
fn explicit_ordered_correspondence_rejects_missing_mapping_wrong_id_unknown_reference_and_duplicates()
 {
    let value = synthetic(false);
    let bytes = serde_json::to_vec(&value).unwrap();
    let reference = decode(&value).unwrap();
    let queries = requested(&bytes);
    let invoke = |rows| ProjectionPlan::new(&reference, policy(), rows, limits());
    assert!(
        invoke(vec![ProjectionRowInput {
            id: row_id(0),
            reference: queries[0].clone(),
            request: None
        }])
        .is_err()
    );
    assert!(
        invoke(vec![ProjectionRowInput {
            id: row_id(1),
            reference: queries[1].clone(),
            request: Some(mapped(1, ActorScope::Player))
        }])
        .is_err()
    );
    assert!(
        invoke(vec![ProjectionRowInput {
            id: row_id(0),
            reference: queries[0].clone(),
            request: Some(mapped(9, ActorScope::Player))
        }])
        .is_err()
    );
    assert!(
        invoke(vec![ProjectionRowInput {
            id: row_id(0),
            reference: MetricQuery {
                actor: ActorScope::Player,
                id: "not-recorded".into()
            },
            request: Some(mapped(0, ActorScope::Player))
        }])
        .is_err()
    );
    let row = ProjectionRowInput {
        id: row_id(0),
        reference: queries[0].clone(),
        request: Some(mapped(0, ActorScope::Player)),
    };
    assert!(matches!(
        invoke(vec![row.clone(), row]),
        Err(ProjectionError::DuplicateRow(_))
    ));
    let plan = plan(&reference, &queries);
    assert_eq!(
        plan.query_draft()
            .requests
            .members
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>(),
        vec![row_id(0), row_id(2)]
    );
}

#[test]
fn result_id_join_rejects_missing_duplicate_extra_and_reference_only_ids() {
    let value = synthetic(false);
    let reference = decode(&value).unwrap();
    let plan = plan(&reference, &requested(&serde_json::to_vec(&value).unwrap()));
    for ids in [
        vec![row_id(0)],
        vec![row_id(0), row_id(0)],
        vec![row_id(0), row_id(2), row_id(9)],
        vec![row_id(0), row_id(1)],
        vec![row_id(0), row_id(9)],
    ] {
        assert!(plan.join_ids(&ids, limits()).is_err());
    }
    let joined = plan.join_ids(&[row_id(2), row_id(0)], limits()).unwrap();
    assert_eq!(
        joined.rows[0].association,
        JoinedAssociation::Evaluate { result_index: 1 }
    );
    assert_eq!(
        joined.rows[2].association,
        JoinedAssociation::Evaluate { result_index: 0 }
    );
    let serialized = serde_json::to_value(joined).unwrap();
    assert!(serialized.get("request").is_none());
    assert!(serialized.get("parity").is_none());
}

#[test]
fn byte_row_attachment_and_nested_draft_limits_are_explicit_and_rechecked() {
    let value = synthetic(false);
    let bytes = serde_json::to_vec(&value).unwrap();
    let exact = ProjectionLimits {
        max_report_bytes: bytes.len(),
        ..limits()
    };
    let reference = RecordedReference::from_cli_report(&bytes, hash('a'), exact).unwrap();
    assert!(
        RecordedReference::from_cli_report(
            &bytes,
            hash('a'),
            ProjectionLimits {
                max_report_bytes: bytes.len() - 1,
                ..limits()
            }
        )
        .is_err()
    );
    assert!(
        RecordedReference::from_cli_report(
            &bytes,
            hash('a'),
            ProjectionLimits {
                max_rows: 3,
                ..limits()
            }
        )
        .is_err()
    );
    let plan = plan(&reference, &requested(&bytes));
    assert!(
        plan.identity(ProjectionLimits {
            max_rows: 3,
            ..limits()
        })
        .is_err()
    );
    let plan_bytes = serde_json::to_vec(&plan).unwrap().len();
    assert!(
        plan.identity(ProjectionLimits {
            max_plan_bytes: plan_bytes,
            ..limits()
        })
        .is_ok()
    );
    assert!(
        plan.identity(ProjectionLimits {
            max_plan_bytes: plan_bytes - 1,
            ..limits()
        })
        .is_err()
    );
    let mut invalid = limits();
    invalid.draft.max_issues = 0;
    assert!(matches!(
        plan.identity(invalid),
        Err(ProjectionError::InvalidLimit(_))
    ));
}

#[test]
fn report_duplicate_fields_and_malformed_snapshot_are_not_silently_normalized() {
    let value = synthetic(false);
    let text = serde_json::to_string(&value).unwrap();
    let duplicated = text.replacen(
        "\"schema_version\":3",
        "\"schema_version\":3,\"schema_version\":3",
        1,
    );
    assert_ne!(text, duplicated);
    assert!(
        RecordedReference::from_cli_report(duplicated.as_bytes(), hash('a'), limits()).is_err()
    );
    let mut value = value;
    value["evaluation"]["attachments"][0]["content"] = json!("{malformed");
    assert!(decode(&value).is_err());
}

const NORMALIZED_XML: &str = r#"<PathOfBuilding2><Build level="80"/><Tree><Spec nodes=""/></Tree><Skills><SkillSet id="1"/></Skills><Items><ItemSet id="1"/></Items><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#;
fn normalized_fixture(
    order: &[usize],
    seed: u8,
    xml: &str,
) -> poe_optimizer_import::owned_normalize::NormalizedImport {
    use poe_optimizer_data::owned_schema::*;
    use poe_optimizer_import::{
        build_instance::{ImportedBuildInstance, InstanceImportLimits},
        decode_build,
        owned_mapping::*,
        owned_normalize::*,
        owned_reward_policy::*,
        owned_skill_catalog::*,
        owned_source::*,
        owned_value::*,
        owned_value_policy::*,
    };
    let registry = OwnedIdRegistry::empty(ns(), OwnedMappingLimits::default()).unwrap();
    let definitions = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("projection-fixture"),
            semantics_version: key("empty-schema"),
            definitions: vec![],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let pin = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "a".repeat(40),
        files: vec![SourceFilePin {
            path: "test.json".into(),
            sha256: "b".repeat(64),
        }],
    };
    let mappings = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: registry.identity().unwrap(),
            definitions: definitions.identity().clone(),
            source: pin.clone(),
            policy_version: key("mapping-policy"),
            entries: vec![],
        },
        &registry,
        &definitions,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: ns(),
            definitions: definitions.identity().clone(),
            mapping: *mappings.identity(),
            compilation: SkillCatalogReceipt {
                source: pin,
                catalog_digest: hash('c'),
                policy: SkillCatalogPolicy {
                    version: mappings.input().policy_version.clone(),
                    absent_support: AbsentSupportPolicy::Pending,
                    absent_from_tree: AbsentFromTreePolicy::Pending,
                },
                base_registry: registry.identity().unwrap(),
                staged_registry: registry.identity().unwrap(),
                gem_count: 0,
                skill_count: 0,
            },
            roles: vec![],
        },
        &mappings,
        &definitions,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    let rewards = OwnedRewardPolicy::new(
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: ns(),
            version: key("empty-rewards"),
            definitions: definitions.identity().clone(),
            mapping: *mappings.identity(),
            rules: vec![],
        },
        &mappings,
        &definitions,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    let value_recipe = |name: &str, boolean| ValueRecipeInput {
        id: key(name),
        codec: ValueCodecInput {
            namespace: ns(),
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
                name: if boolean { "enabled" } else { "level" }.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: MissingValuePolicy::Pending,
    };
    let policy = NormalizationPolicy {
        version: key("normalization-test"),
        namespace: ns(),
        character_level: value_recipe("character-level", false),
        gem_level: value_recipe("gem-level", false),
        gem_enabled: value_recipe("gem-enabled", true),
        group_enabled: value_recipe("group-enabled", true),
        manual_skill_sources: vec![SourceComponent::Missing],
        empty_item_keys: vec![SourceComponent::Text("0".into())],
        generated_support_prefixes: vec![],
        allocation_attribute: "nodes".into(),
        single_active_support_target: false,
        equipment_loadouts: vec![],
        gem_quality: GemQualityPolicy::Unconverted,
    };
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([seed; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let queries: Vec<_> = order
        .iter()
        .map(|i| ImportQueryTemplate {
            id: row_id(*i),
            metric: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::Metric,
                key: SourceComponent::Text(format!("metric-{i}")),
                version: SourceComponent::Missing,
                variant: SourceComponent::Missing,
            },
            target: ImportQueryTarget::Player,
        })
        .collect();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&definitions),
            item_source: &empty_item_source(&definitions),
            mappings: &mappings,
            registry: &registry,
            definitions: &definitions,
            roles: &roles,
            rewards: &rewards,
        },
        &policy,
        &queries,
        NormalizationLimits::default(),
    )
    .unwrap()
}
fn normalized_plan(
    normalized: &poe_optimizer_import::owned_normalize::NormalizedImport,
) -> ProjectionPlan {
    let mut value = synthetic(false);
    value["source"]["xml_sha256"] = json!(normalized.sidecar().source_sha256);
    let bytes = serde_json::to_vec(&value).unwrap();
    let reference = RecordedReference::from_cli_report(
        &bytes,
        normalized.sidecar().source_sha256.parse().unwrap(),
        limits(),
    )
    .unwrap();
    let sidecar = normalized.sidecar();
    let policy = ProjectionPolicyBinding {
        version: key("query-routing"),
        game_version: ns(),
        normalization_policy: sidecar.policy,
        reward_policy: sidecar.reward_policy,
        item_policy: sidecar.item_policy,
        item_source_policy: sidecar.item_source_policy,
        mapping: sidecar.mapping,
        mapping_source: sidecar.mapping_source,
        registry: sidecar.registry,
        definitions: sidecar.definitions.clone(),
        skill_roles: sidecar.skill_roles,
    };
    let queries = &normalized.draft().input().query_presets.members[0]
        .queries
        .requests
        .members;
    let rows = requested(&bytes)
        .into_iter()
        .enumerate()
        .map(|(i, q)| ProjectionRowInput {
            id: row_id(i),
            request: if reference.requires_evaluation(&q).unwrap() {
                Some(
                    queries
                        .iter()
                        .find(|row| row.id == row_id(i))
                        .unwrap()
                        .clone(),
                )
            } else {
                None
            },
            reference: q,
        })
        .collect();
    ProjectionPlan::new(&reference, policy, rows, limits()).unwrap()
}
#[test]
fn exact_query_draft_binding_succeeds_without_fabricating_a_full_build_selection() {
    let normalized = normalized_fixture(&[0, 2], 0x71, NORMALIZED_XML);
    let plan = normalized_plan(&normalized);
    let input = normalized.draft().input();
    let preset = input.query_presets.members[0].id;
    let binding = plan
        .validate_normalized(&normalized, preset, limits())
        .unwrap();
    assert_eq!(binding.draft, normalized.sidecar().draft);
    assert_eq!(binding.query_preset, preset);
    assert_eq!(plan.query_draft(), input.query_presets.members[0].queries);
    assert!(input.weapon_loadouts.members.is_empty());
    let unavailable_selection = EvaluationSelection {
        build: poe_optimizer_core::owned_project::VariantSelection {
            character: input.character_presets.members[0].id,
            equipment: input.equipment_presets.members[0].id,
            allocations: input.allocation_presets.members[0].id,
            skills: input.skill_presets.members[0].id,
            choices: input.choice_presets.members[0].id,
            active_weapon_loadout: WeaponLoadoutId::from_instance_id(
                input.character_presets.members[0].id.instance_id(),
            ),
        },
        scenario: input.scenario_presets.members[0].id,
        queries: preset,
    };
    assert!(matches!(
        plan.bind(&normalized, unavailable_selection, limits()),
        Err(ProjectionError::Binding("selected loadout"))
    ));
    assert!(
        plan.query_draft()
            .requests
            .members
            .iter()
            .any(|r| r.to_resolved().is_none())
    );
    assert_eq!(
        plan.join_ids(&[row_id(2), row_id(0)], limits())
            .unwrap()
            .rows
            .len(),
        4
    );
}
#[test]
fn changed_source_lineage_query_order_or_artifacts_cannot_reuse_exact_query_binding() {
    let first = normalized_fixture(&[0, 2], 0x71, NORMALIZED_XML);
    let plan = normalized_plan(&first);
    let preset = first.draft().input().query_presets.members[0].id;
    assert!(plan.validate_normalized(&first, preset, limits()).is_ok());
    for other in [
        normalized_fixture(&[2, 0], 0x71, NORMALIZED_XML),
        normalized_fixture(&[0, 2], 0x72, NORMALIZED_XML),
        normalized_fixture(&[0, 2], 0x71, &NORMALIZED_XML.replace("80", "81")),
    ] {
        let selected = other.draft().input().query_presets.members[0].id;
        assert!(
            plan.validate_normalized(&other, selected, limits())
                .is_err()
        );
    }
    assert!(
        plan.validate_normalized(
            &first,
            QueryPresetId::from_instance_id(
                first.draft().input().character_presets.members[0]
                    .id
                    .instance_id()
            ),
            limits()
        )
        .is_err()
    );
    let mut value = synthetic(false);
    value["source"]["xml_sha256"] = json!(first.sidecar().source_sha256);
    let bytes = serde_json::to_vec(&value).unwrap();
    let reference = RecordedReference::from_cli_report(
        &bytes,
        first.sidecar().source_sha256.parse().unwrap(),
        limits(),
    )
    .unwrap();
    let rows: Vec<_> = plan
        .rows()
        .iter()
        .map(|r| ProjectionRowInput {
            id: r.id.clone(),
            reference: r.reference.clone(),
            request: match &r.route {
                ProjectionRoute::Evaluate { request } => Some((**request).clone()),
                _ => None,
            },
        })
        .collect();
    for mutate in 0..6 {
        let mut wrong = plan.policy().clone();
        match mutate {
            0 => wrong.reward_policy = hash('0'),
            1 => wrong.mapping = hash('0'),
            2 => wrong.normalization_policy = hash('0'),
            3 => wrong.item_policy = hash('0'),
            4 => wrong.item_source_policy = hash('0'),
            _ => wrong.definitions.content_sha256 = "0".repeat(64),
        }
        let stale = ProjectionPlan::new(&reference, wrong, rows.clone(), limits()).unwrap();
        assert!(matches!(
            stale.validate_normalized(&first, preset, limits()),
            Err(ProjectionError::Binding("normalization artifacts"))
        ));
    }
}
