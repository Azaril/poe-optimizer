//! A finite injected source census closes configuration reward occurrences only.
use super::*;
use serde_json::Value;

fn reviewed(a: &Artifacts) -> NormalizationPolicy {
    let mut p = policy();
    p.configuration_reward_inventory = Some(
        ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
            mapping_source: *a.mapping.source_identity(),
            reward_policy: *a.rewards.identity(),
            controls: a
                .rewards
                .rules()
                .map(|rule| ConfigurationRewardControl {
                    recipe: rule.recipe.id.clone(),
                    selector: rule.recipe.tiers[0].selectors[0].clone(),
                })
                .collect(),
        },
    );
    p
}

fn xml(body: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
    )
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
        &queries(),
        limits,
    )
}

fn completed(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .choice_presets
        .members
        .iter()
        .map(|p| p.rewards.completion == DraftListCompletion::Complete)
        .collect()
}

fn replace_rewards(a: &mut Artifacts, input: RewardPolicyInput) {
    a.rewards = OwnedRewardPolicy::new(input, &a.mapping, &a.schema, Default::default()).unwrap();
}

#[test]
fn omission_preserves_wire_source_allocations_and_legacy_work() {
    let a = with_reward_policy();
    let p = policy();
    let wire = serde_json::to_value(&p).unwrap();
    assert!(wire.get("configuration_reward_inventory").is_none());
    let mut explicit_none = wire.clone();
    explicit_none["configuration_reward_inventory"] = Value::Null;
    let roundtrip: NormalizationPolicy = serde_json::from_value(explicit_none).unwrap();
    assert_eq!(wire, serde_json::to_value(&roundtrip).unwrap());
    let source = source(
        &xml(r#"<Input name="caller-toggle" boolean="true"/>"#),
        0x51,
    );
    let before = normalize(&source, &a, &p, Default::default()).unwrap();
    let after = normalize(&source, &a, &roundtrip, Default::default()).unwrap();
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(
        serde_json::to_vec(before.sidecar()).unwrap(),
        serde_json::to_vec(after.sidecar()).unwrap()
    );
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(completed(&after), vec![false]);
    origin_integrity(&source, &after);
}

#[test]
fn exact_output_bijection_preserves_ids_order_watermark_and_scoped_provenance() {
    let a = with_reward_policy();
    let source = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="caller-toggle" boolean="true"/><Input name="unrelated" number="12"/><CustomModifierBlock title="retained" enabled="true">not a reward</CustomModifierBlock></ConfigSet><ConfigSet id="2"><Input name="caller-toggle" boolean="false"/><Placeholder name="unrelated" number="3"/></ConfigSet><ConfigSet id="3"/></Config></PathOfBuilding2>"#,
        0x52,
    );
    let before = normalize(&source, &a, &policy(), Default::default()).unwrap();
    let after = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
    assert_eq!(completed(&after), vec![true, true, true]);
    assert_eq!(
        after
            .draft()
            .input()
            .choice_presets
            .members
            .iter()
            .map(|p| p.rewards.members.len())
            .collect::<Vec<_>>(),
        vec![1, 0, 1]
    );
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(
        before.draft().input().rewards,
        after.draft().input().rewards
    );
    let mut restored = after.draft().input().clone();
    for (actual, prior) in restored
        .choice_presets
        .members
        .iter_mut()
        .zip(&before.draft().input().choice_presets.members)
    {
        actual.rewards.completion = prior.rewards.completion.clone();
    }
    assert_eq!(restored, *before.draft().input());

    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let scopes: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "ConfigSet")
        .collect();
    let mut expected = before.sidecar().origins.clone();
    for (scope, preset) in scopes
        .iter()
        .zip(&before.draft().input().choice_presets.members)
    {
        let DraftListCompletion::Pending {
            id: reward_issue, ..
        } = preset.rewards.completion
        else {
            panic!()
        };
        let DraftListCompletion::Pending {
            id: roles_issue, ..
        } = preset.choices.completion
        else {
            panic!()
        };
        let scope_id = scope.occurrence().id();
        for id in std::iter::once(&scope_id).chain(scope.children()) {
            let origin = &mut expected[id.ordinal() as usize];
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(reward_issue));
            let row = &evidence.rows()[id.ordinal() as usize];
            if row.occurrence().name() == "Input"
                && row.attribute("name").and_then(|v| v.decoded().ok()) == Some("caller-toggle")
            {
                origin
                    .links
                    .push(OwnedOriginTarget::ChoicePreset(preset.id));
            } else if origin.links.is_empty() {
                origin.links.push(OwnedOriginTarget::Issue(roles_issue));
            }
        }
    }
    assert_eq!(
        serde_json::to_value(&expected).unwrap(),
        serde_json::to_value(&after.sidecar().origins).unwrap()
    );
    let mut sidecar = serde_json::to_value(after.sidecar()).unwrap();
    let prior = serde_json::to_value(before.sidecar()).unwrap();
    for field in ["policy", "draft", "origins"] {
        sidecar[field] = prior[field].clone();
    }
    assert_eq!(sidecar, prior);
    origin_integrity_with_retired(&source, &after, 3);
}

#[test]
fn source_aliases_unknown_shape_and_duplicates_never_gain_inventory_authority() {
    let mut a = with_reward_policy();
    for body in [
        r#"<Placeholder name="caller-toggle" string="false"/>"#,
        r#"<Input name="caller-toggle" boolean="true"/><Placeholder name="caller-toggle" number="1"/>"#,
        r#"<Input name="caller-toggle" boolean="true" string="false"/>"#,
        r#"<Input name="caller-toggle" string="true"/>"#,
        r#"<Input name="caller-toggle" boolean="invalid"/>"#,
        r#"<Input name="caller-toggle" boolean="true"><Unknown/></Input>"#,
        r#"<Input name="caller-toggle" boolean="true" extra="unknown"/>"#,
        r#"<x:Input xmlns:x="future" name="caller-toggle" boolean="true"/>"#,
        r#"<Future/><Input name="caller-toggle" boolean="true"/>"#,
        r#"<CustomModifierBlock enabled="maybe">unknown</CustomModifierBlock>"#,
    ] {
        let source = source(&xml(body), 0x53);
        let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
        assert_eq!(completed(&next), vec![false], "{body}");
        assert_eq!(old.draft().input(), next.draft().input(), "{body}");
        assert_eq!(
            serde_json::to_value(&old.sidecar().origins).unwrap(),
            serde_json::to_value(&next.sidecar().origins).unwrap(),
            "{body}"
        );
        origin_integrity(&source, &next);
    }
    let source = source(
        &xml(
            r#"<Input name="caller-toggle" boolean="false"/><Input name="caller-toggle" boolean="true"/>"#,
        ),
        0x53,
    );
    for p in [policy(), reviewed(&a)] {
        assert!(matches!(
            normalize(&source, &a, &p, Default::default()),
            Err(NormalizationError::Reward(RewardPolicyError::Value(
                ValuePolicyError::MultipleValues { tier: 0, count: 2 }
            )))
        ));
    }
    // The partial API may retain an explicitly authored last-write rule. The
    // independent inventory proof still excludes duplicate source controls.
    let mut input = a.rewards.input().clone();
    input.rules[0].recipe.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder;
    replace_rewards(&mut a, input);
    let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
    let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
    assert_eq!(completed(&next), vec![false]);
    assert_eq!(old.draft().input(), next.draft().input());
    assert_eq!(next.draft().input().rewards.members.len(), 1);
    origin_integrity(&source, &next);
}

#[test]
fn retirement_preserves_other_scoped_adapter_authority_without_roles_fallback() {
    let a = with_reward_policy();
    let mut prior_policy = policy();
    let mut placeholder = value_recipe("finite-enemy-level", "target-level", false);
    placeholder.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber;
    prior_policy.enemy_level = Some(EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 {
        mapping_source: *a.mapping.source_identity(),
        absent_input_names: vec!["target-level".into()],
        placeholder,
        expected_level: 47,
    });
    let mut next_policy = reviewed(&a);
    next_policy.enemy_level = prior_policy.enemy_level.clone();
    let source = source(
        &xml(r#"<Placeholder name="target-level" number="47"/>"#),
        0x5b,
    );
    let before = normalize(&source, &a, &prior_policy, Default::default()).unwrap();
    let after = normalize(&source, &a, &next_policy, Default::default()).unwrap();
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(
        before.draft().input().scenario_presets,
        after.draft().input().scenario_presets
    );
    assert_eq!(completed(&after), vec![true]);
    let scenario = after.draft().input().scenario_presets.members[0].id;
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let row = evidence
        .rows()
        .iter()
        .find(|row| row.occurrence().name() == "Placeholder")
        .unwrap();
    let origin = &after.sidecar().origins[row.occurrence().id().ordinal() as usize];
    assert_eq!(
        origin.links,
        vec![OwnedOriginTarget::ScenarioPreset(scenario)]
    );
    origin_integrity_with_retired(&source, &after, 1);
}

#[test]
fn whole_container_uncertainty_does_not_select_a_convenient_scope() {
    let a = with_reward_policy();
    for xml in [
        r#"<PathOfBuilding2><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="01"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/><ConfigSet id="2"><Future/></ConfigSet></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/></Config><Config activeConfigSet="2"><ConfigSet id="2"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><ConfigSet id="1"/><Config activeConfigSet="2"><ConfigSet id="2"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config><Input name="caller-toggle" boolean="true"/></Config></PathOfBuilding2>"#,
    ] {
        let source = source(xml, 0x54);
        let old = normalize(&source, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
        assert!(completed(&next).iter().all(|v| !v), "{xml}");
        assert_eq!(old.draft().input(), next.draft().input(), "{xml}");
        origin_integrity(&source, &next);
    }
}

#[test]
fn explicit_none_is_distinct_from_unknown_missing_or_partial_outcomes() {
    let mut a = with_reward_policy();
    let mut input = a.rewards.input().clone();
    input.rules[0]
        .outcomes
        .retain(|case| case.when == RewardValue::Boolean(false));
    replace_rewards(&mut a, input);
    let source = source(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"><Input name="caller-toggle" boolean="true"/></ConfigSet><ConfigSet id="2"><Input name="caller-toggle" boolean="false"/></ConfigSet><ConfigSet id="3"/></Config></PathOfBuilding2>"#,
        0x55,
    );
    let before = normalize(&source, &a, &policy(), Default::default()).unwrap();
    let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
    assert_eq!(completed(&next), vec![false, true, false]);
    assert!(next.draft().input().rewards.members.is_empty());
    assert_eq!(before.allocator_after(), next.allocator_after());
    origin_integrity_with_retired(&source, &next, 1);
    let mut input = a.rewards.input().clone();
    input.rules[0].recipe.missing = MissingValuePolicy::Pending;
    replace_rewards(&mut a, input);
    let source = super::source(&xml(""), 0x56);
    let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
    assert_eq!(completed(&next), vec![false]);
}

#[test]
fn partial_reward_parameter_schema_withholds_only_its_unknown_outcome() {
    let mut a = with_reward_policy();
    let mut schema = a.schema.input().clone();
    let entry = schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Reward(entry) => Some(entry),
            _ => None,
        })
        .unwrap();
    let target = subject(&entry.id);
    let SchemaState::Known(known) = &mut entry.schema else {
        panic!()
    };
    known.declarations.parameters.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: target,
            facet: SchemaFacet::InputSchema,
            code: key("uncertain-reward-inputs"),
        }],
    };
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = a.mapping.input().clone();
    mapping.definitions = a.schema.identity().clone();
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, Default::default()).unwrap();
    let mut input = a.rewards.input().clone();
    input.mapping = *a.mapping.identity();
    input.definitions = a.schema.identity().clone();
    replace_rewards(&mut a, input);
    for (value, expected) in [("true", false), ("false", true)] {
        let source = source(
            &xml(&format!(
                r#"<Input name="caller-toggle" boolean="{value}"/>"#
            )),
            0x57,
        );
        let next = normalize(&source, &a, &reviewed(&a), Default::default()).unwrap();
        assert_eq!(completed(&next), vec![expected]);
        assert!(next.draft().input().rewards.members.is_empty());
    }
}

#[test]
fn census_bindings_are_exact_and_partial_rule_api_remains_available() {
    let mut a = with_reward_policy();
    let source = source(&xml(""), 0x58);
    let p = reviewed(&a);
    for mutation in 0..6 {
        let mut bad = p.clone();
        let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
            mapping_source,
            reward_policy,
            controls,
        }) = &mut bad.configuration_reward_inventory
        else {
            panic!()
        };
        match mutation {
            0 => *mapping_source = *a.rewards.identity(),
            1 => *reward_policy = *a.mapping.identity(),
            2 => controls.clear(),
            3 => controls.push(controls[0].clone()),
            4 => controls[0].recipe = key("not-a-reward-recipe"),
            5 => controls[0].selector.name = "unknown-control".into(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                normalize(&source, &a, &bad, Default::default()),
                Err(NormalizationError::Binding | NormalizationError::Policy(_))
            ),
            "{mutation}"
        );
    }
    let mut input = a.rewards.input().clone();
    let mut second = input.rules[0].recipe.tiers[0].clone();
    second.selectors[0].lane = ValueLane::InputString;
    input.rules[0].recipe.tiers.push(second);
    replace_rewards(&mut a, input);
    assert!(normalize(&source, &a, &policy(), Default::default()).is_ok());
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a), Default::default()),
        Err(NormalizationError::Policy(
            "configuration reward rule selector"
        ))
    ));
}

#[test]
fn recipe_order_not_census_order_controls_occurrence_bijection() {
    let mut a = with_reward_policy();
    let mut input = a.rewards.input().clone();
    let mut second = input.rules[0].clone();
    second.recipe.id = key("second-recipe");
    second.recipe.tiers[0].selectors[0].name = "second-toggle".into();
    input.rules.push(second);
    replace_rewards(&mut a, input);
    let mut p = reviewed(&a);
    let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { controls, .. }) =
        &mut p.configuration_reward_inventory
    else {
        panic!()
    };
    controls.reverse();
    let source = source(&xml(""), 0x59);
    let before = normalize(&source, &a, &policy(), Default::default()).unwrap();
    let next = normalize(&source, &a, &p, Default::default()).unwrap();
    assert_eq!(completed(&next), vec![true]);
    assert_eq!(before.draft().input().rewards, next.draft().input().rewards);
    assert_eq!(next.draft().input().rewards.members.len(), 2);
    assert_ne!(
        next.draft().input().rewards.members[0].id,
        next.draft().input().rewards.members[1].id
    );
    assert_eq!(before.allocator_after(), next.allocator_after());
    origin_integrity_with_retired(&source, &next, 1);
}

#[test]
fn policy_and_source_walks_obey_tightened_resource_bounds() {
    let a = with_reward_policy();
    let source = source(
        &xml(&format!(
            r#"<CustomModifierBlock>{}</CustomModifierBlock>"#,
            "x".repeat(4096)
        )),
        0x5a,
    );
    let limits = NormalizationLimits {
        max_work: 1024,
        ..Default::default()
    };
    assert!(normalize(&source, &a, &policy(), limits).is_ok());
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a), limits),
        Err(NormalizationError::Limit(_))
    ));
    let mut limits = NormalizationLimits::default();
    limits.value.max_total_selector_bytes = 1;
    assert!(normalize(&source, &a, &reviewed(&a), limits).is_err());
    let mut p = reviewed(&a);
    let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { controls, .. }) =
        &mut p.configuration_reward_inventory
    else {
        panic!()
    };
    controls.resize(257, controls[0].clone());
    assert!(matches!(
        normalize(&source, &a, &p, Default::default()),
        Err(NormalizationError::Policy(
            "configuration reward control census"
        ))
    ));
}
