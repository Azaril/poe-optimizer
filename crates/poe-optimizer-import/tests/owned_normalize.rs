//! Fresh normalization contracts with injected artifacts; no evaluator or VM.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
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
    Artifacts {
        registry,
        schema,
        mapping,
        roles,
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
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
        },
        &policy(),
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
        OwnedOriginTarget::Item(v) | OwnedOriginTarget::ItemReference(v) => v.instance_id(),
        OwnedOriginTarget::Equipment(v) => v.instance_id(),
        OwnedOriginTarget::Gem(v) => v.instance_id(),
        OwnedOriginTarget::Skill(v) => v.instance_id(),
        OwnedOriginTarget::Support(v) => v.instance_id(),
        OwnedOriginTarget::Allocation(v) => v.instance_id(),
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
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
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
                mappings: &artifacts.mapping,
                registry: &artifacts.registry,
                definitions: &artifacts.schema,
                roles: &artifacts.roles,
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
                mappings: &artifacts.mapping,
                registry: &artifacts.registry,
                definitions: &artifacts.schema,
                roles: &artifacts.roles
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
