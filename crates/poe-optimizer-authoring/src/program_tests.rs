//! Full-program syntax transports existing Core declarations; only Engine grants
//! their contextual authority. Constructor matrices below are not complete builds.
use crate::{lowering_tests as native, *};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact, RuleLimits,
};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn id<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("authored-rule-tests", "v1").unwrap(),
        value,
    )
    .unwrap()
}

fn quantity(value: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, id(unit)).unwrap())
}

fn quantity_type(unit: &str) -> ComputedValueType {
    ComputedValueType::Quantity { unit: id(unit) }
}

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}

fn symbols(f: &native::fixture::Fixture) -> SymbolRegistry {
    let definitions: Vec<_> = f
        .schema
        .input()
        .definitions
        .iter()
        .map(|entry| {
            let address = entry.address();
            (address.key().as_str().replace('-', "_"), address)
        })
        .collect();
    let slots: Vec<_> = f
        .schema
        .input()
        .slots
        .iter()
        .map(|entry| {
            let address = entry.address();
            (address.key().as_str().replace('-', "_"), address)
        })
        .collect();
    let definitions: Vec<_> = definitions
        .iter()
        .map(|(name, address)| (name.as_str(), address.clone()))
        .collect();
    let slots: Vec<_> = slots
        .iter()
        .map(|(name, address)| (name.as_str(), address.clone()))
        .collect();
    SymbolRegistry::new(&f.schema, &definitions, &slots, SymbolLimits::default()).unwrap()
}

fn lower(source: &str, f: &native::fixture::Fixture) -> LoweredProgram {
    lower_program(
        &parse_program(source, ProgramParseLimits::default()).unwrap(),
        &f.schema,
        &symbols(f),
        &f.rules.tables,
        ProgramLowerLimits::default(),
    )
    .unwrap()
}

fn reject(source: &str, f: &native::fixture::Fixture) -> Diagnostic {
    let error = match parse_program(source, ProgramParseLimits::default()) {
        Ok(parsed) => lower_program(
            &parsed,
            &f.schema,
            &symbols(f),
            &f.rules.tables,
            ProgramLowerLimits::default(),
        )
        .unwrap_err(),
        Err(error) => error,
    };
    assert!(error.span.text(source).is_some(), "{source}: {error}");
    error
}

const SUPPORT: &str = r#"
program "support-applicability" context action {
    read "capability" as supported: boolean = capability(current, capability.supported_action);
    let applicable = supported;
    let factor = quantity(1.5, unit.factor);
    effect "applicability" = support_applicability(applicable);
    effect "supported-factor" when applicable = contribute(current, stat.damage, multiply, factor);
}
"#;

#[test]
fn complete_support_program_is_alpha_equivalent_and_preserves_both_effects() {
    let mut f = native::fixture::fixture();
    let original = f.rules.owners[2].programs.members[0].clone();
    let reference =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let lowered = lower(SUPPORT, &f).program;
    assert_eq!(lowered.nodes.len(), 2);
    let mut expected = original.clone();
    let applicable = lowered.nodes[0].id.clone();
    let factor = lowered.nodes[1].id.clone();
    expected.nodes[0].id = applicable.clone();
    expected.nodes[1].id = factor.clone();
    expected.effects[0].effect = RuleEffectKind::SupportApplicability {
        applicable: applicable.clone(),
    };
    expected.effects[1].when = Some(applicable);
    let RuleEffectKind::Contribute { value, .. } = &mut expected.effects[1].effect else {
        panic!("the independently authored fixture must contain a contribution")
    };
    *value = factor;
    // Reads, IDs, context, source/target, contribution kind, guards and expression
    // bodies all compare. Only the two local SSA node identities were renamed.
    assert_eq!(lowered, expected);
    f.rules.owners[2].programs.members[0] = lowered;
    let compiled =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let mut observed = [false; 2];
    for case in f.cases.iter().filter(|case| case.program == original.id) {
        let facts: Vec<_> = case
            .facts
            .iter()
            .map(|(read, value)| RuleFact {
                read: read.clone(),
                value: value.clone(),
            })
            .collect();
        let before = reference
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut reference.new_scratch(),
            )
            .unwrap();
        let after = compiled
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(after.effects.len(), 2);
        for ((after, before), expected) in after
            .effects
            .iter()
            .zip(&before.effects)
            .zip(&case.expected)
        {
            assert_eq!(after.id, expected.id);
            assert_eq!(after.disposition, before.disposition);
            assert_eq!(
                after.disposition,
                match &expected.value {
                    Some(value) => EffectDisposition::Applied {
                        value: value.clone()
                    },
                    None => EffectDisposition::Inactive,
                }
            );
        }
        observed[usize::from(matches!(
            after.effects[1].disposition,
            EffectDisposition::Inactive
        ))] = true;
    }
    assert_eq!(observed, [true, true]);
}

#[test]
fn effect_guards_and_shared_selects_preserve_missing_and_numerical_failures() {
    let mut f = native::fixture::fixture();
    let source = SUPPORT.replace(
        "let factor = quantity(1.5, unit.factor);",
        "let invalid = quantity(1, unit.factor) / quantity(0, unit.factor);\n\
         let factor = if applicable { quantity(1.5, unit.factor) } else { invalid };",
    );
    f.rules.owners[2].programs.members[0] = lower(&source, &f).program;
    let owner = f.rules.owners[2].owner.clone();
    let run = |f: &native::fixture::Fixture, selected: Option<bool>| {
        let compiled =
            CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
        let facts: Vec<_> = selected
            .into_iter()
            .map(|value| RuleFact {
                read: key("capability"),
                value: ParameterValue::Boolean(value),
            })
            .collect();
        compiled
            .evaluate(
                &owner,
                &key("support-applicability"),
                &facts,
                &f.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap()
            .effects
    };
    let disabled = run(&f, Some(false));
    assert_eq!(
        disabled[0].disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(false)
        }
    );
    assert_eq!(disabled[1].disposition, EffectDisposition::Inactive);
    let enabled = run(&f, Some(true));
    assert_eq!(
        enabled[1].disposition,
        EffectDisposition::Applied {
            value: quantity(1.5, "unit.factor")
        }
    );
    for effect in run(&f, None) {
        assert_eq!(
            effect.disposition,
            EffectDisposition::Unresolved {
                input: key("capability")
            }
        );
    }
    // Removing the authored effect guard makes the same invalid branch demanded.
    // This proves the first result was not zero substitution or discarded syntax.
    f.rules.owners[2].programs.members[0].effects[1].when = None;
    assert!(matches!(
        run(&f, Some(false))[1].disposition,
        EffectDisposition::NumericalError {
            reason: NumericalFailure::DivisionByZero,
            ..
        }
    ));
}

fn enriched_fixture() -> native::fixture::Fixture {
    let mut f = native::fixture::fixture();
    let mut input = f.schema.input().clone();
    let blank = match f
        .schema
        .definition(&id::<ModifierDefinition>("modifier.conditioned"))
    {
        SchemaLookup::Known(schema) => schema.declarations.clone(),
        other => panic!("fixture modifier must be known: {other:?}"),
    };
    let range = IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    };
    let item = SlotOwnerDefId::ItemTemplate(id("item.template"));
    let parameter = DeclaredSlot {
        declaration: item.clone(),
        slot: id::<ParameterSlotDefinition>("parameter.owner"),
    };
    let choice = DeclaredSlot {
        declaration: item.clone(),
        slot: id::<ChoiceSlotDefinition>("choice.owner"),
    };
    let skill_parameter = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("skill.generated")),
        slot: id::<ParameterSlotDefinition>("parameter.skill"),
    };
    let supply = DeclaredSlot {
        declaration: item,
        slot: id::<SkillGrantSlotDefinition>("supply.skill"),
    };
    for entry in &mut input.definitions {
        if let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = entry
        {
            schema
                .declarations
                .parameters
                .members
                .push(parameter.clone());
            schema.declarations.choices.members.push(choice.clone());
            schema
                .declarations
                .skill_grants
                .members
                .push(supply.clone());
        }
    }
    let mut skill_declarations = blank.clone();
    skill_declarations
        .parameters
        .members
        .push(skill_parameter.clone());
    input.definitions.extend([
        DefinitionDescriptor::Class(known(
            id("class.player"),
            ClassSchema {
                level: range.clone(),
                ascendancies: DeclaredSet::complete(vec![id("ascendancy.player")]),
                implicit_passives: DeclaredSet::complete(vec![]),
                declarations: blank.clone(),
            },
        )),
        DefinitionDescriptor::Ascendancy(known(
            id("ascendancy.player"),
            AscendancySchema {
                classes: DeclaredSet::complete(vec![id("class.player")]),
                implicit_passives: DeclaredSet::complete(vec![]),
                declarations: blank.clone(),
            },
        )),
        DefinitionDescriptor::EquipmentSlot(known(
            id("equipment.ring"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Selected,
            },
        )),
        DefinitionDescriptor::ActionPart(known(id("part.primary"), ActionPartSchema {})),
        DefinitionDescriptor::ActionMode(known(id("mode.normal"), ActionModeSchema {})),
        DefinitionDescriptor::ActionStatSet(known(id("statset.primary"), ActionStatSetSchema {})),
        DefinitionDescriptor::Option(known(id("option.enabled"), OptionSchema {})),
        DefinitionDescriptor::ExternalInput(known(
            id("external.value"),
            ExternalInputSchema {
                value: ValueSchema::Integer(range.clone()),
                targets: vec![AssumptionTargetKind::Environment],
            },
        )),
        DefinitionDescriptor::Skill(known(
            id("skill.generated"),
            SkillSchema {
                directly_selectable: false,
                declarations: skill_declarations,
            },
        )),
        DefinitionDescriptor::Modifier(known(
            id("modifier.other"),
            ModifierSchema {
                declarations: blank,
            },
        )),
        DefinitionDescriptor::Stat(known(
            id("stat.factor"),
            StatSchema {
                value: quantity_type("unit.factor"),
                targets: vec![RuleEntityKind::Modifier],
            },
        )),
        DefinitionDescriptor::Stat(known(
            id("stat.eligible"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Modifier],
            },
        )),
    ]);
    input.slots.extend([
        SlotDescriptor::Parameter(known(
            parameter,
            ParameterSlotSchema {
                value: ValueSchema::Integer(range.clone()),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ItemParameter],
                skill_input: None,
            },
        )),
        SlotDescriptor::Choice(known(
            choice,
            ChoiceSlotSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(vec![id("option.enabled")]),
                },
                presence: SlotPresence::OptionalOnce,
                owners: vec![ChoiceOwnerScope::EquipmentUse],
            },
        )),
        SlotDescriptor::Parameter(known(
            skill_parameter,
            ParameterSlotSchema {
                value: ValueSchema::Integer(range),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
                skill_input: Some(SkillInputAuthority::Projected),
            },
        )),
        SlotDescriptor::SkillGrant(known(
            supply,
            SkillGrantSlotSchema {
                skill: id("skill.generated"),
                outputs: DeclaredSet::complete(vec![]),
                preset_inputs: None,
            },
        )),
    ]);
    f.schema = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    f.rules.definitions = f.schema.identity().clone();
    f
}

#[test]
fn every_read_constructor_preserves_the_exact_existing_core_variant() {
    let f = enriched_fixture();
    let parameter = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
        slot: id("parameter.owner"),
    };
    let choice = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
        slot: id("choice.owner"),
    };
    let skill_parameter = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("skill.generated")),
        slot: id("parameter.skill"),
    };
    // These are syntax/identity transport cases. One context cannot authorize all
    // these sources; the independent Engine refusal below retains that boundary.
    let cases = vec![
        (
            "integer",
            "effect_source_parameter(parameter.skill)",
            ComputedValueType::Integer,
            RuleReadSource::EffectSourceParameter {
                slot: skill_parameter,
            },
        ),
        (
            "option",
            "effect_source_choice(choice.owner)",
            ComputedValueType::Option,
            RuleReadSource::EffectSourceChoice {
                slot: choice.clone(),
            },
        ),
        (
            "integer",
            "parameter(parameter.owner)",
            ComputedValueType::Integer,
            RuleReadSource::Parameter { slot: parameter },
        ),
        (
            "option",
            "choice(choice.owner)",
            ComputedValueType::Option,
            RuleReadSource::Choice { slot: choice },
        ),
        (
            "integer",
            "character_level()",
            ComputedValueType::Integer,
            RuleReadSource::CharacterLevel,
        ),
        (
            "integer",
            "enemy_level()",
            ComputedValueType::Integer,
            RuleReadSource::EnemyLevel,
        ),
        (
            "boolean",
            "character_class_is(class.player)",
            ComputedValueType::Boolean,
            RuleReadSource::CharacterClassIs {
                class: id("class.player"),
            },
        ),
        (
            "boolean",
            "character_ascendancy_is(ascendancy.player)",
            ComputedValueType::Boolean,
            RuleReadSource::CharacterAscendancyIs {
                ascendancy: id("ascendancy.player"),
            },
        ),
        (
            "boolean",
            "player_equipment_slot(equipment.ring, occupied())",
            ComputedValueType::Boolean,
            RuleReadSource::PlayerEquipmentSlot {
                slot: id("equipment.ring"),
                read: PlayerEquipmentSlotRead::Occupied,
            },
        ),
        (
            "quantity(unit.damage)",
            "player_equipment_slot(equipment.ring, stat(stat.local_damage))",
            quantity_type("unit.damage"),
            RuleReadSource::PlayerEquipmentSlot {
                slot: id("equipment.ring"),
                read: PlayerEquipmentSlotRead::Stat {
                    stat: id("stat.local-damage"),
                },
            },
        ),
        (
            "boolean",
            "player_equipment_slot(equipment.ring, capability(capability.supported_action))",
            ComputedValueType::Boolean,
            RuleReadSource::PlayerEquipmentSlot {
                slot: id("equipment.ring"),
                read: PlayerEquipmentSlotRead::Capability {
                    capability: id("capability.supported-action"),
                },
            },
        ),
        (
            "boolean",
            "action_part_is(part.primary)",
            ComputedValueType::Boolean,
            RuleReadSource::ActionPartIs {
                part: id("part.primary"),
            },
        ),
        (
            "boolean",
            "action_mode_is(mode.normal)",
            ComputedValueType::Boolean,
            RuleReadSource::ActionModeIs {
                mode: id("mode.normal"),
            },
        ),
        (
            "boolean",
            "action_stat_set_is(statset.primary)",
            ComputedValueType::Boolean,
            RuleReadSource::ActionStatSetIs {
                stat_set: id("statset.primary"),
            },
        ),
        (
            "integer",
            "gem_level()",
            ComputedValueType::Integer,
            RuleReadSource::GemLevel,
        ),
        (
            "integer",
            "item_level()",
            ComputedValueType::Integer,
            RuleReadSource::ItemLevel,
        ),
        (
            "quantity(unit.percent)",
            "item_quality_amount(quality.local)",
            quantity_type("unit.percent"),
            RuleReadSource::ItemQualityAmount {
                quality: id("quality.local"),
            },
        ),
        (
            "boolean",
            "has_item_quality(quality.local)",
            ComputedValueType::Boolean,
            RuleReadSource::HasItemQuality {
                quality: id("quality.local"),
            },
        ),
        (
            "quantity(unit.percent)",
            "gem_quality_amount(quality.local)",
            quantity_type("unit.percent"),
            RuleReadSource::GemQualityAmount {
                quality: id("quality.local"),
            },
        ),
        (
            "boolean",
            "has_gem_quality(quality.local)",
            ComputedValueType::Boolean,
            RuleReadSource::HasGemQuality {
                quality: id("quality.local"),
            },
        ),
        (
            "quantity(unit.damage)",
            "stat(current, stat.local_damage)",
            quantity_type("unit.damage"),
            RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: id("stat.local-damage"),
            },
        ),
        (
            "quantity(unit.factor)",
            "modifier_transforms(stat.factor, stat.factor)",
            quantity_type("unit.factor"),
            RuleReadSource::ModifierTransforms {
                stat: id("stat.factor"),
                initial: id("stat.factor"),
            },
        ),
        (
            "boolean",
            "capability(current, capability.supported_action)",
            ComputedValueType::Boolean,
            RuleReadSource::Capability {
                entity: RuleEntity::Current,
                capability: id("capability.supported-action"),
            },
        ),
        (
            "integer",
            "external(environment, external.value)",
            ComputedValueType::Integer,
            RuleReadSource::External {
                entity: RuleEntity::Environment,
                input: id("external.value"),
            },
        ),
        (
            "quantity(unit.damage)",
            "contributions(current, stat.local_damage, add, sum, quantity(0, unit.damage))",
            quantity_type("unit.damage"),
            RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: id("stat.local-damage"),
                contribution: ContributionKind::Add,
                reduction: ContributionReduction::Sum,
                empty: quantity(0.0, "unit.damage"),
            },
        ),
        (
            "quantity(unit.damage)",
            "contribution_query(player, \"query-id\", \"group-id\")",
            quantity_type("unit.damage"),
            RuleReadSource::ContributionQuery {
                entity: RuleEntity::Player,
                query: key("query-id"),
                group: key("group-id"),
            },
        ),
        (
            "boolean",
            "contribution_selection(player, \"query-id\", \"group-id\", present)",
            ComputedValueType::Boolean,
            RuleReadSource::ContributionSelection {
                entity: RuleEntity::Player,
                query: key("query-id"),
                group: key("group-id"),
                projection: ContributionSelectionProjection::Present,
            },
        ),
        (
            "quantity(unit.damage)",
            "contribution_selection(player, \"query-id\", \"group-id\", value)",
            quantity_type("unit.damage"),
            RuleReadSource::ContributionSelection {
                entity: RuleEntity::Player,
                query: key("query-id"),
                group: key("group-id"),
                projection: ContributionSelectionProjection::Value,
            },
        ),
    ];
    assert_eq!(cases.len(), 28); // 25 variants + two equipment forms + second selection.
    for (ty, constructor, value_type, source) in cases {
        let text = format!(
            "program \"transport\" context actor {{ read \"source-id\" as input: {ty} = {constructor}; }}"
        );
        let actual = lower(&text, &f).program;
        assert_eq!(
            actual.reads,
            vec![RuleRead {
                id: key("source-id"),
                value_type,
                source
            }],
            "{constructor}"
        );
        assert!(
            actual.nodes.is_empty(),
            "read metadata must not emit nodes: {constructor}"
        );
        assert!(actual.effects.is_empty());
    }
}

#[test]
fn every_effect_constructor_preserves_source_order_targets_slots_and_values() {
    let f = enriched_fixture();
    let source = r#"program "effects" context equipment_use {
        let flag = true;
        let amount = quantity(2, unit.damage);
        let factor = quantity(1.5, unit.factor);
        let count = 2;
        effect "z-contribution" when flag = contribute(current, stat.local_damage, add, amount);
        effect "a-transform" = project_modifier_transform(stat.factor, targets(target(modifier.conditioned), target(modifier.other, stat.eligible)), 2, multiply, factor);
        effect "derive" = derive(current, stat.local_damage, amount);
        effect "capability" = capability(current, capability.supported_action, flag);
        effect "support" = support_applicability(flag);
        effect "grant" = activate_grant(grant.actor, flag);
        effect "skill" = project_skill_parameter(supply.skill, parameter.skill, count);
        effect "actor" = project_actor_stat(actor.supplied, stat.damage, amount);
        effect "requirement" = requirement(flag, "requirement-code");
    }"#;
    let program = lower(source, &f).program;
    assert_eq!(
        program.nodes.len(),
        4,
        "effect references emit no incidental nodes"
    );
    let flag = program.nodes[0].id.clone();
    let amount = program.nodes[1].id.clone();
    let factor = program.nodes[2].id.clone();
    let count = program.nodes[3].id.clone();
    let item = SlotOwnerDefId::ItemTemplate(id("item.template"));
    let kinds = vec![
        (
            "z-contribution",
            Some(flag.clone()),
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: id("stat.local-damage"),
                contribution: ContributionKind::Add,
                value: amount.clone(),
            },
        ),
        (
            "a-transform",
            None,
            RuleEffectKind::ProjectModifierTransform {
                stat: id("stat.factor"),
                targets: vec![
                    ModifierTransformTarget {
                        definition: id("modifier.conditioned"),
                        when: None,
                    },
                    ModifierTransformTarget {
                        definition: id("modifier.other"),
                        when: Some(id("stat.eligible")),
                    },
                ],
                order: BoundedInteger::new(2).unwrap(),
                operation: ModifierTransformOperation::Multiply,
                value: factor,
            },
        ),
        (
            "derive",
            None,
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("stat.local-damage"),
                value: amount.clone(),
            },
        ),
        (
            "capability",
            None,
            RuleEffectKind::Capability {
                entity: RuleEntity::Current,
                capability: id("capability.supported-action"),
                enabled: flag.clone(),
            },
        ),
        (
            "support",
            None,
            RuleEffectKind::SupportApplicability {
                applicable: flag.clone(),
            },
        ),
        (
            "grant",
            None,
            RuleEffectKind::ActivateGrant {
                slot: DeclaredSlot {
                    declaration: item.clone(),
                    slot: id("grant.actor"),
                },
                enabled: flag.clone(),
            },
        ),
        (
            "skill",
            None,
            RuleEffectKind::ProjectSkillParameter {
                skill: DeclaredSlot {
                    declaration: item.clone(),
                    slot: id("supply.skill"),
                },
                parameter: DeclaredSlot {
                    declaration: SlotOwnerDefId::Skill(id("skill.generated")),
                    slot: id("parameter.skill"),
                },
                value: count,
            },
        ),
        (
            "actor",
            None,
            RuleEffectKind::ProjectActorStat {
                actor: DeclaredSlot {
                    declaration: item,
                    slot: id("actor.supplied"),
                },
                stat: id("stat.damage"),
                value: amount,
            },
        ),
        (
            "requirement",
            None,
            RuleEffectKind::Requirement {
                satisfied: flag,
                code: key("requirement-code"),
            },
        ),
    ];
    assert_eq!(
        program.effects,
        kinds
            .into_iter()
            .map(|(id, when, effect)| RuleEffect {
                id: key(id),
                when,
                effect
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn static_metadata_rejects_unknown_wrong_kind_and_executable_arguments() {
    let f = enriched_fixture();
    for constructor in [
        "stat(current, unit.damage)",
        "stat(current, stat.unknown)",
        "parameter(grant.actor)",
        "character_class_is(ascendancy.player)",
        "player_equipment_slot(item.template, occupied())",
        "player_equipment_slot(equipment.ring, raw_json(1))",
        "player_equipment_slot(equipment.ring, occupied(1))",
        "contribution_query(player, query, \"group\")",
        "contribution_query(player, \"query\", \"group\", 1)",
        "contribution_selection(player, \"query\", \"group\", unknown)",
        "contributions(current, stat.local_damage, add, sum, 1 + 2)",
        "character_level(1)",
        "unknown()",
    ] {
        let text = format!(
            "program \"bad\" context actor {{ read \"r\" as input: integer = {constructor}; }}"
        );
        reject(&text, &f);
    }
    for constructor in [
        "derive(current, unit.damage, amount)",
        "derive(current, stat.local_damage, input)",
        "derive(current, stat.local_damage, amount + amount)",
        "derive(current, stat.local_damage, quantity(2, unit.damage))",
        "activate_grant(actor.supplied, flag)",
        "project_skill_parameter(actor.supplied, parameter.skill, count)",
        "project_actor_stat(supply.skill, stat.damage, amount)",
        "project_modifier_transform(stat.factor, targets(target(modifier.conditioned)), count, multiply, factor)",
        "project_modifier_transform(stat.factor, targets(target(unit.damage)), 0, multiply, factor)",
        "project_modifier_transform(stat.factor, targets(target(modifier.conditioned)), 0, divide, factor)",
        "requirement(flag, code)",
        "unknown(flag)",
    ] {
        let text = format!(
            "program \"bad\" context actor {{ read \"r\" as input: integer = character_level(); let flag = true; let amount = quantity(2, unit.damage); let factor = quantity(1, unit.factor); let count = 2; effect \"e\" = {constructor}; }}"
        );
        reject(&text, &f);
    }
}

#[test]
fn duplicate_ids_shadowing_forward_values_and_bad_guards_are_refused() {
    let f = native::fixture::fixture();
    for body in [
        "read \"same\" as a: integer = character_level(); read \"same\" as b: integer = enemy_level();",
        "read \"a\" as same: integer = character_level(); read \"b\" as same: integer = enemy_level();",
        "read \"r\" as a: integer = character_level(); let a = 1;",
        "let a = 1; let a = 2;",
        "let a = b; let b = 1;",
        "let a = true; effect \"e\" = requirement(a, \"c\"); effect \"e\" = requirement(a, \"d\");",
        "let a = 1; effect \"e\" when a = requirement(a, \"c\");",
        "read \"r\" as input: boolean = capability(current, capability.supported_action); let a = true; effect \"e\" when input = requirement(a, \"c\");",
        "let a = true; effect \"e\" when unknown = requirement(a, \"c\");",
        "let a = true; effect \"e\" when not a = requirement(a, \"c\");",
    ] {
        reject(&format!("program \"bad\" context actor {{ {body} }}"), &f);
    }
    // Core has distinct read/node/effect ID domains. Contextual words are valid
    // aliases, while `let` alone remains reserved by the expression lexer.
    let source = r#"program "read" context actor {
        read "same" as effect: integer = character_level();
        let when = effect > 0;
        effect "same" = requirement(when, "same");
    }"#;
    let program = lower(source, &f).program;
    assert_eq!(program.reads[0].id, program.effects[0].id);
    assert_eq!(program.id, key("read"));
}

#[test]
fn exact_context_and_relative_entity_tokens_do_not_infer_authority() {
    let mut f = native::fixture::fixture();
    for (text, context) in [
        ("actor", RuleEntityKind::Actor),
        ("action", RuleEntityKind::Action),
        ("equipment_use", RuleEntityKind::EquipmentUse),
        ("environment", RuleEntityKind::Environment),
        ("support_origin", RuleEntityKind::SupportOrigin),
        ("skill", RuleEntityKind::Skill),
    ] {
        assert_eq!(
            lower(&format!("program \"empty\" context {text} {{}}"), &f)
                .program
                .context,
            context
        );
    }
    for text in ["modifier", "Actor", "player", "unknown"] {
        reject(&format!("program \"bad\" context {text} {{}}"), &f);
    }
    for (text, entity) in [
        ("current", RuleEntity::Current),
        ("modifier", RuleEntity::Modifier),
        ("actor", RuleEntity::Actor),
        ("player", RuleEntity::Player),
        ("enemy", RuleEntity::Enemy),
        ("environment", RuleEntity::Environment),
        ("support_origin", RuleEntity::SupportOrigin),
        ("skill", RuleEntity::Skill),
        ("assigned_skill", RuleEntity::AssignedSkill),
        ("effect_source", RuleEntity::EffectSource),
        ("property_owner", RuleEntity::PropertyOwner),
    ] {
        let source = format!(
            "program \"transport\" context actor {{ read \"r\" as input: integer = stat({text}, stat.effective_attribute); }}"
        );
        assert_eq!(
            lower(&source, &f).program.reads[0].source,
            RuleReadSource::Stat {
                entity,
                stat: id("stat.effective-attribute")
            }
        );
    }
    let source = r#"program "support-applicability" context action {
        read "level" as level: integer = item_level();
        let positive = level > 0;
        effect "applicability" = support_applicability(positive);
    }"#;
    let lowered = lower(source, &f);
    f.rules.owners[2].programs.members[0] = lowered.program;
    let error =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap_err();
    assert!(
        error.to_string().contains("ItemLevel requires item owner"),
        "{error}"
    );

    let source = r#"program "support-applicability" context action {
        read "capability" as supported: integer = capability(current, capability.supported_action);
        let positive = supported > 0;
        effect "applicability" = support_applicability(positive);
    }"#;
    f.rules.owners[2].programs.members[0] = lower(source, &f).program;
    let error =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("declared read type does not match source schema"),
        "{error}"
    );
}

#[test]
fn program_source_maps_and_all_lowering_budgets_cover_metadata_and_formulas() {
    let f = native::fixture::fixture();
    let parsed = parse_program(SUPPORT, ProgramParseLimits::default()).unwrap();
    let registry = symbols(&f);
    let exact = ProgramLowerLimits {
        formulas: LowerLimits {
            max_syntax_nodes: parsed.nodes().len(),
            max_nodes: 2,
            max_statements: 2,
        },
        bindings: BindingLimits::default(),
        max_reads: 1,
        max_effects: 2,
    };
    let actual = lower_program(&parsed, &f.schema, &registry, &f.rules.tables, exact).unwrap();
    assert_eq!(actual.spans.id.text(SUPPORT), Some("support-applicability"));
    assert_eq!(actual.spans.context.text(SUPPORT), Some("action"));
    assert_eq!(actual.spans.reads.len(), actual.program.reads.len());
    assert_eq!(actual.spans.nodes.len(), actual.program.nodes.len());
    assert_eq!(actual.spans.effects.len(), actual.program.effects.len());
    assert_eq!(actual.bindings.len(), 2);
    assert_eq!(actual.bindings[0].name, "applicable");
    assert_eq!(actual.bindings[0].value, actual.program.nodes[0].id);
    for span in actual
        .spans
        .reads
        .iter()
        .chain(&actual.spans.nodes)
        .chain(&actual.spans.effects)
    {
        assert!(span.text(SUPPORT).is_some());
    }
    for limits in [
        ProgramLowerLimits {
            max_reads: 0,
            ..exact
        },
        ProgramLowerLimits {
            max_effects: 1,
            ..exact
        },
        ProgramLowerLimits {
            formulas: LowerLimits {
                max_syntax_nodes: parsed.nodes().len() - 1,
                ..exact.formulas
            },
            ..exact
        },
        ProgramLowerLimits {
            formulas: LowerLimits {
                max_nodes: 1,
                ..exact.formulas
            },
            ..exact
        },
        ProgramLowerLimits {
            formulas: LowerLimits {
                max_statements: 1,
                ..exact.formulas
            },
            ..exact
        },
    ] {
        let error =
            lower_program(&parsed, &f.schema, &registry, &f.rules.tables, limits).unwrap_err();
        assert!(
            matches!(error.code, DiagnosticCode::LimitExceeded(_)),
            "{error}"
        );
        assert!(error.span.text(SUPPORT).is_some());
    }
    let too_few_bindings = ProgramLowerLimits {
        bindings: BindingLimits {
            max_bindings: 3,
            ..BindingLimits::default()
        },
        ..exact
    };
    // The fixture has three Unit aliases plus one read; neither set is dropped
    // to make a bounded registry fit.
    assert!(
        lower_program(
            &parsed,
            &f.schema,
            &registry,
            &f.rules.tables,
            too_few_bindings
        )
        .is_err()
    );
    let empty = parse_program(
        "program \"empty\" context actor {}",
        ProgramParseLimits::default(),
    )
    .unwrap();
    let lowered = lower_program(
        &empty,
        &f.schema,
        &registry,
        &f.rules.tables,
        ProgramLowerLimits {
            max_reads: 0,
            max_effects: 0,
            ..ProgramLowerLimits::default()
        },
    )
    .unwrap();
    assert!(lowered.program.reads.is_empty());
    assert!(lowered.program.nodes.is_empty());
    assert!(lowered.program.effects.is_empty());

    let metadata = parse_program(
        "program \"metadata\" context actor { read \"r\" as value: integer = stat(current, stat.effective_attribute); }",
        ProgramParseLimits::default(),
    )
    .unwrap();
    let metadata_limit = ProgramLowerLimits {
        formulas: LowerLimits {
            max_syntax_nodes: metadata.nodes().len() - 1,
            ..LowerLimits::default()
        },
        ..ProgramLowerLimits::default()
    };
    assert!(matches!(
        lower_program(
            &metadata,
            &f.schema,
            &registry,
            &f.rules.tables,
            metadata_limit
        )
        .unwrap_err()
        .code,
        DiagnosticCode::LimitExceeded(LimitKind::Nodes)
    ));
}

#[test]
fn a_registry_cannot_carry_definition_or_slot_authority_into_a_different_index() {
    // Deliberately hostile index adapter: even an unchanged advertised identity
    // cannot authorize a missing descriptor. The normal cases use Data's loader.
    struct Missing<'a> {
        base: &'a OwnedDefinitionSchemaPackage,
        definition: Option<DefinitionAddress>,
        slot: Option<SlotAddress>,
    }
    impl DefinitionSchemaIndex for Missing<'_> {
        fn identity(&self) -> &poe_optimizer_core::data::DataIdentity {
            self.base.identity()
        }
        fn namespace(&self) -> &GameVersionNamespace {
            self.base.namespace()
        }
        fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
            if self.definition.as_ref() == Some(address) {
                None
            } else {
                self.base.lookup_definition(address)
            }
        }
        fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
            if self.slot.as_ref() == Some(address) {
                None
            } else {
                self.base.lookup_slot(address)
            }
        }
    }
    let f = native::fixture::fixture();
    let registry = symbols(&f);
    let parsed = parse_program(SUPPORT, ProgramParseLimits::default()).unwrap();
    let absent_stat = Missing {
        base: &f.schema,
        definition: Some(DefinitionAddress::Stat(id("stat.damage"))),
        slot: None,
    };
    assert_eq!(
        lower_program(
            &parsed,
            &absent_stat,
            &registry,
            &f.rules.tables,
            ProgramLowerLimits::default()
        )
        .unwrap_err()
        .code,
        DiagnosticCode::InvalidArguments
    );
    let source = "program \"grant\" context actor { let enabled = true; effect \"grant\" = activate_grant(grant.actor, enabled); }";
    let parsed = parse_program(source, ProgramParseLimits::default()).unwrap();
    let absent_slot = Missing {
        base: &f.schema,
        definition: None,
        slot: Some(SlotAddress::Grant(DeclaredSlot {
            declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
            slot: id("grant.actor"),
        })),
    };
    assert_eq!(
        lower_program(
            &parsed,
            &absent_slot,
            &registry,
            &f.rules.tables,
            ProgramLowerLimits::default()
        )
        .unwrap_err()
        .code,
        DiagnosticCode::InvalidArguments
    );
}

#[test]
fn program_syntax_keys_sections_and_metadata_are_strict_and_utf8_spanned() {
    let f = native::fixture::fixture();
    for source in [
        "program unquoted context actor {}",
        "program \"\" context actor {}",
        "program \"Upper\" context actor {}",
        "program \"bad key\" context actor {}",
        "program \"escaped\\n\" context actor {}",
        "program \"p\" context actor { read \"r\": integer = character_level(); }",
        "program \"p\" context actor { let a = 1; read \"r\" as level: integer = character_level(); }",
        "program \"p\" context actor { let a = true; effect \"e\" = requirement(a, \"c\"); let b = 2; }",
        "program \"p\" context actor { output result = 1; }",
        "program \"p\" context actor { let a = \"not-a-runtime-value\"; }",
        "program \"p\" context actor { let a = 1; } trailing",
        "// ⚡\nprogram \"p\" context actor { let a = 💥; }",
    ] {
        reject(source, &f);
    }
    let source = "// ⚡ program\nprogram \"p-1\" context actor {\nread \"r-1\" as amount: integer = character_level();\nlet result = amount;\neffect \"e-1\" = requirement(result, \"code-1\");\n}";
    let parsed = parse_program(source, ProgramParseLimits::default()).unwrap();
    for node in parsed.nodes() {
        assert!(node.span.text(source).is_some());
    }
    for read in parsed.reads() {
        assert!(read.span.text(source).unwrap().starts_with("read "));
    }
    for effect in parsed.effects() {
        assert!(effect.span.text(source).unwrap().starts_with("effect "));
    }
    for limits in [
        ProgramParseLimits {
            max_reads: 0,
            ..ProgramParseLimits::default()
        },
        ProgramParseLimits {
            max_statements: 0,
            ..ProgramParseLimits::default()
        },
        ProgramParseLimits {
            max_effects: 0,
            ..ProgramParseLimits::default()
        },
        ProgramParseLimits {
            expressions: ParseLimits {
                max_source_bytes: source.len() - 1,
                ..ParseLimits::default()
            },
            ..ProgramParseLimits::default()
        },
    ] {
        let error = parse_program(source, limits).unwrap_err();
        assert!(
            matches!(error.code, DiagnosticCode::LimitExceeded(_)),
            "{error}"
        );
    }
}
