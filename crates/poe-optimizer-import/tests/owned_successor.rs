//! Exact offline succession using shipped inputs; no source checkout or VM.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{FiniteQuantity, OwnedDefinitionKey},
    owned_schema::{DefinitionDescriptor, SchemaState},
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{
    owned_normalize::GemQualityPolicy, owned_skill_catalog::OwnedGemMaterialization,
    owned_successor::*,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/owned/poe2/3887ae68")
}
fn load<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn input() -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: load("import/compiled/recipe.json"),
        successor: load("resistance/recipe.json"),
        mapping: load("import/compiled/mapping.json"),
        roles: load("import/compiled/roles.json"),
        normalization: load("import/policies/normalization.json"),
        rewards: load("import/policies/rewards.json"),
        query_sets: (1..=5)
            .map(|i| NamedQuerySet {
                name: OwnedDefinitionKey::new(format!("original-{i:02}")).unwrap(),
                queries: load(&format!("import/queries/original-{i:02}.json")),
            })
            .collect(),
        items: load("resistance/items.json"),
        item_source: load("resistance/item-source.json"),
    }
}
fn stage(input: SuccessorBundleInput) -> StagedSuccessorBundle {
    transition_owned_bundle(input, SuccessorBundleLimits::default()).unwrap()
}
fn schema_rebind(input: &mut SuccessorBundleInput) {
    let schema = OwnedDefinitionSchemaPackage::new(
        input.successor.schema.clone(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    input.successor.rules.definitions = schema.identity().clone();
    input.successor.routing.definitions = schema.identity().clone();
}
#[test]
fn existing_mechanics_and_all_five_import_inputs_form_one_checked_successor() {
    let source = input();
    let previous = source.clone();
    let staged = stage(source);
    assert_eq!(staged.recipe(), &previous.successor);
    assert_eq!(staged.mapping().input().entries, previous.mapping.entries);
    assert_eq!(staged.mapping().input().source, previous.mapping.source);
    assert_eq!(
        staged.mapping().input().policy_version,
        previous.mapping.policy_version
    );
    assert_eq!(staged.roles().input().roles, previous.roles.roles);
    assert_eq!(
        staged.roles().input().compilation,
        previous.roles.compilation
    );
    assert_eq!(staged.rewards().input().rules, previous.rewards.rules);
    let mut expected_policy = previous.normalization;
    if let GemQualityPolicy::Attributes(quality) = &mut expected_policy.gem_quality {
        quality.definitions = staged.assembled().schema().identity().clone();
    }
    assert_eq!(staged.normalization(), &expected_policy);
    assert_eq!(staged.items().input(), &previous.items);
    assert_eq!(staged.item_source().input(), &previous.item_source);
    assert_eq!(staged.query_sets(), &previous.query_sets);
    assert_eq!(staged.transition().query_rows, 110);
    assert_eq!(
        staged.transition().preserved_registry_entries,
        previous.prior.registry.entries.len()
    );
    assert_eq!(
        staged.transition().preserved_definitions,
        previous.prior.schema.definitions.len()
    );
    assert_eq!(
        staged.transition().preserved_slots,
        previous.prior.schema.slots.len()
    );
    assert_ne!(
        staged.transition().before.rules,
        staged.transition().after.rules
    );
    assert_ne!(
        staged.transition().before.mapping,
        staged.transition().after.mapping
    );
    assert_eq!(staged.transition().calculation, "not_run");
    assert_eq!(staged.transition().whole_build_parity, "not_established");
    let artifacts: BTreeMap<_, _> = staged.artifacts().collect();
    assert_eq!(artifacts.len(), 18);
    for artifact in &staged.transition().artifacts {
        let bytes = artifacts[artifact.file.as_str()];
        assert_eq!(bytes.len(), artifact.bytes);
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), artifact.sha256);
    }
    for queries in staged.query_sets() {
        let bytes = artifacts[format!("queries-{}.json", queries.name.as_str()).as_str()];
        assert_eq!(serde_json::from_slice::<Vec<poe_optimizer_import::owned_normalize::ImportQueryTemplate>>(bytes).unwrap(), queries.queries);
    }
}
#[test]
fn already_bound_endpoint_is_a_valid_no_allocation_transition() {
    let first = stage(input());
    let next = SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: first.recipe().clone(),
        successor: first.recipe().clone(),
        mapping: first.mapping().input().clone(),
        roles: first.roles().input().clone(),
        normalization: first.normalization().clone(),
        rewards: first.rewards().input().clone(),
        query_sets: first.query_sets().to_vec(),
        items: first.items().input().clone(),
        item_source: first.item_source().input().clone(),
    };
    let second = stage(next);
    assert_eq!(second.recipe(), first.recipe());
    assert_eq!(second.transition().before, second.transition().after);
    assert_eq!(second.mapping().identity(), first.mapping().identity());
    assert_eq!(second.roles().identity(), first.roles().identity());
}
#[test]
fn new_rule_values_require_compilation_but_are_not_forced_to_equal_prior_rules() {
    let mut changed = input();
    let value = changed
        .successor
        .rules
        .tables
        .iter_mut()
        .flat_map(|t| &mut t.rows)
        .find_map(|v| match v {
            ParameterValue::Quantity(q) => Some(q),
            _ => None,
        })
        .unwrap();
    *value = FiniteQuantity::new(value.value() + 0.125, value.unit().clone()).unwrap();
    let expected = changed.successor.rules.clone();
    let result = stage(changed);
    assert_eq!(result.assembled().rules().input(), &expected);
    assert_ne!(
        result.transition().before.rules,
        result.transition().after.rules
    );
    let mut invalid = input();
    invalid.successor.rules.operations_version =
        OwnedDefinitionKey::new("unknown-native-operations").unwrap();
    assert!(transition_owned_bundle(invalid, SuccessorBundleLimits::default()).is_err());
}
#[test]
fn stale_prior_bindings_are_rejected_before_any_repair() {
    let base = input();
    let mut mapping = base.clone();
    mapping.mapping.definitions = mapping.successor.rules.definitions.clone();
    let mut roles = base.clone();
    roles.roles.definitions = roles.successor.rules.definitions.clone();
    let mut rewards = base.clone();
    rewards.rewards.definitions = rewards.successor.rules.definitions.clone();
    let mut quality = base;
    if let GemQualityPolicy::Attributes(value) = &mut quality.normalization.gem_quality {
        value.definitions = quality.successor.rules.definitions.clone();
    }
    for bad in [mapping, roles, rewards, quality] {
        assert!(transition_owned_bundle(bad, SuccessorBundleLimits::default()).is_err());
    }
}
#[test]
fn registry_conflicts_and_changed_old_declarations_cannot_be_rebound() {
    let mut conflicting = input();
    conflicting.successor.registry.entries.swap(0, 1);
    conflicting.successor.registry.entries[0].target =
        conflicting.successor.registry.entries[1].target.clone();
    assert!(transition_owned_bundle(conflicting, SuccessorBundleLimits::default()).is_err());
    let mut changed = input();
    let gem = changed
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(entry) => match &mut entry.schema {
                SchemaState::Known(schema) => Some(schema),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    gem.level.maximum =
        poe_optimizer_core::owned_definitions::BoundedInteger::new(gem.level.maximum.get() - 1)
            .unwrap();
    schema_rebind(&mut changed);
    assert!(matches!(
        transition_owned_bundle(changed, SuccessorBundleLimits::default()),
        Err(SuccessorBundleError::ChangedDeclaration)
    ));
}
#[test]
fn contradictory_roles_and_stale_successor_item_policies_fail() {
    let mut role = input();
    let known_gem = role
        .prior
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(entry) if matches!(entry.schema, SchemaState::Known(_)) => {
                Some(entry.id.clone())
            }
            _ => None,
        })
        .unwrap();
    role.roles
        .roles
        .iter_mut()
        .find(|r| r.gem == known_gem)
        .unwrap()
        .materialization = OwnedGemMaterialization::ProviderOnly;
    assert!(transition_owned_bundle(role, SuccessorBundleLimits::default()).is_err());
    let mut items = input();
    items.items.definitions = items.prior.rules.definitions.clone();
    assert!(transition_owned_bundle(items, SuccessorBundleLimits::default()).is_err());
    let mut layout = input();
    layout.item_source.item_lines =
        poe_optimizer_core::owned_content::digest_owned("different-policy", &true, 1024).unwrap();
    assert!(transition_owned_bundle(layout, SuccessorBundleLimits::default()).is_err());
}
#[test]
fn names_duplicate_rows_and_aggregate_limits_reject_without_mutation() {
    let base = input();
    let snapshot = serde_json::to_vec(&base).unwrap();
    for limits in [
        SuccessorBundleLimits {
            max_input_bytes: 64,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_output_bytes: 64,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_query_sets: 1,
            ..Default::default()
        },
        SuccessorBundleLimits {
            max_queries: 30,
            ..Default::default()
        },
    ] {
        assert!(transition_owned_bundle(base.clone(), limits).is_err());
    }
    assert_eq!(serde_json::to_vec(&base).unwrap(), snapshot);
    let mut duplicate = base.clone();
    duplicate.query_sets[1].name = duplicate.query_sets[0].name.clone();
    assert!(matches!(
        transition_owned_bundle(duplicate, Default::default()),
        Err(SuccessorBundleError::QuerySetName)
    ));
    let mut unsafe_name = base.clone();
    unsafe_name.query_sets[0].name = OwnedDefinitionKey::new("parent.child").unwrap();
    assert!(matches!(
        transition_owned_bundle(unsafe_name, Default::default()),
        Err(SuccessorBundleError::QuerySetName)
    ));
    let mut duplicate_row = base;
    let row = duplicate_row.query_sets[0].queries[0].clone();
    duplicate_row.query_sets[0].queries.push(row);
    assert!(transition_owned_bundle(duplicate_row, Default::default()).is_err());
}
#[test]
fn strict_bundle_wire_rejects_unknown_duplicate_and_missing_fields() {
    let raw = serde_json::to_string(&input()).unwrap();
    let duplicate = raw.replacen("{", "{\"schema_version\":1,", 1);
    assert!(decode_successor_bundle(duplicate.as_bytes(), Default::default()).is_err());
    let unknown = raw.replacen("{", "{\"hidden_rebind\":true,", 1);
    assert!(decode_successor_bundle(unknown.as_bytes(), Default::default()).is_err());
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value.as_object_mut().unwrap().remove("item_source");
    assert!(
        decode_successor_bundle(&serde_json::to_vec(&value).unwrap(), Default::default()).is_err()
    );
}

#[test]
fn normalization_mapping_and_combined_successor_query_budgets_are_honored() {
    let base = input();
    let mut limits = SuccessorBundleLimits::default();
    limits.normalization.mapping.max_entries = 1;
    assert!(transition_owned_bundle(base, limits).is_err());

    let mut changed = input();
    changed.query_sets.truncate(1);
    let before_bytes =
        serde_json::to_vec(&(&changed.normalization, &changed.query_sets[0].queries))
            .unwrap()
            .len();
    changed.successor.schema.release =
        OwnedDefinitionKey::new("expanded-release-".to_owned() + &"x".repeat(100)).unwrap();
    schema_rebind(&mut changed);
    let mut rebound = changed.normalization.clone();
    if let GemQualityPolicy::Attributes(quality) = &mut rebound.gem_quality {
        quality.definitions = changed.successor.rules.definitions.clone();
    }
    let after_bytes = serde_json::to_vec(&(&rebound, &changed.query_sets[0].queries))
        .unwrap()
        .len();
    assert!(after_bytes > before_bytes);
    let mut limits = SuccessorBundleLimits::default();
    limits.normalization.max_policy_bytes = before_bytes;
    assert!(
        matches!(transition_owned_bundle(changed, limits), Err(SuccessorBundleError::Digest(poe_optimizer_core::owned_content::ContentDigestError::TooLarge { maximum })) if maximum == before_bytes)
    );
}
#[test]
fn rule_programs_and_table_cells_consume_the_aggregate_entry_budget() {
    let base = input();
    let declarations_only: usize = [&base.prior, &base.successor]
        .iter()
        .map(|r| r.registry.entries.len() + r.schema.definitions.len() + r.schema.slots.len())
        .sum::<usize>()
        + base.mapping.entries.len()
        + base.roles.roles.len()
        + base.rewards.rules.len()
        + base
            .rewards
            .rules
            .iter()
            .map(|r| r.outcomes.len())
            .sum::<usize>()
        + base.items.rules.len()
        + base.item_source.rule_layouts.len()
        + base.item_source.template_layouts.len()
        + base
            .query_sets
            .iter()
            .map(|q| q.queries.len())
            .sum::<usize>();
    assert!(!base.successor.rules.tables.is_empty());
    assert!(matches!(
        transition_owned_bundle(
            base,
            SuccessorBundleLimits {
                max_validation_entries: declarations_only,
                ..Default::default()
            }
        ),
        Err(SuccessorBundleError::Limit("validation entries"))
    ));
}

fn current_input() -> SuccessorBundleInput {
    let recipe = load("current/recipe.json");
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: recipe,
        successor: load("current/recipe.json"),
        mapping: load("current/mapping.json"),
        roles: load("current/roles.json"),
        normalization: load("current/normalization.json"),
        rewards: load("current/rewards.json"),
        query_sets: (1..=5)
            .map(|i| NamedQuerySet {
                name: OwnedDefinitionKey::new(format!("original-{i:02}")).unwrap(),
                queries: load(&format!("current/queries-original-{i:02}.json")),
            })
            .collect(),
        items: load("current/items.json"),
        item_source: load("current/item-source.json"),
    }
}
fn catalog_input() -> (SuccessorBundleInput, CatalogAppend) {
    use poe_optimizer_core::{
        owned_definitions::OptionDefinition,
        owned_schema::{DefinitionAddress, DefinitionEntry, OptionSchema, SchemaSubject},
    };
    use poe_optimizer_import::owned_mapping::*;
    let mut input = current_input();
    let mut registry =
        OwnedIdRegistry::new(input.prior.registry.clone(), OwnedMappingLimits::default()).unwrap();
    let option = registry.allocate_definition::<OptionDefinition>().unwrap();
    input.successor.registry = registry.input().clone();
    input
        .successor
        .schema
        .definitions
        .push(DefinitionDescriptor::Option(DefinitionEntry {
            id: option.clone(),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    schema_rebind(&mut input);
    let append = CatalogAppend {
        mappings: vec![MappingEntry {
            source: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::Option,
                key: SourceComponent::Text("new-independent-catalog-option".into()),
                version: SourceComponent::Text("v1".into()),
                variant: SourceComponent::Missing,
            },
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Option(option)),
                basis: MappingBasis::Exact,
            },
        }],
        source: SourcePin {
            system: input.mapping.source.system,
            revision: input.mapping.source.revision.clone(),
            files: vec![SourceFilePin {
                path: "test/independent-catalog.json".into(),
                sha256: "ab".repeat(32),
            }],
        },
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    (input, append)
}
#[test]
fn catalog_additions_share_checked_finalization_and_preserve_existing_facts() {
    let (input, append) = catalog_input();
    let previous = input.clone();
    let expected_append = append.clone();
    let result = transition_owned_catalog(input, append, Default::default()).unwrap();
    assert_eq!(
        result.recipe().registry.last_issued.get(),
        previous.prior.registry.last_issued.get() + 1
    );
    assert_eq!(
        &result.recipe().registry.entries[..previous.prior.registry.entries.len()],
        &previous.prior.registry.entries
    );
    assert_eq!(
        result.mapping().input().entries.len(),
        previous.mapping.entries.len() + 1
    );
    for entry in &previous.mapping.entries {
        assert!(result.mapping().input().entries.contains(entry));
    }
    assert!(
        result
            .mapping()
            .input()
            .entries
            .contains(&expected_append.mappings[0])
    );
    assert_eq!(
        result.mapping().input().source.files.len(),
        previous.mapping.source.files.len() + 1
    );
    assert_eq!(
        result.roles().input().compilation,
        previous.roles.compilation
    );
    assert_eq!(result.roles().input().roles, previous.roles.roles);
    assert_eq!(result.roles().input().mapping, *result.mapping().identity());
    assert_eq!(result.query_sets(), previous.query_sets);
    assert_eq!(result.rewards().input().rules, previous.rewards.rules);
    assert_eq!(result.items().input().rules, previous.items.rules);
    assert_eq!(
        result.item_source().input().source,
        previous.item_source.source
    );
    let mut expected_source = previous.item_source;
    expected_source.item_lines = *result.items().identity();
    assert_eq!(result.item_source().input(), &expected_source);
    assert_eq!(
        result.transition().item_policy_mode,
        "validated_prior_binding_rebind"
    );
    assert_eq!(result.recipe().rules.owners, previous.prior.rules.owners);
    assert_eq!(result.recipe().rules.tables, previous.prior.rules.tables);
    let raw = result
        .artifacts()
        .find(|(name, _)| *name == "catalog-append.json")
        .unwrap()
        .1;
    assert_eq!(
        serde_json::from_slice::<CatalogAppend>(raw).unwrap(),
        expected_append
    );
    assert!(
        result
            .transition()
            .artifacts
            .iter()
            .any(|a| a.file == "catalog-append.json")
    );
}
#[test]
fn catalog_repeat_accepts_matching_overlaps_but_no_existing_selector_append() {
    let input = current_input();
    let append = CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let before = input.clone();
    let result = transition_owned_catalog(input, append.clone(), Default::default()).unwrap();
    assert_eq!(result.recipe(), &before.prior);
    assert_eq!(result.mapping().input(), &before.mapping);
    assert_eq!(result.roles().input(), &before.roles);
    assert_eq!(result.transition().before, result.transition().after);
    let mut bad = append;
    bad.mappings.push(before.mapping.entries[0].clone());
    assert!(matches!(
        transition_owned_catalog(before, bad, Default::default()),
        Err(SuccessorBundleError::CatalogConflict(
            "existing or duplicate mapping selector"
        ))
    ));
}
#[test]
fn catalog_conflicts_fail_before_publication_and_leave_inputs_unchanged() {
    use poe_optimizer_import::owned_mapping::ExternalSourceSystem;
    let (input, append) = catalog_input();
    let snapshot = serde_json::to_vec(&input).unwrap();
    let mut duplicate_mapping = append.clone();
    let duplicate_entry = duplicate_mapping.mappings[0].clone();
    duplicate_mapping.mappings.push(duplicate_entry);
    let mut duplicate_pin = append.clone();
    let duplicate_file = duplicate_pin.source.files[0].clone();
    duplicate_pin.source.files.push(duplicate_file);
    let mut wrong_hash = append.clone();
    wrong_hash.source.files = vec![input.mapping.source.files[0].clone()];
    wrong_hash.source.files[0].sha256 = "cd".repeat(32);
    let mut revision = append.clone();
    revision.source.revision.push_str("-different");
    let mut system = append;
    system.source.system = ExternalSourceSystem::PathOfBuilding1;
    for bad in [
        duplicate_mapping,
        duplicate_pin,
        wrong_hash,
        revision,
        system,
    ] {
        assert!(matches!(
            transition_owned_catalog(input.clone(), bad, Default::default()),
            Err(SuccessorBundleError::CatalogConflict(_))
        ));
    }
    assert_eq!(serde_json::to_vec(&input).unwrap(), snapshot);
}
#[test]
fn catalog_item_rebind_never_repairs_stale_or_implicitly_supplied_bindings() {
    let (input, append) = catalog_input();
    let mut stale = input.clone();
    stale.items.definitions = stale.successor.rules.definitions.clone();
    assert!(transition_owned_catalog(stale, append.clone(), Default::default()).is_err());
    let mut supplied = append;
    supplied.item_policies = CatalogItemPolicyMode::SuppliedSuccessor;
    assert!(transition_owned_catalog(input, supplied, Default::default()).is_err());
}
#[test]
fn catalog_append_uses_combined_bytes_and_combined_mapping_bounds() {
    let (input, append) = catalog_input();
    let limits = SuccessorBundleLimits {
        max_input_bytes: serde_json::to_vec(&input).unwrap().len(),
        ..Default::default()
    };
    assert!(transition_owned_catalog(input.clone(), append.clone(), limits).is_err());
    let mut limits = SuccessorBundleLimits::default();
    limits.catalog.mapping.max_collection_entries = input.mapping.entries.len();
    assert!(transition_owned_catalog(input, append, limits).is_err());
}
