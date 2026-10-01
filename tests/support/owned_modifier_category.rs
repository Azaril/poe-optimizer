//! Checked append then explicit one-descriptor inventory correction.
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::*,
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemRollTemplate, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceCategoryBinding, ItemSourceDialect},
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
};
use serde::{Deserialize, de::DeserializeOwned};
use std::{fs, path::PathBuf};
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/category-inputs")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
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
pub fn authoring() -> serde_json::Value {
    read("authoring.json")
}
pub struct Prepared {
    pub appended: StagedOwnedRelease,
    pub revision: OwnedReleaseRevisionInput,
    pub final_release: StagedOwnedRelease,
}
pub fn stage(prior: &StagedOwnedRelease) -> Prepared {
    assert_eq!(
        prior.receipt().input.to_string(),
        authoring()["prior_input"]
    );
    let extension = extension();
    let compiled = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(compiled.receipt.allocated_entries, 4);
    assert_eq!(compiled.receipt.refined_subjects, 1);
    let next = assemble_owned_recipe(compiled.successor.clone(), Default::default()).unwrap();
    let b = prior.input();
    let mut items = b.items.clone();
    items.schema_version = 7;
    items.version = key("modifier-category-items-v1");
    items.definitions = next.schema().identity().clone();
    let rule = items
        .rules
        .iter_mut()
        .find(|v| v.id.as_str() == "fixed-global-minion-level")
        .unwrap();
    let [poe_optimizer_import::owned_item_lines::ItemEmission::Modifier { definition, rolls }] =
        rule.emissions.as_mut_slice()
    else {
        panic!("one exact family emission")
    };
    assert_eq!(*definition, bindings().modifier);
    assert_eq!(rolls.len(), 23);
    rolls.push(roll());
    let checked =
        OwnedItemLinePolicy::new(items.clone(), next.schema(), Default::default()).unwrap();
    let mut source = b.item_source.clone();
    source.schema_version = 8;
    source.version = key("modifier-category-source-v1");
    source.item_lines = *checked.identity();
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings,
        metadata_rules,
        single_modifier_conditions,
        preamble_observations,
    } = source.dialect
    else {
        panic!("exact prior source contract")
    };
    source.dialect = ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        flag_bindings,
        metadata_rules,
        single_modifier_conditions,
        preamble_observations,
        category_bindings: vec![source_binding()],
    };
    let transition = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: compiled.successor,
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
        compiled.refinement.unwrap(),
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
        kind: key("explicit-modifier-category-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-modifier-category-inputs-v1",
            &(extension, roll(), source_binding(), authoring()),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let appended = assemble_owned_release(full, Default::default()).unwrap();
    let id = bindings().modifier;
    let mut descriptor = appended
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|v| v.address() == id.address())
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
    let revision = OwnedReleaseRevisionInput {
        schema_version: 1,
        before: appended.receipt().input,
        release: key("pob-3887ae68-modifier-category-inputs-v1"),
        reason: key("explicit-modifier-category-input-inventory"),
        definitions: vec![descriptor],
        slots: vec![],
    };
    let final_release =
        compile_owned_release_revision(&appended, revision.clone(), Default::default()).unwrap();
    Prepared {
        appended,
        revision,
        final_release,
    }
}
