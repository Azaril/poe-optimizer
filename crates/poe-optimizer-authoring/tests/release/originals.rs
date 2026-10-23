//! Unchanged originals consume the maintained release, without predecessor stages.
use super::tracked_release;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{ActorKey, MetricTarget, ProviderKey, ProviderRoot},
    owned_definitions::MetricDefId,
    owned_draft::{DraftAuthoredSkillSource, DraftFinalization, DraftMetricTarget},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        ImportActionTarget, ImportActorTarget, ImportQueryTarget, NormalizationArtifacts,
        NormalizedImport, OwnedOriginTarget, normalize_fresh,
    },
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

#[path = "originals/selection.rs"]
mod selection;

// Only source identity and ordered query identities are consumed. Recorded PoB
// values and historical implementation metadata are not native answers.
#[derive(Deserialize)]
struct Reference {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    source_line: usize,
    input: Input,
    measurements: Vec<Measurement>,
}
#[derive(Deserialize)]
struct Input {
    xml_path: String,
    xml_sha256: String,
}
#[derive(Deserialize)]
struct Measurement {
    query: Query,
}
#[derive(Deserialize)]
struct Query {
    id: String,
}
#[derive(Deserialize)]
struct MetricIdentities {
    metrics: Vec<MetricIdentity>,
}
#[derive(Deserialize)]
struct MetricIdentity {
    name: String,
    metric: MetricDefId,
    calculation_binding: String,
}
#[derive(Deserialize)]
struct SelectionEvidence {
    original: usize,
    source_sha256: String,
    occurrence_ordinal: u32,
    skill_set: String,
    main_active_skill: String,
    attributes: BTreeMap<String, String>,
    query_ids: Vec<String>,
}

fn check_target(
    normalized: &NormalizedImport,
    source: &SourceProjectEvidence<'_>,
    expected: &SelectionEvidence,
    authored: &ImportActionTarget,
    actual: &DraftMetricTarget,
) {
    let locator = &authored.provider.skill_use;
    assert_eq!(locator.source_sha256, expected.source_sha256);
    assert_eq!(locator.occurrence_ordinal, expected.occurrence_ordinal);
    let row = &source.rows()[expected.occurrence_ordinal as usize];
    assert_eq!(row.occurrence().name(), "Gem");
    for (name, value) in &expected.attributes {
        assert_eq!(row.attribute(name).unwrap().decoded().unwrap(), value);
    }
    let group = &source.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(
        group
            .attribute("mainActiveSkill")
            .unwrap()
            .decoded()
            .unwrap(),
        expected.main_active_skill,
    );
    let preset = &source.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(
        preset.attribute("id").unwrap().decoded().unwrap(),
        expected.skill_set
    );

    let origins: Vec<_> = normalized
        .sidecar()
        .origins
        .iter()
        .filter(|origin| origin.source.ordinal() == expected.occurrence_ordinal)
        .collect();
    assert_eq!(origins.len(), 1);
    let origin = origins[0];
    assert_eq!(origin.source.source_sha256(), expected.source_sha256);
    let skills: Vec<_> = origin
        .links
        .iter()
        .filter_map(|link| match link {
            OwnedOriginTarget::Skill(id) => Some(*id),
            _ => None,
        })
        .collect();
    let gems: Vec<_> = origin
        .links
        .iter()
        .filter_map(|link| match link {
            OwnedOriginTarget::Gem(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!((skills.len(), gems.len()), (1, 1));
    let (skill_id, gem_id) = (skills[0], gems[0]);
    let input = normalized.draft().input();
    let skill = input
        .skills
        .members
        .iter()
        .find(|skill| skill.id == skill_id)
        .unwrap();
    let DraftAuthoredSkillSource::Gem(source_gem) = &skill.source else {
        panic!("query must retain its existing physical skill root")
    };
    assert_eq!(source_gem.to_resolved(), Some(gem_id));
    let gem = input
        .gems
        .members
        .iter()
        .find(|gem| gem.id == gem_id)
        .unwrap();
    assert_eq!(
        gem.definition.to_resolved(),
        Some(locator.expected_gem.clone())
    );
    let Some(MetricTarget::Action(actual)) = actual.to_resolved() else {
        panic!("query action correspondence must be resolved")
    };
    assert_eq!(
        actual.action.provider,
        ProviderKey {
            root: ProviderRoot::SkillUse(skill_id),
            grant_path: authored.provider.grant_path.clone(),
        }
    );
    assert_eq!(actual.action.output, authored.output);
    assert_eq!(
        (&actual.part, &actual.mode, &actual.stat_set),
        (&authored.part, &authored.mode, &authored.stat_set)
    );
    assert_eq!(authored.part.key().as_str(), "def.0000000000000007");
    assert_eq!(authored.mode.key().as_str(), "def.0000000000000008");
    assert_eq!(authored.stat_set.key().as_str(), "def.0000000000000009");
    let path: Vec<_> = authored
        .provider
        .grant_path
        .iter()
        .map(|step| step.slot.key().as_str())
        .collect();
    match (expected.original, &authored.actor, actual.action.actor) {
        (2, ImportActorTarget::Player, ActorKey::Player) => {
            assert_eq!(locator.expected_gem.key().as_str(), "def.000000000000000a");
            assert_eq!(path, ["def.0000000000000010"]);
            assert_eq!(authored.output.slot.key().as_str(), "def.000000000000000e");
        }
        (5, ImportActorTarget::Owned { provider, slot }, ActorKey::Owned(actor)) => {
            assert_eq!(locator.expected_gem.key().as_str(), "def.0000000000000011");
            assert_eq!(provider.skill_use, *locator);
            // The current Actor supplies its attack Skill explicitly; this third
            // grant is declared in actor-ability-supply/bindings.json.
            assert_eq!(
                path,
                [
                    "def.0000000000000017",
                    "def.0000000000000020",
                    "def.0000000000003093",
                ]
            );
            assert_eq!(provider.grant_path, authored.provider.grant_path[..1]);
            assert_eq!(
                actor.provider,
                ProviderKey {
                    root: ProviderRoot::SkillUse(skill_id),
                    grant_path: provider.grant_path.clone(),
                }
            );
            assert_eq!(actor.slot, *slot);
            assert_eq!(slot.slot.key().as_str(), "def.000000000000001f");
            assert_eq!(authored.output.slot.key().as_str(), "def.0000000000000022");
        }
        _ => panic!("wrong Player/supplied Actor correspondence"),
    }
    assert_eq!(
        origin
            .links
            .iter()
            .filter(|link| matches!(link,
                OwnedOriginTarget::QueryPreset(id) if *id == input.query_presets.members[0].id
            ))
            .count(),
        1
    );
}

#[test]
fn tracked_release_preserves_five_originals_metric_identities_and_exact_action_origins() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let references = root.join("tests/fixtures/breadth-expectations");
    let reference: Reference =
        serde_json::from_slice(&fs::read(references.join("originals-v1.json")).unwrap()).unwrap();
    let selections: Vec<SelectionEvidence> = serde_json::from_slice(
        &fs::read(root.join("data/owned/poe2/3887ae68/selected-actions/selection-evidence.json"))
            .unwrap(),
    )
    .unwrap();
    let metric_identities: MetricIdentities = serde_json::from_slice(
        &fs::read(root.join("data/owned/poe2/3887ae68/requested-metrics/bindings.json")).unwrap(),
    )
    .unwrap();
    let metrics: BTreeMap<_, _> = metric_identities
        .metrics
        .iter()
        .map(|entry| {
            assert_eq!(entry.calculation_binding, "absent");
            (entry.name.as_str(), &entry.metric)
        })
        .collect();
    assert_eq!((metrics.len(), metric_identities.metrics.len()), (20, 20));
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    assert!(
        release.evaluation().is_none(),
        "known metric identities do not supply calculations"
    );
    assert_eq!(reference.cases.len(), 5);
    assert_eq!(release.query_sets().len(), 5);
    assert_eq!(selections.len(), 2);
    let mut action_checks = 0;
    let mut query_checks = 0;
    for (index, (case, query_set)) in reference.cases.iter().zip(release.query_sets()).enumerate() {
        assert_eq!(case.source_line, index + 1);
        assert_eq!(
            query_set.name.as_str(),
            format!("original-{:02}", index + 1)
        );
        let xml = fs::read(references.join(&case.input.xml_path)).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&xml)), case.input.xml_sha256);
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            BuildLineage::from_bytes([index as u8 + 1; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let normalize = || {
            normalize_fresh(
                &evidence,
                *source.allocator_state(),
                NormalizationArtifacts {
                    mappings: release.mapping(),
                    registry: recipe.registry(),
                    definitions: recipe.schema().as_ref(),
                    roles: release.roles(),
                    rewards: release.rewards(),
                    items: release.items(),
                    item_source: release.item_source(),
                    tree: release.tree(),
                },
                release.normalization(),
                &query_set.queries,
                Default::default(),
            )
            .unwrap()
        };
        let normalized = normalize();
        let replay = normalize();
        assert_eq!(
            normalized.draft(),
            replay.draft(),
            "fresh replay from the same allocator"
        );
        assert_eq!(
            serde_json::to_vec(normalized.sidecar()).unwrap(),
            serde_json::to_vec(replay.sidecar()).unwrap(),
            "exact source provenance replay"
        );
        assert!(
            !normalized
                .draft()
                .validate_limits(Default::default())
                .unwrap()
                .issues
                .is_empty(),
            "current coverage must remain incomplete for original {}",
            index + 1
        );
        let input = normalized.draft().input();
        assert_eq!(input.query_presets.members.len(), 1);
        let saved_selection = selection::saved(&evidence, &normalized);
        let proof = normalized
            .draft()
            .prove_intent(
                recipe.schema().as_ref(),
                Default::default(),
                Default::default(),
            )
            .unwrap();
        let checked = normalized
            .draft()
            .finalize_selection_checked(
                recipe.schema().as_ref(),
                saved_selection,
                &proof,
                Default::default(),
                Default::default(),
            )
            .unwrap();
        assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
        let DraftFinalization::Pending {
            selection: actual_selection,
            issues,
            queries,
            ..
        } = checked.finalization()
        else {
            panic!(
                "original {} must retain its selected input obligations",
                index + 1
            )
        };
        assert_eq!(**actual_selection, saved_selection);
        assert_eq!(queries, &input.query_presets.members[0].queries);
        assert_eq!(
            issues.len(),
            [123, 137, 127, 144, 5][index],
            "selected input obligations for original {}: {issues:#?}",
            index + 1
        );
        if index == 4 {
            let obligations: Vec<_> = issues
                .iter()
                .map(|issue| (issue.code.as_str(), issue.path.as_str()))
                .collect();
            assert_eq!(
                obligations,
                [
                    (
                        "usage-preferences-not-converted",
                        "skill_presets.members[2].intent.usage.completion",
                    ),
                    (
                        "configuration-roles-not-converted",
                        "choice_presets.members[0].choices.completion",
                    ),
                    (
                        "external-assumptions-not-converted",
                        "scenario_presets.members[0].scenario.assumptions.completion",
                    ),
                    (
                        "usage-not-converted",
                        "scenario_presets.members[0].scenario.usage.completion",
                    ),
                    (
                        "scenario-usage-intents-not-converted",
                        "scenario_presets.members[0].scenario.usage_intents.completion",
                    ),
                ],
                "Original05 keeps every actual selected input obligation"
            );
        }
        assert_eq!(normalized.draft(), replay.draft(), "selection is read-only");
        assert_eq!(
            serde_json::to_vec(normalized.sidecar()).unwrap(),
            serde_json::to_vec(replay.sidecar()).unwrap(),
            "selection preserves source provenance"
        );
        let requests = &input.query_presets.members[0].queries.requests.members;
        assert_eq!(
            (
                requests.len(),
                query_set.queries.len(),
                case.measurements.len()
            ),
            (22, 22, 22)
        );
        for (ordinal, ((actual, authored), reference)) in requests
            .iter()
            .zip(&query_set.queries)
            .zip(&case.measurements)
            .enumerate()
        {
            assert_eq!(actual.id, authored.id);
            assert_eq!(actual.id.as_str(), format!("reference-{ordinal:02}"));
            assert_eq!(
                actual.metric.to_resolved().as_ref(),
                Some(metrics[reference.query.id.as_str()])
            );
            let poe_optimizer_import::owned_mapping::ExternalSelector::Catalog { key, .. } =
                &authored.metric
            else {
                panic!("expected an explicit external metric identity")
            };
            assert_eq!(
                key,
                &poe_optimizer_import::owned_mapping::SourceComponent::Text(
                    reference.query.id.clone()
                )
            );
            query_checks += 1;
            if let Some(selection) = selections.iter().find(|selection| {
                selection.original == index + 1
                    && selection
                        .query_ids
                        .iter()
                        .any(|id| id == actual.id.as_str())
            }) {
                let ImportQueryTarget::Action(target) = &authored.target else {
                    panic!("selected action was lost")
                };
                check_target(&normalized, &evidence, selection, target, &actual.target);
                action_checks += 1;
            }
        }
    }
    assert_eq!((query_checks, action_checks), (110, 4));
}
