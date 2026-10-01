//! Checked fixed Fire Damage numeric family; receiving damage semantics stay open.
#[path = "owned_family_preservation.rs"]
mod preserve;

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
        ItemSourceLayoutPolicy, ItemSourceTemplateDefaults,
    },
    owned_modifier_value_recipe::{
        ModifierValueBinding, ModifierValuePolicy, compile_owned_modifier_values,
    },
    owned_normalize::{ItemModifierMembershipPolicy, ItemParameterInputsPolicy},
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
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireDamageBindings {
    pub modifier: ModifierDefId,
    pub amount: DeclaredSlot<ParameterSlotDefId>,
    pub properties: BTreeMap<String, DeclaredSlot<ParameterSlotDefId>>,
    pub corrupted_base: DeclaredSlot<ParameterSlotDefId>,
    pub category: DeclaredSlot<ParameterSlotDefId>,
    pub unit: UnitDefId,
    pub factor_unit: UnitDefId,
    pub effective: StatDefId,
    pub templates: Vec<ItemTemplateDefId>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/fire-damage-modifier")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn bindings() -> FireDamageBindings {
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
        version: key("fire-damage-numeric-v1"),
        definitions,
        factor_unit: bindings().factor_unit,
        bindings: vec![binding],
    }
}
fn patch(prior: &StagedOwnedRelease, e: &OwnedRecipeExtension) -> RecipeMembershipPatchInput {
    let b = bindings();
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("fire-damage-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", e, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.templates,
            add: vec![b.modifier],
        }],
    }
}
pub fn compiled_extension(
    prior: &StagedOwnedRelease,
) -> (OwnedRecipeExtension, ModifierValuePolicy) {
    let mut e = extension();
    assert_eq!(e.schema.len(), 25);
    assert_eq!(e.owners.len(), 1);
    assert_eq!(e.owners[0].programs.members.len(), 3);
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
    let extra = &compiled.extension.owners[0];
    assert_eq!(extra.owner, e.owners[0].owner);
    assert_eq!(extra.programs.closure, e.owners[0].programs.closure);
    assert_eq!(extra.programs.members.len(), 1);
    e.owners[0]
        .programs
        .members
        .extend(extra.programs.members.clone());
    (e, numeric)
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    assert_eq!(a["source_validation"]["status"], "passed");
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(receipt["input"], a["before"]);
    for name in [
        "definitions",
        "registry",
        "items",
        "item_source",
        "normalization",
        "tree",
    ] {
        assert_eq!(receipt[name], a[name], "exact predecessor {name}");
    }
    assert!(prior.input().evaluation.is_none());
    let (e, numeric) = compiled_extension(prior);
    let membership = patch(prior, &e);
    let extended = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &e,
        &membership,
        Default::default(),
    )
    .unwrap();
    let recipe =
        assemble_owned_recipe(extended.staged.successor.clone(), Default::default()).unwrap();
    let b = prior.input();
    let carried = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extended.staged.successor,
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
        extended.staged.refinement.unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|v| v.input().clone());
    let rule = item_rule();
    assert!(!full.items.rules.iter().any(|r| r.id == rule.id));
    full.items.version = key("fire-damage-items-v1");
    full.items.rules.push(rule.clone());
    let checked_items =
        OwnedItemLinePolicy::new(full.items.clone(), recipe.schema(), Default::default()).unwrap();
    full.item_source.item_lines = *checked_items.identity();
    full.item_source.version = key("fire-damage-source-v1");
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut full.item_source.dialect
    else {
        panic!("reviewed category dialect")
    };
    let condition = source_condition();
    assert!(
        !single_modifier_conditions
            .iter()
            .any(|c| c.rule == condition.rule)
    );
    single_modifier_conditions.push(condition);
    assert!(
        !full
            .item_source
            .rule_layouts
            .iter()
            .any(|r| r.rule == rule.id)
    );
    full.item_source.rule_layouts.push(ItemRuleSourceLayout {
        rule: rule.id,
        role: ItemRuleSourceRole::Unresolved,
    });
    let default = source_default();
    assert!(
        !full
            .item_source
            .template_defaults
            .iter()
            .any(|d| d.template == default.template)
    );
    full.item_source.template_defaults.push(default);
    let checked_source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &checked_items,
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    match full
        .normalization
        .item_modifier_membership
        .as_mut()
        .unwrap()
    {
        ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            item_lines,
            item_source,
            ..
        }
        | ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
            item_lines,
            item_source,
            ..
        }
        | ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            item_lines,
            item_source,
            ..
        } => {
            *item_lines = *checked_items.identity();
            *item_source = *checked_source.identity();
        }
    }
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        item_lines,
        item_source,
        ..
    }) = &mut full.normalization.item_parameter_inputs
    else {
        panic!("preserved physical proof")
    };
    *item_lines = *checked_items.identity();
    *item_source = *checked_source.identity();
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
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
        kind: key("explicit-fixed-fire-damage"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-fire-damage-modifier-v1",
            &(
                &e,
                membership,
                numeric,
                item_rule(),
                source_condition(),
                source_default(),
                a,
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(
        next.input().recipe.rules.operations_version,
        prior.input().recipe.rules.operations_version
    );
    assert_eq!(
        next.input().recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v14"
    );
    preserve_with(prior, &next, &e);
    next
}
fn preserve_with(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, e: &OwnedRecipeExtension) {
    let b = bindings();
    preserve::check(
        prior,
        next,
        preserve::FamilyChange {
            modifier: &b.modifier,
            templates: &b.templates,
            dependencies: &dependency_definitions(),
            extension: e,
            last_issued: 0x31fc,
            rule: &item_rule(),
            condition: &source_condition(),
            rule_change: preserve::RuleChange::Append,
            default: Some(&source_default()),
        },
    );
}
