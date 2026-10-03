//! Frost Bomb physical inputs and exact requested global-effect preference.
//! A stored preference does not prove cold source action-presence parity.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SchemaState, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, PrimarySkillGemInventory, PrimarySkillUsageInput, UsageInputPolicy,
        usage_inputs_identity,
    },
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/frost-bomb-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

pub fn check_authored() {
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let usage: PrimarySkillUsageInput = read("usage.json");
    let inventory: PrimarySkillGemInventory = read("inventory.json");
    let native: Value = read("native-inputs.json");
    let authoring: Value = read("authoring.json");
    assert_eq!(migration.schema_version, 3);
    assert_eq!(migration.contract.schema_version, 5);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert_eq!(migration.schema.len(), 7);
    assert_eq!(migration.owners.len(), 2);
    assert_eq!(
        migration
            .owners
            .iter()
            .map(|owner| owner.programs.members.len())
            .sum::<usize>(),
        2
    );
    assert!(migration.tables.is_empty());
    assert!(migration.receivers.is_empty());
    assert!(migration.query_targets.is_empty());
    assert!(migration.evaluation.is_none());
    let skill = migration
        .schema
        .iter()
        .find_map(|entry| match entry {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(row))
                if row.id == usage.primary =>
            {
                Some(&row.schema)
            }
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(skill) = skill else {
        panic!("explicit primary Skill schema")
    };
    assert!(!skill.directly_selectable);
    for declaration in [
        skill.declarations.parameters.is_complete(),
        skill.declarations.choices.is_complete(),
        skill.declarations.grants.is_complete(),
        skill.declarations.actors.is_complete(),
        skill.declarations.skill_grants.is_complete(),
        skill.declarations.outputs.is_complete(),
        skill.declarations.sockets.is_complete(),
    ] {
        assert!(
            !declaration,
            "input storage does not prove mechanics coverage"
        );
    }
    assert!(skill.declarations.parameters.members.is_empty());
    let gem = migration
        .schema
        .iter()
        .find_map(|entry| match entry {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Gem(row))
                if row.id == usage.gem =>
            {
                Some(&row.schema)
            }
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(gem) = gem else {
        panic!("existing physical Gem schema")
    };
    for complete in [
        gem.skills.is_complete(),
        gem.declarations.parameters.is_complete(),
        gem.declarations.choices.is_complete(),
        gem.declarations.grants.is_complete(),
        gem.declarations.actors.is_complete(),
        gem.declarations.skill_grants.is_complete(),
        gem.declarations.outputs.is_complete(),
        gem.declarations.sockets.is_complete(),
    ] {
        assert!(!complete, "physical storage does not close Gem mechanics");
    }
    let supply = migration
        .schema
        .iter()
        .find_map(|entry| match entry {
            SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(row))
                if row.id == usage.supply =>
            {
                Some(&row.schema)
            }
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(supply) = supply else {
        panic!("explicit primary supply")
    };
    assert!(!supply.outputs.is_complete());
    let gem_owner = migration
        .owners
        .iter()
        .find(|owner| {
            json!(owner.owner)
                == json!({"kind":"definition", "value":{"kind":"gem", "value":usage.gem}})
        })
        .unwrap();
    assert!(!gem_owner.programs.is_complete());
    assert_eq!(usage.parameters.len(), 1);
    assert_eq!(inventory.physical.gem, usage.gem);
    assert_eq!(inventory.physical.skill_id, usage.skill_id);
    assert_eq!(inventory.usage_policy, usage.policy);
    for (field, expected) in [
        ("policy", json!(usage.policy)),
        ("parameter", json!(usage.parameters[0].slot)),
        ("gem", json!(usage.gem)),
        ("skill", json!(usage.primary)),
        ("primary_supply", json!(usage.supply)),
        ("entering_grant", json!(usage.grant)),
    ] {
        assert_eq!(native[field], expected, "native binding {field}");
    }
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        authoring["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
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
    for name in ["migration", "usage", "inventory", "native-inputs"] {
        assert_eq!(
            authoring["artifact_sha256"][name],
            format!(
                "{:x}",
                Sha256::digest(fs::read(data().join(format!("{name}.json"))).unwrap())
            )
        );
    }
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap();
    let matching: Vec<_> = catalog["gems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["game_id"] == usage.game_id && row["variant_id"] == usage.variant_id)
        .collect();
    assert_eq!(matching.len(), 1);
    let gem = matching[0];
    assert_eq!(gem["primary_effect_id"], usage.skill_id);
    assert_eq!(gem["effect_list"], json!([usage.skill_id]));
    for field in [
        "declared_additional_effects",
        "declared_additional_stat_sets",
        "constructed_additional_effects",
        "additional_effects",
    ] {
        assert_eq!(gem[field], json!([]), "finite singleton source domain");
    }
    assert_eq!(authoring["new_definitions"], 5);
    assert_eq!(authoring["new_programs"], 2);
    assert_eq!(authoring["new_tables"], 0);
    assert_eq!(authoring["usage_inventory"], "pending");
    assert_eq!(
        authoring["usage_semantics"],
        "requested-global-effect-setting"
    );
    assert_eq!(authoring["native_mechanics_coverage"], "partial");
}

fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut reports = Vec::new();
    for (path, bytes, sha) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes_read = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes_read.len() as u64, proof[bytes].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes_read)), proof[sha]);
        reports.push(bytes_read);
    }
    assert!(
        reports[0] == reports[1],
        "complete reports agree across JIT modes"
    );
    let evidence: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(evidence["source_revision"], authoring["source_revision"]);
    assert_eq!(evidence["catalog_digest"], authoring["catalog"]);
    assert_eq!(
        evidence["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(evidence["native_inventory_authority"], false);
    assert_eq!(evidence["native_build_parity"], false);
    assert_eq!(evidence["exposure_arithmetic_claimed"], false);
    assert_eq!(evidence["phase_independent_activity_claimed"], false);
    assert_eq!(
        evidence["lifecycle_stages"],
        json!([
            "fresh_complete_load",
            "passive_original_frame",
            "requested_original_frame_rebuild_1",
            "requested_original_frame_rebuild_2"
        ])
    );
    assert_eq!(evidence["count_domain"], json!([1]));
    for pin in authoring["source_files"].as_array().unwrap() {
        assert_eq!(
            evidence["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| { row["path"] == pin["path"] && row["sha256"] == pin["sha256"] })
                .count(),
            1
        );
    }
    let expected = [
        "original-01",
        "original-02",
        "original-03",
        "original-04",
        "original-05",
        "global-1-false",
        "global-2-false",
        "disabled-gem",
        "disabled-group",
        "duplicate-first-active",
        "duplicate-second-active",
        "archived-only",
        "global-1-missing",
        "global-1-malformed",
        "repeat-original-05",
    ];
    let cases = evidence["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected.len());
    for (case, expected) in cases.iter().zip(expected) {
        assert_eq!(case["name"], expected);
    }
    for (index, case) in cases.iter().take(5).enumerate() {
        let original = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            index + 1
        )))
        .unwrap();
        assert_eq!(
            case["xml_sha256"],
            format!("{:x}", Sha256::digest(original))
        );
    }
    for case in cases {
        for stage in ["state", "passive_frame", "rebuilt_once", "rebuilt_twice"] {
            assert!(case[stage].is_object(), "complete lifecycle stage {stage}");
        }
    }
    for field in [
        "xml_sha256",
        "source_identity",
        "source_joins",
        "state",
        "passive_frame",
        "rebuilt_once",
        "rebuilt_twice",
    ] {
        assert_eq!(
            cases[4][field], cases[14][field],
            "repeated original {field}"
        );
    }
}

fn native_dependencies(prior: &StagedOwnedRelease) {
    let snapshot: Value = serde_json::from_slice(
        &fs::read(root().join(
            "crates/poe-optimizer-engine/tests/support/frost_bomb_physical_dependencies.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let identity = json!(prior.receipt().definitions);
    assert_eq!(snapshot["source"]["release"], identity["release"]);
    assert_eq!(
        snapshot["source"]["schema_sha256"],
        identity["content_sha256"]
    );
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(snapshot["definitions"].clone()).unwrap();
    let slots: Vec<SlotDescriptor> = serde_json::from_value(snapshot["slots"].clone()).unwrap();
    assert!(!definitions.is_empty() && !slots.is_empty());
    let mut seen = BTreeSet::new();
    for row in &definitions {
        assert!(seen.insert(row.address()));
        assert!(
            prior.input().recipe.schema.definitions.contains(row),
            "native physical dependency must be an exact predecessor definition"
        );
    }
    let mut seen = BTreeSet::new();
    for row in &slots {
        assert!(seen.insert(row.address()));
        assert!(
            prior.input().recipe.schema.slots.contains(row),
            "native physical dependency must be an exact predecessor slot"
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    source_proof(&authoring);
    let receipt = json!(prior.receipt());
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(
            authoring[field], receipt[field],
            "exact predecessor {field}"
        );
    }
    assert_eq!(authoring["before"], receipt["input"]);
    native_dependencies(prior);
    assert_eq!(
        authoring["catalog"],
        json!(prior.roles().input().compilation.catalog_digest)
    );
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    let b = migrated.input();
    let mut normalization = b.normalization.clone();
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 { gems, .. } =
        normalization.usage_inputs.as_mut().unwrap();
    let usage: PrimarySkillUsageInput = read("usage.json");
    assert!(!gems.iter().any(|row| row.gem == usage.gem));
    gems.push(usage);
    let usage_identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV2 {
        primary_skills,
        usage_inputs,
        ..
    }) = &mut normalization.gem_inventory
    else {
        panic!("exact predecessor physical inventory policy")
    };
    let inventory: PrimarySkillGemInventory = read("inventory.json");
    assert!(
        !primary_skills
            .iter()
            .any(|row| row.physical.gem == inventory.physical.gem)
    );
    primary_skills.push(inventory);
    *usage_inputs = usage_identity;
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: b.recipe.clone(),
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        b.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("frost-bomb-physical-and-primary-usage-inputs"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-frost-bomb-input-authoring-v1",
            &(
                authoring,
                read::<Value>("usage.json"),
                read::<Value>("inventory.json"),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().query_sets == prior.input().query_sets);
    assert!(next.evaluation().is_none());
    let mut restored = next.input().clone();
    restored.normalization = b.normalization.clone();
    restored.tree = b.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *b,
        "only authored policy rows and their commitments change"
    );
    assert_eq!(
        next.input().recipe.registry.entries[..prior.input().recipe.registry.entries.len()],
        prior.input().recipe.registry.entries
    );
    assert_eq!(
        next.input().recipe.registry.entries.len(),
        prior.input().recipe.registry.entries.len() + 5
    );
    assert_eq!(
        next.input().recipe.rules.effect_applications,
        prior.input().recipe.rules.effect_applications
    );
    next
}
