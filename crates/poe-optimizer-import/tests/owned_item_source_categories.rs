//! Only a proved complete source layout may provide owned category Options.
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

struct Fixture {
    a: Artifacts,
    options: [OptionDefId; 3],
    outside: OptionDefId,
    slot: DeclaredSlot<ParameterSlotDefId>,
}

fn bindings(input: &mut ItemSourceLayoutPolicyInput) -> &mut Vec<ItemSourceCategoryBinding> {
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        category_bindings, ..
    } = &mut input.dialect
    else {
        panic!("category dialect")
    };
    category_bindings
}

fn fixture() -> Fixture {
    let mut a = artifacts();
    let modifier = a.modifiers["spell"].definition.clone();
    let mut schema = a.schema.input().clone();
    let mut lines = a.items.input().clone();
    let mut input = a.item_source.input().clone();
    let options = std::array::from_fn(|_| {
        let id = a
            .registry
            .allocate_definition::<OptionDefinition>()
            .unwrap();
        schema
            .definitions
            .push(DefinitionDescriptor::Option(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(OptionSchema {}),
            }));
        id
    });
    let outside = a
        .registry
        .allocate_definition::<OptionDefinition>()
        .unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::Option(DefinitionEntry {
            id: outside.clone(),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    let slot = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: slot.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(options.to_vec()),
                },
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ModifierRoll],
            }),
        }));
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::Modifier(row) = definition
            && row.id == modifier
            && let SchemaState::Known(s) = &mut row.schema
        {
            s.declarations.parameters.members.push(slot.clone());
        }
    }
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("spell"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("modifier")
    };
    rolls.push(ItemRollTemplate {
        slot: slot.clone(),
        value: ItemLineValue::ContextOption {
            input: key("category"),
        },
    });
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    lines.schema_version = OWNED_ITEM_LINE_POLICY_V7;
    lines.definitions = a.schema.identity().clone();
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    input.schema_version = OWNED_ITEM_SOURCE_CATEGORY_POLICY_VERSION;
    input.item_lines = *a.items.identity();
    input.dialect = ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        flag_bindings: vec![],
        metadata_rules: vec![],
        single_modifier_conditions: vec![],
        preamble_observations: vec![],
        category_bindings: vec![ItemSourceCategoryBinding {
            input: key("category"),
            values: [
                SourceModifierCategory::Explicit,
                SourceModifierCategory::Implicit,
                SourceModifierCategory::Enchant,
            ]
            .into_iter()
            .zip(options.iter())
            .map(|(category, value)| ItemSourceCategoryValue {
                category,
                value: value.clone(),
            })
            .collect(),
        }],
    };
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
    Fixture {
        a,
        options,
        outside,
        slot,
    }
}

fn xml(count: usize, body: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nImplicits: {count}\n{body}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(a: &Artifacts, xml: &str) -> ItemRangeAttribution {
    let imported = source(xml);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}
fn category_values(f: &Fixture, plan: &ItemRangeAttribution) -> Vec<OptionDefId> {
    let converted = plan.convert(&f.a.items).unwrap();
    converted
        .modifiers
        .iter()
        .flat_map(|m| &m.rolls)
        .filter_map(|roll| {
            if roll.slot != f.slot {
                return None;
            }
            let ParameterValue::Option(value) = &roll.value else {
                panic!("Option")
            };
            Some(value.clone())
        })
        .collect()
}
fn compile(
    f: &Fixture,
    input: ItemSourceLayoutPolicyInput,
) -> std::result::Result<ItemSourceLayoutPolicy, ItemSourceError> {
    ItemSourceLayoutPolicy::new(input, &f.a.items, &f.a.schema, Default::default())
}

#[test]
fn proved_categories_follow_header_counts_tag_precedence_and_distinct_occurrences() {
    let f = fixture();
    for (count, body, expected) in [
        (0, "128% increased Spell Damage", vec![0]),
        (1, "128% increased Spell Damage", vec![1]),
        (
            1,
            "128% increased Spell Damage\n129% increased Spell Damage",
            vec![1, 0],
        ),
        (0, "{implicit}128% increased Spell Damage", vec![1]),
        (0, "{enchant}128% increased Spell Damage", vec![2]),
        (0, "{implicit}{enchant}128% increased Spell Damage", vec![2]),
        (0, "{enchant}{implicit}128% increased Spell Damage", vec![2]),
        (
            1,
            "{enchant}128% increased Spell Damage\n129% increased Spell Damage",
            vec![2, 0],
        ),
    ] {
        let plan = attribute(&f.a, &xml(count, body));
        assert!(
            matches!(plan.report().layout, ItemLayoutStatus::Proven),
            "{:?}",
            plan.report()
        );
        assert_eq!(
            category_values(&f, &plan),
            expected
                .into_iter()
                .map(|i| f.options[i].clone())
                .collect::<Vec<_>>()
        );
        assert!(plan.convert(&f.a.items).unwrap().issues.is_empty());
    }
}

#[test]
fn pending_local_category_guesses_never_supply_context_or_erase_source_candidates() {
    let f = fixture();
    // A first unknown can consume an implicit slot in PoB, so the later local
    // guessed position must not become a canonical Option. A later unknown also
    // keeps the whole proof incomplete even for an earlier recognized member.
    for body in [
        "Unknown source member\n49% increased Attack Speed\n128% increased Spell Damage",
        "128% increased Spell Damage\nUnknown source member",
    ] {
        let plan = attribute(&f.a, &xml(3, body));
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
        let row = plan
            .report()
            .lines
            .iter()
            .find(|r| r.semantic_text == "128% increased Spell Damage")
            .unwrap();
        assert!(
            row.member.is_some(),
            "this regression needs a tempting diagnostic guess"
        );
        assert_eq!(row.rule, Some(key("spell")));
        assert!(category_values(&f, &plan).is_empty());
        let converted = plan.convert(&f.a.items).unwrap();
        let row = converted
            .lines
            .iter()
            .find(|r| r.text == "128% increased Spell Damage")
            .unwrap();
        assert_eq!(
            row.outcome,
            ItemLineOutcome::Pending {
                reason: ItemLinePending::MissingContextOption {
                    input: key("category")
                },
                candidates: vec![key("spell")],
            }
        );
    }
    let invalid_overlay = xml(0, "128% increased Spell Damage")
        .replace("</Item>", "<ModRange id=\"1\" range=\"2\"/></Item>");
    let plan = attribute(&f.a, &invalid_overlay);
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
    assert!(
        category_values(&f, &plan).is_empty(),
        "projection must follow final overlay validation"
    );
}

#[test]
fn missing_category_mapping_is_unresolved_with_no_explicit_fallback() {
    let mut f = fixture();
    let mut input = f.a.item_source.input().clone();
    bindings(&mut input)[0]
        .values
        .retain(|r| r.category == SourceModifierCategory::Explicit);
    f.a.item_source = compile(&f, input).unwrap();
    for body in [
        "{implicit}128% increased Spell Damage",
        "{enchant}128% increased Spell Damage",
    ] {
        let plan = attribute(&f.a, &xml(0, body));
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
        assert!(category_values(&f, &plan).is_empty());
        let converted = plan.convert(&f.a.items).unwrap();
        assert!(matches!(
            &converted.lines.last().unwrap().outcome,
            ItemLineOutcome::Pending {
                reason: ItemLinePending::MissingContextOption { .. },
                ..
            }
        ));
    }
    assert_eq!(
        category_values(&f, &attribute(&f.a, &xml(0, "128% increased Spell Damage"))),
        vec![f.options[0].clone()]
    );
}

#[test]
fn proved_category_does_not_close_partial_modifier_input_membership() {
    let mut f = fixture();
    let modifier = f.a.modifiers["spell"].definition.clone();
    let closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(modifier.address()),
            facet: SchemaFacet::InputSchema,
            code: key("remaining-owned-inputs"),
        }],
    };
    let mut schema = f.a.schema.input().clone();
    for row in &mut schema.definitions {
        if let DefinitionDescriptor::Modifier(row) = row
            && row.id == modifier
            && let SchemaState::Known(s) = &mut row.schema
        {
            s.declarations.parameters.closure = closure.clone();
        }
    }
    f.a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut lines = f.a.items.input().clone();
    lines.definitions = f.a.schema.identity().clone();
    f.a.items = OwnedItemLinePolicy::new(lines, &f.a.schema, Default::default()).unwrap();
    let mut input = f.a.item_source.input().clone();
    input.item_lines = *f.a.items.identity();
    f.a.item_source = compile(&f, input).unwrap();
    let plan = attribute(&f.a, &xml(0, "128% increased Spell Damage"));
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    assert_eq!(category_values(&f, &plan), vec![f.options[0].clone()]);
    let converted = plan.convert(&f.a.items).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert_eq!(converted.modifiers[0].rolls_closure, closure);
}

#[test]
fn invalid_binding_shapes_and_owned_option_memberships_are_rejected() {
    let f = fixture();
    for invalid in 0..10 {
        let mut input = f.a.item_source.input().clone();
        match invalid {
            0 => {
                let copy = bindings(&mut input)[0].clone();
                bindings(&mut input).push(copy);
            }
            1 => {
                let copy = bindings(&mut input)[0].values[0].clone();
                bindings(&mut input)[0].values.push(copy);
            }
            2 => bindings(&mut input)[0].values[1].value = f.options[0].clone(),
            3 => bindings(&mut input)[0].values.clear(),
            4 => {
                bindings(&mut input)[0].values[0].value = OptionDefId::parse(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    "outside",
                )
                .unwrap()
            }
            5 => {
                bindings(&mut input)[0].values[0].value =
                    OptionDefId::parse(input.namespace.clone(), "unknown").unwrap()
            }
            6 => bindings(&mut input)[0].values[0].value = f.outside.clone(),
            7 => bindings(&mut input).clear(),
            8 => {
                let mut unused = bindings(&mut input)[0].clone();
                unused.input = key("unused");
                bindings(&mut input).push(unused);
            }
            9 => input.property_bindings.push(ItemSourcePropertyBinding {
                label: "different".into(),
                property: key("category"),
            }),
            _ => unreachable!(),
        }
        assert!(compile(&f, input).is_err(), "invalid case {invalid}");
    }
    let mut lines = f.a.items.input().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("spell"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("modifier")
    };
    rolls[0].value = ItemLineValue::ContextOption {
        input: key("category"),
    };
    assert!(
        OwnedItemLinePolicy::new(lines, &f.a.schema, Default::default()).is_err(),
        "a category cannot populate a quantity slot"
    );
}

fn minimum_budget(maximum: usize, mut accepts: impl FnMut(usize) -> bool) -> usize {
    assert!(accepts(maximum));
    let (mut lower, mut upper) = (1, maximum);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if accepts(middle) {
            upper = middle
        } else {
            lower = middle + 1
        }
    }
    lower
}

#[test]
fn category_and_existing_default_validation_share_one_schema_work_allowance() {
    let f = fixture();
    let mut full = f.a.item_source.input().clone();
    full.template_defaults.push(ItemSourceTemplateDefaults {
        template: f.a.staff.clone(),
        parameters: vec![],
        item_level: ItemSourceAbsentPolicy::Absent,
        quality: ItemSourceAbsentPolicy::Pending,
    });
    let plain = f.a.item_source.input().clone();
    let minimum = |input: &ItemSourceLayoutPolicyInput| {
        minimum_budget(
            ItemSourceLimits::default().max_schema_work,
            |max_schema_work| {
                ItemSourceLayoutPolicy::new(
                    input.clone(),
                    &f.a.items,
                    &f.a.schema,
                    ItemSourceLimits {
                        max_schema_work,
                        ..Default::default()
                    },
                )
                .is_ok()
            },
        )
    };
    let plain_work = minimum(&plain);
    let full_work = minimum(&full);
    assert!(full_work > plain_work);
    let compiled = compile(&f, full.clone()).unwrap();
    let bytes = encode_item_source_policy(&compiled, Default::default()).unwrap();
    for work in [plain_work, full_work - 1] {
        let limits = ItemSourceLimits {
            max_schema_work: work,
            ..Default::default()
        };
        assert!(
            ItemSourceLayoutPolicy::new(full.clone(), &f.a.items, &f.a.schema, limits).is_err()
        );
        assert!(encode_item_source_policy(&compiled, limits).is_err());
        assert!(decode_item_source_policy(&bytes, &f.a.items, &f.a.schema, limits).is_err());
    }
    let limits = ItemSourceLimits {
        max_schema_work: full_work,
        ..Default::default()
    };
    assert_eq!(encode_item_source_policy(&compiled, limits).unwrap(), bytes);
    assert_eq!(
        decode_item_source_policy(&bytes, &f.a.items, &f.a.schema, limits)
            .unwrap()
            .identity(),
        compiled.identity()
    );
}

#[test]
fn lower_text_wire_and_runtime_caps_apply_to_compiled_category_policies() {
    let f = fixture();
    let input = f.a.item_source.input().clone();
    let bytes = encode_item_source_policy(&f.a.item_source, Default::default()).unwrap();
    let text = minimum_budget(
        ItemSourceLimits::default().max_policy_text_bytes,
        |max_policy_text_bytes| {
            ItemSourceLayoutPolicy::new(
                input.clone(),
                &f.a.items,
                &f.a.schema,
                ItemSourceLimits {
                    max_policy_text_bytes,
                    ..Default::default()
                },
            )
            .is_ok()
        },
    );
    for limits in [
        ItemSourceLimits {
            max_policy_text_bytes: text - 1,
            ..Default::default()
        },
        ItemSourceLimits {
            max_wire_bytes: bytes.len() - 1,
            ..Default::default()
        },
    ] {
        assert!(encode_item_source_policy(&f.a.item_source, limits).is_err());
        assert!(decode_item_source_policy(&bytes, &f.a.items, &f.a.schema, limits).is_err());
    }
    let body = xml(
        1,
        "128% increased Spell Damage\n129% increased Spell Damage",
    );
    let imported = source(&body);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let accepts = |limits| {
        let p =
            ItemSourceLayoutPolicy::new(input.clone(), &f.a.items, &f.a.schema, limits).unwrap();
        p.attribute(&evidence, item_source(&imported, "7"), &f.a.items)
            .and_then(|v| v.convert(&f.a.items).map(|_| ()))
            .is_ok()
    };
    let needed = minimum_budget(ItemSourceLimits::default().max_work, |max_work| {
        accepts(ItemSourceLimits {
            max_work,
            ..Default::default()
        })
    });
    assert!(!accepts(ItemSourceLimits {
        max_work: needed - 1,
        ..Default::default()
    }));
    let outputs = minimum_budget(
        ItemSourceLimits::default().max_output_records,
        |max_output_records| {
            accepts(ItemSourceLimits {
                max_output_records,
                ..Default::default()
            })
        },
    );
    assert!(!accepts(ItemSourceLimits {
        max_output_records: outputs - 1,
        ..Default::default()
    }));
}

#[test]
fn v8_is_explicitly_versioned_and_legacy_v7_bytes_do_not_acquire_options() {
    let f = fixture();
    let bytes = encode_item_source_policy(&f.a.item_source, Default::default()).unwrap();
    assert_eq!(
        *f.a.item_source.identity(),
        digest_owned(
            "owned-item-source-policy-v8",
            f.a.item_source.input(),
            ItemSourceLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    let decoded =
        decode_item_source_policy(&bytes, &f.a.items, &f.a.schema, Default::default()).unwrap();
    assert_eq!(
        encode_item_source_policy(&decoded, Default::default()).unwrap(),
        bytes
    );
    let mut input = f.a.item_source.input().clone();
    input.schema_version = OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION;
    assert!(
        compile(&f, input.clone()).is_err(),
        "V8 dialect cannot claim V7"
    );
    input.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings: vec![],
        metadata_rules: vec![],
        single_modifier_conditions: vec![],
        preamble_observations: vec![],
    };
    assert!(
        compile(&f, input).is_err(),
        "V7 cannot bind a newly required option context"
    );

    let mut old = artifacts();
    let mut input = old.item_source.input().clone();
    input.schema_version = OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION;
    input.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings: vec![],
        metadata_rules: vec![],
        single_modifier_conditions: vec![],
        preamble_observations: vec![],
    };
    let prior_bytes = serde_json::to_vec(&input).unwrap();
    let expected = digest_owned(
        "owned-item-source-policy-v7",
        &input,
        ItemSourceLimits::default().max_wire_bytes,
    )
    .unwrap();
    old.item_source =
        ItemSourceLayoutPolicy::new(input, &old.items, &old.schema, Default::default()).unwrap();
    assert_eq!(*old.item_source.identity(), expected);
    assert_eq!(
        encode_item_source_policy(&old.item_source, Default::default()).unwrap(),
        prior_bytes
    );
    let loaded =
        decode_item_source_policy(&prior_bytes, &old.items, &old.schema, Default::default())
            .unwrap();
    assert_eq!(loaded.identity(), &expected);
    let plan = attribute(&old, &xml(1, "128% increased Spell Damage"));
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    let result = plan.convert(&old.items).unwrap();
    assert_eq!(result.modifiers.len(), 1);
    assert!(
        result.modifiers[0]
            .rolls
            .iter()
            .all(|r| !matches!(r.value, ParameterValue::Option(_)))
    );
    let mut unexpected: serde_json::Value = serde_json::from_slice(&prior_bytes).unwrap();
    unexpected["dialect"]["pob_exported_single_text_observations_v1"]["category_bindings"] =
        serde_json::json!([]);
    assert!(
        decode_item_source_policy(
            &serde_json::to_vec(&unexpected).unwrap(),
            &old.items,
            &old.schema,
            Default::default()
        )
        .is_err()
    );
}
