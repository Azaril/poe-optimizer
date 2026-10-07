//! Global Energy Shield inputs reuse ordinary scalar programs and the existing
//! Actor contribution channel. Local item Energy Shield remains a separate family.
#[path = "owned_release_migration_preservation.rs"]
mod migration_preservation;

use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceLayoutPolicy,
    },
    owned_mapping::OwnedIdRegistry,
    owned_modifier_value_recipe::{
        ModifierValueBinding, ModifierValuePolicy, compile_owned_modifier_values,
    },
    owned_normalize::{NormalizationPolicy, equipment_membership_identity},
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
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub const KIND: &str = "global-energy-shield-inputs";
const PAYLOADS: [&str; 7] = [
    "bindings.json",
    "dependencies.json",
    "extension.json",
    "numeric-binding.json",
    "item-rule.json",
    "source-condition.json",
    "source-vectors.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/global-energy-shield-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
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
pub fn bindings() -> Bindings {
    read("bindings.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn item_rule() -> ItemLineRule {
    read("item-rule.json")
}
pub fn source_condition() -> ItemSourceConditionalMember {
    read("source-condition.json")
}
pub fn numeric_policy(definitions: DataIdentity) -> ModifierValuePolicy {
    ModifierValuePolicy {
        schema_version: 1,
        version: key("global-energy-shield-numeric-v1"),
        definitions,
        factor_unit: bindings().factor_unit,
        bindings: vec![read("numeric-binding.json")],
    }
}
fn digest() -> OwnedContentDigest {
    let mut values: Vec<Value> = PAYLOADS.into_iter().map(read).collect();
    values.push(read("authoring.json"));
    digest_owned(KIND, &values, 4 * 1024 * 1024).unwrap()
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
    let b = bindings();
    let e = extension();
    let numeric: ModifierValueBinding = read("numeric-binding.json");
    assert_eq!(b.modifier.key().as_str(), "def.0000000000003334");
    assert_eq!(b.amount.slot.key().as_str(), "def.0000000000003335");
    // Twenty source-property flags plus the separate unscalable control.
    assert_eq!(b.properties.len(), 21);
    assert_eq!(b.corrupted_base.slot.key().as_str(), "def.000000000000334b");
    assert_eq!(b.category.slot.key().as_str(), "def.000000000000334c");
    assert_eq!(b.unit.key().as_str(), "def.0000000000000002");
    assert_eq!(b.factor_unit.key().as_str(), "def.0000000000000001");
    assert_eq!(b.effective.key().as_str(), "def.000000000000253e");
    assert_eq!(b.contribution.key().as_str(), "def.00000000000029f0");
    assert_eq!(b.templates.len(), 1756);
    assert!(b.templates.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(e.schema.len(), 25);
    assert!(e.operations_version.is_none() && e.tables.is_empty() && e.receivers.is_empty());
    assert_eq!(e.owners.len(), 1);
    assert_eq!(
        e.owners[0].owner,
        SchemaSubject::Definition(b.modifier.address())
    );
    assert!(!e.owners[0].programs.is_complete());
    assert_eq!(e.owners[0].programs.members.len(), 4);
    let delivery = e.owners[0].programs.members.last().unwrap();
    assert_eq!(
        delivery.id.as_str(),
        "contribute-player-global-energy-shield"
    );
    assert_eq!(delivery.context, RuleEntityKind::EquipmentUse);
    assert_eq!(delivery.reads.len(), 1);
    assert_eq!(delivery.nodes.len(), 1);
    assert_eq!(delivery.effects.len(), 1);
    assert!(delivery.effects[0].when.is_none());
    assert!(
        matches!(&delivery.effects[0].effect, RuleEffectKind::Contribute {
        entity: RuleEntity::Player, stat, contribution: ContributionKind::Increase, value
    } if *stat == b.contribution && value.as_str() == "effective")
    );
    assert_eq!(numeric.modifier, b.modifier);
    assert_eq!(numeric.input, b.amount);
    assert_eq!(numeric.unit, b.unit);
    assert_eq!(numeric.output, b.effective);
    assert_eq!(numeric.precision, 1);
    assert_eq!(numeric.display_precision, 0);
    let rule: Value = read("item-rule.json");
    assert_eq!(rule["id"], "fixed-global-energy-shield-increase");
    assert_eq!(
        rule["pattern"],
        json!([
            {"kind":"numeric_capture","value":{"capture":"amount","syntax":"integer","sign":"forbidden"}},
            {"kind":"literal","value":"% increased maximum Energy Shield"}
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
    let all_slots: BTreeSet<_> = rolls
        .iter()
        .map(|r| {
            serde_json::from_value::<DeclaredSlot<ParameterSlotDefId>>(r["slot"].clone()).unwrap()
        })
        .collect();
    assert_eq!(all_slots.len(), 24);
    for (property, slot) in &b.properties {
        let row = rolls.iter().find(|r| r["slot"] == json!(slot)).unwrap();
        assert_eq!(
            row["value"],
            if property == "unscalable" {
                json!({"kind":"literal","value":{"kind":"boolean","value":false}})
            } else {
                json!({"kind":"property","value":{"property":property}})
            }
        );
    }
    assert_eq!(
        read::<Value>("source-condition.json"),
        json!({"rule":"fixed-global-energy-shield-increase","all":[
            {"kind":"no_source_tags"},{"kind":"no_generated_buff_members"},
            {"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
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
    for expected in read::<Vec<DefinitionDescriptor>>("dependencies.json") {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|r| r.address() == expected.address()),
            Some(&expected)
        );
    }
}
fn patch(
    prior: &StagedOwnedRelease,
    extension: &OwnedRecipeExtension,
) -> RecipeMembershipPatchInput {
    let b = bindings();
    RecipeMembershipPatchInput {
        schema_version: 1,
        version: key("global-energy-shield-membership-v1"),
        before: RecipeMembershipPatchBindings::from_recipe(prior.assembled()),
        extension: digest_owned("owned-recipe-extension-v1", extension, 16 * 1024 * 1024).unwrap(),
        patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
            templates: b.templates,
            add: vec![b.modifier],
        }],
    }
}
pub fn compiled_extension(prior: &StagedOwnedRelease) -> OwnedRecipeExtension {
    let mut e = extension();
    let first = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &e,
        &patch(prior, &e),
        Default::default(),
    )
    .unwrap();
    let first = assemble_owned_recipe(first.staged.successor, Default::default()).unwrap();
    let numeric = compile_owned_modifier_values(
        &first,
        &numeric_policy(first.schema().identity().clone()),
        Default::default(),
    )
    .unwrap();
    assert_eq!(numeric.extension.owners.len(), 1);
    let owner = &numeric.extension.owners[0];
    assert_eq!(owner.owner, e.owners[0].owner);
    assert_eq!(owner.programs.closure, e.owners[0].programs.closure);
    assert_eq!(owner.programs.members.len(), 1);
    e.owners[0]
        .programs
        .members
        .extend(owner.programs.members.clone());
    e
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let a: Value = read("authoring.json");
    let b = bindings();
    let e = extension();
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.registry.last_issued.get(), 0x334c);
    assert_eq!(endpoint.input().items.rules.last(), Some(&item_rule()));
    assert_eq!(
        conditions(&endpoint.input().item_source.dialect).last(),
        Some(&source_condition())
    );
    let owner = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|r| r.owner == e.owners[0].owner)
        .unwrap();
    assert_eq!(owner.programs.closure, e.owners[0].programs.closure);
    assert_eq!(owner.programs.members.len(), 5);
    for p in &e.owners[0].programs.members {
        assert_eq!(
            owner.programs.members.iter().find(|r| r.id == p.id),
            Some(p)
        );
    }
    // Regenerate the one normal formatter against this exact owned schema;
    // no copied interpreter or reference formula stands in for the compiler.
    let numeric = compile_owned_modifier_values(
        endpoint.assembled(),
        &numeric_policy(endpoint.assembled().schema().identity().clone()),
        Default::default(),
    )
    .unwrap();
    assert_eq!(numeric.extension.owners.len(), 1);
    assert_eq!(numeric.extension.owners[0].programs.members.len(), 1);
    let program = &numeric.extension.owners[0].programs.members[0];
    assert_eq!(
        owner.programs.members.iter().find(|r| r.id == program.id),
        Some(program)
    );
    for id in &b.templates {
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
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x3333);
    let e = compiled_extension(prior);
    let expanded = compile_owned_recipe_membership_patch(
        prior.assembled(),
        &e,
        &patch(prior, &e),
        Default::default(),
    )
    .unwrap();
    assert_eq!(expanded.receipt.patched_templates, 1756);
    assert_eq!(expanded.receipt.inserted_members, 1756);
    assert_eq!(expanded.staged.receipt.allocated_entries, 25);
    assert_eq!(expanded.staged.receipt.appended_programs, 5);
    assert_eq!(expanded.staged.receipt.appended_receivers, 0);
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
    // Exact first inverse includes typed allocation order, all new bodies and
    // memberships; every old local/global provider and query survives intact.
    let binding = bindings();
    let mut registry = OwnedIdRegistry::new(b.recipe.registry.clone(), Default::default()).unwrap();
    assert_eq!(
        registry
            .allocate_definition::<ModifierDefinition>()
            .unwrap(),
        binding.modifier
    );
    let mut recipe = carried.input().recipe.clone();
    for addition in &e.schema {
        match addition {
            SchemaExtensionEntry::Definition(d) => {
                let position = recipe
                    .schema
                    .definitions
                    .iter()
                    .position(|r| r.address() == d.address())
                    .unwrap();
                assert_eq!(recipe.schema.definitions.remove(position), *d);
            }
            SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(slot)) => {
                assert_eq!(
                    registry
                        .allocate_slot::<ParameterSlotDefinition>(slot.id.declaration.clone())
                        .unwrap(),
                    slot.id
                );
                let position = recipe
                    .schema
                    .slots
                    .iter()
                    .position(|r| r.address() == SlotDescriptor::Parameter(slot.clone()).address())
                    .unwrap();
                assert_eq!(
                    recipe.schema.slots.remove(position),
                    SlotDescriptor::Parameter(slot.clone())
                );
            }
            _ => panic!("only the one modifier and its parameter slots are allocated"),
        }
    }
    assert_eq!(recipe.registry, *registry.input());
    for id in &binding.templates {
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
                .filter(|m| **m == binding.modifier)
                .count(),
            1
        );
        schema.modifiers.members.retain(|m| *m != binding.modifier);
    }
    let position = recipe
        .rules
        .owners
        .iter()
        .position(|o| o.owner == e.owners[0].owner)
        .unwrap();
    assert_eq!(recipe.rules.owners.remove(position), e.owners[0]);
    recipe.registry = b.recipe.registry.clone();
    recipe.rules.definitions = b.recipe.rules.definitions.clone();
    recipe.routing.definitions = b.recipe.routing.definitions.clone();
    assert!(
        recipe == b.recipe,
        "only the new global modifier, slots, programs and structural membership may change the recipe"
    );
    migration_preservation::assert_import_rebindings_only(prior, &carried);
    let mut full = carried.input().clone();
    full.items.version = key("global-energy-shield-items-v1");
    let rule = item_rule();
    assert!(!full.items.rules.iter().any(|r| r.id == rule.id));
    full.items.rules.push(rule.clone());
    let items = OwnedItemLinePolicy::new(
        full.items.clone(),
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    full.item_source.version = key("global-energy-shield-source-v1");
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
    // Second inverse allows only the additional import grammar and its exact
    // dependency rebinding, including the inherited equipment-policy digest.
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
            rule: key("fixed-global-energy-shield-increase"),
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
        "global Energy Shield input whole-release inverse"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
