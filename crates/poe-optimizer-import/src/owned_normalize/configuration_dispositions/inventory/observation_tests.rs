//! Private proof-consumer controls. These synthetic records are not production
//! source authority; integration tests separately exercise the real producer.
use super::*;

fn scenario(local: u64) -> ScenarioPresetId {
    ScenarioPresetId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([39; 16]), local).unwrap(),
    )
}

fn with_observation(
    test: impl FnOnce(
        &mut Builder<'_, '_>,
        SourceOccurrenceId,
        CompiledInventory,
        accounting::CheckedInapplicable,
    ),
) {
    let xml = r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills><Config activeConfigSet="1"><ConfigSet id="1"><Placeholder name="reviewed" number="0"/></ConfigSet><ConfigSet id="2"><Placeholder name="reviewed" number="0"/></ConfigSet></Config></PathOfBuilding2>"#;
    gem_inventory::with_usage_inventory_destination(xml, |b, _, _, _, _| {
        let scope = b
            .evidence
            .rows()
            .iter()
            .find(|r| r.occurrence().name() == "ConfigSet")
            .unwrap()
            .occurrence()
            .id();
        let origin = b.evidence.rows()[scope.ordinal() as usize].children()[0];
        let selector = ValueSelector {
            lane: ValueLane::PlaceholderNumber,
            name: "reviewed".into(),
        };
        let rule = key("reviewed-branch");
        let reason = key("checked-inapplicable");
        b.origins[origin.ordinal() as usize].links =
            vec![OwnedOriginTarget::ScenarioPreset(scenario(1))];
        b.origins[origin.ordinal() as usize].disposition =
            SourceDisposition::SourceOnly(reason.clone());
        let inventory = CompiledInventory {
            complete: true,
            strict_empty_custom_modifiers: false,
            controls: vec![(
                ConfigurationInventoryControl {
                    id: key("source-control"),
                    selector: Some(selector.clone()),
                    source_states: vec![ConfigurationSourceState::Placeholder],
                },
                ConfigurationInventoryTarget::Inapplicable { rule: rule.clone() },
            )],
        };
        test(
            b,
            scope,
            inventory,
            accounting::CheckedInapplicable {
                scope,
                origin,
                scenario: scenario(1),
                rule,
                selector,
                reason,
            },
        );
    });
}

#[test]
fn observation_consumer_rechecks_exact_current_authority_and_single_use() {
    for mutation in 0..17 {
        with_observation(|b, scope, mut inventory, mut receipt| {
            let origin = receipt.origin;
            match mutation {
                0 => {}
                1 => {
                    receipt.scope = b.evidence.rows()[scope.ordinal() as usize]
                        .occurrence()
                        .parent()
                        .unwrap()
                }
                2 => receipt.scenario = scenario(2),
                3 => receipt.origin = scope,
                4 => receipt.rule = key("foreign-rule"),
                5 => receipt.selector.name = "other".into(),
                6 => receipt.reason = key("other-reason"),
                7 => {
                    b.origins[origin.ordinal() as usize].disposition =
                        SourceDisposition::Contributes
                }
                8 => {
                    b.origins[origin.ordinal() as usize].disposition =
                        SourceDisposition::SourceOnly(key("other-reason"))
                }
                9 => b.origins[origin.ordinal() as usize]
                    .links
                    .push(OwnedOriginTarget::ScenarioPreset(scenario(2))),
                10 => b.origins[origin.ordinal() as usize]
                    .links
                    .push(OwnedOriginTarget::ScenarioPreset(scenario(1))),
                11 => b.origins[origin.ordinal() as usize].links.clear(),
                12 => {
                    inventory.controls[0].0.source_states = vec![ConfigurationSourceState::Absent]
                }
                13 => inventory.controls.clear(),
                14 => inventory.complete = false,
                15 => inventory.controls.push(inventory.controls[0].clone()),
                16 => receipt.selector.lane = ValueLane::InputNumber,
                _ => unreachable!(),
            }
            let before = b.origins.clone();
            let allocator = b.allocator.state();
            let result =
                match_receipts(b, scope, scenario(1), &inventory, &[], &[receipt], None).unwrap();
            assert_eq!(result, mutation == 0, "mutation {mutation}");
            assert_eq!(b.origins, before);
            assert_eq!(b.allocator.state(), allocator);
        });
    }
}

#[test]
fn observation_receipts_cannot_be_missing_duplicated_or_borrowed_from_another_set() {
    for mutation in 0..4 {
        with_observation(|b, scope, inventory, receipt| {
            let mut observations = vec![receipt];
            match mutation {
                0 => observations.clear(),
                1 | 2 => {
                    let r = &observations[0];
                    let mut duplicate = accounting::CheckedInapplicable {
                        scope: r.scope,
                        origin: r.origin,
                        scenario: r.scenario,
                        rule: r.rule.clone(),
                        selector: r.selector.clone(),
                        reason: r.reason.clone(),
                    };
                    if mutation == 2 {
                        duplicate.rule = key("surplus-rule");
                    }
                    observations.push(duplicate);
                }
                3 => {
                    let foreign = b
                        .evidence
                        .rows()
                        .iter()
                        .filter(|r| r.occurrence().name() == "ConfigSet")
                        .nth(1)
                        .unwrap();
                    observations[0].scope = foreign.occurrence().id();
                    observations[0].origin = foreign.children()[0];
                }
                _ => unreachable!(),
            }
            assert!(
                !match_receipts(b, scope, scenario(1), &inventory, &[], &observations, None)
                    .unwrap(),
                "mutation {mutation}"
            );
        });
    }
}

#[test]
fn observation_consumer_has_an_exact_work_limit_without_mutations() {
    with_observation(|b, scope, inventory, receipt| {
        let observations = [receipt];
        let start = b.work;
        assert!(
            match_receipts(b, scope, scenario(1), &inventory, &[], &observations, None).unwrap()
        );
        let required = b.work;
        let before = b.origins.clone();
        for sufficient in [true, false] {
            b.work = start;
            b.limits.max_work = required - usize::from(!sufficient);
            let result =
                match_receipts(b, scope, scenario(1), &inventory, &[], &observations, None);
            if sufficient {
                assert!(result.unwrap());
            } else {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
            }
            assert_eq!(b.origins, before);
        }
    });
}
