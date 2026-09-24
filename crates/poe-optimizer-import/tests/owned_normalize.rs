//! Fresh normalization contracts with injected artifacts; no evaluator or VM.
#[path = "support/empty_owned_items.rs"]
mod empty_owned_items;
use empty_owned_items::{empty_item_source, empty_items};

use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_source::*,
    owned_value::*,
    owned_value_policy::*,
};
use std::collections::BTreeSet;

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("normalization-test", "v1").unwrap()
}
fn pin() -> SourcePin {
    SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "c".repeat(40),
        files: vec![SourceFilePin {
            path: "injected/definitions.json".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn unknown<I: SchemaDefinitionId, T>(id: I) -> DefinitionEntry<I, T> {
    let target = subject(&id);
    DefinitionEntry {
        id,
        schema: SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject: target,
                facet: SchemaFacet::InputSchema,
                code: key("fixture-schema-unmapped"),
            }],
        },
    }
}
fn gem_selector(game: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(game.into()),
        variant_id: SourceComponent::Text("v".into()),
    })
}
fn metric_selector(name: &str) -> ExternalSelector {
    ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Metric,
        key: SourceComponent::Text(name.into()),
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    }
}
struct Artifacts {
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    rewards: OwnedRewardPolicy,
}
fn artifacts(mapped: bool) -> Artifacts {
    let limits = OwnedMappingLimits::default();
    let mut registry = OwnedIdRegistry::empty(ns(), limits).unwrap();
    let base = registry.identity().unwrap();
    let mut definitions = vec![];
    let mut entries = vec![];
    let mut role_rows = vec![];
    if mapped {
        for (name, role) in [
            ("active", AuthoredGemRole::SkillUse),
            ("support", AuthoredGemRole::SupportAssignment),
        ] {
            let skill = registry.allocate_definition::<SkillDefinition>().unwrap();
            definitions.push(DefinitionDescriptor::Skill(unknown(skill.clone())));
            let gem = registry.allocate_definition::<GemDefinition>().unwrap();
            definitions.push(DefinitionDescriptor::Gem(unknown(gem.clone())));
            entries.push(MappingEntry {
                source: gem_selector(name),
                outcome: MappingOutcome::Mapped {
                    target: subject(&gem),
                    basis: MappingBasis::Exact,
                },
            });
            role_rows.push(OwnedGemRoleRow {
                gem,
                primary: OwnedPrimarySkill::Known(skill),
                role: OwnedGemRole::Known(role),
                materialization: OwnedGemMaterialization::Physical,
            });
        }
        let metric = registry.allocate_definition::<MetricDefinition>().unwrap();
        definitions.push(DefinitionDescriptor::Metric(unknown(metric.clone())));
        entries.push(MappingEntry {
            source: metric_selector("known-metric"),
            outcome: MappingOutcome::Mapped {
                target: subject(&metric),
                basis: MappingBasis::Exact,
            },
        });
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("release"),
            semantics_version: key("schema-v1"),
            definitions,
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mapping = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: pin(),
            policy_version: key("mapping-v1"),
            entries,
        },
        &registry,
        &schema,
        limits,
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: ns(),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            compilation: SkillCatalogReceipt {
                source: pin(),
                catalog_digest: "b".repeat(64).parse().unwrap(),
                policy: SkillCatalogPolicy {
                    version: key("mapping-v1"),
                    absent_support: AbsentSupportPolicy::Pending,
                    absent_from_tree: AbsentFromTreePolicy::Physical,
                },
                base_registry: base,
                staged_registry: registry.identity().unwrap(),
                gem_count: role_rows.len(),
                skill_count: role_rows.len(),
            },
            roles: role_rows,
        },
        &mapping,
        &schema,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    let rewards = empty_rewards(&mapping, &schema);
    Artifacts {
        registry,
        schema,
        mapping,
        roles,
        rewards,
    }
}
fn replace_materialization(
    artifacts: &mut Artifacts,
    name: &str,
    materialization: OwnedGemMaterialization,
) {
    let entry = artifacts
        .mapping
        .input()
        .entries
        .iter()
        .find(|entry| entry.source == gem_selector(name))
        .unwrap();
    let MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
        ..
    } = &entry.outcome
    else {
        panic!("known injected gem missing")
    };
    let mut input = artifacts.roles.input().clone();
    input
        .roles
        .iter_mut()
        .find(|row| &row.gem == gem)
        .unwrap()
        .materialization = materialization;
    artifacts.roles = OwnedSkillRoleIndex::new(
        input,
        &artifacts.mapping,
        &artifacts.schema,
        SkillCatalogLimits::default(),
    )
    .unwrap();
}

fn add_active_sibling(artifacts: &mut Artifacts, materialization: OwnedGemMaterialization) {
    let skill = artifacts
        .registry
        .allocate_definition::<SkillDefinition>()
        .unwrap();
    let gem = artifacts
        .registry
        .allocate_definition::<GemDefinition>()
        .unwrap();
    let mut schema = artifacts.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Skill(unknown(skill.clone())));
    schema
        .definitions
        .push(DefinitionDescriptor::Gem(unknown(gem.clone())));
    let schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut mapping = artifacts.mapping.input().clone();
    mapping.registry = artifacts.registry.identity().unwrap();
    mapping.definitions = schema.identity().clone();
    mapping.entries.push(MappingEntry {
        source: gem_selector("sibling"),
        outcome: MappingOutcome::Mapped {
            target: subject(&gem),
            basis: MappingBasis::Exact,
        },
    });
    let mapping = OwnedMappingIndex::new(
        mapping,
        &artifacts.registry,
        &schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = artifacts.roles.input().clone();
    roles.mapping = *mapping.identity();
    roles.definitions = schema.identity().clone();
    roles.compilation.gem_count += 1;
    roles.compilation.skill_count += 1;
    roles.roles.push(OwnedGemRoleRow {
        gem,
        primary: OwnedPrimarySkill::Known(skill),
        role: OwnedGemRole::Known(AuthoredGemRole::SkillUse),
        materialization,
    });
    let roles =
        OwnedSkillRoleIndex::new(roles, &mapping, &schema, SkillCatalogLimits::default()).unwrap();
    artifacts.schema = schema;
    artifacts.mapping = mapping;
    artifacts.roles = roles;
    artifacts.rewards = empty_rewards(&artifacts.mapping, &artifacts.schema);
}

fn value_recipe(id: &str, attribute: &str, boolean: bool) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(id),
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
                name: attribute.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: MissingValuePolicy::Pending,
    }
}
fn policy() -> NormalizationPolicy {
    NormalizationPolicy {
        version: key("normalization-v1"),
        namespace: ns(),
        character_level: value_recipe("character-level", "level", false),
        gem_level: value_recipe("gem-level", "level", false),
        gem_enabled: value_recipe("gem-enabled", "enabled", true),
        group_enabled: value_recipe("group-enabled", "enabled", true),
        manual_skill_sources: vec![
            SourceComponent::Missing,
            SourceComponent::Text(String::new()),
        ],
        empty_item_keys: vec![SourceComponent::Text("0".into())],
        generated_support_prefixes: vec!["Tree:".into(), "Item:".into()],
        allocation_attribute: "nodes".into(),
        single_active_support_target: true,
        equipment_loadouts: vec![],
        skill_scopes: None,
        gem_inputs: None,
        gem_quality: GemQualityPolicy::Unconverted,
    }
}
fn source(xml: &str, seed: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([seed; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn queries() -> Vec<ImportQueryTemplate> {
    vec![
        ImportQueryTemplate {
            id: QueryId::new("z-first").unwrap(),
            metric: metric_selector("known-metric"),
            target: ImportQueryTarget::Player,
        },
        ImportQueryTemplate {
            id: QueryId::new("a-second").unwrap(),
            metric: metric_selector("unknown-metric"),
            target: ImportQueryTarget::Unresolved(key("missing-selected-minion")),
        },
    ]
}
fn run(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    queries: &[ImportQueryTemplate],
) -> NormalizedImport {
    run_with_policy(source, artifacts, queries, &policy())
}
fn run_with_policy(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    queries: &[ImportQueryTemplate],
    policy: &NormalizationPolicy,
) -> NormalizedImport {
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&artifacts.schema),
            item_source: &empty_item_source(&artifacts.schema),
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
            rewards: &artifacts.rewards,
        },
        policy,
        queries,
        NormalizationLimits::default(),
    )
    .unwrap()
}
const ACTIVE: &str = r#"<Gem gemId="active" variantId="v" level="17" enabled="true"/>"#;
const SUPPORT: &str = r#"<Gem gemId="support" variantId="v" level="1" enabled="true"/>"#;
fn group(gems: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="73"/><Tree><Spec nodes="1,2" classInternalId="class" ascendancyInternalId="asc" treeVersion="tree"/></Tree><Skills><SkillSet id="7"><Skill enabled="true">{gems}</Skill></SkillSet></Skills><Items><Item id="1">Item payload</Item><ItemSet id="7"><Slot name="Ring 1" itemId="1"/><Slot name="Ring 2" itemId="1"/></ItemSet></Items><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#
    )
}
fn ids(target: &OwnedOriginTarget) -> InstanceId {
    match target {
        OwnedOriginTarget::WeaponLoadout { id, .. } => id.instance_id(),
        OwnedOriginTarget::Item(v) | OwnedOriginTarget::ItemReference(v) => v.instance_id(),
        OwnedOriginTarget::Reward(v) => v.instance_id(),
        OwnedOriginTarget::Modifier(v) => v.instance_id(),
        OwnedOriginTarget::Equipment(v) => v.instance_id(),
        OwnedOriginTarget::Gem(v) => v.instance_id(),
        OwnedOriginTarget::Skill(v) => v.instance_id(),
        OwnedOriginTarget::Support(v) => v.instance_id(),
        OwnedOriginTarget::Allocation(v) => v.instance_id(),
        OwnedOriginTarget::ImplicitPassive { character, .. } => character.instance_id(),
        OwnedOriginTarget::CharacterPreset(v) => v.instance_id(),
        OwnedOriginTarget::EquipmentPreset(v) => v.instance_id(),
        OwnedOriginTarget::AllocationPreset(v) => v.instance_id(),
        OwnedOriginTarget::SkillPreset(v) => v.instance_id(),
        OwnedOriginTarget::ChoicePreset(v) => v.instance_id(),
        OwnedOriginTarget::ScenarioPreset(v) => v.instance_id(),
        OwnedOriginTarget::QueryPreset(v) => v.instance_id(),
        OwnedOriginTarget::Issue(v) => v.instance_id(),
    }
}
fn origin_integrity(source: &ImportedBuildInstance, result: &NormalizedImport) {
    let sidecar = result.sidecar();
    assert_eq!(sidecar.origins.len(), source.occurrences().len());
    let report = validate_draft(result.draft().input(), DraftLimits::default()).unwrap();
    let actual_issues: BTreeSet<_> = report.issues.iter().map(|i| i.id).collect();
    let mut linked_issues = BTreeSet::new();
    let mut all_ids = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    for (origin, expected) in sidecar.origins.iter().zip(source.occurrences()) {
        assert_eq!(origin.source, expected.id());
        assert!(source.occurrence(origin.source).is_ok());
        assert!(source_ids.insert(origin.source));
        match &origin.disposition {
            SourceDisposition::Contributes => assert!(!origin.links.is_empty()),
            SourceDisposition::SourceOnly(_) => assert!(origin.links.is_empty()),
        }
        for target in &origin.links {
            let id = ids(target);
            assert_eq!(id.lineage(), sidecar.allocator_before.lineage());
            assert!(id.local() > sidecar.allocator_before.last_issued());
            assert!(id.local() <= sidecar.allocator_after.last_issued());
            all_ids.insert(id);
            if let OwnedOriginTarget::Issue(issue) = target {
                assert!(actual_issues.contains(issue));
                linked_issues.insert(*issue);
            }
        }
    }
    assert_eq!(linked_issues, actual_issues);
    assert_eq!(
        all_ids.len() as u64,
        sidecar.allocator_after.last_issued() - sidecar.allocator_before.last_issued()
    );
    let source_authored: BTreeSet<_> = source
        .instances()
        .iter()
        .map(|binding| binding.instance().instance_id())
        .collect();
    assert!(all_ids.is_disjoint(&source_authored));
    assert_eq!(
        sidecar.draft,
        result
            .draft()
            .digest(DraftLimits::default().input.max_wire_bytes)
            .unwrap()
    );
}

#[test]
fn fresh_normalization_is_deterministic_above_source_watermark_and_keeps_input_immutable() {
    let source = source(&group(&format!("{ACTIVE}{SUPPORT}")), 0x31);
    let artifacts = artifacts(true);
    let before = artifacts.registry.identity().unwrap();
    let original = source.source_xml().to_string();
    let a = run(&source, &artifacts, &queries());
    let b = run(&source, &artifacts, &queries());
    assert_eq!(a.draft(), b.draft());
    assert_eq!(
        serde_json::to_vec(a.sidecar()).unwrap(),
        serde_json::to_vec(b.sidecar()).unwrap()
    );
    assert_eq!(before, artifacts.registry.identity().unwrap());
    assert_eq!(source.source_xml(), original);
    origin_integrity(&source, &a);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let reserved = InstanceAllocatorState::from_parts(
        source.lineage(),
        source.allocator_state().last_issued() + 100,
    );
    let later = normalize_fresh(
        &evidence,
        reserved,
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&artifacts.schema),
            item_source: &empty_item_source(&artifacts.schema),
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
            rewards: &artifacts.rewards,
        },
        &policy(),
        &queries(),
        NormalizationLimits::default(),
    )
    .unwrap();
    assert_eq!(later.sidecar().allocator_before, reserved);
    origin_integrity(&source, &later);
    assert_ne!(
        a.draft().input().gems.members[0].id,
        later.draft().input().gems.members[0].id
    );
}

#[test]
fn fresh_lineages_do_not_alias_identical_source_occurrences() {
    let xml = group(ACTIVE);
    let first = source(&xml, 0x31);
    let second = source(&xml, 0x32);
    let artifacts = artifacts(true);
    assert_eq!(first.occurrences()[0].id(), second.occurrences()[0].id());
    let a = run(&first, &artifacts, &[]);
    let b = run(&second, &artifacts, &[]);
    assert_ne!(
        a.draft().input().gems.members[0].id,
        b.draft().input().gems.members[0].id
    );
    assert_ne!(a.sidecar().draft, b.sidecar().draft);
}

#[test]
fn failure_and_overflow_do_not_advance_caller_allocator_or_mutate_artifacts() {
    let source = source(&group(ACTIVE), 0x31);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let artifacts = artifacts(true);
    let before = artifacts.registry.identity().unwrap();
    let allocator = *source.allocator_state();
    let invoke = |allocator, limits| {
        normalize_fresh(
            &evidence,
            allocator,
            NormalizationArtifacts {
                tree: None,
                items: &empty_items(&artifacts.schema),
                item_source: &empty_item_source(&artifacts.schema),
                mappings: &artifacts.mapping,
                registry: &artifacts.registry,
                definitions: &artifacts.schema,
                roles: &artifacts.roles,
                rewards: &artifacts.rewards,
            },
            &policy(),
            &queries(),
            limits,
        )
    };
    assert!(matches!(
        invoke(
            allocator,
            NormalizationLimits {
                max_work: 1,
                ..NormalizationLimits::default()
            }
        ),
        Err(NormalizationError::Limit("work"))
    ));
    assert!(matches!(
        invoke(
            InstanceAllocatorState::from_parts(source.lineage(), u64::MAX),
            NormalizationLimits::default()
        ),
        Err(NormalizationError::Identity(_))
    ));
    assert!(matches!(
        invoke(
            InstanceAllocatorState::from_parts(source.lineage(), 0),
            NormalizationLimits::default()
        ),
        Err(NormalizationError::AllocatorBinding)
    ));
    assert!(matches!(
        invoke(
            InstanceAllocatorState::from_parts(
                BuildLineage::from_bytes([0x33; 16]),
                allocator.last_issued()
            ),
            NormalizationLimits::default()
        ),
        Err(NormalizationError::AllocatorBinding)
    ));
    assert_eq!(*source.allocator_state(), allocator);
    assert_eq!(artifacts.registry.identity().unwrap(), before);
}

#[test]
fn tiny_empty_artifacts_preserve_known_values_and_real_pending_obligations() {
    let source = source(&group(ACTIVE), 0x31);
    let result = run(&source, &artifacts(false), &queries());
    let input = result.draft().input();
    assert_eq!(
        input.character_presets.members[0].level.to_resolved(),
        Some(73)
    );
    assert!(input.gems.members.is_empty());
    let source_gem = source
        .occurrences()
        .iter()
        .find(|row| row.name() == "Gem")
        .unwrap();
    assert_eq!(
        source
            .attribute(source_gem.id(), "level")
            .unwrap()
            .unwrap()
            .decoded(),
        "17"
    );
    assert!(input.skills.members.is_empty());
    assert!(matches!(
        input.skill_presets.members[0].skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        input.saved_variants.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(input.saved_variants.members.is_empty());
    assert!(input.weapon_loadouts.members.is_empty());
    origin_integrity(&source, &result);
}

#[test]
fn duplicate_support_definitions_keep_distinct_uses_and_one_explicit_manual_target() {
    let source = source(&group(&format!("{ACTIVE}{SUPPORT}{SUPPORT}")), 0x31);
    let result = run(&source, &artifacts(true), &[]);
    let input = result.draft().input();
    assert_eq!(input.skills.members.len(), 1);
    assert_eq!(input.supports.members.len(), 2);
    assert_eq!(input.gems.members.len(), 3);
    assert_eq!(input.gems.members[0].level.to_resolved(), Some(17));
    let target = input.skills.members[0].id;
    let a = &input.supports.members[0];
    let b = &input.supports.members[1];
    assert_ne!(a.id, b.id);
    assert_ne!(a.support.to_resolved(), b.support.to_resolved());
    for support in &input.supports.members {
        assert_eq!(
            support.target.to_resolved(),
            Some(SkillTarget::Authored(target))
        );
    }
    assert_eq!(
        input.gems.members[1].definition,
        input.gems.members[2].definition
    );
    origin_integrity(&source, &result);
}

#[test]
fn generated_active_representations_do_not_become_free_skills_and_support_targets_stay_pending() {
    let xml = group(&format!("{ACTIVE}{SUPPORT}")).replace(
        "<Skill enabled=\"true\">",
        "<Skill enabled=\"true\" source=\"Item:1\">",
    );
    let source = source(&xml, 0x31);
    let result = run(&source, &artifacts(true), &[]);
    let input = result.draft().input();
    assert!(input.skills.members.is_empty());
    assert_eq!(input.gems.members.len(), 1);
    assert_eq!(input.supports.members.len(), 1);
    assert!(matches!(
        input.supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    let active_source = source
        .occurrences()
        .iter()
        .find(|row| {
            row.name() == "Gem"
                && source
                    .attribute(row.id(), "gemId")
                    .unwrap()
                    .is_some_and(|v| v.decoded() == "active")
        })
        .unwrap()
        .id();
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|row| row.source == active_source)
        .unwrap();
    assert!(
        origin
            .links
            .iter()
            .all(|v| matches!(v, OwnedOriginTarget::Issue(_)))
    );
    origin_integrity(&source, &result);
}

#[test]
fn unknown_siblings_and_multiple_actives_prevent_false_unique_support_targets() {
    for sibling in [
        r#"<Gem gemId="unknown" variantId="v" level="1" enabled="true"/>"#,
        r#"<UnconvertedSkillRole/>"#,
        ACTIVE,
    ] {
        let source = source(&group(&format!("{ACTIVE}{SUPPORT}{sibling}")), 0x31);
        let result = run(&source, &artifacts(true), &[]);
        assert_eq!(result.draft().input().supports.members.len(), 1);
        assert!(matches!(
            result.draft().input().supports.members[0].target,
            DraftSkillTarget::Pending(_)
        ));
        origin_integrity(&source, &result);
    }
}

#[test]
fn query_rows_remain_ordered_with_known_and_unmapped_metrics_and_absent_targets() {
    let source = source(&group(ACTIVE), 0x31);
    let requested = queries();
    let result = run(&source, &artifacts(true), &requested);
    let rows = &result.draft().input().query_presets.members[0]
        .queries
        .requests
        .members;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter().map(|r| &r.id).collect::<Vec<_>>(),
        requested.iter().map(|q| &q.id).collect::<Vec<_>>()
    );
    assert!(matches!(rows[0].metric, DraftField::Known { .. }));
    assert_eq!(
        rows[0].target.to_resolved(),
        Some(MetricTarget::Actor(ActorKey::Player))
    );
    assert!(matches!(rows[1].metric, DraftField::Pending(_)));
    let DraftMetricTarget::Pending(pending) = &rows[1].target else {
        panic!("absent target substituted")
    };
    assert_eq!(pending.code, key("missing-selected-minion"));
    origin_integrity(&source, &result);
}

#[test]
fn inactive_sets_keep_independent_memberships_without_source_key_pairing() {
    let xml = format!(
        r#"<PathOfBuilding2><Build level="73"/><Tree><Spec nodes="1"/><Spec nodes="2"/></Tree><Skills><SkillSet id="7"><Skill enabled="true">{ACTIVE}</Skill></SkillSet><SkillSet id="3"><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill></SkillSet></Skills><Items><Item id="1">A</Item><Item id="2">B</Item><ItemSet id="3"><Slot name="Ring 1" itemId="1"/></ItemSet><ItemSet id="7"><Slot name="Ring 2" itemId="2"/></ItemSet></Items></PathOfBuilding2>"#
    );
    let source = source(&xml, 0x31);
    let result = run(&source, &artifacts(true), &[]);
    let input = result.draft().input();
    assert_eq!(input.skill_presets.members.len(), 2);
    assert_eq!(input.equipment_presets.members.len(), 2);
    assert_eq!(input.allocation_presets.members.len(), 2);
    assert_eq!(input.skill_presets.members[0].supports.members.len(), 0);
    assert_eq!(input.skill_presets.members[1].supports.members.len(), 1);
    for preset in &input.equipment_presets.members {
        assert_eq!(preset.equipment.members.len(), 1);
    }
    let skill_ids: BTreeSet<_> = input
        .skill_presets
        .members
        .iter()
        .flat_map(|p| &p.skills.members)
        .collect();
    assert_eq!(skill_ids.len(), 2);
    assert_ne!(
        input.skill_presets.members[0].id.instance_id(),
        input.equipment_presets.members[0].id.instance_id()
    );
    assert!(input.saved_variants.members.is_empty());
    origin_integrity(&source, &result);
}

#[test]
fn spec_sockets_keep_own_membership_with_equal_item_refs_and_ambiguous_or_missing_items() {
    let xml = r#"<PathOfBuilding2><Build level="42"/><Tree activeSpec="2"><Spec title="left" nodes="10,11"><Sockets><Socket nodeId="10" itemId="1"/><Socket nodeId="11" itemId="2"/><Socket nodeId="12" itemId="0"/></Sockets></Spec><Spec title="right" nodes="20"><Sockets><Socket nodeId="20" itemId="1"/></Sockets></Spec></Tree><Items activeItemSet="9"><Item id="1">Shared backing item</Item><Item id="2">First conflicting key</Item><Item id="2">Second conflicting key</Item><ItemSet id="9"><Slot name="left" itemId="1"/><RuneSlot slotName="unknown rune" runeName="None"/><Slot name="empty" itemId="0"/></ItemSet><ItemSet id="3"><Slot name="right" itemId="1"/></ItemSet></Items></PathOfBuilding2>"#;
    let source = source(xml, 0x32);
    let result = run(&source, &artifacts(false), &queries());
    let input = result.draft().input();
    assert_eq!(input.items.members.len(), 3);
    assert_eq!(input.equipment.members.len(), 6);
    assert_eq!(input.allocation_presets.members.len(), 2);
    assert_eq!(input.equipment_presets.members.len(), 2);
    let use_at = |tag: &str, attribute: &str, value: &str| {
        let row = source
            .occurrences()
            .iter()
            .find(|row| {
                row.name() == tag
                    && source
                        .attribute(row.id(), attribute)
                        .unwrap()
                        .is_some_and(|text| text.decoded() == value)
            })
            .unwrap();
        let origin = &result.sidecar().origins[row.id().ordinal() as usize];
        origin.links.iter().find_map(|link| match link {
            OwnedOriginTarget::Equipment(id) => Some(*id),
            _ => None,
        })
    };
    let left_socket = use_at("Socket", "nodeId", "10").unwrap();
    let ambiguous_socket = use_at("Socket", "nodeId", "11").unwrap();
    let right_socket = use_at("Socket", "nodeId", "20").unwrap();
    let left_slot = use_at("Slot", "name", "left").unwrap();
    let right_slot = use_at("Slot", "name", "right").unwrap();
    let rune = use_at("RuneSlot", "slotName", "unknown rune").unwrap();
    assert_eq!(use_at("Socket", "nodeId", "12"), None);
    assert_eq!(use_at("Slot", "name", "empty"), None);
    assert_eq!(
        input.allocation_presets.members[0].equipment.members,
        vec![left_socket, ambiguous_socket]
    );
    assert_eq!(
        input.allocation_presets.members[1].equipment.members,
        vec![right_socket]
    );
    assert_eq!(
        input.equipment_presets.members[0].equipment.members,
        vec![left_slot, rune]
    );
    assert_eq!(
        input.equipment_presets.members[1].equipment.members,
        vec![right_slot]
    );
    let tree_uses = BTreeSet::from([left_socket, ambiguous_socket, right_socket]);
    let ordinary_uses = BTreeSet::from([left_slot, rune, right_slot]);
    assert!(tree_uses.is_disjoint(&ordinary_uses));
    // Changing either selected axis would contribute only that preset's IDs.
    // There is no ItemSet-by-Spec expansion and no union of all saved jewels.
    for allocation in &input.allocation_presets.members {
        let DraftListCompletion::Pending { code, .. } = &allocation.equipment.completion else {
            panic!("source enumeration cannot certify complete equipment membership");
        };
        assert_eq!(code, &key("allocation-equipment-membership-not-converted"));
        for equipment in &input.equipment_presets.members {
            let combined: BTreeSet<_> = allocation
                .equipment
                .members
                .iter()
                .chain(&equipment.equipment.members)
                .copied()
                .collect();
            assert_eq!(
                combined.len(),
                allocation.equipment.members.len() + equipment.equipment.members.len()
            );
            let other = if allocation.id == input.allocation_presets.members[0].id {
                right_socket
            } else {
                left_socket
            };
            assert!(!combined.contains(&other));
        }
    }
    let shared = [left_socket, right_socket, left_slot, right_slot];
    let backing: BTreeSet<_> = shared
        .iter()
        .map(|id| {
            input
                .equipment
                .members
                .iter()
                .find(|row| row.id == *id)
                .unwrap()
                .item
                .to_resolved()
                .unwrap()
        })
        .collect();
    assert_eq!(backing.len(), 1);
    assert_eq!(shared.into_iter().collect::<BTreeSet<_>>().len(), 4);
    for id in [ambiguous_socket, rune] {
        let row = input
            .equipment
            .members
            .iter()
            .find(|row| row.id == id)
            .unwrap();
        assert!(matches!(row.item, DraftField::Pending(_)));
    }
    for id in tree_uses.iter().copied().chain([rune]) {
        let row = input
            .equipment
            .members
            .iter()
            .find(|row| row.id == id)
            .unwrap();
        assert!(matches!(
            row.destination,
            DraftEquipmentDestination::Pending(_)
        ));
        assert!(matches!(row.scope, DraftField::Pending(_)));
    }
    assert!(
        input
            .allocations
            .members
            .iter()
            .all(|row| matches!(row.pool, DraftField::Pending(_)))
    );
    assert_eq!(
        input.query_presets.members[0]
            .queries
            .requests
            .members
            .len(),
        2
    );
    origin_integrity(&source, &result);
    assert_eq!(source.source_xml(), xml);
}

#[test]
fn undecodable_item_key_keeps_a_spec_receiving_use_pending_without_lower_tier_join() {
    let xml = r#"<PathOfBuilding2><Tree><Spec nodes="10"><Sockets><Socket nodeId="10" itemId="1"/></Sockets></Spec><Spec nodes=""/></Tree><Items><Item id="1">Known</Item><Item id="&#49;">Lexically unavailable equal key</Item><ItemSet id="1"/></Items></PathOfBuilding2>"#;
    let source = source(xml, 0x33);
    let result = run(&source, &artifacts(false), &[]);
    let input = result.draft().input();
    assert_eq!(input.items.members.len(), 2);
    assert_eq!(input.equipment.members.len(), 1);
    let row = &input.equipment.members[0];
    assert!(matches!(row.item, DraftField::Pending(_)));
    assert!(matches!(
        row.destination,
        DraftEquipmentDestination::Pending(_)
    ));
    assert_eq!(
        input.allocation_presets.members[0].equipment.members,
        vec![row.id]
    );
    assert!(
        input.allocation_presets.members[1]
            .equipment
            .members
            .is_empty()
    );
    assert!(
        input.equipment_presets.members[0]
            .equipment
            .members
            .is_empty()
    );
    assert!(
        input
            .allocation_presets
            .members
            .iter()
            .all(|preset| matches!(
                preset.equipment.completion,
                DraftListCompletion::Pending { .. }
            ))
    );
    origin_integrity(&source, &result);
}

const ORIGINALS: [&str; 5] = [
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-03.xml"),
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-04.xml"),
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
];
#[test]
fn all_five_originals_normalize_with_empty_artifacts_and_account_for_every_gem_shaped_source() {
    let artifacts = artifacts(false);
    let mut total_gems = 0;
    for (i, xml) in ORIGINALS.iter().enumerate() {
        let source = source(xml, 0x40 + i as u8);
        let result = run(&source, &artifacts, &queries());
        let input = result.draft().input();
        origin_integrity(&source, &result);
        let gems = source
            .instances()
            .iter()
            .filter(|row| matches!(row.instance(), AuthoredInstanceId::SkillEntry(_)))
            .count();
        assert_eq!(gems, [62, 174, 62, 62, 181][i]);
        total_gems += gems;
        assert_eq!(input.items.members.len(), [16, 34, 17, 21, 28][i]);
        assert_eq!(input.skill_presets.members.len(), [1, 6, 1, 1, 6][i]);
        assert_eq!(input.equipment_presets.members.len(), [1, 6, 1, 1, 6][i]);
        assert_eq!(input.allocation_presets.members.len(), [1, 6, 1, 1, 7][i]);
        assert!(
            input
                .items
                .members
                .iter()
                .all(|row| matches!(row.template, DraftField::Pending(_)))
        );
        assert!(input.gems.members.is_empty());
        assert!(input.skills.members.is_empty());
        assert!(input.supports.members.is_empty());
    }
    assert_eq!(total_gems, 541);
}

#[test]
fn sniper_item26_is_one_record_with_eight_saved_uses_and_two_in_selected_item_set() {
    let source = source(ORIGINALS[4], 0x45);
    let result = run(&source, &artifacts(false), &[]);
    let input = result.draft().input();
    let source_item = source
        .occurrences()
        .iter()
        .find(|row| {
            row.name() == "Item"
                && source
                    .attribute(row.id(), "id")
                    .unwrap()
                    .is_some_and(|v| v.decoded() == "26")
        })
        .unwrap()
        .id();
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|o| o.source == source_item)
        .unwrap();
    let items: Vec<_> = origin
        .links
        .iter()
        .filter_map(|t| {
            if let OwnedOriginTarget::Item(id) = t {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(items.len(), 1);
    let item = items[0];
    let uses: BTreeSet<_> = input
        .equipment
        .members
        .iter()
        .filter(|r| r.item.to_resolved() == Some(item))
        .map(|r| r.id)
        .collect();
    assert_eq!(uses.len(), 8);
    let counts: Vec<_> = input
        .equipment_presets
        .members
        .iter()
        .map(|p| {
            p.equipment
                .members
                .iter()
                .filter(|id| uses.contains(id))
                .count()
        })
        .filter(|n| *n > 0)
        .collect();
    assert_eq!(counts, vec![2; 4]);
    let source_set = source
        .occurrences()
        .iter()
        .find(|row| {
            row.name() == "ItemSet"
                && source
                    .attribute(row.id(), "id")
                    .unwrap()
                    .is_some_and(|v| v.decoded() == "2")
        })
        .unwrap()
        .id();
    let preset = result
        .sidecar()
        .origins
        .iter()
        .find(|o| o.source == source_set)
        .unwrap()
        .links
        .iter()
        .find_map(|t| {
            if let OwnedOriginTarget::EquipmentPreset(id) = t {
                Some(*id)
            } else {
                None
            }
        })
        .unwrap();
    let selected = input
        .equipment_presets
        .members
        .iter()
        .find(|p| p.id == preset)
        .unwrap();
    assert_eq!(
        selected
            .equipment
            .members
            .iter()
            .filter(|id| uses.contains(id))
            .count(),
        2
    );
    origin_integrity(&source, &result);
}

#[test]
fn ambiguous_gem_mapping_retains_bound_candidates_without_inventing_a_physical_occurrence() {
    let mut artifacts = artifacts(true);
    let new_gem = artifacts
        .registry
        .allocate_definition::<GemDefinition>()
        .unwrap();
    let mut schema_input = artifacts.schema.input().clone();
    schema_input
        .definitions
        .push(DefinitionDescriptor::Gem(unknown(new_gem.clone())));
    let schema =
        OwnedDefinitionSchemaPackage::new(schema_input, OwnedSchemaLimits::default()).unwrap();
    let mut mapping_input = artifacts.mapping.input().clone();
    mapping_input.registry = artifacts.registry.identity().unwrap();
    mapping_input.definitions = schema.identity().clone();
    let active = mapping_input
        .entries
        .iter_mut()
        .find(|entry| entry.source == gem_selector("active"))
        .unwrap();
    let MappingOutcome::Mapped { target, .. } = &active.outcome else {
        panic!("fixture active mapping missing")
    };
    active.outcome = MappingOutcome::Ambiguous {
        candidates: vec![target.clone(), subject(&new_gem)],
        issue: key("two-gem-identities"),
    };
    let mapping = OwnedMappingIndex::new(
        mapping_input,
        &artifacts.registry,
        &schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut role_input = artifacts.roles.input().clone();
    role_input.mapping = *mapping.identity();
    role_input.definitions = schema.identity().clone();
    role_input.compilation.gem_count += 1;
    role_input.roles.push(OwnedGemRoleRow {
        gem: new_gem,
        primary: OwnedPrimarySkill::Unmapped {
            issue: key("primary-unknown"),
        },
        role: OwnedGemRole::Unmapped {
            issue: key("role-unknown"),
        },
        materialization: OwnedGemMaterialization::Unmapped {
            issue: key("physical-origin-unknown"),
        },
    });
    let roles =
        OwnedSkillRoleIndex::new(role_input, &mapping, &schema, SkillCatalogLimits::default())
            .unwrap();
    artifacts.schema = schema;
    artifacts.mapping = mapping;
    artifacts.roles = roles;
    artifacts.rewards = empty_rewards(&artifacts.mapping, &artifacts.schema);
    let source = source(&group(&format!("{ACTIVE}{SUPPORT}")), 0x31);
    let result = run(&source, &artifacts, &[]);
    let input = result.draft().input();
    let entry = artifacts
        .mapping
        .input()
        .entries
        .iter()
        .find(|entry| entry.source == gem_selector("active"))
        .unwrap();
    let MappingOutcome::Ambiguous { candidates, issue } = &entry.outcome else {
        panic!("ambiguous mapping was replaced")
    };
    assert_eq!(issue, &key("two-gem-identities"));
    assert_eq!(candidates.len(), 2);
    assert_eq!(result.sidecar().mapping, *artifacts.mapping.identity());
    assert!(input.skills.members.is_empty());
    assert_eq!(input.gems.members.len(), 1);
    assert_eq!(input.supports.members.len(), 1);
    assert!(matches!(
        input.supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    origin_integrity(&source, &result);
}

#[test]
fn duplicate_external_item_keys_remain_two_records_and_an_unresolved_receiving_reference() {
    let xml = r#"<PathOfBuilding2><Items><Item id="1">First</Item><Item id="1">Second</Item><ItemSet id="1"><Slot name="Ring 1" itemId="1"/></ItemSet></Items></PathOfBuilding2>"#;
    let source = source(xml, 0x31);
    let result = run(&source, &artifacts(false), &[]);
    let input = result.draft().input();
    assert_eq!(input.items.members.len(), 2);
    assert_ne!(input.items.members[0].id, input.items.members[1].id);
    assert_eq!(input.equipment.members.len(), 1);
    assert!(matches!(
        input.equipment.members[0].item,
        DraftField::Pending(_)
    ));
    origin_integrity(&source, &result);
}

#[test]
fn duplicate_requested_query_ids_reject_instead_of_dropping_rows() {
    let source = source(&group(ACTIVE), 0x31);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let artifacts = artifacts(true);
    let mut requested = queries();
    requested[1].id = requested[0].id.clone();
    assert!(matches!(
        normalize_fresh(
            &evidence,
            *source.allocator_state(),
            NormalizationArtifacts {
                tree: None,
                items: &empty_items(&artifacts.schema),
                item_source: &empty_item_source(&artifacts.schema),
                mappings: &artifacts.mapping,
                registry: &artifacts.registry,
                definitions: &artifacts.schema,
                roles: &artifacts.roles,
                rewards: &artifacts.rewards,
            },
            &policy(),
            &requested,
            NormalizationLimits::default()
        ),
        Err(NormalizationError::Policy("query list"))
    ));
}

#[test]
fn name_only_unknown_rows_do_not_create_material_gems_or_false_unique_support_targets() {
    let xml = group(&format!(
        "{ACTIVE}{SUPPORT}<Gem nameSpec=\"Unconverted generated ability\" level=\"1\" enabled=\"true\"/>"
    ));
    let source = source(&xml, 0x31);
    let result = run(&source, &artifacts(true), &[]);
    let input = result.draft().input();
    assert_eq!(input.gems.members.len(), 2);
    assert_eq!(input.skills.members.len(), 1);
    assert_eq!(input.supports.members.len(), 1);
    assert!(matches!(
        input.supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    let row = source
        .occurrences()
        .iter()
        .find(|row| {
            row.name() == "Gem" && source.attribute(row.id(), "nameSpec").unwrap().is_some()
        })
        .unwrap();
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|o| o.source == row.id())
        .unwrap();
    assert!(!origin.links.is_empty());
    assert!(
        origin
            .links
            .iter()
            .all(|v| matches!(v, OwnedOriginTarget::Issue(_)))
    );
    origin_integrity(&source, &result);
}

#[test]
fn missing_item_reference_stays_pending_while_explicit_empty_slot_is_source_only() {
    let xml = r#"<PathOfBuilding2><Items><ItemSet id="1"><Slot name="Ring 1"/><Slot name="Ring 2" itemId="0"/></ItemSet></Items></PathOfBuilding2>"#;
    let source = source(xml, 0x31);
    let result = run(&source, &artifacts(false), &[]);
    let input = result.draft().input();
    assert_eq!(input.equipment.members.len(), 1);
    assert!(matches!(
        input.equipment.members[0].item,
        DraftField::Pending(_)
    ));
    let empty = source
        .occurrences()
        .iter()
        .find(|row| {
            row.name() == "Slot"
                && source
                    .attribute(row.id(), "itemId")
                    .unwrap()
                    .is_some_and(|v| v.decoded() == "0")
        })
        .unwrap()
        .id();
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|o| o.source == empty)
        .unwrap();
    assert!(matches!(
        origin.disposition,
        SourceDisposition::SourceOnly(_)
    ));
    assert!(origin.links.is_empty());
    origin_integrity(&source, &result);
}

#[test]
fn manual_source_syntax_needs_physical_materialization_proof_for_active_and_support_rows() {
    for materialization in [
        OwnedGemMaterialization::ProviderOnly,
        OwnedGemMaterialization::Unmapped {
            issue: key("physical-origin-unmapped"),
        },
    ] {
        for name in ["active", "support"] {
            let mut artifacts = artifacts(true);
            replace_materialization(&mut artifacts, name, materialization.clone());
            let source = source(&group(&format!("{ACTIVE}{SUPPORT}")), 0x31);
            let result = run(&source, &artifacts, &[]);
            let input = result.draft().input();
            assert_eq!(input.gems.members.len(), 1);
            if name == "active" {
                assert!(input.skills.members.is_empty());
                assert_eq!(input.supports.members.len(), 1);
                assert!(matches!(
                    input.supports.members[0].target,
                    DraftSkillTarget::Pending(_)
                ));
            } else {
                assert_eq!(input.skills.members.len(), 1);
                assert!(input.supports.members.is_empty());
            }
            let skipped = source
                .occurrences()
                .iter()
                .find(|row| {
                    row.name() == "Gem"
                        && source
                            .attribute(row.id(), "gemId")
                            .unwrap()
                            .is_some_and(|v| v.decoded() == name)
                })
                .unwrap()
                .id();
            let origin = result
                .sidecar()
                .origins
                .iter()
                .find(|row| row.source == skipped)
                .unwrap();
            assert!(!origin.links.is_empty());
            assert!(
                origin
                    .links
                    .iter()
                    .all(|v| matches!(v, OwnedOriginTarget::Issue(_)))
            );
            origin_integrity(&source, &result);
        }
    }
}

#[test]
fn provider_only_or_unmapped_sibling_cannot_make_a_support_target_falsely_unique() {
    for materialization in [
        OwnedGemMaterialization::ProviderOnly,
        OwnedGemMaterialization::Unmapped {
            issue: key("physical-origin-unmapped"),
        },
    ] {
        let mut artifacts = artifacts(true);
        add_active_sibling(&mut artifacts, materialization);
        let source = source(
            &group(&format!(
                "{ACTIVE}{SUPPORT}<Gem gemId=\"sibling\" variantId=\"v\" level=\"1\" enabled=\"true\"/>"
            )),
            0x31,
        );
        let result = run(&source, &artifacts, &[]);
        let input = result.draft().input();
        assert_eq!(input.gems.members.len(), 2);
        assert_eq!(input.skills.members.len(), 1);
        assert_eq!(input.supports.members.len(), 1);
        assert!(matches!(
            input.supports.members[0].target,
            DraftSkillTarget::Pending(_)
        ));
        origin_integrity(&source, &result);
    }
}

fn empty_rewards(
    mapping: &OwnedMappingIndex,
    schema: &OwnedDefinitionSchemaPackage,
) -> OwnedRewardPolicy {
    OwnedRewardPolicy::new(
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: schema.input().namespace.clone(),
            version: key("fixture-empty-rewards"),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            rules: vec![],
        },
        mapping,
        schema,
        RewardPolicyLimits::default(),
    )
    .unwrap()
}

// Synthetic content names deliberately differ from every game fixture. The same
// normalizer must accept injected reward data without knowing any quest identity.
fn with_reward_policy() -> Artifacts {
    let mut a = artifacts(false);
    let reward = a
        .registry
        .allocate_definition::<RewardDefinition>()
        .unwrap();
    let empty = DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    };
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Reward(DefinitionEntry {
            id: reward.clone(),
            schema: SchemaState::Known(RewardSchema {
                declarations: empty,
            }),
        }));
    a.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Reward {
        key: SourceComponent::Text("caller-award".into()),
    });
    let mut mapping = a.mapping.input().clone();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    mapping.entries.push(MappingEntry {
        source: selector.clone(),
        outcome: MappingOutcome::Mapped {
            target: subject(&reward),
            basis: MappingBasis::Exact,
        },
    });
    a.mapping = OwnedMappingIndex::new(
        mapping,
        &a.registry,
        &a.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, SkillCatalogLimits::default())
        .unwrap();
    let mut recipe = value_recipe("caller-reward-rule", "caller-toggle", true);
    recipe.tiers[0].selectors[0].lane = ValueLane::InputBoolean;
    recipe.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Boolean(true),
    };
    a.rewards = OwnedRewardPolicy::new(
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: ns(),
            version: key("caller-policy"),
            definitions: a.schema.identity().clone(),
            mapping: *a.mapping.identity(),
            rules: vec![RewardRuleInput {
                recipe,
                outcomes: vec![
                    RewardOutcomeCase {
                        when: RewardValue::Boolean(false),
                        outcome: RewardTemplate::None,
                    },
                    RewardOutcomeCase {
                        when: RewardValue::Boolean(true),
                        outcome: RewardTemplate::Reward {
                            selector,
                            parameters: vec![],
                        },
                    },
                ],
            }],
        },
        &a.mapping,
        &a.schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    a
}

#[test]
fn rewards_are_injected_independent_config_contributions_and_never_close_catalogs() {
    let a = with_reward_policy();
    let source = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="caller-toggle" boolean="true"/></ConfigSet><ConfigSet id="2"><Input name="caller-toggle" boolean="false"/></ConfigSet><ConfigSet id="3"/></Config></PathOfBuilding2>"#,
        99,
    );
    let normalized = run(&source, &a, &[]);
    let d = normalized.draft().input();
    assert_eq!(d.rewards.members.len(), 2);
    assert_eq!(
        d.choice_presets
            .members
            .iter()
            .map(|p| p.rewards.members.len())
            .collect::<Vec<_>>(),
        vec![1, 0, 1]
    );
    assert_ne!(d.rewards.members[0].id, d.rewards.members[1].id);
    assert_eq!(
        d.rewards.members[0].definition,
        d.rewards.members[1].definition
    );
    assert!(matches!(
        d.rewards.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(
        d.choice_presets
            .members
            .iter()
            .all(|p| matches!(p.rewards.completion, DraftListCompletion::Pending { .. }))
    );
    assert_eq!(normalized.sidecar().reward_policy, *a.rewards.identity());
    let validation = normalized
        .draft()
        .validate_limits(DraftLimits::default())
        .unwrap();
    let issues: BTreeSet<_> = validation.issues.iter().map(|i| i.id).collect();
    for origin in &normalized.sidecar().origins {
        for link in &origin.links {
            if let OwnedOriginTarget::Issue(id) = link {
                assert!(issues.contains(id));
            }
        }
    }
}

#[test]
fn malformed_or_unknown_config_presence_never_activates_reward_defaults() {
    let a = with_reward_policy();
    for body in [
        r#"<Input name="caller-toggle" boolean="invalid"/>"#,
        r#"<Input name="caller-toggle" string="true"/>"#,
        r#"<Input name="caller-toggle"/>"#,
        r#"<Input name="caller-toggle" boolean="true" number="1"/>"#,
        r#"<Input name="caller-toggle" boolean="true"><Future/></Input>"#,
        r#"<Input boolean="true"/>"#,
        r#"<Input name="" boolean="true"/>"#,
        r#"<Placeholder name="caller-toggle" boolean="true"/>"#,
        r#"<Future/>"#,
        r#"<x:Input xmlns:x="future" name="caller-toggle" boolean="true"/>"#,
    ] {
        let source = source(
            &format!(
                r#"<PathOfBuilding2><Config><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
            ),
            98,
        );
        let normalized = run(&source, &a, &[]);
        assert!(
            normalized.draft().input().rewards.members.is_empty(),
            "{body}"
        );
        assert!(matches!(
            normalized.draft().input().choice_presets.members[0]
                .rewards
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    // A separately named placeholder cannot overwrite this recipe's input.
    let source = source(
        r#"<PathOfBuilding2><Config><Input name="caller-toggle" boolean="true"/><Placeholder name="unrelated" number="1"/></Config></PathOfBuilding2>"#,
        97,
    );
    assert_eq!(
        run(&source, &a, &[]).draft().input().rewards.members.len(),
        1
    );
}

#[test]
fn reward_binding_rejects_stale_policy_before_any_normalization_is_returned() {
    let mut a = with_reward_policy();
    a.rewards = artifacts(false).rewards;
    let source = source("<PathOfBuilding2/>", 96);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    assert!(matches!(
        normalize_fresh(
            &evidence,
            *source.allocator_state(),
            NormalizationArtifacts {
                tree: None,
                items: &empty_items(&a.schema),
                item_source: &empty_item_source(&a.schema),
                mappings: &a.mapping,
                registry: &a.registry,
                definitions: &a.schema,
                roles: &a.roles,
                rewards: &a.rewards,
            },
            &policy(),
            &[],
            NormalizationLimits::default()
        ),
        Err(NormalizationError::Reward(RewardPolicyError::Binding))
    ));
}

#[test]
fn reward_recipes_can_select_same_name_across_admitted_typed_lanes() {
    let mut a = with_reward_policy();
    let mut input = a.rewards.input().clone();
    let boolean = input.rules[0].recipe.tiers[0].clone();
    let mut string = boolean.clone();
    string.selectors[0].lane = ValueLane::InputString;
    input.rules[0].recipe.tiers = vec![string, boolean];
    a.rewards = OwnedRewardPolicy::new(input, &a.mapping, &a.schema, RewardPolicyLimits::default())
        .unwrap();
    for body in [
        r#"<Input name="caller-toggle" string="true"/>"#,
        r#"<Input name="caller-toggle" boolean="true"/>"#,
        r#"<Input name="caller-toggle" string="true"/><Input name="caller-toggle" boolean="false"/>"#,
    ] {
        let source = source(
            &format!(
                r#"<PathOfBuilding2><Config><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
            ),
            94,
        );
        assert_eq!(
            run(&source, &a, &[]).draft().input().rewards.members.len(),
            1,
            "{body}"
        );
    }
}

fn loadout_artifacts() -> (Artifacts, NormalizationPolicy) {
    let mut a = artifacts(false);
    let mut definitions = a.schema.input().clone();
    let mut mapping = a.mapping.input().clone();
    let mut policy = policy();
    for (source, selected) in [
        ("blade", Some("first")),
        ("blade-alt", Some("second")),
        ("coat", None),
    ] {
        let destination = a
            .registry
            .allocate_definition::<EquipmentSlotDefinition>()
            .unwrap();
        definitions
            .definitions
            .push(DefinitionDescriptor::EquipmentSlot(DefinitionEntry {
                id: destination.clone(),
                schema: SchemaState::Known(EquipmentSlotSchema {
                    scope: if selected.is_some() {
                        ScopePolicy::Selected
                    } else {
                        ScopePolicy::Shared
                    },
                }),
            }));
        mapping.entries.push(MappingEntry {
            source: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::EquipmentSlot,
                key: SourceComponent::Text(source.into()),
                version: SourceComponent::Missing,
                variant: SourceComponent::Missing,
            },
            outcome: MappingOutcome::Mapped {
                target: subject(&destination),
                basis: MappingBasis::Exact,
            },
        });
        policy.equipment_loadouts.push(EquipmentLoadoutRule {
            source_slot: SourceComponent::Text(source.into()),
            destination,
            scope: match selected {
                Some(name) => ImportEquipmentScope::Selected {
                    loadouts: vec![key(name)],
                },
                None => ImportEquipmentScope::Shared,
            },
        });
    }
    a.schema =
        OwnedDefinitionSchemaPackage::new(definitions, OwnedSchemaLimits::default()).unwrap();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping = OwnedMappingIndex::new(
        mapping,
        &a.registry,
        &a.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, SkillCatalogLimits::default())
        .unwrap();
    a.rewards = empty_rewards(&a.mapping, &a.schema);
    (a, policy)
}
fn normalize_with_loadouts(
    xml: &str,
    a: &Artifacts,
    policy: &NormalizationPolicy,
) -> Result<NormalizedImport, NormalizationError> {
    let source = source(xml, 93);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&a.schema),
            item_source: &empty_item_source(&a.schema),
            mappings: &a.mapping,
            registry: &a.registry,
            definitions: &a.schema,
            roles: &a.roles,
            rewards: &a.rewards,
        },
        policy,
        &[],
        NormalizationLimits::default(),
    )
}
fn proven_loadouts(
    result: &NormalizedImport,
) -> std::collections::BTreeMap<OwnedDefinitionKey, WeaponLoadoutId> {
    result
        .sidecar()
        .origins
        .iter()
        .flat_map(|row| &row.links)
        .filter_map(|link| match link {
            OwnedOriginTarget::WeaponLoadout { key, id } => Some((key.clone(), *id)),
            _ => None,
        })
        .collect()
}
#[test]
fn observed_exact_slots_share_loadout_identity_across_independent_item_sets() {
    let (a, policy) = loadout_artifacts();
    let xml = r#"<PathOfBuilding2><Items useSecondWeaponSet="nil"><Item id="1">Unconverted</Item><ItemSet id="left" useSecondWeaponSet="false"><Slot name="blade" itemId="1"/><Slot name="blade-alt" itemId="0"/><Slot name="coat" itemId="1"/></ItemSet><ItemSet id="right" useSecondWeaponSet="true"><Slot name="blade" itemId="0"/><Slot name="blade-alt" itemId="1"/><Slot name="coat" itemId="1"/></ItemSet></Items></PathOfBuilding2>"#;
    let result = normalize_with_loadouts(xml, &a, &policy).unwrap();
    let known = proven_loadouts(&result);
    assert_eq!(known.len(), 2);
    assert_ne!(known[&key("first")], known[&key("second")]);
    let draft = result.draft().input();
    assert_eq!(draft.weapon_loadouts.members.len(), 2);
    assert!(matches!(
        draft.weapon_loadouts.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert_eq!(draft.equipment_presets.members.len(), 2);
    assert_eq!(draft.equipment.members.len(), 4);
    let scopes: Vec<_> = draft
        .equipment
        .members
        .iter()
        .map(|r| r.scope.to_resolved().unwrap())
        .collect();
    assert_eq!(
        scopes,
        vec![
            LoadoutScope::Selected {
                loadouts: vec![known[&key("first")]]
            },
            LoadoutScope::Shared,
            LoadoutScope::Selected {
                loadouts: vec![known[&key("second")]]
            },
            LoadoutScope::Shared,
        ]
    );
    assert!(
        draft
            .equipment_presets
            .members
            .iter()
            .all(|p| p.equipment.members.len() == 2)
    );
    assert!(
        draft.saved_variants.members.is_empty(),
        "nil/boolean source choices do not silently select a preset"
    );
    assert_eq!(result.sidecar().schema_version, 12);
    let mut no_rules = policy.clone();
    no_rules.equipment_loadouts.clear();
    let empty = normalize_with_loadouts(xml, &a, &no_rules).unwrap();
    assert!(empty.draft().input().weapon_loadouts.members.is_empty());
    assert_ne!(result.sidecar().policy, empty.sidecar().policy);
}
#[test]
fn duplicate_unknown_and_lexically_unavailable_slot_names_do_not_prove_scopes() {
    let (a, policy) = loadout_artifacts();
    for slots in [
        r#"<Slot name="blade" itemId="1"/><Slot name="blade" itemId="1"/>"#,
        r#"<Slot name="blade" itemId="1"/><Slot name="bl&#97;de" itemId="1"/>"#,
        r#"<Slot name="unreviewed" itemId="1"/><Slot itemId="1"/>"#,
        r#"<Slot xmlns="urn:other" name="blade" itemId="1"/>"#,
    ] {
        let xml = format!(
            "<PathOfBuilding2><Items><Item id=\"1\">Unconverted</Item><ItemSet id=\"1\">{slots}</ItemSet></Items></PathOfBuilding2>"
        );
        let result = normalize_with_loadouts(&xml, &a, &policy).unwrap();
        assert!(
            result.draft().input().weapon_loadouts.members.is_empty(),
            "{slots}"
        );
        assert!(
            result
                .draft()
                .input()
                .equipment
                .members
                .iter()
                .all(|row| matches!(row.scope, DraftField::Pending(_))),
            "{slots}"
        );
    }
    // An unattached Slot cannot establish an ItemSet-owned receiving scope.
    let result = normalize_with_loadouts(r#"<PathOfBuilding2><Items><Item id="1">Unconverted</Item><Slot name="blade" itemId="1"/></Items></PathOfBuilding2>"#, &a, &policy).unwrap();
    assert!(result.draft().input().weapon_loadouts.members.is_empty());
}
#[test]
fn loadout_policy_rejects_known_mapping_and_scope_contradictions_and_stale_wire() {
    let (a, baseline) = loadout_artifacts();
    let xml = r#"<PathOfBuilding2><Items><ItemSet id="1"><Slot name="blade" itemId="0"/></ItemSet></Items></PathOfBuilding2>"#;
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].destination = baseline.equipment_loadouts[1].destination.clone();
    assert!(matches!(
        normalize_with_loadouts(xml, &a, &changed),
        Err(NormalizationError::Policy(
            "equipment loadout mapping contradiction"
        ))
    ));
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].scope = ImportEquipmentScope::Shared;
    assert!(matches!(
        normalize_with_loadouts(xml, &a, &changed),
        Err(NormalizationError::Policy(
            "equipment loadout schema contradiction"
        ))
    ));
    let mut changed = baseline.clone();
    changed
        .equipment_loadouts
        .push(changed.equipment_loadouts[0].clone());
    assert!(normalize_with_loadouts(xml, &a, &changed).is_err());
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].scope = ImportEquipmentScope::Selected { loadouts: vec![] };
    assert!(normalize_with_loadouts(xml, &a, &changed).is_err());
    changed.equipment_loadouts[0].scope = ImportEquipmentScope::Selected {
        loadouts: vec![key("same"), key("same")],
    };
    assert!(normalize_with_loadouts(xml, &a, &changed).is_err());
    let mut stale = serde_json::to_value(&baseline).unwrap();
    stale.as_object_mut().unwrap().remove("equipment_loadouts");
    assert!(serde_json::from_value::<NormalizationPolicy>(stale).is_err());
    let mut unknown = serde_json::to_value(&baseline).unwrap();
    unknown["equipment_loadouts"][0]["extra"] = serde_json::json!(true);
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
    // A rule absent from the mapping cannot allocate even an observed key.
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].source_slot = SourceComponent::Text("unmapped".into());
    let result = normalize_with_loadouts(&xml.replace("blade", "unmapped"), &a, &changed).unwrap();
    assert!(result.draft().input().weapon_loadouts.members.is_empty());
}

#[test]
fn equipment_loadout_policy_limits_and_namespace_are_checked_without_source_fallback() {
    let (a, baseline) = loadout_artifacts();
    let xml = "<PathOfBuilding2><Items><ItemSet id=\"1\"/></Items></PathOfBuilding2>";
    let mut changed = baseline.clone();
    changed.equipment_loadouts = (0..129)
        .map(|i| {
            let mut rule = baseline.equipment_loadouts[0].clone();
            rule.source_slot = SourceComponent::Text(format!("slot-{i}"));
            rule
        })
        .collect();
    assert!(matches!(
        normalize_with_loadouts(xml, &a, &changed),
        Err(NormalizationError::Policy("equipment loadout rules"))
    ));
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].scope = ImportEquipmentScope::Selected {
        loadouts: (0..65).map(|i| key(&format!("loadout-{i}"))).collect(),
    };
    assert!(matches!(
        normalize_with_loadouts(xml, &a, &changed),
        Err(NormalizationError::Policy("equipment loadout keys"))
    ));
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].source_slot = SourceComponent::Missing;
    assert!(normalize_with_loadouts(xml, &a, &changed).is_err());
    let mut changed = baseline.clone();
    changed.equipment_loadouts[0].source_slot =
        SourceComponent::Text("x".repeat(OwnedMappingLimits::default().max_string_bytes + 1));
    assert!(normalize_with_loadouts(xml, &a, &changed).is_err());
    let mut changed = baseline;
    changed.equipment_loadouts[0].destination =
        EquipmentSlotDefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "slot")
            .unwrap();
    assert!(matches!(
        normalize_with_loadouts(xml, &a, &changed),
        Err(NormalizationError::Binding)
    ));
}

fn rebind_quality_schema(
    a: &mut Artifacts,
    policy: &mut NormalizationPolicy,
    schema: SchemaPackageInput,
) {
    a.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut mapping = a.mapping.input().clone();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping = OwnedMappingIndex::new(
        mapping,
        &a.registry,
        &a.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, SkillCatalogLimits::default())
        .unwrap();
    a.rewards = empty_rewards(&a.mapping, &a.schema);
    if let GemQualityPolicy::Attributes(input) = &mut policy.gem_quality {
        input.definitions = a.schema.identity().clone();
    }
    if let Some(input) = &mut policy.gem_inputs {
        input.definitions = a.schema.identity().clone();
    }
}
fn quality_artifacts() -> (Artifacts, NormalizationPolicy, QualityDefId, UnitDefId) {
    let mut a = artifacts(true);
    let mut policy = policy();
    let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let kind = a
        .registry
        .allocate_definition::<QualityDefinition>()
        .unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            }),
        }));
    schema
        .definitions
        .push(DefinitionDescriptor::Quality(DefinitionEntry {
            id: kind.clone(),
            schema: SchemaState::Known(QualitySchema {
                amount: QuantityRange {
                    minimum: FiniteQuantity::new(0.0, unit.clone()).unwrap(),
                    maximum: FiniteQuantity::new(100.0, unit.clone()).unwrap(),
                },
            }),
        }));
    let mut amount = value_recipe("explicit-quality", "quality", false);
    amount.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Decimal,
        unit: unit.clone(),
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    };
    policy.gem_quality = GemQualityPolicy::Attributes(Box::new(GemQualityPolicyInput {
        definitions: a.schema.identity().clone(),
        amount,
        kind_attribute: "quality-kind".into(),
        kinds: vec![
            GemQualityKindRule {
                source: SourceComponent::Missing,
                kind: kind.clone(),
            },
            GemQualityKindRule {
                source: SourceComponent::Text("normal".into()),
                kind: kind.clone(),
            },
        ],
    }));
    rebind_quality_schema(&mut a, &mut policy, schema);
    (a, policy, kind, unit)
}
fn quality_xml(attributes: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="1"><Skill enabled="true"><Gem gemId="active" variantId="v" level="17" enabled="true" {attributes}/></Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}
fn quality_pending_code(result: &NormalizedImport) -> &str {
    match &result.draft().input().gems.members[0].quality {
        DraftQuality::Pending(value) => value.code.as_str(),
        _ => panic!("expected unresolved authored quality"),
    }
}
#[test]
fn physical_gem_quality_preserves_zero_decimal_and_exact_explicit_kind_without_closing_schema() {
    let (a, policy, kind, unit) = quality_artifacts();
    for (attributes, expected) in [
        (r#"quality="0""#, 0.0),
        (r#"quality="20" quality-kind="normal""#, 20.0),
        (r#"quality="1.5""#, 1.5),
    ] {
        let normalized = normalize_with_loadouts(&quality_xml(attributes), &a, &policy).unwrap();
        let gem = &normalized.draft().input().gems.members[0];
        let Some(Some(quality)) = gem.quality.to_resolved() else {
            panic!("explicit quality lost");
        };
        assert_eq!(quality.kind, kind);
        assert_eq!(quality.amount.unit(), &unit);
        assert_eq!(quality.amount.value(), expected);
        assert!(matches!(
            gem.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            a.schema.definition(&gem.definition.to_resolved().unwrap()),
            SchemaLookup::Unmapped(_)
        ));
    }
    let mut explicit = policy.clone();
    let GemQualityPolicy::Attributes(input) = &mut explicit.gem_quality else {
        unreachable!()
    };
    input
        .kinds
        .retain(|row| row.source != SourceComponent::Missing);
    let result = normalize_with_loadouts(&quality_xml(r#"quality="20""#), &a, &explicit).unwrap();
    assert_eq!(quality_pending_code(&result), "gem-quality-kind-unmapped");
}
#[test]
fn absent_malformed_ambiguous_and_unknown_quality_evidence_stays_distinct_and_pending() {
    let (a, policy, _, _) = quality_artifacts();
    for (attributes, code) in [
        ("", "gem-quality-amount-missing"),
        (r#"quality="""#, "gem-quality-amount-malformed"),
        (r#"quality="NaN""#, "gem-quality-amount-malformed"),
        (r#"quality="1e2""#, "gem-quality-amount-malformed"),
        (r#"quality="&#50;0""#, "gem-quality-amount-unavailable"),
        (r#"quality="-1""#, "gem-quality-amount-outside-schema"),
        (r#"quality="101""#, "gem-quality-amount-outside-schema"),
        (
            r#"quality="20" quality-kind="unknown""#,
            "gem-quality-kind-unmapped",
        ),
        (
            r#"quality="20" quality-kind="""#,
            "gem-quality-kind-unmapped",
        ),
        (
            r#"quality="20" quality-kind="norm&#97;l""#,
            "gem-quality-kind-unavailable",
        ),
    ] {
        let result = normalize_with_loadouts(&quality_xml(attributes), &a, &policy).unwrap();
        assert_eq!(quality_pending_code(&result), code, "{attributes}");
        assert_eq!(
            result.draft().input().gems.members[0].quality.to_resolved(),
            None
        );
    }
    let mut alternate = policy.clone();
    let GemQualityPolicy::Attributes(input) = &mut alternate.gem_quality else {
        unreachable!()
    };
    input.amount.tiers[0].selectors.push(ValueSelector {
        lane: ValueLane::Attribute,
        name: "alternate-quality".into(),
    });
    let result = normalize_with_loadouts(
        &quality_xml(r#"quality="20" alternate-quality="30""#),
        &a,
        &alternate,
    )
    .unwrap();
    assert_eq!(
        quality_pending_code(&result),
        "gem-quality-amount-ambiguous"
    );
    let GemQualityPolicy::Attributes(input) = &mut alternate.gem_quality else {
        unreachable!()
    };
    input.amount.tiers[0].selectors.pop();
    input.amount.tiers.push(ValueTier {
        selectors: vec![ValueSelector {
            lane: ValueLane::Attribute,
            name: "alternate-quality".into(),
        }],
        duplicates: DuplicatePolicy::Reject,
    });
    let result = normalize_with_loadouts(
        &quality_xml(r#"quality="NaN" alternate-quality="20""#),
        &a,
        &alternate,
    )
    .unwrap();
    assert_eq!(
        quality_pending_code(&result),
        "gem-quality-amount-malformed",
        "present invalid higher tier cannot choose fallback"
    );
}
#[test]
fn missing_quality_or_unit_schema_is_unresolved_and_known_unit_contradiction_rejects() {
    for unknown_unit in [false, true] {
        let (mut a, mut policy, kind, unit) = quality_artifacts();
        let mut schema = a.schema.input().clone();
        for entry in &mut schema.definitions {
            match entry {
                DefinitionDescriptor::Quality(row) if !unknown_unit && row.id == kind => {
                    *row = unknown(kind.clone());
                }
                DefinitionDescriptor::Unit(row) if unknown_unit && row.id == unit => {
                    *row = unknown(unit.clone());
                }
                _ => {}
            }
        }
        rebind_quality_schema(&mut a, &mut policy, schema);
        let result = normalize_with_loadouts(&quality_xml(r#"quality="20""#), &a, &policy).unwrap();
        assert_eq!(
            quality_pending_code(&result),
            "gem-quality-schema-unresolved"
        );
    }
    let (mut a, mut policy, _, _) = quality_artifacts();
    let other_unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: other_unit.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            }),
        }));
    rebind_quality_schema(&mut a, &mut policy, schema);
    let GemQualityPolicy::Attributes(input) = &mut policy.gem_quality else {
        unreachable!()
    };
    let ValueCodecKind::Quantity { unit, .. } = &mut input.amount.codec.codec else {
        unreachable!()
    };
    *unit = other_unit; // Same dimension is not the same owned unit.
    assert!(matches!(
        normalize_with_loadouts(&quality_xml(r#"quality="20""#), &a, &policy),
        Err(NormalizationError::Policy(
            "gem quality schema unit or range"
        ))
    ));
}

#[test]
fn known_gem_quality_presence_and_complete_or_partial_membership_are_respected() {
    for (presence, include, partial, pending) in [
        (QualityPresence::Optional, true, false, None),
        (
            QualityPresence::Forbidden,
            false,
            false,
            Some("gem-quality-forbidden"),
        ),
        (
            QualityPresence::Optional,
            false,
            false,
            Some("gem-quality-kind-not-allowed"),
        ),
        (
            QualityPresence::Optional,
            false,
            true,
            Some("gem-quality-owner-schema-unresolved"),
        ),
        (QualityPresence::Optional, true, true, None),
    ] {
        let (mut a, mut policy, kind, _) = quality_artifacts();
        let role = a
            .roles
            .input()
            .roles
            .iter()
            .find(|row| matches!(row.role, OwnedGemRole::Known(AuthoredGemRole::SkillUse)))
            .unwrap()
            .clone();
        let OwnedPrimarySkill::Known(primary) = role.primary else {
            unreachable!()
        };
        let members = if include { vec![kind] } else { vec![] };
        let allowed_kinds = if partial {
            DeclaredSet::partial(
                members,
                vec![SchemaGap {
                    subject: subject(&role.gem),
                    facet: SchemaFacet::InputSchema,
                    code: key("quality-membership-partial"),
                }],
            )
        } else {
            DeclaredSet::complete(members)
        };
        let mut schema = a.schema.input().clone();
        for entry in &mut schema.definitions {
            if let DefinitionDescriptor::Gem(row) = entry
                && row.id == role.gem
            {
                row.schema = SchemaState::Known(GemSchema {
                    level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(100).unwrap(),
                    },
                    roles: vec![AuthoredGemRole::SkillUse],
                    skills: DeclaredSet::complete(vec![primary.clone()]),
                    quality: QualityUseSchema {
                        presence,
                        allowed_kinds: allowed_kinds.clone(),
                    },
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![]),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                });
            }
        }
        rebind_quality_schema(&mut a, &mut policy, schema);
        let result = normalize_with_loadouts(&quality_xml(r#"quality="20""#), &a, &policy).unwrap();
        if let Some(code) = pending {
            assert_eq!(quality_pending_code(&result), code);
        } else {
            assert!(matches!(
                result.draft().input().gems.members[0].quality.to_resolved(),
                Some(Some(_))
            ));
        }
    }
}
#[test]
fn gem_quality_policy_rejects_stale_identity_defaults_lanes_duplicates_and_resource_overflow() {
    let (a, baseline, kind, unit) = quality_artifacts();
    let xml = quality_xml(r#"quality="20""#);
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.definitions = artifacts(false).schema.identity().clone();
    assert!(matches!(
        normalize_with_loadouts(&xml, &a, &changed),
        Err(NormalizationError::Binding)
    ));
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.amount.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Quantity(FiniteQuantity::new(0.0, unit).unwrap()),
    };
    assert!(matches!(
        normalize_with_loadouts(&xml, &a, &changed),
        Err(NormalizationError::Policy(
            "gem quality needs explicit attribute amount"
        ))
    ));
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.amount.tiers[0].selectors[0].lane = ValueLane::ParentAttribute;
    assert!(normalize_with_loadouts(&xml, &a, &changed).is_err());
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.kinds.push(input.kinds[0].clone());
    assert!(normalize_with_loadouts(&xml, &a, &changed).is_err());
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.kinds = (0..65)
        .map(|index| GemQualityKindRule {
            source: SourceComponent::Text(format!("kind-{index}")),
            kind: kind.clone(),
        })
        .collect();
    assert!(normalize_with_loadouts(&xml, &a, &changed).is_err());
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.kinds[0].kind =
        QualityDefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "kind").unwrap();
    assert!(matches!(
        normalize_with_loadouts(&xml, &a, &changed),
        Err(NormalizationError::Binding)
    ));
    let mut changed = baseline.clone();
    let GemQualityPolicy::Attributes(input) = &mut changed.gem_quality else {
        unreachable!()
    };
    input.kind_attribute = "x".repeat(ValuePolicyLimits::default().max_selector_bytes + 1);
    assert!(normalize_with_loadouts(&xml, &a, &changed).is_err());
    let oversized = "1".repeat(OwnedValueLimits::default().max_source_bytes + 1);
    assert!(
        normalize_with_loadouts(
            &quality_xml(&format!("quality=\"{oversized}\"")),
            &a,
            &baseline
        )
        .is_err()
    );
    let mut stale = serde_json::to_value(&baseline).unwrap();
    stale.as_object_mut().unwrap().remove("gem_quality");
    assert!(serde_json::from_value::<NormalizationPolicy>(stale).is_err());
    let mut unknown = serde_json::to_value(&baseline).unwrap();
    unknown["gem_quality"]["value"]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
    let repeated = serde_json::to_string(&baseline).unwrap().replace(
        "\"kind_attribute\":\"quality-kind\"",
        "\"kind_attribute\":\"quality-kind\",\"kind_attribute\":\"other\"",
    );
    assert!(serde_json::from_str::<NormalizationPolicy>(&repeated).is_err());
    let bad_unconverted = serde_json::json!({"kind":"unconverted","value":{"unknown":true}});
    assert!(serde_json::from_value::<GemQualityPolicy>(bad_unconverted).is_err());
}

fn socket_membership_issues(result: &NormalizedImport) -> [DraftIssueId; 2] {
    let draft = result.draft().input();
    [
        (
            &draft.items.completion,
            "socketed-item-membership-not-converted",
        ),
        (
            &draft.equipment.completion,
            "socketed-equipment-membership-not-converted",
        ),
    ]
    .map(|(completion, expected)| {
        let DraftListCompletion::Pending { id, code } = completion else {
            panic!("unmaterialized rune children cannot be a closed collection");
        };
        assert_eq!(code.as_str(), expected);
        *id
    })
}

#[test]
fn original_rune_children_keep_semantic_membership_open_without_fabricating_occurrences() {
    let artifacts = artifacts(false);
    for (index, item_count, use_count) in [(1, 34, 61), (2, 17, 17)] {
        let imported = source(ORIGINALS[index], 0x71 + index as u8);
        let result = run(&imported, &artifacts, &queries());
        let issues = socket_membership_issues(&result);
        let draft = result.draft().input();
        assert_eq!(draft.items.members.len(), item_count);
        assert_eq!(draft.equipment.members.len(), use_count);
        assert!(
            draft
                .equipment
                .members
                .iter()
                .all(|u| !matches!(u.destination, DraftEquipmentDestination::ItemSocket { .. }))
        );
        assert!(
            draft
                .equipment_presets
                .members
                .iter()
                .all(|p| matches!(p.equipment.completion, DraftListCompletion::Pending { .. }))
        );
        for item in &result.sidecar().item_texts {
            if !item
                .attribution
                .lines
                .iter()
                .any(|l| l.raw.trim_ascii().starts_with("Rune:") || l.raw.contains("{rune}"))
            {
                continue;
            }
            let origin = result
                .sidecar()
                .origins
                .iter()
                .find(|o| o.source == item.source)
                .unwrap();
            for issue in issues {
                assert!(origin.links.contains(&OwnedOriginTarget::Issue(issue)));
            }
        }
        origin_integrity(&imported, &result);
        // Contrasting source has the same XML Item/Slot graph and no rune
        // evidence. Existing known records stay identical, but membership can
        // remain closed; this is not a blanket closure downgrade for all items.
        let plain_xml = ORIGINALS[index]
            .lines()
            .filter(|line| !line.trim_ascii().starts_with("Rune:") && !line.contains("{rune}"))
            .collect::<Vec<_>>()
            .join("\n");
        let plain_source = source(&plain_xml, 0x71 + index as u8);
        let plain = run(&plain_source, &artifacts, &queries());
        assert!(matches!(
            plain.draft().input().items.completion,
            DraftListCompletion::Complete
        ));
        assert!(matches!(
            plain.draft().input().equipment.completion,
            DraftListCompletion::Complete
        ));
        assert_eq!(plain.draft().input().items.members, draft.items.members);
        assert_eq!(
            plain.draft().input().equipment.members,
            draft.equipment.members
        );
        origin_integrity(&plain_source, &plain);
    }
}

#[test]
fn unreviewed_rune_headers_and_saved_tagged_lines_cannot_prove_empty_children() {
    for evidence in [
        "Rune: Unknown Rune",
        "Rune: None",
        "{enchant}{rune}18% increased Physical Damage",
    ] {
        let xml = format!(
            r#"<PathOfBuilding2><Build level="42"/><Items><Item id="1">Rarity: NORMAL
Grand Spear
Sockets: S
{evidence}
Implicits: 0</Item><ItemSet id="1"><Slot name="Weapon 1" itemId="1"/></ItemSet></Items></PathOfBuilding2>"#
        );
        let imported = source(&xml, 0x77);
        let result = run(&imported, &artifacts(false), &[]);
        socket_membership_issues(&result);
        assert_eq!(result.draft().input().items.members.len(), 1);
        assert_eq!(result.draft().input().equipment.members.len(), 1);
        origin_integrity(&imported, &result);
    }
}

#[test]
fn plain_items_and_rune_named_presentation_titles_keep_original_collection_closure() {
    for text in [
        "Rarity: NORMAL\nGrand Spear\nImplicits: 0",
        "Rarity: RARE\nRune: Presentation Title\nGrand Spear\nImplicits: 0",
    ] {
        let xml = format!(
            r#"<PathOfBuilding2><Build level="42"/><Items><Item id="1">{text}</Item><ItemSet id="1"><Slot name="Weapon 1" itemId="1"/></ItemSet></Items></PathOfBuilding2>"#
        );
        let imported = source(&xml, 0x78);
        let result = run(&imported, &artifacts(false), &[]);
        assert!(matches!(
            result.draft().input().items.completion,
            DraftListCompletion::Complete
        ));
        assert!(matches!(
            result.draft().input().equipment.completion,
            DraftListCompletion::Complete
        ));
        origin_integrity(&imported, &result);
    }
}

#[derive(Clone, Copy)]
enum GemParameterShape {
    Empty,
    PartialEmpty,
    Required,
    Optional,
    Unmapped,
}

// Only this test helper assigns domain knowledge. The production normalizer sees
// the same schema-bound direct declarations for arbitrary injected gem identities.
fn replace_gem_input_schema(
    a: &mut Artifacts,
    role: AuthoredGemRole,
    parameters: GemParameterShape,
    declare_choice: bool,
) {
    let role_row = a
        .roles
        .input()
        .roles
        .iter()
        .find(|row| matches!(&row.role, OwnedGemRole::Known(value) if *value == role))
        .unwrap()
        .clone();
    if matches!(parameters, GemParameterShape::Unmapped) {
        return;
    }
    let OwnedPrimarySkill::Known(primary) = role_row.primary else {
        unreachable!()
    };
    let owner = SlotOwnerDefId::Gem(role_row.gem.clone());
    let mut schema = a.schema.input().clone();
    let parameters = match parameters {
        GemParameterShape::Empty => DeclaredSet::complete(vec![]),
        GemParameterShape::PartialEmpty => DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject: subject(&role_row.gem),
                facet: SchemaFacet::InputSchema,
                code: key("gem-parameter-membership-unconverted"),
            }],
        ),
        GemParameterShape::Required | GemParameterShape::Optional => {
            let slot: DeclaredSlot<ParameterSlotDefId> =
                a.registry.allocate_slot(owner.clone()).unwrap();
            schema
                .slots
                .push(SlotDescriptor::Parameter(DefinitionEntry {
                    id: slot.clone(),
                    schema: SchemaState::Known(ParameterSlotSchema {
                        value: ValueSchema::Boolean,
                        presence: if matches!(parameters, GemParameterShape::Required) {
                            SlotPresence::RequiredOnce
                        } else {
                            SlotPresence::OptionalOnce
                        },
                        sites: vec![ParameterSite::GemParameter],
                    }),
                }));
            DeclaredSet::complete(vec![slot])
        }
        GemParameterShape::Unmapped => unreachable!(),
    };
    let choices = if declare_choice {
        let slot: DeclaredSlot<ChoiceSlotDefId> = a.registry.allocate_slot(owner).unwrap();
        schema.slots.push(SlotDescriptor::Choice(DefinitionEntry {
            id: slot.clone(),
            schema: SchemaState::Known(ChoiceSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                owners: vec![ChoiceOwnerScope::Provider(match role {
                    AuthoredGemRole::SkillUse => ProviderRole::SkillUse,
                    AuthoredGemRole::SupportAssignment => ProviderRole::SupportAssignment,
                })],
            }),
        }));
        DeclaredSet::complete(vec![slot])
    } else {
        DeclaredSet::complete(vec![])
    };
    let gem = schema
        .definitions
        .iter_mut()
        .find_map(|entry| match entry {
            DefinitionDescriptor::Gem(row) if row.id == role_row.gem => Some(row),
            _ => None,
        })
        .unwrap();
    gem.schema = SchemaState::Known(GemSchema {
        level: IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        },
        roles: vec![role],
        skills: DeclaredSet::complete(vec![primary]),
        quality: QualityUseSchema {
            presence: QualityPresence::Forbidden,
            allowed_kinds: DeclaredSet::complete(vec![]),
        },
        declarations: DeclaredSlots {
            parameters,
            choices,
            grants: DeclaredSet::complete(vec![]),
            actors: DeclaredSet::complete(vec![]),
            skill_grants: DeclaredSet::complete(vec![]),
            outputs: DeclaredSet::complete(vec![]),
            sockets: DeclaredSet::complete(vec![]),
        },
    });
    rebind_quality_schema(a, &mut policy(), schema);
}

fn explicit_empty_gem_policy(a: &Artifacts) -> NormalizationPolicy {
    let mut policy = policy();
    policy.gem_inputs = Some(GemInputPolicy {
        definitions: a.schema.identity().clone(),
        gems: a
            .roles
            .input()
            .roles
            .iter()
            .filter_map(|row| {
                let SchemaLookup::Known(schema) = a.schema.definition(&row.gem) else {
                    return None;
                };
                (schema.declarations.parameters.is_complete()
                    && schema.declarations.parameters.members.is_empty())
                .then(|| GemInputRule {
                    gem: row.gem.clone(),
                    guards: vec![GemInputGuard {
                        attribute: "neutral-input".into(),
                        allowed: vec![SourceComponent::Missing],
                    }],
                    parameters: vec![],
                })
            })
            .collect(),
    });
    policy
}
#[test]
fn complete_empty_gem_parameters_close_for_distinct_active_and_support_occurrences() {
    let mut a = artifacts(true);
    for role in [
        AuthoredGemRole::SkillUse,
        AuthoredGemRole::SupportAssignment,
    ] {
        replace_gem_input_schema(&mut a, role, GemParameterShape::Empty, false);
    }
    let xml = group(&format!("{ACTIVE}{SUPPORT}{ACTIVE}{SUPPORT}"));
    let source = source(&xml, 0x81);
    let result = run_with_policy(&source, &a, &queries(), &explicit_empty_gem_policy(&a));
    let draft = result.draft().input();
    assert_eq!(draft.gems.members.len(), 4);
    assert_eq!(draft.skills.members.len(), 2);
    assert_eq!(draft.supports.members.len(), 2);
    assert_eq!(
        draft
            .gems
            .members
            .iter()
            .map(|gem| gem.id)
            .collect::<BTreeSet<_>>()
            .len(),
        4,
        "equal definitions do not merge physical gem occurrences"
    );
    for gem in &draft.gems.members {
        assert!(gem.parameters.members.is_empty());
        assert!(matches!(
            gem.parameters.completion,
            DraftListCompletion::Complete
        ));
        assert!(matches!(gem.quality, DraftQuality::Pending(_)));
    }
    assert!(
        draft
            .skills
            .members
            .iter()
            .all(|skill| matches!(skill.scope, DraftField::Pending(_)))
    );
    assert!(
        draft
            .supports
            .members
            .iter()
            .all(|support| matches!(support.target, DraftSkillTarget::Pending(_)))
    );
    assert!(draft.skill_presets.members.iter().all(|preset| matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    )));
    origin_integrity(&source, &result);
}

#[test]
fn partial_nonempty_and_unmapped_gem_parameters_never_infer_absence_from_xml() {
    for unresolved in [
        GemParameterShape::PartialEmpty,
        GemParameterShape::Required,
        GemParameterShape::Optional,
        GemParameterShape::Unmapped,
    ] {
        for unresolved_role in [
            AuthoredGemRole::SkillUse,
            AuthoredGemRole::SupportAssignment,
        ] {
            let mut a = artifacts(true);
            for role in [
                AuthoredGemRole::SkillUse,
                AuthoredGemRole::SupportAssignment,
            ] {
                replace_gem_input_schema(
                    &mut a,
                    role,
                    if role == unresolved_role {
                        unresolved
                    } else {
                        GemParameterShape::Empty
                    },
                    false,
                );
            }
            let xml = group(&format!("{ACTIVE}{SUPPORT}"));
            let source = source(&xml, 0x82);
            let result = run_with_policy(&source, &a, &[], &explicit_empty_gem_policy(&a));
            let draft = result.draft().input();
            assert_eq!(draft.gems.members.len(), 2);
            for (gem, role) in draft.gems.members.iter().zip([
                AuthoredGemRole::SkillUse,
                AuthoredGemRole::SupportAssignment,
            ]) {
                assert!(gem.parameters.members.is_empty());
                if role == unresolved_role {
                    let DraftListCompletion::Pending { code, .. } = &gem.parameters.completion
                    else {
                        panic!(
                            "incomplete owner inputs cannot borrow another owner's empty declaration"
                        );
                    };
                    assert_eq!(code.as_str(), "gem-parameters-not-converted");
                } else {
                    assert!(matches!(
                        gem.parameters.completion,
                        DraftListCompletion::Complete
                    ));
                }
            }
            origin_integrity(&source, &result);
        }
    }
}

#[test]
fn empty_gem_parameter_schema_does_not_consume_source_fields_or_close_choices() {
    let mut a = artifacts(true);
    for role in [
        AuthoredGemRole::SkillUse,
        AuthoredGemRole::SupportAssignment,
    ] {
        replace_gem_input_schema(&mut a, role, GemParameterShape::Empty, true);
    }
    let xml = group(
        r#"<Gem gemId="active" variantId="v" future-option="false"/>
        <Gem gemId="support" variantId="v" future-option="&#48;"/>"#,
    );
    let source = source(&xml, 0x83);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let source_rows: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Gem")
        .collect();
    assert_eq!(source_rows.len(), 2);
    assert_eq!(
        source_rows[0]
            .attribute("future-option")
            .unwrap()
            .decoded()
            .unwrap(),
        "false"
    );
    assert!(
        source_rows[1]
            .attribute("future-option")
            .unwrap()
            .decoded()
            .is_err()
    );
    let result = run_with_policy(&source, &a, &queries(), &explicit_empty_gem_policy(&a));
    let draft = result.draft().input();
    for gem in &draft.gems.members {
        assert!(matches!(
            gem.parameters.completion,
            DraftListCompletion::Complete
        ));
        assert!(gem.parameters.members.is_empty());
        assert!(matches!(gem.level, DraftField::Pending(_)));
        assert!(matches!(gem.quality, DraftQuality::Pending(_)));
    }
    assert!(
        draft
            .skills
            .members
            .iter()
            .all(|skill| matches!(skill.enabled, DraftField::Pending(_)))
    );
    assert!(
        draft
            .supports
            .members
            .iter()
            .all(|support| matches!(support.enabled, DraftField::Pending(_)))
    );
    for preset in &draft.choice_presets.members {
        assert!(preset.choices.members.is_empty());
        assert!(matches!(
            preset.choices.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    for row in source_rows {
        let origin = result
            .sidecar()
            .origins
            .iter()
            .find(|origin| origin.source == row.occurrence().id())
            .unwrap();
        assert!(
            origin
                .links
                .iter()
                .any(|target| matches!(target, OwnedOriginTarget::Gem(_)))
        );
        assert!(
            origin
                .links
                .iter()
                .any(|target| matches!(target, OwnedOriginTarget::Issue(_)))
        );
    }
    origin_integrity(&source, &result);
}

fn shared_skill_policy(values: Vec<SourceComponent>) -> NormalizationPolicy {
    let mut result = policy();
    result.skill_scopes = Some(SkillScopePolicy {
        slot_attribute: "slot".into(),
        shared_slots: values,
    });
    result
}

#[test]
fn skill_scope_is_explicit_and_matches_missing_empty_and_named_slots_independently() {
    let a = artifacts(true);
    for (attribute, expected_component) in [
        ("", SourceComponent::Missing),
        (r#" slot="""#, SourceComponent::Text(String::new())),
        (
            r#" slot="reviewed-shared""#,
            SourceComponent::Text("reviewed-shared".into()),
        ),
    ] {
        let xml = group(ACTIVE).replace(
            r#"<Skill enabled="true">"#,
            &format!(r#"<Skill enabled="true"{attribute}>"#),
        );
        for component in [
            SourceComponent::Missing,
            SourceComponent::Text(String::new()),
            SourceComponent::Text("reviewed-shared".into()),
        ] {
            let result =
                normalize_with_loadouts(&xml, &a, &shared_skill_policy(vec![component.clone()]))
                    .unwrap();
            assert_eq!(result.draft().input().skills.members.len(), 1);
            let scope = &result.draft().input().skills.members[0].scope;
            if component == expected_component {
                assert_eq!(scope.to_resolved(), Some(LoadoutScope::Shared));
            } else {
                assert!(matches!(scope, DraftField::Pending(_)));
            }
            assert!(result.draft().input().weapon_loadouts.members.is_empty());
            assert!(matches!(
                result.draft().input().weapon_loadouts.completion,
                DraftListCompletion::Pending { .. }
            ));
            origin_integrity(&source(&xml, 93), &result);
        }
        for unresolved in [policy(), shared_skill_policy(vec![])] {
            let result = normalize_with_loadouts(&xml, &a, &unresolved).unwrap();
            assert!(matches!(
                result.draft().input().skills.members[0].scope,
                DraftField::Pending(_)
            ));
        }
    }
}

#[test]
fn skill_scope_uses_injected_attribute_and_never_borrows_sibling_slot_evidence() {
    let a = artifacts(true);
    let mut reviewed = shared_skill_policy(vec![SourceComponent::Text("common".into())]);
    reviewed.skill_scopes.as_mut().unwrap().slot_attribute = "placement".into();
    let xml = format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="1"><Skill enabled="true" slot="common">{ACTIVE}</Skill><Skill enabled="true" placement="common" slot="unreviewed">{ACTIVE}</Skill><Skill enabled="true" placement="Common">{ACTIVE}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = normalize_with_loadouts(&xml, &a, &reviewed).unwrap();
    let skills = &result.draft().input().skills.members;
    assert_eq!(skills.len(), 3);
    assert!(matches!(skills[0].scope, DraftField::Pending(_)));
    assert_eq!(skills[1].scope.to_resolved(), Some(LoadoutScope::Shared));
    assert!(matches!(skills[2].scope, DraftField::Pending(_)));
    assert_eq!(
        skills
            .iter()
            .map(|row| row.id)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    origin_integrity(&source(&xml, 93), &result);
}

#[test]
fn unknown_unavailable_and_namespaced_skill_slots_never_fall_back_to_missing() {
    let a = artifacts(true);
    let reviewed = shared_skill_policy(vec![SourceComponent::Missing]);
    for attribute in [
        r#"slot="unknown""#,
        r#"slot="""#,
        r#"slot="&#48;""#,
        r#"xmlns:x="urn:unreviewed" x:slot="unknown""#,
        r#"xmlns="urn:unreviewed" slot="unknown""#,
    ] {
        let xml = group(ACTIVE).replace(
            r#"<Skill enabled="true">"#,
            &format!(r#"<Skill enabled="true" {attribute}>"#),
        );
        let result = normalize_with_loadouts(&xml, &a, &reviewed).unwrap();
        assert!(
            result
                .draft()
                .input()
                .skills
                .members
                .iter()
                .all(|row| matches!(row.scope, DraftField::Pending(_))),
            "{attribute}"
        );
        if !attribute.starts_with("xmlns") {
            assert_eq!(result.draft().input().skills.members.len(), 1);
        }
        origin_integrity(&source(&xml, 93), &result);
    }
    // A namespaced document root is rejected before source normalization.
    let xml = group(ACTIVE).replace(
        "<PathOfBuilding2>",
        r#"<PathOfBuilding2 xmlns="urn:unreviewed">"#,
    );
    assert!(decode_build(xml.as_bytes()).is_err());
    // A namespace on an admitted document's ancestor must not authorize scope.
    let xml = group(ACTIVE).replace("<Skills>", r#"<Skills xmlns="urn:unreviewed">"#);
    let result = normalize_with_loadouts(&xml, &a, &reviewed).unwrap();
    assert!(
        result
            .draft()
            .input()
            .skills
            .members
            .iter()
            .all(|row| matches!(row.scope, DraftField::Pending(_)))
    );
}

#[test]
fn shared_skill_scope_does_not_override_enabled_or_global_effect_inputs() {
    let a = artifacts(true);
    let reviewed = shared_skill_policy(vec![SourceComponent::Missing]);
    for (gem_enabled, group_enabled, expected) in [
        ("true", "true", Some(true)),
        ("false", "true", Some(false)),
        ("true", "false", Some(false)),
        ("false", "false", Some(false)),
        ("invalid", "true", None),
        ("true", "invalid", None),
    ] {
        for global in ["true", "false", "invalid"] {
            let xml = group(&format!(
                r#"<Gem gemId="active" variantId="v" level="17" enabled="{gem_enabled}" enableGlobal1="{global}" enableGlobal2="{global}"/>"#
            ))
            .replace(
                r#"<Skill enabled="true">"#,
                &format!(r#"<Skill enabled="{group_enabled}">"#),
            );
            let result = normalize_with_loadouts(&xml, &a, &reviewed).unwrap();
            let skill = &result.draft().input().skills.members[0];
            assert_eq!(skill.scope.to_resolved(), Some(LoadoutScope::Shared));
            assert_eq!(skill.enabled.to_resolved(), expected);
            assert!(matches!(
                result.draft().input().gems.members[0].parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                result.draft().input().skill_presets.members[0]
                    .skills
                    .completion,
                DraftListCompletion::Pending { .. }
            ));
            origin_integrity(&source(&xml, 93), &result);
        }
    }
}

#[test]
fn shared_scopes_keep_independent_presets_and_ambiguous_support_targets() {
    let a = artifacts(true);
    let reviewed = shared_skill_policy(vec![SourceComponent::Missing]);
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill enabled="true">{ACTIVE}{ACTIVE}{SUPPORT}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = normalize_with_loadouts(&xml, &a, &reviewed).unwrap();
    let draft = result.draft().input();
    assert_eq!(draft.skills.members.len(), 3);
    assert_eq!(draft.supports.members.len(), 2);
    assert_eq!(draft.gems.members.len(), 5);
    assert_eq!(draft.skill_presets.members.len(), 2);
    assert!(
        draft
            .skills
            .members
            .iter()
            .all(|row| row.scope.to_resolved() == Some(LoadoutScope::Shared))
    );
    assert_eq!(draft.skill_presets.members[0].skills.members.len(), 2);
    assert_eq!(draft.skill_presets.members[1].skills.members.len(), 1);
    assert!(matches!(
        draft.supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    assert_eq!(
        draft.supports.members[1].target.to_resolved(),
        Some(SkillTarget::Authored(draft.skills.members[2].id))
    );
    assert!(draft.saved_variants.members.is_empty());
    for preset in &draft.skill_presets.members {
        assert!(matches!(
            preset.skills.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            preset.supports.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            preset.payload_links.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    origin_integrity(&source(&xml, 93), &result);
}

#[test]
fn shared_skill_policy_never_materializes_generated_or_provider_only_actives() {
    let reviewed = shared_skill_policy(vec![SourceComponent::Missing]);
    for generated in [false, true] {
        let mut a = artifacts(true);
        let mut xml = group(&format!("{ACTIVE}{SUPPORT}"));
        if generated {
            xml = xml.replace(
                r#"<Skill enabled="true">"#,
                r#"<Skill enabled="true" source="Item:1">"#,
            );
        } else {
            replace_materialization(&mut a, "active", OwnedGemMaterialization::ProviderOnly);
        }
        let result = normalize_with_loadouts(&xml, &a, &reviewed).unwrap();
        let draft = result.draft().input();
        assert!(draft.skills.members.is_empty());
        assert_eq!(draft.gems.members.len(), 1);
        assert_eq!(draft.supports.members.len(), 1);
        assert!(matches!(
            draft.supports.members[0].target,
            DraftSkillTarget::Pending(_)
        ));
        origin_integrity(&source(&xml, 93), &result);
    }
}

#[test]
fn skill_scope_policy_limits_and_wire_shape_reject_before_source_inference() {
    let a = artifacts(true);
    let xml = group(ACTIVE);
    let reviewed = shared_skill_policy(vec![SourceComponent::Missing]);
    let too_long = OwnedMappingLimits::default().max_string_bytes + 1;
    for scope in [
        SkillScopePolicy {
            slot_attribute: String::new(),
            shared_slots: vec![],
        },
        SkillScopePolicy {
            slot_attribute: "x".repeat(129),
            shared_slots: vec![],
        },
        SkillScopePolicy {
            slot_attribute: "slot".into(),
            shared_slots: vec![SourceComponent::Missing; 2],
        },
        SkillScopePolicy {
            slot_attribute: "slot".into(),
            shared_slots: vec![SourceComponent::Text("same".into()); 2],
        },
        SkillScopePolicy {
            slot_attribute: "slot".into(),
            shared_slots: (0..65)
                .map(|i| SourceComponent::Text(format!("slot-{i}")))
                .collect(),
        },
        SkillScopePolicy {
            slot_attribute: "slot".into(),
            shared_slots: vec![SourceComponent::Text("x".repeat(too_long))],
        },
    ] {
        let mut invalid = reviewed.clone();
        invalid.skill_scopes = Some(scope);
        assert!(matches!(
            normalize_with_loadouts(&xml, &a, &invalid),
            Err(NormalizationError::Policy(_))
        ));
    }
    let oversized = xml.replace(
        r#"<Skill enabled="true">"#,
        &format!(r#"<Skill enabled="true" slot="{}">"#, "x".repeat(too_long)),
    );
    assert!(matches!(
        normalize_with_loadouts(&oversized, &a, &reviewed),
        Err(NormalizationError::Limit(_))
    ));
    let mut unknown = serde_json::to_value(&reviewed).unwrap();
    unknown["skill_scopes"]["selected_loadouts"] = serde_json::json!(["unreviewed"]);
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
    let mut missing_field = serde_json::to_value(&reviewed).unwrap();
    missing_field["skill_scopes"]
        .as_object_mut()
        .unwrap()
        .remove("shared_slots");
    assert!(serde_json::from_value::<NormalizationPolicy>(missing_field).is_err());
    let mut unsupported = serde_json::to_value(&reviewed).unwrap();
    unsupported["skill_scopes"]["shared_slots"] = serde_json::json!([{"kind":"wildcard"}]);
    assert!(serde_json::from_value::<NormalizationPolicy>(unsupported).is_err());
}

#[test]
fn omitted_skill_policy_preserves_legacy_canonical_bytes_and_tree_binding() {
    use poe_optimizer_core::owned_content::digest_owned;
    const LEGACY: &str =
        include_str!("../../../data/owned/poe2/3887ae68/current/normalization.json");
    let restored: NormalizationPolicy = serde_json::from_str(LEGACY).unwrap();
    assert!(restored.skill_scopes.is_none());
    assert_eq!(serde_json::to_string(&restored).unwrap(), LEGACY.trim());
    let legacy_digest = digest_owned(
        "owned-normalization-policy-v3",
        &restored,
        NormalizationLimits::default().max_policy_bytes,
    )
    .unwrap();
    assert_eq!(
        legacy_digest.to_string(),
        "a194afb11f92394cda0025f6fd0663f150bbf53e8dab50aea5ee568acb33d003"
    );
    let mut changed = restored.clone();
    changed.skill_scopes = Some(SkillScopePolicy {
        slot_attribute: "slot".into(),
        shared_slots: vec![SourceComponent::Missing],
    });
    assert_ne!(
        digest_owned(
            "owned-normalization-policy-v3",
            &changed,
            NormalizationLimits::default().max_policy_bytes,
        )
        .unwrap(),
        legacy_digest
    );
    let a = artifacts(true);
    let xml = group(ACTIVE);
    let baseline = normalize_with_loadouts(&xml, &a, &policy()).unwrap();
    let reviewed = normalize_with_loadouts(
        &xml,
        &a,
        &shared_skill_policy(vec![SourceComponent::Missing]),
    )
    .unwrap();
    assert_ne!(baseline.sidecar().policy, reviewed.sidecar().policy);
    assert!(matches!(
        baseline.draft().input().skills.members[0].scope,
        DraftField::Pending(_)
    ));
    assert_eq!(
        reviewed.draft().input().skills.members[0]
            .scope
            .to_resolved(),
        Some(LoadoutScope::Shared)
    );
}

#[path = "support/owned_gem_inputs.rs"]
mod gem_input_tests;
