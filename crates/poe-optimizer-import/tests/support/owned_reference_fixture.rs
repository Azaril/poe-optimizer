//! Exact compressed recorded evidence, never a fabricated EvaluationResult.
use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs::File, io::Read, path::Path};

const MAX_REPORT_BYTES: usize = 8 * 1024 * 1024;
const MAX_COMPRESSED_BYTES: usize = 1024 * 1024;

pub struct ReferenceFixture {
    pub source_line: usize,
    pub source_xml_sha256: String,
    pub bytes: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    scope: String,
    whole_build_parity: String,
    expectations_digest_format: String,
    expectations_sha256: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    source_line: usize,
    file: String,
    source_xml_sha256: String,
    report_sha256: String,
    report_bytes: usize,
    gzip_sha256: String,
    gzip_bytes: usize,
    report_schema_version: u32,
    measurement_rows: usize,
}
fn read(path: &Path, maximum: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= maximum, "fixture exceeds bound");
    bytes
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn load_references() -> Vec<ReferenceFixture> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = crate_root.join("tests/fixtures/owned-reference");
    let manifest: Manifest =
        serde_json::from_slice(&read(&root.join("manifest.json"), 64 * 1024)).unwrap();
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.scope, "exact_recorded_reference_reports");
    assert_eq!(manifest.whole_build_parity, "not_established");
    assert_eq!(manifest.cases.len(), 5);
    assert_eq!(
        manifest.expectations_digest_format,
        "sha256_lf_normalized_bytes"
    );
    assert_eq!(
        hash(
            String::from_utf8(read(
                &crate_root.join("../../tests/fixtures/breadth-expectations/originals-v1.json"),
                MAX_REPORT_BYTES,
            ))
            .unwrap()
            .replace("\r\n", "\n")
            .as_bytes()
        ),
        manifest.expectations_sha256
    );
    let mut seen = BTreeSet::new();
    manifest
        .cases
        .into_iter()
        .map(|case| {
            assert!(seen.insert(case.source_line));
            assert!(
                case.id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            );
            assert_eq!(case.file, format!("{}.report.json.gz", case.id));
            assert_eq!(case.report_schema_version, 3);
            assert_eq!(case.measurement_rows, 22);
            assert!(case.report_bytes <= MAX_REPORT_BYTES);
            assert!(case.gzip_bytes <= MAX_COMPRESSED_BYTES);
            let compressed = read(&root.join(case.file), MAX_COMPRESSED_BYTES);
            assert_eq!(compressed.len(), case.gzip_bytes);
            assert_eq!(hash(&compressed), case.gzip_sha256);
            let mut bytes = Vec::new();
            GzDecoder::new(compressed.as_slice())
                .take(case.report_bytes as u64 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            assert_eq!(bytes.len(), case.report_bytes);
            assert_eq!(hash(&bytes), case.report_sha256);
            ReferenceFixture {
                source_line: case.source_line,
                source_xml_sha256: case.source_xml_sha256,
                bytes,
            }
        })
        .collect()
}
