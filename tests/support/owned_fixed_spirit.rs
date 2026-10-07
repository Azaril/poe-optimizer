//! One additional source grammar for the existing Spirit modifier. Structural
//! item membership is not affix legality, and no numerical owner is completed.
#[path = "owned_release_migration_preservation.rs"]
mod migration_preservation;

use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{ItemTemplateDefId, ModifierDefId, OwnedDefinitionKey, ParameterSlotDefId},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceLayoutPolicy,
    },
    owned_normalize::{NormalizationPolicy, equipment_membership_identity},
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
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "fixed-spirit-input";
const PAYLOADS: [&str; 6] = [
    "bindings.json",
    "dependencies.json",
    "extension.json",
    "item-rule.json",
    "source-condition.json",
    "source-vectors.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/fixed-spirit")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
#[derive(Deserialize)]
pub struct Bindings {
    pub modifier: ModifierDefId,
    pub amount: DeclaredSlot<ParameterSlotDefId>,
    pub existing_templates: Vec<ItemTemplateDefId>,
    pub added_templates: Vec<ItemTemplateDefId>,
}
#[derive(Deserialize)]
struct Dependencies {
    modifier: DefinitionDescriptor,
    slots: Vec<SlotDescriptor>,
    owner: DefinitionRules,
    ranged_rule: ItemLineRule,
    ranged_condition: ItemSourceConditionalMember,
}
pub fn item_rule() -> ItemLineRule {
    read("item-rule.json")
}
pub fn source_condition() -> ItemSourceConditionalMember {
    read("source-condition.json")
}
fn digest() -> OwnedContentDigest {
    let mut values: Vec<Value> = PAYLOADS.into_iter().map(read).collect();
    values.push(read("authoring.json"));
    digest_owned(KIND, &values, 2 * 1024 * 1024).unwrap()
}
fn conditions(source: &ItemSourceDialect) -> &[ItemSourceConditionalMember] {
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = source
    else {
        panic!("checked category source dialect")
    };
    single_modifier_conditions
}
fn conditions_mut(source: &mut ItemSourceDialect) -> &mut Vec<ItemSourceConditionalMember> {
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = source
    else {
        panic!("checked category source dialect")
    };
    single_modifier_conditions
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Bindings = read("bindings.json");
    let raw_bindings: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    assert_eq!(a["scope"], raw_bindings["scope"]);
    assert_eq!(b.modifier.key().as_str(), "def.000000000000314d");
    assert_eq!(b.amount.slot.key().as_str(), "def.000000000000314e");
    assert_eq!(b.existing_templates.len(), 1);
    assert_eq!(b.added_templates.len(), 1755);
    assert_eq!(
        b.added_templates.iter().collect::<BTreeSet<_>>().len(),
        1755
    );
    assert!(b.added_templates.windows(2).all(|w| w[0] < w[1]));
    assert!(!b.added_templates.contains(&b.existing_templates[0]));
    assert_eq!(d.slots.len(), 24);
    assert_eq!(d.owner.programs.members.len(), 5);
    assert!(!d.owner.programs.is_complete());
    let e: OwnedRecipeExtension = read("extension.json");
    assert!(
        e.schema.is_empty() && e.owners.is_empty() && e.tables.is_empty() && e.receivers.is_empty()
    );
    assert!(e.operations_version.is_none());
    let rule: Value = read("item-rule.json");
    assert_eq!(rule["id"], "fixed-spirit");
    assert_eq!(
        rule["pattern"],
        json!([
            {"kind":"literal","value":"+"},
            {"kind":"numeric_capture","value":{"capture":"amount","syntax":"integer","sign":"forbidden"}},
            {"kind":"literal","value":" to Spirit"}
        ])
    );
    assert_eq!(rule["emissions"].as_array().unwrap().len(), 1);
    assert_eq!(
        rule["emissions"][0]["value"]["definition"],
        json!(b.modifier)
    );
    let rolls = rule["emissions"][0]["value"]["rolls"].as_array().unwrap();
    assert_eq!(rolls.len(), 24);
    assert_eq!(rolls[0]["slot"], json!(b.amount));
    assert_eq!(
        rolls[0]["value"],
        json!({"kind":"numeric_projection","value":{
            "source":{"kind":"capture","value":"amount"},"negate":false,
            "decimal":{"kind":"exact"},"result":{"kind":"signed_quantity"}
        }})
    );
    let ranged = json!(d.ranged_rule);
    assert_eq!(
        &rolls[1..],
        &ranged["emissions"][0]["value"]["rolls"].as_array().unwrap()[1..]
    );
    let mut expected_capture = ranged["captures"][0].clone();
    expected_capture["id"] = json!("amount");
    assert_eq!(rule["captures"], json!([expected_capture]));
    assert_eq!(
        read::<Value>("source-condition.json"),
        json!({"rule":"fixed-spirit","all":[
            {"kind":"no_source_tags"},{"kind":"no_generated_buff_members"},
            {"kind":"unsigned_integer_capture","value":{"capture":"amount","min":1,"max":1000000}}
        ]})
    );
    assert_eq!(a["artifacts"].as_object().unwrap().len(), PAYLOADS.len());
    for name in PAYLOADS {
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(bytes.ends_with(b"\n") && !bytes.contains(&b'\r'));
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(&bytes))
        );
    }
    super::validation::verify_source(false);
}
fn dependencies(endpoint: &StagedOwnedRelease) {
    let d: Dependencies = read("dependencies.json");
    let input = endpoint.input();
    assert_eq!(
        input
            .recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == d.modifier.address()),
        Some(&d.modifier)
    );
    for slot in &d.slots {
        assert_eq!(
            input
                .recipe
                .schema
                .slots
                .iter()
                .find(|s| s.address() == slot.address()),
            Some(slot)
        );
    }
    assert_eq!(
        input
            .recipe
            .rules
            .owners
            .iter()
            .find(|r| r.owner == d.owner.owner),
        Some(&d.owner)
    );
    assert_eq!(
        input.items.rules.iter().find(|r| r.id == d.ranged_rule.id),
        Some(&d.ranged_rule)
    );
    assert_eq!(
        conditions(&input.item_source.dialect)
            .iter()
            .find(|r| r.rule == d.ranged_condition.rule),
        Some(&d.ranged_condition)
    );
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let a: Value = read("authoring.json");
    let b: Bindings = read("bindings.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.registry.last_issued.get(), 0x3333);
    assert_eq!(endpoint.input().items.rules.last(), Some(&item_rule()));
    assert_eq!(
        conditions(&endpoint.input().item_source.dialect).last(),
        Some(&source_condition())
    );
    for id in b.existing_templates.iter().chain(&b.added_templates) {
        let DefinitionDescriptor::ItemTemplate(row) = endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == id.address())
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(schema) = &row.schema else {
            panic!()
        };
        assert!(!schema.modifiers.is_complete());
        assert_eq!(
            schema
                .modifiers
                .members
                .iter()
                .filter(|m| **m == b.modifier)
                .count(),
            1
        );
    }
}
fn patch(
    prior: &StagedOwnedRelease,
    extension: &OwnedRecipeExtension,
) -> RecipeMembershipPatchInput {
    let b: Bindings = read("bindings.json");
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("fixed-spirit-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", extension, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.added_templates,
            add: vec![b.modifier],
        }],
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    super::validation::verify_source(true);
    dependencies(prior);
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for (name, value) in a["prior"].as_object().unwrap() {
        assert_eq!(&receipt[name], value);
    }
    let extension: OwnedRecipeExtension = read("extension.json");
    let expanded = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &extension,
        &patch(prior, &extension),
        Default::default(),
    )
    .unwrap();
    assert_eq!(expanded.receipt.patched_templates, 1755);
    assert_eq!(expanded.receipt.inserted_members, 1755);
    assert_eq!(expanded.staged.receipt.allocated_entries, 0);
    assert_eq!(expanded.staged.receipt.appended_programs, 0);
    let b = prior.input();
    let transition = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: expanded.staged.successor,
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
        expanded.staged.refinement.unwrap(),
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
    full.tree = transition.tree().map(|t| t.input().clone());
    full.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let carried = assemble_owned_release(full, Default::default()).unwrap();
    // First inverse: precisely 1755 membership insertions; every program,
    // declaration, closure and ordered query body is unchanged.
    let bindings: Bindings = read("bindings.json");
    let mut recipe = carried.input().recipe.clone();
    for id in &bindings.added_templates {
        let DefinitionDescriptor::ItemTemplate(row) = recipe
            .schema
            .definitions
            .iter_mut()
            .find(|r| r.address() == id.address())
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(schema) = &mut row.schema else {
            panic!()
        };
        assert!(!schema.modifiers.is_complete());
        assert_eq!(
            schema
                .modifiers
                .members
                .iter()
                .filter(|m| **m == bindings.modifier)
                .count(),
            1
        );
        schema.modifiers.members.retain(|m| *m != bindings.modifier);
    }
    recipe.rules.definitions = b.recipe.rules.definitions.clone();
    recipe.routing.definitions = b.recipe.routing.definitions.clone();
    assert!(
        recipe == b.recipe,
        "only structural Spirit membership may change the recipe"
    );
    migration_preservation::assert_import_rebindings_only(prior, &carried);
    // Add one source form after authenticated unchanged-policy rebinding.
    let mut full = carried.input().clone();
    full.items.version = key("fixed-spirit-items-v1");
    let rule = item_rule();
    assert!(!full.items.rules.iter().any(|r| r.id == rule.id));
    full.items.rules.push(rule.clone());
    let items = OwnedItemLinePolicy::new(
        full.items.clone(),
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    full.item_source.version = key("fixed-spirit-source-v1");
    full.item_source.item_lines = *items.identity();
    conditions_mut(&mut full.item_source.dialect).push(source_condition());
    full.item_source.rule_layouts.push(ItemRuleSourceLayout {
        rule: rule.id,
        role: ItemRuleSourceRole::Unresolved,
    });
    let source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &items,
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    let mut normalization = json!(full.normalization);
    for policy in [
        "equipment_membership",
        "passive_socket_membership",
        "item_modifier_membership",
        "item_parameter_inputs",
    ] {
        assert_eq!(
            normalization[policy]["item_lines"],
            json!(carried.receipt().items)
        );
        assert_eq!(
            normalization[policy]["item_source"],
            json!(carried.receipt().item_source)
        );
        normalization[policy]["item_lines"] = json!(items.identity());
        normalization[policy]["item_source"] = json!(source.identity());
    }
    let equipment = serde_json::from_value(normalization["equipment_membership"].clone()).unwrap();
    normalization["passive_socket_membership"]["equipment"] =
        json!(equipment_membership_identity(&equipment, Default::default()).unwrap());
    full.normalization = serde_json::from_value(normalization).unwrap();
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
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_endpoint(&next);
    // Second inverse: remove exactly the fixed-form rule and source condition,
    // then restore their explicitly enumerated dependency commitments.
    let mut restored = next.input().clone();
    let before = carried.input();
    assert_eq!(restored.items.rules.pop(), Some(item_rule()));
    restored.items.version = before.items.version.clone();
    assert_eq!(
        conditions_mut(&mut restored.item_source.dialect).pop(),
        Some(source_condition())
    );
    assert_eq!(
        restored.item_source.rule_layouts.pop(),
        Some(ItemRuleSourceLayout {
            rule: key("fixed-spirit"),
            role: ItemRuleSourceRole::Unresolved
        })
    );
    restored.item_source.version = before.item_source.version.clone();
    restored.item_source.item_lines = before.item_source.item_lines;
    let old_normalization = json!(before.normalization);
    let mut normalization = json!(restored.normalization);
    for policy in [
        "equipment_membership",
        "passive_socket_membership",
        "item_modifier_membership",
        "item_parameter_inputs",
    ] {
        for dependency in ["item_lines", "item_source"] {
            normalization[policy][dependency] = old_normalization[policy][dependency].clone();
        }
    }
    normalization["passive_socket_membership"]["equipment"] =
        old_normalization["passive_socket_membership"]["equipment"].clone();
    let normalization: NormalizationPolicy = serde_json::from_value(normalization).unwrap();
    assert!(
        normalization == before.normalization,
        "only exact item dependency bindings change"
    );
    restored.normalization = normalization;
    restored.tree.as_mut().unwrap().normalization = before.tree.as_ref().unwrap().normalization;
    assert!(
        restored == *before,
        "fixed Spirit addition whole-input inverse"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
