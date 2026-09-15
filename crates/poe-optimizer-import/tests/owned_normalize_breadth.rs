//! Offline breadth regression: pinned identity data -> owned artifacts -> drafts.
//! The fixed reference manifest supplies ordered query identities only. Its
//! numerical results, source programs and UI selections are never evaluated.
#[path = "support/production_owned_artifacts.rs"]
mod production;

use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{LoadoutScope, QueryId},
    owned_definitions::OwnedDefinitionKey,
    owned_draft::*,
};
use poe_optimizer_data::skill_identities::{SkillIdentity, SkillIdentityCatalog};
use poe_optimizer_import::{
    build_instance::{
        AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits, ProjectionKind,
        ProjectionState,
    },
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_reference_projection::*,
    owned_skill_catalog::OwnedGemMaterialization,
    owned_source::*,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs::File, io::Read, path::Path};

#[path = "support/owned_reference_fixture.rs"]
mod reference_fixture;

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
// Intentional test-only projections of existing artifacts. Unrelated package
// sections/reference values are skipped by serde, rather than loaded as rules.
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

// Source enumeration, not selected-set pairing, determines receiving ownership.
fn assert_spec_socket_membership(
    evidence: &SourceProjectEvidence<'_>,
    normalized: &NormalizedImport,
    expected_counts: &[usize],
) {
    let input = normalized.draft().input();
    let origins = &normalized.sidecar().origins;
    let specs: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            matches!(
                row.authored_instance(),
                Some(AuthoredInstanceId::PassiveSpec(_))
            )
        })
        .collect();
    assert_eq!(specs.len(), expected_counts.len());
    let ordinary: BTreeSet<_> = input
        .equipment_presets
        .members
        .iter()
        .flat_map(|preset| preset.equipment.members.iter().copied())
        .collect();
    let mut all_spec_uses = BTreeSet::new();
    for (spec, expected_count) in specs.iter().zip(expected_counts) {
        let preset_id = origins[spec.occurrence().id().ordinal() as usize]
            .links
            .iter()
            .find_map(|link| match link {
                OwnedOriginTarget::AllocationPreset(id) => Some(*id),
                _ => None,
            })
            .unwrap();
        let preset = input
            .allocation_presets
            .members
            .iter()
            .find(|preset| preset.id == preset_id)
            .unwrap();
        let source_sockets: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| {
                if row.occurrence().name() != "Socket" {
                    return false;
                }
                let Some(parent) = row.occurrence().parent() else {
                    return false;
                };
                let container = evidence.row(parent).unwrap();
                container.occurrence().name() == "Sockets"
                    && container.occurrence().parent() == Some(spec.occurrence().id())
            })
            .collect();
        assert_eq!(source_sockets.len(), *expected_count);
        let mut expected_uses = vec![];
        for socket in source_sockets {
            assert_ne!(attribute(socket, "itemId"), Some("0"));
            let use_ids: Vec<_> = origins[socket.occurrence().id().ordinal() as usize]
                .links
                .iter()
                .filter_map(|link| match link {
                    OwnedOriginTarget::Equipment(id) => Some(*id),
                    _ => None,
                })
                .collect();
            assert_eq!(use_ids.len(), 1);
            let id = use_ids[0];
            assert!(
                all_spec_uses.insert(id),
                "one source receiving use leaked across Specs"
            );
            assert!(
                !ordinary.contains(&id),
                "Spec socket was mixed into an ItemSet"
            );
            let receiving = input
                .equipment
                .members
                .iter()
                .find(|row| row.id == id)
                .unwrap();
            assert!(matches!(receiving.item, DraftField::Known { .. }));
            assert!(matches!(
                receiving.destination,
                DraftEquipmentDestination::Pending(_)
            ));
            assert!(matches!(receiving.scope, DraftField::Pending(_)));
            expected_uses.push(id);
        }
        assert_eq!(preset.equipment.members, expected_uses);
        let DraftListCompletion::Pending { code, .. } = &preset.equipment.completion else {
            panic!("enumerated source sockets do not certify complete equipment membership");
        };
        assert_eq!(
            code.as_str(),
            "allocation-equipment-membership-not-converted"
        );
    }
    assert_eq!(all_spec_uses.len(), expected_counts.iter().sum::<usize>());
    assert!(
        input
            .allocations
            .members
            .iter()
            .all(|allocation| matches!(allocation.pool, DraftField::Pending(_)))
    );
    // Missing itemId is not the injected exact empty-item token, even for a
    // source RuneSlot whose display name says None.
    for row in evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "RuneSlot")
    {
        assert!(row.attribute("itemId").is_none());
        let origin = &origins[row.occurrence().id().ordinal() as usize];
        assert!(matches!(origin.disposition, SourceDisposition::Contributes));
        let id = origin
            .links
            .iter()
            .find_map(|link| match link {
                OwnedOriginTarget::Equipment(id) => Some(*id),
                _ => None,
            })
            .expect("unknown rune source must retain a receiving occurrence");
        assert!(ordinary.contains(&id));
        assert!(!all_spec_uses.contains(&id));
        let receiving = input
            .equipment
            .members
            .iter()
            .find(|row| row.id == id)
            .unwrap();
        assert!(matches!(receiving.item, DraftField::Pending(_)));
        assert!(matches!(
            receiving.destination,
            DraftEquipmentDestination::Pending(_)
        ));
    }
}

fn assert_observed_loadouts(
    evidence: &SourceProjectEvidence<'_>,
    normalized: &NormalizedImport,
    expected: [usize; 3],
) {
    use std::collections::BTreeMap;
    let draft = normalized.draft().input();
    let loadouts: BTreeMap<_, _> = normalized
        .sidecar()
        .origins
        .iter()
        .flat_map(|o| &o.links)
        .filter_map(|link| match link {
            OwnedOriginTarget::WeaponLoadout { key, id } => Some((key.as_str(), *id)),
            _ => None,
        })
        .collect();
    assert_eq!(loadouts.len(), 2);
    assert_eq!(draft.weapon_loadouts.members.len(), 2);
    assert!(matches!(
        draft.weapon_loadouts.completion,
        DraftListCompletion::Pending { .. }
    ));
    let receiving: BTreeMap<_, _> = draft
        .equipment
        .members
        .iter()
        .map(|row| (row.id, row))
        .collect();
    let mut counts = [0usize; 3];
    for row in evidence.rows() {
        let Some(parent) = row.occurrence().parent() else {
            continue;
        };
        if row.occurrence().name() != "Slot"
            || evidence.row(parent).unwrap().occurrence().name() != "ItemSet"
        {
            continue;
        }
        let origin = &normalized.sidecar().origins[row.occurrence().id().ordinal() as usize];
        let source_name = attribute(row, "name").unwrap();
        let index = match source_name {
            "Weapon 1" | "Weapon 2" => 0,
            "Weapon 1 Swap" | "Weapon 2 Swap" => 1,
            _ => 2,
        };
        if index < 2 {
            assert!(
                origin
                    .links
                    .iter()
                    .any(|link| matches!(link, OwnedOriginTarget::WeaponLoadout { .. })),
                "empty slots still prove observed loadouts"
            );
        }
        if attribute(row, "itemId") == Some("0") {
            continue;
        }
        let id = origin
            .links
            .iter()
            .find_map(|link| match link {
                OwnedOriginTarget::Equipment(id) => Some(id),
                _ => None,
            })
            .unwrap();
        let scope = receiving[id].scope.to_resolved().unwrap();
        let expected_scope = if index == 2 {
            LoadoutScope::Shared
        } else {
            LoadoutScope::Selected {
                loadouts: vec![
                    loadouts[if index == 0 {
                        "weapon-set-one"
                    } else {
                        "weapon-set-two"
                    }],
                ],
            }
        };
        assert_eq!(scope, expected_scope);
        counts[index] += 1;
    }
    assert_eq!(counts, expected);
    // Explicit caller selections exercise the formerly missing loadout lookup.
    // These are not inferred source active selections or complete build claims.
    for loadout in loadouts.values() {
        let selection = EvaluationSelection {
            build: poe_optimizer_core::owned_project::VariantSelection {
                character: draft.character_presets.members[0].id,
                equipment: draft.equipment_presets.members[0].id,
                allocations: draft.allocation_presets.members[0].id,
                skills: draft.skill_presets.members[0].id,
                choices: draft.choice_presets.members[0].id,
                active_weapon_loadout: *loadout,
            },
            scenario: draft.scenario_presets.members[0].id,
            queries: draft.query_presets.members[0].id,
        };
        let finalization = normalized
            .draft()
            .finalize_selection(selection, DraftLimits::default())
            .unwrap();
        let DraftFinalization::Pending {
            issues, queries, ..
        } = finalization
        else {
            panic!("source scope admission cannot complete missing semantic coverage");
        };
        assert!(!issues.is_empty());
        assert_eq!(queries.requests.members.len(), 22);
    }
    println!(
        "observed owned loadouts=2; mapped receiving scopes first/second/shared={counts:?}; active selection not inferred; membership pending"
    );
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
    let artifacts = production::load(&root);
    let policy = artifacts.policy.clone();
    let GemQualityPolicy::Attributes(quality_policy) = &policy.gem_quality else {
        panic!("production quality conversion missing")
    };
    let expected_quality = &quality_policy.kinds[0].kind;
    let poe_optimizer_import::owned_value::ValueCodecKind::Quantity {
        unit: quality_unit, ..
    } = &quality_policy.amount.codec.codec
    else {
        panic!("production quality quantity codec missing")
    };
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
                items: &artifacts.items,
                item_source: &artifacts.item_source,
                mappings: &artifacts.mappings,
                registry: &artifacts.registry,
                definitions: &artifacts.definitions,
                roles: &artifacts.roles,
                rewards: &artifacts.rewards,
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
        assert_eq!(sidecar.schema_version, 8);
        assert_observed_loadouts(
            &evidence,
            &normalized,
            [[2, 0, 11], [6, 9, 43], [1, 0, 13], [1, 1, 13], [11, 0, 58]][index],
        );
        let socket_counts: &[usize] = match index {
            0 => &[3],
            1 => &[0, 0, 0, 0, 0, 3],
            2 => &[3],
            3 => &[6],
            4 => &[0, 0, 0, 0, 3, 3, 0],
            _ => unreachable!(),
        };
        assert_spec_socket_membership(&evidence, &normalized, socket_counts);
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
        let mut quality_zeros = 0;
        let gem_by_id: std::collections::BTreeMap<_, _> =
            draft.gems.members.iter().map(|gem| (gem.id, gem)).collect();
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
                    let gem_id = origin
                        .links
                        .iter()
                        .find_map(|link| match link {
                            OwnedOriginTarget::Gem(id) => Some(id),
                            _ => None,
                        })
                        .unwrap();
                    let Some(Some(quality)) = gem_by_id[gem_id].quality.to_resolved() else {
                        panic!("explicit physical-gem quality must be known");
                    };
                    assert_eq!(&quality.kind, expected_quality);
                    assert_eq!(quality.amount.unit(), quality_unit);
                    let source_quality: f64 = attribute(row, "quality").unwrap().parse().unwrap();
                    assert_eq!(quality.amount.value(), source_quality);
                    assert!(row.attribute("qualityId").is_none());
                    quality_zeros += usize::from(source_quality == 0.0);
                    assert!(matches!(
                        gem_by_id[gem_id].parameters.completion,
                        DraftListCompletion::Pending { .. }
                    ));

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
        assert_eq!(quality_zeros, [50, 146, 51, 52, 149][index]);
        println!(
            "original-{:02}: known physical gem qualities={} explicit_zero={} input-schema/rule coverage remains unresolved",
            index + 1,
            draft.gems.members.len(),
            quality_zeros
        );
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
        let choices = &draft.choice_presets.members[0];
        assert_eq!(choices.rewards.members.len(), [16, 17, 15, 16, 17][index]);
        assert_eq!(draft.rewards.members.len(), choices.rewards.members.len());
        assert_eq!(sidecar.reward_policy, *artifacts.rewards.identity());
        let DraftListCompletion::Pending { code, .. } = &choices.rewards.completion else {
            panic!("unconverted configuration rewards must remain pending");
        };
        assert_eq!(code.as_str(), "configuration-rewards-not-converted");
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
            items: &artifacts.items,
            item_source: &artifacts.item_source,
            mappings: &artifacts.mappings,
            registry: &artifacts.registry,
            definitions: &artifacts.definitions,
            roles: &artifacts.roles,
            rewards: &artifacts.rewards,
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
        "owned normalization breadth: catalog=966_gems/1436_skills physical_gems=478 skill_uses=140 supports=338 generated_pending=42 manual_provider_only_pending=18 name_only_pending=3 ordered_queries=110; definitions=production_components_partial legality=not_checked calculation=not_run native_completion=not_claimed"
    );
}

#[test]
fn original_reference_projection_binds_fresh_owned_drafts_without_losing_rows() {
    use poe_optimizer_core::metrics::{ActorScope, MetricQuery};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let reference_dir = root.join("tests/fixtures/breadth-expectations");
    let manifest: ReferenceManifest = serde_json::from_slice(&read_bounded(
        &reference_dir.join("originals-v1.json"),
        1024 * 1024,
    ))
    .unwrap();
    let artifacts = production::load(&root);
    let limits = NormalizationLimits::default();
    let projection_limits = ProjectionLimits::default();
    let mut totals = [0usize; 2];
    for fixture in reference_fixture::load_references() {
        let case = &manifest.cases[fixture.source_line - 1];
        assert_eq!(fixture.source_xml_sha256, case.input.xml_sha256);
        let bytes = read_bounded(
            &reference_dir.join(&case.input.xml_path),
            poe_optimizer_import::MAX_XML_BYTES,
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            case.input.xml_sha256
        );
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([fixture.source_line as u8 + 20; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let reference = RecordedReference::from_cli_report(
            &fixture.bytes,
            case.input.xml_sha256.parse().unwrap(),
            projection_limits,
        )
        .unwrap();
        let templates = queries(case);
        let reference_queries: Vec<_> = case
            .measurements
            .iter()
            .map(|m| MetricQuery {
                actor: match m.query.actor.as_str() {
                    "player" => ActorScope::Player,
                    "selected_minion" => ActorScope::SelectedMinion,
                    _ => panic!("unreviewed actor"),
                },
                id: m.query.id.clone(),
            })
            .collect();
        // Filter templates before fresh normalization. Do not revise a normalized
        // draft behind its digest/allocator boundary to remove unavailable rows.
        let selected: Vec<_> = templates
            .iter()
            .zip(&reference_queries)
            .filter_map(|(t, q)| {
                reference
                    .requires_evaluation(q)
                    .unwrap()
                    .then_some(t.clone())
            })
            .collect();
        let normalized = normalize_fresh(
            &evidence,
            *source.allocator_state(),
            NormalizationArtifacts {
                items: &artifacts.items,
                item_source: &artifacts.item_source,
                mappings: &artifacts.mappings,
                registry: &artifacts.registry,
                definitions: &artifacts.definitions,
                roles: &artifacts.roles,
                rewards: &artifacts.rewards,
            },
            &artifacts.policy,
            &selected,
            limits,
        )
        .unwrap();
        let preset = &normalized.draft().input().query_presets.members[0];
        let sidecar = normalized.sidecar();
        let policy_binding = ProjectionPolicyBinding {
            version: key("reviewed-original-query-routing-v1"),
            game_version: artifacts.policy.namespace.clone(),
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
        let rows = templates
            .iter()
            .zip(reference_queries)
            .map(|(t, reference)| ProjectionRowInput {
                id: t.id.clone(),
                reference,
                request: preset
                    .queries
                    .requests
                    .members
                    .iter()
                    .find(|r| r.id == t.id)
                    .cloned(),
            })
            .collect();
        let plan =
            ProjectionPlan::new(&reference, policy_binding, rows, projection_limits).unwrap();
        let binding = plan
            .validate_normalized(&normalized, preset.id, projection_limits)
            .unwrap();
        assert_eq!(binding.draft, sidecar.draft);
        assert_eq!(plan.query_draft(), preset.queries);
        let mut ids: Vec<_> = preset
            .queries
            .requests
            .members
            .iter()
            .map(|r| r.id.clone())
            .collect();
        ids.reverse(); // Join by identity, never assume evaluator order.
        let joined = plan.join_ids(&ids, projection_limits).unwrap();
        assert_eq!(joined.rows.len(), 22);
        for (row, t) in joined.rows.iter().zip(&templates) {
            assert_eq!(row.id, t.id);
            match row.association {
                JoinedAssociation::Evaluate { result_index } => {
                    assert_eq!(ids[result_index], row.id);
                    totals[0] += 1;
                }
                JoinedAssociation::ReferenceKnownUnavailable { .. } => {
                    assert!(!ids.contains(&row.id));
                    totals[1] += 1;
                }
            }
        }
        // This is structural routing. Definitions, selected loadout and other
        // mechanics remain pending; no native measurement or parity is certified.
        assert!(
            !normalized
                .draft()
                .validate_limits(limits.draft)
                .unwrap()
                .issues
                .is_empty()
        );
    }
    assert_eq!(totals, [104, 6]);
}
