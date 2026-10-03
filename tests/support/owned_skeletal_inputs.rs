//! Checked source-bound skeletal topology and physical-input disposition publication.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_normalize::{GemInventoryPolicy, PrimaryGemInputDisposition},
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
    root().join("data/owned/poe2/3887ae68/skeletal-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn bindings() -> Value {
    read("bindings.json")
}
pub fn dispositions() -> Vec<PrimaryGemInputDisposition> {
    read("dispositions.json")
}
fn catalog() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b = bindings();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let d = dispositions();
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert_eq!(json!(m.before), a["before"]);
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 38);
    assert_eq!(m.owners.len(), 15);
    assert_eq!(
        m.owners
            .iter()
            .map(|row| row.programs.members.len())
            .sum::<usize>(),
        9
    );
    assert!(m.owners.iter().all(|row| !row.programs.is_complete()));
    assert_eq!(d.len(), 3);
    assert_eq!(b["families"].as_array().unwrap().len(), 3);
    for field in ["before", "definitions", "registry", "catalog"] {
        assert_eq!(b[field], a[field]);
    }
    for (name, hash) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*hash, json!(format!("{:x}", Sha256::digest(bytes))));
    }
    assert!(
        !fs::read(data().join("authoring.json"))
            .unwrap()
            .contains(&b'\r')
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|candidate| candidate["path"] == pin["path"]
                    && candidate["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let catalog = catalog();
    let checked = poe_optimizer_data::skill_identities::SkillIdentityCatalog::new(
        serde_json::from_value(catalog.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        json!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                checked.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        ),
        a["catalog"]
    );
    for (index, (row, family)) in d.iter().zip(b["families"].as_array().unwrap()).enumerate() {
        let source = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| {
                source["game_id"] == row.physical.game_id
                    && source["variant_id"] == row.physical.variant_id
            })
            .unwrap();
        assert_eq!(family["physical_identity"], *source);
        assert_eq!(source["primary_effect_id"], row.physical.skill_id);
        assert_eq!(source["effect_list"], json!([row.physical.skill_id]));
        for field in [
            "declared_additional_effects",
            "constructed_additional_effects",
        ] {
            assert_eq!(
                source[field],
                json!([{"index":1,"id":family["missing_command"]}])
            );
        }
        assert_eq!(source["additional_effects"], json!([]));
        assert!(
            !catalog["skills"]
                .as_array()
                .unwrap()
                .iter()
                .any(|skill| skill["id"] == family["missing_command"])
        );
        let reference = json!(row.reference_action);
        for field in ["definitions", "roles", "catalog"] {
            assert_eq!(reference[field], a[field]);
        }
        for field in [
            "gem",
            "primary",
            "primary_supply",
            "entering_grant",
            "minion",
        ] {
            assert_eq!(reference[field], family[field]);
        }
        assert_eq!(reference["actions"], json!([family["first_child"]]));
        let stat_sets: Vec<_> = family["stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|set| json!({"source_index":set["source_index"],"stat_set":set["stat_set"]}))
            .collect();
        assert_eq!(reference["actions"][0]["stat_sets"], json!(stat_sets));
        assert_eq!(
            reference["actions"][0]["stat_sets"]
                .as_array()
                .unwrap()
                .len(),
            [2, 2, 1][index]
        );
        assert_eq!(
            family["stat_sets"].as_array().unwrap().len(),
            [2, 2, 1][index]
        );
        assert_eq!(reference["absent_action"], 1);
        assert_eq!(reference["minion"]["allow_absent"], true);
        assert_eq!(
            reference["actions"][0]["absent_stat_set"],
            family["stat_sets"][0]["stat_set"]
        );
        assert_eq!(json!(row.physical.corrupted), family["corrupted"]);
        assert_eq!(
            json!(row.physical.corruption_level),
            family["corruption_level"]
        );
        let fields: Vec<_> = json!(row.deferred_usage)
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value["field"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            fields,
            [
                "gem_count",
                "gem_global1",
                "gem_global2",
                "group_count",
                "group_full_dps"
            ]
        );
    }
    assert_eq!(a["registry_last_issued_before"], 0x3284);
    assert_eq!(a["registry_last_issued_after"], 0x32a1);
    assert_eq!(a["allocated_definitions"], 8);
    assert_eq!(a["allocated_slots"], 21);
    for field in ["new_mappings", "new_tables", "new_scalar_values"] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["usage_inventory"], "pending");
    assert_eq!(a["native_mechanics_coverage"], "partial");
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let proof = &a["source_validation"];
    assert_eq!(
        proof["status"], "passed",
        "Pending source proof cannot authorize publication"
    );
    let mut reports = Vec::new();
    for (path, size, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, proof[size].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), proof[hash]);
        reports.push(bytes);
    }
    assert!(
        reports[0] == reports[1],
        "complete JIT observations remain byte-identical"
    );
    let r: Value = serde_json::from_slice(&reports[0]).unwrap();
    for (source, author) in [
        ("manifest_sha256", "source_manifest_sha256"),
        ("catalog_digest", "catalog"),
        ("source_revision", "source_revision"),
    ] {
        assert_eq!(r[source], a[author]);
    }
    assert_eq!(r["files"], a["source_files"]);
    assert_eq!(r["files"].as_array().unwrap().len(), 13);
    for flag in [
        "business_wrappers",
        "native_inventory_authority",
        "native_build_parity",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(r[flag], false);
    }
    let b = bindings();
    let families = b["families"].as_array().unwrap();
    assert_eq!(r["physical_identities"].as_array().unwrap().len(), 3);
    for family in families {
        assert_eq!(
            r["physical_identities"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|identity| **identity == family["physical_identity"])
                .count(),
            1
        );
    }
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 62);
    assert_eq!(proof["cases"], 62);
    assert_eq!(proof["lifecycle_stages"], 3);
    assert_eq!(r["original_sources"].as_array().unwrap().len(), 5);
    for case in 1..=5 {
        let path = format!("tests/fixtures/builds/breadth-20260908/build-{case:02}.xml");
        let hash = format!(
            "{:x}",
            Sha256::digest(fs::read(root().join(&path)).unwrap())
        );
        assert_eq!(
            r["original_sources"][case - 1],
            json!({"path":path,"sha256":hash})
        );
        assert_eq!(cases[case - 1]["name"], format!("original-{case:02}"));
        assert_eq!(cases[case - 1]["xml_sha256"], hash);
    }
    let repeat = cases.last().unwrap();
    assert_eq!(repeat["name"], "repeat-original-05");
    assert!(cases[4]["states"] == repeat["states"]);
    assert_eq!(cases[4]["source_joins"], repeat["source_joins"]);
    let mut topology_seen = BTreeSet::new();
    for case in cases {
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &case["states"][stage];
            for flag in [
                "source_methods_preserved",
                "loader_observer_removed",
                "requested_jit_mode_verified",
                "exact_physical_objects",
                "physical_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(state[flag], true, "{} {stage} {flag}", case["name"]);
            }
            for row in state["saved"].as_array().into_iter().flatten() {
                let family = families
                    .iter()
                    .find(|family| {
                        family["physical_identity"]["game_id"] == row["attributes"]["gemId"]
                    })
                    .unwrap();
                assert_eq!(row["resolved_additional_count"], 0);
                for mode in ["MAIN", "CALCS"] {
                    for action in row[mode].as_array().into_iter().flatten() {
                        assert_eq!(
                            action["effect"],
                            family["physical_identity"]["primary_effect_id"]
                        );
                        assert_eq!(action["actor"]["type"], family["minion"]["source_id"]);
                        assert_eq!(action["exact_physical_object"], true);
                        let children = action["actor"]["children"].as_array().unwrap();
                        assert_eq!(children.len(), 2);
                        let child = children.iter().find(|child| child["index"] == 1).unwrap();
                        assert_eq!(child["effect"], family["first_child"]["skill_id"]);
                        assert_eq!(child["exact_actor"], true);
                        assert_eq!(child["exact_summon"], true);
                        let observed = child["stat_sets"].as_array().unwrap();
                        let authored = family["stat_sets"].as_array().unwrap();
                        assert_eq!(observed.len(), authored.len());
                        for set in authored {
                            let source = observed
                                .iter()
                                .find(|row| row["index"] == set["source_index"])
                                .unwrap();
                            assert_eq!(source["label"], set["label"]);
                            assert_eq!(
                                source["stat_description_scope"],
                                set["stat_description_scope"]
                            );
                        }
                        let second = children.iter().find(|child| child["index"] == 2).unwrap();
                        assert_eq!(second["effect"], family["unreviewed_child"]["skill_id"]);
                        topology_seen.insert(family["key"].as_str().unwrap());
                    }
                }
            }
        }
    }
    assert_eq!(topology_seen.len(), 3);
    for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
        for (family_index, family) in families.iter().enumerate() {
            let count: usize = [0, 4]
                .into_iter()
                .map(|case| {
                    cases[case]["states"][stage]["saved"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|row| {
                            row["attributes"]["gemId"] == family["physical_identity"]["game_id"]
                        })
                        .count()
                })
                .sum();
            assert_eq!(count, [6, 4, 5][family_index]);
        }
    }
}

fn restored_identity_fields(before: &Value, after: &Value, pointers: &[&str], label: &str) {
    let mut restored = after.clone();
    for path in pointers {
        let value = before
            .pointer(path)
            .unwrap_or_else(|| panic!("{label}: prior {path}"));
        *restored
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("{label}: new {path}")) = value.clone();
    }
    assert!(
        restored == *before,
        "all non-binding {label} content and order survive"
    );
}
fn preserve_migration(
    prior: &StagedOwnedRelease,
    migrated: &StagedOwnedRelease,
    migration: &OwnedReleaseMigrationInput,
) {
    let old = prior.input();
    let new = migrated.input();
    let old_json = json!(old);
    let new_json = json!(new);
    let mut expected = old.recipe.schema.clone();
    expected.release = migration.release.clone();
    for entry in &migration.schema {
        match entry {
            SchemaExtensionEntry::Definition(row) => {
                if let Some(existing) = expected
                    .definitions
                    .iter_mut()
                    .find(|candidate| candidate.address() == row.address())
                {
                    *existing = row.clone();
                } else {
                    expected.definitions.push(row.clone());
                }
            }
            SchemaExtensionEntry::Slot(row) => {
                if let Some(existing) = expected
                    .slots
                    .iter_mut()
                    .find(|candidate| candidate.address() == row.address())
                {
                    *existing = row.clone();
                } else {
                    expected.slots.push(row.clone());
                }
            }
        }
    }
    // Storage orders descriptors by typed address, not allocation chronology.
    // Every descriptor and its complete contents still compare exactly.
    expected
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    expected.slots.sort_by_cached_key(SlotDescriptor::address);
    assert!(
        expected == new.recipe.schema,
        "only the exact authored schema replacement/append"
    );
    for family in bindings()["families"].as_array().unwrap() {
        let id = &family["gem"];
        let before = old_json["recipe"]["schema"]["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["value"]["id"] == *id)
            .unwrap();
        let after = new_json["recipe"]["schema"]["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["value"]["id"] == *id)
            .unwrap();
        let mut expected = before.clone();
        expected["value"]["schema"]["value"]["declarations"]["grants"]["members"]
            .as_array_mut()
            .unwrap()
            .push(family["entering_grant"].clone());
        expected["value"]["schema"]["value"]["declarations"]["skill_grants"]["members"]
            .as_array_mut()
            .unwrap()
            .push(family["primary_supply"].clone());
        assert!(
            expected == *after,
            "all old Gem scalar/quality/closure fields survive"
        );
        for id in [&family["primary"], &family["first_child"]["skill"]] {
            assert_eq!(
                old_json["recipe"]["schema"]["definitions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["value"]["id"] == *id)
                    .unwrap()["value"]["schema"]["kind"],
                "unmapped"
            );
        }
    }
    assert_eq!(new.recipe.registry.last_issued.get(), 0x32a1);
    assert_eq!(
        new.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 29
    );
    assert!(
        new.recipe.registry.entries[..old.recipe.registry.entries.len()]
            == old.recipe.registry.entries
    );
    restored_identity_fields(
        &old_json["recipe"]["registry"],
        &new_json["recipe"]["registry"],
        &["/revision", "/last_issued", "/entries"],
        "registry metadata",
    );
    let mut rules = json!(old.recipe.rules);
    rules["definitions"] = json!(migrated.receipt().definitions);
    for row in &migration.owners {
        assert!(
            !old.recipe
                .rules
                .owners
                .iter()
                .any(|old| old.owner == row.owner)
        );
        rules["owners"].as_array_mut().unwrap().push(json!(row));
    }
    assert!(
        rules == json!(new.recipe.rules),
        "old rules/tables/receivers/applications remain exact"
    );
    restored_identity_fields(
        &old_json["recipe"]["routing"],
        &new_json["recipe"]["routing"],
        &["/definitions"],
        "routing",
    );
    restored_identity_fields(
        &old_json["mapping"],
        &new_json["mapping"],
        &["/definitions", "/registry"],
        "mapping",
    );
    restored_identity_fields(
        &old_json["roles"],
        &new_json["roles"],
        &["/definitions", "/mapping"],
        "roles",
    );
    restored_identity_fields(
        &old_json["rewards"],
        &new_json["rewards"],
        &["/definitions", "/mapping"],
        "rewards",
    );
    restored_identity_fields(
        &old_json["items"],
        &new_json["items"],
        &["/definitions"],
        "items",
    );
    restored_identity_fields(
        &old_json["item_source"],
        &new_json["item_source"],
        &["/item_lines"],
        "item source",
    );
    restored_identity_fields(
        &old_json["tree"],
        &new_json["tree"],
        &["/definitions", "/registry", "/mapping", "/normalization"],
        "tree",
    );
    let mut pointers = vec![
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/direct_skill_inputs/definitions",
        "/direct_skill_inputs/roles",
        "/gem_inventory/definitions",
        "/gem_inventory/roles",
        "/gem_inventory/scalar_inputs",
        "/gem_inventory/usage_inputs",
        "/usage_inputs/definitions",
        "/usage_inputs/roles",
        "/usage_inputs/scalar_inputs",
        "/support_origin_order/roles",
        "/payload_inventory/roles",
        "/configuration_reward_inventory/reward_policy",
        "/equipment_membership/definitions",
        "/equipment_membership/item_lines",
        "/equipment_membership/item_source",
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
        "/item_parameter_inputs/definitions",
        "/item_parameter_inputs/item_lines",
        "/item_parameter_inputs/item_source",
        "/passive_socket_membership/definitions",
        "/passive_socket_membership/mapping",
        "/passive_socket_membership/item_lines",
        "/passive_socket_membership/item_source",
        "/passive_socket_membership/equipment",
    ];
    let reference_pointers: Vec<_> = (0..2)
        .flat_map(|index| {
            ["definitions", "roles"].map(move |field| {
                format!("/gem_inventory/primary_dispositions/{index}/reference_action/{field}")
            })
        })
        .collect();
    pointers.extend(reference_pointers.iter().map(String::as_str));
    restored_identity_fields(
        &old_json["normalization"],
        &new_json["normalization"],
        &pointers,
        "normalization",
    );
    assert!(
        old.query_sets == new.query_sets,
        "all 110 query rows and source locators survive"
    );
    assert_eq!(old.schema_version, new.schema_version);
    assert_eq!(old.evaluation, new.evaluation);
    assert_eq!(new.provenance.len(), old.provenance.len() + 1);
    assert!(new.provenance[..old.provenance.len()] == old.provenance);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(receipt[field], a[field]);
    }
    let rows = dispositions();
    for row in &rows {
        assert_eq!(
            json!(row.reference_action)["source"],
            json!(prior.roles().input().compilation.source)
        );
    }
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    preserve_migration(prior, &migrated, &migration);
    let base = migrated.input();
    let mut normalization = base.normalization.clone();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        supports,
        primary_skills,
        primary_dispositions,
        ..
    }) = &mut normalization.gem_inventory
    else {
        panic!("exact V3 prior inventory required")
    };
    assert_eq!(supports.len(), 514);
    assert_eq!(primary_skills.len(), 2);
    assert_eq!(primary_dispositions.len(), 2);
    for row in rows {
        let mut value = json!(row);
        value["reference_action"]["definitions"] = json!(migrated.receipt().definitions);
        value["reference_action"]["roles"] = json!(migrated.receipt().roles);
        primary_dispositions.push(serde_json::from_value(value).unwrap());
    }
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: base.recipe.clone(),
            successor: base.recipe.clone(),
            mapping: base.mapping.clone(),
            roles: base.roles.clone(),
            normalization: base.normalization.clone(),
            rewards: base.rewards.clone(),
            query_sets: base.query_sets.clone(),
            items: base.items.clone(),
            item_source: base.item_source.clone(),
        },
        base.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = base.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert!(full.normalization == normalization);
    assert!(full.tree.as_ref().unwrap().content == base.tree.as_ref().unwrap().content);
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("skeletal-physical-inventories-with-deferred-usage"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-skeletal-inputs-v1",
            &(a, bindings(), dispositions()),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization = base.normalization.clone();
    restored.tree = base.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *base,
        "only disposition inventory, dependent tree binding and provenance change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
