//! Real source controls transported to typed Scenario intent, without execution.
#[allow(dead_code)]
#[path = "support/owned_scenario_stage_inputs.rs"]
mod packet;

use packet::{CRY_NAME, Fixture, SIGIL_NAME, field, values, xml};
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_definitions::SkillGrantSlotDefId,
    owned_draft::DraftListCompletion, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::*,
};
use serde_json::json;

#[test]
#[ignore = "explicit offline Rust authoring from authenticated current predecessor"]
fn author_scenario_stage_packet_from_current_predecessor() {
    packet::author_packet();
}

#[test]
fn authenticated_packet_preserves_unknown_mechanics_and_exact_positive_relations() {
    packet::check_authored().unwrap();
    let prior = packet::prior_descriptors();
    let migration = packet::checked_migration();
    assert_eq!(migration.schema.len(), 19);
    assert!(
        migration.owners.is_empty()
            && migration.receivers.is_empty()
            && migration.tables.is_empty()
    );
    assert!(migration.evaluation.is_none() && migration.query_targets.is_empty());
    for row in &migration.schema {
        if let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Slot(
            SlotDescriptor::SkillGrant(DefinitionEntry {
                schema: SchemaState::Known(supply),
                ..
            }),
        ) = row
        {
            assert!(supply.outputs.members.is_empty() && !supply.outputs.is_complete());
            assert!(supply.preset_inputs.is_none());
        }
    }
    for id in ["02fb", "05a5", "05a6", "05a7"] {
        let descriptor = prior
            .iter()
            .find(|d| d.address().key().as_str().ends_with(id))
            .unwrap();
        assert!(matches!(
            descriptor,
            DefinitionDescriptor::Skill(DefinitionEntry {
                schema: SchemaState::Unmapped { .. },
                ..
            })
        ));
        assert!(
            !migration
                .schema
                .iter()
                .any(|d| d.subject() == SchemaSubject::Definition(descriptor.address()))
        );
    }
    for mutation in 0..4 {
        let mut bad = migration.clone();
        match mutation {
            0 => bad.schema.pop(),
            1 => {
                bad.schema.swap(0, 1);
                None
            }
            2 => {
                bad.before = "00".repeat(32).parse().unwrap();
                None
            }
            _ => {
                bad.reason = poe_optimizer_core::owned_definitions::OwnedDefinitionKey::new(
                    "unreviewed-reason",
                )
                .unwrap();
                None
            }
        };
        assert!(packet::check_delta(&bad, &prior).is_err());
    }
    let proof: serde_json::Value = packet::read("source-evidence.json");
    for mutation in 0..4 {
        let mut bad = proof.clone();
        match mutation {
            0 => bad["files"][0]["path"] = json!("src/Classes/Unreviewed.lua"),
            1 => bad["files"][0]["sha256"] = json!("0".repeat(64)),
            2 => bad["files"][0]["archive"] = json!("../outside.gz"),
            _ => bad["scope"]["supplier_execution"] = json!(true),
        }
        assert!(packet::check_source(&bad).is_err());
    }
}

#[test]
fn missing_registered_supply_and_contradictory_grant_are_rejected() {
    let f = Fixture::new();
    let original = f.schema.input();
    let supplies: Vec<_> = original
        .slots
        .iter()
        .filter_map(|row| {
            if let SlotDescriptor::SkillGrant(row) = row {
                Some(row.id.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(supplies.len(), 5);
    for missing in [false, true] {
        let mut bad = original.clone();
        if missing {
            bad.slots
                .retain(|row| row.address() != SkillGrantSlotDefId::address(&supplies[0]));
        } else {
            let grant = bad
                .slots
                .iter_mut()
                .find_map(|row| {
                    if let SlotDescriptor::Grant(DefinitionEntry {
                        schema: SchemaState::Known(grant),
                        ..
                    }) = row
                    {
                        (grant.target == GrantTarget::Skill(supplies[0].clone())).then_some(grant)
                    } else {
                        None
                    }
                })
                .unwrap();
            grant.target = GrantTarget::Skill(supplies[1].clone());
        }
        assert!(OwnedDefinitionSchemaPackage::new(bad, Default::default()).is_err());
    }
}

#[test]
fn real_stages_are_retained_without_a_consumer_and_keep_inventory_obligations() {
    let f = Fixture::new();
    let text =
        xml(&(field("Placeholder", SIGIL_NAME, "1") + &field("Placeholder", CRY_NAME, "10")));
    let result = f.run(&text).unwrap();
    let got = values(&result);
    assert_eq!(
        got,
        vec![vec![
            (f.rows()[0].policy.clone(), 1),
            (f.rows()[1].policy.clone(), 10)
        ]]
    );
    let preset = &result.draft().input().scenario_presets.members[0];
    assert!(matches!(
        preset.scenario.usage_intents.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.scenario.usage.completion,
        DraftListCompletion::Pending { .. }
    ));
    for row in &f.schema.input().definitions {
        if let DefinitionDescriptor::UsagePolicy(row) = row {
            let SchemaState::Known(schema) = &row.schema else {
                panic!()
            };
            let authority = schema.scenario_skill_applicability.as_ref().unwrap();
            assert!(!authority.members.is_empty() && !authority.is_complete());
        }
    }
    let baseline = f
        .run_with(&text, &f.without_stage_rows(), Default::default())
        .unwrap();
    assert_eq!(
        result.draft().input().skill_presets,
        baseline.draft().input().skill_presets
    );
    assert_eq!(
        result.draft().input().equipment_presets,
        baseline.draft().input().equipment_presets
    );
    assert_eq!(
        result.draft().input().choice_presets,
        baseline.draft().input().choice_presets
    );
    assert_eq!(
        result.draft().input().scenario_presets.members[0]
            .scenario
            .usage,
        baseline.draft().input().scenario_presets.members[0]
            .scenario
            .usage
    );
    let again = f.run(&text).unwrap();
    assert_eq!(result.draft(), again.draft());
    assert_eq!(json!(result.sidecar()), json!(again.sidecar()));
}

#[test]
fn independent_config_sets_do_not_copy_intents_into_skill_presets() {
    let f = Fixture::new();
    let text = format!(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1">{}{}</ConfigSet><ConfigSet id="2">{}{}</ConfigSet></Config><Skills activeSkillSet="8"><SkillSet id="7"/><SkillSet id="8"/></Skills></PathOfBuilding2>"#,
        field("Input", SIGIL_NAME, "0"),
        field("Placeholder", CRY_NAME, "10"),
        field("Placeholder", SIGIL_NAME, "4"),
        field("Input", CRY_NAME, "2")
    );
    let result = f.run(&text).unwrap();
    assert_eq!(
        values(&result),
        vec![
            vec![
                (f.rows()[0].policy.clone(), 0),
                (f.rows()[1].policy.clone(), 10)
            ],
            vec![
                (f.rows()[0].policy.clone(), 4),
                (f.rows()[1].policy.clone(), 2)
            ],
        ]
    );
    let baseline = f
        .run_with(&text, &f.without_stage_rows(), Default::default())
        .unwrap();
    assert_eq!(
        result.draft().input().skill_presets,
        baseline.draft().input().skill_presets
    );
    assert_eq!(result.draft().input().skill_presets.members.len(), 2);
}

#[test]
fn zero_selection_and_missing_lanes_follow_each_actual_source_reader() {
    let f = Fixture::new();
    for (name, expected) in [(SIGIL_NAME, Some(0)), (CRY_NAME, Some(10))] {
        let text = xml(&(field("Input", name, "0") + &field("Placeholder", name, "10")));
        let result = f.run(&text).unwrap();
        assert_eq!(values(&result)[0].first().map(|r| r.1), expected);
    }
    for body in [
        String::new(),
        field("Input", CRY_NAME, "0"),
        field("Placeholder", CRY_NAME, "0"),
        field("Input", CRY_NAME, "0") + &field("Placeholder", CRY_NAME, "0"),
    ] {
        let result = f.run(&xml(&body)).unwrap();
        assert!(values(&result)[0].is_empty());
        assert!(matches!(
            result.draft().input().scenario_presets.members[0]
                .scenario
                .usage_intents
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn malformed_losing_lanes_duplicates_and_unsupported_domains_do_not_emit_intent() {
    let f = Fixture::new();
    for name in [SIGIL_NAME, CRY_NAME] {
        for bad in ["bad", "-1", "1.5", "9007199254740992"] {
            for losing in [false, true] {
                let body = if losing {
                    field("Input", name, "3") + &field("Placeholder", name, bad)
                } else {
                    field("Input", name, bad) + &field("Placeholder", name, "3")
                };
                let result = f.run(&xml(&body)).unwrap();
                assert!(
                    values(&result)[0].is_empty(),
                    "{name} {bad} losing={losing}"
                );
            }
        }
        for lane in ["Input", "Placeholder"] {
            let body = field(lane, name, "2") + &field(lane, name, "3");
            assert!(values(&f.run(&xml(&body)).unwrap())[0].is_empty());
        }
    }
}

#[test]
fn complete_intent_receipts_account_both_lanes_but_not_missing_extra_destinations() {
    let f = Fixture::new();
    let text = xml(&(field("Input", CRY_NAME, "0") + &field("Placeholder", CRY_NAME, "10")));
    let result = f.run(&text).unwrap();
    let target = OwnedOriginTarget::ScenarioUsageIntent {
        scenario: result.draft().input().scenario_presets.members[0].id,
        policy: f.rows()[1].policy.clone(),
    };
    let source = ImportedBuildInstance::from_decoded(
        decode_build(text.as_bytes()).unwrap(),
        BuildLineage::from_bytes([0xce; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let leaves: Vec<_> = source
        .occurrences()
        .iter()
        .filter(|o| matches!(o.name(), "Input" | "Placeholder"))
        .map(|o| o.id().ordinal() as usize)
        .collect();
    assert_eq!(leaves.len(), 2);
    for index in &leaves {
        let origin = &result.sidecar().origins[*index];
        assert!(origin.links.contains(&target));
        assert!(
            !origin
                .links
                .iter()
                .any(|l| matches!(l, OwnedOriginTarget::Issue(_)))
        );
    }
    let scope = source
        .occurrences()
        .iter()
        .find(|o| o.name() == "ConfigSet")
        .unwrap()
        .id()
        .ordinal() as usize;
    assert!(result.sidecar().origins[scope].links.contains(&target));
    assert!(
        result.sidecar().origins[scope]
            .links
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::Issue(_)))
    );
    let mut bad = f.policy.clone();
    bad.configuration_dispositions.as_mut().unwrap().rules[2]
        .destinations
        .push(ConfigurationDestination::Adapter {
            control: ConfigurationAdapterControl::UsageIntent {
                policy: f.rows()[0].policy.clone(),
                parameter: f.rows()[0].parameters[0].slot.clone(),
            },
        });
    assert!(f.run_with(&text, &bad, Default::default()).is_err());
    let mut bad = f.policy.clone();
    bad.configuration_dispositions.as_mut().unwrap().rules[2]
        .destinations
        .push(ConfigurationDestination::Deferred {
            responsibility: ConfigurationResponsibility::ScenarioUsage,
        });
    let result = f.run_with(&text, &bad, Default::default()).unwrap();
    assert!(
        leaves
            .iter()
            .map(|i| &result.sidecar().origins[*i])
            .any(|o| {
                o.links.contains(&target)
                    && o.links
                        .iter()
                        .any(|l| matches!(l, OwnedOriginTarget::Issue(_)))
            })
    );
}

#[test]
fn append_is_exact_atomic_and_rejects_duplicates() {
    let f = Fixture::new();
    let mut policy = f.without_stage_rows();
    let before = policy.clone();
    packet::checked_append(&mut policy).unwrap();
    let mut inverse = policy.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
        inverse.usage_inputs.as_mut().unwrap();
    assert_eq!(*scenario, packet::scenario_rows());
    scenario.clear();
    assert_eq!(
        inverse.configuration_dispositions.as_ref().unwrap().rules,
        packet::disposition_rows()
    );
    inverse
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .rules
        .clear();
    assert_eq!(inverse, before);
    let once = policy.clone();
    assert!(packet::checked_append(&mut policy).is_err());
    assert_eq!(policy, once);
}

#[test]
fn stale_dependencies_required_parameters_and_work_bounds_are_rejected() {
    let f = Fixture::new();
    let text = xml(&field("Placeholder", SIGIL_NAME, "1"));
    for mode in 0..3 {
        let mut bad = f.policy.clone();
        let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
            bad.usage_inputs.as_mut().unwrap();
        match mode {
            0 => scenario[0].parameters.clear(),
            1 => {
                scenario[0].parameters[0].recipe.tiers[0].selectors[0].name = "changed-stage".into()
            }
            _ => scenario[0].parameters[0].slot = scenario[1].parameters[0].slot.clone(),
        }
        assert!(f.run_with(&text, &bad, Default::default()).is_err());
    }
    assert!(
        f.run_with(
            &text,
            &f.policy,
            NormalizationLimits {
                max_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(values(&f.run(&text).unwrap())[0][0].1, 1);
}
