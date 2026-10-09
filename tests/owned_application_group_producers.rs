//! Current-format release rebuild and unchanged input corpus. No old runtime loader.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
use poe_optimizer_core::owned_rules::RulePackageInput;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

// Inverse only for the offline cutover proof. Production accepts the current
// typed format exclusively; this function cannot construct a rule package.
fn undo_member_envelopes(value: &mut Value) -> usize {
    match value {
        Value::Object(object)
            if object.len() == 2
                && object.contains_key("producer")
                && object.contains_key("order") =>
        {
            let mut producer = object.remove("producer").unwrap();
            assert_eq!(producer["kind"], "program_effect");
            let producer = producer.as_object_mut().unwrap();
            producer.remove("kind");
            assert_eq!(producer.len(), 4);
            object.extend(std::mem::take(producer));
            1
        }
        Value::Object(object) => object.values_mut().map(undo_member_envelopes).sum(),
        Value::Array(array) => array.iter_mut().map(undo_member_envelopes).sum(),
        _ => 0,
    }
}

#[test]
fn rebuilt_authoring_pins_cover_every_changed_query_packet() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proof =
        read(root.join("data/owned/poe2/3887ae68/application-group-producers/rebuild.json"));
    assert_eq!(proof["scope"]["numeric_rules_changed"], false);
    let artifacts = proof["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 8);
    for artifact in artifacts {
        let bytes = fs::read(root.join(artifact["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            artifact["after_sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
        assert_ne!(artifact["before_sha256"], artifact["after_sha256"]);
    }
    assert_eq!(proof["authoring_rebindings"].as_array().unwrap().len(), 5);
}

#[test]
#[ignore = "requires APPLICATION_GROUP_PRIOR, APPLICATION_GROUP_RELEASE and a fresh APPLICATION_GROUP_OUTPUT"]
fn current_release_rebuild_preserves_all_five_originals() {
    let env = |name| PathBuf::from(std::env::var_os(name).expect(name));
    let prior = env("POE_OPTIMIZER_TEST_APPLICATION_GROUP_PRIOR");
    let package = env("POE_OPTIMIZER_TEST_APPLICATION_GROUP_RELEASE");
    let output = env("POE_OPTIMIZER_TEST_APPLICATION_GROUP_OUTPUT");
    fs::create_dir(&output).unwrap();
    let current = release::load(&package);
    assert_eq!(current.receipt().query_rows, 110);
    assert_eq!(current.receipt().query_sets, 5);
    let mut restored_rules = read(package.join("rules.json"));
    assert_eq!(undo_member_envelopes(&mut restored_rules), 2017);
    assert_eq!(
        restored_rules,
        read(prior.join("rules.json")),
        "entire rule package differs only in producer envelopes"
    );
    assert!(
        serde_json::from_slice::<RulePackageInput>(&fs::read(prior.join("rules.json")).unwrap())
            .is_err(),
        "no implicit old-member compatibility"
    );
    let before_files = release::inventory(&prior);
    let after_files = release::inventory(&package);
    assert_eq!(before_files.len(), 18);
    assert_eq!(
        before_files.keys().collect::<Vec<_>>(),
        after_files.keys().collect::<Vec<_>>()
    );
    let changed: Vec<_> = before_files
        .iter()
        .filter(|(name, hash)| after_files[*name] != **hash)
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(changed, ["manifest.json", "release.json", "rules.json"]);
    let mut reports = Vec::new();
    for (case, count) in [107, 117, 109, 123, 4].into_iter().enumerate() {
        let case = case + 1;
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let source = fs::read(&xml).unwrap();
        let old = output.join(format!("prior-{case:02}"));
        let new = output.join(format!("current-{case:02}"));
        // Normalization consumes unchanged import artifacts, not either rule format.
        release::normalize(&prior, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let mut old_side = read(old.join("sidecar.json"));
        let mut new_side = read(new.join("sidecar.json"));
        for (directory, side) in [(&old, &old_side), (&new, &new_side)] {
            let limits = DraftLimits::default();
            let draft =
                decode_draft(&fs::read(directory.join("draft.json")).unwrap(), limits).unwrap();
            assert_eq!(
                side["draft"],
                json!(draft.digest(limits.input.max_wire_bytes).unwrap())
            );
        }
        let mut old_draft = read(old.join("draft.json"));
        let mut new_draft = read(new.join("draft.json"));
        // Fresh imports intentionally mint new project lineages. Authenticate
        // each digest above, then compare every other field and every local ID.
        for value in [&mut old_draft, &mut new_draft, &mut old_side, &mut new_side] {
            selected::canonical(value);
        }
        assert!(
            old_draft == new_draft,
            "whole draft modulo fresh lineage, original {case}"
        );
        let old_digest = old_side["draft"].clone();
        let new_digest = new_side["draft"].clone();
        new_side["draft"] = old_digest.clone();
        assert!(
            old_side == new_side,
            "whole sidecar modulo authenticated lineage digest, original {case}"
        );
        let mut before = selected::finalize_with_definitions(
            &source,
            &old,
            &output.join(format!("prior-selected-{case:02}.json")),
            &prior.join("schema.json"),
        );
        let mut after = selected::finalize_with_definitions(
            &source,
            &new,
            &output.join(format!("current-selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        for pointer in [
            "/draft_digest",
            "/finalization/draft_digest",
            "/intent_validation/draft_digest",
            "/intent_validation/finalization/draft_digest",
            "/intent_validation/finalization/finalization/draft_digest",
        ] {
            for (report, digest) in [(&before, &old_digest), (&after, &new_digest)] {
                assert_eq!(report.pointer(pointer).unwrap(), digest);
            }
            *after.pointer_mut(pointer).unwrap() = old_digest.clone();
        }
        selected::canonical(&mut before);
        selected::canonical(&mut after);
        assert!(
            before == after,
            "whole finalization modulo authenticated lineage digest, original {case}"
        );
        assert_eq!(
            after["finalization"]["issues"].as_array().unwrap().len(),
            count
        );
        assert!(
            after["intent_validation"]["schema_issues"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        reports.push(json!({"original":case,"selected_input_obligations":count,"whole_draft_preserved":true,"whole_sidecar_preserved":true,"whole_finalization_preserved":true,"fresh_lineage_and_authenticated_digests_rebased":true}));
    }
    fs::write(output.join("report.json"), serde_json::to_vec_pretty(&json!({"input":current.receipt().input,"changed_artifacts":changed,"originals":reports,"complete_builds":0})).unwrap()).unwrap();
}
