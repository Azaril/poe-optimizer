//! Publish bounded Life equipment reductions without closing supplier coverage.
#[allow(unused_imports)]
use super::{preservation, publication, release, selected};
#[allow(dead_code)]
#[path = "../support/owned_life_equipment_domain.rs"]
mod family;
use poe_optimizer_core::owned_rules::{RuleEffectKind, RuleExpression};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authenticated_guards_preserve_arithmetic_and_every_other_coverage_obligation() {
    family::check_authored();
}

#[test]
fn exact_guard_inverse_rejects_bypass_changed_recipient_and_unrelated_closure() {
    let programs = family::programs();
    let query = family::query();
    for mutation in 0..6 {
        let mut p = programs.clone();
        let mut q = query.clone();
        match mutation {
            0 => {
                let RuleEffectKind::Contribute { value, .. } = &mut p[0].effects[0].effect else {
                    panic!()
                };
                *value = "life-points".parse().unwrap();
            }
            1 => {
                p[1].nodes
                    .iter_mut()
                    .find(|n| n.id.as_str() == "effective")
                    .unwrap()
                    .expression = RuleExpression::Read {
                    input: "effective".parse().unwrap(),
                };
            }
            2 => {
                let RuleEffectKind::Contribute { entity, .. } = &mut p[1].effects[0].effect else {
                    panic!()
                };
                *entity = poe_optimizer_core::owned_rules::RuleEntity::Enemy;
            }
            3 => p[0].effects[0].when = None,
            4 => q.groups[0].members.members.clear(),
            5 => q
                .groups
                .iter_mut()
                .find(|g| g.id.as_str() == "equipment")
                .unwrap()
                .members
                .members
                .reverse(),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_transition(&p, &q)).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
#[ignore = "requires LIFE_EQUIPMENT_DOMAIN_PRIOR/OUTPUT; preserves all five exact imports"]
fn publish_bounded_life_equipment_preserving_all_five() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_EQUIPMENT_DOMAIN_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_EQUIPMENT_DOMAIN_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json", "proof.json"],
        family::stage,
        json!({"changed_programs":2,"numeric_guards":3,"closed_equipment_groups":1,
            "new_definitions":0,"source_order_claim":false,"supplier_admission":false,
            "complete_life":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
