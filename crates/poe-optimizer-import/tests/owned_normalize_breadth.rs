//! Offline breadth regression: pinned identity data -> owned artifacts -> drafts.
//! The fixed reference manifest supplies ordered query identities only. Its
//! numerical results, source programs and UI selections are never evaluated.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{ParameterValue, QueryId},
    owned_definitions::{GameVersionNamespace, OwnedDefinitionKey},
    owned_draft::*,
    owned_schema::{DefinitionDescriptor, SchemaState},
};
use poe_optimizer_data::{
    owned_schema::*,
    skill_identities::{SkillIdentity, SkillIdentityCatalog, SkillIdentityData},
};
use poe_optimizer_import::{
    build_instance::{
        AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits, ProjectionKind,
        ProjectionState,
    },
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_skill_catalog::*,
    owned_source::*,
    owned_value::*,
    owned_value_policy::*,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs::File, io::Read, path::Path};

const CATALOG_SHA256: &str = "a90217d9bab6c0469917a2ba75ed9517205d2ac2b3d59f8d68580604f26df07f";

fn read_bounded(path: &Path, maximum: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(
        bytes.len() <= maximum,
        "{} exceeds test bound",
        path.display()
    );
    bytes
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "breadth-owned-identity-v1").unwrap()
}

// Intentional test-only projections of existing artifacts. Unrelated package
// sections/reference values are skipped by serde, rather than loaded as rules.
#[derive(Deserialize)]
struct IdentityProjection {
    manifest: PackageVersion,
    skill_identities: SkillIdentityData,
}
#[derive(Deserialize)]
struct PackageVersion {
    schema_version: u32,
}
#[derive(Deserialize)]
struct ReferenceManifest {
    schema_version: u32,
    cases: Vec<ReferenceCase>,
}
#[derive(Deserialize)]
struct ReferenceCase {
    source_line: usize,
    input: ReferenceInput,
    measurements: Vec<ReferenceMeasurement>,
}
#[derive(Deserialize)]
struct ReferenceInput {
    xml_path: String,
    xml_sha256: String,
}
#[derive(Deserialize)]
struct ReferenceMeasurement {
    query: ReferenceQuery,
}
#[derive(Deserialize)]
struct ReferenceQuery {
    actor: String,
    id: String,
}

struct Artifacts {
    catalog: SkillIdentityCatalog,
    registry: OwnedIdRegistry,
    definitions: OwnedDefinitionSchemaPackage,
    mappings: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
}
fn artifacts(root: &Path) -> Artifacts {
    let bytes = read_bounded(
        &root.join("crates/poe-optimizer-data/data/game-data.json"),
        32 * 1024 * 1024,
    );
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), CATALOG_SHA256);
    let projection: IdentityProjection = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(projection.manifest.schema_version, 40);
    let catalog = SkillIdentityCatalog::new(projection.skill_identities).unwrap();
    assert_eq!(catalog.data().gems.len(), 966);
    assert_eq!(catalog.data().skills.len(), 1_436);
    let source = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: catalog.data().source.upstream_revision.clone(),
        files: catalog
            .data()
            .source
            .files
            .iter()
            .map(|(path, sha256)| SourceFilePin {
                path: path.clone(),
                sha256: sha256.clone(),
            })
            .collect(),
    };
    let limits = SkillCatalogLimits::default();
    let base = OwnedIdRegistry::empty(namespace(), limits.mapping).unwrap();
    let base_digest = base.identity().unwrap();
    let compiled = compile_fresh_owned_skill_catalog(
        &catalog,
        &base,
        &source,
        &SkillCatalogPolicy {
            version: key("reviewed-pob2-identity-role-v1"),
            // Reviewed source identity convention; not a production default.
            absent_support: AbsentSupportPolicy::NonSupport,
            absent_from_tree: AbsentFromTreePolicy::Physical,
        },
        limits,
    )
    .unwrap();
    assert_eq!(base.identity().unwrap(), base_digest);
    assert_eq!(compiled.receipt.gem_count, 966);
    assert_eq!(compiled.receipt.skill_count, 1_436);
    assert_eq!(compiled.definitions.len(), 2_402);
    assert_eq!(compiled.roles.len(), 966);
    assert!(!compiled.mappings.is_empty());
    for descriptor in &compiled.definitions {
        // Identity-only compilation cannot certify level/quality/rules coverage.
        match descriptor {
            DefinitionDescriptor::Gem(row) => {
                assert!(matches!(row.schema, SchemaState::Unmapped { .. }));
            }
            DefinitionDescriptor::Skill(row) => {
                assert!(matches!(row.schema, SchemaState::Unmapped { .. }));
            }
            other => panic!("unexpected identity definition: {other:?}"),
        }
    }
    let definitions = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("reviewed-breadth-identities-v1"),
            semantics_version: key("identity-only-unmapped-input-schema-v1"),
            definitions: compiled.definitions,
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mappings = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: namespace(),
            registry: compiled.registry.identity().unwrap(),
            definitions: definitions.identity().clone(),
            source,
            policy_version: compiled.receipt.policy.version.clone(),
            entries: compiled.mappings,
        },
        &compiled.registry,
        &definitions,
        limits.mapping,
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: namespace(),
            definitions: definitions.identity().clone(),
            mapping: *mappings.identity(),
            compilation: compiled.receipt,
            roles: compiled.roles,
        },
        &mappings,
        &definitions,
        limits,
    )
    .unwrap();
    Artifacts {
        catalog,
        registry: compiled.registry,
        definitions,
        mappings,
        roles,
    }
}

fn recipe(name: &str, boolean: bool) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(if boolean {
            "reviewed-enabled"
        } else {
            "reviewed-level"
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
    // These finite syntax choices are injected for this reviewed source version.
    // No class/item/config values or build-specific conversion branches are added.
    NormalizationPolicy {
        version: key("reviewed-pob2-breadth-import-v1"),
        namespace: namespace(),
        character_level: recipe("level", false),
        gem_level: recipe("level", false),
        gem_enabled: recipe("enabled", true),
        group_enabled: recipe("enabled", true),
        manual_skill_sources: vec![
            SourceComponent::Missing,
            SourceComponent::Text(String::new()),
        ],
        empty_item_keys: vec![SourceComponent::Text("0".into())],
        // Generated provider and support correspondence awaits a separate policy.
        generated_support_prefixes: vec![],
        allocation_attribute: "nodes".into(),
        single_active_support_target: true,
    }
}
fn queries(case: &ReferenceCase) -> Vec<ImportQueryTemplate> {
    assert_eq!(case.measurements.len(), 22);
    case.measurements
        .iter()
        .enumerate()
        .map(|(index, row)| {
            ImportQueryTemplate {
                // Caller request identity follows fixed manifest order, not a game ID.
                id: QueryId::new(format!("reference-{index:02}")).unwrap(),
                metric: ExternalSelector::Catalog {
                    kind: ExternalCatalogKind::Metric,
                    key: SourceComponent::Text(row.query.id.clone()),
                    version: SourceComponent::Missing,
                    variant: SourceComponent::Missing,
                },
                target: match row.query.actor.as_str() {
                    "player" => ImportQueryTarget::Player,
                    "selected_minion" => {
                        ImportQueryTarget::Unresolved(key("reference-selected-minion-unresolved"))
                    }
                    actor => panic!("unreviewed reference actor: {actor}"),
                },
            }
        })
        .collect()
}
fn known<T>(field: &DraftField<T>) -> &T {
    match field {
        DraftField::Known { value } => value,
        DraftField::Pending(_) => panic!("expected preserved known field"),
    }
}
fn attribute<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> Option<&'a str> {
    row.attribute(name).map(|a| a.decoded().unwrap())
}
// Independent source-fact join: no owned compiler role/materialization result is
// consulted, and missing/unknown variants never use a single-candidate fallback.
fn exact_primary<'a>(
    catalog: &'a SkillIdentityCatalog,
    row: &SourceEvidenceRow<'_>,
) -> Option<&'a SkillIdentity> {
    let game_id = attribute(row, "gemId")?;
    let variant_id = attribute(row, "variantId")?;
    let mut candidates = catalog
        .gems_for_external_id(game_id)
        .filter(|gem| gem.variant_id == variant_id);
    let gem = candidates.next()?;
    assert!(
        candidates.next().is_none(),
        "unexpected source identity collision"
    );
    catalog.skill_by_id(&gem.primary_effect_id)
}
fn has_physical_link(origin: &SourceOwnedOrigin) -> bool {
    origin.links.iter().any(|link| {
        matches!(
            link,
            OwnedOriginTarget::Gem(_) | OwnedOriginTarget::Skill(_) | OwnedOriginTarget::Support(_)
        )
    })
}

#[test]
fn full_identity_catalog_normalizes_all_five_without_fabricating_missing_semantics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let reference_dir = root.join("tests/fixtures/breadth-expectations");
    let manifest: ReferenceManifest = serde_json::from_slice(&read_bounded(
        &reference_dir.join("originals-v1.json"),
        1024 * 1024,
    ))
    .unwrap();
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.cases.len(), 5);
    let artifacts = artifacts(&root);
    let policy = policy();
    let limits = NormalizationLimits::default();
    // Independently reviewed source census, not results computed by this adapter.
    let expected = [
        (62, 9, 43, 4, 3, 3, 1, 1, 1, 16),
        (174, 63, 90, 18, 3, 0, 6, 6, 6, 34),
        (62, 9, 48, 3, 2, 0, 1, 1, 1, 17),
        (62, 13, 46, 2, 1, 0, 1, 1, 1, 21),
        (181, 46, 111, 15, 9, 0, 7, 6, 6, 28),
    ];
    let mut totals = [0usize; 8];
    for (index, case) in manifest.cases.iter().enumerate() {
        assert_eq!(case.source_line, index + 1);
        let xml_path = reference_dir.join(&case.input.xml_path);
        let bytes = read_bounded(&xml_path, poe_optimizer_import::MAX_XML_BYTES);
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            case.input.xml_sha256
        );
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([index as u8 + 1; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let caller_queries = queries(case);
        let normalized = normalize_fresh(
            &evidence,
            *source.allocator_state(),
            NormalizationArtifacts {
                mappings: &artifacts.mappings,
                registry: &artifacts.registry,
                definitions: &artifacts.definitions,
                roles: &artifacts.roles,
            },
            &policy,
            &caller_queries,
            limits,
        )
        .unwrap_or_else(|error| panic!("original {}: {error}", index + 1));
        let draft = normalized.draft().input();
        let sidecar = normalized.sidecar();
        let validation = normalized.draft().validate_limits(limits.draft).unwrap();
        assert!(
            !validation.issues.is_empty(),
            "identity evidence is not complete semantics"
        );
        let issue_ids: BTreeSet<_> = validation.issues.iter().map(|issue| issue.id).collect();
        assert_eq!(sidecar.origins.len(), evidence.rows().len());
        assert_eq!(sidecar.source_sha256, case.input.xml_sha256);
        assert_eq!(sidecar.registry, artifacts.registry.identity().unwrap());
        assert_eq!(sidecar.mapping, *artifacts.mappings.identity());
        assert_eq!(sidecar.definitions, *artifacts.definitions.identity());
        assert_eq!(sidecar.skill_roles, *artifacts.roles.identity());
        assert_eq!(sidecar.allocator_before, *source.allocator_state());
        assert_eq!(sidecar.allocator_after, draft.allocator);
        let mut generated = 0;
        let mut manual_provider_only = 0;
        let mut independent_skills = 0;
        let mut independent_supports = 0;
        let mut name_only = 0;
        let mut gem_rows = 0;
        let mut config_rows = 0;
        let mut lexical_config_errors = 0;
        for (row, origin) in evidence.rows().iter().zip(&sidecar.origins) {
            assert_eq!(origin.source, row.occurrence().id());
            for link in &origin.links {
                if let OwnedOriginTarget::Issue(id) = link {
                    assert!(issue_ids.contains(id));
                }
            }
            if matches!(
                row.authored_instance(),
                Some(AuthoredInstanceId::SkillEntry(_))
            ) {
                gem_rows += 1;
                let group = evidence.row(row.occurrence().parent().unwrap()).unwrap();
                let manual = attribute(group, "source").is_none_or(str::is_empty);
                let has_gem_id = attribute(row, "gemId").is_some_and(|v| !v.is_empty());
                let primary = exact_primary(&artifacts.catalog, row);
                let provider_only =
                    manual && primary.is_some_and(|skill| skill.from_tree == Some(true));
                if !manual || !has_gem_id || provider_only {
                    generated += usize::from(!manual);
                    name_only += usize::from(manual && !has_gem_id);
                    manual_provider_only += usize::from(provider_only);
                    if provider_only {
                        println!(
                            "original-{:02}: manual_provider_only source_ordinal={} primary_skill={} from_tree=true",
                            index + 1,
                            row.occurrence().id().ordinal(),
                            primary.unwrap().id
                        );
                    }
                    assert!(
                        !has_physical_link(origin),
                        "generated/provider-only/name-only source became authored"
                    );
                    assert!(
                        origin
                            .links
                            .iter()
                            .any(|v| matches!(v, OwnedOriginTarget::Issue(_)))
                    );
                } else {
                    let primary =
                        primary.expect("physical manual row must have exact primary identity");
                    if primary.support == Some(true) {
                        independent_supports += 1;
                    } else {
                        independent_skills += 1;
                    }
                    assert_eq!(
                        origin
                            .links
                            .iter()
                            .filter(|v| matches!(v, OwnedOriginTarget::Gem(_)))
                            .count(),
                        1
                    );
                    assert_eq!(
                        origin
                            .links
                            .iter()
                            .filter(|v| matches!(
                                v,
                                OwnedOriginTarget::Skill(_) | OwnedOriginTarget::Support(_)
                            ))
                            .count(),
                        1
                    );
                }
            }
            if row.section() == SourceSectionKind::Config {
                config_rows += 1;
                lexical_config_errors += row
                    .attributes()
                    .iter()
                    .filter(|a| a.decoded().is_err())
                    .count();
                assert!(
                    origin
                        .links
                        .iter()
                        .any(|v| matches!(v, OwnedOriginTarget::Issue(_)))
                );
                assert_eq!(
                    evidence.source_fragment(origin.source).unwrap(),
                    &source.source_xml()[row.occurrence().range()]
                );
            }
        }
        assert!(config_rows > 0);
        // A typed projection can reject a source shape/value whose every lexical
        // attribute is available. Both facts and every raw row must survive.
        assert_eq!(
            serde_json::to_value(evidence.projections()).unwrap(),
            serde_json::to_value(source.projections()).unwrap()
        );
        let config_projection = evidence
            .projections()
            .iter()
            .find(|state| {
                matches!(
                    state,
                    ProjectionState::Available {
                        projection: ProjectionKind::Configuration
                    } | ProjectionState::Unavailable {
                        projection: ProjectionKind::Configuration,
                        ..
                    }
                )
            })
            .unwrap();
        assert!(matches!(
            config_projection,
            ProjectionState::Available {
                projection: ProjectionKind::Configuration,
            }
        ));
        assert_eq!(config_rows, [53, 51, 45, 46, 44][index]);
        assert_eq!(lexical_config_errors, 0);
        println!(
            "original-{:02}: configuration_projection={config_projection:?} config_rows={config_rows} lexical_attribute_errors={lexical_config_errors}",
            index + 1
        );
        let (
            rows,
            skills,
            supports,
            generated_expected,
            manual_provider_expected,
            names,
            specs,
            skill_sets,
            item_sets,
            items,
        ) = expected[index];
        assert_eq!(
            (gem_rows, generated, manual_provider_only, name_only),
            (rows, generated_expected, manual_provider_expected, names)
        );
        assert_eq!(
            (independent_skills, independent_supports),
            (skills, supports)
        );
        assert_eq!(
            (
                draft.gems.members.len(),
                draft.skills.members.len(),
                draft.supports.members.len()
            ),
            (skills + supports, skills, supports)
        );
        assert_eq!(draft.items.members.len(), items);
        assert_eq!(draft.character_presets.members.len(), specs);
        assert_eq!(draft.allocation_presets.members.len(), specs);
        assert_eq!(draft.skill_presets.members.len(), skill_sets);
        assert_eq!(draft.equipment_presets.members.len(), item_sets);
        assert_eq!(draft.choice_presets.members.len(), 1);
        assert_eq!(draft.scenario_presets.members.len(), 1);
        assert!(draft.skill_presets.members.iter().all(|preset| matches!(
            preset.skills.completion,
            DraftListCompletion::Pending { .. }
        ) && matches!(
            preset.supports.completion,
            DraftListCompletion::Pending { .. }
        )));
        for gem in &draft.gems.members {
            assert_eq!(
                artifacts
                    .roles
                    .role(known(&gem.definition))
                    .unwrap()
                    .materialization,
                OwnedGemMaterialization::Physical
            );
        }
        assert_eq!(draft.query_presets.members.len(), 1);
        let requests = &draft.query_presets.members[0].queries.requests;
        assert!(matches!(requests.completion, DraftListCompletion::Complete));
        assert_eq!(requests.members.len(), 22);
        for (request, template) in requests.members.iter().zip(&caller_queries) {
            assert_eq!(request.id, template.id);
            assert!(matches!(request.metric, DraftField::Pending(_)));
            match template.target {
                ImportQueryTarget::Player => assert!(matches!(
                    request.target,
                    DraftMetricTarget::Actor(DraftActorKey::Player)
                )),
                ImportQueryTarget::Unresolved(_) => {
                    assert!(matches!(request.target, DraftMetricTarget::Pending(_)))
                }
            }
        }
        if index == 4 {
            // One saved physical item identity, eight receiving uses in four
            // inactive/active alternatives. No eight-copy supply claim follows.
            let item_origins: Vec<_> = evidence
                .rows()
                .iter()
                .filter(|row| {
                    matches!(
                        row.authored_instance(),
                        Some(AuthoredInstanceId::ItemRecord(_))
                    ) && attribute(row, "id") == Some("26")
                })
                .collect();
            assert_eq!(item_origins.len(), 1);
            let origin = &sidecar.origins[item_origins[0].occurrence().id().ordinal() as usize];
            let items: Vec<_> = origin
                .links
                .iter()
                .filter_map(|v| match v {
                    OwnedOriginTarget::Item(id) => Some(*id),
                    _ => None,
                })
                .collect();
            assert_eq!(items.len(), 1);
            let uses: Vec<_> = draft
                .equipment
                .members
                .iter()
                .filter(
                    |use_| matches!(&use_.item, DraftField::Known { value } if *value == items[0]),
                )
                .collect();
            assert_eq!(uses.len(), 8);
            assert_eq!(
                uses.iter()
                    .map(|use_| use_.id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                8
            );
            assert_eq!(
                draft
                    .equipment_presets
                    .members
                    .iter()
                    .map(|preset| preset
                        .equipment
                        .members
                        .iter()
                        .filter(|id| uses.iter().any(|use_| use_.id == **id))
                        .count())
                    .collect::<Vec<_>>(),
                vec![2, 2, 2, 2, 0, 0]
            );
        }
        let encoded = encode_draft(normalized.draft(), limits.draft).unwrap();
        let restored = decode_draft(&encoded, limits.draft).unwrap();
        assert_eq!(restored.input(), draft);
        assert_eq!(
            restored.digest(limits.draft.input.max_wire_bytes).unwrap(),
            sidecar.draft
        );
        assert_eq!(
            read_bounded(&xml_path, poe_optimizer_import::MAX_XML_BYTES),
            bytes
        );
        for (sum, value) in totals.iter_mut().zip([
            gem_rows,
            skills + supports,
            skills,
            supports,
            generated,
            manual_provider_only,
            name_only,
            requests.members.len(),
        ]) {
            *sum += value;
        }
        println!(
            "original-{:02}: source_rows={} gems={} skill_uses={} supports={} generated_pending={} manual_provider_only_pending={} name_only_pending={} specs={} skill_sets={} item_sets={} query_rows={} config_rows={} issues={}; calculation=not_run",
            index + 1,
            gem_rows,
            skills + supports,
            skills,
            supports,
            generated,
            manual_provider_only,
            name_only,
            specs,
            skill_sets,
            item_sets,
            requests.members.len(),
            config_rows,
            validation.issues.len()
        );
    }
    assert_eq!(totals, [541, 478, 140, 338, 42, 18, 3, 110]);
    // Contrast with the valid originals: typed Boolean/Number rejection must
    // preserve lexically available Config rows and their pending obligations.
    // This is separately authored source, never a modification of a protected XML.
    let malformed_xml = r#"<PathOfBuilding2><Config><ConfigSet id="1"><Input name="caller-flag" boolean="not-canonical"/><Input name="caller-number" number="NaN"/><Placeholder name="caller-number" number="0"/></ConfigSet></Config></PathOfBuilding2>"#;
    let malformed = ImportedBuildInstance::from_decoded(
        decode_build(malformed_xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([6; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&malformed, SourceEvidenceLimits::default()).unwrap();
    let error = malformed.project_configuration().unwrap_err();
    assert_eq!(error.reason, "configuration boolean must be true or false");
    assert!(evidence.projections().iter().any(|state| matches!(state,
        ProjectionState::Unavailable { projection: ProjectionKind::Configuration, error: retained }
            if retained == &error)));
    assert!(
        evidence
            .rows()
            .iter()
            .flat_map(|row| row.attributes())
            .all(|a| a.decoded().is_ok())
    );
    let normalized = normalize_fresh(
        &evidence,
        *malformed.allocator_state(),
        NormalizationArtifacts {
            mappings: &artifacts.mappings,
            registry: &artifacts.registry,
            definitions: &artifacts.definitions,
            roles: &artifacts.roles,
        },
        &policy,
        &queries(&manifest.cases[0]),
        limits,
    )
    .unwrap();
    let config_rows: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.section() == SourceSectionKind::Config)
        .collect();
    assert_eq!(config_rows.len(), 5);
    for row in config_rows {
        let origin = &normalized.sidecar().origins[row.occurrence().id().ordinal() as usize];
        assert_eq!(origin.source, row.occurrence().id());
        assert!(
            origin
                .links
                .iter()
                .any(|v| matches!(v, OwnedOriginTarget::Issue(_)))
        );
        assert_eq!(
            evidence.source_fragment(origin.source).unwrap(),
            &malformed_xml[row.occurrence().range()]
        );
    }
    let scalars: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| matches!(row.occurrence().name(), "Input" | "Placeholder"))
        .collect();
    assert_eq!(attribute(scalars[0], "boolean"), Some("not-canonical"));
    assert_eq!(attribute(scalars[1], "number"), Some("NaN"));
    assert_eq!(attribute(scalars[2], "number"), Some("0"));
    assert!(matches!(
        normalized.draft().input().choice_presets.members[0]
            .choices
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    assert_eq!(
        normalized.draft().input().query_presets.members[0]
            .queries
            .requests
            .members
            .len(),
        22
    );
    assert!(
        !normalized
            .draft()
            .validate_limits(limits.draft)
            .unwrap()
            .issues
            .is_empty()
    );
    println!(
        "authored malformed-config contrast: projection=unavailable reason={} config_rows=5 lexical_attribute_errors=0; malformed present values retained, configuration semantics pending",
        error.reason
    );

    println!(
        "owned normalization breadth: catalog=966_gems/1436_skills physical_gems=478 skill_uses=140 supports=338 generated_pending=42 manual_provider_only_pending=18 name_only_pending=3 ordered_queries=110; definitions=identity_only legality=not_checked calculation=not_run native_completion=not_claimed"
    );
}
