//! Remove computed collector indirection without changing supplier authority.
#[allow(dead_code)]
#[path = "support/owned_mana_direct_queries.rs"]
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
use poe_optimizer_core::owned_rules::*;
use serde_json::json;
use std::path::PathBuf;

fn component() -> RulePackageInput {
    let d = family::dependencies();
    let mut owners = vec![d.owner];
    owners.extend(d.channels.iter().map(|c| c.owner.clone()));
    let mut receivers = vec![d.receiver];
    receivers.extend(d.channels.iter().map(|c| c.receiver.clone()));
    serde_json::from_value(json!({
        "schema_version":3,"namespace":{"game":"poe2","version":"owned-mechanics-v1"},
        "release":"fixture-mana-direct-queries","semantics_version":"fixture",
        "operations_version":OWNED_RULE_OPERATIONS_V27,
        "definitions":{"game":"poe2","release":"fixture","schema_version":6,
            "content_sha256":"0".repeat(64),"semantics_version":"fixture"},
        "tables":[],"owners":owners,
        "receivers":{"members":receivers,"closure":{"kind":"complete"}},
        "effect_applications":{"members":[],"closure":{"kind":"complete"}},
        "contribution_queries":{"members":d.channels.iter().map(|c| &c.query).collect::<Vec<_>>(),"closure":{"kind":"complete"}}
    })).unwrap()
}

#[test]
fn direct_queries_preserve_formula_channels_membership_and_receiver() {
    family::check_authored();
    let old = component();
    let new = family::rewrite_rules(&old);
    assert_eq!(new.owners, vec![family::consumer().owner]);
    assert_eq!(new.receivers.members, vec![family::dependencies().receiver]);
    assert_eq!(new.contribution_queries, old.contribution_queries);
    assert_eq!(new.effect_applications, old.effect_applications);
    assert_eq!(new.receivers.closure, old.receivers.closure);
}

#[test]
fn reconciliation_refuses_changed_producers_receivers_queries_and_new_readers() {
    for mutation in 0..6 {
        let mut input = component();
        match mutation {
            0 => {
                input.owners[1].programs.members[0].effects[0].when =
                    Some("resolved".parse().unwrap())
            }
            1 => input.receivers.members[1].targets.clear(),
            2 => {
                input.contribution_queries.as_mut().unwrap().members.pop();
            }
            3 => {
                let q = &mut input.contribution_queries.as_mut().unwrap().members[0];
                q.groups[0].empty = Some(poe_optimizer_core::owned_build::ParameterValue::Boolean(
                    false,
                ));
            }
            4 => {
                // A separate reader, not stale metadata or a modified old collector.
                let read = input.owners[0].programs.members[0].reads.iter().find(|r| {
                    matches!(&r.source, RuleReadSource::Stat { stat, .. } if stat == &family::dependencies().channels[0].receiver.stat)
                }).unwrap().clone();
                let owner: DefinitionRules = serde_json::from_value(json!({
                    "owner":{"kind":"definition","value":{"kind":"actor","value":{"kind":"actor","namespace":input.namespace,"key":"def.000000000000332a"}}},
                    "programs":{"closure":{"kind":"complete"},"members":[{
                        "id":"unexpected-mana-reader","context":"actor","reads":[read],"nodes":[],"effects":[]
                    }]}
                })).unwrap();
                input.owners.push(owner);
            }
            _ => {
                let read = input.owners[0].programs.members[0].reads[0].clone();
                input.owners[0].programs.members[0].reads.push(read);
            }
        }
        assert!(
            std::panic::catch_unwind(|| family::rewrite_rules(&input)).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn current_component_refuses_later_computed_reads_in_owners_and_applications() {
    let old = component();
    let query_read = family::dependencies().channels[0].owner.programs.members[0].reads[0].clone();
    let mut read = query_read;
    read.source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: family::dependencies().channels[0].receiver.stat.clone(),
    };
    for application in [false, true] {
        let mut changed = family::rewrite_rules(&old);
        if application {
            let mut program = changed.owners[0].programs.members[0].clone();
            program.reads = vec![read.clone()];
            changed.effect_applications.as_mut().unwrap().members.push(serde_json::from_value(json!({
                "id":"unexpected-mana-application", "source":{"kind":"skill","value":{"skill":{"kind":"skill","namespace":changed.namespace,"key":"def.000000000000000a"}}},
                "targets":[{"kind":"player"}],"activation":"active","program":program,"stacking":[]
            })).unwrap());
        } else {
            changed.owners[0].programs.members[0]
                .reads
                .push(read.clone());
        }
        assert!(std::panic::catch_unwind(|| family::assert_no_computed_readers(&changed)).is_err());
    }
}

#[test]
#[ignore = "explicit MANA_DIRECT_QUERIES_PRIOR/OUTPUT publication; no coverage promotion"]
fn publish_direct_mana_queries_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_DIRECT_QUERIES_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_DIRECT_QUERIES_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json"],
        family::stage,
        json!({"removed_programs":5,"removed_receivers":5,"new_definitions":0,"new_queries":0,
            "preserved_channels":5,"supplier_admission":false,"complete_mana":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
