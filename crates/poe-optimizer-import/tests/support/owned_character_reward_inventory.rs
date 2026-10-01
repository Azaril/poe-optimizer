//! Synthetic source frames; game-source authority is independently witnessed.
use super::*;
use serde_json::Value;

const BUILD: &str = r#"<Build targetVersion="v" viewMode="IMPORT" level="73" className="Caller" ascendClassName="Other" mainSocketGroup="1" characterLevelAutoMode="false"><PlayerStat stat="not-a-reward-selector" value="99"/><MinionStat stat="caller-output" value="12"/><Buffs buffList="caller-state"/><BeastCompanion id="caller-beast"/><TimelessData devotionVariant1="1" devotionVariant2="2" searchList="" searchListFallback="" socketFilterDistance="0"/></Build>"#;
const SPEC: &str = r#"<Spec title="A" treeVersion="injected_tree" classId="1" classInternalId="1" ascendClassId="0" ascendancyInternalId="" secondaryAscendClassId="nil" nodes="1,2" masteryEffects=""><URL>caller-url</URL><Sockets><Socket nodeId="10" itemId="3"/></Sockets><Overrides><AttributeOverride strNodes="1" dexNodes="2" intNodes=""/></Overrides><WeaponSet1 nodes="1"/><WeaponSet2 nodes="2"/></Spec>"#;

fn xml(specs: &str) -> String {
    format!(
        r#"<PathOfBuilding2>{BUILD}<Tree activeSpec="1">{specs}</Tree><Config activeConfigSet="1"><ConfigSet id="1"><Input name="caller-toggle" boolean="true"/><Input name="unrelated-role" number="7"/></ConfigSet></Config></PathOfBuilding2>"#
    )
}
fn reviewed(a: &Artifacts) -> NormalizationPolicy {
    let mut p = policy();
    p.character_reward_inventory = Some(
        CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
            mapping_source: *a.mapping.source_identity(),
            target_version: "v".into(),
            tree_version: "injected_tree".into(),
        },
    );
    p
}
fn normalize(
    source: &ImportedBuildInstance,
    a: &Artifacts,
    p: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(source, Default::default()).unwrap();
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
        p,
        &[],
        limits,
    )
}
fn assert_only_character_inventory_changed(
    source: &ImportedBuildInstance,
    old: &NormalizedImport,
    next: &NormalizedImport,
) {
    let mut expected = old.draft().input().clone();
    let retired: BTreeSet<_> = expected
        .character_presets
        .members
        .iter_mut()
        .map(|row| {
            assert!(row.rewards.members.is_empty());
            let DraftListCompletion::Pending { id, code } = &row.rewards.completion else {
                panic!("old obligation")
            };
            assert_eq!(code.as_str(), "character-rewards-not-converted");
            let id = *id;
            row.rewards.completion = DraftListCompletion::Complete;
            id
        })
        .collect();
    assert_eq!(next.draft().input(), &expected);
    let mut origins = old.sidecar().origins.clone();
    for row in &mut origins {
        row.links
            .retain(|link| !matches!(link, OwnedOriginTarget::Issue(id) if retired.contains(id)));
    }
    assert_eq!(next.sidecar().origins, origins);
    assert_eq!(
        next.sidecar().allocator_after,
        old.sidecar().allocator_after
    );
    origin_integrity_with_retired(source, next, retired.len());
}

#[test]
fn character_rewards_none_preserves_wire_draft_work_and_provenance() {
    let a = artifacts(false);
    let p = policy();
    let wire = serde_json::to_value(&p).unwrap();
    assert!(wire.get("character_reward_inventory").is_none());
    let mut explicit_none = wire.clone();
    explicit_none["character_reward_inventory"] = Value::Null;
    let decoded: NormalizationPolicy = serde_json::from_value(explicit_none).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), wire);
    let source = source(&xml(SPEC), 0x71);
    let first = normalize(&source, &a, &p, Default::default()).unwrap();
    let second = normalize(&source, &a, &decoded, Default::default()).unwrap();
    assert_eq!(first.draft().input(), second.draft().input());
    assert_eq!(
        serde_json::to_value(first.sidecar()).unwrap(),
        serde_json::to_value(second.sidecar()).unwrap()
    );
}

#[test]
fn character_rewards_close_each_spec_without_erasing_config_rewards_or_other_inputs() {
    let a = with_reward_policy();
    let source = source(&xml(&format!("{SPEC}{SPEC}")), 0x72);
    let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
    let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
    assert_eq!(next.draft().input().character_presets.members.len(), 2);
    assert_eq!(next.draft().input().rewards.members.len(), 1);
    assert_eq!(
        next.draft().input().choice_presets.members[0]
            .rewards
            .members
            .len(),
        1
    );
    assert_only_character_inventory_changed(&source, &old, &next);
}

#[test]
fn cached_values_and_known_nonreward_state_do_not_supply_character_rewards() {
    let a = with_reward_policy();
    for text in [
        xml(SPEC),
        xml(SPEC).replace("value=\"99\"", "value=\"unavailable\""),
        xml(SPEC).replace("not-a-reward-selector", "caller-toggle"),
        xml(SPEC).replace("boolean=\"true\"", "boolean=\"false\""),
        xml(SPEC).replace("<Socket nodeId=\"10\" itemId=\"3\"/>", ""),
        xml(SPEC).replace("title=\"A\" ", ""),
    ] {
        let source = source(&text, 0x73);
        let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
        assert_only_character_inventory_changed(&source, &old, &next);
    }
}

#[test]
fn ambiguous_or_unreviewed_character_frames_keep_every_old_obligation() {
    let a = with_reward_policy();
    let base = xml(SPEC);
    let cases = [
        base.replace(BUILD, &format!("{BUILD}{BUILD}")),
        base.replace(
            "</PathOfBuilding2>",
            "<Tree activeSpec=\"1\"/></PathOfBuilding2>",
        ),
        base.replace("</PathOfBuilding2>", &format!("{SPEC}</PathOfBuilding2>")),
        base.replace("<Build ", "<Build extra=\"reward\" "),
        base.replace("targetVersion=\"v\"", "targetVersion=\"other\""),
        base.replace("targetVersion=\"v\"", ""),
        base.replace("<Spec ", "<Spec reward=\"caller\" "),
        base.replace("</Build>", "<Reward name=\"caller\"/></Build>"),
        base.replace("</Spec>", "<Reward name=\"caller\"/></Spec>"),
        base.replace(
            "characterLevelAutoMode=\"false\"",
            "characterLevelAutoMode=\"true\"",
        ),
        base.replace("characterLevelAutoMode=\"false\"", ""),
        base.replace("level=\"73\"", "level=\"073\""),
        base.replace("level=\"73\"", "level=\"0\""),
        base.replace("level=\"73\"", "level=\"101\""),
        base.replace("mainSocketGroup=\"1\"", "mainSkillIndex=\"1\""),
        base.replace("activeSpec=\"1\"", "activeSpec=\"2\""),
        base.replace("activeSpec=\"1\"", "activeSpec=\"01\""),
        base.replace("treeVersion=\"injected_tree\"", "treeVersion=\"other\""),
        base.replace("nodes=\"1,2\"", ""),
        base.replace("masteryEffects=\"\"", "masteryEffects=\"{1,2}\""),
        base.replace(
            "secondaryAscendClassId=\"nil\"",
            "secondaryAscendClassId=\"2\"",
        ),
        base.replace("nodeId=\"10\"", "nodeId=\"010\""),
        base.replace(
            "<Socket nodeId=\"10\" itemId=\"3\"/>",
            "<Socket nodeId=\"10\" itemId=\"3\"/><Socket nodeId=\"10\" itemId=\"4\"/>",
        ),
        base.replace("WeaponSet2", "WeaponSet3"),
        base.replace(
            "<URL>caller-url</URL>",
            "<URL>caller-url</URL><URL>second</URL>",
        ),
        base.replace("<URL>caller-url</URL>", "<URL><Reward/></URL>"),
        base.replace("<Buffs ", "<Buffs unknown=\"value\" "),
        base.replace("</Build>", "<Spectre id=\"caller\"/></Build>"),
        base.replace("<TimelessData ", "<TimelessData jewelTypeId=\"1\" "),
        base.replace("<Spec ", "<Spec xmlns=\"urn:foreign\" "),
        base.replace("<Build ", "<Build xmlns=\"urn:foreign\" "),
        base.replace(
            "</PathOfBuilding2>",
            "<x:Tree xmlns:x=\"urn:foreign\"/></PathOfBuilding2>",
        ),
        base.replace("<Tree activeSpec", "<Tree>text</Tree><Tree activeSpec"),
        base.replace("<PlayerStat ", "<PlayerStat xmlns=\"urn:foreign\" "),
        base.replace("<Overrides>", "<Overrides>unexpected"),
    ];
    for text in cases {
        let source = source(&text, 0x74);
        let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
        assert_eq!(next.draft().input(), old.draft().input(), "{text}");
        assert_eq!(next.sidecar().origins, old.sidecar().origins, "{text}");
        origin_integrity(&source, &next);
    }
}

#[test]
fn malformed_sibling_cannot_leave_a_partial_set_of_character_proofs() {
    let a = artifacts(false);
    for specs in [
        format!(
            "{SPEC}{}",
            SPEC.replace("treeVersion=\"injected_tree\"", "treeVersion=\"unknown\"")
        ),
        format!("{SPEC}<Spec/>"),
        format!("{SPEC}<Other/>"),
    ] {
        let source = source(&xml(&specs), 0x75);
        let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
        assert_eq!(next.draft().input(), old.draft().input());
        assert_eq!(next.sidecar().origins, old.sidecar().origins);
    }
}

#[test]
fn character_source_authority_and_policy_bounds_are_checked_before_normalization() {
    let a = artifacts(false);
    let source = source(&xml(SPEC), 0x76);
    for (version, stale) in [("", false), ("bad version", false), ("injected_tree", true)] {
        let mut p = reviewed(&a);
        let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
            mapping_source,
            tree_version,
            ..
        } = p.character_reward_inventory.as_mut().unwrap();
        *tree_version = version.into();
        if stale {
            *mapping_source =
                poe_optimizer_core::owned_content::digest_owned("other-source", &0, 100).unwrap();
        }
        assert!(normalize(&source, &a, &p, Default::default()).is_err());
    }
    let mut p = reviewed(&a);
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { tree_version, .. } =
        p.character_reward_inventory.as_mut().unwrap();
    *tree_version = "x".repeat(65);
    assert!(matches!(
        normalize(&source, &a, &p, Default::default()),
        Err(NormalizationError::Limit(_))
    ));
    let mut wire = serde_json::to_value(reviewed(&a)).unwrap();
    wire["character_reward_inventory"]["force_complete"] = Value::Bool(true);
    assert!(serde_json::from_value::<NormalizationPolicy>(wire).is_err());
}

#[test]
fn character_frame_scans_share_work_and_collection_limits() {
    let a = artifacts(false);
    let source = source(&xml(SPEC), 0x77);
    let mut low = 0usize;
    let mut high = 100_000usize;
    while low < high {
        let middle = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: middle,
            ..Default::default()
        };
        if normalize(&source, &a, &policy(), limits).is_ok() {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let limits = NormalizationLimits {
        max_work: high,
        ..Default::default()
    };
    normalize(&source, &a, &policy(), limits).unwrap();
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a), limits),
        Err(NormalizationError::Limit(_))
    ));
    let mut limits = NormalizationLimits::default();
    limits.draft.input.max_collection_entries = 4;
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a), limits),
        Err(NormalizationError::Limit(_))
    ));
}
