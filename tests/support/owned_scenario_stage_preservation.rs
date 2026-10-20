//! Exact corpus inverse for two saved configuration leaves becoming Scenario intent.
//! This proves transport and preservation, never Skill applicability or execution.
use crate::{family, preservation, selected};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterAssignment, ParameterValue, ScenarioUsageIntent},
    owned_definitions::{
        BoundedInteger, GameVersionNamespace, ParameterSlotDefId, SlotOwnerDefId, UsagePolicyDefId,
    },
    owned_draft::ScenarioUsageIntentDraft,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn attribute<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    row.attribute(name).unwrap().decoded().unwrap()
}
fn canonical_lineage(value: &mut Value, expected: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(lineage) = fields.get_mut("lineage") {
                assert_eq!(lineage, expected, "one authenticated project lineage");
                *lineage = json!("canonical-test-lineage");
            }
            for child in fields.values_mut() {
                canonical_lineage(child, expected);
            }
        }
        Value::Array(children) => {
            for child in children {
                canonical_lineage(child, expected);
            }
        }
        _ => {}
    }
}
fn remove_exact(rows: &mut Vec<Value>, expected: &Value) {
    let positions: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (row == expected).then_some(index))
        .collect();
    assert_eq!(positions.len(), 1, "exact single added row: {expected}");
    rows.remove(positions[0]);
}
fn origin(sidecar: &Value, source: SourceOccurrenceId) -> &Value {
    let expected = json!(source);
    let matches: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"] == expected)
        .collect();
    assert_eq!(matches.len(), 1, "exact full source occurrence");
    matches[0]
}
fn origin_mut(sidecar: &mut Value, source: SourceOccurrenceId) -> &mut Value {
    // Establish uniqueness before taking the mutable reference.
    origin(sidecar, source);
    let expected = json!(source);
    sidecar["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["source"] == expected)
        .unwrap()
}
fn member<'a>(draft: &'a Value, collection: &str, id: &Value) -> &'a Value {
    let matching: Vec<_> = draft["draft"][collection]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| &row["id"] == id)
        .collect();
    assert_eq!(matching.len(), 1, "exact {collection} member");
    matching[0]
}
fn unique_link(sidecar: &Value, source: SourceOccurrenceId, kind: &str) -> Value {
    let matching: Vec<_> = origin(sidecar, source)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == kind)
        .collect();
    assert_eq!(matching.len(), 1, "exact {kind} source link");
    matching[0].clone()
}
fn expected_intent(
    namespace: &GameVersionNamespace,
    policy: &str,
    parameter: &str,
    value: i64,
) -> (Value, Value) {
    let policy = UsagePolicyDefId::parse(namespace.clone(), policy).unwrap();
    let intent = ScenarioUsageIntent {
        policy: policy.clone(),
        parameters: vec![ParameterAssignment {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::UsagePolicy(policy.clone()),
                slot: ParameterSlotDefId::parse(namespace.clone(), parameter).unwrap(),
            },
            value: ParameterValue::Integer(BoundedInteger::new(value).unwrap()),
        }],
    };
    (json!(ScenarioUsageIntentDraft::from(intent)), json!(policy))
}

pub fn compare(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior_path: &Path,
    package: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
    assert!((1..=5).contains(&case));
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(prior.receipt().query_rows, 110);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(prior.evaluation().is_none());
    assert!(next.evaluation().is_none());
    let old_dir = out.join(format!("prior-original-{case:02}"));
    let new_dir = out.join(format!("original-{case:02}"));
    let mut before = read(old_dir.join("draft.json"));
    let mut after = read(new_dir.join("draft.json"));
    let mut old_sidecar = read(old_dir.join("sidecar.json"));
    let mut new_sidecar = read(new_dir.join("sidecar.json"));
    preservation::authenticate(&old_sidecar, &old_dir, prior_path, case, prior);
    preservation::authenticate(&new_sidecar, &new_dir, package, case, next);
    let old_lineage = old_sidecar["allocator_before"]["lineage"].clone();
    let new_lineage = new_sidecar["allocator_before"]["lineage"].clone();
    assert!(old_lineage.is_string() && new_lineage.is_string());
    for (draft, sidecar) in [(&before, &old_sidecar), (&after, &new_sidecar)] {
        assert_eq!(draft["draft"]["allocator"], sidecar["allocator_after"]);
        assert_eq!(
            sidecar["source_sha256"],
            json!(format!("{:x}", Sha256::digest(xml)))
        );
        assert_eq!(sidecar["source_bytes"], json!(xml.len()));
    }
    for value in [&mut before, &mut old_sidecar] {
        canonical_lineage(value, &old_lineage);
    }
    for value in [&mut after, &mut new_sidecar] {
        canonical_lineage(value, &new_lineage);
    }
    for field in ["source_allocator", "allocator_before", "allocator_after"] {
        assert_eq!(old_sidecar[field], new_sidecar[field], "{field} unchanged");
    }

    // Source occurrence ordinals are derived from the complete unchanged XML;
    // no hard-coded build ordinal or guessed Scenario ID establishes authority.
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    for sidecar in [&old_sidecar, &new_sidecar] {
        let origins = sidecar["origins"].as_array().unwrap();
        assert_eq!(origins.len(), evidence.rows().len());
        for (actual, expected) in origins.iter().zip(evidence.rows()) {
            assert_eq!(actual["source"], json!(expected.occurrence().id()));
        }
    }
    let configs: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Config")
        .collect();
    assert_eq!(configs.len(), 1);
    let config = configs[0];
    let scopes: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().parent() == Some(config.occurrence().id())
                && row.occurrence().name() == "ConfigSet"
        })
        .collect();
    assert_eq!(scopes.len(), 1, "these originals each retain one ConfigSet");
    let scope = scopes[0];
    assert_eq!(attribute(config, "activeConfigSet"), attribute(scope, "id"));
    let scope_id = scope.occurrence().id();
    let scenario_link = unique_link(&old_sidecar, scope_id, "scenario_preset");
    assert_eq!(
        unique_link(&new_sidecar, scope_id, "scenario_preset"),
        scenario_link
    );
    let scenario_id = scenario_link["value"].clone();
    let choice = unique_link(&old_sidecar, scope_id, "choice_preset");
    let completion = &member(&before, "choice_presets", &choice["value"])["choices"]["completion"];
    assert_eq!(completion["kind"], "pending");
    assert_eq!(completion["code"], "configuration-roles-not-converted");
    let fallback = json!({"kind":"issue","value":completion["id"]});
    assert!(
        origin(&old_sidecar, scope_id)["links"]
            .as_array()
            .unwrap()
            .contains(&fallback)
    );
    let old_scenario = member(&before, "scenario_presets", &scenario_id);
    assert_eq!(
        old_scenario["scenario"]["assumptions"]["members"]
            .as_array()
            .unwrap()
            .len(),
        34
    );
    assert_eq!(
        before["draft"]["rewards"]["members"]
            .as_array()
            .unwrap()
            .len(),
        [17, 18, 16, 17, 18][case - 1]
    );
    assert_eq!(
        old_scenario["scenario"]["usage_intents"]["members"],
        json!([])
    );
    member(&after, "scenario_presets", &scenario_id);
    let scenario = after["draft"]["scenario_presets"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == scenario_id)
        .unwrap();
    let intents = scenario["scenario"]["usage_intents"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(intents.len(), 2, "only two typed intent records are added");
    let mut source_leaves = vec![];
    for (name, policy, parameter, value) in [
        (
            "sigilOfPowerStages",
            family::SIGIL_POLICY,
            family::SIGIL_PARAMETER,
            1,
        ),
        (
            "conditionCorruptingCryStages",
            family::CRY_POLICY,
            family::CRY_PARAMETER,
            10,
        ),
    ] {
        let leaves: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| {
                row.occurrence().parent() == Some(scope_id)
                    && row
                        .attribute("name")
                        .is_some_and(|attribute| attribute.decoded().unwrap() == name)
            })
            .collect();
        assert_eq!(leaves.len(), 1, "one saved lane for {name}");
        let leaf = leaves[0];
        assert_eq!(leaf.occurrence().name(), "Placeholder");
        assert_eq!(attribute(leaf, "number"), value.to_string());
        assert!(leaf.children().is_empty());
        let leaf_id = leaf.occurrence().id();
        let (expected, policy) = expected_intent(
            &next.input().recipe.schema.namespace,
            policy,
            parameter,
            value,
        );
        remove_exact(intents, &expected);
        let intent_link = json!({
            "kind":"scenario_usage_intent",
            "value":{"scenario":scenario_id,"policy":policy}
        });
        assert!(
            !origin(&old_sidecar, scope_id)["links"]
                .as_array()
                .unwrap()
                .contains(&intent_link)
        );
        remove_exact(
            origin_mut(&mut new_sidecar, scope_id)["links"]
                .as_array_mut()
                .unwrap(),
            &intent_link,
        );
        assert_eq!(
            origin(&old_sidecar, leaf_id)["links"],
            json!([fallback]),
            "only the exact configuration fallback formerly accounted for this leaf"
        );
        let new_links = origin_mut(&mut new_sidecar, leaf_id)["links"]
            .as_array_mut()
            .unwrap();
        remove_exact(new_links, &scenario_link);
        remove_exact(new_links, &intent_link);
        assert!(
            new_links.is_empty(),
            "no extra or surviving fallback leaf link"
        );
        new_links.push(fallback.clone());
        source_leaves.push(json!({"name":name,"source":leaf_id,"saved_value":value}));
    }
    assert!(intents.is_empty());
    assert_eq!(
        before, after,
        "whole-draft inverse preserves every value, identity, order and pending obligation"
    );

    // Both complete commitment sets were authenticated before any replacement.
    // Mapping source, raw source identity, allocator and all unlisted fields
    // remain subject to exact whole-sidecar comparison.
    for field in [
        "definitions",
        "mapping",
        "registry",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
        "policy",
        "draft",
    ] {
        new_sidecar[field] = old_sidecar[field].clone();
    }
    let old_items = old_sidecar["item_texts"].as_array().unwrap();
    let new_items = new_sidecar["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_items.len(), new_items.len());
    for (old, new) in old_items.iter().zip(new_items) {
        assert_eq!(old["source"], new["source"]);
        assert_eq!(old["content_entry"], new["content_entry"]);
        for field in ["item_lines", "policy"] {
            new["attribution"][field] = old["attribution"][field].clone();
        }
    }
    assert_eq!(
        old_sidecar, new_sidecar,
        "whole-sidecar inverse permits only exact intent links, two leaf fallbacks and authenticated commitments"
    );
    let mut old_selection = selected::selection(xml, &old_dir);
    let mut new_selection = selected::selection(xml, &new_dir);
    canonical_lineage(&mut old_selection, &old_lineage);
    canonical_lineage(&mut new_selection, &new_lineage);
    assert_eq!(old_selection, new_selection, "saved selection is unchanged");
    assert_eq!(new_selection["scenario"], scenario_id);
    let old_report = selected::finalize_with_definitions(
        xml,
        &old_dir,
        &out.join(format!("prior-selection-{case:02}.json")),
        &prior_path.join("schema.json"),
    );
    let new_report = selected::finalize_with_definitions(
        xml,
        &new_dir,
        &out.join(format!("selection-{case:02}.json")),
        &package.join("schema.json"),
    );
    for (report, directory) in [(&old_report, &old_dir), (&new_report, &new_dir)] {
        let sidecar = read(directory.join("sidecar.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    let mut old_issues = old_report["finalization"]["issues"].clone();
    let mut new_issues = new_report["finalization"]["issues"].clone();
    canonical_lineage(&mut old_issues, &old_lineage);
    canonical_lineage(&mut new_issues, &new_lineage);
    assert_eq!(old_issues, new_issues, "no whole input issue retires");
    let count = new_issues.as_array().unwrap().len();
    assert_eq!(count, [107, 118, 110, 124, 5][case - 1]);
    fs::write(
        out.join(format!("report-{case:02}.json")),
        serde_json::to_vec(&new_report).unwrap(),
    )
    .unwrap();
    json!({
        "original":case,
        "added_intents":2,
        "source_leaves":source_leaves,
        "retired_leaf_fallback_links":2,
        "retired_whole_issues":0,
        "preserved_assumptions":34,
        "preserved_rewards":([17,18,16,17,18][case-1]),
        "selected_unresolved":count,
        "exact_draft_inverse":true,
        "exact_sidecar_inverse":true,
        "calculation":"not_run"
    })
}
