//! Exact source correspondence uses the existing Item/member normalizer fixture.
//! These tests do not supply activation, numerical rules, or complete usage.
use super::*;
use poe_optimizer_core::owned_preset_intent::PresetApplicability;

fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn ports() -> DeclaredSlots {
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
fn generated_fixture() -> Fixture {
    let mut f = member_fixture();
    add_active_sibling(&mut f.artifacts, OwnedGemMaterialization::Physical);
    let role = f
        .artifacts
        .roles
        .input()
        .roles
        .iter()
        .find(|r| r.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse))
        .unwrap()
        .clone();
    let OwnedPrimarySkill::Known(skill) = role.primary else {
        panic!()
    };
    let modifier = f
        .items
        .input()
        .rules
        .iter()
        .find(|r| r.id == key("one-member"))
        .unwrap()
        .emissions
        .iter()
        .find_map(|e| {
            if let ItemEmission::Modifier { definition, .. } = e {
                Some(definition.clone())
            } else {
                None
            }
        })
        .unwrap();
    let template = match f.policy.item_modifier_membership.as_ref().unwrap() {
        ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 { templates, .. } => {
            templates[0].template.clone()
        }
        _ => panic!(),
    };
    let passive = f
        .artifacts
        .registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let unit = f
        .artifacts
        .registry
        .allocate_definition::<UnitDefinition>()
        .unwrap();
    let quality = f
        .artifacts
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Skill(skill.clone()))
        .unwrap();
    let level = f
        .artifacts
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let tree_supply = f
        .artifacts
        .registry
        .allocate_slot::<SkillGrantSlotDefinition>(SlotOwnerDefId::PassiveNode(passive.clone()))
        .unwrap();
    let item_supply = f
        .artifacts
        .registry
        .allocate_slot::<SkillGrantSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let mut schema = f.artifacts.schema.input().clone();
    schema.schema_version = OWNED_SCHEMA_PACKAGE_V6;
    let mut declarations = ports();
    declarations.skill_grants.members.push(tree_supply.clone());
    schema
        .definitions
        .push(DefinitionDescriptor::PassiveNode(known(
            passive.clone(),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![]),
                adjacent: DeclaredSet::complete(vec![]),
                declarations,
            },
        )));
    schema.definitions.push(DefinitionDescriptor::Unit(known(
        unit.clone(),
        UnitSchema {
            dimension: UnitDimension::PercentagePoints,
        },
    )));
    for definition in &mut schema.definitions {
        match definition {
            DefinitionDescriptor::Skill(row) if row.id == skill => {
                let mut declarations = ports();
                declarations.parameters.members.push(quality.clone());
                row.schema = SchemaState::Known(SkillSchema {
                    directly_selectable: false,
                    declarations,
                });
            }
            DefinitionDescriptor::Modifier(row) if row.id == modifier => {
                let SchemaState::Known(schema) = &mut row.schema else {
                    panic!()
                };
                schema.declarations.parameters.members.push(level.clone());
                schema
                    .declarations
                    .skill_grants
                    .members
                    .push(item_supply.clone());
            }
            _ => {}
        }
    }
    schema.slots.push(SlotDescriptor::Parameter(known(
        quality.clone(),
        ParameterSlotSchema {
            value: ValueSchema::Quantity(QuantityRange {
                minimum: FiniteQuantity::new(-100.0, unit.clone()).unwrap(),
                maximum: FiniteQuantity::new(200.0, unit.clone()).unwrap(),
            }),
            presence: SlotPresence::RequiredOnce,
            sites: vec![],
            skill_input: Some(SkillInputAuthority::Projected),
        },
    )));
    schema.slots.push(SlotDescriptor::Parameter(known(
        level.clone(),
        ParameterSlotSchema {
            value: ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            }),
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::ModifierRoll],
            skill_input: None,
        },
    )));
    for supply in [&tree_supply, &item_supply] {
        schema.slots.push(SlotDescriptor::SkillGrant(known(
            supply.clone(),
            SkillGrantSlotSchema {
                skill: skill.clone(),
                outputs: DeclaredSet::complete(vec![]),
                preset_inputs: Some(PresetSkillInputPermission {
                    schema_version: 1,
                    parameters: DeclaredSet::complete(vec![quality.clone()]),
                }),
            },
        )));
    }
    rebind_quality_schema(&mut f.artifacts, &mut f.policy, schema);
    let mut mapping = f.artifacts.mapping.input().clone();
    mapping.entries.extend([
        MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                effect_id: SourceComponent::Text("generated-effect".into()),
            }),
            outcome: MappingOutcome::Mapped {
                target: subject(&skill),
                basis: MappingBasis::Exact,
            },
        },
        MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
                tree_version: SourceComponent::Text("tree".into()),
                node_id: SourceComponent::Text("7".into()),
                view: SourceComponent::Missing,
            }),
            outcome: MappingOutcome::Mapped {
                target: subject(&passive),
                basis: MappingBasis::Exact,
            },
        },
    ]);
    f.artifacts.mapping = OwnedMappingIndex::new(
        mapping,
        &f.artifacts.registry,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = f.artifacts.roles.input().clone();
    roles.mapping = *f.artifacts.mapping.identity();
    f.artifacts.roles = OwnedSkillRoleIndex::new(
        roles,
        &f.artifacts.mapping,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    f.artifacts.rewards = empty_rewards(&f.artifacts.mapping, &f.artifacts.schema);
    let mut items = f.items.input().clone();
    items.definitions = f.artifacts.schema.identity().clone();
    let rule = items
        .rules
        .iter_mut()
        .find(|r| r.id == key("one-member"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!()
    };
    rolls.push(ItemRollTemplate {
        slot: level.clone(),
        value: ItemLineValue::Literal(ParameterValue::Integer(BoundedInteger::new(7).unwrap())),
    });
    f.items = OwnedItemLinePolicy::new(items, &f.artifacts.schema, Default::default()).unwrap();
    let mut item_source = f.item_source.input().clone();
    item_source.item_lines = *f.items.identity();
    f.item_source = ItemSourceLayoutPolicy::new(
        item_source,
        &f.items,
        &f.artifacts.schema,
        Default::default(),
    )
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
    let mut quality_recipe = value_recipe("generated-quality", "quality", false);
    quality_recipe.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Scientific,
        unit,
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    };
    let row = |provider, provider_level| GeneratedSkillInputRule {
        gem: role.gem.clone(),
        game_id: "sibling".into(),
        variant_id: "v".into(),
        skill_id: "generated-effect".into(),
        name_spec: "Generated".into(),
        skill: skill.clone(),
        provider,
        saved_level: value_recipe("saved-level", "level", false),
        provider_level,
        parameters: vec![GeneratedSkillParameterInput {
            field: GeneratedSkillInputField::Quality,
            slot: quality.clone(),
            value: quality_recipe.clone(),
        }],
    };
    f.policy.generated_skill_inputs = Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
        definitions: f.artifacts.schema.identity().clone(),
        source: pin(),
        roles: *f.artifacts.roles.identity(),
        catalog: f.artifacts.roles.input().compilation.catalog_digest,
        rows: vec![
            row(
                GeneratedSkillInputProvider::TreeAllocation {
                    source_node_id: "7".into(),
                    passive,
                    skill_supply: tree_supply,
                },
                GeneratedSkillProviderLevel::Fixed {
                    value: BoundedInteger::new(1).unwrap(),
                },
            ),
            row(
                GeneratedSkillInputProvider::ItemModifier {
                    modifier,
                    skill_supply: item_supply,
                    template,
                    source_name: "Test Title, Ordinary Base".into(),
                    name_lines: ["Rarity: RARE", "Test Title", "Ordinary Base"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, text)| GeneratedItemNameLine {
                            index,
                            text: text.into(),
                        })
                        .collect(),
                },
                GeneratedSkillProviderLevel::ModifierRoll { slot: level },
            ),
        ],
    });
    f
}
fn groups(quality: &str) -> String {
    format!(
        r#"<Skill source="Tree:7"><Gem gemId="sibling" variantId="v" skillId="generated-effect" nameSpec="Generated" level="1" quality="{quality}"/></Skill><Skill source="Item:1:Test Title, Ordinary Base" slot="coat"><Gem gemId="sibling" variantId="v" skillId="generated-effect" nameSpec="Generated" level="7" quality="20.25"/></Skill>"#
    )
}
fn document() -> String {
    format!(
        r#"<PathOfBuilding2><Tree activeSpec="2"><Spec treeVersion="tree" nodes="7"/><Spec treeVersion="tree" nodes="7"/></Tree><Items activeItemSet="2"><Item id="1">{SINGLE}</Item><ItemSet id="1">{SLOT}</ItemSet><ItemSet id="2">{SLOT}</ItemSet></Items><Skills activeSkillSet="2"><SkillSet id="1">{}</SkillSet><SkillSet id="2">{}</SkillSet><SkillSet id="3"/></Skills></PathOfBuilding2>"#,
        groups("31.5"),
        groups("12.5")
    )
}
fn bindings(
    result: &NormalizedImport,
    preset: usize,
) -> &DraftList<GeneratedSkillInputBindingDraft> {
    &result.draft().input().skill_presets.members[preset]
        .intent
        .as_ref()
        .unwrap()
        .generated_inputs
}

fn support_input_fixture() -> Fixture {
    let mut f = generated_fixture();
    f.policy.support_origin_order = Some(
        SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
            mapping_source: *f.artifacts.mapping.source_identity(),
            roles: *f.artifacts.roles.identity(),
        },
    );
    f
}

#[test]
fn authored_support_inputs_reuse_exact_generated_sources_but_not_archived_syntax() {
    let f = support_input_fixture();
    let result = run(&document(), &f);
    let presets = &result.draft().input().skill_presets.members;
    assert!(matches!(
        presets[0]
            .authored_support_order
            .as_ref()
            .unwrap()
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    for preset in &presets[1..] {
        let order = preset
            .authored_support_order
            .as_ref()
            .unwrap()
            .to_resolved()
            .unwrap();
        assert!(order.is_empty());
        assert!(preset.supports.to_resolved().unwrap().is_empty());
    }
    let mut absent = f.policy.clone();
    absent.generated_skill_inputs = None;
    let unsupported = normalize(&document(), &f, &absent, Default::default()).unwrap();
    assert!(matches!(
        unsupported.draft().input().skill_presets.members[1]
            .authored_support_order
            .as_ref()
            .unwrap()
            .completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn generated_support_correspondence_does_not_borrow_raw_quality_or_activation_readiness() {
    let f = support_input_fixture();
    for replacement in ["", "quality=\"bad\"", "quality=\"201\""] {
        let text = document().replace("quality=\"12.5\"", replacement).replace(
            "<Skill source=\"Tree:7\">",
            "<Skill source=\"Tree:7\" enabled=\"false\">",
        );
        let result = run(&text, &f);
        let preset = &result.draft().input().skill_presets.members[1];
        assert_eq!(
            preset.authored_support_order.as_ref().unwrap().completion,
            DraftListCompletion::Complete
        );
        assert!(
            bindings(&result, 1)
                .members
                .iter()
                .any(|r| r.parameters.to_resolved().is_none())
        );
    }
}

#[test]
fn generated_support_inputs_refuse_sharing_stale_providers_and_ambiguous_sources() {
    let f = support_input_fixture();
    let text = document();
    for changed in [
        text.replace("Item:1:Test Title", "Item:9:Test Title"),
        text.replace("slot=\"coat\"", "slot=\"coat Swap\""),
        text.replace("Tree:7", "Tree:07"),
        text.replace("level=\"7\"", "level=\"8\""),
        text.replace(
            "<SkillSet id=\"2\">",
            &format!("<SkillSet id=\"2\">{}", groups("18")),
        ),
        text.replace(
            "<SkillSet id=\"2\">",
            &format!(
                "<SkillSet id=\"2\"><Skill enabled=\"false\" slot=\"coat\">{}</Skill>",
                ACTIVE.replace("gemId=\"active\"", "gemId=\"sibling\"")
            ),
        ),
    ] {
        let result = run(&changed, &f);
        let preset = &result.draft().input().skill_presets.members[1];
        assert_eq!(
            preset.supports.completion,
            DraftListCompletion::Complete,
            "{changed}"
        );
        assert!(
            matches!(
                preset.authored_support_order.as_ref().unwrap().completion,
                DraftListCompletion::Pending { .. }
            ),
            "{changed}"
        );
    }
}

#[test]
fn exact_selected_axes_join_repeated_tree_and_item_providers_without_new_roots() {
    let f = generated_fixture();
    let source = document();
    let mut absent = f.policy.clone();
    absent.generated_skill_inputs = None;
    let old = normalize(&source, &f, &absent, Default::default()).unwrap();
    let new = run(&source, &f);
    assert!(matches!(
        bindings(&new, 0).completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(bindings(&new, 0).members.is_empty());
    assert!(matches!(
        bindings(&new, 2).completion,
        DraftListCompletion::Complete
    ));
    assert_eq!(new.draft().input().allocations.members.len(), 2);
    assert!(
        new.draft()
            .input()
            .allocations
            .members
            .iter()
            .all(|row| row.node.to_resolved().is_some())
    );
    assert!(
        closed(&new),
        "provider Item must retain the singleton modifier proof: {:?}",
        new.draft().input().items.members[0].modifiers.completion
    );
    let item = &new.draft().input().items.members[0];
    assert!(
        item.modifiers.members[0].rolls.to_resolved().is_some(),
        "provider raw level must be known"
    );
    let text = &new.sidecar().item_texts[0];
    assert!(matches!(text.attribution.layout, ItemLayoutStatus::Proven));
    assert_eq!(
        text.attribution
            .lines
            .iter()
            .take(3)
            .map(|row| row.semantic_text.as_str())
            .collect::<Vec<_>>(),
        ["Rarity: RARE", "Test Title", "Ordinary Base"]
    );
    assert!(
        matches!(bindings(&new, 1).completion, DraftListCompletion::Complete),
        "selected binding count={}, roots={:?}",
        bindings(&new, 1).members.len(),
        bindings(&new, 1)
            .members
            .iter()
            .map(|row| row.target.to_resolved().map(|v| v.provider.root))
            .collect::<Vec<_>>()
    );
    let rows = bindings(&new, 1).to_resolved().unwrap();
    assert_eq!(rows.len(), 2);
    let draft = new.draft().input();
    assert_eq!(
        rows[0].applicability,
        PresetApplicability::WhenExactSourceSelected
    );
    let allocations = &draft.allocation_presets.members[1].allocations.members;
    let equipment = &draft.equipment_presets.members[1].equipment.members;
    for row in &rows {
        let expected = match row.target.provider.root {
            ProviderRoot::Allocation(id) => {
                assert!(allocations.contains(&id));
                12.5
            }
            ProviderRoot::ItemModifier {
                equipment_use,
                modifier,
            } => {
                assert!(equipment.contains(&equipment_use));
                assert_eq!(modifier, draft.items.members[0].modifiers.members[0].id);
                20.25
            }
            _ => panic!("unexpected source root"),
        };
        assert!(row.target.provider.grant_path.is_empty());
        let ParameterValue::Quantity(value) = &row.parameters[0].value else {
            panic!()
        };
        assert_eq!(value.value(), expected);
    }
    let mut restored = draft.clone();
    restored.allocator = old.draft().input().allocator;
    for (new, old) in restored
        .skill_presets
        .members
        .iter_mut()
        .zip(&old.draft().input().skill_presets.members)
    {
        new.intent = None;
        new.usage_preferences = old.usage_preferences.clone();
    }
    assert_eq!(
        &restored,
        old.draft().input(),
        "no old instance, input or obligation changes"
    );
    let mut expected = serde_json::to_value(old.sidecar()).unwrap();
    let mut actual = serde_json::to_value(new.sidecar()).unwrap();
    assert_eq!(actual["schema_version"], 22);
    for field in ["policy", "draft", "allocator_after", "schema_version"] {
        expected[field] = actual[field].clone();
    }
    for origin in actual["origins"].as_array_mut().unwrap() {
        origin["links"].as_array_mut().unwrap().retain(|link| {
            link["kind"] != "generated_skill_input"
                && !(link["kind"] == "issue"
                    && serde_json::from_value::<DraftIssueId>(link["value"].clone())
                        .unwrap()
                        .instance_id()
                        .local()
                        > old.allocator_after().last_issued())
        });
    }
    assert_eq!(
        actual, expected,
        "every old provenance link is retained exactly"
    );
}

#[test]
fn attributed_name_positions_are_independent_of_raw_blank_line_numbers() {
    let f = generated_fixture();
    let padded = format!("\n  {}\n", SINGLE.replace('\n', "\n  "));
    let result = run(&document().replace(SINGLE, &padded), &f);
    let rows = bindings(&result, 1).to_resolved().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|row| matches!(row.target.provider.root, ProviderRoot::ItemModifier { .. }))
    );
    assert_eq!(
        rows,
        bindings(&run(&document(), &f), 1).to_resolved().unwrap()
    );
}

#[test]
fn archived_activation_rejoins_current_axes_without_first_match_or_fanout() {
    let f = generated_fixture();
    let text = document()
        .replace("activeSkillSet=\"2\"", "activeSkillSet=\"1\"")
        .replace("activeSpec=\"2\"", "activeSpec=\"1\"")
        .replace("activeItemSet=\"2\"", "activeItemSet=\"1\"");
    let result = run(&text, &f);
    let rows = bindings(&result, 0).to_resolved().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(
        |row| matches!(&row.parameters[0].value, ParameterValue::Quantity(v) if v.value() == 31.5)
    ));
    assert!(bindings(&result, 1).members.is_empty());
    assert_eq!(
        serde_json::to_value(run(&document(), &f).draft().input()).unwrap(),
        serde_json::to_value(run(&document(), &f).draft().input()).unwrap()
    );
}

#[test]
fn missing_and_malformed_quality_keep_exact_target_but_never_default() {
    let f = generated_fixture();
    for replacement in ["", "quality=\"bad\"", "quality=\"201\""] {
        let result = run(&document().replace("quality=\"12.5\"", replacement), &f);
        assert!(matches!(
            bindings(&result, 1).completion,
            DraftListCompletion::Complete
        ));
        assert_eq!(bindings(&result, 1).members.len(), 2);
        assert!(
            bindings(&result, 1)
                .members
                .iter()
                .any(|row| matches!(&row.parameters.members[0].value,
            DraftField::Pending(p) if p.code.as_str() == "generated-skill-input-value-unresolved"))
        );
    }
}

#[test]
fn stale_or_ambiguous_provider_source_is_pending_not_a_default_or_first_match() {
    let f = generated_fixture();
    let text = document();
    for changed in [
        text.replace("activeSpec=\"2\"", "activeSpec=\"0\""),
        text.replace("Tree:7", "Tree:07"),
        text.replace("nodes=\"7\"", "nodes=\"7,7\""),
        text.replace("slot=\"coat\"", "slot=\"coat Swap\""),
        text.replace("Item:1:Test Title", "Item:9:Test Title"),
        text.replace("Test Title\nOrdinary", "Renamed\nOrdinary"),
        text.replace("level=\"7\"", "level=\"8\""),
        text.replace("level=\"1\"", "level=\"2\""),
        text.replace(
            "<SkillSet id=\"2\">",
            &format!("<SkillSet id=\"2\">{}", groups("18")),
        ),
        text.replace("activeItemSet=\"2\"", "activeItemSet=\"02\""),
        text.replace("<ItemSet id=\"2\">", &format!("<ItemSet id=\"2\">{SLOT}")),
    ] {
        let result = run(&changed, &f);
        assert!(
            matches!(
                bindings(&result, 1).completion,
                DraftListCompletion::Pending { .. }
            ),
            "{changed}"
        );
        assert!(bindings(&result, 1).members.len() < 2);
    }
}

#[test]
fn policy_is_opt_in_strict_and_bound_to_real_permission_and_catalog() {
    let f = generated_fixture();
    let mut absent = f.policy.clone();
    absent.generated_skill_inputs = None;
    let wire = serde_json::to_value(&absent).unwrap();
    assert!(wire.get("generated_skill_inputs").is_none());
    let mut null = wire;
    null["generated_skill_inputs"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<NormalizationPolicy>(null).is_err());
    let mut unknown = serde_json::to_value(&f.policy).unwrap();
    unknown["generated_skill_inputs"]["kind"] = "pob_saved_generated_inputs_v2".into();
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
    for control in 0..5 {
        let mut policy = f.policy.clone();
        let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { roles, rows, .. }) =
            &mut policy.generated_skill_inputs
        else {
            panic!()
        };
        match control {
            0 => *roles = "f".repeat(64).parse().unwrap(),
            1 => rows[0].parameters[0].value.tiers[0].selectors[0].name = "level".into(),
            2 => rows[0].parameters[0].value.missing = MissingValuePolicy::Absent,
            3 => rows.push(rows[0].clone()),
            4 => rows[0].skill_id = "unmapped-effect".into(),
            _ => unreachable!(),
        }
        assert!(normalize(&document(), &f, &policy, Default::default()).is_err());
    }
}

#[test]
fn unadmitted_competing_source_row_cannot_select_the_later_quality() {
    let f = generated_fixture();
    for first in [
        r#"<Skill source="Tree:7"><Gem gemId="sibling" variantId="v" skillId="generated-effect" nameSpec="Generated" level="1.0" quality="10"/></Skill>"#,
        r#"<Skill source="Tree:7"><Gem gemId="sibling" variantId="v" skillId="generated-effect" nameSpec="Generated" level="1" quality="10"/><Gem skillId="unreviewed"/></Skill>"#,
    ] {
        let result = run(
            &document().replace(
                "<SkillSet id=\"2\">",
                &format!("<SkillSet id=\"2\">{first}"),
            ),
            &f,
        );
        let rows = bindings(&result, 1);
        assert!(matches!(
            rows.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(
            rows.members.len(),
            1,
            "only independent Item binding survives"
        );
        assert!(matches!(
            rows.members[0].target.to_resolved().unwrap().provider.root,
            ProviderRoot::ItemModifier { .. }
        ));
    }
}

#[test]
fn work_and_candidate_limits_fail_atomically_and_fresh_retry_is_identical() {
    let f = generated_fixture();
    let text = document();
    let first = run(&text, &f);
    assert!(
        normalize(
            &text,
            &f,
            &f.policy,
            NormalizationLimits {
                max_work: 100,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut limits = NormalizationLimits::default();
    // The bound includes the selector name as well as its value bytes.
    limits.value.max_total_candidate_bytes = 12;
    normalize(&text, &f, &f.policy, limits).unwrap();
    assert!(matches!(
        normalize(
            &text.replace("12.5", "12.5000000001"),
            &f,
            &f.policy,
            limits
        ),
        Err(NormalizationError::Value(
            ValuePolicyError::ResourceLimit { .. }
        ))
    ));
    assert_eq!(
        serde_json::to_value(first.sidecar()).unwrap(),
        serde_json::to_value(run(&text, &f).sidecar()).unwrap()
    );
}

#[path = "owned_occurrence_usage.rs"]
mod occurrence_usage_tests;
