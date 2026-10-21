//! Authenticate the reverse-primary cutover, then compare full current outputs.
//! Historical policy bytes are evidence, never deserialized as an old runtime API.
use crate::{preservation, selected};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_draft::{DraftLimits, decode_draft},
};
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use poe_optimizer_import::{
    owned_catalog_recipe::{StagedCatalogRecipe, extend_owned_catalog_recipe},
    owned_normalize::NormalizationLimits,
    owned_release::StagedOwnedRelease,
    owned_skill_catalog::{
        OwnedReversePrimaryCensus, OwnedSkillRoleIndex, SkillCatalogError,
        verify_owned_skill_catalog_reproduction,
    },
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

pub const PRIOR_INPUT: &str = "348f0c7588e915f7b7ea6c2de9b1d2ce5089ebdb4393f5822ed818537a4c7560";
const PRIOR_RELEASE_SHA: &str = "860994818079984781b93c742a16351819601be0693aa89796049cec7ba244bf";
const PRIOR_ENDPOINT_SHA: &str = "ee20e4b2255de4b2ba092c7127a8816e821836e47e44839d43f6d743695d5dba";
const CATALOG_SHA: &str = "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e";
const COMPILED_PRIOR_TRANSITION_SHA: &str =
    "5ca97989545ed0ef5441d580dca09d9f6bf9dbe39531ee9f59f2b14b780093a0";
const SELECTED: [usize; 5] = [123, 137, 127, 144, 5];
const BASELINE_DIRECTORY: &str = "runs/reverse-primary-baseline-20261021";
const BASELINE_RECEIPT_SHA: &str =
    "9f5473d37d7e20564cf20879e28b1bd2a8d610c79c356e8b5ef4d5fc86d30ed7";
// The first three are immutable prior-publication outputs: draft, sidecar and
// normalization report. The fourth is independently captured by the pinned old
// CLI in BASELINE_DIRECTORY; prior selection files are requests, not reports.
const SAVED: [[(&str, usize); 4]; 5] = [
    [
        (
            "63ffda2d05cda6ddadba9bc3c44ebb7211013d4756191ae577388f0423eff2e0",
            417229,
        ),
        (
            "a177a8c276c7362149b305cb913a926197b7f4e24d6ff7b52effea7fa4c4f5c0",
            463818,
        ),
        (
            "0a8b097bf338d7a651915e76d7a12c7410c644c2d105b54ad2f2165e43028aff",
            2729,
        ),
        (
            "83b9514baa93776eaa24bc2b53b6bf8ad0b8cea313aa8fd09224670c4e9aca4f",
            225473,
        ),
    ],
    [
        (
            "b2d441e7da82e4e44eb4e45d945e42d0264dcaa1ae301f757bb13c7055969465",
            1065628,
        ),
        (
            "d7860003ab536f9b9b66a99eb2f19262a261550347714477b534893162e79ed0",
            990011,
        ),
        (
            "84d5c8e7a36eae8d696913cec620dd50658cbc0dc5a2a647375a0621f0db47f6",
            2662,
        ),
        (
            "2fcb12d584a084bfa642bd2f47c3ea8faddd771b02f0cbb9f71ca1ac5db8cc62",
            363746,
        ),
    ],
    [
        (
            "cb3e75143b058f275bc1ec7986428195108dfc70babe5a19cf808fe42cdb4748",
            383738,
        ),
        (
            "628f1cb3716417b59ada043fea12f6d0968d18c1c2559cd58cfff21082f48444",
            472070,
        ),
        (
            "f847d9d8c597ca038ac248c095484ecd477536aa69eb9a58bbce6b3311e5b8c3",
            2645,
        ),
        (
            "2a827863705a1defc7c3fe19984e89569d16a1e67a8ab3be7611736e7d33b133",
            232288,
        ),
    ],
    [
        (
            "624a6b2efe2295e2dfa2ed432ae5a0b6bb8b2aade85a713e6d85275791ac5279",
            482449,
        ),
        (
            "f41fa6cf7f054bf7d5d65ca5db46a4355157f2c8fb693d46959a6568c8085bf3",
            541071,
        ),
        (
            "9458051f51872f865d86b943862038d6a4d25e99540af8dd6fb4d92a8908e888",
            2645,
        ),
        (
            "9a89dcd28a640e0623ac9ca825147d86c2189356b9d72c277abfb466025055af",
            256561,
        ),
    ],
    [
        (
            "2e13d612d308e2e919e3c400c8a03962e86abc9d6aad15cc836d44e2898d3dfe",
            1262415,
        ),
        (
            "bdd562f4e3e2bd236364ec97f26f01e05437be1b7c9d997dfd1efd5aa93a7d1f",
            955139,
        ),
        (
            "aa0408b9f77248e239f1e203aa595b92dbbd2e912fb12b0de52668e35794c2a6",
            2601,
        ),
        (
            "50a3bec8e9dd2eb422a9b2501d7a13edc25ff79b35a5095304b8291a12e5a5fc",
            179678,
        ),
    ],
];
pub fn read_bytes(path: impl AsRef<Path>) -> Vec<u8> {
    let path = path.as_ref();
    assert!(fs::metadata(path).unwrap().len() <= 64 * 1024 * 1024);
    let bytes = fs::read(path).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    bytes
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&read_bytes(path)).unwrap()
}
pub fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn pinned(path: impl AsRef<Path>, sha: &str) -> Vec<u8> {
    let bytes = read_bytes(path);
    assert_eq!(hash(&bytes), sha);
    bytes
}
fn raw_digest(domain: &str, pieces: &[&[u8]]) -> Value {
    assert!(pieces.iter().map(|p| p.len()).sum::<usize>() <= 64 * 1024 * 1024);
    let mut hash = Sha256::new();
    hash.update(b"poe-optimizer-owned-content-v1\0");
    hash.update((domain.len() as u64).to_le_bytes());
    hash.update(domain.as_bytes());
    for piece in pieces {
        hash.update(piece);
    }
    json!(format!("{:x}", hash.finalize()))
}
fn artifacts(package: &Path, receipt: &Value) {
    let mut names = BTreeSet::new();
    for artifact in receipt["artifacts"].as_array().unwrap() {
        let name = artifact["file"].as_str().unwrap();
        assert!(!name.contains(['/', '\\']) && !matches!(name, "." | ".."));
        assert!(names.insert(name));
        let bytes = pinned(package.join(name), artifact["sha256"].as_str().unwrap());
        assert_eq!(bytes.len() as u64, artifact["bytes"].as_u64().unwrap());
    }
    names.insert("release.json");
    let actual: BTreeSet<_> = fs::read_dir(package)
        .unwrap()
        .map(|file| {
            let file = file.unwrap();
            assert!(file.file_type().unwrap().is_file());
            file.file_name().into_string().unwrap()
        })
        .collect();
    assert_eq!(actual, names.into_iter().map(str::to_owned).collect());
}
fn changed_leaves(path: &str, a: &Value, b: &Value, output: &mut Vec<Value>) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => {
            for (key, a) in a {
                changed_leaves(&format!("{path}/{key}"), a, &b[key], output);
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                changed_leaves(&format!("{path}/{index}"), a, b, output);
            }
        }
        _ if a == b => {}
        _ => output.push(json!({"path":path,"before":a,"after":b})),
    }
}
fn source_catalog(root: &Path) -> SkillIdentityCatalog {
    SkillIdentityCatalog::new(
        serde_json::from_slice::<SkillIdentityData>(&pinned(
            root.join("data/owned/poe2/3887ae68/import/skill-identities.json"),
            CATALOG_SHA,
        ))
        .unwrap(),
    )
    .unwrap()
}
fn reproduce_compiled_roles(root: &Path, catalog: &SkillIdentityCatalog) -> StagedCatalogRecipe {
    let import = root.join("data/owned/poe2/3887ae68/import");
    // These are the original source-compiler inputs, not the later native
    // package or self-authored candidate transition commitments.
    extend_owned_catalog_recipe(
        serde_json::from_slice(&pinned(
            import.join("recipe-seed.json"),
            "6305c9332970aa4f4b6ae3830baa46efc76c9bb8720ae8a7fceeb8cb391a48e4",
        ))
        .unwrap(),
        serde_json::from_slice(&pinned(
            import.join("mapping-seed.json"),
            "368e88d6d676b1e2b3baf53dcf05ff2c0cde8517f6fe1e9cebd5cacecaa5a4c7",
        ))
        .unwrap(),
        catalog,
        &serde_json::from_slice(&pinned(
            import.join("source-pin.json"),
            "150e8aef37a5395f42a8f8526d7afb08e87f50e466f767d02dadbfb6c5fa519b",
        ))
        .unwrap(),
        &serde_json::from_slice(&pinned(
            import.join("skill-catalog-policy.json"),
            "24da50a2def13eb4eb29e8097f758804a34a82fd14d0595463833a4419b1d288",
        ))
        .unwrap(),
        Default::default(),
    )
    .unwrap()
}
fn authenticate_compiled_artifacts(candidate: &Path, compiled: &StagedCatalogRecipe) {
    let directory = candidate.join("compiled");
    let mut expected = BTreeSet::new();
    for (name, bytes) in compiled.artifacts() {
        assert!(expected.insert(name.to_owned()));
        assert_eq!(
            read_bytes(directory.join(name)),
            bytes,
            "candidate compiler artifact must reproduce: {name}"
        );
    }
    let actual: BTreeSet<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            entry.file_name().into_string().unwrap()
        })
        .collect();
    assert_eq!(actual, expected, "exact compiler artifact set");
}
fn checked_authoring(
    proof: &Value,
    reproduced_roles: &OwnedContentDigest,
    changes: &[Value],
) -> OwnedContentDigest {
    let compiled_roles: OwnedContentDigest =
        serde_json::from_value(proof["compiled_roles"].clone()).unwrap();
    assert_eq!(
        &compiled_roles, reproduced_roles,
        "cutover authority must identify the reproduced compiler roles"
    );
    let authoring = digest_owned(
        "owned-import-reverse-primary-cutover-v1",
        &(PRIOR_INPUT, CATALOG_SHA, compiled_roles, changes),
        4 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(proof["authoring"], json!(authoring));
    authoring
}

#[test]
fn cutover_rejects_unavailable_reverse_census_even_with_self_consistent_authoring() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog = source_catalog(root);
    let compiled = reproduce_compiled_roles(root, &catalog);
    let changes = vec![json!({
        "path":"/normalization/direct_skill_inputs/roles",
        "before":PRIOR_INPUT,"after":compiled.roles().identity()
    })];
    let make_proof = |roles: OwnedContentDigest| {
        json!({
            "compiled_roles":roles,
            "authoring":digest_owned(
                "owned-import-reverse-primary-cutover-v1",
                &(PRIOR_INPUT,CATALOG_SHA,roles,&changes),
                4*1024*1024
            ).unwrap()
        })
    };
    let honest = make_proof(*compiled.roles().identity());
    assert_eq!(
        json!(checked_authoring(
            &honest,
            compiled.roles().identity(),
            &changes
        )),
        honest["authoring"]
    );
    let mut unavailable = compiled.roles().input().clone();
    unavailable.reverse_primary = OwnedReversePrimaryCensus::Unavailable {};
    let unavailable = OwnedSkillRoleIndex::new(
        unavailable,
        compiled.mapping(),
        compiled.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        verify_owned_skill_catalog_reproduction(
            &catalog,
            &unavailable,
            compiled.mapping(),
            Default::default(),
        ),
        Err(SkillCatalogError::ReproductionMismatch)
    ));
    assert_ne!(unavailable.identity(), compiled.roles().identity());
    let forged = make_proof(*unavailable.identity());
    // A valid structural artifact and freshly recomputed caller authoring hash
    // still cannot replace the independently reproduced full catalog census.
    assert!(
        std::panic::catch_unwind(|| {
            checked_authoring(&forged, compiled.roles().identity(), &changes)
        })
        .is_err()
    );
}

pub fn authenticate_cutover(prior: &Path, candidate: &Path, next: &StagedOwnedRelease) -> Value {
    let old_package = prior.join("package");
    let package = candidate.join("package");
    let old: Value =
        serde_json::from_slice(&pinned(old_package.join("release.json"), PRIOR_RELEASE_SHA))
            .unwrap();
    assert_eq!(old["input"], PRIOR_INPUT);
    artifacts(&old_package, &old);
    let endpoint = pinned(prior.join("endpoint.json"), PRIOR_ENDPOINT_SHA);
    assert_eq!(
        raw_digest("owned-data-release-input-v1", &[&endpoint]),
        PRIOR_INPUT
    );
    drop(endpoint);
    let endpoint = read_bytes(candidate.join("endpoint.json"));
    assert_eq!(
        raw_digest("owned-data-release-input-v1", &[&endpoint]),
        json!(next.receipt().input)
    );
    assert_eq!(endpoint, serde_json::to_vec(next.input()).unwrap());
    drop(endpoint);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    for name in [
        "registry.json",
        "schema.json",
        "rules.json",
        "routing.json",
        "manifest.json",
        "mapping.json",
        "rewards.json",
        "items.json",
        "item-source.json",
        "queries-original-01.json",
        "queries-original-02.json",
        "queries-original-03.json",
        "queries-original-04.json",
        "queries-original-05.json",
    ] {
        assert_eq!(
            read_bytes(old_package.join(name)),
            read_bytes(package.join(name)),
            "{name}"
        );
    }
    let mut role_inverse = read(package.join("roles.json"));
    assert_eq!(role_inverse["roles"].as_array().unwrap().len(), 966);
    let census = role_inverse
        .as_object_mut()
        .unwrap()
        .remove("reverse_primary")
        .unwrap();
    assert_eq!(census["kind"], "observed");
    assert_eq!(census["routes"].as_array().unwrap().len(), 1436);
    assert_eq!(role_inverse, read(old_package.join("roles.json")));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog = source_catalog(root);
    let compiled = reproduce_compiled_roles(root, &catalog);
    authenticate_compiled_artifacts(candidate, &compiled);
    assert_eq!(
        next.roles().input().roles,
        compiled.roles().input().roles,
        "final package retains every reproduced compiler observation"
    );
    assert_eq!(
        next.roles().input().reverse_primary,
        compiled.roles().input().reverse_primary,
        "final package retains the entire independently reproduced reverse census"
    );
    assert_eq!(
        next.roles().input().compilation,
        compiled.transition().compilation,
        "the native rebind preserves the source compilation receipt"
    );
    assert_eq!(compiled.transition().roles, *compiled.roles().identity());
    verify_owned_skill_catalog_reproduction(
        &catalog,
        next.roles(),
        next.mapping(),
        Default::default(),
    )
    .unwrap();
    let old_policy = read(old_package.join("normalization.json"));
    let new_policy = read(package.join("normalization.json"));
    let old_tree = read(old_package.join("tree-normalization.json"));
    let new_tree = read(package.join("tree-normalization.json"));
    let mut changes = Vec::new();
    changed_leaves("/normalization", &old_policy, &new_policy, &mut changes);
    changed_leaves("/tree", &old_tree, &new_tree, &mut changes);
    let allowed: BTreeSet<_> = [
        "/normalization/direct_skill_inputs/roles",
        "/normalization/direct_skill_inputs/dispositions/0/reference_action/roles",
        "/normalization/direct_skill_inputs/dispositions/1/reference_action/roles",
        "/normalization/direct_support_targets/direct_inputs",
        "/normalization/generated_skill_inputs/roles",
        "/normalization/gem_inventory/roles",
        "/normalization/gem_inventory/usage_inputs",
        "/normalization/gem_inventory/primary_dispositions/0/reference_action/roles",
        "/normalization/gem_inventory/primary_dispositions/1/reference_action/roles",
        "/normalization/gem_inventory/primary_dispositions/2/reference_action/roles",
        "/normalization/gem_inventory/primary_dispositions/3/reference_action/roles",
        "/normalization/gem_inventory/primary_dispositions/4/reference_action/roles",
        "/normalization/gem_inventory/primary_dispositions/5/reference_action/roles",
        "/normalization/usage_inputs/roles",
        "/normalization/support_origin_order/roles",
        "/normalization/payload_inventory/roles",
        "/normalization/skill_inventory/roles",
        "/normalization/skill_inventory/direct_inputs",
        "/normalization/configuration_dispositions/dependencies/usage_inputs",
        "/tree/normalization",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = changes
        .iter()
        .map(|v| v["path"].as_str().unwrap())
        .collect();
    assert_eq!(actual, allowed, "exact dependency-leaf scope");
    for change in &changes {
        for field in ["before", "after"] {
            serde_json::from_value::<OwnedContentDigest>(change[field].clone()).unwrap();
        }
    }
    let proof = read(candidate.join("cutover-proof.json"));
    assert_eq!(proof["prior_input"], PRIOR_INPUT);
    assert_eq!(proof["prior_release_sha256"], PRIOR_RELEASE_SHA);
    assert_eq!(proof["prior_endpoint_sha256"], PRIOR_ENDPOINT_SHA);
    assert_eq!(proof["catalog_sha256"], CATALOG_SHA);
    assert_eq!(
        proof["compiled_prior_transition_sha256"],
        COMPILED_PRIOR_TRANSITION_SHA
    );
    assert_eq!(proof["input"], json!(next.receipt().input));
    assert_eq!(proof["dependency_changes"], json!(changes));
    assert_eq!(proof["roles"], 966);
    assert_eq!(proof["reverse_primary_routes"], 1436);
    assert_eq!(proof["query_rows"], 110);
    assert_eq!(proof["native_recipe"], "exactly_unchanged");
    assert_eq!(proof["calculation"], "not_run");
    let authoring = checked_authoring(&proof, compiled.roles().identity(), &changes);
    let mut provenance = json!(next.receipt().provenance);
    assert_eq!(
        provenance.as_array_mut().unwrap().pop().unwrap(),
        json!({
            "kind":"import-reverse-primary-cutover","prior_input":PRIOR_INPUT,"authoring_input":authoring
        })
    );
    assert_eq!(provenance, old["provenance"]);
    old
}

fn lineage(value: &mut Value, expected: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(actual) = fields.get_mut("lineage") {
                assert_eq!(
                    actual, expected,
                    "only the authenticated fresh project lineage"
                );
                *actual = json!("canonical-test-lineage");
            }
            for child in fields.values_mut() {
                lineage(child, expected);
            }
        }
        Value::Array(values) => {
            for child in values {
                lineage(child, expected);
            }
        }
        _ => {}
    }
}
fn old_policy(package: &Path, case: usize) -> Value {
    let policy = read_bytes(package.join("normalization.json"));
    let queries = read_bytes(package.join(format!("queries-original-{case:02}.json")));
    assert!(policy.len() + queries.len() + 3 <= NormalizationLimits::default().max_policy_bytes);
    raw_digest(
        "owned-normalization-policy-v3",
        &[b"[", &policy, b",", &queries, b"]"],
    )
}
fn authenticate_saved(prior: &Path, case: usize) {
    for (index, name) in ["draft.json", "sidecar.json", "report.json"]
        .into_iter()
        .enumerate()
    {
        let (sha, bytes) = SAVED[case - 1][index];
        assert_eq!(
            pinned(prior.join(format!("original-{case:02}/{name}")), sha).len(),
            bytes
        );
    }
    let (sha, bytes) = SAVED[case - 1][3];
    let baseline = Path::new(env!("CARGO_MANIFEST_DIR")).join(BASELINE_DIRECTORY);
    let receipt: Value =
        serde_json::from_slice(&pinned(baseline.join("receipt.json"), BASELINE_RECEIPT_SHA))
            .unwrap();
    assert_eq!(receipt["source_input"], PRIOR_INPUT);
    assert_eq!(receipt["head"], "32d48986992c252938231269b8df5a56e8ca4d69");
    assert_eq!(
        receipt["binary_sha256"],
        "01e5d703e3062f388ef2fddd1d549359d44e6c4ae7cccf7f29af2a1f70fc5dbf"
    );
    assert_eq!(
        pinned(baseline.join(format!("report-{case:02}.json")), sha).len(),
        bytes
    );
}
fn authenticated_outputs(
    directory: &Path,
    xml: &[u8],
    old: Option<(&Path, &Value, usize)>,
    current: Option<(&Path, &StagedOwnedRelease, usize)>,
) -> [Value; 3] {
    let mut draft = read(directory.join("draft.json"));
    let sidecar_bytes = read_bytes(directory.join("sidecar.json"));
    let mut sidecar: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
    let mut report = read(directory.join("report.json"));
    assert!(old.is_some() ^ current.is_some());
    if let Some((package, receipt, case)) = old {
        let policy = read_bytes(package.join("normalization.json"));
        assert_eq!(
            raw_digest("owned-normalization-policy-v3", &[&policy]),
            receipt["normalization"]
        );
        assert_eq!(sidecar["policy"], old_policy(package, case));
        for (field, receipt_field) in [
            ("registry", "registry"),
            ("mapping", "mapping"),
            ("definitions", "definitions"),
            ("skill_roles", "roles"),
            ("reward_policy", "rewards"),
            ("item_policy", "items"),
            ("item_source_policy", "item_source"),
            ("tree_policy", "tree"),
        ] {
            assert_eq!(sidecar[field], receipt[receipt_field]);
        }
        for row in sidecar["item_texts"].as_array().unwrap() {
            assert_eq!(row["attribution"]["item_lines"], receipt["items"]);
            assert_eq!(row["attribution"]["policy"], receipt["item_source"]);
        }
    }
    if let Some((package, release, case)) = current {
        preservation::authenticate(&sidecar, directory, package, case, release);
    }
    let typed = decode_draft(
        &read_bytes(directory.join("draft.json")),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(
        sidecar["draft"],
        json!(
            typed
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    assert_eq!(sidecar["source_sha256"], hash(xml));
    assert_eq!(sidecar["source_bytes"], xml.len());
    assert_eq!(report["source"]["sha256"], hash(xml));
    assert_eq!(report["source"]["bytes"], xml.len());
    assert_eq!(report["allocator_before"], sidecar["allocator_before"]);
    assert_eq!(report["allocator_after"], sidecar["allocator_after"]);
    assert_eq!(draft["draft"]["allocator"], sidecar["allocator_after"]);
    assert_eq!(report["draft_digest"], sidecar["draft"]);
    assert_eq!(report["sidecar_sha256"], hash(&sidecar_bytes));
    assert_eq!(report["sidecar_bytes"], sidecar_bytes.len());
    assert_eq!(
        report["output"]["directory"],
        json!(fs::canonicalize(directory).unwrap())
    );
    assert_eq!(report["verification"]["calculation"], "not_run");
    assert_eq!(report["normalization_status"], "pending");
    let expected_lineage = sidecar["allocator_before"]["lineage"].clone();
    assert!(expected_lineage.is_string());
    for value in [&mut draft, &mut sidecar, &mut report] {
        lineage(value, &expected_lineage);
    }
    // All commitments/paths below were checked above; no other field is removed.
    sidecar["draft"] = Value::Null;
    report["draft_digest"] = Value::Null;
    report["sidecar_sha256"] = Value::Null;
    report["sidecar_bytes"] = Value::Null;
    report["output"]["directory"] = Value::Null;
    [draft, sidecar, report]
}
fn finalization_report(mut report: Value, directory: &Path) -> Value {
    let sidecar = read(directory.join("sidecar.json"));
    assert_eq!(
        report["checked_bytes"],
        read_bytes(directory.join("draft.json")).len()
    );
    for pointer in [
        "/draft_digest",
        "/finalization/draft_digest",
        "/intent_validation/draft_digest",
        "/intent_validation/finalization/draft_digest",
        "/intent_validation/finalization/finalization/draft_digest",
    ] {
        let value = report.pointer_mut(pointer).unwrap();
        assert_eq!(*value, sidecar["draft"], "{pointer}");
        *value = Value::Null;
    }
    assert_eq!(report["verification"]["calculation"], "not_run");
    assert_eq!(report["finalization"]["status"], "pending");
    lineage(&mut report, &sidecar["allocator_before"]["lineage"]);
    report
}
pub fn compare(
    case: usize,
    xml: &[u8],
    prior: &Path,
    candidate: &Path,
    out: &Path,
    receipt: &Value,
    next: &StagedOwnedRelease,
) -> Value {
    authenticate_saved(prior, case);
    let old_dir = prior.join(format!("original-{case:02}"));
    let first = out.join(format!("original-{case:02}"));
    let replay = out.join(format!("replayed-original-{case:02}"));
    let package = candidate.join("package");
    let before = authenticated_outputs(
        &old_dir,
        xml,
        Some((&prior.join("package"), receipt, case)),
        None,
    );
    let after = authenticated_outputs(&first, xml, None, Some((&package, next, case)));
    assert_eq!(
        after,
        authenticated_outputs(&replay, xml, None, Some((&package, next, case))),
        "complete fresh replay outputs"
    );
    assert_eq!(
        before[0], after[0],
        "every draft value, pending obligation, order and local identity"
    );
    assert_eq!(before[2], after[2], "complete normalization report");
    assert_eq!(
        before[1]["mapping_source"],
        json!(next.mapping().source_identity())
    );
    let mut inverse = after[1].clone();
    for field in ["skill_roles", "policy", "tree_policy"] {
        assert_ne!(
            inverse[field], before[1][field],
            "{field} is the verified changed commitment"
        );
        inverse[field] = before[1][field].clone();
    }
    assert_eq!(
        before[1], inverse,
        "whole sidecar except three authenticated dependency commitments"
    );
    let mut old_selection = selected::selection(xml, &old_dir);
    let mut new_selection = selected::selection(xml, &first);
    lineage(
        &mut old_selection,
        &read(old_dir.join("sidecar.json"))["allocator_before"]["lineage"],
    );
    lineage(
        &mut new_selection,
        &read(first.join("sidecar.json"))["allocator_before"]["lineage"],
    );
    assert_eq!(old_selection, new_selection);
    let actual = selected::finalize_with_definitions(
        xml,
        &first,
        &out.join(format!("selection-{case:02}.json")),
        &package.join("schema.json"),
    );
    write(out.join(format!("report-{case:02}.json")), &actual);
    let expected = finalization_report(
        read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(BASELINE_DIRECTORY)
                .join(format!("report-{case:02}.json")),
        ),
        &old_dir,
    );
    let actual = finalization_report(actual, &first);
    assert_eq!(
        expected, actual,
        "complete selected request diagnostics and unselected input observations"
    );
    assert_eq!(
        actual["finalization"]["issues"].as_array().unwrap().len(),
        SELECTED[case - 1]
    );
    json!({"original":case,"selected_unresolved":SELECTED[case-1],
        "exact_draft_inverse":true,"exact_sidecar_inverse":true,"fresh_replay_identical":true,
        "retired_input_issues":0,"calculation":"not_run"})
}
