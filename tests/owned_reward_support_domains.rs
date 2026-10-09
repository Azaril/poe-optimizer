//! Complete numeric reward owners gain explicit composed-support capabilities.
#[path = "support/owned_reward_support_domains.rs"]
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
use poe_optimizer_core::owned_schema::*;
use poe_optimizer_engine::owned_plan::*;
use serde_json::json;
use std::{fs, path::PathBuf};
#[test]
fn closed_numeric_reward_domains_exclude_missing_partial_and_extra_capabilities() {
    family::check_authored();
    let deps: family::Dependencies = family::read("dependencies.json");
    let o = deps
        .owners
        .iter()
        .find(|o| o.programs.is_complete())
        .unwrap();
    let d = deps
        .definitions
        .iter()
        .find(|d| o.owner == SchemaSubject::Definition(d.address()))
        .unwrap();
    assert!(family::admissible(d, o));
    let mut owner = o.clone();
    owner.programs.closure=serde_json::from_value(json!({"kind":"partial","value":{"gaps":[{"subject":o.owner,"facet":"game_rules","code":"unreviewed-reward"}]}})).unwrap();
    assert!(!family::admissible(d, &owner));
    let mut owner = o.clone();
    owner.programs.members[0].effects[0].when = Some("conditional".parse().unwrap());
    assert!(!family::admissible(d, &owner));
    let mut v = json!(d);
    v["value"]["schema"]["value"]["declarations"]["grants"]["closure"] = json!({"kind":"partial","value":{"gaps":[{"subject":o.owner,"facet":"game_rules","code":"unreviewed-grants"}]}});
    let changed: DefinitionDescriptor = serde_json::from_value(v).unwrap();
    assert!(!family::admissible(&changed, o));
}
fn world() -> replay::ReplayInput {
    replay::ReplayInput::decode(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/owned-sniper-replay.json.gz"),
        )
        .unwrap(),
    )
}
fn is_reward(s: &SchemaSubject) -> bool {
    matches!(s, SchemaSubject::Definition(DefinitionAddress::Reward(_)))
}
#[test]
fn real_reward_domains_close_only_their_native_composition_gaps() {
    let original = world();
    let mut missing = original.clone();
    missing
        .rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .retain(|p| !is_reward(&p.owner));
    missing.rebind_test_edit().unwrap();
    let p = missing.compile().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| matches!(g.reason, PlanGapReason::MissingSupportSources))
    );
    let mut restored = missing.clone();
    let deps: family::Dependencies = family::read("dependencies.json");
    let selected: Vec<_> = restored
        .rules
        .owners
        .iter()
        .filter(|o| is_reward(&o.owner))
        .map(|o| o.owner.clone())
        .collect();
    assert_eq!(selected.len(), 2);
    for domain in family::extension()
        .support_source_domains
        .into_iter()
        .filter(|p| selected.contains(&p.owner))
    {
        let o = restored
            .rules
            .owners
            .iter()
            .find(|o| o.owner == domain.owner)
            .unwrap();
        assert!(deps.owners.contains(o));
        let d = restored
            .schema
            .definitions
            .iter()
            .find(|d| domain.owner == SchemaSubject::Definition(d.address()))
            .unwrap();
        assert!(deps.definitions.contains(d));
        restored
            .rules
            .support_discovery
            .as_mut()
            .unwrap()
            .providers
            .push(domain);
    }
    restored.rebind_test_edit().unwrap();
    let p = restored.compile().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    let original = original.compile().unwrap();
    assert_eq!(
        report,
        original.evaluate(&mut original.new_scratch()).unwrap()
    );
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
    let mut partial = restored;
    let row = partial
        .rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .iter_mut()
        .find(|p| is_reward(&p.owner))
        .unwrap();
    row.domain = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: row.owner.clone(),
            facet: SchemaFacet::GameRules,
            code: "unsupported-reward-origin".parse().unwrap(),
        }],
    };
    partial.rebind_test_edit().unwrap();
    let partial = partial.compile().unwrap();
    assert!(matches!(
        partial
            .evaluate(&mut partial.new_scratch())
            .unwrap()
            .outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}
#[test]
#[ignore = "requires REWARD_SUPPORT_PRIOR/OUTPUT"]
fn publish_reward_support_domains_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_REWARD_SUPPORT_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_REWARD_SUPPORT_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json"],
        family::stage,
        json!({"new_support_domains":18,"remaining_reward_domains_unknown":13,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
