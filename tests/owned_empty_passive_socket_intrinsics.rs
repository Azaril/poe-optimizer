//! Default socket definition closure; no external jewel or whole-build claim.
#[allow(dead_code)]
#[path = "support/owned_empty_passive_socket_intrinsics.rs"]
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
    owned_rules::RuleProgram,
    owned_schema::{DefinitionDescriptor, RuleEntityKind, SchemaState},
};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn empty_intrinsic_socket_has_complete_source_and_retains_its_existing_declaration() {
    family::check_authored();
}
#[test]
fn intrinsic_closure_refuses_lost_socket_changed_topology_or_added_program() {
    let base: family::Closure = family::read("closure.json");
    for mutation in 0..3 {
        let mut changed = base.clone();
        if mutation == 2 {
            changed.owner.programs.members.push(RuleProgram {
                id: "unreviewed".parse().unwrap(),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![],
                effects: vec![],
            });
        } else {
            let DefinitionDescriptor::PassiveNode(node) = &mut changed.definition else {
                panic!()
            };
            let SchemaState::Known(schema) = &mut node.schema else {
                panic!()
            };
            if mutation == 0 {
                schema.declarations.sockets.members.clear();
            } else {
                schema.adjacent.members.pop();
            }
        }
        assert!(std::panic::catch_unwind(|| family::check_transition(&changed)).is_err());
    }
}
#[test]
fn source_closure_refuses_changed_node_span_archive_identity_and_catalog() {
    let base: Value = family::read("source-evidence.json");
    for mutation in 0..4 {
        let mut changed = base.clone();
        match mutation {
            0 => {
                changed["spans"][0]["text"] =
                    json!("stats={} is insufficient without the complete node")
            }
            1 => changed["archives"][0]["source_path"] = json!("src/TreeData/0_4/tree.lua"),
            2 => changed["catalog"]["row"]["views"] = json!(["unreviewed-choice"]),
            3 => {
                changed["contextual_reports"][0]["original_state"]["specs"][4]["jewels"] = json!({})
            }
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_source(&changed)).is_err());
    }
}
#[test]
#[ignore = "requires EMPTY_PASSIVE_SOCKET_PRIOR/OUTPUT; preserves all five imports"]
fn publish_empty_socket_intrinsic_closure_preserving_all_five() {
    publication::run_with_item_parameter_completions_and_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_PASSIVE_SOCKET_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_PASSIVE_SOCKET_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "source-evidence.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 1,
            passive_refinement: true,
            extra: json!({"default_intrinsic_programs":0,"closed_declaration_inventories":7,
                "new_definitions":0,"external_provider_closure":false,"whole_build_parity":false}),
        },
        &[],
        [106, 117, 109, 123, 4],
    );
}
