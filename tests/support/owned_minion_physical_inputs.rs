//! Checked V4 physical input authoring; no source execution or inventory closure.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{
        DefinitionAddress, DefinitionDescriptor, SchemaState, SchemaSubject, SlotDescriptor,
    },
};
use poe_optimizer_data::skill_identities::SkillIdentityData;
use poe_optimizer_import::{
    owned_gem_schema::{GemSchemaMigrationInput, stage_owned_gem_schema},
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{
        GemInputPolicy, GemInventoryPolicy, UsageInputPolicy, gem_inventory_scalar_inputs_identity,
    },
    owned_release::{
        OwnedReleaseInput, OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
    },
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, StagedSuccessorBundle, SuccessorBundleInput,
        TreePolicyTransitionInput, transition_owned_catalog_with_gem_refinement_compact,
        transition_owned_normalization_with_tree_compact,
    },
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/minion-physical-inputs")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn check_authored() {
    let migration: GemSchemaMigrationInput = read("migration.json");
    let inputs: GemInputPolicy = read("inputs.json");
    assert_eq!(migration.gems.len(), 3);
    assert_eq!(migration.parameters.len(), 6);
    assert_eq!(inputs.gems.len(), 3);
    assert_eq!(inputs.definitions, migration.before);
    for (gem, rule) in migration.gems.iter().zip(&inputs.gems) {
        assert_eq!(gem.id, rule.gem);
        let SchemaState::Known(schema) = &gem.schema else {
            panic!("reviewed Known Gem input schema")
        };
        assert_eq!(
            (schema.level.minimum.get(), schema.level.maximum.get()),
            (1, 40)
        );
        assert_eq!(schema.skills.members.len(), 1);
        assert_eq!(schema.declarations.parameters.members.len(), 2);
        assert_eq!(rule.parameters.len(), 2);
        assert!(!schema.skills.is_complete());
        assert!(!schema.quality.allowed_kinds.is_complete());
        assert!(!schema.declarations.parameters.is_complete());
        assert!(!schema.declarations.choices.is_complete());
        assert!(!schema.declarations.grants.is_complete());
        assert!(!schema.declarations.actors.is_complete());
        assert!(!schema.declarations.skill_grants.is_complete());
        assert!(!schema.declarations.outputs.is_complete());
        assert!(!schema.declarations.sockets.is_complete());
        assert!(rule.parameters.iter().all(|parameter| {
            schema
                .declarations
                .parameters
                .members
                .contains(&parameter.slot)
        }));
    }
    let authoring: Value = read("authoring.json");
    for name in ["migration", "inputs", "bindings"] {
        let bytes = fs::read(
            root()
                .join("data/owned/poe2/3887ae68/minion-physical-inputs")
                .join(format!("{name}.json")),
        )
        .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            authoring["artifact_sha256"][name]
        );
    }
    let vectors_bytes = fs::read(
        root().join("data/owned/poe2/3887ae68/minion-physical-inputs/reference-vectors.json"),
    )
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&vectors_bytes)),
        authoring["source_validation"]["reference_vectors_sha256"]
    );
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    assert_eq!(
        authoring["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    for pin in authoring["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let catalog_bytes =
        fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&catalog_bytes)),
        authoring["catalog_sha256"]
    );
    let catalog_data: SkillIdentityData = serde_json::from_slice(&catalog_bytes).unwrap();
    assert_eq!(
        json!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                &catalog_data,
                64 * 1024 * 1024
            )
            .unwrap()
        ),
        authoring["catalog_digest"]
    );
    let catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    let bindings: Value = read("bindings.json");
    let vectors: Value = serde_json::from_slice(&vectors_bytes).unwrap();
    for binding in bindings["gems"]
        .as_array()
        .unwrap()
        .iter()
        .chain(std::iter::once(&bindings["sniper"]))
    {
        let rows: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["game_id"] == binding["game_id"] && row["variant_id"] == binding["variant_id"]
            })
            .collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row["primary_effect_id"], binding["primary_effect_id"]);
        assert_eq!(row["additional_effects"], json!([]));
        assert_eq!(row["effect_list"], json!([binding["primary_effect_id"]]));
        assert_eq!(
            row["declared_additional_effects"],
            json!([{"id":binding["missing_command"],"index":1}])
        );
        assert_eq!(
            row["constructed_additional_effects"],
            row["declared_additional_effects"]
        );
        let unresolved: Vec<_> = catalog["missing_references"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|value| value["gem_key"] == row["key"])
            .collect();
        assert_eq!(unresolved.len(), 2);
        assert!(
            unresolved
                .iter()
                .all(|value| value["effect_id"] == binding["missing_command"])
        );
        let observed = vectors["catalog"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["game_id"] == binding["game_id"])
            .unwrap();
        assert_eq!(observed["variant_id"], binding["variant_id"]);
        assert_eq!(observed["primary"], binding["primary_effect_id"]);
        assert_eq!(
            observed["unresolved_references"],
            json!([{"id":binding["missing_command"],"in_effect_list":false,"index":1,"standalone_skill":false}])
        );
    }
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut observations = vec![];
    for (path, hash, size) in [
        ("evidence_json", "evidence_sha256", "evidence_bytes"),
        (
            "evidence_on_json",
            "evidence_on_sha256",
            "evidence_on_bytes",
        ),
    ] {
        let bytes = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, proof[size].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), proof[hash]);
        let observed: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            observed.get("elapsed_ms").is_none(),
            "published source evidence excludes elapsed time"
        );
        observations.push(observed);
    }
    assert!(
        observations[0] == observations[1],
        "all authenticated source observations match across JIT modes"
    );
    let report = &observations[0];
    let evidence = &report["evidence"];
    assert_eq!(report["source_hash"], authoring["source_manifest_sha256"]);
    assert_eq!(
        evidence["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(evidence["files"], authoring["source_files"]);
    for name in ["catalog_sha256", "catalog_digest", "xml_sha256"] {
        assert_eq!(evidence[name], authoring[name]);
    }
    let original =
        fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&original)),
        evidence["xml_sha256"]
    );
    for name in [
        "full_build_numeric_parity",
        "minion_action_inventory_complete",
        "missing_commands_resolved",
        "native_parity",
        "physical_input_inventory_complete",
    ] {
        assert_eq!(evidence[name], false);
    }
    let cases = report["additional_observation"]["load_cases"]
        .as_array()
        .unwrap();
    assert_eq!(cases.len(), 140);
    assert_eq!(cases.iter().filter(|row| row["ok"] == true).count(), 128);
    let projection: Vec<_> = cases
        .iter()
        .map(|row| {
            let mut projected = serde_json::Map::new();
            for name in [
                "id",
                "label",
                "attributes",
                "ok",
                "physical_gems",
                "published_groups",
                "error",
            ] {
                if let Some(value) = row.get(name) {
                    projected.insert(name.into(), value.clone());
                }
            }
            for name in ["after", "reprocessed"] {
                if let Some(value) = row.get(name) {
                    let mut scalars = serde_json::Map::new();
                    for field in ["level", "quality", "corrupted", "corrupt_level"] {
                        if let Some(value) = value.get(field) {
                            scalars.insert(field.into(), value.clone());
                        }
                    }
                    projected.insert(name.into(), Value::Object(scalars));
                }
            }
            Value::Object(projected)
        })
        .collect();
    let expected = json!({"schema_version":1,"source_revision":authoring["source_revision"],"source_hash":report["source_hash"],
        "xml_sha256":evidence["xml_sha256"],"catalog_sha256":evidence["catalog_sha256"],"catalog_digest":evidence["catalog_digest"],
        "catalog":report["additional_observation"]["catalog"],"cases":projection});
    assert!(
        read::<Value>("reference-vectors.json") == expected,
        "compact vectors must reproduce every measured scalar and absent field exactly"
    );
}
fn bundle(input: &OwnedReleaseInput) -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: 1,
        prior: input.recipe.clone(),
        successor: input.recipe.clone(),
        mapping: input.mapping.clone(),
        roles: input.roles.clone(),
        normalization: input.normalization.clone(),
        rewards: input.rewards.clone(),
        query_sets: input.query_sets.clone(),
        items: input.items.clone(),
        item_source: input.item_source.clone(),
    }
}
fn carry(full: &mut OwnedReleaseInput, carried: &StagedSuccessorBundle) {
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|tree| tree.input().clone());
    assert!(
        full.query_sets == carried.query_sets(),
        "queries preserved by checked carry-forward"
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    source_proof(&authoring);
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    let receipt = json!(prior.receipt());
    for name in [
        "definitions",
        "registry",
        "roles",
        "mapping",
        "normalization",
    ] {
        assert_eq!(authoring[name], receipt[name]);
    }
    let b = prior.input();
    assert!(b.evaluation.is_none());
    let migration: GemSchemaMigrationInput = read("migration.json");
    let bindings: Value = read("bindings.json");
    for binding in bindings["gems"]
        .as_array()
        .unwrap()
        .iter()
        .chain(std::iter::once(&bindings["sniper"]))
    {
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(binding["game_id"].as_str().unwrap().into()),
            variant_id: SourceComponent::Text(binding["variant_id"].as_str().unwrap().into()),
        });
        assert!(
            matches!(prior.mapping().lookup(&selector), Some(MappingOutcome::Mapped { target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)), basis: MappingBasis::Exact }) if json!(gem) == binding["gem"])
        );
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(binding["primary_effect_id"].as_str().unwrap().into()),
        });
        assert!(
            matches!(prior.mapping().lookup(&selector), Some(MappingOutcome::Mapped { target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)), basis: MappingBasis::Exact }) if json!(skill) == binding["primary"])
        );
    }
    let staged = stage_owned_gem_schema(
        prior.assembled(),
        prior.mapping(),
        prior.roles(),
        &migration,
        Default::default(),
    )
    .unwrap();
    assert_eq!(staged.receipt.promoted_gems, 3);
    assert_eq!(staged.receipt.allocated_parameters, 6);
    let mut next_bundle = bundle(b);
    next_bundle.successor = staged.successor;
    let carried = transition_owned_catalog_with_gem_refinement_compact(
        next_bundle,
        CatalogAppend {
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        staged.refinement,
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    carry(&mut full, &carried);
    let additions: GemInputPolicy = read("inputs.json");
    assert_eq!(
        &additions.definitions,
        prior.assembled().schema().identity()
    );
    let mut normalization = full.normalization.clone();
    let rules = &mut normalization.gem_inputs.as_mut().unwrap().gems;
    for rule in &additions.gems {
        assert!(!rules.iter().any(|old| old.gem == rule.gem));
        rules.push(rule.clone());
    }
    normalization.version = key("minion-physical-inputs-v1");
    let scalar = gem_inventory_scalar_inputs_identity(&normalization, Default::default()).unwrap();
    if let Some(GemInventoryPolicy::PobFreshSingleSupportV1 { scalar_inputs, .. }) =
        &mut normalization.gem_inventory
    {
        *scalar_inputs = scalar;
    }
    if let Some(UsageInputPolicy::PobOccurrenceUsageV3 { scalar_inputs, .. }) =
        &mut normalization.usage_inputs
    {
        *scalar_inputs = scalar;
    }
    let normalized = transition_owned_normalization_with_tree_compact(
        bundle(&full),
        full.tree.clone().unwrap(),
        normalization,
        Default::default(),
    )
    .unwrap();
    carry(&mut full, &normalized);
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("minion-physical-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-minion-physical-inputs-authoring-v1",
            &(
                authoring,
                &migration,
                &additions,
                read::<Value>("bindings.json"),
                read::<Value>("reference-vectors.json"),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    let n = next.input();
    assert!(
        n.query_sets == b.query_sets,
        "all original query sets preserved"
    );
    assert!(
        n.recipe.registry.entries[..b.recipe.registry.entries.len()] == b.recipe.registry.entries,
        "all prior registry entries preserved"
    );
    assert_eq!(
        n.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 6
    );
    let mut restored = n.recipe.schema.clone();
    for replacement in &migration.gems {
        let at = restored
            .definitions
            .iter()
            .position(|row| {
                row.address() == DefinitionDescriptor::Gem(replacement.clone()).address()
            })
            .unwrap();
        assert!(restored.definitions[at] == DefinitionDescriptor::Gem(replacement.clone()));
        restored.definitions[at] = b
            .recipe
            .schema
            .definitions
            .iter()
            .find(|row| row.address() == restored.definitions[at].address())
            .unwrap()
            .clone();
    }
    for addition in &migration.parameters {
        let expected = SlotDescriptor::Parameter(addition.clone());
        let at = restored
            .slots
            .iter()
            .position(|row| row.address() == expected.address())
            .unwrap();
        assert!(restored.slots.remove(at) == expected);
    }
    assert!(
        restored == b.recipe.schema,
        "only three Gem schemas and six scalar slots changed; Sniper and every other descriptor remain exact"
    );
    let mut rules = n.recipe.rules.clone();
    rules.definitions = b.recipe.rules.definitions.clone();
    assert!(
        rules == b.recipe.rules,
        "all programs, tables, applications, and operations remain exact"
    );
    let mut routing = n.recipe.routing.clone();
    routing.definitions = b.recipe.routing.definitions.clone();
    assert!(routing == b.recipe.routing, "routing remains exact");
    let mut mapping = n.mapping.clone();
    mapping.definitions = b.mapping.definitions.clone();
    mapping.registry = b.mapping.registry;
    assert!(mapping == b.mapping, "mapping content/source unchanged");
    let mut roles = n.roles.clone();
    roles.definitions = b.roles.definitions.clone();
    roles.mapping = b.roles.mapping;
    assert!(
        roles == b.roles,
        "all roles and unresolved Command evidence unchanged"
    );
    assert!(
        n.tree.as_ref().unwrap().content == b.tree.as_ref().unwrap().content,
        "tree content unchanged"
    );
    let mut restored_policy = n.normalization.clone();
    restored_policy
        .gem_inputs
        .as_mut()
        .unwrap()
        .gems
        .retain(|row| !additions.gems.iter().any(|added| added.gem == row.gem));
    restored_policy.version = carried.normalization().version.clone();
    if let (
        Some(GemInventoryPolicy::PobFreshSingleSupportV1 { scalar_inputs, .. }),
        Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
            scalar_inputs: old, ..
        }),
    ) = (
        &mut restored_policy.gem_inventory,
        &carried.normalization().gem_inventory,
    ) {
        *scalar_inputs = *old;
    }
    if let (
        Some(UsageInputPolicy::PobOccurrenceUsageV3 { scalar_inputs, .. }),
        Some(UsageInputPolicy::PobOccurrenceUsageV3 {
            scalar_inputs: old, ..
        }),
    ) = (
        &mut restored_policy.usage_inputs,
        &carried.normalization().usage_inputs,
    ) {
        *scalar_inputs = *old;
    }
    assert!(
        restored_policy == *carried.normalization(),
        "only three ordinary scalar rules and their checked commitments added"
    );
    let mut rewards = n.rewards.clone();
    rewards.definitions = b.rewards.definitions.clone();
    rewards.mapping = b.rewards.mapping;
    assert!(
        rewards == b.rewards,
        "reward source rules and order unchanged"
    );
    let mut items = n.items.clone();
    items.definitions = b.items.definitions.clone();
    assert!(items == b.items, "item rules and order unchanged");
    let mut item_source = n.item_source.clone();
    item_source.item_lines = b.item_source.item_lines;
    assert!(
        item_source == b.item_source,
        "item source, attribution layouts, and order unchanged"
    );
    next
}
