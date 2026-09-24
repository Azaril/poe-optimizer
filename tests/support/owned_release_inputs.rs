//! A deliberate coverage correction is a new release, not monotonic knowledge.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, success},
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_draft::{DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_normalize::{
        GemQualityPolicy, ImportQueryTemplate, NormalizationLimits, NormalizationPolicy,
    },
    owned_release_revision::OwnedReleaseRevisionInput,
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn publish(cwd: &Path, prior: &Path, output: &Path, revision: Option<&Path>) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(cwd)
        .arg("assemble-owned-release")
        .arg(prior)
        .arg("--output")
        .arg(output);
    if let Some(revision) = revision {
        command.arg("--revision").arg(revision);
    }
    success(command.output().unwrap())
}

pub(super) fn check_release_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    let policy_path = data().join("releases/open-gem-inputs-v2.json");
    let policy_bytes = fs::read(&policy_path).unwrap();
    let policy: OwnedReleaseRevisionInput = serde_json::from_slice(&policy_bytes).unwrap();
    assert_eq!(
        serde_json::to_value(policy.before).unwrap(),
        "deb870ee12054739397a7c346741b769a444ffd6342417b2f074da716bac0c81"
    );
    assert_eq!(policy.definitions.len(), 2);
    assert!(policy.slots.is_empty());
    let before_bytes = bundle(prior);
    let before = recipe(prior);
    let transition = json(prior.join("transition.json"));
    let output = cwd.join("open-gem-inputs-release");
    let report = publish(cwd, prior, &output, Some(&policy_path));
    assert_eq!(report, json(output.join("release.json")));
    assert_eq!(
        report["input"],
        "82cd2fd99a751460ec7d0aad5d3bdbd82aea1cbd1c47fd75d99191b49ffed8ac"
    );
    assert_eq!(report["provenance"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["provenance"][0]["prior_input"],
        serde_json::to_value(policy.before).unwrap()
    );
    assert_eq!(
        report["provenance"][0]["kind"],
        serde_json::to_value(&policy.reason).unwrap()
    );
    assert_eq!(
        (report["query_sets"].as_u64(), report["query_rows"].as_u64()),
        (Some(5), Some(110))
    );
    assert!(!output.join("transition.json").exists());
    let after = recipe(&output);
    assert_eq!(
        fs::read(output.join("registry.json")).unwrap(),
        before_bytes["registry.json"]
    );
    assert_eq!(after.registry, before.registry);
    assert_eq!(json(output.join("registry.json"))["last_issued"], 12_360);
    assert_eq!(after.schema.release, policy.release);
    let mut restored = after.schema.clone();
    restored.release = before.schema.release.clone();
    let mut selected = BTreeSet::new();
    for replacement in &policy.definitions {
        let DefinitionDescriptor::Gem(new) = replacement else {
            panic!("Gem-only correction")
        };
        assert!(selected.insert(new.id.clone()));
        let old = before
            .schema
            .definitions
            .iter()
            .find(|row| row.address() == replacement.address())
            .unwrap();
        let DefinitionDescriptor::Gem(old_gem) = old else {
            unreachable!()
        };
        let (SchemaState::Known(old_schema), SchemaState::Known(new_schema)) =
            (&old_gem.schema, &new.schema)
        else {
            panic!("known Gem coverage correction")
        };
        let mut expected = old_schema.clone();
        for (old_list, new_list) in [
            (
                &old_schema.declarations.parameters.closure,
                &new_schema.declarations.parameters.closure,
            ),
            (
                &old_schema.declarations.choices.closure,
                &new_schema.declarations.choices.closure,
            ),
        ] {
            assert_eq!(*old_list, SchemaClosure::Complete);
            let SchemaClosure::Partial { gaps } = new_list else {
                panic!("intrinsic coverage must remain pending")
            };
            assert_eq!(gaps.len(), 1);
            assert_eq!(gaps[0].subject, SchemaSubject::Definition(new.id.address()));
            assert_eq!(gaps[0].facet, SchemaFacet::InputSchema);
            assert_eq!(
                gaps[0].code.as_str(),
                "physical-gem-input-review-incomplete"
            );
        }
        expected.declarations.parameters.closure =
            new_schema.declarations.parameters.closure.clone();
        expected.declarations.choices.closure = new_schema.declarations.choices.closure.clone();
        assert_eq!(
            &expected, new_schema,
            "correction changed more than two closure facets"
        );
        let row = restored
            .definitions
            .iter_mut()
            .find(|row| row.address() == replacement.address())
            .unwrap();
        assert_eq!(row, replacement);
        *row = old.clone();
    }
    assert_eq!(
        selected
            .iter()
            .map(|id| id.key().as_str())
            .collect::<Vec<_>>(),
        ["def.000000000000000a", "def.0000000000000011"]
    );
    assert!(
        restored == before.schema,
        "unrelated schema content or slots changed"
    );
    let mut rules = after.rules.clone();
    rules.definitions = before.rules.definitions.clone();
    assert!(rules == before.rules, "rule programs changed");
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);
    for (name, fields) in [
        ("mapping.json", &[("definitions", "definitions")][..]),
        (
            "roles.json",
            &[("definitions", "definitions"), ("mapping", "mapping")][..],
        ),
        (
            "rewards.json",
            &[("definitions", "definitions"), ("mapping", "mapping")][..],
        ),
        ("items.json", &[("definitions", "definitions")][..]),
        ("item-source.json", &[("item_lines", "items")][..]),
        (
            "tree-normalization.json",
            &[
                ("definitions", "definitions"),
                ("mapping", "mapping"),
                ("normalization", "normalization"),
            ][..],
        ),
    ] {
        let old = json(prior.join(name));
        let mut new = json(output.join(name));
        for (field, binding) in fields {
            assert_eq!(new[*field], report[*binding]);
            assert_ne!(new[*field], old[*field]);
            new[*field] = old[*field].clone();
        }
        assert!(new == old, "unrelated {name} content changed");
    }
    let old_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let mut current_policy: NormalizationPolicy =
        serde_json::from_value(json(output.join("normalization.json"))).unwrap();
    let published_policy = current_policy.clone();
    let normalization_limits = NormalizationLimits::default();
    for (policy, expected) in [
        (&old_policy, &transition["after"]["normalization"]),
        (&published_policy, &report["normalization"]),
    ] {
        assert_eq!(
            serde_json::to_value(
                digest_owned(
                    "owned-normalization-policy-v3",
                    policy,
                    normalization_limits.max_policy_bytes,
                )
                .unwrap()
            )
            .unwrap(),
            *expected,
        );
    }
    let (GemQualityPolicy::Attributes(new), GemQualityPolicy::Attributes(old)) =
        (&mut current_policy.gem_quality, &old_policy.gem_quality)
    else {
        panic!("quality bindings")
    };
    assert_eq!(new.definitions, after.rules.definitions);
    new.definitions = old.definitions.clone();
    let inputs = current_policy.gem_inputs.as_mut().unwrap();
    assert_eq!(inputs.definitions, after.rules.definitions);
    inputs.definitions = old_policy.gem_inputs.as_ref().unwrap().definitions.clone();
    assert_eq!(current_policy, old_policy);
    let mut totals = [0usize; 7]; // Gems, Complete, Pending, Boolean, Quantity, modifiers, queries.
    let mut gains = [0usize; 5];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&query)).unwrap(), before_bytes[&query]);
        // Fresh normalization commits the ordered queries with the policy;
        // package receipts and tree bindings commit the base policy alone.
        let queries: Vec<ImportQueryTemplate> =
            serde_json::from_slice(&before_bytes[&query]).unwrap();
        let query_binding = |policy: &NormalizationPolicy| {
            serde_json::to_value(
                digest_owned(
                    "owned-normalization-policy-v3",
                    &(policy, &queries),
                    normalization_limits.max_policy_bytes,
                )
                .unwrap(),
            )
            .unwrap()
        };
        let old_query_binding = query_binding(&old_policy);
        let new_query_binding = query_binding(&published_policy);
        let previous = cwd.join(format!("multieffect-support-gem-inputs-original-{case}"));
        let destination = cwd.join(format!("open-gem-inputs-original-{case}"));
        let normalization = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(normalization["normalization_status"], "pending");
        assert_eq!(normalization["verification"]["calculation"], "not_run");
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
        let mut old_sidecar = json(previous.join("sidecar.json"));
        let mut new_sidecar = json(destination.join("sidecar.json"));
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        assert_eq!(
            old.input().gems.members.len(),
            new.input().gems.members.len()
        );
        let mut added = vec![];
        for (index, (old_gem, gem)) in old
            .input()
            .gems
            .members
            .iter()
            .zip(&new.input().gems.members)
            .enumerate()
        {
            totals[0] += 1;
            match &gem.parameters.completion {
                DraftListCompletion::Complete => totals[1] += 1,
                DraftListCompletion::Pending { .. } => totals[2] += 1,
            }
            for member in &gem.parameters.members {
                match member.value.to_resolved().unwrap() {
                    ParameterValue::Boolean(_) => totals[3] += 1,
                    ParameterValue::Quantity(_) => totals[4] += 1,
                    _ => panic!("unexpected original intrinsic value"),
                }
            }
            if !selected.contains(&gem.definition.to_resolved().unwrap()) {
                continue;
            }
            assert_eq!(old_gem.definition, gem.definition);
            assert_eq!(old_gem.parameters.completion, DraftListCompletion::Complete);
            assert!(old_gem.parameters.members.is_empty() && gem.parameters.members.is_empty());
            let DraftListCompletion::Pending { id, code } = &gem.parameters.completion else {
                panic!("closed intrinsic coverage")
            };
            assert_eq!(code.as_str(), "gem-parameters-not-converted");
            added.push(id.instance_id().local());
            new_wire["gems"]["members"][index]["parameters"]["completion"] =
                serde_json::to_value(&old_gem.parameters.completion).unwrap();
            // Independently bind the new issue to the same original occurrence as
            // its physical Gem, then remove exactly that newly allocated link.
            let origins = |side: &Value, gem: Value| {
                side["origins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|origin| {
                        origin["links"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|link| link["kind"] == "gem" && link["value"] == gem)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let old_origins = origins(&old_sidecar, serde_json::to_value(old_gem.id).unwrap());
            let new_origins = origins(&new_sidecar, serde_json::to_value(gem.id).unwrap());
            assert_eq!((old_origins.len(), new_origins.len()), (1, 1));
            assert_eq!(old_origins[0]["source"], new_origins[0]["source"]);
            let mut removed = 0;
            for origin in new_sidecar["origins"].as_array_mut().unwrap() {
                origin["links"].as_array_mut().unwrap().retain(|link| {
                    let matching = link["kind"] == "issue"
                        && link["value"] == serde_json::to_value(id).unwrap();
                    if matching {
                        removed += 1;
                    }
                    !matching
                });
                if origin["source"] == new_origins[0]["source"] {
                    assert_eq!(
                        new_origins[0]["links"].as_array().unwrap().len(),
                        origin["links"].as_array().unwrap().len() + 1
                    );
                }
            }
            assert_eq!(removed, 1);
        }
        added.sort_unstable();
        assert!(added.windows(2).all(|pair| pair[0] < pair[1]));
        gains[case - 1] = added.len();
        assert_eq!(
            new.input().allocator.last_issued() - old.input().allocator.last_issued(),
            added.len() as u64
        );
        assert_eq!(
            canonical_instances(&mut old_wire, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut new_wire, new.input().allocator.lineage(), &added)
        );
        assert!(
            old_wire == new_wire,
            "original {case}: unrelated draft changed"
        );
        for (field, old_binding, new_binding) in [
            ("policy", &old_query_binding, &new_query_binding),
            (
                "mapping",
                &transition["after"]["mapping"],
                &report["mapping"],
            ),
            (
                "registry",
                &transition["after"]["registry"],
                &report["registry"],
            ),
            (
                "definitions",
                &transition["after"]["definitions"],
                &report["definitions"],
            ),
            (
                "skill_roles",
                &transition["after"]["roles"],
                &report["roles"],
            ),
            (
                "reward_policy",
                &transition["after"]["rewards"],
                &report["rewards"],
            ),
            ("item_policy", &transition["items"], &report["items"]),
            (
                "item_source_policy",
                &transition["item_source"],
                &report["item_source"],
            ),
            ("tree_policy", &transition["tree"], &report["tree"]),
        ] {
            assert_eq!(&old_sidecar[field], old_binding);
            assert_eq!(&new_sidecar[field], new_binding);
            new_sidecar[field] = old_sidecar[field].clone();
        }
        let old_traces = old_sidecar["item_texts"].as_array().unwrap();
        let new_traces = new_sidecar["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_traces.len(), new_traces.len());
        for (previous, current) in old_traces.iter().zip(new_traces) {
            for (binding, manifest) in [("item_lines", "items"), ("policy", "item_source")] {
                assert_eq!(previous["attribution"][binding], transition[manifest]);
                assert_eq!(current["attribution"][binding], report[manifest]);
                current["attribution"][binding] = previous["attribution"][binding].clone();
            }
        }
        for (sidecar, draft) in [(&mut old_sidecar, &old), (&mut new_sidecar, &new)] {
            let recorded = sidecar.as_object_mut().unwrap().remove("draft").unwrap();
            assert_eq!(
                recorded,
                serde_json::to_value(
                    draft
                        .digest(DraftLimits::default().input.max_wire_bytes)
                        .unwrap()
                )
                .unwrap()
            );
        }
        // Earlier source allocators precede the added normalization issues and
        // therefore keep their watermarks; only the final allocator shifts.
        for field in ["allocator_before", "source_allocator"] {
            let mut a = old_sidecar.as_object_mut().unwrap().remove(field).unwrap();
            let mut b = new_sidecar.as_object_mut().unwrap().remove(field).unwrap();
            canonical_instances(&mut a, old.input().allocator.lineage(), &[]);
            canonical_instances(&mut b, new.input().allocator.lineage(), &[]);
            assert_eq!(a, b);
        }
        assert_eq!(
            canonical_instances(&mut old_sidecar, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut new_sidecar, new.input().allocator.lineage(), &added)
        );
        assert!(
            old_sidecar == new_sidecar,
            "original {case}: unrelated sidecar changed"
        );
        totals[5] += new
            .input()
            .items
            .members
            .iter()
            .map(|item| item.modifiers.members.len())
            .sum::<usize>();
        let queries = new
            .input()
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum::<usize>();
        assert_eq!(queries, 22);
        totals[6] += queries;
        assert!(
            !new.validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
    }
    assert_eq!(gains, [1, 6, 0, 0, 5]);
    assert_eq!(totals, [478, 0, 478, 338, 338, 96, 110]);
    let published = bundle(&output);
    assert_eq!(published.len(), 18);
    let reproduced = cwd.join("open-gem-inputs-release-reproduced");
    assert_eq!(publish(cwd, &output, &reproduced, None), report);
    assert!(
        bundle(&reproduced) == published,
        "release did not reproduce byte-for-byte"
    );
    assert!(bundle(prior) == before_bytes, "previous checkpoint changed");
    assert_eq!(fs::read(policy_path).unwrap(), policy_bytes);
    output
}
