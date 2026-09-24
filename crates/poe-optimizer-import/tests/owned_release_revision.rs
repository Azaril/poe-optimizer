//! A new release corrects data without changing monotonic migration semantics.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey, owned_schema::*,
};
use poe_optimizer_import::{owned_release::*, owned_release_revision::*, owned_successor::*};
use std::path::PathBuf;

fn prior() -> StagedOwnedRelease {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let initial = transition_owned_bundle(fixture::input(&root), Default::default()).unwrap();
    let next = fixture::next(&initial);
    let bundle = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        fixture::tree(&next),
        Default::default(),
    )
    .unwrap();
    assemble_owned_release(
        OwnedReleaseInput {
            schema_version: 1,
            recipe: bundle.recipe().clone(),
            mapping: bundle.mapping().input().clone(),
            roles: bundle.roles().input().clone(),
            normalization: bundle.normalization().clone(),
            rewards: bundle.rewards().input().clone(),
            items: bundle.items().input().clone(),
            item_source: bundle.item_source().input().clone(),
            tree: bundle.tree().map(|tree| tree.input().clone()),
            query_sets: bundle.query_sets().to_vec(),
            provenance: vec![],
        },
        Default::default(),
    )
    .unwrap()
}
fn correction(prior: &StagedOwnedRelease) -> OwnedReleaseRevisionInput {
    let mut gem = prior.input().recipe.schema.definitions.iter().find(|row| {
        matches!(row, DefinitionDescriptor::Gem(entry) if matches!(&entry.schema, SchemaState::Known(schema)
            if schema.declarations.parameters.is_complete()))
    }).unwrap().clone();
    let subject = SchemaSubject::Definition(gem.address());
    let DefinitionDescriptor::Gem(entry) = &mut gem else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let gaps = vec![SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: OwnedDefinitionKey::new("intrinsic-input-review-incomplete").unwrap(),
    }];
    schema.declarations.parameters.closure = SchemaClosure::Partial { gaps: gaps.clone() };
    schema.declarations.choices.closure = SchemaClosure::Partial { gaps };
    OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: OwnedDefinitionKey::new("explicit-corrected-release").unwrap(),
        reason: OwnedDefinitionKey::new("correct-premature-input-closure").unwrap(),
        definitions: vec![gem],
        slots: vec![],
    }
}

#[test]
fn explicit_release_correction_preserves_ids_rules_policies_queries_and_prior_bytes() {
    let prior = prior();
    let prior_bytes = serde_json::to_vec(prior.input()).unwrap();
    let revision = correction(&prior);
    let target = revision.definitions[0].address();
    let correction_hash =
        digest_owned("owned-release-schema-revision-v1", &revision, 1024 * 1024).unwrap();
    let revised =
        compile_owned_release_revision(&prior, revision.clone(), Default::default()).unwrap();
    assert_ne!(revised.receipt().input, prior.receipt().input);
    assert_ne!(revised.receipt().definitions, prior.receipt().definitions);
    assert_eq!(
        revised.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(revised.query_sets(), prior.query_sets());
    assert_eq!(revised.receipt().query_rows, 110);
    assert_eq!(revised.receipt().source, prior.receipt().source);
    let provenance = revised.input().provenance.last().unwrap();
    assert_eq!(provenance.prior_input, prior.receipt().input);
    assert_eq!(provenance.authoring_input, correction_hash);
    assert_eq!(provenance.kind, revision.reason);
    assert_eq!(serde_json::to_vec(prior.input()).unwrap(), prior_bytes);

    let mut restored = revised.input().clone();
    restored.recipe.schema.release = prior.input().recipe.schema.release.clone();
    let original = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == target)
        .unwrap();
    *restored
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|row| row.address() == target)
        .unwrap() = original.clone();
    restored.recipe.rules.definitions = prior.receipt().definitions.clone();
    restored.recipe.routing.definitions = prior.receipt().definitions.clone();
    restored.mapping.definitions = prior.receipt().definitions.clone();
    restored.roles.definitions = prior.receipt().definitions.clone();
    restored.roles.mapping = prior.receipt().mapping;
    if let poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(quality) =
        &mut restored.normalization.gem_quality
    {
        quality.definitions = prior.receipt().definitions.clone();
    }
    if let Some(gems) = &mut restored.normalization.gem_inputs {
        gems.definitions = prior.receipt().definitions.clone();
    }
    restored.rewards.definitions = prior.receipt().definitions.clone();
    restored.rewards.mapping = prior.receipt().mapping;
    restored.items.definitions = prior.receipt().definitions.clone();
    restored.item_source.item_lines = prior.receipt().items;
    // Every tree content field must stay equal; only its verified binding fields differ.
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    restored.tree = prior.input().tree.clone();
    restored.provenance = prior.input().provenance.clone();
    assert!(
        restored == *prior.input(),
        "undeclared semantic change in release correction"
    );
    let repeated = compile_owned_release_revision(&prior, revision, Default::default()).unwrap();
    assert!(
        repeated.artifacts().eq(revised.artifacts()),
        "release build must be reproducible"
    );
    assert!(
        compile_owned_release_revision(&revised, correction(&prior), Default::default()).is_err()
    );
}

#[test]
fn revision_rejects_stale_unsupported_duplicate_unchanged_and_noncanonical_targets() {
    let prior = prior();
    let valid = correction(&prior);
    let mut inputs = vec![];
    let mut input = valid.clone();
    input.schema_version = 2;
    inputs.push(input);
    let mut input = valid.clone();
    input.before = digest_owned("wrong-endpoint", &1, 100).unwrap();
    inputs.push(input);
    let mut input = valid.clone();
    input.release = prior.input().recipe.schema.release.clone();
    inputs.push(input);
    let mut input = valid.clone();
    input.definitions.clear();
    inputs.push(input);
    let mut input = valid.clone();
    input.definitions.push(input.definitions[0].clone());
    inputs.push(input);
    let mut input = valid.clone();
    let target = input.definitions[0].address();
    input.definitions[0] = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == target)
        .unwrap()
        .clone();
    inputs.push(input);
    for (index, input) in inputs.into_iter().enumerate() {
        assert!(
            compile_owned_release_revision(&prior, input, Default::default()).is_err(),
            "invalid case {index}"
        );
    }
    let mut wire = serde_json::to_value(valid).unwrap();
    wire["skip_preservation"] = true.into();
    assert!(serde_json::from_value::<OwnedReleaseRevisionInput>(wire).is_err());
}

#[test]
fn revision_compiles_changed_schema_and_bounds_work_before_expansion() {
    let prior = prior();
    let mut invalid = correction(&prior);
    let DefinitionDescriptor::Gem(entry) = &mut invalid.definitions[0] else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema.level.minimum = schema.level.maximum;
    schema.level.maximum = poe_optimizer_core::owned_definitions::BoundedInteger::new(0).unwrap();
    assert!(compile_owned_release_revision(&prior, invalid, Default::default()).is_err());
    let valid = correction(&prior);
    for limits in [
        OwnedReleaseLimits {
            max_artifact_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_input_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 0,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_release_revision(&prior, valid.clone(), limits).is_err());
    }
}

#[test]
fn existing_slot_revision_rebinds_without_allocating_or_changing_its_owner() {
    use poe_optimizer_core::owned_definitions::FiniteQuantity;
    let prior = prior();
    let mut row = prior
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| {
            matches!(row, SlotDescriptor::Parameter(entry) if matches!(&entry.schema,
            SchemaState::Known(ParameterSlotSchema { value: ValueSchema::Quantity(range), .. })
            if range.maximum.value().abs() < 1e12))
        })
        .expect("fixture has bounded numeric input slots")
        .clone();
    let target = row.address();
    let SlotDescriptor::Parameter(entry) = &mut row else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let ValueSchema::Quantity(range) = &mut schema.value else {
        unreachable!()
    };
    range.maximum =
        FiniteQuantity::new(range.maximum.value() + 1.0, range.maximum.unit().clone()).unwrap();
    let mut policy = correction(&prior);
    policy.definitions.clear();
    policy.slots = vec![row.clone()];
    let result =
        compile_owned_release_revision(&prior, policy.clone(), Default::default()).unwrap();
    assert_eq!(
        result.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(
        result
            .input()
            .recipe
            .schema
            .slots
            .iter()
            .find(|slot| slot.address() == target),
        Some(&row)
    );
    assert_eq!(
        result.input().recipe.schema.definitions,
        prior.input().recipe.schema.definitions
    );
    assert_eq!(result.query_sets(), prior.query_sets());
    policy.slots.push(row);
    assert!(compile_owned_release_revision(&prior, policy, Default::default()).is_err());
    let mut with_provenance = prior.input().clone();
    with_provenance.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("reviewed-parent").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned("test-prior", &1, 100).unwrap(),
    });
    let prior = assemble_owned_release(with_provenance, Default::default()).unwrap();
    assert!(
        compile_owned_release_revision(
            &prior,
            correction(&prior),
            OwnedReleaseLimits {
                max_provenance_entries: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
