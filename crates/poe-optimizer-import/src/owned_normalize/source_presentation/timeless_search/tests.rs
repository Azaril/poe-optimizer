//! Private candidate staging and mutable-origin controls. Public source-shape
//! and policy binding laws live in the existing normalization integration suite.
use super::*;
use crate::owned_value::{DecimalSyntax, ValueCodecInput, WhitespacePolicy};

const SOURCE: &str = r#"<PathOfBuilding2><Build targetVersion="v" viewMode="IMPORT" level="73" className="label" ascendClassName="label" mainSocketGroup="1" characterLevelAutoMode="false"><Buffs/><TimelessData devotionVariant1="1" devotionVariant2="1" searchList="" searchListFallback=""/></Build><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills><Notes/></PathOfBuilding2>"#;

fn policy(b: &Builder<'_, '_>) -> NormalizationPolicy {
    let namespace = b.roles.input().namespace.clone();
    let recipe = ValueRecipeInput {
        id: key("unused-timeless-fixture-recipe"),
        codec: ValueCodecInput {
            namespace: namespace.clone(),
            whitespace: WhitespacePolicy::Exact,
            codec: ValueCodecKind::Integer {
                syntax: DecimalSyntax::Integer,
            },
        },
        tiers: vec![],
        missing: MissingValuePolicy::Pending,
        numeric_aliases: vec![],
    };
    // These two independently source-bound adapters are compiled through their
    // real entry points. Unused scalar adapters grant no runtime authority.
    NormalizationPolicy {
        version: key("timeless-fixture"),
        namespace,
        character_level: recipe.clone(),
        gem_level: recipe.clone(),
        gem_enabled: recipe.clone(),
        group_enabled: recipe,
        manual_skill_sources: vec![],
        empty_item_keys: vec![],
        generated_support_prefixes: vec![],
        allocation_attribute: "nodes".into(),
        single_active_support_target: false,
        equipment_loadouts: vec![],
        gem_quality: GemQualityPolicy::Unconverted,
        skill_scopes: None,
        gem_inputs: None,
        direct_skill_inputs: None,
        direct_support_targets: None,
        generated_skill_inputs: None,
        gem_inventory: None,
        usage_inputs: None,
        support_origin_order: None,
        payload_inventory: None,
        skill_inventory: None,
        equipment_membership: None,
        passive_socket_membership: None,
        item_modifier_membership: None,
        item_parameter_inputs: None,
        enemy_level: None,
        configuration_reward_inventory: None,
        encounter: None,
        character_reward_inventory: Some(
            CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
                mapping_source: *b.mappings.source_identity(),
                target_version: "v".into(),
                tree_version: "v".into(),
            },
        ),
        configuration_inputs: None,
        configuration_dispositions: None,
        source_presentation: Some(SourcePresentationPolicy::PobFreshPresentationV1 {
            mapping_source: *b.mappings.source_identity(),
            empty_socket_urls: false,
            calcs_sections: false,
            tree_view: false,
            empty_notes: false,
            cached_build_buffs: false,
            empty_party: false,
            trade_search_weights: false,
            import_preferences: false,
            timeless_search_defaults: true,
        }),
    }
}

fn with_case(test: impl FnOnce(&mut Builder<'_, '_>, SourceOccurrenceId)) {
    gem_inventory::with_usage_inventory_destination(SOURCE, |b, _, _, _, _| {
        let source = b
            .evidence
            .rows()
            .iter()
            .find(|row| row.occurrence().name() == "TimelessData")
            .unwrap()
            .occurrence()
            .id();
        assert!(b.origins[source.ordinal() as usize].links.is_empty());
        assert!(matches!(
            b.origins[source.ordinal() as usize].disposition,
            SourceDisposition::Contributes
        ));
        test(b, source);
    });
}

#[test]
fn timeless_defaults_require_independent_authorization_and_do_not_scan_when_disabled() {
    with_case(|b, source| {
        let mut policy = policy(b);
        let frame = character_reward_inventory::compile(&policy, b.mappings, b.limits).unwrap();
        let compiled = super::super::compile(&policy, b.mappings, b.limits).unwrap();
        let origins = b.origins.clone();
        let work = b.work;
        super::super::apply(b, None, frame.as_ref()).unwrap();
        assert_eq!(b.work, work);
        assert!(b.fresh_build_frame.is_none());
        super::super::apply(b, compiled.as_ref(), None).unwrap();
        assert_eq!(b.work, work + 1);
        assert_eq!(b.origins, origins);
        assert!(b.fresh_build_frame.is_none());
        let SourcePresentationPolicy::PobFreshPresentationV1 {
            timeless_search_defaults,
            ..
        } = policy.source_presentation.as_mut().unwrap();
        *timeless_search_defaults = false;
        let frame = character_reward_inventory::compile(&policy, b.mappings, b.limits).unwrap();
        let compiled = super::super::compile(&policy, b.mappings, b.limits).unwrap();
        let work = b.work;
        super::super::apply(b, compiled.as_ref(), frame.as_ref()).unwrap();
        assert_eq!(b.work, work + 1);
        assert_eq!(b.origins, origins);
        assert!(b.fresh_build_frame.is_none());
        assert_eq!(source, origins[source.ordinal() as usize].source);
    });
}

#[test]
fn timeless_default_candidates_preserve_semantic_links_and_exact_source_identity() {
    for case in 0..4 {
        with_case(|b, source| {
            let policy = policy(b);
            let frame = character_reward_inventory::compile(&policy, b.mappings, b.limits).unwrap();
            let compiled = super::super::compile(&policy, b.mappings, b.limits).unwrap();
            assert_eq!(
                collect(b, frame.as_ref()).unwrap(),
                vec![(source, key("source-timeless-search-defaults"))]
            );
            match case {
                0 => {}
                1 => {
                    let owner: SkillPresetId = b.id().unwrap();
                    b.link(source, OwnedOriginTarget::SkillPreset(owner))
                        .unwrap();
                }
                2 => {
                    b.origins[source.ordinal() as usize].disposition =
                        SourceDisposition::SourceOnly(key("other-owner"))
                }
                3 => {
                    b.origins[source.ordinal() as usize].source =
                        b.evidence.rows()[0].occurrence().id()
                }
                _ => unreachable!(),
            }
            let (mut expected, allocator, issues, links) =
                (b.origins.clone(), b.allocator.state(), b.issues, b.links);
            if case == 0 {
                expected[source.ordinal() as usize].disposition =
                    SourceDisposition::SourceOnly(key("source-timeless-search-defaults"));
            }
            super::super::apply(b, compiled.as_ref(), frame.as_ref()).unwrap();
            assert_eq!(b.origins, expected, "case {case}");
            assert_eq!(b.allocator.state(), allocator);
            assert_eq!(b.issues, issues);
            assert_eq!(b.links, links);
            super::super::apply(b, compiled.as_ref(), frame.as_ref()).unwrap();
            assert_eq!(b.origins, expected, "repeat {case}");
        });
    }
}

#[test]
fn timeless_and_other_presentation_candidates_commit_only_after_full_budget_admission() {
    for cached in [false, true] {
        let mut exact = 0;
        with_case(|b, _| {
            let mut policy = policy(b);
            let SourcePresentationPolicy::PobFreshPresentationV1 { empty_notes, .. } =
                policy.source_presentation.as_mut().unwrap();
            *empty_notes = true;
            let frame = character_reward_inventory::compile(&policy, b.mappings, b.limits).unwrap();
            let compiled = super::super::compile(&policy, b.mappings, b.limits).unwrap();
            if cached {
                assert_eq!(collect(b, frame.as_ref()).unwrap().len(), 1);
            }
            let before = b.work;
            super::super::apply(b, compiled.as_ref(), frame.as_ref()).unwrap();
            exact = b.work - before;
        });
        for remaining in [exact - 1, exact] {
            with_case(|b, source| {
                let mut policy = policy(b);
                let SourcePresentationPolicy::PobFreshPresentationV1 { empty_notes, .. } =
                    policy.source_presentation.as_mut().unwrap();
                *empty_notes = true;
                let frame =
                    character_reward_inventory::compile(&policy, b.mappings, b.limits).unwrap();
                let compiled = super::super::compile(&policy, b.mappings, b.limits).unwrap();
                if cached {
                    assert_eq!(collect(b, frame.as_ref()).unwrap().len(), 1);
                }
                let (origins, allocator, issues, links) =
                    (b.origins.clone(), b.allocator.state(), b.issues, b.links);
                b.limits.max_work = b.work + remaining;
                let result = super::super::apply(b, compiled.as_ref(), frame.as_ref());
                if remaining < exact {
                    assert!(matches!(result, Err(NormalizationError::Limit("work"))));
                    assert_eq!(b.origins, origins);
                    b.limits = Default::default();
                    super::super::apply(b, compiled.as_ref(), frame.as_ref()).unwrap();
                } else {
                    result.unwrap();
                }
                let mut expected = origins;
                expected[source.ordinal() as usize].disposition =
                    SourceDisposition::SourceOnly(key("source-timeless-search-defaults"));
                let notes = b
                    .evidence
                    .rows()
                    .iter()
                    .find(|row| row.occurrence().name() == "Notes")
                    .unwrap()
                    .occurrence()
                    .id();
                expected[notes.ordinal() as usize].disposition =
                    SourceDisposition::SourceOnly(key("source-empty-notes"));
                assert_eq!(b.origins, expected);
                assert_eq!(b.allocator.state(), allocator);
                assert_eq!(b.issues, issues);
                assert_eq!(b.links, links);
            });
        }
    }
}
