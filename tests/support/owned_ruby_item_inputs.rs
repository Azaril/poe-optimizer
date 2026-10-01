//! Exact finite physical inputs; static template and rule coverage stay Partial.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_definitions::{ItemTemplateDefId, OwnedDefinitionKey, ParameterSlotDefId},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_item_lines::{ItemEmission, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceError, ItemSourceLayoutPolicy, ItemSourceTemplateDefaults},
    owned_normalize::{
        EquipmentAugmentBase, EquipmentMembershipPolicy, ImportedItemConstructionProfile,
        ItemModifierMembershipPolicy, ItemParameterInputsPolicy, NormalizationArtifacts,
        NormalizationError, OrdinaryItemParameterInputs, OrdinaryMemberCensusBase, normalize_fresh,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
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
    pub census_templates: Vec<OrdinaryMemberCensusBase>,
    pub modifier_rules: Vec<OwnedDefinitionKey>,
}
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/ruby-item-inputs")
}
fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(file)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
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
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn membership() -> MembershipDelta {
    read("membership.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}

pub fn dependency_definitions() -> Vec<DefinitionDescriptor> {
    read("dependencies.json")
}
pub fn construction_profile() -> ImportedItemConstructionProfile {
    read("construction-profile.json")
}
pub fn augment_base() -> EquipmentAugmentBase {
    read("augment-base.json")
}
fn verify_source(a: &Value) {
    let proof = &a["source_validation"];
    assert_eq!(proof["status"], "passed");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes = fs::read(root.join(proof["evidence_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        bytes.len() as u64,
        proof["evidence_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(source["source_revision"], a["source_revision"]);
    assert_eq!(source["source_hash"], a["source_manifest_sha256"]);
    assert!(root.join(a["source_test"].as_str().unwrap()).is_file());
    let manifest =
        fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest)),
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
        assert!(
            source["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|observed| {
                    observed["path"] == pin["path"] && observed["sha256"] == pin["sha256"]
                })
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    verify_source(&a);
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
        assert_eq!(receipt[field], a[field], "exact predecessor {field}");
    }
    let b = prior.input();
    assert!(b.evaluation.is_none());
    let e = extension();
    let extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 6);
    assert_eq!(extended.receipt.refined_subjects, 1);
    assert_eq!(extended.receipt.appended_programs, 1);
    // The checked transition validates the complete old interpretation before
    // changing schema commitments, including the existing Gem inventory proof.
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
    for (id, selection) in [("catalyst-kind", true), ("catalyst-amount", false)] {
        let rule = full
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
            panic!("reviewed shared header")
        };
        for binding in catalyst_bindings() {
            let slot = if selection {
                binding.selection
            } else {
                binding.amount
            };
            assert!(!slots.iter().any(|v| v.declaration == slot.declaration));
            slots.push(slot);
        }
    }
    full.items.version = key("ruby-item-inputs-v1");
    let items = OwnedItemLinePolicy::new(
        full.items.clone(),
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    full.item_source.item_lines = *items.identity();
    full.item_source.version = key("ruby-item-inputs-v1");
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
        // Quality and item-level absence were already independently proved.
        row.parameters = default.parameters;
    }
    let source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &items,
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions,
        mut templates,
        source_base_names,
        loader_jewel_fallback_titles,
    }) = full.normalization.equipment_membership.take()
    else {
        panic!("prior ordinary equipment profile")
    };
    assert_eq!(definitions, carried.assembled().schema().identity().clone());
    let base = augment_base();
    assert!(!templates.iter().any(|row| row.template == base.template));
    templates.push(base);
    full.normalization.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines: *items.identity(),
            item_source: *source.identity(),
            imported_profiles: vec![construction_profile()],
        },
    );
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
        definitions,
        item_lines,
        item_source,
        templates,
        modifier_rules,
        paired_templates: _,
        census_templates,
    }) = &mut full.normalization.item_modifier_membership
    else {
        panic!("prior singleton, pair and census profile")
    };
    assert_eq!(
        *definitions,
        carried.assembled().schema().identity().clone()
    );
    assert_eq!(*item_lines, *carried.items().identity());
    assert_eq!(*item_source, *carried.item_source().identity());
    for row in membership().census_templates {
        assert!(!templates.iter().any(|old| old.template == row.template));
        assert!(
            !census_templates
                .iter()
                .any(|old| old.template == row.template)
        );
        census_templates.push(row);
    }
    for rule in membership().modifier_rules {
        assert!(!modifier_rules.contains(&rule));
        modifier_rules.push(rule);
    }
    *item_lines = *items.identity();
    *item_source = *source.identity();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        definitions,
        item_lines,
        item_source,
        templates,
    }) = &mut full.normalization.item_parameter_inputs
    else {
        panic!("prior physical-input profile")
    };
    assert_eq!(
        *definitions,
        carried.assembled().schema().identity().clone()
    );
    assert_eq!(*item_lines, *carried.items().identity());
    assert_eq!(*item_source, *carried.item_source().identity());
    for row in bindings() {
        assert!(!templates.iter().any(|v| v.template == row.template));
        templates.push(row);
    }
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
        kind: key("explicit-ruby-item-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ruby-item-inputs-v1",
            &(
                a,
                &e,
                bindings(),
                catalyst_bindings(),
                defaults(),
                membership(),
                construction_profile(),
                augment_base(),
                dependency_definitions(),
            ),
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
    for dependency in dependency_definitions() {
        assert_eq!(
            b.recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == dependency.address()),
            Some(&dependency)
        );
    }
    assert_eq!(
        r.recipe.rules.operations_version,
        b.recipe.rules.operations_version
    );
    assert_eq!(
        r.recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v14"
    );
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 6
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x3202);
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len()
    );
    assert_eq!(r.recipe.schema.slots.len(), b.recipe.schema.slots.len() + 6);
    for requested in &e.schema {
        match requested {
            SchemaExtensionEntry::Definition(d) => {
                let row = r
                    .recipe
                    .schema
                    .definitions
                    .iter_mut()
                    .find(|v| v.address() == d.address())
                    .unwrap();
                assert_eq!(*row, *d);
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
                let i = r
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .position(|v| v.address() == s.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.slots.remove(i), *s);
            }
        }
    }
    for requested in &e.owners {
        let owner = r
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|v| v.owner == requested.owner)
            .unwrap();
        let before = b
            .recipe
            .rules
            .owners
            .iter()
            .find(|v| v.owner == requested.owner)
            .unwrap();
        assert_eq!(*owner, *requested);
        assert_eq!(owner.programs.closure, before.programs.closure);
        assert_eq!(
            owner.programs.members.len(),
            before.programs.members.len() + 1
        );
        for program in &before.programs.members {
            assert!(owner.programs.members.contains(program));
        }
        assert_eq!(
            owner
                .programs
                .members
                .iter()
                .filter(|p| p.id.as_str() == "catalyst-inputs")
                .count(),
            1
        );
        *owner = before.clone();
    }
    for default in defaults() {
        let row = r
            .item_source
            .template_defaults
            .iter_mut()
            .find(|v| v.template == default.template)
            .unwrap();
        assert_eq!(*row, default);
        *row = b
            .item_source
            .template_defaults
            .iter()
            .find(|v| v.template == default.template)
            .unwrap()
            .clone();
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
            panic!("reviewed header")
        };
        for binding in catalyst_bindings() {
            let expected = if selection {
                binding.selection
            } else {
                binding.amount
            };
            let i = slots.iter().position(|v| v == &expected).unwrap();
            assert_eq!(slots.remove(i), expected);
        }
    }
    let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        definitions,
        mut templates,
        source_base_names,
        loader_jewel_fallback_titles,
        item_lines,
        item_source,
        imported_profiles,
    }) = r.normalization.equipment_membership.take()
    else {
        panic!("new imported equipment profile")
    };
    assert_eq!(item_lines, *next.items().identity());
    assert_eq!(item_source, *next.item_source().identity());
    assert_eq!(imported_profiles, vec![construction_profile()]);
    assert_eq!(templates.pop().unwrap(), augment_base());
    r.normalization.equipment_membership = Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
    });
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
        modifier_rules,
        census_templates,
        ..
    }) = &mut r.normalization.item_modifier_membership
    else {
        panic!("retained declared census profile")
    };
    for expected in membership().census_templates {
        let index = census_templates
            .iter()
            .position(|row| row.template == expected.template)
            .unwrap();
        assert_eq!(census_templates.remove(index), expected);
    }
    for expected in membership().modifier_rules {
        let index = modifier_rules
            .iter()
            .position(|rule| rule == &expected)
            .unwrap();
        assert_eq!(modifier_rules.remove(index), expected);
    }
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut r.normalization.item_parameter_inputs
    else {
        panic!()
    };
    for expected in bindings() {
        let i = templates
            .iter()
            .position(|v| v.template == expected.template)
            .unwrap();
        assert_eq!(templates.remove(i), expected);
    }
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.version = b.items.version.clone();
    r.items.definitions = b.items.definitions.clone();
    r.item_source.version = b.item_source.version.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    // Only these exact dependency commitments may differ in the carried policy.
    // Gem scalar/source semantics, role catalogue, every old row and all options
    // are checked by the final whole-endpoint equality assertion below.
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
        "/configuration_reward_inventory/reward_policy",
    ] {
        *n.pointer_mut(path).unwrap() = old.pointer(path).unwrap().clone();
    }
    r.normalization = serde_json::from_value(n).unwrap();
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    r.provenance.pop();
    assert!(
        r == *b,
        "endpoint differs outside the six slots, one transport program, explicit imported profile and exact dependency bindings"
    );
}

pub fn assert_cross_owner_rejected(next: &StagedOwnedRelease) {
    let ruby = catalyst_bindings().remove(0);
    let foreign_slot = next
        .item_source()
        .input()
        .template_defaults
        .iter()
        .filter(|d| d.template != ruby.template)
        .flat_map(|d| &d.parameters)
        .find(|d| {
            matches!(
                d.assignment.value,
                poe_optimizer_core::owned_build::ParameterValue::Option(_)
            )
        })
        .unwrap()
        .assignment
        .slot
        .clone();
    let mut wrong_default = next.item_source().input().clone();
    let row = wrong_default
        .template_defaults
        .iter_mut()
        .find(|row| row.template == ruby.template)
        .unwrap();
    let value = row
        .parameters
        .iter_mut()
        .find(|value| value.assignment.slot == ruby.selection)
        .unwrap();
    value.assignment.slot = foreign_slot;
    assert!(matches!(
        ItemSourceLayoutPolicy::new(
            wrong_default,
            next.items(),
            next.assembled().schema(),
            Default::default()
        ),
        Err(ItemSourceError::Policy(
            "invalid default parameter owner, duplicate or headers"
        ))
    ));

    let ruby_raw = bindings().remove(0);
    let mut wrong_raw = next.normalization().clone();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut wrong_raw.item_parameter_inputs
    else {
        panic!("physical input profile")
    };
    let foreign_raw = templates
        .iter()
        .find(|v| v.template != ruby_raw.template)
        .unwrap()
        .corruption_slot
        .clone();
    let row = templates
        .iter_mut()
        .find(|row| row.template == ruby_raw.template)
        .unwrap();
    assert_eq!(row.corruption_slot, ruby_raw.corruption_slot);
    row.corruption_slot = foreign_raw;
    let xml = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-04.xml"),
    )
    .unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(&xml).unwrap(),
        BuildLineage::from_bytes([0x5c; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let queries = &next
        .input()
        .query_sets
        .iter()
        .find(|set| set.name.as_str() == "original-04")
        .unwrap()
        .queries;
    assert!(matches!(
        normalize_fresh(
            &evidence,
            *imported.allocator_state(),
            NormalizationArtifacts {
                mappings: next.mapping(),
                registry: next.assembled().registry(),
                definitions: next.assembled().schema(),
                roles: next.roles(),
                rewards: next.rewards(),
                items: next.items(),
                item_source: next.item_source(),
                tree: next.tree(),
            },
            &wrong_raw,
            queries,
            Default::default(),
        ),
        Err(NormalizationError::Policy("item parameter projected slot"))
    ));
}
