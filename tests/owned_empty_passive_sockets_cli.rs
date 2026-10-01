//! Explicit empty saved Spec sockets close only allocation equipment membership.
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected_request;
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::PassiveSocketMembershipPolicy,
    owned_release::{OwnedReleaseProvenance, assemble_owned_release},
};
use selected_request::{canonical, finalize, selection};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/empty-passive-socket-membership")
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn run(name: &str, args: &[&Path]) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg(name)
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}

#[test]
fn grammar_policy_has_no_build_or_game_definition_parameters_and_pins_full_loader() {
    let p: PassiveSocketMembershipPolicy = read(data().join("policy.json"));
    assert_eq!(
        serde_json::to_value(p).unwrap(),
        json!({"kind":"pob_explicit_empty_spec_sockets_v1"})
    );
    let a: Value = read(data().join("authoring.json"));
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let m: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], m["upstream_revision"]);
    assert_eq!(a["source_files"].as_array().unwrap().len(), 3);
    for pin in a["source_files"].as_array().unwrap() {
        let row = m["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["path"] == pin["path"])
            .unwrap();
        assert_eq!(pin["sha256"], row["sha256"]);
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
}

fn compare_original(xml: &[u8], prior: &Path, next: &Path, out: &Path, case: usize) -> Value {
    let mut a: Value = read(prior.join("draft.json"));
    let mut b: Value = read(next.join("draft.json"));
    canonical(&mut a);
    canonical(&mut b);
    let mut retired = BTreeSet::new();
    let aa = a["draft"]["allocation_presets"]["members"]
        .as_array()
        .unwrap();
    let bb = b["draft"]["allocation_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(aa.len(), bb.len());
    for (old, new) in aa.iter().zip(bb) {
        if old["equipment"]["completion"] != new["equipment"]["completion"] {
            let old_completion = &old["equipment"]["completion"];
            assert_eq!(old_completion["kind"], "pending");
            assert_eq!(
                old_completion["code"],
                "allocation-equipment-membership-not-converted"
            );
            assert_eq!(
                new["equipment"],
                json!({"completion":{"kind":"complete"},"members":[]})
            );
            retired.insert(old_completion["id"]["local"].as_str().unwrap().to_string());
            new["equipment"]["completion"] = old_completion.clone();
        }
    }
    assert_eq!(
        a, b,
        "only proven allocation equipment membership changes; all IDs remain spent"
    );
    let mut sa: Value = read(prior.join("sidecar.json"));
    let mut sb: Value = read(next.join("sidecar.json"));
    canonical(&mut sa);
    canonical(&mut sb);
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|v| {
            v["kind"] != "issue"
                || !v["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id))
        });
    }
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(
        sa, sb,
        "only retired issue links and dependency commitments change"
    );
    let before = finalize(
        xml,
        prior,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = finalize(
        xml,
        next,
        &out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        out.join(format!("original-{case:02}-prior-selected-report.json")),
        &before,
    );
    write(
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    canonical(&mut x);
    canonical(&mut y);
    let count = x.as_array().unwrap().len();
    x.as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "all other selected issues remain exact");
    assert_eq!(count - y.as_array().unwrap().len(), usize::from(case == 5));
    if case == 5 {
        let d: Value = read(next.join("draft.json"));
        let specs = d["draft"]["allocation_presets"]["members"]
            .as_array()
            .unwrap();
        assert_eq!(specs.len(), 7);
        for (i, spec) in specs.iter().enumerate() {
            let expected = if [4, 5].contains(&i) {
                "pending"
            } else {
                "complete"
            };
            assert_eq!(
                spec["equipment"]["completion"]["kind"],
                expected,
                "actual Spec{}",
                i + 1
            );
        }
        assert_eq!(selection(xml, next)["build"]["allocations"], specs[2]["id"]);
        assert_eq!(retired.len(), 5, "only five explicit empty sibling Specs");
    }
    json!({"original":case,"selected_before":count,"selected_after":y.as_array().unwrap().len(),"completed_presets":retired.len(),"selected_issue_summary":after["selected_issue_summary"],"status":"pending"})
}

#[test]
#[ignore = "requires the explicit checked ordinary equipment membership release"]
fn real_publication_closes_only_explicit_empty_spec_socket_membership() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_PASSIVE_SOCKETS_PRIOR")
            .expect("explicit predecessor"),
    );
    let prior_hashes = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let a: Value = read(data().join("authoring.json"));
    let policy: PassiveSocketMembershipPolicy = read(data().join("policy.json"));
    assert_eq!(
        serde_json::to_value(prior.receipt().input).unwrap(),
        a["before"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().normalization).unwrap(),
        a["normalization"]
    );
    assert_eq!(
        serde_json::to_value(&prior.receipt().definitions).unwrap(),
        a["definitions"]
    );
    assert_eq!(prior.input().provenance.len(), 9);
    assert!(prior.normalization().passive_socket_membership.is_none());
    let mut normalization = prior.normalization().clone();
    normalization.passive_socket_membership = Some(policy.clone());
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_PASSIVE_SOCKETS_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("normalization.json"), &normalization);
    let compact = out.join("compact");
    write(
        out.join("publication.json"),
        &run(
            "publish-owned-normalization",
            &[
                &prior_path,
                Path::new("--normalization"),
                &out.join("normalization.json"),
                Path::new("--output"),
                &compact,
            ],
        ),
    );
    let mut input = prior.input().clone();
    input.normalization = read(compact.join("normalization.json"));
    input.tree = Some(read(compact.join("tree-normalization.json")));
    assert_eq!(input.normalization, normalization);
    assert_eq!(
        input.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-empty-passive-socket-membership"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-empty-passive-socket-membership-v1",
            &(&a, &policy),
            1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    let mut restored = input.clone();
    restored.normalization.passive_socket_membership = None;
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(restored, *prior.input());
    write(out.join("endpoint.json"), &input);
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            run("assemble-owned-release", &[src, Path::new("--output"), dst]),
            serde_json::to_value(staged.receipt()).unwrap()
        );
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    for (name, bytes) in staged.artifacts() {
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&name)
        {
            assert_eq!(bytes, fs::read(prior_path.join(name)).unwrap());
        }
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &source, case, &old);
        release::normalize(&package, &source, case, &new);
        reports.push(compare_original(
            &fs::read(&source).unwrap(),
            &old,
            &new,
            &out,
            case,
        ));
    }
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.match_indices("<Spec ").nth(2).unwrap().0;
    let end = start + original[start..].find("</Spec>").unwrap() + 7;
    let spec = &original[start..end];
    assert_eq!(spec.matches("<Sockets/>").count(), 1);
    let probes = [
        ("missing", ""),
        (
            "occupied",
            "<Sockets><Socket nodeId=\"7960\" itemId=\"16\"/></Sockets>",
        ),
        ("namespaced", "<p:Sockets xmlns:p=\"urn:test\"/>"),
        (
            "nested",
            "<Sockets><Wrapper><Socket nodeId=\"7960\" itemId=\"16\"/></Wrapper></Sockets>",
        ),
        ("unknown", "<Sockets/><Equipment/>"),
        ("duplicate", "<Sockets/><Sockets/>"),
    ];
    for (label, replacement) in probes {
        let changed = spec.replace("<Sockets/>", replacement);
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let source = out.join(format!("probe-{label}.xml"));
        fs::write(&source, &xml).unwrap();
        let dst = out.join(format!("probe-{label}"));
        release::normalize(&package, &source, 5, &dst);
        let r = finalize(
            xml.as_bytes(),
            &dst,
            &out.join(format!("probe-{label}-selection.json")),
        );
        assert!(
            r["finalization"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["code"] == "allocation-equipment-membership-not-converted"),
            "{label}"
        );
        write(out.join(format!("probe-{label}-selected-report.json")), &r);
    }
    assert_eq!(prior_hashes, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.receipt().definitions,"registry":staged.receipt().registry,"prior_provenance":9,"final_provenance":10,"queries":110,"originals":reports,"probes":probes.len(),"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
