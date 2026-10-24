//! Reviewed search preferences change source accounting only. They do not
//! resolve configuration, native queries, effective jewels or saved selection.
use super::{originals, tracked_release};
use poe_optimizer_authoring::ReleaseSourceParts;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_draft::{DraftFinalization, DraftListCompletion},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        NormalizationArtifacts, NormalizationLimits, NormalizedImport, OwnedOriginTarget,
        SourceDisposition, SourcePresentationPolicy, normalize_fresh,
    },
    owned_release::{
        StagedOwnedRelease, assemble_owned_release, check_release_source_dependencies,
    },
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, sync::OnceLock};

const REASON: &str = "source-timeless-search-defaults";

/// Restore exactly this flag's contribution to the historical receipt inverse.
/// Checked authored construction, owned by the caller, derives all new seals.
pub(super) fn restore_prior_flag(presentation: &mut Option<SourcePresentationPolicy>) {
    let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
        timeless_search_defaults,
        ..
    }) = presentation
    else {
        panic!("maintained source-presentation policy")
    };
    assert!(*timeless_search_defaults, "tracked opt-in must be explicit");
    *timeless_search_defaults = false;
}

fn without_timeless_accounting() -> &'static StagedOwnedRelease {
    static RELEASE: OnceLock<Box<StagedOwnedRelease>> = OnceLock::new();
    RELEASE.get_or_init(|| {
        let current = tracked_release().release();
        let checked =
            check_release_source_dependencies(current.input(), Default::default()).unwrap();
        let mut parts = ReleaseSourceParts::from_checked(checked).unwrap();
        let mut expected_content = parts.imports.clone();
        restore_prior_flag(&mut parts.imports.normalization.source_presentation);
        expected_content.normalization.source_presentation =
            parts.imports.normalization.source_presentation.clone();
        assert_eq!(parts.imports, expected_content);
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
fn maintained_opt_in_changes_only_presentation_and_its_checked_tree_commitment() {
    let current = tracked_release().release();
    let before = without_timeless_accounting();
    let mut expected = current.input().clone();
    restore_prior_flag(&mut expected.normalization.source_presentation);
    let current_tree = expected.tree.as_mut().unwrap();
    let prior_tree = before.input().tree.as_ref().unwrap();
    assert_ne!(current_tree.normalization, prior_tree.normalization);
    current_tree.normalization = prior_tree.normalization;
    assert_eq!(before.input(), &expected, "no other release input changed");

    let mut current_wire =
        serde_json::to_value(&current.normalization().source_presentation).unwrap();
    assert_eq!(
        current_wire
            .as_object_mut()
            .unwrap()
            .remove("timeless_search_defaults"),
        Some(serde_json::Value::Bool(true))
    );
    assert_eq!(
        serde_json::to_value(&before.normalization().source_presentation).unwrap(),
        current_wire,
        "disabled flag retains the historical omitted-field representation"
    );

    let actual: BTreeMap<_, _> = current.artifacts().collect();
    let prior: BTreeMap<_, _> = before.artifacts().collect();
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        prior.keys().collect::<Vec<_>>()
    );
    let changed: Vec<_> = actual
        .iter()
        .filter_map(|(name, bytes)| (*bytes != prior[name]).then_some(*name))
        .collect();
    assert_eq!(
        changed,
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    assert!(current.evaluation().is_none());
    assert_eq!(current.query_sets(), before.query_sets());
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

fn normalize(
    release: &StagedOwnedRelease,
    evidence: &SourceProjectEvidence<'_>,
    original: usize,
) -> NormalizedImport {
    let recipe = release.assembled().checked();
    let query_set = &release.query_sets()[original - 1];
    assert_eq!(query_set.name.as_str(), format!("original-{original:02}"));
    normalize_fresh(
        evidence,
        evidence.identity().allocator,
        NormalizationArtifacts {
            mappings: release.mapping(),
            registry: recipe.registry(),
            definitions: recipe.schema().as_ref(),
            roles: release.roles(),
            rewards: release.rewards(),
            items: release.items(),
            item_source: release.item_source(),
            tree: release.tree(),
        },
        release.normalization(),
        &query_set.queries,
        Default::default(),
    )
    .unwrap()
}

fn finalization(
    evidence: &SourceProjectEvidence<'_>,
    normalized: &NormalizedImport,
) -> DraftFinalization {
    let schema = tracked_release().release().assembled().checked().schema();
    let draft = normalized.draft();
    let proof = draft
        .prove_intent(schema.as_ref(), Default::default(), Default::default())
        .unwrap();
    let selection = originals::saved(evidence, normalized);
    let checked = draft
        .finalize_selection_checked(
            schema.as_ref(),
            selection,
            &proof,
            Default::default(),
            Default::default(),
        )
        .unwrap();
    assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
    checked.finalization().clone()
}

#[test]
fn five_originals_only_reclassify_exact_defaults_and_keep_every_selected_obligation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let references = root.join("tests/fixtures/breadth-expectations");
    let originals: References =
        serde_json::from_slice(&fs::read(references.join("originals-v1.json")).unwrap()).unwrap();
    assert_eq!(originals.cases.len(), 5);
    let current = tracked_release().release();
    let disabled = without_timeless_accounting();
    let mut changed_origins = Vec::new();
    for (index, original) in originals.cases.iter().enumerate() {
        let number = index + 1;
        assert_eq!(original.source_line, number);
        let xml = fs::read(references.join(&original.input.xml_path)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&xml)),
            original.input.xml_sha256
        );
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            BuildLineage::from_bytes([number as u8; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let before = normalize(disabled, &evidence, number);
        let after = normalize(current, &evidence, number);
        assert_eq!(before.draft(), after.draft(), "original {number}");
        assert_eq!(before.allocator_after(), after.allocator_after());
        assert_eq!(
            serde_json::to_vec(&before.draft().input().query_presets).unwrap(),
            serde_json::to_vec(&after.draft().input().query_presets).unwrap()
        );
        assert_eq!(
            after.draft().input().query_presets.members[0]
                .queries
                .requests
                .members
                .len(),
            22
        );

        let rows: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "TimelessData")
            .collect();
        let [row] = rows.as_slice() else {
            panic!("one independently saved TimelessData for original {number}")
        };
        let parent = evidence.row(row.occurrence().parent().unwrap()).unwrap();
        assert_eq!(parent.occurrence().name(), "Build");
        let attrs: BTreeMap<_, _> = row
            .attributes()
            .iter()
            .map(|attr| {
                assert!(attr.origin().namespace.is_none());
                (attr.origin().name.as_str(), attr.raw())
            })
            .collect();
        assert_eq!(attrs.len(), row.attributes().len());
        let mut expected_attrs = BTreeMap::from([
            ("devotionVariant1", "1"),
            ("devotionVariant2", "1"),
            ("searchList", ""),
            ("searchListFallback", ""),
        ]);
        let admitted = matches!(number, 1 | 4 | 5);
        if !admitted {
            expected_attrs.insert("socketFilterDistance", "0");
        }
        assert_eq!(attrs, expected_attrs, "full retained source attributes");

        let ordinal = row.occurrence().id().ordinal() as usize;
        let prior_origin = &before.sidecar().origins[ordinal];
        assert_eq!(prior_origin.source, row.occurrence().id());
        assert_eq!(prior_origin.disposition, SourceDisposition::Contributes);
        let DraftListCompletion::Pending { id: choice, code } =
            &before.draft().input().choice_presets.members[0]
                .choices
                .completion
        else {
            panic!("independent configuration responsibility remains")
        };
        assert_eq!(code.as_str(), "configuration-roles-not-converted");
        assert!(
            prior_origin
                .links
                .contains(&OwnedOriginTarget::Issue(*choice))
        );
        assert!(
            prior_origin
                .links
                .iter()
                .all(|link| matches!(link, OwnedOriginTarget::Issue(_)))
        );
        if number == 5 {
            assert_eq!(prior_origin.links, [OwnedOriginTarget::Issue(*choice)]);
        }
        let mut expected = before.sidecar().clone();
        if admitted {
            expected.origins[ordinal].disposition =
                SourceDisposition::SourceOnly(OwnedDefinitionKey::new(REASON).unwrap());
            expected.origins[ordinal].links.clear();
        }
        assert_eq!(expected.origins, after.sidecar().origins);
        changed_origins.push(usize::from(admitted));

        // Only the checked policy and dependent tree commitments differ. All
        // source identity, text, draft, allocator and other sidecar fields match.
        let policy_commitment = |release: &StagedOwnedRelease| {
            digest_owned(
                "owned-normalization-policy-v3",
                &(
                    release.normalization(),
                    &release.query_sets()[index].queries,
                ),
                NormalizationLimits::default().max_policy_bytes,
            )
            .unwrap()
        };
        assert_eq!(before.sidecar().policy, policy_commitment(disabled));
        assert_eq!(after.sidecar().policy, policy_commitment(current));
        assert_eq!(before.sidecar().tree_policy, disabled.receipt().tree);
        assert_eq!(after.sidecar().tree_policy, current.receipt().tree);
        expected.policy = policy_commitment(current);
        expected.tree_policy = current.receipt().tree;
        assert_eq!(
            serde_json::to_value(expected).unwrap(),
            serde_json::to_value(after.sidecar()).unwrap()
        );

        let before_finalization = finalization(&evidence, &before);
        let after_finalization = finalization(&evidence, &after);
        assert_eq!(before_finalization, after_finalization);
        let DraftFinalization::Pending { issues, .. } = after_finalization else {
            panic!("source preferences do not complete original {number}")
        };
        assert_eq!(issues.len(), [123, 137, 127, 144, 5][index]);
        if number == 5 {
            let obligations: Vec<_> = issues
                .iter()
                .map(|issue| (issue.code.as_str(), issue.path.as_str()))
                .collect();
            assert_eq!(
                obligations,
                [
                    (
                        "usage-preferences-not-converted",
                        "skill_presets.members[2].intent.usage.completion",
                    ),
                    (
                        "configuration-roles-not-converted",
                        "choice_presets.members[0].choices.completion",
                    ),
                    (
                        "external-assumptions-not-converted",
                        "scenario_presets.members[0].scenario.assumptions.completion",
                    ),
                    (
                        "usage-not-converted",
                        "scenario_presets.members[0].scenario.usage.completion",
                    ),
                    (
                        "scenario-usage-intents-not-converted",
                        "scenario_presets.members[0].scenario.usage_intents.completion",
                    ),
                ]
            );
            let replay = normalize(current, &evidence, number);
            assert_eq!(replay.draft(), after.draft());
            assert_eq!(
                serde_json::to_vec(replay.sidecar()).unwrap(),
                serde_json::to_vec(after.sidecar()).unwrap()
            );
        }
    }
    assert_eq!(changed_origins, [1, 0, 0, 1, 1]);
}
