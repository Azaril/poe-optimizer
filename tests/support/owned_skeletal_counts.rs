//! Append reviewed requested counts; retain incomplete usage and mechanics.
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        GemInventoryPolicy, PrimarySkillNumericUsageInput, UsageInputPolicy, usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
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
    root().join("data/owned/poe2/3887ae68/skeletal-counts")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn usages() -> Vec<PrimarySkillNumericUsageInput> {
    read("usage.json")
}
fn read_path<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn catalog_row<'a>(catalog: &'a Value, input: &Value) -> &'a Value {
    let matches: Vec<_> = rows(&catalog["gems"])
        .iter()
        .filter(|r| r["game_id"] == input["game_id"] && r["variant_id"] == input["variant_id"])
        .collect();
    assert_eq!(matches.len(), 1);
    let row = matches[0];
    assert_eq!(row["primary_effect_id"], input["skill_id"]);
    assert_eq!(
        input["name_spec"].as_str().unwrap(),
        row["name_spec"]
            .as_str()
            .unwrap_or_else(|| row["name"].as_str().unwrap())
    );
    row
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let usage = usages();
    assert_eq!(usage.len(), 3);
    assert_eq!(a["families"], 3);
    assert_eq!(a["original_occurrences"], 15);
    assert_eq!(a["original_family_occurrences"], json!([6, 4, 5]));
    for field in [
        "new_definitions",
        "new_slots",
        "new_rules",
        "new_tables",
        "new_defaults",
    ] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["usage_inventory"], "pending");
    assert_eq!(a["native_mechanics_coverage"], "partial");
    assert_eq!(a["reservation_rates"], "unimplemented");
    assert_eq!(a["preserved_query_rows"], 110);
    assert_eq!(
        a["parameter_domain"],
        json!({"kind":"integer","minimum":0,"maximum":4})
    );
    for file in ["usage.json", "authoring.json"] {
        assert!(!fs::read(data().join(file)).unwrap().contains(&b'\r'));
    }
    assert_eq!(
        a["artifact_sha256"]["usage"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(data().join("usage.json")).unwrap())
        )
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in rows(&a["source_files"]) {
        assert_eq!(pin.as_object().unwrap().len(), 2);
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let catalog = SkillIdentityCatalog::new(read_path(
        "data/owned/poe2/3887ae68/import/skill-identities.json",
    ))
    .unwrap();
    assert_eq!(
        a["catalog"],
        json!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
    );
    let catalog = json!(catalog.data());
    let original: Value = read_path("data/owned/poe2/3887ae68/sniper-reservation/usage.json");
    let topology: Value = read_path("data/owned/poe2/3887ae68/skeletal-inputs/bindings.json");
    for (index, row) in usage.iter().enumerate() {
        let row = json!(row);
        let family = &topology["families"][index];
        assert_eq!(row["gem"], family["gem"]);
        assert_eq!(row["primary"], family["primary"]);
        assert_eq!(row["supply"], family["primary_supply"]);
        assert_eq!(row["grant"], family["entering_grant"]);
        assert_eq!(catalog_row(&catalog, &row), &family["physical_identity"]);
        assert_eq!(row["policy"], original["policy"]);
        assert_eq!(rows(&row["parameters"]).len(), 1);
        assert_eq!(
            row["parameters"][0]["slot"],
            original["parameters"][0]["slot"]
        );
        let source = &row["parameters"][0]["source"];
        assert_eq!(source["kind"], "containing_group_override");
        for field in ["group", "occurrence"] {
            let mut actual = source[field].clone();
            actual["id"] = original["parameters"][0]["source"][field]["id"].clone();
            assert_eq!(
                actual, original["parameters"][0]["source"][field],
                "same bounded numeric codec and missing policy"
            );
        }
        assert_eq!(
            source["fallback_admission"]["kind"],
            "unique_reviewed_primary"
        );
        let companions = rows(&source["fallback_admission"]["companions"]);
        assert_eq!(companions.len(), [5, 1, 4][index]);
        let mut unique = BTreeSet::new();
        for companion in companions {
            assert!(unique.insert(companion["gem"].to_string()));
            let c = catalog_row(&catalog, companion);
            assert_ne!(c["primary_effect_id"], row["skill_id"]);
            assert!(!c["primary_effect_id"].is_null());
            for field in ["effect_list", "additional_effects"] {
                assert!(
                    rows(&c[field]).iter().all(|e| e != &row["skill_id"]),
                    "all resolved effects exclude target"
                );
            }
            for field in [
                "declared_additional_effects",
                "constructed_additional_effects",
                "declared_additional_stat_sets",
            ] {
                assert!(
                    rows(&c[field]).iter().all(|e| e["id"] != row["skill_id"]),
                    "unresolved declarations also exclude target"
                );
            }
        }
    }
}
fn report(a: &Value, kind: &str) -> Value {
    let proof = &a["source_validation"][kind];
    assert_eq!(a["source_validation"]["status"], "passed");
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
        "exact complete report equality across JIT modes"
    );
    let r: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(r["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(r["catalog_digest"], a["catalog"]);
    assert_eq!(r["files"], proof["files"]);
    assert_eq!(rows(&r["files"]).len(), 13);
    for pin in rows(&r["files"]) {
        assert!(rows(&a["source_files"]).contains(pin));
    }
    for flag in ["native_inventory_authority", "native_build_parity"] {
        assert_eq!(r[flag], false);
    }
    r
}
fn source_proof(a: &Value) {
    let counts = report(a, "count_semantics");
    let inventory = report(a, "all_occurrences");
    let usage = usages();
    let original05 =
        fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    assert_eq!(
        counts["original_xml_sha256"],
        format!("{:x}", Sha256::digest(&original05))
    );
    assert_eq!(counts["missing_commands_resolved"], false);
    assert!(counts["original"] == counts["repeated_original"]);
    assert!(counts["original"] == counts["warm_original"]);
    assert_eq!(rows(&counts["controls"]).len(), 68);
    for control in rows(&counts["controls"]) {
        assert_eq!(control["state"]["source_hash"], a["source_manifest_sha256"]);
        for flag in [
            "source_methods_preserved",
            "saved_instances_preserved",
            "source_catalog_preserved",
            "main_and_calcs_outputs_preserved",
            "selected_state_preserved",
        ] {
            assert_eq!(control["state"]["occurrence"][flag], true);
        }
        assert_eq!(
            control["state"]["minions"]["source_methods_preserved"],
            true
        );
    }
    for row in &usage {
        let j = json!(row);
        assert!(
            rows(&counts["reviewed_gems"])
                .iter()
                .any(|g| g["game_id"] == row.game_id && g["primary_effect_id"] == row.skill_id)
        );
        for (name, expected) in [
            ("focused", 1),
            ("count-three", 3),
            ("group-four", 4),
            ("group-zero", 0),
            ("global-one-false", 1),
            ("global-two-false", 1),
        ] {
            let controls: Vec<_> = rows(&counts["controls"])
                .iter()
                .filter(|c| c["physical_id"] == row.game_id && c["name"] == name)
                .collect();
            assert_eq!(controls.len(), 1);
            let state = &controls[0]["state"];
            let physical: Vec<_> = rows(&state["minions"]["selected"])
                .iter()
                .filter(|g| g["physical_id"] == row.game_id)
                .collect();
            assert_eq!(physical.len(), 1);
            for mode in ["MAIN", "CALCS"] {
                let actions = rows(&physical[0][mode]);
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0]["effect"], j["skill_id"]);
                assert_eq!(actions[0]["count"], expected);
                assert_eq!(actions[0]["physical_source"], true);
                assert_eq!(actions[0]["count_enabled"], true);
            }
        }
    }
    assert_eq!(inventory["source_revision"], a["source_revision"]);
    assert_eq!(inventory["business_wrappers"], false);
    assert_eq!(inventory["canonical_parity_lifecycle_selected"], false);
    assert_eq!(rows(&inventory["cases"]).len(), 62);
    assert_eq!(rows(&inventory["original_sources"]).len(), 5);
    let mut totals = [0usize; 3];
    let mut companion_seen: [BTreeSet<String>; 3] = Default::default();
    for original in 1..=5 {
        let path = format!("tests/fixtures/builds/breadth-20260908/build-{original:02}.xml");
        let bytes = fs::read(root().join(&path)).unwrap();
        let sha = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(
            inventory["original_sources"][original - 1],
            json!({"path":path,"sha256":sha})
        );
        let cases: Vec<_> = rows(&inventory["cases"])
            .iter()
            .filter(|c| c["name"] == format!("original-{original:02}"))
            .collect();
        assert_eq!(cases.len(), 1);
        let case = cases[0];
        assert_eq!(case["xml_sha256"], sha);
        assert_eq!(case["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(case["source_identity"]["source_sha256"], sha);
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([96; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
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
                assert_eq!(state[flag], true);
            }
            assert_eq!(rows(&state["saved"]).len(), [3, 0, 0, 0, 12][original - 1]);
            for saved in rows(&state["saved"]) {
                let index = usage
                    .iter()
                    .position(|u| saved["attributes"]["gemId"] == u.game_id)
                    .unwrap();
                let u = &usage[index];
                assert_eq!(saved["attributes"]["variantId"], u.variant_id);
                assert_eq!(saved["attributes"]["skillId"], u.skill_id);
                assert_eq!(saved["attributes"]["count"], "1");
                assert_eq!(saved["loaded"]["count"], 1);
                assert!(saved["group_state"].get("group_count").is_none());
                let source = &evidence.rows()[saved["source_ordinal"].as_u64().unwrap() as usize];
                assert_eq!(
                    source.attribute("gemId").unwrap().decoded().unwrap(),
                    u.game_id
                );
                assert_eq!(source.attribute("count").unwrap().decoded().unwrap(), "1");
                let group =
                    &evidence.rows()[source.occurrence().parent().unwrap().ordinal() as usize];
                let set = &evidence.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
                let joins: Vec<_> = rows(&case["source_joins"])
                    .iter()
                    .filter(|join| join["source_ordinal"] == saved["source_ordinal"])
                    .collect();
                assert_eq!(joins.len(), 1);
                assert_eq!(joins[0]["source"]["source_sha256"], sha);
                assert_eq!(joins[0]["source"]["ordinal"], saved["source_ordinal"]);
                assert_eq!(joins[0]["preset"], saved["preset"]);
                assert_eq!(
                    set.attribute("id")
                        .unwrap()
                        .decoded()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap(),
                    saved["preset"].as_u64().unwrap()
                );
                if stage == "fresh" {
                    totals[index] += 1;
                    let j = json!(u);
                    let companions =
                        rows(&j["parameters"][0]["source"]["fallback_admission"]["companions"]);
                    for sibling in group
                        .children()
                        .iter()
                        .filter(|id| **id != source.occurrence().id())
                    {
                        let sibling = &evidence.rows()[sibling.ordinal() as usize];
                        assert_eq!(sibling.occurrence().name(), "Gem");
                        let matching: Vec<_> = companions
                            .iter()
                            .filter(|c| {
                                [
                                    ("gemId", "game_id"),
                                    ("variantId", "variant_id"),
                                    ("skillId", "skill_id"),
                                    ("nameSpec", "name_spec"),
                                ]
                                .iter()
                                .all(|(field, key)| {
                                    sibling.attribute(field).unwrap().decoded().unwrap()
                                        == c[*key].as_str().unwrap()
                                })
                            })
                            .collect();
                        assert_eq!(
                            matching.len(),
                            1,
                            "every original companion is explicitly reviewed"
                        );
                        companion_seen[index].insert(matching[0]["gem"].to_string());
                    }
                }
            }
        }
    }
    assert_eq!(totals, [6, 4, 5]);
    assert_eq!(companion_seen.map(|s| s.len()), [5, 1, 4]);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    source_proof(&a);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "mapping",
        "roles",
        "normalization",
        "tree",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    let b = prior.input();
    let mut normalization = b.normalization.clone();
    let Some(UsageInputPolicy::PobPhysicalPrimarySkillV2 {
        numeric_gems,
        gems,
        catalog,
        scalar_inputs,
        ..
    }) = &mut normalization.usage_inputs
    else {
        panic!("exact UsageV2 predecessor");
    };
    assert_eq!(numeric_gems.len(), 1);
    assert_eq!(gems.len(), 2);
    assert_eq!(json!(catalog), a["catalog"]);
    assert_eq!(json!(scalar_inputs), a["scalar_inputs"]);
    numeric_gems.extend(usages());
    let usage_identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("retain V3 physical inventories");
    };
    assert_eq!(json!(usage_inputs), a["prior_usage_inputs"]);
    *usage_inputs = usage_identity;
    assert!(normalization.direct_skill_inputs == b.normalization.direct_skill_inputs);
    assert!(normalization.skill_inventory == b.normalization.skill_inventory);
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
    assert!(full.normalization == normalization);
    assert!(full.tree.as_ref().unwrap().content == b.tree.as_ref().unwrap().content);
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("skeletal-requested-counts").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-skeletal-requested-counts-v1",
            &(a, usages()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization = b.normalization.clone();
    restored.tree = b.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *b,
        "no definitions, mechanics, existing rules, queries or other release data change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
