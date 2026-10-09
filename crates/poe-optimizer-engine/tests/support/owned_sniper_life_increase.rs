//! Published Life delivery on the shared replay. These finite source controls
//! compare the Increase channel only, not legal tree mutations or final Life.
use super::*;
use poe_optimizer_core::owned_schema::*;
use std::collections::BTreeSet;

const RECEIVED: &str = "received-minion-life-increase";
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn packet(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/minion-life-increase")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn received_owner() -> DefinitionRules {
    serde_json::from_value(packet("migration.json")["owners"][0].clone()).unwrap()
}
fn known(n: f64) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Quantity(FiniteQuantity::new(n, def(2)).unwrap()),
    }
}
fn refused(report: &SupportEffectsReport, reason: PlanGapReason) {
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(
        report.gaps.iter().any(|gap| gap.reason == reason),
        "{:?}",
        report.gaps
    );
}
fn check(i: &ReplayInput, report: &SupportEffectsReport, expected: f64) {
    let r = effects(report);
    let bindings = packet("bindings.json");
    let nodes = bindings["nodes"].as_array().unwrap();
    let allocations: Vec<_> = i
        .build
        .allocations
        .iter()
        .filter(|a| {
            nodes
                .iter()
                .any(|n| n["definition"] == serde_json::json!(a.node))
        })
        .collect();
    let carrier: Vec<_> = r
        .effects
        .iter()
        .filter(|e| {
            matches!(&e.target,
                BoundEffectTarget::Contribution { key } if key.stat == def(0x32e5)
            )
        })
        .collect();
    assert_eq!(carrier.len(), allocations.len());
    let mut witnessed = BTreeSet::new();
    let mut sum = 0.;
    for row in carrier {
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!("exact passive provider required")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("allocated passive required")
        };
        assert!(provider.grant_path.is_empty() && witnessed.insert(id));
        let allocation = allocations.iter().find(|a| a.id == id).unwrap();
        let source = nodes
            .iter()
            .find(|n| n["definition"] == serde_json::json!(allocation.node))
            .unwrap();
        let amount = source["amount"].as_f64().unwrap();
        sum += amount;
        assert_eq!(row.key.invocation.program, key("ordinary-minion-life"));
        assert_eq!(
            row.key.invocation.owner,
            SchemaSubject::Definition(DefinitionAddress::PassiveNode(allocation.node.clone()))
        );
        assert_eq!(row.value, known(amount));
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(0x32e5),
                    kind: ContributionKind::Increase,
                }
            }
        );
    }
    assert_eq!(witnessed, allocations.iter().map(|a| a.id).collect());
    assert_eq!(sum, expected);
    let received: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(RECEIVED))
        .collect();
    assert_eq!(
        received.len(),
        2,
        "one delivery per exact Sniper occurrence"
    );
    for actor in actors(i) {
        let rows: Vec<_> = received
            .iter()
            .filter(|e| {
                matches!(&e.target,
                    BoundEffectTarget::Contribution { key } if key == &ContributionKey {
                        entity: ConcreteEntity::Actor(actor.clone()),
                        stat: def(0x311a),
                        kind: ContributionKind::Increase,
                    }
                )
            })
            .collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row.key.invocation.owner, received_owner().owner);
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::Actor(actor.clone())
        );
        assert_eq!(row.value, known(expected));
        let ActorKey::Owned(actor) = actor else {
            unreachable!()
        };
        let mut provider = actor.provider;
        provider.grant_path.push(DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(def(0x12)),
            slot: def(0x20),
        });
        assert_eq!(row.key.invocation.origin, RuleOrigin::Provider { provider });
    }
    assert!(
        !r.values.iter().any(|v| matches!(&v.key,
            PlanValueKey::Stat { stat, .. } if *stat == def(0x311a)
        )),
        "Life contributions are not a final Life pool"
    );
}

#[test]
fn published_life_programs_preserve_exact_sources_and_recipients() {
    let i = load();
    let mut owners: Vec<DefinitionRules> =
        serde_json::from_value(packet("dependencies.json")["passive_owners"].clone()).unwrap();
    assert_eq!(owners.len(), 6);
    owners.push(received_owner());
    for owner in owners {
        let actual = i
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner.owner)
            .unwrap();
        for program in owner.programs.members {
            assert_eq!(
                actual
                    .programs
                    .members
                    .iter()
                    .filter(|p| **p == program)
                    .count(),
                1
            );
        }
    }
    let total = packet("source-vectors.json")["native_cases"][0]["received_increase"]
        .as_f64()
        .unwrap();
    check(&i, &run(&i), total);
}

#[test]
fn retained_life_channel_controls_survive_reused_parallel_workers() {
    let original = load();
    let bindings = packet("bindings.json");
    let vectors = packet("source-vectors.json");
    let cases = vectors["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    let inputs: Vec<_> = cases
        .iter()
        .map(|case| {
            let mut i = original.clone();
            // PoB also prunes Gigantic with node229 and cooldown14945 with1218.
            // Deliberately isolate this channel; no final-Life/MORE parity claim.
            for node in bindings["nodes"].as_array().unwrap() {
                if !case["eligible_sources"]
                    .as_array()
                    .unwrap()
                    .contains(&node["source_id"])
                {
                    let node: PassiveNodeDefId =
                        serde_json::from_value(node["definition"].clone()).unwrap();
                    let before = i.build.allocations.len();
                    i.build.allocations.retain(|a| a.node != node);
                    assert_eq!(i.build.allocations.len() + 1, before);
                }
            }
            i
        })
        .collect();
    let plans: Vec<_> = inputs.iter().map(|i| i.compile().unwrap()).collect();
    let fresh: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    for ((input, report), case) in inputs.iter().zip(&fresh).zip(cases) {
        check(input, report, case["received_increase"].as_f64().unwrap());
    }
    let mut scratch = plans[0].new_scratch();
    for index in [0, 3, 4, 5, 6, 0] {
        assert!(
            plans[index].evaluate(&mut scratch).unwrap() == fresh[index],
            "reused case{index}"
        );
    }
    let reports = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..28)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |s, n| plans[n % 7].evaluate(s).unwrap(),
                )
                .collect::<Vec<_>>()
        });
    for (n, report) in reports.into_iter().enumerate() {
        assert!(report == fresh[n % 7], "parallel case{n}");
    }
    let mut changed = original;
    quality(&mut changed, [1., 20.]);
    changed.build.skills.reverse();
    changed.build.allocations.reverse();
    check(
        &changed,
        &run(&changed),
        cases[0]["received_increase"].as_f64().unwrap(),
    );
}

#[test]
fn missing_and_partial_life_authority_cannot_become_empty_zero() {
    let original = load();
    let incoming = SchemaSubject::Definition(DefinitionAddress::PassiveNode(def(0xaef)));
    let actor = received_owner();
    for subject in [&incoming, &actor.owner] {
        let mut partial = original.clone();
        let owner = partial
            .rules
            .owners
            .iter_mut()
            .find(|o| &o.owner == subject)
            .unwrap();
        owner.programs.closure = if subject == &actor.owner {
            actor.programs.closure.clone()
        } else {
            SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: subject.clone(),
                    facet: SchemaFacet::GameRules,
                    code: key("unreviewed-incoming-life"),
                }],
            }
        };
        partial.rebind_test_edit().unwrap();
        let p = partial.compile().unwrap();
        refused(
            &p.evaluate(&mut p.new_scratch()).unwrap(),
            PlanGapReason::PartialPrograms,
        );
    }
    let mut missing = original;
    missing.rules.owners.retain(|o| o.owner != incoming);
    missing
        .stages
        .programs
        .members
        .retain(|p| p.owner != incoming);
    missing
        .stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .retain(|p| p.owner != incoming);
    missing.rebind_test_edit().unwrap();
    let p = missing.compile().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    refused(&r, PlanGapReason::MissingPrograms);
}

#[test]
fn life_delivery_requires_the_frozen_player_source_channel() {
    let mut early = load();
    early
        .stages
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key(RECEIVED))
        .unwrap()
        .stage = key("facts");
    let error = early
        .rebind_test_edit()
        .expect_err("delivery must follow the frozen incoming channel");
    assert!(
        error.contains("frozen channel read occurs before or outside frozen stage"),
        "{error}"
    );
}
