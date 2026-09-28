//! The real actor migration preserves authored inputs and unresolved metrics.
//! Publication and normalization establish structure, not numerical parity.
use super::{
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, success},
};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, MetricTarget, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::GrantSlotDefId,
    owned_draft::{DraftField, DraftLimits, DraftMetricTarget, decode_draft},
    owned_rules::RulePackageInput,
};
use poe_optimizer_data::owned_schema::SchemaPackageInput;
use poe_optimizer_import::{
    owned_mapping::{RegistryInput, RegistryState},
    owned_normalize::{
        ImportQueryTarget, ImportQueryTemplate, NormalizationLimits, NormalizationPolicy,
    },
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

fn check_registry_and_contract(
    prior: &Path,
    output: &Path,
    migration: &OwnedReleaseMigrationInput,
) {
    let before: RegistryInput = load(prior.join("registry.json"));
    let after: RegistryInput = load(output.join("registry.json"));
    assert_eq!(before.last_issued.get(), 0x3090);
    assert_eq!(after.last_issued.get(), 0x309b);
    assert_eq!(after.revision.get(), before.revision.get() + 11);
    assert_eq!(after.schema_version, before.schema_version);
    assert_eq!(after.namespace, before.namespace);
    assert_eq!(after.entries.len(), before.entries.len() + 11);
    assert_eq!(&after.entries[..before.entries.len()], &before.entries);
    assert_eq!(migration.schema.len(), 14); // Three existing records, eleven new IDs.
    for (offset, (actual, authored)) in after.entries[before.entries.len()..]
        .iter()
        .zip(&migration.schema[3..])
        .enumerate()
    {
        assert_eq!(actual.sequence.get(), 0x3091 + offset as i64);
        assert_eq!(actual.state, RegistryState::Active);
        assert_eq!(actual.target, authored.subject());
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

fn check_query_targets(
    case: usize,
    before: &[ImportQueryTemplate],
    after: &[ImportQueryTemplate],
    grant: &DeclaredSlot<GrantSlotDefId>,
) {
    assert_eq!((before.len(), after.len()), (22, 22));
    let mut changed = 0;
    for (old, new) in before.iter().zip(after) {
        assert_eq!((&old.id, &old.metric), (&new.id, &new.metric));
        let selected = case == 5 && ["reference-14", "reference-16"].contains(&new.id.as_str());
        if !selected {
            assert_eq!(old, new);
            continue;
        }
        let ImportQueryTarget::Action(old_action) = &old.target else {
            panic!("selected original-05 action")
        };
        let ImportQueryTarget::Action(new_action) = &new.target else {
            panic!("migrated original-05 action")
        };
        assert_eq!(old_action.provider.grant_path.len(), 2);
        assert_eq!(new_action.provider.grant_path.len(), 3);
        let mut restored = new_action.clone();
        assert_eq!(restored.provider.grant_path.pop().as_ref(), Some(grant));
        // Includes the source locator, actor, output and all action dimensions.
        assert_eq!(&restored, old_action);
        changed += 1;
    }
    assert_eq!(changed, if case == 5 { 2 } else { 0 });
}

pub(super) fn check_actor_ability_supply(cwd: &Path, prior: &Path) -> PathBuf {
    let before = bundle(prior);
    let before_receipt = json(prior.join("release.json"));
    let migration_path = data().join("actor-ability-supply/migration.json");
    let migration_bytes = fs::read(&migration_path).unwrap();
    let migration: OwnedReleaseMigrationInput = serde_json::from_slice(&migration_bytes).unwrap();
    assert_eq!(
        before_receipt["input"],
        serde_json::to_value(migration.before).unwrap()
    );
    assert_eq!(
        before_receipt["input"],
        "751754e24a65031251c4047e97e4999a0114f87e750d662ae1a085129409a1d4"
    );
    let bindings = json(data().join("actor-ability-supply/bindings.json"));
    let grant: DeclaredSlot<GrantSlotDefId> =
        serde_json::from_value(bindings["abilities"][0]["grant"].clone()).unwrap();
    assert_eq!(grant.slot.key().as_str(), "def.0000000000003093");
    let output = cwd.join("actor-ability-supply-release");
    let receipt = success(assemble(cwd, prior, &output, Some(&migration_path)));
    assert_eq!(receipt["query_sets"], 5);
    assert_eq!(receipt["query_rows"], 110);
    assert_eq!(receipt["source"], before_receipt["source"]);
    assert_eq!(
        receipt["provenance"][0]["prior_input"],
        before_receipt["input"]
    );
    assert_eq!(receipt["provenance"].as_array().unwrap().len(), 1);
    check_registry_and_contract(prior, &output, &migration);
    let old_policy: NormalizationPolicy = load(prior.join("normalization.json"));
    let new_policy: NormalizationPolicy = load(output.join("normalization.json"));
    let mut totals = [0usize; 5]; // Gems, Boolean/Quantity inputs, modifiers, pending metrics.
    for case in 1..=5 {
        let old_queries: Vec<ImportQueryTemplate> =
            load(prior.join(format!("queries-original-{case:02}.json")));
        let new_queries: Vec<ImportQueryTemplate> =
            load(output.join(format!("queries-original-{case:02}.json")));
        check_query_targets(case, &old_queries, &new_queries, &grant);
        let previous = cwd.join(format!("selected-actions-original-{case}"));
        let destination = cwd.join(format!("actor-ability-supply-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
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
        assert_eq!(
            (
                old.input().query_presets.members.len(),
                new.input().query_presets.members.len()
            ),
            (1, 1)
        );
        let mut restored = new.input().clone();
        let requests = &mut restored.query_presets.members[0].queries.requests.members;
        assert_eq!(requests.len(), 22);
        for (actual, authored) in requests.iter_mut().zip(&new_queries) {
            assert_eq!(actual.id, authored.id);
            assert!(matches!(actual.metric, DraftField::Pending(_)));
            totals[4] += 1;
            if case != 5 || !["reference-14", "reference-16"].contains(&actual.id.as_str()) {
                continue;
            }
            assert!(matches!(
                actual.target.to_resolved(),
                Some(MetricTarget::Action(_))
            ));
            let DraftMetricTarget::Action(action) = &mut actual.target else {
                unreachable!()
            };
            assert_eq!(action.action.provider.grant_path.members.len(), 3);
            assert_eq!(
                action
                    .action
                    .provider
                    .grant_path
                    .members
                    .pop()
                    .unwrap()
                    .to_resolved()
                    .as_ref(),
                Some(&grant)
            );
        }
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(restored).unwrap();
        assert_eq!(
            canonical_instances(&mut old_wire, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut new_wire, new.input().allocator.lineage(), &[])
        );
        assert!(
            old_wire == new_wire,
            "original {case}: unrelated draft changed"
        );
        let mut a = json(previous.join("sidecar.json"));
        let mut b = json(destination.join("sidecar.json"));
        for (sidecar, draft, policy, queries) in [
            (&mut a, &old, &old_policy, &old_queries),
            (&mut b, &new, &new_policy, &new_queries),
        ] {
            let pair = digest_owned(
                "owned-normalization-policy-v3",
                &(policy, queries),
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
        assert!(a == b, "original {case}: unrelated sidecar changed");
        totals[0] += new.input().gems.members.len();
        for parameter in new
            .input()
            .gems
            .members
            .iter()
            .flat_map(|gem| &gem.parameters.members)
        {
            match parameter.value.to_resolved().unwrap() {
                ParameterValue::Boolean(_) => totals[1] += 1,
                ParameterValue::Quantity(_) => totals[2] += 1,
                _ => panic!("unexpected intrinsic value"),
            }
        }
        totals[3] += new
            .input()
            .items
            .members
            .iter()
            .map(|item| item.modifiers.members.len())
            .sum::<usize>();
    }
    assert_eq!(totals, [478, 415, 415, 96, 110]);
    let published = bundle(&output);
    let reproduced = cwd.join("actor-ability-supply-release-reproduced");
    assert_eq!(success(assemble(cwd, &output, &reproduced, None)), receipt);
    assert!(
        bundle(&reproduced) == published,
        "release reproduction changed bytes"
    );
    let repeated = cwd.join("actor-ability-supply-release-repeated");
    assert_eq!(
        success(assemble(cwd, prior, &repeated, Some(&migration_path))),
        receipt
    );
    assert!(
        bundle(&repeated) == published,
        "migration reproduction changed bytes"
    );
    let stale = cwd.join("actor-ability-supply-stale");
    let failure = assemble(cwd, &output, &stale, Some(&migration_path));
    assert!(!failure.status.success());
    assert!(failure.stdout.is_empty());
    assert!(!stale.exists());
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
