//! Checked addition of incoming-hit inputs; existing inventories remain Partial.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SchemaState},
};
use poe_optimizer_import::{
    owned_normalize::ConfigurationInputsPolicy,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/incoming-damage-inputs")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).unwrap()
}
pub fn policy() -> ConfigurationInputsPolicy {
    read("policy.json")
}
fn prior_policy() -> ConfigurationInputsPolicy {
    serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/configuration-block-inputs/policy.json"))
            .unwrap(),
    )
    .unwrap()
}
pub fn check_authored() {
    let ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
        mapping_source: old_source,
        encounter: old_encounter,
        inputs: old,
        placeholder_fallback_inputs: old_fallbacks,
    } = prior_policy()
    else {
        panic!("historical V2 configuration inputs")
    };
    let ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source,
        encounter,
        inputs,
        placeholder_fallback_inputs,
        option_inputs,
        ..
    } = policy()
    else {
        panic!("explicit V3 string option authority")
    };
    assert_eq!(old.len(), 6);
    assert_eq!(old_fallbacks.len(), 1);
    assert_eq!(inputs.len(), 14);
    assert_eq!(inputs[..6], old);
    assert_eq!(placeholder_fallback_inputs, old_fallbacks);
    assert_eq!(mapping_source, old_source);
    assert_eq!(encounter, old_encounter);
    assert_eq!(option_inputs.len(), 1);
    let native: Value = read("native-inputs.json");
    assert_eq!(native["encounter"], json!(encounter));
    assert_eq!(
        native["category_input"],
        json!(option_inputs[0].value_input)
    );
    assert_eq!(native["inputs"].as_array().unwrap().len(), 8);
    for (input, actual) in inputs
        .iter()
        .skip(6)
        .zip(native["inputs"].as_array().unwrap())
    {
        assert_eq!(json!(input.source_name), actual["source_name"]);
        assert_eq!(json!(input.presence_input), actual["presence_input"]);
        assert_eq!(json!(input.value_input), actual["value_input"]);
    }
    let option = &option_inputs[0];
    assert_eq!(option.source_name, "enemyDamageType");
    let recipe = json!(option.recipe);
    assert_eq!(recipe["codec"]["whitespace"], "exact");
    assert_eq!(recipe["tiers"][0]["selectors"][0]["lane"], "input_string");
    assert_eq!(recipe["tiers"][0]["duplicates"], "reject");
    assert_eq!(recipe["missing"]["kind"], "pending");
    let tokens = recipe["codec"]["codec"]["value"]["tokens"]
        .as_array()
        .unwrap();
    let categories = native["categories"].as_array().unwrap();
    assert_eq!(tokens.len(), 7);
    assert_eq!(tokens.len(), categories.len());
    for (token, category) in tokens.iter().zip(categories) {
        assert_eq!(token["token"], category["name"]);
        assert_eq!(token["value"], category["option"]);
    }
    assert_eq!(categories[0]["name"], "Average");
    assert_eq!(json!(option.constructor_default), categories[0]["option"]);
    let extension: OwnedRecipeExtension = read("extension.json");
    assert_eq!(extension.schema.len(), 44);
    assert_eq!(extension.tables.len(), 1);
    assert_eq!(extension.owners.len(), 1);
    assert_eq!(extension.owners[0].programs.members.len(), 1);
    assert!(!extension.owners[0].programs.is_complete());
    assert!(extension.receivers.is_empty());
    assert_eq!(
        extension.operations_version,
        Some(key("owned-domain-operations-v15"))
    );
    let authored: Value = read("authoring.json");
    assert_eq!(authored["allocated_definitions"], 43);
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(authored["source_revision"], manifest["upstream_revision"]);
    assert_eq!(
        authored["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    for pin in authored["source_files"].as_array().unwrap() {
        let matched: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|actual| actual["path"] == pin["path"] && actual["sha256"] == pin["sha256"])
            .collect();
        assert_eq!(matched.len(), 1, "exact pinned source identity");
        if let Some(bytes) = pin.get("bytes") {
            assert_eq!(matched[0]["bytes"], *bytes);
        }
    }
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(
        proof["status"], "passed",
        "publication requires a completed source witness"
    );
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(
        off == on,
        "exact complete-source JIT evidence; full bytes remain in witness files"
    );
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(evidence["source_revision"], authoring["source_revision"]);
    assert_eq!(evidence["source_hash"], authoring["source_manifest_sha256"]);
    assert_eq!(evidence["evidence"]["business_method_wrappers"], false);
    assert_eq!(evidence["evidence"]["native_coverage"], false);
    assert_eq!(evidence["evidence"]["whole_build_parity"], false);
    let observer = fs::read(
        root().join("crates/poe-optimizer-pob/tests/support/owned_incoming_damage_source.lua"),
    )
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&observer)),
        evidence["evidence"]["observer_sha256"]
    );
    let pins = authoring["source_files"].as_array().unwrap();
    assert!(!pins.is_empty());
    for pin in pins {
        assert!(
            evidence["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|actual| actual["path"] == pin["path"] && actual["sha256"] == pin["sha256"])
        );
    }
    let originals = evidence["evidence"]["originals"].as_array().unwrap();
    assert_eq!(originals.len(), 5);
    for case in 1..=5 {
        let bytes = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(
            originals
                .iter()
                .filter(|row| row["sha256"] == digest)
                .count(),
            1
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authored: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    for field in ["definitions", "registry"] {
        assert_eq!(authored[field], receipt[field]);
    }
    assert_eq!(authored["before"], receipt["input"]);
    source_proof(&authored);
    let b = prior.input();
    assert_eq!(b.normalization.configuration_inputs, Some(prior_policy()));
    assert!(b.evaluation.is_none());
    let extension: OwnedRecipeExtension = read("extension.json");
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 43);
    assert_eq!(extended.receipt.refined_subjects, 1);
    assert_eq!(extended.receipt.appended_programs, 1);
    assert_eq!(extended.receipt.appended_tables, 1);
    assert_eq!(extended.receipt.appended_receivers, 0);
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
        extended
            .refinement
            .expect("exact encounter input membership addition"),
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
    full.normalization.configuration_inputs = Some(policy());
    full.normalization.version = key("selected-incoming-damage-inputs-v3");
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            b.tree.as_ref().unwrap().content.clone(),
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
        kind: key("explicit-incoming-damage-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-incoming-damage-inputs-authoring-v1",
            &(
                authored,
                &extension,
                policy(),
                read::<Value>("native-inputs.json"),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &next, &extension);
    next
}
fn preservation(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    extension: &OwnedRecipeExtension,
) {
    let b = prior.input();
    let mut r = next.input().clone();
    assert_eq!(
        r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries
    );
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 43
    );
    assert_eq!(
        r.recipe.registry.last_issued.get(),
        b.recipe.registry.last_issued.get() + 43
    );
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 43
    );
    let ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        encounter,
        inputs,
        option_inputs,
        ..
    } = policy()
    else {
        panic!()
    };
    let additions: Vec<_> = inputs
        .iter()
        .skip(6)
        .flat_map(|input| [input.presence_input.clone(), input.value_input.clone()])
        .chain(option_inputs.iter().map(|input| input.value_input.clone()))
        .collect();
    assert_eq!(additions.len(), 17);
    for row in &extension.schema {
        let SchemaExtensionEntry::Definition(expected) = row else {
            panic!("no slot additions")
        };
        let at = r
            .recipe
            .schema
            .definitions
            .iter()
            .position(|row| row.address() == expected.address())
            .unwrap();
        assert_eq!(&r.recipe.schema.definitions[at], expected);
        if let Some(old) = b
            .recipe
            .schema
            .definitions
            .iter()
            .find(|row| row.address() == expected.address())
        {
            let (DefinitionDescriptor::Encounter(old), DefinitionDescriptor::Encounter(new)) =
                (old, expected)
            else {
                panic!("only Encounter membership refines")
            };
            assert_eq!(new.id, encounter);
            let (SchemaState::Known(old_schema), SchemaState::Known(new_schema)) =
                (&old.schema, &new.schema)
            else {
                panic!("known Encounter")
            };
            let mut restored = new_schema.clone();
            assert_eq!(
                restored.external_inputs.members.len(),
                old_schema.external_inputs.members.len() + additions.len()
            );
            for id in &additions {
                assert!(!old_schema.external_inputs.members.contains(id));
                assert_eq!(
                    restored
                        .external_inputs
                        .members
                        .iter()
                        .filter(|row| *row == id)
                        .count(),
                    1
                );
                restored.external_inputs.members.retain(|row| row != id);
            }
            assert_eq!(&restored, old_schema);
            r.recipe.schema.definitions[at] = DefinitionDescriptor::Encounter(old.clone());
        } else {
            r.recipe.schema.definitions.remove(at);
        }
    }
    assert_eq!(r.recipe.schema, b.recipe.schema);
    let addition = &extension.owners[0];
    let old = b
        .recipe
        .rules
        .owners
        .iter()
        .find(|row| row.owner == addition.owner)
        .unwrap();
    let new = r
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|row| row.owner == addition.owner)
        .unwrap();
    assert_eq!(new.programs.closure, old.programs.closure);
    assert_eq!(addition.programs.closure, old.programs.closure);
    let mut programs = old.programs.members.clone();
    programs.extend(addition.programs.members.clone());
    assert_eq!(new.programs.members, programs);
    *new = old.clone();
    let mut tables = b.recipe.rules.tables.clone();
    tables.extend(extension.tables.clone());
    assert_eq!(r.recipe.rules.tables, tables);
    r.recipe.rules.tables = b.recipe.rules.tables.clone();
    assert_eq!(
        r.recipe.rules.operations_version,
        key("owned-domain-operations-v15")
    );
    assert_eq!(
        r.recipe.rules.operations_version,
        b.recipe.rules.operations_version
    );
    assert!(b.recipe.rules.effect_applications.is_some());
    assert_eq!(
        r.recipe.rules.effect_applications,
        b.recipe.rules.effect_applications
    );
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
    assert_eq!(r.normalization.configuration_inputs, Some(policy()));
    assert_eq!(
        r.normalization.version,
        key("selected-incoming-damage-inputs-v3")
    );
    r.normalization.configuration_inputs = b.normalization.configuration_inputs.clone();
    r.normalization.version = b.normalization.version.clone();
    let mut normalization = json!(r.normalization);
    let old_normalization = json!(b.normalization);
    for path in [
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/support_origin_order/roles",
        "/payload_inventory/roles",
        "/gem_inventory/definitions",
        "/gem_inventory/roles",
        "/gem_inventory/scalar_inputs",
        "/equipment_membership/definitions",
        "/equipment_membership/item_lines",
        "/equipment_membership/item_source",
        "/passive_socket_membership/definitions",
        "/passive_socket_membership/mapping",
        "/passive_socket_membership/equipment",
        "/passive_socket_membership/item_lines",
        "/passive_socket_membership/item_source",
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
        "/item_parameter_inputs/definitions",
        "/item_parameter_inputs/item_lines",
        "/item_parameter_inputs/item_source",
        "/configuration_reward_inventory/reward_policy",
    ] {
        *normalization.pointer_mut(path).unwrap() =
            old_normalization.pointer(path).unwrap().clone();
    }
    r.normalization = serde_json::from_value(normalization).unwrap();
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    let added = r.provenance.pop().unwrap();
    assert_eq!(added.kind, key("explicit-incoming-damage-inputs"));
    assert_eq!(added.prior_input, prior.receipt().input);
    assert!(
        r == *b,
        "changes are limited to 43 definitions, 17 Encounter members, one program/table, V3 configuration policy and checked dependency bindings"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().evaluation.is_none());
}
