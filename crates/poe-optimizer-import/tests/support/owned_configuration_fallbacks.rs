//! Saved fallback values retain their exact source and never complete a scenario.
use super::*;

pub(super) fn fallback_policy(f: &Fixture) -> NormalizationPolicy {
    let mut policy = reviewed(f);
    let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
        mapping_source,
        encounter,
        mut inputs,
    }) = policy.configuration_inputs.take()
    else {
        panic!("expected historical fixture")
    };
    let mut fallback = inputs.pop().unwrap();
    let mut tier = fallback.recipe.tiers[0].clone();
    tier.selectors[0].lane = ValueLane::PlaceholderNumber;
    fallback.recipe.tiers.push(tier);
    policy.configuration_inputs = Some(
        ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
            mapping_source,
            encounter,
            inputs,
            placeholder_fallback_inputs: vec![fallback],
        },
    );
    policy
}

fn quantity(f: &Fixture, value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, f.unit.clone()).unwrap())
}

#[test]
fn fallback_selects_input_including_zero_and_exact_winning_source_in_either_order() {
    let f = fixture();
    for (body, expected, winner) in [
        ("", None, None),
        (
            r#"<Placeholder name="raw-1" number="37"/>"#,
            Some(37.),
            Some("Placeholder"),
        ),
        (
            r#"<Placeholder name="raw-1" number="0"/>"#,
            Some(0.),
            Some("Placeholder"),
        ),
        (
            r#"<Input name="raw-1" number="-17.25"/>"#,
            Some(-17.25),
            Some("Input"),
        ),
        (
            r#"<Input name="raw-1" number="0"/><Placeholder name="raw-1" number="37"/>"#,
            Some(0.),
            Some("Input"),
        ),
        (
            r#"<Placeholder name="raw-1" number="37"/><Input name="raw-1" number="0"/>"#,
            Some(0.),
            Some("Input"),
        ),
        (
            r#"<Input name="raw-1" number="1e2"/><Placeholder name="raw-1" number="-9"/>"#,
            Some(100.),
            Some("Input"),
        ),
    ] {
        let imported = source(&xml(body), 0x91);
        let before = normalize(&imported, &f, &reviewed(&f), Default::default()).unwrap();
        let after = normalize(&imported, &f, &fallback_policy(&f), Default::default()).unwrap();
        let mut wanted = vec![
            (f.inputs[0].0.clone(), ParameterValue::Boolean(false)),
            (
                f.inputs[1].0.clone(),
                ParameterValue::Boolean(expected.is_some()),
            ),
        ];
        if let Some(expected) = expected {
            wanted.push((f.inputs[1].1.clone(), quantity(&f, expected)));
        }
        assert_eq!(values(&after, 0), wanted, "{body}");
        let scenario = after.draft().input().scenario_presets.members[0].id;
        let mut restored = after.draft().input().clone();
        restored.scenario_presets.members[0]
            .scenario
            .assumptions
            .members = before.draft().input().scenario_presets.members[0]
            .scenario
            .assumptions
            .members
            .clone();
        assert_eq!(restored, *before.draft().input());
        assert_eq!(after.allocator_after(), before.allocator_after());
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        let mut expected_origins = before.sidecar().origins.clone();
        for occurrence in imported
            .occurrences()
            .iter()
            .filter(|row| matches!(row.name(), "Input" | "Placeholder"))
        {
            // The raw-only predecessor accounts its overwritten placeholder.
            // The fallback policy consumes that lane, retaining its obligation.
            if occurrence.name() == "Placeholder" {
                let expected = expected_origins
                    .iter_mut()
                    .find(|row| row.source == occurrence.id())
                    .unwrap();
                let DraftListCompletion::Pending { id, .. } =
                    after.draft().input().choice_presets.members[0]
                        .choices
                        .completion
                else {
                    panic!()
                };
                expected.disposition = SourceDisposition::Contributes;
                expected.links = vec![OwnedOriginTarget::Issue(id)];
            }
            let original = before
                .sidecar()
                .origins
                .iter()
                .find(|row| row.source == occurrence.id())
                .unwrap();
            let actual = after
                .sidecar()
                .origins
                .iter()
                .find(|row| row.source == occurrence.id())
                .unwrap();
            let link = OwnedOriginTarget::ScenarioPreset(scenario);
            if winner == Some(occurrence.name()) {
                let expected = expected_origins
                    .iter_mut()
                    .find(|row| row.source == occurrence.id())
                    .unwrap();
                if !expected.links.contains(&link) {
                    expected.links.push(link.clone());
                }
                assert!(actual.links.contains(&link), "winning row {body}");
            } else if occurrence.name() != "Placeholder" {
                assert_eq!(actual.links, original.links, "losing row {body}");
            }
        }
        assert_eq!(
            serde_json::to_value(&after.sidecar().origins).unwrap(),
            serde_json::to_value(expected_origins).unwrap()
        );
        let mut sidecar = serde_json::to_value(after.sidecar()).unwrap();
        let old = serde_json::to_value(before.sidecar()).unwrap();
        for field in ["policy", "draft", "origins"] {
            sidecar[field] = old[field].clone();
        }
        assert_eq!(sidecar, old);
    }
}

#[test]
fn malformed_or_ambiguous_saved_fallbacks_never_establish_absence() {
    let f = fixture();
    for body in [
        r#"<Placeholder name="raw-1" string="37"/>"#,
        r#"<Input name="raw-1" boolean="false"/><Placeholder name="raw-1" number="37"/>"#,
        r#"<Input name="raw-1" number="NaN"/><Placeholder name="raw-1" number="37"/>"#,
        r#"<Input name="raw-1" number="1"/><Placeholder name="raw-1" number="bad"/>"#,
        r#"<Input name="raw-1" number="1"/><Placeholder name="raw-1" number="100.1"/>"#,
        r#"<Placeholder name="raw-1" number="-100.1"/>"#,
        r#"<Placeholder name="raw-1" number=" 37"/>"#,
        r#"<Placeholder name="raw-1" number="37"/><Placeholder name="raw-1" number="38"/>"#,
        r#"<Input name="raw-1" number="1"/><Input name="raw-1" number="2"/>"#,
        r#"<Placeholder name="raw-1" number="37"><Future/></Placeholder>"#,
        r#"<Placeholder xmlns:q="future" name="raw-1" number="37"/>"#,
    ] {
        let imported = source(&xml(body), 0x92);
        let after = normalize(&imported, &f, &fallback_policy(&f), Default::default()).unwrap();
        assert!(
            values(&after, 0)
                .iter()
                .all(|(id, _)| id != &f.inputs[1].0 && id != &f.inputs[1].1),
            "{body}"
        );
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn fallback_is_scoped_to_exact_configuration_and_matching_encounter() {
    let f = fixture();
    let imported = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Placeholder name="raw-1" number="37"/></ConfigSet><ConfigSet id="2"><Input name="raw-1" number="0"/><Placeholder name="raw-1" number="61"/></ConfigSet><ConfigSet id="3"><Input name="encounter-selector" string="other"/><Placeholder name="raw-1" number="92"/></ConfigSet></Config></PathOfBuilding2>"#,
        0x93,
    );
    let after = normalize(&imported, &f, &fallback_policy(&f), Default::default()).unwrap();
    for (index, raw) in [(0, 37.), (1, 0.)] {
        assert_eq!(
            values(&after, index),
            vec![
                (f.inputs[0].0.clone(), ParameterValue::Boolean(false)),
                (f.inputs[1].0.clone(), ParameterValue::Boolean(true)),
                (f.inputs[1].1.clone(), quantity(&f, raw))
            ]
        );
    }
    assert!(values(&after, 2).is_empty());
}

#[test]
fn fallback_policy_rejects_stale_bindings_cross_lane_aliases_and_unreviewed_recipes() {
    let f = fixture();
    let imported = source(&xml(""), 0x94);
    for mutation in 0..10 {
        let mut p = fallback_policy(&f);
        let Some(ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
            mapping_source,
            inputs,
            placeholder_fallback_inputs,
            ..
        }) = &mut p.configuration_inputs
        else {
            panic!()
        };
        match mutation {
            0 => *mapping_source = *f.a.mapping.identity(),
            1 => placeholder_fallback_inputs[0].presence_input = inputs[0].presence_input.clone(),
            2 => placeholder_fallback_inputs[0].source_name = inputs[0].source_name.clone(),
            3 => placeholder_fallback_inputs[0].recipe.id = inputs[0].recipe.id.clone(),
            4 => placeholder_fallback_inputs[0].recipe.tiers.reverse(),
            5 => {
                placeholder_fallback_inputs[0].recipe.tiers[1].duplicates =
                    DuplicatePolicy::LastInSourceOrder
            }
            6 => {
                placeholder_fallback_inputs[0].recipe.tiers[1].selectors[0].lane =
                    ValueLane::PlaceholderString
            }
            7 => placeholder_fallback_inputs[0].recipe.missing = MissingValuePolicy::Absent,
            8 => {
                inputs.clear();
                placeholder_fallback_inputs.clear();
                *mapping_source = *f.a.mapping.identity();
            }
            9 => placeholder_fallback_inputs[0].recipe.tiers[1].selectors[0].name = "other".into(),
            _ => unreachable!(),
        }
        assert!(
            normalize(&imported, &f, &p, Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn fallback_resource_limits_include_both_selectors_and_decoded_values() {
    let f = fixture();
    let imported = source(&xml(r#"<Placeholder name="raw-1" number="17.25"/>"#), 0x95);
    for mutation in 0..5 {
        let mut limits = NormalizationLimits::default();
        match mutation {
            0 => limits.value.max_selectors = 2,
            1 => limits.value.max_total_selector_bytes = 12,
            2 => limits.value.value.max_source_bytes = 4,
            3 => limits.draft.input.max_collection_entries = 3,
            4 => limits.max_work = 1,
            _ => unreachable!(),
        }
        assert!(
            normalize(&imported, &f, &fallback_policy(&f), limits).is_err(),
            "limit {mutation}"
        );
    }
}

#[test]
fn empty_v2_fallback_list_keeps_v1_facts_and_wire_contract_separate() {
    let f = fixture();
    let v1 = reviewed(&f);
    let v1_wire = serde_json::to_value(&v1).unwrap();
    assert_eq!(
        v1_wire["configuration_inputs"]["kind"],
        "pob_fresh_numeric_config_overrides_v1"
    );
    assert!(
        v1_wire["configuration_inputs"]
            .get("placeholder_fallback_inputs")
            .is_none()
    );
    let mut v2 = v1.clone();
    let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
        mapping_source,
        encounter,
        inputs,
    }) = v2.configuration_inputs.take()
    else {
        panic!()
    };
    v2.configuration_inputs = Some(
        ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
            mapping_source,
            encounter,
            inputs,
            placeholder_fallback_inputs: vec![],
        },
    );
    let imported = source(&xml(r#"<Placeholder name="raw-1" number="37"/>"#), 0x96);
    let old = normalize(&imported, &f, &v1, Default::default()).unwrap();
    let new = normalize(&imported, &f, &v2, Default::default()).unwrap();
    assert_eq!(old.draft(), new.draft());
    assert_eq!(old.allocator_after(), new.allocator_after());
    let mut new_sidecar = serde_json::to_value(new.sidecar()).unwrap();
    let old_sidecar = serde_json::to_value(old.sidecar()).unwrap();
    new_sidecar["policy"] = old_sidecar["policy"].clone();
    assert_eq!(new_sidecar, old_sidecar);
    assert_eq!(serde_json::to_value(v1).unwrap(), v1_wire);
}
