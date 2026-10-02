//! Checked offline publication of an explicitly Partial effect producer family.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{EffectApplicationRule, OWNED_RULE_OPERATIONS_V15},
    owned_schema::{
        DeclaredSet, DefinitionDescriptor, SchemaDefinitionId, SchemaSubject, SlotDescriptor,
    },
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_effective_gem_recipe::{
        EffectiveGemRecipeBinding, EffectiveGemRecipeInput, EffectiveGemRecipeOutput,
        compile_effective_gem_recipe,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/pain-offering")
                .join(file),
        )
        .unwrap(),
    )
    .unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn check_authored() {
    let extension: OwnedRecipeExtension = read("extension.json");
    let applications: DeclaredSet<EffectApplicationRule> = read("applications.json");
    let bindings: Vec<EffectiveGemRecipeBinding> = read("effective-input-binding.json");
    let _: DefinitionDescriptor = read("skill-declaration.json");
    let _: Vec<DefinitionDescriptor> = read("dependencies.json");
    let _: Vec<SlotDescriptor> = read("dependency-slots.json");
    assert!(extension.operations_version.is_none());
    assert!(!extension.schema.is_empty());
    assert!(!extension.owners.is_empty());
    assert!(extension.owners.iter().all(|o| !o.programs.is_complete()));
    assert_eq!(applications.members.len(), 1);
    assert!(!applications.is_complete());
    assert_eq!(bindings.len(), 1);
    assert_eq!(extension.tables.len(), 1);
    assert_eq!(extension.tables[0].rows.len(), 40);
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(
        off == on,
        "fresh complete-source observations agree in both JIT modes"
    );
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(evidence["source_revision"], authoring["source_revision"]);
    assert_eq!(evidence["source_hash"], authoring["source_manifest_sha256"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(
            evidence["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| { row["path"] == pin["path"] && row["sha256"] == pin["sha256"] })
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    source_proof(&authoring);
    for definition in read::<Vec<DefinitionDescriptor>>("dependencies.json") {
        assert!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .contains(&definition)
        );
    }
    for slot in read::<Vec<SlotDescriptor>>("dependency-slots.json") {
        assert!(prior.input().recipe.schema.slots.contains(&slot));
    }
    let declaration: DefinitionDescriptor = read("skill-declaration.json");
    let revision = OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key("pob-3887ae68-pain-offering-v1"),
        reason: key("declare-partial-pain-offering-skill"),
        definitions: vec![declaration],
        slots: vec![],
    };
    let revised =
        compile_owned_release_revision(prior, revision.clone(), Default::default()).unwrap();
    let extension: OwnedRecipeExtension = read("extension.json");
    let mut extended =
        extend_owned_recipe(revised.assembled(), &extension, Default::default()).unwrap();
    let schema =
        OwnedDefinitionSchemaPackage::new(extended.successor.schema.clone(), Default::default())
            .unwrap();
    let policy = EffectiveGemRecipeInput {
        schema_version: 1,
        version: key("pain-offering-pre-support-inputs-v1"),
        definitions: schema.identity().clone(),
        bindings: read("effective-input-binding.json"),
    };
    let compiled = compile_effective_gem_recipe(&policy, &schema, Default::default()).unwrap();
    assert_eq!(compiled.programs.len(), 1);
    for row in &compiled.programs {
        let subject = SchemaSubject::Definition(row.gem.address());
        let owner = extended
            .successor
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject)
            .unwrap();
        assert!(!owner.programs.is_complete());
        assert!(
            !owner
                .programs
                .members
                .iter()
                .any(|p| p.id == row.program.id)
        );
        owner.programs.members.push(row.program.clone());
    }
    let base = revised.input();
    let carried = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: base.recipe.clone(),
            successor: extended.successor,
            mapping: base.mapping.clone(),
            roles: base.roles.clone(),
            normalization: base.normalization.clone(),
            rewards: base.rewards.clone(),
            query_sets: base.query_sets.clone(),
            items: base.items.clone(),
            item_source: base.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: base.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(base.tree.clone().unwrap()),
        },
        extended
            .refinement
            .expect("explicit Gem and Skill membership refinement"),
        Default::default(),
    )
    .unwrap();
    let mut full = base.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|t| t.input().clone());
    let applications: DeclaredSet<EffectApplicationRule> = read("applications.json");
    assert!(full.recipe.rules.effect_applications.is_none());
    full.recipe.rules.operations_version = key(OWNED_RULE_OPERATIONS_V15);
    full.recipe.rules.effect_applications = Some(applications.clone());
    // The full assembler admits the explicit V15 registry. Older migration and
    // extension contracts remain unchanged and cannot invent this inventory.
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("native-pain-offering-damage-applications"),
        prior_input: revised.receipt().input,
        authoring_input: digest_owned(
            "owned-pain-offering-publication-v1",
            &(
                authoring,
                revision,
                &extension,
                applications,
                policy,
                &compiled,
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preserve(prior, &next, &extension, &compiled);
    next
}

fn preserve(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    extension: &OwnedRecipeExtension,
    compiled: &EffectiveGemRecipeOutput,
) {
    let before = prior.input();
    let mut restored = next.input().clone();
    assert_eq!(next.receipt().source, prior.receipt().source);
    assert_eq!(
        (next.receipt().query_sets, next.receipt().query_rows),
        (5, 110)
    );
    assert_eq!(
        restored.recipe.registry.entries.len(),
        before.recipe.registry.entries.len() + 13
    );
    assert_eq!(
        &restored.recipe.registry.entries[..before.recipe.registry.entries.len()],
        &before.recipe.registry.entries
    );
    assert_eq!(restored.recipe.registry.last_issued.get(), 0x322d);
    restored.recipe.registry = before.recipe.registry.clone();
    for entry in &extension.schema {
        match entry {
            SchemaExtensionEntry::Definition(row) => {
                let index = restored
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .position(|d| d.address() == row.address())
                    .unwrap();
                assert_eq!(&restored.recipe.schema.definitions[index], row);
                if let Some(old) = before
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == row.address())
                {
                    restored.recipe.schema.definitions[index] = old.clone();
                } else {
                    restored.recipe.schema.definitions.remove(index);
                }
            }
            SchemaExtensionEntry::Slot(row) => {
                let index = restored
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .position(|s| s.address() == row.address())
                    .unwrap();
                assert_eq!(&restored.recipe.schema.slots[index], row);
                assert!(
                    !before
                        .recipe
                        .schema
                        .slots
                        .iter()
                        .any(|s| s.address() == row.address())
                );
                restored.recipe.schema.slots.remove(index);
            }
        }
    }
    restored.recipe.schema.release = before.recipe.schema.release.clone();
    let mut expected = extension.owners.clone();
    for row in &compiled.programs {
        expected
            .iter_mut()
            .find(|o| o.owner == SchemaSubject::Definition(row.gem.address()))
            .unwrap()
            .programs
            .members
            .push(row.program.clone());
    }
    for mut owner in expected {
        assert!(
            !before
                .recipe
                .rules
                .owners
                .iter()
                .any(|o| o.owner == owner.owner)
        );
        owner.programs.members.sort_by(|a, b| a.id.cmp(&b.id));
        let index = restored
            .recipe
            .rules
            .owners
            .iter()
            .position(|o| o.owner == owner.owner)
            .unwrap();
        let mut actual = restored.recipe.rules.owners.remove(index);
        actual.programs.members.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(actual, owner);
    }
    for table in &extension.tables {
        assert!(!before.recipe.rules.tables.iter().any(|t| t.id == table.id));
        let index = restored
            .recipe
            .rules
            .tables
            .iter()
            .position(|t| t.id == table.id)
            .unwrap();
        assert_eq!(restored.recipe.rules.tables.remove(index), *table);
    }
    assert!(extension.receivers.is_empty());
    assert_eq!(
        restored.recipe.rules.effect_applications,
        Some(read("applications.json"))
    );
    restored.recipe.rules.effect_applications = before.recipe.rules.effect_applications.clone();
    restored.recipe.rules.operations_version = before.recipe.rules.operations_version.clone();
    restored.recipe.rules.definitions = before.recipe.rules.definitions.clone();
    restored.recipe.routing.definitions = before.recipe.routing.definitions.clone();
    restored.mapping.registry = before.mapping.registry;
    restored.mapping.definitions = before.mapping.definitions.clone();
    restored.roles.mapping = before.roles.mapping;
    restored.roles.definitions = before.roles.definitions.clone();
    restored.rewards.mapping = before.rewards.mapping;
    restored.rewards.definitions = before.rewards.definitions.clone();
    restored.items.definitions = before.items.definitions.clone();
    restored.item_source.item_lines = before.item_source.item_lines;
    // Each path is a dependency commitment checked by the public constructors.
    // Every other policy field, recipe and completeness declaration must match.
    let mut normalization = json!(restored.normalization);
    let old_normalization = json!(before.normalization);
    for path in [
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/gem_inventory/definitions",
        "/gem_inventory/roles",
        "/gem_inventory/scalar_inputs",
        "/support_origin_order/roles",
        "/equipment_membership/definitions",
        "/equipment_membership/item_lines",
        "/equipment_membership/item_source",
        "/passive_socket_membership/definitions",
        "/passive_socket_membership/mapping",
        "/passive_socket_membership/item_lines",
        "/passive_socket_membership/item_source",
        "/passive_socket_membership/equipment",
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
        "/item_parameter_inputs/definitions",
        "/item_parameter_inputs/item_lines",
        "/item_parameter_inputs/item_source",
        "/configuration_reward_inventory/reward_policy",
    ] {
        *normalization
            .pointer_mut(path)
            .expect("known prior policy binding") =
            old_normalization.pointer(path).unwrap().clone();
    }
    restored.normalization = serde_json::from_value(normalization).unwrap();
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        before.tree.as_ref().unwrap().content
    );
    restored.tree = before.tree.clone();
    assert_eq!(restored.provenance.len(), before.provenance.len() + 2);
    restored.provenance.truncate(before.provenance.len());
    assert!(
        restored == *before,
        "only exact Offering declarations/programs and validated dependency commitments change"
    );
}
