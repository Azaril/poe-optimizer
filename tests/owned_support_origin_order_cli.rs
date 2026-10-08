//! Publish the existing reviewed local support-order policy for the actual originals.
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    build_identity::*,
    data::DataIdentity,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_draft::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_normalize::SupportOriginOrderPolicy,
    owned_release::{OwnedReleaseProvenance, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/support-origin-order")
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
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg(name)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authoring {
    schema_version: u32,
    before: OwnedContentDigest,
    definitions: DataIdentity,
    normalization: OwnedContentDigest,
    scope: String,
}
#[test]
fn shipped_policy_selects_only_existing_saved_manual_order_semantics() {
    let p: SupportOriginOrderPolicy = read(data().join("policy.json"));
    assert_eq!(p, SupportOriginOrderPolicy::SavedManualGroupOrder {});
    let a: Authoring = read(data().join("authoring.json"));
    assert_eq!(a.schema_version, 1);
    assert_eq!(a.definitions.schema_version, 4);
    for value in [
        json!({"kind":"sorted_assignment_ids"}),
        json!({"kind":"saved_manual_group_order","complete":true}),
    ] {
        assert!(serde_json::from_value::<SupportOriginOrderPolicy>(value).is_err());
    }
}
fn linked<T: DeserializeOwned>(sidecar: &Value, source: SourceOccurrenceId, kind: &str) -> Vec<T> {
    let row = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source"] == serde_json::to_value(source).unwrap())
        .unwrap();
    row["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == kind)
        .map(|v| serde_json::from_value(v["value"].clone()).unwrap())
        .collect()
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    row.attribute(name).unwrap().decoded().unwrap()
}
fn selected_group<'a, 's>(
    e: &'a SourceProjectEvidence<'s>,
) -> (&'a SourceEvidenceRow<'s>, &'a SourceEvidenceRow<'s>) {
    let skills = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Skills")
        .unwrap();
    let selected = attr(skills, "activeSkillSet");
    let set = e
        .rows()
        .iter()
        .find(|r| {
            r.occurrence().name() == "SkillSet"
                && r.occurrence().parent() == Some(skills.occurrence().id())
                && attr(r, "id") == selected
        })
        .unwrap();
    let build = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Build")
        .unwrap();
    let group: usize = attr(build, "mainSocketGroup").parse().unwrap();
    let groups: Vec<_> = set
        .children()
        .iter()
        .map(|id| e.row(*id).unwrap())
        .filter(|r| r.occurrence().name() == "Skill")
        .collect();
    (set, groups[group - 1])
}
fn check_order(xml: &[u8], out: &Path, selected: Option<&str>) -> Value {
    let draft = decode_draft(
        &fs::read(out.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let d = draft.input();
    let sidecar: Value = read(out.join("sidecar.json"));
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        d.allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    assert_eq!(sidecar["source_sha256"], source.source_sha256());
    let supports: BTreeMap<_, _> = d.supports.members.iter().map(|s| (s.id, s)).collect();
    let presets: BTreeMap<_, _> = d
        .skill_presets
        .members
        .iter()
        .flat_map(|p| p.supports.members.iter().map(move |s| (*s, p.id)))
        .collect();
    let mut expected = BTreeMap::<(SkillPresetId, SkillUseId), Vec<SupportAssignmentId>>::new();
    let mut source_rows = BTreeMap::new();
    for row in e.rows() {
        for id in linked::<SupportAssignmentId>(&sidecar, row.occurrence().id(), "support") {
            source_rows.insert(id, row);
            if let DraftSkillTarget::Authored(DraftField::Known { value: target }) =
                &supports[&id].target
            {
                expected
                    .entry((presets[&id], *target))
                    .or_default()
                    .push(id);
            }
        }
    }
    let mut actual = BTreeMap::new();
    let mut complete_presets = Vec::new();
    let mut pending_presets = Vec::new();
    for p in &d.skill_presets.members {
        let order = p.authored_support_order.as_ref().unwrap();
        match &order.completion {
            DraftListCompletion::Complete => {
                assert!(order.to_resolved().is_some());
                assert_eq!(p.supports.completion, DraftListCompletion::Complete);
                complete_presets.push(p.id);
            }
            DraftListCompletion::Pending { code, .. } => {
                assert_eq!(code.as_str(), "support-origin-discovery-not-converted");
                assert!(order.to_resolved().is_none());
                pending_presets.push(p.id);
            }
        }
        for row in &order.members {
            let DraftSkillTarget::Authored(DraftField::Known { value: target }) = &row.target
            else {
                panic!("exact authored target")
            };
            let DraftField::Known { value: origins } = &row.assignments else {
                panic!("known local order")
            };
            assert!(actual.insert((p.id, *target), origins.clone()).is_none());
        }
    }
    assert_eq!(
        actual, expected,
        "exact source encounter order including disabled and repeated definitions"
    );
    assert_eq!(source_rows.len(), d.supports.members.len());
    let mut report = json!({"supports":d.supports.members.len(),"ordered":actual.values().map(Vec::len).sum::<usize>(),"sequences":actual.len(),"queries":d.query_presets.members.iter().map(|p|p.queries.requests.members.len()).sum::<usize>(),"unresolved_targets":supports.values().filter(|s|s.target.to_resolved().is_none()).count(),"authored_complete_presets":complete_presets,"authored_pending_presets":pending_presets});
    if let Some(label) = selected {
        let (set, group) = selected_group(&e);
        let preset = linked::<SkillPresetId>(&sidecar, set.occurrence().id(), "skill_preset");
        assert_eq!(preset.len(), 1);
        let children: Vec<_> = group
            .children()
            .iter()
            .map(|id| e.row(*id).unwrap())
            .filter(|r| r.occurrence().name() == "Gem")
            .collect();
        let selected_supports: Vec<_> = children
            .iter()
            .flat_map(|r| linked::<SupportAssignmentId>(&sidecar, r.occurrence().id(), "support"))
            .collect();
        let targets: Vec<_> = children
            .iter()
            .flat_map(|r| linked::<SkillUseId>(&sidecar, r.occurrence().id(), "skill"))
            .collect();
        let labels: Vec<_> = selected_supports
            .iter()
            .map(|id| attr(source_rows[id], "variantId"))
            .collect();
        match label {
            "twister" => {
                assert_eq!(attr(set, "id"), "6");
                assert_eq!(targets.len(), 1);
                assert_eq!(
                    labels,
                    [
                        "RetreatSupportTwo",
                        "ElementalArmamentSupportTwo",
                        "ProjectileAccelerationSupportThree",
                        "SalvoSupport",
                        "ProlongedDurationSupportTwo"
                    ]
                );
                assert_eq!(actual[&(preset[0], targets[0])], selected_supports);
            }
            "sniper" => {
                assert_eq!(attr(set, "id"), "4");
                assert!(selected_supports.is_empty());
                for target in &targets {
                    assert!(!actual.contains_key(&(preset[0], *target)));
                }
            }
            "disabled_duplicate" => {
                assert_eq!(targets.len(), 1);
                assert_eq!(selected_supports.len(), 6);
                assert_eq!(&labels[..2], ["RetreatSupportTwo", "RetreatSupportTwo"]);
                assert_eq!(
                    supports[&selected_supports[0]].enabled.to_resolved(),
                    Some(false)
                );
                assert_eq!(
                    supports[&selected_supports[1]].enabled.to_resolved(),
                    Some(true)
                );
                assert_ne!(selected_supports[0], selected_supports[1]);
            }
            "ambiguous" => {
                assert!(!selected_supports.is_empty());
                for id in &selected_supports {
                    assert!(supports[id].target.to_resolved().is_none());
                }
                for target in &targets {
                    assert!(!actual.contains_key(&(preset[0], *target)));
                }
            }
            "generated" => {
                // The real policy has no reviewed generated-support prefixes.
                // Retain source rows without inventing physical assignments.
                assert_eq!(attr(group, "source"), "Item:1");
                assert_eq!(children.len(), 6);
                assert!(selected_supports.is_empty());
                assert!(targets.is_empty());
            }
            _ => panic!("unknown test probe"),
        }
        report["selected"] = json!({"case":label,"source_set":attr(set,"id"),"support_variants":labels,"support_assignments":selected_supports,"authored_targets":targets});
    }
    report
}
fn mutated_twister(xml: &str, mode: &str) -> String {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([31; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let (_, group) = selected_group(&e);
    let range = group.occurrence().range();
    let text = &xml[range.clone()];
    let revised = match mode {
        "disabled_duplicate" => {
            let row = group
                .children()
                .iter()
                .map(|id| e.row(*id).unwrap())
                .find(|r| {
                    r.attribute("variantId")
                        .is_some_and(|v| v.decoded().unwrap() == "RetreatSupportTwo")
                })
                .unwrap();
            let gem = &xml[row.occurrence().range()];
            text.replacen(
                gem,
                &format!(
                    "{}{}",
                    gem.replace("enabled=\"true\"", "enabled=\"false\""),
                    gem
                ),
                1,
            )
        }
        "ambiguous" => {
            let row = group
                .children()
                .iter()
                .map(|id| e.row(*id).unwrap())
                .find(|r| {
                    r.attribute("variantId")
                        .is_some_and(|v| v.decoded().unwrap() == "Twister")
                })
                .unwrap();
            let gem = &xml[row.occurrence().range()];
            text.replacen(gem, &format!("{gem}{gem}"), 1)
        }
        "generated" => {
            assert!(group.attribute("source").is_none());
            text.replacen("<Skill ", "<Skill source=\"Item:1\" ", 1)
        }
        _ => panic!("probe"),
    };
    assert_ne!(revised, text);
    format!("{}{}{}", &xml[..range.start], revised, &xml[range.end..])
}

#[test]
#[ignore = "requires SUPPORT_ORDER_CURRENT_RELEASE and a fresh SUPPORT_ORDER_REIMPORT_OUTPUT"]
fn current_release_reimports_all_five_with_exact_authored_assignment_order() {
    let package = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_ORDER_CURRENT_RELEASE")
            .expect("checked current release"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_ORDER_REIMPORT_OUTPUT")
            .expect("fresh output directory"),
    );
    assert!(!out.exists());
    let before = release::inventory(&package);
    let loaded = release::load(&package);
    fs::create_dir_all(&out).unwrap();
    let mut reports = vec![];
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        let directory = out.join(format!("original-{case:02}"));
        release::normalize(&package, &source, case, &directory);
        let order = check_order(&bytes, &directory, None);
        let selection = selected::finalize_with_definitions(
            &bytes,
            &directory,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        if case == 5 {
            let saved: Value = read(out.join("selected-05.json"));
            assert!(
                order["authored_complete_presets"]
                    .as_array()
                    .unwrap()
                    .contains(&saved["build"]["skills"])
            );
            let codes: std::collections::BTreeSet<_> = selection["finalization"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .map(|issue| issue["code"].as_str().unwrap())
                .collect();
            assert_eq!(
                codes,
                [
                    "usage-preferences-not-converted",
                    "configuration-roles-not-converted",
                    "external-assumptions-not-converted",
                    "usage-not-converted"
                ]
                .into_iter()
                .collect()
            );
        }
        reports.push(json!({"case":case,"authored_order":order,"selection":selection}));
    }
    assert_eq!(release::inventory(&package), before);
    write(
        out.join("validation.json"),
        &json!({"release":loaded.receipt().input,"cases":reports}),
    );
}
#[test]
#[ignore = "requires the explicit checked twelve-family successor"]
fn real_publication_retains_exact_twister_order_and_pending_unknown_origins() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_ORDER_PRIOR").expect("explicit predecessor"),
    );
    let hashes = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let a: Authoring = read(data().join("authoring.json"));
    let p: SupportOriginOrderPolicy = read(data().join("policy.json"));
    assert_eq!(prior.receipt().input, a.before);
    assert_eq!(*prior.assembled().schema().identity(), a.definitions);
    assert_eq!(prior.receipt().normalization, a.normalization);
    assert!(prior.normalization().support_origin_order.is_none());
    assert!(prior.normalization().generated_support_prefixes.is_empty());
    let mut policy = prior.normalization().clone();
    policy.support_origin_order = Some(p.clone());
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_ORDER_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let policy_path = out.join("normalization.json");
    write(&policy_path, &policy);
    let compact = out.join("compact");
    let r = run(
        "publish-owned-normalization",
        &[
            &prior_path,
            Path::new("--normalization"),
            &policy_path,
            Path::new("--output"),
            &compact,
        ],
    );
    write(out.join("normalization-receipt.json"), &r);
    let mut input = prior.input().clone();
    input.normalization = read(compact.join("normalization.json"));
    input.tree = Some(read(compact.join("tree-normalization.json")));
    assert_eq!(input.normalization, policy);
    assert_eq!(
        input.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-saved-group-support-order"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned("owned-saved-group-support-order-v1", &(&a, &p), 1024 * 1024)
            .unwrap(),
    });
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    let mut restored = staged.input().clone();
    restored.normalization.support_origin_order = None;
    assert_eq!(restored.normalization, prior.input().normalization);
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(restored, *prior.input());
    let endpoint = out.join("endpoint.json");
    write(&endpoint, &input);
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&endpoint, &package), (&package, &rebuilt)] {
        assert_eq!(
            run("assemble-owned-release", &[src, Path::new("--output"), dst]),
            serde_json::to_value(staged.receipt()).unwrap()
        );
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    for (name, bytes) in staged.artifacts() {
        assert_eq!(bytes, fs::read(package.join(name)).unwrap());
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&name)
        {
            assert_eq!(
                bytes,
                fs::read(prior_path.join(name)).unwrap(),
                "unchanged {name}"
            );
        }
    }
    let mut reports = Vec::new();
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let dst = out.join(format!("original-{case:02}"));
        release::normalize(&package, &source, case, &dst);
        let report = check_order(
            &fs::read(source).unwrap(),
            &dst,
            match case {
                2 => Some("twister"),
                5 => Some("sniper"),
                _ => None,
            },
        );
        write(out.join(format!("original-{case:02}-order.json")), &report);
        reports.push(report);
    }
    assert_eq!(
        reports
            .iter()
            .map(|v| v["supports"].as_u64().unwrap())
            .sum::<u64>(),
        338
    );
    assert_eq!(
        reports
            .iter()
            .map(|v| v["ordered"].as_u64().unwrap())
            .sum::<u64>(),
        273
    );
    assert_eq!(
        reports
            .iter()
            .map(|v| v["queries"].as_u64().unwrap())
            .sum::<u64>(),
        110
    );
    assert!(
        reports
            .iter()
            .any(|v| v["unresolved_targets"].as_u64().unwrap() > 0)
    );
    let xml =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
            .unwrap();
    for mode in ["disabled_duplicate", "ambiguous", "generated"] {
        let xml = mutated_twister(&xml, mode);
        let source = out.join(format!("probe-{mode}.xml"));
        fs::write(&source, xml.as_bytes()).unwrap();
        let dst = out.join(format!("probe-{mode}"));
        release::normalize(&package, &source, 2, &dst);
        write(
            out.join(format!("probe-{mode}-order.json")),
            &check_order(xml.as_bytes(), &dst, Some(mode)),
        );
    }
    assert_eq!(hashes, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.receipt().definitions,"registry":staged.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":staged.input().provenance.len(),"physical_supports":338,"ordered_assignments":273,"queries":110,"twister_selected_ordered":5,"sniper_selected_empty_not_complete":true,"originals_pending":5,"probes":3,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
