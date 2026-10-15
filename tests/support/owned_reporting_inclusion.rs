//! Retained reporting intent, with no ordinary gameplay effects or aggregate query.
use super::migration_preservation;
use poe_optimizer_core::owned_content::*;
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, NormalizationPolicy, OccurrenceUsagePolicy, OccurrenceUsageRule,
        PrimarySkillUsageInput, UsageInputPolicy, usage_inputs_identity,
    },
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

pub const KIND: &str = "reporting-inclusion";
pub const POLICY: &str = "def.000000000000338e";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/reporting-inclusion")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn usage() -> OccurrenceUsagePolicy {
    read("usage.json")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn compressed(name: &str, maximum: u64) -> Vec<u8> {
    let path = data().join(name);
    assert!(fs::metadata(&path).unwrap().len() <= maximum);
    let bytes = fs::read(path).unwrap();
    assert!(bytes.len() as u64 <= maximum);
    bytes
}
fn unpack(name: &str, maximum: u64) -> Vec<u8> {
    let bytes = compressed(name, 4 * 1024 * 1024);
    let mut raw = vec![];
    flate2::read::GzDecoder::new(bytes.as_slice())
        .take(maximum + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= maximum);
    raw
}
fn reconstruct_source() -> Value {
    let p: Value = read("source-evidence.json");
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        hash(&manifest_bytes),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(p["source_manifest_sha256"], hash(&manifest_bytes));
    assert_eq!(p["source_revision"], manifest["upstream_revision"]);
    assert_eq!(
        p["semantics"],
        json!({"true":true,"false":false,"nil":false,"absent_import":"pending","reporting_only":true,"ordinary_gameplay_effects":false,"aggregate_execution":false,"complete_usage_inventory":false})
    );
    let archives = p["archives"].as_array().unwrap();
    assert_eq!(archives.len(), 2);
    for (a, (expected, archive)) in archives.iter().zip([
        ("src/Classes/SkillsTab.lua", "SkillsTab.lua.gz"),
        ("src/Modules/Calcs.lua", "Calcs.lua.gz"),
    ]) {
        assert_eq!(a["source"]["path"], expected);
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| **v == a["source"])
                .count(),
            1
        );
        let name = a["archive"].as_str().unwrap();
        assert_eq!(name, archive);
        let compressed = compressed(name, 1024 * 1024);
        assert_eq!(compressed.len(), a["compressed_bytes"]);
        assert_eq!(hash(&compressed), a["compressed_sha256"]);
        let raw = unpack(name, 1024 * 1024);
        assert_eq!(raw.len(), a["raw_bytes"]);
        assert_eq!(hash(&raw), a["source"]["sha256"]);
        let text = std::str::from_utf8(&raw).unwrap();
        if expected.ends_with("SkillsTab.lua") {
            assert_eq!(
                text.lines().nth(309).unwrap().trim(),
                "socketGroup.includeInFullDPS = node.attrib.includeInFullDPS and node.attrib.includeInFullDPS == \"true\""
            );
        } else {
            assert_eq!(
                text.lines().nth(291).unwrap().trim(),
                "if activeSkill.socketGroup and activeSkill.socketGroup.includeInFullDPS then"
            );
        }
    }
    assert_eq!(
        p["source_spans"],
        json!([
            {"path":"src/Classes/SkillsTab.lua","first":303,"last":315,"role":"load-inclusion-literal"},
            {"path":"src/Modules/Calcs.lua","first":251,"last":312,"role":"explicit-full-dps-inclusion-and-count"}
        ])
    );
    let a = &p["attribution"];
    assert_eq!(a["archive"], "source-attribution.json.gz");
    assert_eq!(
        a["raw_sha256"],
        "c25597169569bbc08d57d5043b296cfd67327ae9855e9638e55cb73296d95018"
    );
    assert_eq!(a["raw_bytes"], 16_334_100);
    assert_eq!(
        a["acquisition_paths"],
        json!([
            "runs/owned-full-dps-attribution-source-03/source-jit-off.json",
            "runs/owned-full-dps-attribution-source-03/source-jit-on.json"
        ])
    );
    let compressed = compressed("source-attribution.json.gz", 4 * 1024 * 1024);
    assert_eq!(hash(&compressed), a["compressed_sha256"]);
    assert_eq!(compressed.len(), a["compressed_bytes"]);
    let raw = unpack("source-attribution.json.gz", 32 * 1024 * 1024);
    assert_eq!(raw.len(), a["raw_bytes"]);
    assert_eq!(hash(&raw), a["raw_sha256"]);
    let report: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(report["source_revision"], p["source_revision"]);
    assert_eq!(report["manifest_sha256"], p["source_manifest_sha256"]);
    assert_eq!(report["native_inventory_authority"], false);
    assert_eq!(report["native_build_parity"], false);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 44);
    assert_eq!(
        json!(cases.iter().map(|c| c["name"].clone()).collect::<Vec<_>>()),
        a["cases"]
    );
    for case in cases {
        assert_eq!(case["unobserved_outputs_equal"], true);
        assert_eq!(case["unobserved_selections_equal"], true);
        for state in case["states"].as_object().unwrap().values() {
            assert_eq!(state["original_functions_preserved"], true);
            assert_eq!(state["formulas_reimplemented"], false);
        }
    }
    p
}
pub fn source() -> Value {
    static PROOF: OnceLock<Value> = OnceLock::new();
    PROOF.get_or_init(reconstruct_source).clone()
}
pub fn authenticate_source(proof: &Value) {
    assert!(
        proof == &source(),
        "complete retained source proof identity and body"
    );
}
pub fn check_authored() {
    authenticate_source(&read::<Value>("source-evidence.json"));
    let a: Value = read("authoring.json");
    for pin in a["artifacts"].as_array().unwrap() {
        let bytes = fs::read(data().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len(), pin["bytes"]);
        assert_eq!(hash(&bytes), pin["sha256"]);
    }
    let e = extension();
    assert_eq!(e.schema.len(), 2);
    assert_eq!(e.owners.len(), 1);
    assert!(e.owners[0].programs.is_complete());
    assert!(e.owners[0].programs.members.is_empty());
    assert!(e.tables.is_empty() && e.receivers.is_empty());
    assert_eq!(usage().policy.key().as_str(), POLICY);
    let u: Value = read("usage.json");
    assert_eq!(u["parameters"].as_array().unwrap().len(), 1);
    assert_eq!(u["parameters"][0]["source"]["kind"], "containing_group");
    assert_eq!(
        u["parameters"][0]["source"]["value"]["codec"]["codec"]["value"]["tokens"],
        json!([
            {"token":"true","value":true},{"token":"false","value":false},{"token":"nil","value":false}
        ])
    );
    assert_eq!(
        u["parameters"][0]["source"]["value"]["missing"]["kind"],
        "pending"
    );
    let n: Value = read("native.json");
    assert_eq!(n["support"]["owner"], json!(e.owners[0].owner));
    assert_eq!(
        n["support"]["domain"],
        json!({"kind":"known","value":"authored_assignments_only"})
    );
    assert_eq!(
        n["scope"],
        json!({"ordinary_gameplay_effects":false,"report_execution":false,"aggregate_metric":false,"preset_complete":false})
    );
    let changes: Value = read("changes.json");
    for (kind, count) in [("physical", 7), ("occurrences", 6)] {
        assert_eq!(changes[kind].as_array().unwrap().len(), count);
        for row in changes[kind].as_array().unwrap() {
            let mut after = row["after"].clone();
            assert_eq!(
                after["policies"].as_array_mut().unwrap().pop(),
                Some(u.clone())
            );
            assert_eq!(after, row["before"]);
        }
    }
}
fn edit(policy: &mut NormalizationPolicy, forward: bool) {
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        physical,
        occurrences,
        ..
    } = policy.usage_inputs.as_mut().unwrap();
    let c: Value = read("changes.json");
    assert_eq!(physical.len(), 7);
    assert_eq!(occurrences.len(), 6);
    for (actual, row) in physical.iter_mut().zip(c["physical"].as_array().unwrap()) {
        let before: PrimarySkillUsageInput = serde_json::from_value(row["before"].clone()).unwrap();
        let after: PrimarySkillUsageInput = serde_json::from_value(row["after"].clone()).unwrap();
        let (expected, replacement) = if forward {
            (before, after)
        } else {
            (after, before)
        };
        assert_eq!(*actual, expected);
        *actual = replacement;
    }
    for (actual, row) in occurrences
        .iter_mut()
        .zip(c["occurrences"].as_array().unwrap())
    {
        let before: OccurrenceUsageRule = serde_json::from_value(row["before"].clone()).unwrap();
        let after: OccurrenceUsageRule = serde_json::from_value(row["after"].clone()).unwrap();
        let (expected, replacement) = if forward {
            (before, after)
        } else {
            (after, before)
        };
        assert_eq!(*actual, expected);
        *actual = replacement;
    }
    let identity = usage_inputs_identity(policy, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut policy.gem_inventory
    else {
        panic!("current usage inventory binding")
    };
    *usage_inputs = identity;
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(json!(prior.receipt().input), a["before"]);
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x338d);
    let migrated = compile_owned_release_migration(
        prior,
        OwnedReleaseMigrationInput {
            schema_version: 5,
            before: prior.receipt().input,
            release: serde_json::from_value(a["release"].clone()).unwrap(),
            reason: KIND.parse().unwrap(),
            contract: serde_json::from_value(a["contract"].clone()).unwrap(),
            schema: extension().schema,
            tables: vec![],
            owners: vec![],
            receivers: vec![],
            query_targets: vec![],
            evaluation: None,
        },
        Default::default(),
    )
    .unwrap();
    migration_preservation::assert_import_rebindings_only(prior, &migrated);
    let mut base = migrated.input().clone();
    let owner = extension().owners.remove(0);
    let native: Value = read("native.json");
    let support = serde_json::from_value(native["support"].clone()).unwrap();
    base.recipe.rules.owners.push(owner.clone());
    base.recipe
        .rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .push(support);
    let mut normalization = base.normalization.clone();
    edit(&mut normalization, true);
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
        normalization,
        Default::default(),
    )
    .unwrap();
    base.normalization = transition.normalization().clone();
    base.tree = transition.tree().map(|t| t.input().clone());
    *base.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            KIND,
            &json!({"authoring":a,"source":source()}),
            4 * 1024 * 1024,
        )
        .unwrap(),
    };
    let next = assemble_owned_release(base, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    edit(&mut inverse.normalization, false);
    inverse.tree.as_mut().unwrap().normalization =
        migrated.input().tree.as_ref().unwrap().normalization;
    assert_eq!(inverse.recipe.rules.owners.pop(), Some(owner));
    assert_eq!(
        json!(
            inverse
                .recipe
                .rules
                .support_discovery
                .as_mut()
                .unwrap()
                .providers
                .pop()
                .unwrap()
        ),
        native["support"]
    );
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "only exact policy/slot/owner/support declaration and thirteen usage appends"
    );
    assert_eq!(prior.query_sets(), next.query_sets());
    assert!(next.evaluation().is_none());
    next
}
