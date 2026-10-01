//! Two authored numerical components; local charm delivery and final outputs stay open.
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::*,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceLayoutPolicy, ItemSourcePropertyBinding, ItemSourceTemplateDefaults,
    },
    owned_modifier_value_recipe::{ModifierValuePolicy, compile_owned_modifier_values},
    owned_normalize::{ItemModifierMembershipPolicy, ItemParameterInputsPolicy},
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
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
pub struct FamilyBindings {
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
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeltBindings {
    pub charm: FamilyBindings,
    pub flask: FamilyBindings,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/fine-belt-modifiers")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn bindings() -> BeltBindings {
    read("bindings.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn item_rules() -> Vec<ItemLineRule> {
    read("item-rules.json")
}
pub fn source_conditions() -> Vec<ItemSourceConditionalMember> {
    read("source-conditions.json")
}
pub fn source_property() -> ItemSourcePropertyBinding {
    read("source-property.json")
}
pub fn source_default() -> ItemSourceTemplateDefaults {
    read("source-default.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn numeric_policy(definitions: DataIdentity) -> ModifierValuePolicy {
    ModifierValuePolicy {
        schema_version: 1,
        version: key("fine-belt-numeric-v1"),
        definitions,
        factor_unit: bindings().charm.factor_unit,
        bindings: read("numeric-bindings.json"),
    }
}
fn patch(prior: &StagedOwnedRelease, e: &OwnedRecipeExtension) -> RecipeMembershipPatchInput {
    let b = bindings();
    assert_eq!(b.charm.templates, b.flask.templates);
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("fine-belt-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", e, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.charm.templates,
            add: vec![b.charm.modifier, b.flask.modifier],
        }],
    }
}
fn compiled_extension(prior: &StagedOwnedRelease) -> (OwnedRecipeExtension, ModifierValuePolicy) {
    let mut e = extension();
    assert_eq!(e.schema.len(), 53);
    assert_eq!(e.owners.len(), 2);
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
    assert_eq!(compiled.extension.owners.len(), 2);
    for extra in compiled.extension.owners {
        let owner = e
            .owners
            .iter_mut()
            .find(|o| o.owner == extra.owner)
            .unwrap();
        assert_eq!(extra.programs.closure, owner.programs.closure);
        assert_eq!(owner.programs.members.len(), 3);
        assert_eq!(extra.programs.members.len(), 1);
        owner.programs.members.extend(extra.programs.members);
    }
    (e, numeric)
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
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
    assert_eq!(receipt["input"], a["before"]);
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
    full.items.version = key("fine-belt-items-v1");
    for rule in item_rules() {
        assert!(!full.items.rules.iter().any(|r| r.id == rule.id));
        full.items.rules.push(rule);
    }
    let checked_items =
        OwnedItemLinePolicy::new(full.items.clone(), recipe.schema(), Default::default()).unwrap();
    full.item_source.item_lines = *checked_items.identity();
    full.item_source.version = key("fine-belt-source-v1");
    let property = source_property();
    assert!(
        !full
            .item_source
            .property_bindings
            .iter()
            .any(|p| p.label == property.label)
    );
    full.item_source.property_bindings.push(property);
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut full.item_source.dialect
    else {
        panic!("reviewed category dialect")
    };
    for condition in source_conditions() {
        assert!(
            !single_modifier_conditions
                .iter()
                .any(|c| c.rule == condition.rule)
        );
        single_modifier_conditions.push(condition);
    }
    for rule in item_rules() {
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
    }
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
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
        item_lines,
        item_source,
        ..
    }) = &mut full.normalization.item_modifier_membership
    else {
        panic!("preserved paired proof")
    };
    *item_lines = *checked_items.identity();
    *item_source = *checked_source.identity();
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
        kind: key("explicit-fine-belt-modifiers"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-fine-belt-modifiers-v1",
            &(
                &e,
                membership,
                numeric,
                item_rules(),
                source_conditions(),
                source_property(),
                source_default(),
                a,
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preserve_with(prior, &next, &e);
    next
}
pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    preserve_with(prior, next, &compiled_extension(prior).0);
}
fn preserve_with(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, e: &OwnedRecipeExtension) {
    let old = prior.input();
    let mut r = next.input().clone();
    assert_eq!(
        r.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 53
    );
    assert_eq!(
        &r.recipe.registry.entries[..old.recipe.registry.entries.len()],
        old.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x31a6);
    r.recipe.registry = old.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        old.recipe.schema.definitions.len() + 4
    );
    assert_eq!(
        r.recipe.schema.slots.len(),
        old.recipe.schema.slots.len() + 49
    );
    for entry in &e.schema {
        match entry {
            SchemaExtensionEntry::Definition(d) => {
                let at = r
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .position(|v| v.address() == d.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.definitions.remove(at), *d);
            }
            SchemaExtensionEntry::Slot(s) => {
                let at = r
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .position(|v| v.address() == s.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.slots.remove(at), *s);
            }
        }
    }
    let b = bindings();
    for template in &b.charm.templates {
        let row = r
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == template.address())
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(row) = row else {
            panic!()
        };
        let SchemaState::Known(schema) = &mut row.schema else {
            panic!()
        };
        for modifier in [&b.charm.modifier, &b.flask.modifier] {
            let at = schema
                .modifiers
                .members
                .iter()
                .position(|v| v == modifier)
                .unwrap();
            assert_eq!(schema.modifiers.members.remove(at), *modifier);
        }
    }
    for owner in &e.owners {
        let at = r
            .recipe
            .rules
            .owners
            .iter()
            .position(|v| v.owner == owner.owner)
            .unwrap();
        assert_eq!(r.recipe.rules.owners.remove(at), *owner);
    }
    r.recipe.rules.definitions = old.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = old.recipe.routing.definitions.clone();
    for rule in item_rules() {
        let at = r.items.rules.iter().position(|v| v.id == rule.id).unwrap();
        assert_eq!(r.items.rules.remove(at), rule);
        let at = r
            .item_source
            .rule_layouts
            .iter()
            .position(|v| v.rule == rule.id)
            .unwrap();
        assert_eq!(
            r.item_source.rule_layouts.remove(at),
            ItemRuleSourceLayout {
                rule: rule.id,
                role: ItemRuleSourceRole::Unresolved
            }
        );
    }
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut r.item_source.dialect
    else {
        panic!()
    };
    for condition in source_conditions() {
        let at = single_modifier_conditions
            .iter()
            .position(|c| c.rule == condition.rule)
            .unwrap();
        assert_eq!(single_modifier_conditions.remove(at), condition);
    }
    let property = source_property();
    let at = r
        .item_source
        .property_bindings
        .iter()
        .position(|v| v.label == property.label)
        .unwrap();
    assert_eq!(r.item_source.property_bindings.remove(at), property);
    let default = source_default();
    let at = r
        .item_source
        .template_defaults
        .iter()
        .position(|v| v.template == default.template)
        .unwrap();
    assert_eq!(r.item_source.template_defaults.remove(at), default);
    r.items.version = old.items.version.clone();
    r.items.definitions = old.items.definitions.clone();
    r.item_source.version = old.item_source.version.clone();
    r.item_source.item_lines = old.item_source.item_lines;
    r.mapping.definitions = old.mapping.definitions.clone();
    r.mapping.registry = old.mapping.registry;
    r.roles.definitions = old.roles.definitions.clone();
    r.roles.mapping = old.roles.mapping;
    r.rewards.definitions = old.rewards.definitions.clone();
    r.rewards.mapping = old.rewards.mapping;
    let before = serde_json::to_value(&old.normalization).unwrap();
    let mut after = serde_json::to_value(&r.normalization).unwrap();
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
        *after.pointer_mut(path).unwrap() = before.pointer(path).unwrap().clone();
    }
    r.normalization = serde_json::from_value(after).unwrap();
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    r.tree = old.tree.clone();
    assert_eq!(r.provenance.len(), old.provenance.len() + 1);
    let added = r.provenance.pop().unwrap();
    assert_eq!(added.kind.as_str(), "explicit-fine-belt-modifiers");
    assert_eq!(added.prior_input, prior.receipt().input);
    assert!(
        r == *old,
        "Fine Belt endpoint changed outside explicit family/default authoring and dependent bindings"
    );
}
