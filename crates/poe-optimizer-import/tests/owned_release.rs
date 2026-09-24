//! Full releases validate one explicit endpoint; they do not reinterpret old releases.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;

use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_gem_schema::{GemSchemaMigrationInput, stage_owned_gem_schema},
    owned_item_lines::OwnedItemLinePolicy,
    owned_mapping::OwnedMappingIndex,
    owned_normalize::{GemInputPolicy, GemQualityPolicy, ImportQueryTemplate},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_release::*,
    owned_skill_catalog::OwnedSkillRoleIndex,
    owned_successor::*,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn other_digest() -> OwnedContentDigest {
    digest_owned("release-test-unrelated-content", &17, 100).unwrap()
}
fn input() -> OwnedReleaseInput {
    static INPUT: OnceLock<OwnedReleaseInput> = OnceLock::new();
    INPUT
        .get_or_init(|| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            let first =
                transition_owned_bundle_compact(fixture::input(&root), Default::default()).unwrap();
            let next = fixture::next(&first);
            let endpoint = transition_owned_catalog_with_tree_compact(
                next.clone(),
                fixture::append(&next),
                fixture::tree(&next),
                Default::default(),
            )
            .unwrap();
            OwnedReleaseInput {
                schema_version: OWNED_RELEASE_VERSION,
                recipe: endpoint.recipe().clone(),
                mapping: endpoint.mapping().input().clone(),
                roles: endpoint.roles().input().clone(),
                normalization: endpoint.normalization().clone(),
                rewards: endpoint.rewards().input().clone(),
                items: endpoint.items().input().clone(),
                item_source: endpoint.item_source().input().clone(),
                tree: Some(endpoint.tree().unwrap().input().clone()),
                query_sets: endpoint.query_sets().to_vec(),
                provenance: vec![],
            }
        })
        .clone()
}
fn stage(input: OwnedReleaseInput) -> StagedOwnedRelease {
    assemble_owned_release(input, Default::default()).unwrap()
}
fn files(release: &StagedOwnedRelease) -> BTreeMap<String, Vec<u8>> {
    release
        .artifacts()
        .map(|(name, bytes)| (name.to_string(), bytes.to_vec()))
        .collect()
}

#[test]
fn independent_assembly_preserves_every_exact_bound_artifact_and_all_ordered_queries() {
    let supplied = input();
    let release = stage(supplied.clone());
    assert!(release.input() == &supplied);
    assert_eq!(release.mapping().input(), &supplied.mapping);
    assert_eq!(release.roles().input(), &supplied.roles);
    assert_eq!(release.normalization(), &supplied.normalization);
    assert_eq!(release.rewards().input(), &supplied.rewards);
    assert_eq!(release.items().input(), &supplied.items);
    assert_eq!(release.item_source().input(), &supplied.item_source);
    assert_eq!(
        release.tree().unwrap().input(),
        supplied.tree.as_ref().unwrap()
    );
    assert_eq!(release.query_sets(), supplied.query_sets);
    let receipt = release.receipt();
    assert_eq!(receipt.schema_version, OWNED_RELEASE_VERSION);
    assert_eq!(
        receipt.registry,
        release.assembled().registry().identity().unwrap()
    );
    assert_eq!(
        &receipt.definitions,
        release.assembled().schema().identity()
    );
    assert_eq!(&receipt.mapping, release.mapping().identity());
    assert_eq!(&receipt.roles, release.roles().identity());
    assert_eq!(&receipt.rewards, release.rewards().identity());
    assert_eq!(&receipt.items, release.items().identity());
    assert_eq!(&receipt.item_source, release.item_source().identity());
    assert_eq!(
        receipt.tree.as_ref(),
        Some(release.tree().unwrap().identity())
    );
    assert_eq!(receipt.source.system, supplied.mapping.source.system);
    assert_eq!(receipt.source.revision, supplied.mapping.source.revision);
    let mut source_files = BTreeMap::new();
    for source in [
        &supplied.mapping.source,
        &supplied.roles.compilation.source,
        &supplied.item_source.source,
        &supplied.tree.as_ref().unwrap().content.source,
    ] {
        for file in &source.files {
            if let Some(previous) = source_files.insert(file.path.clone(), file.sha256.clone()) {
                assert_eq!(previous, file.sha256);
            }
        }
    }
    assert_eq!(
        receipt
            .source
            .files
            .iter()
            .map(|file| (file.path.clone(), file.sha256.clone()))
            .collect::<BTreeMap<_, _>>(),
        source_files
    );
    assert_eq!(receipt.query_sets, 5);
    assert_eq!(receipt.query_rows, 110);
    assert!(receipt.provenance.is_empty());
    let emitted = files(&release);
    assert_eq!(emitted.len(), receipt.artifacts.len() + 1);
    assert!(!emitted.contains_key("transition.json"));
    assert!(!emitted.contains_key("catalog-append.json"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&emitted["release.json"]).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    for artifact in &receipt.artifacts {
        let bytes = &emitted[&artifact.file];
        assert_eq!(bytes.len(), artifact.bytes);
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), artifact.sha256);
    }
    for artifact in release.assembled().artifacts() {
        assert_eq!(artifact.bytes(), emitted[artifact.name()]);
    }
    for set in &supplied.query_sets {
        let bytes = &emitted[&format!("queries-{}.json", set.name.as_str())];
        assert_eq!(
            serde_json::from_slice::<Vec<ImportQueryTemplate>>(bytes).unwrap(),
            set.queries
        );
    }
    let reconstructed = OwnedRecipeInput {
        schema_version: supplied.recipe.schema_version,
        registry: serde_json::from_slice(&emitted["registry.json"]).unwrap(),
        schema: serde_json::from_slice(&emitted["schema.json"]).unwrap(),
        rules: serde_json::from_slice(&emitted["rules.json"]).unwrap(),
        routing: serde_json::from_slice(&emitted["routing.json"]).unwrap(),
    };
    assert_eq!(reconstructed, supplied.recipe);
    let wire = serde_json::to_vec(&supplied).unwrap();
    let decoded = decode_owned_release(&wire, Default::default()).unwrap();
    assert!(decoded.input() == &supplied);
    assert_eq!(files(&decoded), emitted);
    assert_eq!(files(&stage(supplied)), emitted);
}

#[test]
fn every_dependency_rejects_stale_bindings_instead_of_repairing_them() {
    let base = input();
    for case in 0..15 {
        let mut bad = base.clone();
        let mut wrong_schema = bad.recipe.rules.definitions.clone();
        wrong_schema.content_sha256 = "0".repeat(64);
        match case {
            0 => bad.mapping.registry = other_digest(),
            1 => bad.mapping.definitions = wrong_schema,
            2 => bad.roles.mapping = other_digest(),
            3 => bad.roles.definitions = wrong_schema,
            4 => bad.rewards.mapping = other_digest(),
            5 => bad.rewards.definitions = wrong_schema,
            6 => bad.items.definitions = wrong_schema,
            7 => bad.item_source.item_lines = other_digest(),
            8 => bad.recipe.rules.definitions = wrong_schema,
            9 => bad.recipe.routing.definitions = wrong_schema,
            10 => bad.tree.as_mut().unwrap().registry = other_digest(),
            11 => bad.tree.as_mut().unwrap().definitions = wrong_schema,
            12 => bad.tree.as_mut().unwrap().mapping = other_digest(),
            13 => bad.tree.as_mut().unwrap().normalization = other_digest(),
            _ => {
                let GemQualityPolicy::Attributes(quality) = &mut bad.normalization.gem_quality
                else {
                    panic!("fixture must exercise schema-bound quality policy")
                };
                quality.definitions = wrong_schema;
            }
        }
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "case {case}"
        );
    }
    // A stale optional intrinsic policy must be checked even with zero recipes.
    let mut bad = base;
    let mut definitions = bad.recipe.rules.definitions.clone();
    definitions.content_sha256 = "1".repeat(64);
    bad.normalization.gem_inputs = Some(GemInputPolicy {
        definitions,
        gems: vec![],
    });
    assert!(assemble_owned_release(bad, Default::default()).is_err());
}

#[test]
fn optional_tree_is_explicit_and_cannot_retain_a_foreign_normalization_commitment() {
    let mut supplied = input();
    supplied.tree = None;
    let without = stage(supplied.clone());
    assert!(without.tree().is_none());
    assert!(without.receipt().tree.is_none());
    assert!(!files(&without).contains_key("tree-normalization.json"));
    let bytes = serde_json::to_vec(&supplied).unwrap();
    assert!(
        serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap()
            .get("tree")
            .is_none()
    );
    let mut mixed = input();
    mixed.normalization.version = key("separately-authored-normalization");
    assert!(assemble_owned_release(mixed, Default::default()).is_err());
    supplied.normalization.version = key("separately-authored-normalization");
    stage(supplied);
}

#[test]
fn duplicate_unsafe_and_invalid_query_sets_are_rejected_but_valid_order_is_preserved() {
    let base = input();
    for name in [
        "Original",
        "original.one",
        "../escape",
        "a/b",
        "a\\b",
        &"x".repeat(65),
    ] {
        let mut bad = base.clone();
        // Owned symbols already reject some unsafe spellings. The release must
        // reject every unsafe spelling that can reach its public typed input.
        if let Ok(name) = OwnedDefinitionKey::new(name) {
            bad.query_sets[0].name = name;
            assert!(assemble_owned_release(bad, Default::default()).is_err());
        }
    }
    let mut duplicate = base.clone();
    duplicate.query_sets.push(duplicate.query_sets[0].clone());
    assert!(assemble_owned_release(duplicate, Default::default()).is_err());
    let mut duplicate_query = base.clone();
    let query = duplicate_query.query_sets[0].queries[0].clone();
    duplicate_query.query_sets[0].queries.push(query);
    assert!(assemble_owned_release(duplicate_query, Default::default()).is_err());
    let mut wrong_domain = base.clone();
    wrong_domain.query_sets[0].queries[0].metric = wrong_domain
        .mapping
        .entries
        .iter()
        .find(|row| {
            matches!(
                row.source,
                poe_optimizer_import::owned_mapping::ExternalSelector::Definition(_)
            )
        })
        .unwrap()
        .source
        .clone();
    assert!(assemble_owned_release(wrong_domain, Default::default()).is_err());
    let old = stage(base.clone());
    let mut changed = base;
    changed.query_sets.reverse();
    changed.query_sets[0].queries.reverse();
    let new = stage(changed.clone());
    assert_eq!(new.query_sets(), changed.query_sets);
    assert_ne!(old.receipt().input, new.receipt().input);
}

#[test]
fn aggregate_and_constituent_limits_bound_release_work_and_output() {
    let supplied = input();
    let release = stage(supplied.clone());
    let emitted = files(&release);
    let output_bytes: usize = emitted.values().map(Vec::len).sum();
    let largest_artifact = emitted.values().map(Vec::len).max().unwrap();
    let wire = serde_json::to_vec(&supplied).unwrap();
    for limits in [
        OwnedReleaseLimits {
            max_input_bytes: wire.len() - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_output_bytes: output_bytes - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_artifact_bytes: largest_artifact - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_query_sets: supplied.query_sets.len() - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_queries: 109,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_query_commitment_bytes: release.receipt().query_policy_bytes - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_input_bytes: 0,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_queries: OwnedReleaseLimits::default().max_queries + 1,
            ..Default::default()
        },
    ] {
        assert!(assemble_owned_release(supplied.clone(), limits).is_err());
    }
    let mut limits = OwnedReleaseLimits::default();
    limits.tree.max_entries = 1;
    assert!(assemble_owned_release(supplied.clone(), limits).is_err());
    assert!(
        decode_owned_release(
            &wire,
            OwnedReleaseLimits {
                max_input_bytes: wire.len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn strict_wire_rejects_unknown_duplicate_missing_fields_and_versions() {
    let supplied = input();
    let wire = serde_json::to_vec(&supplied).unwrap();
    let mut unknown = serde_json::to_value(&supplied).unwrap();
    unknown["assume_valid"] = true.into();
    assert!(
        decode_owned_release(&serde_json::to_vec(&unknown).unwrap(), Default::default()).is_err()
    );
    let mut missing = serde_json::to_value(&supplied).unwrap();
    missing.as_object_mut().unwrap().remove("mapping");
    assert!(
        decode_owned_release(&serde_json::to_vec(&missing).unwrap(), Default::default()).is_err()
    );
    let duplicate = String::from_utf8(wire).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(decode_owned_release(duplicate.as_bytes(), Default::default()).is_err());
    let mut bad = supplied;
    bad.schema_version = OWNED_RELEASE_VERSION + 1;
    assert!(assemble_owned_release(bad, Default::default()).is_err());
}

#[test]
fn declared_authoring_provenance_is_committed_bounded_and_does_not_change_game_data() {
    let base = input();
    let old = stage(base.clone());
    assert!(
        serde_json::to_value(&base)
            .unwrap()
            .get("provenance")
            .is_none()
    );
    let mut declared = base;
    declared.provenance = vec![OwnedReleaseProvenance {
        kind: key("test-schema-authoring"),
        prior_input: old.receipt().input,
        authoring_input: other_digest(),
    }];
    let new = stage(declared.clone());
    assert_eq!(new.receipt().provenance, declared.provenance);
    assert_ne!(old.receipt().input, new.receipt().input);
    assert_eq!(old.receipt().definitions, new.receipt().definitions);
    assert_eq!(old.query_sets(), new.query_sets());
    let mut old_files = files(&old);
    let mut new_files = files(&new);
    old_files.remove("release.json");
    new_files.remove("release.json");
    assert_eq!(old_files, new_files);
    declared.provenance.push(OwnedReleaseProvenance {
        kind: key("second-authoring-step"),
        prior_input: new.receipt().input,
        authoring_input: other_digest(),
    });
    assert!(
        assemble_owned_release(
            declared,
            OwnedReleaseLimits {
                max_provenance_entries: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
/// Test-side authoring deliberately updates each dependency. This helper is not
/// a production repair path: the release assembler receives the finished input.
fn rebind(input: &mut OwnedReleaseInput) {
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), Default::default()).unwrap();
    input.recipe.rules.definitions = schema.identity().clone();
    input.recipe.routing.definitions = schema.identity().clone();
    let recipe = assemble_owned_recipe(input.recipe.clone(), Default::default()).unwrap();
    input.recipe.registry = recipe.registry().input().clone();
    input.recipe.schema = recipe.schema().input().clone();
    input.recipe.rules = recipe.rules().input().clone();
    input.recipe.routing = recipe.routing().input().clone();
    input.mapping.registry = recipe.registry().identity().unwrap();
    input.mapping.definitions = recipe.schema().identity().clone();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        recipe.registry(),
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    input.roles.definitions = recipe.schema().identity().clone();
    input.roles.mapping = *mapping.identity();
    input.rewards.definitions = recipe.schema().identity().clone();
    input.rewards.mapping = *mapping.identity();
    if let GemQualityPolicy::Attributes(quality) = &mut input.normalization.gem_quality {
        quality.definitions = recipe.schema().identity().clone();
    }
    if let Some(gems) = &mut input.normalization.gem_inputs {
        gems.definitions = recipe.schema().identity().clone();
    }
    input.items.definitions = recipe.schema().identity().clone();
    let items =
        OwnedItemLinePolicy::new(input.items.clone(), recipe.schema(), Default::default()).unwrap();
    input.item_source.item_lines = *items.identity();
    if let Some(tree) = &mut input.tree {
        *tree = OwnedTreeNormalizationPolicy::bind_new(
            tree.content.clone(),
            recipe.registry(),
            recipe.schema(),
            &mapping,
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone();
    }
}

#[test]
fn explicit_schema_correction_assembles_without_weakening_v3_or_v4() {
    let supplied = input();
    let before = stage(supplied.clone());
    let mut corrected = supplied.clone();
    let gem = corrected.recipe.schema.definitions.iter().find_map(|row| match row {
        DefinitionDescriptor::Gem(entry) if matches!(&entry.schema, SchemaState::Known(schema)
            if schema.declarations.parameters.is_complete() && schema.declarations.parameters.members.is_empty()) => Some(entry.id.clone()),
        _ => None,
    }).unwrap();
    let mut registry = before.assembled().registry().clone();
    let slot = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
        .unwrap();
    corrected.recipe.registry = registry.input().clone();
    let parameter = DefinitionEntry {
        id: slot.clone(),
        schema: SchemaState::Known(ParameterSlotSchema {
            value: ValueSchema::Boolean,
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::GemParameter],
        }),
    };
    corrected
        .recipe
        .schema
        .slots
        .push(SlotDescriptor::Parameter(parameter.clone()));
    let entry = corrected
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(entry) if entry.id == gem => Some(entry),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(schema) = &mut entry.schema else {
        panic!()
    };
    schema.declarations.parameters = DeclaredSet::partial(
        vec![slot],
        vec![SchemaGap {
            subject: SchemaSubject::Definition(gem.address()),
            facet: SchemaFacet::InputSchema,
            code: key("reviewed-intrinsic-schema-correction"),
        }],
    );
    let replacement = entry.clone();
    corrected.recipe.schema.release = key("explicit-schema-correction");
    rebind(&mut corrected);
    let after = stage(corrected.clone());
    assert!(after.input() == &corrected);
    assert_ne!(before.receipt().definitions, after.receipt().definitions);
    assert_eq!(before.query_sets(), after.query_sets());
    assert_eq!(
        before.mapping().input().entries,
        after.mapping().input().entries
    );
    assert_eq!(before.roles().input().roles, after.roles().input().roles);
    assert_eq!(before.assembled().schema().input(), &supplied.recipe.schema);
    assert!(
        matches!(after.assembled().schema().definition(&gem), SchemaLookup::Known(s)
        if s.declarations.parameters.members.len() == 1 && !s.declarations.parameters.is_complete())
    );
    let membership = SchemaMembershipRefinement {
        schema_version: 3,
        before: before.receipt().definitions.clone(),
        after: after.receipt().definitions.clone(),
        subjects: vec![SchemaSubject::Definition(gem.address())],
    };
    let error =
        validate_schema_membership_refinement(&membership, before.assembled(), after.assembled())
            .err()
            .unwrap()
            .to_string();
    assert!(error.contains("Complete set"), "{error}");
    let roles = OwnedSkillRoleIndex::new(
        supplied.roles.clone(),
        before.mapping(),
        before.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    let migration = GemSchemaMigrationInput {
        schema_version: 1,
        before: before.receipt().definitions.clone(),
        mapping: *before.mapping().identity(),
        roles: *roles.identity(),
        source: supplied.mapping.source.clone(),
        gems: vec![replacement],
        parameters: vec![parameter],
    };
    let error = stage_owned_gem_schema(
        before.assembled(),
        before.mapping(),
        &roles,
        &migration,
        Default::default(),
    )
    .err()
    .unwrap()
    .to_string();
    assert!(
        error.contains("cannot rewrite Known descriptors"),
        "{error}"
    );
    let mut stale = corrected;
    stale.tree = supplied.tree;
    assert!(assemble_owned_release(stale, Default::default()).is_err());
}
