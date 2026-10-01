//! Authored catalyst input transport, with exact full-release preservation.
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::{ItemTemplateDefId, OwnedDefinitionKey, ParameterSlotDefId},
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemEmission, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceTemplateDefaults},
    owned_normalize::{EquipmentMembershipPolicy, GemQualityPolicy, ItemModifierMembershipPolicy},
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalystBinding {
    pub template: ItemTemplateDefId,
    pub selection: DeclaredSlot<ParameterSlotDefId>,
    pub amount: DeclaredSlot<ParameterSlotDefId>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/ordinary-item-catalyst-inputs")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn bindings() -> Vec<CatalystBinding> {
    read("bindings.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn defaults() -> Vec<ItemSourceTemplateDefaults> {
    read("source-defaults.json")
}
pub fn dependency_definitions() -> Vec<DefinitionDescriptor> {
    read("dependencies.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    let b = prior.input();
    assert_eq!(prior.receipt().input.to_string(), a["before"]);
    assert_eq!(
        serde_json::to_value(&prior.receipt().definitions).unwrap(),
        a["definitions"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().items).unwrap(),
        a["items"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().item_source).unwrap(),
        a["item_source"]
    );
    let e = extension();
    let extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 4);
    // This first checked transition changes only schema memberships and their
    // bindings. Unchanged source interpretation is validated before rebinding.
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
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|v| v.input().clone());
    for (rule_id, selection) in [("catalyst-kind", true), ("catalyst-amount", false)] {
        let rule = full
            .items
            .rules
            .iter_mut()
            .find(|v| v.id.as_str() == rule_id)
            .unwrap();
        let [
            ItemEmission::TemplateParameter {
                bindings: slots, ..
            },
        ] = rule.emissions.as_mut_slice()
        else {
            panic!("reviewed shared header")
        };
        assert_eq!(slots.len(), 338);
        for binding in bindings() {
            let slot = if selection {
                binding.selection
            } else {
                binding.amount
            };
            assert!(!slots.iter().any(|v| v.declaration == slot.declaration));
            slots.push(slot);
        }
    }
    full.items.version = key("ordinary-item-catalyst-inputs-v1");
    let items = OwnedItemLinePolicy::new(
        full.items.clone(),
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    full.item_source.item_lines = *items.identity();
    full.item_source.version = key("ordinary-item-catalyst-inputs-v1");
    for default in defaults() {
        let row = full
            .item_source
            .template_defaults
            .iter_mut()
            .find(|v| v.template == default.template)
            .unwrap();
        assert!(row.parameters.is_empty());
        assert_eq!(row.item_level, default.item_level);
        assert_eq!(row.quality, default.quality);
        row.parameters = default.parameters;
    }
    let source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &items,
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        item_lines,
        item_source,
        ..
    }) = &mut full.normalization.item_modifier_membership
    else {
        panic!("prior singleton proof")
    };
    assert_eq!(
        *definitions,
        carried.assembled().schema().identity().clone()
    );
    assert_eq!(*item_lines, *carried.items().identity());
    assert_eq!(*item_source, *carried.item_source().identity());
    // Explicit authored endpoint bindings, not a supplied-successor repair.
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
        kind: key("explicit-ordinary-item-catalyst-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ordinary-item-catalyst-inputs-v1",
            &(a, &e, bindings(), defaults(), dependency_definitions()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &next);
    next
}

pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = prior.input();
    let mut r = next.input().clone();
    let e = extension();
    for d in dependency_definitions() {
        assert_eq!(
            b.recipe
                .schema
                .definitions
                .iter()
                .find(|v| v.address() == d.address()),
            Some(&d)
        );
    }
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 4
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x311e);
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len()
    );
    assert_eq!(r.recipe.schema.slots.len(), b.recipe.schema.slots.len() + 4);
    for binding in bindings() {
        let d = r
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == binding.template.address())
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(entry) = d else {
            panic!()
        };
        let SchemaState::Known(schema) = &mut entry.schema else {
            panic!()
        };
        assert_eq!(
            schema.declarations.parameters.members,
            [binding.selection.clone(), binding.amount.clone()]
        );
        schema.declarations.parameters.members.clear();
        for slot in [&binding.selection, &binding.amount] {
            let requested = e
                .schema
                .iter()
                .find_map(|v| match v {
                    SchemaExtensionEntry::Slot(s) if s.address().key() == slot.slot.key() => {
                        Some(s)
                    }
                    _ => None,
                })
                .unwrap();
            let i = r
                .recipe
                .schema
                .slots
                .iter()
                .position(|v| v.address() == requested.address())
                .unwrap();
            assert_eq!(r.recipe.schema.slots.remove(i), *requested);
        }
        let requested = e
            .owners
            .iter()
            .find(|v| {
                v.owner
                    == poe_optimizer_core::owned_schema::SchemaSubject::Definition(
                        binding.template.address(),
                    )
            })
            .unwrap();
        let owner = r
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|v| v.owner == requested.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, requested.programs.closure);
        let i = owner
            .programs
            .members
            .iter()
            .position(|v| v.id.as_str() == "catalyst-inputs")
            .unwrap();
        assert_eq!(
            owner.programs.members.remove(i),
            requested.programs.members[0]
        );
        let default = defaults()
            .into_iter()
            .find(|v| v.template == binding.template)
            .unwrap();
        let row = r
            .item_source
            .template_defaults
            .iter_mut()
            .find(|v| v.template == binding.template)
            .unwrap();
        assert_eq!(*row, default);
        row.parameters.clear();
    }
    for (id, selection) in [("catalyst-kind", true), ("catalyst-amount", false)] {
        let rule = r
            .items
            .rules
            .iter_mut()
            .find(|v| v.id.as_str() == id)
            .unwrap();
        let [
            ItemEmission::TemplateParameter {
                bindings: slots, ..
            },
        ] = rule.emissions.as_mut_slice()
        else {
            panic!()
        };
        assert_eq!(slots.len(), 340);
        for b in bindings().into_iter().rev() {
            assert_eq!(
                slots.pop().unwrap(),
                if selection { b.selection } else { b.amount }
            );
        }
    }
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    let (GemQualityPolicy::Attributes(x), GemQualityPolicy::Attributes(y)) = (
        &mut r.normalization.gem_quality,
        &b.normalization.gem_quality,
    ) else {
        panic!()
    };
    x.definitions = y.definitions.clone();
    r.normalization.gem_inputs.as_mut().unwrap().definitions = b
        .normalization
        .gem_inputs
        .as_ref()
        .unwrap()
        .definitions
        .clone();
    let (
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions: x, .. }),
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions: y, .. }),
    ) = (
        &mut r.normalization.equipment_membership,
        &b.normalization.equipment_membership,
    )
    else {
        panic!()
    };
    *x = y.clone();
    let (
        Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions: x,
            item_lines: xi,
            item_source: xs,
            ..
        }),
        Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions: y,
            item_lines: yi,
            item_source: ys,
            ..
        }),
    ) = (
        &mut r.normalization.item_modifier_membership,
        &b.normalization.item_modifier_membership,
    )
    else {
        panic!()
    };
    assert_eq!(*x, next.receipt().definitions);
    assert_eq!(*xi, next.receipt().items);
    assert_eq!(*xs, next.receipt().item_source);
    *x = y.clone();
    *xi = *yi;
    *xs = *ys;
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.version = b.items.version.clone();
    r.items.definitions = b.items.definitions.clone();
    r.item_source.version = b.item_source.version.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    r.provenance.pop();
    assert!(
        r == *b,
        "full endpoint differs outside four slots, two producers, header/default bindings and their exact dependencies"
    );
}
