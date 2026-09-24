//! Explicit query targets select existing authored uses; they create no actors or skills.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, root, success},
};
use poe_optimizer_core::{
    build_identity::{GemInstanceId, SkillUseId},
    owned_build::{
        ActionSelection, ActorKey, MetricTarget, ParameterValue, ProviderKey, ProviderRoot,
    },
    owned_content::digest_owned,
    owned_definitions::*,
    owned_draft::{
        DraftActorKey, DraftAuthoredSkillSource, DraftField, DraftLimits, DraftMetricTarget,
        DraftSession, decode_draft,
    },
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        ImportActionTarget, ImportActorTarget, ImportQueryTarget, ImportQueryTemplate,
        NormalizationLimits,
    },
    owned_release::{OWNED_RELEASE_VERSION, OwnedReleaseInput},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::NamedQuerySet,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn assemble(cwd: &Path, input: &Path, output: &Path) -> Value {
    success(
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(cwd)
            .arg("assemble-owned-release")
            .arg(input)
            .arg("--output")
            .arg(output)
            .output()
            .unwrap(),
    )
}
fn targets(case: usize) -> &'static [&'static str] {
    match case {
        2 => &["reference-13", "reference-15"],
        5 => &["reference-14", "reference-16"],
        _ => &[],
    }
}
fn location(case: usize) -> (&'static str, u32) {
    match case {
        2 => (
            "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
            382,
        ),
        5 => (
            "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
            211,
        ),
        _ => unreachable!(),
    }
}

fn check_target(
    draft: &DraftSession,
    sidecar: &Value,
    authored: &ImportActionTarget,
    actual: &DraftMetricTarget,
    case: usize,
) {
    let (sha, ordinal) = location(case);
    let locator = &authored.provider.skill_use;
    assert_eq!(
        (&*locator.source_sha256, locator.occurrence_ordinal),
        (sha, ordinal)
    );
    let origins: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(origins.len(), 1);
    assert_eq!(origins[0]["source"]["source_sha256"], sha);
    let linked = |kind: &str| {
        let values: Vec<_> = origins[0]["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == kind)
            .map(|link| link["value"].clone())
            .collect();
        assert_eq!(values.len(), 1);
        values[0].clone()
    };
    let skill_id: SkillUseId = serde_json::from_value(linked("skill")).unwrap();
    let gem_id: GemInstanceId = serde_json::from_value(linked("gem")).unwrap();
    let skill = draft
        .input()
        .skills
        .members
        .iter()
        .find(|skill| skill.id == skill_id)
        .unwrap();
    let DraftAuthoredSkillSource::Gem(source_gem) = &skill.source else {
        panic!("existing physical skill root")
    };
    assert_eq!(source_gem.to_resolved(), Some(gem_id));
    let gem = draft
        .input()
        .gems
        .members
        .iter()
        .find(|gem| gem.id == gem_id)
        .unwrap();
    assert_eq!(
        gem.definition.to_resolved(),
        Some(locator.expected_gem.clone())
    );
    let Some(MetricTarget::Action(resolved)) = actual.to_resolved() else {
        panic!("query action correspondence remained unresolved")
    };
    let ActionSelection {
        action,
        part,
        mode,
        stat_set,
    } = *resolved;
    assert_eq!(
        action.provider,
        ProviderKey {
            root: ProviderRoot::SkillUse(skill_id),
            grant_path: authored.provider.grant_path.clone()
        }
    );
    assert_eq!(action.output, authored.output);
    assert_eq!(
        (part, mode, stat_set),
        (
            authored.part.clone(),
            authored.mode.clone(),
            authored.stat_set.clone()
        )
    );
    assert_eq!(authored.part.key().as_str(), "def.0000000000000007");
    assert_eq!(authored.mode.key().as_str(), "def.0000000000000008");
    assert_eq!(authored.stat_set.key().as_str(), "def.0000000000000009");
    let path: Vec<_> = authored
        .provider
        .grant_path
        .iter()
        .map(|step| step.slot.key().as_str())
        .collect();
    match (case, &authored.actor, action.actor) {
        (2, ImportActorTarget::Player, ActorKey::Player) => {
            assert_eq!(locator.expected_gem.key().as_str(), "def.000000000000000a");
            assert_eq!(path, ["def.0000000000000010"]);
            assert_eq!(authored.output.slot.key().as_str(), "def.000000000000000e");
        }
        (5, ImportActorTarget::Owned { provider, slot }, ActorKey::Owned(actor)) => {
            assert_eq!(locator.expected_gem.key().as_str(), "def.0000000000000011");
            assert_eq!(provider.skill_use, *locator);
            assert_eq!(path, ["def.0000000000000017", "def.0000000000000020"]);
            assert_eq!(provider.grant_path, authored.provider.grant_path[..1]);
            assert_eq!(
                actor.provider,
                ProviderKey {
                    root: ProviderRoot::SkillUse(skill_id),
                    grant_path: provider.grant_path.clone()
                }
            );
            assert_eq!(actor.slot, *slot);
            assert_eq!(slot.slot.key().as_str(), "def.000000000000001f");
            assert_eq!(authored.output.slot.key().as_str(), "def.0000000000000022");
        }
        _ => panic!("wrong player/owned actor correspondence"),
    }
}

pub(super) fn check_selected_actions(cwd: &Path, prior: &Path) -> PathBuf {
    let before = bundle(prior);
    let evidence = json(data().join("selected-actions/selection-evidence.json"));
    assert_eq!(evidence.as_array().unwrap().len(), 2);
    let mut input = OwnedReleaseInput {
        schema_version: OWNED_RELEASE_VERSION,
        recipe: recipe(prior),
        mapping: serde_json::from_value(json(prior.join("mapping.json"))).unwrap(),
        roles: serde_json::from_value(json(prior.join("roles.json"))).unwrap(),
        normalization: serde_json::from_value(json(prior.join("normalization.json"))).unwrap(),
        rewards: serde_json::from_value(json(prior.join("rewards.json"))).unwrap(),
        items: serde_json::from_value(json(prior.join("items.json"))).unwrap(),
        item_source: serde_json::from_value(json(prior.join("item-source.json"))).unwrap(),
        tree: Some(serde_json::from_value(json(prior.join("tree-normalization.json"))).unwrap()),
        query_sets: (1..=5)
            .map(|case| NamedQuerySet {
                name: OwnedDefinitionKey::new(format!("original-{case:02}")).unwrap(),
                queries: serde_json::from_slice(
                    &before[&format!("queries-original-{case:02}.json")],
                )
                .unwrap(),
            })
            .collect(),
        provenance: vec![],
    };
    let old_queries = input.query_sets.clone();
    let mut corrected = 0;
    for case in [2, 5] {
        let queries: Vec<ImportQueryTemplate> = serde_json::from_value(json(
            data().join(format!("selected-actions/original-{case:02}.json")),
        ))
        .unwrap();
        let old = &old_queries[case - 1].queries;
        assert_eq!((queries.len(), old.len()), (22, 22));
        for (a, b) in old.iter().zip(&queries) {
            assert_eq!((&a.id, &a.metric), (&b.id, &b.metric));
            if targets(case).contains(&b.id.as_str()) {
                assert!(matches!(b.target, ImportQueryTarget::Action(_)));
                assert!(
                    matches!(a.target, ImportQueryTarget::Player) && case == 2
                        || matches!(a.target, ImportQueryTarget::Unresolved(_)) && case == 5
                );
                corrected += 1;
            } else {
                assert_eq!(a, b);
            }
        }
        input.query_sets[case - 1].queries = queries;
    }
    assert_eq!(corrected, 4);
    let input_path = cwd.join("selected-actions-release-input.json");
    fs::write(&input_path, serde_json::to_vec(&input).unwrap()).unwrap();
    let output = cwd.join("selected-actions-release");
    let receipt = assemble(cwd, &input_path, &output);
    assert_eq!(receipt["query_sets"], 5);
    assert_eq!(receipt["query_rows"], 110);
    let published = bundle(&output);
    for (name, bytes) in &published {
        if ![
            "release.json",
            "queries-original-02.json",
            "queries-original-05.json",
        ]
        .contains(&name.as_str())
        {
            assert_eq!(bytes, &before[name], "unchanged release component {name}");
        }
    }
    let mut counts = [0usize; 5]; // Gems, Boolean values, Quantity values, item modifiers, pending metrics.
    let mut removals = [0usize; 5];
    for case in 1..=5 {
        let previous = cwd.join(format!("active-gem-inputs-original-{case}"));
        let destination = cwd.join(format!("selected-actions-original-{case}"));
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
        let mut a = json(previous.join("sidecar.json"));
        let mut b = json(destination.join("sidecar.json"));
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        assert_eq!(
            (
                old.input().query_presets.members.len(),
                new.input().query_presets.members.len()
            ),
            (1, 1)
        );
        let old_preset = &old.input().query_presets.members[0];
        let new_preset = &new.input().query_presets.members[0];
        assert_eq!(
            (
                old_preset.queries.requests.members.len(),
                new_preset.queries.requests.members.len()
            ),
            (22, 22)
        );
        let mut removed = vec![];
        for (index, ((prior, current), authored)) in old_preset
            .queries
            .requests
            .members
            .iter()
            .zip(&new_preset.queries.requests.members)
            .zip(&input.query_sets[case - 1].queries)
            .enumerate()
        {
            assert_eq!((&prior.id, &current.id), (&authored.id, &authored.id));
            assert!(matches!(current.metric, DraftField::Pending(_)));
            counts[4] += 1;
            if !targets(case).contains(&current.id.as_str()) {
                continue;
            }
            let ImportQueryTarget::Action(target) = &authored.target else {
                unreachable!()
            };
            check_target(&new, &b, target, &current.target, case);
            match &prior.target {
                DraftMetricTarget::Actor(DraftActorKey::Player) => assert_eq!(case, 2),
                DraftMetricTarget::Pending(pending) => {
                    assert_eq!(case, 5);
                    let ImportQueryTarget::Unresolved(code) =
                        &old_queries[case - 1].queries[index].target
                    else {
                        unreachable!()
                    };
                    assert_eq!(&pending.code, code);
                    assert!(pending.candidates.is_empty());
                    removed.push(pending.id.instance_id().local());
                    let mut occurrences = 0;
                    for origin in a["origins"].as_array_mut().unwrap() {
                        let has_preset = origin["links"].as_array().unwrap().iter().any(|link| {
                            link["kind"] == "query_preset"
                                && link["value"] == serde_json::to_value(old_preset.id).unwrap()
                        });
                        origin["links"].as_array_mut().unwrap().retain(|link| {
                            let matching = link["kind"] == "issue"
                                && link["value"] == serde_json::to_value(pending.id).unwrap();
                            if matching {
                                assert!(has_preset);
                                occurrences += 1;
                            }
                            !matching
                        });
                    }
                    assert_eq!(occurrences, 1);
                }
                _ => panic!("unexpected prior target"),
            }
            // Both targets were checked independently above. Mask only those
            // four target fields before comparing every remaining typed value.
            old_wire["query_presets"]["members"][0]["queries"]["requests"]["members"][index]["target"] =
                Value::Null;
            new_wire["query_presets"]["members"][0]["queries"]["requests"]["members"][index]["target"] =
                Value::Null;
        }
        if !targets(case).is_empty() {
            let (sha, ordinal) = location(case);
            let witness = evidence
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["original"] == case)
                .unwrap();
            assert_eq!(witness["source_sha256"], sha);
            assert_eq!(witness["occurrence_ordinal"], ordinal);
            assert_eq!(
                witness["query_ids"],
                serde_json::to_value(targets(case)).unwrap()
            );
            let xml = fs::read(root().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
            )))
            .unwrap();
            let source = ImportedBuildInstance::from_decoded(
                decode_build(&xml).unwrap(),
                new.input().allocator.lineage(),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let source_evidence =
                SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
            assert_eq!(source.source_sha256(), sha);
            let row = &source_evidence.rows()[ordinal as usize];
            assert_eq!(row.occurrence().name(), "Gem");
            for (name, value) in witness["attributes"].as_object().unwrap() {
                assert_eq!(
                    row.attribute(name).unwrap().decoded().unwrap(),
                    value.as_str().unwrap()
                );
            }
            let group =
                &source_evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(
                group
                    .attribute("mainActiveSkill")
                    .unwrap()
                    .decoded()
                    .unwrap(),
                witness["main_active_skill"].as_str().unwrap()
            );
            let skill_set =
                &source_evidence.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(
                skill_set.attribute("id").unwrap().decoded().unwrap(),
                witness["skill_set"].as_str().unwrap()
            );
            let origin = b["origins"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|origin| origin["source"]["ordinal"] == ordinal)
                .unwrap();
            let before = origin["links"].as_array().unwrap().len();
            origin["links"].as_array_mut().unwrap().retain(|link| {
                !(link["kind"] == "query_preset"
                    && link["value"] == serde_json::to_value(new_preset.id).unwrap())
            });
            assert_eq!(origin["links"].as_array().unwrap().len() + 1, before);
        }
        removed.sort_unstable();
        assert!(removed.windows(2).all(|ids| ids[0] < ids[1]));
        removals[case - 1] = removed.len();
        assert_eq!(
            old.input().allocator.last_issued() - new.input().allocator.last_issued(),
            removed.len() as u64
        );
        assert_eq!(
            canonical_instances(&mut old_wire, old.input().allocator.lineage(), &removed),
            canonical_instances(&mut new_wire, new.input().allocator.lineage(), &[])
        );
        assert!(
            old_wire == new_wire,
            "original {case}: unrelated draft changed"
        );
        for (sidecar, draft, queries) in [
            (&mut a, &old, &old_queries[case - 1].queries),
            (&mut b, &new, &input.query_sets[case - 1].queries),
        ] {
            let recorded_policy = sidecar.as_object_mut().unwrap().remove("policy").unwrap();
            assert_eq!(
                recorded_policy,
                serde_json::to_value(
                    digest_owned(
                        "owned-normalization-policy-v3",
                        &(&input.normalization, queries),
                        NormalizationLimits::default().max_policy_bytes
                    )
                    .unwrap()
                )
                .unwrap()
            );
            let recorded_draft = sidecar.as_object_mut().unwrap().remove("draft").unwrap();
            assert_eq!(
                recorded_draft,
                serde_json::to_value(
                    draft
                        .digest(DraftLimits::default().input.max_wire_bytes)
                        .unwrap()
                )
                .unwrap()
            );
        }
        for field in ["allocator_before", "source_allocator"] {
            let mut old_allocator = a.as_object_mut().unwrap().remove(field).unwrap();
            let mut new_allocator = b.as_object_mut().unwrap().remove(field).unwrap();
            canonical_instances(&mut old_allocator, old.input().allocator.lineage(), &[]);
            canonical_instances(&mut new_allocator, new.input().allocator.lineage(), &[]);
            assert_eq!(old_allocator, new_allocator);
        }
        assert_eq!(
            canonical_instances(&mut a, old.input().allocator.lineage(), &removed),
            canonical_instances(&mut b, new.input().allocator.lineage(), &[])
        );
        assert!(a == b, "original {case}: unrelated sidecar changed");
        counts[0] += new.input().gems.members.len();
        for parameter in new
            .input()
            .gems
            .members
            .iter()
            .flat_map(|gem| &gem.parameters.members)
        {
            match parameter.value.to_resolved().unwrap() {
                ParameterValue::Boolean(_) => counts[1] += 1,
                ParameterValue::Quantity(_) => counts[2] += 1,
                _ => panic!("unexpected intrinsic value"),
            }
        }
        counts[3] += new
            .input()
            .items
            .members
            .iter()
            .map(|item| item.modifiers.members.len())
            .sum::<usize>();
    }
    assert_eq!(removals, [0, 0, 0, 0, 2]);
    assert_eq!(counts, [478, 415, 415, 96, 110]);
    let reproduced = cwd.join("selected-actions-release-reproduced");
    assert_eq!(assemble(cwd, &output, &reproduced), receipt);
    assert!(
        bundle(&reproduced) == published,
        "release reproduction changed bytes"
    );
    assert!(bundle(prior) == before, "previous package changed");
    output
}
