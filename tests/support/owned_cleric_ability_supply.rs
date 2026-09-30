//! A real Cleric supply migration preserves the five authored build drafts.
//! Declared topology is component evidence, not complete numerical coverage.
use super::{
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, success},
};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_draft::{DraftField, DraftLimits, decode_draft},
    owned_rules::RulePackageInput,
};
use poe_optimizer_data::owned_schema::SchemaPackageInput;
use poe_optimizer_import::{
    owned_mapping::{RegistryInput, RegistryState},
    owned_normalize::{ImportQueryTemplate, NormalizationLimits, NormalizationPolicy},
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn load<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn assemble(cwd: &Path, input: &Path, output: &Path, migration: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(cwd)
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output);
    if let Some(migration) = migration {
        command.arg("--migration").arg(migration);
    }
    command.output().unwrap()
}
fn rejected(output: Output, destination: &Path) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert!(!destination.exists());
}

fn check_registry_and_contract(
    prior: &Path,
    output: &Path,
    migration: &OwnedReleaseMigrationInput,
) {
    let before: RegistryInput = load(prior.join("registry.json"));
    let after: RegistryInput = load(output.join("registry.json"));
    assert_eq!(before.last_issued.get(), 0x309b);
    assert_eq!(after.last_issued.get(), 0x30a8);
    assert_eq!(after.revision.get(), before.revision.get() + 13);
    assert_eq!(after.schema_version, before.schema_version);
    assert_eq!(after.namespace, before.namespace);
    assert_eq!(after.entries.len(), before.entries.len() + 13);
    assert_eq!(&after.entries[..before.entries.len()], &before.entries);
    for (offset, actual) in after.entries[before.entries.len()..].iter().enumerate() {
        assert_eq!(actual.sequence.get(), 0x309c + offset as i64);
        assert_eq!(actual.state, RegistryState::Active);
        assert!(
            migration
                .schema
                .iter()
                .any(|authored| authored.subject() == actual.target),
            "new allocation is absent from authored migration"
        );
    }
    let schema: SchemaPackageInput = load(output.join("schema.json"));
    assert_eq!(schema.schema_version, 3);
    assert_eq!(schema.release, migration.release);
    assert_eq!(
        schema.semantics_version,
        migration.contract.schema_semantics_version
    );
    drop(schema);
    let rules: RulePackageInput = load(output.join("rules.json"));
    assert_eq!(
        rules.operations_version.as_str(),
        "owned-domain-operations-v11"
    );
    assert_eq!(
        rules.semantics_version,
        migration.contract.rule_semantics_version
    );
}

fn check_normalized_originals(cwd: &Path, prior: &Path, output: &Path) {
    let before_receipt = json(prior.join("release.json"));
    let receipt = json(output.join("release.json"));
    let old_policy: NormalizationPolicy = load(prior.join("normalization.json"));
    let new_policy: NormalizationPolicy = load(output.join("normalization.json"));
    let mut queries = 0;
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior.join(&name)).unwrap(),
            fs::read(output.join(&name)).unwrap(),
            "original {case}: authored query bytes changed"
        );
        // Original-01 retains its selected-Sand-Djinn source intent and unresolved
        // query targets. Cleric topology is not permission to repoint that request.
        let authored: Vec<ImportQueryTemplate> = load(output.join(&name));
        assert_eq!(authored.len(), 22);
        let previous = cwd.join(format!("actor-ability-supply-original-{case}"));
        let destination = cwd.join(format!("cleric-ability-supply-original-{case}"));
        let report = success(normalize(cwd, output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let old = decode_draft(
            &fs::read(previous.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let new = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert_eq!(
            old.input().allocator.last_issued(),
            new.input().allocator.last_issued()
        );
        assert_eq!(new.input().query_presets.members.len(), 1);
        let requests = &new.input().query_presets.members[0]
            .queries
            .requests
            .members;
        assert_eq!(requests.len(), 22);
        for (request, authored) in requests.iter().zip(&authored) {
            assert_eq!(request.id, authored.id);
            assert!(matches!(request.metric, DraftField::Pending(_)));
            queries += 1;
        }
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        assert_eq!(
            canonical_instances(&mut old_wire, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut new_wire, new.input().allocator.lineage(), &[])
        );
        assert!(
            old_wire == new_wire,
            "original {case}: draft declarations or IDs changed"
        );

        let mut a = json(previous.join("sidecar.json"));
        let mut b = json(destination.join("sidecar.json"));
        for (sidecar, draft, policy) in [(&mut a, &old, &old_policy), (&mut b, &new, &new_policy)] {
            let pair = digest_owned(
                "owned-normalization-policy-v3",
                &(policy, &authored),
                NormalizationLimits::default().max_policy_bytes,
            )
            .unwrap();
            assert_eq!(
                sidecar.as_object_mut().unwrap().remove("policy").unwrap(),
                serde_json::to_value(pair).unwrap()
            );
            assert_eq!(
                sidecar.as_object_mut().unwrap().remove("draft").unwrap(),
                serde_json::to_value(
                    draft
                        .digest(DraftLimits::default().input.max_wire_bytes)
                        .unwrap()
                )
                .unwrap()
            );
        }
        // Check each changed commitment before factoring it out of the source
        // evidence comparison; no source observations or issue IDs are erased.
        for (field, binding) in [
            ("mapping", "mapping"),
            ("registry", "registry"),
            ("definitions", "definitions"),
            ("skill_roles", "roles"),
            ("reward_policy", "rewards"),
            ("item_policy", "items"),
            ("item_source_policy", "item_source"),
            ("tree_policy", "tree"),
        ] {
            assert_eq!(a[field], before_receipt[binding]);
            assert_eq!(b[field], receipt[binding]);
            b[field] = a[field].clone();
        }
        let old_traces = a["item_texts"].as_array().unwrap();
        let new_traces = b["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_traces.len(), new_traces.len());
        for (old, new) in old_traces.iter().zip(new_traces) {
            for (field, binding) in [("item_lines", "items"), ("policy", "item_source")] {
                assert_eq!(old["attribution"][field], before_receipt[binding]);
                assert_eq!(new["attribution"][field], receipt[binding]);
                new["attribution"][field] = old["attribution"][field].clone();
            }
        }
        assert_eq!(
            canonical_instances(&mut a, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut b, new.input().allocator.lineage(), &[])
        );
        assert!(a == b, "original {case}: source evidence changed");
    }
    assert_eq!(queries, 110);
}

pub(super) fn check_cleric_ability_supply(cwd: &Path, prior: &Path) -> PathBuf {
    let before = bundle(prior);
    let before_receipt = json(prior.join("release.json"));
    let migration_path = data().join("cleric-ability-supply/migration.json");
    let migration_bytes = fs::read(&migration_path).unwrap();
    let migration: OwnedReleaseMigrationInput = serde_json::from_slice(&migration_bytes).unwrap();
    assert_eq!(
        before_receipt["input"],
        serde_json::to_value(migration.before).unwrap()
    );
    assert_eq!(
        before_receipt["input"],
        "36b82e2924aa5a34cd2a35d01d395d462ea637527e24b39f64d84cf2a0deb07f"
    );
    assert!(migration.query_targets.is_empty());
    assert!(migration.evaluation.is_none());
    let output = cwd.join("cleric-ability-supply-release");
    let receipt = success(assemble(cwd, prior, &output, Some(&migration_path)));
    assert_eq!(receipt["query_sets"], 5);
    assert_eq!(receipt["query_rows"], 110);
    assert_eq!(receipt["source"], before_receipt["source"]);
    let old_provenance = before_receipt["provenance"].as_array().unwrap();
    let provenance = receipt["provenance"].as_array().unwrap();
    assert_eq!(provenance.len(), old_provenance.len() + 1);
    assert_eq!(&provenance[..old_provenance.len()], old_provenance);
    assert_eq!(
        provenance.last().unwrap()["prior_input"],
        before_receipt["input"]
    );
    check_registry_and_contract(prior, &output, &migration);
    check_normalized_originals(cwd, prior, &output);
    super::cleric_native::check_native_cleric_supply(&output);

    let published = bundle(&output);
    let rebuilt = cwd.join("cleric-ability-supply-release-rebuilt");
    assert_eq!(success(assemble(cwd, &output, &rebuilt, None)), receipt);
    assert!(
        bundle(&rebuilt) == published,
        "release reproduction changed bytes"
    );
    let stale = cwd.join("cleric-ability-supply-stale");
    rejected(
        assemble(cwd, &output, &stale, Some(&migration_path)),
        &stale,
    );
    let missing = cwd.join("cleric-ability-supply-missing-policy");
    rejected(
        assemble(
            cwd,
            prior,
            &missing,
            Some(&cwd.join("missing-cleric-migration.json")),
        ),
        &missing,
    );
    let failure = assemble(cwd, prior, &output, Some(&migration_path));
    assert!(!failure.status.success());
    assert!(failure.stdout.is_empty());
    assert!(
        bundle(&output) == published,
        "immutable publication changed"
    );
    assert!(bundle(prior) == before, "previous release changed");
    assert_eq!(fs::read(migration_path).unwrap(), migration_bytes);
    output
}
