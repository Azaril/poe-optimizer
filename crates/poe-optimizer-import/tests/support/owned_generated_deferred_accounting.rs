//! Archived source accounting retains three real obligations, without a provider.
use super::*;
use poe_optimizer_core::build_identity::DraftIssueId;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    owned_mapping::*, owned_reward_policy::OwnedRewardPolicy,
    owned_skill_catalog::OwnedSkillRoleIndex,
};
use std::collections::BTreeSet;

fn configured() -> Fixture {
    let mut f = Fixture::new();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { skills, .. } =
        f.base.policy.direct_skill_inputs.as_ref().unwrap()
    else {
        unreachable!()
    };
    let direct = skills[0].clone();
    let quality = direct
        .parameters
        .iter()
        .find(|p| p.value.tiers[0].selectors[0].name == "quality")
        .unwrap()
        .clone();
    let a = &mut f.base.base;
    let passive = a
        .registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let supply = a
        .registry
        .allocate_slot::<SkillGrantSlotDefinition>(SlotOwnerDefId::PassiveNode(passive.clone()))
        .unwrap();
    let mut schema = a.schema.input().clone();
    schema.schema_version = OWNED_SCHEMA_PACKAGE_V6;
    schema
        .definitions
        .push(DefinitionDescriptor::PassiveNode(DefinitionEntry {
            id: passive.clone(),
            schema: SchemaState::Known(PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![]),
                adjacent: DeclaredSet::complete(vec![]),
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![supply.clone()]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            }),
        }));
    schema
        .slots
        .push(SlotDescriptor::SkillGrant(DefinitionEntry {
            id: supply.clone(),
            schema: SchemaState::Known(SkillGrantSlotSchema {
                skill: direct.skill.clone(),
                outputs: DeclaredSet::complete(vec![]),
                preset_inputs: Some(PresetSkillInputPermission {
                    schema_version: 1,
                    parameters: DeclaredSet::complete(vec![quality.slot.clone()]),
                }),
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
    let GemQualityPolicy::Attributes(quality_policy) = &mut f.base.policy.gem_quality else {
        unreachable!()
    };
    quality_policy.definitions = definitions.clone();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions: direct_definitions,
        roles: direct_roles,
        dispositions,
        ..
    } = f.base.policy.direct_skill_inputs.as_mut().unwrap()
    else {
        unreachable!()
    };
    *direct_definitions = definitions.clone();
    *direct_roles = roles;
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
        definitions: reference_definitions,
        roles: reference_roles,
        ..
    } = &mut dispositions[0].reference_action
    else {
        unreachable!()
    };
    *reference_definitions = definitions.clone();
    *reference_roles = roles;
    f.reference = dispositions[0].reference_action.clone();
    let mut saved_level = f.base.policy.gem_level.clone();
    saved_level.id = actions::key("archived-source-level");
    f.base.policy.generated_skill_inputs =
        Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
            definitions,
            source,
            roles,
            catalog,
            rows: vec![GeneratedSkillInputRule {
                gem: direct.gem,
                game_id: direct.game_id,
                variant_id: direct.variant_id,
                skill_id: direct.skill_id,
                name_spec: direct.name_spec,
                skill: direct.skill,
                provider: GeneratedSkillInputProvider::TreeAllocation {
                    source_node_id: "7".into(),
                    passive,
                    skill_supply: supply,
                },
                saved_level,
                provider_level: GeneratedSkillProviderLevel::Fixed {
                    value: BoundedInteger::new(1).unwrap(),
                },
                parameters: vec![GeneratedSkillParameterInput {
                    field: GeneratedSkillInputField::Quality,
                    slot: quality.slot,
                    value: quality.value,
                }],
            }],
        });
    f.base.policy.support_origin_order = Some(SupportOriginOrderPolicy::SavedManualGroupOrder {});
    f
}

fn generated() -> String {
    inventory::GEM
        .replace("level=\"17\"", "level=\"1\"")
        .replace("corrupted=\"false\"", "corrupted=\"nil\"")
        .replace("corruptLevel=\"0\"", "corruptLevel=\"nil\"")
}
fn document(gem: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="2"><SkillSet id="1"><Skill enabled="true">{}</Skill><Skill source="Tree:7" enabled="true">{gem}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{}</Skill></SkillSet></Skills></PathOfBuilding2>"#,
        inventory::GEM,
        inventory::GEM
    )
}
fn pair(text: &str) -> [u32; 2] {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(text.as_bytes()).unwrap(),
        BuildLineage::from_bytes([71; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let group = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Skill" && r.attribute("source").is_some())
        .unwrap();
    let gem = evidence
        .rows()
        .iter()
        .find(|r| {
            r.occurrence().parent() == Some(group.occurrence().id())
                && r.occurrence().name() == "Gem"
        })
        .unwrap();
    [
        group.occurrence().id().ordinal(),
        gem.occurrence().id().ordinal(),
    ]
}
fn configuration(result: &NormalizedImport) -> BTreeSet<DraftIssueId> {
    result
        .draft()
        .input()
        .choice_presets
        .members
        .iter()
        .filter_map(|p| match &p.choices.completion {
            DraftListCompletion::Pending { id, code }
                if code.as_str() == "configuration-roles-not-converted" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect()
}
fn is_fallback(result: &NormalizedImport, ordinal: u32) -> bool {
    let configuration = configuration(result);
    result.sidecar().origins[ordinal as usize]
        .links
        .iter()
        .any(|l| matches!(l, OwnedOriginTarget::Issue(id) if configuration.contains(id)))
}
fn owner(completion: &DraftListCompletion, expected: &str) -> DraftIssueId {
    let DraftListCompletion::Pending { id, code } = completion else {
        panic!("must retain {expected}")
    };
    assert_eq!(code.as_str(), expected);
    *id
}
fn three_owners(result: &NormalizedImport) -> [DraftIssueId; 3] {
    let preset = &result.draft().input().skill_presets.members[0];
    let intent = preset.intent.as_ref().unwrap();
    assert!(intent.generated_inputs.members.is_empty());
    assert!(intent.usage.members.is_empty());
    let ids = [
        owner(
            &intent.generated_inputs.completion,
            "generated-skill-inputs-not-converted",
        ),
        owner(&intent.usage.completion, "usage-preferences-not-converted"),
        owner(
            &preset.authored_support_order.as_ref().unwrap().completion,
            "support-origin-discovery-not-converted",
        ),
    ];
    assert_eq!(ids.into_iter().collect::<BTreeSet<_>>().len(), 3);
    ids
}

#[test]
fn archived_generated_accounting_retains_three_owners_without_emitting_inputs_or_providers() {
    let f = configured();
    let text = document(&generated());
    let result = run(&f, &text);
    let owners = three_owners(&result);
    let preset = result.draft().input().skill_presets.members[0].id;
    for ordinal in pair(&text) {
        assert!(!is_fallback(&result, ordinal), "source {ordinal}");
        let links = &result.sidecar().origins[ordinal as usize].links;
        assert!(links.contains(&OwnedOriginTarget::SkillPreset(preset)));
        for id in owners {
            assert!(links.contains(&OwnedOriginTarget::Issue(id)));
        }
        assert!(!links.iter().any(|l| matches!(
            l,
            OwnedOriginTarget::GeneratedSkillInput { .. }
                | OwnedOriginTarget::Skill(_)
                | OwnedOriginTarget::Gem(_)
        )));
    }
    let input = result.draft().input();
    assert_eq!(
        input.skills.members.len(),
        2,
        "only the two manual occurrences exist"
    );
    assert!(input.gems.members.is_empty() && input.allocations.members.is_empty());
    assert!(input.items.members.is_empty() && input.equipment.members.is_empty());
    let again = run(&f, &text);
    assert_eq!(result.draft(), again.draft());
    assert_eq!(
        serde_json::to_value(result.sidecar()).unwrap(),
        serde_json::to_value(again.sidecar()).unwrap()
    );
    assert_eq!(result.allocator_after(), again.allocator_after());
    let sentinels = document(
        &generated()
            .replace("count=\"1\"", "count=\"nil\"")
            .replace("enableGlobal2=\"true\"", "enableGlobal2=\"nil\""),
    );
    let result = run(&f, &sentinels);
    assert_eq!(
        result.draft(),
        again.draft(),
        "saved sentinels remain deferred, never defaulted into inputs"
    );
    for ordinal in pair(&sentinels) {
        assert!(!is_fallback(&result, ordinal));
    }
    for gem in [
        generated()
            .replace("count=\"1\"", "")
            .replace("enableGlobal1=\"true\"", "")
            .replace("enableGlobal2=\"true\"", ""),
        generated()
            .replace("enableGlobal1=\"true\"", "enableGlobal1=\"false\"")
            .replace("enableGlobal2=\"true\"", "enableGlobal2=\"false\""),
    ] {
        let text = document(&gem);
        let result = run(&f, &text);
        assert_eq!(
            result.draft(),
            again.draft(),
            "deferred fields do not manufacture usage values"
        );
        for ordinal in pair(&text) {
            assert!(!is_fallback(&result, ordinal));
        }
    }
    let maps = minions::maps(&minions::entry_map("1", "1"), &minions::entry_map("2", "2"));
    let selectors = generated().replace(" quality=", " skillMinion=\"fixture-minion\" skillMinionSkill=\"2\" skillMinionSkillCalcs=\"1\" quality=")
        .replace("/>", &format!(">{maps}</Gem>"));
    let text = document(&selectors);
    let result = run(&f, &text);
    assert_eq!(
        result.draft(),
        again.draft(),
        "reference selectors create no native inputs or providers"
    );
    for ordinal in pair(&text) {
        assert!(!is_fallback(&result, ordinal));
    }
}

#[test]
fn archived_generated_malformed_or_unknown_fields_preserve_fallback_and_owned_values() {
    let f = configured();
    let original = document(&generated());
    let baseline = run(&f, &original);
    for (before, after) in [
        ("quality=\"0\"", "quality=\"bad\""),
        ("quality=\"0\"", "quality=\"1001\""),
        ("quality=\"0\"", ""),
        ("quality=\"0\"", "quality=\"false\""),
        ("quality=\"0\"", "quality=\"nil\""),
        ("level=\"1\"", ""),
        ("level=\"1\"", "level=\"false\""),
        ("level=\"1\"", "level=\"nil\""),
        ("level=\"1\"", "level=\"1.5\""),
        ("count=\"1\"", "count=\"bad\""),
        ("count=\"1\"", "count=\"false\""),
        ("enableGlobal1=\"true\"", "enableGlobal1=\"maybe\""),
        ("enableGlobal2=\"true\"", "enableGlobal2=\"unknown\""),
        ("enabled=\"true\"", "enabled=\"nil\""),
        ("corrupted=\"nil\"", "corrupted=\"true\""),
        ("corruptLevel=\"nil\"", "corruptLevel=\"0\""),
        ("quality=\"0\"", "unknown=\"1\" quality=\"0\""),
        ("quality=\"0\"", "skillMinion=\"unknown\" quality=\"0\""),
        ("quality=\"0\"", "skillMinionSkill=\"999\" quality=\"0\""),
        ("/>", "><Unexpected/></Gem>"),
    ] {
        let text = document(&generated().replace(before, after));
        let result = run(&f, &text);
        for ordinal in pair(&text) {
            assert!(is_fallback(&result, ordinal), "{after}: source {ordinal}");
        }
        assert_eq!(
            result.draft(),
            baseline.draft(),
            "{after}: no value or ID changes"
        );
        assert_eq!(three_owners(&result), three_owners(&baseline));
    }
    for changed in [
        original.replace("source=\"Tree:7\"", "source=\"Tree:07\""),
        original.replace("source=\"Tree:7\"", "source=\"Tree:8\""),
        original.replace(
            "source=\"Tree:7\"",
            "source=\"Item:9:Unknown\" slot=\"Weapon 1\"",
        ),
        original.replace("source=\"Tree:7\"", "source=\"Tree:7\" slot=\"Weapon 1\""),
        original.replace(
            "source=\"Tree:7\"",
            "source=\"Tree:7\" mainActiveSkill=\"2\"",
        ),
        original.replace("source=\"Tree:7\"", "source=\"Tree:7\" note=\"unreviewed\""),
    ] {
        let result = run(&f, &changed);
        for ordinal in pair(&changed) {
            assert!(is_fallback(&result, ordinal), "{changed}");
        }
    }
    let duplicate = document(&generated().replace("quality=\"0\"", "quality=\"0\" quality=\"1\""));
    assert!(decode_build(duplicate.as_bytes()).is_err());
}

#[test]
fn archived_generated_accounting_refuses_unselected_ambiguity_or_missing_existing_owner() {
    let original = document(&generated());
    for changed in [
        original.replace("activeSkillSet=\"2\"", "activeSkillSet=\"1\""),
        original.replace("activeSkillSet=\"2\"", "activeSkillSet=\"9\""),
        original.replace("activeSkillSet=\"2\"", ""),
        original.replace(
            "<SkillSet id=\"1\">",
            &format!(
                "<SkillSet id=\"1\"><Skill source=\"Tree:7\" enabled=\"true\">{}</Skill>",
                generated()
            ),
        ),
    ] {
        let result = run(&configured(), &changed);
        for ordinal in pair(&changed) {
            assert!(is_fallback(&result, ordinal), "{changed}");
        }
    }
    let mut f = configured();
    f.base.policy.support_origin_order = None;
    let result = run(&f, &original);
    assert!(
        result.draft().input().skill_presets.members[0]
            .authored_support_order
            .is_none()
    );
    for ordinal in pair(&original) {
        assert!(is_fallback(&result, ordinal));
    }
    let no_sibling = original.replacen(
        &format!("<Skill enabled=\"true\">{}</Skill>", inventory::GEM),
        "",
        1,
    );
    let result = run(&configured(), &no_sibling);
    assert!(matches!(
        result.draft().input().skill_presets.members[0]
            .intent
            .as_ref()
            .unwrap()
            .usage
            .completion,
        DraftListCompletion::Complete
    ));
    for ordinal in pair(&no_sibling) {
        assert!(is_fallback(&result, ordinal));
    }
    for origin in &result.sidecar().origins {
        assert!(
            !origin
                .links
                .iter()
                .any(|l| matches!(l, OwnedOriginTarget::GeneratedSkillInput { .. }))
        );
    }
}
