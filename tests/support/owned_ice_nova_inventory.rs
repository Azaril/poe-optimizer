//! Source-disposition publication changes only physical inventory proof.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{GemInventoryPolicy, PrimaryGemInputDisposition},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ice-nova-inventory")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn disposition() -> PrimaryGemInputDisposition {
    read("disposition.json")
}
fn catalog() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let d = disposition();
    let j = json!(d);
    let bytes = fs::read(data().join("disposition.json")).unwrap();
    assert!(!bytes.contains(&b'\r'));
    assert!(
        !fs::read(data().join("authoring.json"))
            .unwrap()
            .contains(&b'\r')
    );
    assert_eq!(
        a["artifact_sha256"]["disposition"],
        format!("{:x}", Sha256::digest(bytes))
    );
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
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
    let matching: Vec<_> = catalog["gems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["game_id"] == d.physical.game_id && row["variant_id"] == d.physical.variant_id
        })
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["primary_effect_id"], d.physical.skill_id);
    assert_eq!(matching[0]["effect_list"], json!([d.physical.skill_id]));
    for field in [
        "declared_additional_effects",
        "constructed_additional_effects",
        "additional_effects",
    ] {
        assert_eq!(matching[0][field], json!([]));
    }
    assert_eq!(
        matching[0]["declared_additional_stat_sets"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for field in ["definitions", "roles", "catalog"] {
        assert_eq!(j["reference_action"][field], a[field]);
    }
    for field in ["gem", "game_id", "variant_id", "skill_id", "name_spec"] {
        assert_eq!(j["reference_action"][field], j["physical"][field]);
    }
    assert_eq!(
        j["reference_action"]["stat_sets"].as_array().unwrap().len(),
        2
    );
    let fields: Vec<_> = j["deferred_usage"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["field"].as_str().unwrap())
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
    for field in [
        "new_definitions",
        "new_programs",
        "new_tables",
        "new_scalar_values",
    ] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["usage_inventory"], "pending");
    assert_eq!(a["native_mechanics_coverage"], "partial");
}

fn source_proof(a: &Value) {
    let proof = &a["source_validation"];
    assert_eq!(
        proof["status"], "passed",
        "pending source evidence cannot authorize publication"
    );
    let mut reports = vec![];
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
        "all complete observations agree across JIT modes"
    );
    let r: Value = serde_json::from_slice(&reports[0]).unwrap();
    for (source, author) in [
        ("manifest_sha256", "source_manifest_sha256"),
        ("source_revision", "source_revision"),
        ("catalog_digest", "catalog"),
    ] {
        assert_eq!(r[source], a[author]);
    }
    assert_eq!(r["files"], a["source_files"]);
    assert_eq!(r["files"].as_array().unwrap().len(), 12);
    for field in [
        "business_wrappers",
        "native_inventory_authority",
        "native_build_parity",
        "phase_independent_activity_claimed",
        "universal_count_inertness_claimed",
        "canonical_parity_lifecycle_selected",
        "full_dps_rows_have_physical_identity",
    ] {
        assert_eq!(r[field], false);
    }
    let d = disposition();
    let catalog = catalog();
    let physical = catalog["gems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["game_id"] == d.physical.game_id && v["variant_id"] == d.physical.variant_id)
        .unwrap();
    assert_eq!(&r["physical_identity"], physical);
    let cases = r["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 23);
    assert_eq!(proof["cases"], 23);
    assert_eq!(proof["lifecycle_stages"], 3);
    assert_eq!(r["original_sources"].as_array().unwrap().len(), 5);
    for case in 1..=5 {
        let path = format!("tests/fixtures/builds/breadth-20260908/build-{case:02}.xml");
        let sha = format!(
            "{:x}",
            Sha256::digest(fs::read(root().join(&path)).unwrap())
        );
        let pin = &r["original_sources"][case - 1];
        assert_eq!(pin, &json!({"path":path,"sha256":sha}));
        assert_eq!(cases[case - 1]["name"], format!("original-{case:02}"));
        assert_eq!(cases[case - 1]["xml_sha256"], sha);
    }
    let expected = [
        "global-1-false",
        "global-2-false",
        "both-globals-false",
        "count-zero",
        "count-three",
        "count-missing",
        "count-malformed",
        "group-zero-over-count-three",
        "group-four-over-count-three",
        "full-dps-count-one",
        "full-dps-count-three",
        "disabled-gem",
        "disabled-group",
        "main-two-calcs-one",
        "main-one-calcs-two",
        "duplicate-independent-inputs",
        "archived-only",
        "repeat-original-05",
    ];
    for (case, name) in cases[5..].iter().zip(expected) {
        assert_eq!(case["name"], name);
    }
    assert!(
        cases[4]["states"] == cases[22]["states"],
        "restored source observations"
    );
    assert_eq!(cases[4]["source_joins"], cases[22]["source_joins"]);
    for case in cases {
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &case["states"][stage];
            for flag in [
                "source_methods_preserved",
                "exact_physical_objects",
                "physical_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(state[flag], true, "{} {stage} {flag}", case["name"]);
            }
        }
    }
    for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
        assert_eq!(
            cases[4]["states"][stage]["saved"].as_array().unwrap().len(),
            4
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    source_proof(&a);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(receipt[field], a[field], "exact predecessor {field}");
    }
    let b = prior.input();
    let Some(GemInventoryPolicy::PobFreshPhysicalV2 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        usage_inputs,
        supports,
        primary_skills,
    }) = &b.normalization.gem_inventory
    else {
        panic!("exact V2 predecessor required");
    };
    for (field, value) in [
        ("catalog", json!(catalog)),
        ("scalar_inputs", json!(scalar_inputs)),
        ("usage_inputs", json!(usage_inputs)),
    ] {
        assert_eq!(a[field], value);
    }
    assert_eq!(supports.len(), 514);
    assert_eq!(primary_skills.len(), 2);
    let row = disposition();
    assert_eq!(
        json!(row.reference_action)["source"],
        json!(prior.roles().input().compilation.source)
    );
    let mut normalization = b.normalization.clone();
    normalization.gem_inventory = Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        definitions: definitions.clone(),
        roles: *roles,
        catalog: *catalog,
        scalar_inputs: *scalar_inputs,
        usage_inputs: *usage_inputs,
        supports: supports.clone(),
        primary_skills: primary_skills.clone(),
        primary_dispositions: vec![row.clone()],
    });
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
    assert!(
        full.normalization == normalization,
        "checked normalization transition"
    );
    assert!(
        full.tree.as_ref().unwrap().content == b.tree.as_ref().unwrap().content,
        "tree content remains exact"
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("ice-nova-physical-inventory-with-deferred-usage").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ice-nova-physical-disposition-v1",
            &(a, row),
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
        "only inventory policy, dependent tree commitment and provenance may change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
