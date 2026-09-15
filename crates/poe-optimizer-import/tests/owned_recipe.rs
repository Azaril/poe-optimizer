#[path = "support/owned_recipe_fixture.rs"]
mod fixture;
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_rules::*};
use poe_optimizer_data::{
    owned_routing::decode_action_routing, owned_rules::decode_rule_package,
    owned_schema::decode_schema_package,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{owned_mapping::*, owned_recipe::*};
use sha2::{Digest, Sha256};
fn bytes<'a>(p: &'a StagedOwnedRecipe, name: &str) -> &'a [u8] {
    p.artifacts()
        .iter()
        .find(|a| a.name() == name)
        .unwrap()
        .bytes()
}

#[test]
fn deterministic_staging_reloads_each_public_artifact_without_allocating() {
    let input = fixture::recipe(7);
    let limits = OwnedRecipeLimits::default();
    let a = assemble_owned_recipe(input.clone(), limits).unwrap();
    let b = decode_owned_recipe(&serde_json::to_vec_pretty(&input).unwrap(), limits).unwrap();
    assert_eq!(a.registry().input(), &input.registry);
    assert_eq!(a.manifest(), b.manifest());
    assert_eq!(a.manifest().partial_rule_owners, 1);
    assert_eq!(a.manifest().whole_build_parity, "not_established");
    assert_eq!(a.manifest().calculation, "not_run");
    for (left, right) in a.artifacts().iter().zip(b.artifacts()) {
        assert_eq!(left.name(), right.name());
        assert_eq!(left.bytes(), right.bytes());
    }
    for entry in &a.manifest().artifacts {
        let data = bytes(&a, entry.file);
        assert_eq!(data.len(), entry.bytes);
        assert_eq!(format!("{:x}", Sha256::digest(data)), entry.sha256);
    }
    let registry = decode_registry(bytes(&a, "registry.json"), limits.registry).unwrap();
    assert_eq!(registry.input(), &input.registry);
    let schema = decode_schema_package(bytes(&a, "schema.json"), limits.schema).unwrap();
    let rules = decode_rule_package(bytes(&a, "rules.json"), &schema, limits.rules).unwrap();
    let routing =
        decode_action_routing(bytes(&a, "routing.json"), &schema, limits.routing).unwrap();
    assert_eq!(routing.identity(), a.routing().identity());
    let compiled = CompiledRulePackage::compile(rules.input(), &schema, limits.compile).unwrap();
    assert_eq!(compiled.identity(), a.manifest().compiled_rules);
    let owner = &rules.input().owners[0];
    let evaluation = compiled
        .evaluate(
            &owner.owner,
            &owner.programs.members[0].id,
            &[],
            &schema,
            &mut compiled.new_scratch(),
        )
        .unwrap();
    assert!(matches!(&evaluation.effects[0].disposition,
        EffectDisposition::Applied { value: poe_optimizer_core::owned_build::ParameterValue::Integer(v) } if v.get() == 7));
    assert!(!owner.programs.is_complete());
}
#[test]
fn coefficient_edits_retain_ids_but_change_rule_and_recipe_identity() {
    let l = OwnedRecipeLimits::default();
    let a = assemble_owned_recipe(fixture::recipe(7), l).unwrap();
    let b = assemble_owned_recipe(fixture::recipe(8), l).unwrap();
    assert_eq!(a.registry().input(), b.registry().input());
    assert_eq!(a.schema().identity(), b.schema().identity());
    assert_eq!(a.routing().identity(), b.routing().identity());
    assert_ne!(a.rules().identity(), b.rules().identity());
    assert_ne!(a.manifest().recipe, b.manifest().recipe);
}
#[test]
fn foreign_stale_duplicate_and_retired_declarations_are_never_repaired() {
    let l = OwnedRecipeLimits::default();
    let base = fixture::recipe(7);
    let before = serde_json::to_vec(&base).unwrap();
    let mut i = base.clone();
    i.rules.definitions.content_sha256 = "ab".repeat(32);
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Rules(_))
    ));
    let mut i = base.clone();
    i.routing.definitions.content_sha256 = "ab".repeat(32);
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Routing(_))
    ));
    let mut i = base.clone();
    i.registry.namespace = GameVersionNamespace::new("foreign", "v1").unwrap();
    assert!(assemble_owned_recipe(i, l).is_err());
    let mut i = base.clone();
    i.schema.definitions.push(i.schema.definitions[0].clone());
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Schema(_))
    ));
    let mut i = base.clone();
    let mut registry = OwnedIdRegistry::new(i.registry, l.registry).unwrap();
    registry
        .retire(
            &i.rules.owners[0].owner,
            OwnedDefinitionKey::new("retired").unwrap(),
        )
        .unwrap();
    i.registry = registry.input().clone();
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Unregistered(_))
    ));
    let mut i = base.clone();
    i.registry = OwnedIdRegistry::empty(i.schema.namespace.clone(), l.registry)
        .unwrap()
        .input()
        .clone();
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Unregistered(_))
    ));
    assert_eq!(serde_json::to_vec(&base).unwrap(), before);
}
#[test]
fn strict_recipe_wire_and_all_wrapper_resource_ceilings_reject() {
    let l = OwnedRecipeLimits::default();
    let input = fixture::recipe(7);
    let raw = serde_json::to_string(&input).unwrap();
    let duplicate = format!("{{\"schema_version\":1,{}", &raw[1..]);
    assert!(decode_owned_recipe(duplicate.as_bytes(), l).is_err());
    for remove in [true, false] {
        let mut v = serde_json::to_value(&input).unwrap();
        if remove {
            v.as_object_mut().unwrap().remove("registry");
        } else {
            v["extra"] = true.into();
        }
        assert!(decode_owned_recipe(&serde_json::to_vec(&v).unwrap(), l).is_err());
    }
    assert!(
        decode_owned_recipe(
            raw.as_bytes(),
            OwnedRecipeLimits {
                max_wire_bytes: raw.len() - 1,
                ..l
            }
        )
        .is_err()
    );
    for limits in [
        OwnedRecipeLimits {
            max_wire_bytes: 0,
            ..l
        },
        OwnedRecipeLimits {
            max_wire_bytes: l.max_wire_bytes + 1,
            ..l
        },
        OwnedRecipeLimits {
            max_output_bytes: 0,
            ..l
        },
        OwnedRecipeLimits {
            max_output_bytes: l.max_output_bytes + 1,
            ..l
        },
        OwnedRecipeLimits {
            max_registry_checks: 0,
            ..l
        },
        OwnedRecipeLimits {
            max_registry_checks: l.max_registry_checks + 1,
            ..l
        },
        OwnedRecipeLimits {
            max_registry_checks: 1,
            ..l
        },
        OwnedRecipeLimits {
            max_output_bytes: 1,
            ..l
        },
    ] {
        assert!(assemble_owned_recipe(input.clone(), limits).is_err());
    }
    let mut i = input.clone();
    i.schema_version += 1;
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Version(_))
    ));
    // Constituent limits still apply under a generous wrapper limit.
    let mut tight = l;
    tight.compile.max_nodes = 1;
    assert!(matches!(
        assemble_owned_recipe(input.clone(), tight),
        Err(OwnedRecipeError::Compile(_))
    ));
    let mut tight = l;
    tight.registry.max_entries = 1;
    assert!(matches!(
        assemble_owned_recipe(input, tight),
        Err(OwnedRecipeError::Registry(_))
    ));
}
#[test]
fn invalid_table_domain_type_and_operation_are_publication_failures() {
    let l = OwnedRecipeLimits::default();
    let input = fixture::recipe(7);
    let mut i = input.clone();
    i.rules.tables[0].rows.pop();
    assert!(assemble_owned_recipe(i, l).is_err());
    let mut i = input.clone();
    i.rules.tables[0].rows[0] = poe_optimizer_core::owned_build::ParameterValue::Boolean(false);
    assert!(assemble_owned_recipe(i, l).is_err());
    let mut i = input.clone();
    i.rules.owners[0].programs.members[0].nodes[1].expression =
        RuleExpression::LookupIntegerTable {
            table: OwnedDefinitionKey::new("missing").unwrap(),
            key: OwnedDefinitionKey::new("key").unwrap(),
        };
    assert!(assemble_owned_recipe(i, l).is_err());
    let mut i = input;
    i.rules.operations_version = OwnedDefinitionKey::new("unsupported-operations").unwrap();
    assert!(matches!(
        assemble_owned_recipe(i, l),
        Err(OwnedRecipeError::Compile(_))
    ));
}
#[test]
fn append_only_registry_history_and_schema_canonical_order_preserve_existing_symbols() {
    let l = OwnedRecipeLimits::default();
    let base = fixture::recipe(7);
    let original = OwnedIdRegistry::new(base.registry.clone(), l.registry).unwrap();
    let mut next = original.clone();
    let _future: OptionDefId = next.allocate_definition().unwrap();
    original.validate_successor(&next).unwrap();
    let mut input = base.clone();
    input.registry = next.input().clone();
    // Unused future allocations do not invent schema completeness.
    let staged = assemble_owned_recipe(input, l).unwrap();
    assert_eq!(staged.schema().input(), &base.schema);
    assert_eq!(
        &staged.registry().input().entries[..2],
        &base.registry.entries
    );
    let mut reordered = base.clone();
    reordered.schema.definitions.reverse();
    let other = assemble_owned_recipe(reordered, l).unwrap();
    assert_eq!(other.schema().input(), &base.schema);
    assert_eq!(other.schema().identity(), staged.schema().identity());
    assert_eq!(
        other.manifest().recipe,
        digest_owned(
            "owned-recipe-input-v1",
            &{
                let mut i = base;
                i.schema.definitions.reverse();
                i
            },
            l.max_wire_bytes
        )
        .unwrap()
    );
}
