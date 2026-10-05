//! Offline catalogue correction: source associations are not executable supplies.
//! No source evaluator, numerical rule, mapping, or coverage closure is changed.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey,
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_mapping::MappingEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

const CATALOG: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
const MAPPING: &str = "data/owned/poe2/3887ae68/import/compiled/mapping.json";
const POLICY: &str = "data/owned/poe2/3887ae68/import/skill-catalog-policy.json";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/gem-executable-memberships-v1")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn tracked(name: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn by_key<'a>(values: &'a [Value], field: &str) -> BTreeMap<&'a str, &'a Value> {
    let mut result = BTreeMap::new();
    for row in values {
        assert!(result.insert(text(&row[field]), row).is_none());
    }
    result
}

fn check_census(bindings: &Value, catalog: &Value, mapping: &Value) {
    let mut skills = BTreeMap::new();
    let mut gems = BTreeMap::new();
    for entry in rows(&mapping["entries"]) {
        if entry["source"]["kind"] != "definition" {
            continue;
        }
        let source = &entry["source"]["value"];
        if source["kind"] != "skill" && source["kind"] != "gem" {
            continue;
        }
        assert_eq!(entry["outcome"]["kind"], "mapped");
        assert_eq!(entry["outcome"]["value"]["basis"]["kind"], "exact");
        let target = &entry["outcome"]["value"]["target"];
        assert_eq!(target["kind"], "definition");
        assert_eq!(target["value"]["kind"], source["kind"]);
        let target = &target["value"]["value"];
        if source["kind"] == "skill" {
            let id = &source["value"]["effect_id"];
            assert_eq!(id["kind"], "text");
            assert!(skills.insert(text(&id["value"]), target).is_none());
        } else {
            let game = &source["value"]["game_id"];
            let variant = &source["value"]["variant_id"];
            assert_eq!(game["kind"], "text");
            assert_eq!(variant["kind"], "text");
            assert!(
                gems.insert((text(&game["value"]), text(&variant["value"])), target)
                    .is_none()
            );
        }
    }
    let source_skills = by_key(rows(&catalog["skills"]), "id");
    assert_eq!(skills.len(), 1436);
    assert_eq!(gems.len(), 966);
    assert_eq!(source_skills.len(), 1436);
    // The constructed winners retain the exact declaration's role flag. A
    // display-order position, name, or primary association never supplies it.
    for skill in source_skills.values() {
        let declaration = rows(&catalog["skill_declarations"])
            .iter()
            .find(|row| row["index"] == skill["winning_declaration"])
            .unwrap();
        assert_eq!(declaration["id"], skill["id"]);
        assert_eq!(declaration["identity"]["support"], skill["support"]);
        assert!(skill["support"].is_null() || skill["support"].is_boolean());
    }
    assert_eq!(rows(&bindings["gems"]).len(), rows(&catalog["gems"]).len());
    let mut known = 0;
    let mut affected = 0;
    let mut removed_count = 0;
    let mut support_primary = 0;
    let mut mixed = 0;
    let mut missing = Vec::new();
    for (row, source) in rows(&bindings["gems"]).iter().zip(rows(&catalog["gems"])) {
        assert_eq!(row["catalog_key"], source["key"]);
        assert_eq!(
            &row["gem"],
            gems[&(text(&source["game_id"]), text(&source["variant_id"]))]
        );
        assert_eq!(
            &row["primary_skill"],
            skills[text(&source["primary_effect_id"])]
        );
        let effects: Vec<_> = rows(&source["effect_list"]).iter().map(|id| {
            json!({"source_id":id,"skill":skills[text(id)],"support":source_skills[text(id)]["support"]})
        }).collect();
        assert_eq!(row["effects"], json!(effects));
        support_primary +=
            usize::from(source_skills[text(&source["primary_effect_id"])]["support"] == true);
        mixed += usize::from(
            effects.iter().any(|x| x["support"] == true)
                && effects.iter().any(|x| x["support"] != true),
        );
        let mut absent = Vec::new();
        for (field, kind) in [
            ("declared_additional_effects", "declared_additional_effect"),
            (
                "constructed_additional_effects",
                "constructed_additional_effect",
            ),
        ] {
            for reference in rows(&source[field]) {
                if !source_skills.contains_key(text(&reference["id"])) {
                    absent.push(json!({"gem_key":source["key"],"kind":kind,"index":reference["index"],"effect_id":reference["id"]}));
                }
            }
        }
        assert_eq!(row["missing_references"], json!(absent));
        missing.extend(absent);
        if row["schema_kind"] == "known" {
            known += 1;
            let members = rows(&row["before_skills"]["members"]);
            let mut all_effects: Vec<_> = effects.iter().map(|x| x["skill"].clone()).collect();
            all_effects.sort_by(|a, b| text(&a["key"]).cmp(text(&b["key"])));
            assert_eq!(members, all_effects);
            let removed: Vec<_> = members
                .iter()
                .filter(|id| {
                    effects
                        .iter()
                        .any(|effect| effect["support"] == true && effect["skill"] == **id)
                })
                .cloned()
                .collect();
            affected += usize::from(!removed.is_empty());
            removed_count += removed.len();
            assert_eq!(row["removed_skills"], json!(removed));
        } else {
            assert_eq!(row["schema_kind"], "unmapped");
            assert!(row["before_skills"].is_null());
            assert!(rows(&row["removed_skills"]).is_empty());
        }
    }
    assert_eq!(catalog["missing_references"], json!(missing));
    assert_eq!(
        bindings["census"],
        json!({"gems":966,"known_gems":known,
        "unmapped_gems":966-known,"affected_known_gems":affected,"removed_members":removed_count,
        "support_primary_gems":support_primary,"mixed_resolved_effect_gems":mixed,
        "skills":1436,"missing_reference_rows":missing.len()})
    );
    assert_eq!(
        (
            known,
            affected,
            removed_count,
            support_primary,
            mixed,
            missing.len()
        ),
        (619, 568, 568, 567, 81, 16)
    );
}

fn check_delta(bindings: &Value, dependencies: &Value, migration: &Value) {
    assert_eq!(rows(&dependencies["definitions"]).len(), 568);
    assert!(rows(&dependencies["slots"]).is_empty());
    let mut expected = Vec::new();
    for old in rows(&dependencies["definitions"]) {
        assert_eq!(old["kind"], "gem");
        let row = rows(&bindings["gems"])
            .iter()
            .find(|row| row["gem"] == old["value"]["id"])
            .unwrap();
        let removed = rows(&row["removed_skills"]);
        assert!(!removed.is_empty());
        assert_eq!(old["value"]["schema"]["kind"], "known");
        assert_eq!(
            old["value"]["schema"]["value"]["skills"],
            row["before_skills"]
        );
        let mut corrected = old.clone();
        corrected["value"]["schema"]["value"]["skills"]["members"]
            .as_array_mut()
            .unwrap()
            .retain(|skill| !removed.contains(skill));
        expected.push(json!({"kind":"definition","value":corrected}));
    }
    expected.sort_by(|a, b| {
        text(&a["value"]["value"]["id"]["key"]).cmp(text(&b["value"]["value"]["id"]["key"]))
    });
    assert!(
        expected
            .windows(2)
            .all(|pair| pair[0]["value"]["value"]["id"] != pair[1]["value"]["value"]["id"])
    );
    assert!(
        migration["schema"] == json!(expected),
        "only exact support members may be removed; every other field and closure stays exact"
    );
    assert_eq!(migration["schema_version"], 4);
    assert_eq!(migration["before"], bindings["before"]);
    assert_eq!(migration["release"], bindings["release"]);
    for field in ["owners", "tables", "receivers", "query_targets"] {
        assert!(rows(&migration[field]).is_empty());
    }
    assert!(migration.get("evaluation").is_none());
    let _: OwnedReleaseMigrationInput = serde_json::from_value(migration.clone()).unwrap();
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: Value = read("migration.json");
    assert_eq!(b["membership_semantics"], "owned-potential-skill-supply-v2");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["census"], b["census"]);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["registry_last_issued_before"], 13048);
    assert_eq!(a["registry_last_issued_after"], 13048);
    assert_eq!(a["allocated_definitions"], 0);
    assert_eq!(a["new_programs"], 0);
    assert_eq!(
        a["artifact_sha256"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration"]
    );
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    assert_eq!(
        rows(&a["tracked_inputs"])
            .iter()
            .map(|v| text(&v["path"]))
            .collect::<Vec<_>>(),
        [CATALOG, POLICY, MAPPING]
    );
    for input in rows(&a["tracked_inputs"]) {
        let bytes = fs::read(root().join(text(&input["path"]))).unwrap();
        assert_eq!(input["bytes"], bytes.len());
        assert_eq!(input["sha256"], hash(&bytes));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    let catalog = tracked(CATALOG);
    assert_eq!(catalog["capability"], "identity_only");
    assert_eq!(a["source_revision"], catalog["source"]["upstream_revision"]);
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let pins: Vec<_> = catalog["source"]["files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(path, sha)| json!({"path":path,"sha256":sha}))
        .collect();
    assert_eq!(a["source_files"], json!(pins));
    for pin in pins {
        let found: Vec<_> = rows(&manifest["files"])
            .iter()
            .filter(|v| v["path"] == pin["path"])
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["sha256"], pin["sha256"]);
    }
    assert_eq!(tracked(POLICY)["absent_support"], "non_support");
    check_census(&b, &catalog, &tracked(MAPPING));
    check_delta(&b, &d, &m);
    check_counterexamples(&b, &m);
}

fn check_counterexamples(bindings: &Value, migration: &Value) {
    let find = |suffix: &str| {
        rows(&bindings["gems"])
            .iter()
            .find(|row| text(&row["catalog_key"]).ends_with(suffix))
            .unwrap()
    };
    let remaining = |row: &Value| {
        let definition = rows(&migration["schema"])
            .iter()
            .find(|v| v["value"]["value"]["id"] == row["gem"])
            .unwrap();
        assert_eq!(
            definition["value"]["value"]["schema"]["value"]["skills"]["closure"],
            row["before_skills"]["closure"]
        );
        rows(&row["effects"])
            .iter()
            .filter(|effect| {
                rows(&definition["value"]["value"]["schema"]["value"]["skills"]["members"])
                    .contains(&effect["skill"])
            })
            .map(|v| text(&v["source_id"]).to_owned())
            .collect::<Vec<_>>()
    };
    for suffix in ["SkillGemBiddingSupportTwo", "SkillGemBiddingSupportThree"] {
        assert!(remaining(find(suffix)).is_empty());
    }
    assert_eq!(
        remaining(find("SkillGemBarbsSupport")),
        ["TriggeredBarbsPlayer"]
    );
    assert_eq!(
        remaining(find("SkillGemEmpoweredSparksSupport")),
        [
            "TriggeredSparkEmpowerPlayer",
            "TriggeredEmpoweredSparkPlayer"
        ]
    );
    for suffix in ["SkillGemMirageArcher", "SkillGemWolfPounce"] {
        let row = find(suffix);
        assert_eq!(row["schema_kind"], "unmapped");
        assert!(rows(&row["effects"]).iter().any(|v| v["support"] == true));
        assert!(rows(&row["effects"]).iter().any(|v| v["support"] != true));
        assert!(
            rows(&migration["schema"])
                .iter()
                .all(|v| v["value"]["value"]["id"] != row["gem"])
        );
    }
    let unresolved = find("SkillGemConcussiveRunesSupport");
    assert_eq!(unresolved["schema_kind"], "unmapped");
    assert_eq!(rows(&unresolved["missing_references"]).len(), 2);
}

pub fn rejects_non_support_loss_and_closure_promotion() {
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: Value = read("migration.json");
    for closure in [false, true] {
        let mut changed = m.clone();
        let row = changed["schema"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| {
                !rows(&v["value"]["value"]["schema"]["value"]["skills"]["members"]).is_empty()
            })
            .unwrap();
        let skills = &mut row["value"]["value"]["schema"]["value"]["skills"];
        if closure {
            skills["closure"] = json!({"kind":"complete"});
        } else {
            skills["members"] = json!([]);
        }
        assert!(std::panic::catch_unwind(|| check_delta(&b, &d, &changed)).is_err());
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
        "rules",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert!(prior.evaluation().is_none());
    let definitions = json!(prior.input().recipe.schema.definitions);
    let by_id: BTreeMap<_, _> = rows(&definitions)
        .iter()
        .filter(|d| d["kind"] == "gem")
        .map(|d| (text(&d["value"]["id"]["key"]), d))
        .collect();
    for row in rows(&b["gems"]) {
        let actual = by_id[text(&row["gem"]["key"])];
        assert_eq!(actual["value"]["id"], row["gem"]);
        assert_eq!(actual["value"]["schema"]["kind"], row["schema_kind"]);
        if row["schema_kind"] == "known" {
            assert_eq!(
                actual["value"]["schema"]["value"]["skills"],
                row["before_skills"]
            );
        }
    }
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    for definition in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|v| *v == definition)
                .count(),
            1
        );
    }
    let mapping = tracked(MAPPING);
    for entry in serde_json::from_value::<Vec<MappingEntry>>(mapping["entries"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|v| **v == entry)
                .count(),
            1
        );
    }
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("gem-executable-memberships-v1").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-gem-executable-memberships-v1",
            &(a, b, d, migration),
            8 * 1024 * 1024,
        )
        .unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().recipe.clone();
    for definition in old {
        let row = restored
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == definition.address())
            .unwrap();
        *row = definition;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only the 568 reviewed membership lists and bound release identities change"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.input().mapping.entries, prior.input().mapping.entries);
    assert_eq!(next.input().roles.roles, prior.input().roles.roles);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
