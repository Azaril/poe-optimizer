//! Immutable caller-build evidence remains independent of the editable example file.
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[test]
fn supplied_corpus_imports_decode_to_exact_preserved_documents() {
    let folder =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(folder.join("index.json")).unwrap()).unwrap();
    let input = fs::read(folder.join(index["input_file"].as_str().unwrap())).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&input)),
        index["input_sha256"]
    );
    assert_eq!(input.len() as u64, index["input_bytes"].as_u64().unwrap());
    let lines = std::str::from_utf8(&input)
        .unwrap()
        .trim_start_matches('\u{feff}')
        .lines()
        .collect::<Vec<_>>();
    let mut sources = BTreeSet::new();
    for entry in index["builds"].as_array().unwrap() {
        let source = lines[entry["source_line"].as_u64().unwrap() as usize - 1].trim();
        let digest = format!("{:x}", Sha256::digest(source.as_bytes()));
        assert_eq!(digest, entry["import_sha256"]);
        assert!(sources.insert(digest));
        let decoded = poe_optimizer_import::decode_build(source.as_bytes()).unwrap();
        let expected = fs::read(folder.join(entry["xml"].as_str().unwrap())).unwrap();
        assert_eq!(decoded.xml.as_bytes(), expected, "{}", entry["id"]);
        assert_eq!(
            format!("{:x}", Sha256::digest(&expected)),
            entry["xml_sha256"]
        );
        assert_eq!(expected.len() as u64, entry["xml_bytes"].as_u64().unwrap());
    }
    assert_eq!(sources.len(), 5);
}
