//! Exact Actor-profile producer joined to the existing item/preparation graph.
//! This retains an individual added-damage factor, not a final aggregate or DPS.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
};

const PROFILE_STAGE: &str = "intrinsic-added-damage-profile";
const ACTION_STAGE: &str = "intrinsic-added-damage-action";

pub(super) fn install(w: &mut sniper::World, endpoint: &StagedOwnedRelease) {
    added_damage_family::assert_component(endpoint);
    let inner = &mut w.base.source.base.inner;
    for entry in added_damage_family::migration().schema {
        let SchemaExtensionEntry::Definition(descriptor) = entry else {
            panic!("intrinsic source adds stat definitions only")
        };
        if let Some(existing) = inner
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == descriptor.address())
        {
            assert_eq!(existing, &descriptor);
        } else {
            inner.schema.definitions.push(descriptor.clone());
        }
        inner.owner_mut(SchemaSubject::Definition(descriptor.address()));
    }
    for owner in added_damage_family::consumer().owners {
        let existing = inner.owner_mut(owner.owner);
        for program in owner.programs.members {
            assert!(!existing.programs.members.iter().any(|p| p.id == program.id));
            existing.programs.members.push(program);
        }
    }
    for route in added_damage_family::routes() {
        let existing = w
            .base
            .action_routes
            .iter_mut()
            .find(|r| r.output == route.output)
            .unwrap();
        for member in route.routes.members {
            assert!(!existing.routes.members.iter().any(|r| r.id == member.id));
            existing.routes.members.push(member);
        }
    }
}

pub(super) fn configure(s: &mut EvaluationStagesInput) {
    s.stages.extend([
        EvaluationStage {
            id: key(PROFILE_STAGE),
            predecessors: vec![key("facts")],
        },
        EvaluationStage {
            id: key(ACTION_STAGE),
            predecessors: vec![s.routing_stage.clone()],
        },
    ]);
    // Routes execute in the graph's shared routing stage. Bind both sides:
    // the Actor producer precedes routing, then the Action consumes its result.
    s.stages
        .iter_mut()
        .find(|stage| stage.id == s.routing_stage)
        .unwrap()
        .predecessors
        .push(key(PROFILE_STAGE));
    for row in &mut s.programs.members {
        row.stage = match row.program.as_str() {
            added_damage_family::PROFILE_PROGRAM => key(PROFILE_STAGE),
            added_damage_family::ACTION_PROGRAM => key(ACTION_STAGE),
            _ => row.stage.clone(),
        };
    }
    s.frozen_channels.extend([
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x2537),
            },
            stage: key("facts"),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x336b),
            },
            stage: key(PROFILE_STAGE),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Action,
                stat: d(0x336c),
            },
            stage: s.routing_stage.clone(),
        },
    ]);
}

#[test]
#[ignore = "requires the current intrinsic added-damage release; joined native graph"]
fn intrinsic_added_damage_keeps_exact_source_and_action_occurrences() {
    let w = World::load();
    let report = w.evaluate();
    let effects = sniper::offering::effects(&report);
    for index in 0..2 {
        let raw = (1.15_f64 - 1.) * 100.;
        for (action, stat) in [(false, 0x336b), (true, 0x336c)] {
            assert_eq!(
                w.value(effects, index, action, stat),
                &EffectValue::Known {
                    value: quantity(raw, &d(2))
                }
            );
        }
        let action = w.action(index);
        let rows: Vec<_> = effects.effects.iter().filter(|e| matches!(&e.target,
            BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Action(Box::new(action.clone()))
                && key.stat == d(0x336d) && key.kind == ContributionKind::Multiply
        )).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].key.invocation.program,
            key(added_damage_family::ACTION_PROGRAM)
        );
        assert_eq!(
            rows[0].value,
            EffectValue::Known {
                value: quantity(1.15, &d(1))
            }
        );
    }
    assert_eq!(
        effects
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key(added_damage_family::ACTION_PROGRAM))
            .count(),
        2,
        "no contribution to the three Gas modes on either Actor"
    );
}
