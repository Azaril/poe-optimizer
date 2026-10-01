//! Explicit raw item-grant supply; saved group settings and effective inputs stay open.
#[path = "owned_spell_damage_modifier.rs"]
pub mod spell;
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::*,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceLayoutPolicy,
    },
    owned_normalize::{ItemModifierMembershipPolicy, ItemParameterInputsPolicy},
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
    owned_recipe_membership_patch::{
        RecipeMembershipPatch, RecipeMembershipPatchBindings, RecipeMembershipPatchInput,
        compile_owned_recipe_membership_patch,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub modifier: ModifierDefId,
    pub level: DeclaredSlot<ParameterSlotDefId>,
    pub grant: DeclaredSlot<GrantSlotDefId>,
    pub supply: DeclaredSlot<SkillGrantSlotDefId>,
    pub skill: SkillDefId,
    pub raw_level: DeclaredSlot<ParameterSlotDefId>,
    pub templates: Vec<ItemTemplateDefId>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/firebolt-item-grant")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn bindings() -> Bindings {
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
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn revision(prior: &StagedOwnedRelease) -> OwnedReleaseRevisionInput {
    let corrected: DefinitionDescriptor = read("skill-input-revision.json");
    let old = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == corrected.address())
        .unwrap();
    let (DefinitionDescriptor::Skill(old), DefinitionDescriptor::Skill(new)) = (old, &corrected)
    else {
        panic!("skill knowledge revision")
    };
    let (SchemaState::Unmapped { gaps }, SchemaState::Known(schema)) = (&old.schema, &new.schema)
    else {
        panic!("unmapped to explicitly partial skill")
    };
    assert_eq!(old.id, new.id);
    assert!(!schema.directly_selectable);
    // Preserve every existing uncertainty independently on each undeclared facet.
    let declarations = serde_json::to_value(&schema.declarations).unwrap();
    for set in declarations.as_object().unwrap().values() {
        assert!(set["members"].as_array().unwrap().is_empty());
        assert_eq!(
            set["closure"],
            serde_json::json!({"kind":"partial","value":{"gaps":gaps}})
        );
    }
    OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key(authoring()["correction_release"].as_str().unwrap()),
        reason: key("explicit-item-firebolt-raw-input-schema"),
        definitions: vec![corrected],
        slots: vec![],
    }
}
fn patch(prior: &StagedOwnedRelease, e: &OwnedRecipeExtension) -> RecipeMembershipPatchInput {
    let b = bindings();
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("item-firebolt-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", e, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.templates,
            add: vec![b.modifier],
        }],
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    assert_eq!(
        serde_json::to_value(prior.receipt().input).unwrap(),
        a["before"]
    );
    assert_eq!(
        serde_json::to_value(&prior.receipt().definitions).unwrap(),
        a["definitions"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().registry).unwrap(),
        a["registry"]
    );
    let numeric = spell::stage(prior);
    let revision = revision(&numeric);
    let corrected =
        compile_owned_release_revision(&numeric, revision.clone(), Default::default()).unwrap();
    let e = extension();
    let membership = patch(&corrected, &e);
    let extended = compile_owned_recipe_membership_patch(
        corrected.assembled(),
        &e,
        &membership,
        Default::default(),
    )
    .unwrap();
    assert_eq!(extended.staged.receipt.allocated_entries, 5);
    let recipe =
        assemble_owned_recipe(extended.staged.successor.clone(), Default::default()).unwrap();
    let b = corrected.input();
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
    full.items.version = key("item-firebolt-grant-lines-v1");
    for rule in item_rules() {
        assert!(!full.items.rules.iter().any(|r| r.id == rule.id));
        full.items.rules.push(rule);
    }
    let items =
        OwnedItemLinePolicy::new(full.items.clone(), recipe.schema(), Default::default()).unwrap();
    full.item_source.item_lines = *items.identity();
    full.item_source.version = key("item-firebolt-grant-source-v1");
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut full.item_source.dialect
    else {
        panic!()
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
    let source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &items,
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
        item_lines,
        item_source,
        ..
    }) = &mut full.normalization.item_modifier_membership
    else {
        panic!()
    };
    *item_lines = *items.identity();
    *item_source = *source.identity();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        item_lines,
        item_source,
        ..
    }) = &mut full.normalization.item_parameter_inputs
    else {
        panic!()
    };
    *item_lines = *items.identity();
    *item_source = *source.identity();
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
        kind: key("explicit-item-firebolt-raw-grant"),
        prior_input: corrected.receipt().input,
        authoring_input: digest_owned(
            "owned-item-firebolt-raw-grant-v1",
            &(
                a,
                revision,
                e,
                membership,
                item_rules(),
                source_conditions(),
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preserve_grant(&numeric, &next);
    next
}
fn preserve_grant(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = prior.input();
    let binding = bindings();
    let e = extension();
    let mut r = next.input().clone();
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 5
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        &b.recipe.registry.entries
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x31ca);
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 1
    );
    assert_eq!(r.recipe.schema.slots.len(), b.recipe.schema.slots.len() + 4);
    for change in e.schema {
        match change {
            SchemaExtensionEntry::Definition(d) => {
                let at = r
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .position(|v| v.address() == d.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.definitions[at], d);
                if let Some(old) = b
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|v| v.address() == d.address())
                {
                    r.recipe.schema.definitions[at] = old.clone();
                } else {
                    r.recipe.schema.definitions.remove(at);
                }
            }
            SchemaExtensionEntry::Slot(s) => {
                let at = r
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .position(|v| v.address() == s.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.slots.remove(at), s);
            }
        }
    }
    for template in binding.templates {
        let DefinitionDescriptor::ItemTemplate(d) = r
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == template.address())
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(s) = &mut d.schema else {
            panic!()
        };
        let at = s
            .modifiers
            .members
            .iter()
            .position(|v| v == &binding.modifier)
            .unwrap();
        s.modifiers.members.remove(at);
    }
    assert_eq!(r.recipe.rules.owners.len(), b.recipe.rules.owners.len() + 1);
    let owner = &e.owners[0];
    let at = r
        .recipe
        .rules
        .owners
        .iter()
        .position(|v| v.owner == owner.owner)
        .unwrap();
    assert_eq!(r.recipe.rules.owners.remove(at), *owner);
    for rule in item_rules().into_iter().rev() {
        assert_eq!(r.items.rules.pop().unwrap(), rule);
        assert_eq!(
            r.item_source.rule_layouts.pop().unwrap(),
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
    for condition in source_conditions().into_iter().rev() {
        assert_eq!(single_modifier_conditions.pop().unwrap(), condition);
    }
    // Only dependent identities and the explicit release label may change.
    let mut value = serde_json::to_value(r).unwrap();
    let original = serde_json::to_value(b).unwrap();
    for path in [
        "/recipe/schema/release",
        "/recipe/rules/definitions",
        "/recipe/routing/definitions",
        "/mapping/definitions",
        "/mapping/registry",
        "/roles/definitions",
        "/roles/mapping",
        "/rewards/definitions",
        "/rewards/mapping",
        "/items/definitions",
        "/items/version",
        "/item_source/item_lines",
        "/item_source/version",
        "/normalization/gem_quality/value/definitions",
        "/normalization/gem_inputs/definitions",
        "/normalization/equipment_membership/definitions",
        "/normalization/item_modifier_membership/definitions",
        "/normalization/item_modifier_membership/item_lines",
        "/normalization/item_modifier_membership/item_source",
        "/normalization/item_parameter_inputs/definitions",
        "/normalization/item_parameter_inputs/item_lines",
        "/normalization/item_parameter_inputs/item_source",
        "/normalization/gem_inventory/definitions",
        "/normalization/gem_inventory/roles",
        "/normalization/gem_inventory/scalar_inputs",
    ] {
        match (value.pointer_mut(path), original.pointer(path)) {
            (Some(a), Some(b)) => *a = b.clone(),
            (None, None) => {}
            _ => panic!("binding shape: {path}"),
        }
    }
    assert_eq!(value["tree"]["content"], original["tree"]["content"]);
    value["tree"] = original["tree"].clone();
    let rows = value["provenance"].as_array_mut().unwrap();
    assert_eq!(rows.len(), b.provenance.len() + 2);
    rows.truncate(b.provenance.len());
    assert!(
        value == original,
        "changes outside the explicit raw-grant/schema migration"
    );
}
