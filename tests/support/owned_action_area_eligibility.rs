//! Checked publication of a finite, data-defined Action eligibility channel.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub const KIND: &str = "action-area-eligibility-v1";
pub const PROGRAM: &str = "area-modifier-eligibility";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/action-area-eligibility")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().expect("bounded array")
}
// Only the optional observer's JSON uses {} for an empty Lua table. Owned
// authoring arrays still use the strict reader above.
fn source_rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn authoring_digest() -> OwnedContentDigest {
    digest_owned(
        "owned-action-area-eligibility-v1",
        &(
            read::<Value>("authoring.json"),
            read::<Value>("bindings.json"),
            read::<Value>("dependencies.json"),
            read::<OwnedReleaseMigrationInput>("migration.json"),
        ),
        8 * 1024 * 1024,
    )
    .unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(m.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(b["before"], a["before"]);
    assert_eq!(d["input"], a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
    );
    assert!(m.evaluation.is_none());
    assert_eq!(
        (
            a["new_definitions"].as_u64(),
            a["appended_programs"].as_u64(),
            a["complete_owners"].as_u64()
        ),
        (Some(0), Some(11), Some(0))
    );
    assert_eq!(a["whole_build_parity"], false);
    assert_eq!(m.owners.len(), 11);
    assert_eq!(rows(&b["endpoints"]).len(), 11);
    assert_eq!(rows(&d["outputs"]).len(), 11);
    let mut identities = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    for endpoint in rows(&b["endpoints"]) {
        let output: DeclaredSlot<ActionOutputDefId> = decode(&endpoint["output"]);
        assert!(identities.insert(output.clone()));
        let owner = SchemaSubject::Slot(SlotAddress::ActionOutput(output.clone()));
        let entry: Vec<_> = m.owners.iter().filter(|r| r.owner == owner).collect();
        assert_eq!(entry.len(), 1);
        assert!(!entry[0].programs.is_complete());
        let previous: Vec<_> = rows(&d["outputs"])
            .iter()
            .filter(|r| r["output"]["value"]["id"] == endpoint["output"])
            .collect();
        assert_eq!(previous.len(), 1);
        let mut inherited = if previous[0]["owner"].is_null() {
            vec![]
        } else {
            let old: DefinitionRules = decode(&previous[0]["owner"]);
            assert_eq!(entry[0].programs.closure, old.programs.closure);
            old.programs.members
        };
        let added: Vec<_> = entry[0]
            .programs
            .members
            .iter()
            .filter(|p| p.id == key(PROGRAM))
            .collect();
        assert_eq!(added.len(), 1);
        let p = added[0];
        assert_eq!(p.context, RuleEntityKind::Action);
        assert_eq!(p.effects.len(), 1);
        assert!(
            matches!(&p.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Current, stat, .. } if *stat == decode::<StatDefId>(&b["channel"]))
        );
        assert!(
            p.effects[0].when.is_some(),
            "unreviewed selections must not receive false as a default"
        );
        assert!(
            p.reads
                .iter()
                .all(|r| r.value_type == ComputedValueType::Boolean
                    && matches!(
                        r.source,
                        RuleReadSource::ActionPartIs { .. }
                            | RuleReadSource::ActionModeIs { .. }
                            | RuleReadSource::ActionStatSetIs { .. }
                    ))
        );
        inherited.push(p.clone());
        inherited.sort_by(|a, b| a.id.cmp(&b.id));
        let mut actual = entry[0].programs.members.clone();
        actual.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(actual, inherited, "preserve every old program body");
        for set in rows(&endpoint["stat_sets"]) {
            assert!(set["area_eligible"].is_boolean());
            assert!(pairs.insert((
                endpoint["source_effect"].as_str().unwrap(),
                set["source_index"].as_u64().unwrap()
            )));
        }
    }
    assert_eq!(pairs.len(), 14);
}

fn authenticated_bytes(row: &Value, limit: usize) -> Vec<u8> {
    let bytes = fs::read(root().join(row["path"].as_str().unwrap())).unwrap();
    assert!(bytes.len() <= limit);
    assert_eq!(bytes.len() as u64, row["bytes"].as_u64().unwrap());
    assert_eq!(hash(&bytes), row["sha256"]);
    bytes
}

/// Full runtime catalog, writer and exact tuple proof, not a status-bit gate.
pub fn check_source() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let authority = &a["source_inventory"];
    assert_eq!(authority["status"], "passed");
    let observer =
        fs::read(root().join("crates/poe-optimizer-pob/tests/support/area_eligibility_source.lua"))
            .unwrap();
    assert_eq!(hash(&observer), authority["observer_sha256"]);
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest_bytes), authority["source_manifest_sha256"]);
    assert_eq!(
        authority["source_manifest_sha256"],
        a["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let reports = rows(&authority["reports"]);
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    let bytes = authenticated_bytes(&reports[0], 16 * 1024 * 1024);
    assert_eq!(bytes, authenticated_bytes(&reports[1], 16 * 1024 * 1024));
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["source_revision"], a["source_revision"]);
    assert_eq!(report["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(report["observer_sha256"], authority["observer_sha256"]);
    for pin in rows(&report["files"]) {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|r| *r == pin)
                .count(),
            1
        );
    }
    let catalog_bytes =
        fs::read(root().join(report["catalog_reference"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&catalog_bytes), report["catalog_reference"]["sha256"]);
    let catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    let catalog_set = |support_only: bool| -> BTreeSet<&str> {
        rows(&catalog["skills"])
            .iter()
            .filter(|r| !support_only || r["support"] == true)
            .map(|r| r["id"].as_str().unwrap())
            .collect()
    };
    let expected: BTreeMap<_, _> = rows(&b["endpoints"])
        .iter()
        .flat_map(|endpoint| {
            rows(&endpoint["stat_sets"]).iter().map(move |set| {
                (
                    (
                        endpoint["source_effect"].as_str().unwrap(),
                        set["source_index"].as_u64().unwrap(),
                    ),
                    set["area_eligible"].as_bool().unwrap(),
                )
            })
        })
        .collect();
    assert_eq!(expected.len(), 14);
    let source04 = &report["source04"];
    assert_eq!(rows(&source04["matrix"]).len(), 14);
    let original_reports = rows(&source04["reports"]);
    assert_eq!(original_reports.len(), 2);
    let original = authenticated_bytes(&original_reports[0], 128 * 1024 * 1024);
    assert_eq!(
        original,
        authenticated_bytes(&original_reports[1], 128 * 1024 * 1024)
    );
    for row in rows(&source04["matrix"]) {
        assert_eq!(
            row["cfg_area"].as_bool().unwrap(),
            expected[&(
                row["effect"].as_str().unwrap(),
                row["stat_set_index"].as_u64().unwrap()
            )]
        );
        assert!(row["observations"].as_u64().unwrap() > 0);
    }
    assert_eq!(rows(&report["cases"]).len(), 2);
    let first = &report["cases"][0]["states"]["fresh"];
    for case in rows(&report["cases"]) {
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &case["states"][stage];
            assert_eq!(
                state, first,
                "strict independent fresh/lifecycle determinism"
            );
            for field in [
                "business_wrappers",
                "source_tables_mutated",
                "source_cfg_modified",
            ] {
                assert_eq!(state[field], false);
            }
            for field in ["original_methods_preserved", "jit_mode_preserved"] {
                assert_eq!(state[field], true);
            }
            assert_eq!(state["immutable_snapshot"]["verified"], true);
            let all: BTreeSet<_> = rows(&state["catalog"])
                .iter()
                .map(|r| r["id"].as_str().unwrap())
                .collect();
            assert_eq!(all, catalog_set(false));
            assert_eq!(
                all.len(),
                rows(&state["catalog"]).len(),
                "no duplicate catalog IDs"
            );
            let supports: BTreeSet<_> = rows(&state["catalog"])
                .iter()
                .filter(|r| r["support"] == true)
                .map(|r| r["id"].as_str().unwrap())
                .collect();
            assert_eq!(supports, catalog_set(true));
            assert_eq!(
                state["support_count"].as_u64().unwrap() as usize,
                supports.len()
            );
            assert_eq!(state["effect_count"].as_u64().unwrap() as usize, all.len());
            for row in rows(&state["catalog"]) {
                // Authenticated root fields, not stat-set fields: indentation
                // in act_str.lua does not define the source table structure.
                let totem = matches!(
                    row["id"].as_str().unwrap(),
                    "SupportAncestralWarriorTotemPlayer"
                        | "SupportMetaTotemSpellTotemPlayer"
                        | "SupportMortarCannonPlayer"
                );
                assert_eq!(row["add_flags"]["present"], totem);
                if totem {
                    assert_eq!(row["support"], true);
                    assert_eq!(
                        row["add_flags"]["entries"],
                        serde_json::json!([{"name":"totem","value":true}])
                    );
                } else {
                    assert!(source_rows(&row["add_flags"]["entries"]).is_empty());
                }
            }
            for row in rows(&state["global_map"]) {
                if row["skill_flag_present"] == true {
                    assert_eq!(row["skill_flag"], "arrow");
                }
            }
            assert_eq!(rows(&state["definitions"]).len(), 11);
            let mut actual = BTreeMap::new();
            for definition in rows(&state["definitions"]) {
                let effect = definition["effect"].as_str().unwrap();
                assert_eq!(definition["parts_present"], false);
                assert!(source_rows(&definition["parts"]).is_empty());
                assert_eq!(definition["root_fallback_exact"], true);
                for row in source_rows(&definition["root_map"]) {
                    assert_ne!(row["skill_flag"], "area");
                }
                for (i, set) in rows(&definition["stat_sets"]).iter().enumerate() {
                    assert_eq!(set["index"].as_u64().unwrap() as usize, i + 1);
                    let area = source_rows(&set["base_flags"]["entries"])
                        .iter()
                        .any(|r| r["name"] == "area" && r["value"] == true);
                    assert!(actual.insert((effect, (i + 1) as u64), area).is_none());
                    for row in source_rows(&set["local_map"]) {
                        assert_ne!(row["skill_flag"], "area");
                    }
                }
            }
            assert_eq!(actual, expected);
            assert!(!rows(&state["observations"]).is_empty());
            for observation in rows(&state["observations"]) {
                for field in ["cfg_available", "exact_definition", "exact_stat_set"] {
                    assert_eq!(observation[field], true);
                }
                assert_eq!(observation["cfg_area"], observation["definition_base_area"]);
                assert_eq!(
                    observation["cfg_area"].as_bool().unwrap(),
                    expected[&(
                        observation["effect"].as_str().unwrap(),
                        observation["stat_set_index"].as_u64().unwrap()
                    )]
                );
            }
        }
    }
}

pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let provenance = endpoint.input().provenance.last().unwrap();
    assert_eq!(provenance.kind, key(KIND));
    assert_eq!(provenance.prior_input, m.before);
    assert_eq!(provenance.authoring_input, authoring_digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    assert_eq!(
        endpoint.input().recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    for owner in &m.owners {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(json!(prior.receipt().input), a["before"]);
    assert_eq!(json!(prior.receipt().definitions), d["definitions"]);
    assert_eq!(json!(prior.receipt().registry), d["registry"]);
    assert!(prior.evaluation().is_none());
    for dependency in rows(&d["outputs"]) {
        let slot: SlotDescriptor = decode(&dependency["output"]);
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|r| **r == slot)
                .count(),
            1
        );
        if !dependency["owner"].is_null() {
            let owner: DefinitionRules = decode(&dependency["owner"]);
            assert_eq!(
                prior
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .filter(|r| **r == owner)
                    .count(),
                1
            );
        } else {
            let owner = SchemaSubject::Slot(slot.address());
            assert!(
                !prior
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .any(|r| r.owner == owner)
            );
        }
    }
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: authoring_digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().recipe.clone();
    for dependency in rows(&d["outputs"]) {
        let slot: SlotDescriptor = decode(&dependency["output"]);
        let owner = SchemaSubject::Slot(slot.address());
        if dependency["owner"].is_null() {
            restored.rules.owners.retain(|r| r.owner != owner);
        } else {
            *restored
                .rules
                .owners
                .iter_mut()
                .find(|r| r.owner == owner)
                .unwrap() = decode(&dependency["owner"]);
        }
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.rules.operations_version = prior.input().recipe.rules.operations_version.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only 11 appended programs and explicit V20 upgrade"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    assert_endpoint(&next);
    next
}
