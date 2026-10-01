//! Actual authored movement family reused by publication and finite native tests.
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::{
        ItemTemplateDefId, ModifierDefId, OwnedDefinitionKey, ParameterSlotDefId, StatDefId,
        UnitDefId,
    },
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceTemplateDefaults,
    },
    owned_modifier_value_recipe::{
        ModifierValueBinding, ModifierValuePolicy, compile_owned_modifier_values,
    },
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::OwnedRecipeExtension,
    owned_recipe_membership_patch::{
        RecipeMembershipPatch, RecipeMembershipPatchBindings, RecipeMembershipPatchInput,
        compile_owned_recipe_membership_patch,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementBindings {
    pub modifier: ModifierDefId,
    pub amount: DeclaredSlot<ParameterSlotDefId>,
    pub properties: BTreeMap<String, DeclaredSlot<ParameterSlotDefId>>,
    pub corrupted_base: DeclaredSlot<ParameterSlotDefId>,
    pub category: DeclaredSlot<ParameterSlotDefId>,
    pub unit: UnitDefId,
    pub factor_unit: UnitDefId,
    pub effective: StatDefId,
    pub contribution: StatDefId,
    pub templates: Vec<ItemTemplateDefId>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/movement-speed")
}
fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(file)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn bindings() -> MovementBindings {
    read("bindings.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn dependency_definitions() -> Vec<DefinitionDescriptor> {
    read("dependencies.json")
}
pub fn item_rule() -> ItemLineRule {
    read("item-rule.json")
}
pub fn source_condition() -> ItemSourceConditionalMember {
    read("source-condition.json")
}
pub fn source_default() -> ItemSourceTemplateDefaults {
    read("source-default.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn numeric_policy(definitions: DataIdentity) -> ModifierValuePolicy {
    let binding: ModifierValueBinding = read("numeric-binding.json");
    ModifierValuePolicy {
        schema_version: 1,
        version: key("movement-speed-numeric-v1"),
        definitions,
        factor_unit: bindings().factor_unit,
        bindings: vec![binding],
    }
}
fn patch(prior: &StagedOwnedRelease, e: &OwnedRecipeExtension) -> RecipeMembershipPatchInput {
    let b = bindings();
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("movement-speed-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", e, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.templates,
            add: vec![b.modifier],
        }],
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    assert_eq!(prior.receipt().input.to_string(), authoring()["before"]);
    let mut e = extension();
    let first = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &e,
        &patch(prior, &e),
        Default::default(),
    )
    .unwrap();
    let first = assemble_owned_recipe(first.staged.successor, Default::default()).unwrap();
    let numeric = numeric_policy(first.schema().identity().clone());
    let compiled = compile_owned_modifier_values(&first, &numeric, Default::default()).unwrap();
    assert_eq!(compiled.extension.owners.len(), 1);
    assert_eq!(
        compiled.extension.owners[0].programs.closure,
        e.owners[0].programs.closure
    );
    e.owners[0]
        .programs
        .members
        .extend(compiled.extension.owners[0].programs.members.clone());
    let membership = patch(prior, &e);
    let extension = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &e,
        &membership,
        Default::default(),
    )
    .unwrap();
    let recipe =
        assemble_owned_recipe(extension.staged.successor.clone(), Default::default()).unwrap();
    let b = prior.input();
    let mut items = b.items.clone();
    items.definitions = recipe.schema().identity().clone();
    items.version = key("movement-speed-items-v1");
    items.rules.push(item_rule());
    let checked =
        OwnedItemLinePolicy::new(items.clone(), recipe.schema(), Default::default()).unwrap();
    let mut source = b.item_source.clone();
    source.version = key("movement-speed-source-v1");
    source.item_lines = *checked.identity();
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut source.dialect
    else {
        panic!("exact category-aware source dialect")
    };
    single_modifier_conditions.push(source_condition());
    source.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("fixed-increased-movement-speed"),
        role: ItemRuleSourceRole::Unresolved,
    });
    let default = source_default();
    assert!(
        !source
            .template_defaults
            .iter()
            .any(|v| v.template == default.template)
    );
    source.template_defaults.push(default);
    let transition = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extension.staged.successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items,
            item_source: source,
        },
        CatalogAppend {
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::SuppliedSuccessor,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        extension.staged.refinement.unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.recipe = transition.recipe().clone();
    full.mapping = transition.mapping().input().clone();
    full.roles = transition.roles().input().clone();
    full.normalization = transition.normalization().clone();
    full.rewards = transition.rewards().input().clone();
    full.items = transition.items().input().clone();
    full.item_source = transition.item_source().input().clone();
    full.tree = transition.tree().map(|v| v.input().clone());
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-increased-movement-speed"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-movement-speed-authoring-v1",
            &(
                e,
                membership,
                numeric,
                item_rule(),
                source_condition(),
                source_default(),
                authoring(),
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    assemble_owned_release(full, Default::default()).unwrap()
}
