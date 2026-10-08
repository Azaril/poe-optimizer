//! One source-data obligation retired; no numerical or import behavior changes.
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
};
use poe_optimizer_import::{
    owned_modifier_value_recipe::{ModifierValuePolicy, compile_owned_modifier_values},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-flat-life-scalability";
const REMOVED: &str = "numeric-component-scalability-unproved";
const FILES: [&str; 5] = [
    "authoring.json",
    "bindings.json",
    "dependencies.json",
    "coverage.json",
    "source-vectors.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/flat-life-scalability")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = FILES.map(read).into();
    digest_owned("owned-flat-life-scalability-v1", &values, 1024 * 1024).unwrap()
}
pub fn expected_owner(after: bool) -> DefinitionRules {
    let d: Value = read("dependencies.json");
    let c: Value = read("coverage.json");
    let mut owner: DefinitionRules = serde_json::from_value(d["owner"].clone()).unwrap();
    if after {
        owner.programs.closure = serde_json::from_value(c["after"].clone()).unwrap();
    }
    owner
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("coverage.json");
    let v: Value = read("source-vectors.json");
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    assert_eq!(a["registry_last_issued_before"], 0x3352);
    assert_eq!(a["registry_last_issued_after"], 0x3352);
    assert_eq!(b["modifier"]["key"], "def.0000000000003100");
    assert_eq!(b["retired_code"], REMOVED);
    assert_eq!(
        c["owner"],
        json!({"kind":"definition","value":{"kind":"modifier","value":b["modifier"]}})
    );
    assert_eq!(c["owner"], d["owner"]["owner"]);
    assert_eq!(c["before"], d["owner"]["programs"]["closure"]);
    let before = c["before"]["value"]["gaps"].as_array().unwrap();
    let after = c["after"]["value"]["gaps"].as_array().unwrap();
    assert_eq!(c["before"]["kind"], "partial");
    assert_eq!(c["after"]["kind"], "partial");
    assert_eq!((before.len(), after.len()), (7, 6));
    assert_eq!(before.iter().filter(|g| g["code"] == REMOVED).count(), 1);
    assert_eq!(
        before.iter().find(|g| g["code"] == REMOVED),
        Some(&c["retired"])
    );
    assert_eq!(
        after,
        &before
            .iter()
            .filter(|g| g["code"] != REMOVED)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        after
            .iter()
            .map(|g| g["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "source-encoding-and-corrupted-range-unproved",
            "remaining-ordered-magnitude-transforms-unconverted",
            "canonical-input-admission-unproved",
            "ordinary-item-routing-unconverted",
            "amulet-bonus-copy-unconverted",
            "external-contributor-membership-unconverted",
        ]
    );
    assert_eq!(
        d["owner"]["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "catalyst-scalar",
            "ordered-magnitude-scalar",
            "corrupted-base-factor",
            "contribute-player-flat-life",
            "effective-amount",
        ]
    );
    assert_eq!(
        expected_owner(false).programs.members,
        expected_owner(true).programs.members
    );
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    assert_eq!(
        b["scope"],
        json!({"retired_obligations":1,"retained_obligations":6,"closed_existing_rule_owners":0,"new_definitions":0,"new_programs":0,"numerical_program_changes":0,"import_guard_changes":0,"source_admission_widened":false,"routing_completed":false,"whole_build_parity":false,"final_life":false})
    );
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    check_admission(&json!([d["item_rule"]]), &json!([d["source_condition"]]));
    check_evidence(&a, &b, &v, false);
}

/// The census covers every current item rule, including rules unused by the five
/// fixtures. A new emitter requires its own reviewed component metadata.
pub fn check_admission(rules: &Value, conditions: &Value) {
    let d: Value = read("dependencies.json");
    let b: Value = read("bindings.json");
    let emitters: Vec<_> = rules
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["emissions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "modifier" && e["value"]["definition"] == b["modifier"])
        })
        .collect();
    assert_eq!(emitters, [&d["item_rule"]]);
    let guards: Vec<_> = conditions
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["rule"] == "fixed-life")
        .collect();
    assert_eq!(guards, [&d["source_condition"]]);
    assert_eq!(
        d["source_condition"],
        json!({"rule":"fixed-life","all":[
            {"kind":"no_source_scaling_tags"},{"kind":"initial_scaling_is_one"},
            {"kind":"no_generated_buff_members"},{"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
        ]})
    );
    let rule = &d["item_rule"];
    assert_eq!(rule["emissions"].as_array().unwrap().len(), 1);
    assert_eq!(rule["captures"].as_array().unwrap().len(), 1);
    assert_eq!(rule["captures"][0]["id"], "amount");
    assert_eq!(
        rule["pattern"],
        json!([
            {"kind":"literal","value":"+"},
            {"kind":"numeric_capture","value":{"capture":"amount","syntax":"decimal","sign":"forbidden"}},
            {"kind":"literal","value":" to maximum Life"}
        ])
    );
    let rolls = rule["emissions"][0]["value"]["rolls"].as_array().unwrap();
    let projected: Vec<_> = rolls
        .iter()
        .filter(|r| r["value"]["kind"] == "numeric_projection")
        .collect();
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0]["slot"]["slot"]["key"], "def.0000000000003101");
    assert_eq!(
        projected[0]["value"]["value"]["source"],
        json!({"kind":"capture","value":"amount"})
    );
    let unscalable = rolls
        .iter()
        .find(|r| r["slot"]["slot"]["key"] == "def.0000000000003116")
        .unwrap();
    assert_eq!(
        unscalable["value"],
        json!({"kind":"literal","value":{"kind":"boolean","value":false}})
    );
}

fn dependencies(endpoint: &StagedOwnedRelease, after: bool) {
    let owner = expected_owner(after);
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| **o == owner)
            .count(),
        1
    );
    let items = json!(endpoint.input().items);
    let source = json!(endpoint.input().item_source);
    let dialect = source["dialect"].as_object().unwrap();
    assert_eq!(dialect.len(), 1);
    check_admission(
        &items["rules"],
        &dialect.values().next().unwrap()["single_modifier_conditions"],
    );
    // Use the existing native authoring compiler, not a copied formatting law.
    // Its idempotent path rejects any difference from the actual current writer.
    let v: Value = read("source-vectors.json");
    let factor_unit = owner
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "effective-amount")
        .unwrap()
        .reads[1]
        .value_type
        .clone();
    let policy = ModifierValuePolicy {
        schema_version: 1,
        version: OwnedDefinitionKey::new("flat-life-scalability-authentication").unwrap(),
        definitions: endpoint.receipt().definitions.clone(),
        factor_unit: serde_json::from_value(json!(factor_unit)["value"]["unit"].clone()).unwrap(),
        bindings: vec![serde_json::from_value(v["numeric_binding"].clone()).unwrap()],
    };
    let compiled =
        compile_owned_modifier_values(endpoint.assembled(), &policy, Default::default()).unwrap();
    assert_eq!(compiled.extension.owners.len(), 1);
    let generated = &compiled.extension.owners[0];
    assert_eq!(generated.owner, owner.owner);
    assert_eq!(generated.programs.closure, owner.programs.closure);
    assert_eq!(generated.programs.members.len(), 1);
    assert_eq!(
        owner
            .programs
            .members
            .iter()
            .filter(|p| **p == generated.programs.members[0])
            .count(),
        1
    );
}

pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(json!(endpoint.receipt().definitions), a["definitions"]);
    assert_eq!(json!(endpoint.receipt().registry), a["registry"]);
    dependencies(endpoint, true);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let v: Value = read("source-vectors.json");
    check_evidence(&a, &b, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    dependencies(prior, false);
    assert!(prior.evaluation().is_none());
    let old = expected_owner(false);
    let mut input = prior.input().clone();
    *input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap() = expected_owner(true);
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    let restored = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap();
    *restored = old;
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "only the exact scalability gap and authenticated provenance may change"
    );
    let next_receipt = json!(next.receipt());
    for field in [
        "definitions",
        "registry",
        "routing",
        "mapping",
        "roles",
        "normalization",
        "rewards",
        "items",
        "item_source",
        "tree",
        "query_sets",
        "query_rows",
        "query_policy_bytes",
    ] {
        assert_eq!(receipt[field], next_receipt[field], "unchanged {field}");
    }
    assert_ne!(prior.receipt().input, next.receipt().input);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

fn check_evidence(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    assert_eq!(v["source_hash"], a["source_manifest_sha256"]);
    assert_eq!(
        v["evidence"]["manifest_sha256"],
        a["source_manifest_sha256"]
    );
    assert_eq!(v["evidence"]["native_parity"], false);
    assert_eq!(v["evidence"]["whole_life_metric_coverage"], false);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 8);
    assert_eq!(v["evidence"]["files"].as_array().unwrap().len(), 8);
    let mut unique = BTreeSet::new();
    for pin in pins {
        assert!(unique.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| *p == pin)
                .count(),
            1
        );
        assert_eq!(
            v["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
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
    let lines = v["source_lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        json!({"path":"src/Data/ModScalability.lua","first":434,"last":434,"lines":["\t[\"# to maximum Life\"] = { { isScalable = true } },"]})
    );
    assert_eq!(
        (lines[1]["first"].as_u64(), lines[1]["last"].as_u64()),
        (Some(275), Some(278))
    );
    assert_eq!(lines[1]["path"], "src/Modules/ItemTools.lua");
    if full {
        for row in lines {
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(row["path"].as_str().unwrap()),
            )
            .unwrap();
            let first = row["first"].as_u64().unwrap() as usize;
            let last = row["last"].as_u64().unwrap() as usize;
            assert_eq!(
                json!(
                    text.lines()
                        .skip(first - 1)
                        .take(last - first + 1)
                        .collect::<Vec<_>>()
                ),
                row["lines"]
            );
        }
    }
    let numeric = &v["numeric_binding"];
    assert_eq!(numeric["modifier"], b["modifier"]);
    assert_eq!(numeric["input"]["slot"]["key"], "def.0000000000003101");
    assert_eq!(numeric["scaling"]["kind"], "scaled");
    assert_eq!(numeric["precision"], 1);
    assert_eq!(numeric["display_precision"], 0);
    assert_eq!(
        v["numeric_binding_path"],
        "data/owned/poe2/3887ae68/flat-life/numeric-binding.json"
    );
    let numeric_bytes = fs::read(root().join(v["numeric_binding_path"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&numeric_bytes), v["numeric_binding_sha256"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&numeric_bytes).unwrap(),
        *numeric
    );
    let d: Value = read("dependencies.json");
    let dependencies = v["dependencies"].as_array().unwrap();
    assert_eq!(dependencies.len(), 3);
    for (pin, expected) in
        dependencies
            .iter()
            .zip([numeric, &d["item_rule"], &d["source_condition"]])
    {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        assert_eq!(&serde_json::from_slice::<Value>(&bytes).unwrap(), expected);
    }
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for (case, (build, variant, physical_count, probe_count)) in cases.iter().zip([
        (1, "original", 1, 0),
        (2, "original", 8, 0),
        (3, "original", 1, 0),
        (4, "original", 7, 0),
        (5, "original", 7, 23),
        (5, "overlay-zero", 7, 0),
        (5, "overlay-one", 7, 0),
    ]) {
        assert_eq!(case["build"], build);
        assert_eq!(case["variant"], variant);
        assert_eq!(case["scalability"], json!([{"isScalable":true}]));
        assert_eq!(case["physical_line_count"], physical_count);
        assert_eq!(case["probe_count"], probe_count);
        if variant == "original" {
            let xml = fs::read(root().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{build:02}.xml"
            )))
            .unwrap();
            assert_eq!(hash(&xml), case["xml_sha256"]);
        }
    }
    let observations = v["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 23);
    assert_eq!(
        v["observation_fields"],
        json!(["name", "input_line", "active"])
    );
    let names: BTreeSet<_> = observations
        .iter()
        .map(|r| r["value"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 23);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    for (i, pin) in reports.iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "runs/owned-flat-life-source-01/source-jit-{}.json",
                if i == 0 { "off" } else { "on" }
            )
        );
        assert_eq!(pin["bytes"], 397880);
        assert_eq!(
            pin["sha256"],
            "3bbf0ef76d795f8477a16554db6aefdd1ad9bb320c4ad466460330d862c65b61"
        );
    }
    if !full {
        return;
    }
    let mut first: Option<Value> = None;
    for pin in reports {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["source_hash"], v["source_hash"]);
        assert_eq!(report["evidence"], v["evidence"]);
        assert_eq!(report["cases"].as_array().unwrap().len(), 7);
        for (observed, expected) in report["cases"].as_array().unwrap().iter().zip(cases) {
            for field in ["build", "variant", "xml_sha256"] {
                assert_eq!(observed[field], expected[field]);
            }
            let state = &observed["state"];
            assert_eq!(state["scalability"], expected["scalability"]);
            assert_eq!(
                state["physical_lines"].as_array().unwrap().len() as u64,
                expected["physical_line_count"].as_u64().unwrap()
            );
            // Lua encodes the original empty table as {}; it is retained, not
            // interpreted as a missing populated probe array.
            if expected["probe_count"] == 0 {
                assert_eq!(state["probes"], json!({}));
            } else {
                assert_eq!(state["probes"].as_array().unwrap().len(), 23);
            }
            for field in [
                "saved_items_preserved",
                "saved_selections_preserved",
                "main_output_preserved",
                "original_functions_preserved",
            ] {
                assert_eq!(state[field], true);
            }
        }
        for row in observations {
            let probe = report.pointer(row["pointer"].as_str().unwrap()).unwrap();
            assert_eq!(
                row["value"],
                json!({"name":probe["name"],"input_line":probe["input_line"],"active":probe["active"]})
            );
        }
        if let Some(previous) = &first {
            assert_eq!(&report, previous);
        }
        first = Some(report);
    }
}
