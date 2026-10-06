//! One source-bound derived inherent amount. No attribute or flag producer closes here.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::RuleProgram,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId},
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
pub const KIND: &str = "source-bound-strength-life";
const DOMAIN: &str = "owned-strength-life-v1";
pub const FIELDS: [&str; 5] = [
    "all_inherent_disabled",
    "strength_inherent_disabled",
    "strength_life_disabled",
    "inherent_doubled",
    "strength_life_halved",
];
const FLAGS: [&str; 5] = [
    "NoAttributeBonuses",
    "NoStrengthAttributeBonuses",
    "NoStrBonusToLife",
    "DoubledInherentAttributeBonuses",
    "HalvesLifeFromStrength",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/strength-life")
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
    let literal =
        |id: &str, value: Value| json!({"id":id,"expression":{"kind":"literal","value":value}});
    let select = |id: &str, c: &str, t: &str, f: &str| json!({"id":id,"expression":{"kind":"select","condition":c,"when_true":t,"when_false":f}});
    let mut reads = vec![
        json!({"id":"strength","value_type":{"kind":"integer"},"source":{"kind":"stat","value":{"entity":"current","stat":b["strength"]}}}),
    ];
    let mut nodes = vec![json!({"id":"strength","expression":{"kind":"read","input":"strength"}})];
    for field in FIELDS {
        let name = field.replace('_', "-");
        reads.push(json!({"id":name,"value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["inputs"][field]}}}));
        nodes.push(json!({"id":name,"expression":{"kind":"read","input":name}}));
    }
    nodes.extend([literal("normal-per-strength",q(2.)),literal("halved-per-strength",q(1.)),literal("zero",q(0.)),literal("single",json!({"kind":"integer","value":1})),literal("double",json!({"kind":"integer","value":2})),
        select("per-strength","strength-life-halved","halved-per-strength","normal-per-strength"),json!({"id":"base","expression":{"kind":"scale_integer","value":"per-strength","count":"strength"}}),
        select("multiplier","inherent-doubled","double","single"),json!({"id":"scaled","expression":{"kind":"scale_integer","value":"base","count":"multiplier"}}),
        select("life-specific","strength-life-disabled","zero","scaled"),select("life-strength","strength-inherent-disabled","zero","life-specific"),select("life","all-inherent-disabled","zero","life-strength")]);
    json!({"id":b["program"],"context":"actor","reads":reads,"nodes":nodes,"effects":[{"id":"life","when":null,"effect":{"kind":"derive","entity":"current","stat":b["amount"],"value":"life"}}]})
}
pub fn program() -> RuleProgram {
    serde_json::from_value(expected_program(&read("bindings.json"))).unwrap()
}
fn expected_receiver(b: &Value) -> Value {
    json!({"id":b["receiver"],"stat":b["amount"],"program":b["program"],"targets":[{"kind":"player"}]})
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (k, n) in [
        ("allocated_definitions", 6),
        ("new_programs", 1),
        ("new_receivers", 1),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x3314),
        ("registry_last_issued_after", 0x331a),
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
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-strength-life-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (6, 1, 1)
    );
    assert_eq!(b["program"], "inherent-strength-life");
    assert_eq!(b["receiver"], "player-inherent-strength-life");
    assert_eq!(b["strength"]["key"], "def.0000000000001d2e");
    assert_eq!(b["unit"]["key"], "def.0000000000003119");
    assert_eq!(b["amount"]["key"], "def.000000000000331a");
    for (i, k) in FIELDS.iter().enumerate() {
        assert_eq!(b["inputs"][k]["key"], format!("def.{:016x}", 0x3315 + i));
        assert_eq!(
            json!(m.schema[i]),
            json!({"kind":"definition","value":{"kind":"stat","value":{"id":b["inputs"][k],"schema":{"kind":"known","value":{"value":{"kind":"boolean"},"targets":["actor"]}}}}})
        );
        assert_eq!(
            v["source_inputs"][i],
            json!({"field":k,"source_flag":FLAGS[i]})
        );
    }
    assert_eq!(
        json!(m.schema[5]),
        json!({"kind":"definition","value":{"kind":"stat","value":{"id":b["amount"],"schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":b["unit"]}},"targets":["actor"]}}}}})
    );
    assert_eq!(
        json!(m.owners[0]),
        json!({"owner":{"kind":"definition","value":{"kind":"stat","value":b["amount"]}},"programs":{"members":[expected_program(&b)],"closure":{"kind":"complete"}}})
    );
    assert_eq!(json!(m.receivers[0]), expected_receiver(&b));
    assert_eq!(rows(&d["definitions"]).len(), 2);
    assert!(rows(&d["existing_owners_changed"]).is_empty());
    assert_eq!(b["normal_per_strength"], 2);
    assert_eq!(b["halved_per_strength"], 1);
    assert_eq!(b["doubled_multiplier"], 2);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for k in [
        "final_life",
        "attribute_contributor_closure",
        "flag_producer_closure",
        "whole_build_parity",
        "contribution_emission",
    ] {
        assert_eq!(b["scope"][k], false);
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
    source(&a, &v, false);
}
fn dependencies(endpoint: &StagedOwnedRelease) {
    let d: Value = read("dependencies.json");
    let defs: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    for definition in defs {
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
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let a: Value = read("authoring.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    for schema in m.schema {
        let value = json!(schema);
        let row: DefinitionDescriptor = serde_json::from_value(value["value"].clone()).unwrap();
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|x| **x == m.owners[0])
            .count(),
        1
    );
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|x| **x == m.receivers[0])
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    dependencies(prior);
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    source(&a, &v, true);
    let m: OwnedReleaseMigrationInput = read("migration.json");
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
    assert!(prior.evaluation().is_none());
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
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
    assert!(inverse == *migrated.input(), "provenance-only inverse");
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added: Vec<_> = (0..6)
        .map(|_| {
            registry
                .allocate_definition::<StatDefinition>()
                .unwrap()
                .address()
        })
        .collect();
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 6
    );
    restored
        .schema
        .definitions
        .retain(|d| !added.contains(&d.address()));
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    assert_eq!(
        restored.rules.owners.len(),
        prior.input().recipe.rules.owners.len() + 1
    );
    restored
        .rules
        .owners
        .retain(|o| o.owner != m.owners[0].owner);
    assert_eq!(
        restored.rules.receivers.members.len(),
        prior.input().recipe.rules.receivers.members.len() + 1
    );
    restored
        .rules
        .receivers
        .members
        .retain(|r| r.id != m.receivers[0].id);
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "only six new Stats, one pure derived owner and one Player receiver may differ"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn no_capture(mut host: Value) -> Value {
    let s = host["state"].as_object_mut().unwrap();
    s.remove("hooked");
    for mode in ["MAIN", "CALCS"] {
        s.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    host
}
fn projections(report: &Value) -> Value {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 13);
    assert!(
        cases[4]["original"] == cases[12]["original"],
        "fresh original replay differs"
    );
    json!(cases.iter().map(|c|{assert!(no_capture(c["original"].clone())==no_capture(c["unhooked"].clone()),"hooked/hookless output differs");let mut s=c["original"]["state"].clone();for mode in["MAIN","CALCS"]{s["modes"][mode].as_object_mut().unwrap().remove("player_output");}json!({"name":c["name"],"xml_sha256":c["xml_sha256"],"control_text":c["control_text"],"source_only_control":c["source_only_control"],"state":s})}).collect::<Vec<_>>())
}
fn source(a: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let pins = rows(&a["source_files"]);
    assert_eq!(pins.len(), 9);
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
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(text.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(text.as_bytes()), pin["sha256"]);
        }
    }
    let m = &v["report_metadata"];
    for (k, field) in [
        ("source_revision", "source_revision"),
        ("manifest_sha256", "source_manifest_sha256"),
        ("observer_sha256", "observer_sha256"),
        ("bootstrap_sha256", "bootstrap_sha256"),
    ] {
        assert_eq!(m[k], a[field]);
    }
    assert_eq!(m["case_count"], 13);
    assert_eq!(m["complete_loads_per_jit"], 26);
    for k in [
        "original_complete_player_stage",
        "original_attribute_function",
        "original_sum",
        "fresh_unhooked_comparison",
        "fresh_repeat",
        "parsed_control_inputs",
        "enabled_zero_distinct_from_disabled_absence",
    ] {
        assert_eq!(m["scope"][k], true);
    }
    for k in [
        "final_life",
        "attribute_contributor_closure",
        "flag_producer_closure",
        "game_obtainability_claim",
        "native_source_interpreter",
    ] {
        assert_eq!(m["scope"][k], false);
    }
    assert_eq!(rows(&m["files"]).len(), 9);
    for pin in pins {
        assert_eq!(
            rows(&m["files"])
                .iter()
                .filter(|x| x["path"] == pin["path"] && x["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    assert_eq!(rows(&v["projections"]).len(), 13);
    assert_eq!(rows(&v["native_cases"]).len(), 13);
    for (i, c) in rows(&v["projections"]).iter().enumerate() {
        let s = &c["state"];
        assert_eq!(s["hooked"], true);
        assert_eq!(s["methods"]["actor"]["first"], 264);
        assert_eq!(s["methods"]["attributes"]["first"], 233);
        for mode in ["MAIN", "CALCS"] {
            let r = &s["modes"][mode];
            let records = rows(&r["records"]);
            assert!(records.len() <= 1);
            assert_eq!(r["provenance"]["stage_strength"], r["strength"]);
            assert_eq!(r["provenance"]["stage_flags"], r["flags"]);
            assert_eq!(
                r["provenance"]["emitted_count"].as_u64().unwrap(),
                records.len() as u64
            );
            assert_eq!(r["provenance"]["exact_emitted_record"], records.len() == 1);
            let disabled = FLAGS[..3].iter().any(|f| r["flags"][f]["resolved"] == true);
            assert_eq!(
                records.is_empty(),
                disabled,
                "explicit source disable controls own absence"
            );
            if let Some(record) = records.first() {
                assert_eq!(record["source"], "Strength");
                assert_eq!(record["name"], "Life");
                assert_eq!(record["type"], "BASE");
                assert_eq!(record["value"], r["inherent_life"]);
            } else {
                assert_eq!(r["inherent_life"], 0);
            }
        }
        let r = &s["modes"]["MAIN"];
        let flags = FLAGS
            .map(|f| (f.to_owned(), r["flags"][f]["resolved"].clone()))
            .into_iter()
            .collect::<serde_json::Map<_, _>>();
        assert_eq!(
            v["native_cases"][i],
            json!({"case_index":i,"name":c["name"],"source_only_control":c["source_only_control"],"strength":r["strength"],"flags":flags,"emitted_count":rows(&r["records"]).len(),"inherent_life":r["inherent_life"]})
        );
    }
    assert_eq!(rows(&v["reports"]).len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    assert_eq!(v["reports"][0]["bytes"], v["reports"][1]["bytes"]);
    if !full {
        return;
    }
    for (field, path) in [
        (
            "observer_sha256",
            "crates/poe-optimizer-pob/tests/support/strength_life_source.lua",
        ),
        (
            "bootstrap_sha256",
            "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
        ),
    ] {
        assert_eq!(hash(&fs::read(root().join(path)).unwrap()), a[field]);
    }
    assert_eq!(rows(&m["original_sources"]).len(), 5);
    for (i, pin) in rows(&m["original_sources"]).iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                i + 1
            )
        );
        assert_eq!(
            hash(&fs::read(root().join(pin["path"].as_str().unwrap())).unwrap()),
            pin["sha256"]
        );
    }
    let mut first = None;
    for pin in rows(&v["reports"]) {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        if let Some(prior) = &first {
            assert!(&bytes == prior, "JIT reports differ");
        } else {
            first = Some(bytes.clone());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut metadata = report.clone();
        for k in ["cases", "native_cases"] {
            metadata.as_object_mut().unwrap().remove(k);
        }
        assert_eq!(metadata, *m);
        assert!(
            projections(&report) == v["projections"],
            "source projection differs"
        );
        assert_eq!(report["native_cases"], v["native_cases"]);
    }
}
