//! Explicit source-policy authoring closes scope alone, with unchanged game data.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::{BuildLineage, BuildRevision, InstanceAllocatorState, InstanceId},
    owned_build::LoadoutScope,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
};
use poe_optimizer_import::{
    owned_mapping::{OwnedMappingIndex, SourceComponent},
    owned_normalize::{NormalizationPolicy, SkillScopePolicy},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, NamedQuerySet, OWNED_SUCCESSOR_VERSION,
        SuccessorBundleInput, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
    owned_tree_policy::{OwnedTreeNormalizationPolicy, TreeNormalizationPackageInput},
};
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}

// ItemSourceBinding is export-only. This strict test wire preserves every field
// while distinguishing its original-source watermark from the later draft state.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceBindingWire {
    source_sha256: String,
    source_bytes: usize,
    source_schema: u32,
    lineage: BuildLineage,
    revision: BuildRevision,
    allocator: InstanceAllocatorState,
}
// Fresh CLI imports receive independent host lineages. Resolving a scope also
// removes one issue allocation from the same monotonic allocator. Account only
// for those exact former issue IDs; preserve all other order and references.
pub(super) fn canonical_instances(
    value: &mut Value,
    expected: BuildLineage,
    removed: &[u64],
) -> usize {
    match value {
        Value::Object(object)
            if object.contains_key("lineage") && object.contains_key("allocator") =>
        {
            let mut binding: SourceBindingWire = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(binding.lineage, expected, "unexpected source lineage");
            assert_eq!(
                binding.allocator.lineage(),
                expected,
                "foreign source allocator"
            );
            // Normalization issues are allocated after importing the source.
            // They cannot change that earlier source identity or its watermark.
            assert!(
                removed
                    .iter()
                    .all(|id| *id > binding.allocator.last_issued())
            );
            let canonical = BuildLineage::from_bytes([0; 16]);
            binding.lineage = canonical;
            binding.allocator =
                InstanceAllocatorState::from_parts(canonical, binding.allocator.last_issued());
            *value = serde_json::to_value(binding).unwrap();
            2
        }
        Value::Object(object) if object.contains_key("lineage") => {
            let canonical = BuildLineage::from_bytes([0; 16]);
            if object.contains_key("local") {
                let id: InstanceId = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(id.lineage(), expected, "unexpected foreign lineage");
                assert!(
                    !removed.contains(&id.local()),
                    "removed issue still referenced"
                );
                let local = id.local()
                    - u64::try_from(removed.iter().filter(|n| **n < id.local()).count()).unwrap();
                *value = serde_json::to_value(InstanceId::from_parts(canonical, local).unwrap())
                    .unwrap();
            } else {
                let allocator: InstanceAllocatorState =
                    serde_json::from_value(value.clone()).unwrap();
                assert_eq!(allocator.lineage(), expected, "unexpected foreign lineage");
                assert!(removed.iter().all(|n| *n <= allocator.last_issued()));
                *value = serde_json::to_value(InstanceAllocatorState::from_parts(
                    canonical,
                    allocator.last_issued() - u64::try_from(removed.len()).unwrap(),
                ))
                .unwrap();
            }
            1
        }
        Value::Object(object) => object
            .values_mut()
            .map(|value| canonical_instances(value, expected, removed))
            .sum(),
        Value::Array(values) => values
            .iter_mut()
            .map(|value| canonical_instances(value, expected, removed))
            .sum(),
        _ => 0,
    }
}
#[test]
fn source_binding_normalization_preserves_earlier_watermark_and_all_source_fields() {
    let lineage = BuildLineage::from_bytes([7; 16]);
    let canonical = BuildLineage::from_bytes([0; 16]);
    let source = serde_json::json!({
        "source_sha256": "a".repeat(64),
        "source_bytes": 734,
        "source_schema": 1,
        "lineage": lineage,
        "revision": "0000000000000003",
        "allocator": InstanceAllocatorState::from_parts(lineage, 12),
    });
    let mut evidence = serde_json::json!({
        "source": source,
        "modifier": InstanceId::from_parts(lineage, 17).unwrap(),
        "draft_allocator": InstanceAllocatorState::from_parts(lineage, 24),
    });
    assert_eq!(canonical_instances(&mut evidence, lineage, &[14, 22]), 4);
    let mut expected_source = source.clone();
    expected_source["lineage"] = serde_json::to_value(canonical).unwrap();
    expected_source["allocator"] =
        serde_json::to_value(InstanceAllocatorState::from_parts(canonical, 12)).unwrap();
    assert_eq!(evidence["source"], expected_source);
    assert_eq!(
        evidence["modifier"],
        serde_json::to_value(InstanceId::from_parts(canonical, 16).unwrap()).unwrap()
    );
    assert_eq!(
        evidence["draft_allocator"],
        serde_json::to_value(InstanceAllocatorState::from_parts(canonical, 22)).unwrap()
    );

    for case in 0..3 {
        let mut invalid = source.clone();
        let removed = match case {
            0 => {
                invalid["unexpected"] = serde_json::json!(true);
                14
            }
            1 => {
                invalid["allocator"] = serde_json::to_value(InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([8; 16]),
                    12,
                ))
                .unwrap();
                14
            }
            _ => 12,
        };
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                canonical_instances(&mut invalid, lineage, &[removed]);
            }))
            .is_err(),
            "invalid source binding case {case} was accepted"
        );
    }
}
pub fn check_skill_scopes(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("skill-scope-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let old_recipe = recipe(prior);
    let base = assemble_owned_recipe(old_recipe.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(prior.join("mapping.json"))).unwrap(),
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let old_normalization: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    assert!(old_normalization.skill_scopes.is_none());
    let old_tree: TreeNormalizationPackageInput =
        serde_json::from_value(json(prior.join("tree-normalization.json"))).unwrap();
    let checked_tree = OwnedTreeNormalizationPolicy::new(
        old_tree.clone(),
        base.registry(),
        base.schema(),
        &mapping,
        &old_normalization,
        Default::default(),
    )
    .unwrap();
    let policy: SkillScopePolicy =
        serde_json::from_value(json(authored.join("policy.json"))).unwrap();
    assert_eq!(policy.slot_attribute, "slot");
    assert_eq!(policy.shared_slots, [SourceComponent::Missing]);
    let mut normalization = old_normalization.clone();
    normalization.skill_scopes = Some(policy);
    let query_sets = (1..=5)
        .map(|case| NamedQuerySet {
            name: format!("original-{case:02}").parse().unwrap(),
            queries: serde_json::from_value(json(
                prior.join(format!("queries-original-{case:02}.json")),
            ))
            .unwrap(),
        })
        .collect();
    // A new authored normalization input is supplied explicitly. Validate the old
    // tree above, then install its unchanged content with this new binding; the
    // new transition is not a byte-preserving history claim about the old policy.
    let staged = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: OWNED_SUCCESSOR_VERSION,
            prior: old_recipe.clone(),
            successor: old_recipe.clone(),
            mapping: mapping.input().clone(),
            roles: serde_json::from_value(json(prior.join("roles.json"))).unwrap(),
            normalization: normalization.clone(),
            rewards: serde_json::from_value(json(prior.join("rewards.json"))).unwrap(),
            query_sets,
            items: serde_json::from_value(json(prior.join("items.json"))).unwrap(),
            item_source: serde_json::from_value(json(prior.join("item-source.json"))).unwrap(),
        },
        CatalogAppend {
            mappings: vec![],
            source: mapping.input().source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::Install {
            content: Box::new(checked_tree.input().content.clone()),
        },
        SuccessorBundleLimits::default(),
    )
    .unwrap();
    assert_eq!(staged.recipe(), &old_recipe);
    assert_eq!(staged.mapping().input(), mapping.input());
    assert_eq!(staged.normalization(), &normalization);
    assert_eq!(staged.tree().unwrap().input().content, old_tree.content);
    assert_ne!(staged.tree().unwrap().identity(), checked_tree.identity());
    assert_eq!(staged.transition().query_rows, 110);
    assert_eq!(staged.transition().whole_build_parity, "not_established");
    assert_eq!(staged.transition().calculation, "not_run");

    let output = cwd.join("skill-scope-successor");
    fs::create_dir(&output).unwrap();
    for (name, bytes) in staged.artifacts() {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }
    let published = bundle(&output);
    for name in [
        "registry.json",
        "schema.json",
        "rules.json",
        "routing.json",
        "mapping.json",
        "roles.json",
        "rewards.json",
        "items.json",
        "item-source.json",
    ] {
        assert_eq!(
            published[name], prior_bytes[name],
            "scope authoring changed {name}"
        );
    }
    let mut restored_normalization = staged.normalization().clone();
    restored_normalization.skill_scopes = None;
    assert_eq!(restored_normalization, old_normalization);

    let mut scope_counts = Vec::new();
    let mut complete_gems = 0;
    let mut pending_gems = 0;
    let mut modifiers = 0;
    let mut displays = 0;
    let mut queries = 0;
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(published[&query], prior_bytes[&query]);
        let destination = cwd.join(format!("skill-scope-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let previous = decode_draft(
            &fs::read(cwd.join(format!("item-rarity-chaos-original-{case}/draft.json"))).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        assert!(input.skills.members.iter().all(|skill| matches!(
            skill.scope,
            DraftField::Known {
                value: LoadoutScope::Shared
            }
        )));
        assert!(
            previous
                .input()
                .skills
                .members
                .iter()
                .all(|skill| matches!(skill.scope, DraftField::Pending(_)))
        );
        scope_counts.push(input.skills.members.len());
        let mut current = serde_json::to_value(input).unwrap();
        let mut old = serde_json::to_value(previous.input()).unwrap();
        let removed: Vec<_> = previous
            .input()
            .skills
            .members
            .iter()
            .map(|skill| {
                let DraftField::Pending(pending) = &skill.scope else {
                    unreachable!()
                };
                assert_eq!(pending.code.as_str(), "skill-scope-not-converted");
                pending.id.instance_id().local()
            })
            .collect();
        assert_eq!(
            previous.input().allocator.last_issued() - input.allocator.last_issued(),
            u64::try_from(removed.len()).unwrap()
        );
        let current_skills = current["skills"]["members"].as_array().unwrap();
        let previous_skills = old["skills"]["members"].as_array_mut().unwrap();
        assert_eq!(current_skills.len(), previous_skills.len());
        for (skill, previous) in current_skills.iter().zip(previous_skills) {
            previous["scope"] = skill["scope"].clone();
        }
        let count = canonical_instances(&mut current, input.allocator.lineage(), &[]);
        assert!(count > 0);
        assert_eq!(
            count,
            canonical_instances(&mut old, previous.input().allocator.lineage(), &removed)
        );
        assert_eq!(
            current, old,
            "original-{case}: fields beyond skill scope changed"
        );
        for gem in &input.gems.members {
            assert!(gem.parameters.members.is_empty());
            match gem.parameters.completion {
                DraftListCompletion::Complete => complete_gems += 1,
                DraftListCompletion::Pending { .. } => pending_gems += 1,
            }
        }
        assert!(
            !draft
                .validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
        assert!(!input.allocations.members.is_empty());
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|allocation| matches!(allocation.access, DraftAllocationAccess::Pending(_)))
        );
        for item in &input.items.members {
            assert!(item.to_resolved().is_none());
            assert!(matches!(
                item.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                item.modifiers.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(item.modifier_order, DraftField::Pending(_)));
        }
        if matches!(case, 2 | 3) {
            assert!(matches!(
                input.items.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                input.equipment.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        let query_count = input
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum::<usize>();
        assert_eq!(query_count, 22);
        queries += query_count;
        let sidecar = json(destination.join("sidecar.json"));
        let old_sidecar = json(cwd.join(format!("item-rarity-chaos-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(sidecar[field], old_sidecar[field]);
        }
        // Retain exact source-line-to-modifier attribution as well as the owned
        // draft. Counts alone cannot detect swapped references on equal rolls.
        let mut current_item_texts = sidecar["item_texts"].clone();
        let mut previous_item_texts = old_sidecar["item_texts"].clone();
        let item_reference_count =
            canonical_instances(&mut current_item_texts, input.allocator.lineage(), &[]);
        assert!(item_reference_count > 0);
        assert_eq!(
            item_reference_count,
            canonical_instances(
                &mut previous_item_texts,
                previous.input().allocator.lineage(),
                &removed,
            )
        );
        assert_eq!(
            current_item_texts, previous_item_texts,
            "original-{case}: item source attribution changed"
        );
        let new_items = sidecar["item_texts"].as_array().unwrap();
        let old_items = old_sidecar["item_texts"].as_array().unwrap();
        assert_eq!(new_items.len(), old_items.len());
        for (item, old_item) in new_items.iter().zip(old_items) {
            assert_eq!(item["source"], old_item["source"]);
            let lines = item["lines"].as_array().unwrap();
            let old_lines = old_item["lines"].as_array().unwrap();
            assert_eq!(lines.len(), old_lines.len());
            for (line, old_line) in lines.iter().zip(old_lines) {
                for field in ["index", "text", "outcome"] {
                    assert_eq!(line[field], old_line[field]);
                }
                let emitted = line["modifiers"].as_array().unwrap().len();
                assert_eq!(emitted, old_line["modifiers"].as_array().unwrap().len());
                modifiers += emitted;
                if line["outcome"]["kind"] == "known"
                    && line["outcome"]["value"]["rule"]
                        .as_str()
                        .is_some_and(|rule| rule.starts_with("observed-"))
                {
                    displays += 1;
                }
            }
        }
    }
    assert_eq!(scope_counts, [9, 63, 9, 13, 46]);
    assert_eq!((complete_gems, pending_gems), (0, 478));
    assert_eq!((modifiers, displays, queries), (64, 53, 110));
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
