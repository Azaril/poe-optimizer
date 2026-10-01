//! One authored family shared by real publication and isolated native tests.
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_definitions::{
        ItemTemplateDefId, ModifierDefId, OwnedDefinitionKey, ParameterSlotDefId, StatDefId,
        UnitDefId,
    },
};
use poe_optimizer_import::{
    owned_modifier_value_recipe::{ModifierValueBinding, ModifierValuePolicy},
    owned_recipe_extension::OwnedRecipeExtension,
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyBindings {
    pub modifier: ModifierDefId,
    pub amount: DeclaredSlot<ParameterSlotDefId>,
    pub properties: BTreeMap<String, DeclaredSlot<ParameterSlotDefId>>,
    pub corrupted_base: DeclaredSlot<ParameterSlotDefId>,
    pub unit: UnitDefId,
    pub factor_unit: UnitDefId,
    pub effective: StatDefId,
    pub minion_level: StatDefId,
    pub templates: Vec<ItemTemplateDefId>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/global-minion-gem-level")
}
pub fn bindings() -> FamilyBindings {
    serde_json::from_slice(&fs::read(data().join("bindings.json")).unwrap()).unwrap()
}
pub fn extension() -> OwnedRecipeExtension {
    serde_json::from_slice(&fs::read(data().join("extension.json")).unwrap()).unwrap()
}
pub fn dependency_definitions() -> Vec<poe_optimizer_core::owned_schema::DefinitionDescriptor> {
    serde_json::from_slice(&fs::read(data().join("dependencies.json")).unwrap()).unwrap()
}
pub fn numeric_policy(definitions: DataIdentity) -> ModifierValuePolicy {
    let binding: ModifierValueBinding =
        serde_json::from_slice(&fs::read(data().join("numeric-binding.json")).unwrap()).unwrap();
    ModifierValuePolicy {
        schema_version: 1,
        version: OwnedDefinitionKey::new("global-minion-gem-level-numeric-v1").unwrap(),
        definitions,
        factor_unit: bindings().factor_unit,
        bindings: vec![binding],
    }
}

use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceAbsentPolicy,
        ItemSourceConditionalMember, ItemSourceDialect, ItemSourceTemplateDefaults,
    },
    owned_modifier_value_recipe::compile_owned_modifier_values,
    owned_recipe::assemble_owned_recipe,
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
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn item_rule() -> ItemLineRule {
    serde_json::from_slice(&fs::read(data().join("item-rule.json")).unwrap()).unwrap()
}
pub fn source_condition() -> ItemSourceConditionalMember {
    serde_json::from_slice(&fs::read(data().join("source-condition.json")).unwrap()).unwrap()
}
pub fn source_default() -> ItemSourceTemplateDefaults {
    serde_json::from_slice(&fs::read(data().join("source-default.json")).unwrap()).unwrap()
}
pub fn authoring() -> serde_json::Value {
    serde_json::from_slice(&fs::read(data().join("authoring.json")).unwrap()).unwrap()
}
fn patch(prior: &StagedOwnedRelease, e: &OwnedRecipeExtension) -> RecipeMembershipPatchInput {
    let b = bindings();
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("global-minion-gem-level-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", e, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.templates,
            add: vec![b.modifier],
        }],
    }
}
/// Actual production compilers validate allocation, membership and numeric code;
/// the full endpoint restores all prior provenance omitted by compact publication.
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    assert_eq!(
        prior.receipt().input.to_string(),
        "023f6927cdce2b4ee6b26b2f9e43b97c8837a02336d142e85981d35b4b0652bb"
    );
    let mut e = extension();
    let first = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &e,
        &patch(prior, &e),
        Default::default(),
    )
    .unwrap();
    let first_recipe = assemble_owned_recipe(first.staged.successor, Default::default()).unwrap();
    let policy = numeric_policy(first_recipe.schema().identity().clone());
    let generated =
        compile_owned_modifier_values(&first_recipe, &policy, Default::default()).unwrap();
    assert_eq!(generated.extension.owners.len(), 1);
    assert_eq!(
        generated.extension.owners[0].programs.closure,
        e.owners[0].programs.closure
    );
    e.owners[0]
        .programs
        .members
        .extend(generated.extension.owners[0].programs.members.clone());
    let p = patch(prior, &e);
    let compiled =
        compile_owned_recipe_membership_patch(prior.assembled(), &e, &p, Default::default())
            .unwrap();
    let next =
        assemble_owned_recipe(compiled.staged.successor.clone(), Default::default()).unwrap();
    let mut items = prior.input().items.clone();
    items.version = key("global-minion-gem-level-items-v1");
    items.definitions = next.schema().identity().clone();
    items.rules.push(item_rule());
    let checked =
        OwnedItemLinePolicy::new(items.clone(), next.schema(), Default::default()).unwrap();
    let mut source = prior.input().item_source.clone();
    source.version = key("global-minion-gem-level-source-v1");
    source.item_lines = *checked.identity();
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        single_modifier_conditions,
        ..
    } = &mut source.dialect
    else {
        panic!("exact reviewed dialect")
    };
    single_modifier_conditions.push(source_condition());
    source.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("fixed-global-minion-level"),
        role: ItemRuleSourceRole::Unresolved,
    });
    let crown = bindings().templates[0].clone();
    assert_eq!(crown.key().as_str(), "def.0000000000001f1c");
    assert!(!source.template_defaults.iter().any(|v| v.template == crown));
    let default = source_default();
    assert_eq!(default.template, crown);
    assert_eq!(default.item_level, ItemSourceAbsentPolicy::Absent);
    source.template_defaults.push(default);
    let b = prior.input();
    let transition = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: compiled.staged.successor,
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
        compiled.staged.refinement.unwrap(),
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
        kind: key("explicit-global-minion-gem-level"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-global-minion-gem-level-v1",
            &(
                &e,
                &p,
                &policy,
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
