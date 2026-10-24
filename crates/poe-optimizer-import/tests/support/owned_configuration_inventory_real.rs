//! Explicit real-build refusal gate. A published package is an invocation input,
//! not an ordinary test dependency on an ignored local run directory.
use super::*;
use poe_optimizer_import::owned_release::{
    OwnedReleaseInput, OwnedReleaseReceipt, StagedOwnedRelease, assemble_owned_release,
};
use poe_optimizer_import::owned_tree_policy::OwnedTreeNormalizationPolicy;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn load(path: &Path, input: &str, receipt_sha: &str) -> StagedOwnedRelease {
    let bytes = fs::read(path.join("release.json")).unwrap();
    assert!(bytes.len() < 1024 * 1024);
    assert_eq!(sha(&bytes), receipt_sha);
    let receipt: OwnedReleaseReceipt = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(receipt.input.to_string(), input);
    assert_eq!(receipt.artifacts.len(), 17);
    assert!(receipt.evaluation.is_none());
    let mut artifacts = BTreeMap::new();
    for row in &receipt.artifacts {
        assert!(!row.file.contains(['/', '\\']));
        assert!(row.bytes <= 64 * 1024 * 1024);
        let b = fs::read(path.join(&row.file)).unwrap();
        assert_eq!(b.len(), row.bytes);
        assert_eq!(sha(&b), row.sha256);
        assert!(artifacts.insert(row.file.clone(), b).is_none());
    }
    let value =
        |name: &str| -> Value { serde_json::from_slice(artifacts.get(name).unwrap()).unwrap() };
    let input:OwnedReleaseInput=serde_json::from_value(json!({"schema_version":1,"recipe":{"schema_version":1,"registry":value("registry.json"),"schema":poe_optimizer_data::owned_schema::decode_schema_input(&artifacts["schema.json"], Default::default()).unwrap(),"rules":value("rules.json"),"routing":value("routing.json")},"mapping":value("mapping.json"),"roles":value("roles.json"),"normalization":value("normalization.json"),"rewards":value("rewards.json"),"items":value("items.json"),"item_source":value("item-source.json"),"tree":value("tree-normalization.json"),"query_sets":(1..=5).map(|i|json!({"name":format!("original-{i:02}"),"queries":value(&format!("queries-original-{i:02}.json"))})).collect::<Vec<_>>(),"provenance":receipt.provenance})).unwrap();
    let staged = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(staged.receipt(), &receipt);
    artifacts.insert("release.json".into(), bytes);
    assert_eq!(
        staged
            .artifacts()
            .map(|(name, b)| (name.to_owned(), b.to_vec()))
            .collect::<BTreeMap<_, _>>(),
        artifacts
    );
    staged
}

#[test]
#[ignore = "requires authenticated published current package; does not publish or change originals"]
fn unchanged_originals_keep_configuration_obligations_without_independent_full_source_census() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package = std::env::var("POE_OPTIMIZER_CONFIGURATION_INVENTORY_PRIOR")
        .expect("explicit current package");
    let expected_input = std::env::var("POE_OPTIMIZER_CONFIGURATION_INVENTORY_INPUT")
        .expect("exact input commitment");
    let expected_release = std::env::var("POE_OPTIMIZER_CONFIGURATION_INVENTORY_RELEASE_SHA")
        .expect("exact receipt commitment");
    let staged = load(Path::new(&package), &expected_input, &expected_release);
    assert!(
        staged
            .normalization()
            .configuration_dispositions
            .as_ref()
            .unwrap()
            .inventory
            .is_none()
    );
    for case in 1..=5 {
        let bytes = fs::read(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        if case == 5 {
            assert_eq!(
                sha(&bytes),
                "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
            );
        }
        let imported = ImportedBuildInstance::from_decoded(
            poe_optimizer_import::decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([case as u8; 16]),
            Default::default(),
        )
        .unwrap();
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let queries = &staged
            .query_sets()
            .iter()
            .find(|q| q.name.as_str() == format!("original-{case:02}"))
            .unwrap()
            .queries;
        assert_eq!(queries.len(), 22);
        let run = |policy: &NormalizationPolicy| {
            // The tree authenticates the complete normalization policy, even
            // when its tree semantics are unaffected by an inventory test.
            let prior_tree = staged.tree().unwrap();
            let tree = OwnedTreeNormalizationPolicy::bind_new(
                prior_tree.input().content.clone(),
                staged.assembled().registry(),
                staged.assembled().schema(),
                staged.mapping(),
                policy,
                Default::default(),
            )
            .unwrap();
            let mut inverse = tree.input().clone();
            inverse.normalization = prior_tree.input().normalization;
            assert_eq!(&inverse, prior_tree.input());
            let result = normalize_fresh(
                &evidence,
                *imported.allocator_state(),
                NormalizationArtifacts {
                    mappings: staged.mapping(),
                    registry: staged.assembled().registry(),
                    definitions: staged.assembled().schema(),
                    roles: staged.roles(),
                    rewards: staged.rewards(),
                    items: staged.items(),
                    item_source: staged.item_source(),
                    tree: Some(&tree),
                },
                policy,
                queries,
                Default::default(),
            )
            .unwrap();
            assert_eq!(result.sidecar().tree_policy, Some(*tree.identity()));
            result
        };
        let baseline = run(staged.normalization());
        if case == 5 {
            let tree = evidence
                .rows()
                .iter()
                .find(|r| r.occurrence().name() == "Tree")
                .unwrap();
            let index: usize = tree.attribute("activeSpec").unwrap().raw().parse().unwrap();
            assert_eq!(index, 3);
            let selected = tree.children()[index - 1];
            let owners: Vec<_> = baseline.sidecar().origins[selected.ordinal() as usize]
                .links
                .iter()
                .filter(|link| {
                    matches!(
                        link,
                        OwnedOriginTarget::CharacterPreset(_)
                            | OwnedOriginTarget::AllocationPreset(_)
                    )
                })
                .cloned()
                .collect();
            assert_eq!(owners.len(), 2);
            let DraftListCompletion::Pending { id, code } =
                &baseline.draft().input().saved_variants.completion
            else {
                panic!("saved selection remains Pending")
            };
            assert_eq!(code.as_str(), "saved-selection-not-converted");
            let links =
                &baseline.sidecar().origins[tree.occurrence().id().ordinal() as usize].links;
            assert_eq!(links.len(), 3);
            assert!(owners.iter().all(|owner| links.contains(owner)));
            assert!(links.contains(&OwnedOriginTarget::Issue(*id)));

            let items = evidence
                .rows()
                .iter()
                .find(|r| r.occurrence().name() == "Items")
                .unwrap();
            assert_eq!(items.attribute("activeItemSet").unwrap().raw(), "2");
            let selected = items
                .children()
                .iter()
                .map(|id| &evidence.rows()[id.ordinal() as usize])
                .find(|r| r.occurrence().name() == "ItemSet")
                .unwrap();
            // In this unchanged original key 2 is the first set, not position 2.
            assert_eq!(selected.attribute("id").unwrap().raw(), "2");
            assert_eq!(selected.occurrence().id().ordinal(), 597);
            let owners: Vec<_> = baseline.sidecar().origins[597]
                .links
                .iter()
                .filter(|link| matches!(link, OwnedOriginTarget::EquipmentPreset(_)))
                .cloned()
                .collect();
            assert_eq!(owners.len(), 1);
            let links =
                &baseline.sidecar().origins[items.occurrence().id().ordinal() as usize].links;
            assert_eq!(links.len(), 2);
            assert!(links.contains(&owners[0]));
            assert!(links.contains(&OwnedOriginTarget::Issue(*id)));

            let skills = evidence
                .rows()
                .iter()
                .find(|r| r.occurrence().name() == "Skills")
                .unwrap();
            assert_eq!(skills.attribute("activeSkillSet").unwrap().raw(), "4");
            let selected = skills
                .children()
                .iter()
                .map(|id| &evidence.rows()[id.ordinal() as usize])
                .find(|r| r.attribute("id").is_some_and(|id| id.raw() == "4"))
                .unwrap();
            assert_eq!(selected.occurrence().id().ordinal(), 204);
            let owners: Vec<_> = baseline.sidecar().origins[204]
                .links
                .iter()
                .filter(|link| matches!(link, OwnedOriginTarget::SkillPreset(_)))
                .cloned()
                .collect();
            assert_eq!(owners.len(), 1);
            let links =
                &baseline.sidecar().origins[skills.occurrence().id().ordinal() as usize].links;
            assert_eq!(links.len(), 2);
            assert!(links.contains(&owners[0]));
            assert!(links.contains(&OwnedOriginTarget::Issue(*id)));
        }
        for retained_control in [false, true] {
            let mut policy = staged.normalization().clone();
            let owner = policy.configuration_dispositions.as_mut().unwrap();
            let mut controls = vec![];
            let mut dispositions = vec![];
            if retained_control {
                controls.push(ConfigurationInventoryControl {
                    id: key("reviewed-existing-encounter-constructor"),
                    selector: None,
                    source_states: vec![ConfigurationSourceState::Absent],
                });
                dispositions.push(ConfigurationInventoryDisposition {
                    source_control: key("reviewed-existing-encounter-constructor"),
                    target: ConfigurationInventoryTarget::Adapters {
                        controls: vec![ConfigurationAdapterControl::Encounter],
                    },
                });
            }
            // These are known missing source reviews, not native values or a
            // fake complete catalogue made from the successful output subset.
            let census = ConfigurationSourceCensus {
                mapping_source: owner.mapping_source,
                catalogue_evidence: owner.catalogue_evidence,
                constructor_evidence: owner.constructor_evidence,
                closure: ConfigurationCensusClosure::Partial {
                    gaps: vec![
                        key("full-source-control-and-direct-reader-census-not-reviewed"),
                        key("installed-default-responsibilities-not-accounted"),
                        key("external-fallback-origin-responsibilities-not-accounted"),
                    ],
                },
                controls,
            };
            owner.inventory = Some(ConfigurationInventoryPolicy {
                source_census_identity: configuration_inventory_census_identity(
                    &census,
                    Default::default(),
                )
                .unwrap(),
                source_census: census,
                dispositions,
            });
            let actual = run(&policy);
            assert_eq!(baseline.draft(), actual.draft());
            assert_eq!(baseline.allocator_after(), actual.allocator_after());
            assert_eq!(baseline.sidecar().origins, actual.sidecar().origins);
            let before = serde_json::to_value(baseline.sidecar()).unwrap();
            let mut inverse = serde_json::to_value(actual.sidecar()).unwrap();
            assert_ne!(inverse["policy"], before["policy"]);
            assert_ne!(inverse["tree_policy"], before["tree_policy"]);
            // Both policy/tree identities were independently checked above;
            // preserve every other sidecar field in the exact inverse.
            inverse["policy"] = before["policy"].clone();
            inverse["tree_policy"] = before["tree_policy"].clone();
            assert_eq!(before, inverse);
            assert!(completed(&actual).iter().all(|v| !*v));
            let repeated = run(&policy);
            assert_eq!(
                serde_json::to_value(actual.sidecar()).unwrap(),
                serde_json::to_value(repeated.sidecar()).unwrap()
            );
            if case == 5 {
                let set = evidence
                    .rows()
                    .iter()
                    .find(|r| r.occurrence().name() == "ConfigSet")
                    .unwrap()
                    .occurrence()
                    .id();
                let links = &actual.sidecar().origins[set.ordinal() as usize].links;
                let choice = actual
                    .draft()
                    .input()
                    .choice_presets
                    .members
                    .iter()
                    .find(|c| links.contains(&OwnedOriginTarget::ChoicePreset(c.id)))
                    .unwrap();
                let DraftListCompletion::Pending { code, .. } = &choice.choices.completion else {
                    panic!()
                };
                assert_eq!(code.as_str(), "configuration-roles-not-converted");
                // Exact draft/origin/allocator equality above preserves every
                // current member and obligation. A previous checkpoint's origin
                // ordinals or member counts are not this census's authority.
            }
        }
    }
}
