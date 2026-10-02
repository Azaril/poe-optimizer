//! Source-proved absence; this family changes no owned game definitions.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        EmptyCharacterRuneSelections, EquipmentMembershipPolicy, PassiveSocketMembershipPolicy,
        equipment_membership_identity,
    },
    owned_release::{
        OwnedReleaseInput, OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/empty-character-runes")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn policy() -> EmptyCharacterRuneSelections {
    read("policy.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}

pub fn check_authored() {
    let p = policy();
    let a = authoring();
    assert_eq!(p.slot_names.len(), 5);
    assert_eq!(p.slot_names.iter().collect::<BTreeSet<_>>().len(), 5);
    assert_eq!(p.explicit_empty_selection, "None");
    assert_eq!(json!(p.mapping_source), a["mapping_source"]);
    assert_eq!(a["allocated_definitions"], 0);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes =
        fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 22);
    let mut names = BTreeSet::new();
    for pin in pins {
        assert!(names.insert(pin["path"].as_str().unwrap()));
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    assert!(names.contains("src/Classes/DropDownControl.lua"));
    assert!(names.contains("src/Classes/ItemsTab.lua"));
    assert!(names.contains("src/Modules/CalcSetup.lua"));
    assert!(root.join(a["source_test"].as_str().unwrap()).is_file());
    assert!(root.join(a["source_observer"].as_str().unwrap()).is_file());
}

fn verify_source(a: &Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proof = &a["source_validation"];
    assert_eq!(
        proof["status"], "passed",
        "complete source witness must pass before publication"
    );
    assert_eq!(proof["jit_modes"], json!(["off", "on"]));
    let bytes = fs::read(root.join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root.join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(bytes, on, "identical complete JIT evidence");
    assert_eq!(
        bytes.len() as u64,
        proof["evidence_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(evidence["source_revision"], a["source_revision"]);
    assert_eq!(evidence["source_hash"], a["source_manifest_sha256"]);
    for field in [
        "case_count",
        "complete_load_attempts_per_jit",
        "full_controls",
        "expected_loader_failure_controls",
    ] {
        assert_eq!(evidence["evidence"][field], proof[field]);
    }
    assert_eq!(evidence["evidence"]["saved_explicit_empty_occurrences"], 5);
    assert_eq!(evidence["evidence"]["character_slot_count"], 5);
    assert_eq!(evidence["evidence"]["whole_build_parity"], false);
    let observer = fs::read(root.join(a["source_observer"].as_str().unwrap())).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(observer)),
        evidence["evidence"]["observer_sha256"]
    );
    let pins = a["source_files"].as_array().unwrap();
    let observed = evidence["evidence"]["files"].as_array().unwrap();
    assert_eq!(pins.len(), observed.len());
    for (pin, found) in pins.iter().zip(observed) {
        assert_eq!(pin["path"], found["path"]);
        assert_eq!(pin["sha256"], found["sha256"]);
    }
    for n in 1..=5 {
        let name = format!("original-{n:02}");
        let case = evidence["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap();
        assert_eq!(case["available"], true);
        let xml = fs::read(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{n:02}.xml"
        )))
        .unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(xml)), case["xml_sha256"]);
        for flag in [
            "saved_items_preserved",
            "saved_specs_preserved",
            "saved_item_sets_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "calcs_output_preserved",
            "original_functions_preserved",
            "fresh_loaded_objects",
        ] {
            assert_eq!(case["state"][flag], true, "{name} {flag}");
        }
    }
}

pub fn rebind_tree(prior: &StagedOwnedRelease, full: &mut OwnedReleaseInput) {
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            prior.assembled().registry(),
            prior.assembled().schema(),
            prior.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
}
pub fn rebind_equipment(full: &mut OwnedReleaseInput) {
    let digest = equipment_membership_identity(
        full.normalization.equipment_membership.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 { equipment, .. }) =
        &mut full.normalization.passive_socket_membership
    else {
        panic!("exact predecessor passive placement policy")
    };
    *equipment = digest;
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a = authoring();
    verify_source(&a);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "mapping",
        "items",
        "item_source",
        "normalization",
        "tree",
    ] {
        assert_eq!(receipt[field], a[field], "exact predecessor {field}");
    }
    assert_eq!(
        json!(prior.mapping().source_identity()),
        a["mapping_source"]
    );
    let before = prior.input();
    let mut full = before.clone();
    let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
        item_lines,
        item_source,
        imported_profiles,
    }) = full.normalization.equipment_membership.take()
    else {
        panic!("exact V2 predecessor")
    };
    full.normalization.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines,
            item_source,
            imported_profiles,
            empty_character_runes: policy(),
        },
    );
    rebind_equipment(&mut full);
    rebind_tree(prior, &mut full);
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("explicit-empty-character-rune-selections").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-empty-character-rune-selections-v1",
            &(a, policy()),
            1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        before.tree.as_ref().unwrap().content
    );
    let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
        item_lines,
        item_source,
        imported_profiles,
        empty_character_runes,
    }) = restored.normalization.equipment_membership.take()
    else {
        panic!("published V3")
    };
    assert_eq!(empty_character_runes, policy());
    restored.normalization.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines,
            item_source,
            imported_profiles,
        },
    );
    let prior_digest = equipment_membership_identity(
        before.normalization.equipment_membership.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 { equipment, .. }) =
        &mut restored.normalization.passive_socket_membership
    else {
        panic!("retained passive policy")
    };
    *equipment = prior_digest;
    restored.tree = before.tree.clone();
    assert_eq!(restored.provenance.len(), before.provenance.len() + 1);
    restored.provenance.pop();
    assert!(
        restored == *before,
        "absence family changed unrelated release facts"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.receipt().registry, prior.receipt().registry);
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.input().recipe.registry.last_issued.get(), 0x320e);
    next
}
