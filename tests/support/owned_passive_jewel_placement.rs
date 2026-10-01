//! Checked placement data; physical items and wider mechanics retain their gaps.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_import::{
    owned_normalize::{
        OrdinaryPassiveJewelBase, OrdinaryPassiveSocketBinding, PassiveSocketMembershipPolicy,
        equipment_membership_identity,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::{OwnedTreeNormalizationPolicy, tree_content_identity},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/ordinary-passive-jewel-placement")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn dependencies() -> Vec<DefinitionDescriptor> {
    read("dependencies.json")
}
pub fn bindings() -> Vec<OrdinaryPassiveSocketBinding> {
    read("bindings.json")
}
pub fn source_bases() -> Vec<OrdinaryPassiveJewelBase> {
    read("source-bases.json")
}

pub fn check_authored_memberships() {
    let extension = extension();
    let prior = dependencies();
    let bindings = bindings();
    let bases = source_bases();
    assert_eq!(extension.schema.len(), 30);
    assert_eq!(prior.len(), 18);
    assert_eq!(bindings.len(), 12);
    assert_eq!(bases.len(), 6);
    assert!(
        extension.tables.is_empty()
            && extension.owners.is_empty()
            && extension.receivers.is_empty()
    );
    assert_eq!(
        extension.operations_version.as_ref().unwrap().as_str(),
        "owned-domain-operations-v14"
    );
    let templates: BTreeSet<_> = bases.iter().map(|b| b.template.clone()).collect();
    assert_eq!(templates.len(), 6);
    let mut slots = BTreeSet::new();
    let mut nodes = BTreeSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        assert_eq!(
            binding.slot.key().as_str(),
            format!("def.{:016x}", 0x3203 + index)
        );
        assert!(slots.insert(binding.slot.clone()) && nodes.insert(binding.node.clone()));
        assert_eq!(binding.templates.len(), 6);
        assert_eq!(
            binding.templates.iter().cloned().collect::<BTreeSet<_>>(),
            templates
        );
    }
    for entry in extension.schema {
        let SchemaExtensionEntry::Definition(after) = entry else {
            panic!("only full definitions")
        };
        match &after {
            DefinitionDescriptor::SocketSlot(row) => {
                let binding = bindings.iter().find(|b| b.slot == row.id).unwrap();
                assert_eq!(
                    row.schema,
                    SchemaState::Known(SocketSlotSchema {
                        owner: SlotOwnerDefId::PassiveNode(binding.node.clone()),
                        kind: SocketKind::Passive,
                        scope: ScopePolicy::Shared,
                    })
                );
                assert!(!prior.iter().any(|d| d.address() == after.address()));
            }
            DefinitionDescriptor::PassiveNode(row) => {
                let mut expected = prior
                    .iter()
                    .find(|d| d.address() == after.address())
                    .unwrap()
                    .clone();
                let DefinitionDescriptor::PassiveNode(DefinitionEntry {
                    schema: SchemaState::Known(schema),
                    ..
                }) = &mut expected
                else {
                    panic!()
                };
                let binding = bindings.iter().find(|b| b.node == row.id).unwrap();
                assert!(matches!(
                    schema.declarations.sockets.closure,
                    SchemaClosure::Partial { .. }
                ));
                assert!(!schema.declarations.sockets.members.contains(&binding.slot));
                schema
                    .declarations
                    .sockets
                    .members
                    .push(binding.slot.clone());
                assert_eq!(after, expected);
            }
            DefinitionDescriptor::ItemTemplate(row) => {
                let mut expected = prior
                    .iter()
                    .find(|d| d.address() == after.address())
                    .unwrap()
                    .clone();
                let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                    schema: SchemaState::Known(schema),
                    ..
                }) = &mut expected
                else {
                    panic!()
                };
                assert!(templates.contains(&row.id));
                assert!(matches!(
                    schema.socket_destinations.closure,
                    SchemaClosure::Partial { .. }
                ));
                for binding in &bindings {
                    assert!(binding.templates.contains(&row.id));
                    assert!(!schema.socket_destinations.members.contains(&binding.slot));
                    schema
                        .socket_destinations
                        .members
                        .push(binding.slot.clone());
                }
                assert_eq!(after, expected);
            }
            _ => panic!("unrelated descriptor refinement"),
        }
    }
}

fn verify_source(authoring: &Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let bytes = fs::read(root.join(proof["evidence_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        bytes.len() as u64,
        proof["evidence_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(evidence["source_revision"], authoring["source_revision"]);
    assert_eq!(evidence["source_hash"], authoring["source_manifest_sha256"]);
    let manifest_bytes =
        fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest_bytes)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
        assert!(
            evidence["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|observed| {
                    observed["path"] == pin["path"] && observed["sha256"] == pin["sha256"]
                })
        );
    }
    assert!(
        root.join(authoring["source_test"].as_str().unwrap())
            .is_file()
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored_memberships();
    let authoring = authoring();
    verify_source(&authoring);
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(receipt["input"], authoring["before"]);
    for field in [
        "definitions",
        "registry",
        "mapping",
        "items",
        "item_source",
        "normalization",
        "tree",
    ] {
        assert_eq!(
            receipt[field], authoring[field],
            "exact predecessor {field}"
        );
    }
    let before = prior.input();
    assert!(before.evaluation.is_none());
    for dependency in dependencies() {
        assert_eq!(
            before
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == dependency.address()),
            Some(&dependency)
        );
    }
    let extension = extension();
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 12);
    assert_eq!(extended.receipt.refined_subjects, 18);
    assert_eq!(extended.receipt.appended_programs, 0);
    assert_eq!(extended.receipt.appended_receivers, 0);
    assert_eq!(extended.receipt.appended_tables, 0);
    let carried = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: before.recipe.clone(),
            successor: extended.successor,
            mapping: before.mapping.clone(),
            roles: before.roles.clone(),
            normalization: before.normalization.clone(),
            rewards: before.rewards.clone(),
            query_sets: before.query_sets.clone(),
            items: before.items.clone(),
            item_source: before.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: before.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(before.tree.clone().unwrap()),
        },
        extended.refinement.unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut full = before.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|v| v.input().clone());
    let prior_placement = full.clone();
    assert!(matches!(
        full.normalization.passive_socket_membership,
        Some(PassiveSocketMembershipPolicy::PobExplicitEmptySpecSocketsV1 {})
    ));
    full.normalization.passive_socket_membership = Some(
        PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
            definitions: carried.assembled().schema().identity().clone(),
            mapping: *carried.mapping().identity(),
            mapping_source: *carried.mapping().source_identity(),
            item_lines: *carried.items().identity(),
            item_source: *carried.item_source().identity(),
            equipment: equipment_membership_identity(
                full.normalization.equipment_membership.as_ref().unwrap(),
                Default::default(),
            )
            .unwrap(),
            tree_content: tree_content_identity(
                &full.tree.as_ref().unwrap().content,
                Default::default(),
            )
            .unwrap(),
            source_bases: source_bases(),
            bindings: bindings(),
        },
    );
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            carried.assembled().registry(),
            carried.assembled().schema(),
            carried.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("explicit-ordinary-passive-jewel-placement").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ordinary-passive-jewel-placement-v1",
            &(
                authoring,
                &extension,
                dependencies(),
                bindings(),
                source_bases(),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    // The checked transition already preserves the complete old interpretation
    // while admitting exactly the membership refinement above. No second data
    // edit is allowed when attaching the placement policy.
    let mut restored = next.input().clone();
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        prior_placement.tree.as_ref().unwrap().content
    );
    restored.tree = prior_placement.tree.clone();
    restored.normalization.passive_socket_membership = prior_placement
        .normalization
        .passive_socket_membership
        .clone();
    assert_eq!(restored.provenance.len(), before.provenance.len() + 1);
    restored.provenance.pop();
    assert!(
        restored == prior_placement,
        "placement attachment changed unrelated carried facts"
    );
    assert_eq!(next.input().query_sets, before.query_sets);
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.input().recipe.registry.last_issued.get(), 0x320e);
    next
}
