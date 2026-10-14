//! Numeric domains are evaluator availability, never game requirements.
use super::*;
use rayon::prelude::*;

fn int_domain(minimum: i64, maximum: i64) -> NumericDomain {
    NumericDomain::Integer(IntegerRange {
        minimum: BoundedInteger::new(minimum).unwrap(),
        maximum: BoundedInteger::new(maximum).unwrap(),
    })
}
fn qty_domain(f: &Fixture, minimum: f64, maximum: f64) -> NumericDomain {
    let unit = id(f, "unit.damage");
    NumericDomain::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(minimum, unit.clone()).unwrap(),
        maximum: FiniteQuantity::new(maximum, unit).unwrap(),
    })
}
fn world(domain: NumericDomain) -> Fixture {
    let mut f = fixture();
    f.rules.operations_version = key(OWNED_RULE_OPERATIONS_V27);
    f.rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
    f.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    let (ty, stat) = match &domain {
        NumericDomain::Integer(_) => (
            ComputedValueType::Integer,
            id(&f, "stat.effective-attribute"),
        ),
        NumericDomain::Quantity(r) => (
            ComputedValueType::Quantity {
                unit: r.minimum.unit().clone(),
            },
            id(&f, "stat.damage"),
        ),
    };
    let mut p = empty_program();
    p.reads.push(RuleRead {
        id: key("input"),
        value_type: ty,
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: stat.clone(),
        },
    });
    p.nodes.extend([
        node(
            "input",
            RuleExpression::Read {
                input: key("input"),
            },
        ),
        node(
            "checked",
            RuleExpression::CheckNumericDomain {
                value: key("input"),
                domain,
            },
        ),
        node(
            "satisfied",
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: key("checked"),
                right: key("input"),
            },
        ),
    ]);
    p.effects.extend([
        RuleEffect {
            id: key("value"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat,
                value: key("checked"),
            },
        },
        requirement("requirement", "satisfied", None),
    ]);
    replace(&mut f, p);
    f
}
fn run(
    f: &Fixture,
    c: &CompiledRulePackage,
    value: Option<ParameterValue>,
    scratch: &mut RuleScratch,
) -> ProgramEvaluation {
    let facts: Vec<_> = value
        .into_iter()
        .map(|value| RuleFact {
            read: key("input"),
            value,
        })
        .collect();
    c.evaluate(
        &f.rules.owners[0].owner,
        &key("test"),
        &facts,
        &f.schema,
        scratch,
    )
    .unwrap()
}
fn domain(f: &Fixture) -> NumericDomain {
    let RuleExpression::CheckNumericDomain { domain, .. } =
        &f.rules.owners[0].programs.members[0].nodes[1].expression
    else {
        panic!()
    };
    domain.clone()
}
fn rejected(f: &Fixture, value: ParameterValue) -> EffectDisposition {
    EffectDisposition::UnsupportedDomain {
        node: key("checked"),
        domain: UnsupportedDomainCause::NumericRange {
            value,
            domain: Box::new(domain(f)),
        },
    }
}

#[test]
fn inclusive_numeric_endpoints_adjacent_values_and_canonical_zero_pass_through() {
    let integer = world(int_domain(-7, 11));
    let quantities = world(qty_domain(&fixture(), -7., 11.));
    for (f, values) in [
        (
            &integer,
            vec![
                (integer_value(-8), false),
                (integer_value(-7), true),
                (integer_value(0), true),
                (integer_value(11), true),
                (integer_value(12), false),
            ],
        ),
        (
            &quantities,
            [(-7_f64).next_down(), -7., -0., 0., 11., 11_f64.next_up()]
                .into_iter()
                .map(|v| {
                    (
                        quantity(v, &id(&quantities, "unit.damage")),
                        (-7.0..=11.0).contains(&v),
                    )
                })
                .collect(),
        ),
    ] {
        let c = compile(f);
        let mut scratch = c.new_scratch();
        for (value, inside) in values {
            let result = run(f, &c, Some(value.clone()), &mut scratch);
            let expected = if inside {
                EffectDisposition::Applied {
                    value: value.clone(),
                }
            } else {
                rejected(f, value.clone())
            };
            assert_eq!(result.effects[0].disposition, expected);
            assert_eq!(
                result.effects[1].disposition,
                if inside {
                    EffectDisposition::Applied {
                        value: ParameterValue::Boolean(true),
                    }
                } else {
                    expected
                }
            );
            if let EffectDisposition::Applied {
                value: ParameterValue::Quantity(v),
            } = &result.effects[0].disposition
                && v.value() == 0.
            {
                assert_eq!(v.value().to_bits(), 0_f64.to_bits());
            }
        }
    }
}
fn integer_value(n: i64) -> ParameterValue {
    integer(n)
}

#[test]
fn finite_extremes_are_supported_only_when_explicitly_in_domain() {
    let f = world(int_domain(BoundedInteger::MIN, BoundedInteger::MAX));
    let q = world(qty_domain(&fixture(), -f64::MAX, f64::MAX));
    for (f, values) in [
        (
            &f,
            vec![integer(BoundedInteger::MIN), integer(BoundedInteger::MAX)],
        ),
        (
            &q,
            vec![
                quantity(-f64::MAX, &id(&q, "unit.damage")),
                quantity(f64::MAX, &id(&q, "unit.damage")),
            ],
        ),
    ] {
        let c = compile(f);
        for value in values {
            assert_eq!(
                run(f, &c, Some(value.clone()), &mut c.new_scratch()).effects[0].disposition,
                EffectDisposition::Applied { value }
            );
        }
    }
}

#[test]
fn compiler_rejects_reversed_domains_mismatched_types_units_and_unavailable_units() {
    let mut f = world(int_domain(2, 1));
    bad(&f, "reversed numeric integer domain");
    f = world(qty_domain(&fixture(), 2., 1.));
    bad(&f, "invalid numeric quantity domain");
    for mode in 0..4 {
        let mut f = world(qty_domain(&fixture(), 0., 5.));
        let other = id(
            &f,
            if mode == 2 {
                "missing.unit"
            } else {
                "unit.factor"
            },
        );
        let p = program(&mut f.rules, "test");
        let RuleExpression::CheckNumericDomain { domain, .. } = &mut p.nodes[1].expression else {
            panic!()
        };
        if mode == 0 {
            *domain = int_domain(0, 5);
        } else if let NumericDomain::Quantity(r) = domain {
            r.maximum = FiniteQuantity::new(5., other.clone()).unwrap();
            if mode >= 2 {
                r.minimum = FiniteQuantity::new(0., other).unwrap();
            }
        }
        bad(
            &f,
            match mode {
                0 | 3 => "numeric domain type/unit differs",
                1 => "invalid numeric quantity domain",
                _ => "definition/schema is missing",
            },
        );
    }
    let mut f = world(int_domain(0, 5));
    program(&mut f.rules, "test").nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    bad(&f, "numeric domain type/unit differs");
    f.rules.operations_version = key(OWNED_RULE_OPERATIONS_V26);
    bad(&f, "numeric domains require");
}

#[test]
fn missing_arithmetic_failures_and_lazy_branches_remain_distinct() {
    let mut f = world(int_domain(0, 5));
    let c = compile(&f);
    assert_eq!(
        run(&f, &c, None, &mut c.new_scratch()).effects[0].disposition,
        EffectDisposition::Unresolved {
            input: key("input")
        }
    );
    let p = program(&mut f.rules, "test");
    p.nodes.extend([
        literal("choose", ParameterValue::Boolean(false)),
        literal("zero", integer(0)),
        node(
            "selected",
            RuleExpression::Select {
                condition: key("choose"),
                when_true: key("checked"),
                when_false: key("zero"),
            },
        ),
    ]);
    let RuleEffectKind::Derive { value, .. } = &mut p.effects[0].effect else {
        panic!()
    };
    *value = key("selected");
    p.effects.pop();
    let c = compile(&f);
    for input in [None, Some(integer(6))] {
        assert_eq!(
            run(&f, &c, input, &mut c.new_scratch()).effects[0].disposition,
            EffectDisposition::Applied { value: integer(0) }
        );
    }
    program(&mut f.rules, "test")
        .nodes
        .iter_mut()
        .find(|n| n.id == key("choose"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(true),
    };
    let c = compile(&f);
    assert_eq!(
        run(&f, &c, Some(integer(6)), &mut c.new_scratch()).effects[0].disposition,
        rejected(&f, integer(6))
    );

    let mut f = world(qty_domain(&fixture(), 0., 5.));
    let unit = id(&f, "unit.factor");
    let p = program(&mut f.rules, "test");
    p.nodes.push(literal("divisor", quantity(0., &unit)));
    p.nodes.push(node(
        "division",
        RuleExpression::DivideFactor {
            value: key("input"),
            divisor: key("divisor"),
        },
    ));
    let RuleExpression::CheckNumericDomain { value, .. } = &mut p.nodes[1].expression else {
        panic!()
    };
    *value = key("division");
    let c = compile(&f);
    let r = run(
        &f,
        &c,
        Some(quantity(1., &id(&f, "unit.damage"))),
        &mut c.new_scratch(),
    );
    assert!(r.effects.iter().all(|e| e.disposition
        == EffectDisposition::NumericalError {
            node: key("division"),
            reason: NumericalFailure::DivisionByZero
        }));
}

#[test]
fn numeric_domains_change_identity_and_restore_scratch_across_parallel_workers() {
    let a = world(int_domain(0, 5));
    let b = world(int_domain(0, 6));
    let ca = compile(&a);
    let cb = compile(&b);
    assert_ne!(ca.identity(), cb.identity());
    let inputs = [Some(integer(5)), Some(integer(6)), None, Some(integer(0))];
    let expected: Vec<_> = inputs
        .iter()
        .map(|v| run(&a, &ca, v.clone(), &mut ca.new_scratch()))
        .collect();
    let mut scratch = ca.new_scratch();
    for n in [0, 1, 2, 3, 1, 0] {
        assert_eq!(run(&a, &ca, inputs[n].clone(), &mut scratch), expected[n]);
    }
    assert!(matches!(
        run(&b, &cb, Some(integer(6)), &mut scratch).effects[0].disposition,
        EffectDisposition::Applied { .. }
    ));
    assert_eq!(run(&a, &ca, Some(integer(6)), &mut scratch), expected[1]);
    let results = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..24)
                .into_par_iter()
                .map_init(
                    || ca.new_scratch(),
                    |s, n| run(&a, &ca, inputs[n % 4].clone(), s),
                )
                .collect::<Vec<_>>()
        });
    for (n, result) in results.into_iter().enumerate() {
        assert_eq!(result, expected[n % 4]);
    }
}
