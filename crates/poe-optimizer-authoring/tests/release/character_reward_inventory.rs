//! Current-release character reward proofs, with exact Spec ownership.
//! Build and Timeless projections share frame authority, not reward authority.
use super::{originals, tracked_release};
use poe_optimizer_authoring::ReleaseSourceParts;
use poe_optimizer_core::{
    build_identity::{BuildLineage, DraftIssueId},
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_draft::{DraftFinalization, DraftListCompletion},
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        CharacterRewardInventoryPolicy, NormalizationArtifacts, NormalizationError,
        NormalizationLimits, NormalizedImport, OwnedOriginTarget, SourceDisposition,
        normalize_fresh,
    },
    owned_release::{
        OwnedReleaseError, StagedOwnedRelease, assemble_owned_release,
        check_release_source_dependencies,
    },
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path, sync::OnceLock};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}

fn without_inventory() -> &'static StagedOwnedRelease {
    static RELEASE: OnceLock<Box<StagedOwnedRelease>> = OnceLock::new();
    RELEASE.get_or_init(|| {
        let current = tracked_release().release();
        let checked =
            check_release_source_dependencies(current.input(), Default::default()).unwrap();
        let mut parts = ReleaseSourceParts::from_checked(checked).unwrap();
        assert!(
            parts
                .imports
                .normalization
                .character_reward_inventory
                .take()
                .is_some()
        );
        let imports = parts
            .imports
            .into_input(&parts.registry, &parts.schema, Default::default())
            .unwrap();
        let mut input = current.input().clone();
        input.mapping = imports.mapping;
        input.roles = imports.roles;
        input.normalization = imports.normalization;
        input.rewards = imports.rewards;
        input.items = imports.items;
        input.item_source = imports.item_source;
        input.tree = imports.tree;
        Box::new(assemble_owned_release(input, Default::default()).unwrap())
    })
}

#[test]
fn maintained_character_policy_keeps_exact_source_authority_and_only_its_dependency_delta() {
    let current = tracked_release().release();
    let policy = current
        .normalization()
        .character_reward_inventory
        .as_ref()
        .unwrap();
    let historical: CharacterRewardInventoryPolicy = serde_json::from_value(read(
        "data/owned/poe2/3887ae68/character-reward-inventory/policy.json",
    ))
    .unwrap();
    assert_eq!(policy, &historical);
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
        mapping_source,
        target_version,
        tree_version,
    } = policy;
    assert_eq!(mapping_source, current.mapping().source_identity());
    assert_eq!(
        (target_version.as_str(), tree_version.as_str()),
        ("0_1", "0_5")
    );
    let authored = read("data/owned/poe2/3887ae68/character-reward-inventory/authoring.json");
    let manifest = read("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    assert_eq!(authored["source_revision"], manifest["upstream_revision"]);
    assert_eq!(authored["source_target_version"], *target_version);
    assert_eq!(authored["tree_version"], *tree_version);
    assert_eq!(
        authored["source_manifest_sha256"],
        format!(
            "{:x}",
            Sha256::digest(
                fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
                    .unwrap()
            )
        )
    );
    let mut paths = BTreeSet::new();
    for pin in authored["source_files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    for required in [
        "src/Modules/Build.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Classes/TreeTab.lua",
        "src/GameVersions.lua",
    ] {
        assert!(paths.contains(required));
    }
    let wire = serde_json::to_value(policy).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), 4);
    assert_eq!(wire["kind"], "pob_fresh_character_only_empty_v1");

    let disabled = without_inventory();
    let mut expected = current.input().clone();
    expected.normalization.character_reward_inventory = None;
    assert_ne!(
        expected.tree.as_ref().unwrap().normalization,
        disabled.input().tree.as_ref().unwrap().normalization
    );
    expected.tree.as_mut().unwrap().normalization =
        disabled.input().tree.as_ref().unwrap().normalization;
    assert_eq!(
        disabled.input(),
        &expected,
        "only policy and checked Tree commitment differ"
    );
    assert_eq!(current.query_sets(), disabled.query_sets());
    assert_eq!(
        current.receipt().definitions,
        disabled.receipt().definitions
    );
    assert_eq!(current.receipt().registry, disabled.receipt().registry);
    assert!(current.evaluation().is_none());

    let mut stale = current.input().clone();
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { mapping_source, .. } = stale
        .normalization
        .character_reward_inventory
        .as_mut()
        .unwrap();
    *mapping_source = digest_owned("foreign-character-source", &0, 128).unwrap();
    assert!(matches!(
        assemble_owned_release(stale, Default::default()),
        Err(OwnedReleaseError::Normalization(_))
    ));
}

fn normalize_result(
    release: &StagedOwnedRelease,
    evidence: &SourceProjectEvidence<'_>,
    original: usize,
) -> Result<NormalizedImport, NormalizationError> {
    let checked = release.assembled().checked();
    let queries = &release.query_sets()[original - 1];
    assert_eq!(queries.name.as_str(), format!("original-{original:02}"));
    normalize_fresh(
        evidence,
        evidence.identity().allocator,
        NormalizationArtifacts {
            mappings: release.mapping(),
            registry: checked.registry(),
            definitions: checked.schema().as_ref(),
            roles: release.roles(),
            rewards: release.rewards(),
            items: release.items(),
            item_source: release.item_source(),
            tree: release.tree(),
        },
        release.normalization(),
        &queries.queries,
        Default::default(),
    )
}

fn normalize(
    release: &StagedOwnedRelease,
    evidence: &SourceProjectEvidence<'_>,
    original: usize,
) -> NormalizedImport {
    normalize_result(release, evidence, original).unwrap_or_else(|error| {
        panic!(
            "original {original}, character reward policy present={}: {error:?}",
            release.normalization().character_reward_inventory.is_some()
        )
    })
}

#[derive(Clone, Copy)]
struct Expected {
    rewards: bool,
    build: bool,
    timeless: bool,
}

fn assert_delta(
    evidence: &SourceProjectEvidence<'_>,
    before: &NormalizedImport,
    after: &NormalizedImport,
    original: usize,
    expected: Expected,
) -> BTreeSet<DraftIssueId> {
    let mut draft = before.draft().input().clone();
    assert!(!draft.character_presets.members.is_empty());
    let mut retired = BTreeSet::new();
    let mut spec_owners = BTreeSet::new();
    let source_root = evidence.rows()[0].occurrence().id();
    for character in &mut draft.character_presets.members {
        assert!(character.rewards.members.is_empty());
        let DraftListCompletion::Pending { id, code } = &character.rewards.completion else {
            panic!("disabled character reward inventory must remain Pending")
        };
        assert_eq!(code.as_str(), "character-rewards-not-converted");
        if expected.rewards {
            // Select the owning Spec first. Build/Tree are legitimate additional
            // projections and must never be mistaken for duplicate Spec owners.
            let specs: Vec<_> = evidence
                .rows()
                .iter()
                .filter(|row| {
                    row.occurrence().name() == "Spec"
                        && matches!(
                            row.authored_instance(),
                            Some(AuthoredInstanceId::PassiveSpec(_))
                        )
                        && before.sidecar().origins[row.occurrence().id().ordinal() as usize]
                            .links
                            .contains(&OwnedOriginTarget::CharacterPreset(character.id))
                })
                .collect();
            let [spec] = specs.as_slice() else {
                panic!("one exact Spec per Character")
            };
            assert!(!spec.occurrence().has_namespace_context());
            let tree = evidence.row(spec.occurrence().parent().unwrap()).unwrap();
            assert_eq!(tree.occurrence().name(), "Tree");
            assert_eq!(tree.occurrence().parent(), Some(source_root));
            assert!(spec_owners.insert(spec.occurrence().id()));
            let origin = &before.sidecar().origins[spec.occurrence().id().ordinal() as usize];
            assert_eq!(origin.source, spec.occurrence().id());
            assert_eq!(origin.disposition, SourceDisposition::Contributes);
            assert_eq!(
                origin
                    .links
                    .iter()
                    .filter(|link| matches!(link, OwnedOriginTarget::CharacterPreset(_)))
                    .count(),
                1
            );
            assert!(origin.links.contains(&OwnedOriginTarget::Issue(*id)));
            assert!(retired.insert(*id));
            character.rewards.completion = DraftListCompletion::Complete;
        }
    }
    if expected.rewards {
        let specs: BTreeSet<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "Spec")
            .map(|row| row.occurrence().id())
            .collect();
        assert_eq!(
            spec_owners, specs,
            "all selected and archived Specs participate"
        );
    }
    assert_eq!(
        after.draft().input(),
        &draft,
        "only exact character inventories change"
    );
    assert_eq!(before.allocator_after(), after.allocator_after());

    let mut sidecar = before.sidecar().clone();
    for origin in &mut sidecar.origins {
        origin
            .links
            .retain(|link| !matches!(link, OwnedOriginTarget::Issue(id) if retired.contains(id)));
    }
    let DraftListCompletion::Pending {
        id: selection,
        code,
    } = &draft.saved_variants.completion
    else {
        panic!("saved source selection remains unresolved")
    };
    assert_eq!(code.as_str(), "saved-selection-not-converted");
    assert!(draft.saved_variants.members.is_empty());
    let choices: BTreeSet<_> = draft
        .choice_presets
        .members
        .iter()
        .map(|row| {
            let DraftListCompletion::Pending { id, code } = &row.choices.completion else {
                panic!("Config responsibilities stay unresolved")
            };
            assert_eq!(code.as_str(), "configuration-roles-not-converted");
            *id
        })
        .collect();
    if expected.build {
        let builds: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| {
                row.occurrence().name() == "Build" && row.occurrence().parent() == Some(source_root)
            })
            .collect();
        let [build] = builds.as_slice() else {
            panic!("one ordinary Build")
        };
        let origin = &mut sidecar.origins[build.occurrence().id().ordinal() as usize];
        assert_eq!(origin.source, build.occurrence().id());
        assert_eq!(origin.disposition, SourceDisposition::Contributes);
        assert!(
            origin
                .links
                .iter()
                .all(|link| matches!(link, OwnedOriginTarget::Issue(_)))
        );
        assert!(
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::Issue(id) if choices.contains(id)))
        );
        let skill_source = originals::selected_skill_set(evidence);
        let skills: Vec<_> = before.sidecar().origins
            [skill_source.occurrence().id().ordinal() as usize]
            .links
            .iter()
            .filter_map(|link| match link {
                OwnedOriginTarget::SkillPreset(id) => Some(*id),
                _ => None,
            })
            .collect();
        let [skill] = skills.as_slice() else {
            panic!("exact selected SkillSet owner")
        };
        origin.links = draft
            .character_presets
            .members
            .iter()
            .map(|row| OwnedOriginTarget::CharacterPreset(row.id))
            .collect();
        origin.links.extend([
            OwnedOriginTarget::SkillPreset(*skill),
            OwnedOriginTarget::Issue(*selection),
        ]);
    }
    if expected.timeless {
        let rows: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "TimelessData")
            .collect();
        let [row] = rows.as_slice() else {
            panic!("one default Timeless source")
        };
        let parent = evidence.row(row.occurrence().parent().unwrap()).unwrap();
        assert_eq!(parent.occurrence().name(), "Build");
        assert_eq!(parent.occurrence().parent(), Some(source_root));
        assert_eq!(row.attributes().len(), 4);
        for (name, expected) in [
            ("devotionVariant1", "1"),
            ("devotionVariant2", "1"),
            ("searchList", ""),
            ("searchListFallback", ""),
        ] {
            let attr = row.attribute(name).unwrap();
            assert!(attr.origin().namespace.is_none());
            assert_eq!(attr.raw(), expected);
            assert_eq!(attr.decoded().unwrap(), expected);
        }
        let origin = &mut sidecar.origins[row.occurrence().id().ordinal() as usize];
        assert_eq!(origin.source, row.occurrence().id());
        assert_eq!(origin.disposition, SourceDisposition::Contributes);
        assert!(
            origin
                .links
                .iter()
                .all(|link| matches!(link, OwnedOriginTarget::Issue(_)))
        );
        assert!(
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::Issue(id) if choices.contains(id)))
        );
        origin.disposition = SourceDisposition::SourceOnly(
            OwnedDefinitionKey::new("source-timeless-search-defaults").unwrap(),
        );
        origin.links.clear();
    }
    assert_eq!(
        sidecar.origins,
        after.sidecar().origins,
        "all other source responsibilities survive"
    );

    let commitment = |release: &StagedOwnedRelease| {
        digest_owned(
            "owned-normalization-policy-v3",
            &(
                release.normalization(),
                &release.query_sets()[original - 1].queries,
            ),
            NormalizationLimits::default().max_policy_bytes,
        )
        .unwrap()
    };
    let current = tracked_release().release();
    let disabled = without_inventory();
    assert_eq!(sidecar.policy, commitment(disabled));
    assert_eq!(after.sidecar().policy, commitment(current));
    assert_eq!(sidecar.tree_policy, disabled.receipt().tree);
    assert_eq!(after.sidecar().tree_policy, current.receipt().tree);
    assert_eq!(
        sidecar.draft,
        before
            .draft()
            .digest(NormalizationLimits::default().draft.input.max_wire_bytes)
            .unwrap()
    );
    assert_eq!(
        after.sidecar().draft,
        after
            .draft()
            .digest(NormalizationLimits::default().draft.input.max_wire_bytes)
            .unwrap()
    );
    sidecar.policy = commitment(current);
    sidecar.tree_policy = current.receipt().tree;
    sidecar.draft = after
        .draft()
        .digest(NormalizationLimits::default().draft.input.max_wire_bytes)
        .unwrap();
    assert_eq!(
        serde_json::to_value(sidecar).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
    retired
}

fn finalization(
    evidence: &SourceProjectEvidence<'_>,
    value: &NormalizedImport,
) -> DraftFinalization {
    let schema = tracked_release().release().assembled().checked().schema();
    let proof = value
        .draft()
        .prove_intent(schema.as_ref(), Default::default(), Default::default())
        .unwrap();
    value
        .draft()
        .finalize_selection_checked(
            schema.as_ref(),
            originals::saved(evidence, value),
            &proof,
            Default::default(),
            Default::default(),
        )
        .unwrap()
        .finalization()
        .clone()
}

#[derive(Deserialize)]
struct References {
    cases: Vec<Original>,
}
#[derive(Deserialize)]
struct Original {
    source_line: usize,
    input: OriginalInput,
}
#[derive(Deserialize)]
struct OriginalInput {
    xml_path: String,
    xml_sha256: String,
}

#[test]
fn five_originals_retire_exact_spec_rewards_and_preserve_every_other_selected_obligation() {
    let references: References = serde_json::from_value(read(
        "tests/fixtures/breadth-expectations/originals-v1.json",
    ))
    .unwrap();
    assert_eq!(references.cases.len(), 5);
    let mut total_characters = 0;
    for (index, original) in references.cases.iter().enumerate() {
        let number = index + 1;
        assert_eq!(original.source_line, number);
        let bytes = fs::read(
            root()
                .join("tests/fixtures/breadth-expectations")
                .join(&original.input.xml_path),
        )
        .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            original.input.xml_sha256
        );
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([number as u8; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let before = normalize(without_inventory(), &evidence, number);
        let after = normalize(tracked_release().release(), &evidence, number);
        let retired = assert_delta(
            &evidence,
            &before,
            &after,
            number,
            Expected {
                rewards: true,
                build: true,
                timeless: matches!(number, 1 | 4 | 5),
            },
        );
        assert_eq!(retired.len(), [1, 6, 1, 1, 7][index]);
        total_characters += retired.len();
        let choices = &after.draft().input().choice_presets.members;
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].rewards.completion, DraftListCompletion::Complete);
        assert!(!choices[0].rewards.members.is_empty());
        assert_eq!(choices, &before.draft().input().choice_presets.members);
        assert_eq!(
            after.draft().input().rewards,
            before.draft().input().rewards
        );
        assert_eq!(
            after.draft().input().rewards.members.len(),
            choices[0].rewards.members.len()
        );
        assert_eq!(
            originals::saved(&evidence, &before),
            originals::saved(&evidence, &after)
        );
        let DraftFinalization::Pending {
            mut issues,
            queries,
            selection,
            ..
        } = finalization(&evidence, &before)
        else {
            panic!("disabled proof cannot finish a build")
        };
        let prior = issues.len();
        issues.retain(|issue| !retired.contains(&issue.id));
        assert_eq!(
            prior - issues.len(),
            1,
            "only selected Character reward issue retires"
        );
        let DraftFinalization::Pending {
            issues: actual,
            queries: actual_queries,
            selection: actual_selection,
            ..
        } = finalization(&evidence, &after)
        else {
            panic!("empty character rewards do not establish native readiness")
        };
        assert_eq!(issues, actual);
        assert_eq!(queries, actual_queries);
        assert_eq!(selection, actual_selection);
        assert_eq!(queries.requests.members.len(), 22);
        let replay = normalize(tracked_release().release(), &evidence, number);
        assert_eq!(after.draft(), replay.draft());
        assert_eq!(
            serde_json::to_value(after.sidecar()).unwrap(),
            serde_json::to_value(replay.sidecar()).unwrap()
        );
    }
    assert_eq!(total_characters, 16);
}

fn replace_once(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "source anchor {old}");
    text.replacen(old, new, 1)
}

#[test]
fn real_source_contrasts_preserve_distinct_reward_build_and_timeless_authority() {
    let xml =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let stat = r#"<PlayerStat stat="ActiveMinionLimit" value="0"/>"#;
    assert_eq!(xml.matches(stat).count(), 2);
    let cases = [
        (
            "cached-stat",
            xml.replacen(
                stat,
                r#"<PlayerStat stat="ActiveMinionLimit" value="42"/>"#,
                1,
            ),
            (true, true, true),
        ),
        (
            "config-unrelated",
            replace_once(
                &xml,
                "</ConfigSet>",
                "<Input name=\"unreviewed-character-test\" string=\"unknown\"/></ConfigSet>",
            ),
            (true, true, true),
        ),
        (
            "config-none",
            replace_once(
                &xml,
                r#"<Input string="+5% to Lightning Resistance" name="questAct 4Halls Of The DeadTawhoa&apos;s Test"/>"#,
                r#"<Input string="None" name="questAct 4Halls Of The DeadTawhoa&apos;s Test"/>"#,
            ),
            (true, true, true),
        ),
        (
            "unknown-build-child",
            replace_once(&xml, "</Build>", "<Reward id=\"unreviewed\"/></Build>"),
            (false, false, false),
        ),
        (
            "unknown-spec-child",
            xml.replacen("</Spec>", "<Reward id=\"unreviewed\"/></Spec>", 1),
            (false, true, true),
        ),
        (
            "auto-level",
            replace_once(
                &xml,
                "characterLevelAutoMode=\"false\"",
                "characterLevelAutoMode=\"true\"",
            ),
            (false, false, false),
        ),
        (
            "missing-auto-level",
            replace_once(&xml, " characterLevelAutoMode=\"false\"", ""),
            (false, false, false),
        ),
        (
            "namespace-build",
            xml.replacen("<Build ", "<Build xmlns=\"urn:unreviewed\" ", 1),
            (false, false, false),
        ),
        (
            "duplicate-build",
            replace_once(&xml, "</Build>", "</Build><Build/>"),
            (false, false, false),
        ),
        (
            "duplicate-tree",
            replace_once(&xml, "</Tree>", "</Tree><Tree activeSpec=\"1\"/>"),
            (false, false, true),
        ),
        (
            "legacy-root-spec",
            replace_once(&xml, "</PathOfBuilding2>", "<Spec/></PathOfBuilding2>"),
            (false, false, true),
        ),
        (
            "unknown-tree-version",
            xml.replacen("treeVersion=\"0_5\"", "treeVersion=\"unreviewed\"", 1),
            (false, true, true),
        ),
        (
            "unknown-target-version",
            replace_once(
                &xml,
                "targetVersion=\"0_1\"",
                "targetVersion=\"unreviewed\"",
            ),
            (false, false, false),
        ),
        (
            "invalid-active-spec",
            replace_once(&xml, "<Tree activeSpec=\"3\">", "<Tree activeSpec=\"0\">"),
            (false, false, true),
        ),
        (
            "mixed-build-text",
            replace_once(&xml, "</Build>", "unreviewed</Build>"),
            (false, false, false),
        ),
        (
            "nested-cached-stat",
            xml.replacen(
                stat,
                r#"<PlayerStat stat="ActiveMinionLimit" value="0"><Reward/></PlayerStat>"#,
                1,
            ),
            (false, false, false),
        ),
    ];
    let mut refusals = Vec::new();
    for (name, changed, (rewards, build, timeless)) in cases {
        eprintln!("character reward source case: {name}");
        assert_ne!(changed, xml, "{name}");
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(changed.as_bytes()).unwrap(),
            BuildLineage::from_bytes([5; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let allocator = *imported.allocator_state();
        let before = normalize_result(without_inventory(), &evidence, 5);
        let after = normalize_result(tracked_release().release(), &evidence, 5);
        assert_eq!(*imported.allocator_state(), allocator);
        let (before, after) = match (before, after) {
            (Ok(before), Ok(after)) => (before, after),
            (before, after) => {
                // A successful side of an asymmetric resource refusal still
                // has to retain every exact Character reward obligation. The
                // failed side supplies no semantic result to compare with it.
                for (release, result) in [
                    (without_inventory(), &before),
                    (tracked_release().release(), &after),
                ] {
                    let Ok(value) = result else { continue };
                    assert!(!rewards);
                    let mut spec_owners = BTreeSet::new();
                    let mut pending = BTreeSet::new();
                    for character in &value.draft().input().character_presets.members {
                        assert!(character.rewards.members.is_empty());
                        let DraftListCompletion::Pending { id, code } =
                            &character.rewards.completion
                        else {
                            panic!("{name}: bounded refusal cannot complete rewards")
                        };
                        assert_eq!(code.as_str(), "character-rewards-not-converted");
                        assert!(pending.insert(*id));
                        let owners: Vec<_> = evidence
                            .rows()
                            .iter()
                            .filter(|row| {
                                matches!(
                                    row.authored_instance(),
                                    Some(AuthoredInstanceId::PassiveSpec(_))
                                ) && value.sidecar().origins
                                    [row.occurrence().id().ordinal() as usize]
                                    .links
                                    .contains(&OwnedOriginTarget::CharacterPreset(character.id))
                            })
                            .collect();
                        let [owner] = owners.as_slice() else {
                            panic!("exact Spec owner")
                        };
                        assert!(spec_owners.insert(owner.occurrence().id()));
                        let origin =
                            &value.sidecar().origins[owner.occurrence().id().ordinal() as usize];
                        assert_eq!(origin.source, owner.occurrence().id());
                        assert!(origin.links.contains(&OwnedOriginTarget::Issue(*id)));
                    }
                    assert!(!pending.is_empty());
                    assert_eq!(
                        spec_owners,
                        evidence
                            .rows()
                            .iter()
                            .filter(|row| row.occurrence().name() == "Spec")
                            .map(|row| row.occurrence().id())
                            .collect::<BTreeSet<_>>()
                    );
                    let replay = normalize(release, &evidence, 5);
                    assert_eq!(value.draft(), replay.draft());
                    assert_eq!(
                        serde_json::to_value(value.sidecar()).unwrap(),
                        serde_json::to_value(replay.sidecar()).unwrap()
                    );
                    assert_eq!(value.allocator_after(), replay.allocator_after());
                }
                let status = |result: Result<NormalizedImport, NormalizationError>| match result {
                    Ok(_) => "accepted".to_owned(),
                    Err(NormalizationError::Limit("work")) => "work-limit".to_owned(),
                    Err(error) => format!("unexpected error: {error:?}"),
                };
                let states = (name, status(before), status(after));
                eprintln!("character reward bounded refusal: {states:?}");
                refusals.push(states);
                continue;
            }
        };
        let retired = assert_delta(
            &evidence,
            &before,
            &after,
            5,
            Expected {
                rewards,
                build,
                timeless,
            },
        );
        assert_eq!(retired.is_empty(), !rewards, "{name}");
    }
    // A resource refusal returns no semantic result. Keep it separate from the
    // exact successful inverses, and never admit an arbitrary new failure.
    assert_eq!(
        refusals,
        vec![
            (
                "unknown-build-child",
                "accepted".to_owned(),
                "work-limit".to_owned()
            ),
            (
                "unknown-spec-child",
                "work-limit".to_owned(),
                "work-limit".to_owned()
            ),
            (
                "namespace-build",
                "work-limit".to_owned(),
                "work-limit".to_owned()
            ),
        ],
    );
}
