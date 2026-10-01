//! Source-bound encounter identity without enemy-level or numerical authority.
use super::*;
use serde_json::Value;

fn selector() -> ExternalSelector {
    ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Encounter,
        key: SourceComponent::Text("reviewed-default".into()),
        version: SourceComponent::Text("fixture-source".into()),
        variant: SourceComponent::Text("finite-default-selectors".into()),
    }
}

fn rebind(a: &mut Artifacts, schema: SchemaPackageInput, mut mapping: MappingPackageInput) {
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, Default::default()).unwrap();
    a.rewards = empty_rewards(&a.mapping, &a.schema);
}

fn fixture() -> (Artifacts, EncounterDefId) {
    let mut a = artifacts(false);
    let encounter = a
        .registry
        .allocate_definition::<EncounterDefinition>()
        .unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Encounter(DefinitionEntry {
            id: encounter.clone(),
            schema: SchemaState::Known(EncounterSchema {
                enemy_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(90).unwrap(),
                },
                external_inputs: DeclaredSet::partial(
                    vec![],
                    vec![SchemaGap {
                        subject: subject(&encounter),
                        facet: SchemaFacet::StaticLinks,
                        code: key("external-inputs-still-unconverted"),
                    }],
                ),
            }),
        }));
    let mut mapping = a.mapping.input().clone();
    mapping.entries.push(MappingEntry {
        source: selector(),
        outcome: MappingOutcome::Mapped {
            target: subject(&encounter),
            basis: MappingBasis::Exact,
        },
    });
    rebind(&mut a, schema, mapping);
    (a, encounter)
}

fn reviewed(a: &Artifacts, target: &EncounterDefId) -> NormalizationPolicy {
    let mut p = policy();
    p.encounter = Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        mapping_source: *a.mapping.source_identity(),
        selector: selector(),
        target: target.clone(),
        absent_input_names: vec![
            "branch-kind".into(),
            "boss-skill".into(),
            "size-choice".into(),
        ],
    });
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
    policy: &NormalizationPolicy,
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
        policy,
        &queries(),
        limits,
    )
}

fn encounters(result: &NormalizedImport) -> Vec<Option<EncounterDefId>> {
    result
        .draft()
        .input()
        .scenario_presets
        .members
        .iter()
        .map(|p| p.scenario.enemy.encounter.to_resolved())
        .collect()
}

#[test]
fn omitted_encounter_policy_preserves_bytes_allocations_and_pending_identity() {
    let (a, _) = fixture();
    let p = policy();
    let wire = serde_json::to_value(&p).unwrap();
    assert!(wire.get("encounter").is_none());
    let mut nullable = wire.clone();
    nullable["encounter"] = Value::Null;
    let restored: NormalizationPolicy = serde_json::from_value(nullable).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    let source = source(&xml(""), 0x61);
    let before = normalize(&source, &a, &p, Default::default()).unwrap();
    let after = normalize(&source, &a, &restored, Default::default()).unwrap();
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(
        serde_json::to_vec(before.sidecar()).unwrap(),
        serde_json::to_vec(after.sidecar()).unwrap()
    );
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(encounters(&after), vec![None]);
    origin_integrity(&source, &after);
}

#[test]
fn exact_default_identity_retires_only_its_issue_and_keeps_scoped_authority() {
    let (a, target) = fixture();
    let source = source(
        &xml(
            r#"<Input name="other" number="23"/><CustomModifierBlock title="unconverted" enabled="true">opaque</CustomModifierBlock>"#,
        ),
        0x62,
    );
    let before = normalize(&source, &a, &policy(), Default::default()).unwrap();
    let after = normalize(&source, &a, &reviewed(&a, &target), Default::default()).unwrap();
    assert_eq!(encounters(&after), vec![Some(target.clone())]);
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut restored = after.draft().input().clone();
    let mut retired = BTreeSet::new();
    for (next, old) in restored
        .scenario_presets
        .members
        .iter_mut()
        .zip(&before.draft().input().scenario_presets.members)
    {
        let DraftField::Pending(pending) = &old.scenario.enemy.encounter else {
            panic!()
        };
        retired.insert(pending.id);
        next.scenario.enemy.encounter = old.scenario.enemy.encounter.clone();
    }
    assert_eq!(restored, *before.draft().input());
    let mut origins = before.sidecar().origins.clone();
    for origin in &mut origins {
        origin
            .links
            .retain(|link| !matches!(link, OwnedOriginTarget::Issue(id) if retired.contains(id)));
    }
    assert_eq!(
        serde_json::to_value(&origins).unwrap(),
        serde_json::to_value(&after.sidecar().origins).unwrap()
    );
    let mut sidecar = serde_json::to_value(after.sidecar()).unwrap();
    let prior = serde_json::to_value(before.sidecar()).unwrap();
    for field in ["draft", "policy", "origins"] {
        sidecar[field] = prior[field].clone();
    }
    assert_eq!(sidecar, prior);
    let SchemaLookup::Known(schema) = a.schema.definition(&target) else {
        panic!()
    };
    assert!(!schema.external_inputs.is_complete());
    assert_eq!(schema.enemy_level.minimum.get(), 1);
    assert_eq!(schema.enemy_level.maximum.get(), 90);
    assert!(matches!(
        after.draft().input().scenario_presets.members[0]
            .scenario
            .enemy
            .level,
        DraftField::Pending(_)
    ));
    origin_integrity_with_retired(&source, &after, 1);
}

#[test]
fn encounter_and_level_proofs_are_independent() {
    let (a, target) = fixture();
    let mut p = reviewed(&a, &target);
    let mut placeholder = value_recipe("independent-level", "target-level", false);
    placeholder.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber;
    p.enemy_level = Some(EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 {
        mapping_source: *a.mapping.source_identity(),
        absent_input_names: vec!["target-level".into()],
        placeholder,
        expected_level: 47,
    });
    for (body, known_encounter, level) in [
        ("", true, None),
        (r#"<Input name="target-level" number="20"/>"#, true, None),
        (
            r#"<Placeholder name="target-level" number="47"/>"#,
            true,
            Some(47),
        ),
        (
            r#"<Placeholder name="target-level" number="47"/><Input name="branch-kind" string="explicit"/>"#,
            false,
            Some(47),
        ),
    ] {
        let source = source(&xml(body), 0x63);
        let result = normalize(&source, &a, &p, Default::default()).unwrap();
        assert_eq!(
            encounters(&result),
            vec![known_encounter.then_some(target.clone())],
            "{body}"
        );
        assert_eq!(
            result.draft().input().scenario_presets.members[0]
                .scenario
                .enemy
                .level
                .to_resolved(),
            level,
            "{body}"
        );
        origin_integrity_with_retired(&source, &result, usize::from(known_encounter));
    }
}

#[test]
fn all_guarded_input_and_placeholder_lanes_stay_pending_without_losing_other_values() {
    let (a, target) = fixture();
    for name in ["branch-kind", "boss-skill", "size-choice"] {
        for field in ["string=\"any\"", "boolean=\"false\"", "number=\"0\""] {
            for element in ["Input", "Placeholder"] {
                let source = source(&xml(&format!("<{element} name=\"{name}\" {field}/>")), 0x64);
                let before = normalize(&source, &a, &policy(), Default::default()).unwrap();
                let after =
                    normalize(&source, &a, &reviewed(&a, &target), Default::default()).unwrap();
                assert_eq!(encounters(&after), vec![None], "{element} {name} {field}");
                assert_eq!(before.draft().input(), after.draft().input());
                origin_integrity(&source, &after);
            }
        }
    }
}

#[test]
fn valid_scopes_are_independent_but_ambiguous_containers_never_supply_defaults() {
    let (a, target) = fixture();
    let source = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"/><ConfigSet id="2"><Input name="boss-skill" string="explicit"/></ConfigSet><ConfigSet id="3"/></Config></PathOfBuilding2>"#,
        0x65,
    );
    let next = normalize(&source, &a, &reviewed(&a, &target), Default::default()).unwrap();
    assert_eq!(
        encounters(&next),
        vec![Some(target.clone()), None, Some(target.clone())]
    );
    origin_integrity_with_retired(&source, &next, 2);
    for xml in [
        r#"<PathOfBuilding2><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="01"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"><Input name="other" string="a"/><Input name="other" string="b"/></ConfigSet></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"><Future/></ConfigSet></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"><Input name="branch-kind" string="x"><Future/></Input></ConfigSet></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet xmlns:q="future" id="1"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/></Config><Config activeConfigSet="2"><ConfigSet id="2"/></Config></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config><Input name="other" string="legacy"/></Config></PathOfBuilding2>"#,
    ] {
        let source = super::source(xml, 0x66);
        let before = normalize(&source, &a, &policy(), Default::default()).unwrap();
        let after = normalize(&source, &a, &reviewed(&a, &target), Default::default()).unwrap();
        assert!(encounters(&after).iter().all(Option::is_none), "{xml}");
        assert_eq!(before.draft().input(), after.draft().input());
        origin_integrity(&source, &after);
    }
}

#[test]
fn stale_source_wrong_kind_namespace_and_target_never_compile_to_authority() {
    let (a, target) = fixture();
    let source = source(&xml(""), 0x67);
    for mutation in 0..6 {
        let mut p = reviewed(&a, &target);
        let Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
            mapping_source,
            selector,
            target,
            absent_input_names,
        }) = &mut p.encounter
        else {
            panic!()
        };
        match mutation {
            0 => *mapping_source = *a.mapping.identity(),
            1 => {
                let ExternalSelector::Catalog { kind, .. } = selector else {
                    panic!()
                };
                *kind = ExternalCatalogKind::Metric;
            }
            2 => {
                *target = EncounterDefId::new(
                    GameVersionNamespace::new("other", "v1").unwrap(),
                    key("foreign"),
                )
            }
            3 => *target = EncounterDefId::new(ns(), key("not-the-mapped-target")),
            4 => {
                let ExternalSelector::Catalog {
                    key: source_key, ..
                } = selector
                else {
                    panic!()
                };
                *source_key = SourceComponent::Text("missing-catalog-key".into());
            }
            5 => absent_input_names.push(absent_input_names[0].clone()),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                normalize(&source, &a, &p, Default::default()),
                Err(NormalizationError::Policy(_) | NormalizationError::Binding)
            ),
            "{mutation}"
        );
    }
}

#[test]
fn unmapped_encounter_or_alias_mapping_is_not_a_known_schema() {
    let (mut a, target) = fixture();
    let source = source(&xml(""), 0x68);
    let mut schema = a.schema.input().clone();
    schema.definitions[0] = DefinitionDescriptor::Encounter(unknown(target.clone()));
    let mapping = a.mapping.input().clone();
    rebind(&mut a, schema, mapping);
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a, &target), Default::default()),
        Err(NormalizationError::Policy("encounter schema unresolved"))
    ));
    let (mut a, target) = fixture();
    let schema = a.schema.input().clone();
    let mut mapping = a.mapping.input().clone();
    let MappingOutcome::Mapped { basis, .. } = &mut mapping.entries[0].outcome else {
        panic!()
    };
    *basis = MappingBasis::ReviewedAlias {
        reason: key("not-exact"),
    };
    rebind(&mut a, schema, mapping);
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a, &target), Default::default()),
        Err(NormalizationError::Policy(
            "encounter mapping contradiction"
        ))
    ));
}

#[test]
fn encounter_policy_and_source_work_remain_bounded() {
    let (a, target) = fixture();
    let source = source(
        &xml(&format!(
            r#"<CustomModifierBlock>{}</CustomModifierBlock>"#,
            "x".repeat(4096)
        )),
        0x69,
    );
    let limits = NormalizationLimits {
        max_work: 1024,
        ..Default::default()
    };
    assert!(normalize(&source, &a, &policy(), limits).is_ok());
    assert!(matches!(
        normalize(&source, &a, &reviewed(&a, &target), limits),
        Err(NormalizationError::Limit(_))
    ));
    for names in [
        vec![],
        vec!["branch-kind".into(); 17],
        vec![" padded ".into()],
        vec!["x".repeat(129)],
    ] {
        let mut p = reviewed(&a, &target);
        let Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
            absent_input_names, ..
        }) = &mut p.encounter
        else {
            panic!()
        };
        *absent_input_names = names;
        assert!(matches!(
            normalize(&source, &a, &p, Default::default()),
            Err(NormalizationError::Policy("encounter absent keys"))
        ));
    }
}
