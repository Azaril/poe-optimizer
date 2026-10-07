//! Append requested participation through existing exact source/typed usage seams.
//! No rule, readiness, allocation, or completeness authority is added.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, OccurrenceUsageRule, PrimarySkillUsageInput, UsageInputPolicy,
        usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const KIND: &str = "occurrence-participation-inputs";
pub const PRIOR: &str = "03504a9f21dee158483d990ddad77d56c60559b35ac6ecef1fe89664ee75c8ae";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn packet(name: &str) -> Value {
    read(
        root()
            .join("data/owned/poe2/3887ae68/occurrence-participation-inputs")
            .join(name),
    )
}
fn pin(row: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(row["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len() as u64, row["bytes"]);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row["sha256"]);
    bytes
}
pub fn check_authored(full: bool) {
    let c = packet("changes.json");
    let e = packet("evidence.json");
    assert_eq!(e["prior_input"], PRIOR);
    assert_eq!(c["schema_version"], 1);
    assert_eq!(
        c["usage"],
        read(root().join("data/owned/poe2/3887ae68/skill-participation/usage.json"))
    );
    assert_eq!(c["physical"].as_array().unwrap().len(), 6);
    assert_eq!(c["occurrences"].as_array().unwrap().len(), 2);
    for row in c["physical"]
        .as_array()
        .unwrap()
        .iter()
        .chain(c["occurrences"].as_array().unwrap())
    {
        let mut after = row["after"].clone();
        assert_eq!(
            after["policies"].as_array_mut().unwrap().pop(),
            Some(c["usage"].clone())
        );
        if row["before"].is_null() {
            assert_eq!(after["gem"]["key"], "def.00000000000007dd");
            assert_eq!(after["policies"], json!([]));
        } else {
            if row["before"]["group_attributes"] == json!([]) {
                assert!(
                    after["group_attributes"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("enabled"))
                );
                after["group_attributes"] = json!([]);
            }
            assert_eq!(
                after, row["before"],
                "only participation and necessary group framing change"
            );
        }
    }
    for row in e["authored_files"].as_array().unwrap() {
        pin(row);
    }
    let manifest: Value = serde_json::from_slice(&pin(&e["source_manifest"])).unwrap();
    assert_eq!(e["source_revision"], manifest["upstream_revision"]);
    for row in e["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| *p == row)
                .count(),
            1
        );
    }
    for row in e["original_inventory"].as_array().unwrap() {
        pin(&row["fixture"]);
    }
    if full {
        for row in e["retained_reports"].as_array().unwrap() {
            pin(row);
        }
    }
}

fn edit(input: &mut UsageInputPolicy, forward: bool) {
    let c = packet("changes.json");
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        physical,
        occurrences,
        ..
    } = input;
    for row in c["physical"].as_array().unwrap() {
        let after: PrimarySkillUsageInput = serde_json::from_value(row["after"].clone()).unwrap();
        let index = physical.iter().position(|p| p.gem == after.gem);
        if row["before"].is_null() {
            if forward {
                assert!(index.is_none());
                physical.push(after);
            } else {
                assert_eq!(physical.remove(index.unwrap()), after);
            }
        } else {
            let before = serde_json::from_value(row["before"].clone()).unwrap();
            let (expected, replacement) = if forward {
                (before, after)
            } else {
                (after, before)
            };
            assert_eq!(physical[index.unwrap()], expected);
            physical[index.unwrap()] = replacement;
        }
    }
    for row in c["occurrences"].as_array().unwrap() {
        let before: OccurrenceUsageRule = serde_json::from_value(row["before"].clone()).unwrap();
        let after: OccurrenceUsageRule = serde_json::from_value(row["after"].clone()).unwrap();
        let index = occurrences
            .iter()
            .position(|p| p.target == before.target)
            .unwrap();
        let (expected, replacement) = if forward {
            (before, after)
        } else {
            (after, before)
        };
        assert_eq!(occurrences[index], expected);
        occurrences[index] = replacement;
    }
}
fn rebind_usage(input: &mut poe_optimizer_import::owned_normalize::NormalizationPolicy) {
    let digest = usage_inputs_identity(input, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut input.gem_inventory
    else {
        panic!("current physical inventory");
    };
    *usage_inputs = digest;
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored(true);
    assert_eq!(json!(prior.receipt().input), PRIOR);
    let base = prior.input();
    let mut normalization = base.normalization.clone();
    edit(normalization.usage_inputs.as_mut().unwrap(), true);
    rebind_usage(&mut normalization);
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: base.recipe.clone(),
            successor: base.recipe.clone(),
            mapping: base.mapping.clone(),
            roles: base.roles.clone(),
            normalization: base.normalization.clone(),
            rewards: base.rewards.clone(),
            query_sets: base.query_sets.clone(),
            items: base.items.clone(),
            item_source: base.item_source.clone(),
        },
        base.tree.clone().unwrap(),
        normalization,
        Default::default(),
    )
    .unwrap();
    let mut input = base.clone();
    input.normalization = transition.normalization().clone();
    input.tree = transition.tree().map(|t| t.input().clone());
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            KIND,
            &json!({"changes":packet("changes.json"),"evidence":packet("evidence.json")}),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert!(next.evaluation().is_none());
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    let mut inverse = next.input().clone();
    edit(inverse.normalization.usage_inputs.as_mut().unwrap(), false);
    rebind_usage(&mut inverse.normalization);
    inverse.tree.as_mut().unwrap().normalization = base.tree.as_ref().unwrap().normalization;
    let added = inverse.provenance.pop().unwrap();
    assert_eq!(added.kind.as_str(), KIND);
    assert_eq!(added.prior_input, prior.receipt().input);
    assert_eq!(
        inverse, *base,
        "whole package inverse; no readiness or numerical changes"
    );
    next
}
