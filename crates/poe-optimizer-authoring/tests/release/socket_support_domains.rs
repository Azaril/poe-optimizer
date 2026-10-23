//! Intrinsic socket proof against the maintained release. Saved jewel placement
//! remains an independent input inventory, not authority for external mechanics.
use super::{originals, socket_support_source as source, tracked_release};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_draft::{DraftFinalization, DraftListCompletion},
    owned_rules::{RuleProgram, SupportSourceDomain, SupportSourceDomainDeclaration},
    owned_schema::{DefinitionDescriptor, RuleEntityKind, SchemaState},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{NormalizationArtifacts, OwnedOriginTarget, normalize_fresh},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::OnceLock};

pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    static VERIFIED: OnceLock<()> = OnceLock::new();
    VERIFIED.get_or_init(source::check_authored);
    let closure: source::Closure = source::read("closure.json");
    vec![SupportSourceDomainDeclaration {
        owner: closure.owner.owner,
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    }]
}

#[test]
fn empty_intrinsic_socket_has_complete_source_and_retains_its_existing_declaration() {
    source::check_authored();
    let closure: source::Closure = source::read("closure.json");
    let dependencies: source::Dependencies = source::read("dependencies.json");
    let release = tracked_release().release();
    let recipe = &release.input().recipe;
    for expected in [&closure.definition, &dependencies.socket] {
        let actual: Vec<_> = recipe
            .schema
            .definitions
            .iter()
            .filter(|row| row.address() == expected.address())
            .collect();
        assert_eq!(actual, vec![expected]);
    }
    let owners: Vec<_> = recipe
        .rules
        .owners
        .iter()
        .filter(|row| row.owner == closure.owner.owner)
        .collect();
    assert_eq!(owners, vec![&closure.owner]);
    let domain = declarations().remove(0);
    let actual: Vec<_> = recipe
        .rules
        .support_discovery
        .as_ref()
        .unwrap()
        .providers
        .iter()
        .filter(|row| row.owner == domain.owner)
        .collect();
    assert_eq!(actual, vec![&domain]);
    for mapping in dependencies.mappings {
        assert_eq!(
            release.mapping().lookup(&mapping.source),
            Some(&mapping.outcome)
        );
    }
    assert!(release.evaluation().is_none());
}

#[test]
fn intrinsic_closure_refuses_lost_socket_changed_topology_or_added_program() {
    let base: source::Closure = source::read("closure.json");
    source::check_transition(&base);
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
                assert!(schema.adjacent.members.pop().is_some());
            }
        }
        assert!(std::panic::catch_unwind(|| source::check_transition(&changed)).is_err());
    }
}

#[test]
fn source_closure_refuses_changed_node_span_archive_identity_and_catalog() {
    let base: Value = source::read("source-evidence.json");
    source::check_source(&base);
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
        assert!(std::panic::catch_unwind(|| source::check_source(&changed)).is_err());
    }
}

#[test]
fn saved_sniper_retains_selected_socket_and_occupied_sibling_presets() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let xml = fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(&xml).unwrap(),
        BuildLineage::from_bytes([5; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let query_set = release
        .query_sets()
        .iter()
        .find(|set| set.name.as_str() == "original-05")
        .unwrap();
    let normalized = normalize_fresh(
        &evidence,
        *imported.allocator_state(),
        NormalizationArtifacts {
            mappings: release.mapping(),
            registry: recipe.registry(),
            definitions: recipe.schema().as_ref(),
            roles: release.roles(),
            rewards: release.rewards(),
            items: release.items(),
            item_source: release.item_source(),
            tree: release.tree(),
        },
        release.normalization(),
        &query_set.queries,
        Default::default(),
    )
    .unwrap();
    let selection = originals::saved(&evidence, &normalized);
    let draft = normalized.draft();
    let snapshot = draft.clone();
    let specs: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Spec")
        .collect();
    assert_eq!(specs.len(), 7);
    for (index, expected_items) in [(2, 0), (4, 3), (5, 3)] {
        let origin: Vec<_> = normalized
            .sidecar()
            .origins
            .iter()
            .filter(|origin| origin.source == specs[index].occurrence().id())
            .collect();
        assert_eq!(origin.len(), 1);
        let presets: Vec<_> = origin[0]
            .links
            .iter()
            .filter_map(|link| match link {
                OwnedOriginTarget::AllocationPreset(id) => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(presets.len(), 1);
        let preset = draft
            .input()
            .allocation_presets
            .members
            .iter()
            .find(|row| row.id == presets[0])
            .unwrap();
        assert_eq!(preset.equipment.members.len(), expected_items);
        assert_eq!(preset.equipment.completion, DraftListCompletion::Complete);
        if index == 2 {
            assert_eq!(preset.id, selection.build.allocations);
        }
        let sockets: Vec<_> = draft
            .input()
            .allocations
            .members
            .iter()
            .filter(|row| preset.allocations.members.contains(&row.id))
            .filter(|row| {
                row.node
                    .to_resolved()
                    .is_some_and(|node| node.key().as_str() == "def.0000000000001b48")
            })
            .collect();
        assert_eq!(sockets.len(), 1);
        // Every occupied sibling placement retains its authored exact occurrence.
        for id in &preset.equipment.members {
            assert_eq!(
                draft
                    .input()
                    .equipment
                    .members
                    .iter()
                    .filter(|row| row.id == *id)
                    .count(),
                1
            );
        }
    }
    let proof = draft
        .prove_intent(
            recipe.schema().as_ref(),
            Default::default(),
            Default::default(),
        )
        .unwrap();
    let checked = draft
        .finalize_selection_checked(
            recipe.schema().as_ref(),
            selection,
            &proof,
            Default::default(),
            Default::default(),
        )
        .unwrap();
    assert!(
        matches!(checked.finalization(), DraftFinalization::Pending { issues, .. } if issues.len() == 5)
    );
    assert_eq!(draft, &snapshot);
    assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
}
