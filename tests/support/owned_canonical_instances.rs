//! Test-only canonicalization of fresh lineages and explicitly changed issue allocations.
use poe_optimizer_core::build_identity::{
    BuildLineage, BuildRevision, InstanceAllocatorState, InstanceId,
};
use serde_json::Value;

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
pub(crate) fn canonical_instances(
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
