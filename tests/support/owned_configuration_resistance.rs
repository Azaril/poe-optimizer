//! Checked addition of raw overrides and native default/contribution programs.
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SchemaState};
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::ConfigurationInputsPolicy,
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/configuration-resistance-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).unwrap()
}
pub fn policy() -> ConfigurationInputsPolicy {
    read("policy.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let authored = authoring();
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(authored["before"], receipt["input"]);
    for field in ["definitions", "registry"] {
        assert_eq!(authored[field], receipt[field]);
    }
    assert_eq!(authored["source_validation"]["status"], "passed");
    let b = prior.input();
    assert!(b.normalization.configuration_inputs.is_none());
    assert!(b.evaluation.is_none());
    let extension: OwnedRecipeExtension = read("extension.json");
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 12);
    assert_eq!(extended.receipt.refined_subjects, 1);
    assert_eq!(extended.receipt.appended_programs, 4);
    assert_eq!(extended.receipt.appended_tables, 0);
    assert_eq!(extended.receipt.appended_receivers, 0);
    // This transition validates every old descriptor, rule and source binding;
    // the explicit refinement permits only the declared Partial-set additions.
    let carried = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extended.successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        extended.refinement.unwrap(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(carried.mapping().input().entries, b.mapping.entries);
    assert_eq!(
        carried.mapping().source_identity(),
        prior.mapping().source_identity()
    );
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.normalization.configuration_inputs = Some(policy());
    full.normalization.version = key("selected-configuration-resistance-inputs-v1");
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            b.tree.as_ref().unwrap().content.clone(),
            carried.assembled().registry(),
            carried.assembled().schema(),
            carried.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-configuration-resistance-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-configuration-resistance-authoring-v1",
            &(
                authored,
                &extension,
                policy(),
                read::<Value>("native-inputs.json"),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.input().query_sets, b.query_sets);
    assert_eq!(next.input().provenance[..b.provenance.len()], b.provenance);
    assert_eq!(next.input().provenance.len(), b.provenance.len() + 1);
    assert_eq!(
        next.input().tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    assert!(next.input().evaluation.is_none());
    preservation(prior, &next, &extension);
    next
}

fn preservation(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    extension: &OwnedRecipeExtension,
) {
    use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
    let b = prior.input();
    let mut r = next.input().clone();
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries
    );
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 12
    );
    assert_eq!(
        r.recipe.registry.last_issued.get(),
        b.recipe.registry.last_issued.get() + 12
    );
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 12
    );
    for row in &extension.schema {
        let SchemaExtensionEntry::Definition(expected) = row else {
            panic!("no slots in this family")
        };
        let at = r
            .recipe
            .schema
            .definitions
            .iter()
            .position(|d| d.address() == expected.address())
            .unwrap();
        assert_eq!(&r.recipe.schema.definitions[at], expected);
        if let Some(old) = b
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == expected.address())
        {
            let (DefinitionDescriptor::Encounter(old), DefinitionDescriptor::Encounter(new)) =
                (old, expected)
            else {
                panic!("only encounter membership may grow")
            };
            let (SchemaState::Known(old_schema), SchemaState::Known(new_schema)) =
                (&old.schema, &new.schema)
            else {
                panic!("known encounter")
            };
            let ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
                encounter,
                inputs,
                ..
            } = policy();
            assert_eq!(new.id, encounter);
            let mut restored = new_schema.clone();
            let additions: Vec<_> = inputs
                .into_iter()
                .flat_map(|row| [row.presence_input, row.value_input])
                .collect();
            assert_eq!(
                restored.external_inputs.members.len(),
                old_schema.external_inputs.members.len() + additions.len()
            );
            for id in additions {
                assert!(!old_schema.external_inputs.members.contains(&id));
                assert_eq!(
                    restored
                        .external_inputs
                        .members
                        .iter()
                        .filter(|v| *v == &id)
                        .count(),
                    1
                );
                restored.external_inputs.members.retain(|v| v != &id);
            }
            assert_eq!(&restored, old_schema);
            r.recipe.schema.definitions[at] = DefinitionDescriptor::Encounter(old.clone());
        } else {
            r.recipe.schema.definitions.remove(at);
        }
    }
    assert_eq!(r.recipe.schema, b.recipe.schema);
    assert_eq!(extension.owners.len(), 1);
    let addition = &extension.owners[0];
    let old = b
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == addition.owner)
        .unwrap();
    let new = r
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == addition.owner)
        .unwrap();
    assert_eq!(new.programs.closure, old.programs.closure);
    assert_eq!(addition.programs.closure, old.programs.closure);
    let mut programs = old.programs.members.clone();
    programs.extend(addition.programs.members.clone());
    assert_eq!(new.programs.members, programs);
    *new = old.clone();
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.definitions = b.items.definitions.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(r.normalization.configuration_inputs, Some(policy()));
    assert_eq!(
        r.normalization.version,
        key("selected-configuration-resistance-inputs-v1")
    );
    r.normalization.configuration_inputs = b.normalization.configuration_inputs.clone();
    r.normalization.version = b.normalization.version.clone();
    let mut normalization = serde_json::to_value(&r.normalization).unwrap();
    let old_normalization = serde_json::to_value(&b.normalization).unwrap();
    for path in [
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/equipment_membership/definitions",
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
        "/item_parameter_inputs/definitions",
        "/item_parameter_inputs/item_lines",
        "/item_parameter_inputs/item_source",
        "/gem_inventory/definitions",
        "/gem_inventory/roles",
        "/gem_inventory/scalar_inputs",
        "/configuration_reward_inventory/reward_policy",
    ] {
        *normalization.pointer_mut(path).unwrap() =
            old_normalization.pointer(path).unwrap().clone();
    }
    r.normalization = serde_json::from_value(normalization).unwrap();
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    let added = r.provenance.pop().unwrap();
    assert_eq!(added.kind, key("explicit-configuration-resistance-inputs"));
    assert_eq!(added.prior_input, prior.receipt().input);
    assert!(
        r == *b,
        "unexpected change outside twelve definitions, eight members, four programs and exact checked bindings"
    );
}
