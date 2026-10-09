//! A pinned, finite joined component graph replayed through public native APIs.
//! This is not a complete build or permission to close production coverage.
#[path = "support/owned_sniper_added_damage.rs"]
mod added_damage;
#[allow(dead_code)]
#[path = "../../../tests/support/owned_plan_replay.rs"]
mod replay;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use replay::ReplayInput;
use serde_json::Value;
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn load() -> ReplayInput {
    ReplayInput::decode(
        &fs::read(root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    )
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{n:016x}"),
    )
    .unwrap()
}
fn actors(input: &ReplayInput) -> Vec<ActorKey> {
    let gems: Vec<_> = input
        .build
        .gems
        .iter()
        .filter(|g| g.definition == def(0x11))
        .collect();
    assert_eq!(gems.len(), 2);
    gems.into_iter()
        .map(|gem| {
            let skills: Vec<_> = input
                .build
                .skills
                .iter()
                .filter(|s| s.source == AuthoredSkillSource::Gem(gem.id))
                .collect();
            assert_eq!(skills.len(), 1);
            ActorKey::Owned(Box::new(OwnedActorKey {
                provider: ProviderKey {
                    root: ProviderRoot::SkillUse(skills[0].id),
                    grant_path: vec![DeclaredSlot {
                        declaration: SlotOwnerDefId::Gem(def(0x11)),
                        slot: def(0x17),
                    }],
                },
                slot: DeclaredSlot {
                    declaration: SlotOwnerDefId::Skill(def(0x12)),
                    slot: def(0x1f),
                },
            }))
        })
        .collect()
}
fn effects(r: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("expected evaluated component: {:?}", r.outcome);
    };
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    effects
}
fn value<'a>(r: &'a SupportEffectsReport, actor: &ActorKey, stat: u64) -> &'a EffectValue {
    &effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: def(stat),
                }
        })
        .unwrap_or_else(|| panic!("missing actor stat {stat:x}"))
        .value
}
fn expected(case: &str) -> (f64, f64, f64) {
    let v: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/mixed-minion-damage/source-vectors.json"))
            .unwrap(),
    )
    .unwrap();
    let v = v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case"] == case)
        .unwrap();
    (
        v["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["value"].as_f64().unwrap())
            .sum(),
        v["increased_factor"].as_f64().unwrap(),
        v["more_factor"].as_f64().unwrap(),
    )
}
fn check(input: &ReplayInput, r: &SupportEffectsReport, cases: [&str; 2]) {
    for (actor, case) in actors(input).iter().zip(cases) {
        let (increased, factor, more) = expected(case);
        for (stat, expected, unit) in [
            (0x3353, increased, 2),
            (0x3354, factor, 1),
            (0x330b, more, 1),
        ] {
            assert_eq!(
                value(r, actor, stat),
                &EffectValue::Known {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(expected, def(unit)).unwrap()
                    ),
                },
                "{case}: {stat:x}"
            );
        }
    }
    check_actions(input, r, cases.map(|case| expected(case).0), 55.);
}
fn action_value<'a>(
    r: &'a SupportEffectsReport,
    action: &ActionSelection,
    stat: u64,
) -> &'a EffectValue {
    &effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Action(Box::new(action.clone())),
                    stat: def(stat),
                }
        })
        .expect("exact Action value")
        .value
}
fn actions(input: &ReplayInput) -> Vec<&ActionSelection> {
    let found: Vec<_> = input
        .queries
        .requests
        .iter()
        .filter_map(|r| match &r.target {
            MetricTarget::Action(a) if [def(0x22), def(0x25)].contains(&a.action.output.slot) => {
                Some(a.as_ref())
            }
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 8, "Basic and three Gas modes per exact Sniper");
    found
}
fn check_actions(input: &ReplayInput, r: &SupportEffectsReport, shared: [f64; 2], command: f64) {
    let recipients = actors(input);
    for action in actions(input) {
        let i = recipients
            .iter()
            .position(|a| *a == action.action.actor)
            .unwrap();
        let total = shared[i]
            + if action.action.output.slot == def(0x25) {
                command
            } else {
                0.
            };
        for (stat, expected, unit) in [(0x335c, total, 2), (0x335d, 1. + total / 100., 1)] {
            assert_eq!(
                action_value(r, action, stat),
                &EffectValue::Known {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(expected, def(unit)).unwrap()
                    ),
                }
            );
        }
    }
}
fn quality(input: &mut ReplayInput, values: [f64; 2]) {
    let gems: Vec<_> = input
        .build
        .gems
        .iter_mut()
        .filter(|g| g.definition == def(0x11))
        .collect();
    assert_eq!(gems.len(), 2);
    for (gem, amount) in gems.into_iter().zip(values) {
        gem.quality.as_mut().unwrap().amount = FiniteQuantity::new(amount, def(2)).unwrap();
    }
}
fn run(input: &ReplayInput) -> SupportEffectsReport {
    let p = input.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}

#[test]
fn replay_uses_current_published_damage_consumers_and_queries() {
    let i = load();
    let packet = root().join("data/owned/poe2/3887ae68/mixed-minion-damage");
    let consumer: Value =
        serde_json::from_slice(&fs::read(packet.join("consumer.json")).unwrap()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(consumer["owners"].clone()).unwrap();
    for owner in owners {
        assert_eq!(
            i.rules
                .owners
                .iter()
                .find(|o| o.owner == owner.owner)
                .unwrap(),
            &owner
        );
    }
    let receivers: Vec<StatReceiver> =
        serde_json::from_value(consumer["receivers"].clone()).unwrap();
    for receiver in receivers {
        assert!(i.rules.receivers.members.contains(&receiver));
    }
    let queries: Vec<ContributionQuery> =
        serde_json::from_slice(&fs::read(packet.join("queries.json")).unwrap()).unwrap();
    for query in queries {
        assert!(
            i.rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&query)
        );
    }
    let dependencies: Value =
        serde_json::from_slice(&fs::read(packet.join("dependencies.json")).unwrap()).unwrap();
    let sources = dependencies["source_programs"].as_array().unwrap();
    assert_eq!(
        sources.len(),
        2,
        "review the upstream dependency census when it changes"
    );
    for source in sources {
        let owner = serde_json::from_value(source["owner"].clone()).unwrap();
        let program: RuleProgram = serde_json::from_value(source["program"].clone()).unwrap();
        assert!(
            i.rules
                .owners
                .iter()
                .find(|o| o.owner == owner)
                .unwrap()
                .programs
                .members
                .contains(&program)
        );
    }
    let packet = root().join("data/owned/poe2/3887ae68/action-minion-damage");
    let consumer: Value =
        serde_json::from_slice(&fs::read(packet.join("consumer.json")).unwrap()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(consumer["owners"].clone()).unwrap();
    for owner in owners {
        let current = i
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner.owner)
            .unwrap();
        for program in owner.programs.members {
            assert!(current.programs.members.contains(&program));
        }
    }
    let queries: Vec<ContributionQuery> =
        serde_json::from_slice(&fs::read(packet.join("queries.json")).unwrap()).unwrap();
    for query in queries {
        assert!(
            i.rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&query)
        );
    }
    let dependencies: Value =
        serde_json::from_slice(&fs::read(packet.join("dependencies.json")).unwrap()).unwrap();
    let sources = dependencies["source_programs"].as_array().unwrap();
    assert_eq!(sources.len(), 9);
    for source in sources {
        let owner = serde_json::from_value(source["owner"].clone()).unwrap();
        let program: RuleProgram = serde_json::from_value(source["program"].clone()).unwrap();
        assert!(
            i.rules
                .owners
                .iter()
                .find(|o| o.owner == owner)
                .unwrap()
                .programs
                .members
                .contains(&program)
        );
    }
    check(&i, &run(&i), ["original-05", "original-05"]);
}

#[test]
fn command_modes_and_source_removals_match_retained_original_calls() {
    let packet = root().join("data/owned/poe2/3887ae68");
    let vectors: Value = serde_json::from_slice(
        &fs::read(packet.join("action-minion-damage/source-vectors.json")).unwrap(),
    )
    .unwrap();
    let bindings: Value =
        serde_json::from_slice(&fs::read(packet.join("command-damage/bindings.json")).unwrap())
            .unwrap();
    let original = load();
    let mut scratch = original.compile().unwrap().new_scratch();
    for vector in vectors["vectors"].as_array().unwrap() {
        let mut i = original.clone();
        for removed in vector["removed"].as_array().unwrap() {
            let source_id = removed.as_u64().unwrap().to_string();
            if let Some(binding) = bindings["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["source_id"].as_str() == Some(source_id.as_str()))
            {
                let node: PassiveNodeDefId =
                    serde_json::from_value(binding["definition"].clone()).unwrap();
                let before = i.build.allocations.len();
                i.build.allocations.retain(|a| a.node != node);
                assert_eq!(i.build.allocations.len() + 1, before);
            }
        }
        let plan = i.compile().unwrap();
        let r = plan.evaluate(&mut scratch).unwrap();
        assert_eq!(plan.evaluate(&mut plan.new_scratch()).unwrap(), r);
        let gas = vector["effect_id"] == "GasShotSkeletonSniperMinion";
        let stat_set = if gas {
            [0x32ec, 0x32ed, 0x32ee][vector["stat_set"].as_u64().unwrap() as usize - 1]
        } else {
            9
        };
        let selected: Vec<_> = actions(&i)
            .into_iter()
            .filter(|a| a.stat_set == def(stat_set))
            .collect();
        assert_eq!(selected.len(), 2);
        for a in selected {
            for (stat, field, unit) in [(0x335c, "subtotal", 2), (0x335d, "increase_factor", 1)] {
                assert_eq!(
                    action_value(&r, a, stat),
                    &EffectValue::Known {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(vector[field].as_f64().unwrap(), def(unit))
                                .unwrap()
                        ),
                    },
                    "{}: {field}",
                    vector["case"]
                );
            }
        }
    }
}

#[test]
fn replay_preserves_artifact_bindings_and_rejects_stale_rules() {
    let bytes = fs::read(root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap();
    let mut i = ReplayInput::decode(&bytes);
    assert!(
        i.encode() == bytes,
        "deterministic current fixture encoding"
    );
    i.rules.semantics_version = OwnedDefinitionKey::new("fixture.changed-semantics").unwrap();
    let error = match i.compile() {
        Ok(_) => panic!("stale stage binding must not be repaired implicitly"),
        Err(error) => error,
    };
    assert!(
        error.contains("evaluation stages schema/rule/routing binding mismatch"),
        "{error}"
    );
}

#[test]
fn real_preparation_preserves_independent_quality_and_removed_gigantic() {
    let mut i = load();
    quality(&mut i, [1., 20.]);
    check(&i, &run(&i), ["sniper-quality-1", "sniper-quality-20"]);
    quality(&mut i, [0., 20.]);
    let before = i.build.allocations.len();
    i.build.allocations.retain(|a| a.node != def(0x1532));
    assert_eq!(i.build.allocations.len() + 1, before);
    check(
        &i,
        &run(&i),
        ["without-gigantic", "quality-20-without-gigantic"],
    );
}

#[test]
fn offering_stacking_activation_and_missing_inputs_survive_reused_parallel_workers() {
    let a = load();
    let mut b = a.clone();
    quality(&mut b, [1., 20.]);
    let mut unknown = a.clone();
    let index = unknown
        .scenario
        .usage
        .iter()
        .position(|u| u.policy == def(0x3259))
        .unwrap();
    unknown.scenario.usage.remove(index);
    let mut inactive = a.clone();
    let mut changed = 0;
    for usage in &mut inactive.scenario.usage {
        if usage.policy == def(0x3259) {
            assert_eq!(usage.parameters.len(), 1);
            usage.parameters[0].value = ParameterValue::Boolean(false);
            changed += 1;
        }
    }
    assert_eq!(changed, 2, "two non-stacking Offering occurrences");
    let plans = [&a, &unknown, &b, &inactive].map(|i| i.compile().unwrap());
    let fresh = plans
        .each_ref()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap());
    check(&a, &fresh[0], ["original-05", "offering-duplicate-equal"]);
    check(&b, &fresh[2], ["sniper-quality-1", "sniper-quality-20"]);
    check(
        &inactive,
        &fresh[3],
        ["offering-disabled", "offering-disabled"],
    );
    for actor in actors(&unknown) {
        for stat in [0x3353, 0x3354] {
            assert!(matches!(
                value(&fresh[1], &actor, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    for action in actions(&unknown) {
        for stat in [0x335c, 0x335d] {
            assert!(matches!(
                action_value(&fresh[1], action, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    let mut scratch = plans[0].new_scratch();
    for index in [0, 1, 2, 3, 0] {
        assert!(plans[index].evaluate(&mut scratch).unwrap() == fresh[index]);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, index| plans[index % 4].evaluate(scratch).unwrap(),
            )
            .collect::<Vec<_>>()
    });
    for (index, report) in reports.into_iter().enumerate() {
        assert!(report == fresh[index % 4], "parallel result {index}");
    }
}
