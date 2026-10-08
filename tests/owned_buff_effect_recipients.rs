//! Publish existing Actor-scoped recipient channels with a checked empty domain.
#[allow(dead_code)]
#[path = "support/owned_buff_effect_recipients.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_rules::*, owned_schema::RuleEntityKind,
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn recipient_buff_effect_authoring_uses_exact_actor_queries_without_neutral_writers() {
    family::check_authored();
    let programs = family::programs();
    assert_eq!(programs.len(), 2);
    assert!(programs.iter().all(|p| {
        p.program
            .nodes
            .iter()
            .all(|n| !matches!(n.expression, RuleExpression::Literal { .. }))
    }));
    assert!(family::queries().iter().all(|q| q.groups.len() == 1
        && q.groups[0].members.is_complete()
        && q.groups[0].members.members.is_empty()));
}

#[test]
fn recipient_buff_effect_authoring_rejects_scope_bypass_and_nonempty_domain() {
    for change in 0..5 {
        let mut programs = family::programs();
        match change {
            0 => {
                let RuleReadSource::ContributionQuery { entity, .. } =
                    &mut programs[0].program.reads[0].source
                else {
                    unreachable!()
                };
                *entity = RuleEntity::Player;
            }
            1 => {
                programs[0].program.nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Boolean(false),
                }
            }
            2 => programs[0].program.context = RuleEntityKind::Skill,
            3 => programs[1].program.effects[0].when = Some("resolved".parse().unwrap()),
            4 => {
                let RuleEffectKind::Derive { entity, .. } =
                    &mut programs[1].program.effects[0].effect
                else {
                    unreachable!()
                };
                *entity = RuleEntity::Player;
            }
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_programs(&programs)).is_err());
    }
    for changed in 0..2 {
        let mut queries = family::queries();
        let producer = family::programs().remove(0);
        queries[changed].groups[0]
            .members
            .members
            .push(ContributionMember {
                owner: producer.owner,
                program: "fixture-unreviewed-producer".parse().unwrap(),
                effect: "value".parse().unwrap(),
                origin: ContributionOrigin::Character,
                order: Some(ContributionOrder {
                    source_rank: 0,
                    program_rank: 0,
                    effect_rank: 0,
                    slot_ranks: vec![],
                }),
            });
        assert!(
            std::panic::catch_unwind(|| family::check_queries(&queries)).is_err(),
            "no arbitrary nonempty reduction law"
        );
    }
}

#[test]
#[ignore = "requires retained full source reports; authenticates observations without source execution"]
fn recipient_buff_effect_authenticates_retained_source_observations() {
    family::authenticate_source();
}

#[test]
#[ignore = "requires BUFF_EFFECT_RECIPIENTS_PRIOR and fresh BUFF_EFFECT_RECIPIENTS_OUTPUT"]
fn publish_recipient_buff_effect_preserving_all_five_originals() {
    let prior = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_BUFF_EFFECT_RECIPIENTS_PRIOR")
            .expect("checked predecessor"),
    );
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_BUFF_EFFECT_RECIPIENTS_OUTPUT")
            .expect("fresh immutable output"),
    );
    publication::run_with_expected_selected_counts(
        prior.clone(),
        output.clone(),
        &family::data(),
        &[],
        &[
            "bindings.json",
            "programs.json",
            "queries.json",
            "dependencies.json",
            "source-vectors.json",
        ],
        family::stage,
        json!({"new_definitions":0,"new_programs":2,"new_queries":2,
            "supported_empty_domains":2,"new_stat_receivers":0,"closed_existing_rule_owners":0,
            "real_producer_absence_claim":false,"nonempty_composition_supported":false,
            "full_input_inverse":true,"engine_policy_changes":0}),
        [107, 117, 109, 123, 5],
    );
    family::assert_transition(
        &release::load(&prior),
        &release::load(&output.join("package")),
    );
}
