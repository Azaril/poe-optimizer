//! Import numeric operation laws. Shared fixed vectors are also checked against
//! executed pinned Common/ItemTools by Engine's optional item range source oracle.
#[path = "support/item_range_vectors.rs"]
mod vectors;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{owned_item_lines::*, owned_value::*};
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("range-test", "v1").unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn quantity(v: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, id("points")).unwrap())
}
fn integer(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn slot() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Modifier(id("modifier")),
        slot: id("roll"),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn policy(
    rounding: ItemRangeRounding,
    quantum: ParameterValue,
    offset: bool,
) -> OwnedItemLinePolicy {
    let is_integer = matches!(&quantum, ParameterValue::Integer(_));
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("test"),
            semantics_version: key("test"),
            definitions: vec![
                DefinitionDescriptor::Unit(known(
                    id("points"),
                    UnitSchema {
                        dimension: UnitDimension::PercentagePoints,
                    },
                )),
                DefinitionDescriptor::Modifier(known(
                    id("modifier"),
                    ModifierSchema {
                        declarations: DeclaredSlots {
                            parameters: DeclaredSet::complete(vec![slot()]),
                            choices: DeclaredSet::complete(vec![]),
                            grants: DeclaredSet::complete(vec![]),
                            actors: DeclaredSet::complete(vec![]),
                            skill_grants: DeclaredSet::complete(vec![]),
                            outputs: DeclaredSet::complete(vec![]),
                            sockets: DeclaredSet::complete(vec![]),
                        },
                    },
                )),
            ],
            slots: vec![SlotDescriptor::Parameter(known(
                slot(),
                ParameterSlotSchema {
                    value: if is_integer {
                        ValueSchema::Integer(IntegerRange {
                            minimum: BoundedInteger::new(-9_007_199_254_740_991).unwrap(),
                            maximum: BoundedInteger::new(9_007_199_254_740_991).unwrap(),
                        })
                    } else {
                        ValueSchema::Quantity(QuantityRange {
                            minimum: FiniteQuantity::new(-f64::MAX, id("points")).unwrap(),
                            maximum: FiniteQuantity::new(f64::MAX, id("points")).unwrap(),
                        })
                    },
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                },
            ))],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let codec = ItemCaptureCodec::Value(ValueCodecInput {
        namespace: ns(),
        whitespace: WhitespacePolicy::Exact,
        codec: if is_integer {
            ValueCodecKind::Integer {
                syntax: DecimalSyntax::Scientific,
            }
        } else {
            ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit: id("points"),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            }
        },
    });
    let value = if offset {
        ItemLineValue::InterpolateOffset {
            lower: key("lo"),
            upper: key("hi"),
            quantum,
            rounding,
        }
    } else {
        ItemLineValue::Interpolate {
            lower: key("lo"),
            upper: key("hi"),
            quantum,
            rounding,
        }
    };
    OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: ns(),
            version: key("test"),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules: vec![ItemLineRule {
                id: key("range"),
                pattern: vec![
                    ItemPatternPart::Literal("Range(".into()),
                    ItemPatternPart::Capture(key("lo")),
                    ItemPatternPart::Literal(",".into()),
                    ItemPatternPart::Capture(key("hi")),
                    ItemPatternPart::Literal(")".into()),
                ],
                captures: vec![
                    ItemCapture {
                        id: key("lo"),
                        codec: codec.clone(),
                    },
                    ItemCapture {
                        id: key("hi"),
                        codec,
                    },
                ],
                emissions: vec![ItemEmission::Modifier {
                    definition: id("modifier"),
                    rolls: vec![ItemRollTemplate {
                        slot: slot(),
                        value,
                    }],
                }],
            }],
        },
        &schema,
        ItemLineLimits::default(),
    )
    .unwrap()
}
fn number(p: &OwnedItemLinePolicy, a: f64, b: f64, fraction: f64) -> f64 {
    let text = format!("Range({a:?},{b:?})");
    let line = p.convert_line(7, &text, Some(fraction)).unwrap();
    assert_eq!(line.index, 7);
    assert_eq!(line.text, text);
    let emissions = match line.outcome {
        ItemLineOutcome::Known { emissions, .. } => emissions,
        other => panic!("unexpected pending: {other:?}"),
    };
    let [ConvertedItemEmission::Modifier { rolls, .. }] = emissions.as_slice() else {
        panic!("one modifier required")
    };
    assert_eq!(rolls.len(), 1);
    match &rolls[0].value {
        ParameterValue::Quantity(q) => {
            assert_eq!(q.unit(), &id("points"));
            q.value()
        }
        ParameterValue::Integer(v) => v.get() as f64,
        _ => panic!("numeric result required"),
    }
}
fn bits(actual: f64, expected: f64) {
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "{actual:?} != {expected:?}"
    );
}
fn pending(p: &OwnedItemLinePolicy, text: &str, fraction: Option<f64>, expected: ItemLinePending) {
    let line = p.convert_line(9, text, fraction).unwrap();
    assert!(matches!(line.outcome, ItemLineOutcome::Pending { reason, .. } if reason == expected));
}
#[test]
fn signed_half_offset_preserves_literal_precision_effects_and_core_zero_normalization() {
    let p = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(1.0), false);
    for &(raw, expected) in vectors::SYMMETRIC_HALF_OFFSET {
        // FiniteQuantity deliberately canonicalizes both zero signs at the owned
        // value boundary. The executed source test separately checks raw signs.
        bits(
            number(&p, raw, raw, 0.0),
            if expected == 0.0 { 0.0 } else { expected },
        );
    }
    let prior = policy(ItemRangeRounding::NearestTiesPositive, quantity(1.0), false);
    let below_half = f64::from_bits(0x3fdfffffffffffff);
    bits(number(&p, below_half, below_half, 0.0), 1.0);
    bits(number(&prior, below_half, below_half, 0.0), 0.0);
    bits(
        number(
            &prior,
            4_503_599_627_370_497.0,
            4_503_599_627_370_497.0,
            0.0,
        ),
        4_503_599_627_370_497.0,
    );
}
#[test]
fn signed_integer_ties_and_fraction_endpoints_resolve_without_unsigned_clamps() {
    let p = policy(ItemRangeRounding::SymmetricHalfOffset, integer(1), true);
    for (a, b, f, value) in [
        (-4.0, -3.0, 0.5, -4.0),
        (3.0, 4.0, 0.5, 4.0),
        (-1.0, 0.0, 0.5, -1.0),
        (0.0, 1.0, 0.5, 1.0),
        (-3.0, 4.0, 0.0, -3.0),
        (-3.0, 4.0, 1.0, 4.0),
        (-3.0, 4.0, 0.5, 1.0),
        (-4.0, 3.0, 0.5, -1.0),
    ] {
        bits(number(&p, a, b, f), value);
    }
    let q = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(0.5), true);
    for (raw, expected) in [(-1.25, -1.5), (-0.25, -0.5), (0.25, 0.5), (1.25, 1.5)] {
        bits(number(&q, raw, raw, 0.0), expected);
    }
}
#[test]
fn offset_arithmetic_is_explicit_and_preserves_crossing_zero_source_order() {
    let source = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(1.0), true);
    let stable = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(1.0), false);
    bits(number(&source, -3.0, 2.0, 0.7), 1.0);
    bits(number(&stable, -3.0, 2.0, 0.7), 0.0);
    let before = source.input().clone();
    let encoded = serde_json::to_vec(&before).unwrap();
    let decoded: ItemLinePolicyInput = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(before, decoded);
    let text = String::from_utf8(encoded).unwrap();
    assert!(text.contains("interpolate_offset"));
    assert!(text.contains("symmetric_half_offset"));
}
#[test]
fn nonfinite_intermediates_and_out_of_domain_integer_results_remain_pending() {
    let source = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(1.0), true);
    let stable = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(1.0), false);
    let extreme = format!("Range({:?},{:?})", -f64::MAX, f64::MAX);
    for fraction in [0.0, 0.5, 1.0] {
        pending(
            &source,
            &extreme,
            Some(fraction),
            ItemLinePending::InvalidRange,
        );
    }
    bits(number(&stable, -f64::MAX, f64::MAX, 0.5), 0.0);
    let scaled_overflow = policy(ItemRangeRounding::SymmetricHalfOffset, quantity(0.5), false);
    let maximum = format!("Range({0:?},{0:?})", f64::MAX);
    pending(
        &scaled_overflow,
        &maximum,
        Some(0.0),
        ItemLinePending::InvalidRange,
    );
    let final_overflow = policy(
        ItemRangeRounding::SymmetricHalfOffset,
        quantity(1e308),
        false,
    );
    pending(
        &final_overflow,
        "Range(1.5e308,1.5e308)",
        Some(0.0),
        ItemLinePending::InvalidRange,
    );
    let integer = policy(ItemRangeRounding::SymmetricHalfOffset, integer(1), true);
    for text in [
        "Range(9007199254740991,9007199254740991)",
        "Range(-9007199254740991,-9007199254740991)",
    ] {
        pending(&integer, text, Some(0.0), ItemLinePending::InvalidRange);
    }
}
#[test]
fn other_modes_remain_distinct_and_missing_or_malformed_fractions_never_fall_back() {
    for (mode, negative, positive) in [
        (ItemRangeRounding::Floor, -2.0, 1.0),
        (ItemRangeRounding::Ceiling, -1.0, 2.0),
        (ItemRangeRounding::Truncate, -1.0, 1.0),
        (ItemRangeRounding::NearestTiesPositive, -1.0, 2.0),
        (ItemRangeRounding::SymmetricHalfOffset, -2.0, 2.0),
    ] {
        let p = policy(mode, quantity(1.0), true);
        bits(number(&p, -2.0, -1.0, 0.5), negative);
        bits(number(&p, 1.0, 2.0, 0.5), positive);
        pending(
            &p,
            "Range(1,2)",
            None,
            ItemLinePending::MissingRangeFraction,
        );
        for fraction in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            pending(
                &p,
                "Range(1,2)",
                Some(fraction),
                ItemLinePending::InvalidRangeFraction,
            );
        }
        pending(&p, "Range(2,1)", Some(0.5), ItemLinePending::InvalidRange);
    }
}
