//! One-off offline authoring for the approved, unshipped Import contract cutover.
//! Temporarily included as a library test to use its private checked rebinder.
//! This is not a runtime old-format reader or a general commitment repair API.
use crate::{
    owned_catalog_recipe::*, owned_release::*, owned_release_revision::rebind_release_dependencies,
    owned_skill_catalog::*,
};
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[path = "owned_reverse_primary_seed_rebuild.rs"]
mod seed;

const PRIOR_INPUT: &str = "348f0c7588e915f7b7ea6c2de9b1d2ce5089ebdb4393f5822ed818537a4c7560";
const PRIOR_ENDPOINT_SHA: &str = "ee20e4b2255de4b2ba092c7127a8816e821836e47e44839d43f6d743695d5dba";
const PRIOR_RELEASE_SHA: &str = "860994818079984781b93c742a16351819601be0693aa89796049cec7ba244bf";
const CATALOG_SHA: &str = "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e";
const COMPILED_TRANSITION_SHA: &str =
    "5ca97989545ed0ef5441d580dca09d9f6bf9dbe39531ee9f59f2b14b780093a0";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: impl AsRef<Path>) -> Vec<u8> {
    let path = path.as_ref();
    assert!(fs::metadata(path).unwrap().len() <= 64 * 1024 * 1024);
    let bytes = fs::read(path).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    bytes
}
fn load<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&read(path)).unwrap()
}
fn pinned(path: impl AsRef<Path>, expected: &str) -> Vec<u8> {
    let bytes = read(path);
    assert_eq!(hash(&bytes), expected, "authenticated prior bytes");
    bytes
}
fn write_json(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn raw_content_digest(domain: &str, bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"poe-optimizer-owned-content-v1\0");
    h.update((domain.len() as u64).to_le_bytes());
    h.update(domain.as_bytes());
    h.update(bytes);
    format!("{:x}", h.finalize())
}
fn strip_reverse_primary(value: &mut Value) {
    assert!(
        value
            .as_object_mut()
            .unwrap()
            .remove("reverse_primary")
            .is_some()
    );
}
fn authenticate_artifacts(directory: &Path, manifest: &Value) {
    for item in manifest["artifacts"].as_array().unwrap() {
        let name = item["file"].as_str().unwrap();
        assert!(!name.contains(['/', '\\']) && name != "." && name != "..");
        let bytes = pinned(directory.join(name), item["sha256"].as_str().unwrap());
        assert_eq!(
            bytes.len() as u64,
            item["bytes"].as_u64().unwrap(),
            "{name}"
        );
    }
}
fn changed_leaves(path: &str, before: &Value, after: &Value, output: &mut Vec<Value>) {
    match (before, after) {
        (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => {
            for (key, value) in a {
                changed_leaves(&format!("{path}/{key}"), value, &b[key], output);
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                changed_leaves(&format!("{path}/{i}"), a, b, output);
            }
        }
        _ if before == after => {}
        _ => output.push(json!({"path":path,"before":before,"after":after})),
    }
}

#[test]
#[ignore = "one-off explicit cutover; writes immutable candidates only"]
fn author_reverse_primary_cutover() {
    let root = root();
    let data = root.join("data/owned/poe2/3887ae68");
    let import = data.join("import");
    let prior = root.join("runs/owned-generated-enabled-publication-01");
    let out = PathBuf::from(std::env::var_os("POE_OPTIMIZER_REVERSE_PRIMARY_OUTPUT").unwrap());
    assert!(!out.exists(), "immutable output");
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_slice::<SkillIdentityData>(&pinned(
            import.join("skill-identities.json"),
            CATALOG_SHA,
        ))
        .unwrap(),
    )
    .unwrap();
    let old_transition: Value = serde_json::from_slice(&pinned(
        import.join("compiled/transition.json"),
        COMPILED_TRANSITION_SHA,
    ))
    .unwrap();
    authenticate_artifacts(&import.join("compiled"), &old_transition);
    let compiled = extend_owned_catalog_recipe(
        load(import.join("recipe-seed.json")),
        load(import.join("mapping-seed.json")),
        &catalog,
        &load(import.join("source-pin.json")),
        &load(import.join("skill-catalog-policy.json")),
        Default::default(),
    )
    .unwrap();
    let mut old_shape = json!(compiled.roles().input());
    strip_reverse_primary(&mut old_shape);
    assert_eq!(old_shape, load::<Value>(import.join("compiled/roles.json")));
    assert_eq!(
        json!(compiled.transition().compilation),
        old_transition["compilation"]
    );
    for (name, bytes) in compiled.artifacts() {
        if !matches!(name, "roles.json" | "transition.json") {
            assert_eq!(bytes, read(import.join("compiled").join(name)), "{name}");
        }
    }
    assert_eq!(compiled.roles().input().roles.len(), 966);
    verify_owned_skill_catalog_reproduction(
        &catalog,
        compiled.roles(),
        compiled.mapping(),
        Default::default(),
    )
    .unwrap();

    let endpoint = pinned(prior.join("endpoint.json"), PRIOR_ENDPOINT_SHA);
    assert_eq!(
        raw_content_digest("owned-data-release-input-v1", &endpoint),
        PRIOR_INPUT
    );
    let mut before: Value = serde_json::from_slice(&endpoint).unwrap();
    drop(endpoint);
    let receipt: Value = serde_json::from_slice(&pinned(
        prior.join("package/release.json"),
        PRIOR_RELEASE_SHA,
    ))
    .unwrap();
    assert_eq!(receipt["input"], PRIOR_INPUT);
    authenticate_artifacts(&prior.join("package"), &receipt);
    assert_eq!(
        before["roles"],
        load::<Value>(prior.join("package/roles.json"))
    );
    let old_roles = before["roles"].clone();
    assert_eq!(
        before["roles"]["roles"],
        json!(compiled.roles().input().roles)
    );
    assert!(before["roles"].get("reverse_primary").is_none());
    before["roles"]["reverse_primary"] = json!(compiled.roles().input().reverse_primary);
    let mut input: OwnedReleaseInput = serde_json::from_value(before).unwrap();
    let old_normalization = json!(input.normalization);
    let old_tree = json!(input.tree);
    let old_recipe = input.recipe.clone();
    let old_mapping = input.mapping.clone();
    let old_queries = input.query_sets.clone();
    let old_rewards = input.rewards.clone();
    let old_items = input.items.clone();
    let old_item_source = input.item_source.clone();
    rebind_release_dependencies(&mut input, Default::default()).unwrap();
    assert_eq!(input.recipe, old_recipe);
    assert_eq!(input.mapping, old_mapping);
    assert_eq!(input.query_sets, old_queries);
    assert_eq!(input.rewards, old_rewards);
    assert_eq!(input.items, old_items);
    assert_eq!(input.item_source, old_item_source);
    let mut inverse_roles = json!(input.roles);
    strip_reverse_primary(&mut inverse_roles);
    assert_eq!(inverse_roles, old_roles);
    let mut deltas = vec![];
    changed_leaves(
        "/normalization",
        &old_normalization,
        &json!(input.normalization),
        &mut deltas,
    );
    changed_leaves("/tree", &old_tree, &json!(input.tree), &mut deltas);
    for delta in &deltas {
        let path = delta["path"].as_str().unwrap();
        let name = path.rsplit('/').next().unwrap();
        assert!(
            matches!(
                name,
                "roles" | "direct_inputs" | "scalar_inputs" | "usage_inputs"
            ) || path == "/tree/normalization",
            "unexpected dependency change: {path}"
        );
        for value in [&delta["before"], &delta["after"]] {
            let digest = value.as_str().expect("only digest leaves may change");
            assert_eq!(digest.len(), 64);
            assert!(digest.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
    let authoring = poe_optimizer_core::owned_content::digest_owned(
        "owned-import-reverse-primary-cutover-v1",
        &(
            PRIOR_INPUT,
            CATALOG_SHA,
            compiled.roles().identity(),
            &deltas,
        ),
        4 * 1024 * 1024,
    )
    .unwrap();
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("import-reverse-primary-cutover").unwrap(),
        prior_input: PRIOR_INPUT.parse().unwrap(),
        authoring_input: authoring,
    });
    let staged = assemble_owned_release(input, Default::default()).unwrap();
    verify_owned_skill_catalog_reproduction(
        &catalog,
        staged.roles(),
        staged.mapping(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(staged.receipt().query_rows, 110);
    assert!(staged.evaluation().is_none());
    let repeated = assemble_owned_release(staged.input().clone(), Default::default()).unwrap();
    assert_eq!(staged.receipt(), repeated.receipt());
    assert!(staged.artifacts().eq(repeated.artifacts()));
    fs::create_dir_all(out.join("compiled")).unwrap();
    fs::create_dir(out.join("package")).unwrap();
    let seed_report = seed::rebuild_current(&root, &out, &compiled);
    for (name, bytes) in compiled.artifacts() {
        fs::write(out.join("compiled").join(name), bytes).unwrap();
    }
    for (name, bytes) in staged.artifacts() {
        fs::write(out.join("package").join(name), bytes).unwrap();
    }
    write_json(out.join("endpoint.json"), staged.input());
    write_json(
        out.join("cutover-proof.json"),
        &json!({
            "schema_version":1, "prior_input":PRIOR_INPUT, "prior_endpoint_sha256":PRIOR_ENDPOINT_SHA,
            "prior_release_sha256":PRIOR_RELEASE_SHA, "catalog_sha256":CATALOG_SHA,
            "compiled_prior_transition_sha256":COMPILED_TRANSITION_SHA,
            "compiled_roles":compiled.roles().identity(),
            "input":staged.receipt().input,"authoring":authoring,"roles":966,
            "reverse_primary_routes":catalog.data().skills.len(),
            "endpoint_bytes":serde_json::to_vec(staged.input()).unwrap().len(),
            "native_recipe":"exactly_unchanged","query_rows":110,"dependency_changes":deltas,
            "calculation":"not_run","whole_build_parity":"not_established",
            "repeated_assembly":"byte_identical","current_seed":seed_report,
        }),
    );
}
