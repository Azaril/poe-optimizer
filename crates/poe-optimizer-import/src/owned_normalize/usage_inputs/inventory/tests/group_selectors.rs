//! Consumer controls for a joined private group proof. The public integration
//! target separately exercises real normalizer receipt production; these tests
//! preserve the actual private Builder/sidecar links while tampering snapshots.
use super::*;
use crate::owned_skill_roles::{
    ObservedGemSourceConstruction, OwnedIndexedSourceEffectReference, OwnedMissingSourceEffect,
    OwnedMissingSourceEffectKind, OwnedSourceEffectReference, OwnedSourceEffectResolution,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
};

fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}

fn with_joined_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_joined_case_roles(|_| {}, test);
}

fn with_joined_case_roles(
    change: impl FnOnce(&mut Vec<OwnedGemRoleRow>),
    test: impl FnOnce(&mut Builder<'_, '_>, Case),
) {
    with_joined_source_case(false, change, test);
}

fn with_joined_source_case(
    direct: bool,
    change: impl FnOnce(&mut Vec<OwnedGemRoleRow>),
    test: impl FnOnce(&mut Builder<'_, '_>, Case),
) {
    let mut xml = SOURCE.replace(
        r#"<Skill enabled="true"><Gem/>"#,
        r#"<Skill enabled="true" mainActiveSkill="nil" mainActiveSkillCalcs="1"><Gem gemId="primary" variantId="v" skillId="primary-effect" enableGlobal2="true"/><Gem gemId="support" variantId="v" skillId="support-effect" enableGlobal2="true"/>"#,
    );
    if direct {
        xml = xml.replace(
            "<Gem gemId=\"primary\"",
            "<Gem enabled=\"true\" enableGlobal1=\"true\" nameSpec=\"Primary\" gemId=\"primary\"",
        );
    }
    with_case(&xml, |outer, mut case| {
        let ns = outer.roles.input().namespace.clone();
        let mut registry = OwnedIdRegistry::empty(ns.clone(), Default::default()).unwrap();
        let primary = registry.allocate_definition::<GemDefinition>().unwrap();
        let support = registry.allocate_definition::<GemDefinition>().unwrap();
        let primary_effect = registry.allocate_definition::<SkillDefinition>().unwrap();
        let support_effect = registry.allocate_definition::<SkillDefinition>().unwrap();
        assert_eq!(
            case.receipts.inputs[0].identity,
            UsageInventorySource::Physical {
                gem: primary.clone()
            }
        );
        let mut descriptors = Vec::new();
        for (gem, effect, role) in [
            (&primary, &primary_effect, AuthoredGemRole::SkillUse),
            (
                &support,
                &support_effect,
                AuthoredGemRole::SupportAssignment,
            ),
        ] {
            let active = role == AuthoredGemRole::SkillUse;
            descriptors.push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: gem.clone(),
                schema: SchemaState::Known(GemSchema {
                    level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(30).unwrap(),
                    },
                    roles: vec![role],
                    skills: DeclaredSet::complete(if active {
                        vec![effect.clone()]
                    } else {
                        vec![]
                    }),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: declarations(),
                }),
            }));
            descriptors.push(DefinitionDescriptor::Skill(DefinitionEntry {
                id: effect.clone(),
                schema: SchemaState::Known(SkillSchema {
                    directly_selectable: false,
                    declarations: declarations(),
                }),
            }));
        }
        if direct {
            for descriptor in &mut descriptors {
                match descriptor {
                    DefinitionDescriptor::Gem(row) if row.id == primary => {
                        row.schema = SchemaState::Unmapped {
                            gaps: vec![SchemaGap {
                                subject: SchemaSubject::Definition(DefinitionAddress::Gem(
                                    primary.clone(),
                                )),
                                facet: SchemaFacet::InputSchema,
                                code: key("catalog-only-direct-fixture"),
                            }],
                        };
                    }
                    DefinitionDescriptor::Skill(row) if row.id == primary_effect => {
                        let SchemaState::Known(skill) = &mut row.schema else {
                            unreachable!()
                        };
                        skill.directly_selectable = true;
                    }
                    _ => {}
                }
            }
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns.clone(),
                release: key("joined-selectors"),
                semantics_version: key("fixture-v1"),
                definitions: descriptors,
                slots: vec![],
            },
            Default::default(),
        )
        .unwrap();
        let mut mapping = outer.mappings.input().clone();
        mapping.registry = registry.identity().unwrap();
        mapping.definitions = schema.identity().clone();
        for (gem, effect, game_id, effect_id) in [
            (&primary, &primary_effect, "primary", "primary-effect"),
            (&support, &support_effect, "support", "support-effect"),
        ] {
            mapping.entries.extend([
                MappingEntry {
                    source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                        game_id: SourceComponent::Text(game_id.into()),
                        variant_id: SourceComponent::Text("v".into()),
                    }),
                    outcome: MappingOutcome::Mapped {
                        target: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                        basis: MappingBasis::Exact,
                    },
                },
                MappingEntry {
                    source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                        effect_id: SourceComponent::Text(effect_id.into()),
                    }),
                    outcome: MappingOutcome::Mapped {
                        target: SchemaSubject::Definition(DefinitionAddress::Skill(effect.clone())),
                        basis: MappingBasis::Exact,
                    },
                },
            ]);
        }
        let mapping =
            OwnedMappingIndex::new(mapping, &registry, &schema, Default::default()).unwrap();
        let mut roles = outer.roles.input().clone();
        roles.mapping = *mapping.identity();
        roles.definitions = schema.identity().clone();
        roles.compilation.staged_registry = registry.identity().unwrap();
        roles.compilation.gem_count = 2;
        roles.compilation.skill_count = 2;
        for (gem, effect, game_id, effect_id, role, declaration) in [
            (
                primary,
                primary_effect,
                "primary",
                "primary-effect",
                AuthoredGemRole::SkillUse,
                1,
            ),
            (
                support.clone(),
                support_effect,
                "support",
                "support-effect",
                AuthoredGemRole::SupportAssignment,
                2,
            ),
        ] {
            let reference = OwnedSourceEffectReference {
                effect_id: effect_id.into(),
                resolution: OwnedSourceEffectResolution::Mapped(effect.clone()),
            };
            roles.roles.push(OwnedGemRoleRow {
                gem,
                primary: OwnedPrimarySkill::Known(effect),
                role: OwnedGemRole::Known(role),
                materialization: if direct && role == AuthoredGemRole::SkillUse {
                    OwnedGemMaterialization::ProviderOnly
                } else {
                    OwnedGemMaterialization::Physical
                },
                construction: OwnedGemSourceConstruction::Observed(Box::new(
                    ObservedGemSourceConstruction {
                        catalog_gem_key: game_id.into(),
                        winning_declaration: declaration,
                        game_id: game_id.into(),
                        variant_id: "v".into(),
                        primary: reference.clone(),
                        declared_additional_effects: vec![],
                        constructed_additional_effects: vec![],
                        resolved_additional_effects: vec![],
                        resolved_effects: vec![reference],
                        display_order: None,
                        additional_stat_sets: vec![],
                        missing_references: vec![],
                    },
                )),
            });
        }
        change(&mut roles.roles);
        let roles = OwnedSkillRoleIndex::new(roles, &mapping, &schema, Default::default()).unwrap();
        // Local reborrow replaces only the empty fixture's read-only catalog.
        // Other artifact families are not consulted by this private consumer.
        let mut b = Builder {
            evidence: outer.evidence,
            mappings: &mapping,
            roles: &roles,
            rewards: outer.rewards,
            items: outer.items,
            item_source: outer.item_source,
            item_texts: vec![],
            item_range_origins_attached: false,
            allocator: InstanceAllocator::from_state(outer.allocator.state()),
            limits: outer.limits,
            work: outer.work,
            links: outer.links,
            issues: outer.issues,
            origins: outer.origins.clone(),
            attributes: outer.attributes.clone(),
            ordinary_items_source: None,
            fresh_configuration_sets: None,
            fresh_skill_containers: None,
            fresh_skill_sets: None,
            fresh_build_frame: None,
        };
        let group = case.receipts.inputs[0].group;
        let source = b.evidence.rows()[group.ordinal() as usize].children()[1];
        let preset = &mut case.draft.skill_presets.members[0];
        let gem = GemDraft {
            id: b.id().unwrap(),
            definition: support.clone().into(),
            level: 17.into(),
            quality: DraftQuality::Known { value: None },
            parameters: DraftList {
                completion: DraftListCompletion::Complete,
                members: vec![],
            },
        };
        let assignment = SupportDraft {
            id: b.id().unwrap(),
            support: gem.id.into(),
            target: DraftSkillTarget::Authored(case.draft.skills.members[0].id.into()),
            enabled: true.into(),
        };
        b.link(source, OwnedOriginTarget::Gem(gem.id)).unwrap();
        b.link(source, OwnedOriginTarget::Support(assignment.id))
            .unwrap();
        preset.supports.members.push(assignment.id);
        case.receipts.inputs.push(InputReceipt {
            source,
            group,
            preset: preset.id,
            identity: UsageInventorySource::Physical { gem: support },
            output: InputOutput::Support {
                support: Box::new(assignment.clone()),
                gem: Box::new(gem.clone()),
            },
            fields: BTreeSet::new(),
            descendants: BTreeSet::new(),
        });
        case.draft.gems.members.push(gem);
        case.draft.supports.members.push(assignment);
        if direct {
            let role = &roles.input().roles[0];
            let OwnedPrimarySkill::Known(definition) = &role.primary else {
                unreachable!()
            };
            let receipt = &mut case.receipts.inputs[0];
            let InputOutput::Authored { skill, gem, .. } = &mut receipt.output else {
                unreachable!()
            };
            let physical = gem.take().unwrap();
            case.draft.gems.members.retain(|row| row.id != physical.id);
            b.origins[receipt.source.ordinal() as usize]
                .links
                .retain(|link| !matches!(link, OwnedOriginTarget::Gem(_)));
            skill.source = DraftAuthoredSkillSource::Direct(definition.clone().into());
            case.draft.skills.members[0] = skill.as_ref().clone();
            receipt.identity = UsageInventorySource::Occurrence {
                target: Box::new(OccurrenceUsageTarget::AuthoredDirect {
                    gem: role.gem.clone(),
                    game_id: "primary".into(),
                    variant_id: "v".into(),
                    skill_id: "primary-effect".into(),
                    name_spec: "Primary".into(),
                    skill: Box::new(definition.clone()),
                }),
            };
        }
        test(&mut b, case);
    });
}

fn proof(
    b: &mut Builder<'_, '_>,
    case: &Case,
) -> Result<Option<super::super::group_selectors::GroupSelectors>> {
    let group = &b.evidence.rows()[case.receipts.inputs[0].group.ordinal() as usize];
    super::super::group_selectors::prove(
        b,
        &case.draft,
        &case.draft.skill_presets.members[0],
        group,
        &case.receipts,
    )
}

#[test]
fn live_support_companion_is_accepted_without_borrowing_its_usage_fields() {
    with_joined_case(|b, case| {
        for (index, receipt) in case.receipts.inputs.iter().enumerate() {
            let UsageInventorySource::Physical { gem } = &receipt.identity else {
                unreachable!()
            };
            let row = &b.evidence.rows()[receipt.source.ordinal() as usize];
            let roles = b.roles;
            let role = skill_input_disposition::charged_source_role(b, roles, row, gem)
                .unwrap()
                .unwrap();
            assert!(skill_input_disposition::physical_source_is_singleton(
                row, gem, role
            ));
            assert_eq!(
                skill_input_disposition::physical_secondary_global_is_unconsumed(row, gem, role),
                index == 0,
                "shared construction does not grant the support secondary-switch authority"
            );
        }
        let before = case.draft.clone();
        let origins = b.origins.clone();
        let watermark = b.allocator.state();
        let proof = proof(b, &case)
            .unwrap()
            .expect("the joined selector proof succeeds");
        for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            assert!(proof.proves(&case.receipts.inputs[0], &group_field(name)));
            assert!(!proof.proves(&case.receipts.inputs[1], &group_field(name)));
        }
        for name in ["count", "enableGlobal1", "enableGlobal2"] {
            assert!(!proof.proves(&case.receipts.inputs[1], &occurrence_field(name)));
        }
        assert_eq!(case.draft, before);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), watermark);
    });
}

#[test]
fn joined_proof_rechecks_support_and_primary_snapshots_before_accounting() {
    for mode in 0..8 {
        with_joined_case(|b, mut case| {
            let source = case.receipts.inputs[1].source;
            match mode {
                0 => case.draft.gems.members[1].level = 18.into(),
                1 => case.draft.supports.members[0].enabled = false.into(),
                2 => case.draft.skill_presets.members[0].supports.members.clear(),
                3 => b.origins[source.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::Support(_))),
                4 => {
                    case.receipts.inputs.pop();
                }
                5 => case.draft.skills.members[0].enabled = false.into(),
                6 => {
                    let id = b.id().unwrap();
                    b.link(source, OwnedOriginTarget::Support(id)).unwrap();
                }
                _ => {
                    // A self-consistent snapshot with an existing, selected but
                    // different target still cannot be this group's companion.
                    let mut other = case.draft.skills.members[0].clone();
                    other.id = b.id().unwrap();
                    case.draft.skill_presets.members[0]
                        .skills
                        .members
                        .push(other.id);
                    case.draft.supports.members[0].target =
                        DraftSkillTarget::Authored(other.id.into());
                    let InputOutput::Support { support, .. } = &mut case.receipts.inputs[1].output
                    else {
                        unreachable!()
                    };
                    support.target = DraftSkillTarget::Authored(other.id.into());
                    case.draft.skills.members.push(other);
                }
            }
            let before = case.draft.clone();
            let origins = b.origins.clone();
            assert!(proof(b, &case).unwrap().is_none(), "mode {mode}");
            assert_eq!(case.draft, before);
            assert_eq!(b.origins, origins);
        });
    }
}

fn missing_additions(role: &mut OwnedGemRoleRow, count: u32) {
    let OwnedGemSourceConstruction::Observed(facts) = &mut role.construction else {
        unreachable!()
    };
    for index in 1..=count {
        let effect_id = format!("unresolved-command-{index}");
        let reference = OwnedIndexedSourceEffectReference {
            index,
            effect: OwnedSourceEffectReference {
                effect_id: effect_id.clone(),
                resolution: OwnedSourceEffectResolution::Missing,
            },
        };
        facts.declared_additional_effects.push(reference.clone());
        facts.constructed_additional_effects.push(reference);
        for kind in [
            OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
            OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
        ] {
            facts.missing_references.push(OwnedMissingSourceEffect {
                kind,
                index: Some(index),
                effect_id: effect_id.clone(),
            });
        }
    }
    facts.missing_references.sort();
}

#[test]
fn constructed_primary_selection_retains_missing_references_and_strict_consumers() {
    with_joined_case_roles(
        |roles| missing_additions(&mut roles[0], 1),
        |b, case| {
            let roles_before = b.roles.input().clone();
            let receipt = &case.receipts.inputs[0];
            let UsageInventorySource::Physical { gem } = &receipt.identity else {
                unreachable!()
            };
            let row = &b.evidence.rows()[receipt.source.ordinal() as usize];
            let roles = b.roles;
            let role = skill_input_disposition::charged_source_role(b, roles, row, gem)
                .unwrap()
                .unwrap();
            assert!(!skill_input_disposition::physical_source_is_singleton(
                row, gem, role
            ));
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            let selector = proof(b, &case).unwrap().expect("one constructed primary");
            for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
                assert!(selector.proves(receipt, &group_field(name)));
                assert!(!selector.proves(&case.receipts.inputs[1], &group_field(name)));
            }
            for name in ["count", "enableGlobal1", "enableGlobal2"] {
                assert!(!selector.proves(receipt, &occurrence_field(name)));
            }
            assert_eq!(
                b.roles.input(),
                &roles_before,
                "missing references are not consumed"
            );
            assert_eq!(case.draft, before);
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        },
    );
}

#[test]
fn constructed_selector_refuses_omitted_mapped_additions_and_unproved_supports() {
    for mode in 0..3 {
        with_joined_case_roles(
            |roles| {
                missing_additions(&mut roles[0], 1);
                match mode {
                    0 => {
                        let OwnedGemSourceConstruction::Observed(facts) =
                            &mut roles[0].construction
                        else {
                            unreachable!()
                        };
                        // Structurally valid mapped reference, but deliberately
                        // absent from the alleged final construction list.
                        facts.constructed_additional_effects.push(
                            OwnedIndexedSourceEffectReference {
                                index: 2,
                                effect: facts.primary.clone(),
                            },
                        );
                    }
                    1 => missing_additions(&mut roles[1], 1),
                    _ => roles[0].construction = OwnedGemSourceConstruction::Unavailable,
                }
            },
            |b, case| {
                let before = case.draft.clone();
                let origins = b.origins.clone();
                assert!(proof(b, &case).unwrap().is_none(), "mode {mode}");
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
            },
        );
    }
}

#[test]
fn constructed_reference_scans_are_precharged_and_budget_failure_is_read_only() {
    let mut measured = Vec::new();
    for count in [1, 40] {
        with_joined_case_roles(
            |roles| missing_additions(&mut roles[0], count),
            |b, case| {
                let start = b.work;
                assert!(proof(b, &case).unwrap().is_some());
                measured.push(b.work - start);
            },
        );
    }
    assert_eq!(
        measured[1] - measured[0],
        78,
        "both added reference inventories are charged"
    );
    with_joined_case_roles(
        |roles| missing_additions(&mut roles[0], 40),
        |b, case| {
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            b.limits.max_work = b.work + measured[1] - 1;
            assert!(matches!(proof(b, &case), Err(NormalizationError::Limit(_))));
            assert_eq!(case.draft, before);
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        },
    );
}

#[test]
fn direct_prefix_joins_only_live_supports_to_the_exact_authored_occurrence() {
    // This is a private consumer fixture, not another receipt producer. Public
    // Direct normalizer tests separately prove successful reference attachment.
    with_joined_source_case(
        true,
        |_| {},
        |b, case| {
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            let selectors = proof(b, &case)
                .unwrap()
                .expect("live Direct + physical support");
            for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
                assert!(selectors.proves(&case.receipts.inputs[0], &group_field(name)));
                assert!(!selectors.proves(&case.receipts.inputs[1], &group_field(name)));
            }
            for name in ["count", "enableGlobal1", "enableGlobal2"] {
                assert!(!selectors.proves(&case.receipts.inputs[0], &occurrence_field(name)));
                assert!(!selectors.proves(&case.receipts.inputs[1], &occurrence_field(name)));
            }
            assert_eq!(case.draft, before);
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        },
    );
    for mode in 0..7 {
        with_joined_source_case(
            true,
            |_| {},
            |b, mut case| {
                match mode {
                    0 => case.draft.supports.members[0].enabled = false.into(),
                    1 => case.draft.skill_presets.members[0].supports.members.clear(),
                    2 => case.draft.skill_presets.members[0].skills.members.clear(),
                    3 => b.origins[case.source.ordinal() as usize]
                        .links
                        .retain(|link| !matches!(link, OwnedOriginTarget::Skill(_))),
                    4 => {
                        case.receipts.inputs.pop();
                    }
                    5 => case.draft.skills.members[0].enabled = false.into(),
                    _ => {
                        // Even a live selected alternative and consistently changed
                        // support snapshot cannot borrow this group's Direct root.
                        let mut other = case.draft.skills.members[0].clone();
                        other.id = b.id().unwrap();
                        case.draft.skill_presets.members[0]
                            .skills
                            .members
                            .push(other.id);
                        case.draft.supports.members[0].target =
                            DraftSkillTarget::Authored(other.id.into());
                        let InputOutput::Support { support, .. } =
                            &mut case.receipts.inputs[1].output
                        else {
                            unreachable!()
                        };
                        support.target = DraftSkillTarget::Authored(other.id.into());
                        case.draft.skills.members.push(other);
                    }
                }
                let before = case.draft.clone();
                let origins = b.origins.clone();
                let allocator = b.allocator.state();
                assert!(proof(b, &case).unwrap().is_none(), "mode {mode}");
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
                assert_eq!(b.allocator.state(), allocator);
            },
        );
    }
}
