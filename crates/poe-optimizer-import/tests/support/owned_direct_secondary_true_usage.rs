//! Public normalizer controls using the actual manual Direct disposition producer.
use super::*;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::*, owned_reward_policy::OwnedRewardPolicy,
    owned_skill_roles::OwnedSkillRoleIndex,
};

fn configured() -> Fixture {
    let mut f = Fixture::new();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { skills, .. } =
        f.base.policy.direct_skill_inputs.as_ref().unwrap();
    let direct = skills[0].clone();
    let count = f.row().deferred_usage[0].value.clone();
    let ValueCodecKind::Quantity { unit, .. } = &count.codec.codec else {
        panic!()
    };
    let a = &mut f.base.base;
    let policy = a
        .registry
        .allocate_definition::<UsagePolicyDefinition>()
        .unwrap();
    let parameter = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(policy.clone()))
        .unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
            id: policy.clone(),
            schema: SchemaState::Known(UsagePolicySchema {
                scenario_skill_applicability: None,
                targets: vec![UsageTargetKind::Skill],
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![parameter.clone()]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            }),
        }));
    schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: parameter.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(0.0, unit.clone()).unwrap(),
                    maximum: FiniteQuantity::new(1000.0, unit.clone()).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::UsagePolicyParameter],
                skill_input: None,
            }),
        }));
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = a.mapping.input().clone();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let mut roles = a.roles.input().clone();
    roles.definitions = a.schema.identity().clone();
    roles.mapping = *a.mapping.identity();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, Default::default()).unwrap();
    let definitions = a.schema.identity().clone();
    let roles = *a.roles.identity();
    let catalog = a.roles.input().compilation.catalog_digest;
    let source = a.roles.input().compilation.source.clone();
    let mut rewards = f.base.rewards.input().clone();
    rewards.definitions = definitions.clone();
    rewards.mapping = *a.mapping.identity();
    f.base.rewards =
        OwnedRewardPolicy::new(rewards, &a.mapping, &a.schema, Default::default()).unwrap();
    let GemQualityPolicy::Attributes(quality) = &mut f.base.policy.gem_quality else {
        panic!()
    };
    quality.definitions = definitions.clone();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions: direct_definitions,
        roles: direct_roles,
        dispositions,
        ..
    } = f.base.policy.direct_skill_inputs.as_mut().unwrap();
    *direct_definitions = definitions.clone();
    *direct_roles = roles;
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
        definitions: reference_definitions,
        roles: reference_roles,
        ..
    } = &mut dispositions[0].reference_action
    else {
        panic!()
    };
    *reference_definitions = definitions.clone();
    *reference_roles = roles;
    f.reference = dispositions[0].reference_action.clone();
    let target = OccurrenceUsageTarget::AuthoredDirect {
        gem: direct.gem.clone(),
        game_id: direct.game_id.clone(),
        variant_id: direct.variant_id.clone(),
        skill_id: direct.skill_id.clone(),
        name_spec: direct.name_spec.clone(),
        skill: Box::new(direct.skill.clone()),
    };
    let mut fields: Vec<_> = direct
        .attributes
        .iter()
        // This fixture inventories the exact fields present in inventory::GEM.
        // Absence of legacy/minion selectors is a separate reference-proof
        // responsibility; this test does not grant an absence receipt.
        .filter(|name| {
            !matches!(
                name.as_str(),
                "statSetIndex"
                    | "statSetIndexCalcs"
                    | "skillMinion"
                    | "skillMinionCalcs"
                    | "skillMinionSkill"
                    | "skillMinionSkillCalcs"
            )
        })
        .map(|name| UsageInventoryField {
            field: UsageInventoryFieldSelector::OccurrenceAttribute { name: name.clone() },
            destinations: vec![if name == "count" {
                UsageInventoryDestination::UsageParameter {
                    policy: policy.clone(),
                    parameter: parameter.clone(),
                }
            } else {
                UsageInventoryDestination::InputDisposition
            }],
        })
        .collect();
    fields.extend([
        UsageInventoryField {
            field: UsageInventoryFieldSelector::GroupAttribute {
                name: "enabled".into(),
            },
            destinations: vec![UsageInventoryDestination::InputDisposition],
        },
        UsageInventoryField {
            field: UsageInventoryFieldSelector::ReferenceDescendants,
            destinations: vec![UsageInventoryDestination::InputDisposition],
        },
    ]);
    let scalar_inputs =
        gem_inventory_scalar_inputs_identity(&f.base.policy, Default::default()).unwrap();
    f.base.policy.usage_inputs = Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        source,
        roles,
        catalog,
        scalar_inputs,
        physical: vec![],
        scenario: vec![],
        occurrences: vec![OccurrenceUsageRule {
            target: target.clone(),
            attributes: direct.attributes,
            guards: vec![],
            group_attributes: vec!["source".into(), "enabled".into()],
            group_guards: vec![],
            policies: vec![OccurrenceUsagePolicy {
                policy,
                parameters: vec![UsageParameterInput {
                    slot: parameter,
                    source: UsageValueSource::Occurrence { value: count },
                }],
            }],
        }],
        inventory: Some(UsageInventoryPolicy {
            evidence: "a".repeat(64).parse().unwrap(),
            rows: vec![UsageInventoryRow {
                source: UsageInventorySource::Occurrence {
                    target: Box::new(target),
                },
                fields,
            }],
        }),
    });
    f
}

fn usage_complete(result: &NormalizedImport) -> bool {
    matches!(
        result.draft().input().skill_presets.members[0]
            .usage_preferences
            .as_ref()
            .map(|v| &v.completion),
        Some(DraftListCompletion::Complete)
    )
}
fn inventory_mut(f: &mut Fixture) -> &mut UsageInventoryPolicy {
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.base.policy.usage_inputs.as_mut().unwrap();
    inventory.as_mut().unwrap()
}
fn rebind_usage(f: &mut Fixture) {
    let current = gem_inventory_scalar_inputs_identity(&f.base.policy, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 { scalar_inputs, .. } =
        f.base.policy.usage_inputs.as_mut().unwrap();
    *scalar_inputs = current;
}

#[test]
fn direct_secondary_true_completes_only_after_real_reference_disposition_and_count_transport() {
    let mut f = configured();
    let text = inventory::xml(inventory::GEM);
    let after = run(&f, &text);
    assert!(usage_complete(&after));
    assert_eq!(complete(&after), [true]);
    assert!(after.draft().input().gems.members.is_empty());
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.base.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    let before = run(&f, &text);
    let mut expected = before.draft().input().clone();
    let usage = expected.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap();
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        panic!()
    };
    usage.completion = DraftListCompletion::Complete;
    let mut origins = before.sidecar().origins.clone();
    for origin in &mut origins {
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert_eq!(&expected, after.draft().input());
    assert_eq!(origins, after.sidecar().origins);
    assert_eq!(before.allocator_after(), after.allocator_after());
}

#[test]
fn direct_secondary_true_cannot_replace_an_input_reference_or_second_consumer() {
    let mut f = configured();
    for token in [
        "false",
        "nil",
        "",
        " true",
        "true ",
        "TRUE",
        "1",
        "tr&#117;e",
    ] {
        let text = inventory::xml(&inventory::GEM.replace(
            r#"enableGlobal2="true""#,
            &format!(r#"enableGlobal2="{token}""#),
        ));
        assert!(!usage_complete(&run(&f, &text)), "{token}");
    }
    for text in [
        inventory::xml(&inventory::GEM.replace(r#" enableGlobal2="true""#, "")),
        inventory::xml(&inventory::GEM.replace(r#"quality="0""#, r#"quality="bad""#)),
        inventory::xml(&inventory::GEM.replace(r#"count="1""#, r#"count="bad""#)),
        inventory::xml(&inventory::with_children("<Unknown/>")),
    ] {
        assert!(!usage_complete(&run(&f, &text)));
    }
    inventory_mut(&mut f).rows[0].fields.iter_mut().find(|field|
        matches!(&field.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "enableGlobal2")
    ).unwrap().destinations.push(UsageInventoryDestination::Deferred { reason: actions::key("independent-secondary-consumer") });
    assert!(!usage_complete(&run(&f, &inventory::xml(inventory::GEM))));
    let mut f = configured();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { dispositions, .. } =
        f.base.policy.direct_skill_inputs.as_mut().unwrap();
    dispositions.clear();
    rebind_usage(&mut f);
    let raw = run(&f, &inventory::xml(inventory::GEM));
    assert!(!usage_complete(&raw));
    assert_eq!(
        complete(&raw),
        [false],
        "raw inputs alone are not a disposition receipt"
    );
}

fn configured_reference_absence() -> Fixture {
    let mut f = configured();
    for name in [
        "statSetIndex",
        "statSetIndexCalcs",
        "skillMinion",
        "skillMinionCalcs",
        "skillMinionSkill",
        "skillMinionSkillCalcs",
    ] {
        inventory_mut(&mut f).rows[0]
            .fields
            .push(UsageInventoryField {
                field: UsageInventoryFieldSelector::OccurrenceAttribute { name: name.into() },
                destinations: vec![UsageInventoryDestination::InputDisposition],
            });
    }
    f
}

#[test]
fn direct_checked_reference_accounts_absent_fields_with_only_usage_retirement() {
    // The first case exercises all six absent fields. The later cases preserve
    // the distinction between overwritten scalar headers and present selectors.
    for saved in [
        "",
        r#" statSetIndex="nil" statSetIndexCalcs="nil""#,
        r#" skillMinion="fixture-minion" skillMinionSkill="1" skillMinionSkillCalcs="1""#,
    ] {
        let mut f = configured_reference_absence();
        let text = inventory::xml(&inventory::GEM.replace("/>", &format!("{saved}/>")));
        let after = run(&f, &text);
        assert!(usage_complete(&after));
        assert_eq!(complete(&after), [true]);
        assert!(after.draft().input().gems.members.is_empty());
        assert_eq!(
            values(&after.draft().input().skills.members[0]),
            [Some(17.0), Some(0.0)]
        );
        let again = run(&f, &text);
        assert_eq!(again.draft().input(), after.draft().input());
        assert_eq!(again.allocator_after(), after.allocator_after());
        assert_eq!(
            serde_json::to_value(again.sidecar()).unwrap(),
            serde_json::to_value(after.sidecar()).unwrap()
        );

        let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
            f.base.policy.usage_inputs.as_mut().unwrap();
        *inventory = None;
        let before = run(&f, &text);
        assert!(!usage_complete(&before));
        assert_eq!(complete(&before), [true], "scalar authority is unchanged");
        let mut expected = before.draft().input().clone();
        let usage = expected.skill_presets.members[0]
            .usage_preferences
            .as_mut()
            .unwrap();
        let DraftListCompletion::Pending { id, .. } = usage.completion else {
            panic!()
        };
        assert_eq!(
            usage.members.len(),
            1,
            "the existing requested-count binding is preserved"
        );
        assert_eq!(usage.members[0].parameters.members.len(), 1);
        usage.completion = DraftListCompletion::Complete;
        assert_eq!(
            &expected,
            after.draft().input(),
            "no native field, query, input or identifier changes"
        );
        let mut origins = before.sidecar().origins.clone();
        let mut removed = 0;
        for origin in &mut origins {
            removed += origin
                .links
                .iter()
                .filter(|link| **link == OwnedOriginTarget::Issue(id))
                .count();
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(id));
        }
        assert!(removed > 0);
        assert_eq!(origins, after.sidecar().origins);
        assert_eq!(before.allocator_after(), after.allocator_after());
        assert_eq!(
            decode_build(text.as_bytes()).unwrap().sha256,
            after.sidecar().source_sha256
        );
        assert!(matches!(
            after.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn direct_absence_receipts_reject_failed_foreign_and_independent_consumers() {
    for mode in 0..9 {
        let mut f = configured_reference_absence();
        let mut gem = inventory::GEM.to_owned();
        let mut foreign = false;
        match mode {
            0 => {
                let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { minion, .. } = &mut f.row().reference_action else { panic!() };
                minion.allow_absent = false;
            }
            1 => {
                let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { absent_action, .. } = &mut f.row().reference_action else { panic!() };
                *absent_action = None;
            }
            2 => {
                let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { actions, .. } = &mut f.row().reference_action else { panic!() };
                actions[0].absent_stat_set = None;
            }
            3 => gem = gem.replace("/>", r#" skillMinionCalcs="unknown-profile"/>"#),
            4 => gem = gem.replace("/>", r#" skillMinionSkillCalcs="0"/>"#),
            5 => gem = inventory::with_children("<Unhandled/>"),
            6 => foreign = true,
            7 => {
                let DirectSkillInputPolicy::PobManualDirectSkillV2 { dispositions, .. } = f.base.policy.direct_skill_inputs.as_mut().unwrap();
                dispositions.clear();
            }
            _ => inventory_mut(&mut f).rows[0].fields.iter_mut().find(|field| matches!(&field.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "skillMinionCalcs")).unwrap().destinations.push(UsageInventoryDestination::Deferred { reason: actions::key("independent-missing-profile-consumer") }),
        }
        rebind_usage(&mut f);
        let text = if foreign {
            inventory::xml(&gem).replace(
                r#"<Skill enabled="true">"#,
                r#"<Skill enabled="true" source="Tree:1">"#,
            )
        } else {
            inventory::xml(&gem)
        };
        let result = run(&f, &text);
        assert!(!usage_complete(&result), "mode {mode}");
        if mode == 8 {
            assert_eq!(
                complete(&result),
                [true],
                "independent consumer cannot invalidate proven scalar inputs"
            );
        }
        if foreign {
            assert!(
                result.draft().input().skills.members.is_empty(),
                "foreign generated source cannot borrow a manual Direct disposition"
            );
        }
        assert!(matches!(
            result.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(
            decode_build(text.as_bytes()).unwrap().sha256,
            result.sidecar().source_sha256
        );
        // A failed receipt must not retire another obligation or partially edit
        // the outputs. Compare against the same failed source without inventory.
        let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
            f.base.policy.usage_inputs.as_mut().unwrap();
        *inventory = None;
        let baseline = run(&f, &text);
        assert_eq!(
            baseline.draft().input(),
            result.draft().input(),
            "mode {mode}"
        );
        assert_eq!(
            baseline.sidecar().origins,
            result.sidecar().origins,
            "mode {mode}"
        );
        assert_eq!(
            baseline.allocator_after(),
            result.allocator_after(),
            "mode {mode}"
        );
    }
}

#[path = "owned_direct_group_selectors.rs"]
mod group_selectors;
