//! Source-action CLI resolution consumes a checked release without calculation.
#[allow(dead_code)]
#[path = "../../crates/poe-optimizer-import/tests/support/source_actions_fixture.rs"]
mod actions;
#[allow(dead_code)]
#[path = "../../crates/poe-optimizer-import/tests/support/empty_owned_items.rs"]
mod empty_items;

use actions::key;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_definitions::SlotOwnerDefId,
    owned_routing::{ActionRoutingInput, OWNED_ACTION_ROUTING_VERSION},
    owned_rules::{OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_PACKAGE_VERSION, RulePackageInput},
    owned_schema::DeclaredSet,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_normalize::{GemQualityPolicy, ImportActorTarget, ImportQueryTarget},
    owned_recipe::{OWNED_RECIPE_VERSION, OwnedRecipeInput},
    owned_release::{
        OWNED_RELEASE_VERSION, OwnedReleaseInput, StagedOwnedRelease, assemble_owned_release,
    },
    owned_reward_policy::{OWNED_REWARD_POLICY_VERSION, RewardPolicyInput},
    owned_source::SourceProjectEvidence,
    owned_source_actions::{
        ImportReferenceContext, SourceActionCorrespondenceInput, SourceActionSelection,
    },
    owned_value::{BooleanToken, ValueCodecKind},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn release(f: &actions::Fixture) -> StagedOwnedRelease {
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 { index, .. } = &f.input
    else {
        unreachable!()
    };
    let mut enabled = index.clone();
    enabled.codec.codec = ValueCodecKind::Boolean {
        tokens: vec![
            BooleanToken {
                token: "true".into(),
                value: true,
            },
            BooleanToken {
                token: "false".into(),
                value: false,
            },
        ],
    };
    let items = empty_items::empty_items(&f.schema);
    let mut item_source = empty_items::empty_source_for_items(&f.schema, &items)
        .input()
        .clone();
    item_source.source = f.mapping.input().source.clone();
    assemble_owned_release(
        OwnedReleaseInput {
            schema_version: OWNED_RELEASE_VERSION,
            recipe: OwnedRecipeInput {
                schema_version: OWNED_RECIPE_VERSION,
                registry: f.registry.input().clone(),
                schema: f.schema.input().clone(),
                rules: RulePackageInput {
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: actions::ns(),
                    release: key("empty-source-action-rules"),
                    semantics_version: key("source-action-fixture"),
                    operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                    definitions: f.schema.identity().clone(),
                    tables: vec![],
                    owners: vec![],
                    receivers: DeclaredSet::complete(vec![]),
                    effect_applications: Some(DeclaredSet::complete(vec![])),
                    contribution_queries: Some(DeclaredSet::complete(vec![])),
                    existing_actor_rules: None,
                    support_discovery: None,
                },
                routing: ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: actions::ns(),
                    release: key("empty-source-action-routing"),
                    definitions: f.schema.identity().clone(),
                    outputs: vec![],
                },
            },
            mapping: f.mapping.input().clone(),
            roles: f.roles.input().clone(),
            // Unconverted optional policies stay absent. These scalar codecs
            // validate the fixture only; the action host does not normalize it.
            normalization: serde_json::from_value(json!({
                "version": "source-action-fixture",
                "namespace": actions::ns(),
                "character_level": index,
                "gem_level": index,
                "gem_enabled": enabled,
                "group_enabled": enabled,
                "manual_skill_sources": [],
                "empty_item_keys": [],
                "generated_support_prefixes": [],
                "allocation_attribute": "nodes",
                "single_active_support_target": false,
                "equipment_loadouts": [],
                "gem_quality": GemQualityPolicy::Unconverted
            }))
            .unwrap(),
            rewards: RewardPolicyInput {
                schema_version: OWNED_REWARD_POLICY_VERSION,
                namespace: actions::ns(),
                version: key("empty-source-action-rewards"),
                definitions: f.schema.identity().clone(),
                mapping: *f.mapping.identity(),
                rules: vec![],
            },
            items: items.input().clone(),
            item_source,
            tree: None,
            evaluation: None,
            query_sets: vec![],
            provenance: vec![],
        },
        Default::default(),
    )
    .unwrap()
}

fn resolve(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("resolve-owned-action")
        .arg(root.join("build.xml"))
        .arg("--release")
        .arg(root.join("release"))
        .arg("--correspondence")
        .arg(root.join("correspondence.json"))
        .arg("--request")
        .arg(root.join("request.json"))
        .output()
        .unwrap()
}

#[test]
fn checked_release_resolves_exact_actions_and_rejects_rehashed_stale_bindings() {
    let f = actions::Fixture::new();
    let release = release(&f);
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("release");
    fs::create_dir(&directory).unwrap();
    for (name, bytes) in release.artifacts() {
        fs::write(directory.join(name), bytes).unwrap();
    }
    let xml = actions::xml(&actions::gem("", &actions::maps("effect", "2", "1")));
    fs::write(temp.path().join("build.xml"), &xml).unwrap();
    fs::write(
        temp.path().join("correspondence.json"),
        serde_json::to_vec(&f.input).unwrap(),
    )
    .unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([42; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let correspondence = f.compile(Default::default()).unwrap();
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
        primary, output, ..
    } = &f.input
    else {
        unreachable!()
    };
    for (context, selected) in [
        (ImportReferenceContext::Main, 1),
        (ImportReferenceContext::Calcs, 0),
    ] {
        let request = f.request(&source, 0, context);
        let expected = correspondence.resolve(&evidence, &request).unwrap();
        assert!(
            matches!(&expected.selection, SourceActionSelection::Explicit { source_index, .. } if *source_index == selected as u32 + 1)
        );
        let ImportQueryTarget::Action(target) = &expected.target else {
            panic!("fixture must resolve an exact action")
        };
        assert_eq!(&target.output, output);
        assert_eq!(
            target.output.declaration,
            SlotOwnerDefId::Skill(primary.clone())
        );
        assert_eq!(target.stat_set, f.sets[selected]);
        assert_eq!(target.provider.skill_use, request.skill_use);
        assert_eq!(target.provider.grant_path, vec![f.grant.clone()]);
        assert_eq!(target.actor, ImportActorTarget::Player);
        fs::write(
            temp.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        let actual = resolve(temp.path());
        assert!(
            actual.status.success(),
            "{}",
            String::from_utf8_lossy(&actual.stderr)
        );
        let actual: Value = serde_json::from_slice(&actual.stdout).unwrap();
        assert_eq!(actual["document_kind"], "owned_source_action_resolution");
        assert_eq!(
            actual["release"],
            serde_json::to_value(release.receipt().input).unwrap()
        );
        assert_eq!(
            actual["correspondence"],
            serde_json::to_value(correspondence.identity()).unwrap()
        );
        assert_eq!(actual["report"], serde_json::to_value(expected).unwrap());
        for field in ["source_execution", "calculation", "whole_build_parity"] {
            assert_eq!(actual[field], false);
        }
    }
    // The bytes match the updated receipt, but this role package binds a different
    // mapping. The host must reject the release before publishing any resolution.
    let mut roles = f.roles.input().clone();
    assert_ne!(roles.mapping, release.receipt().rules);
    roles.mapping = release.receipt().rules;
    let bytes = serde_json::to_vec(&roles).unwrap();
    fs::write(directory.join("roles.json"), &bytes).unwrap();
    let mut receipt = release.receipt().clone();
    let artifact = receipt
        .artifacts
        .iter_mut()
        .find(|row| row.file == "roles.json")
        .unwrap();
    artifact.bytes = bytes.len();
    artifact.sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(
        directory.join("release.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    let rejected = resolve(temp.path());
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("binding mismatch"));
}
