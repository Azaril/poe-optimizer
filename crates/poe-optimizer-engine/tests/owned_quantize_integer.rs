//! Synthetic arithmetic component coverage; facts do not establish incoming
//! contribution completeness or full original-build attribute parity.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_content::digest_owned, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, RuleStorageLimits, decode_rule_package, encode_rule_package},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_engine::owned_rules::*;
#[allow(dead_code)]
#[path = "support/owned_rule_fixture.rs"]
mod owned_rule_fixture;
use owned_rule_fixture::{Fixture, fixture};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn id<K: DefinitionDomain>(f: &Fixture, s: &str) -> DefId<K> {
    DefId::parse(f.rules.namespace.clone(), s).unwrap()
}
fn integer(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn quantity(v: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, unit.clone()).unwrap())
}
fn node(s: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(s),
        expression,
    }
}
fn literal(s: &str, value: ParameterValue) -> RuleNode {
    node(s, RuleExpression::Literal { value })
}
fn program(f: &mut Fixture) -> &mut RuleProgram {
    &mut f.rules.owners[0].programs.members[0]
}
fn compile(f: &Fixture) -> CompiledRulePackage {
    CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap()
}
fn bad(f: &Fixture, expected: &str) {
    let err = CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap_err();
    assert!(err.to_string().contains(expected), "{err}");
}
fn quantizer(quantum: f64, mode: RuleRounding) -> Fixture {
    let mut f = fixture();
    let unit = id(&f, "unit.damage");
    let owner = f.rules.owners[0].owner.clone();
    f.rules.operations_version = key(OWNED_RULE_OPERATIONS_V8);
    f.rules.owners = vec![DefinitionRules {
        owner,
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("quantize"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("input"),
                value_type: ComputedValueType::Quantity { unit: unit.clone() },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: id(&f, "stat.damage"),
                },
            }],
            nodes: vec![
                node(
                    "input-value",
                    RuleExpression::Read {
                        input: key("input"),
                    },
                ),
                node(
                    "result",
                    RuleExpression::QuantizeInteger {
                        value: key("input-value"),
                        quantum: FiniteQuantity::new(quantum, unit).unwrap(),
                        mode,
                    },
                ),
            ],
            effects: vec![RuleEffect {
                id: key("result"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: id(&f, "stat.effective-attribute"),
                    value: key("result"),
                },
            }],
        }]),
    }];
    f
}
fn evaluate(
    f: &Fixture,
    compiled: &CompiledRulePackage,
    scratch: &mut RuleScratch,
    input: Option<f64>,
) -> EffectDisposition {
    let facts = input
        .map(|v| RuleFact {
            read: key("input"),
            value: quantity(v, &id(f, "unit.damage")),
        })
        .into_iter()
        .collect::<Vec<_>>();
    compiled
        .evaluate(
            &f.rules.owners[0].owner,
            &key("quantize"),
            &facts,
            &f.schema,
            scratch,
        )
        .unwrap()
        .effects
        .remove(0)
        .disposition
}
fn applied(value: i64) -> EffectDisposition {
    EffectDisposition::Applied {
        value: integer(value),
    }
}

#[test]
fn all_rounding_modes_cover_signed_ties_adjacent_values_and_exact_integer_endpoints() {
    let below_half = f64::from_bits(0.5_f64.to_bits() - 1);
    let above_half = f64::from_bits(0.5_f64.to_bits() + 1);
    let cases = [
        (2.5, [2, 3, 2, 3]),
        (-2.5, [-3, -2, -2, -2]),
        (0.5, [0, 1, 0, 1]),
        (-0.5, [-1, 0, 0, 0]),
        (below_half, [0, 1, 0, 0]),
        (above_half, [0, 1, 0, 1]),
        (-below_half, [-1, 0, 0, 0]),
        (-above_half, [-1, 0, 0, -1]),
        (0.0, [0; 4]),
        (-0.0, [0; 4]),
        (BoundedInteger::MAX as f64, [BoundedInteger::MAX; 4]),
        (BoundedInteger::MIN as f64, [BoundedInteger::MIN; 4]),
        (
            (BoundedInteger::MAX - 2) as f64,
            [BoundedInteger::MAX - 2; 4],
        ),
    ];
    for (mode_index, mode) in [
        RuleRounding::Floor,
        RuleRounding::Ceiling,
        RuleRounding::Truncate,
        RuleRounding::NearestTiesPositive,
    ]
    .into_iter()
    .enumerate()
    {
        let f = quantizer(1.0, mode);
        let c = compile(&f);
        let mut scratch = c.new_scratch();
        for (value, expected) in cases {
            assert_eq!(
                evaluate(&f, &c, &mut scratch, Some(value)),
                applied(expected[mode_index]),
                "{value:?} {mode:?}"
            );
        }
    }
    let f = quantizer(0.25, RuleRounding::NearestTiesPositive);
    let c = compile(&f);
    assert_eq!(
        evaluate(&f, &c, &mut c.new_scratch(), Some(1.375)),
        applied(6)
    );
    assert_eq!(
        evaluate(&f, &c, &mut c.new_scratch(), Some(-1.375)),
        applied(-5)
    );
}

#[test]
fn directional_underflow_is_preserved_and_nonfinite_or_out_of_range_counts_fail() {
    for (value, mode, expected) in [
        (-f64::MIN_POSITIVE, RuleRounding::Floor, -1),
        (f64::MIN_POSITIVE, RuleRounding::Ceiling, 1),
        (-f64::MIN_POSITIVE, RuleRounding::Truncate, 0),
        (f64::MIN_POSITIVE, RuleRounding::NearestTiesPositive, 0),
    ] {
        let f = quantizer(f64::MAX, mode);
        let c = compile(&f);
        assert_eq!(
            evaluate(&f, &c, &mut c.new_scratch(), Some(value)),
            applied(expected)
        );
    }
    for (quantum, value, reason) in [
        (
            1.0,
            BoundedInteger::MAX as f64 + 1.0,
            NumericalFailure::IntegerOverflow,
        ),
        (
            1.0,
            BoundedInteger::MIN as f64 - 1.0,
            NumericalFailure::IntegerOverflow,
        ),
        (1.0, f64::MAX, NumericalFailure::IntegerOverflow),
        (1.0, -f64::MAX, NumericalFailure::IntegerOverflow),
        (f64::MIN_POSITIVE, f64::MAX, NumericalFailure::NonFinite),
        (f64::MIN_POSITIVE, -f64::MAX, NumericalFailure::NonFinite),
    ] {
        let f = quantizer(quantum, RuleRounding::NearestTiesPositive);
        let c = compile(&f);
        let mut scratch = c.new_scratch();
        assert_eq!(
            evaluate(&f, &c, &mut scratch, Some(value)),
            EffectDisposition::NumericalError {
                node: key("result"),
                reason
            }
        );
        assert_eq!(evaluate(&f, &c, &mut scratch, Some(0.0)), applied(0));
        assert_eq!(
            evaluate(&f, &c, &mut scratch, None),
            EffectDisposition::Unresolved {
                input: key("input")
            }
        );
    }
}

#[test]
fn compiler_rejects_wrong_value_types_quantum_units_and_unusable_definitions() {
    for quantum in [0.0, -0.0, -1.0] {
        bad(
            &quantizer(quantum, RuleRounding::Floor),
            "integer quantum must be positive",
        );
    }
    for value in [integer(1), ParameterValue::Boolean(true)] {
        let mut f = quantizer(1.0, RuleRounding::Floor);
        program(&mut f).nodes[0].expression = RuleExpression::Literal { value };
        bad(
            &f,
            "integer quantum must be positive and share the exact value unit",
        );
    }
    for unit_case in [
        "known-other",
        "missing",
        "foreign",
        "unmapped",
        "same-dimension",
    ] {
        let mut f = quantizer(1.0, RuleRounding::Floor);
        let unit: UnitDefId;
        match unit_case {
            "known-other" => unit = id(&f, "unit.percent"),
            "missing" => unit = id(&f, "unit.absent"),
            "foreign" => {
                unit = DefId::parse(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    "unit.damage",
                )
                .unwrap()
            }
            "unmapped" => {
                unit = id(&f, "unit.unmapped");
                let mut input = f.schema.input().clone();
                input
                    .definitions
                    .push(DefinitionDescriptor::Unit(DefinitionEntry {
                        id: unit.clone(),
                        schema: SchemaState::Unmapped {
                            gaps: vec![SchemaGap {
                                subject: SchemaSubject::Definition(unit.address()),
                                facet: SchemaFacet::InputSchema,
                                code: key("unmapped-test"),
                            }],
                        },
                    }));
                f.schema =
                    OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
                f.rules.definitions = f.schema.identity().clone();
            }
            "same-dimension" => {
                unit = id(&f, "unit.other-damage");
                let mut input = f.schema.input().clone();
                input
                    .definitions
                    .push(DefinitionDescriptor::Unit(DefinitionEntry {
                        id: unit.clone(),
                        schema: SchemaState::Known(UnitSchema {
                            dimension: UnitDimension::Damage,
                        }),
                    }));
                f.schema =
                    OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
                f.rules.definitions = f.schema.identity().clone();
            }
            _ => unreachable!(),
        }
        if let RuleExpression::QuantizeInteger { quantum, .. } =
            &mut program(&mut f).nodes[1].expression
        {
            *quantum = FiniteQuantity::new(1.0, unit).unwrap();
        }
        bad(
            &f,
            if matches!(unit_case, "known-other" | "same-dimension") {
                "exact value unit"
            } else {
                "missing, unmapped, foreign or inconsistent"
            },
        );
    }
    let f = quantizer(1.0, RuleRounding::Floor);
    let wire =
        serde_json::to_value(&f.rules.owners[0].programs.members[0].nodes[1].expression).unwrap();
    for (field, value) in [
        ("mode", serde_json::json!("unknown")),
        ("extra", serde_json::json!(1)),
    ] {
        let mut bad = wire.clone();
        bad[field] = value;
        assert!(serde_json::from_value::<RuleExpression>(bad).is_err());
    }
    let mut bad = wire;
    bad.as_object_mut().unwrap().remove("quantum");
    assert!(serde_json::from_value::<RuleExpression>(bad).is_err());
}

#[test]
fn lazy_selection_skips_unavailable_or_overflowing_quantization_without_masking_demanded_failures()
{
    let mut f = quantizer(1.0, RuleRounding::Floor);
    program(&mut f).nodes.extend([
        literal("disabled", ParameterValue::Boolean(false)),
        literal("fallback", integer(9)),
        node(
            "selected",
            RuleExpression::Select {
                condition: key("disabled"),
                when_true: key("result"),
                when_false: key("fallback"),
            },
        ),
    ]);
    if let RuleEffectKind::Derive { value, .. } = &mut program(&mut f).effects[0].effect {
        *value = key("selected");
    }
    let c = compile(&f);
    let mut scratch = c.new_scratch();
    assert_eq!(evaluate(&f, &c, &mut scratch, None), applied(9));
    assert_eq!(evaluate(&f, &c, &mut scratch, Some(f64::MAX)), applied(9));
    program(&mut f).nodes[2] = literal("disabled", ParameterValue::Boolean(true));
    let c = compile(&f);
    assert_eq!(
        evaluate(&f, &c, &mut scratch, None),
        EffectDisposition::Unresolved {
            input: key("input")
        }
    );
    assert_eq!(
        evaluate(&f, &c, &mut scratch, Some(f64::MAX)),
        EffectDisposition::NumericalError {
            node: key("result"),
            reason: NumericalFailure::IntegerOverflow
        }
    );
    assert_eq!(evaluate(&f, &c, &mut scratch, Some(4.9)), applied(4));
}

#[test]
fn storage_round_trip_tracks_quantization_edges_and_compilation_rejects_cycles() {
    let f = quantizer(1.0, RuleRounding::Floor);
    let limits = RuleStorageLimits::default();
    let package = OwnedRulePackage::new(f.rules.clone(), &f.schema, limits).unwrap();
    let wire = encode_rule_package(&package, limits).unwrap();
    let decoded = decode_rule_package(&wire, &f.schema, limits).unwrap();
    assert_eq!(decoded.input(), package.input());
    assert_eq!(decoded.identity(), package.identity());
    assert_eq!(decoded.resources().edges, 3);
    assert!(
        decode_rule_package(
            &wire,
            &f.schema,
            RuleStorageLimits {
                max_edges: 2,
                ..limits
            }
        )
        .is_err()
    );
    for target in ["missing", "result"] {
        let mut f = quantizer(1.0, RuleRounding::Floor);
        if let RuleExpression::QuantizeInteger { value, .. } =
            &mut program(&mut f).nodes[1].expression
        {
            *value = key(target);
        }
        if target == "missing" {
            assert!(OwnedRulePackage::new(f.rules.clone(), &f.schema, limits).is_err());
            bad(&f, "unknown node reference");
        } else {
            bad(&f, "cycle");
        }
    }
}

#[test]
fn v8_is_explicit_and_prior_versions_retain_old_program_bytes_and_identities() {
    let f = quantizer(1.0, RuleRounding::Floor);
    assert_eq!(
        f.rules.operations_version.as_str(),
        "owned-domain-operations-v8"
    );
    let mut current = quantizer(1.0, RuleRounding::Floor);
    current.rules.operations_version = key(OWNED_RULE_OPERATIONS_VERSION);
    let compiled_current = compile(&current);
    assert_eq!(
        evaluate(
            &current,
            &compiled_current,
            &mut compiled_current.new_scratch(),
            Some(4.9)
        ),
        applied(4)
    );
    for version in [OWNED_RULE_OPERATIONS_V6, OWNED_RULE_OPERATIONS_V7] {
        let mut old = quantizer(1.0, RuleRounding::Floor);
        old.rules.operations_version = key(version);
        // Even an unused new operation is outside the old finite operation set.
        program(&mut old).effects.clear();
        bad(&old, "QuantizeInteger requires owned-domain-operations-v8");
        let mut unchanged = quantizer(1.0, RuleRounding::Floor);
        unchanged.rules.operations_version = key(version);
        program(&mut unchanged).nodes[1] = literal("result", integer(7));
        let bytes = serde_json::to_vec(&unchanged.rules).unwrap();
        let expected = digest_owned(
            "owned-rule-programs-v2",
            &unchanged.rules,
            RuleLimits::default().max_wire_bytes,
        )
        .unwrap();
        let c = compile(&unchanged);
        assert_eq!(c.identity(), expected);
        assert_eq!(serde_json::to_vec(c.input()).unwrap(), bytes);
        assert_eq!(serde_json::to_vec(&unchanged.rules).unwrap(), bytes);
        assert_eq!(
            evaluate(&unchanged, &c, &mut c.new_scratch(), None),
            applied(7)
        );
    }
    let mut unknown = f;
    unknown.rules.operations_version = key("owned-domain-operations-future");
    bad(&unknown, "unsupported operation version");
}

#[test]
fn immutable_programs_reuse_worker_scratch_and_authored_quantum_changes_identity() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<CompiledRulePackage>();
    send_sync::<RuleScratch>();
    let first = quantizer(1.0, RuleRounding::NearestTiesPositive);
    let second = quantizer(0.25, RuleRounding::NearestTiesPositive);
    let a = compile(&first);
    let b = compile(&second);
    assert_ne!(a.identity(), b.identity());
    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let mut scratch = a.new_scratch();
                    for _ in 0..32 {
                        assert_eq!(evaluate(&first, &a, &mut scratch, Some(1.4)), applied(1));
                        assert_eq!(evaluate(&second, &b, &mut scratch, Some(1.4)), applied(6));
                        assert_eq!(
                            evaluate(&first, &a, &mut scratch, None),
                            EffectDisposition::Unresolved {
                                input: key("input")
                            }
                        );
                    }
                })
            })
            .collect::<Vec<_>>();
        for thread in handles {
            thread.join().unwrap();
        }
    });
}

#[test]
fn authored_integer_scaling_can_apply_percentage_and_more_before_quantizing_and_clamping() {
    let mut f = quantizer(1.0, RuleRounding::NearestTiesPositive);
    let unit: UnitDefId = id(&f, "unit.damage");
    let percent: UnitDefId = id(&f, "unit.percent");
    let factor: UnitDefId = id(&f, "unit.factor");
    let stat: StatDefId = id(&f, "stat.effective-attribute");
    let p = program(&mut f);
    p.reads = [
        (
            "base",
            ComputedValueType::Integer,
            ContributionKind::Add,
            ContributionReduction::Sum,
            integer(0),
        ),
        (
            "inc",
            ComputedValueType::Quantity {
                unit: percent.clone(),
            },
            ContributionKind::Increase,
            ContributionReduction::Sum,
            quantity(0.0, &percent),
        ),
        (
            "more",
            ComputedValueType::Quantity {
                unit: factor.clone(),
            },
            ContributionKind::Multiply,
            ContributionReduction::Product,
            quantity(1.0, &factor),
        ),
    ]
    .into_iter()
    .map(
        |(name, value_type, contribution, reduction, empty)| RuleRead {
            id: key(name),
            value_type,
            source: RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: stat.clone(),
                contribution,
                reduction,
                empty,
            },
        },
    )
    .collect();
    p.nodes = vec![
        node("base", RuleExpression::Read { input: key("base") }),
        node("inc", RuleExpression::Read { input: key("inc") }),
        node("more", RuleExpression::Read { input: key("more") }),
        literal("one-unit", quantity(1.0, &unit)),
        literal("one-factor", quantity(1.0, &factor)),
        literal("zero", integer(0)),
        node(
            "base-quantity",
            RuleExpression::ScaleInteger {
                value: key("one-unit"),
                count: key("base"),
            },
        ),
        node(
            "inc-ratio",
            RuleExpression::PercentAsFactor {
                percent: key("inc"),
                unit: factor.clone(),
            },
        ),
        node(
            "inc-factor",
            RuleExpression::Add {
                left: key("one-factor"),
                right: key("inc-ratio"),
            },
        ),
        node(
            "combined-factor",
            RuleExpression::Scale {
                value: key("inc-factor"),
                factor: key("more"),
            },
        ),
        node(
            "scaled",
            RuleExpression::Scale {
                value: key("base-quantity"),
                factor: key("combined-factor"),
            },
        ),
        node(
            "rounded",
            RuleExpression::QuantizeInteger {
                value: key("scaled"),
                quantum: FiniteQuantity::new(1.0, unit).unwrap(),
                mode: RuleRounding::NearestTiesPositive,
            },
        ),
        node(
            "nonnegative",
            RuleExpression::Maximum {
                left: key("rounded"),
                right: key("zero"),
            },
        ),
        node(
            "base-zero",
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: key("base"),
                right: key("zero"),
            },
        ),
        node(
            "result",
            RuleExpression::Select {
                condition: key("base-zero"),
                when_true: key("zero"),
                when_false: key("nonnegative"),
            },
        ),
    ];
    let c = compile(&f);
    let mut scratch = c.new_scratch();
    for (base, inc, more, expected) in [
        (25, 10.0, 1.5, 41),
        (30, 50.0, 1.5, 68),
        (-25, 10.0, 1.5, 0),
    ] {
        let facts = vec![
            RuleFact {
                read: key("base"),
                value: integer(base),
            },
            RuleFact {
                read: key("inc"),
                value: quantity(inc, &percent),
            },
            RuleFact {
                read: key("more"),
                value: quantity(more, &factor),
            },
        ];
        let result = c
            .evaluate(
                &f.rules.owners[0].owner,
                &key("quantize"),
                &facts,
                &f.schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects[0].disposition, applied(expected));
    }
    let zero = vec![RuleFact {
        read: key("base"),
        value: integer(0),
    }];
    assert_eq!(
        c.evaluate(
            &f.rules.owners[0].owner,
            &key("quantize"),
            &zero,
            &f.schema,
            &mut scratch
        )
        .unwrap()
        .effects[0]
            .disposition,
        applied(0)
    );
    let nonzero = vec![RuleFact {
        read: key("base"),
        value: integer(1),
    }];
    assert!(matches!(
        c.evaluate(
            &f.rules.owners[0].owner,
            &key("quantize"),
            &nonzero,
            &f.schema,
            &mut scratch
        )
        .unwrap()
        .effects[0]
            .disposition,
        EffectDisposition::Unresolved { .. }
    ));
}
