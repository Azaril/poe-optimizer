use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_rules::*;

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("table-tests", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn bounded(n: i64) -> BoundedInteger {
    BoundedInteger::new(n).unwrap()
}
fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(bounded(n))
}
fn node(s: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(s),
        expression,
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Stat(id("output")))
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    input: RulePackageInput,
}
impl Fixture {
    fn new(ty: ComputedValueType, rows: Vec<ParameterValue>) -> Self {
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns(),
                release: key("schema"),
                semantics_version: key("tests-v1"),
                definitions: vec![
                    DefinitionDescriptor::Stat(known(
                        id("input"),
                        StatSchema {
                            value: ComputedValueType::Integer,
                            targets: vec![RuleEntityKind::Actor],
                        },
                    )),
                    DefinitionDescriptor::Stat(known(
                        id("output"),
                        StatSchema {
                            value: ty.clone(),
                            targets: vec![RuleEntityKind::Actor],
                        },
                    )),
                    DefinitionDescriptor::Unit(known(
                        id("factor"),
                        UnitSchema {
                            dimension: UnitDimension::DimensionlessFactor,
                        },
                    )),
                    DefinitionDescriptor::Unit(known(
                        id("other-factor"),
                        UnitSchema {
                            dimension: UnitDimension::DimensionlessFactor,
                        },
                    )),
                    DefinitionDescriptor::Option(known(id("option"), OptionSchema {})),
                ],
                slots: vec![],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        let input = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("tests-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![IntegerRuleTable {
                id: key("curve"),
                minimum: bounded(-1),
                maximum: bounded(rows.len() as i64 - 2),
                value_type: ty,
                rows,
            }],
            owners: vec![DefinitionRules {
                owner: owner(),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key("lookup"),
                    context: RuleEntityKind::Actor,
                    reads: vec![RuleRead {
                        id: key("level"),
                        value_type: ComputedValueType::Integer,
                        source: RuleReadSource::Stat {
                            entity: RuleEntity::Current,
                            stat: id("input"),
                        },
                    }],
                    nodes: vec![
                        node(
                            "key",
                            RuleExpression::Read {
                                input: key("level"),
                            },
                        ),
                        node(
                            "lookup",
                            RuleExpression::LookupIntegerTable {
                                table: key("curve"),
                                key: key("key"),
                            },
                        ),
                    ],
                    effects: vec![RuleEffect {
                        id: key("output"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat: id("output"),
                            value: key("lookup"),
                        },
                    }],
                }]),
            }],
        };
        Self { schema, input }
    }
    fn ints() -> Self {
        Self::new(
            ComputedValueType::Integer,
            vec![integer(7), integer(0), integer(13)],
        )
    }
    fn program(&mut self) -> &mut RuleProgram {
        &mut self.input.owners[0].programs.members[0]
    }
    fn compile(&self) -> CompiledRulePackage {
        CompiledRulePackage::compile(&self.input, &self.schema, RuleLimits::default()).unwrap()
    }
    fn evaluate(
        &self,
        compiled: &CompiledRulePackage,
        level: Option<i64>,
        scratch: &mut RuleScratch,
    ) -> EffectDisposition {
        let facts = level
            .map(|n| RuleFact {
                read: key("level"),
                value: integer(n),
            })
            .into_iter()
            .collect::<Vec<_>>();
        compiled
            .evaluate(&owner(), &key("lookup"), &facts, &self.schema, scratch)
            .unwrap()
            .effects
            .remove(0)
            .disposition
    }
    fn both_reject(&self) {
        assert!(
            OwnedRulePackage::new(
                self.input.clone(),
                &self.schema,
                RuleStorageLimits::default()
            )
            .is_err()
        );
        assert!(
            CompiledRulePackage::compile(&self.input, &self.schema, RuleLimits::default()).is_err()
        );
    }
}
fn applied(value: ParameterValue) -> EffectDisposition {
    EffectDisposition::Applied { value }
}

#[test]
fn exact_signed_integer_domain_preserves_zero_and_reports_original_lookup_context() {
    let f = Fixture::ints();
    let c = f.compile();
    let mut s = c.new_scratch();
    for (level, expected) in [(-1, 7), (0, 0), (1, 13)] {
        assert_eq!(
            f.evaluate(&c, Some(level), &mut s),
            applied(integer(expected))
        );
    }
    for level in [BoundedInteger::MIN, -2, 2, BoundedInteger::MAX] {
        assert_eq!(
            f.evaluate(&c, Some(level), &mut s),
            EffectDisposition::UnsupportedDomain {
                node: key("lookup"),
                table: key("curve"),
                key: bounded(level),
                minimum: bounded(-1),
                maximum: bounded(1),
            }
        );
    }
    assert_eq!(
        f.evaluate(&c, None, &mut s),
        EffectDisposition::Unresolved {
            input: key("level")
        }
    );
    // Indexed lookup must remain safe near the browser-exact integer extremes.
    let mut edge = Fixture::ints();
    edge.input.tables[0].minimum = bounded(BoundedInteger::MAX - 2);
    edge.input.tables[0].maximum = bounded(BoundedInteger::MAX);
    assert_eq!(
        edge.evaluate(&edge.compile(), Some(BoundedInteger::MAX), &mut s),
        applied(integer(13))
    );
    edge.input.tables[0].minimum = bounded(BoundedInteger::MIN);
    edge.input.tables[0].maximum = bounded(BoundedInteger::MIN + 2);
    assert_eq!(
        edge.evaluate(&edge.compile(), Some(BoundedInteger::MIN), &mut s),
        applied(integer(7))
    );
}

#[test]
fn lazy_guards_skip_lookup_and_arithmetic_propagates_the_demanded_domain_failure() {
    let mut f = Fixture::ints();
    f.program().nodes.extend([
        node(
            "false",
            RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        ),
        node("fallback", RuleExpression::Literal { value: integer(42) }),
        node(
            "select",
            RuleExpression::Select {
                condition: key("false"),
                when_true: key("lookup"),
                when_false: key("fallback"),
            },
        ),
        node(
            "sum",
            RuleExpression::Add {
                left: key("lookup"),
                right: key("fallback"),
            },
        ),
    ]);
    f.program().effects[0].when = Some(key("false"));
    let c = f.compile();
    let mut s = c.new_scratch();
    assert_eq!(f.evaluate(&c, None, &mut s), EffectDisposition::Inactive);
    assert_eq!(
        f.evaluate(&c, Some(99), &mut s),
        EffectDisposition::Inactive
    );
    f.program().effects[0].when = None;
    let RuleEffectKind::Derive { value, .. } = &mut f.program().effects[0].effect else {
        panic!()
    };
    *value = key("select");
    let c = f.compile();
    assert_eq!(f.evaluate(&c, None, &mut s), applied(integer(42)));
    assert_eq!(f.evaluate(&c, Some(99), &mut s), applied(integer(42)));
    let RuleEffectKind::Derive { value, .. } = &mut f.program().effects[0].effect else {
        panic!()
    };
    *value = key("sum");
    let c = f.compile();
    assert!(
        matches!(f.evaluate(&c,Some(99),&mut s),EffectDisposition::UnsupportedDomain{node:n,..} if n==key("lookup"))
    );
}

#[test]
fn all_scalar_cells_are_typed_and_every_table_is_checked_even_when_unused() {
    let factor: UnitDefId = id("factor");
    for (ty, rows) in [
        (
            ComputedValueType::Boolean,
            vec![
                ParameterValue::Boolean(false),
                ParameterValue::Boolean(true),
            ],
        ),
        (
            ComputedValueType::Option,
            vec![ParameterValue::Option(id("option"))],
        ),
        (
            ComputedValueType::Quantity {
                unit: factor.clone(),
            },
            vec![ParameterValue::Quantity(
                FiniteQuantity::new(0.0, factor.clone()).unwrap(),
            )],
        ),
    ] {
        let f = Fixture::new(ty, rows.clone());
        let c = f.compile();
        assert_eq!(
            f.evaluate(&c, Some(-1), &mut c.new_scratch()),
            applied(rows[0].clone())
        );
        OwnedRulePackage::new(f.input.clone(), &f.schema, RuleStorageLimits::default()).unwrap();
    }
    // Matching scalar kinds must still resolve exact known definition IDs.
    let f = Fixture::new(
        ComputedValueType::Option,
        vec![ParameterValue::Option(id("missing"))],
    );
    f.both_reject();
    let foreign = DefId::new(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        key("option"),
    );
    let f = Fixture::new(
        ComputedValueType::Option,
        vec![ParameterValue::Option(foreign)],
    );
    f.both_reject();
    let mut f = Fixture::ints();
    f.input.tables.push(f.input.tables[0].clone());
    f.both_reject();
    let mut f = Fixture::ints();
    f.input.tables[0].rows.pop();
    f.both_reject();
    let mut f = Fixture::ints();
    f.input.tables[0].rows.clear();
    f.both_reject();
    let mut f = Fixture::ints();
    f.input.tables[0].minimum = bounded(2);
    f.both_reject();
    let mut f = Fixture::ints();
    f.input.tables[0].rows[0] = ParameterValue::Boolean(false);
    f.both_reject();
    let mut f = Fixture::ints();
    let mut unused = f.input.tables[0].clone();
    unused.id = key("unused");
    unused.rows[0] = ParameterValue::Option(id("missing"));
    f.input.tables.push(unused);
    f.both_reject();
    let mut f = Fixture::new(
        ComputedValueType::Quantity {
            unit: factor.clone(),
        },
        vec![ParameterValue::Quantity(
            FiniteQuantity::new(2.0, factor).unwrap(),
        )],
    );
    f.input.tables[0].rows[0] =
        ParameterValue::Quantity(FiniteQuantity::new(2.0, id("other-factor")).unwrap());
    f.both_reject();
    let mut f = Fixture::ints();
    f.program().nodes[1].expression = RuleExpression::LookupIntegerTable {
        table: key("missing"),
        key: key("key"),
    };
    f.both_reject();
    let mut f = Fixture::ints();
    f.program().nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(true),
    };
    assert!(CompiledRulePackage::compile(&f.input, &f.schema, RuleLimits::default()).is_err());
    let mut f = Fixture::ints();
    f.program().nodes[1].expression = RuleExpression::LookupIntegerTable {
        table: key("curve"),
        key: key("lookup"),
    };
    assert!(
        CompiledRulePackage::compile(&f.input, &f.schema, RuleLimits::default())
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
}

#[test]
fn table_rows_and_references_have_independent_storage_and_compile_budgets() {
    let mut f = Fixture::ints();
    let storage =
        OwnedRulePackage::new(f.input.clone(), &f.schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(storage.resources().tables, 1);
    assert_eq!(storage.resources().table_cells, 3);
    assert_eq!(storage.resources().edges, 4); // read, lookup key, table reference, effect
    assert!(
        OwnedRulePackage::new(
            f.input.clone(),
            &f.schema,
            RuleStorageLimits {
                max_edges: 3,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        CompiledRulePackage::compile(
            &f.input,
            &f.schema,
            RuleLimits {
                max_edges: 3,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        OwnedRulePackage::new(
            f.input.clone(),
            &f.schema,
            RuleStorageLimits {
                max_table_cells: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        CompiledRulePackage::compile(
            &f.input,
            &f.schema,
            RuleLimits {
                max_table_cells: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut second = f.input.tables[0].clone();
    second.id = key("second");
    f.input.tables.push(second);
    assert!(
        OwnedRulePackage::new(
            f.input.clone(),
            &f.schema,
            RuleStorageLimits {
                max_tables: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        CompiledRulePackage::compile(
            &f.input,
            &f.schema,
            RuleLimits {
                max_tables: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    f.input.tables[0].minimum = bounded(BoundedInteger::MIN);
    f.input.tables[0].maximum = bounded(BoundedInteger::MAX);
    f.both_reject();
}

#[test]
fn coefficient_edits_keep_definition_ids_and_worker_reuse_independent() {
    let a = Fixture::ints();
    let mut b = Fixture::ints();
    b.input.tables[0].rows[1] = integer(91);
    let ca = a.compile();
    let cb = b.compile();
    assert_ne!(ca.identity(), cb.identity());
    assert_eq!(a.input.definitions, b.input.definitions);
    assert_eq!(a.input.tables[0].id, b.input.tables[0].id);
    let mut s = ca.new_scratch();
    let first = a.evaluate(&ca, Some(0), &mut s);
    assert_eq!(first, applied(integer(0)));
    assert_eq!(b.evaluate(&cb, Some(0), &mut s), applied(integer(91)));
    assert_eq!(a.evaluate(&ca, Some(0), &mut s), first);
    std::thread::scope(|scope| {
        let tasks = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let mut s = ca.new_scratch();
                    for _ in 0..8 {
                        assert_eq!(a.evaluate(&ca, Some(0), &mut s), first);
                        assert_eq!(b.evaluate(&cb, Some(0), &mut s), applied(integer(91)));
                    }
                })
            })
            .collect::<Vec<_>>();
        for task in tasks {
            task.join().unwrap();
        }
    });
    let mut f = Fixture::ints();
    let mut other = f.input.tables[0].clone();
    other.id = key("another");
    f.input.tables.push(other);
    let original = f.compile().identity();
    f.input.tables.reverse();
    assert_eq!(original, f.compile().identity());
}

#[test]
fn table_wire_is_strict_and_roundtrips_without_source_or_runtime_state() {
    let f = Fixture::ints();
    let l = RuleStorageLimits::default();
    let p = OwnedRulePackage::new(f.input.clone(), &f.schema, l).unwrap();
    let bytes = encode_rule_package(&p, l).unwrap();
    let roundtrip = decode_rule_package(&bytes, &f.schema, l).unwrap();
    assert_eq!(p.identity(), roundtrip.identity());
    let mut wire = serde_json::to_value(&f.input).unwrap();
    wire["tables"][0]["default"] = serde_json::json!(0);
    assert!(decode_rule_package(&serde_json::to_vec(&wire).unwrap(), &f.schema, l).is_err());
    let mut wire = serde_json::to_value(&f.input).unwrap();
    wire.as_object_mut().unwrap().remove("tables");
    assert!(decode_rule_package(&serde_json::to_vec(&wire).unwrap(), &f.schema, l).is_err());
    let mut f = Fixture::ints();
    f.input.operations_version = key("owned-domain-operations-v4");
    assert!(CompiledRulePackage::compile(&f.input, &f.schema, RuleLimits::default()).is_err());
}
