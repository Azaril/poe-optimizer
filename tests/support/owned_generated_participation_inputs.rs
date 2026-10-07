//! Saved participation transport through existing exact occurrence bindings.
//! This packet changes no Skill schema, rules, readiness, or usage completeness.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, NormalizationPolicy, OccurrenceUsagePolicy, UsageInputPolicy,
        usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

pub const KIND: &str = "generated-participation-inputs";
pub const PRIOR: &str = "21b327c2364f5f5d9ae7c47c5c2293ac310d5e5c113e536d5bde25d93b410ff9";
pub const ROWS: [usize; 4] = [0, 2, 4, 5];

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/generated-participation")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
pub fn policy() -> OccurrenceUsagePolicy {
    read("usage.json")
}

pub fn check_authored() {
    let actual: Value = read("usage.json");
    let existing: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/skill-participation/usage.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        actual, existing,
        "reuse the existing two-input policy verbatim"
    );
    let typed = policy();
    assert_eq!(typed.policy.key().as_str(), "def.000000000000332b");
    assert_eq!(typed.parameters.len(), 2);
    for (i, (kind, slot)) in [
        ("containing_group", "def.000000000000332c"),
        ("occurrence", "def.000000000000332d"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(actual["parameters"][i]["source"]["kind"], kind);
        assert_eq!(actual["parameters"][i]["slot"]["slot"]["key"], slot);
        assert_eq!(
            actual["parameters"][i]["source"]["value"]["missing"]["kind"],
            "pending"
        );
    }
}

fn check_rows(normalization: &NormalizationPolicy, appended: bool) {
    let UsageInputPolicy::PobOccurrenceUsageV3 { occurrences, .. } =
        normalization.usage_inputs.as_ref().unwrap();
    assert_eq!(occurrences.len(), 6);
    let actual = json!(occurrences);
    for (i, (kind, skill)) in [
        ("authored_direct", "SummonSandDjinnPlayer"),
        ("authored_direct", "SummonWaterDjinnPlayer"),
        ("generated", "SummonSandDjinnPlayer"),
        ("generated", "SummonWaterDjinnPlayer"),
        ("generated", "FireboltPlayer"),
        ("generated", "FireboltPlayer"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(actual[i]["target"]["kind"], kind);
        let target = if kind == "generated" {
            &actual[i]["target"]["correspondence"]
        } else {
            &actual[i]["target"]
        };
        assert_eq!(target["skill_id"], skill);
        assert_eq!(
            occurrences[i].policies.len(),
            1 + usize::from(appended && ROWS.contains(&i))
        );
        assert_eq!(
            occurrences[i].policies[0].policy.key().as_str(),
            "def.000000000000326a"
        );
        if appended && ROWS.contains(&i) {
            assert_eq!(occurrences[i].policies[1], policy());
        }
    }
    assert_eq!(
        actual[2]["target"]["correspondence"]["provider"]["source_node_id"],
        "13289"
    );
    for (i, name) in [(4, "Sol Pole, Ashen Staff"), (5, "New Item, Ashen Staff")] {
        let provider = &actual[i]["target"]["correspondence"]["provider"];
        assert_eq!(provider["kind"], "item_modifier");
        assert_eq!(provider["modifier"]["key"], "def.00000000000031c6");
        assert_eq!(
            provider["skill_supply"]["slot"]["key"],
            "def.00000000000031c9"
        );
        assert_eq!(provider["source_name"], name);
    }
}

pub fn assert_endpoint(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    check_rows(prior.normalization(), false);
    check_rows(next.normalization(), true);
    assert_eq!(prior.input().recipe, next.input().recipe);
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        physical: before, ..
    } = prior.normalization().usage_inputs.as_ref().unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        physical: after, ..
    } = next.normalization().usage_inputs.as_ref().unwrap();
    assert_eq!(before, after, "physical policies remain exact");
    let mut inverse = next.input().clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { occurrences, .. } =
        inverse.normalization.usage_inputs.as_mut().unwrap();
    for i in ROWS {
        assert_eq!(occurrences[i].policies.pop(), Some(policy()));
    }
    let restored_usage = usage_inputs_identity(&inverse.normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut inverse.normalization.gem_inventory
    else {
        panic!("current physical inventory")
    };
    *usage_inputs = restored_usage;
    // Release assembly validated each tree's actual normalization digest.
    // Restore that dependency; keep all content and other bindings exact.
    inverse.tree.as_mut().unwrap().normalization =
        prior.input().tree.as_ref().unwrap().normalization;
    assert_eq!(inverse.provenance.len(), prior.input().provenance.len() + 1);
    let added = inverse.provenance.pop().unwrap();
    assert_eq!(added.kind.as_str(), KIND);
    assert_eq!(added.prior_input, prior.receipt().input);
    assert!(
        inverse == *prior.input(),
        "whole release input differs beyond the four exact policy appends and authenticated commitments"
    );
}

/// The caller authenticates the source evidence before passing its owned payload.
/// No old migration or rule allocation is replayed to append usage bindings.
pub fn stage(prior: &StagedOwnedRelease, source_evidence: &Value) -> StagedOwnedRelease {
    check_authored();
    assert_eq!(json!(prior.receipt().input), PRIOR);
    check_rows(prior.normalization(), false);
    let base = prior.input();
    let mut normalization = base.normalization.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { occurrences, .. } =
        normalization.usage_inputs.as_mut().unwrap();
    for i in ROWS {
        occurrences[i].policies.push(policy());
    }
    let usage = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("current physical inventory");
    };
    *usage_inputs = usage;
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
    input.tree = transition.tree().map(|tree| tree.input().clone());
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            KIND,
            &json!({"usage": policy(), "occurrence_rows": ROWS, "source_evidence": source_evidence}),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(prior, &next);
    next
}
