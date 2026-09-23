//! Explicit missing-field policies require a complete, unambiguous source scope.
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

fn quantity(a: &Artifacts, n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, a.percent.clone()).unwrap())
}
fn fixture() -> Artifacts {
    let mut a = artifacts();
    let mut schema = a.schema.input().clone();
    let mut line_input = a.items.input().clone();
    let mut input = a.item_source.input().clone();
    let owner = SlotOwnerDefId::ItemTemplate(a.staff.clone());
    let presence = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(owner.clone())
        .unwrap();
    let amount = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(owner)
        .unwrap();
    for (slot, value) in [
        (presence.clone(), ValueSchema::Boolean),
        (
            amount.clone(),
            ValueSchema::Quantity(QuantityRange {
                minimum: FiniteQuantity::new(0.0, a.percent.clone()).unwrap(),
                maximum: FiniteQuantity::new(100.0, a.percent.clone()).unwrap(),
            }),
        ),
    ] {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ItemParameter],
                }),
            }));
        for row in &mut schema.definitions {
            if let DefinitionDescriptor::ItemTemplate(row) = row
                && row.id == a.staff
                && let SchemaState::Known(value) = &mut row.schema
            {
                value.declarations.parameters.members.push(slot.clone());
            }
        }
    }
    let mut amount_rule = line_input
        .rules
        .iter()
        .find(|r| {
            r.emissions
                .iter()
                .any(|e| matches!(e, ItemEmission::Quality { .. }))
        })
        .unwrap()
        .clone();
    amount_rule.id = key("catalyst-amount");
    amount_rule.pattern[0] = ItemPatternPart::Literal("CatalystQuality: ".into());
    let ItemEmission::Quality { amount: value, .. } = &amount_rule.emissions[0] else {
        panic!("quality");
    };
    amount_rule.emissions = vec![ItemEmission::ItemParameter {
        slot: amount.clone(),
        value: value.clone(),
    }];
    line_input.rules.push(amount_rule);
    line_input.rules.push(ItemLineRule {
        id: key("catalyst"),
        pattern: vec![ItemPatternPart::Literal("Catalyst: Tul".into())],
        captures: vec![],
        emissions: vec![ItemEmission::ItemParameter {
            slot: presence.clone(),
            value: ItemLineValue::Literal(ParameterValue::Boolean(true)),
        }],
    });
    for id in ["catalyst-amount", "catalyst"] {
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule: key(id),
            role: ItemRuleSourceRole::Header,
        });
    }
    input.template_defaults = vec![ItemSourceTemplateDefaults {
        template: a.staff.clone(),
        item_level: ItemSourceAbsentPolicy::Absent,
        quality: ItemSourceAbsentPolicy::Absent,
        parameters: vec![
            ItemSourceParameterDefault {
                assignment: ParameterAssignment {
                    slot: presence,
                    value: ParameterValue::Boolean(false),
                },
                headers: vec!["Catalyst".into()],
            },
            ItemSourceParameterDefault {
                assignment: ParameterAssignment {
                    slot: amount,
                    value: quantity(&a, 20.0),
                },
                headers: vec!["CatalystQuality".into()],
            },
        ],
    }];
    a.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    line_input.definitions = a.schema.identity().clone();
    a.items = OwnedItemLinePolicy::new(line_input, &a.schema, ItemLineLimits::default()).unwrap();
    input.item_lines = *a.items.identity();
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, ItemSourceLimits::default())
            .unwrap();
    a
}
fn xml(headers: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\n{headers}Implicits: 0\n128% increased Spell Damage</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(a: &Artifacts, xml: &str) -> ItemRangeAttribution {
    let imported = source(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}
fn rebuild(a: &mut Artifacts, input: ItemSourceLayoutPolicyInput) {
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, ItemSourceLimits::default())
            .unwrap();
}
#[test]
fn complete_source_scope_supplies_defaults_with_separate_provenance() {
    let a = fixture();
    let plan = attribute(&a, &xml(""));
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    assert!(matches!(
        plan.report().default_scope,
        ItemSourceDefaultScope::Proven { .. }
    ));
    let converted = plan.convert(&a.items).unwrap();
    assert!(converted.parameters.is_empty());
    assert_eq!(converted.defaults.parameters.len(), 2);
    assert_eq!(
        converted.defaults.parameters[0].value,
        ParameterValue::Boolean(false)
    );
    assert_eq!(converted.defaults.parameters[1].value, quantity(&a, 20.0));
    assert!(converted.defaults.item_level_absent && converted.defaults.quality_absent);
    assert!(converted.lines.iter().all(|l| l.index > 0));
    // Direct text parsing cannot establish source absence or receive defaults.
    let direct = a
        .items
        .convert_text("Ashen Staff\n128% increased Spell Damage")
        .unwrap();
    assert_eq!(direct.defaults, ItemDefaultedInputs::default());
}
#[test]
fn authored_values_zero_and_quality_remain_distinct_from_missing_defaults() {
    let a = fixture();
    for amount in [0.0, 7.5, 20.0] {
        let plan = attribute(
            &a,
            &xml(&format!(
                "Catalyst: Tul\nCatalystQuality: {amount}\nQuality: 12\nItem Level: 40\n"
            )),
        );
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(converted.parameters.len(), 2, "{:?}", converted);
        assert_eq!(
            converted.parameters[1].assignment.value,
            quantity(&a, amount)
        );
        assert!(converted.defaults.parameters.is_empty());
        assert!(!converted.defaults.item_level_absent && !converted.defaults.quality_absent);
        assert!(matches!(converted.quality, ItemField::Known { .. }));
    }
    let plan = attribute(&a, &xml("Catalyst: Tul\n"));
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.parameters.len(), 1);
    assert_eq!(converted.defaults.parameters.len(), 1);
    assert_eq!(converted.defaults.parameters[0].value, quantity(&a, 20.0));
}
#[test]
fn unknown_malformed_duplicate_and_unsupported_scope_never_activate_defaults() {
    let a = fixture();
    for headers in [
        "Catalyst: Unknown\n",
        "CatalystQuality: broken\n",
        "CatalystQuality: 7garbage\n",
        "Catalyst: Tul\nCatalyst: Tul\n",
        "CatalystQuality: 0\nCatalystQuality: 20\n",
        "Quality: 10\nQuality: 20\n",
        "Unknown Header: x\n",
    ] {
        let plan = attribute(&a, &xml(headers));
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(
            converted.defaults,
            ItemDefaultedInputs::default(),
            "{headers}"
        );
        assert!(
            !matches!(
                plan.report().default_scope,
                ItemSourceDefaultScope::Proven { .. }
            ),
            "{headers}"
        );
    }
    let malformed = xml("").replace("</Item>", "<Unknown/></Item>");
    assert_eq!(
        attribute(&a, &malformed)
            .convert(&a.items)
            .unwrap()
            .defaults,
        ItemDefaultedInputs::default()
    );
}
#[test]
fn opaque_headers_block_absence_and_a_different_template_cannot_borrow_defaults() {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    for rule in &mut lines.rules {
        if rule.id == key("catalyst")
            || rule.emissions.iter().any(|e| {
                matches!(
                    e,
                    ItemEmission::Quality { .. } | ItemEmission::ItemLevel { .. }
                )
            })
        {
            rule.emissions = vec![ItemEmission::Metadata {
                role: key("opaque"),
            }];
        }
    }
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, ItemLineLimits::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.item_lines = *a.items.identity();
    rebuild(&mut a, input);
    let plan = attribute(&a, &xml("Catalyst: Tul\nQuality: 0\nItem Level: 40\n"));
    let converted = plan.convert(&a.items).unwrap();
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    assert_eq!(converted.defaults.parameters.len(), 1); // only absent amount
    assert!(!converted.defaults.quality_absent && !converted.defaults.item_level_absent);
    let different = xml("").replace("Ashen Staff", "Grand Spear");
    assert_eq!(
        attribute(&a, &different)
            .convert(&a.items)
            .unwrap()
            .defaults,
        ItemDefaultedInputs::default()
    );
    // A title which looks like a header is presentation, not presence.
    let titled = xml("").replace("New Item", "Catalyst: Tul");
    assert_eq!(
        attribute(&a, &titled)
            .convert(&a.items)
            .unwrap()
            .defaults
            .parameters
            .len(),
        2
    );
}
#[test]
fn defaults_validate_exact_owner_values_and_explicit_wire_fields() {
    let a = fixture();
    let input = a.item_source.input().clone();
    let reject = |value| {
        ItemSourceLayoutPolicy::new(value, &a.items, &a.schema, ItemSourceLimits::default())
            .is_err()
    };
    let mut value = input.clone();
    value.schema_version = 2;
    assert!(reject(value));
    let mut value = input.clone();
    value
        .template_defaults
        .push(value.template_defaults[0].clone());
    assert!(reject(value));
    let mut value = input.clone();
    let duplicate = value.template_defaults[0].parameters[0].clone();
    value.template_defaults[0].parameters.push(duplicate);
    assert!(reject(value));
    let mut value = input.clone();
    value.template_defaults[0].template = a.spear.clone();
    assert!(reject(value));
    let mut value = input.clone();
    value.template_defaults[0].parameters[0].assignment.value = quantity(&a, 20.0);
    assert!(reject(value));
    let mut value = input.clone();
    value.template_defaults[0].parameters[1].assignment.value = quantity(&a, 101.0);
    assert!(reject(value));
    for header in ["", "Catalyst:", " Catalyst", "Catalyst\n"] {
        let mut value = input.clone();
        value.template_defaults[0].parameters[0].headers = vec![header.into()];
        assert!(reject(value));
    }
    let mut json = serde_json::to_value(input).unwrap();
    json.as_object_mut().unwrap().remove("template_defaults");
    assert!(serde_json::from_value::<ItemSourceLayoutPolicyInput>(json).is_err());
}
#[test]
fn constructor_encoder_and_runtime_enforce_tightened_default_budgets() {
    let a = fixture();
    let input = a.item_source.input().clone();
    let limits = ItemSourceLimits {
        max_default_parameters: 1,
        ..Default::default()
    };
    assert!(ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_err());
    assert!(encode_item_source_policy(&a.item_source, limits).is_err());
    let limits = ItemSourceLimits {
        max_schema_work: 1,
        ..Default::default()
    };
    assert!(ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_err());
    assert!(encode_item_source_policy(&a.item_source, limits).is_err());
    let bytes = encode_item_source_policy(&a.item_source, ItemSourceLimits::default()).unwrap();
    let decoded =
        decode_item_source_policy(&bytes, &a.items, &a.schema, ItemSourceLimits::default())
            .unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    let mut low = 1;
    let mut high = ItemSourceLimits::default().max_policy_text_bytes;
    while low < high {
        let mid = (low + high) / 2;
        let limits = ItemSourceLimits {
            max_policy_text_bytes: mid,
            ..Default::default()
        };
        if ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let limits = ItemSourceLimits {
        max_policy_text_bytes: low,
        ..Default::default()
    };
    let bytes = encode_item_source_policy(&a.item_source, limits).unwrap();
    assert!(decode_item_source_policy(&bytes, &a.items, &a.schema, limits).is_ok());
    let limits = ItemSourceLimits {
        max_policy_text_bytes: low - 1,
        ..Default::default()
    };
    assert!(encode_item_source_policy(&a.item_source, limits).is_err());
}

#[test]
fn schema_budget_includes_all_item_rule_emissions() {
    let mut a = fixture();
    let input = a.item_source.input().clone();
    let mut low = 1;
    let mut high = ItemSourceLimits::default().max_schema_work;
    while low < high {
        let mid = (low + high) / 2;
        let limits = ItemSourceLimits {
            max_schema_work: mid,
            ..Default::default()
        };
        if ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let mut lines = a.items.input().clone();
    for index in 0..256 {
        lines.rules.push(ItemLineRule {
            id: key(&format!("extra-{index}")),
            pattern: vec![ItemPatternPart::Literal(format!("extra-{index}"))],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata { role: key("extra") }],
        });
    }
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, ItemLineLimits::default()).unwrap();
    let mut input = input;
    input.item_lines = *a.items.identity();
    assert!(
        ItemSourceLayoutPolicy::new(
            input.clone(),
            &a.items,
            &a.schema,
            ItemSourceLimits::default()
        )
        .is_ok()
    );
    let limits = ItemSourceLimits {
        max_schema_work: low + 100,
        ..Default::default()
    };
    assert!(matches!(
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, limits),
        Err(ItemSourceError::Limit("schema work"))
    ));
}

#[test]
fn independently_authored_alias_suppresses_a_default_for_the_same_slot() {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    let mut alias = lines
        .rules
        .iter()
        .find(|r| r.id == key("catalyst-amount"))
        .unwrap()
        .clone();
    alias.id = key("amount-alias");
    alias.pattern[0] = ItemPatternPart::Literal("LevelReq: ".into());
    lines.rules.retain(|r|!matches!(r.pattern.first(),Some(ItemPatternPart::Literal(s)) if s.starts_with("LevelReq:")));
    lines.rules.push(alias);
    let mut input = a.item_source.input().clone();
    let existing: std::collections::BTreeSet<_> =
        lines.rules.iter().map(|r| r.id.clone()).collect();
    input.rule_layouts.retain(|r| existing.contains(&r.rule));
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("amount-alias"),
        role: ItemRuleSourceRole::Header,
    });
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, ItemLineLimits::default()).unwrap();
    input.item_lines = *a.items.identity();
    rebuild(&mut a, input);
    let plan = attribute(&a, &xml("LevelReq: 7\n"));
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.parameters.len(), 1);
    assert_eq!(converted.parameters[0].assignment.value, quantity(&a, 7.0));
    assert_eq!(converted.defaults.parameters.len(), 1); // selector only, never overwrite amount
    assert_eq!(
        converted.defaults.parameters[0].value,
        ParameterValue::Boolean(false)
    );
}

#[test]
fn production_normalization_preserves_default_provenance_and_partial_closure() {
    use poe_optimizer_core::owned_draft::DraftListCompletion;
    use poe_optimizer_import::{owned_mapping::*, owned_reward_policy::*, owned_skill_catalog::*};
    let mut a = fixture();
    // Rebind this directly authored fixture after adding the template's input slots.
    let mut mapping = a.mapping.input().clone();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping = OwnedMappingIndex::new(
        mapping,
        &a.registry,
        &a.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = a.roles.input().clone();
    roles.definitions = a.schema.identity().clone();
    roles.mapping = *a.mapping.identity();
    roles.compilation.base_registry = a.registry.identity().unwrap();
    roles.compilation.staged_registry = a.registry.identity().unwrap();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, SkillCatalogLimits::default())
        .unwrap();
    let mut rewards = a.rewards.input().clone();
    rewards.definitions = a.schema.identity().clone();
    rewards.mapping = *a.mapping.identity();
    a.rewards = OwnedRewardPolicy::new(
        rewards,
        &a.mapping,
        &a.schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    for (headers, defaulted, absent) in [
        ("", 2, true),
        (
            "Catalyst: Tul\nCatalystQuality: 0\nQuality: 12\nItem Level: 40\n",
            0,
            false,
        ),
    ] {
        let imported = source(&xml(headers));
        let normalized = support::normalize(&imported, &a);
        let record = &normalized.draft().input().items.members[0];
        let provenance = &normalized.sidecar().item_texts[0];
        assert_eq!(normalized.sidecar().schema_version, 11);
        assert_eq!(record.parameters.members.len(), 2);
        assert_eq!(provenance.defaults.parameters.len(), defaulted);
        assert_eq!(
            record.item_level.to_resolved(),
            Some(if absent { None } else { Some(40) })
        );
        assert_eq!(record.quality.to_resolved().unwrap().is_none(), absent);
        assert!(matches!(
            record.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            record.modifiers.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            provenance.attribution.default_scope,
            ItemSourceDefaultScope::Proven { .. }
        ));
        assert_eq!(
            record.parameters.members[1].value.to_resolved(),
            Some(quantity(&a, if absent { 20.0 } else { 0.0 }))
        );
        assert!(provenance.lines.iter().all(|line| line.index > 0));
    }
    let imported = source(&xml("CatalystQuality: broken\n"));
    let normalized = support::normalize(&imported, &a);
    let record = &normalized.draft().input().items.members[0];
    assert_eq!(record.item_level.to_resolved(), None);
    assert_eq!(record.quality.to_resolved(), None);
    assert!(
        normalized.sidecar().item_texts[0]
            .defaults
            .parameters
            .is_empty()
    );
}

#[test]
fn sparse_defaults_scale_to_a_full_base_catalog_without_defaulting_unrelated_templates() {
    let mut a = fixture();
    let expected_missing = attribute(&a, &xml("")).convert(&a.items).unwrap().defaults;
    let expected_authored = attribute(&a, &xml("Catalyst: Tul\nCatalystQuality: 0\n"))
        .convert(&a.items)
        .unwrap()
        .defaults;
    let mut schema = a.schema.input().clone();
    let SchemaLookup::Known(unrelated_schema) = a.schema.definition(&a.spear) else {
        panic!("known fixture template");
    };
    let unrelated_schema = unrelated_schema.clone();
    let mut lines = a.items.input().clone();
    let mut input = a.item_source.input().clone();
    assert_eq!(input.template_defaults.len(), 1);
    // Two fixture templates plus 1,754 unrelated bases reproduce the breadth
    // of the final source catalog, while only the staff requests defaults.
    for index in 0..1754 {
        let template = a
            .registry
            .allocate_definition::<ItemTemplateDefinition>()
            .unwrap();
        schema
            .definitions
            .push(DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                id: template.clone(),
                schema: SchemaState::Known(unrelated_schema.clone()),
            }));
        let rule = key(&format!("unrelated-base-{index}"));
        lines.rules.push(ItemLineRule {
            id: rule.clone(),
            pattern: vec![ItemPatternPart::Literal(format!("Unrelated Base {index}"))],
            captures: vec![],
            emissions: vec![ItemEmission::Template {
                definition: template.clone(),
            }],
        });
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule,
            role: ItemRuleSourceRole::Header,
        });
        input.template_layouts.push(ItemTemplateSourceLayout {
            template,
            load_index_prefix: ItemLoadIndexPrefix::Unresolved,
        });
    }
    a.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    lines.definitions = a.schema.identity().clone();
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, ItemLineLimits::default()).unwrap();
    input.item_lines = *a.items.identity();
    let limits = ItemSourceLimits {
        max_schema_work: 200_000,
        ..Default::default()
    };
    a.item_source =
        ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).unwrap();
    assert_eq!(
        attribute(&a, &xml("")).convert(&a.items).unwrap().defaults,
        expected_missing
    );
    assert_eq!(
        attribute(&a, &xml("Catalyst: Tul\nCatalystQuality: 0\n"))
            .convert(&a.items)
            .unwrap()
            .defaults,
        expected_authored
    );
    let unrelated = xml("").replace("Ashen Staff", "Unrelated Base 1753");
    assert_eq!(
        attribute(&a, &unrelated)
            .convert(&a.items)
            .unwrap()
            .defaults,
        ItemDefaultedInputs::default()
    );
    let malformed = xml("CatalystQuality: malformed\n");
    assert_eq!(
        attribute(&a, &malformed)
            .convert(&a.items)
            .unwrap()
            .defaults,
        ItemDefaultedInputs::default()
    );
    let bytes = encode_item_source_policy(&a.item_source, limits).unwrap();
    let decoded = decode_item_source_policy(&bytes, &a.items, &a.schema, limits).unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    // Unrelated emissions still consume bounded work; the optimization is not
    // an exemption from scanning or accounting for supplied catalog content.
    assert!(matches!(
        ItemSourceLayoutPolicy::new(
            input.clone(),
            &a.items,
            &a.schema,
            ItemSourceLimits {
                max_schema_work: 1_000,
                ..Default::default()
            }
        ),
        Err(ItemSourceError::Limit("schema work"))
    ));
    // Finding many other templates must never satisfy the requested binding.
    let mut missing = a.items.input().clone();
    missing.rules.retain(|rule| {
        !rule.emissions.iter().any(|emission|
        matches!(emission, ItemEmission::Template { definition } if definition == &a.staff))
    });
    input
        .rule_layouts
        .retain(|layout| missing.rules.iter().any(|rule| rule.id == layout.rule));
    let missing = OwnedItemLinePolicy::new(missing, &a.schema, ItemLineLimits::default()).unwrap();
    input.item_lines = *missing.identity();
    assert!(matches!(
        ItemSourceLayoutPolicy::new(input, &missing, &a.schema, limits),
        Err(ItemSourceError::Policy(
            "default template has no source layout or line binding"
        ))
    ));
}
