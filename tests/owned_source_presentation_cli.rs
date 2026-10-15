//! Checked presentation dispositions preserve every original native obligation.
#[path = "support/owned_source_empty_party.rs"]
mod empty_party;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slice {
    CachedBuffs,
    EmptyParty,
}
impl Slice {
    fn directory(self) -> &'static str {
        match self {
            Self::CachedBuffs => "source-presentation-v1",
            Self::EmptyParty => "source-empty-party",
        }
    }
    fn disposition(self) -> &'static str {
        match self {
            Self::CachedBuffs => "source-cached-build-buffs",
            Self::EmptyParty => "source-empty-party",
        }
    }
    fn config_count(self) -> usize {
        match self {
            Self::CachedBuffs => 27,
            Self::EmptyParty => 26,
        }
    }
}

use poe_optimizer_core::{
    build_identity::BuildLineage, owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{NormalizationPolicy, SourcePresentationPolicy},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data(slice: Slice) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68")
        .join(slice.directory())
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn hash(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

#[test]
fn presentation_authoring_is_pinned_and_has_no_native_rules() {
    check_authoring(Slice::CachedBuffs);
}

fn check_authoring(slice: Slice) {
    let authoring: Value = read(data(slice).join("authoring.json"));
    let policy: SourcePresentationPolicy = read(data(slice).join("policy.json"));
    assert!(matches!(
        policy,
        SourcePresentationPolicy::PobFreshPresentationV1 {
            empty_socket_urls: true,
            calcs_sections: true,
            tree_view: true,
            empty_notes: true,
            cached_build_buffs: true,
            ..
        }
    ));
    let SourcePresentationPolicy::PobFreshPresentationV1 { empty_party, .. } = &policy;
    assert_eq!(*empty_party, slice == Slice::EmptyParty);
    for (field, file) in [
        ("policy", "policy.json"),
        ("source_facts", "source-facts.json"),
    ] {
        assert_eq!(
            authoring["artifact_sha256"][field],
            hash(data(slice).join(file))
        );
    }
    for field in ["new_definitions", "new_slots", "new_programs", "new_tables"] {
        assert_eq!(authoring[field], 0);
    }
    assert_eq!(authoring["inventory_retirement"], "none");
    let path = root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    assert_eq!(authoring["source_manifest_sha256"], hash(&path));
    let manifest: Value = read(path);
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    let facts: Value = read(data(slice).join("source-facts.json"));
    assert_eq!(facts["source_revision"], authoring["source_revision"]);
    assert_eq!(facts["runtime_observations"], "none");
    if slice == Slice::EmptyParty {
        empty_party::authenticate(&facts);
        return;
    }
    for excerpt in facts["excerpts"].as_array().unwrap() {
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|file| file["path"] == excerpt["file"]["path"]
                    && file["sha256"] == excerpt["file"]["sha256"])
        );
        assert_eq!(
            excerpt["lines"].as_array().unwrap().len() as u64,
            excerpt["last_line"].as_u64().unwrap() - excerpt["first_line"].as_u64().unwrap() + 1
        );
    }
}

fn verify_source(slice: Slice) {
    if slice == Slice::CachedBuffs {
        let facts: Value = read(data(slice).join("source-facts.json"));
        for excerpt in facts["excerpts"].as_array().unwrap() {
            let source = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(excerpt["file"]["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(
                format!("{:x}", Sha256::digest(source.as_bytes())),
                excerpt["file"]["sha256"]
            );
            let first = excerpt["first_line"].as_u64().unwrap() as usize;
            let last = excerpt["last_line"].as_u64().unwrap() as usize;
            assert_eq!(
                json!(
                    source
                        .lines()
                        .skip(first - 1)
                        .take(last - first + 1)
                        .collect::<Vec<_>>()
                ),
                excerpt["lines"]
            );
        }
    }
}

fn stage_party(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let slice = Slice::EmptyParty;
    check_authoring(slice);
    let authoring: Value = read(data(slice).join("authoring.json"));
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    verify_source(slice);
    let before = prior.input();
    let mut policy = before.normalization.clone();
    let proposed: SourcePresentationPolicy = read(data(slice).join("policy.json"));
    let mut inherited = proposed.clone();
    let SourcePresentationPolicy::PobFreshPresentationV1 { empty_party, .. } = &mut inherited;
    assert!(*empty_party);
    *empty_party = false;
    assert_eq!(
        policy.source_presentation,
        Some(inherited),
        "the current policy differs only by the new opt-in"
    );
    policy.source_presentation = Some(proposed);
    let provenance = OwnedReleaseProvenance {
        kind: key("reviewed-empty-party"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-source-empty-party",
            &(authoring, &policy.source_presentation),
            1024 * 1024,
        )
        .unwrap(),
    };
    transition(prior, policy, provenance)
}

fn transition(
    prior: &StagedOwnedRelease,
    policy: NormalizationPolicy,
    provenance: OwnedReleaseProvenance,
) -> StagedOwnedRelease {
    let before = prior.input();
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: before.recipe.clone(),
            successor: before.recipe.clone(),
            mapping: before.mapping.clone(),
            roles: before.roles.clone(),
            normalization: before.normalization.clone(),
            rewards: before.rewards.clone(),
            query_sets: before.query_sets.clone(),
            items: before.items.clone(),
            item_source: before.item_source.clone(),
        },
        before.tree.clone().unwrap(),
        policy.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = before.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, policy);
    let mut tree_inverse = full.tree.clone().unwrap();
    tree_inverse.normalization = before.tree.as_ref().unwrap().normalization;
    assert_eq!(&tree_inverse, before.tree.as_ref().unwrap());
    full.provenance.push(provenance);
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    inverse.normalization = before.normalization.clone();
    inverse.tree = before.tree.clone();
    inverse.provenance.pop();
    assert_eq!(&inverse, before);
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(
        next.receipt().compiled_rules,
        prior.receipt().compiled_rules
    );
    assert!(next.evaluation().is_none());
    next
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

fn audit_lineages(value: &Value, lineage: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(actual) = fields.get("lineage") {
                assert_eq!(
                    actual, lineage,
                    "foreign lineage must not be canonicalized away"
                );
            }
            for v in fields.values() {
                audit_lineages(v, lineage);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                audit_lineages(row, lineage);
            }
        }
        _ => {}
    }
}
fn compare(
    case: usize,
    xml: &[u8],
    prior: &Path,
    out: &Path,
    slice: Slice,
    old_release: &StagedOwnedRelease,
    new_release: &StagedOwnedRelease,
) -> Value {
    let before_dir = out.join(format!("prior-original-{case:02}"));
    let after_dir = out.join(format!("original-{case:02}"));
    let mut before: Value = read(before_dir.join("draft.json"));
    let mut after: Value = read(after_dir.join("draft.json"));
    let before_side: Value = read(before_dir.join("sidecar.json"));
    let after_side: Value = read(after_dir.join("sidecar.json"));
    preservation::authenticate(&before_side, &before_dir, prior, case, old_release);
    preservation::authenticate(
        &after_side,
        &after_dir,
        &out.join("package"),
        case,
        new_release,
    );
    let before_lineage = before["draft"]["allocator"]["lineage"].clone();
    let after_lineage = after["draft"]["allocator"]["lineage"].clone();
    for (draft, side, lineage) in [
        (&before, &before_side, &before_lineage),
        (&after, &after_side, &after_lineage),
    ] {
        audit_lineages(draft, lineage);
        audit_lineages(side, lineage);
        assert_eq!(side["source_sha256"], format!("{:x}", Sha256::digest(xml)));
        assert_eq!(side["source_bytes"], xml.len());
        assert_eq!(side["allocator_after"], draft["draft"]["allocator"]);
    }
    selected::canonical(&mut before);
    selected::canonical(&mut after);
    assert_eq!(before, after, "no native input, issue or allocator changes");
    let mut historical: Value = read(
        prior
            .parent()
            .unwrap()
            .join(format!("original-{case:02}/draft.json")),
    );
    audit_lineages(&historical, &historical["draft"]["allocator"]["lineage"]);
    selected::canonical(&mut historical);
    assert_eq!(
        before, historical,
        "presentation accounting preserves the checked current baseline"
    );

    assert_eq!(before_side["schema_version"], 23);
    assert_eq!(after_side["schema_version"], 23);
    let mut old_origins = before_side["origins"].clone();
    let mut new_origins = after_side["origins"].clone();
    selected::canonical(&mut old_origins);
    selected::canonical(&mut new_origins);
    let instance = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let mut expected = std::collections::BTreeMap::new();
    for row in evidence.rows() {
        let parent = row
            .occurrence()
            .parent()
            .map(|id| &evidence.rows()[id.ordinal() as usize]);
        let parent_name = parent.map(|row| row.occurrence().name());
        let reason = match (row.occurrence().name(), parent_name) {
            ("Buffs", Some("Build")) if slice == Slice::CachedBuffs => {
                assert!(row.children().is_empty());
                assert!(!row.occurrence().has_namespace_context());
                assert!(row.attributes().iter().all(|a| {
                    ["buffList", "combatList", "curseList"].contains(&a.origin().name.as_str())
                        && a.decoded().is_ok()
                }));
                let root = parent.unwrap().occurrence().parent().unwrap();
                assert_eq!(root.ordinal(), 0);
                assert_eq!(evidence.rows()[0].occurrence().name(), "PathOfBuilding2");
                Some("source-cached-build-buffs")
            }
            ("Party", Some("PathOfBuilding2")) if slice == Slice::EmptyParty => {
                assert!(row.children().is_empty());
                assert!(!row.occurrence().has_namespace_context());
                assert_eq!(parent.unwrap().occurrence().id().ordinal(), 0);
                let attrs: std::collections::BTreeMap<_, _> = row
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.as_str(), a.decoded().unwrap()))
                    .collect();
                assert_eq!(
                    attrs,
                    [
                        ("destination", "All"),
                        ("append", "false"),
                        ("ShowAdvanceTools", "false")
                    ]
                    .into_iter()
                    .collect()
                );
                Some(slice.disposition())
            }
            _ => None,
        };
        if let Some(reason) = reason {
            expected.insert(u64::from(row.occurrence().id().ordinal()), reason);
        }
    }
    assert_eq!(expected.len(), 1);
    let expected_leaf = *expected.keys().next().unwrap() as usize;
    assert_eq!(
        old_origins.as_array().unwrap().len(),
        new_origins.as_array().unwrap().len()
    );
    let mut changed = std::collections::BTreeMap::<String, usize>::new();
    for (old, new) in old_origins
        .as_array()
        .unwrap()
        .iter()
        .zip(new_origins.as_array().unwrap())
    {
        if old == new {
            continue;
        }
        assert_eq!(old["source"], new["source"]);
        assert_eq!(old["disposition"]["kind"], "contributes");
        assert!(
            old["links"]
                .as_array()
                .unwrap()
                .iter()
                .all(|link| link["kind"] == "issue")
        );
        assert_eq!(new["disposition"]["kind"], "source_only");
        assert!(new["links"].as_array().unwrap().is_empty());
        let reason = new["disposition"]["value"].as_str().unwrap().to_owned();
        assert_eq!(
            expected.remove(&new["source"]["ordinal"].as_u64().unwrap()),
            Some(reason.as_str())
        );
        *changed.entry(reason).or_default() += 1;
    }
    assert!(
        expected.is_empty(),
        "the one independently censused presentation leaf is accounted for"
    );
    if case == 5 {
        let count = |side: &Value| {
            side["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|origin| {
                    origin["links"].as_array().unwrap().iter().any(|link| {
                        link["kind"] == "issue" && link["value"]["local"] == "00000000000001f2"
                    })
                })
                .count()
        };
        let owned_ranges = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "ModRange")
            .filter(|row| {
                before_side["origins"][row.occurrence().id().ordinal() as usize]["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|link| link["kind"] == "item")
            })
            .count();
        assert_eq!(owned_ranges, 101);
        assert_eq!(count(&before_side), slice.config_count());
        assert_eq!(count(&after_side), slice.config_count() - 1);
    }
    let links = before_side["origins"][expected_leaf]["links"]
        .as_array()
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["kind"], "issue");
    let issue_local = links[0]["value"]["local"].clone();
    let census = |side: &Value| {
        let mut total = 0usize;
        let mut inside = 0usize;
        for origin in side["origins"].as_array().unwrap() {
            if !origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["kind"] == "issue" && l["value"]["local"] == issue_local)
            {
                continue;
            }
            total += 1;
            let mut at = Some(
                evidence.rows()[origin["source"]["ordinal"].as_u64().unwrap() as usize]
                    .occurrence()
                    .id(),
            );
            while let Some(id) = at {
                let row = &evidence.rows()[id.ordinal() as usize];
                if row.occurrence().name() == "Config" {
                    inside += 1;
                    break;
                }
                at = row.occurrence().parent();
            }
        }
        (total, inside, total - inside)
    };
    let counts_before = census(&before_side);
    let counts_after = census(&after_side);
    assert_eq!(
        counts_after,
        (counts_before.0 - 1, counts_before.1, counts_before.2 - 1)
    );
    if slice == Slice::EmptyParty && case == 5 {
        assert_eq!(counts_before, (26, 11, 15));
        assert_eq!(counts_after, (25, 11, 14));
    }
    let mut old_side = before_side;
    for field in [
        "schema_version",
        "policy",
        "tree_policy",
        "draft",
        "origins",
    ] {
        old_side[field] = after_side[field].clone();
    }
    let mut new_side = after_side;
    selected::canonical(&mut old_side);
    selected::canonical(&mut new_side);
    assert_eq!(
        old_side, new_side,
        "all other source metadata and item provenance survives"
    );
    let mut old_selection = selected::selection(xml, &before_dir);
    let mut new_selection = selected::selection(xml, &after_dir);
    audit_lineages(&old_selection, &before_lineage);
    audit_lineages(&new_selection, &after_lineage);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    let old_report = selected::finalize_with_definitions(
        xml,
        &before_dir,
        &out.join(format!("before-selection-{case:02}.json")),
        &prior.join("schema.json"),
    );
    let new_report = selected::finalize_with_definitions(
        xml,
        &after_dir,
        &out.join(format!("selection-{case:02}.json")),
        &out.join("package/schema.json"),
    );
    for report in [&old_report, &new_report] {
        assert!(
            report["intent_validation"]["schema_issues"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let mut old_final = old_report["finalization"].clone();
    let mut new_final = new_report["finalization"].clone();
    // Fresh CLI imports allocate independent document lineages. Authenticate
    // each digest against its actual draft receipt before comparing canonical
    // semantic content (whose complete draft equality was already checked).
    assert_eq!(
        old_final["draft_digest"],
        read::<Value>(before_dir.join("sidecar.json"))["draft"]
    );
    assert_eq!(
        new_final["draft_digest"],
        read::<Value>(after_dir.join("sidecar.json"))["draft"]
    );
    old_final.as_object_mut().unwrap().remove("draft_digest");
    new_final.as_object_mut().unwrap().remove("draft_digest");
    audit_lineages(&old_final, &before_lineage);
    audit_lineages(&new_final, &after_lineage);
    selected::canonical(&mut old_final);
    selected::canonical(&mut new_final);
    assert_eq!(old_final, new_final, "every selected obligation survives");
    if slice == Slice::EmptyParty {
        assert_eq!(
            new_final["issues"].as_array().unwrap().len(),
            [106, 117, 109, 123, 4][case - 1]
        );
    }
    let query_file = format!("queries-original-{case:02}.json");
    assert_eq!(
        fs::read(prior.join(&query_file)).unwrap(),
        fs::read(out.join("package").join(query_file)).unwrap()
    );
    json!({"original":case,"dispositions":changed,"configuration_before":counts_before,"configuration_after":counts_after,"selected_issues":new_final["issues"].as_array().unwrap().len(),"finalization":new_final,"draft_unchanged":true,"selection_unchanged":true})
}

#[test]
#[ignore = "current-data cached-Buffs disable/re-enable regression; pinned source checkout"]
fn presentation_publication_preserves_all_five_original_requests() {
    cached_buffs_roundtrip();
}

fn run_party_publication() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PRESENTATION_PRIOR").expect("explicit prior package"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PRESENTATION_OUTPUT").expect("new output directory"),
    );
    assert!(!out.exists());
    let prior = release::load(&prior_path);
    let next = stage_party(&prior);
    publish_checked(&prior_path, &out, &prior, &next, Slice::EmptyParty);
}

fn publish_checked(
    prior_path: &Path,
    out: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    slice: Slice,
) {
    assert!(!out.exists());
    let old_files = release::inventory(prior_path);
    fs::create_dir_all(out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    let new_files = release::inventory(&package);
    assert_eq!(
        old_files.keys().collect::<Vec<_>>(),
        new_files.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        old_files
            .iter()
            .filter(|(name, hash)| new_files[*name] != **hash)
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    let mut originals = Vec::new();
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            prior_path,
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
        originals.push(compare(
            case,
            &fs::read(xml).unwrap(),
            prior_path,
            out,
            slice,
            prior,
            next,
        ));
    }
    assert_eq!(old_files, release::inventory(prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "queries":110,"rebuild_byte_identical":true,"complete_original_builds":0,
            "numerical_rules_unchanged":true,"source_execution":"not_run"
        }),
    );
}

#[test]
fn empty_party_authoring_is_source_bound_without_native_completion() {
    check_authoring(Slice::EmptyParty);
}

#[test]
fn empty_party_source_proof_rejects_changed_authority_and_scope() {
    let original = empty_party::facts();
    for index in 0..7 {
        let mut changed = original.clone();
        match index {
            0 => changed["archives"][0]["sha256"] = json!("0".repeat(64)),
            1 => changed["excerpts"][1]["first"] = json!(517),
            2 => changed["excerpts"][0]["sha256"] = json!("0".repeat(64)),
            3 => changed["defaults"]["append"] = json!(true),
            4 => changed["destinations"][0] = json!("Future"),
            5 => changed["scope"]["native_inventory_completion"] = json!(true),
            6 => changed["archives"][0]["archive"] = json!("evidence/Build.lua.gz"),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| empty_party::authenticate(&changed)).is_err(),
            "control {index}"
        );
    }
}

#[test]
#[ignore = "requires the checked current release; source archives are portable"]
fn empty_party_publication_preserves_all_five_original_requests() {
    run_party_publication();
}

#[test]
fn presentation_comparison_rejects_foreign_row_lineage() {
    let lineage = json!("expected");
    let mut original = json!({"allocator":{"lineage":"expected"},"origins":[{"source":{"local":"0001","lineage":"expected"}}]});
    audit_lineages(&original, &lineage);
    original["origins"][0]["source"]["lineage"] = json!("foreign");
    assert!(std::panic::catch_unwind(|| audit_lineages(&original, &lineage)).is_err());
}

fn cached_buffs_roundtrip() {
    check_authoring(Slice::CachedBuffs);
    verify_source(Slice::CachedBuffs);
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PRESENTATION_PRIOR").expect("current checked package"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PRESENTATION_OUTPUT")
            .expect("fresh behavior regression output"),
    );
    let original_files = release::inventory(&path);
    let original = release::load(&path);
    let party_authoring: Value = read(data(Slice::EmptyParty).join("authoring.json"));
    assert_eq!(
        json!(original.receipt().input),
        party_authoring["before"],
        "this control uses the authenticated ailment predecessor, not an arbitrary future package"
    );
    // Current-data predecessor, explicitly authored for this behavior control.
    // No historical rule parser or stale receipt compatibility is introduced.
    let mut disabled = original.normalization().clone();
    let expected: SourcePresentationPolicy = read(data(Slice::CachedBuffs).join("policy.json"));
    assert_eq!(disabled.source_presentation, Some(expected));
    let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
        cached_build_buffs, ..
    }) = &mut disabled.source_presentation
    else {
        unreachable!()
    };
    assert!(*cached_build_buffs);
    *cached_build_buffs = false;
    let provenance = |prior: &StagedOwnedRelease, policy: &NormalizationPolicy, kind: &str| {
        OwnedReleaseProvenance {
            kind: key(kind),
            prior_input: prior.receipt().input,
            authoring_input: digest_owned(
                "test-current-cached-buffs-roundtrip",
                &(
                    prior.receipt().input,
                    policy,
                    hash(data(Slice::CachedBuffs).join("source-facts.json")),
                ),
                8 * 1024 * 1024,
            )
            .unwrap(),
        }
    };
    let before = transition(
        &original,
        disabled.clone(),
        provenance(&original, &disabled, "test-disabled-cached-build-buffs"),
    );
    let enabled = original.normalization().clone();
    let after = transition(
        &before,
        enabled.clone(),
        provenance(&before, &enabled, "test-restored-cached-build-buffs"),
    );
    let mut inverse = after.input().clone();
    assert_eq!(
        inverse.provenance.pop().unwrap().kind,
        key("test-restored-cached-build-buffs")
    );
    assert_eq!(
        inverse.provenance.pop().unwrap().kind,
        key("test-disabled-cached-build-buffs")
    );
    assert_eq!(
        &inverse,
        original.input(),
        "exact current full-release roundtrip"
    );
    let baseline = out.with_file_name(format!(
        "{}-baseline",
        out.file_name().unwrap().to_str().unwrap()
    ));
    assert!(!baseline.exists());
    fs::create_dir_all(&baseline).unwrap();
    write(baseline.join("endpoint.json"), before.input());
    let package = baseline.join("package");
    assert_eq!(
        publish(&baseline.join("endpoint.json"), &package),
        json!(before.receipt())
    );
    let loaded = release::load(&package);
    assert_eq!(loaded.input(), before.input());
    // The common comparison independently authenticates this predecessor's
    // saved imports as well as its newly normalized comparison endpoints.
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &package,
            &xml,
            case,
            &baseline.join(format!("original-{case:02}")),
        );
    }
    publish_checked(&package, &out, &before, &after, Slice::CachedBuffs);
    assert_eq!(original_files, release::inventory(&path));
}
