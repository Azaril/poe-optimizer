//! Exact authored source correspondence, independent of action activation.
#[path = "support/production_owned_artifacts.rs"]
mod production;
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*, owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_reward_policy::OwnedRewardPolicy,
    owned_skill_catalog::*,
    owned_source::*,
};
use std::{path::PathBuf, sync::OnceLock};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn artifacts() -> &'static production::ProductionArtifacts {
    static ARTIFACTS: OnceLock<production::ProductionArtifacts> = OnceLock::new();
    ARTIFACTS
        .get_or_init(|| production::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")))
}
fn selected(role: AuthoredGemRole) -> Vec<GemDefId> {
    artifacts()
        .roles
        .input()
        .roles
        .iter()
        .filter(|row| {
            row.role == OwnedGemRole::Known(role)
                && row.materialization == OwnedGemMaterialization::Physical
        })
        .map(|row| row.gem.clone())
        .collect()
}
fn gem_xml(id: &GemDefId) -> String {
    let entry = artifacts().mappings.input().entries.iter().find(|row| {
        matches!(&row.source,ExternalSelector::Definition(ExternalOwnerSelector::Gem {..}))
            && matches!(&row.outcome,MappingOutcome::Mapped {target:SchemaSubject::Definition(DefinitionAddress::Gem(gem)),..} if gem==id)
    }).unwrap();
    let ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(game),
        variant_id: SourceComponent::Text(variant),
    }) = &entry.source
    else {
        panic!("fixture needs exact external identity")
    };
    let catalog_match = artifacts().catalog.resolve_external(game, Some(variant));
    assert_eq!(
        catalog_match.status,
        poe_optimizer_data::skill_identities::GemIdentityResolutionStatus::Exact,
        "fixture mapping must name one exact catalog Gem identity"
    );
    assert_eq!(catalog_match.candidates.len(), 1);
    assert!(!game.contains(['<', '>', '&', '"']) && !variant.contains(['<', '>', '&', '"']));
    format!(r#"<Gem gemId="{game}" variantId="{variant}" level="20" quality="0" enabled="true"/>"#)
}
fn xml() -> String {
    let gem = gem_xml(&selected(AuthoredGemRole::SkillUse)[0]);
    let support = gem_xml(&selected(AuthoredGemRole::SupportAssignment)[0]);
    format!(
        r#"<PathOfBuilding2><Build level="70"/><Skills><SkillSet id="1"><Skill enabled="true">{gem}</Skill><Skill enabled="true">{gem}</Skill><Skill enabled="true" source="Item:1">{gem}</Skill><Skill enabled="true">{support}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}
fn source(seed: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml().as_bytes()).unwrap(),
        BuildLineage::from_bytes([seed; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn locator(source: &ImportedBuildInstance, index: usize, gem: GemDefId) -> ImportSkillUseLocator {
    ImportSkillUseLocator {
        source_sha256: source.source_sha256().into(),
        occurrence_ordinal: source
            .occurrences()
            .iter()
            .filter(|row| row.name() == "Gem")
            .nth(index)
            .unwrap()
            .id()
            .ordinal(),
        expected_gem: gem,
    }
}
fn declared<S>(gem: &GemDefId, slot: S) -> DeclaredSlot<S> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(gem.clone()),
        slot,
    }
}
fn action(source: &ImportedBuildInstance, index: usize) -> ImportActionTarget {
    let gem = selected(AuthoredGemRole::SkillUse)[0].clone();
    let namespace = gem.namespace().clone();
    ImportActionTarget {
        provider: ImportProviderTarget {
            skill_use: locator(source, index, gem.clone()),
            grant_path: vec![],
        },
        actor: ImportActorTarget::Player,
        // These typed addresses intentionally have no activation/topology proof.
        // Import must preserve explicit correspondence and let Core diagnose it.
        output: declared(
            &gem,
            ActionOutputDefId::new(namespace.clone(), key("test-output")),
        ),
        part: ActionPartDefId::new(namespace.clone(), key("test-part")),
        mode: ActionModeDefId::new(namespace.clone(), key("test-mode")),
        stat_set: ActionStatSetDefId::new(namespace, key("test-stat-set")),
    }
}
fn request(id: &str, target: ImportQueryTarget) -> ImportQueryTemplate {
    // Source-to-action correspondence is independent of a mapped metric. This
    // explicitly unreviewed metric must remain Pending in the normalized draft.
    let metric = ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Metric,
        key: SourceComponent::Text("fixture-unreviewed-metric".into()),
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    };
    ImportQueryTemplate {
        id: QueryId::new(id).unwrap(),
        metric,
        target,
    }
}
fn run_with(
    source: &ImportedBuildInstance,
    queries: &[ImportQueryTemplate],
    limits: NormalizationLimits,
    mappings: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    rewards: &OwnedRewardPolicy,
) -> Result<NormalizedImport, NormalizationError> {
    let a = artifacts();
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    let result = normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            mappings,
            registry: &a.registry,
            definitions: &a.definitions,
            roles,
            rewards,
            items: &a.items,
            item_source: &a.item_source,
            tree: None,
        },
        &a.policy,
        queries,
        limits,
    )?;
    assert!(
        requests(&result)
            .iter()
            .all(|row| matches!(row.metric, DraftField::Pending(_))),
        "query correspondence must not invent a metric mapping"
    );
    Ok(result)
}
fn run(source: &ImportedBuildInstance, queries: &[ImportQueryTemplate]) -> NormalizedImport {
    let a = artifacts();
    run_with(
        source,
        queries,
        Default::default(),
        &a.mappings,
        &a.roles,
        &a.rewards,
    )
    .unwrap()
}
fn requests(result: &NormalizedImport) -> &[MetricRequestDraft] {
    &result.draft().input().query_presets.members[0]
        .queries
        .requests
        .members
}
fn resolved_action(result: &NormalizedImport, index: usize) -> ActionSelection {
    let Some(MetricTarget::Action(value)) = requests(result)[index].target.to_resolved() else {
        panic!("expected action correspondence")
    };
    *value
}
fn source_skill(result: &NormalizedImport, ordinal: u32) -> SkillUseId {
    let ids: Vec<_> = result.sidecar().origins[ordinal as usize]
        .links
        .iter()
        .filter_map(|link| {
            if let OwnedOriginTarget::Skill(id) = link {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(ids.len(), 1);
    ids[0]
}
fn query_links(result: &NormalizedImport, ordinal: u32) -> usize {
    result.sidecar().origins[ordinal as usize]
        .links
        .iter()
        .filter(|link| matches!(link, OwnedOriginTarget::QueryPreset(_)))
        .count()
}

#[test]
fn repeated_identical_gems_resolve_exact_occurrences_and_rebase_fresh_lineages() {
    let original = source(0x31);
    let first = action(&original, 0);
    let second = action(&original, 1);
    let queries = vec![
        request("first", ImportQueryTarget::Action(Box::new(first.clone()))),
        request(
            "second",
            ImportQueryTarget::Action(Box::new(second.clone())),
        ),
        request("again", ImportQueryTarget::Action(Box::new(second.clone()))),
    ];
    let a = run(&original, &queries);
    let first_id = source_skill(&a, first.provider.skill_use.occurrence_ordinal);
    let second_id = source_skill(&a, second.provider.skill_use.occurrence_ordinal);
    assert_ne!(first_id, second_id);
    assert_eq!(
        resolved_action(&a, 0).action.provider.root,
        ProviderRoot::SkillUse(first_id)
    );
    assert_eq!(
        resolved_action(&a, 1).action.provider.root,
        ProviderRoot::SkillUse(second_id)
    );
    assert_eq!(resolved_action(&a, 1), resolved_action(&a, 2));
    assert_eq!(
        query_links(&a, first.provider.skill_use.occurrence_ordinal),
        1
    );
    assert_eq!(
        query_links(&a, second.provider.skill_use.occurrence_ordinal),
        1
    );
    let fresh = source(0x32);
    assert_eq!(fresh.source_sha256(), original.source_sha256());
    let b = run(&fresh, &queries);
    let fresh_id = source_skill(&b, second.provider.skill_use.occurrence_ordinal);
    assert_ne!(fresh_id, second_id);
    assert_eq!(
        resolved_action(&b, 1).action.provider.root,
        ProviderRoot::SkillUse(fresh_id)
    );
    assert_eq!(
        requests(&a).iter().map(|row| &row.id).collect::<Vec<_>>(),
        requests(&b).iter().map(|row| &row.id).collect::<Vec<_>>()
    );
}

#[test]
fn actor_and_action_providers_resolve_independently_and_preserve_declared_paths() {
    let source = source(0x33);
    let mut target = action(&source, 1);
    let first = action(&source, 0);
    let gem = target.provider.skill_use.expected_gem.clone();
    let ns = gem.namespace().clone();
    let a = declared(&gem, GrantSlotDefId::new(ns.clone(), key("path-a")));
    let b = declared(&gem, GrantSlotDefId::new(ns.clone(), key("path-b")));
    target.provider.grant_path = vec![b.clone(), a.clone()];
    target.actor = ImportActorTarget::Owned {
        provider: Box::new(ImportProviderTarget {
            grant_path: vec![a.clone()],
            ..first.provider.clone()
        }),
        slot: declared(&gem, ActorSlotDefId::new(ns, key("test-actor"))),
    };
    let result = run(
        &source,
        &[request(
            "owned",
            ImportQueryTarget::Action(Box::new(target.clone())),
        )],
    );
    let resolved = resolved_action(&result, 0);
    assert_eq!(
        resolved.action.provider.root,
        ProviderRoot::SkillUse(source_skill(
            &result,
            target.provider.skill_use.occurrence_ordinal
        ))
    );
    assert_eq!(resolved.action.provider.grant_path, vec![b, a.clone()]);
    let ActorKey::Owned(actor) = resolved.action.actor else {
        panic!("owned actor")
    };
    assert_eq!(
        actor.provider.root,
        ProviderRoot::SkillUse(source_skill(
            &result,
            first.provider.skill_use.occurrence_ordinal
        ))
    );
    assert_eq!(actor.provider.grant_path, vec![a]);
    assert_ne!(actor.provider.root, resolved.action.provider.root);
    assert_eq!(
        query_links(&result, first.provider.skill_use.occurrence_ordinal),
        1
    );
    assert_eq!(
        query_links(&result, target.provider.skill_use.occurrence_ordinal),
        1
    );
    // An unresolved actor does not silently turn this into a player action.
    let ImportActorTarget::Owned { provider, .. } = &mut target.actor else {
        unreachable!()
    };
    provider.skill_use.source_sha256 = "a".repeat(64);
    let pending = run(
        &source,
        &[request(
            "owned",
            ImportQueryTarget::Action(Box::new(target)),
        )],
    );
    assert!(matches!(
        requests(&pending)[0].target,
        DraftMetricTarget::Pending(_)
    ));
    assert_eq!(
        query_links(&pending, first.provider.skill_use.occurrence_ordinal),
        0
    );
    assert!(
        pending.sidecar().origins.iter().all(|origin| {
            origin.source.ordinal() == 0
                || !origin
                    .links
                    .iter()
                    .any(|link| matches!(link, OwnedOriginTarget::QueryPreset(_)))
        }),
        "an unresolved actor must not leave a partial action-origin query link"
    );
}

#[test]
fn unmatched_snapshot_gem_occurrence_and_unmaterialized_origins_stay_pending() {
    let source = source(0x34);
    let base = action(&source, 0);
    let active = selected(AuthoredGemRole::SkillUse);
    let mut targets = vec![];
    let mut value = base.clone();
    value.provider.skill_use.source_sha256 = "a".repeat(64);
    targets.push(value);
    let mut value = base.clone();
    value.provider.skill_use.expected_gem = active[1].clone();
    targets.push(value);
    let mut value = base.clone();
    value.provider.skill_use.occurrence_ordinal = u32::MAX;
    targets.push(value);
    let mut value = base.clone();
    value.provider.skill_use.occurrence_ordinal = 0;
    targets.push(value);
    targets.push(action(&source, 2)); // Item-granted source group never materialized as a physical SkillUse.
    let mut value = base.clone();
    value.provider.skill_use = locator(
        &source,
        3,
        selected(AuthoredGemRole::SupportAssignment)[0].clone(),
    );
    targets.push(value);
    let queries: Vec<_> = targets
        .into_iter()
        .enumerate()
        .map(|(i, target)| {
            request(
                &format!("pending-{i}"),
                ImportQueryTarget::Action(Box::new(target)),
            )
        })
        .collect();
    let result = run(&source, &queries);
    assert_eq!(requests(&result).len(), 6);
    assert!(requests(&result).iter().all(
        |row| matches!(&row.target,DraftMetricTarget::Pending(value) if value.candidates.is_empty())
    ));
    assert!(
        result
            .sidecar()
            .origins
            .iter()
            .filter(|row| row.source.ordinal() != 0)
            .all(|row| !row
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::QueryPreset(_))))
    );
}

#[test]
fn ambiguous_source_mapping_cannot_select_the_first_gem_or_materialize_a_target() {
    let source = source(0x35);
    let target = action(&source, 0);
    let a = artifacts();
    let active = selected(AuthoredGemRole::SkillUse);
    let mut mapping = a.mappings.input().clone();
    let entry=mapping.entries.iter_mut().find(|row|matches!(&row.outcome,MappingOutcome::Mapped {target:SchemaSubject::Definition(DefinitionAddress::Gem(gem)),..} if gem==&active[0])).unwrap();
    entry.outcome = MappingOutcome::Ambiguous {
        candidates: vec![
            SchemaSubject::Definition(active[0].address()),
            SchemaSubject::Definition(active[1].address()),
        ],
        issue: key("fixture-ambiguous-gem"),
    };
    let mappings =
        OwnedMappingIndex::new(mapping, &a.registry, &a.definitions, Default::default()).unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *mappings.identity();
    let roles =
        OwnedSkillRoleIndex::new(roles, &mappings, &a.definitions, Default::default()).unwrap();
    let mut rewards = a.rewards.input().clone();
    rewards.mapping = *mappings.identity();
    let rewards =
        OwnedRewardPolicy::new(rewards, &mappings, &a.definitions, Default::default()).unwrap();
    let result = run_with(
        &source,
        &[request(
            "ambiguous",
            ImportQueryTarget::Action(Box::new(target)),
        )],
        Default::default(),
        &mappings,
        &roles,
        &rewards,
    )
    .unwrap();
    assert!(matches!(
        requests(&result)[0].target,
        DraftMetricTarget::Pending(_)
    ));
}

#[test]
fn target_configuration_rejects_invalid_hash_namespace_path_and_work_bounds() {
    let source = source(0x36);
    let base = action(&source, 0);
    let a = artifacts();
    for target in {
        let mut hash = base.clone();
        hash.provider.skill_use.source_sha256 = "not-a-hash".into();
        let mut namespace = base.clone();
        namespace.mode = ActionModeDefId::new(
            GameVersionNamespace::new("other", "v1").unwrap(),
            key("mode"),
        );
        [hash, namespace]
    } {
        assert!(
            run_with(
                &source,
                &[request(
                    "invalid",
                    ImportQueryTarget::Action(Box::new(target))
                )],
                Default::default(),
                &a.mappings,
                &a.roles,
                &a.rewards
            )
            .is_err()
        );
    }
    let mut path = base;
    let gem = path.provider.skill_use.expected_gem.clone();
    let step = declared(
        &gem,
        GrantSlotDefId::new(gem.namespace().clone(), key("step")),
    );
    path.provider.grant_path = vec![step.clone(), step];
    let query = [request(
        "bounded",
        ImportQueryTarget::Action(Box::new(path)),
    )];
    let mut limits = NormalizationLimits::default();
    limits.draft.input.max_provider_steps = 1;
    assert!(run_with(&source, &query, limits, &a.mappings, &a.roles, &a.rewards).is_err());
    let limits = NormalizationLimits {
        max_work: 1,
        ..Default::default()
    };
    assert!(run_with(&source, &query, limits, &a.mappings, &a.roles, &a.rewards).is_err());
}

#[test]
fn old_target_wire_is_unchanged_and_new_target_wire_rejects_unknown_fields() {
    assert_eq!(
        serde_json::to_string(&ImportQueryTarget::Player).unwrap(),
        r#"{"kind":"player"}"#
    );
    assert_eq!(
        serde_json::to_string(&ImportQueryTarget::Unresolved(key("waiting"))).unwrap(),
        r#"{"kind":"unresolved","value":"waiting"}"#
    );
    let target = ImportQueryTarget::Action(Box::new(action(&source(0x37), 0)));
    let value = serde_json::to_value(&target).unwrap();
    assert_eq!(
        serde_json::from_value::<ImportQueryTarget>(value.clone()).unwrap(),
        target
    );
    let mut bad = value.clone();
    bad["value"]["provider"]["skill_use"]["first_match"] = true.into();
    assert!(serde_json::from_value::<ImportQueryTarget>(bad).is_err());
    let mut bad = value;
    bad["value"]["output"]["slot"]["kind"] = "grant_slot".into();
    assert!(serde_json::from_value::<ImportQueryTarget>(bad).is_err());
}
