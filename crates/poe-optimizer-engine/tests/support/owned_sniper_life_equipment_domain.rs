//! Actual guarded producers and equipment subtotal on the finite joined graph.
//! Supplied rule facts below test numeric contracts, not new source parity.
use super::*;
use poe_optimizer_core::{owned_readiness::*, owned_schema::*, owned_stages::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch,
};

const BOUND: f64 = 137_438_953_472.;
const DIRECT: &str = "contribute-player-flat-life";
const COPY: &str = "amulet-copy-flat-life";
const PROBE: &str = "fixture.life-equipment-subtotal";
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn packet(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/life-equipment-domain")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Modifier(def(0x3100)))
}
fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn known(n: f64) -> EffectValue {
    EffectValue::Known {
        value: quantity(n, 0x3119),
    }
}
fn probe_stat() -> StatDefId {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        PROBE,
    )
    .unwrap()
}
fn world() -> ReplayInput {
    let mut i = load();
    let mut expected: DefinitionRules =
        serde_json::from_value(packet("dependencies.json")["owner"].clone()).unwrap();
    assert_eq!(expected.owner, owner());
    for program in serde_json::from_value::<Vec<RuleProgram>>(packet("programs.json")).unwrap() {
        let matches: Vec<_> = expected
            .programs
            .members
            .iter_mut()
            .filter(|p| p.id == program.id)
            .collect();
        assert_eq!(matches.len(), 1);
        *matches.into_iter().next().unwrap() = program;
    }
    let actual = i.rules.owners.iter().find(|o| o.owner == owner()).unwrap();
    // The fixture's finite closure is not production source-completeness proof.
    // Published guards are already present; authenticate instead of reinstalling.
    assert!(
        actual.programs.members == expected.programs.members,
        "exact published equipment Life owner programs"
    );
    let definitions: Value = serde_json::from_slice(
        &fs::read(
            root().join("data/owned/poe2/3887ae68/life-contribution-queries/dependencies.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for descriptor in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(definitions["definitions"].clone())
            .unwrap()
    {
        if matches!(descriptor, DefinitionDescriptor::EquipmentSlot(_))
            && !i
                .schema
                .definitions
                .iter()
                .any(|d| d.address() == descriptor.address())
        {
            i.rules.owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(descriptor.address()),
                programs: DeclaredSet::complete(vec![]),
            });
            i.schema.definitions.push(descriptor);
        }
    }
    let query: ContributionQuery = serde_json::from_value(packet("query.json")).unwrap();
    assert!(
        query
            .groups
            .iter()
            .find(|g| g.id == key("equipment"))
            .unwrap()
            .members
            .is_complete()
    );
    let query_id = query.id.clone();
    assert!(
        !i.rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .members
            .iter()
            .any(|q| q.id == query_id)
    );
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .push(query);
    let stat = probe_stat();
    let subject = SchemaSubject::Definition(DefinitionAddress::Stat(stat.clone()));
    let ty = ComputedValueType::Quantity { unit: def(0x3119) };
    i.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: stat.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ty.clone(),
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    i.rules.owners.push(DefinitionRules {
        owner: subject.clone(),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key(PROBE),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("subtotal"),
                value_type: ty,
                source: RuleReadSource::ContributionQuery {
                    entity: RuleEntity::Current,
                    query: query_id,
                    group: key("equipment"),
                },
            }],
            nodes: vec![RuleNode {
                id: key("subtotal"),
                expression: RuleExpression::Read {
                    input: key("subtotal"),
                },
            }],
            effects: vec![RuleEffect {
                id: key("observe"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: stat.clone(),
                    value: key("subtotal"),
                },
            }],
        }]),
    });
    i.rules.receivers.members.push(StatReceiver {
        id: key(PROBE),
        stat,
        program: key(PROBE),
        targets: vec![StatReceiverTarget::Player],
    });
    i.stages.stages.extend([
        EvaluationStage {
            id: key("life-equipment-ready"),
            predecessors: i.stages.stages.iter().map(|s| s.id.clone()).collect(),
        },
        EvaluationStage {
            id: key("life-equipment-probe"),
            predecessors: vec![key("life-equipment-ready")],
        },
    ]);
    i.stages.programs.members.push(StagedRuleProgram {
        owner: subject.clone(),
        program: key(PROBE),
        stage: key("life-equipment-probe"),
    });
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .push(ReadinessProgram {
            owner: subject,
            program: key(PROBE),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    i.stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: def(0x311a),
            contribution: ContributionKind::Add,
        },
        stage: key("life-equipment-ready"),
    });
    i.rebind_test_edit().unwrap();
    i
}
fn subtotal(report: &SupportEffectsReport) -> &EffectValue {
    &effects(report)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: probe_stat(),
                }
        })
        .unwrap()
        .value
}
fn run_plan(i: &ReplayInput) -> SupportEffectsReport {
    let p = i.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn component() -> (
    ReplayInput,
    OwnedDefinitionSchemaPackage,
    CompiledRulePackage,
) {
    let i = world();
    let schema = OwnedDefinitionSchemaPackage::new(i.schema.clone(), Default::default()).unwrap();
    let c = CompiledRulePackage::compile(&i.rules, &schema, Default::default()).unwrap();
    (i, schema, c)
}
fn evaluate(
    c: &CompiledRulePackage,
    schema: &OwnedDefinitionSchemaPackage,
    id: &str,
    facts: &[(&str, ParameterValue)],
    scratch: &mut RuleScratch,
) -> EffectDisposition {
    let facts: Vec<_> = facts
        .iter()
        .map(|(s, v)| RuleFact {
            read: key(s),
            value: v.clone(),
        })
        .collect();
    let result = c
        .evaluate(&owner(), &key(id), &facts, schema, scratch)
        .unwrap();
    assert_eq!(result.effects.len(), 1);
    result.effects[0].disposition.clone()
}
fn copy_facts(n: f64, percent: f64, unscalable: bool) -> Vec<(&'static str, ParameterValue)> {
    vec![
        ("eligible", ParameterValue::Boolean(true)),
        ("effective", quantity(n, 0x295a)),
        ("pre-amulet-percent", quantity(percent, 2)),
        ("unscalable", ParameterValue::Boolean(unscalable)),
    ]
}
fn unsupported(node: &str, n: f64, unit: u64) -> EffectDisposition {
    EffectDisposition::UnsupportedDomain {
        node: key(node),
        domain: UnsupportedDomainCause::NumericRange {
            value: quantity(n, unit),
            domain: Box::new(NumericDomain::Quantity(QuantityRange {
                minimum: FiniteQuantity::new(-BOUND, def(unit)).unwrap(),
                maximum: FiniteQuantity::new(BOUND, def(unit)).unwrap(),
            })),
        },
    }
}

#[test]
fn actual_direct_and_copy_guards_preserve_inclusive_values_and_exact_causes() {
    let (_, schema, c) = component();
    let mut scratch = c.new_scratch();
    for n in [-BOUND, -1., -0., 0., 1., BOUND] {
        for id in [DIRECT, COPY] {
            let facts = if id == DIRECT {
                vec![
                    ("direct-applicability", ParameterValue::Boolean(true)),
                    ("effective", quantity(n, 0x295a)),
                ]
            } else {
                copy_facts(n, 100., false)
            };
            assert_eq!(
                evaluate(&c, &schema, id, &facts, &mut scratch),
                EffectDisposition::Applied {
                    value: quantity(n, 0x3119)
                }
            );
        }
    }
    for n in [(-BOUND).next_down(), BOUND.next_up()] {
        assert_eq!(
            evaluate(
                &c,
                &schema,
                DIRECT,
                &[
                    ("direct-applicability", ParameterValue::Boolean(true)),
                    ("effective", quantity(n, 0x295a))
                ],
                &mut scratch
            ),
            unsupported("bounded-life-points", n, 0x3119)
        );
        assert_eq!(
            evaluate(&c, &schema, COPY, &copy_facts(n, 100., false), &mut scratch),
            unsupported("effective", n, 0x295a)
        );
    }
    for (n, percent, expected) in [(BOUND, 200., BOUND * 2.), (-BOUND, 200., -BOUND * 2.)] {
        assert_eq!(
            evaluate(
                &c,
                &schema,
                COPY,
                &copy_facts(n, percent, false),
                &mut scratch
            ),
            unsupported("bounded-life-points", expected, 0x3119)
        );
    }
    // The prior formatter discrepancy plus a tiny copy factor fits the output
    // bound. It is a derived control, not an additional source observation.
    let large = 1_000_997_998_001_001.;
    assert_eq!(
        evaluate(
            &c,
            &schema,
            COPY,
            &copy_facts(large, 0.009_990_029_970_159_346, false),
            &mut scratch
        ),
        unsupported("effective", large, 0x295a)
    );
    assert_eq!(
        evaluate(
            &c,
            &schema,
            COPY,
            &copy_facts(large, 0., true),
            &mut scratch
        ),
        unsupported("effective", large, 0x295a)
    );
}

#[test]
fn inactive_missing_and_zero_do_not_conflate_availability() {
    let (_, schema, c) = component();
    let mut scratch = c.new_scratch();
    for (id, activation) in [(DIRECT, "direct-applicability"), (COPY, "eligible")] {
        assert_eq!(
            evaluate(
                &c,
                &schema,
                id,
                &[(activation, ParameterValue::Boolean(false))],
                &mut scratch
            ),
            EffectDisposition::Inactive
        );
        let mut missing = if id == COPY {
            copy_facts(9., 100., false)
        } else {
            vec![(activation, ParameterValue::Boolean(true))]
        };
        missing.retain(|(read, _)| *read != "effective");
        assert_eq!(
            evaluate(&c, &schema, id, &missing, &mut scratch),
            EffectDisposition::Unresolved {
                input: key("effective")
            }
        );
    }
    assert_eq!(
        evaluate(&c, &schema, COPY, &copy_facts(9., 0., false), &mut scratch),
        EffectDisposition::Applied {
            value: quantity(0., 0x3119)
        }
    );
    assert_eq!(
        evaluate(
            &c,
            &schema,
            COPY,
            &copy_facts(-9., 50., false),
            &mut scratch
        ),
        EffectDisposition::Applied {
            value: quantity(-4., 0x3119)
        }
    );
}

#[test]
fn exact_current_effective_writer_produces_integral_inputs_without_replacement() {
    let (i, schema, c) = component();
    let old: DefinitionRules =
        serde_json::from_value(packet("dependencies.json")["owner"].clone()).unwrap();
    let actual = i.rules.owners.iter().find(|o| o.owner == owner()).unwrap();
    assert_eq!(
        actual
            .programs
            .members
            .iter()
            .find(|p| p.id == key("effective-amount")),
        old.programs
            .members
            .iter()
            .find(|p| p.id == key("effective-amount"))
    );
    let mut scratch = c.new_scratch();
    for (raw, corruption, magnitude) in [
        (0., 1., 1.),
        (0.49, 1., 1.),
        (0.5, 1., 1.),
        (17., 1.5, 1.25),
        (1_000_000., 1_000_000., 1.),
    ] {
        let value = evaluate(
            &c,
            &schema,
            "effective-amount",
            &[
                ("component", quantity(raw, 0x295a)),
                ("corruption-factor", quantity(corruption, 1)),
                ("magnitude-factor", quantity(magnitude, 1)),
            ],
            &mut scratch,
        );
        let EffectDisposition::Applied {
            value: ParameterValue::Quantity(effective),
        } = value
        else {
            panic!("effective writer unavailable")
        };
        assert_eq!(effective.value().fract(), 0.);
        let direct = evaluate(
            &c,
            &schema,
            DIRECT,
            &[
                ("direct-applicability", ParameterValue::Boolean(true)),
                ("effective", ParameterValue::Quantity(effective.clone())),
            ],
            &mut scratch,
        );
        if effective.value() <= BOUND {
            assert_eq!(
                direct,
                EffectDisposition::Applied {
                    value: quantity(effective.value(), 0x3119)
                }
            );
        } else {
            assert_eq!(
                direct,
                unsupported("bounded-life-points", effective.value(), 0x3119)
            );
        }
    }
}

fn change_ring(i: &mut ReplayInput, amount: f64, corruption: f64) {
    let item = i
        .build
        .items
        .iter_mut()
        .find(|i| i.template == def(0x09dc))
        .unwrap();
    let modifier = item
        .modifiers
        .iter_mut()
        .find(|m| m.definition == def(0x3100))
        .unwrap();
    for (slot, value) in [
        (0x3101, quantity(amount, 0x295a)),
        (0x3117, quantity(corruption, 1)),
    ] {
        modifier
            .rolls
            .iter_mut()
            .find(|r| r.slot.slot == def(slot))
            .unwrap()
            .value = value;
    }
}

#[test]
fn authored_equipment_query_reduces_exact_item_occurrences_and_propagates_range_failure() {
    let original = world();
    let baseline = run_plan(&original);
    assert_eq!(subtotal(&baseline), &known(63.));
    let mut changed = original.clone();
    change_ring(&mut changed, 14., 1.);
    assert_eq!(subtotal(&run_plan(&changed)), &known(71.));
    let mut zero = original.clone();
    change_ring(&mut zero, 0., 1.);
    assert_eq!(subtotal(&run_plan(&zero)), &known(43.));
    let mut outside = original.clone();
    change_ring(&mut outside, 1_000_000., 1_000_000.);
    let report = run_plan(&outside);
    assert!(
        matches!(subtotal(&report), EffectValue::UnsupportedDomain { node, domain:UnsupportedDomainCause::NumericRange { value:ParameterValue::Quantity(value),.. }, .. } if *node==key("bounded-life-points")&&value.value()==1_000_000_000_000.)
    );
    let mut partial = original.clone();
    let closure: DefinitionRules =
        serde_json::from_value(packet("dependencies.json")["owner"].clone()).unwrap();
    assert!(!closure.programs.is_complete());
    partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner())
        .unwrap()
        .programs
        .closure = closure.programs.closure;
    // Restoring the actual unresolved owner is refused at the earliest
    // preparation boundary; the completed numeric subgroup cannot bypass it.
    assert_eq!(
        partial.rebind_test_edit().unwrap_err(),
        "invalid evaluation stages: early readiness needs an early phase and complete owner programs"
    );
}

#[test]
fn bounded_duplicate_item_terms_reduce_above_the_per_term_bound_in_either_slot_order() {
    let mut i = world();
    // Both exact ring uses contribute BOUND. The group, unlike each input,
    // legitimately exceeds BOUND and preserves the other three item terms.
    change_ring(&mut i, 1_000_000., BOUND / 1_000_000.);
    assert_eq!(subtotal(&run_plan(&i)), &known(2. * BOUND + 43.));
    let query = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == key("life-base-contributions"))
        .unwrap();
    let equipment = query
        .groups
        .iter_mut()
        .find(|g| g.id == key("equipment"))
        .unwrap();
    for member in &mut equipment.members.members {
        let ranks = &mut member.order.as_mut().unwrap().slot_ranks;
        let last = ranks.iter().map(|r| r.rank).max().unwrap();
        for rank in ranks {
            rank.rank = last - rank.rank;
        }
    }
    i.rebind_test_edit().unwrap();
    assert_eq!(subtotal(&run_plan(&i)), &known(2. * BOUND + 43.));
}

#[test]
fn guarded_equipment_execution_matches_fresh_reused_and_rayon_workers() {
    let original = world();
    let inputs: Vec<_> = [(10., 1.), (0., 1.), (14., 1.), (1_000_000., 1_000_000.)]
        .into_iter()
        .map(|(amount, factor)| {
            let mut i = original.clone();
            change_ring(&mut i, amount, factor);
            i
        })
        .collect();
    let plans: Vec<_> = inputs.iter().map(|i| i.compile().unwrap()).collect();
    let fresh: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for index in [3, 0, 2, 1, 3, 0] {
        assert!(
            plans[index].evaluate(&mut scratch).unwrap() == fresh[index],
            "reused case {index}"
        );
    }
    for workers in [1, 2, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .unwrap();
        let parallel: Vec<_> = pool.install(|| {
            plans
                .par_iter()
                .map_init(|| plans[0].new_scratch(), |s, p| p.evaluate(s).unwrap())
                .collect()
        });
        assert!(parallel == fresh, "{workers} workers");
    }
}

#[test]
fn effect_bound_is_a_hard_public_compile_limit() {
    let i = world();
    let limits = PlanLimits::default();
    assert_eq!(limits.max_effects, 65_536);
    assert_eq!(
        i.compile_with_limits(PlanLimits {
            max_effects: limits.max_effects + 1,
            ..limits
        })
        .err()
        .unwrap(),
        "invalid effects limit"
    );
}
