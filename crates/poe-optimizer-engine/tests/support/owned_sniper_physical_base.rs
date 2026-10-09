//! Unpublished arithmetic, composed with real intrinsic endpoint producers.
//! The eight literal operands below are finite test inputs, not game producers.
use super::*;
use poe_optimizer_core::{owned_readiness::*, owned_schema::*, owned_stages::*};
use sha2::{Digest, Sha256};

const PROGRAM: &str = "physical-base-damage";
const OPERANDS: &str = "finite-physical-base-operands";
const INPUT_STAGE: &str = "finite-physical-base-inputs";
const OUTPUT_STAGE: &str = "physical-base-damage";
const NEUTRAL: [f64; 8] = [0., 0., 0., 0., 0., 0., 1.15, 1.];

fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn packet(file: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/physical-base-damage")
                .join(file),
        )
        .unwrap(),
    )
    .unwrap()
}
fn owner() -> SchemaSubject {
    serde_json::from_value(packet("bindings.json")["owner"].clone()).unwrap()
}
fn program() -> RuleProgram {
    serde_json::from_value(packet("consumer.json")["owners"][0]["programs"]["members"][0].clone())
        .unwrap()
}
fn install(i: &mut ReplayInput, p: RuleProgram, stage: &str) {
    let owner = owner();
    i.stages.programs.members.push(StagedRuleProgram {
        owner: owner.clone(),
        program: p.id.clone(),
        stage: key(stage),
    });
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .push(ReadinessProgram {
            owner: owner.clone(),
            program: p.id.clone(),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner)
        .unwrap()
        .programs
        .members
        .push(p);
}
fn fixture() -> ReplayInput {
    let mut i = load();
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(packet("definitions.json")).unwrap();
    assert_eq!(definitions.len(), 10);
    for (n, definition) in (0x336e..=0x3377).zip(definitions) {
        let DefinitionDescriptor::Stat(entry) = &definition else {
            panic!("only reserved draft Stats")
        };
        assert_eq!(entry.id, def(n));
        assert!(!i.schema.definitions.contains(&definition));
        // These are computed value declarations in this finite test graph, not
        // game source inventories or closures copied into a release.
        i.rules.owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(DefinitionAddress::Stat(entry.id.clone())),
            programs: DeclaredSet::complete(vec![]),
        });
        i.schema.definitions.push(definition);
    }
    i.stages.stages.extend([
        EvaluationStage {
            id: key(INPUT_STAGE),
            predecessors: vec![i.stages.routing_stage.clone()],
        },
        EvaluationStage {
            id: key(OUTPUT_STAGE),
            predecessors: vec![key(INPUT_STAGE)],
        },
    ]);
    for n in [0x3212, 0x3213].into_iter().chain(0x3370..=0x3377) {
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Action,
                stat: def(n),
            },
            stage: if n < 0x3300 {
                i.stages.routing_stage.clone()
            } else {
                key(INPUT_STAGE)
            },
        });
    }
    install(&mut i, program(), OUTPUT_STAGE);
    i.rebind_test_edit().unwrap();
    i
}
fn inject(i: &mut ReplayInput, operands: [Option<f64>; 8]) {
    let mut p = RuleProgram {
        id: key(OPERANDS),
        context: RuleEntityKind::Action,
        reads: vec![],
        nodes: vec![],
        effects: vec![],
    };
    for (n, amount) in (0x3370..=0x3377).zip(operands) {
        let Some(amount) = amount else { continue };
        let id = key(&format!("operand-{n:x}"));
        p.nodes.push(RuleNode {
            id: id.clone(),
            expression: RuleExpression::Literal {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(amount, def(if n >= 0x3376 { 1 } else { 0x1d3a })).unwrap(),
                ),
            },
        });
        p.effects.push(RuleEffect {
            id: id.clone(),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: def(n),
                value: id,
            },
        });
    }
    install(i, p, INPUT_STAGE);
    i.rebind_test_edit().unwrap();
}
fn supplied(operands: [f64; 8]) -> ReplayInput {
    let mut i = fixture();
    inject(&mut i, operands.map(Some));
    i
}
fn number(r: &SupportEffectsReport, a: &ActionSelection, stat: u64) -> f64 {
    let EffectValue::Known {
        value: ParameterValue::Quantity(v),
    } = action_value(r, a, stat)
    else {
        panic!("known exact Action quantity {stat:x}")
    };
    assert_eq!(v.unit(), &def(0x1d3a));
    v.value()
}
fn check(i: &ReplayInput, r: &SupportEffectsReport, expected: [f64; 2]) {
    let mut seen = 0;
    for a in actions(i) {
        if a.action.output.slot == def(0x22) {
            seen += 1;
            for (stat, expected) in (0x336e..=0x336f).zip(expected) {
                assert_eq!(number(r, a, stat), expected);
            }
        } else {
            for stat in [0x336e, 0x336f] {
                assert!(!effects(r).values.iter().any(|v| v.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Action(Box::new(a.clone())),
                        stat: def(stat),
                    }));
            }
        }
    }
    assert_eq!(seen, 2, "independent exact Basic Actions, never Gas");
    let derived: Vec<_> = effects(r)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(PROGRAM))
        .collect();
    assert_eq!(derived.len(), 4, "two endpoints for each exact occurrence");
}

#[test]
fn retained_original_operands_match_the_native_arithmetic_boundary() {
    let bindings = packet("bindings.json");
    assert_eq!(bindings["status"], "unpublished-arithmetic-fragment");
    let pin = &bindings["source_vectors"];
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), pin["sha256"]);
    let evidence: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(evidence["status"], "passed");
    assert_eq!(evidence["source_revision"], bindings["source_revision"]);
    assert_eq!(
        evidence["reports"][0]["sha256"],
        evidence["reports"][1]["sha256"]
    );
    let neutral = supplied(NEUTRAL);
    let flat = supplied([0., 0., 3., 7., 0., 0., 1.15, 1.]);
    let reports = [run(&neutral), run(&flat)];
    let inputs = [&neutral, &flat];
    let mut observed = 0;
    for row in evidence["vectors"].as_array().unwrap() {
        if row["observed"] != true {
            assert!(row.get("base_call").is_none());
            continue;
        }
        observed += 1;
        let c = &row["base_call"];
        assert_eq!(row["child_effect"], "MinionMeleeBow");
        assert_eq!(c["damage_type"], "Physical");
        assert_eq!(c["observed_at"], 4137);
        assert_eq!(c["query_state_preserved"], true);
        // Source absence is established in these recorded cases only. The
        // native draft still requires explicit resolved bonus operands.
        assert!(c["source"].get("PhysicalBonusMin").is_none());
        assert!(c["source"].get("PhysicalBonusMax").is_none());
        assert_eq!(c["enemy_minimum"]["value"], 0.);
        assert_eq!(c["enemy_maximum"]["value"], 0.);
        assert_eq!(c["added_multiplier"], 1.15);
        assert_eq!(c["base_multiplier"], 1.);
        let index = usize::from(c["minimum"]["value"] == 3.);
        assert_eq!(c["minimum"]["value"], [0., 3.][index]);
        assert_eq!(c["maximum"]["value"], [0., 7.][index]);
        let i = inputs[index];
        let r = &reports[index];
        for a in actions(i)
            .into_iter()
            .filter(|a| a.action.output.slot == def(0x22))
        {
            assert_eq!(
                number(r, a, 0x3212),
                c["source"]["PhysicalMin"].as_f64().unwrap()
            );
            assert_eq!(
                number(r, a, 0x3213),
                c["source"]["PhysicalMax"].as_f64().unwrap()
            );
        }
        check(
            i,
            r,
            [
                c["base_min"].as_f64().unwrap(),
                c["base_max"].as_f64().unwrap(),
            ],
        );
    }
    assert_eq!(observed, 13);
    check(&neutral, &reports[0], [208., 387.]);
    check(&flat, &reports[1], [211.45, 395.05]);
}

#[test]
fn signed_inputs_and_coefficients_follow_declared_operation_order_without_rounding() {
    for (operands, expected) in [
        (
            [2.5, -3.25, 3., 7., 0.5, -1., 1.5, 0.25],
            [53.9375, 98.1875],
        ),
        ([-300., -500., -3., -7., -2., 1., -2., -0.5], [41., 50.5]),
        ([1., -1., 2., -2., 3., -3., 0., 1.], [209., 386.]),
        ([1., -1., 2., -2., 3., -3., 1., 0.], [0., 0.]),
        ([-208., -387., 1e-20, 1e-20, 0., 0., 1., 1.], [1e-20, 1e-20]),
    ] {
        let i = supplied(operands);
        check(&i, &run(&i), expected);
    }
}

#[test]
fn missing_operands_remain_unresolved_even_when_the_coefficient_is_zero() {
    let bare = fixture();
    let r = run(&bare);
    for a in actions(&bare)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
    {
        for stat in [0x336e, 0x336f] {
            assert!(matches!(
                action_value(&r, a, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    for absent in 0..8 {
        let mut i = fixture();
        let mut inputs = NEUTRAL.map(Some);
        inputs[7] = Some(0.);
        inputs[absent] = None;
        inject(&mut i, inputs);
        let r = run(&i);
        for a in actions(&i)
            .into_iter()
            .filter(|a| a.action.output.slot == def(0x22))
        {
            for (endpoint, stat) in [0x336e, 0x336f].into_iter().enumerate() {
                let unresolved = absent >= 6 || absent % 2 == endpoint;
                if unresolved {
                    assert!(matches!(
                        action_value(&r, a, stat),
                        EffectValue::Unresolved { .. }
                    ));
                } else {
                    assert_eq!(number(&r, a, stat), 0.);
                }
            }
        }
    }
    for stat in [0x3212, 0x3213] {
        let mut i = supplied(NEUTRAL);
        let route = i
            .routing
            .outputs
            .iter_mut()
            .find(|r| r.output.slot == def(0x22))
            .unwrap();
        route.routes.members.retain(|r| r.target != def(stat));
        i.rebind_test_edit().unwrap();
        let r = run(&i);
        for a in actions(&i)
            .into_iter()
            .filter(|a| a.action.output.slot == def(0x22))
        {
            assert!(matches!(
                action_value(&r, a, stat - 0x3212 + 0x336e),
                EffectValue::Unresolved { .. }
            ));
        }
    }
}

#[test]
fn exact_repeated_actions_survive_input_order_scratch_reuse_and_parallel_workers() {
    let a = supplied(NEUTRAL);
    let mut b = supplied([2.5, -3.25, 3., 7., 0.5, -1., 1.5, 0.25]);
    b.build
        .gems
        .iter_mut()
        .find(|g| g.definition == def(0x11))
        .unwrap()
        .level -= 1;
    let mut missing = fixture();
    let mut values = NEUTRAL.map(Some);
    values[0] = None;
    inject(&mut missing, values);
    let inputs = [&a, &b, &missing];
    let plans = inputs.map(|i| i.compile().unwrap());
    let fresh = plans
        .each_ref()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap());
    let exact: Vec<_> = actions(&b)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
        .collect();
    assert_ne!(exact[0].action.actor, exact[1].action.actor);
    assert_ne!(
        number(&fresh[1], exact[0], 0x3212),
        number(&fresh[1], exact[1], 0x3212)
    );
    assert_ne!(
        number(&fresh[1], exact[0], 0x336e),
        number(&fresh[1], exact[1], 0x336e)
    );
    let mut reordered = b.clone();
    reordered.build.skills.reverse();
    reordered.build.gems.reverse();
    assert_eq!(run(&reordered), fresh[1]);
    let mut scratch = plans[0].new_scratch();
    for n in [0, 1, 2, 1, 0] {
        assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), fresh[n]);
    }
    let reports = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |scratch, n| plans[n % 3].evaluate(scratch).unwrap(),
                )
                .collect::<Vec<_>>()
        });
    for (n, r) in reports.into_iter().enumerate() {
        assert_eq!(r, fresh[n % 3]);
    }
}

#[test]
fn data_binding_units_staging_and_partial_scope_are_checked() {
    let consumer = packet("consumer.json");
    assert_eq!(consumer["owners"].as_array().unwrap().len(), 1);
    assert_eq!(
        consumer["owners"][0]["owner"],
        packet("bindings.json")["owner"]
    );
    assert_eq!(
        consumer["owners"][0]["programs"]["closure"]["kind"],
        "partial"
    );
    let original = load();
    let i = fixture();
    assert_eq!(i.build, original.build);
    assert_eq!(i.scenario, original.scenario);
    assert_eq!(i.queries, original.queries);
    for previous in &original.rules.owners {
        let current = i
            .rules
            .owners
            .iter()
            .find(|o| o.owner == previous.owner)
            .unwrap();
        assert_eq!(current.programs.closure, previous.programs.closure);
        let mut inverse = current.clone();
        if current.owner == owner() {
            assert_eq!(inverse.programs.members.pop().unwrap(), program());
        }
        assert_eq!(inverse, *previous);
    }
    let mut wrong_unit = i.clone();
    let p = wrong_unit
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner())
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(PROGRAM))
        .unwrap();
    p.reads
        .iter_mut()
        .find(|r| r.id == key("added-multiplier"))
        .unwrap()
        .value_type = ComputedValueType::Quantity { unit: def(0x1d3a) };
    wrong_unit.rebind_test_edit().unwrap();
    assert!(
        wrong_unit.compile().is_err(),
        "semantic compiler checks units"
    );
    let mut early = i;
    early
        .stages
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key(PROGRAM))
        .unwrap()
        .stage = key("facts");
    assert!(
        early
            .rebind_test_edit()
            .unwrap_err()
            .contains("frozen channel read")
    );
}
