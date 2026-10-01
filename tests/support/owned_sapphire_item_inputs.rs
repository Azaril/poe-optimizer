//! Checked Sapphire input correction and four-slot append; immutable predecessor retained.
#[path = "owned_cold_category_inputs.rs"]
pub mod cold;
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::{ItemTemplateDefId, OwnedDefinitionKey, ParameterSlotDefId},
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaState},
};
use poe_optimizer_import::{
    owned_item_source::ItemSourceTemplateDefaults,
    owned_normalize::{
        ItemModifierMembershipPolicy, ItemParameterInputsPolicy, OrdinaryImplicitExplicitBase,
        OrdinaryItemParameterInputs,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipDelta {
    pub paired_templates: Vec<OrdinaryImplicitExplicitBase>,
    pub modifier_rules: Vec<OwnedDefinitionKey>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/sapphire-item-inputs")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn bindings() -> Vec<OrdinaryItemParameterInputs> {
    read("bindings.json")
}
pub fn catalyst_bindings() -> Vec<CatalystBinding> {
    read("catalyst-bindings.json")
}
pub fn defaults() -> Vec<ItemSourceTemplateDefaults> {
    read("source-defaults.json")
}
pub fn membership() -> MembershipDelta {
    read("membership.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn correction() -> DefinitionDescriptor {
    read("correction.json")
}

pub fn revision(prior: &StagedOwnedRelease) -> OwnedReleaseRevisionInput {
    let corrected = correction();
    let mut actual = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|v| v.address() == corrected.address())
        .unwrap()
        .clone();
    let (DefinitionDescriptor::ItemTemplate(a), DefinitionDescriptor::ItemTemplate(b)) =
        (&mut actual, &corrected)
    else {
        panic!("ring descriptor")
    };
    let (SchemaState::Known(a), SchemaState::Known(b)) = (&mut a.schema, &b.schema) else {
        panic!("known ring")
    };
    assert_eq!(a.declarations.parameters.members.len(), 2);
    assert_eq!(a.declarations.parameters.closure, SchemaClosure::Complete);
    assert!(matches!(
        b.declarations.parameters.closure,
        SchemaClosure::Partial { .. }
    ));
    a.declarations.parameters.closure = b.declarations.parameters.closure.clone();
    assert_eq!(
        actual, corrected,
        "correction only names the omitted raw input inventory"
    );
    OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key(authoring()["correction_release"].as_str().unwrap()),
        reason: key("explicit-sapphire-input-census-correction"),
        definitions: vec![corrected],
        slots: vec![],
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "items",
        "item_source",
        "normalization",
        "tree",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    let cold = cold::stage(prior);
    cold::preservation(prior, &cold);
    let revision = revision(&cold);
    let corrected =
        compile_owned_release_revision(&cold, revision.clone(), Default::default()).unwrap();
    let e = extension();
    let extended = extend_owned_recipe(corrected.assembled(), &e, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 4);
    assert_eq!(extended.receipt.refined_subjects, 1);
    assert_eq!(extended.receipt.appended_programs, 0);
    let b = corrected.input();
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
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
        paired_templates,
        modifier_rules,
        ..
    }) = &mut full.normalization.item_modifier_membership
    else {
        panic!("existing paired profile")
    };
    for row in membership().paired_templates {
        assert!(!paired_templates.iter().any(|v| v.template == row.template));
        paired_templates.push(row);
    }
    for rule in membership().modifier_rules {
        assert!(!modifier_rules.contains(&rule));
        modifier_rules.push(rule);
    }
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut full.normalization.item_parameter_inputs
    else {
        panic!("existing physical profile")
    };
    for row in bindings() {
        assert!(!templates.iter().any(|v| v.template == row.template));
        templates.push(row);
    }
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
        kind: key("explicit-sapphire-item-inputs"),
        prior_input: corrected.receipt().input,
        authoring_input: digest_owned(
            "owned-sapphire-item-inputs-v1",
            &(
                a,
                &e,
                revision,
                bindings(),
                catalyst_bindings(),
                defaults(),
                membership(),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preservation(&cold, &next);
    next
}

/// Restore the exact checked cold prerequisite. That prerequisite independently
/// restores the complete Solar predecessor after only its declared cold changes.
pub fn preservation(cold: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = cold.input();
    let mut r = next.input().clone();
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 4
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x3171);
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len()
    );
    assert_eq!(r.recipe.schema.slots.len(), b.recipe.schema.slots.len() + 4);
    for requested in extension().schema {
        match requested {
            SchemaExtensionEntry::Definition(d) => {
                let row = r
                    .recipe
                    .schema
                    .definitions
                    .iter_mut()
                    .find(|v| v.address() == d.address())
                    .unwrap();
                assert_eq!(*row, d);
                *row = b
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|v| v.address() == d.address())
                    .unwrap()
                    .clone();
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
    r.recipe.schema.release = b.recipe.schema.release.clone();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
        paired_templates,
        modifier_rules,
        ..
    }) = &mut r.normalization.item_modifier_membership
    else {
        panic!()
    };
    for row in membership().paired_templates {
        let at = paired_templates
            .iter()
            .position(|v| v.template == row.template)
            .unwrap();
        assert_eq!(paired_templates.remove(at), row);
    }
    for rule in membership().modifier_rules {
        let at = modifier_rules.iter().position(|v| v == &rule).unwrap();
        assert_eq!(modifier_rules.remove(at), rule);
    }
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut r.normalization.item_parameter_inputs
    else {
        panic!()
    };
    for row in bindings() {
        let at = templates
            .iter()
            .position(|v| v.template == row.template)
            .unwrap();
        assert_eq!(templates.remove(at), row);
    }
    for default in defaults() {
        assert_eq!(
            r.item_source
                .template_defaults
                .iter()
                .find(|v| v.template == default.template),
            Some(&default)
        );
    }
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
    let mut n = serde_json::to_value(&r.normalization).unwrap();
    let old = serde_json::to_value(&b.normalization).unwrap();
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
        *n.pointer_mut(path).unwrap() = old.pointer(path).unwrap().clone();
    }
    r.normalization = serde_json::from_value(n).unwrap();
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 2);
    assert_eq!(
        r.provenance.pop().unwrap().kind.as_str(),
        "explicit-sapphire-item-inputs"
    );
    assert_eq!(
        r.provenance.pop().unwrap().kind.as_str(),
        "explicit-sapphire-input-census-correction"
    );
    assert!(
        r == *b,
        "exact cold prerequisite restored; all pre-existing owners, source facts, queries and allocation history preserved"
    );
}
