//! Explicit neutral source guards replace inferred empty-input closure.
use super::{
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, root, success},
};
use poe_optimizer_core::owned_draft::{DraftLimits, DraftListCompletion, decode_draft};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::SourceComponent,
    owned_normalize::NormalizationPolicy,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(cwd: &Path, prior: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("publish-owned-normalization")
        .arg(prior)
        .arg("--normalization")
        .arg(policy)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
pub(super) fn check_guarded_gem_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    let policy_path = data().join("gem-inputs/normalization.json");
    let authored_bytes = fs::read(&policy_path).unwrap();
    let policy: NormalizationPolicy = serde_json::from_slice(&authored_bytes).unwrap();
    let prior_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    assert!(prior_policy.gem_inputs.is_none());
    let mut restored = policy.clone();
    restored.gem_inputs = None;
    assert_eq!(
        restored, prior_policy,
        "only explicit Gem input knowledge changes"
    );
    let gem_policy = policy.gem_inputs.as_ref().unwrap();
    assert_eq!(gem_policy.gems.len(), 2);
    assert_eq!(
        gem_policy
            .gems
            .iter()
            .map(|rule| rule.gem.key().as_str())
            .collect::<Vec<_>>(),
        ["def.000000000000000a", "def.0000000000000011"]
    );
    for rule in &gem_policy.gems {
        assert!(rule.parameters.is_empty());
        assert_eq!(rule.guards.len(), 2);
        assert_eq!(rule.guards[0].attribute, "corrupted");
        assert_eq!(
            rule.guards[0].allowed,
            [
                SourceComponent::Text("false".into()),
                SourceComponent::Text("nil".into())
            ]
        );
        assert_eq!(rule.guards[1].attribute, "corruptLevel");
        assert_eq!(rule.guards[1].allowed, [SourceComponent::Text("0".into())]);
    }
    let prior_bytes = bundle(prior);
    let prior_transition = json(prior.join("transition.json"));
    let output = cwd.join("guarded-gem-inputs-successor");
    let report = success(publish(cwd, prior, &policy_path, &output));
    let transition = &report["publication"];
    assert_eq!(transition["before"], prior_transition["after"]);
    assert_eq!(transition["query_rows"], 110);
    assert_eq!(transition["calculation"], "not_run");
    assert_eq!(transition["whole_build_parity"], "not_established");
    assert_ne!(
        transition["before"]["normalization"],
        transition["after"]["normalization"]
    );
    let published = bundle(&output);
    for (name, bytes) in &prior_bytes {
        if ![
            "transition.json",
            "normalization.json",
            "tree-normalization.json",
            "catalog-append.json",
        ]
        .contains(&name.as_str())
        {
            assert_eq!(&published[name], bytes, "normalization changed {name}");
        }
    }
    let tree = json(output.join("tree-normalization.json"));
    assert_eq!(
        tree["content"],
        json(prior.join("tree-normalization.json"))["content"]
    );
    assert_ne!(transition["tree"], prior_transition["tree"]);
    assert_eq!(
        serde_json::from_slice::<NormalizationPolicy>(&published["normalization.json"]).unwrap(),
        policy
    );

    let mut summary = Vec::new();
    let mut totals = [0usize; 3];
    for case in 1..=5 {
        let previous_path = cwd.join(format!("item-quality-inputs-original-{case}"));
        let destination = cwd.join(format!("guarded-gem-inputs-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let previous = decode_draft(
            &fs::read(previous_path.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let current = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let (old, new) = (previous.input(), current.input());
        let old_sidecar = json(previous_path.join("sidecar.json"));
        let new_sidecar = json(destination.join("sidecar.json"));
        assert_eq!(new_sidecar["schema_version"], 12);
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            new.allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let mut removed = Vec::new();
        let mut resolved = 0;
        assert_eq!(old.gems.members.len(), new.gems.members.len());
        for (old, new) in old.gems.members.iter().zip(&new.gems.members) {
            assert!(old.parameters.members.is_empty() && new.parameters.members.is_empty());
            let DraftListCompletion::Pending { id, code } = &old.parameters.completion else {
                panic!("historical policy inferred parameters")
            };
            assert_eq!(code.as_str(), "gem-parameters-not-converted");
            if !matches!(new.parameters.completion, DraftListCompletion::Complete) {
                continue;
            }
            let definition = new.definition.to_resolved().unwrap();
            let rule = gem_policy
                .gems
                .iter()
                .find(|rule| rule.gem == definition)
                .unwrap();
            assert_eq!(rule.guards.len(), 2);
            let origins: Vec<_> = new_sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|origin| {
                    origin["links"].as_array().unwrap().iter().any(|link| {
                        link["kind"] == "gem"
                            && link["value"] == serde_json::to_value(new.id).unwrap()
                    })
                })
                .collect();
            assert_eq!(origins.len(), 1);
            let row = evidence
                .rows()
                .iter()
                .find(|row| {
                    serde_json::to_value(row.occurrence().id()).unwrap() == origins[0]["source"]
                })
                .unwrap();
            assert!(matches!(
                row.attribute("corrupted").unwrap().decoded().unwrap(),
                "false" | "nil"
            ));
            assert_eq!(
                row.attribute("corruptLevel").unwrap().decoded().unwrap(),
                "0"
            );
            removed.push(id.instance_id().local());
            resolved += 1;
        }
        assert_eq!(resolved, [1, 6, 0, 0, 5][case - 1]);
        assert_eq!(
            old.allocator.last_issued() - new.allocator.last_issued(),
            removed.len() as u64
        );
        let mut old_wire = serde_json::to_value(old).unwrap();
        let mut new_wire = serde_json::to_value(new).unwrap();
        for (old, new) in old_wire["gems"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(new_wire["gems"]["members"].as_array().unwrap())
        {
            if new["parameters"]["completion"]["kind"] == "complete" {
                old["parameters"] = new["parameters"].clone();
            }
        }
        assert_eq!(
            canonical_instances(&mut old_wire, old.allocator.lineage(), &removed),
            canonical_instances(&mut new_wire, new.allocator.lineage(), &[])
        );
        assert_eq!(
            old_wire, new_wire,
            "original {case}: unrelated draft input changed"
        );
        for field in [
            "source_sha256",
            "source_bytes",
            "source_schema",
            "revision",
            "definitions",
            "registry",
            "mapping",
            "mapping_source",
            "skill_roles",
            "reward_policy",
            "item_policy",
            "item_source_policy",
        ] {
            assert_eq!(
                old_sidecar[field], new_sidecar[field],
                "original {case}: {field}"
            );
        }
        for field in ["origins", "item_texts"] {
            let mut old_field = old_sidecar[field].clone();
            let mut new_field = new_sidecar[field].clone();
            if field == "origins" {
                for origin in old_field.as_array_mut().unwrap() {
                    origin["links"].as_array_mut().unwrap().retain(|link| {
                        if link["kind"] != "issue" {
                            return true;
                        }
                        let issue: poe_optimizer_core::build_identity::DraftIssueId =
                            serde_json::from_value(link["value"].clone()).unwrap();
                        !removed.contains(&issue.instance_id().local())
                    });
                }
            }
            assert_eq!(
                canonical_instances(&mut old_field, old.allocator.lineage(), &removed),
                canonical_instances(&mut new_field, new.allocator.lineage(), &[])
            );
            assert_eq!(old_field, new_field, "original {case}: {field} changed");
        }
        let queries: usize = new
            .query_presets
            .members
            .iter()
            .map(|p| p.queries.requests.members.len())
            .sum();
        assert_eq!(queries, 22);
        totals[0] += resolved;
        totals[1] += new.gems.members.len() - resolved;
        totals[2] += queries;
        summary.push(serde_json::json!({"original":case,"complete_parameters":resolved,"pending_parameters":new.gems.members.len()-resolved,"queries":queries,"calculation":"not_run"}));
    }
    assert_eq!(totals, [12, 466, 110]);
    fs::write(
        cwd.join("guarded-gem-inputs-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    assert!(!publish(cwd, prior, &policy_path, &output).status.success());
    assert_eq!(bundle(&output), published);
    let rerun = cwd.join("guarded-gem-inputs-rerun");
    let rerun_report = success(publish(cwd, &output, &policy_path, &rerun));
    assert_eq!(
        rerun_report["publication"]["before"],
        rerun_report["publication"]["after"]
    );
    let mut stale = policy.clone();
    stale
        .gem_inputs
        .as_mut()
        .unwrap()
        .definitions
        .content_sha256 = "0".repeat(64);
    let stale_path = cwd.join("stale-gem-inputs.json");
    fs::write(&stale_path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = cwd.join("stale-gem-inputs-output");
    assert!(!publish(cwd, prior, &stale_path, &rejected).status.success());
    assert!(!rejected.exists());
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(fs::read(policy_path).unwrap(), authored_bytes);
    output
}
