//! Synthetic finite source domains exercise the proof, not real game rules.
use super::*;

fn fixture() -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p, quality, _) = quality_artifacts();
    let role = a
        .roles
        .input()
        .roles
        .iter()
        .find(|r| r.role == OwnedGemRole::Known(AuthoredGemRole::SupportAssignment))
        .unwrap()
        .clone();
    let OwnedPrimarySkill::Known(primary) = role.primary else {
        unreachable!()
    };
    let count = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let owner = SlotOwnerDefId::Gem(role.gem.clone());
    let corrupted = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(owner.clone())
        .unwrap();
    let corruption_level = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(owner)
        .unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: count.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }));
    let partial = |members| {
        DeclaredSet::partial(
            members,
            vec![SchemaGap {
                subject: subject(&role.gem),
                facet: SchemaFacet::InputSchema,
                code: key("synthetic-unconverted-definition-inputs"),
            }],
        )
    };
    let row = schema
        .definitions
        .iter_mut()
        .find_map(|r| match r {
            DefinitionDescriptor::Gem(r) if r.id == role.gem => Some(r),
            _ => None,
        })
        .unwrap();
    row.schema = SchemaState::Known(GemSchema {
        level: IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(40).unwrap(),
        },
        roles: vec![AuthoredGemRole::SupportAssignment],
        skills: DeclaredSet::partial(
            vec![primary],
            vec![SchemaGap {
                subject: subject(&role.gem),
                facet: SchemaFacet::StaticLinks,
                code: key("unconverted-effects"),
            }],
        ),
        quality: QualityUseSchema {
            presence: QualityPresence::Required,
            allowed_kinds: DeclaredSet::complete(vec![quality]),
        },
        declarations: DeclaredSlots {
            parameters: partial(vec![corrupted.clone(), corruption_level.clone()]),
            choices: DeclaredSet::complete(vec![]),
            grants: DeclaredSet::complete(vec![]),
            actors: DeclaredSet::complete(vec![]),
            skill_grants: DeclaredSet::complete(vec![]),
            outputs: DeclaredSet::complete(vec![]),
            sockets: DeclaredSet::complete(vec![]),
        },
    });
    for (slot, value) in [
        (corrupted.clone(), ValueSchema::Boolean),
        (
            corruption_level.clone(),
            ValueSchema::Quantity(QuantityRange {
                minimum: FiniteQuantity::new(-10.0, count.clone()).unwrap(),
                maximum: FiniteQuantity::new(10.0, count.clone()).unwrap(),
            }),
        ),
    ] {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot,
                schema: SchemaState::Known(ParameterSlotSchema {
                    value,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                }),
            }));
    }
    let mut flag = value_recipe("physical-flag", "corrupted", true);
    let ValueCodecKind::Boolean { tokens } = &mut flag.codec.codec else {
        unreachable!()
    };
    tokens.push(BooleanToken {
        token: "nil".into(),
        value: false,
    });
    let mut delta = value_recipe("physical-delta", "corruptLevel", false);
    delta.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Scientific,
        unit: count,
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    };
    delta.numeric_aliases.push(NumericTokenAlias {
        token: "nil".into(),
        replacement: "0".into(),
    });
    p.gem_inputs = Some(GemInputPolicy {
        definitions: a.schema.identity().clone(),
        gems: vec![GemInputRule {
            gem: role.gem.clone(),
            guards: vec![],
            parameters: vec![
                GemParameterInput {
                    slot: corrupted.clone(),
                    value: flag,
                },
                GemParameterInput {
                    slot: corruption_level.clone(),
                    value: delta,
                },
            ],
        }],
    });
    rebind_quality_schema(&mut a, &mut p, schema);
    let GemQualityPolicy::Attributes(q) = &mut p.gem_quality else {
        unreachable!()
    };
    q.kind_attribute = "qualityId".into();
    q.kinds.retain(|v| v.source == SourceComponent::Missing);
    p.gem_inventory = Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
        definitions: a.schema.identity().clone(),
        roles: *a.roles.identity(),
        catalog: a.roles.input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(&p, NormalizationLimits::default())
            .unwrap(),
        gems: vec![SingleSupportGemInventory {
            gem: role.gem,
            game_id: "support".into(),
            variant_id: "v".into(),
            skill_id: "synthetic-support-effect".into(),
            name_spec: "Synthetic support".into(),
            corrupted,
            corruption_level,
        }],
    });
    (a, p)
}
const GEM: &str = r#"<Gem gemId="support" variantId="v" skillId="synthetic-support-effect" nameSpec="Synthetic support" level="1" quality="0" corrupted="false" corruptLevel="0" enabled="true" count="1" enableGlobal1="true" enableGlobal2="true"/>"#;
fn xml(gems: &str) -> String {
    group(gems)
}
fn complete_parameters(r: &NormalizedImport) -> bool {
    !r.draft().input().gems.members.is_empty()
        && r.draft()
            .input()
            .gems
            .members
            .iter()
            .all(|g| matches!(g.parameters.completion, DraftListCompletion::Complete))
}
fn refresh(p: &mut NormalizationPolicy) {
    let digest = gem_inventory_scalar_inputs_identity(p, NormalizationLimits::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshSingleSupportV1 { scalar_inputs, .. }) =
        &mut p.gem_inventory
    else {
        unreachable!()
    };
    *scalar_inputs = digest;
}

#[test]
fn finite_support_inventory_closes_only_physical_assignments_and_keeps_partial_definitions() {
    let (a, p) = fixture();
    let schema_before = a.schema.input().clone();
    for raw in [
        GEM.to_string(),
        GEM.replace("/>", r#" statSetIndex="nil" statSetIndexCalcs="nil"/>"#),
        GEM.replace("corrupted=\"false\"", "corrupted=\"true\"")
            .replace("corruptLevel=\"0\"", "corruptLevel=\"-0.5\""),
        GEM.replace("corrupted=\"false\"", "corrupted=\"nil\"")
            .replace("corruptLevel=\"0\"", "corruptLevel=\"nil\""),
        GEM.replace("enabled=\"true\"", "enabled=\"false\""),
    ] {
        let result = normalize_with_loadouts(&xml(&raw), &a, &p).unwrap();
        assert!(complete_parameters(&result), "{raw}");
        assert_eq!(
            result.draft().input().gems.members[0]
                .parameters
                .members
                .len(),
            2
        );
        assert!(matches!(
            result.draft().input().supports.members[0].target,
            DraftSkillTarget::Pending(_)
        ));
    }
    assert_eq!(&schema_before, a.schema.input());
    let GemInventoryPolicy::PobFreshSingleSupportV1 { gems, .. } =
        p.gem_inventory.as_ref().unwrap();
    let SchemaLookup::Known(schema) = a.schema.definition(&gems[0].gem) else {
        unreachable!()
    };
    assert!(!schema.declarations.parameters.is_complete());
    assert!(!schema.skills.is_complete());
    let source = source(&xml(&format!("{GEM}{GEM}")), 77);
    let result = run_with_policy(&source, &a, &[], &p);
    assert!(complete_parameters(&result));
    assert_ne!(
        result.draft().input().gems.members[0].id,
        result.draft().input().gems.members[1].id
    );
    origin_integrity(&source, &result);
}

#[test]
fn every_saved_field_and_intrinsic_must_be_accounted_without_losing_successful_scalars() {
    let (a, p) = fixture();
    for (old, new) in [
        ("gemId=\"support\"", "gemId=\"missing\""),
        ("variantId=\"v\"", "variantId=\"wrong\""),
        ("skillId=\"synthetic-support-effect\"", "skillId=\"wrong\""),
        ("nameSpec=\"Synthetic support\"", "nameSpec=\"different\""),
        ("level=\"1\"", "level=\"0\""),
        ("level=\"1\"", "level=\"41\""),
        ("level=\"1\"", "level=\"bad\""),
        ("quality=\"0\"", "quality=\"bad\""),
        ("quality=\"0\"", "quality=\"101\""),
        ("count=\"1\"", "count=\"2\""),
        ("count=\"1\"", "count=\"1.0\""),
        ("enableGlobal1=\"true\"", "enableGlobal1=\"false\""),
        ("enableGlobal2=\"true\"", "enableGlobal2=\"nil\""),
        ("enabled=\"true\"", "enabled=\"bad\""),
    ] {
        let result = normalize_with_loadouts(&xml(&GEM.replace(old, new)), &a, &p).unwrap();
        assert!(!complete_parameters(&result), "{old} -> {new}");
        for gem in &result.draft().input().gems.members {
            if gem.definition.to_resolved().is_some() {
                assert_eq!(gem.parameters.members.len(), 2);
            }
        }
    }
    for attribute in [
        "level=\"1\"",
        "quality=\"0\"",
        "enabled=\"true\"",
        "count=\"1\"",
        "enableGlobal1=\"true\"",
        "enableGlobal2=\"true\"",
        "skillId=\"synthetic-support-effect\"",
        "nameSpec=\"Synthetic support\"",
    ] {
        assert!(!complete_parameters(
            &normalize_with_loadouts(&xml(&GEM.replace(attribute, "")), &a, &p).unwrap()
        ));
    }
    for (old, new) in [
        ("corrupted=\"false\"", ""),
        ("corruptLevel=\"0\"", ""),
        ("corrupted=\"false\"", "corrupted=\"bad\""),
        ("corruptLevel=\"0\"", "corruptLevel=\"11\""),
    ] {
        let result = normalize_with_loadouts(&xml(&GEM.replace(old, new)), &a, &p).unwrap();
        assert!(!complete_parameters(&result));
        assert_eq!(
            result.draft().input().gems.members[0]
                .parameters
                .members
                .len(),
            1
        );
    }
}

#[test]
fn extra_namespace_child_legacy_and_group_context_are_not_inventory_authority() {
    let (a, p) = fixture();
    // These never reach collected source evidence: the XML boundary rejects
    // unknown entities and duplicate attributes before normalization begins.
    for malformed in [
        xml(&GEM.replace("/>", r#" statSetIndex="&bad;"/>"#)),
        xml(&GEM.replace("/>", r#" level="1"/>"#)),
        xml(GEM).replace(
            "<Skill enabled=\"true\">",
            "<Skill enabled=\"true\" enabled=\"true\">",
        ),
    ] {
        assert!(decode_build(malformed.as_bytes()).is_err());
    }
    let mut cases = vec![];
    for attribute in [
        r#"statSetIndex="1""#,
        r#"statSetIndex="""#,
        r#"statSetIndex="bad""#,
        r#"skillPart="1""#,
        r#"skillMinion="x""#,
        r#"note="author""#,
        r#"extra="1""#,
        r#"xmlns:x="urn:x" x:flag="0""#,
    ] {
        cases.push(xml(&GEM.replace("/>", &format!(" {attribute}/>"))));
    }
    for content in [
        "text",
        "<StatSetIndex grantedEffect=\"x\" index=\"1\"/>",
        "<!-- annotation -->",
    ] {
        cases.push(xml(&GEM.replace("/>", &format!(">{content}</Gem>"))));
    }
    for attribute in [
        r#"skillPart="1""#,
        r#"groupCount="1""#,
        r#"slot="Weapon 1""#,
        r#"unknown="x""#,
        r#"xmlns:x="urn:x""#,
    ] {
        cases.push(xml(GEM).replace(
            "<Skill enabled=\"true\">",
            &format!("<Skill enabled=\"true\" {attribute}>"),
        ));
    }
    cases.push(xml(GEM).replace("</Skill>", "<Unreviewed/></Skill>"));
    for case in cases {
        match normalize_with_loadouts(&case, &a, &p) {
            Ok(result) => assert!(!complete_parameters(&result), "{case}"),
            Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {}
            Err(other) => panic!("unexpected proof error {other}: {case}"),
        }
    }
}

#[test]
fn generated_materialization_is_exactly_bound_and_never_fills_support_targets() {
    let (a, p) = fixture();
    let generated = xml(GEM).replace(
        "<Skill enabled=\"true\">",
        "<Skill enabled=\"true\" source=\"Item:reviewed\">",
    );
    let result = normalize_with_loadouts(&generated, &a, &p).unwrap();
    assert!(complete_parameters(&result));
    assert!(matches!(
        result.draft().input().supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    let mut changed = p.clone();
    changed.generated_support_prefixes.clear();
    refresh(&mut changed);
    let result = normalize_with_loadouts(&generated, &a, &changed).unwrap();
    assert!(result.draft().input().gems.members.is_empty());
}

#[test]
fn bindings_domains_and_closed_wire_reject_stale_or_ambiguous_authoring() {
    let (a, p) = fixture();
    for case in 0..9 {
        let mut bad = p.clone();
        let GemInventoryPolicy::PobFreshSingleSupportV1 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
        } = bad.gem_inventory.as_mut().unwrap();
        match case {
            0 => definitions.content_sha256 = "0".repeat(64),
            1 => {
                *roles = poe_optimizer_core::owned_content::digest_owned("wrong", &1, 100).unwrap()
            }
            2 => {
                *catalog =
                    poe_optimizer_core::owned_content::digest_owned("wrong", &2, 100).unwrap()
            }
            3 => {
                *scalar_inputs =
                    poe_optimizer_core::owned_content::digest_owned("wrong", &3, 100).unwrap()
            }
            4 => gems.push(gems[0].clone()),
            5 => gems[0].corruption_level = gems[0].corrupted.clone(),
            6 => gems[0].game_id = "active".into(),
            7 => gems[0].variant_id = "other".into(),
            8 => gems[0].name_spec.clear(),
            _ => unreachable!(),
        }
        assert!(
            normalize_with_loadouts(&xml(GEM), &a, &bad).is_err(),
            "case {case}"
        );
    }
    let mut changed = p.clone();
    changed.gem_inputs.as_mut().unwrap().gems[0]
        .parameters
        .pop();
    refresh(&mut changed);
    assert!(normalize_with_loadouts(&xml(GEM), &a, &changed).is_err());
    let mut wire = serde_json::to_value(&p).unwrap();
    wire["gem_inventory"]["force_complete"] = true.into();
    assert!(serde_json::from_value::<NormalizationPolicy>(wire).is_err());
    let mut empty = p.clone();
    let GemInventoryPolicy::PobFreshSingleSupportV1 { gems, .. } =
        empty.gem_inventory.as_mut().unwrap();
    gems.clear();
    assert!(!complete_parameters(
        &normalize_with_loadouts(&xml(GEM), &a, &empty).unwrap()
    ));
}

#[test]
fn omitted_policy_bytes_allocations_and_partial_known_values_are_unchanged() {
    let (a, mut p) = fixture();
    p.gem_inventory = None;
    let encoded = serde_json::to_vec(&p).unwrap();
    let decoded: NormalizationPolicy = serde_json::from_slice(&encoded).unwrap();
    assert!(
        serde_json::to_value(&p)
            .unwrap()
            .get("gem_inventory")
            .is_none()
    );
    assert_eq!(encoded, serde_json::to_vec(&decoded).unwrap());
    let before = normalize_with_loadouts(&xml(GEM), &a, &p).unwrap();
    let after = normalize_with_loadouts(&xml(GEM), &a, &decoded).unwrap();
    assert!(!complete_parameters(&before));
    assert_eq!(
        serde_json::to_value(before.draft().input()).unwrap(),
        serde_json::to_value(after.draft().input()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(before.sidecar()).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
    assert_eq!(
        before.draft().input().gems.members[0]
            .parameters
            .members
            .len(),
        2
    );
}

#[test]
fn explicit_lower_work_and_policy_byte_limits_remain_enforced() {
    let (a, p) = fixture();
    let imported = source(&xml(GEM), 42);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    for limits in [
        NormalizationLimits {
            max_work: 1,
            ..NormalizationLimits::default()
        },
        NormalizationLimits {
            max_policy_bytes: 100,
            ..NormalizationLimits::default()
        },
    ] {
        assert!(
            normalize_fresh(
                &evidence,
                *imported.allocator_state(),
                NormalizationArtifacts {
                    mappings: &a.mapping,
                    registry: &a.registry,
                    definitions: &a.schema,
                    roles: &a.roles,
                    rewards: &a.rewards,
                    items: &empty_items(&a.schema),
                    item_source: &empty_item_source(&a.schema),
                    tree: None,
                },
                &p,
                &[],
                limits
            )
            .is_err()
        );
    }
}
