//! Directly authored allocation data laws, with no source or evaluator dependency.
use poe_optimizer_core::{
    data::DataIdentity, owned_allocations::*, owned_build::DeclaredSlot, owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::{owned_allocations::*, owned_schema::*};
use serde_json::json;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("allocation-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::new(ns(), key(value))
}
fn integer(value: i64) -> BoundedInteger {
    BoundedInteger::new(value).unwrap()
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn declarations() -> DeclaredSlots {
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
fn gap(code: &str) -> SchemaGap {
    SchemaGap {
        subject: SchemaSubject::Definition(DefinitionAddress::PointPool(id("ordinary"))),
        facet: SchemaFacet::GameRules,
        code: key(code),
    }
}
fn parameter() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::PassiveNode(id("a")),
        slot: id("node-parameter"),
    }
}
fn schema_input() -> SchemaPackageInput {
    let mut definitions = vec![];
    for (name, scope) in [
        ("ordinary", PointPoolScope::Either),
        ("ascendancy", PointPoolScope::Shared),
        ("scoped", PointPoolScope::PerLoadout),
    ] {
        definitions.push(DefinitionDescriptor::PointPool(known(
            id(name),
            PointPoolSchema { scope },
        )));
    }
    for (name, pools) in [
        ("a", vec![id("ordinary"), id("ascendancy")]),
        ("b", vec![id("ordinary")]),
        ("implicit-root", vec![]),
    ] {
        let mut d = declarations();
        if name == "a" {
            d.parameters.members.push(parameter());
        }
        definitions.push(DefinitionDescriptor::PassiveNode(known(
            id(name),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(pools),
                adjacent: DeclaredSet::complete(vec![]),
                declarations: d,
            },
        )));
    }
    definitions.push(DefinitionDescriptor::Unit(known(
        id("points-unit"),
        UnitSchema {
            dimension: UnitDimension::Rating,
        },
    )));
    for (name, value, target) in [
        (
            "capacity",
            ComputedValueType::Integer,
            RuleEntityKind::Actor,
        ),
        (
            "other-capacity",
            ComputedValueType::Integer,
            RuleEntityKind::Actor,
        ),
        (
            "action-capacity",
            ComputedValueType::Integer,
            RuleEntityKind::Action,
        ),
        (
            "quantity-capacity",
            ComputedValueType::Quantity {
                unit: id("points-unit"),
            },
            RuleEntityKind::Actor,
        ),
        (
            "boolean-capacity",
            ComputedValueType::Boolean,
            RuleEntityKind::Actor,
        ),
        (
            "option-capacity",
            ComputedValueType::Option,
            RuleEntityKind::Actor,
        ),
    ] {
        definitions.push(DefinitionDescriptor::Stat(known(
            id(name),
            StatSchema {
                value,
                targets: vec![target],
            },
        )));
    }
    definitions.push(DefinitionDescriptor::Stat(DefinitionEntry {
        id: id("unmapped-capacity"),
        schema: SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(DefinitionAddress::Stat(id(
                    "unmapped-capacity",
                ))),
                facet: SchemaFacet::InputSchema,
                code: key("unconverted-stat"),
            }],
        },
    }));
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("schema"),
        semantics_version: key("test-v1"),
        definitions,
        slots: vec![SlotDescriptor::Parameter(known(
            parameter(),
            ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: integer(0),
                    maximum: integer(10),
                }),
                presence: SlotPresence::OptionalOnce,
                sites: vec![],
            },
        ))],
    }
}
fn schema(raw: SchemaPackageInput) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}
fn node<'a>(raw: &'a mut SchemaPackageInput, name: &str) -> &'a mut PassiveNodeSchema {
    raw.definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::PassiveNode(e) if e.id == id(name) => match &mut e.schema {
                SchemaState::Known(s) => Some(s),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}
fn input(s: &OwnedDefinitionSchemaPackage) -> AllocationRulesInput {
    AllocationRulesInput {
        schema_version: OWNED_ALLOCATION_RULES_VERSION,
        namespace: ns(),
        release: key("costs-v1"),
        definitions: s.identity().clone(),
        costs: vec![
            AllocationCost {
                node: id("a"),
                pool: id("ordinary"),
                points: integer(1),
            },
            AllocationCost {
                node: id("a"),
                pool: id("ascendancy"),
                points: integer(0),
            },
            AllocationCost {
                node: id("b"),
                pool: id("ordinary"),
                points: integer(2),
            },
        ],
        budgets: DeclaredSet::partial(
            vec![
                AllocationBudget {
                    id: key("total"),
                    pools: vec![id("ordinary"), id("ascendancy")],
                    usage: AllocationBudgetUsage::Total,
                    capacity: SchemaState::Known(AllocationCapacity::Uniform {
                        stat: id("capacity"),
                    }),
                },
                AllocationBudget {
                    id: key("each"),
                    pools: vec![id("ordinary")],
                    usage: AllocationBudgetUsage::EachScope {
                        include_shared: false,
                    },
                    capacity: SchemaState::Known(AllocationCapacity::PerScope {
                        stat: id("capacity"),
                    }),
                },
                AllocationBudget {
                    id: key("unknown"),
                    pools: vec![id("ascendancy")],
                    usage: AllocationBudgetUsage::SharedPlusMaximumScoped,
                    capacity: SchemaState::Unmapped {
                        gaps: vec![gap("progression"), gap("capacity-producers")],
                    },
                },
            ],
            vec![gap("other-budgets"), gap("conditional-budgets")],
        ),
    }
}
fn package(i: AllocationRulesInput, s: &OwnedDefinitionSchemaPackage) -> OwnedAllocationRules {
    OwnedAllocationRules::new(i, s, AllocationRuleLimits::default()).unwrap()
}
fn error(i: AllocationRulesInput, s: &OwnedDefinitionSchemaPackage) -> String {
    OwnedAllocationRules::new(i, s, AllocationRuleLimits::default())
        .unwrap_err()
        .to_string()
}
fn budget<'a>(i: &'a mut AllocationRulesInput, name: &str) -> &'a mut AllocationBudget {
    i.budgets
        .members
        .iter_mut()
        .find(|v| v.id == key(name))
        .unwrap()
}

#[test]
fn canonical_order_and_roundtrip_preserve_explicit_zero_missing_costs_and_partial_evidence() {
    let s = schema(schema_input());
    let a = package(input(&s), &s);
    let mut reverse = input(&s);
    reverse.costs.reverse();
    reverse.budgets.members.reverse();
    if let SchemaClosure::Partial { gaps } = &mut reverse.budgets.closure {
        gaps.reverse();
    }
    for b in &mut reverse.budgets.members {
        b.pools.reverse();
        if let SchemaState::Unmapped { gaps } = &mut b.capacity {
            gaps.reverse();
        }
    }
    let b = package(reverse, &s);
    assert_eq!(a.input(), b.input());
    assert_eq!(a.identity(), b.identity());
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    assert_eq!(a.resources(), b.resources());
    assert_eq!(
        a.cost_for(&id("a"), &id("ascendancy"))
            .unwrap()
            .points
            .get(),
        0
    );
    assert_eq!(
        a.cost_for(&id("a"), &id("ordinary")).unwrap().points.get(),
        1
    );
    assert!(a.cost_for(&id("b"), &id("ascendancy")).is_none());
    assert!(a.cost_for(&id("implicit-root"), &id("ordinary")).is_none());
    assert!(!a.input().budgets.is_complete());
    assert!(matches!(
        a.input()
            .budgets
            .members
            .iter()
            .find(|b| b.id == key("unknown"))
            .unwrap()
            .capacity,
        SchemaState::Unmapped { .. }
    ));
    let bytes = encode_allocation_rules(&a, AllocationRuleLimits::default()).unwrap();
    assert_eq!(bytes, a.canonical_bytes());
    let restored = decode_allocation_rules(&bytes, &s, AllocationRuleLimits::default()).unwrap();
    assert_eq!(restored.input(), a.input());
    assert_eq!(restored.identity(), a.identity());
    let mut changed = input(&s);
    changed.costs[0].points = integer(2);
    assert_ne!(package(changed, &s).identity(), a.identity());
    let mut empty = input(&s);
    empty.costs.clear();
    empty.budgets = DeclaredSet::complete(vec![]);
    let empty = package(empty, &s);
    assert!(empty.cost_for(&id("a"), &id("ordinary")).is_none());
    assert!(empty.input().budgets.is_complete());
}
#[test]
fn duplicate_cost_keys_budget_ids_and_pool_members_do_not_select_a_winner() {
    let s = schema(schema_input());
    for points in [1, 99] {
        let mut i = input(&s);
        let mut row = i.costs[0].clone();
        row.points = integer(points);
        i.costs.push(row);
        assert!(error(i, &s).contains("duplicate node/pool"));
    }
    let mut i = input(&s);
    i.budgets.members.push(i.budgets.members[0].clone());
    assert!(error(i, &s).contains("duplicate budget id"));
    let mut i = input(&s);
    budget(&mut i, "total").pools.push(id("ordinary"));
    assert!(error(i, &s).contains("duplicate budget pool"));
    let mut i = input(&s);
    budget(&mut i, "total").pools.clear();
    assert!(error(i, &s).contains("no pools"));
}
#[test]
fn negative_costs_and_undeclared_pools_are_rejected_without_inferred_root_costs() {
    let s = schema(schema_input());
    let mut i = input(&s);
    i.costs[0].points = integer(-1);
    assert!(error(i, &s).contains("negative point cost"));
    let mut i = input(&s);
    i.costs[0].pool = id("scoped");
    assert!(error(i, &s).contains("pool not declared"));
    let mut i = input(&s);
    i.costs[0].node = id("implicit-root");
    i.costs[0].points = integer(0);
    assert!(error(i, &s).contains("pool not declared"));
    let mut i = input(&s);
    i.costs[0].points = integer(BoundedInteger::MAX);
    package(i, &s);
    // An explicit member remains usable under Partial; Partial never admits a
    // different pool merely because its membership is unknown.
    let mut raw = schema_input();
    node(&mut raw, "a").pools.closure = SchemaClosure::Partial {
        gaps: vec![gap("pool-coverage")],
    };
    let partial = schema(raw);
    package(input(&partial), &partial);
    let mut i = input(&partial);
    i.costs[0].pool = id("scoped");
    assert!(error(i, &partial).contains("pool not declared"));
}
#[test]
fn capacity_declares_exact_integer_actor_stat_and_per_scope_requires_each_scope() {
    let s = schema(schema_input());
    for name in ["quantity-capacity", "boolean-capacity", "option-capacity"] {
        let mut i = input(&s);
        budget(&mut i, "total").capacity =
            SchemaState::Known(AllocationCapacity::Uniform { stat: id(name) });
        assert!(error(i, &s).contains("exact Integer"), "{name}");
    }
    let mut i = input(&s);
    budget(&mut i, "total").capacity = SchemaState::Known(AllocationCapacity::Uniform {
        stat: id("action-capacity"),
    });
    assert!(error(i, &s).contains("excludes Actor"));
    for usage in [
        AllocationBudgetUsage::Total,
        AllocationBudgetUsage::SharedPlusMaximumScoped,
    ] {
        let mut i = input(&s);
        budget(&mut i, "each").usage = usage;
        assert!(error(i, &s).contains("requires EachScope"));
    }
    for include_shared in [false, true] {
        let mut i = input(&s);
        let b = budget(&mut i, "each");
        b.usage = AllocationBudgetUsage::EachScope { include_shared };
        b.capacity = SchemaState::Known(AllocationCapacity::Uniform {
            stat: id("capacity"),
        });
        package(i, &s);
    }
    // The storage boundary permits overlapping constraints and all known pool
    // scopes; it does not infer uniformity, acquired values or active loadouts.
    let mut i = input(&s);
    budget(&mut i, "total").pools.push(id("scoped"));
    package(i, &s);
}
#[test]
fn missing_unmapped_and_foreign_node_pool_and_capacity_schemas_remain_errors() {
    let s = schema(schema_input());
    for which in ["node", "cost-pool", "budget-pool", "capacity"] {
        let mut i = input(&s);
        match which {
            "node" => i.costs[0].node = id("missing"),
            "cost-pool" => i.costs[0].pool = id("missing"),
            "budget-pool" => budget(&mut i, "total").pools = vec![id("missing")],
            _ => {
                budget(&mut i, "total").capacity = SchemaState::Known(AllocationCapacity::Uniform {
                    stat: id("missing"),
                })
            }
        }
        assert!(error(i, &s).contains("missing schema"), "{which}");
        let mut i = input(&s);
        let foreign = GameVersionNamespace::new("foreign", "v1").unwrap();
        match which {
            "node" => i.costs[0].node = DefId::new(foreign, key("a")),
            "cost-pool" => i.costs[0].pool = DefId::new(foreign, key("ordinary")),
            "budget-pool" => {
                budget(&mut i, "total").pools = vec![DefId::new(foreign, key("ordinary"))]
            }
            _ => {
                budget(&mut i, "total").capacity = SchemaState::Known(AllocationCapacity::Uniform {
                    stat: DefId::new(foreign, key("capacity")),
                })
            }
        }
        assert!(error(i, &s).contains("foreign namespace"), "{which}");
    }
    let mut i = input(&s);
    budget(&mut i, "total").capacity = SchemaState::Known(AllocationCapacity::Uniform {
        stat: id("unmapped-capacity"),
    });
    assert!(error(i, &s).contains("unmapped schema"));
    for which in ["node", "pool"] {
        let mut raw = schema_input();
        for row in &mut raw.definitions {
            let subject = SchemaSubject::Definition(row.address());
            let gaps = vec![SchemaGap {
                subject,
                facet: SchemaFacet::InputSchema,
                code: key("missing-input-schema"),
            }];
            match row {
                DefinitionDescriptor::PassiveNode(e) if which == "node" && e.id == id("a") => {
                    e.schema = SchemaState::Unmapped { gaps }
                }
                DefinitionDescriptor::PointPool(e) if which == "pool" && e.id == id("ordinary") => {
                    e.schema = SchemaState::Unmapped { gaps }
                }
                _ => {}
            }
        }
        let partial = schema(raw);
        assert!(error(input(&partial), &partial).contains("unmapped schema"));
    }
}
#[test]
fn every_partial_or_unmapped_state_requires_distinct_owned_gap_evidence() {
    let s = schema(schema_input());
    for capacity in [false, true] {
        for gaps in [vec![], vec![gap("same"), gap("same")]] {
            let mut i = input(&s);
            if capacity {
                budget(&mut i, "unknown").capacity = SchemaState::Unmapped { gaps };
            } else {
                i.budgets.closure = SchemaClosure::Partial { gaps };
            }
            assert!(OwnedAllocationRules::new(i, &s, AllocationRuleLimits::default()).is_err());
        }
    }
    for subject in [
        SchemaSubject::Definition(DefinitionAddress::Stat(id("missing"))),
        SchemaSubject::Slot(SlotAddress::Parameter(DeclaredSlot {
            declaration: SlotOwnerDefId::PassiveNode(id("a")),
            slot: id("missing"),
        })),
        SchemaSubject::Definition(DefinitionAddress::PointPool(DefId::new(
            GameVersionNamespace::new("foreign", "v1").unwrap(),
            key("ordinary"),
        ))),
    ] {
        let mut i = input(&s);
        let mut g = gap("bad");
        g.subject = subject;
        i.budgets.closure = SchemaClosure::Partial { gaps: vec![g] };
        assert!(error(i, &s).contains("gap subject"));
    }
    // Known slot identities and registered Unmapped identities can be honest
    // unresolved subjects. Neither is promoted to an available capacity.
    let mut i = input(&s);
    let mut g = gap("slot-rule");
    g.subject = SchemaSubject::Slot(SlotAddress::Parameter(parameter()));
    let mut other = gap("stat-rule");
    other.subject = SchemaSubject::Definition(DefinitionAddress::Stat(id("unmapped-capacity")));
    budget(&mut i, "unknown").capacity = SchemaState::Unmapped {
        gaps: vec![g, other],
    };
    package(i, &s);
}
#[test]
fn strict_wire_rejects_unknown_duplicate_missing_fields_wrong_domains_and_versions() {
    let s = schema(schema_input());
    let p = package(input(&s), &s);
    let original: serde_json::Value = serde_json::from_slice(p.canonical_bytes()).unwrap();
    let mut cases = vec![];
    let mut v = original.clone();
    v["source"] = json!("not-owned");
    cases.push(v);
    let mut v = original.clone();
    v["costs"][0]["extra"] = json!(1);
    cases.push(v);
    let mut v = original.clone();
    v["costs"][0]["node"]["kind"] = json!("point_pool");
    cases.push(v);
    let mut v = original.clone();
    v["costs"][0]["points"] = json!(1.5);
    cases.push(v);
    let mut v = original.clone();
    v["costs"][0]["points"] = json!(BoundedInteger::MAX + 1);
    cases.push(v);
    let mut v = original.clone();
    v["costs"][0].as_object_mut().unwrap().remove("points");
    cases.push(v);
    let mut v = original.clone();
    v["budgets"]["members"][0]
        .as_object_mut()
        .unwrap()
        .remove("capacity");
    cases.push(v);
    let mut v = original.clone();
    v["budgets"]["members"][0]["id"] = json!("");
    cases.push(v);
    let mut v = original.clone();
    v["budgets"]["members"][0]["usage"]["unexpected"] = json!(true);
    cases.push(v);
    let mut v = original.clone();
    v["schema_version"] = json!(OWNED_ALLOCATION_RULES_VERSION + 1);
    cases.push(v);
    let mut v = original;
    v.as_object_mut().unwrap().remove("budgets");
    cases.push(v);
    for v in cases {
        assert!(
            decode_allocation_rules(
                &serde_json::to_vec(&v).unwrap(),
                &s,
                AllocationRuleLimits::default()
            )
            .is_err(),
            "{v}"
        );
    }
    let text = String::from_utf8(p.canonical_bytes().to_vec()).unwrap();
    for (old, new) in [
        (
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
        ),
        ("\"points\":1", "\"points\":1,\"points\":1"),
        (
            "\"include_shared\":false",
            "\"include_shared\":false,\"include_shared\":false",
        ),
    ] {
        let duplicate = text.replacen(old, new, 1);
        assert_ne!(duplicate, text);
        assert!(
            decode_allocation_rules(duplicate.as_bytes(), &s, AllocationRuleLimits::default())
                .is_err()
        );
    }
    assert!(
        decode_allocation_rules(
            b"{\"costs\":[{\"points\":NaN}]}",
            &s,
            AllocationRuleLimits::default()
        )
        .is_err()
    );
}
#[test]
fn exact_schema_binding_and_all_limits_apply_to_new_decode_encode_and_revalidation() {
    let s = schema(schema_input());
    let original = input(&s);
    let p = package(original.clone(), &s);
    p.verify_bindings(&s).unwrap();
    let mut raw = schema_input();
    raw.release = key("changed-schema");
    let changed = schema(raw);
    assert!(p.verify_bindings(&changed).is_err());
    assert!(
        OwnedAllocationRules::new(original.clone(), &changed, AllocationRuleLimits::default())
            .is_err()
    );
    let mut foreign = original.clone();
    foreign.namespace = GameVersionNamespace::new("foreign", "v1").unwrap();
    assert!(OwnedAllocationRules::new(foreign, &s, AllocationRuleLimits::default()).is_err());
    let u = p.resources();
    let n = p.canonical_bytes().len();
    assert_eq!(
        (u.costs, u.budgets, u.pool_memberships, u.gaps),
        (3, 3, 4, 4)
    );
    let exact = AllocationRuleLimits {
        max_costs: u.costs,
        max_budgets: u.budgets,
        max_pool_memberships: u.pool_memberships,
        max_gaps: u.gaps,
        max_schema_work: u.schema_work,
        max_wire_bytes: n,
    };
    p.validate_limits(exact).unwrap();
    assert_eq!(
        decode_allocation_rules(p.canonical_bytes(), &s, exact)
            .unwrap()
            .identity(),
        p.identity()
    );
    for limits in [
        AllocationRuleLimits {
            max_costs: 2,
            ..exact
        },
        AllocationRuleLimits {
            max_budgets: 2,
            ..exact
        },
        AllocationRuleLimits {
            max_pool_memberships: 3,
            ..exact
        },
        AllocationRuleLimits {
            max_gaps: 3,
            ..exact
        },
        AllocationRuleLimits {
            max_schema_work: u.schema_work - 1,
            ..exact
        },
        AllocationRuleLimits {
            max_wire_bytes: n - 1,
            ..exact
        },
    ] {
        assert!(OwnedAllocationRules::new(original.clone(), &s, limits).is_err());
        assert!(decode_allocation_rules(p.canonical_bytes(), &s, limits).is_err());
        assert!(encode_allocation_rules(&p, limits).is_err());
        assert!(p.validate_limits(limits).is_err());
    }
    for value in [0, usize::MAX] {
        for limits in [
            AllocationRuleLimits {
                max_costs: value,
                ..exact
            },
            AllocationRuleLimits {
                max_budgets: value,
                ..exact
            },
            AllocationRuleLimits {
                max_pool_memberships: value,
                ..exact
            },
            AllocationRuleLimits {
                max_gaps: value,
                ..exact
            },
            AllocationRuleLimits {
                max_schema_work: value,
                ..exact
            },
            AllocationRuleLimits {
                max_wire_bytes: value,
                ..exact
            },
        ] {
            assert!(limits.validate().is_err());
        }
    }
}
struct NoLookup<'a>(&'a OwnedDefinitionSchemaPackage);
impl DefinitionSchemaIndex for NoLookup<'_> {
    fn identity(&self) -> &DataIdentity {
        self.0.identity()
    }
    fn namespace(&self) -> &GameVersionNamespace {
        self.0.namespace()
    }
    fn lookup_definition(&self, _: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        panic!("preflight must fail before schema lookup")
    }
    fn lookup_slot(&self, _: &SlotAddress) -> Option<&SlotDescriptor> {
        panic!("preflight must fail before schema lookup")
    }
}
#[test]
fn aggregate_resource_rejections_precede_schema_lookups_and_zero_work_is_invalid() {
    let s = schema(schema_input());
    let i = input(&s);
    for limits in [
        AllocationRuleLimits {
            max_costs: 2,
            ..Default::default()
        },
        AllocationRuleLimits {
            max_budgets: 2,
            ..Default::default()
        },
        AllocationRuleLimits {
            max_pool_memberships: 3,
            ..Default::default()
        },
        AllocationRuleLimits {
            max_gaps: 3,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            OwnedAllocationRules::new(i.clone(), &NoLookup(&s), limits),
            Err(AllocationRuleError::Limit(_))
        ));
    }
}
#[test]
fn inconsistent_index_cannot_substitute_another_definition_or_gap_identity() {
    struct Inconsistent<'a> {
        schema: &'a OwnedDefinitionSchemaPackage,
        requested: DefinitionAddress,
        replacement: DefinitionAddress,
    }
    impl DefinitionSchemaIndex for Inconsistent<'_> {
        fn identity(&self) -> &DataIdentity {
            self.schema.identity()
        }
        fn namespace(&self) -> &GameVersionNamespace {
            self.schema.namespace()
        }
        fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
            self.schema
                .lookup_definition(if address == &self.requested {
                    &self.replacement
                } else {
                    address
                })
        }
        fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
            self.schema.lookup_slot(address)
        }
    }
    let s = schema(schema_input());
    for (requested, replacement) in [
        (
            DefinitionAddress::PassiveNode(id("a")),
            DefinitionAddress::PassiveNode(id("b")),
        ),
        (
            DefinitionAddress::Stat(id("capacity")),
            DefinitionAddress::Stat(id("other-capacity")),
        ),
        (
            DefinitionAddress::PointPool(id("ordinary")),
            DefinitionAddress::PointPool(id("ascendancy")),
        ),
    ] {
        let index = Inconsistent {
            schema: &s,
            requested,
            replacement,
        };
        assert!(
            OwnedAllocationRules::new(input(&s), &index, AllocationRuleLimits::default()).is_err()
        );
    }
}

#[test]
fn shipped_cost_component_loads_against_the_owned_schema_without_source_tools() {
    let schema = decode_schema_package(
        include_bytes!("../../../data/owned/poe2/3887ae68/current/schema.json"),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let rules = decode_allocation_rules(
        include_bytes!("../../../data/owned/poe2/3887ae68/allocations/rules.json"),
        &schema,
        AllocationRuleLimits::default(),
    )
    .unwrap();
    assert_eq!(rules.input().costs.len(), 4503);
    assert_eq!(
        rules
            .input()
            .costs
            .iter()
            .filter(|row| row.points.get() == 0)
            .count(),
        3
    );
    assert_eq!(rules.input().budgets.members.len(), 3);
    assert!(matches!(
        rules.input().budgets.closure,
        SchemaClosure::Partial { .. }
    ));
    assert!(
        rules
            .input()
            .budgets
            .members
            .iter()
            .all(|b| matches!(b.capacity, SchemaState::Unmapped { .. }))
    );
    assert_eq!(&rules.input().definitions, schema.identity());
}
