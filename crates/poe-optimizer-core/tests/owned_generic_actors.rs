use poe_optimizer_core::{
    owned_build::{ActorKey, AssumptionTarget},
    owned_draft::{DraftActorKey, DraftAssumptionTarget},
    owned_metrics::MetricBindingRole,
    owned_rules::{ContributionOrigin, ExistingActorRuleTarget},
    owned_schema::{AssumptionTargetKind, MetricActorRole, RuleEntityKind},
};
use serde_json::json;

#[test]
fn scenario_opponent_has_one_canonical_actor_address_in_resolved_and_draft_inputs() {
    let actor = ActorKey::ScenarioEnemy;
    assert_eq!(
        serde_json::to_value(&actor).unwrap(),
        json!({"kind":"scenario_enemy"})
    );
    let assumption = AssumptionTarget::Actor(actor.clone());
    let wire = json!({"kind":"actor","value":{"kind":"scenario_enemy"}});
    assert_eq!(serde_json::to_value(&assumption).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<AssumptionTarget>(wire.clone()).unwrap(),
        assumption
    );
    let draft: DraftAssumptionTarget = serde_json::from_value(wire).unwrap();
    assert_eq!(draft.to_resolved(), Some(assumption));
    assert_eq!(
        DraftActorKey::from(actor.clone()).to_resolved(),
        Some(actor)
    );
    for old in [
        json!({"kind":"enemy"}),
        json!({"kind":"actor","value":{"kind":"enemy"}}),
    ] {
        assert!(serde_json::from_value::<AssumptionTarget>(old.clone()).is_err());
        assert!(serde_json::from_value::<DraftAssumptionTarget>(old).is_err());
    }
    assert!(serde_json::from_value::<RuleEntityKind>(json!("enemy")).is_err());
}

#[test]
fn actor_role_permissions_are_explicit_and_old_broad_input_tokens_are_rejected() {
    assert_eq!(
        AssumptionTargetKind::for_actor(&ActorKey::ScenarioEnemy),
        AssumptionTargetKind::ScenarioEnemyActor
    );
    assert_eq!(
        AssumptionTargetKind::for_actor(&ActorKey::Player),
        AssumptionTargetKind::PlayerActor
    );
    for old in ["actor", "enemy"] {
        assert!(serde_json::from_value::<AssumptionTargetKind>(json!(old)).is_err());
    }
    assert_eq!(
        serde_json::to_value(MetricActorRole::ScenarioEnemy).unwrap(),
        "scenario_enemy"
    );
    assert_eq!(
        serde_json::to_value(MetricBindingRole::ScenarioEnemyActor).unwrap(),
        "scenario_enemy_actor"
    );
    assert_eq!(
        serde_json::to_value(ExistingActorRuleTarget::ScenarioEnemy).unwrap(),
        "scenario_enemy"
    );
    assert_eq!(
        serde_json::to_value(ContributionOrigin::Encounter).unwrap(),
        json!({"kind":"encounter"})
    );
}
