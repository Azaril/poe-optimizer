//! Explicit cold input-category correction. Numerical owners remain unchanged.
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::*,
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemEmission, ItemLineRule, ItemRollTemplate, OwnedItemLinePolicy},
    owned_item_source::{
        ItemSourceCategoryBinding, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceLayoutPolicy,
    },
    owned_mapping::OwnedMappingIndex,
    owned_normalize::{GemInventoryPolicy, gem_inventory_scalar_inputs_identity},
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_skill_catalog::OwnedSkillRoleIndex,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub modifier: ModifierDefId,
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub explicit: OptionDefId,
    pub implicit: OptionDefId,
    pub enchant: OptionDefId,
    pub input: OwnedDefinitionKey,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/cold-category-inputs")
}
fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(file)).unwrap()).unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
pub fn bindings() -> Bindings {
    read("bindings.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn roll() -> ItemRollTemplate {
    read("roll.json")
}
pub fn source_binding() -> ItemSourceCategoryBinding {
    read("source-binding.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn prior_rules() -> Vec<ItemLineRule> {
    read("prior-rules.json")
}
pub fn source_conditions() -> Vec<ItemSourceConditionalMember> {
    read("source-conditions.json")
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    assert_eq!(
        prior.receipt().input.to_string(),
        authoring()["prior_input"]
    );
    assert!(prior.input().evaluation.is_none());
    let b = bindings();
    let e = extension();
    let appended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert_eq!(appended.receipt.allocated_entries, 1);
    assert_eq!(appended.receipt.refined_subjects, 1);
    assert_eq!(appended.receipt.appended_programs, 0);
    let recipe = assemble_owned_recipe(appended.successor.clone(), Default::default()).unwrap();
    let mut full = prior.input().clone();
    full.recipe = appended.successor;
    full.mapping.registry = recipe.registry().identity().unwrap();
    full.mapping.definitions = recipe.schema().identity().clone();
    let mapping = OwnedMappingIndex::new(
        full.mapping.clone(),
        recipe.registry(),
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    full.roles.mapping = *mapping.identity();
    full.roles.definitions = recipe.schema().identity().clone();
    let roles = OwnedSkillRoleIndex::new(
        full.roles.clone(),
        &mapping,
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    full.rewards.mapping = *mapping.identity();
    full.rewards.definitions = recipe.schema().identity().clone();

    full.items.definitions = recipe.schema().identity().clone();
    full.items.version = key("cold-category-inputs-v1");
    let expected = prior_rules();
    assert_eq!(expected.len(), 4);
    let actual: Vec<_> = full
        .items
        .rules
        .iter()
        .filter(|rule| {
            rule.emissions.iter().any(|e|
        matches!(e, ItemEmission::Modifier { definition, .. } if definition == &b.modifier))
        })
        .cloned()
        .collect();
    assert_eq!(actual, expected);
    for old in &expected {
        let rule = full
            .items
            .rules
            .iter_mut()
            .find(|r| r.id == old.id)
            .unwrap();
        assert_eq!(rule, old);
        let [ItemEmission::Modifier { definition, rolls }] = rule.emissions.as_mut_slice() else {
            panic!()
        };
        assert_eq!(*definition, b.modifier);
        assert_eq!(rolls.len(), 23);
        assert!(!rolls.iter().any(|r| r.slot == b.slot));
        rolls.push(roll());
    }
    let items =
        OwnedItemLinePolicy::new(full.items.clone(), recipe.schema(), Default::default()).unwrap();
    full.item_source.item_lines = *items.identity();
    full.item_source.version = key("cold-category-inputs-v1");
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        category_bindings,
        single_modifier_conditions,
        ..
    } = &full.item_source.dialect
    else {
        panic!()
    };
    assert!(category_bindings.contains(&source_binding()));
    let conditions: Vec<_> = single_modifier_conditions
        .iter()
        .filter(|c| expected.iter().any(|r| c.rule == r.id))
        .cloned()
        .collect();
    assert_eq!(conditions, source_conditions());
    let source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &items,
        recipe.schema(),
        Default::default(),
    )
    .unwrap();

    // Simultaneous required-roll + recipe authoring uses the checked full endpoint.
    // Carry-only publication correctly cannot compile the old 23-roll recipe
    // against the new RequiredOnce slot. No temporary optional slot is invented.
    let mut normalization = serde_json::to_value(&full.normalization).unwrap();
    let identity = serde_json::to_value(recipe.schema().identity()).unwrap();
    for path in [
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/equipment_membership/definitions",
        "/item_modifier_membership/definitions",
        "/item_parameter_inputs/definitions",
        "/gem_inventory/definitions",
    ] {
        assert_eq!(
            normalization.pointer(path).unwrap(),
            &serde_json::to_value(prior.assembled().schema().identity()).unwrap()
        );
        *normalization.pointer_mut(path).unwrap() = identity.clone();
    }
    for policy in ["item_modifier_membership", "item_parameter_inputs"] {
        let row = normalization.get_mut(policy).unwrap();
        assert_eq!(row["item_lines"], json!(prior.items().identity()));
        assert_eq!(row["item_source"], json!(prior.item_source().identity()));
        row["item_lines"] = json!(items.identity());
        row["item_source"] = json!(source.identity());
    }
    full.normalization = serde_json::from_value(normalization).unwrap();
    let scalar_inputs =
        gem_inventory_scalar_inputs_identity(&full.normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
        roles: bound_roles,
        scalar_inputs: bound_scalars,
        ..
    }) = &mut full.normalization.gem_inventory
    else {
        panic!()
    };
    *bound_roles = *roles.identity();
    *bound_scalars = scalar_inputs;
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            recipe.registry(),
            recipe.schema(),
            &mapping,
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-cold-category-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-cold-category-inputs-v1",
            &(
                authoring(),
                e,
                roll(),
                source_binding(),
                expected,
                conditions,
            ),
            1024 * 1024,
        )
        .unwrap(),
    });
    let appended = assemble_owned_release(full, Default::default()).unwrap();
    let mut descriptor = appended
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|v| v.address() == b.modifier.address())
        .unwrap()
        .clone();
    let DefinitionDescriptor::Modifier(row) = &mut descriptor else {
        panic!()
    };
    let SchemaState::Known(s) = &mut row.schema else {
        panic!()
    };
    assert_eq!(s.declarations.parameters.members.len(), 24);
    assert!(!s.declarations.parameters.is_complete());
    s.declarations.parameters.closure = SchemaClosure::Complete;
    let final_release = compile_owned_release_revision(
        &appended,
        OwnedReleaseRevisionInput {
            schema_version: 1,
            before: appended.receipt().input,
            release: key("pob-3887ae68-cold-category-inputs-v1"),
            reason: key("explicit-cold-category-input-inventory"),
            definitions: vec![descriptor],
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    preservation(prior, &final_release);
    final_release
}

/// Restore the exact authorized deltas and compare the entire predecessor.
pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let before = prior.input();
    let mut restored = next.input().clone();
    assert_eq!(
        restored.recipe.registry.entries.len(),
        before.recipe.registry.entries.len() + 1
    );
    assert_eq!(
        &restored.recipe.registry.entries[..before.recipe.registry.entries.len()],
        before.recipe.registry.entries.as_slice()
    );
    assert_eq!(restored.recipe.registry.last_issued.get(), 0x316d);
    restored.recipe.registry = before.recipe.registry.clone();
    assert_eq!(
        restored.recipe.schema.slots.len(),
        before.recipe.schema.slots.len() + 1
    );
    assert_eq!(
        restored.recipe.schema.definitions.len(),
        before.recipe.schema.definitions.len()
    );
    for row in extension().schema {
        match row {
            SchemaExtensionEntry::Definition(mut expected) => {
                let DefinitionDescriptor::Modifier(entry) = &mut expected else {
                    panic!()
                };
                let SchemaState::Known(s) = &mut entry.schema else {
                    panic!()
                };
                s.declarations.parameters.closure = SchemaClosure::Complete;
                let position = restored
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .position(|d| d.address() == expected.address())
                    .unwrap();
                assert_eq!(restored.recipe.schema.definitions[position], expected);
                restored.recipe.schema.definitions[position] = before
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == expected.address())
                    .unwrap()
                    .clone();
            }
            SchemaExtensionEntry::Slot(expected) => {
                let position = restored
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .position(|s| s.address() == expected.address())
                    .unwrap();
                assert_eq!(restored.recipe.schema.slots.remove(position), expected);
            }
        }
    }
    for expected in prior_rules() {
        let actual = restored
            .items
            .rules
            .iter_mut()
            .find(|r| r.id == expected.id)
            .unwrap();
        let [ItemEmission::Modifier { rolls, .. }] = actual.emissions.as_mut_slice() else {
            panic!()
        };
        assert_eq!(rolls.len(), 24);
        assert_eq!(rolls.pop(), Some(roll()));
        assert_eq!(*actual, expected);
    }
    restored.recipe.schema.release = before.recipe.schema.release.clone();
    restored.recipe.rules.definitions = before.recipe.rules.definitions.clone();
    restored.recipe.routing.definitions = before.recipe.routing.definitions.clone();
    restored.mapping.definitions = before.mapping.definitions.clone();
    restored.mapping.registry = before.mapping.registry;
    restored.roles.definitions = before.roles.definitions.clone();
    restored.roles.mapping = before.roles.mapping;
    restored.rewards.definitions = before.rewards.definitions.clone();
    restored.rewards.mapping = before.rewards.mapping;
    restored.items.definitions = before.items.definitions.clone();
    restored.items.version = before.items.version.clone();
    restored.item_source.item_lines = before.item_source.item_lines;
    restored.item_source.version = before.item_source.version.clone();
    let mut normalized = serde_json::to_value(&restored.normalization).unwrap();
    let old = serde_json::to_value(&before.normalization).unwrap();
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
    ] {
        *normalized.pointer_mut(path).unwrap() = old.pointer(path).unwrap().clone();
    }
    restored.normalization = serde_json::from_value(normalized).unwrap();
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        before.tree.as_ref().unwrap().content
    );
    restored.tree = before.tree.clone();
    assert_eq!(restored.provenance.len(), before.provenance.len() + 2);
    assert_eq!(
        &restored.provenance[..before.provenance.len()],
        before.provenance.as_slice()
    );
    assert_eq!(
        restored.provenance[before.provenance.len()].kind.as_str(),
        "explicit-cold-category-inputs"
    );
    assert_eq!(
        restored.provenance.last().unwrap().kind.as_str(),
        "explicit-cold-category-input-inventory"
    );
    restored.provenance.truncate(before.provenance.len());
    assert!(
        restored == *before,
        "cold stage changed data outside the exact category slot, four recipe rolls, declaration closure and explicit bindings/provenance"
    );
}
