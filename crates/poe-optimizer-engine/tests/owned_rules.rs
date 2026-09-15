use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::owned_rules::*;
#[path = "support/owned_rule_fixture.rs"]
mod owned_rule_fixture;
use owned_rule_fixture::{Fixture, fixture};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn integer(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn quantity(v: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, unit.clone()).unwrap())
}
fn id<K: DefinitionDomain>(f: &Fixture, s: &str) -> DefId<K> {
    DefId::parse(f.rules.namespace.clone(), s).unwrap()
}
fn facts(values: &[(OwnedDefinitionKey, ParameterValue)]) -> Vec<RuleFact> {
    values
        .iter()
        .map(|(read, value)| RuleFact {
            read: read.clone(),
            value: value.clone(),
        })
        .collect()
}
fn program<'a>(rules: &'a mut RulePackageInput, id: &str) -> &'a mut RuleProgram {
    rules
        .owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .find(|p| p.id == key(id))
        .unwrap()
}
fn compile(f: &Fixture) -> CompiledRulePackage {
    CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap()
}
fn bad(f: &Fixture, contains: &str) {
    let err = CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap_err();
    assert!(err.to_string().contains(contains), "{err}");
}
fn node(id: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(id),
        expression,
    }
}
fn literal(id: &str, value: ParameterValue) -> RuleNode {
    node(id, RuleExpression::Literal { value })
}
fn requirement(id: &str, value: &str, when: Option<&str>) -> RuleEffect {
    RuleEffect {
        id: key(id),
        when: when.map(key),
        effect: RuleEffectKind::Requirement {
            satisfied: key(value),
            code: key("test-requirement"),
        },
    }
}
fn replace(f: &mut Fixture, p: RuleProgram) -> SchemaSubject {
    let owner = f.rules.owners[0].owner.clone();
    f.rules.owners = vec![DefinitionRules {
        owner: owner.clone(),
        programs: DeclaredSet::complete(vec![p]),
    }];
    owner
}
fn empty_program() -> RuleProgram {
    RuleProgram {
        id: key("test"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![],
        effects: vec![],
    }
}

#[test]
fn four_injected_effect_families_preserve_all_effects_and_false_vs_inactive() {
    let f = fixture();
    let compiled = compile(&f);
    let mut scratch = compiled.new_scratch();
    assert_eq!(f.cases.len(), 8);
    for case in &f.cases {
        let actual = compiled
            .evaluate(
                &case.owner,
                &case.program,
                &facts(&case.facts),
                &f.schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(actual.owner, case.owner);
        assert_eq!(actual.program, case.program);
        assert_eq!(actual.owner_programs_closure, SchemaClosure::Complete);
        assert_eq!(actual.effects.len(), case.expected.len());
        for (actual, expected) in actual.effects.iter().zip(&case.expected) {
            assert_eq!(actual.id, expected.id, "{}", case.name);
            assert_eq!(
                actual.disposition,
                match &expected.value {
                    Some(value) => EffectDisposition::Applied {
                        value: value.clone()
                    },
                    None => EffectDisposition::Inactive,
                },
                "{}",
                case.name
            );
        }
    }
}
#[test]
fn data_changes_scratch_cross_package_reuse_and_parallel_workers_are_isolated() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<CompiledRulePackage>();
    send_sync::<RuleScratch>();
    let a = fixture();
    let mut b = fixture();
    let factor = program(&mut b.rules, "support-applicability")
        .nodes
        .iter_mut()
        .find(|n| n.id == key("factor"))
        .unwrap();
    let RuleExpression::Literal {
        value: ParameterValue::Quantity(v),
    } = &mut factor.expression
    else {
        panic!()
    };
    *v = FiniteQuantity::new(2.0, v.unit().clone()).unwrap();
    let ca = compile(&a);
    let cb = compile(&b);
    assert_ne!(ca.identity(), cb.identity());
    let case = a
        .cases
        .iter()
        .find(|c| c.name == "support applicable")
        .unwrap();
    let input = facts(&case.facts);
    let mut scratch = ca.new_scratch();
    let first = ca
        .evaluate(&case.owner, &case.program, &input, &a.schema, &mut scratch)
        .unwrap();
    let changed = cb
        .evaluate(&case.owner, &case.program, &input, &b.schema, &mut scratch)
        .unwrap();
    assert_ne!(first, changed);
    assert_eq!(
        first,
        ca.evaluate(&case.owner, &case.program, &input, &a.schema, &mut scratch)
            .unwrap()
    );
    std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let mut scratch = ca.new_scratch();
                    ca.evaluate(&case.owner, &case.program, &input, &a.schema, &mut scratch)
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        for h in handles {
            assert_eq!(h.join().unwrap(), first);
        }
    });
}
#[test]
fn canonical_indices_ignore_declaration_order_but_keep_effect_ledger_order() {
    let mut f = fixture();
    let first = compile(&f);
    f.rules.owners.reverse();
    for o in &mut f.rules.owners {
        o.programs.members.reverse();
        for p in &mut o.programs.members {
            p.nodes.reverse();
            p.reads.reverse();
        }
    }
    let second = compile(&f);
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.input(), second.input());
    program(&mut f.rules, "support-applicability")
        .effects
        .reverse();
    assert_ne!(first.identity(), compile(&f).identity());
}
#[test]
fn missing_quality_amount_is_demanded_only_on_selected_branch_and_never_zero() {
    let f = fixture();
    let c = compile(&f);
    let case = &f.cases[0];
    let mut input = facts(&case.facts);
    input.retain(|f| f.read != key("quality"));
    let mut s = c.new_scratch();
    let missing = c
        .evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
        .unwrap();
    assert_eq!(
        missing.effects[0].disposition,
        EffectDisposition::Unresolved {
            input: key("quality")
        }
    );
    input
        .iter_mut()
        .find(|f| f.read == key("has-quality"))
        .unwrap()
        .value = ParameterValue::Boolean(false);
    let result = c
        .evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
        .unwrap();
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied {
            value: f.cases[1].expected[0].value.clone().unwrap()
        }
    );
    input.retain(|f| f.read != key("flat"));
    let result = c
        .evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
        .unwrap();
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Unresolved { input: key("flat") }
    );
}
#[test]
fn lazy_all_any_and_guards_skip_missing_boolean_reads_and_keep_false_values() {
    let mut f = fixture();
    let mut p = empty_program();
    let capability = id(&f, "capability.supported-action");
    p.context = RuleEntityKind::Action;
    p.reads = vec![RuleRead {
        id: key("missing"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Capability {
            entity: RuleEntity::Current,
            capability,
        },
    }];
    p.nodes = vec![
        literal("false", ParameterValue::Boolean(false)),
        literal("true", ParameterValue::Boolean(true)),
        node(
            "missing-value",
            RuleExpression::Read {
                input: key("missing"),
            },
        ),
        node(
            "all",
            RuleExpression::All {
                values: vec![key("false"), key("missing-value")],
            },
        ),
        node(
            "any",
            RuleExpression::Any {
                values: vec![key("true"), key("missing-value")],
            },
        ),
        node("empty-all", RuleExpression::All { values: vec![] }),
        node("empty-any", RuleExpression::Any { values: vec![] }),
    ];
    p.effects = vec![
        requirement("all", "all", None),
        requirement("any", "any", None),
        requirement("guarded", "missing-value", Some("false")),
        requirement("demanded", "missing-value", None),
        requirement("empty-all", "empty-all", None),
        requirement("empty-any", "empty-any", None),
    ];
    let owner = replace(&mut f, p);
    let c = compile(&f);
    let result = c
        .evaluate(&owner, &key("test"), &[], &f.schema, &mut c.new_scratch())
        .unwrap();
    let d = result
        .effects
        .into_iter()
        .map(|e| e.disposition)
        .collect::<Vec<_>>();
    assert_eq!(
        d,
        vec![
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(false)
            },
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(true)
            },
            EffectDisposition::Inactive,
            EffectDisposition::Unresolved {
                input: key("missing")
            },
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(true)
            },
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(false)
            }
        ]
    );
}
#[test]
fn unreachable_cycles_references_units_and_types_are_always_rejected() {
    let mut f = fixture();
    let p = program(&mut f.rules, "conditional-grant");
    p.nodes
        .push(node("dead", RuleExpression::Not { value: key("dead") }));
    bad(&f, "cycle");
    let mut f = fixture();
    program(&mut f.rules, "conditional-grant").nodes.push(node(
        "dead",
        RuleExpression::Not {
            value: key("absent"),
        },
    ));
    bad(&f, "unknown node");
    let mut f = fixture();
    let unit = id(&f, "unit.absent");
    program(&mut f.rules, "conditional-grant")
        .nodes
        .push(literal("dead", quantity(1.0, &unit)));
    bad(&f, "missing, unmapped");
    let mut f = fixture();
    program(&mut f.rules, "conditional-grant").nodes.push(node(
        "dead",
        RuleExpression::Not {
            value: key("threshold"),
        },
    ));
    bad(&f, "Not requires boolean");
    let mut f = fixture();
    program(&mut f.rules, "conditional-grant").nodes.push(node(
        "dead",
        RuleExpression::Read {
            input: key("missing"),
        },
    ));
    bad(&f, "unknown read");
}
#[test]
fn duplicate_keys_versions_and_schema_binding_are_rejected() {
    let mut f = fixture();
    f.rules.owners.push(f.rules.owners[0].clone());
    bad(&f, "duplicate owner");
    let mut f = fixture();
    let p = f.rules.owners[0].programs.members[0].clone();
    f.rules.owners[0].programs.members.push(p);
    bad(&f, "duplicate program");
    for field in 0..3 {
        let mut f = fixture();
        let p = program(&mut f.rules, "conditional-grant");
        match field {
            0 => p.reads.push(p.reads[0].clone()),
            1 => p.nodes.push(p.nodes[0].clone()),
            _ => p.effects.push(p.effects[0].clone()),
        };
        bad(&f, "duplicate");
    }
    let mut f = fixture();
    f.rules.operations_version = key("unknown-operations");
    bad(&f, "unsupported operation");
    let mut f = fixture();
    f.rules.schema_version += 1;
    bad(&f, "unsupported rule package");
    let mut f = fixture();
    f.rules.definitions.release.push_str("-changed");
    bad(&f, "identity/namespace");
}
#[test]
fn dimension_mismatches_and_nonpositive_rounding_quantum_reject() {
    let mut f = fixture();
    let damage = id(&f, "unit.damage");
    let p = program(&mut f.rules, "local-item");
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("increased"))
        .unwrap()
        .expression = RuleExpression::Scale {
        value: key("base"),
        factor: key("base"),
    };
    bad(&f, "wrong unit dimension");
    let mut f = fixture();
    program(&mut f.rules, "local-item")
        .nodes
        .iter_mut()
        .find(|n| n.id == key("rounded"))
        .unwrap()
        .expression = RuleExpression::Round {
        value: key("base"),
        quantum: FiniteQuantity::new(0.0, damage.clone()).unwrap(),
        mode: RuleRounding::Floor,
    };
    bad(&f, "round quantum");
    let mut f = fixture();
    program(&mut f.rules, "local-item").nodes.push(node(
        "dead",
        RuleExpression::Add {
            left: key("base"),
            right: key("one"),
        },
    ));
    bad(&f, "identical types/units");
    let mut f = fixture();
    program(&mut f.rules, "local-item").nodes.push(node(
        "dead",
        RuleExpression::PercentAsFactor {
            percent: key("base"),
            unit: damage,
        },
    ));
    bad(&f, "wrong unit dimension");
}
#[test]
fn current_relative_targets_and_grant_declarations_are_not_interchangeable() {
    let mut f = fixture();
    program(&mut f.rules, "support-applicability").context = RuleEntityKind::Actor;
    bad(&f, "capability target/context");
    let mut f = fixture();
    let p = program(&mut f.rules, "conditional-grant");
    p.context = RuleEntityKind::EquipmentUse;
    bad(&f, "relative Actor");
    let mut f = fixture();
    let gem = id(&f, "gem.support");
    let p = program(&mut f.rules, "conditional-grant");
    let RuleEffectKind::ActivateGrant { slot, .. } = &mut p.effects[0].effect else {
        panic!()
    };
    slot.declaration = SlotOwnerDefId::Gem(gem);
    bad(&f, "grant declaration");
    let mut f = fixture();
    let missing = id(&f, "grant.missing");
    let p = program(&mut f.rules, "conditional-grant");
    let RuleEffectKind::ActivateGrant { slot, .. } = &mut p.effects[0].effect else {
        panic!()
    };
    slot.slot = missing;
    bad(&f, "not a declared member");
}
#[test]
fn contribution_identity_reduction_and_effect_units_are_checked() {
    let mut f = fixture();
    let p = program(&mut f.rules, "local-item");
    let RuleReadSource::Contributions { reduction, .. } = &mut p.reads[0].source else {
        panic!()
    };
    *reduction = ContributionReduction::Product;
    bad(&f, "reduction mismatch");
    let mut f = fixture();
    let p = program(&mut f.rules, "local-item");
    let RuleReadSource::Contributions {
        empty: ParameterValue::Quantity(v),
        ..
    } = &mut p.reads[0].source
    else {
        panic!()
    };
    *v = FiniteQuantity::new(1.0, v.unit().clone()).unwrap();
    bad(&f, "empty identity");
    let mut f = fixture();
    let p = program(&mut f.rules, "support-applicability");
    let RuleEffectKind::Contribute { contribution, .. } = &mut p.effects[1].effect else {
        panic!()
    };
    *contribution = ContributionKind::Add;
    bad(&f, "exact stat value type/unit");
}
#[test]
fn all_supplied_facts_are_validated_even_when_a_branch_is_inactive() {
    let f = fixture();
    let c = compile(&f);
    let case = &f.cases[1];
    let base = facts(&case.facts);
    let mut s = c.new_scratch();
    let mut input = base.clone();
    input.push(RuleFact {
        read: key("quality"),
        value: integer(25),
    });
    assert!(
        c.evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
            .unwrap_err()
            .to_string()
            .contains("type/unit")
    );
    let unit = id(&f, "unit.percent");
    let mut input = base.clone();
    input.push(RuleFact {
        read: key("quality"),
        value: quantity(101.0, &unit),
    });
    assert!(
        c.evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
            .unwrap_err()
            .to_string()
            .contains("value schema")
    );
    let mut input = base.clone();
    input.push(input[0].clone());
    assert!(
        c.evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
            .is_err()
    );
    let mut input = base.clone();
    input[0].read = key("not-declared");
    assert!(
        c.evaluate(&case.owner, &case.program, &input, &f.schema, &mut s)
            .unwrap_err()
            .to_string()
            .contains("unknown fact")
    );
    assert!(
        c.evaluate(&case.owner, &key("missing"), &base, &f.schema, &mut s)
            .is_err()
    );
    let mut changed = f.schema.input().clone();
    changed.release = key("different-schema");
    let changed = OwnedDefinitionSchemaPackage::new(changed, OwnedSchemaLimits::default()).unwrap();
    assert!(
        c.evaluate(&case.owner, &case.program, &base, &changed, &mut s)
            .unwrap_err()
            .to_string()
            .contains("identity/namespace")
    );
    assert!(matches!(
        c.evaluate(&case.owner, &case.program, &base, &f.schema, &mut s)
            .unwrap()
            .effects[0]
            .disposition,
        EffectDisposition::Applied { .. }
    ));
}
#[test]
fn quality_kind_requires_exact_owner_policy_and_known_schema() {
    let mut f = fixture();
    let p = program(&mut f.rules, "local-item");
    p.reads[3].source = RuleReadSource::GemQualityAmount {
        quality: id::<QualityDefinition>(&fixture(), "quality.local"),
    };
    bad(&f, "exact item/gem owner");
    let mut f = fixture();
    let missing = id(&f, "quality.absent");
    let p = program(&mut f.rules, "local-item");
    p.reads[3].source = RuleReadSource::ItemQualityAmount { quality: missing };
    bad(&f, "not a declared member");
}
#[test]
fn partial_program_membership_is_reported_not_silently_completed() {
    let mut f = fixture();
    let owner = f.rules.owners[0].owner.clone();
    let closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unimplemented-rule-family"),
        }],
    };
    f.rules.owners[0].programs.closure = closure.clone();
    let c = compile(&f);
    let case = &f.cases[0];
    let result = c
        .evaluate(
            &case.owner,
            &case.program,
            &facts(&case.facts),
            &f.schema,
            &mut c.new_scratch(),
        )
        .unwrap();
    assert_eq!(result.owner_programs_closure, closure);
    assert_eq!(result.effects.len(), 1);
    f.rules.owners[0].programs.closure = SchemaClosure::Partial { gaps: vec![] };
    bad(&f, "requires gaps");
}
#[test]
fn resource_bounds_cover_whole_package_including_unreachable_nodes_and_wire() {
    let f = fixture();
    for limits in [
        RuleLimits {
            max_owners: 1,
            ..Default::default()
        },
        RuleLimits {
            max_programs: 1,
            ..Default::default()
        },
        RuleLimits {
            max_reads: 1,
            ..Default::default()
        },
        RuleLimits {
            max_nodes: 1,
            ..Default::default()
        },
        RuleLimits {
            max_edges: 1,
            ..Default::default()
        },
        RuleLimits {
            max_effects: 1,
            ..Default::default()
        },
        RuleLimits {
            max_work: 1,
            ..Default::default()
        },
        RuleLimits {
            max_wire_bytes: 32,
            ..Default::default()
        },
        RuleLimits {
            max_nodes: 0,
            ..Default::default()
        },
    ] {
        assert!(CompiledRulePackage::compile(&f.rules, &f.schema, limits).is_err());
    }
}

#[test]
fn arithmetic_preserves_exact_units_integer_bounds_and_explicit_numerical_errors() {
    let mut f = fixture();
    let damage = id(&f, "unit.damage");
    let factor = id(&f, "unit.factor");
    let percent = id(&f, "unit.percent");
    let damage_stat = id(&f, "stat.damage");
    let attribute_stat = id(&f, "stat.effective-attribute");
    let mut p = empty_program();
    p.nodes = vec![
        literal("two", quantity(2.0, &damage)),
        literal("three", quantity(3.0, &damage)),
        literal("half", quantity(0.5, &factor)),
        literal("zero", quantity(0.0, &factor)),
        literal("percent", quantity(50.0, &percent)),
        literal("count", integer(3)),
        literal("negative-half", quantity(-2.5, &damage)),
        literal("max", integer(BoundedInteger::MAX)),
        literal("one-int", integer(1)),
        literal("huge", quantity(f64::MAX, &damage)),
        literal("twice", quantity(2.0, &factor)),
        literal("false", ParameterValue::Boolean(false)),
    ];
    let cases = vec![
        (
            "add",
            RuleExpression::Add {
                left: key("two"),
                right: key("three"),
            },
            quantity(5.0, &damage),
        ),
        (
            "subtract",
            RuleExpression::Subtract {
                left: key("three"),
                right: key("two"),
            },
            quantity(1.0, &damage),
        ),
        (
            "minimum",
            RuleExpression::Minimum {
                left: key("three"),
                right: key("two"),
            },
            quantity(2.0, &damage),
        ),
        (
            "maximum",
            RuleExpression::Maximum {
                left: key("three"),
                right: key("two"),
            },
            quantity(3.0, &damage),
        ),
        (
            "scale",
            RuleExpression::Scale {
                value: key("two"),
                factor: key("half"),
            },
            quantity(1.0, &damage),
        ),
        (
            "divide",
            RuleExpression::DivideFactor {
                value: key("two"),
                divisor: key("half"),
            },
            quantity(4.0, &damage),
        ),
        (
            "scale-integer",
            RuleExpression::ScaleInteger {
                value: key("two"),
                count: key("count"),
            },
            quantity(6.0, &damage),
        ),
        (
            "ratio",
            RuleExpression::Ratio {
                numerator: key("three"),
                denominator: key("two"),
                unit: factor.clone(),
            },
            quantity(1.5, &factor),
        ),
        (
            "percent-factor",
            RuleExpression::PercentAsFactor {
                percent: key("percent"),
                unit: factor.clone(),
            },
            quantity(0.5, &factor),
        ),
        (
            "round-positive",
            RuleExpression::Round {
                value: key("negative-half"),
                quantum: FiniteQuantity::new(1.0, damage.clone()).unwrap(),
                mode: RuleRounding::NearestTiesPositive,
            },
            quantity(-2.0, &damage),
        ),
        (
            "round-floor",
            RuleExpression::Round {
                value: key("negative-half"),
                quantum: FiniteQuantity::new(1.0, damage.clone()).unwrap(),
                mode: RuleRounding::Floor,
            },
            quantity(-3.0, &damage),
        ),
        (
            "round-ceiling",
            RuleExpression::Round {
                value: key("negative-half"),
                quantum: FiniteQuantity::new(1.0, damage.clone()).unwrap(),
                mode: RuleRounding::Ceiling,
            },
            quantity(-2.0, &damage),
        ),
        (
            "round-truncate",
            RuleExpression::Round {
                value: key("negative-half"),
                quantum: FiniteQuantity::new(1.0, damage.clone()).unwrap(),
                mode: RuleRounding::Truncate,
            },
            quantity(-2.0, &damage),
        ),
        (
            "integer-scale",
            RuleExpression::ScaleInteger {
                value: key("count"),
                count: key("count"),
            },
            integer(9),
        ),
    ];
    for (name, expression, value) in cases {
        p.nodes.push(node(name, expression));
        p.nodes.push(literal(&format!("expected-{name}"), value));
        p.nodes.push(node(
            &format!("compare-{name}"),
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: key(name),
                right: key(&format!("expected-{name}")),
            },
        ));
        p.effects
            .push(requirement(name, &format!("compare-{name}"), None));
    }
    let good = p.effects.len();
    p.nodes.extend([
        node(
            "division-zero",
            RuleExpression::DivideFactor {
                value: key("two"),
                divisor: key("zero"),
            },
        ),
        node(
            "integer-overflow",
            RuleExpression::Add {
                left: key("max"),
                right: key("one-int"),
            },
        ),
        node(
            "float-overflow",
            RuleExpression::Scale {
                value: key("huge"),
                factor: key("twice"),
            },
        ),
    ]);
    for (name, stat, when) in [
        ("division-zero", damage_stat.clone(), None),
        ("integer-overflow", attribute_stat, None),
        ("float-overflow", damage_stat.clone(), None),
        ("guarded-overflow", damage_stat, Some("false")),
    ] {
        p.effects.push(RuleEffect {
            id: key(name),
            when: when.map(key),
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat,
                value: key(if name == "guarded-overflow" {
                    "float-overflow"
                } else {
                    name
                }),
            },
        });
    }
    let owner = replace(&mut f, p);
    let c = compile(&f);
    let result = c
        .evaluate(&owner, &key("test"), &[], &f.schema, &mut c.new_scratch())
        .unwrap();
    for effect in &result.effects[..good] {
        assert_eq!(
            effect.disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(true)
            },
            "{}",
            effect.id
        );
    }
    for (offset, name, reason) in [
        (0, "division-zero", NumericalFailure::DivisionByZero),
        (1, "integer-overflow", NumericalFailure::IntegerOverflow),
        (2, "float-overflow", NumericalFailure::NonFinite),
    ] {
        assert_eq!(
            result.effects[good + offset].disposition,
            EffectDisposition::NumericalError {
                node: key(name),
                reason
            }
        );
    }
    assert_eq!(
        result.effects[good + 3].disposition,
        EffectDisposition::Inactive
    );
}
#[test]
fn option_facts_and_parameter_membership_remain_typed_and_schema_bound() {
    let mut f = fixture();
    let modifier = id(&f, "modifier.conditioned");
    let option_a = id(&f, "option.a");
    let option_b = id(&f, "option.b");
    let option_stat = id(&f, "stat.option");
    let slot = DeclaredSlot {
        declaration: SlotOwnerDefId::Modifier(modifier.clone()),
        slot: id::<ParameterSlotDefinition>(&f, "parameter.option"),
    };
    let mut schema = f.schema.input().clone();
    for option in [&option_a, &option_b] {
        schema
            .definitions
            .push(DefinitionDescriptor::Option(DefinitionEntry {
                id: option.clone(),
                schema: SchemaState::Known(OptionSchema {}),
            }));
    }
    schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: option_stat.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Option,
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    for d in &mut schema.definitions {
        if let DefinitionDescriptor::Modifier(e) = d
            && e.id == modifier
        {
            let SchemaState::Known(s) = &mut e.schema else {
                panic!()
            };
            s.declarations.parameters.members.push(slot.clone());
        }
    }
    schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: slot.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(vec![option_a.clone()]),
                },
                presence: SlotPresence::OptionalOnce,
                sites: vec![ParameterSite::ModifierRoll],
            }),
        }));
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    f.rules.definitions = f.schema.identity().clone();
    let mut p = empty_program();
    p.reads = vec![
        RuleRead {
            id: key("parameter"),
            value_type: ComputedValueType::Option,
            source: RuleReadSource::Parameter { slot: slot.clone() },
        },
        RuleRead {
            id: key("stat"),
            value_type: ComputedValueType::Option,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Actor,
                stat: option_stat,
            },
        },
    ];
    p.nodes = vec![
        node(
            "parameter",
            RuleExpression::Read {
                input: key("parameter"),
            },
        ),
        node("stat", RuleExpression::Read { input: key("stat") }),
        node(
            "equal",
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: key("parameter"),
                right: key("stat"),
            },
        ),
    ];
    p.effects = vec![requirement("equal", "equal", None)];
    let owner = SchemaSubject::Definition(DefinitionAddress::Modifier(modifier));
    f.rules.owners = vec![DefinitionRules {
        owner: owner.clone(),
        programs: DeclaredSet::complete(vec![p]),
    }];
    let c = compile(&f);
    let mut scratch = c.new_scratch();
    let mut input = vec![
        RuleFact {
            read: key("parameter"),
            value: ParameterValue::Option(option_a.clone()),
        },
        RuleFact {
            read: key("stat"),
            value: ParameterValue::Option(option_b.clone()),
        },
    ];
    assert_eq!(
        c.evaluate(&owner, &key("test"), &input, &f.schema, &mut scratch)
            .unwrap()
            .effects[0]
            .disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(false)
        }
    );
    input[1].value = ParameterValue::Option(option_a);
    assert_eq!(
        c.evaluate(&owner, &key("test"), &input, &f.schema, &mut scratch)
            .unwrap()
            .effects[0]
            .disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(true)
        }
    );
    input[0].value = ParameterValue::Option(option_b);
    assert!(
        c.evaluate(&owner, &key("test"), &input, &f.schema, &mut scratch)
            .unwrap_err()
            .to_string()
            .contains("membership")
    );
    input.remove(0);
    input[0].value = ParameterValue::Option(id(&f, "option.unknown"));
    assert!(
        c.evaluate(&owner, &key("test"), &input, &f.schema, &mut scratch)
            .unwrap_err()
            .to_string()
            .contains("missing, unmapped")
    );
    let foreign = OptionDefId::parse(
        GameVersionNamespace::new("other", "v1").unwrap(),
        "option.a",
    )
    .unwrap();
    input[0].value = ParameterValue::Option(foreign);
    assert!(
        c.evaluate(&owner, &key("test"), &input, &f.schema, &mut scratch)
            .is_err()
    );
    let item = id(&f, "item.template");
    let p = program(&mut f.rules, "test");
    let RuleReadSource::Parameter { slot } = &mut p.reads[0].source else {
        panic!()
    };
    slot.declaration = SlotOwnerDefId::ItemTemplate(item);
    bad(&f, "parameter declaration");
}
#[test]
fn program_gaps_require_exact_owner_game_rules_facet_and_unique_codes() {
    for change in 0..3 {
        let mut f = fixture();
        let mut gap = SchemaGap {
            subject: f.rules.owners[0].owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("gap"),
        };
        if change == 0 {
            gap.subject = f.rules.owners[1].owner.clone();
        }
        if change == 1 {
            gap.facet = SchemaFacet::InputSchema;
        }
        let gaps = if change == 2 {
            vec![gap.clone(), gap]
        } else {
            vec![gap]
        };
        f.rules.owners[0].programs.closure = SchemaClosure::Partial { gaps };
        bad(
            &f,
            if change == 2 {
                "duplicate program gap"
            } else {
                "exact owner and GameRules"
            },
        );
    }
}
#[test]
fn unmapped_referenced_schema_cannot_be_treated_as_a_missing_numeric_fact() {
    let mut f = fixture();
    let mut schema = f.schema.input().clone();
    for descriptor in &mut schema.definitions {
        if let DefinitionDescriptor::Stat(entry) = descriptor
            && entry.id.key() == &key("stat.effective-attribute")
        {
            entry.schema = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(DefinitionAddress::Stat(entry.id.clone())),
                    facet: SchemaFacet::InputSchema,
                    code: key("not-mapped"),
                }],
            };
        }
    }
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    f.rules.definitions = f.schema.identity().clone();
    bad(&f, "missing, unmapped");
}

#[test]
fn nearest_rounding_preserves_large_integers_and_adjacent_half_values() {
    let mut f = fixture();
    let damage = id(&f, "unit.damage");
    let stat = id(&f, "stat.damage");
    let mut p = empty_program();
    let cases = [
        (4_503_599_627_370_497.0, 4_503_599_627_370_497.0),
        (f64::from_bits(0.5f64.to_bits() - 1), 0.0),
        (0.5, 1.0),
        (-0.5, 0.0),
        (-2.5, -2.0),
    ];
    for (i, (input, _)) in cases.iter().enumerate() {
        let input_id = format!("input-{i}");
        let output_id = format!("output-{i}");
        p.nodes.push(literal(&input_id, quantity(*input, &damage)));
        p.nodes.push(node(
            &output_id,
            RuleExpression::Round {
                value: key(&input_id),
                quantum: FiniteQuantity::new(1.0, damage.clone()).unwrap(),
                mode: RuleRounding::NearestTiesPositive,
            },
        ));
        p.effects.push(RuleEffect {
            id: key(&output_id),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: stat.clone(),
                value: key(&output_id),
            },
        });
    }
    let owner = replace(&mut f, p);
    let c = compile(&f);
    let result = c
        .evaluate(&owner, &key("test"), &[], &f.schema, &mut c.new_scratch())
        .unwrap();
    for (effect, (_, expected)) in result.effects.iter().zip(cases) {
        assert_eq!(
            effect.disposition,
            EffectDisposition::Applied {
                value: quantity(expected, &damage)
            }
        );
    }
}

#[test]
fn directional_rounding_retains_sign_when_scaled_quotient_underflows() {
    let mut f = fixture();
    let damage = id(&f, "unit.damage");
    let stat = id(&f, "stat.damage");
    let mut p = empty_program();
    let cases = [
        (-f64::MIN_POSITIVE, RuleRounding::Floor, -f64::MAX),
        (f64::MIN_POSITIVE, RuleRounding::Ceiling, f64::MAX),
        (-f64::MIN_POSITIVE, RuleRounding::Truncate, 0.0),
        (f64::MIN_POSITIVE, RuleRounding::NearestTiesPositive, 0.0),
    ];
    for (i, (input, mode, _)) in cases.iter().enumerate() {
        let input_id = format!("input-{i}");
        let output_id = format!("output-{i}");
        p.nodes.push(literal(&input_id, quantity(*input, &damage)));
        p.nodes.push(node(
            &output_id,
            RuleExpression::Round {
                value: key(&input_id),
                quantum: FiniteQuantity::new(f64::MAX, damage.clone()).unwrap(),
                mode: *mode,
            },
        ));
        p.effects.push(RuleEffect {
            id: key(&output_id),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: stat.clone(),
                value: key(&output_id),
            },
        });
    }
    let owner = replace(&mut f, p);
    let c = compile(&f);
    let result = c
        .evaluate(&owner, &key("test"), &[], &f.schema, &mut c.new_scratch())
        .unwrap();
    for (effect, (_, _, expected)) in result.effects.iter().zip(cases) {
        assert_eq!(
            effect.disposition,
            EffectDisposition::Applied {
                value: quantity(expected, &damage)
            }
        );
    }
}
