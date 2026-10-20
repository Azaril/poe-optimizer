//! Authenticate retained old bytes, then compare full current outputs.
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
    owned_skill_catalog::verify_owned_skill_catalog_reproduction,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

pub const PRIOR_INPUT: &str = "a0176def7e3666006faa1b95f08709e81b745e15318024fe63dc147aeeb1693f";
const PRIOR_RELEASE_SHA: &str = "238286b791a2657f63f8d43e51b49e16dc88af4e6e30ca4846c65cf38a5b35c1";
const PRIOR_ENDPOINT_SHA: &str = "afa4244767b0445dd61cb98acf3da008ba6608d4e599f99532a2eeba65f53937";
const CATALOG_SHA: &str = "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e";
const COMPILED_PRIOR_TRANSITION_SHA: &str =
    "3c736dcb95a776a1049bc4f25b259a73214872ee777d5dd3179059252fbad108";
const SELECTED: [usize; 5] = [107, 118, 110, 124, 5];
// Independently retained stage-publication outputs: draft, sidecar, normalize
// report and selected-request report. No regeneration of old observations.
const SAVED: [[(&str, usize); 4]; 5] = [
    [
        (
            "717559b9faa8cde1c6683607af557793dee99cd82be3286007bcc610b9e84995",
            413581,
        ),
        (
            "a2d46684fd306658951f7f876ad139dd85e0babd17f6fa5b38f6a4f6e3357435",
            462292,
        ),
        (
            "5780afd3f679d2df09e81b3eb02a86b73cf2ed4c399e89d62acf747154f0fee4",
            2657,
        ),
        (
            "ed2855ed9519b05ac59303d66e01296593b2053fa4af8ca0c1f6e2b7d3083b11",
            121938,
        ),
    ],
    [
        (
            "80cff2915e42a9bbac8d5a935b7a85c19c51258f236d9dfb57671dbf52e200da",
            1057876,
        ),
        (
            "a485561d4dedf03e7e6b7ff26bf20259551e1c0b15a0ade41c89040c0949a5a5",
            986703,
        ),
        (
            "69fee5a2b2ca0a878ec3e784b7c91733bcd5322a20f6d5a86435daeffc21b073",
            2590,
        ),
        (
            "41ded7630c124289f1120921a69a7930c657c03dd287c8599a9bacbf3671272c",
            199463,
        ),
    ],
    [
        (
            "4318dba74cad31338e45fa7695fe2267748df47827da0ba04e0b70a4782bdd24",
            379862,
        ),
        (
            "a6a282618742d458db91914d86b460bfeb4db749f4a19833689a91b64a2b4e35",
            470445,
        ),
        (
            "a36dd9a0045c4ec32993ae9136ea59c78f77e768f2c357f899144ca5328f1bba",
            2573,
        ),
        (
            "2fee64bb44b876a671da2f88795012aad2c248f6fb603c6753336d57fcf13d3f",
            125135,
        ),
    ],
    [
        (
            "3406864e151aef07ab5017c8754c88a8f8411944cfcb384831147d61710d1beb",
            476541,
        ),
        (
            "7e7aceff4956205056544d2830f0b20190f114cd467260ce548e2a93ed233911",
            539149,
        ),
        (
            "6bc06235f86d5c5622732ca4e83f65cf37caec19ef70e1230deaef004f55a0b4",
            2573,
        ),
        (
            "038e42975cdf9bd28b808d6aecf5884c6afd6a0f0fbd9f850c74f05db615544d",
            137891,
        ),
    ],
    [
        (
            "5ad48f8026d22f98cc4c1134a9c63bae89100b617eccf1b21d10ae2e4d95ed90",
            1247908,
        ),
        (
            "42cfaa5a9ee617bde46c4e6b6fbd8ce6a819fdad445190e7e73d65fe641ae832",
            953217,
        ),
        (
            "4c5feadaea47b7177470f6e846829d1ec37aa58b95ba94d86ef1724611e835eb",
            2529,
        ),
        (
            "764f7fd0a33ebed8ba19e1a6c0a8d9f3f3a579e20e00ca8bd662859fdfb4928b",
            91124,
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
        "owned-import-construction-cutover-v1",
        &(PRIOR_INPUT, CATALOG_SHA, compiled_roles, changes),
        4 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(proof["authoring"], json!(authoring));
    authoring
}

#[test]
fn cutover_rejects_substituted_roles_even_with_self_consistent_authoring() {
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
                "owned-import-construction-cutover-v1",
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
    let unrelated = digest_owned(
        "unrelated-test-role-authority",
        &"syntactically valid but not this compiler output",
        1024,
    )
    .unwrap();
    assert_ne!(&unrelated, compiled.roles().identity());
    let forged = make_proof(unrelated);
    // Recomputing both caller-controlled commitments was sufficient before
    // this repair. The same source reproduction now anchors the positive and
    // refusal paths independently of either value supplied by the caller.
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
    for row in role_inverse["roles"].as_array_mut().unwrap() {
        let construction = row.as_object_mut().unwrap().remove("construction").unwrap();
        assert_eq!(construction["kind"], "observed");
    }
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
    let authoring = checked_authoring(&proof, compiled.roles().identity(), &changes);
    let mut provenance = json!(next.receipt().provenance);
    assert_eq!(
        provenance.as_array_mut().unwrap().pop().unwrap(),
        json!({
            "kind":"import-source-construction-cutover","prior_input":PRIOR_INPUT,"authoring_input":authoring
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
    assert_eq!(
        pinned(prior.join(format!("report-{case:02}.json")), sha).len(),
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
    let expected =
        finalization_report(read(prior.join(format!("report-{case:02}.json"))), &old_dir);
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
