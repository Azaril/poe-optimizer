//! Authored Bidding numerical programs in a deliberately finite actual-Djinn
//! graph. This proves support-origin/recipient behavior, never full-build parity.
#[path = "owned_bidding_delivery_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    build_identity::SupportAssignmentId, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::{collections::BTreeSet, sync::Arc};

fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite component must evaluate: {report:?}")
    };
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    effects
}
fn application(e: &BoundEffectResult) -> Option<&SupportApplicationKey> {
    match &e.key.invocation.origin {
        RuleOrigin::SupportApplication { application } => Some(application),
        _ => None,
    }
}
fn quantity_value(value: &EffectValue, amount: f64, unit: &UnitDefId) {
    assert_eq!(
        value,
        &EffectValue::Known {
            value: quantity(amount, unit)
        }
    );
}
fn contribution<'a>(
    r: &'a OwnedEffectsReport,
    a: &ActionSelection,
    stat: &StatDefId,
) -> Vec<&'a BoundEffectResult> {
    r.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Action(Box::new(a.clone())) && &key.stat==stat)).collect()
}
fn check(w: &World, r: &OwnedEffectsReport, tiers: &[&str]) {
    let selected: Vec<_> = tiers
        .iter()
        .enumerate()
        .map(|(source, effect)| {
            let matches: Vec<_> = w
                .build
                .supports
                .iter()
                .filter(|a| {
                    a.target == SkillTarget::Authored(id(20 + source as u64))
                        && w.build
                            .gems
                            .iter()
                            .any(|g| g.id == a.support && g.definition == w.support_gem(effect))
                })
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "repeated sources require an explicit expected winner"
            );
            matches[0].id
        })
        .collect();
    check_selected(w, r, tiers, &selected);
}
fn check_selected(
    w: &World,
    r: &OwnedEffectsReport,
    tiers: &[&str],
    selected: &[SupportAssignmentId],
) {
    assert_eq!(tiers.len(), w.sources.len());
    assert_eq!(selected.len(), tiers.len());
    let cooldown: StatDefId = decode(&w.channels["cooldown"]);
    let damage: StatDefId = decode(&w.channels["damage_factor"]);
    let percent: UnitDefId = decode(&w.bindings["percent_unit"]);
    let mut exact_applications = BTreeSet::new();
    for (source, effect) in tiers.iter().enumerate() {
        let target = SkillTarget::Authored(id(20 + source as u64));
        let assignment = w
            .build
            .supports
            .iter()
            .find(|a| a.id == selected[source])
            .unwrap();
        assert_eq!(assignment.target, target);
        assert!(
            w.build
                .gems
                .iter()
                .any(|g| g.id == assignment.support && g.definition == w.support_gem(effect))
        );
        let expected_cooldown = match *effect {
            "SupportBiddingPlayerTwo" => 30.0,
            "SupportBiddingPlayerThree" => 80.0,
            _ => panic!("test tier"),
        };
        for action in w.actions(source) {
            let c = contribution(r, &action, &cooldown);
            let d = contribution(r, &action, &damage);
            if action.action.actor == ActorKey::Player {
                assert!(
                    c.is_empty() && d.is_empty(),
                    "MinionModifier must not leak to admitted Player Command action"
                );
                assert!(!r.effects.iter().any(|e| application(e).is_some_and(
                    |a| matches!(&a.receiver,SupportReceiverKey::Action(v) if v.as_ref()==&action)
                )));
                continue;
            }
            assert_eq!(c.len(), 1, "one retained source per exact child action");
            assert_eq!(
                d.len(),
                usize::from(*effect == "SupportBiddingPlayerTwo"),
                "III owns no damage contribution, including no identity-factor surrogate"
            );
            let app = application(c[0]).expect("support application provenance");
            assert_eq!(
                app.prepared.origin,
                SupportOrigin::Assignment(assignment.id)
            );
            assert_eq!(app.prepared.target, target);
            assert_eq!(app.prepared.position, 0);
            assert_eq!(
                app.receiver,
                SupportReceiverKey::Action(Box::new(action.clone()))
            );
            assert!(exact_applications.insert(app.clone()));
            let applicability = r
                .values
                .iter()
                .find(|v| {
                    v.key
                        == PlanValueKey::SupportApplicability {
                            application: Box::new(app.clone()),
                        }
                })
                .unwrap();
            assert_eq!(
                applicability.value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                },
                "delivery condition is distinct from native admission"
            );
            for e in c.iter().chain(&d) {
                assert_eq!(application(e), Some(app));
                assert_eq!(e.key.invocation.program, key("minion-command-delivery"));
                let BoundEffectTarget::Contribution { key: contribution } = &e.target else {
                    unreachable!()
                };
                assert_eq!(
                    contribution.kind,
                    if contribution.stat == cooldown {
                        ContributionKind::Increase
                    } else {
                        ContributionKind::Multiply
                    }
                );
                if w.commandable(&action) {
                    quantity_value(
                        &e.value,
                        if contribution.stat == cooldown {
                            expected_cooldown
                        } else {
                            1.3
                        },
                        if contribution.stat == cooldown {
                            &percent
                        } else {
                            &w.factor_unit
                        },
                    );
                } else {
                    assert_eq!(
                        e.value,
                        EffectValue::Inactive,
                        "actual Water passive cannot acquire a command modifier"
                    );
                }
            }
        }
    }
    assert_eq!(
        exact_applications.len(),
        tiers
            .iter()
            .enumerate()
            .map(|(i, _)| w.actions(i).len() - 1)
            .sum::<usize>()
    );
    for e in &r.effects {
        if let Some(app) = application(e) {
            assert!(
                exact_applications.contains(app),
                "no extra actor/source/Player application"
            );
        }
    }
}
fn report(w: &World) -> SupportEffectsReport {
    let p = w.plan();
    p.evaluate(&mut p.new_scratch()).unwrap()
}

#[test]
#[ignore = "requires exact checked predecessor and authenticated Bidding packet; finite component only"]
fn bidding_actual_ii_iii_programs_reach_exact_djinn_actions_without_player_leak() {
    let mut w = World::load();
    let r = report(&w);
    check(&w, effects(&r), &["SupportBiddingPlayerTwo"; 2]);
    // All ten child variants and both separately queried Player Commands retain
    // their actual output/part/mode/stat-set and actor-provider paths.
    assert_eq!(w.actions(0).len(), 6);
    assert_eq!(w.actions(1).len(), 6);
    assert!(
        w.actions(0)
            .iter()
            .filter(|a| a.action.actor != ActorKey::Player)
            .all(|a| a.action.provider.grant_path.len() == 2)
    );
    w.change_support(0, "SupportBiddingPlayerThree");
    w.change_support(1, "SupportBiddingPlayerThree");
    let r = report(&w);
    check(&w, effects(&r), &["SupportBiddingPlayerThree"; 2]);
}

#[test]
#[ignore = "requires exact checked predecessor and authenticated Bidding packet; finite component only"]
fn bidding_repeated_physical_sources_and_family_selection_preserve_occurrence_identity() {
    let mut w = World::load();
    let ii = w.support_index("SupportBiddingPlayerTwo");
    w.add_source(0, ii);
    let r = report(&w);
    check(&w, effects(&r), &["SupportBiddingPlayerTwo"; 3]);
    assert_ne!(w.actions(0)[1].action.actor, w.actions(2)[1].action.actor);
    assert_eq!(w.actions(0)[1].action.output, w.actions(2)[1].action.output);
    let first = w.build.supports[0].id;
    let water = w.build.supports[1].id;
    let second_sand = w.build.supports[2].id;
    // The source witness's same-definition controls retain the first equal-
    // quality occurrence, but replace it with the later quality-15 occurrence.
    // Every child must inherit the one selected physical source and position.
    w.add_support(0, ii);
    let duplicate = w.build.supports.last().unwrap().id;
    let duplicate_gem = w.build.supports.last().unwrap().support;
    for (quality, winner) in [(0.0, first), (15.0, duplicate)] {
        w.build
            .gems
            .iter_mut()
            .find(|g| g.id == duplicate_gem)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(quality, w.quality_unit.clone()).unwrap();
        let r = report(&w);
        check_selected(
            &w,
            effects(&r),
            &["SupportBiddingPlayerTwo"; 3],
            &[winner, water, second_sand],
        );
    }

    // Different definitions in the shared family use ordered replacement.
    // Reverse only the explicit origin sequence: storage order and physical
    // identities stay fixed, and the last family member wins in both orders.
    let mut mixed = World::load();
    let ii_origin = mixed.build.supports[0].id;
    let water = mixed.build.supports[1].id;
    mixed.add_support(0, mixed.support_index("SupportBiddingPlayerThree"));
    let iii_origin = mixed.build.supports.last().unwrap().id;
    let original_assignments = mixed.build.supports.clone();
    let original_gems = mixed.build.gems.clone();
    for (order, winner, effect) in [
        (
            [ii_origin, iii_origin],
            iii_origin,
            "SupportBiddingPlayerThree",
        ),
        (
            [iii_origin, ii_origin],
            ii_origin,
            "SupportBiddingPlayerTwo",
        ),
    ] {
        mixed
            .build
            .support_origins
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row.target == SkillTarget::Authored(id(20)))
            .unwrap()
            .origins = order.into_iter().map(SupportOrigin::Assignment).collect();
        let r = report(&mixed);
        check_selected(
            &mixed,
            effects(&r),
            &[effect, "SupportBiddingPlayerTwo"],
            &[winner, water],
        );
        assert_eq!(mixed.build.supports, original_assignments);
        assert_eq!(mixed.build.gems, original_gems);
    }
}

#[test]
#[ignore = "requires exact checked predecessor and authenticated Bidding packet; finite component only"]
fn bidding_admission_and_ancestor_activation_are_not_inferred_from_receiver_names() {
    let mut rejected = World::load();
    for root in rejected.families.clone() {
        let skill: SkillDefId = decode(&root["skill"]);
        let p = rejected
            .owner_mut(subject(skill))
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key("fixture.initial-facts"))
            .unwrap();
        for name in ["type.type.commandable-minion", "type.type.commands-minions"] {
            p.nodes
                .iter_mut()
                .find(|n| n.id == key(name))
                .unwrap()
                .expression = RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            };
        }
    }
    let r = report(&rejected);
    let r = effects(&r);
    assert!(
        r.effects
            .iter()
            .filter(|e| application(e).is_some())
            .all(
                |e| !matches!(e.target, BoundEffectTarget::Contribution { .. })
                    || e.value == EffectValue::Inactive
            )
    );
    assert!(r.values.iter().any(
        |v| matches!(v.key, PlanValueKey::SupportApplicability { .. })
            && v.value
                == EffectValue::Known {
                    value: ParameterValue::Boolean(false)
                }
    ));
    let mut disabled = World::load();
    disabled.build.skills[1].enabled = false;
    let r = report(&disabled);
    // All original requests remain present. Querying the six unavailable Water
    // actions must refuse evaluation, rather than manufacture zero contributions.
    assert!(matches!(
        r.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let unavailable = disabled.actions(1);
    assert_eq!(r.gaps.len(), unavailable.len());
    for action in unavailable {
        assert!(
            r.gaps.iter().any(|gap| {
                gap.reason == PlanGapReason::UnresolvedTopology
                    && gap.provider.as_ref() == Some(&action.action.provider)
                    && gap.subject
                        == Some(SchemaSubject::Slot(SlotAddress::ActionOutput(
                            action.action.output.clone(),
                        )))
            }),
            "missing exact disabled-action refusal: {action:?}"
        );
    }
}

#[test]
#[ignore = "requires exact checked predecessor and authenticated Bidding packet; finite component only"]
fn bidding_missing_inputs_facts_and_partial_inventories_never_become_empty_success() {
    let w = World::load();
    let mut raw = w.clone();
    let removed = raw.build.skills[0]
        .parameters
        .as_mut()
        .unwrap()
        .pop()
        .unwrap();
    assert!(matches!(&removed.value,ParameterValue::Quantity(q) if q.unit()==&w.quality_unit));
    let result = raw.checked_plan();
    if let Ok(p) = result {
        assert!(
            !matches!(
                p.evaluate(&mut p.new_scratch()).unwrap().outcome,
                SupportEffectsOutcome::Evaluated { .. }
            ),
            "missing raw quality cannot admit complete delivery"
        );
    }
    let mut absent = w.clone();
    let output = absent.actions(0)[1].action.output.clone();
    absent
        .owner_mut(SchemaSubject::Slot(SlotAddress::ActionOutput(output)))
        .programs
        .members
        .clear();
    let p = absent.plan();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    match &r.outcome {
        SupportEffectsOutcome::Evaluated { effects } => {
            assert!(
                effects.effects.iter().any(|e| application(e).is_some()
                    && matches!(e.value, EffectValue::Unresolved { .. })),
                "missing commandable proof is not false"
            )
        }
        SupportEffectsOutcome::Unavailable { .. }
        | SupportEffectsOutcome::PreparationUnresolved { .. } => {}
    }
    let mut ancestry = w.clone();
    let root_skill: SkillDefId = decode(&ancestry.families[0]["skill"]);
    let entering: DeclaredSlot<GrantSlotDefId> =
        decode(&ancestry.families[0]["minion"]["entering_grant"]);
    ancestry
        .owner_mut(subject(root_skill))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("fixture.activation"))
        .unwrap()
        .effects
        .retain(|e| !matches!(&e.effect,RuleEffectKind::ActivateGrant{slot,..} if slot==&entering));
    let p = ancestry.plan();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        !matches!(r.outcome, SupportEffectsOutcome::Evaluated { .. }),
        "missing entering activation is not a known inactive actor"
    );
    let mut partial = w.clone();
    let gem = partial.support_gem("SupportBiddingPlayerTwo");
    let closure = partial
        .original_owners
        .iter()
        .find(|o| o.owner == subject(gem.clone()))
        .unwrap()
        .programs
        .closure
        .clone();
    partial.owner_mut(subject(gem)).programs.closure = closure;
    let Err(error) = partial.checked_plan() else {
        panic!("a Partial owner cannot authorize fixture early preparation facts")
    };
    assert!(
        error.contains("early readiness needs an early phase and complete owner programs"),
        "{error}"
    );
    // Restore the exact actual catalogue exposure, not a fabricated Known Skill
    // descriptor. Every unresolved source remains a production integration gate.
    let mut catalogue = w.clone();
    catalogue.change_support(1, "SupportBiddingPlayerThree");
    for (gem, original) in catalogue.original_gem_skills.clone() {
        let DefinitionDescriptor::Gem(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = catalogue
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == gem.address())
            .unwrap()
        else {
            panic!("actual Gem")
        };
        schema.skills = original;
    }
    let p = catalogue.plan();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(!matches!(
        r.outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
    for assignment in &catalogue.build.supports {
        let gem = &catalogue
            .build
            .gems
            .iter()
            .find(|g| g.id == assignment.support)
            .unwrap()
            .definition;
        let original = &catalogue
            .original_gem_skills
            .iter()
            .find(|(id, _)| id == gem)
            .unwrap()
            .1;
        assert_eq!(original.members.len(), 1);
        assert!(r.gaps.iter().any(|g| {
            g.reason == PlanGapReason::UnresolvedActivation
                && g.provider.as_ref().is_some_and(|p| {
                    p.root == ProviderRoot::SupportAssignment(assignment.id)
                        && p.grant_path.is_empty()
                })
                && g.subject == Some(subject(original.members[0].clone()))
        }));
    }
    let mut routes = w;
    routes.receiving = routes.original_receiving.clone();
    let p = routes.plan();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(!r.gaps.is_empty());
    assert!(!matches!(
        r.outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
}

#[test]
#[ignore = "requires exact checked predecessor and authenticated Bidding packet; finite component only"]
fn bidding_candidate_changes_reuse_scratch_and_parallel_workers_deterministically() {
    let a = World::load();
    let mut b = a.clone();
    b.change_support(0, "SupportBiddingPlayerThree");
    let plan_a = Arc::new(a.plan());
    let plan_b = b.plan();
    let mut scratch = plan_a.new_scratch();
    let first = plan_a.evaluate(&mut scratch).unwrap();
    check(&a, effects(&first), &["SupportBiddingPlayerTwo"; 2]);
    let changed = plan_b.evaluate(&mut scratch).unwrap();
    check(
        &b,
        effects(&changed),
        &["SupportBiddingPlayerThree", "SupportBiddingPlayerTwo"],
    );
    assert_eq!(first, plan_a.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || plan_a.new_scratch(),
                |scratch, _| plan_a.evaluate(scratch).unwrap(),
            )
            .collect::<Vec<_>>()
    });
    assert!(parallel.iter().all(|r| r == &first));
}
