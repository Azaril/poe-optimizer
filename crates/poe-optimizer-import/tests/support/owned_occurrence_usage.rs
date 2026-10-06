//! Shared occurrence usage: real source joins, independent typed values, no closure.
use super::*;
use crate::direct_skill_input_tests as direct;

fn usage_definitions(a: &mut Artifacts, p: &mut NormalizationPolicy) -> Vec<OccurrenceUsagePolicy> {
    let mut schema = a.schema.input().clone();
    let mut policies = Vec::new();
    for index in 0..2 {
        let id = a
            .registry
            .allocate_definition::<UsagePolicyDefinition>()
            .unwrap();
        let slot = a
            .registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(id.clone()))
            .unwrap();
        let mut declarations = ports();
        declarations.parameters.members.push(slot.clone());
        schema
            .definitions
            .push(DefinitionDescriptor::UsagePolicy(known(
                id.clone(),
                UsagePolicySchema {
                    targets: vec![UsageTargetKind::Skill],
                    declarations,
                },
            )));
        schema.slots.push(SlotDescriptor::Parameter(known(
            slot.clone(),
            ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(4).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::UsagePolicyParameter],
                skill_input: None,
            },
        )));
        let mut occurrence = value_recipe(&format!("request-count-{index}"), "count", false);
        occurrence.numeric_aliases.push(NumericTokenAlias {
            token: "nil".into(),
            replacement: "1".into(),
        });
        policies.push(OccurrenceUsagePolicy {
            policy: id,
            parameters: vec![UsageParameterInput {
                slot,
                source: UsageValueSource::ContainingGroupOverride {
                    group: Box::new(value_recipe(
                        &format!("group-count-{index}"),
                        "groupCount",
                        false,
                    )),
                    occurrence,
                    fallback_admission: UsageFallbackAdmission::RequestedOccurrence,
                },
            }],
        });
    }
    rebind_quality_schema(a, p, schema);
    if let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
        definitions, roles, ..
    }) = &mut p.generated_skill_inputs
    {
        *definitions = a.schema.identity().clone();
        *roles = *a.roles.identity();
    }
    if let Some(DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions, roles, ..
    }) = &mut p.direct_skill_inputs
    {
        *definitions = a.schema.identity().clone();
        *roles = *a.roles.identity();
    }
    policies
}
fn occurrence(
    target: OccurrenceUsageTarget,
    policies: &[OccurrenceUsagePolicy],
) -> OccurrenceUsageRule {
    OccurrenceUsageRule {
        target,
        attributes: [
            "gemId",
            "variantId",
            "skillId",
            "nameSpec",
            "level",
            "quality",
            "enabled",
            "corrupted",
            "count",
        ]
        .map(str::to_string)
        .to_vec(),
        guards: vec![],
        group_attributes: ["source", "enabled", "slot", "groupCount"]
            .map(str::to_string)
            .to_vec(),
        group_guards: vec![],
        policies: policies.to_vec(),
    }
}
fn install(a: &Artifacts, p: &mut NormalizationPolicy, rows: Vec<OccurrenceUsageRule>) {
    p.usage_inputs = Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: a.schema.identity().clone(),
        source: pin(),
        roles: *a.roles.identity(),
        catalog: a.roles.input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(p, Default::default()).unwrap(),
        gems: vec![],
        numeric_gems: vec![],
        occurrences: rows,
    });
}
fn generated() -> Fixture {
    let mut f = generated_fixture();
    let policies = usage_definitions(&mut f.artifacts, &mut f.policy);
    let mut items = f.items.input().clone();
    items.definitions = f.artifacts.schema.identity().clone();
    f.items = OwnedItemLinePolicy::new(items, &f.artifacts.schema, Default::default()).unwrap();
    let mut source = f.item_source.input().clone();
    source.item_lines = *f.items.identity();
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. }) =
        &mut f.policy.equipment_membership
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        item_lines,
        item_source,
        ..
    }) = &mut f.policy.item_modifier_membership
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    *item_lines = *f.items.identity();
    *item_source = *f.item_source.identity();
    let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { rows, .. }) =
        &f.policy.generated_skill_inputs
    else {
        panic!()
    };
    let rows = rows
        .iter()
        .map(|row| {
            let mut value = serde_json::to_value(row).unwrap();
            value.as_object_mut().unwrap().remove("parameters");
            occurrence(
                OccurrenceUsageTarget::Generated {
                    correspondence: serde_json::from_value(value).unwrap(),
                },
                &policies,
            )
        })
        .collect();
    install(&f.artifacts, &mut f.policy, rows);
    f
}
fn manual() -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p) = direct::fixture();
    let policies = usage_definitions(&mut a, &mut p);
    let raw = direct::row(&mut p).clone();
    direct::row(&mut p).attributes.push("count".into());
    let Some(DirectSkillInputPolicy::PobManualDirectSkillV1 {
        group_attributes, ..
    }) = &mut p.direct_skill_inputs
    else {
        panic!()
    };
    group_attributes.push("groupCount".into());
    let mut mapping = a.mapping.input().clone();
    mapping.entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(raw.skill_id.clone()),
        }),
        outcome: MappingOutcome::Mapped {
            target: subject(&raw.skill),
            basis: MappingBasis::Exact,
        },
    });
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, Default::default()).unwrap();
    a.rewards = empty_rewards(&a.mapping, &a.schema);
    let Some(DirectSkillInputPolicy::PobManualDirectSkillV1 { roles, .. }) =
        &mut p.direct_skill_inputs
    else {
        panic!()
    };
    *roles = *a.roles.identity();
    install(
        &a,
        &mut p,
        vec![occurrence(
            OccurrenceUsageTarget::AuthoredDirect {
                gem: raw.gem,
                game_id: raw.game_id,
                variant_id: raw.variant_id,
                skill_id: raw.skill_id,
                name_spec: raw.name_spec,
                skill: Box::new(raw.skill),
            },
            &policies,
        )],
    );
    (a, p)
}
fn new_rows(p: &mut NormalizationPolicy) -> &mut Vec<OccurrenceUsageRule> {
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { occurrences, .. }) = &mut p.usage_inputs
    else {
        panic!()
    };
    occurrences
}
fn source() -> String {
    document().replace(" quality=", " count=\"nil\" quality=")
}
fn usage(result: &NormalizedImport, preset: usize) -> Vec<&UsagePolicyDraft> {
    let preset = &result.draft().input().skill_presets.members[preset];
    if let Some(intent) = &preset.intent {
        intent
            .usage
            .members
            .iter()
            .map(|row| &row.selection)
            .collect()
    } else {
        preset
            .usage_preferences
            .as_ref()
            .map_or(vec![], |usage| usage.members.iter().collect())
    }
}
fn numbers(result: &NormalizedImport, preset: usize) -> Vec<Option<i64>> {
    usage(result, preset)
        .iter()
        .map(|row| {
            row.parameters.to_resolved().map(|p| {
                let ParameterValue::Integer(value) = p[0].value else {
                    panic!()
                };
                value.get()
            })
        })
        .collect()
}
#[test]
fn two_policies_share_exact_generated_target_without_quality_or_preset_fanout() {
    let f = generated();
    let text = source().replace("quality=\"12.5\"", "quality=\"bad\"");
    let mut old_policy = f.policy.clone();
    old_policy.usage_inputs = None;
    let old = normalize(&text, &f, &old_policy, Default::default()).unwrap();
    let result = run(&text, &f);
    assert_eq!(result.sidecar().schema_version, 21);
    assert_eq!(old.sidecar().schema_version, 21);
    assert_eq!(numbers(&result, 1), [Some(1); 4]);
    assert!(usage(&result, 0).is_empty());
    assert!(usage(&result, 2).is_empty());
    for index in 0..3 {
        assert_eq!(
            bindings(&result, index),
            bindings(&old, index),
            "raw inputs and old issues unchanged"
        );
    }
    assert!(matches!(
        result.draft().input().skill_presets.members[1]
            .intent
            .as_ref()
            .unwrap()
            .usage
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    for row in &result.draft().input().skill_presets.members[1]
        .intent
        .as_ref()
        .unwrap()
        .usage
        .members
    {
        assert_eq!(
            row.applicability,
            PresetApplicability::WhenExactSourceSelected
        );
        let UsageTarget::Skill(SkillTarget::Generated(target)) =
            row.selection.target.to_resolved().unwrap()
        else {
            panic!()
        };
        assert!(target.provider.grant_path.is_empty());
        assert!(
            bindings(&result, 1)
                .members
                .iter()
                .any(|row| row.target.to_resolved().as_ref() == Some(target.as_ref()))
        );
    }
    let old_json = serde_json::to_value(&old.draft().input().skill_presets.members[2]).unwrap();
    assert_eq!(
        old_json,
        serde_json::to_value(&result.draft().input().skill_presets.members[2]).unwrap(),
        "unmatched preset untouched"
    );
}
#[test]
fn generated_usage_does_not_require_raw_quality_policy() {
    let mut f = generated();
    f.policy.generated_skill_inputs = None;
    let result = run(&source().replace("quality=\"12.5\"", "quality=\"bad\""), &f);
    assert_eq!(numbers(&result, 1), [Some(1); 4]);
    assert!(matches!(
        bindings(&result, 1).completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(bindings(&result, 1).members.is_empty());
}
#[test]
fn generated_level_identity_duplicate_and_active_axes_remain_independent_gates() {
    let f = generated();
    for replacement in ["level=\"bad\"", "level=\"1.0\"", "level=\"2\"", ""] {
        let text = source().replace("level=\"1\"", replacement);
        assert_eq!(numbers(&run(&text, &f), 1), [Some(1); 2], "{replacement}");
    }
    let duplicate = source().replace(
        "<SkillSet id=\"2\">",
        &format!(
            "<SkillSet id=\"2\">{}",
            groups("99").replace("level=\"1\"", "level=\"1.0\"")
        ),
    );
    assert!(
        usage(&run(&duplicate, &f), 1).is_empty(),
        "unsupported earlier aliases still compete"
    );
    let selected = source()
        .replace("activeSkillSet=\"2\"", "activeSkillSet=\"1\"")
        .replace("activeSpec=\"2\"", "activeSpec=\"1\"")
        .replace("activeItemSet=\"2\"", "activeItemSet=\"1\"");
    let result = run(&selected, &f);
    assert_eq!(numbers(&result, 0), [Some(1); 4]);
    assert!(usage(&result, 1).is_empty());
}
#[test]
fn present_group_zero_overrides_but_malformed_missing_unknown_never_fall_back() {
    let f = generated();
    for (count, expected) in [
        ("0", Some(0)),
        ("4", Some(4)),
        ("bad", None),
        ("nil", None),
        ("5", None),
        ("1.5", None),
    ] {
        let text = source().replace(
            "<Skill source=",
            &format!("<Skill groupCount=\"{count}\" source="),
        );
        assert_eq!(numbers(&run(&text, &f), 1), [expected; 4], "{count}");
    }
    for text in [
        source().replace("count=\"nil\"", ""),
        source().replace("count=\"nil\"", "count=\"unknown\""),
        source().replace("count=\"nil\"", "count=\"5\""),
    ] {
        assert_eq!(numbers(&run(&text, &f), 1), [None; 4]);
    }
}
#[test]
fn direct_occurrences_bind_independent_presets_and_reject_empty_source_ownership() {
    let (a, mut p) = manual();
    let row = direct::DIRECT.replace(" quality=", " count=\"3\" quality=");
    let text = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill enabled="false">{row}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true" groupCount="0">{row}{row}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = direct::run(&a, &p, &text).unwrap();
    assert_eq!(numbers(&result, 0), [Some(3); 2]);
    assert_eq!(numbers(&result, 1), [Some(0); 4]);
    assert!(result.draft().input().gems.members.is_empty());
    assert_eq!(result.draft().input().skills.members.len(), 3);
    for (index, preset) in result
        .draft()
        .input()
        .skill_presets
        .members
        .iter()
        .enumerate()
    {
        assert!(matches!(
            preset.usage_preferences.as_ref().unwrap().completion,
            DraftListCompletion::Pending { .. }
        ));
        for record in usage(&result, index) {
            let UsageTarget::Skill(SkillTarget::Authored(id)) =
                record.target.to_resolved().unwrap()
            else {
                panic!()
            };
            assert!(preset.skills.members.contains(&id));
        }
    }
    let Some(DirectSkillInputPolicy::PobManualDirectSkillV1 { manual_sources, .. }) =
        &mut p.direct_skill_inputs
    else {
        panic!()
    };
    manual_sources.push(SourceComponent::Text(String::new()));
    if !p
        .manual_skill_sources
        .contains(&SourceComponent::Text(String::new()))
    {
        p.manual_skill_sources
            .push(SourceComponent::Text(String::new()));
    }
    let empty = text.replace("<Skill enabled=", "<Skill source=\"\" enabled=");
    let result = direct::run(&a, &p, &empty).unwrap();
    assert_eq!(
        result.draft().input().skills.members.len(),
        3,
        "legacy raw Direct import retained"
    );
    assert!(usage(&result, 0).is_empty());
    assert!(usage(&result, 1).is_empty());
}
#[test]
fn duplicate_policy_and_correspondence_authority_are_rejected() {
    let f = generated();
    for case in 0..3 {
        let mut p = f.policy.clone();
        let rows = new_rows(&mut p);
        match case {
            0 => {
                let duplicate = rows[0].policies[0].clone();
                rows[0].policies.push(duplicate)
            }
            1 => {
                let duplicate = rows[0].clone();
                rows.push(duplicate)
            }
            _ => {
                rows[0].policies[0].parameters[0].slot =
                    rows[0].policies[1].parameters[0].slot.clone()
            }
        }
        assert!(normalize(&source(), &f, &p, Default::default()).is_err());
    }
}
#[test]
fn new_wire_strictness_identity_and_work_limits_fail_closed() {
    let f = generated();
    let wire = serde_json::to_value(f.policy.usage_inputs.as_ref().unwrap()).unwrap();
    for pointer in [
        "/extra",
        "/occurrences/0/extra",
        "/occurrences/0/target/extra",
        "/occurrences/0/target/correspondence/extra",
        "/occurrences/0/policies/0/extra",
    ] {
        let mut value = wire.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<UsageInputPolicy>(value).is_err(),
            "{pointer}"
        );
    }
    for case in 0..4 {
        let mut p = f.policy.clone();
        let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions,
            source: source_pin,
            roles,
            catalog,
            ..
        }) = &mut p.usage_inputs
        else {
            panic!()
        };
        match case {
            0 => definitions.release = "stale".into(),
            1 => source_pin.revision = "d".repeat(40),
            2 => *roles = *catalog,
            _ => *catalog = *roles,
        };
        assert!(normalize(&source(), &f, &p, Default::default()).is_err());
    }
    let limits = NormalizationLimits {
        max_work: 128,
        ..Default::default()
    };
    assert!(normalize(&source(), &f, &f.policy, limits).is_err());
    assert_eq!(
        run(&source(), &f).draft().input(),
        run(&source(), &f).draft().input()
    );
}

#[test]
fn direct_reviewed_companion_fallback_does_not_accept_duplicate_or_unknown_effects() {
    let (a, mut p) = manual();
    let role = a
        .roles
        .input()
        .roles
        .iter()
        .find(|row| row.role == OwnedGemRole::Known(AuthoredGemRole::SupportAssignment))
        .unwrap();
    let companion = UsageGroupCompanion {
        gem: role.gem.clone(),
        game_id: "support".into(),
        variant_id: "v".into(),
        skill_id: "reviewed-support".into(),
        name_spec: "Reviewed support".into(),
    };
    for policy in &mut new_rows(&mut p)[0].policies {
        let UsageValueSource::ContainingGroupOverride {
            fallback_admission, ..
        } = &mut policy.parameters[0].source
        else {
            panic!()
        };
        *fallback_admission = UsageFallbackAdmission::UniqueReviewedPrimary {
            companions: vec![companion.clone()],
        };
    }
    let row = direct::DIRECT.replace(" quality=", " count=\"3\" quality=");
    let support = r#"<Gem gemId="support" variantId="v" skillId="reviewed-support" nameSpec="Reviewed support" level="1" enabled="false"/>"#;
    let xml = |rows: &str, group: &str| {
        format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill {group}>{rows}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        )
    };
    assert_eq!(
        numbers(
            &direct::run(&a, &p, &xml(&format!("{row}{support}"), "enabled=\"true\"")).unwrap(),
            0
        ),
        [Some(3); 2]
    );
    for sibling in [
        row.clone(),
        support.replace("reviewed-support", "unknown-support"),
        support.replace("/>", "><Unknown/></Gem>"),
    ] {
        let result =
            direct::run(&a, &p, &xml(&format!("{row}{sibling}"), "enabled=\"true\"")).unwrap();
        assert!(numbers(&result, 0).iter().all(Option::is_none));
    }
    let override_xml = xml(&format!("{row}{row}"), "enabled=\"true\" groupCount=\"0\"");
    assert_eq!(
        numbers(&direct::run(&a, &p, &override_xml).unwrap(), 0),
        [Some(0); 4]
    );
}

#[path = "owned_generated_field_accounting.rs"]
mod generated_field_accounting;

fn separate_participation_inputs() -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p) = manual();
    let rows = new_rows(&mut p);
    let slots: Vec<_> = rows[0]
        .policies
        .iter()
        .map(|p| p.parameters[0].slot.clone())
        .collect();
    for (index, policy) in rows[0].policies.iter_mut().enumerate() {
        let value = value_recipe(&format!("participation-{index}"), "enabled", true);
        policy.parameters[0].source = if index == 0 {
            UsageValueSource::Occurrence { value }
        } else {
            UsageValueSource::ContainingGroup { value }
        };
    }
    let rows = rows.clone();
    let mut schema = a.schema.input().clone();
    for entry in &mut schema.slots {
        if let SlotDescriptor::Parameter(row) = entry
            && slots.contains(&row.id)
        {
            let SchemaState::Known(value) = &mut row.schema else {
                panic!()
            };
            value.value = ValueSchema::Boolean;
        }
    }
    rebind_quality_schema(&mut a, &mut p, schema);
    let Some(DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions, roles, ..
    }) = &mut p.direct_skill_inputs
    else {
        panic!()
    };
    *definitions = a.schema.identity().clone();
    *roles = *a.roles.identity();
    install(&a, &mut p, rows);
    (a, p)
}

fn participation_values(result: &NormalizedImport) -> Vec<Option<bool>> {
    usage(result, 0)
        .iter()
        .map(|row| {
            row.parameters.to_resolved().map(|parameters| {
                let ParameterValue::Boolean(value) = parameters[0].value else {
                    panic!()
                };
                value
            })
        })
        .collect()
}

#[test]
fn independent_group_and_occurrence_boolean_inputs_preserve_all_four_combinations() {
    let (a, p) = separate_participation_inputs();
    for gem in [false, true] {
        for group in [false, true] {
            let row = direct::DIRECT.replace("enabled=\"true\"", &format!("enabled=\"{gem}\""));
            let xml = format!(
                r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="{group}">{row}</Skill></SkillSet></Skills></PathOfBuilding2>"#
            );
            let result = direct::run(&a, &p, &xml).unwrap();
            assert_eq!(participation_values(&result), [Some(gem), Some(group)]);
            // Transport never closes the still-unconverted usage inventory.
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
}

#[test]
fn strict_group_inputs_never_fall_back_to_valid_occurrence_values() {
    let (a, p) = separate_participation_inputs();
    for group in [
        "",
        "enabled=\"bad\"",
        "enabled=\"nil\"",
        "enabled=\"1\"",
        "enabled=\" true\"",
    ] {
        let row = direct::DIRECT;
        let xml = format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill {group}>{row}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        );
        let result = direct::run(&a, &p, &xml).unwrap();
        assert_eq!(participation_values(&result), [Some(true), None], "{group}");
    }
    let mut invalid = p.clone();
    new_rows(&mut invalid)[0]
        .group_attributes
        .retain(|name| name != "enabled");
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true">{}</Skill></SkillSet></Skills></PathOfBuilding2>"#,
        direct::DIRECT
    );
    assert!(
        direct::run(&a, &invalid, &xml).is_err(),
        "recipe must use the admitted group frame"
    );
    assert_ne!(
        usage_inputs_identity(&p, Default::default()).unwrap(),
        usage_inputs_identity(&invalid, Default::default()).unwrap()
    );
}
