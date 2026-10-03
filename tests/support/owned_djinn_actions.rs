//! Checked manual Direct Djinn topology and input-disposition publication.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_normalize::{DirectSkillInputDisposition, DirectSkillInputPolicy},
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
    root().join("data/owned/poe2/3887ae68/djinn-actions")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn bindings() -> Value {
    read("bindings.json")
}
pub fn dispositions() -> Vec<DirectSkillInputDisposition> {
    read("dispositions.json")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b = bindings();
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let rows = dispositions();
    assert_eq!(migration.schema_version, 3);
    assert_eq!(migration.contract.schema_version, 5);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert_eq!(json!(migration.before), a["before"]);
    assert!(
        migration.tables.is_empty()
            && migration.receivers.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none()
    );
    assert_eq!(migration.schema.len(), 60);
    assert_eq!(migration.owners.len(), 24);
    assert_eq!(
        migration
            .owners
            .iter()
            .map(|o| o.programs.members.len())
            .sum::<usize>(),
        12
    );
    assert!(migration.owners.iter().all(|o| !o.programs.is_complete()));
    let hash_fields: Vec<_> = a["artifact_sha256"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(hash_fields, ["bindings", "dispositions", "migration"]);
    for name in hash_fields {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(
            a["artifact_sha256"][name],
            format!("{:x}", Sha256::digest(bytes))
        );
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
        assert_eq!(pin.as_object().unwrap().len(), 2);
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
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap();
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
    for field in ["before", "definitions", "registry", "roles", "catalog"] {
        assert_eq!(b[field], a[field]);
    }
    let families = b["families"].as_array().unwrap();
    assert_eq!(families.len(), 2);
    assert_eq!(rows.len(), 2);
    for (index, (family, row)) in families.iter().zip(&rows).enumerate() {
        let identity = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["key"] == family["catalog_identity"]["key"])
            .unwrap();
        assert_eq!(*identity, family["catalog_identity"]);
        assert_eq!(identity["primary_effect_id"], family["skill_id"]);
        assert_eq!(
            identity["effect_list"],
            json!([family["skill_id"], family["command"]["skill_id"]])
        );
        assert_eq!(
            identity["additional_effects"],
            json!([family["command"]["skill_id"]])
        );
        assert_eq!(json!(row.skill), family["skill"]);
        let reference = json!(row.reference_action);
        assert_eq!(
            reference["kind"],
            "pob_manual_direct_singleton_minion_actions_v1"
        );
        for field in ["definitions", "roles", "catalog"] {
            assert_eq!(reference[field], a[field]);
        }
        for field in [
            "catalog_gem",
            "game_id",
            "variant_id",
            "skill_id",
            "name_spec",
            "skill",
            "minion",
            "actions",
        ] {
            assert_eq!(reference[field], family[field]);
        }
        assert_eq!(
            reference["manual_sources"],
            json!([{"kind":"missing"},{"kind":"text","value":""}])
        );
        assert_eq!(reference["absent_action"], 1);
        assert_eq!(reference["minion"]["allow_absent"], true);
        assert_eq!(family["actions"].as_array().unwrap().len(), [3, 5][index]);
        let actions =
            std::iter::once(&family["command"]).chain(family["actions"].as_array().unwrap());
        for action in actions {
            let metadata: Vec<_> = family["action_metadata"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["skill_id"] == action["skill_id"])
                .collect();
            assert_eq!(metadata.len(), 1);
            let sets: Vec<_> = metadata[0]["stat_sets"]
                .as_array()
                .unwrap()
                .iter()
                .map(|set| json!({"source_index":set["source_index"],"stat_set":set["stat_set"]}))
                .collect();
            assert_eq!(action["stat_sets"], json!(sets));
            assert_eq!(
                action["absent_stat_set"],
                action["stat_sets"][0]["stat_set"]
            );
        }
        assert_eq!(
            json!(row.inert_fields),
            json!([
                {"attribute":"corrupted","allowed":[{"kind":"text","value":"false"},{"kind":"text","value":"nil"}]},
                {"attribute":"corruptLevel","allowed":[{"kind":"text","value":"0"}]}
            ])
        );
        assert_eq!(
            json!(row.group_guards),
            json!([{"attribute":"label","allowed":[{"kind":"missing"},{"kind":"text","value":""}]}])
        );
    }
    assert_eq!(a["registry_last_issued_before"], 0x32a1);
    assert_eq!(a["registry_last_issued_after"], 0x32d1);
    assert_eq!(a["allocated_definitions"], 14);
    assert_eq!(a["allocated_slots"], 34);
    for field in ["new_mappings", "new_tables", "new_scalar_values"] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["direct_input_inventory"], "reviewed_intrinsic_only");
    for field in ["usage_inventory", "support_targets"] {
        assert_eq!(a[field], "pending");
    }
    assert_eq!(a["native_mechanics_coverage"], "partial");
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let proof = &a["source_validation"];
    assert_eq!(
        proof["status"], "passed",
        "Pending source evidence cannot authorize publication"
    );
    let mut reports = Vec::new();
    for (path, bytes, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let raw = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(raw.len() as u64, proof[bytes].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&raw)), proof[hash]);
        reports.push(raw);
    }
    assert!(
        reports[0] == reports[1],
        "complete observations agree in both JIT modes"
    );
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    for (source, author) in [
        ("manifest_sha256", "source_manifest_sha256"),
        ("source_revision", "source_revision"),
        ("catalog_digest", "catalog"),
    ] {
        assert_eq!(report[source], a[author]);
    }
    assert_eq!(report["files"], a["source_files"]);
    for field in [
        "business_wrappers",
        "native_inventory_authority",
        "native_build_parity",
        "native_supported_property_authority",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(report[field], false);
    }
    assert_eq!(report["original_sources"].as_array().unwrap().len(), 5);
    for index in 1..=5 {
        let path = format!("tests/fixtures/builds/breadth-20260908/build-{index:02}.xml");
        let sha256 = format!(
            "{:x}",
            Sha256::digest(fs::read(root().join(&path)).unwrap())
        );
        assert_eq!(
            report["original_sources"][index - 1],
            json!({"path":path,"sha256":sha256})
        );
    }
    let b = bindings();
    let families = b["families"].as_array().unwrap();
    assert_eq!(report["source_identities"].as_array().unwrap().len(), 2);
    for family in families {
        assert_eq!(
            report["source_identities"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| **row == family["catalog_identity"])
                .count(),
            1
        );
    }
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, proof["cases"].as_u64().unwrap());
    let mut children_seen = BTreeSet::new();
    let mut commands_seen = BTreeSet::new();
    let expected_failures: BTreeSet<_> = [
        (
            "sand-malformed-map-key",
            "SkillsTab.LoadSkill:malformed-map-key",
        ),
        (
            "water-malformed-map-key",
            "SkillsTab.LoadSkill:malformed-map-key",
        ),
        (
            "sand-invalid-statset",
            "CalcTools.buildSkillInstanceStats:unavailable-stat-set",
        ),
        (
            "water-invalid-statset",
            "CalcTools.buildSkillInstanceStats:unavailable-stat-set",
        ),
    ]
    .into_iter()
    .collect();
    let mut failures = BTreeSet::new();
    let mut names = BTreeSet::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name), "one exact result per source case");
        let hash = case["xml_sha256"].as_str().unwrap();
        assert!(hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()));
        if case["status"] == "source_failure" {
            assert!(case["states"].is_null());
            assert!(case["source_identity"].is_null());
            assert!(case["source_joins"].is_null());
            assert_eq!(case["native_inventory_authority"], false);
            let failure = (name, case["source_error_site"].as_str().unwrap());
            assert!(
                expected_failures.contains(&failure),
                "only an actually classified original source error is admitted"
            );
            assert!(failures.insert(failure));
            continue;
        }
        assert_eq!(case["status"], "complete");
        assert!(case["source_error_site"].is_null());
        assert_eq!(case["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(case["source_identity"]["source_sha256"], case["xml_sha256"]);
        assert!(case["source_identity"]["source_bytes"].as_u64().unwrap() > 0);
        assert_eq!(case["source_identity"]["instance_import_schema"], 1);
        let joins = case["source_joins"].as_array().unwrap();
        let mut joined = BTreeSet::new();
        for join in joins {
            assert_eq!(join["source"]["source_sha256"], case["xml_sha256"]);
            assert_eq!(join["source"]["ordinal"], join["source_ordinal"]);
            assert!(joined.insert(join["source_ordinal"].as_u64().unwrap()));
        }
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &case["states"][stage];
            for field in [
                "source_methods_preserved",
                "loader_observer_removed",
                "requested_jit_mode_verified",
                "exact_source_objects",
                "source_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(state[field], true, "{} {stage} {field}", case["name"]);
            }
            let saved = source_rows(&state["saved"]);
            assert_eq!(saved.len(), joins.len());
            for row in saved {
                let join = joins
                    .iter()
                    .find(|j| j["source_ordinal"] == row["source_ordinal"])
                    .unwrap();
                assert_eq!(join["preset"], row["preset"]);
                assert_eq!(join["group_source_ordinal"], row["group_source_ordinal"]);
                assert_eq!(row["source_kind"], "manual_direct");
                let family = families
                    .iter()
                    .find(|family| family["game_id"] == row["attributes"]["gemId"])
                    .unwrap();
                assert_eq!(row["resolved_additional_count"], 1);
                for mode in ["MAIN", "CALCS"] {
                    for action in source_rows(&row[mode]) {
                        assert_eq!(action["effect"], family["skill_id"]);
                        assert_eq!(action["exact_source_object"], true);
                        assert_eq!(action["actor"]["type"], family["minion"]["source_id"]);
                        let children = action["actor"]["children"].as_array().unwrap();
                        let mappings = family["actions"].as_array().unwrap();
                        assert_eq!(children.len(), mappings.len());
                        for mapping in mappings {
                            let child = children
                                .iter()
                                .find(|c| c["index"] == mapping["source_index"])
                                .unwrap();
                            assert_eq!(child["effect"], mapping["skill_id"]);
                            assert_eq!(child["exact_actor"], true);
                            assert_eq!(child["exact_summon"], true);
                            assert_sets(family, &mapping["skill_id"], &child["stat_sets"]);
                            children_seen.insert(mapping["skill_id"].as_str().unwrap());
                        }
                    }
                }
            }
            let direct = &state["direct"];
            for field in [
                "exact_manual_source_joins",
                "allocated_sources_are_separate",
            ] {
                assert_eq!(direct[field], true);
            }
            for field in [
                "native_source_membership_authority",
                "native_supported_property_authority",
            ] {
                assert_eq!(direct[field], false);
            }
            for group in source_rows(&direct["runtime_groups"]) {
                for source in group["sources"].as_array().unwrap() {
                    let effects = source["effects"].as_array().unwrap();
                    assert_eq!(effects.len(), 2);
                    let family = families
                        .iter()
                        .find(|family| family["skill_id"] == effects[0]["id"])
                        .unwrap();
                    assert_eq!(source["catalog_key"], family["catalog_identity"]["key"]);
                    assert_eq!(source["game_id"], family["game_id"]);
                    assert_eq!(source["from_tree"], true);
                    assert_eq!(effects[0]["index"], 1);
                    assert_eq!(effects[1]["index"], 2);
                    assert_eq!(effects[1]["id"], family["command"]["skill_id"]);
                    assert_sets(family, &effects[1]["id"], &effects[1]["stat_sets"]);
                    commands_seen.insert(effects[1]["id"].as_str().unwrap());
                }
                for mode in ["MAIN", "CALCS"] {
                    for action in source_rows(&group[mode]) {
                        assert_eq!(action["exact_source_object"], true);
                        assert_eq!(action["exact_group"], true);
                        if action["effect_index"] == 2 {
                            assert_eq!(action["minion_present"], false);
                        }
                        match action["source_kind"].as_str().unwrap() {
                            "manual_direct" => {
                                assert!(action["manual_source_ordinal"].is_u64());
                                assert!(action["source_node_id"].is_null());
                                assert_eq!(action["source_node_exact"], false);
                            }
                            "allocated" => {
                                assert!(action["manual_source_ordinal"].is_null());
                                assert!(action["source_node_id"].is_u64());
                                assert_eq!(action["source_node_exact"], true);
                                assert_eq!(group["no_supports"], true);
                            }
                            kind => panic!("unreviewed runtime source {kind}"),
                        }
                    }
                }
            }
        }
    }
    assert_eq!(failures, expected_failures);
    assert_eq!(children_seen.len(), 8);
    assert_eq!(commands_seen.len(), 2);
    for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
        for (index, family) in families.iter().enumerate() {
            let count: usize = [0, 4]
                .into_iter()
                .map(|case| {
                    cases[case]["states"][stage]["saved"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|r| r["attributes"]["gemId"] == family["game_id"])
                        .count()
                })
                .sum();
            assert_eq!(count, [6, 5][index]);
        }
    }
    for index in 0..5 {
        assert_eq!(cases[index]["name"], format!("original-{:02}", index + 1));
        assert_eq!(
            cases[index]["xml_sha256"],
            report["original_sources"][index]["sha256"]
        );
        let path = report["original_sources"][index]["path"].as_str().unwrap();
        assert_eq!(
            cases[index]["source_identity"]["source_bytes"],
            fs::metadata(root().join(path)).unwrap().len()
        );
    }
    let repeat = cases.last().unwrap();
    assert_eq!(repeat["name"], "repeat-original-05");
    assert!(repeat["states"] == cases[4]["states"]);
    assert_eq!(repeat["source_joins"], cases[4]["source_joins"]);
}

fn source_rows(value: &Value) -> &[Value] {
    match value {
        Value::Array(rows) => rows,
        Value::Object(object) if object.is_empty() => &[],
        _ => panic!("source list must be an array or an explicit empty Lua table"),
    }
}

fn assert_sets(family: &Value, skill: &Value, observed: &Value) {
    let metadata = family["action_metadata"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["skill_id"] == *skill)
        .unwrap();
    let observed = observed.as_array().unwrap();
    let expected = metadata["stat_sets"].as_array().unwrap();
    assert_eq!(observed.len(), expected.len());
    for set in expected {
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
}

fn preserve_migration(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    m: &OwnedReleaseMigrationInput,
) {
    let old = prior.input();
    let new = next.input();
    let mut schema = old.recipe.schema.clone();
    schema.release = m.release.clone();
    for entry in &m.schema {
        match entry {
            SchemaExtensionEntry::Definition(row) => {
                if let Some(old) = schema
                    .definitions
                    .iter_mut()
                    .find(|x| x.address() == row.address())
                {
                    *old = row.clone();
                } else {
                    schema.definitions.push(row.clone());
                }
            }
            SchemaExtensionEntry::Slot(row) => {
                if let Some(old) = schema
                    .slots
                    .iter_mut()
                    .find(|x| x.address() == row.address())
                {
                    *old = row.clone();
                } else {
                    schema.slots.push(row.clone());
                }
            }
        }
    }
    schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    schema.slots.sort_by_cached_key(SlotDescriptor::address);
    if schema != new.recipe.schema {
        let directory = root().join("runs/owned-djinn-actions-schema-debug");
        fs::create_dir_all(&directory).unwrap();
        for (name, value) in [("expected", &schema), ("actual", &new.recipe.schema)] {
            let bytes = serde_json::to_vec(value).unwrap();
            assert!(bytes.len() <= 64 * 1024 * 1024);
            fs::write(directory.join(format!("{name}.json")), bytes).unwrap();
        }
    }
    assert!(
        schema == new.recipe.schema,
        "only the exact authored schema replacement/append"
    );
    let b = bindings();
    let old_json = json!(old);
    let new_json = json!(new);
    for family in b["families"].as_array().unwrap() {
        let find = |value: &Value| {
            value["recipe"]["schema"]["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["value"]["id"] == family["skill"])
                .unwrap()
                .clone()
        };
        let mut expected = find(&old_json);
        let d = &mut expected["value"]["schema"]["value"]["declarations"];
        // Declared grant sets use typed address order; the source action order
        // remains independently represented in the correspondence mappings.
        d["grants"]["members"].as_array_mut().unwrap().extend([
            family["minion"]["entering_grant"].clone(),
            family["command"]["entering_grant"].clone(),
        ]);
        d["skill_grants"]["members"]
            .as_array_mut()
            .unwrap()
            .push(family["command"]["supply"].clone());
        d["actors"]["members"]
            .as_array_mut()
            .unwrap()
            .push(family["minion"]["population"].clone());
        assert!(
            expected == find(&new_json),
            "all raw Direct declarations and prior coverage survive"
        );
    }
    assert_eq!(new.recipe.registry.last_issued.get(), 0x32d1);
    assert_eq!(
        new.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 48
    );
    assert!(
        new.recipe.registry.entries[..old.recipe.registry.entries.len()]
            == old.recipe.registry.entries
    );
    let mut rules = json!(old.recipe.rules);
    rules["definitions"] = json!(next.receipt().definitions);
    for owner in &m.owners {
        assert!(
            !old.recipe
                .rules
                .owners
                .iter()
                .any(|x| x.owner == owner.owner)
        );
        rules["owners"].as_array_mut().unwrap().push(json!(owner));
    }
    assert!(
        rules == json!(new.recipe.rules),
        "old rules/tables/receivers/applications survive"
    );
    let mut restored = new_json;
    for path in [
        "/recipe/schema",
        "/recipe/rules",
        "/recipe/registry/revision",
        "/recipe/registry/last_issued",
        "/recipe/registry/entries",
        "/recipe/routing/definitions",
        "/mapping/definitions",
        "/mapping/registry",
        "/roles/definitions",
        "/roles/mapping",
        "/rewards/definitions",
        "/rewards/mapping",
        "/items/definitions",
        "/item_source/item_lines",
        "/tree/definitions",
        "/tree/registry",
        "/tree/mapping",
        "/tree/normalization",
        "/normalization/gem_quality/value/definitions",
        "/normalization/gem_inputs/definitions",
        "/normalization/direct_skill_inputs/definitions",
        "/normalization/direct_skill_inputs/roles",
        "/normalization/gem_inventory/definitions",
        "/normalization/gem_inventory/roles",
        "/normalization/gem_inventory/scalar_inputs",
        "/normalization/gem_inventory/usage_inputs",
        "/normalization/usage_inputs/definitions",
        "/normalization/usage_inputs/roles",
        "/normalization/usage_inputs/scalar_inputs",
        "/normalization/support_origin_order/roles",
        "/normalization/payload_inventory/roles",
        "/normalization/configuration_reward_inventory/reward_policy",
        "/normalization/equipment_membership/definitions",
        "/normalization/equipment_membership/item_lines",
        "/normalization/equipment_membership/item_source",
        "/normalization/item_modifier_membership/definitions",
        "/normalization/item_modifier_membership/item_lines",
        "/normalization/item_modifier_membership/item_source",
        "/normalization/item_parameter_inputs/definitions",
        "/normalization/item_parameter_inputs/item_lines",
        "/normalization/item_parameter_inputs/item_source",
        "/normalization/passive_socket_membership/definitions",
        "/normalization/passive_socket_membership/mapping",
        "/normalization/passive_socket_membership/item_lines",
        "/normalization/passive_socket_membership/item_source",
        "/normalization/passive_socket_membership/equipment",
    ] {
        *restored
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("new {path}")) = old_json
            .pointer(path)
            .unwrap_or_else(|| panic!("prior {path}"))
            .clone();
    }
    for index in 0..5 {
        for field in ["definitions", "roles"] {
            let path = format!(
                "/normalization/gem_inventory/primary_dispositions/{index}/reference_action/{field}"
            );
            *restored.pointer_mut(&path).unwrap() = old_json.pointer(&path).unwrap().clone();
        }
    }
    assert_eq!(new.provenance.len(), old.provenance.len() + 1);
    assert!(new.provenance[..old.provenance.len()] == old.provenance);
    restored["provenance"].as_array_mut().unwrap().pop();
    assert!(
        restored == old_json,
        "every other prior release field and ordered query survives"
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(receipt[field], a[field]);
    }
    assert_eq!(receipt["input"], a["before"]);
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    preserve_migration(prior, &migrated, &migration);
    let base = migrated.input();
    let mut normalization = base.normalization.clone();
    let previous = normalization.direct_skill_inputs.take().unwrap();
    let DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions,
        source,
        roles,
        catalog,
        manual_sources,
        group_attributes,
        skills,
    } = previous
    else {
        panic!("exact V1 predecessor required");
    };
    assert_eq!(skills.len(), 2);
    let mut rows = dispositions();
    for row in &mut rows {
        let mut value = json!(&row.reference_action);
        assert_eq!(
            value["source"],
            json!(prior.roles().input().compilation.source)
        );
        value["definitions"] = json!(migrated.receipt().definitions);
        value["roles"] = json!(migrated.receipt().roles);
        row.reference_action = serde_json::from_value(value).unwrap();
    }
    normalization.direct_skill_inputs = Some(DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions,
        source,
        roles,
        catalog,
        manual_sources,
        group_attributes,
        skills,
        dispositions: rows,
    });
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
        kind: OwnedDefinitionKey::new("manual-djinn-actions-with-deferred-usage").unwrap(),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-djinn-actions-v1",
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
        "only Direct disposition, dependent tree commitment and source provenance change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
