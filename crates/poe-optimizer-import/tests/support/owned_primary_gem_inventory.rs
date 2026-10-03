//! Physical completeness requires the actual occurrence's attached usage proof.
//! Synthetic identifiers deliberately do not encode a game skill or build.
use super::*;

const GEM: &str = r#"<Gem gemId="active" variantId="v" skillId="synthetic-effect" nameSpec="Synthetic effect" level="17" quality="0" corrupted="false" corruptLevel="0" enabled="true" count="1" enableGlobal1="true" enableGlobal2="true"/>"#;

fn fixture() -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p) = usage_input_tests::fixture();
    let usage = usage_input_tests::row(&mut p).clone();
    let corrupted = p.gem_inputs.as_ref().unwrap().gems[0].parameters[0]
        .slot
        .clone();
    let delta = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(usage.gem.clone()))
        .unwrap();
    let count = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let percent = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let quality = a
        .registry
        .allocate_definition::<QualityDefinition>()
        .unwrap();
    let mut schema = a.schema.input().clone();
    for (id, dimension) in [
        (count.clone(), UnitDimension::Count),
        (percent.clone(), UnitDimension::PercentagePoints),
    ] {
        schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id,
                schema: SchemaState::Known(UnitSchema { dimension }),
            }));
    }
    schema
        .definitions
        .push(DefinitionDescriptor::Quality(DefinitionEntry {
            id: quality.clone(),
            schema: SchemaState::Known(QualitySchema {
                amount: QuantityRange {
                    minimum: FiniteQuantity::new(0.0, percent.clone()).unwrap(),
                    maximum: FiniteQuantity::new(100.0, percent.clone()).unwrap(),
                },
            }),
        }));
    for definition in &mut schema.definitions {
        match definition {
            DefinitionDescriptor::Gem(row) if row.id == usage.gem => {
                let SchemaState::Known(gem) = &mut row.schema else {
                    unreachable!()
                };
                gem.declarations.parameters.members.push(delta.clone());
                gem.quality = QualityUseSchema {
                    presence: QualityPresence::Required,
                    allowed_kinds: DeclaredSet::complete(vec![quality.clone()]),
                };
            }
            DefinitionDescriptor::UsagePolicy(row) if row.id == usage.policy => {
                let SchemaState::Known(policy) = &mut row.schema else {
                    unreachable!()
                };
                policy.declarations.parameters.members.truncate(1);
            }
            _ => {}
        }
    }
    schema.slots.retain(|slot| {
        !matches!(slot, SlotDescriptor::Parameter(row) if row.id == usage.parameters[1].slot)
    });
    schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: delta.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-10.0, count.clone()).unwrap(),
                    maximum: FiniteQuantity::new(10.0, count.clone()).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::GemParameter],
            }),
        }));
    let mut flag = value_recipe("physical-corruption", "corrupted", true);
    let ValueCodecKind::Boolean { tokens } = &mut flag.codec.codec else {
        unreachable!()
    };
    tokens.push(BooleanToken {
        token: "nil".into(),
        value: false,
    });
    let quantity = |id, attribute, unit| {
        let mut value = value_recipe(id, attribute, false);
        value.codec.codec = ValueCodecKind::Quantity {
            syntax: DecimalSyntax::Scientific,
            unit,
            scale: RationalScale {
                numerator: BoundedInteger::new(1).unwrap(),
                denominator: BoundedInteger::new(1).unwrap(),
            },
        };
        value
    };
    let mut corruption = quantity("physical-corruption-level", "corruptLevel", count);
    corruption.numeric_aliases.push(NumericTokenAlias {
        token: "nil".into(),
        replacement: "0".into(),
    });
    p.gem_inputs.as_mut().unwrap().gems[0].parameters = vec![
        GemParameterInput {
            slot: corrupted.clone(),
            value: flag,
        },
        GemParameterInput {
            slot: delta.clone(),
            value: corruption,
        },
    ];
    p.gem_quality = GemQualityPolicy::Attributes(Box::new(GemQualityPolicyInput {
        definitions: a.schema.identity().clone(),
        amount: quantity("physical-quality", "quality", percent),
        kind_attribute: "qualityId".into(),
        kinds: vec![GemQualityKindRule {
            source: SourceComponent::Missing,
            kind: quality,
        }],
    }));
    rebind_quality_schema(&mut a, &mut p, schema);
    let row = usage_input_tests::row(&mut p);
    row.attributes = [
        "gemId",
        "variantId",
        "skillId",
        "nameSpec",
        "level",
        "quality",
        "corrupted",
        "corruptLevel",
        "enabled",
        "count",
        "enableGlobal1",
        "enableGlobal2",
        "statSetIndex",
        "statSetIndexCalcs",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    row.guards = vec![
        GemInputGuard {
            attribute: "count".into(),
            allowed: vec![
                SourceComponent::Text("1".into()),
                SourceComponent::Text("3".into()),
            ],
        },
        GemInputGuard {
            attribute: "enableGlobal2".into(),
            allowed: vec![SourceComponent::Text("true".into())],
        },
    ];
    row.parameters.truncate(1);
    row.parameters[0].value = value_recipe("primary-effect-usage", "enableGlobal1", true);
    let scalar = gem_inventory_scalar_inputs_identity(&p, Default::default()).unwrap();
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        ..
    } = p.usage_inputs.as_mut().unwrap()
    else {
        panic!("historical V1 fixture")
    };
    *definitions = a.schema.identity().clone();
    *roles = *a.roles.identity();
    *catalog = a.roles.input().compilation.catalog_digest;
    *scalar_inputs = scalar;
    p.gem_inventory = Some(GemInventoryPolicy::PobFreshPhysicalV2 {
        definitions: a.schema.identity().clone(),
        roles: *a.roles.identity(),
        catalog: a.roles.input().compilation.catalog_digest,
        scalar_inputs: scalar,
        usage_inputs: usage_inputs_identity(&p, Default::default()).unwrap(),
        supports: vec![],
        primary_skills: vec![PrimarySkillGemInventory {
            physical: PhysicalGemInputInventory {
                gem: usage.gem,
                game_id: usage.game_id,
                variant_id: usage.variant_id,
                skill_id: usage.skill_id,
                name_spec: usage.name_spec,
                corrupted,
                corruption_level: delta,
            },
            usage_policy: usage.policy,
        }],
    });
    (a, p)
}

fn refresh(p: &mut NormalizationPolicy) {
    let scalar = gem_inventory_scalar_inputs_identity(p, Default::default()).unwrap();
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 { scalar_inputs, .. } =
        p.usage_inputs.as_mut().unwrap()
    else {
        panic!("historical V1 fixture")
    };
    *scalar_inputs = scalar;
    let usage = usage_inputs_identity(p, Default::default()).unwrap();
    let GemInventoryPolicy::PobFreshPhysicalV2 {
        scalar_inputs,
        usage_inputs,
        ..
    } = p.gem_inventory.as_mut().unwrap()
    else {
        unreachable!()
    };
    *scalar_inputs = scalar;
    *usage_inputs = usage;
}

fn completion(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .gems
        .members
        .iter()
        .map(|gem| matches!(gem.parameters.completion, DraftListCompletion::Complete))
        .collect()
}

#[test]
fn primary_inventory_closes_two_intrinsic_values_after_real_usage_but_leaves_usage_pending() {
    let (a, p) = fixture();
    let schema = a.schema.input().clone();
    for raw in [
        GEM.to_string(),
        GEM.replace("enableGlobal1=\"true\"", "enableGlobal1=\"false\""),
        GEM.replace("count=\"1\"", "count=\"3\""),
        GEM.replace("enabled=\"true\"", "enabled=\"false\""),
        GEM.replace("corrupted=\"false\"", "corrupted=\"nil\"")
            .replace("corruptLevel=\"0\"", "corruptLevel=\"nil\""),
        GEM.replace("corrupted=\"false\"", "corrupted=\"true\"")
            .replace("corruptLevel=\"0\"", "corruptLevel=\"-0.5\""),
        GEM.replace("/>", r#" statSetIndex="nil" statSetIndexCalcs="nil"/>"#),
    ] {
        let result = normalize_with_loadouts(&group(&raw), &a, &p).unwrap();
        assert_eq!(completion(&result), [true], "{raw}");
        let draft = result.draft().input();
        assert_eq!(draft.gems.members[0].parameters.members.len(), 2);
        let usage = draft.skill_presets.members[0]
            .usage_preferences
            .as_ref()
            .unwrap();
        assert!(
            matches!(&usage.completion, DraftListCompletion::Pending { code, .. }
            if code.as_str() == "usage-preferences-not-converted")
        );
        assert_eq!(usage.members.len(), 1);
        assert!(matches!(
            usage.members[0].parameters.completion,
            DraftListCompletion::Complete
        ));
        assert_eq!(usage.members[0].parameters.members.len(), 1);
    }
    assert_eq!(schema, *a.schema.input());
    let GemInventoryPolicy::PobFreshPhysicalV2 { primary_skills, .. } =
        p.gem_inventory.as_ref().unwrap()
    else {
        unreachable!()
    };
    let SchemaLookup::Known(gem) = a.schema.definition(&primary_skills[0].physical.gem) else {
        unreachable!()
    };
    assert!(!gem.declarations.parameters.is_complete());
}

#[test]
fn duplicate_occurrences_and_other_presets_cannot_lend_a_usage_proof() {
    let (a, p) = fixture();
    let bad = GEM.replace("enableGlobal1=\"true\"", "enableGlobal1=\"unknown\"");
    let off = GEM.replace("enableGlobal1=\"true\"", "enableGlobal1=\"false\"");
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="8"><SkillSet id="7"><Skill enabled="true">{GEM}{bad}{GEM}</Skill></SkillSet><SkillSet id="8"><Skill enabled="false">{bad}{off}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let imported = source(&xml, 93);
    let result = run_with_policy(&imported, &a, &[], &p);
    assert_eq!(completion(&result), [true, false, true, false, true]);
    let draft = result.draft().input();
    assert_eq!(draft.skill_presets.members.len(), 2);
    assert_eq!(
        draft
            .gems
            .members
            .iter()
            .map(|gem| gem.id)
            .collect::<BTreeSet<_>>()
            .len(),
        5
    );
    let expected = usage_input_tests::row(&mut p.clone()).clone();
    for preset in &draft.skill_presets.members {
        let usage = preset.usage_preferences.as_ref().unwrap();
        assert!(matches!(
            usage.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(usage.members.len(), preset.skills.members.len());
        for (row, skill) in usage.members.iter().zip(&preset.skills.members) {
            assert_eq!(row.policy.to_resolved(), Some(expected.policy.clone()));
            assert_eq!(
                row.target.to_resolved(),
                Some(UsageTarget::Skill(SkillTarget::Generated(Box::new(
                    GeneratedSkillKey {
                        provider: ProviderKey {
                            root: ProviderRoot::SkillUse(*skill),
                            grant_path: vec![]
                        },
                        slot: expected.supply.clone(),
                    }
                ))))
            );
        }
    }
    origin_integrity(&imported, &result);
}

#[test]
fn missing_containing_preset_or_incomplete_usage_never_closes_physical_parameters() {
    let (a, p) = fixture();
    let no_preset = format!(
        r#"<PathOfBuilding2><Skills><Skill enabled="true">{GEM}</Skill></Skills></PathOfBuilding2>"#
    );
    let result = normalize_with_loadouts(&no_preset, &a, &p).unwrap();
    assert_eq!(completion(&result), [false]);
    assert!(result.draft().input().skill_presets.members.is_empty());
    for raw in [
        GEM.replace(" enableGlobal1=\"true\"", ""),
        GEM.replace("enableGlobal1=\"true\"", "enableGlobal1=\"unknown\""),
        GEM.replace("enableGlobal1=\"true\"", "enableGlobal1=\"nil\""),
        GEM.replace(
            "nameSpec=\"Synthetic effect\"",
            "nameSpec=\"Sibling effect\"",
        ),
        GEM.replace("skillId=\"synthetic-effect\"", "skillId=\"sibling-effect\""),
    ] {
        let result = normalize_with_loadouts(&group(&raw), &a, &p).unwrap();
        assert_eq!(completion(&result), [false], "{raw}");
        assert!(matches!(
            result.draft().input().skill_presets.members[0]
                .usage_preferences
                .as_ref()
                .unwrap()
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn unreviewed_gem_fields_children_values_and_counts_preserve_pending_inventory() {
    let (a, p) = fixture();
    for raw in [
        GEM.replace("/>", r#" unknown="true"/>"#),
        GEM.replace("/>", r#"><Unknown/></Gem>"#),
        GEM.replace("/>", r#">text</Gem>"#),
        GEM.replace("level=\"17\"", "level=\"NaN\""),
        GEM.replace("level=\"17\"", "level=\"0\""),
        GEM.replace("quality=\"0\"", "quality=\"101\""),
        GEM.replace("corrupted=\"false\"", "corrupted=\"unknown\""),
        GEM.replace("corruptLevel=\"0\"", "corruptLevel=\"11\""),
        GEM.replace("corruptLevel=\"0\"", "corruptLevel=\"NaN\""),
        GEM.replace("count=\"1\"", "count=\"2\""),
        GEM.replace("count=\"1\"", "count=\"1.0\""),
        GEM.replace("count=\"1\"", "count=\"unknown\""),
        GEM.replace(" count=\"1\"", ""),
        GEM.replace("enableGlobal2=\"true\"", "enableGlobal2=\"false\""),
        GEM.replace("enabled=\"true\"", "enabled=\"unknown\""),
        GEM.replace("/>", r#" statSetIndex="1"/>"#),
        GEM.replace("/>", r#" statSetIndexCalcs="1"/>"#),
        GEM.replace("/>", r#" qualityId="invented"/>"#),
    ] {
        let result = normalize_with_loadouts(&group(&raw), &a, &p).unwrap();
        assert_eq!(completion(&result), [false], "{raw}");
    }
}

#[test]
fn contextual_overrides_and_unknown_siblings_cannot_borrow_plain_group_proof() {
    let (a, p) = fixture();
    for attributes in [
        r#"slot="Helmet""#,
        r#"mainActiveSkill="2""#,
        r#"mainActiveSkillCalcs="2""#,
        r#"includeInFullDPS="unknown""#,
        r#"groupCount="2""#,
        r#"skillPart="2""#,
    ] {
        let xml = group(GEM).replace(
            r#"<Skill enabled="true">"#,
            &format!(r#"<Skill enabled="true" {attributes}>"#),
        );
        let result = normalize_with_loadouts(&xml, &a, &p).unwrap();
        assert_eq!(completion(&result), [false], "{attributes}");
    }
    for extra in ["<Unknown/>", "text", r#"<Gem xmlns="urn:unknown"/>"#] {
        let result = normalize_with_loadouts(&group(&format!("{GEM}{extra}")), &a, &p).unwrap();
        assert_eq!(completion(&result), [false], "{extra}");
    }
    let valid = group(GEM).replace(r#"<Skill enabled="true">"#,
        r#"<Skill enabled="false" label="Reviewed" mainActiveSkill="1" mainActiveSkillCalcs="nil" includeInFullDPS="true">"#);
    assert_eq!(
        completion(&normalize_with_loadouts(&valid, &a, &p).unwrap()),
        [true]
    );
    let generated = group(GEM).replace(
        r#"<Skill enabled="true">"#,
        r#"<Skill enabled="true" source="Item:1">"#,
    );
    assert!(
        normalize_with_loadouts(&generated, &a, &p)
            .unwrap()
            .draft()
            .input()
            .gems
            .members
            .is_empty()
    );
}

#[test]
fn stale_bindings_and_incomplete_or_aliased_inventory_declarations_are_rejected() {
    let (a, p) = fixture();
    for case in 0..11 {
        let mut bad = p.clone();
        let GemInventoryPolicy::PobFreshPhysicalV2 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            usage_inputs,
            primary_skills,
            supports,
        } = bad.gem_inventory.as_mut().unwrap()
        else {
            unreachable!()
        };
        match case {
            0 => definitions.release = "different-release".into(),
            1 => *roles = *catalog,
            2 => *catalog = *roles,
            3 => *scalar_inputs = *roles,
            4 => *usage_inputs = *roles,
            5 => primary_skills.push(primary_skills[0].clone()),
            6 => supports.push(primary_skills[0].physical.clone()),
            7 => {
                primary_skills[0].physical.corruption_level =
                    primary_skills[0].physical.corrupted.clone()
            }
            8 => primary_skills[0].physical.name_spec = "Unreviewed effect".into(),
            9 => primary_skills[0].physical.skill_id.clear(),
            10 => bad.usage_inputs = None,
            _ => unreachable!(),
        }
        assert!(
            normalize_with_loadouts(&group(GEM), &a, &bad).is_err(),
            "case {case}"
        );
    }
    for case in 0..7 {
        let mut bad = p.clone();
        let row = usage_input_tests::row(&mut bad);
        match case {
            0 => row.guards.retain(|guard| guard.attribute != "count"),
            1 => row
                .guards
                .retain(|guard| guard.attribute != "enableGlobal2"),
            2 => row.attributes.retain(|name| name != "quality"),
            3 => row.attributes.push("unreviewed".into()),
            4 => row.parameters[0].value.tiers[0].selectors[0].name = "enabled".into(),
            5 => row.parameters.clear(),
            6 => row.name_spec = "Wrong usage source".into(),
            _ => unreachable!(),
        }
        refresh(&mut bad);
        assert!(
            normalize_with_loadouts(&group(GEM), &a, &bad).is_err(),
            "usage case {case}"
        );
    }
    let mut stale = p.clone();
    usage_input_tests::row(&mut stale).guards[0]
        .allowed
        .push(SourceComponent::Text("2".into()));
    assert!(matches!(
        normalize_with_loadouts(&group(GEM), &a, &stale),
        Err(NormalizationError::Binding)
    ));
    refresh(&mut stale);
    let revised = GEM.replace("count=\"1\"", "count=\"2\"");
    assert_eq!(
        completion(&normalize_with_loadouts(&group(&revised), &a, &stale).unwrap()),
        [true]
    );
}

#[test]
fn legacy_support_wire_and_outputs_stay_unchanged_and_v2_is_explicit() {
    let (a, legacy) = gem_inventory_tests::fixture();
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let decoded: NormalizationPolicy = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    let wire = serde_json::to_value(&decoded).unwrap();
    assert_eq!(wire["gem_inventory"]["kind"], "pob_fresh_single_support_v1");
    assert!(wire["gem_inventory"].get("primary_skills").is_none());
    let xml = group(gem_inventory_tests::GEM);
    let before = normalize_with_loadouts(&xml, &a, &legacy).unwrap();
    let after = normalize_with_loadouts(&xml, &a, &decoded).unwrap();
    assert_eq!(before.draft(), after.draft());
    assert_eq!(
        serde_json::to_value(before.sidecar()).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
    let (active_a, active_p) = fixture();
    let mut no_inventory = active_p.clone();
    no_inventory.gem_inventory = None;
    let result = normalize_with_loadouts(&group(GEM), &active_a, &no_inventory).unwrap();
    assert_eq!(completion(&result), [false]);
    assert_eq!(
        result.draft().input().gems.members[0]
            .parameters
            .members
            .len(),
        2
    );
    let mut wire = serde_json::to_value(&active_p).unwrap();
    assert_eq!(wire["gem_inventory"]["kind"], "pob_fresh_physical_v2");
    wire["gem_inventory"]["force_complete"] = true.into();
    assert!(serde_json::from_value::<NormalizationPolicy>(wire).is_err());
    for field in ["supports", "primary_skills", "usage_inputs"] {
        let mut wire = serde_json::to_value(&active_p).unwrap();
        wire["gem_inventory"].as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<NormalizationPolicy>(wire).is_err(),
            "{field}"
        );
    }
}

#[test]
fn active_inventory_honors_shared_work_and_policy_byte_budgets() {
    let (a, p) = fixture();
    for limits in [
        NormalizationLimits {
            max_work: 1,
            ..Default::default()
        },
        NormalizationLimits {
            max_policy_bytes: 100,
            ..Default::default()
        },
    ] {
        assert!(usage_input_tests::run_limits(&group(GEM), &a, &p, limits).is_err());
    }
    let mut huge = p.clone();
    let GemInventoryPolicy::PobFreshPhysicalV2 { primary_skills, .. } =
        huge.gem_inventory.as_mut().unwrap()
    else {
        unreachable!()
    };
    primary_skills.resize(4097, primary_skills[0].clone());
    assert!(usage_input_tests::run_limits(&group(GEM), &a, &huge, Default::default()).is_err());
}

#[test]
fn usage_v2_retains_boolean_physical_proofs_and_legacy_draft_allocation() {
    let (a, mut policy) = fixture();
    let xml = group(GEM);
    let before = normalize_with_loadouts(&xml, &a, &policy).unwrap();
    assert_eq!(completion(&before), vec![true]);
    let legacy_bytes = serde_json::to_vec(policy.usage_inputs.as_ref().unwrap()).unwrap();
    let roundtrip: UsageInputPolicy = serde_json::from_slice(&legacy_bytes).unwrap();
    assert_eq!(serde_json::to_vec(&roundtrip).unwrap(), legacy_bytes);
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        gems,
    } = policy.usage_inputs.take().unwrap()
    else {
        unreachable!()
    };
    policy.usage_inputs = Some(UsageInputPolicy::PobPhysicalPrimarySkillV2 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        gems,
        numeric_gems: vec![],
    });
    let digest = usage_inputs_identity(&policy, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV2 { usage_inputs, .. }) =
        &mut policy.gem_inventory
    else {
        unreachable!()
    };
    *usage_inputs = digest;
    let after = normalize_with_loadouts(&xml, &a, &policy).unwrap();
    assert_eq!(completion(&after), vec![true]);
    assert_eq!(
        serde_json::to_vec(before.draft().input()).unwrap(),
        serde_json::to_vec(after.draft().input()).unwrap()
    );
    let mut old_sidecar = serde_json::to_value(before.sidecar()).unwrap();
    let new_sidecar = serde_json::to_value(after.sidecar()).unwrap();
    assert_ne!(old_sidecar["policy"], new_sidecar["policy"]);
    old_sidecar["policy"] = new_sidecar["policy"].clone();
    assert_eq!(old_sidecar, new_sidecar);
}
