//! Fixed source witnesses plus the authenticated final constructor capture.
//! This test does not parse Lua or infer winning definitions from source text.
use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const LIMIT: usize = 16 * 1024 * 1024;
const SOURCE_PROOF_SHA256: &str =
    "d7d5f8385f419aea2b4371688e2076fc2ab1e25c3b4352c541929bc78ba69095";
// Fixed commitments from the independently authenticated optional acquisition.
const CATALOG_SHA256: &str = "98b3af560aa998fcc2f7cf4d2e9aeeff8fe909e67a0f8543ab96e6f77c41ad8b";
const CAPTURE_SHA256: &str = "f3deee0d67096433ef571af5a76df013f53e78df708b73fa338a4c6da3dce26d";
const SNIPER: &str = "SummonSkeletalSnipersPlayer";
const ABYSSAL: &str = "TriggeredAbyssalApparitionPlayer";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &Path, limit: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(
        bytes.len() <= limit,
        "bounded fixed evidence {}",
        path.display()
    );
    bytes
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn safe_relative(path: &str) -> bool {
    !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}
fn is_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|value| value.is_ascii_hexdigit())
}
struct Proof {
    spans: BTreeMap<String, String>,
    catalog: Value,
}
fn proof() -> &'static Proof {
    static PROOF: OnceLock<Proof> = OnceLock::new();
    PROOF.get_or_init(|| {
        assert!(is_hash(CATALOG_SHA256) && is_hash(CAPTURE_SHA256));
        let base = root().join("data/owned/poe2/3887ae68");
        let bytes = read(
            &base.join("support-prohibition/source-proof.json"),
            16 * 1024,
        );
        assert_eq!(hash(&bytes), SOURCE_PROOF_SHA256);
        let evidence: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence["schema_version"], 1);
        let manifest_path = evidence["source_manifest"]["path"].as_str().unwrap();
        assert_eq!(
            manifest_path,
            "crates/poe-optimizer-pob/data/pob-source-manifest.json"
        );
        let bytes = read(&root().join(manifest_path), LIMIT);
        assert_eq!(hash(&bytes), evidence["source_manifest"]["sha256"]);
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence["source_commit"], manifest["upstream_revision"]);
        assert_eq!(
            evidence["source_commit"],
            "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
        );
        let mut files = BTreeMap::new();
        let mut encoded_left = LIMIT;
        let mut decoded_left = LIMIT;
        assert_eq!(evidence["files"].as_array().unwrap().len(), 7);
        for file in evidence["files"].as_array().unwrap() {
            let path = file["path"].as_str().unwrap();
            let archive = file["archive"].as_str().unwrap();
            assert!(safe_relative(path) && safe_relative(archive));
            let bytes = read(&base.join(archive), encoded_left);
            encoded_left -= bytes.len();
            assert_eq!(bytes.len() as u64, file["archive_bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), file["archive_sha256"]);
            let mut decoded = Vec::new();
            GzDecoder::new(bytes.as_slice())
                .take(decoded_left as u64 + 1)
                .read_to_end(&mut decoded)
                .unwrap();
            assert!(decoded.len() <= decoded_left);
            decoded_left -= decoded.len();
            assert_eq!(
                decoded.len() as u64,
                file["decoded_bytes"].as_u64().unwrap()
            );
            assert_eq!(hash(&decoded), file["normalized_lf_sha256"]);
            let pins: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == path)
                .collect();
            let [pin] = pins.as_slice() else {
                panic!("one authoritative source pin")
            };
            assert_eq!(pin["sha256"], file["normalized_lf_sha256"]);
            let source = String::from_utf8(decoded).unwrap();
            assert!(!source.contains('\r'));
            assert!(files.insert(path.to_owned(), source).is_none());
        }
        let mut spans = BTreeMap::new();
        assert_eq!(evidence["spans"].as_array().unwrap().len(), 8);
        for row in evidence["spans"].as_array().unwrap() {
            let first = row["first"].as_u64().unwrap() as usize;
            let last = row["last"].as_u64().unwrap() as usize;
            let lines: Vec<_> = files[row["path"].as_str().unwrap()].lines().collect();
            assert!(first > 0 && last >= first && last <= lines.len());
            let span = lines[first - 1..last].join("\n") + "\n";
            assert_eq!(span.len() as u64, row["bytes"].as_u64().unwrap());
            assert_eq!(hash(span.as_bytes()), row["sha256"]);
            assert!(
                spans
                    .insert(row["role"].as_str().unwrap().to_owned(), span)
                    .is_none()
            );
        }
        // These complete pinned construction files preserve the definition
        // property. Final capture resolves module order and any later writes.
        for path in [
            "src/Modules/CalcSetup.lua",
            "src/Modules/CalcActiveSkill.lua",
        ] {
            assert!(!files[path].contains("cannotBeSupported"));
        }
        assert!(source_contract(&spans));
        assert_eq!(
            evidence["final_catalog"],
            "skill-definition-facts/catalog.json"
        );
        assert_eq!(
            evidence["final_evidence"],
            "skill-definition-facts/evidence.json"
        );
        let bytes = read(
            &base.join("skill-definition-facts/catalog.json"),
            2 * 1024 * 1024,
        );
        assert_eq!(hash(&bytes), CATALOG_SHA256);
        let catalog: Value = serde_json::from_slice(&bytes).unwrap();
        let bytes = read(
            &base.join("skill-definition-facts/evidence.json"),
            64 * 1024,
        );
        assert_eq!(hash(&bytes), CAPTURE_SHA256);
        let capture: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(catalog["schema_version"], 1);
        assert_eq!(capture["schema_version"], 1);
        assert_eq!(catalog["source"]["system"], "path_of_building2");
        assert_eq!(catalog["source"]["revision"], evidence["source_commit"]);
        assert_eq!(capture["upstream_revision"], evidence["source_commit"]);
        assert_eq!(
            capture["source_manifest_sha256"],
            evidence["source_manifest"]["sha256"]
        );
        assert_eq!(capture["catalog_sha256"], CATALOG_SHA256);
        assert!(is_hash(capture["extractor_sha256"].as_str().unwrap()));
        let pins = capture["extraction_source_files"].as_array().unwrap();
        assert_eq!(pins.len(), 16);
        assert_eq!(
            catalog["source"]["files"],
            capture["extraction_source_files"]
        );
        assert!(
            pins.windows(2)
                .all(|pair| pair[0]["path"].as_str().unwrap() < pair[1]["path"].as_str().unwrap())
        );
        for pin in pins {
            let matched: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"])
                .collect();
            let [source] = matched.as_slice() else {
                panic!("one exact constructor dependency")
            };
            assert_eq!(pin["sha256"], source["sha256"]);
        }
        let modules = capture["module_order"].as_array().unwrap();
        assert!(
            !modules.is_empty()
                && modules
                    .iter()
                    .all(|module| module.as_str().is_some_and(|module| !module.is_empty()))
        );
        assert!(retained_outcomes(&catalog));
        let rows = catalog["skills"].as_array().unwrap();
        assert_eq!(rows.len(), 1436);
        assert_eq!(capture["absent"], 1430);
        assert_eq!(capture["explicit_false"], 0);
        assert_eq!(capture["explicit_true"], 6);
        assert_eq!(capture["definitions"].as_u64().unwrap(), rows.len() as u64);
        for (key, expected) in [
            ("absent", Value::Null),
            ("explicit_false", Value::Bool(false)),
            ("explicit_true", Value::Bool(true)),
        ] {
            assert_eq!(
                capture[key].as_u64().unwrap(),
                rows.iter()
                    .filter(|row| row["cannot_be_supported"] == expected)
                    .count() as u64
            );
        }
        Proof { spans, catalog }
    })
}
fn source_contract(spans: &BTreeMap<String, String>) -> bool {
    let spec = &spans["database-boolean"];
    let exporter = &spans["exporter-active-branch"];
    spec.contains("name=\"CannotBeSupported\"") && spec.contains("type=\"Bool\"") && spec.contains("list=false")
        && ["boolean-reader-DatFile", "boolean-reader-Dat64File"].iter().all(|name| spans[*name].contains("return b:byte(o) == 1"))
        && exporter.starts_with("\tif granted.IsSupport then\n")
        && exporter.contains("\t\tif granted.CannotBeSupported then\n\t\t\tout:write('\\tcannotBeSupported = true,\\n')\n\t\tend\n")
        && spans["effect-construction"].contains("grantedEffect = grantedEffect,")
        && spans["active-skill-construction"].contains("activeEffect = activeEffect,")
        && spans["admission-consumer"].starts_with("function calcLib.canGrantedEffectSupportActiveSkill(grantedEffect, activeSkill)\n\tif activeSkill.activeEffect.grantedEffect.cannotBeSupported then\n\t\treturn false\n\tend\n")
        && spans["effect-construction"].contains("if not group.noSupports then")
        && spans["source-usage-no-supports"].contains("group.noSupports = grantedSkill.noSupports")
        && !spans["admission-consumer"].contains("noSupports")
}
fn retained_outcomes(catalog: &Value) -> bool {
    let Some(rows) = catalog["skills"].as_array() else {
        return false;
    };
    if rows.is_empty() || rows.len() > 4096 {
        return false;
    }
    if !rows.iter().all(|row| {
        row.as_object().is_some_and(|row| {
            row.len() == 2
                && row
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.is_empty() && id.len() <= 1024)
                && row
                    .get("cannot_be_supported")
                    .is_some_and(|value| value.is_null() || value.is_boolean())
        })
    }) {
        return false;
    }
    if !rows
        .windows(2)
        .all(|pair| pair[0]["id"].as_str().unwrap() < pair[1]["id"].as_str().unwrap())
    {
        return false;
    }
    rows.iter()
        .any(|row| row["id"] == SNIPER && row["cannot_be_supported"].is_null())
        && rows
            .iter()
            .any(|row| row["id"] == ABYSSAL && row["cannot_be_supported"] == true)
}
pub(super) fn check_proof() {
    let _ = proof();
}

#[test]
fn final_constructor_capture_and_exact_boolean_consumer_preserve_absent_and_prohibited_definitions()
{
    check_proof();
}

#[test]
fn changed_boolean_interpretation_reference_identity_and_final_definition_facts_are_rejected() {
    let baseline = proof();
    assert!(source_contract(&baseline.spans) && retained_outcomes(&baseline.catalog));
    for (role, before, after) in [
        ("database-boolean", "type=\"Bool\"", "type=\"Int\""),
        (
            "boolean-reader-DatFile",
            "return b:byte(o) == 1",
            "return b:byte(o)",
        ),
        (
            "boolean-reader-Dat64File",
            "return b:byte(o) == 1",
            "return b:byte(o)",
        ),
        (
            "exporter-active-branch",
            "if granted.CannotBeSupported then",
            "if not granted.CannotBeSupported then",
        ),
        (
            "effect-construction",
            "grantedEffect = grantedEffect,",
            "grantedEffect = other,",
        ),
        (
            "active-skill-construction",
            "activeEffect = activeEffect,",
            "activeEffect = other,",
        ),
        (
            "admission-consumer",
            "activeSkill.activeEffect.grantedEffect.cannotBeSupported",
            "activeSkill.socketGroup.noSupports",
        ),
    ] {
        let mut changed = baseline.spans.clone();
        assert_eq!(
            changed[role].matches(before).count(),
            1,
            "valid mutation control {role}"
        );
        changed.insert(role.into(), changed[role].replacen(before, after, 1));
        assert!(!source_contract(&changed), "{role}");
    }
    for (id, value) in [
        (SNIPER, Value::Bool(true)),
        (SNIPER, Value::Bool(false)),
        (ABYSSAL, Value::Bool(false)),
    ] {
        let mut changed = baseline.catalog.clone();
        changed["skills"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["id"] == id)
            .unwrap()["cannot_be_supported"] = value;
        assert!(
            !retained_outcomes(&changed),
            "captured absence differs from explicit false as well as true"
        );
    }
    for missing_property in [false, true] {
        let mut changed = baseline.catalog.clone();
        let rows = changed["skills"].as_array_mut().unwrap();
        if missing_property {
            rows.iter_mut()
                .find(|row| row["id"] == SNIPER)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove("cannot_be_supported");
        } else {
            rows.retain(|row| row["id"] != SNIPER);
        }
        assert!(!retained_outcomes(&changed));
    }
}
