//! Source-authenticated preparation data only; no release or coverage mutation.
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SchemaState},
    owned_supports::{SupportPreparationDefinition, SupportPreparationInput, SupportTypePredicate},
};
use poe_optimizer_data::owned_supports::OwnedSupportPreparation;
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/djinn-support-preparation")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn bindings() -> Value {
    read("bindings.json")
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
// The original Lua serializer represents an empty sequence as an empty object.
fn rows(value: &Value) -> &[Value] {
    match value {
        Value::Array(values) => values,
        Value::Object(values) if values.is_empty() => &[],
        _ => panic!("expected a source sequence"),
    }
}
fn type_key(bindings: &Value, source: &Value) -> OwnedDefinitionKey {
    let matches: Vec<_> = rows(&bindings["types"])
        .iter()
        .filter(|row| row["source_id"] == source["id"] && row["source_name"] == source["name"])
        .collect();
    assert_eq!(matches.len(), 1);
    key(matches[0]["support_type"].as_str().unwrap())
}
fn predicate(bindings: &Value, expression: &Value) -> Option<SupportTypePredicate> {
    assert_eq!(expression["present"], true);
    let source = rows(&expression["values"]);
    // The only reviewed operator expression is Hulking Minions' exact
    // conjunction. Translate that recipe directly, not through an interpreter.
    if source.last().is_some_and(|v| v["name"] == "AND") {
        assert_eq!(
            expression["values"],
            json!([
                {"id":6,"name":"Minion"},
                {"id":140,"name":"Persistent"},
                {"id":74,"name":"AND"},
            ])
        );
        return Some(SupportTypePredicate::All(
            source[..2]
                .iter()
                .map(|v| SupportTypePredicate::Type(type_key(bindings, v)))
                .collect(),
        ));
    }
    let mut values: Vec<_> = source
        .iter()
        .map(|v| SupportTypePredicate::Type(type_key(bindings, v)))
        .collect();
    // The other reviewed rows contain only positive type tokens. The pinned source
    // matches their residual stack with OR; no source expression interpreter.
    match values.len() {
        0 => None,
        1 => values.pop(),
        _ => Some(SupportTypePredicate::Any(values)),
    }
}
fn source_flag(source: &Value, field: &str) -> bool {
    match source.get(field) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        _ => panic!("reviewed source flag must be absent or Boolean"),
    }
}

/// Checks only versioned, checked-in material; ordinary CI needs no local runs.
pub fn check_authored() {
    let authoring: Value = read("authoring.json");
    let bindings = bindings();
    let vectors: Value = read("source-vectors.json");
    let preparation: SupportPreparationInput = read("preparation.json");
    assert_eq!(preparation.schema_version, 1);
    assert_eq!(json!(preparation.definitions), authoring["definitions"]);
    assert_eq!(json!(preparation.rules), authoring["rules"]);
    assert_eq!(json!(preparation.quality_unit), bindings["quality_unit"]);
    assert_eq!(authoring["registry_last_issued"], 0x32df);
    assert_eq!(authoring["allocated_definitions"], 0);
    assert_eq!(authoring["release_files_changed"], 0);
    assert_eq!(authoring["whole_builds_complete"], 0);
    for field in ["before", "definitions", "rules", "roles"] {
        assert_eq!(authoring[field], bindings[field]);
    }
    let names: Vec<_> = authoring["artifact_sha256"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["bindings", "preparation", "source-vectors"]);
    for name in names {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(authoring["artifact_sha256"][name], sha(&bytes));
    }
    assert!(
        !fs::read(data().join("authoring.json"))
            .unwrap()
            .contains(&b'\r')
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(authoring["source_manifest_sha256"], sha(&manifest_bytes));
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    let mut pinned = BTreeSet::new();
    for pin in rows(&authoring["source_files"]) {
        assert!(pinned.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|v| v["path"] == pin["path"] && v["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    assert_eq!(pinned.len(), 20);
    for original in rows(&authoring["original_sources"]) {
        assert_eq!(
            original["sha256"],
            sha(&fs::read(root().join(original["path"].as_str().unwrap())).unwrap())
        );
    }
    assert_eq!(rows(&authoring["original_sources"]).len(), 5);
    for (path, hash) in authoring["topology_sha256"].as_object().unwrap() {
        assert_eq!(hash, &json!(sha(&fs::read(root().join(path)).unwrap())));
    }
    assert_eq!(authoring["topology_sha256"].as_object().unwrap().len(), 2);
    let source_validation = &authoring["source_validation"];
    assert_eq!(source_validation["status"], "passed");
    assert_eq!(source_validation["cases"], 12);
    assert_eq!(source_validation["contexts"], 1296);
    assert_eq!(
        source_validation["lifecycle_stages"],
        json!(["fresh", "rebuilt_once", "rebuilt_twice"])
    );
    for field in [
        "numerical_delivery_authority",
        "native_inventory_authority",
        "native_build_parity",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(source_validation[field], false);
    }
    let mut source_ids = BTreeSet::new();
    let mut source_names = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    for row in rows(&bindings["types"]) {
        assert!(source_ids.insert(row["source_id"].as_u64().unwrap()));
        assert!(source_names.insert(row["source_name"].as_str().unwrap()));
        assert!(symbols.insert(key(row["support_type"].as_str().unwrap())));
    }
    assert_eq!(symbols.len(), 34);
    assert!(!source_names.contains("AND"));
    assert_eq!(symbols, preparation.types.iter().cloned().collect());
    assert_eq!(preparation.supports.len(), 10);
    assert_eq!(preparation.effects.len(), 10);
    assert_eq!(preparation.families.len(), 8);
    assert_eq!(rows(&bindings["supports"]).len(), 10);
    assert_eq!(rows(&vectors["source_definitions"]).len(), 10);
    for row in rows(&bindings["supports"]) {
        let source = rows(&vectors["source_definitions"])
            .iter()
            .find(|s| s["effect"] == row["source_effect"])
            .unwrap();
        assert_eq!(source["family_present"], true);
        assert_eq!(source["family"], row["source_families"]);
        let expected = SupportPreparationDefinition {
            effect: key(row["effect"].as_str().unwrap()),
            families: Some(
                rows(&row["families"])
                    .iter()
                    .map(|s| key(s.as_str().unwrap()))
                    .collect(),
            ),
            plus_version_of: None,
            requires: predicate(&bindings, &source["require_types"]),
            excludes: predicate(&bindings, &source["exclude_types"]),
            added_types: rows(&source["add_types"]["values"])
                .iter()
                .map(|s| type_key(&bindings, s))
                .collect(),
            gems_only: source_flag(source, "support_gems_only"),
            from_item: source_flag(source, "from_item"),
            is_support: source["support"].as_bool().unwrap(),
            is_trigger: source_flag(source, "is_trigger"),
            ignore_minion_types: source_flag(source, "ignore_minion_types"),
        };
        let actual = preparation
            .supports
            .iter()
            .find(|p| json!(p.gem) == row["gem"])
            .unwrap();
        assert_eq!(actual.preparation, SchemaState::Known(expected));
        let declaration = rows(&vectors["source_declarations"])
            .iter()
            .find(|s| s["source_effect"] == row["source_effect"])
            .unwrap();
        let block = declaration["declaration"].as_str().unwrap();
        assert!(!block.contains("plusVersionOf"));
        assert_eq!(declaration["normalized_lf_sha256"], sha(block.as_bytes()));
        let dependency = rows(&vectors["dependency_definitions"])
            .iter()
            .find(|d| d["value"]["id"] == row["gem"])
            .unwrap();
        let schema = &dependency["value"]["schema"];
        assert_eq!(schema["kind"], "known");
        assert_eq!(schema["value"]["roles"], json!(["support_assignment"]));
        assert_eq!(schema["value"]["skills"]["closure"]["kind"], "partial");
        assert_eq!(
            schema["value"]["declarations"]["parameters"]["closure"]["kind"],
            "partial"
        );
    }
    assert_eq!(rows(&vectors["dependency_definitions"]).len(), 11);
    assert_eq!(vectors["sample"]["case"], "original-05");
    assert_eq!(vectors["sample"]["stage"], "fresh");
    assert_eq!(rows(&vectors["sample"]["contexts"]).len(), 48);
}

/// Full optional source evidence; callers need the authenticated local witness run.
pub fn source_report() -> Value {
    let authoring: Value = read("authoring.json");
    let evidence = &authoring["source_validation"];
    let mut reports = Vec::new();
    for (path, count, digest) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes = fs::read(root().join(evidence[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, evidence[count].as_u64().unwrap());
        assert_eq!(sha(&bytes), evidence[digest].as_str().unwrap());
        reports.push(bytes);
    }
    assert_eq!(reports[0], reports[1]);
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    for (report_name, authored_name) in [
        ("source_revision", "source_revision"),
        ("manifest_sha256", "source_manifest_sha256"),
        ("files", "source_files"),
        ("original_sources", "original_sources"),
    ] {
        assert_eq!(report[report_name], authoring[authored_name]);
    }
    for field in [
        "business_wrappers",
        "source_tables_mutated",
        "numerical_delivery_authority",
        "native_inventory_authority",
        "native_build_parity",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(report[field], false);
    }
    let cases = rows(&report["cases"]);
    let expected = [
        "original-01",
        "original-02",
        "original-03",
        "original-04",
        "original-05",
        "disable-sand-bidding",
        "disable-sand-magnified-area",
        "disable-sand-muster",
        "disable-water-bidding",
        "disable-water-muster",
        "disable-water-frost-nexus",
        "repeat-original-05",
    ];
    assert_eq!(cases.len(), expected.len());
    assert_eq!(report["lifecycle_stages"], evidence["lifecycle_stages"]);
    let vectors: Value = read("source-vectors.json");
    let mut context_count = 0;
    for (case, name) in cases.iter().zip(expected) {
        assert_eq!(case["name"], name);
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let p = &case["states"][stage]["preparation"];
            assert_eq!(p["definitions"], vectors["source_definitions"]);
            assert_eq!(p["original_methods_preserved"], true);
            assert_eq!(p["hook_removed"], true);
            assert_eq!(p["jit_mode_preserved"], true);
            context_count += rows(&p["contexts"]).len();
        }
    }
    assert_eq!(context_count, 1296);
    for (case, original) in cases[..5].iter().zip(rows(&authoring["original_sources"])) {
        assert_eq!(case["xml_sha256"], original["sha256"]);
    }
    assert_eq!(cases[11]["xml_sha256"], cases[4]["xml_sha256"]);
    let original_contexts = rows(&cases[4]["states"]["fresh"]["preparation"]["contexts"]);
    let sampled: Vec<_> = original_contexts
        .iter()
        .map(|context| {
            let parent = context["parent_context"]
                .as_u64()
                .map(|index| &original_contexts[index as usize - 1]);
            let candidates: Vec<_> = rows(&context["candidates"])
                .iter()
                .map(|candidate| {
                    json!({
                        "effect":candidate["effect"], "source":candidate["source"],
                        "accepted":candidate["accepted"],
                    })
                })
                .collect();
            json!({
                "context":context["index"], "mode":context["mode"],
                "effect":context["effect"], "group":context["group"],
                "parent_context":context["parent_context"],
                "initial_inputs":{
                    "definition":context["initial_definition"],
                    "flags":context["constructor_flags"],
                    "parent_prepared_types":parent.map(|p| &p["constructor_types"]),
                },
                "observed":{
                    "constructor_types":context["constructor_types"],
                    "final_types":context["final_types"],
                    "candidate_occurrences":candidates,
                },
            })
        })
        .collect();
    assert_eq!(json!(sampled), vectors["sample"]["contexts"]);
    for declaration in rows(&vectors["source_declarations"]) {
        let path = declaration["path"].as_str().unwrap();
        let bytes = fs::read(root().join("vendor/path-of-building-poe2").join(path)).unwrap();
        let pin = rows(&authoring["source_files"])
            .iter()
            .find(|p| p["path"] == path)
            .unwrap();
        // The pinned source contract normalizes CRLF checkout bytes to LF.
        let normalized = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
        assert_eq!(sha(normalized.as_bytes()), pin["sha256"].as_str().unwrap());
        let block = declaration["declaration"].as_str().unwrap();
        let start = normalized.find(block).unwrap();
        assert_eq!(
            normalized[..start].bytes().filter(|c| *c == b'\n').count() + 1,
            declaration["first_line"].as_u64().unwrap() as usize
        );
    }
    report
}

/// Loads an independent artifact against the exact published dependency set.
/// The release is borrowed, and neither its completeness nor its bytes change.
pub fn load(release: &StagedOwnedRelease) -> OwnedSupportPreparation {
    check_authored();
    let _ = source_report();
    let authoring: Value = read("authoring.json");
    assert_eq!(json!(release.receipt().input), authoring["before"]);
    assert_eq!(
        json!(release.receipt().definitions),
        authoring["definitions"]
    );
    assert_eq!(json!(release.receipt().rules), authoring["rules"]);
    assert_eq!(json!(release.receipt().registry), authoring["registry"]);
    assert_eq!(json!(release.receipt().roles), authoring["roles"]);
    assert_eq!(
        json!(release.receipt().normalization),
        authoring["normalization"]
    );
    assert!(release.evaluation().is_none());
    assert_eq!(release.receipt().query_rows, 110);
    assert_eq!(
        json!(release.receipt().artifacts),
        authoring["release_artifacts"]
    );
    let artifacts: Vec<_> = release.artifacts().collect();
    assert_eq!(artifacts.len(), 18);
    for (name, bytes) in artifacts {
        if name == "release.json" {
            assert_eq!(bytes, serde_json::to_vec(release.receipt()).unwrap());
            continue;
        }
        let expected = rows(&authoring["release_artifacts"])
            .iter()
            .find(|a| a["file"] == name)
            .unwrap();
        assert_eq!(expected["bytes"], bytes.len());
        assert_eq!(expected["sha256"], sha(bytes));
    }
    let vectors: Value = read("source-vectors.json");
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(vectors["dependency_definitions"].clone()).unwrap();
    for dependency in dependencies {
        assert!(
            release
                .assembled()
                .schema()
                .input()
                .definitions
                .contains(&dependency)
        );
    }
    let bindings = bindings();
    for row in rows(&bindings["supports"]) {
        let role = release
            .roles()
            .input()
            .roles
            .iter()
            .find(|r| json!(r.gem) == row["gem"])
            .unwrap();
        assert_eq!(
            json!(role.primary),
            json!({"kind":"known","value":row["catalog_primary"]})
        );
        assert_eq!(
            json!(role.role),
            json!({"kind":"known","value":"support_assignment"})
        );
        assert_eq!(json!(role.materialization), json!({"kind":"physical"}));
    }
    OwnedSupportPreparation::new(
        read("preparation.json"),
        release.assembled().schema(),
        release.assembled().rules(),
        Default::default(),
    )
    .unwrap()
}
