//! Direct source scalars retain exact saved occurrence ownership and Pending coverage.
#[path = "support/owned_direct_skill_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
#[path = "support/owned_direct_support_targets.rs"]
mod support_targets;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::{FiniteQuantity, OwnedDefinitionKey},
    owned_draft::{DraftLimits, decode_draft},
    owned_schema::{SchemaState, SlotDescriptor, ValueSchema},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{DirectSkillInputPolicy, NormalizationLimits},
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
fn local(id: &Value) -> String {
    id["local"].as_str().unwrap().to_owned()
}
fn issued(allocator: &Value) -> u64 {
    u64::from_str_radix(allocator["last_issued"].as_str().unwrap(), 16).unwrap()
}

#[test]
fn direct_input_authoring_keeps_raw_values_and_unproved_inventories_separate() {
    family::check_authored();
}

fn commitments(package: &StagedOwnedRelease, case: usize, path: &Path, sidecar: &Value) {
    let draft = decode_draft(
        &fs::read(path.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(
        sidecar["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let queries = &package
        .input()
        .query_sets
        .iter()
        .find(|set| set.name.as_str() == format!("original-{case:02}"))
        .unwrap()
        .queries;
    assert_eq!(
        sidecar["policy"],
        json!(
            digest_owned(
                "owned-normalization-policy-v3",
                &(package.normalization(), queries),
                NormalizationLimits::default().max_policy_bytes
            )
            .unwrap()
        )
    );
    let receipt = json!(package.receipt());
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
        assert_eq!(sidecar[field], receipt[binding], "{field}");
    }
    assert_eq!(
        sidecar["mapping_source"],
        json!(package.mapping().source_identity())
    );
    for item in sidecar["item_texts"].as_array().unwrap() {
        assert_eq!(item["attribution"]["item_lines"], sidecar["item_policy"]);
        assert_eq!(item["attribution"]["policy"], sidecar["item_source_policy"]);
    }
}

struct DirectRows {
    skills: BTreeSet<String>,
    issues: BTreeSet<String>,
    links: Vec<(Value, Value)>,
    manual_sources: Vec<(Value, Value)>,
    count: usize,
    generated: usize,
}

fn exact_rows(
    xml: &[u8],
    directory: &Path,
    package: &StagedOwnedRelease,
    d: &Value,
    sidecar: &Value,
) -> DirectRows {
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        draft.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let DirectSkillInputPolicy::PobManualDirectSkillV1 { skills, .. } = package
        .normalization()
        .direct_skill_inputs
        .as_ref()
        .unwrap()
    else {
        panic!("fixture requires Direct V1")
    };
    let origins = sidecar["origins"].as_array().unwrap();
    let origin = |id| {
        origins
            .iter()
            .find(|row| row["source"] == json!(id))
            .unwrap()
    };
    let mut found = DirectRows {
        skills: BTreeSet::new(),
        issues: BTreeSet::new(),
        links: vec![],
        manual_sources: vec![],
        count: 0,
        generated: 0,
    };
    for row in evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Gem")
    {
        let Some(rule) = skills.iter().find(|rule| {
            row.attribute("gemId")
                .is_some_and(|v| v.decoded().unwrap() == rule.game_id)
        }) else {
            continue;
        };
        let group = evidence
            .rows()
            .iter()
            .find(|group| Some(group.occurrence().id()) == row.occurrence().parent())
            .unwrap();
        let set = evidence
            .rows()
            .iter()
            .find(|set| Some(set.occurrence().id()) == group.occurrence().parent())
            .unwrap();
        assert_eq!(set.occurrence().name(), "SkillSet");
        let links = origin(row.occurrence().id())["links"].as_array().unwrap();
        assert!(
            !links.iter().any(|link| link["kind"] == "gem"),
            "catalog Gem identity is not physical ownership"
        );
        let linked: Vec<_> = links
            .iter()
            .filter(|link| link["kind"] == "skill")
            .collect();
        if group
            .attribute("source")
            .is_some_and(|v| !v.decoded().unwrap().is_empty())
        {
            assert!(
                linked.is_empty(),
                "allocation-generated sibling remains unmaterialized"
            );
            found.generated += 1;
            continue;
        }
        assert_eq!(
            linked.len(),
            1,
            "one exact Direct occurrence per reviewed manual row"
        );
        let id = &linked[0]["value"];
        assert!(
            found.skills.insert(local(id)),
            "source rows cannot coalesce"
        );
        found
            .manual_sources
            .push((json!(row.occurrence().id()), id.clone()));
        let skill = d["draft"]["skills"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|skill| &skill["id"] == id)
            .unwrap();
        assert_eq!(
            skill["source"],
            json!({"kind":"direct","value":{"kind":"known","value":rule.skill}})
        );
        assert_eq!(row.attribute("enabled").unwrap().decoded().unwrap(), "true");
        assert_eq!(
            group.attribute("enabled").unwrap().decoded().unwrap(),
            "true"
        );
        assert!(group.attribute("slot").is_none());
        assert_eq!(skill["enabled"], json!({"kind":"known","value":true}));
        assert_eq!(
            skill["scope"],
            json!({"kind":"known","value":{"kind":"shared"}})
        );
        assert_eq!(
            skill["parameters"]["completion"]["code"],
            "direct-skill-parameters-not-converted"
        );
        assert!(
            found
                .issues
                .insert(local(&skill["parameters"]["completion"]["id"]))
        );
        let issue_link = json!({"kind":"issue","value":skill["parameters"]["completion"]["id"]});
        assert_eq!(
            links.iter().filter(|link| **link == issue_link).count(),
            1,
            "raw inventory issue retains its exact source row"
        );
        let parameters = skill["parameters"]["members"].as_array().unwrap();
        assert_eq!(parameters.len(), 2);
        for parameter in &rule.parameters {
            let values: Vec<_> = parameters
                .iter()
                .filter(|row| row["slot"] == json!({"kind":"known","value":parameter.slot}))
                .collect();
            assert_eq!(values.len(), 1);
            assert_eq!(values[0]["value"]["kind"], "known");
            let value: ParameterValue =
                serde_json::from_value(values[0]["value"]["value"].clone()).unwrap();
            let ParameterValue::Quantity(value) = value else {
                panic!("source raw input is a Quantity")
            };
            let attribute = &parameter.value.tiers[0].selectors[0].name;
            let raw = row
                .attribute(attribute)
                .unwrap()
                .decoded()
                .unwrap()
                .parse::<f64>()
                .unwrap();
            assert_eq!(value.value(), raw, "exact authored {attribute}");
            assert_eq!(
                raw,
                if attribute == "level" { 20.0 } else { 0.0 },
                "unchanged original scalar"
            );
        }
        let preset_links: Vec<_> = origin(set.occurrence().id())["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "skill_preset")
            .collect();
        assert_eq!(preset_links.len(), 1);
        let preset_id = &preset_links[0]["value"];
        let containing: Vec<_> = d["draft"]["skill_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|preset| preset["skills"]["members"].as_array().unwrap().contains(id))
            .collect();
        assert_eq!(containing.len(), 1);
        assert_eq!(
            &containing[0]["id"], preset_id,
            "physical source row belongs to its actual saved preset"
        );
        for source_id in [row.occurrence().id(), group.occurrence().id()] {
            let expected = json!({"kind":"skill","value":id});
            assert_eq!(
                origin(source_id)["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|link| **link == expected)
                    .count(),
                1
            );
            found.links.push((json!(source_id), expected));
        }
        found.count += 1;
    }
    let direct: Vec<_> = d["draft"]["skills"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"]["kind"] == "direct")
        .collect();
    assert_eq!(
        direct.len(),
        found.count,
        "no invented Direct rows outside the source census"
    );
    found
}

fn compare(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    commitments(prior, case, &old, &sa);
    commitments(next, case, &new, &sb);
    let mut added = exact_rows(xml, &new, next, &b, &sb);
    assert_eq!(added.count, [2, 0, 0, 0, 9][case - 1]);
    assert_eq!(added.generated, added.count);
    let selected_request = selected::selection(xml, &new);
    if case == 5 {
        let preset = b["draft"]["skill_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == selected_request["build"]["skills"])
            .unwrap();
        let supports = preset["supports"]["members"].as_array().unwrap();
        assert_eq!(
            b["draft"]["supports"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| supports.contains(&row["id"]) && row["target"]["kind"] == "pending")
                .count(),
            6
        );
        assert_eq!(
            preset["authored_support_order"]["completion"]["kind"],
            "pending"
        );
    }
    b["draft"]["skills"]["members"]
        .as_array_mut()
        .unwrap()
        .retain(|row| !added.skills.contains(&local(&row["id"])));
    let mut memberships = 0;
    let mut usage = 0;
    let mut usage_moves = vec![];
    let previous = a["draft"]["skill_presets"]["members"].as_array().unwrap();
    let current = b["draft"]["skill_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(previous.len(), current.len());
    for (before, after) in previous.iter().zip(current) {
        let members = after["skills"]["members"].as_array_mut().unwrap();
        let direct_members: Vec<_> = members
            .iter()
            .filter(|id| added.skills.contains(&local(id)))
            .cloned()
            .collect();
        let count = members.len();
        members.retain(|id| !added.skills.contains(&local(id)));
        let materialized = count - members.len();
        memberships += materialized;
        if before.get("usage_preferences").is_none() && after.get("usage_preferences").is_some() {
            assert!(
                materialized > 0,
                "only a preset receiving Direct inputs gains a new usage inventory"
            );
            let removed = after
                .as_object_mut()
                .unwrap()
                .remove("usage_preferences")
                .unwrap();
            assert_eq!(
                removed["completion"]["code"],
                "usage-preferences-not-converted"
            );
            assert!(removed["members"].as_array().unwrap().is_empty());
            assert!(added.issues.insert(local(&removed["completion"]["id"])));
            usage += 1;
        }
        if let (Some(old_usage), Some(new_usage)) = (
            before.get("usage_preferences"),
            after.get("usage_preferences"),
        ) {
            for usage in [old_usage, new_usage] {
                assert_eq!(usage["completion"]["kind"], "pending");
                assert_eq!(
                    usage["completion"]["code"],
                    "usage-preferences-not-converted"
                );
            }
            let old_issue = &old_usage["completion"]["id"];
            let new_issue = &new_usage["completion"]["id"];
            let sources = |sidecar: &Value, issue: &Value| -> Vec<(Value, usize)> {
                let mut sources = vec![];
                for row in sidecar["origins"].as_array().unwrap() {
                    for (index, link) in row["links"].as_array().unwrap().iter().enumerate() {
                        if link["kind"] == "issue" && &link["value"] == issue {
                            sources.push((row["source"].clone(), index));
                        }
                    }
                }
                sources
            };
            let old_sources = sources(&sa, old_issue);
            let new_sources = sources(&sb, new_issue);
            assert_eq!(
                old_sources.len(),
                1,
                "one old source for the shared preset obligation"
            );
            assert_eq!(
                new_sources.len(),
                1,
                "one current source for the same preset obligation"
            );
            if old_sources[0].0 != new_sources[0].0 {
                assert!(!old_usage["members"].as_array().unwrap().is_empty());
                assert_eq!(
                    old_usage["members"].as_array().unwrap().len(),
                    new_usage["members"].as_array().unwrap().len()
                );
                let first_direct = added
                    .manual_sources
                    .iter()
                    .filter(|(_, skill)| direct_members.contains(skill))
                    .min_by_key(|(source, _)| source["ordinal"].as_u64().unwrap())
                    .unwrap();
                assert_eq!(
                    first_direct.0, new_sources[0].0,
                    "the same preset's first admitted Direct source owns its existing inventory obligation"
                );
                assert!(
                    first_direct.0["ordinal"].as_u64().unwrap()
                        < old_sources[0].0["ordinal"].as_u64().unwrap()
                );
                let old_origin = sa["origins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["source"] == old_sources[0].0)
                    .unwrap();
                for kind in ["gem", "skill"] {
                    assert_eq!(
                        old_origin["links"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|link| link["kind"] == kind)
                            .count(),
                        1,
                        "previous inventory source was one exact physical skill"
                    );
                }
                assert!(
                    old_origin["links"]
                        .as_array()
                        .unwrap()
                        .contains(&json!({"kind":"skill_preset","value":before["id"]})),
                    "previous physical source belongs to that same saved preset"
                );
                usage_moves.push((
                    new_sources[0].0.clone(),
                    old_sources[0].0.clone(),
                    old_sources[0].1,
                    new_issue.clone(),
                ));
            }
        }
    }
    assert_eq!(usage_moves.len(), if case == 5 { 3 } else { 0 });
    assert_eq!(memberships, added.count);
    let before_allocator = a["draft"]["allocator"].clone();
    let after_allocator = b["draft"]["allocator"].clone();
    assert_eq!(
        issued(&after_allocator) - issued(&before_allocator),
        (added.skills.len() + added.issues.len()) as u64,
        "only named new objects consume IDs"
    );
    b["draft"]["allocator"] = before_allocator.clone();
    let mut removed_issues = BTreeSet::new();
    let mut removed_skills = 0;
    let before_fallback = &a["draft"]["choice_presets"]["members"][0]["choices"]["completion"];
    let after_fallback = &b["draft"]["choice_presets"]["members"][0]["choices"]["completion"];
    for fallback in [before_fallback, after_fallback] {
        assert_eq!(fallback["code"], "configuration-roles-not-converted");
    }
    let mut displaced_fallbacks = 0;
    let mut moved_usage_links = 0;
    for row in sb["origins"].as_array_mut().unwrap() {
        let source = row["source"].clone();
        row["links"].as_array_mut().unwrap().retain(|link| {
            if link["kind"] == "issue" && added.issues.contains(&local(&link["value"])) {
                assert!(removed_issues.insert(local(&link["value"])));
                return false;
            }
            if link["kind"] == "skill" && added.skills.contains(&local(&link["value"])) {
                assert!(added.links.contains(&(source.clone(), link.clone())));
                removed_skills += 1;
                return false;
            }
            if usage_moves.iter().any(|(from, _, _, issue)| {
                *from == source && link["kind"] == "issue" && link["value"] == *issue
            }) {
                moved_usage_links += 1;
                return false;
            }
            true
        });
        if added.links.iter().any(|(origin, _)| *origin == source) {
            // Previously unlinked rows used the generic configuration fallback.
            // Exact Direct ownership displaces that link, not the old obligation.
            let previous = sa["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["source"] == source)
                .unwrap();
            assert_eq!(
                previous["links"],
                json!([{"kind":"issue","value":before_fallback["id"]}])
            );
            assert!(row["links"].as_array().unwrap().is_empty());
            row["links"] = json!([{"kind":"issue","value":after_fallback["id"]}]);
            displaced_fallbacks += 1;
        }
    }
    assert_eq!(moved_usage_links, usage_moves.len());
    for (_, to, index, issue) in &usage_moves {
        let target = sb["origins"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["source"] == *to)
            .unwrap();
        let links = target["links"].as_array_mut().unwrap();
        assert!(
            !links
                .iter()
                .any(|link| link["kind"] == "issue" && link["value"] == *issue)
        );
        links.insert(*index, json!({"kind":"issue","value":issue}));
    }
    assert_eq!(removed_issues, added.issues);
    assert_eq!(removed_skills, 2 * added.count);
    assert_eq!(displaced_fallbacks, 2 * added.count);
    for (before, after) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array_mut().unwrap())
    {
        after["attribution"]["policy"] = before["attribution"]["policy"].clone();
        after["attribution"]["item_lines"] = before["attribution"]["item_lines"].clone();
    }
    for field in [
        "policy",
        "mapping",
        "registry",
        "definitions",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
        "draft",
    ] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], before_allocator);
    assert_eq!(sb["allocator_after"], after_allocator);
    sb["allocator_after"] = sa["allocator_after"].clone();
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "all old draft values and obligations survive",
    );
    identity::correspond(
        &sa,
        &mut sb,
        &mut ids,
        "all old source rows and links survive",
    );
    let before = selected::finalize(
        xml,
        &old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        &new,
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
    let added_selected: Vec<_> = y
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| added.issues.contains(&local(&row["id"])))
        .cloned()
        .collect();
    y.as_array_mut()
        .unwrap()
        .retain(|row| !added.issues.contains(&local(&row["id"])));
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    identity::relocate(&mut y, &ids);
    assert!(
        x == y,
        "original {case}: every existing selected issue remains"
    );
    assert_eq!(
        before["finalization"]["issues"].as_array().unwrap().len(),
        [116, 116, 108, 121, 18][case - 1]
    );
    let code_count = |code| {
        added_selected
            .iter()
            .filter(|row| row["code"] == code)
            .count()
    };
    assert_eq!(
        code_count("direct-skill-parameters-not-converted"),
        if matches!(case, 1 | 5) { 2 } else { 0 }
    );
    assert_eq!(
        code_count("usage-preferences-not-converted"),
        usize::from(case == 1)
    );
    assert_eq!(code_count("payload-membership-not-converted"), 0);
    assert_eq!(
        added_selected.len(),
        match case {
            1 => 3,
            5 => 2,
            _ => 0,
        }
    );
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected_request;
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    identity::relocate(&mut new_selection, &ids);
    assert_eq!(
        old_selection, new_selection,
        "saved request and query selection remain exact"
    );
    json!({"original":case,"source_sha256":format!("{:x}",Sha256::digest(xml)),"manual_direct_rows":added.count,"generated_siblings_unmaterialized":added.generated,"new_usage_inventories":usage,"same_preset_usage_source_relocations":usage_moves.len(),"payload_inventory_preserved":true,"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn inherited_direct_policy_rebind(next: &StagedOwnedRelease) {
    let prior_policy = next.normalization().direct_skill_inputs.as_ref().unwrap();
    let DirectSkillInputPolicy::PobManualDirectSkillV1 { skills, .. } = prior_policy else {
        panic!("fixture requires Direct V1")
    };
    let parameter = &skills[0].parameters[0].slot;
    let mut slot = next
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| {
            matches!(row,
                SlotDescriptor::Parameter(entry) if &entry.id == parameter
            )
        })
        .unwrap()
        .clone();
    let SlotDescriptor::Parameter(entry) = &mut slot else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let ValueSchema::Quantity(range) = &mut schema.value else {
        unreachable!()
    };
    // Unpublished test-only contract correction; inherited source recipes and
    // exact slot ownership survive while dependent commitments are rebuilt.
    range.minimum = FiniteQuantity::new(-1000.0, range.minimum.unit().clone()).unwrap();
    let revised = compile_owned_release_revision(
        next,
        OwnedReleaseRevisionInput {
            schema_version: 1,
            before: next.receipt().input,
            release: OwnedDefinitionKey::new("test-direct-raw-bound-revision").unwrap(),
            reason: OwnedDefinitionKey::new("test-inherited-direct-policy-rebind").unwrap(),
            definitions: vec![],
            slots: vec![slot],
        },
        Default::default(),
    )
    .unwrap();
    let mut expected = prior_policy.clone();
    let DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions, roles, ..
    } = &mut expected
    else {
        panic!("fixture requires Direct V1")
    };
    *definitions = revised.receipt().definitions.clone();
    *roles = *revised.roles().identity();
    assert_eq!(
        revised.normalization().direct_skill_inputs.as_ref(),
        Some(&expected)
    );
    assert!(revised.input().query_sets == next.input().query_sets);
    for field in ["definitions", "roles"] {
        let mut stale = revised.input().clone();
        let DirectSkillInputPolicy::PobManualDirectSkillV1 {
            definitions, roles, ..
        } = stale.normalization.direct_skill_inputs.as_mut().unwrap()
        else {
            panic!("fixture requires Direct V1")
        };
        if field == "definitions" {
            *definitions = next.receipt().definitions.clone();
        } else {
            *roles = *next.roles().identity();
        }
        stale.tree.as_mut().unwrap().normalization = digest_owned(
            "owned-normalization-policy-v3",
            &stale.normalization,
            TreePolicyLimits::default().max_base_policy_bytes,
        )
        .unwrap();
        assert!(
            assemble_owned_release(stale, Default::default()).is_err(),
            "revision cannot launder stale Direct {field}"
        );
    }
}

#[test]
#[ignore = "requires exact active-inventory predecessor and authenticated manual-Djinn source witness"]
fn direct_skill_input_publication_preserves_five_originals_and_pending_boundaries() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DIRECT_INPUT_PRIOR").expect("explicit checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DIRECT_INPUT_OUTPUT").expect("new output directory"),
    );
    assert!(!out.exists(), "new output directory only");
    let old_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    inherited_direct_policy_rebind(&next);
    for field in ["definitions", "roles", "catalog", "source", "tree"] {
        let mut bad = next.input().clone();
        if field == "tree" {
            bad.tree = prior.input().tree.clone();
        } else {
            let DirectSkillInputPolicy::PobManualDirectSkillV1 {
                definitions,
                roles,
                catalog,
                source,
                ..
            } = bad.normalization.direct_skill_inputs.as_mut().unwrap()
            else {
                panic!("fixture requires Direct V1")
            };
            match field {
                "definitions" => *definitions = prior.receipt().definitions.clone(),
                "roles" => *roles = *prior.roles().identity(),
                "catalog" => *catalog = digest_owned("stale-direct-catalog", &0, 100).unwrap(),
                "source" => source.revision.push_str("-stale"),
                _ => unreachable!(),
            }
            bad.tree.as_mut().unwrap().normalization = digest_owned(
                "owned-normalization-policy-v3",
                &bad.normalization,
                TreePolicyLimits::default().max_base_policy_bytes,
            )
            .unwrap();
        }
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale {field} remains invalid despite outer rebinding"
        );
    }
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(publish(&package, &rebuilt), json!(next.receipt()));
    let inventory = release::inventory(&package);
    assert_eq!(inventory, release::inventory(&rebuilt));
    assert_eq!(inventory.len(), 18);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().query_sets == prior.input().query_sets);
    let mut reports = vec![];
    for case in 1..=5 {
        let query_file = format!("queries-original-{case:02}.json");
        assert_eq!(inventory[&query_file], old_inventory[&query_file]);
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &xml,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
        reports.push(compare(case, &fs::read(xml).unwrap(), &out, &prior, &next));
    }
    assert_eq!(old_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"queries":110,"artifacts":18,"stale_binding_rejections":5,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
