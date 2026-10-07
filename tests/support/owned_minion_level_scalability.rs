//! Retire one proved obligation without changing programs or source admission.
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
pub const KIND: &str = "source-bound-minion-level-scalability";
const DOMAIN: &str = "owned-minion-level-scalability-v1";
const REMOVED: &str = "numeric-component-scalability-unproved";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/minion-level-scalability")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = [
        "authoring",
        "bindings",
        "dependencies",
        "coverage",
        "source-vectors",
    ]
    .map(|name| read(&format!("{name}.json")))
    .into();
    digest_owned(DOMAIN, &values, 1024 * 1024).unwrap()
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
    assert_eq!(a["registry_last_issued_before"], 0x3314);
    assert_eq!(a["registry_last_issued_after"], 0x3314);
    assert_eq!(b["modifier"]["key"], "def.00000000000030ca");
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
    assert_eq!(before.len(), 7);
    assert_eq!(after.len(), 6);
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
            "contribute-player-minion-gem-level",
            "effective-amount",
            "amulet-copy-minion-gem-level",
        ]
    );
    assert_eq!(d["item_rule"]["id"], "fixed-global-minion-level");
    assert_eq!(
        d["source_condition"],
        json!({"rule":"fixed-global-minion-level","all":[
            {"kind":"no_source_scaling_tags"},{"kind":"initial_scaling_is_one"},
            {"kind":"no_generated_buff_members"},{"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
        ]})
    );
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    assert_eq!(b["scope"]["retired_obligations"], 1);
    for field in [
        "closed_existing_rule_owners",
        "new_definitions",
        "new_programs",
        "numerical_program_changes",
        "import_guard_changes",
    ] {
        assert_eq!(b["scope"][field], 0);
    }
    for field in [
        "source_admission_widened",
        "routing_completed",
        "whole_build_parity",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    check_evidence(&a, &b, &v, false);
}
fn expected_owner(after: bool) -> DefinitionRules {
    let d: Value = read("dependencies.json");
    let c: Value = read("coverage.json");
    let mut owner: DefinitionRules = serde_json::from_value(d["owner"].clone()).unwrap();
    if after {
        owner.programs.closure = serde_json::from_value(c["after"].clone()).unwrap();
    }
    owner
}
fn dependencies(endpoint: &StagedOwnedRelease, after: bool) {
    let d: Value = read("dependencies.json");
    let expected = expected_owner(after);
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| **o == expected)
            .count(),
        1
    );
    let items = json!(endpoint.input().items);
    assert_eq!(
        items["rules"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| **r == d["item_rule"])
            .count(),
        1
    );
    let source = json!(endpoint.input().item_source);
    let dialect = source["dialect"].as_object().unwrap();
    assert_eq!(dialect.len(), 1);
    let rows = dialect.values().next().unwrap()["single_modifier_conditions"]
        .as_array()
        .unwrap();
    assert_eq!(
        rows.iter().filter(|r| **r == d["source_condition"]).count(),
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
/// Reuse the exact proved closure refinement on a checked successor. This is
/// independent of which later checkpoint owns the release's final receipt.
#[allow(dead_code)]
pub fn retained_modifier_owner(endpoint: &StagedOwnedRelease) -> DefinitionRules {
    check_authored();
    let a: Value = read("authoring.json");
    let proofs: Vec<_> = endpoint
        .receipt()
        .provenance
        .iter()
        .filter(|p| p.kind.as_str() == KIND)
        .collect();
    assert_eq!(proofs.len(), 1);
    assert_eq!(json!(proofs[0].prior_input), a["before"]);
    assert_eq!(proofs[0].authoring_input, digest());
    dependencies(endpoint, true);
    expected_owner(true)
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
    let new = expected_owner(true);
    assert_eq!(old.programs.members, new.programs.members);
    let mut input = prior.input().clone();
    *input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap() = new;
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    let restored_owner = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap();
    *restored_owner = old;
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "only the single exact owner obligation and authenticated provenance change"
    );
    let prior_receipt = json!(prior.receipt());
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
        assert_eq!(
            prior_receipt[field], next_receipt[field],
            "unchanged {field}"
        );
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
    assert_eq!(v["evidence"]["whole_contributor_coverage"], false);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 8);
    assert_eq!(v["evidence"]["files"].as_array().unwrap().len(), pins.len());
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
    assert_eq!(lines[0]["path"], "src/Data/ModScalability.lua");
    assert_eq!(lines[0]["first"], 318);
    assert_eq!(lines[0]["last"], 318);
    assert_eq!(
        lines[0]["lines"],
        json!(["\t[\"# to Level of all Minion Skills\"] = { { isScalable = true } },"])
    );
    assert_eq!(lines[1]["path"], "src/Modules/ItemTools.lua");
    assert_eq!(lines[1]["first"], 275);
    assert_eq!(lines[1]["last"], 278);
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
    assert_eq!(v["numeric_binding"]["modifier"], b["modifier"]);
    assert_eq!(v["numeric_binding"]["scaling"]["kind"], "scaled");
    assert_eq!(v["numeric_binding"]["precision"], 1);
    assert_eq!(v["numeric_binding"]["display_precision"], 0);
    assert_eq!(
        v["numeric_binding_path"],
        "data/owned/poe2/3887ae68/global-minion-gem-level/numeric-binding.json"
    );
    let numeric = fs::read(root().join(v["numeric_binding_path"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&numeric), v["numeric_binding_sha256"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&numeric).unwrap(),
        v["numeric_binding"]
    );
    let observations = v["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 11);
    for (row, (name, value)) in observations.iter().zip([
        ("amount-five", 5),
        ("untagged-necrotic", 5),
        ("tagged-necrotic", 6),
        ("tagged-quality-fifty", 7),
        ("tagged-minion-magnitude", 7),
        ("unscalable", 5),
        ("corrupted-fixed", 8),
        ("corrupted-fixed-magnitude", 7),
        ("corrupted-ranged-magnitude", 12),
        ("add-then-double", 15),
        ("double-then-add", 12),
    ]) {
        assert_eq!(row["name"], name);
        assert_eq!(row["value"]["name"], name);
        assert_eq!(
            row["value"]["after_build"]["active"][0]["value"]["value"],
            value
        );
        assert_eq!(
            row["value"]["after_build"]["active"][0]["name"],
            "GemProperty"
        );
    }
    if !full {
        return;
    }
    let historical = &v["native_reference"];
    assert_eq!(
        historical["path"],
        "runs/global-minion-level-native-tests.log"
    );
    let log = fs::read(root().join(historical["path"].as_str().unwrap())).unwrap();
    assert_eq!(log.len() as u64, historical["bytes"].as_u64().unwrap());
    assert_eq!(hash(&log), historical["sha256"]);
    assert!(
        String::from_utf8(log)
            .unwrap()
            .contains("test result: ok. 4 passed; 0 failed")
    );
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let mut first: Option<Value> = None;
    for (i, pin) in reports.iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "runs/owned-global-minion-level-01/source-jit-{}.json",
                if i == 0 { "off" } else { "on" }
            )
        );
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["source_hash"], v["source_hash"]);
        assert_eq!(report["evidence"], v["evidence"]);
        assert_eq!(report["builds"].as_array().unwrap().len(), 2);
        for (j, build) in report["builds"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                build["fixture"],
                if j == 0 {
                    "build-01.xml"
                } else {
                    "build-05.xml"
                }
            );
            assert_eq!(build["fixture"], v["builds"][j]["fixture"]);
            assert_eq!(build["xml_sha256"], v["builds"][j]["xml_sha256"]);
            let xml = fs::read(
                root()
                    .join("tests/fixtures/builds/breadth-20260908")
                    .join(build["fixture"].as_str().unwrap()),
            )
            .unwrap();
            assert_eq!(hash(&xml), build["xml_sha256"]);
            for field in [
                "saved_items_preserved",
                "saved_selections_preserved",
                "main_output_preserved",
                "original_functions_preserved",
            ] {
                assert_eq!(build["state"][field], true);
            }
            if let Some(previous) = &first {
                assert_eq!(build["state"], previous["builds"][j]["state"]);
            }
        }
        assert_eq!(
            report["builds"][1]["state"]["probes"]
                .as_array()
                .unwrap()
                .len(),
            32
        );
        for row in observations {
            assert_eq!(
                report.pointer(row["pointer"].as_str().unwrap()).unwrap(),
                &row["value"]
            );
        }
        first = Some(report);
    }
}
