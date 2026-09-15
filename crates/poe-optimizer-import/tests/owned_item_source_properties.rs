//! Source labels are adapter evidence; only declared Boolean modifier inputs escape.
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{
    owned_item_lines::*, owned_item_source::*, owned_source::*, owned_value::DecimalSyntax,
};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

fn fixture() -> Artifacts {
    let mut a = artifacts();
    let modifier = a.modifiers["spell"].definition.clone();
    let mut schema = a.schema.input().clone();
    let mut items = a.items.input().clone();
    let mut source_policy = a.item_source.input().clone();
    let rule = items
        .rules
        .iter_mut()
        .find(|r| r.id == key("spell"))
        .unwrap();
    rule.pattern[0] = ItemPatternPart::NumericCapture {
        capture: rule.captures[0].id.clone(),
        syntax: DecimalSyntax::Decimal,
        sign: ItemNumericSign::OptionalMinus,
    };
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("modifier")
    };
    for property in ["elemental", "cold"] {
        let slot = a
            .registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
            .unwrap();
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
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
        rolls.push(ItemRollTemplate {
            slot,
            value: ItemLineValue::Property {
                property: key(property),
            },
        });
        source_policy
            .property_bindings
            .push(ItemSourcePropertyBinding {
                label: property.into(),
                property: key(property),
            });
    }
    // A source alias is explicit; it does not alter the owned property identity.
    source_policy
        .property_bindings
        .push(ItemSourcePropertyBinding {
            label: "frost".into(),
            property: key("cold"),
        });
    let mut ranged = rule.clone();
    ranged.id = key("ranged-spell");
    let mut lower = ranged.captures[0].clone();
    lower.id = key("lower");
    let mut upper = lower.clone();
    upper.id = key("upper");
    ranged.captures = vec![lower, upper];
    ranged.pattern = vec![
        ItemPatternPart::Literal("(".into()),
        ItemPatternPart::NumericCapture {
            capture: key("lower"),
            syntax: DecimalSyntax::Integer,
            sign: ItemNumericSign::OptionalMinus,
        },
        ItemPatternPart::Literal("-".into()),
        ItemPatternPart::NumericCapture {
            capture: key("upper"),
            syntax: DecimalSyntax::Integer,
            sign: ItemNumericSign::OptionalMinus,
        },
        ItemPatternPart::Literal(")% increased Spell Damage".into()),
    ];
    let ItemEmission::Modifier { rolls, .. } = &mut ranged.emissions[0] else {
        panic!("modifier")
    };
    rolls[0].value = ItemLineValue::InterpolateOffset {
        lower: key("lower"),
        upper: key("upper"),
        quantum: ParameterValue::Quantity(FiniteQuantity::new(1.0, a.percent.clone()).unwrap()),
        rounding: ItemRangeRounding::SymmetricHalfOffset,
    };
    source_policy.rule_layouts.push(ItemRuleSourceLayout {
        rule: ranged.id.clone(),
        role: ItemRuleSourceRole::SingleModifier,
    });
    items.rules.push(ranged);
    a.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    items.definitions = a.schema.identity().clone();
    a.items = OwnedItemLinePolicy::new(items, &a.schema, ItemLineLimits::default()).unwrap();
    source_policy.item_lines = *a.items.identity();
    a.item_source = ItemSourceLayoutPolicy::new(
        source_policy,
        &a.items,
        &a.schema,
        ItemSourceLimits::default(),
    )
    .unwrap();
    a
}
fn xml(body: &str, overlays: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nPrefix: {{tags:not-a-modifier}}ignored\nQuality: 20\nRune: None\nImplicits: 0\n{body}{overlays}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(
    a: &Artifacts,
    text: &str,
) -> std::result::Result<ItemRangeAttribution, ItemSourceError> {
    let imported = source(text);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
}
fn assert_modifier(
    plan: &ItemRangeAttribution,
    a: &Artifacts,
    amount: f64,
    elemental: bool,
    cold: bool,
) {
    assert!(
        matches!(plan.report().layout, ItemLayoutStatus::Proven),
        "{:?}",
        plan.report()
    );
    let c = plan.convert(&a.items).unwrap();
    assert!(c.issues.is_empty(), "{:?}", c.issues);
    assert_eq!(c.modifiers.len(), 1);
    let rolls = &c.modifiers[0].rolls;
    assert_eq!(rolls.len(), 3);
    assert!(matches!(&rolls[0].value, ParameterValue::Quantity(v) if v.value() == amount));
    assert_eq!(rolls[1].value, ParameterValue::Boolean(elemental));
    assert_eq!(rolls[2].value, ParameterValue::Boolean(cold));
}

#[test]
fn exact_labels_explicit_aliases_and_known_empty_membership_become_typed_inputs() {
    let a = fixture();
    for (tags, elemental, cold) in [
        ("", false, false),
        ("{tags:}", false, false),
        ("{tags}", false, false),
        ("{tags:cold}", false, true),
        ("{tags:frost}", false, true),
        ("{tags: cold-123elemental /}", true, true),
        ("{tags:cold,,}{tags:elemental,cold,}", true, true),
    ] {
        let plan = attribute(&a, &xml(&format!("{tags}128% increased Spell Damage"), "")).unwrap();
        assert_modifier(&plan, &a, 128.0, elemental, cold);
        let line = plan.report().lines.last().unwrap();
        assert_eq!(line.properties.len(), 2);
        for token in &line.property_tokens {
            let range = token.decoded_span.start - line.decoded_span.start
                ..token.decoded_span.end - line.decoded_span.start;
            assert_eq!(&line.raw[range], token.label);
            assert!(token.property.is_some());
        }
        assert!(
            plan.report().lines[3].property_tokens.is_empty(),
            "opaque header tags are not modifier inputs"
        );
    }
}

#[test]
fn unknown_or_unconsumed_properties_never_disappear_into_an_existing_modifier_rule() {
    let a = fixture();
    for (body, problem, candidate) in [
        (
            "{tags:cold,mystery}128% increased Spell Damage",
            ItemSourceProblem::UnknownProperty,
            "spell",
        ),
        (
            "{tags:Cold}128% increased Spell Damage",
            ItemSourceProblem::UnknownProperty,
            "spell",
        ),
        (
            "{tags:cold}49% increased Attack Speed",
            ItemSourceProblem::UnconsumedProperty,
            "speed",
        ),
        (
            "{tags:cold}{unscalable}128% increased Spell Damage",
            ItemSourceProblem::UnsupportedTag,
            "spell",
        ),
    ] {
        let plan = attribute(&a, &xml(body, "")).unwrap();
        let line = plan.report().lines.last().unwrap();
        assert!(line.blockers.contains(&problem), "{line:?}");
        assert!(line.properties.is_empty());
        assert!(line.pending_candidates.contains(&key(candidate)));
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
    }
}

#[test]
fn property_reads_do_not_break_range_membership_and_unknown_labels_withhold_overlays() {
    let a = fixture();
    let body = "{tags:elemental,cold}{range:0}(10-30)% increased Spell Damage";
    let plan = attribute(&a, &xml(body, "<ModRange id=\"1\" range=\"0.5\"/>")).unwrap();
    assert_modifier(&plan, &a, 20.0, true, true);
    assert_eq!(plan.report().writes.len(), 2);
    assert!(matches!(
        plan.report().writes[1].target,
        ItemRangeTarget::Line(_)
    ));
    let bad = attribute(
        &a,
        &xml(
            &body.replace("elemental,cold", "elemental,mystery"),
            "<ModRange id=\"1\" range=\"1\"/>",
        ),
    )
    .unwrap();
    assert!(bad.convert(&a.items).unwrap().modifiers.is_empty());
    assert!(matches!(
        bad.report().writes[1].target,
        ItemRangeTarget::Pending
    ));
    assert!(matches!(
        bad.report().lines.last().unwrap().range,
        ItemRangeDecision::Pending
    ));
}

#[test]
fn property_bindings_are_explicit_versioned_and_bounded_before_projection() {
    let a = fixture();
    let base = a.item_source.input().clone();
    for mutate in 0..4 {
        let mut changed = base.clone();
        match mutate {
            0 => {
                changed.property_bindings.pop();
                changed
                    .property_bindings
                    .retain(|b| b.property != key("cold"));
            }
            1 => changed
                .property_bindings
                .push(changed.property_bindings[0].clone()),
            2 => changed.property_bindings.push(ItemSourcePropertyBinding {
                label: "unused".into(),
                property: key("unused"),
            }),
            _ => changed.schema_version = 1,
        }
        assert!(
            ItemSourceLayoutPolicy::new(changed, &a.items, &a.schema, ItemSourceLimits::default())
                .is_err()
        );
    }
    let mut missing = serde_json::to_value(&base).unwrap();
    missing.as_object_mut().unwrap().remove("property_bindings");
    assert!(
        decode_item_source_policy(
            &serde_json::to_vec(&missing).unwrap(),
            &a.items,
            &a.schema,
            ItemSourceLimits::default()
        )
        .is_err()
    );
    assert!(
        ItemSourceLayoutPolicy::new(
            base.clone(),
            &a.items,
            &a.schema,
            ItemSourceLimits {
                max_properties: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        encode_item_source_policy(
            &a.item_source,
            ItemSourceLimits {
                max_properties: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut limited = fixture();
    limited.item_source = ItemSourceLayoutPolicy::new(
        base,
        &a.items,
        &a.schema,
        ItemSourceLimits {
            max_tags: 2,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        attribute(
            &limited,
            &xml("{tags:cold,elemental}128% increased Spell Damage", "")
        ),
        Err(ItemSourceError::Limit("tags"))
    ));
}

#[test]
fn source_labels_use_the_reviewed_token_grammar_and_text_limits_roundtrip() {
    let a = fixture();
    for label in ["cold-fire", "cold2", " cold", "cold:fire", "cold/fire", ""] {
        let mut changed = a.item_source.input().clone();
        changed.property_bindings.push(ItemSourcePropertyBinding {
            label: label.into(),
            property: key("cold"),
        });
        assert!(
            ItemSourceLayoutPolicy::new(changed, &a.items, &a.schema, ItemSourceLimits::default())
                .is_err(),
            "{label}"
        );
    }
    // Both fixed and ranged programs reference the same properties. Encoding
    // must retain the constructor's full projection-text budget, not just the
    // binding strings present in the source-policy JSON.
    let input = a.item_source.input().clone();
    let mut lower = 1;
    let mut upper = ItemSourceLimits::default().max_policy_text_bytes;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let limits = ItemSourceLimits {
            max_policy_text_bytes: middle,
            ..Default::default()
        };
        if ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_ok() {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    let exact = ItemSourceLimits {
        max_policy_text_bytes: lower,
        ..Default::default()
    };
    let too_small = ItemSourceLimits {
        max_policy_text_bytes: lower - 1,
        ..Default::default()
    };
    let bytes = encode_item_source_policy(&a.item_source, exact).unwrap();
    let decoded = decode_item_source_policy(&bytes, &a.items, &a.schema, exact).unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    assert!(matches!(
        encode_item_source_policy(&a.item_source, too_small),
        Err(ItemSourceError::Limit("policy text"))
    ));
    assert!(matches!(
        decode_item_source_policy(&bytes, &a.items, &a.schema, too_small),
        Err(ItemSourceError::Limit("policy text"))
    ));
}

#[test]
fn known_and_unknown_long_prefix_label_lookups_consume_the_shared_work_budget() {
    let mut a = fixture();
    let prefix = "x".repeat(400);
    let known = format!("{prefix}aa");
    let unknown = format!("{prefix}zz");
    let mut base = a.item_source.input().clone();
    base.property_bindings.push(ItemSourcePropertyBinding {
        label: known.clone(),
        property: key("cold"),
    });
    let limited = ItemSourceLimits {
        max_work: 100_000,
        ..Default::default()
    };
    a.item_source =
        ItemSourceLayoutPolicy::new(base.clone(), &a.items, &a.schema, limited).unwrap();
    for label in [&known, &unknown] {
        assert!(
            attribute(
                &a,
                &xml(&format!("{{tags:{label}}}128% increased Spell Damage"), "")
            )
            .is_ok()
        );
    }
    for i in 1..520u16 {
        let label = format!(
            "{prefix}{}{}",
            char::from(b'a' + (i / 26) as u8),
            char::from(b'a' + (i % 26) as u8)
        );
        base.property_bindings.push(ItemSourcePropertyBinding {
            label,
            property: key("cold"),
        });
    }
    a.item_source = ItemSourceLayoutPolicy::new(
        base.clone(),
        &a.items,
        &a.schema,
        ItemSourceLimits::default(),
    )
    .unwrap();
    let body = xml(&format!("{{tags:{known}}}128% increased Spell Damage"), "");
    assert_modifier(&attribute(&a, &body).unwrap(), &a, 128.0, false, true);
    a.item_source = ItemSourceLayoutPolicy::new(base, &a.items, &a.schema, limited).unwrap();
    for label in [&known, &unknown] {
        assert!(matches!(
            attribute(
                &a,
                &xml(&format!("{{tags:{label}}}128% increased Spell Damage"), "")
            ),
            Err(ItemSourceError::Limit("work"))
        ));
    }
}
