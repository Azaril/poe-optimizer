//! Unpublished arithmetic, composed with real intrinsic endpoint producers.
//! Source comparisons retain the real combined factor. Arithmetic controls
//! replace only that exact producer and the proved coefficient literal. Their
//! synthetic values are not game producers.
use super::*;
use poe_optimizer_core::{owned_readiness::*, owned_routing::*, owned_schema::*, owned_stages::*};
use sha2::{Digest, Sha256};

const PROGRAM: &str = "physical-base-damage";
const OPERANDS: &str = "finite-physical-base-operands";
const INPUT_STAGE: &str = "finite-physical-base-inputs";
const OUTPUT_STAGE: &str = "physical-base-damage";
const NEUTRAL: [f64; 6] = [0., 0., 0., 0., 1.15, 1.];
const OPERAND_STATS: [u64; 5] = [0x3371, 0x3372, 0x3373, 0x3374, 0x336e];
const OUTPUT_STATS: [u64; 2] = [0x336f, 0x3370];

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
fn combined_packet(file: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/combined-added-attack-damage")
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
fn authenticated(pin: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), pin["sha256"]);
    bytes
}
fn source_basis() -> ActionOutputRoutes {
    // Reuse the published proof, rather than repeating its upstream source
    // checker or treating the finite vectors' absent fields as a proof.
    let bindings = packet("bindings.json");
    let prerequisite = &bindings["source_selection_prerequisite"];
    let source_bindings: Value =
        serde_json::from_slice(&authenticated(&prerequisite["bindings"])).unwrap();
    let evidence: Value =
        serde_json::from_slice(&authenticated(&prerequisite["evidence"])).unwrap();
    authenticated(&prerequisite["review"]);
    assert_eq!(evidence["upstream_revision"], bindings["source_revision"]);
    assert_eq!(
        evidence["kind"],
        "pinned-constructed-minion-intrinsic-source-selection"
    );
    let mut routes: Vec<ActionOutputRoutes> =
        serde_json::from_slice(&authenticated(&prerequisite["routes"])).unwrap();
    assert_eq!(routes.len(), 1);
    let expected = routes.pop().unwrap();
    assert_eq!(
        serde_json::to_value(&expected.output).unwrap(),
        source_bindings["output"]
    );
    let selectors = expected.source_selectors.as_ref().unwrap();
    assert!(selectors.is_complete());
    assert_eq!(selectors.members.len(), 1);
    assert_eq!(
        serde_json::to_value(&selectors.members[0]).unwrap(),
        source_bindings["selector"]
    );
    assert_eq!(
        prerequisite["endpoint_stats"],
        serde_json::to_value([def::<StatDefinition>(0x3212), def(0x3213)]).unwrap()
    );
    let coefficient = &bindings["coefficient_prerequisite"];
    let coefficient_evidence: Value =
        serde_json::from_slice(&authenticated(&coefficient["evidence"])).unwrap();
    authenticated(&coefficient["review"]);
    assert_eq!(
        coefficient_evidence["upstream_revision"],
        bindings["source_revision"]
    );
    assert_eq!(
        coefficient_evidence["selection_proof"]["sha256"],
        prerequisite["evidence"]["sha256"]
    );
    assert_eq!(coefficient_evidence["scope"]["coefficient"], 1.0);
    assert_eq!(coefficient["value"], 1.0);
    assert_eq!(coefficient["node"], "base-coefficient");
    assert_eq!(
        coefficient["unit"],
        serde_json::to_value(def::<UnitDefinition>(1)).unwrap()
    );
    expected
}
fn source_basis_matches(i: &ReplayInput, expected: &ActionOutputRoutes) -> bool {
    let Some(current) = i
        .routing
        .outputs
        .iter()
        .find(|r| r.output == expected.output)
    else {
        return false;
    };
    current.source_selectors == expected.source_selectors
        && expected
            .routes
            .members
            .iter()
            .filter(|r| matches!(r.source, ActionStatRouteSource::Selected { .. }))
            .all(|required| {
                let matching: Vec<_> = current
                    .routes
                    .members
                    .iter()
                    .filter(|r| r.target == required.target)
                    .collect();
                matching == vec![required]
            })
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
fn composed(synthetic_factor: bool) -> ReplayInput {
    let mut i = load();
    assert!(source_basis_matches(&i, &source_basis()));
    let authored: DefinitionRules =
        serde_json::from_value(combined_packet("consumer.json")["owners"][0].clone()).unwrap();
    assert_eq!(authored.owner, owner());
    assert_eq!(authored.programs.members.len(), 1);
    let factor = &authored.programs.members[0];
    let rules = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == authored.owner)
        .unwrap();
    let positions: Vec<_> = rules
        .programs
        .members
        .iter()
        .enumerate()
        .filter_map(|(n, p)| (p.id == factor.id).then_some(n))
        .collect();
    assert_eq!(positions.len(), 1);
    assert_eq!(&rules.programs.members[positions[0]], factor);
    let staged: Vec<_> = i
        .stages
        .programs
        .members
        .iter()
        .filter(|p| p.owner == authored.owner && p.program == factor.id)
        .cloned()
        .collect();
    assert_eq!(staged.len(), 1);
    let factor_stage = staged[0].stage.clone();
    if synthetic_factor {
        // Only explicit arithmetic controls replace this authenticated real
        // producer. Queries, contributor inventories and all closures remain.
        assert_eq!(rules.programs.members.remove(positions[0]), *factor);
        i.stages.programs.members.retain(|p| p != &staged[0]);
        let readiness = &mut i.stages.readiness.as_mut().unwrap().programs.members;
        let before = readiness.len();
        readiness.retain(|p| !(p.owner == authored.owner && p.program == factor.id));
        assert_eq!(before - readiness.len(), 1);
    }
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(packet("definitions.json")).unwrap();
    assert_eq!(definitions.len(), 6);
    let factor_binding = packet("bindings.json")["adopted_factor_definition"].clone();
    let migration: Value = serde_json::from_slice(
        &fs::read(root().join(factor_binding["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(migration["schema"].as_array().unwrap().len(), 1);
    let real_factor: DefinitionDescriptor = serde_json::from_value(
        migration
            .pointer(factor_binding["pointer"].as_str().unwrap())
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert!(matches!(&real_factor, DefinitionDescriptor::Stat(s) if s.id == def(0x336e)));
    let existing = i
        .schema
        .definitions
        .iter()
        .find(|d| matches!(d, DefinitionDescriptor::Stat(s) if s.id == def(0x336e)))
        .expect("the canonical replay must include the real combined-factor Stat");
    assert_eq!(existing, &real_factor, "exact adopted combined-factor Stat");
    for (n, definition) in (0x336f..=0x3374).zip(definitions) {
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
            predecessors: vec![i.stages.routing_stage.clone(), factor_stage.clone()],
        },
        EvaluationStage {
            id: key(OUTPUT_STAGE),
            predecessors: vec![key(INPUT_STAGE)],
        },
    ]);
    for n in [0x3212, 0x3213].into_iter().chain(OPERAND_STATS) {
        let channel = StageChannel::Stat {
            scope: RuleEntityKind::Action,
            stat: def(n),
        };
        let stage = if n < 0x3300 {
            i.stages.routing_stage.clone()
        } else if n == 0x336e && !synthetic_factor {
            factor_stage.clone()
        } else {
            key(INPUT_STAGE)
        };
        if n == 0x336e
            && let Some(existing) = i
                .stages
                .frozen_channels
                .iter_mut()
                .find(|f| f.channel == channel)
        {
            assert_eq!(existing.stage, factor_stage);
            existing.stage = stage;
        } else {
            i.stages
                .frozen_channels
                .push(FrozenStageChannel { channel, stage });
        }
    }
    install(&mut i, program(), OUTPUT_STAGE);
    i.rebind_test_edit().unwrap();
    i
}
fn fixture() -> ReplayInput {
    composed(true)
}
fn inject(i: &mut ReplayInput, operands: [Option<f64>; 5]) {
    let mut p = RuleProgram {
        id: key(OPERANDS),
        context: RuleEntityKind::Action,
        reads: vec![],
        nodes: vec![],
        effects: vec![],
    };
    for (n, amount) in OPERAND_STATS.into_iter().zip(operands) {
        let Some(amount) = amount else { continue };
        let id = key(&format!("operand-{n:x}"));
        p.nodes.push(RuleNode {
            id: id.clone(),
            expression: RuleExpression::Literal {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(amount, def(if n == 0x336e { 1 } else { 0x1d3a })).unwrap(),
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
fn supplied(operands: [f64; 6]) -> ReplayInput {
    let mut i = fixture();
    let [self_min, self_max, enemy_min, enemy_max, added, coefficient] = operands;
    inject(
        &mut i,
        [self_min, self_max, enemy_min, enemy_max, added].map(Some),
    );
    synthetic_coefficient(&mut i, coefficient);
    i
}
fn coefficient_one() -> RuleExpression {
    RuleExpression::Literal {
        value: ParameterValue::Quantity(FiniteQuantity::new(1.0, def(1)).unwrap()),
    }
}
fn synthetic_coefficient(i: &mut ReplayInput, coefficient: f64) {
    let authored = program();
    let p = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner())
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == authored.id)
        .unwrap();
    assert_eq!(
        &*p, &authored,
        "synthetic edit starts from the exact authored rule"
    );
    let literal = p
        .nodes
        .iter_mut()
        .find(|n| n.id == key("base-coefficient"))
        .unwrap();
    assert_eq!(literal.expression, coefficient_one());
    literal.expression = RuleExpression::Literal {
        value: ParameterValue::Quantity(FiniteQuantity::new(coefficient, def(1)).unwrap()),
    };
    i.rebind_test_edit().unwrap();
}
fn real_supplied(operands: [f64; 4]) -> ReplayInput {
    let mut i = composed(false);
    let [self_min, self_max, enemy_min, enemy_max] = operands;
    inject(
        &mut i,
        [
            Some(self_min),
            Some(self_max),
            Some(enemy_min),
            Some(enemy_max),
            None,
        ],
    );
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
            for (stat, expected) in OUTPUT_STATS.into_iter().zip(expected) {
                assert_eq!(number(r, a, stat), expected);
            }
        } else {
            for stat in OUTPUT_STATS {
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
fn changed_source_choice_requires_a_new_data_binding() {
    let i = load();
    let expected = source_basis();
    assert!(source_basis_matches(&i, &expected));
    for change in 0..3 {
        let mut edited = i.clone();
        let output = edited
            .routing
            .outputs
            .iter_mut()
            .find(|r| r.output == expected.output)
            .unwrap();
        match change {
            0 => output.source_selectors = None,
            1 => {
                output.source_selectors.as_mut().unwrap().members[0].sources[0].origin =
                    ActionSourceOrigin::CurrentAction;
            }
            2 => {
                output
                    .routes
                    .members
                    .iter_mut()
                    .find(|r| r.target == def(0x3212))
                    .unwrap()
                    .source = ActionStatRouteSource::ActionActor { stat: def(0x320f) };
            }
            _ => unreachable!(),
        }
        assert!(
            !source_basis_matches(&edited, &expected),
            "source change {change} cannot inherit the intrinsic-only proof"
        );
    }
}

#[test]
fn retained_original_operands_match_the_native_arithmetic_boundary() {
    let bindings = packet("bindings.json");
    assert_eq!(bindings["status"], "unpublished-arithmetic-fragment");
    let pin = &bindings["source_vectors"];
    let bytes = authenticated(pin);
    let evidence: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(evidence["status"], "passed");
    assert_eq!(evidence["source_revision"], bindings["source_revision"]);
    assert_eq!(
        evidence["reports"][0]["sha256"],
        evidence["reports"][1]["sha256"]
    );
    let neutral = real_supplied([0., 0., 0., 0.]);
    let flat = real_supplied([3., 7., 0., 0.]);
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
        // The separate published source proof establishes the intrinsic range.
        // These observations corroborate that binding; they do not close the
        // remaining flat-input producers. The coefficient is proved separately.
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
            let EffectValue::Known {
                value: ParameterValue::Quantity(factor),
            } = action_value(r, a, 0x336e)
            else {
                panic!("real combined-factor producer must resolve")
            };
            assert_eq!(factor.unit(), &def(1));
            assert_eq!(factor.value(), c["added_multiplier"].as_f64().unwrap());
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
        ([3., 7., 0.5, -1., 1.5, 0.25], [53.3125, 99.]),
        ([-3., -7., -2., 1., -2., -0.5], [-109., -199.5]),
        ([2., -2., 3., -3., 0., 1.], [208., 387.]),
        ([2., -2., 3., -3., 1., 0.], [0., 0.]),
        // Flat inputs combine before addition to the source. Reassociating
        // (source + self_flat) + enemy_flat incorrectly preserves 1e-20 here.
        ([-208., -387., 1e-20, 1e-20, 1., 1.], [0., 0.]),
        // Exact cancellation leaves a fractional result without rounding.
        ([-207.75, -386.5, 0., 0., 1., 0.5], [0.125, 0.25]),
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
        for stat in OUTPUT_STATS {
            assert!(matches!(
                action_value(&r, a, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    for absent in 0..5 {
        let mut i = fixture();
        let mut inputs = [Some(0.), Some(0.), Some(0.), Some(0.), Some(1.15)];
        inputs[absent] = None;
        inject(&mut i, inputs);
        synthetic_coefficient(&mut i, 0.);
        let r = run(&i);
        for a in actions(&i)
            .into_iter()
            .filter(|a| a.action.output.slot == def(0x22))
        {
            for (endpoint, stat) in OUTPUT_STATS.into_iter().enumerate() {
                let unresolved = absent >= 4 || absent % 2 == endpoint;
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
                action_value(&r, a, stat - 0x3212 + 0x336f),
                EffectValue::Unresolved { .. }
            ));
        }
    }
}

#[test]
fn exact_repeated_actions_survive_input_order_scratch_reuse_and_parallel_workers() {
    let a = supplied(NEUTRAL);
    let mut b = supplied([3., 7., 0.5, -1., 1.5, 0.25]);
    b.build
        .gems
        .iter_mut()
        .find(|g| g.definition == def(0x11))
        .unwrap()
        .level -= 1;
    let mut missing = fixture();
    let mut values = [Some(0.), Some(0.), Some(0.), Some(0.), Some(1.15)];
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
        number(&fresh[1], exact[0], 0x336f),
        number(&fresh[1], exact[1], 0x336f)
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
    let p = program();
    let bindings = packet("bindings.json");
    let inputs = bindings["inputs"].as_array().unwrap();
    assert_eq!(p.reads.len(), 7);
    assert_eq!(inputs.len(), p.reads.len());
    for (read, input) in p.reads.iter().zip(inputs) {
        assert_eq!(serde_json::to_value(&read.id).unwrap(), input["name"]);
        assert_eq!(
            serde_json::to_value(&read.source).unwrap()["value"]["stat"],
            input["stat"]
        );
    }
    assert_eq!(p.reads[0].id, key("source-min"));
    assert_eq!(p.reads[1].id, key("source-max"));
    assert_eq!(
        p.nodes
            .iter()
            .find(|n| n.id == key("base-coefficient"))
            .unwrap()
            .expression,
        coefficient_one()
    );
    let original = load();
    let i = composed(false);
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
