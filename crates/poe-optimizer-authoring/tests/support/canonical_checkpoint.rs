//! One authenticated, source-independent allocation predecessor for test harnesses.
//! ZIP is only transport; the ordinary checkpoint loader supplies all authority.
use super::{CheckedAuthoringCheckpoint, CheckpointLimits, load_authoring_checkpoint_artifacts};
use poe_optimizer_core::owned_content::OwnedContentDigest;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Cursor, Read},
    path::Path,
    sync::OnceLock,
};
use zip::ZipArchive;

const IDENTITY: &str = "71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91";
const ARCHIVE_SHA256: &str = "fc1f557b37c1318b5a89842755fa1faf2052d34e24405dcd20f4289caeeccd6f";
const ARCHIVE_BYTES: usize = 1_442_323;
const ENTRIES: [&str; 20] = [
    "checkpoint.json",
    "names.json",
    "runtime/item-source.json",
    "runtime/items.json",
    "runtime/manifest.json",
    "runtime/mapping.json",
    "runtime/normalization.json",
    "runtime/queries-original-01.json",
    "runtime/queries-original-02.json",
    "runtime/queries-original-03.json",
    "runtime/queries-original-04.json",
    "runtime/queries-original-05.json",
    "runtime/registry.json",
    "runtime/release.json",
    "runtime/rewards.json",
    "runtime/roles.json",
    "runtime/routing.json",
    "runtime/rules.json",
    "runtime/schema.json",
    "runtime/tree-normalization.json",
];

pub(super) fn checkpoint() -> &'static CheckedAuthoringCheckpoint {
    static CHECKPOINT: OnceLock<CheckedAuthoringCheckpoint> = OnceLock::new();
    CHECKPOINT.get_or_init(load)
}

fn load() -> CheckedAuthoringCheckpoint {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/authoring-checkpoints")
        .join(format!("{IDENTITY}.zip"));
    let metadata = fs::symlink_metadata(&path).unwrap();
    assert!(
        metadata.file_type().is_file(),
        "checkpoint archive is a file"
    );
    assert_eq!(metadata.len(), ARCHIVE_BYTES as u64);
    let mut encoded = Vec::with_capacity(ARCHIVE_BYTES);
    File::open(&path)
        .unwrap()
        .take(ARCHIVE_BYTES as u64 + 1)
        .read_to_end(&mut encoded)
        .unwrap();
    assert_eq!(encoded.len(), ARCHIVE_BYTES);
    assert_eq!(format!("{:x}", Sha256::digest(&encoded)), ARCHIVE_SHA256);

    // Authentication precedes ZIP metadata allocation. ZipArchive indexes names,
    // so this fixed fixture's hash, not an assertion about its deduplicated index,
    // also excludes archives with additional duplicate central-directory entries.
    let mut archive = ZipArchive::new(Cursor::new(encoded)).unwrap();
    assert_eq!(archive.len(), ENTRIES.len());
    let limits = CheckpointLimits::default();
    limits.validate().unwrap();
    assert!(ENTRIES.len() < limits.max_entries); // The runtime directory also counts.

    // Admit the complete inventory and every decoded size before allocating any
    // payload. Exact names reject traversal, absolute paths, directories and extras.
    let mut seen = [false; ENTRIES.len()];
    let mut decoded_total = 0usize;
    let mut runtime_total = 0usize;
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).unwrap();
        let name = std::str::from_utf8(entry.name_raw()).unwrap();
        let position = ENTRIES.binary_search(&name).expect("known checkpoint path");
        assert!(
            !std::mem::replace(&mut seen[position], true),
            "duplicate path"
        );
        assert!(entry.is_file() && !entry.encrypted());
        if let Some(mode) = entry.unix_mode() {
            assert!(matches!(mode & 0o170_000, 0 | 0o100_000), "regular file");
        }
        let size = usize::try_from(entry.size()).unwrap();
        let maximum = match name {
            "checkpoint.json" => limits.max_receipt_bytes,
            "names.json" => limits.max_names_bytes,
            _ => {
                runtime_total = runtime_total.checked_add(size).unwrap();
                limits.build.release.max_artifact_bytes
            }
        };
        assert!(size <= maximum, "checkpoint entry byte bound");
        decoded_total = decoded_total.checked_add(size).unwrap();
        assert!(decoded_total <= limits.max_checkpoint_bytes);
        assert!(
            runtime_total
                <= limits
                    .build
                    .release
                    .max_input_bytes
                    .min(limits.build.release.max_output_bytes)
        );
    }
    assert!(seen.into_iter().all(|present| present));

    let mut files = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let size = usize::try_from(entry.size()).unwrap();
        let name = std::str::from_utf8(entry.name_raw()).unwrap().to_owned();
        let mut bytes = vec![0; size];
        entry.read_exact(&mut bytes).unwrap();
        assert_eq!(entry.read(&mut [0u8; 1]).unwrap(), 0, "decoded entry size");
        assert!(files.insert(name, bytes).is_none());
    }
    let receipt = files.remove("checkpoint.json").unwrap();
    let names = files.remove("names.json").unwrap();
    let runtime = files
        .into_iter()
        .map(|(path, bytes)| (path.strip_prefix("runtime/").unwrap().to_owned(), bytes))
        .collect();
    let expected: OwnedContentDigest = IDENTITY.parse().unwrap();
    let checked =
        load_authoring_checkpoint_artifacts(&receipt, &names, &runtime, expected, limits).unwrap();
    assert_eq!(checked.identity(), expected);
    checked
}
