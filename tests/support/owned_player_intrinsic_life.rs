//! Exact class-owned intrinsic Life append; all other resource coverage remains open.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{DefinitionRules, RuleProgram},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
pub const KIND: &str = "source-bound-player-intrinsic-life";
const DOMAIN: &str = "owned-player-intrinsic-life-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-intrinsic-life")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Value) -> &[Value] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|x| x.is_empty()));
        &[]
    }
}
fn digest() -> OwnedContentDigest {
    let a: Vec<Value> = ["authoring", "bindings", "dependencies", "source-vectors"]
        .iter()
        .map(|n| read(&format!("{n}.json")))
        .collect();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    digest_owned(DOMAIN, &(a, m), 1024 * 1024).unwrap()
}
fn expected_program(b: &Value) -> Value {
    let q = |n: f64| json!({"kind":"quantity","value":{"value":n,"unit":b["unit"]}});
    json!({"id":b["program"],"context":"actor","reads":[{"id":"level","value_type":{"kind":"integer"},"source":{"kind":"character_level"}}],
        "nodes":[{"id":"level","expression":{"kind":"read","input":"level"}},
            {"id":"per-level","expression":{"kind":"literal","value":q(12.)}},
            {"id":"offset","expression":{"kind":"literal","value":q(16.)}},
            {"id":"leveled","expression":{"kind":"scale_integer","value":"per-level","count":"level"}},
            {"id":"life","expression":{"kind":"add","left":"leveled","right":"offset"}}],
        "effects":[{"id":"intrinsic-life","when":null,"effect":{"kind":"contribute","entity":"player","stat":b["life"],"contribution":"add","value":"life"}}]})
}
pub fn program() -> RuleProgram {
    serde_json::from_value(expected_program(&read("bindings.json"))).unwrap()
}
pub fn verify_source(full: bool) {
    source(
        &read("authoring.json"),
        &read("bindings.json"),
        &read("source-vectors.json"),
        full,
    );
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (k, n) in [
        ("allocated_definitions", 0),
        ("new_programs", 8),
        ("new_receivers", 0),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x3314),
        ("registry_last_issued_after", 0x3314),
    ] {
        assert_eq!(a[k], n);
    }
    for k in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[k], b[k]);
        assert_eq!(a[k], d["source"][if k == "before" { "input" } else { k }]);
    }
    assert_eq!(b["program"], "intrinsic-player-life");
    assert_eq!(b["life"]["key"], "def.000000000000311a");
    assert_eq!(b["unit"]["key"], "def.0000000000003119");
    assert_eq!(b["per_level"], 12);
    assert_eq!(b["offset"], 16);
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-player-intrinsic-life-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    assert_eq!(old.len(), 8);
    assert_eq!(m.owners.len(), 8);
    assert_eq!(rows(&d["definitions"]).len(), 10);
    assert_eq!(rows(&d["class_mapping"]).len(), 8);
    for (i, c) in rows(&b["classes"]).iter().enumerate() {
        assert_eq!(c["class"]["key"], format!("def.{:016x}", 0xa1e + i));
        let owner = json!({"kind":"definition","value":{"kind":"class","value":c["class"]}});
        assert_eq!(json!(old[i].owner), owner);
        assert!(!old[i].programs.is_complete());
        assert_eq!(old[i].programs.members.len(), 2);
        assert!(
            !old[i]
                .programs
                .members
                .iter()
                .any(|p| p.id.as_str() == "intrinsic-player-life")
        );
        assert_eq!(m.owners[i].owner, old[i].owner);
        assert_eq!(m.owners[i].programs.closure, old[i].programs.closure);
        assert_eq!(
            json!(m.owners[i].programs.members),
            json!([expected_program(&b)])
        );
        let descriptor = rows(&d["definitions"])
            .iter()
            .find(|x| x["value"]["id"] == c["class"])
            .unwrap();
        assert_eq!(
            descriptor["value"]["schema"]["value"]["level"],
            json!({"minimum":1,"maximum":100})
        );
        assert_eq!(d["class_mapping"][i]["outcome"]["value"]["target"], owner);
        assert_eq!(
            d["class_mapping"][i]["outcome"]["value"]["basis"],
            json!({"kind":"exact"})
        );
        assert_eq!(
            d["class_mapping"][i]["source"]["value"]["value"]["key"]["value"],
            c["source_id"].as_u64().unwrap().to_string()
        );
    }
    assert_eq!(rows(&b["classes"]).len(), 8);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for f in [
        "final_life",
        "incoming_contributor_closure",
        "whole_build_parity",
        "generic_level_multiplier_compatibility",
    ] {
        assert_eq!(b["scope"][f], false);
    }
    let hashes = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        hashes.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["bindings", "dependencies", "migration", "source-vectors"])
    );
    for (n, expected) in hashes {
        let bytes = fs::read(data().join(format!("{n}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*expected, hash(&bytes));
    }
    source(&a, &b, &v, false);
}
fn dependencies(endpoint: &StagedOwnedRelease) {
    let d: Value = read("dependencies.json");
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    for definition in definitions {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == definition)
                .count(),
            1
        );
    }
    let mapping = json!(endpoint.input().mapping);
    for row in rows(&d["class_mapping"]) {
        assert_eq!(
            rows(&mapping["entries"])
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let a: Value = read("authoring.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let d: Value = read("dependencies.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    for mut owner in old {
        owner.programs.members.push(program());
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| **x == owner)
                .count(),
            1
        );
        assert!(!owner.programs.is_complete());
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    dependencies(prior);
    let a: Value = read("authoring.json");
    verify_source(true);
    let d: Value = read("dependencies.json");
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for k in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[k], a[k]);
    }
    for owner in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| *x == owner)
                .count(),
            1
        );
    }
    assert!(prior.evaluation().is_none());
    let migrated =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert!(
        inverse == *migrated.input(),
        "provenance-only inverse differs"
    );
    let mut restored = next.input().recipe.clone();
    for owner in old {
        let target = restored
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == owner.owner)
            .unwrap();
        *target = owner;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "only eight exact class program appends"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn compact(host: &Value) -> Value {
    let mut s = host["state"].clone();
    for mode in ["MAIN", "CALCS"] {
        s["modes"][mode]
            .as_object_mut()
            .unwrap()
            .remove("player_output");
    }
    s
}
fn no_capture(mut host: Value) -> Value {
    let s = host["state"].as_object_mut().unwrap();
    s.remove("initializers");
    s.remove("hooked");
    for mode in ["MAIN", "CALCS"] {
        s.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    host
}
fn no_history(mut host: Value) -> Value {
    host["state"]
        .as_object_mut()
        .unwrap()
        .remove("initializers");
    host
}
fn projections(report: &Value) -> Value {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 10);
    assert!(
        cases[4]["original"] == cases[8]["original"],
        "fresh original repeat differs"
    );
    assert!(
        no_history(cases[4]["original"].clone()) == no_history(cases[9]["original"].clone()),
        "fresh/warm current state differs"
    );
    assert!(
        cases[9]["original"] == report["warm_repeat"],
        "independent warm replay differs"
    );
    json!(cases.iter().map(|c|{assert!(no_capture(c["original"].clone())==no_capture(c["unhooked"].clone()),"hooked/unhooked state differs");
        json!({"name":c["name"],"xml_sha256":c["xml_sha256"],"warm_xml_sha256":c["warm_xml_sha256"],"state":compact(&c["original"])})}).collect::<Vec<_>>())
}
fn source(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let pins = rows(&a["source_files"]);
    assert_eq!(pins.len(), 7);
    let mut paths = BTreeSet::new();
    for pin in pins {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|x| *x == pin)
                .count(),
            1
        );
        if full {
            let s = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(s.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(s.as_bytes()), pin["sha256"]);
        }
    }
    let m = &v["report_metadata"];
    assert_eq!(m["source_revision"], a["source_revision"]);
    assert_eq!(m["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(m["observer_sha256"], a["observer_sha256"]);
    assert_eq!(m["bootstrap_sha256"], a["bootstrap_sha256"]);
    assert_eq!(m["case_count"], 10);
    assert_eq!(m["complete_loads_per_jit"], 24);
    for field in [
        "original_initializer",
        "original_eval_mod",
        "fresh_unhooked_comparison",
        "fresh_repeat",
        "warm_repeat",
        "independent_warm_repeat",
        "warm_comparison_ignores_initializer_history_only",
    ] {
        assert_eq!(m["scope"][field], true);
    }
    for field in [
        "final_life",
        "contributor_closure",
        "generic_level_multiplier_native_admission",
    ] {
        assert_eq!(m["scope"][field], false);
    }
    assert_eq!(rows(&m["files"]).len(), 7);
    for pin in pins {
        assert_eq!(
            rows(&m["files"])
                .iter()
                .filter(|x| x["path"] == pin["path"] && x["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let values = [1168, 1072, 1132, 1168, 1120, 28, 1108, 1216, 1120, 1120];
    let levels = [96, 88, 93, 96, 92, 1, 91, 100, 92, 92];
    assert_eq!(rows(&v["projections"]).len(), 10);
    assert_eq!(rows(&v["native_cases"]).len(), 10);
    for (i, c) in rows(&v["projections"]).iter().enumerate() {
        let s = &c["state"];
        assert_eq!(s["constant"], 12);
        assert_eq!(s["hooked"], true);
        let mut admitted = BTreeSet::new();
        for class in rows(&b["classes"]) {
            let id = class["source_id"].as_u64().unwrap();
            assert_eq!(
                rows(&s["classes"]).iter().filter(|x| x["id"] == id).count(),
                1
            );
            admitted.insert(id);
        }
        for mode in ["MAIN", "CALCS"] {
            let r = &s["modes"][mode];
            assert!(admitted.contains(&r["class_id"].as_u64().unwrap()));
            assert_eq!(r["character_level"], levels[i]);
            assert_eq!(r["effective_value"], values[i]);
            assert_eq!(r["base_source_sum"], values[i]);
            assert_eq!(
                r["record"],
                json!({"name":"Life","type":"BASE","value":12,"source":"Base","flags":0,"keyword_flags":0,"tags":[{"type":"Multiplier","var":"Level","base":16}]})
            );
            assert_eq!(r["multiplier"]["effective"], levels[i]);
            assert_eq!(r["multiplier"]["extra_base"], 0);
            assert_eq!(r["multiplier"]["override"], json!({"present":false}));
            for parent in rows(&r["multiplier"]["read_set"]) {
                assert!(rows(&parent["multiplier_level"]).is_empty());
            }
            assert_eq!(r["provenance"]["exact_initialized_record"], true);
            assert_eq!(r["provenance"]["initialized_constant"], 12);
            assert_eq!(r["provenance"]["initialized_stored_level"], levels[i]);
        }
        let r = &s["modes"]["MAIN"];
        assert_eq!(
            v["native_cases"][i],
            json!({"case_index":i,"name":c["name"],"class_name":r["class_name"],"character_level":r["character_level"],"intrinsic_life":r["effective_value"]})
        );
    }
    assert_eq!(rows(&v["reports"]).len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    assert_eq!(v["reports"][0]["bytes"], v["reports"][1]["bytes"]);
    // The same initializer appears after, not inside, the class attribute loop.
    // A source-specific generic Level override is not a native CharacterLevel input.
    let branch = &v["initializer_branch"];
    assert_eq!(branch["path"], "src/Modules/CalcSetup.lua");
    assert_eq!(branch["first"], 828);
    assert_eq!(branch["last"], 837);
    assert_eq!(branch["class_dependent_life_branch"], false);
    let text = branch["text"].as_str().unwrap();
    assert!(text.contains("for _, stat in pairs({\"Str\",\"Dex\",\"Int\"}) do"));
    let loop_end = text.find("\n\t\tend\n").unwrap();
    let life = text.find("modDB:NewMod(\"Life\", \"BASE\"").unwrap();
    assert!(loop_end < life);
    assert!(!text[loop_end..life].contains("classStats"));
    assert_eq!(
        v["compatibility_boundary"]["global_source_override_impossible"],
        false
    );
    if !full {
        return;
    }
    for (field, path) in [
        (
            "observer_sha256",
            "crates/poe-optimizer-pob/tests/support/player_intrinsic_life_source.lua",
        ),
        (
            "bootstrap_sha256",
            "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
        ),
    ] {
        assert_eq!(hash(&fs::read(root().join(path)).unwrap()), a[field]);
    }
    assert_eq!(rows(&m["original_sources"]).len(), 5);
    for (index, pin) in rows(&m["original_sources"]).iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                index + 1
            )
        );
        assert_eq!(
            hash(&fs::read(root().join(pin["path"].as_str().unwrap())).unwrap()),
            pin["sha256"]
        );
    }
    let setup =
        fs::read_to_string(root().join("vendor/path-of-building-poe2/src/Modules/CalcSetup.lua"))
            .unwrap()
            .replace("\r\n", "\n");
    let excerpt = setup
        .lines()
        .skip(827)
        .take(10)
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(excerpt, branch["text"]);
    let mut first = None;
    for pin in rows(&v["reports"]) {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        if let Some(prior) = &first {
            assert!(&bytes == prior, "independent JIT reports differ");
        } else {
            first = Some(bytes.clone());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut metadata = report.clone();
        for field in ["cases", "native_cases", "warm_repeat"] {
            metadata.as_object_mut().unwrap().remove(field);
        }
        assert_eq!(metadata, *m);
        assert!(
            projections(&report) == v["projections"],
            "compact source projection differs"
        );
        assert_eq!(report["native_cases"], v["native_cases"]);
    }
}
