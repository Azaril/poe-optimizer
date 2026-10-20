//! One checked current-policy packet and publication path; no runtime compatibility layer.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        GemInventoryPolicy, UsageGroupCompanion, UsageInputPolicy, UsageInventoryPolicy,
        configuration_disposition_dependencies, usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[path = "owned_preset_usage_inventory_companions.rs"]
mod companions;
#[path = "owned_preset_usage_inventory_outputs.rs"]
mod outputs;
pub use outputs::{Output, checked_output};
pub const BEFORE: &str = "b3b7c3810f659e584f23b6dd69b7bdc6b619cc39fcd4f2b9ae259194bf8d7529";
const AUTHORING: &str = "f3ef42847a761db8aa9d73fe7d3882e9a2d5040bf0fb2c18d1b759260ac60571";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/preset-usage-inventory")
}
pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn pin(p: &Path, expected: &Value) {
    let b = fs::read(p).unwrap();
    assert_eq!(json!(sha(&b)), expected["sha256"], "{}", p.display());
    assert_eq!(json!(b.len()), expected["bytes"], "{}", p.display());
}
pub fn check() -> Value {
    let bytes = fs::read(data().join("authoring.json")).unwrap();
    assert_eq!(sha(&bytes), AUTHORING);
    let authoring: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(authoring["predecessor"]["input"], BEFORE);
    for (name, expected) in authoring["files"].as_object().unwrap() {
        pin(&data().join(name), expected);
    }
    let inventory: UsageInventoryPolicy =
        serde_json::from_value(read(data().join("inventory.json"))).unwrap();
    assert_eq!(inventory.rows.len(), 25);
    assert_eq!(
        inventory
            .rows
            .iter()
            .map(|row| row.fields.len())
            .sum::<usize>(),
        454
    );
    assert_eq!(
        inventory.evidence.to_string(),
        sha(&fs::read(data().join("evidence.json")).unwrap())
    );
    check_census();
    companions::check();
    authoring
}
/// Test-only semantic checks independent of the sealed outer packet bytes.
pub fn check_companion_evidence(packet: &Value) {
    companions::check_packet(packet);
}
pub fn checked_predecessor(prior: &Path) -> Value {
    let bytes = fs::read(data().join("authoring.json")).unwrap();
    assert_eq!(sha(&bytes), AUTHORING);
    let a: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["predecessor"]["input"], BEFORE);
    pin(&prior.join("release.json"), &a["predecessor"]["release"]);
    let receipt = read(prior.join("release.json"));
    assert_eq!(receipt["artifacts"], a["predecessor"]["artifacts"]);
    for row in a["predecessor"]["artifacts"].as_array().unwrap() {
        // Release artifacts bind raw bytes; the production loader additionally
        // checks every semantic identity and exact reconstructed artifact.
        let bytes = fs::read(prior.join(row["file"].as_str().unwrap())).unwrap();
        assert_eq!(json!(sha(&bytes)), row["sha256"]);
        assert_eq!(json!(bytes.len()), row["bytes"]);
    }
    for (name, expected) in a["files"].as_object().unwrap() {
        pin(&data().join(name), expected);
    }
    a
}
fn check_census() {
    let census = read(data().join("source-census.json"));
    let mut total = 0;
    for case in census.as_array().unwrap() {
        let xml = fs::read(root().join(case["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(sha(&xml)), case["sha256"]);
        assert_eq!(json!(xml.len()), case["bytes"]);
        let instance = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            BuildLineage::from_bytes([93; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
        let rows = evidence.rows();
        assert_eq!(json!(rows.len()), case["complete_element_count"]);
        let actual: Vec<_> = rows
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Gem"
                    && r.occurrence()
                        .parent()
                        .is_some_and(|p| rows[p.ordinal() as usize].occurrence().name() == "Skill")
            })
            .collect();
        let expected = case["gem_rows"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len());
        total += actual.len();
        for (row, known) in actual.into_iter().zip(expected) {
            assert_eq!(json!(row.occurrence().id().ordinal()), known["ordinal"]);
            let group = &rows[row.occurrence().parent().unwrap().ordinal() as usize];
            let set = &rows[group.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(json!(group.occurrence().id().ordinal()), known["group"]);
            assert_eq!(json!(set.occurrence().id().ordinal()), known["set"]);
            assert_eq!(
                json!(set.attribute("id").unwrap().decoded().unwrap()),
                known["set_id"]
            );
            for (actual, key) in [(row, "attributes"), (group, "group_attributes")] {
                let values: Vec<_> = actual
                    .attributes()
                    .iter()
                    .map(|a| json!({"name":a.origin().name,"decoded":a.decoded().unwrap()}))
                    .collect();
                assert_eq!(json!(values), known[key]);
            }
            assert_eq!(
                json!(
                    row.children()
                        .iter()
                        .map(|s| s.ordinal())
                        .collect::<Vec<_>>()
                ),
                known["child_elements"]
            );
        }
    }
    assert_eq!(total, 541);
}
fn authored_identity() -> OwnedContentDigest {
    digest_owned(
        "owned-usage-inventory-authoring-v1",
        &read(data().join("authoring.json")),
        4 * 1024 * 1024,
    )
    .unwrap()
}
pub fn checked_successor(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let inventory: UsageInventoryPolicy =
        serde_json::from_value(read(data().join("inventory.json"))).unwrap();
    checked_inventory_successor(
        prior,
        next,
        &inventory,
        &companions::included(&[]),
        "bounded-preset-usage-field-accounting",
        authored_identity(),
    );
}
fn checked_inventory_successor(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    expected: &UsageInventoryPolicy,
    added_companions: &[UsageGroupCompanion],
    provenance_kind: &str,
    authoring: OwnedContentDigest,
) {
    assert_eq!(prior.receipt().input.to_string(), BEFORE);
    assert_ne!(next.receipt().input, prior.receipt().input);
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: None, ..
    }) = &prior.input().normalization.usage_inputs
    else {
        panic!("exact inventory-absent predecessor")
    };
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: Some(actual),
        ..
    }) = &next.input().normalization.usage_inputs
    else {
        panic!("published checked inventory")
    };
    assert_eq!(actual, expected);
    companions::authenticate(prior);
    let mut inverse = next.input().clone();
    companions::apply(&mut inverse.normalization, added_companions, true);
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. }) =
        &mut inverse.normalization.usage_inputs
    else {
        unreachable!()
    };
    *inventory = None;
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut inverse.normalization.gem_inventory
    else {
        unreachable!()
    };
    *usage_inputs =
        usage_inputs_identity(&prior.input().normalization, Default::default()).unwrap();
    let old_dep = &prior
        .input()
        .normalization
        .configuration_dispositions
        .as_ref()
        .unwrap()
        .dependencies;
    let dep = &mut inverse
        .normalization
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .dependencies;
    dep.usage_inputs = old_dep.usage_inputs;
    assert_eq!(
        &inverse.normalization,
        &prior.input().normalization,
        "only inventory, exact reviewed companion additions and checked usage commitments differ"
    );
    assert_eq!(
        inverse.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    inverse.tree = prior.input().tree.clone();
    let provenance = inverse.provenance.pop().unwrap();
    assert_eq!(provenance.kind.as_str(), provenance_kind);
    assert_eq!(provenance.prior_input, prior.receipt().input);
    assert_eq!(provenance.authoring_input, authoring);
    assert!(&inverse == prior.input(), "exact complete release inverse");
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check();
    let inventory = serde_json::from_value(read(data().join("inventory.json"))).unwrap();
    stage_inventory(
        prior,
        inventory,
        companions::included(&[]),
        "bounded-preset-usage-field-accounting",
        authored_identity(),
    )
}
/// Test-only conservative omission. No new authority is authored by this helper.
pub fn stage_omission_control(
    prior: &StagedOwnedRelease,
    omitted: &[String],
) -> StagedOwnedRelease {
    check();
    let evidence = read(data().join("evidence.json"));
    let allowed: std::collections::BTreeSet<_> = evidence["support_family_expansion"]["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["gem"]["key"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(allowed.len(), 9);
    let selected: std::collections::BTreeSet<_> = omitted.iter().cloned().collect();
    assert!(!selected.is_empty());
    assert_eq!(selected.len(), omitted.len());
    assert!(selected.is_subset(&allowed));
    let mut inventory: UsageInventoryPolicy =
        serde_json::from_value(read(data().join("inventory.json"))).unwrap();
    let original = inventory.clone();
    inventory.rows.retain(|r| {
        let source = serde_json::to_value(&r.source).unwrap();
        !(source["kind"] == "physical" && selected.contains(source["gem"]["key"].as_str().unwrap()))
    });
    assert_eq!(inventory.rows.len(), 25 - selected.len());
    assert_eq!(
        original
            .rows
            .iter()
            .filter(|r| !inventory.rows.contains(r))
            .count(),
        selected.len()
    );
    let commitment = digest_owned(
        "owned-usage-inventory-omission-control-v1",
        &json!({"authored":authored_identity(),"omitted":selected}),
        4 * 1024 * 1024,
    )
    .unwrap();
    stage_inventory(
        prior,
        inventory,
        companions::included(&[]),
        "test-only-preset-usage-inventory-omission",
        commitment,
    )
}
/// Test-only source-consumer refusal; the complete 25-row field inventory stays intact.
pub fn stage_companion_omission_control(
    prior: &StagedOwnedRelease,
    omitted: &[String],
) -> StagedOwnedRelease {
    check();
    assert!(!omitted.is_empty());
    let additions = companions::included(omitted);
    assert_eq!(additions.len() + omitted.len(), 9);
    let inventory = serde_json::from_value(read(data().join("inventory.json"))).unwrap();
    let commitment = digest_owned(
        "owned-usage-count-companion-omission-control-v1",
        &json!({"authored":authored_identity(),"omitted":omitted}),
        4 * 1024 * 1024,
    )
    .unwrap();
    stage_inventory(
        prior,
        inventory,
        additions,
        "test-only-usage-count-companion-omission",
        commitment,
    )
}
fn stage_inventory(
    prior: &StagedOwnedRelease,
    inventory: UsageInventoryPolicy,
    added_companions: Vec<UsageGroupCompanion>,
    provenance_kind: &str,
    authoring: OwnedContentDigest,
) -> StagedOwnedRelease {
    assert_eq!(prior.receipt().input.to_string(), BEFORE);
    let old = prior.input();
    let expected = inventory.clone();
    companions::authenticate(prior);
    let mut normalization = old.normalization.clone();
    companions::apply(&mut normalization, &added_companions, false);
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: current, ..
    }) = &mut normalization.usage_inputs
    else {
        panic!("current occurrence usage policy")
    };
    assert!(current.is_none());
    *current = Some(inventory);
    let identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("current physical inventory")
    };
    *usage_inputs = identity;
    let dependencies =
        configuration_disposition_dependencies(&normalization, prior.rewards(), Default::default())
            .unwrap();
    let disposition = normalization.configuration_dispositions.as_mut().unwrap();
    let mut dependency_inverse = dependencies.clone();
    assert_eq!(dependencies.usage_inputs, identity);
    dependency_inverse.usage_inputs = disposition.dependencies.usage_inputs;
    assert_eq!(dependency_inverse, disposition.dependencies);
    disposition.dependencies = dependencies;
    let mut inverse = normalization.clone();
    companions::apply(&mut inverse, &added_companions, true);
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. }) = &mut inverse.usage_inputs
    else {
        unreachable!()
    };
    *inventory = None;
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut inverse.gem_inventory
    else {
        unreachable!()
    };
    *usage_inputs = usage_inputs_identity(&old.normalization, Default::default()).unwrap();
    inverse
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .dependencies = old
        .normalization
        .configuration_dispositions
        .as_ref()
        .unwrap()
        .dependencies
        .clone();
    assert_eq!(
        inverse, old.normalization,
        "only declared inventory, reviewed companion additions and exact dependency commitments change"
    );
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: old.recipe.clone(),
            successor: old.recipe.clone(),
            mapping: old.mapping.clone(),
            roles: old.roles.clone(),
            normalization: old.normalization.clone(),
            rewards: old.rewards.clone(),
            query_sets: old.query_sets.clone(),
            items: old.items.clone(),
            item_source: old.item_source.clone(),
        },
        old.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = old.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|t| t.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(provenance_kind).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: authoring,
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    checked_inventory_successor(
        prior,
        &next,
        &expected,
        &added_companions,
        provenance_kind,
        authoring,
    );
    next
}
pub fn publish(input: &Path, out: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
