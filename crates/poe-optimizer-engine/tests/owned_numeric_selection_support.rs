//! Selection nodes first introduced by retained support programs join the same DAG.
#[allow(unused_imports)]
#[path = "support/owned_skill_participation_fixture.rs"]
mod support;
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*, owned_stages::*};
use poe_optimizer_engine::owned_plan::*;
use support::{base, def, key, known, subject};

#[test]
fn retained_support_queries_get_one_recipient_specific_cached_result() {
    let mut f = support::fixture();
    for name in ["late-override", "late-result"] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            },
        )));
        f.owners.push(DefinitionRules {
            owner: subject(def::<StatDefinition>(name)),
            programs: DeclaredSet::complete(vec![]),
        });
    }
    let inputs = support::inputs_with(
        &f,
        |r| {
            r.operations_version = key(OWNED_RULE_OPERATIONS_V27);
            r.contribution_queries = Some(DeclaredSet::complete(vec![ContributionQuery {
                id: key("late-overrides"),
                stat: def("late-override"),
                contribution: ContributionKind::Override,
                groups: vec![ContributionGroup {
                    id: key("all"),
                    reduction: ContributionReduction::RequireAgreement,
                    ordering: ContributionOrdering::Unordered,
                    empty: None,
                    members: DeclaredSet::complete(vec![]),
                }],
            }]));
            let program = r
                .owners
                .iter_mut()
                .find(|o| o.owner == support::delivery::support_owner())
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("actor-deliver"))
                .unwrap();
            for (name, projection, value_type) in [
                (
                    "override-present",
                    ContributionSelectionProjection::Present,
                    ComputedValueType::Boolean,
                ),
                (
                    "override-value",
                    ContributionSelectionProjection::Value,
                    ComputedValueType::Integer,
                ),
            ] {
                program.reads.push(RuleRead {
                    id: key(name),
                    value_type,
                    source: RuleReadSource::ContributionSelection {
                        entity: RuleEntity::Current,
                        query: key("late-overrides"),
                        group: key("all"),
                        projection,
                    },
                });
                program.nodes.push(base::read_node(name, name));
            }
            program.nodes.extend([
                base::literal("override-fallback", 42),
                base::node(
                    "override-result",
                    RuleExpression::Select {
                        condition: key("override-present"),
                        when_true: key("override-value"),
                        when_false: key("override-fallback"),
                    },
                ),
            ]);
            program.effects.push(base::effect(
                "override-result",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def("late-result"),
                    contribution: ContributionKind::Add,
                    value: key("override-result"),
                },
            ));
        },
        |s| {
            s.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: def("late-override"),
                    contribution: ContributionKind::Override,
                },
                stage: key("deliver"),
            })
        },
        |_| {},
    )
    .unwrap();
    let plan = support::readiness::compile_inputs(inputs).unwrap();
    let mut scratch = plan.new_scratch();
    let report = plan.evaluate(&mut scratch).unwrap();
    let effects = support::delivery::evaluated(&report);
    let selections: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| matches!(e.target, BoundEffectTarget::ContributionSelection { .. }))
        .collect();
    assert!(!selections.is_empty());
    for selection in &selections {
        assert_eq!(selection.value, EffectValue::Inactive);
    }
    let results: Vec<_> = effects.effects.iter().filter(|v| matches!(&v.target, BoundEffectTarget::Contribution { key } if key.stat == def::<StatDefinition>("late-result"))).collect();
    assert!(results.len() >= selections.len());
    for selection in selections {
        let BoundEffectTarget::ContributionSelection { key: selected } = &selection.target else {
            panic!()
        };
        assert!(results.iter().any(|result| matches!(&result.target, BoundEffectTarget::Contribution { key } if key.entity == selected.channel.entity)));
    }
    for result in results {
        assert_eq!(
            result.value,
            EffectValue::Known {
                value: base::integer(42)
            }
        );
    }
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), report);
}
