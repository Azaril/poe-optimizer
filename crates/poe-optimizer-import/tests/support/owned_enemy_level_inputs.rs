//! Finite injected configuration authority, independent of any game defaults.
use super::*;
use serde_json::{Value, json};

const PLACEHOLDER: &str = r#"<Placeholder name="target-level" number="47"/>"#;

fn reviewed(artifacts: &Artifacts) -> NormalizationPolicy {
    let mut policy = policy();
    let mut placeholder = value_recipe("finite-default-level", "target-level", false);
    placeholder.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber;
    policy.enemy_level = Some(EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 {
        mapping_source: *artifacts.mapping.source_identity(),
        absent_input_names: vec!["target-level".into(), "target-mode".into()],
        placeholder,
        expected_level: 47,
    });
    policy
}

fn xml(body: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="73"/><Config activeConfigSet="1"><ConfigSet id="1" title="A">{body}</ConfigSet></Config></PathOfBuilding2>"#
    )
}

fn normalize(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    let result = normalize_fresh(
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
        &queries(),
        limits,
    )?;
    origin_integrity(source, &result);
    Ok(result)
}

fn run(xml: &str) -> NormalizedImport {
    let artifacts = artifacts(false);
    normalize(
        &source(xml, 0x39),
        &artifacts,
        &reviewed(&artifacts),
        NormalizationLimits::default(),
    )
    .unwrap()
}

fn levels(result: &NormalizedImport) -> Vec<Option<u16>> {
    result
        .draft()
        .input()
        .scenario_presets
        .members
        .iter()
        .map(|preset| preset.scenario.enemy.level.to_resolved())
        .collect()
}

#[test]
fn omission_preserves_policy_bytes_allocations_and_pending_level() {
    let policy = policy();
    let wire = serde_json::to_value(&policy).unwrap();
    assert!(wire.get("enemy_level").is_none());
    let mut explicit_none = wire.clone();
    explicit_none["enemy_level"] = Value::Null;
    let restored: NormalizationPolicy = serde_json::from_value(explicit_none).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    let artifacts = artifacts(false);
    let source = source(&xml(PLACEHOLDER), 0x39);
    let before = normalize(&source, &artifacts, &policy, Default::default()).unwrap();
    let after = normalize(&source, &artifacts, &restored, Default::default()).unwrap();
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(
        serde_json::to_vec(before.sidecar()).unwrap(),
        serde_json::to_vec(after.sidecar()).unwrap()
    );
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(levels(&after), vec![None]);
}

#[test]
fn exact_injected_level_has_scenario_provenance_and_only_one_retired_obligation() {
    let body = format!(
        r#"{PLACEHOLDER}<Input name="other-value" number="12"/><Input name="other-flag" boolean="true"/><CustomModifierBlock title="retained" enabled="true">opaque modifiers</CustomModifierBlock>"#
    );
    let source = source(&xml(&body), 0x39);
    let artifacts = artifacts(false);
    let before = normalize(&source, &artifacts, &policy(), Default::default()).unwrap();
    let result = normalize(
        &source,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    assert_eq!(levels(&result), vec![Some(47)]);
    assert_eq!(
        before.allocator_after().last_issued(),
        result.allocator_after().last_issued() + 1
    );
    assert_eq!(
        before.sidecar().schema_version,
        result.sidecar().schema_version
    );
    assert_eq!(
        before.sidecar().source_sha256,
        result.sidecar().source_sha256
    );
    assert_eq!(
        before.sidecar().source_allocator,
        result.sidecar().source_allocator
    );
    assert_ne!(before.sidecar().policy, result.sidecar().policy);
    let expected: BTreeSet<_> = validate_draft(before.draft().input(), Default::default())
        .unwrap()
        .issues
        .into_iter()
        .filter(|issue| issue.code != key("enemy-level-not-converted"))
        .map(|issue| (issue.path, issue.code))
        .collect();
    let actual: BTreeSet<_> = validate_draft(result.draft().input(), Default::default())
        .unwrap()
        .issues
        .into_iter()
        .map(|issue| (issue.path, issue.code))
        .collect();
    assert_eq!(actual, expected);
    let preset = &result.draft().input().scenario_presets.members[0];
    assert!(matches!(
        preset.scenario.enemy.encounter,
        DraftField::Pending(_)
    ));
    assert!(matches!(
        preset.scenario.assumptions.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.scenario.usage.completion,
        DraftListCompletion::Pending { .. }
    ));
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let placeholder = evidence
        .rows()
        .iter()
        .find(|row| row.occurrence().name() == "Placeholder")
        .unwrap();
    let config = placeholder.occurrence().parent().unwrap();
    for occurrence in [placeholder.occurrence().id(), config] {
        let origin = &result.sidecar().origins[occurrence.ordinal() as usize];
        assert_eq!(origin.source, occurrence);
        assert_eq!(
            origin
                .links
                .iter()
                .filter(|link| **link == OwnedOriginTarget::ScenarioPreset(preset.id))
                .count(),
            1
        );
    }
    let old_placeholder =
        &before.sidecar().origins[placeholder.occurrence().id().ordinal() as usize];
    assert!(
        !old_placeholder
            .links
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::ScenarioPreset(_)))
    );
}

#[test]
fn scopes_are_independent_without_selecting_or_copying_another_config() {
    let xml = format!(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1">{PLACEHOLDER}</ConfigSet><ConfigSet id="2">{PLACEHOLDER}<Input name="target-mode" string="explicit override"/></ConfigSet><ConfigSet id="3"><Placeholder name="target-level" number="48"/></ConfigSet></Config></PathOfBuilding2>"#
    );
    let result = run(&xml);
    assert_eq!(levels(&result), vec![Some(47), None, None]);
    let original = source(&xml, 0x39);
    let evidence = SourceProjectEvidence::collect(&original, Default::default()).unwrap();
    let rows: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Placeholder")
        .collect();
    for (index, row) in rows.iter().enumerate() {
        let links = &result.sidecar().origins[row.occurrence().id().ordinal() as usize].links;
        assert_eq!(
            links
                .iter()
                .filter(|link| matches!(link, OwnedOriginTarget::ScenarioPreset(_)))
                .count(),
            usize::from(index == 0)
        );
    }
}

#[test]
fn override_alias_wrong_lane_and_lexical_conflicts_never_become_absence() {
    let cases = [
        "".to_owned(),
        PLACEHOLDER.replace("47", "48"),
        PLACEHOLDER.replace("47", "0"),
        PLACEHOLDER.replace("47", "-1"),
        PLACEHOLDER.replace("47", "47.0"),
        PLACEHOLDER.replace("47", "4.7e1"),
        PLACEHOLDER.replace("47", "bad"),
        PLACEHOLDER.replace("47", " 47"),
        PLACEHOLDER.replace("number=", "string="),
        PLACEHOLDER.replace("number=\"47\"", "boolean=\"true\""),
        PLACEHOLDER.replace(" number=\"47\"", ""),
        PLACEHOLDER.replace("number=\"47\"", "number=\"47\" string=\"47\""),
        format!("{PLACEHOLDER}{PLACEHOLDER}"),
        format!(r#"{PLACEHOLDER}<Input name="target-level" number="47"/>"#),
        format!(r#"{PLACEHOLDER}<Input name="target-level" number="bad"/>"#),
        format!(r#"{PLACEHOLDER}<Input name="target-mode" string="default"/>"#),
        format!(r#"{PLACEHOLDER}<Placeholder name="target-mode" string="default"/>"#),
        format!(r#"{PLACEHOLDER}<Placeholder name="target-mode" number="1"/>"#),
        format!(r#"{PLACEHOLDER}<Placeholder name="target-level" string="47"/>"#),
        PLACEHOLDER.replace("/>", r#" extra="opaque"/>"#),
        PLACEHOLDER.replace("/>", "><Nested/></Placeholder>"),
        PLACEHOLDER.replace("/>", ">text</Placeholder>"),
        PLACEHOLDER.replace("/>", r#" xmlns:q="urn:unknown"/>"#),
    ];
    for body in cases {
        let result = run(&xml(&body));
        assert_eq!(levels(&result), vec![None], "{body}");
    }
}

#[test]
fn ambiguous_containers_keys_selections_and_unknown_shape_withhold_all_scopes() {
    let scope = format!(r#"<ConfigSet id="1">{PLACEHOLDER}</ConfigSet>"#);
    let bodies = [
        format!(r#"<Config>{scope}</Config>"#),
        format!(r#"<Config activeConfigSet="2">{scope}</Config>"#),
        format!(r#"<Config activeConfigSet="01">{scope}</Config>"#),
        format!(r#"<Config activeConfigSet="1">{scope}{scope}</Config>"#),
        format!(
            r#"<Config activeConfigSet="1">{scope}<ConfigSet id="01">{PLACEHOLDER}</ConfigSet></Config>"#
        ),
        format!(
            r#"<Config activeConfigSet="1">{scope}<ConfigSet id="9007199254740992">{PLACEHOLDER}</ConfigSet></Config>"#
        ),
        format!(r#"<Config activeConfigSet="1">{scope}{PLACEHOLDER}</Config>"#),
        format!(
            r#"<Config activeConfigSet="1">{scope}</Config><Config activeConfigSet="1">{scope}</Config>"#
        ),
        format!(r#"<Config activeConfigSet="1">{scope}</Config>{scope}"#),
        format!(r#"<Config activeConfigSet="1" extra="opaque">{scope}</Config>"#),
        format!(r#"<Config activeConfigSet="1" xmlns:q="urn:unknown">{scope}</Config>"#),
        format!(
            r#"<Config activeConfigSet="1"><ConfigSet id="1" extra="opaque">{PLACEHOLDER}</ConfigSet></Config>"#
        ),
        format!(
            r#"<Config activeConfigSet="1">{scope}<ConfigSet id="2"><Unknown/></ConfigSet></Config>"#
        ),
        format!(
            r#"<Config activeConfigSet="1">{scope}<ConfigSet id="2"><Input name="other"/></ConfigSet></Config>"#
        ),
        format!(
            r#"<Config activeConfigSet="1">{scope}<ConfigSet id="2"><Input name="other" boolean="invalid"/></ConfigSet></Config>"#
        ),
        format!(
            r#"<Config activeConfigSet="1">{scope}<ConfigSet id="2"><CustomModifierBlock><Nested/></CustomModifierBlock></ConfigSet></Config>"#
        ),
    ];
    for body in bodies {
        let text = format!("<PathOfBuilding2>{body}</PathOfBuilding2>");
        let result = run(&text);
        assert!(levels(&result).iter().all(Option::is_none), "{body}");
    }
}

#[test]
fn policy_rejects_stale_bindings_and_nonfinite_or_defaulting_recipes() {
    let artifacts = artifacts(false);
    let source = source(&xml(PLACEHOLDER), 0x39);
    let good = serde_json::to_value(reviewed(&artifacts)).unwrap();
    let mut cases = Vec::new();
    for (name, value) in [
        ("expected_level", json!(0)),
        ("absent_input_names", json!([])),
        (
            "absent_input_names",
            json!(["target-level", "target-level"]),
        ),
        ("absent_input_names", json!(["target-mode"])),
        ("absent_input_names", json!([""])),
        ("absent_input_names", json!([" target-level"])),
        (
            "absent_input_names",
            json!(["target-level", "x".repeat(129)]),
        ),
        (
            "absent_input_names",
            json!((0..17).map(|n| format!("key{n}")).collect::<Vec<_>>()),
        ),
        ("mapping_source", json!("0".repeat(64))),
    ] {
        let mut changed = good.clone();
        changed["enemy_level"][name] = value;
        cases.push(changed);
    }
    for (path, value) in [
        ("/enemy_level/placeholder/tiers", json!([])),
        (
            "/enemy_level/placeholder/tiers/0/duplicates",
            json!("last_in_source_order"),
        ),
        (
            "/enemy_level/placeholder/tiers/0/selectors/0/lane",
            json!("input_number"),
        ),
        ("/enemy_level/placeholder/missing", json!({"kind":"absent"})),
        (
            "/enemy_level/placeholder/codec/codec/value/syntax",
            json!("decimal"),
        ),
        (
            "/enemy_level/placeholder/codec/namespace",
            serde_json::to_value(GameVersionNamespace::new("foreign", "v1").unwrap()).unwrap(),
        ),
    ] {
        let mut changed = good.clone();
        *changed.pointer_mut(path).expect(path) = value;
        cases.push(changed);
    }
    let mut alias = good.clone();
    alias["enemy_level"]["placeholder"]["numeric_aliases"] =
        json!([{"token":"default","replacement":"47"}]);
    cases.push(alias);
    for changed in cases {
        let policy: NormalizationPolicy = serde_json::from_value(changed.clone()).unwrap();
        assert!(
            normalize(&source, &artifacts, &policy, Default::default()).is_err(),
            "{changed}"
        );
    }
    let mut unknown = good.clone();
    unknown["enemy_level"]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
    let mut explicit_default = reviewed(&artifacts);
    let Some(EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 { placeholder, .. }) =
        &mut explicit_default.enemy_level
    else {
        panic!()
    };
    placeholder.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Integer(BoundedInteger::new(47).unwrap()),
    };
    assert!(normalize(&source, &artifacts, &explicit_default, Default::default()).is_err());
}

#[test]
fn source_byte_and_candidate_work_stays_bounded_before_proof_recording() {
    let artifacts = artifacts(false);
    let body = format!(
        r#"{PLACEHOLDER}<Input name="unrelated" string="{}"/>"#,
        "x".repeat(4096)
    );
    let source = source(&xml(&body), 0x39);
    let limits = NormalizationLimits {
        max_work: 1024,
        ..Default::default()
    };
    assert!(normalize(&source, &artifacts, &policy(), limits).is_ok());
    assert!(matches!(
        normalize(&source, &artifacts, &reviewed(&artifacts), limits),
        Err(NormalizationError::Limit("work"))
    ));
    let mut limits = NormalizationLimits::default();
    limits.value.value.max_source_bytes = 8;
    let long = super::source(&xml(&PLACEHOLDER.replace("47", &"4".repeat(64))), 0x39);
    assert!(normalize(&long, &artifacts, &reviewed(&artifacts), limits).is_err());
    let mut limits = NormalizationLimits::default();
    limits.value.max_selector_bytes = 8;
    assert!(normalize(&source, &artifacts, &reviewed(&artifacts), limits).is_err());
}
