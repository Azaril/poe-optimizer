//! Actual injected Life-copy rules; finite component controls, not a whole build.
#[path = "support/owned_amulet_life_copy.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    build_identity::InstanceAllocator,
    owned_build::*,
    owned_definitions::*,
    owned_readiness::{ReadinessPhase, ReadinessProgram, ReadinessProgramRole},
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_engine::owned_rules::NumericalFailure;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{n:016x}"),
    )
    .unwrap()
}
fn key(n: &str) -> OwnedDefinitionKey {
    n.parse().unwrap()
}
fn life() -> SchemaSubject {
    SchemaSubject::Definition(def::<ModifierDefinition>(0x3100).address())
}
fn staged(
    w: &mut replay::ReplayInput,
    owner: SchemaSubject,
    p: &RuleProgram,
    template: &str,
    stat: StatDefId,
) {
    let mut s = w
        .stages
        .programs
        .members
        .iter()
        .find(|s| s.program.as_str() == template)
        .unwrap()
        .clone();
    s.owner = owner.clone();
    s.program = p.id.clone();
    w.stages.programs.members.push(s);
    let r = w.stages.readiness.as_mut().unwrap();
    let mut s = r
        .programs
        .members
        .iter()
        .find(|s| s.program.as_str() == template)
        .unwrap()
        .clone();
    s.owner = owner;
    s.program = p.id.clone();
    assert!(s.outputs.len() <= 1);
    for output in &mut s.outputs {
        match output {
            StageChannel::Stat { stat: out, .. }
            | StageChannel::Contributions { stat: out, .. } => {
                *out = stat.clone();
            }
            _ => panic!(),
        }
    }
    r.programs.members.push(s);
}
fn retained_copy_rules(w: &replay::ReplayInput, c: &family::Consumer) {
    let readiness = &w.stages.readiness.as_ref().unwrap().programs.members;
    let expected =
        std::iter::once((life(), &c.program, false)).chain(c.eligibility.iter().map(|owner| {
            assert_eq!(owner.programs.members.len(), 1);
            (owner.owner.clone(), &owner.programs.members[0], true)
        }));
    for (owner, program, eligibility) in expected {
        let owners: Vec<_> = w.rules.owners.iter().filter(|o| o.owner == owner).collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(
            owners[0]
                .programs
                .members
                .iter()
                .filter(|p| p.id == program.id)
                .collect::<Vec<_>>(),
            vec![program],
            "retained copy program must exist exactly once and match authored data"
        );
        let stage = StagedRuleProgram {
            owner: owner.clone(),
            program: program.id.clone(),
            stage: key(if eligibility { "prepare" } else { "deliver" }),
        };
        assert_eq!(
            w.stages
                .programs
                .members
                .iter()
                .filter(|p| p.owner == owner && p.program == program.id)
                .collect::<Vec<_>>(),
            vec![&stage]
        );
        let requirement = ReadinessProgram {
            owner: owner.clone(),
            program: program.id.clone(),
            phase: if eligibility {
                ReadinessPhase::Structural
            } else {
                ReadinessPhase::Execution
            },
            role: if eligibility {
                ReadinessProgramRole::PreparationFacts
            } else {
                ReadinessProgramRole::Execution
            },
            outputs: if eligibility {
                vec![StageChannel::Stat {
                    scope: RuleEntityKind::EquipmentUse,
                    stat: def(0x32e3),
                }]
            } else {
                vec![]
            },
        };
        assert_eq!(
            readiness
                .iter()
                .filter(|p| p.owner == owner && p.program == program.id)
                .collect::<Vec<_>>(),
            vec![&requirement]
        );
    }
}
fn world() -> replay::ReplayInput {
    let mut w = replay::ReplayInput::decode(
        &fs::read(family::root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    );
    // Keep the production query's full slot domain even though this replay only
    // contains a finite selection of equipped items.
    let dependencies: Value = serde_json::from_slice(
        &fs::read(
            family::root()
                .join("data/owned/poe2/3887ae68/life-contribution-queries/dependencies.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(dependencies["definitions"].clone()).unwrap();
    for definition in definitions
        .into_iter()
        .filter(|d| matches!(d, DefinitionDescriptor::EquipmentSlot(_)))
    {
        if let Some(existing) = w
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == definition.address())
        {
            assert_eq!(existing, &definition);
        } else {
            w.schema.definitions.push(definition);
        }
    }
    let c = family::consumer();
    // The current replay already carries these exact production programs and
    // scheduling declarations. Assert their identity instead of reinserting them
    // or silently accepting a conflicting implementation from a newer fixture.
    retained_copy_rules(&w, &c);
    let mut query = c.query;
    // Only this finite component's controlled input cases assert equipment
    // closure. Publication retains the actual Partial domain and order gap.
    query
        .groups
        .iter_mut()
        .find(|g| g.id.as_str() == "equipment")
        .unwrap()
        .members
        .closure = SchemaClosure::Complete;
    w.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .push(query);
    w.rebind_test_edit().unwrap();
    w
}
fn add_life(w: &mut replay::ReplayInput, values: &[f64]) {
    let sample = w
        .build
        .items
        .iter()
        .flat_map(|i| &i.modifiers)
        .find(|m| m.definition == def(0x3100))
        .unwrap()
        .clone();
    let DefinitionDescriptor::ItemTemplate(d) = w
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == def::<ItemTemplateDefinition>(0x2343).address())
        .unwrap()
    else {
        panic!()
    };
    let SchemaState::Known(schema) = &mut d.schema else {
        panic!()
    };
    schema.modifiers.members.push(def(0x3100));
    let mut allocator = InstanceAllocator::from_state(w.build.allocator);
    let item = w
        .build
        .items
        .iter_mut()
        .find(|i| i.template == def(0x2343))
        .unwrap();
    for value in values {
        let mut m = sample.clone();
        m.id = allocator.allocate().unwrap();
        m.rolls
            .iter_mut()
            .find(|r| r.slot.slot == def(0x3101))
            .unwrap()
            .value = ParameterValue::Quantity(FiniteQuantity::new(*value, def(0x295a)).unwrap());
        item.modifier_order.push(m.id);
        item.modifiers.push(m);
    }
    w.build.allocator = allocator.state();
}
fn remove_program(w: &mut replay::ReplayInput, owner: SchemaSubject, program: &str) {
    let programs = &mut w
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner)
        .unwrap()
        .programs
        .members;
    let before = programs.len();
    programs.retain(|p| p.id.as_str() != program);
    assert_eq!(programs.len() + 1, before);
    w.stages
        .programs
        .members
        .retain(|p| p.owner != owner || p.program.as_str() != program);
    w.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .retain(|p| p.owner != owner || p.program.as_str() != program);
}
fn literal_program(p: &mut RuleProgram, value: ParameterValue) {
    p.reads.clear();
    p.nodes = vec![RuleNode {
        id: key("test-value"),
        expression: RuleExpression::Literal { value },
    }];
    let RuleEffectKind::Derive { value, .. } = &mut p.effects[0].effect else {
        panic!()
    };
    *value = key("test-value");
}
fn percent(w: &mut replay::ReplayInput, value: f64) {
    let p = w
        .rules
        .owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .find(|p| p.id.as_str() == "pre-amulet-bonus-snapshot")
        .unwrap();
    literal_program(
        p,
        ParameterValue::Quantity(FiniteQuantity::new(value, def(2)).unwrap()),
    );
}
fn effective(w: &mut replay::ReplayInput, value: f64) {
    let p = w
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == life())
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == "effective-amount")
        .unwrap();
    // Explicit scalar-source probe: input is an already computed amount, not a
    // raw roll. The ordinary joined controls keep the real numeric producer.
    literal_program(
        p,
        ParameterValue::Quantity(FiniteQuantity::new(value, def(0x295a)).unwrap()),
    );
}
fn report(mut w: replay::ReplayInput) -> SupportEffectsReport {
    w.rebind_test_edit().unwrap();
    let p = w.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn copies(r: &SupportEffectsReport) -> Vec<&BoundEffectResult> {
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("{:?}", r.gaps)
    };
    effects
        .effects
        .iter()
        .filter(|e| e.key.invocation.program.as_str() == family::PROGRAM)
        .collect()
}
fn known(n: f64) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Quantity(FiniteQuantity::new(n, def(0x3119)).unwrap()),
    }
}
fn active(r: &SupportEffectsReport) -> Vec<&EffectValue> {
    copies(r)
        .into_iter()
        .filter(|e| e.value != EffectValue::Inactive)
        .map(|e| &e.value)
        .collect()
}
fn bypass(w: &mut replay::ReplayInput) {
    for item in &mut w.build.items {
        for m in &mut item.modifiers {
            if m.definition == def(0x3100) {
                m.rolls
                    .iter_mut()
                    .find(|r| r.slot.slot == def(0x3116))
                    .unwrap()
                    .value = ParameterValue::Boolean(true);
            }
        }
    }
}
#[test]
fn retained_copy_rules_reject_missing_duplicate_or_conflicting_fixture_declarations() {
    let baseline = replay::ReplayInput::decode(
        &fs::read(family::root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    );
    let consumer = family::consumer();
    retained_copy_rules(&baseline, &consumer);
    for case in 0..7 {
        let mut changed = baseline.clone();
        match case {
            0..=2 => {
                let programs = &mut changed
                    .rules
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == life())
                    .unwrap()
                    .programs
                    .members;
                let index = programs
                    .iter()
                    .position(|p| p.id == consumer.program.id)
                    .unwrap();
                match case {
                    0 => {
                        programs.remove(index);
                    }
                    1 => programs.push(programs[index].clone()),
                    2 => programs[index].context = RuleEntityKind::Actor,
                    _ => unreachable!(),
                }
            }
            3 | 4 => {
                let stages = &mut changed.stages.programs.members;
                let index = stages
                    .iter()
                    .position(|p| p.owner == life() && p.program == consumer.program.id)
                    .unwrap();
                if case == 3 {
                    stages.push(stages[index].clone());
                } else {
                    stages[index].stage = key("prepare");
                }
            }
            5 => {
                changed
                    .stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.owner == life() && p.program == consumer.program.id)
                    .unwrap()
                    .phase = ReadinessPhase::Structural
            }
            6 => changed
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == consumer.eligibility[0].owner)
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == consumer.eligibility[0].programs.members[0].id)
                .unwrap()
                .nodes
                .clear(),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| retained_copy_rules(&changed, &consumer)).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn actual_life_program_preserves_per_record_copying_and_inactive_sources() {
    family::check_authored();
    let base = report(world());
    assert_eq!(copies(&base).len(), 5);
    assert!(
        copies(&base)
            .iter()
            .all(|e| e.value == EffectValue::Inactive)
    );
    for (amounts, p, expected) in [
        (vec![17.], 0., vec![0.]),
        (vec![17.], 25., vec![4.]),
        (vec![17.], 100., vec![17.]),
        (vec![17., 19.], 25., vec![4., 4.]),
    ] {
        let mut w = world();
        add_life(&mut w, &amounts);
        percent(&mut w, p);
        let r = report(w);
        for e in copies(&r) {
            assert!(
                matches!(&e.target, BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Actor(ActorKey::Player) && key.stat == def(0x311a) && key.kind == ContributionKind::Add)
            );
        }
        assert_eq!(
            active(&r),
            expected
                .iter()
                .map(|n| known(*n))
                .collect::<Vec<_>>()
                .iter()
                .collect::<Vec<_>>()
        );
        let ids: Vec<_> = copies(&r)
            .into_iter()
            .filter(|e| e.value != EffectValue::Inactive)
            .map(|e| &e.key.invocation.origin)
            .collect();
        if ids.len() == 2 {
            assert_ne!(ids[0], ids[1]);
        }
    }
}
#[test]
fn source_scalar_vectors_match_native_numeric_branches() {
    let evidence: Value = family::read("source-vectors.json");
    for v in evidence["probes"].as_array().unwrap() {
        let mut w = world();
        add_life(&mut w, &[17.]);
        effective(&mut w, v["input"]["value"].as_f64().unwrap());
        let factor = v["factor"].as_f64().unwrap();
        assert_eq!((factor * 100. / 100.).to_bits(), factor.to_bits());
        percent(&mut w, factor * 100.);
        if v["name"] == "bypass" {
            bypass(&mut w);
        }
        assert_eq!(
            active(&report(w)),
            vec![&known(v["output"]["value"].as_f64().unwrap())],
            "{}",
            v["name"]
        );
    }
}
#[test]
fn missing_effective_is_unknown_only_for_eligible_copies() {
    let mut w = world();
    add_life(&mut w, &[17., 19.]);
    percent(&mut w, 25.);
    remove_program(&mut w, life(), "effective-amount");
    let r = report(w);
    assert_eq!(copies(&r).len(), 7);
    assert_eq!(
        active(&r),
        vec![
            &EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: Some(key("effective"))
            };
            2
        ]
    );
    assert_eq!(
        copies(&r)
            .into_iter()
            .filter(|e| e.value == EffectValue::Inactive)
            .count(),
        5
    );
}
#[test]
fn overflow_is_explicit_and_identity_or_bypass_skips_unused_arithmetic() {
    let mut w = world();
    add_life(&mut w, &[17.]);
    effective(&mut w, f64::MAX);
    percent(&mut w, 25.);
    assert_eq!(
        active(&report(w.clone())),
        vec![&EffectValue::NumericalError {
            node: key("scaled-hundred"),
            reason: NumericalFailure::NonFinite
        }]
    );
    let mut identity = w.clone();
    percent(&mut identity, 100.);
    assert_eq!(active(&report(identity)), vec![&known(f64::MAX)]);
    bypass(&mut w);
    assert_eq!(active(&report(w)), vec![&known(f64::MAX)]);
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FiniteQuantity::new(n, def(0x295a)).is_err());
    }
}
#[test]
fn source_removal_and_storage_permutations_preserve_exact_copies() {
    let mut w = world();
    add_life(&mut w, &[17., 19.]);
    percent(&mut w, 25.);
    let expected = report(w.clone());
    let mut permuted = w.clone();
    permuted.build.items.reverse();
    permuted.build.equipment.reverse();
    permuted.schema.definitions.reverse();
    permuted.rules.owners.reverse();
    for i in &mut permuted.build.items {
        i.modifiers.reverse();
    }
    assert_eq!(copies(&report(permuted)), copies(&expected));
    let item = w
        .build
        .items
        .iter_mut()
        .find(|i| i.template == def(0x2343))
        .unwrap();
    let removed: Vec<_> = item
        .modifiers
        .iter()
        .filter(|m| m.definition == def(0x3100))
        .map(|m| m.id)
        .collect();
    item.modifiers.retain(|m| !removed.contains(&m.id));
    item.modifier_order.retain(|id| !removed.contains(id));
    let removed_report = report(w);
    assert_eq!(copies(&removed_report).len(), 5);
    assert!(active(&removed_report).is_empty());
}
#[test]
fn current_partial_coverage_missing_membership_and_stage_refusals_remain() {
    let mut w = world();
    w.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == life())
        .unwrap()
        .programs
        .closure = family::consumer().life_closure;
    assert!(
        w.rebind_test_edit()
            .unwrap_err()
            .contains("early readiness needs an early phase and complete owner programs")
    );
    let mut missing = world();
    missing
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == "life-base-contributions")
        .unwrap()
        .groups
        .iter_mut()
        .find(|g| g.id.as_str() == "equipment")
        .unwrap()
        .members
        .members
        .pop();
    let error = missing
        .rebind_test_edit()
        .and_then(|_| missing.compile().map(|_| ()));
    assert!(error.is_err(), "inactive source must still be listed");
    let mut early = world();
    early
        .stages
        .programs
        .members
        .iter_mut()
        .find(|p| p.program.as_str() == family::PROGRAM)
        .unwrap()
        .stage = key("prepare");
    assert!(
        early
            .rebind_test_edit()
            .and_then(|_| early.compile().map(|_| ()))
            .is_err()
    );
}
#[test]
fn published_partial_equipment_query_cannot_be_read_as_a_known_total() {
    let mut w = world();
    add_life(&mut w, &[17., 19.]);
    percent(&mut w, 25.);
    // A test-only consumer reads the real membership through the normal graph.
    // It certifies no final resource formula or gameplay sum-order domain.
    let p: RuleProgram = serde_json::from_value(json!({
        "id":"test-equipment-life-total","context":"actor",
        "reads":[{"id":"incoming","value_type":{"kind":"quantity","value":{"unit":def::<UnitDefinition>(0x3119)}},"source":{"kind":"contribution_query","value":{"entity":"current","query":"life-base-contributions","group":"equipment"}}}],
        "nodes":[{"id":"incoming","expression":{"kind":"read","input":"incoming"}}],
        "effects":[{"id":"total","when":null,"effect":{"kind":"derive","entity":"current","stat":def::<StatDefinition>(0x311a),"value":"incoming"}}]
    })).unwrap();
    let owner = SchemaSubject::Definition(def::<ActorDefinition>(0x332a).address());
    staged(
        &mut w,
        owner.clone(),
        &p,
        "contribute-player-flat-life",
        def(0x311a),
    );
    w.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner)
        .unwrap()
        .programs
        .members
        .push(p);
    let total = |r: SupportEffectsReport| {
        let SupportEffectsOutcome::Evaluated { effects } = r.outcome else {
            panic!("{:?}", r.gaps)
        };
        effects
            .effects
            .into_iter()
            .find(|e| e.key.invocation.program.as_str() == "test-equipment-life-total")
            .unwrap()
            .value
    };
    assert_eq!(total(report(w.clone())), known(107.));
    let actual = family::consumer().query;
    w.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == actual.id)
        .unwrap()
        .groups = actual.groups;
    let partial = report(w);
    assert!(matches!(
        partial.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(
        partial
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::IncompleteContributors)
    );
}
#[test]
fn repeated_and_parallel_evaluation_preserves_copy_identity_and_values() {
    let a = world();
    let mut b = world();
    add_life(&mut b, &[17., 19.]);
    percent(&mut b, 25.);
    let mut unknown = b.clone();
    remove_program(&mut unknown, life(), "effective-amount");
    let worlds = [a, unknown, b];
    let plans: Vec<_> = worlds
        .into_iter()
        .map(|mut w| {
            w.rebind_test_edit().unwrap();
            w.compile().unwrap()
        })
        .collect();
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_eq!(
        active(&expected[1]),
        vec![
            &EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: Some(key("effective"))
            };
            2
        ]
    );
    let run = || {
        let mut scratch = plans[0].new_scratch();
        for n in [0, 1, 2, 0] {
            assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), expected[n]);
        }
    };
    run();
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| (0..8).into_par_iter().for_each(|_| run()));
}
#[test]
#[ignore = "requires AMULET_LIFE_COPY_PRIOR/OUTPUT and retained source reports"]
fn publish_life_copy_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_AMULET_LIFE_COPY_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_AMULET_LIFE_COPY_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json", "source-vectors.json"],
        family::stage,
        json!({"retired_life_owner_gaps":1,"remaining_life_owner_gaps":5,"new_programs":5,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
