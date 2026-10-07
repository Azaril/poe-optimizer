//! Nonempty native support suffixes must join the same checked flag census.
#[allow(unused_imports)]
#[path = "support/owned_skill_participation_fixture.rs"]
mod support;
use poe_optimizer_core::{owned_build::*, owned_rules::*, owned_schema::*, owned_stages::*};
use poe_optimizer_engine::owned_plan::*;
use support::{base, def, key, known, subject};

fn fixture() -> support::Fixture {
    let mut f = support::fixture();
    f.schema.definitions.push(DefinitionDescriptor::Stat(known(
        def("late-flag"),
        StatSchema {
            value: ComputedValueType::Boolean,
            targets: vec![RuleEntityKind::Actor],
        },
    )));
    f.owners.push(DefinitionRules {
        owner: subject(def::<poe_optimizer_core::owned_definitions::StatDefinition>("late-flag")),
        programs: DeclaredSet::complete(vec![]),
    });
    f
}
fn add_flag(rules: &mut RulePackageInput, active: bool) {
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    rules.contribution_queries = Some(DeclaredSet::complete(vec![ContributionQuery {
        id: key("late-flag-census"),
        stat: def("late-flag"),
        contribution: ContributionKind::Flag,
        groups: vec![ContributionGroup {
            id: key("all"),
            reduction: ContributionReduction::Any,
            ordering: ContributionOrdering::Unordered,
            empty: ParameterValue::Boolean(false),
            members: DeclaredSet::complete(vec![]),
        }],
    }]));
    let program = rules
        .owners
        .iter_mut()
        .find(|o| o.owner == support::delivery::support_owner())
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("actor-deliver"))
        .unwrap();
    program.nodes.extend([
        base::bool_node("flag-value", true),
        base::bool_node("flag-active", active),
    ]);
    program.effects.push(RuleEffect {
        id: key("late-flag"),
        when: Some(key("flag-active")),
        effect: RuleEffectKind::Contribute {
            entity: RuleEntity::Current,
            stat: def("late-flag"),
            contribution: ContributionKind::Flag,
            value: key("flag-value"),
        },
    });
}
#[test]
fn appended_active_and_inactive_support_flags_cannot_escape_an_unread_query_inventory() {
    for active in [true, false] {
        let f = fixture();
        let inputs = support::inputs_with(&f, |r| add_flag(r, active), |_| {}, |_| {}).unwrap();
        let plan = support::readiness::compile_inputs(inputs).unwrap();
        let error = plan.evaluate(&mut plan.new_scratch()).unwrap_err();
        assert!(
            matches!(error, PlanError::Invalid(message) if message.contains("no declared membership")),
            "nonempty support suffix must be checked before runtime inactivity"
        );
    }
}
#[test]
fn flag_contribution_channels_preserve_frozen_stage_authority() {
    let f = fixture();
    let inputs = support::inputs_with(
        &f,
        |r| add_flag(r, false),
        |stages| {
            stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: def("late-flag"),
                    contribution: ContributionKind::Flag,
                },
                stage: key("prepare"),
            });
        },
        |_| {},
    );
    assert!(
        inputs.is_err(),
        "a later inactive flag still cannot write a frozen contribution channel"
    );
}
