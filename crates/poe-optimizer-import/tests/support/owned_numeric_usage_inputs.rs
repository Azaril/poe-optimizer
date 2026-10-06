//! Source-bound requested counts; no population, reservation or closure inference.
use super::*;

const GEM: &str = r#"<Gem gemId="active" variantId="v" skillId="synthetic-effect" nameSpec="Synthetic effect" level="17" enabled="true" corrupted="false" count="3" skillMinion="ReviewedActor"/>"#;

fn fixture(quantity: bool) -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p) = usage_input_tests::fixture();
    let original_row = usage_input_tests::row(&mut p).clone();
    let mut schema = a.schema.input().clone();
    let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }));
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::UsagePolicy(row) = definition
            && row.id == original_row.policies[0].policy
        {
            let SchemaState::Known(row) = &mut row.schema else {
                unreachable!()
            };
            row.declarations.parameters.members.truncate(1);
        }
    }
    schema.slots.retain(|entry| !matches!(entry, SlotDescriptor::Parameter(row) if row.id == original_row.policies[0].parameters[1].slot));
    for entry in &mut schema.slots {
        if let SlotDescriptor::Parameter(row) = entry
            && row.id == original_row.policies[0].parameters[0].slot
        {
            let SchemaState::Known(row) = &mut row.schema else {
                unreachable!()
            };
            row.value = if quantity {
                ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(0.0, unit.clone()).unwrap(),
                    maximum: FiniteQuantity::new(20.0, unit.clone()).unwrap(),
                })
            } else {
                ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(20).unwrap(),
                })
            };
        }
    }
    p.gem_inputs.as_mut().unwrap().gems[0].parameters[0].value =
        value_recipe("physical-flag", "corrupted", true);
    rebind_quality_schema(&mut a, &mut p, schema);
    let recipe = |name: &str| {
        let mut recipe = value_recipe(&format!("usage-{}", name.to_ascii_lowercase()), name, false);
        if quantity {
            recipe.codec.codec = ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit: unit.clone(),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            };
        }
        recipe
    };
    p.usage_inputs = Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: a.schema.identity().clone(),
        source: a.roles.input().compilation.source.clone(),
        roles: *a.roles.identity(),
        catalog: a.roles.input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(&p, Default::default()).unwrap(),
        occurrences: vec![],
        physical: vec![PrimarySkillUsageInput {
            gem: original_row.gem,
            game_id: original_row.game_id,
            variant_id: original_row.variant_id,
            skill_id: original_row.skill_id,
            name_spec: original_row.name_spec,
            primary: original_row.primary,
            supply: original_row.supply,
            grant: original_row.grant,
            attributes: [
                "gemId",
                "variantId",
                "skillId",
                "nameSpec",
                "level",
                "enabled",
                "corrupted",
                "count",
                "skillMinion",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            guards: vec![GemInputGuard {
                attribute: "skillMinion".into(),
                allowed: vec![SourceComponent::Text("ReviewedActor".into())],
            }],
            group_attributes: ["enabled", "groupCount", "source"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            group_guards: vec![GemInputGuard {
                attribute: "source".into(),
                allowed: vec![SourceComponent::Missing],
            }],
            policies: vec![OccurrenceUsagePolicy {
                policy: original_row.policies[0].policy.clone(),
                parameters: vec![UsageParameterInput {
                    slot: original_row.policies[0].parameters[0].slot.clone(),
                    source: UsageValueSource::ContainingGroupOverride {
                        group: Box::new(recipe("groupCount")),
                        occurrence: recipe("count"),
                        fallback_admission: UsageFallbackAdmission::RequestedOccurrence,
                    },
                }],
            }],
        }],
    });
    (a, p)
}
fn row(p: &mut NormalizationPolicy) -> &mut PrimarySkillUsageInput {
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } = p.usage_inputs.as_mut().unwrap();
    &mut physical[0]
}
fn xml(gem: &str, group_attributes: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="7"><SkillSet id="7"><Skill enabled="true" {group_attributes}>{gem}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}
fn run(xml: &str, a: &Artifacts, p: &NormalizationPolicy) -> NormalizedImport {
    usage_input_tests::run_limits(xml, a, p, Default::default()).unwrap()
}
fn preferences(result: &NormalizedImport) -> &DraftList<UsagePolicyDraft> {
    result.draft().input().skill_presets.members[0]
        .usage_preferences
        .as_ref()
        .unwrap()
}
fn integer(record: &UsagePolicyDraft) -> Option<i64> {
    match record.parameters.members.first()?.value.to_resolved()? {
        ParameterValue::Integer(value) => Some(value.get()),
        _ => None,
    }
}

#[test]
fn strict_group_numeric_source_requires_its_own_saved_value() {
    let (a, mut p) = fixture(false);
    let UsageValueSource::ContainingGroupOverride { group, .. } =
        row(&mut p).policies[0].parameters[0].source.clone()
    else {
        panic!()
    };
    row(&mut p).policies[0].parameters[0].source =
        UsageValueSource::ContainingGroup { value: *group };
    for (attributes, expected) in [
        ("groupCount=\"0\"", Some(0)),
        ("groupCount=\"4\"", Some(4)),
        ("", None),
        ("groupCount=\"bad\"", None),
    ] {
        let result = run(&xml(GEM, attributes), &a, &p);
        assert_eq!(
            integer(&preferences(&result).members[0]),
            expected,
            "{attributes}"
        );
    }
}

#[test]
fn numeric_usage_override_zero_and_absence_are_distinct_without_source_defaults() {
    let (a, p) = fixture(false);
    for (group, gem, expected) in [
        ("", GEM.to_owned(), Some(3)),
        (r#"groupCount="0""#, GEM.to_owned(), Some(0)),
        (r#"groupCount="4""#, GEM.to_owned(), Some(4)),
        (
            r#"groupCount="0""#,
            GEM.replace("count=\"3\"", "count=\"bad\""),
            Some(0),
        ),
        (r#"groupCount="bad""#, GEM.to_owned(), None),
        (r#"groupCount="""#, GEM.to_owned(), None),
        (r#"groupCount=" 4""#, GEM.to_owned(), None),
        (r#"groupCount="21""#, GEM.to_owned(), None),
        ("", GEM.replace(" count=\"3\"", ""), None),
        ("", GEM.replace("count=\"3\"", "count=\"-1\""), None),
        ("", GEM.replace("count=\"3\"", "count=\"3.5\""), None),
    ] {
        let result = run(&xml(&gem, group), &a, &p);
        let usage = preferences(&result);
        assert!(matches!(
            usage.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(usage.members.len(), 1, "{group} {gem}");
        assert_eq!(integer(&usage.members[0]), expected, "{group} {gem}");
        assert_eq!(
            matches!(
                usage.members[0].parameters.completion,
                DraftListCompletion::Complete
            ),
            expected.is_some()
        );
        assert!(matches!(
            result.draft().input().gems.members[0].parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    assert!(decode_build(xml(GEM, r#"groupCount="0" groupCount="4""#).as_bytes()).is_err());
}

#[test]
fn numeric_usage_preserves_exact_occurrences_presets_disabled_inputs_and_parent_provenance() {
    let (a, mut p) = fixture(false);
    let target = row(&mut p).supply.clone();
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="8"><SkillSet id="7"><Skill enabled="false" groupCount="0">{GEM}{GEM}</Skill></SkillSet><SkillSet id="8"><Skill enabled="true">{GEM}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = run(&xml, &a, &p);
    let draft = result.draft().input();
    assert_eq!(draft.skills.members.len(), 3);
    for (preset, values) in draft
        .skill_presets
        .members
        .iter()
        .zip([vec![0, 0], vec![3]])
    {
        let preferences = preset.usage_preferences.as_ref().unwrap();
        assert_eq!(preferences.members.len(), values.len());
        for ((usage, skill), expected) in preferences
            .members
            .iter()
            .zip(&preset.skills.members)
            .zip(values)
        {
            assert_eq!(integer(usage), Some(expected));
            assert_eq!(
                usage.target.to_resolved(),
                Some(UsageTarget::Skill(SkillTarget::Generated(Box::new(
                    GeneratedSkillKey {
                        provider: ProviderKey {
                            root: ProviderRoot::SkillUse(*skill),
                            grant_path: vec![]
                        },
                        slot: target.clone(),
                    }
                ))))
            );
        }
    }
    assert_eq!(draft.skills.members[0].enabled.to_resolved(), Some(false));
    let source = source(&xml, 93);
    for origin in &result.sidecar().origins {
        if source.occurrence(origin.source).unwrap().name() == "Skill" {
            assert_eq!(
                origin
                    .links
                    .iter()
                    .filter(|link| matches!(link, OwnedOriginTarget::SkillPreset(_)))
                    .count(),
                1
            );
        }
    }
    origin_integrity(&source, &result);
}

#[test]
fn numeric_usage_unknown_frames_and_minion_facts_cannot_be_hidden_by_override() {
    let (a, p) = fixture(false);
    for raw in [
        xml(
            &GEM.replace("ReviewedActor", "UnknownActor"),
            r#"groupCount="0""#,
        ),
        xml(&GEM.replace("/>", " extra=\"1\"/>"), ""),
        xml(&GEM.replace("/>", "><Skill/></Gem>"), ""),
        xml(GEM, r#"source="Tree:unreviewed""#),
        xml(GEM, r#"unknown="1""#),
        xml(GEM, "").replace(
            "<Skills activeSkillSet=\"7\">",
            "<Skills activeSkillSet=\"99\">",
        ),
    ] {
        let result = run(&raw, &a, &p);
        assert!(
            result
                .draft()
                .input()
                .skill_presets
                .members
                .iter()
                .all(|preset| preset
                    .usage_preferences
                    .as_ref()
                    .is_none_or(|usage| usage.members.is_empty())),
            "{raw}"
        );
    }
    // RequestedOccurrence reads its admitted local row, not another group's inventory.
    let unrelated = xml(GEM, "").replace(
        "</SkillSet>",
        "<Skill enabled=\"true\"><Unexpected/></Skill></SkillSet>",
    );
    let result = run(&unrelated, &a, &p);
    assert_eq!(preferences(&result).members.len(), 1);
    assert_eq!(integer(&preferences(&result).members[0]), Some(3));
    assert!(matches!(
        preferences(&result).members[0].parameters.completion,
        DraftListCompletion::Complete
    ));
    assert!(matches!(
        preferences(&result).completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn numeric_usage_quantity_preserves_fraction_and_checks_unit_range() {
    let (a, p) = fixture(true);
    let result = run(
        &xml(&GEM.replace("count=\"3\"", "count=\"3.5\""), ""),
        &a,
        &p,
    );
    let value = preferences(&result).members[0].parameters.members[0]
        .value
        .to_resolved()
        .unwrap();
    let ParameterValue::Quantity(value) = value else {
        panic!("typed quantity")
    };
    assert_eq!(value.value(), 3.5);
    let outside = run(&xml(GEM, r#"groupCount="20.1""#), &a, &p);
    assert!(
        preferences(&outside).members[0]
            .parameters
            .members
            .is_empty()
    );
    let mut bad = p.clone();
    let UsageValueSource::ContainingGroupOverride { group, .. } =
        &mut row(&mut bad).policies[0].parameters[0].source
    else {
        unreachable!()
    };
    group.codec.codec = ValueCodecKind::Integer {
        syntax: DecimalSyntax::Integer,
    };
    assert!(usage_input_tests::run_limits(&xml(GEM, ""), &a, &bad, Default::default()).is_err());
}

#[test]
fn numeric_usage_authoring_rejects_wrong_owners_recipes_frames_and_stale_bindings() {
    let (a, p) = fixture(false);
    for case in 0..17 {
        let mut bad = p.clone();
        let rule = row(&mut bad);
        match case {
            0 => {
                rule.policies[0].parameters[0].slot.declaration =
                    SlotOwnerDefId::Gem(rule.gem.clone())
            }
            1 => rule.supply.declaration = SlotOwnerDefId::Skill(rule.primary.clone()),
            2 => rule.group_attributes.push("groupCount".into()),
            3 => rule.group_guards[0].allowed.clear(),
            4 => {
                let duplicate = rule.policies[0].parameters[0].clone();
                rule.policies[0].parameters.push(duplicate);
            }
            5 => rule.game_id = "unknown".into(),
            6 => rule.group_attributes.clear(),
            7..=15 => {
                let UsageValueSource::ContainingGroupOverride { group, .. } =
                    &mut rule.policies[0].parameters[0].source
                else {
                    unreachable!()
                };
                match case {
                    7 => group.codec.namespace = GameVersionNamespace::new("other", "v1").unwrap(),
                    8 => group.codec.whitespace = WhitespacePolicy::TrimAscii,
                    9 => group.missing = MissingValuePolicy::Absent,
                    10 => group.numeric_aliases.push(NumericTokenAlias {
                        token: "nil".into(),
                        replacement: "0".into(),
                    }),
                    11 => group.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
                    12 => group.tiers[0].selectors[0].lane = ValueLane::ParentAttribute,
                    13 => group.tiers.push(group.tiers[0].clone()),
                    14 => group.tiers[0].selectors[0].name = "unlisted".into(),
                    15 => group.codec.codec = value_recipe("flag", "flag", true).codec.codec,
                    _ => unreachable!(),
                }
            }
            16 => rule
                .attributes
                .extend((0..64).map(|n| format!("extra-{n}"))),
            _ => unreachable!(),
        }
        assert!(
            usage_input_tests::run_limits(&xml(GEM, ""), &a, &bad, Default::default()).is_err(),
            "case {case}"
        );
    }
    for case in 0..5 {
        let mut bad = p.clone();
        let UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            physical,
            ..
        } = bad.usage_inputs.as_mut().unwrap();
        let wrong = "0".repeat(64).parse().unwrap();
        match case {
            0 => definitions.release = "stale".into(),
            1 => *roles = wrong,
            2 => *catalog = wrong,
            3 => *scalar_inputs = wrong,
            4 => physical.push(physical[0].clone()),
            _ => unreachable!(),
        }
        assert!(
            usage_input_tests::run_limits(&xml(GEM, ""), &a, &bad, Default::default()).is_err()
        );
    }
    let mut wire = serde_json::to_value(p.usage_inputs.as_ref().unwrap()).unwrap();
    wire["physical"][0]["policies"][0]["parameters"][0]["source"]["fallback_default"] = 1.into();
    assert!(serde_json::from_value::<UsageInputPolicy>(wire).is_err());
}

#[test]
fn numeric_usage_bounded_work_is_atomic_and_tighter_caller_limits_remain_effective() {
    let (a, p) = fixture(false);
    let xml = xml(GEM, r#"groupCount="4""#);
    let expected = run(&xml, &a, &p);
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        let result = usage_input_tests::run_limits(
            &xml,
            &a,
            &p,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        );
        if result.is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(
        usage_input_tests::run_limits(
            &xml,
            &a,
            &p,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let exact = usage_input_tests::run_limits(
        &xml,
        &a,
        &p,
        NormalizationLimits {
            max_work: low,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(exact.draft().input()).unwrap(),
        serde_json::to_vec(expected.draft().input()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(exact.sidecar()).unwrap(),
        serde_json::to_vec(expected.sidecar()).unwrap()
    );
    let mut limits = NormalizationLimits::default();
    // Shared recipe accounting includes selector bytes as well as token bytes.
    // The unchanged physical `corrupted=false` control needs exactly 9+5 bytes.
    limits.value.max_total_candidate_bytes = "corrupted".len() + "false".len();
    usage_input_tests::run_limits(&xml, &a, &p, limits).unwrap();
    let oversized = xml.replace("groupCount=\"4\"", "groupCount=\"12345\"");
    assert_eq!(
        "groupCount".len() + "12345".len(),
        limits.value.max_total_candidate_bytes + 1
    );
    assert!(matches!(
        usage_input_tests::run_limits(&oversized, &a, &p, limits),
        Err(NormalizationError::Value(
            ValuePolicyError::ResourceLimit { resource: ValuePolicyResource::TotalCandidateBytes, maximum }
        )) if maximum == limits.value.max_total_candidate_bytes
    ));
}

fn unique_companions(a: &Artifacts, p: &mut NormalizationPolicy) -> UsageGroupCompanion {
    let support = a
        .roles
        .input()
        .roles
        .iter()
        .find(|row| row.role == OwnedGemRole::Known(AuthoredGemRole::SupportAssignment))
        .unwrap();
    let companion = UsageGroupCompanion {
        gem: support.gem.clone(),
        game_id: "support".into(),
        variant_id: "v".into(),
        skill_id: "synthetic-support".into(),
        name_spec: "Synthetic support".into(),
    };
    let UsageValueSource::ContainingGroupOverride {
        fallback_admission, ..
    } = &mut row(p).policies[0].parameters[0].source
    else {
        unreachable!()
    };
    *fallback_admission = UsageFallbackAdmission::UniqueReviewedPrimary {
        companions: vec![companion.clone()],
    };
    companion
}
const COMPANION: &str = r#"<Gem gemId="support" variantId="v" skillId="synthetic-support" nameSpec="Synthetic support" level="1" enabled="true"/>"#;

#[test]
fn numeric_usage_unique_fallback_requires_reviewed_companions_even_when_disabled() {
    let (a, mut p) = fixture(false);
    unique_companions(&a, &mut p);
    let good = run(&xml(&format!("{GEM}{COMPANION}{COMPANION}"), ""), &a, &p);
    assert_eq!(integer(&preferences(&good).members[0]), Some(3));
    for companion in [
        COMPANION.replace("synthetic-support", "different-effect"),
        COMPANION.replace("Synthetic support", "Unknown name"),
        COMPANION.replace("gemId=\"support\"", "gemId=\"unknown\""),
        GEM.replace("enabled=\"true\"", "enabled=\"false\""),
        GEM.to_string(),
    ] {
        let raw = format!("{GEM}{companion}");
        let rejected = run(&xml(&raw, ""), &a, &p);
        assert!(!preferences(&rejected).members.is_empty());
        assert!(
            preferences(&rejected)
                .members
                .iter()
                .all(|usage| integer(usage).is_none()
                    && matches!(
                        usage.parameters.completion,
                        DraftListCompletion::Pending { .. }
                    )),
            "{companion}"
        );
        let overridden = run(&xml(&raw, r#"groupCount="0""#), &a, &p);
        assert!(
            preferences(&overridden)
                .members
                .iter()
                .all(|usage| integer(usage) == Some(0))
        );
        let malformed = run(&xml(&raw, r#"groupCount="bad""#), &a, &p);
        assert!(
            preferences(&malformed)
                .members
                .iter()
                .all(|usage| integer(usage).is_none())
        );
    }
    // A different containing group never contaminates this proof.
    let separate = xml(GEM, "").replace(
        "</SkillSet>",
        &format!("<Skill enabled=\"false\">{GEM}</Skill></SkillSet>"),
    );
    let result = run(&separate, &a, &p);
    assert_eq!(preferences(&result).members.len(), 2);
    assert!(
        preferences(&result)
            .members
            .iter()
            .all(|usage| integer(usage) == Some(3))
    );
}

#[test]
fn numeric_usage_unique_fallback_rejects_self_duplicate_foreign_and_unreviewed_companions() {
    let (a, p) = fixture(false);
    for case in 0..7 {
        let mut bad = p.clone();
        let companion = unique_companions(&a, &mut bad);
        let rule = row(&mut bad);
        let self_companion = UsageGroupCompanion {
            gem: rule.gem.clone(),
            game_id: rule.game_id.clone(),
            variant_id: rule.variant_id.clone(),
            skill_id: rule.skill_id.clone(),
            name_spec: rule.name_spec.clone(),
        };
        let UsageValueSource::ContainingGroupOverride {
            fallback_admission: UsageFallbackAdmission::UniqueReviewedPrimary { companions },
            ..
        } = &mut rule.policies[0].parameters[0].source
        else {
            unreachable!()
        };
        match case {
            0 => companions.push(companion),
            1 => companions[0] = self_companion,
            2 => companions[0].game_id = "unknown".into(),
            3 => companions[0].variant_id.clear(),
            4 => {
                companions[0].name_spec =
                    "x".repeat(OwnedMappingLimits::default().max_string_bytes + 1)
            }
            5 => companions.extend(std::iter::repeat_n(companion, 64)),
            6 => companions[0].gem = self_companion.gem,
            _ => unreachable!(),
        }
        assert!(
            usage_input_tests::run_limits(&xml(GEM, ""), &a, &bad, Default::default()).is_err(),
            "case {case}"
        );
    }
    let mut restricted = p.clone();
    unique_companions(&a, &mut restricted);
    let UsageValueSource::ContainingGroupOverride {
        fallback_admission: UsageFallbackAdmission::UniqueReviewedPrimary { companions },
        ..
    } = &mut row(&mut restricted).policies[0].parameters[0].source
    else {
        unreachable!()
    };
    companions.clear();
    let result = run(&xml(&format!("{GEM}{COMPANION}"), ""), &a, &restricted);
    assert!(
        preferences(&result).members[0]
            .parameters
            .members
            .is_empty()
    );
    let mut wire = serde_json::to_value(restricted.usage_inputs.as_ref().unwrap()).unwrap();
    wire["physical"][0]["policies"][0]["parameters"][0]["source"]
        .as_object_mut()
        .unwrap()
        .remove("fallback_admission");
    assert!(serde_json::from_value::<UsageInputPolicy>(wire).is_err());
}

#[test]
fn numeric_usage_raw_occurrence_does_not_claim_group_or_effect_matching() {
    let (a, mut p) = fixture(false);
    let source = &mut row(&mut p).policies[0].parameters[0].source;
    let UsageValueSource::ContainingGroupOverride { occurrence, .. } = source else {
        unreachable!()
    };
    *source = UsageValueSource::Occurrence {
        value: occurrence.clone(),
    };
    let other = GEM.replace("count=\"3\"", "count=\"5\"");
    let result = run(&xml(&format!("{GEM}{other}"), r#"groupCount="0""#), &a, &p);
    assert_eq!(
        preferences(&result)
            .members
            .iter()
            .map(integer)
            .collect::<Vec<_>>(),
        vec![Some(3), Some(5)]
    );
}
