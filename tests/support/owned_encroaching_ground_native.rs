//! Published support bodies over two finite physical Ice occurrences. The cost
//! Product below is a test-owned complete subtotal, never final payable cost,
//! reservation, ground growth, or complete-build authority. It retains native
//! unrounded multiplication (1.3 * 1.1); PoB's separate four-place More rounding
//! is not implemented or claimed by this producer-only packet.
#[allow(dead_code)]
#[path = "owned_bidding_delivery_fixture.rs"]
mod bidding_fixture;
#[allow(dead_code)]
#[path = "owned_magnified_area_fixture.rs"]
mod fixture;

use fixture::{World, decode, def, id, key, quantity, subject};
use poe_optimizer_core::{
    build_identity::SupportAssignmentId, owned_build::*, owned_content::digest_owned,
    owned_definitions::*, owned_rules::*, owned_schema::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

const ENCROACHING: &str = "SupportEncroachingGroundPlayer";
const RAPID: &str = "SupportRapidCastingPlayer";
const SOURCES: [usize; 2] = [2, 3];
const AREA_PROGRAM: &str = "area-modifier-eligibility";

fn packet<T: DeserializeOwned>(family: &str, name: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68")
        .join(family)
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
#[derive(Serialize)]
struct ReceivingDigest<'a> {
    roles: &'a [SupportReceivingRole],
    targets: &'a [SupportTargetReceivingRoles],
    supports: &'a [SupportReceivingEntry],
}
fn authenticate_inherited(
    endpoint: &StagedOwnedRelease,
    family: &str,
) -> OwnedReleaseMigrationInput {
    let a: Value = packet(family, "authoring.json");
    let b: Value = packet(family, "bindings.json");
    let d: Value = packet(family, "dependencies.json");
    let m: OwnedReleaseMigrationInput = packet(family, "migration.json");
    let (kind, commitment) = match family {
        "action-area-eligibility" => (
            "action-area-eligibility-v1",
            digest_owned(
                "owned-action-area-eligibility-v1",
                &(&a, &b, &d, &m),
                8 * 1024 * 1024,
            )
            .unwrap(),
        ),
        "rapid-casting-support-delivery" => {
            let v: Value = packet(family, "source-vectors.json");
            let r: bidding_fixture::Receiving = packet(family, "receiving.json");
            let p: SupportPreparationInput = packet(family, "preparation.json");
            (
                "rapid-casting-support-delivery-v1",
                digest_owned(
                    "owned-rapid-casting-support-delivery-v1",
                    &(
                        &a,
                        &b,
                        &d,
                        &v,
                        &m,
                        ReceivingDigest {
                            roles: &r.roles,
                            targets: &r.targets,
                            supports: &r.supports,
                        },
                        &p,
                    ),
                    8 * 1024 * 1024,
                )
                .unwrap(),
            )
        }
        _ => panic!("only the two exact inherited publications"),
    };
    let rows: Vec<_> = endpoint
        .input()
        .provenance
        .iter()
        .filter(|p| p.kind == key(kind))
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].prior_input, m.before);
    assert_eq!(rows[0].authoring_input, commitment);
    for owner in &m.owners {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
    }
    m
}

fn world() -> World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD
        .get_or_init(|| {
            let path = PathBuf::from(
                std::env::var_os("POE_OPTIMIZER_TEST_ENCROACHING_RELEASE")
                    .expect("checked Encroaching publication"),
            );
            let before = crate::release::inventory(&path);
            let endpoint = crate::release::load(&path);
            crate::family::assert_endpoint(&endpoint);
            let m: OwnedReleaseMigrationInput = crate::family::read("migration.json");
            let b: Value = crate::family::read("bindings.json");
            let p: SupportPreparationInput = crate::family::read("preparation.json");
            let r: bidding_fixture::Receiving = crate::family::read("receiving.json");
            let mut w = World::load_release(&path, false);
            for field in [
                "physical_gem",
                "primary_skill",
                "entering_grant",
                "output",
                "part",
                "mode",
                "stat_sets",
            ] {
                assert_eq!(
                    b["target"][field], w.ice[field],
                    "exact physical Ice {field}"
                );
            }
            assert_eq!(
                b["channels"]["cost_factor"],
                w.bindings["channels"]["cost_factor"]
            );
            w.set_area_fact(None);
            assert!(w.area_usage.is_empty());
            let area = authenticate_inherited(&endpoint, "action-area-eligibility");
            for owner in area.owners {
                let program = owner
                    .programs
                    .members
                    .iter()
                    .find(|p| p.id == key(AREA_PROGRAM))
                    .unwrap();
                w.inner
                    .owner_mut(owner.owner)
                    .programs
                    .members
                    .push(program.clone());
            }
            let rapid = authenticate_inherited(&endpoint, "rapid-casting-support-delivery");
            let mut rb: Value = packet("rapid-casting-support-delivery", "bindings.json");
            rb["supports"]
                .as_array_mut()
                .unwrap()
                .retain(|s| s["source_effect"] == RAPID);
            let rg: GemDefId = decode(&rb["supports"][0]["gem"]);
            let mut rp: SupportPreparationInput =
                packet("rapid-casting-support-delivery", "preparation.json");
            rp.supports.retain(|s| s.gem == rg);
            let SchemaState::Known(prepared) = &rp.supports[0].preparation else {
                panic!("known Rapid preparation")
            };
            rp.effects = vec![prepared.effect.clone()];
            rp.families = prepared.families.clone().unwrap();
            let mut rr: bidding_fixture::Receiving =
                packet("rapid-casting-support-delivery", "receiving.json");
            rr.supports.retain(|s| s.gem == rg);
            let owners: Vec<_> = rapid
                .owners
                .into_iter()
                .filter(|o| o.owner == subject(rg.clone()))
                .collect();
            w.add_support_fragment(&endpoint, &rb, &owners, &rp, &rr);
            w.add_support_fragment(&endpoint, &b, &m.owners, &p, &r);

            // Source-proved finite admission inputs, separately on the assigned Gem
            // and actual receiving Skill. They are not an Area classifier or a
            // general skill-type producer. Actual Area eligibility remains above.
            for owner in [
                subject(decode::<GemDefId>(&w.ice["physical_gem"])),
                subject(decode::<SkillDefId>(&w.ice["primary_skill"])),
            ] {
                let facts = w
                    .inner
                    .owner_mut(owner)
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.id == key("fixture.initial-facts"))
                    .unwrap();
                for name in ["type.type.spell", "type.type.creates-ground-effect"] {
                    facts
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == key(name))
                        .unwrap()
                        .expression = RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    };
                }
            }
            w.inner.build.supports.clear();
            w.inner.build.support_origins = Some(vec![]);
            w.inner.build.gems.retain(|g| g.id == id(900));
            w.inner.build.skills.retain(|s| s.id == id(22));
            let mut gem = w.inner.build.gems[0].clone();
            gem.id = id(901);
            w.inner.build.gems.push(gem);
            let mut skill = w.inner.build.skills[0].clone();
            skill.id = id(23);
            skill.source = AuthoredSkillSource::Gem(id(901));
            w.inner.build.skills.push(skill);
            for source in SOURCES {
                for effect in [fixture::II, RAPID, ENCROACHING] {
                    w.inner.add_support(source, w.inner.support_index(effect));
                }
            }
            add_finite_subtotal(&mut w);
            assert_eq!(before, crate::release::inventory(&path));
            w
        })
        .clone()
}

fn add_finite_subtotal(w: &mut World) {
    let unit: UnitDefId = decode(&w.bindings["factor_unit"]);
    let stat: StatDefId = def("component.encroaching-cost-subtotal");
    w.inner
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: stat.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: unit.clone() },
                targets: vec![RuleEntityKind::Action],
            }),
        }));
    let cost: StatDefId = decode(&w.bindings["channels"]["cost_factor"]);
    let output = decode(&w.ice["output"]);
    w.inner
        .owner_mut(SchemaSubject::Slot(SlotAddress::ActionOutput(output)))
        .programs
        .members
        .push(RuleProgram {
            id: key("component.encroaching-cost-subtotal"),
            context: RuleEntityKind::Action,
            reads: vec![RuleRead {
                id: key("cost-factors"),
                value_type: ComputedValueType::Quantity { unit: unit.clone() },
                source: RuleReadSource::Contributions {
                    entity: RuleEntity::Current,
                    stat: cost,
                    contribution: ContributionKind::Multiply,
                    reduction: ContributionReduction::Product,
                    empty: quantity(1.0, &unit),
                },
            }],
            nodes: vec![RuleNode {
                id: key("subtotal"),
                expression: RuleExpression::Read {
                    input: key("cost-factors"),
                },
            }],
            effects: vec![RuleEffect {
                id: key("subtotal"),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat,
                    contribution: ContributionKind::Add,
                    value: key("subtotal"),
                },
            }],
        });
}
fn actions(w: &World, source: usize) -> Vec<ActionSelection> {
    w.actions(w.ice_source)
        .into_iter()
        .map(|mut a| {
            a.action.provider.root = ProviderRoot::SkillUse(id(20 + source as u64));
            a
        })
        .collect()
}
fn plan(w: &World) -> std::result::Result<bidding_fixture::Plan, String> {
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(w.inner.build.clone(), Default::default()).unwrap(),
        ScenarioSpec::new(
            ScenarioInput {
                game_version: bidding_fixture::ns(),
                enemy: EnemySpec {
                    encounter: def("fixture.encounter"),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
            Default::default(),
        )
        .unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: bidding_fixture::ns(),
                requests: SOURCES
                    .into_iter()
                    .flat_map(|s| actions(w, s))
                    .enumerate()
                    .map(|(i, a)| MetricRequest {
                        id: QueryId::new(format!("encroaching-{i}")).unwrap(),
                        metric: def("fixture.observe"),
                        target: MetricTarget::Action(Box::new(a)),
                    })
                    .collect(),
            },
            Default::default(),
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    w.checked_plan_with_request(request)
}
fn evaluate(w: &World) -> SupportEffectsReport {
    let p = plan(w).unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn assignment(w: &World, source: usize, effect: &str) -> SupportAssignmentId {
    let gem = w.inner.support_gem(effect);
    let rows: Vec<_> = w
        .inner
        .build
        .supports
        .iter()
        .filter(|s| {
            s.target == SkillTarget::Authored(id(20 + source as u64))
                && w.inner
                    .build
                    .gems
                    .iter()
                    .any(|g| g.id == s.support && g.definition == gem)
        })
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0].id
}
fn disable(w: &mut World, assignment: SupportAssignmentId) {
    w.inner
        .build
        .supports
        .iter_mut()
        .find(|s| s.id == assignment)
        .unwrap()
        .enabled = false;
}
fn check(w: &World, report: &SupportEffectsReport, encroaching: [Option<SupportAssignmentId>; 2]) {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite component: {report:?}")
    };
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    let cost: StatDefId = decode(&w.bindings["channels"]["cost_factor"]);
    let area: StatDefId = decode(&w.bindings["channels"]["area_eligible"]);
    let ratio: UnitDefId = decode(&w.bindings["factor_unit"]);
    let speed: StatDefId = def("def.00000000000032fc");
    for (source, winner) in SOURCES.into_iter().zip(encroaching) {
        let mag = assignment(w, source, fixture::II);
        let rapid = assignment(w, source, RAPID);
        let actions = actions(w, source);
        assert_eq!(actions.len(), 2);
        assert_ne!(actions[0].stat_set, actions[1].stat_set);
        for action in actions {
            let entity = ConcreteEntity::Action(Box::new(action.clone()));
            let flags: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key(AREA_PROGRAM)
                        && e.key.invocation.entity == entity
                })
                .collect();
            assert_eq!(flags.len(), 1);
            assert!(
                matches!(&flags[0].target, BoundEffectTarget::Value { key: PlanValueKey::Stat { stat, .. } } if stat == &area)
            );
            assert_eq!(
                flags[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                }
            );
            let rows: Vec<_> = effects.effects.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.entity == entity && key.stat == cost)).collect();
            assert_eq!(rows.len(), 1 + usize::from(winner.is_some()));
            for (origin, expected, position) in [(Some(mag), 1.3, 0), (winner, 1.1, 2)] {
                let Some(origin) = origin else { continue };
                let joined: Vec<_> = rows.iter().filter(|e| matches!(&e.key.invocation.origin, RuleOrigin::SupportApplication { application } if application.prepared.origin == SupportOrigin::Assignment(origin))).collect();
                assert_eq!(joined.len(), 1);
                let row = joined[0];
                let RuleOrigin::SupportApplication { application } = &row.key.invocation.origin
                else {
                    unreachable!()
                };
                assert_eq!(
                    application.prepared.target,
                    SkillTarget::Authored(id(20 + source as u64))
                );
                assert_eq!(application.prepared.position, position);
                assert_eq!(
                    application.receiver,
                    SupportReceiverKey::Action(Box::new(action.clone()))
                );
                assert_eq!(
                    row.value,
                    EffectValue::Known {
                        value: quantity(expected, &ratio)
                    }
                );
                assert!(
                    matches!(&row.target, BoundEffectTarget::Contribution { key } if key.kind == ContributionKind::Multiply)
                );
            }
            let rapid_rows: Vec<_> = effects.effects.iter().filter(|e| matches!(&e.key.invocation.origin, RuleOrigin::SupportApplication { application } if application.prepared.origin == SupportOrigin::Assignment(rapid) && application.receiver == SupportReceiverKey::Action(Box::new(action.clone())))).collect();
            let rapid_contributions: Vec<_> = rapid_rows
                .iter()
                .filter(|e| matches!(e.target, BoundEffectTarget::Contribution { .. }))
                .collect();
            assert_eq!(
                rapid_contributions.len(),
                1,
                "Rapid contributes only cast speed, no identity cost record"
            );
            assert!(
                matches!(&rapid_contributions[0].target, BoundEffectTarget::Contribution { key } if key.stat == speed && key.kind == ContributionKind::Increase)
            );
            let subtotal: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key("component.encroaching-cost-subtotal")
                        && e.key.invocation.entity == entity
                })
                .collect();
            assert_eq!(subtotal.len(), 1);
            assert_eq!(
                subtotal[0].value,
                EffectValue::Known {
                    value: quantity(if winner.is_some() { 1.3 * 1.1 } else { 1.3 }, &ratio)
                }
            );
        }
    }
    for row in &effects.effects {
        if let BoundEffectTarget::Contribution { key } = &row.target {
            assert!(
                matches!(&key.entity, ConcreteEntity::Action(a) if SOURCES.into_iter().flat_map(|s|actions(w,s)).any(|expected| expected==**a)),
                "no Player/other-receiver contribution leak"
            );
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_ENCROACHING_RELEASE; finite component only"]
fn encroaching_and_magnified_multiply_on_both_sets_with_actual_area_guards() {
    let w = world();
    assert_ne!(
        w.inner.build.skills[0].source,
        w.inner.build.skills[1].source
    );
    check(
        &w,
        &evaluate(&w),
        SOURCES.map(|s| Some(assignment(&w, s, ENCROACHING))),
    );
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_ENCROACHING_RELEASE; finite component only"]
fn encroaching_removal_and_disable_preserve_the_other_receiver_and_magnified() {
    for source in SOURCES {
        for remove in [false, true] {
            let mut w = world();
            let mut expected = SOURCES.map(|s| Some(assignment(&w, s, ENCROACHING)));
            let target = expected[source - 2].take().unwrap();
            if remove {
                let gem = w
                    .inner
                    .build
                    .supports
                    .iter()
                    .find(|s| s.id == target)
                    .unwrap()
                    .support;
                w.inner.build.supports.retain(|s| s.id != target);
                w.inner.build.gems.retain(|g| g.id != gem);
                for sequence in w.inner.build.support_origins.as_mut().unwrap() {
                    sequence
                        .origins
                        .retain(|o| *o != SupportOrigin::Assignment(target));
                }
            } else {
                disable(&mut w, target);
            }
            check(&w, &evaluate(&w), expected);
        }
    }
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_ENCROACHING_RELEASE; finite component only"]
fn encroaching_duplicate_quality_retains_one_exact_source_position() {
    let mut w = world();
    let first = assignment(&w, 2, ENCROACHING);
    let other = assignment(&w, 3, ENCROACHING);
    w.inner.add_support(2, w.inner.support_index(ENCROACHING));
    let last = w.inner.build.supports.last().unwrap().clone();
    for (quality, winner) in [(0.0, first), (15.0, last.id)] {
        w.inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == last.support)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(quality, w.inner.quality_unit.clone()).unwrap();
        check(&w, &evaluate(&w), [Some(winner), Some(other)]);
    }
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_ENCROACHING_RELEASE; finite component only"]
fn encroaching_actual_partial_owner_and_receiving_do_not_gain_completeness() {
    let mut w = world();
    let m: OwnedReleaseMigrationInput = crate::family::read("migration.json");
    assert_eq!(m.owners.len(), 1);
    assert!(!m.owners[0].programs.is_complete());
    w.inner
        .owner_mut(m.owners[0].owner.clone())
        .programs
        .closure = m.owners[0].programs.closure.clone();
    assert!(plan(&w).is_err());
    let mut w = world();
    let r: bidding_fixture::Receiving = crate::family::read("receiving.json");
    let target = &r.targets[0];
    assert!(!target.roles.is_complete());
    w.inner
        .receiving
        .targets
        .iter_mut()
        .find(|t| t.owner == target.owner)
        .unwrap()
        .roles
        .closure = target.roles.closure.clone();
    if let Ok(p) = plan(&w) {
        assert!(!matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Evaluated { .. }
        ));
    }
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_ENCROACHING_RELEASE; finite component only"]
fn encroaching_scratch_a_b_a_and_private_rayon_keep_exact_receivers() {
    let a = world();
    let winners = SOURCES.map(|s| Some(assignment(&a, s, ENCROACHING)));
    let mut b = a.clone();
    disable(&mut b, winners[0].unwrap());
    let pa = Arc::new(plan(&a).unwrap());
    let pb = plan(&b).unwrap();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, &first, winners);
    check(&b, &pb.evaluate(&mut scratch).unwrap(), [None, winners[1]]);
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(|| pa.new_scratch(), |s, _| pa.evaluate(s).unwrap())
            .collect::<Vec<_>>()
    });
    assert!(reports.iter().all(|r| r == &first));
}
