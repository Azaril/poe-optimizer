//! Bind physical support assignments to an already admitted manual Direct root.
//! This publication changes occurrence correspondence, not executable readiness.
use super::{commitments, local, publish, read, release, root, selected, write};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_draft::{DraftLimits, decode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        DirectSkillInputPolicy, DirectSupportTargetPolicy, direct_skill_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Historical source receipts retain their exact reports. These are correspondence
// witnesses: they do not certify every support's mechanics or an execution state.
fn source_authority() -> Value {
    let provider: Value =
        read(root().join("data/owned/poe2/3887ae68/djinn-tree-grants/authoring.json"));
    let raw: Value =
        read(root().join("data/owned/poe2/3887ae68/direct-skill-inputs/authoring.json"));
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for authoring in [&provider, &raw] {
        assert_eq!(authoring["source_manifest_sha256"], hash(&manifest_bytes));
        assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
        for pin in authoring["source_files"].as_array().unwrap() {
            let matching: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["path"] == pin["path"])
                .collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0]["sha256"], pin["sha256"]);
        }
        assert_eq!(authoring["source_validation"]["status"], "passed");
    }
    let p = &provider["source_validation"];
    let r = &raw["source_validation"];
    for (off, on, bytes, sha) in [
        (
            &p["evidence_json"],
            &p["evidence_on_json"],
            &p["evidence_bytes"],
            &p["evidence_sha256"],
        ),
        (
            &r["evidence_off"],
            &r["evidence_on"],
            &r["bytes"],
            &r["sha256"],
        ),
    ] {
        let a = fs::read(root().join(off.as_str().unwrap())).unwrap();
        let b = fs::read(root().join(on.as_str().unwrap())).unwrap();
        assert!(
            a == b,
            "complete historical source reports agree across JIT modes"
        );
        assert_eq!(a.len() as u64, bytes.as_u64().unwrap());
        assert_eq!(hash(&a), *sha);
    }
    let observed: Value = read(root().join(p["evidence_json"].as_str().unwrap()));
    assert_eq!(observed["source_hash"], provider["source_manifest_sha256"]);
    assert_eq!(observed["source_revision"], provider["source_revision"]);
    assert_eq!(observed["cases"].as_array().unwrap().len(), 13);
    assert_eq!(observed["evidence"]["native_parity"], false);
    for name in [
        "original",
        "remove-sand-allocation",
        "remove-water-allocation",
        "disable-sand-manual",
        "disable-water-manual",
        "archived-manual-changes",
    ] {
        assert_eq!(
            observed["cases"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["name"] == name)
                .count(),
            1
        );
    }
    let mut code = vec![];
    for (path, expected) in [
        (
            "crates/poe-optimizer-pob/tests/owned_djinn_provider_source.rs",
            "a0ddde415d3136d2e17c71807dc6a3731758bd0238155fc2beab43fb43f7b8a3",
        ),
        (
            "crates/poe-optimizer-pob/tests/support/djinn_provider_source.lua",
            "4710d7fcf625cefbbf94de81076fc51dc70da65b357f7f40be512d1b58229007",
        ),
        (
            "crates/poe-optimizer-pob/tests/owned_djinn_raw_inputs.rs",
            "517c9295ae1b582749134e6bf5a533fe347588afa1001a85d910b7666cf98483",
        ),
    ] {
        let text = fs::read_to_string(root().join(path))
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(hash(text.as_bytes()), expected);
        code.push(json!({"path":path,"sha256":expected,"normalization":"utf8_crlf_to_lf"}));
    }
    json!({"provider":provider,"raw_inputs":raw,"witness_code":code,"scope":"exact-source-occurrence-correspondence","native_execution_complete":false})
}

fn stage(prior: &StagedOwnedRelease, authority: &Value) -> StagedOwnedRelease {
    let b = prior.input();
    assert!(b.normalization.direct_support_targets.is_none());
    let direct = b.normalization.direct_skill_inputs.as_ref().unwrap();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        source,
        dispositions,
        skills,
        ..
    } = direct
    else {
        panic!("exact reviewed V2 Direct source authority required")
    };
    assert_eq!(
        json!(source)["revision"],
        authority["provider"]["source_revision"]
    );
    assert_eq!(dispositions.len(), skills.len());
    let mut normalization = b.normalization.clone();
    normalization.direct_support_targets =
        Some(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 {
            direct_inputs: direct_skill_inputs_identity(direct, Default::default()).unwrap(),
        });
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: b.recipe.clone(),
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        b.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|t| t.input().clone());
    assert_eq!(full.normalization, normalization);
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("manual-direct-support-target-correspondence-v1").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-direct-support-target-authoring-v1",
            &(authority, &normalization.direct_support_targets),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    inverse.normalization.direct_support_targets = None;
    inverse.tree.as_mut().unwrap().normalization = b.tree.as_ref().unwrap().normalization;
    inverse.provenance.pop().unwrap();
    assert_eq!(
        json!(inverse),
        json!(b),
        "only policy, its tree binding and one provenance entry change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
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
    let draft = decode_draft(
        &fs::read(new.join("draft.json")).unwrap(),
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
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    let origins = sb["origins"].as_array().unwrap();
    let origin = |ordinal: u64| {
        origins
            .iter()
            .find(|r| r["source"]["ordinal"] == ordinal)
            .unwrap()
    };
    let all_skills = b["draft"]["skills"]["members"].as_array().unwrap();
    let mut expected = BTreeMap::<String, (Value, u64)>::new();
    // Count every saved manual group, including dormant presets. Neither a name
    // match nor an allocation/grant sibling may stand in for exact source links.
    for group in evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Skill")
    {
        if group.attribute("source").is_some() {
            continue;
        }
        let direct: Vec<_> = group
            .children()
            .iter()
            .flat_map(|id| origin(id.ordinal() as u64)["links"].as_array().unwrap())
            .filter_map(|link| {
                if link["kind"] != "skill" {
                    return None;
                }
                all_skills
                    .iter()
                    .find(|s| s["id"] == link["value"] && s["source"]["kind"] == "direct")
            })
            .collect();
        if direct.len() != 1 {
            continue;
        }
        for id in group.children() {
            for link in origin(id.ordinal() as u64)["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|l| l["kind"] == "support")
            {
                let support = a["draft"]["supports"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == link["value"])
                    .unwrap();
                assert_eq!(support["target"]["kind"], "pending");
                assert!(
                    expected
                        .insert(
                            local(&support["id"]),
                            (direct[0]["id"].clone(), id.ordinal() as u64)
                        )
                        .is_none()
                );
            }
        }
    }
    let mut retired = BTreeMap::<String, u64>::new();
    let mut changed = BTreeMap::<String, Vec<(u64, Value)>>::new();
    let before_supports = a["draft"]["supports"]["members"].as_array().unwrap();
    let after_supports = b["draft"]["supports"]["members"].as_array_mut().unwrap();
    assert_eq!(before_supports.len(), after_supports.len());
    for (before, after) in before_supports.iter().zip(after_supports) {
        assert_eq!(before["id"], after["id"]);
        if before["target"] == after["target"] {
            assert!(!expected.contains_key(&local(&before["id"])));
            continue;
        }
        let (target, ordinal) = expected.get(&local(&before["id"])).unwrap();
        assert_eq!(before["target"]["kind"], "pending");
        assert_eq!(
            before["target"]["value"]["code"],
            "support-target-not-converted"
        );
        assert_eq!(
            after["target"],
            json!({"kind":"authored","value":{"kind":"known","value":target}})
        );
        assert!(
            retired
                .insert(local(&before["target"]["value"]["id"]), *ordinal)
                .is_none()
        );
        changed
            .entry(local(target))
            .or_default()
            .push((*ordinal, before["id"].clone()));
        after["target"] = before["target"].clone();
    }
    assert_eq!(retired.len(), expected.len());
    let mut sequences = 0;
    for (before, after) in a["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .zip(
            b["draft"]["skill_presets"]["members"]
                .as_array_mut()
                .unwrap(),
        )
    {
        assert_eq!(before["id"], after["id"]);
        let members = after["skills"]["members"].as_array().unwrap().clone();
        if let Some(order) = after.get_mut("support_origins") {
            order["members"].as_array_mut().unwrap().retain(|sequence| {
                let target = &sequence["target"]["value"]["value"];
                let Some(rows) = target
                    .get("local")
                    .and_then(Value::as_str)
                    .and_then(|id| changed.get(id))
                else {
                    return true;
                };
                assert!(
                    members.contains(target),
                    "the exact target belongs to this saved preset"
                );
                assert!(
                    !before["support_origins"]["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|s| s["target"] == sequence["target"])
                );
                let mut ordered = rows.clone();
                ordered.sort_by_key(|(ordinal, _)| *ordinal);
                let assignments: Vec<_> = ordered
                    .iter()
                    .map(|(_, id)| json!({"kind":"assignment","value":id}))
                    .collect();
                assert_eq!(
                    sequence["origins"],
                    json!({"kind":"known","value":assignments})
                );
                sequences += 1;
                false
            });
            assert_eq!(
                *order, before["support_origins"],
                "discovery obligation and old sequences survive exactly"
            );
        }
    }
    assert_eq!(sequences, changed.len());
    assert_eq!(
        b, a,
        "every other draft field, query, physical identity and spent ID survives"
    );
    let mut removed = BTreeSet::new();
    assert_eq!(sa["origins"].as_array().unwrap().len(), origins.len());
    for (before, after) in sa["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(sb["origins"].as_array().unwrap())
    {
        let ordinal = before["source"]["ordinal"].as_u64().unwrap();
        before["links"].as_array_mut().unwrap().retain(|link| {
            if link["kind"] == "issue"
                && let Some(expected_ordinal) = retired.get(&local(&link["value"]))
            {
                assert_eq!(ordinal, *expected_ordinal);
                assert!(removed.insert(local(&link["value"])));
                false
            } else {
                true
            }
        });
        assert_eq!(
            *before, *after,
            "no unrelated source link or disposition changes"
        );
    }
    assert_eq!(removed.len(), retired.len());
    assert_eq!(
        sb["schema_version"],
        if expected.is_empty() {
            sa["schema_version"].clone()
        } else {
            json!(20)
        }
    );
    for field in ["schema_version", "policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(
        sb, sa,
        "sidecar changes only declared commitments and exact retired links"
    );
    let before = selected::finalize_with_definitions(
        xml,
        &old,
        &out.join(format!("prior-selection-{case:02}.json")),
        &out.join("prior-schema.json"),
    );
    let after = selected::finalize_with_definitions(
        xml,
        &new,
        &out.join(format!("selection-{case:02}.json")),
        &out.join("package/schema.json"),
    );
    write(out.join(format!("prior-selected-{case:02}.json")), &before);
    write(out.join(format!("selected-{case:02}.json")), &after);
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    let old_count = old_issues.as_array().unwrap().len();
    let new_count = new_issues.as_array().unwrap().len();
    old_issues
        .as_array_mut()
        .unwrap()
        .retain(|issue| !retired.contains_key(&local(&issue["id"])));
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    assert_eq!(
        old_issues, new_issues,
        "all independent selected obligations retain exact IDs"
    );
    assert_eq!(old_count, [114, 117, 109, 122, 11][case - 1]);
    assert_eq!(new_count, [106, 117, 109, 122, 5][case - 1]);
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    json!({"original":case,"source_sha256":hash(xml),"all_saved_resolved_targets":retired.len(),"new_local_sequences":sequences,"selected_before":old_count,"selected_after":new_count,"selected_issue_summary":after["selected_issue_summary"],"all_other_draft_fields_and_source_links_preserved":true,"calculation":"not_run"})
}

#[test]
#[ignore = "requires a checked executable-membership predecessor and historical source reports"]
fn direct_support_target_publication_preserves_five_originals_and_pending_boundaries() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DIRECT_TARGET_PRIOR").expect("explicit checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DIRECT_TARGET_OUTPUT")
            .expect("fresh output directory"),
    );
    assert!(!out.exists(), "fresh publication only");
    let inventory_before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let authority = source_authority();
    let next = stage(&prior, &authority);
    fs::create_dir_all(&out).unwrap();
    write(out.join("authority.json"), &authority);
    write(out.join("endpoint.json"), next.input());
    write(out.join("prior-schema.json"), &prior.input().recipe.schema);
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
    assert!(next.input().query_sets == prior.input().query_sets);
    let mut originals = vec![];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(inventory[&query], inventory_before[&query]);
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
        originals.push(compare(case, &fs::read(xml).unwrap(), &out, &prior, &next));
    }
    assert_eq!(inventory_before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"queries":110,"artifacts":18,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
