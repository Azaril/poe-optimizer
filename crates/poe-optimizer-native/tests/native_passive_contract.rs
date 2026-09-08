//! Native source-admission and export contracts for the finite class/entrance
//! subset. Expected numerical parity comes from the optional fresh PoB matrix.
use poe_optimizer_core::{evaluation::*, metrics::MeasurementValue, options::EvaluationOptions};
use poe_optimizer_data::{
    bundled,
    tree_data::{TreeAscendancy, TreeClass},
};
use poe_optimizer_native::NativeBackend;
use std::collections::{BTreeMap, BTreeSet};

const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
const MACE: &str = include_str!("../../../tests/fixtures/calibration/mace-wooden.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 10_000 };
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn evaluate(xml: &str) -> EvaluationResult {
    Engine::new(NativeBackend::new())
        .evaluate(&request(xml), BUDGET)
        .unwrap()
}
fn measurements(result: &EvaluationResult) -> BTreeMap<&str, &MeasurementValue> {
    result
        .measurements
        .iter()
        .map(|m| (m.query.id.as_str(), &m.value))
        .collect()
}
fn projection(result: &EvaluationResult) -> serde_json::Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|attachment| {
                attachment.media_type == "application/vnd.poe-optimizer.native-tree+json;version=2"
            })
            .expect("native source projection")
            .content,
    )
    .unwrap()
}
fn fixture(
    class: &TreeClass,
    ascendancy: Option<&TreeAscendancy>,
    paid: Option<u32>,
    mace: bool,
) -> String {
    let (xml, prior_name, prior_id, prior_internal) = if mace {
        (MACE, "Warrior", 3, 6)
    } else {
        (SPARK, "Sorceress", 7, 7)
    };
    xml.replace(
        &format!("className=\"{prior_name}\""),
        &format!("className=\"{}\"", class.name),
    )
    .replace(
        "ascendClassName=\"None\"",
        &format!(
            "ascendClassName=\"{}\"",
            ascendancy.map_or("None", |asc| asc.name.as_str())
        ),
    )
    .replace(
        &format!("classId=\"{prior_id}\""),
        &format!("classId=\"{}\"", class.integer_id),
    )
    .replace(
        &format!("classInternalId=\"{prior_internal}\""),
        &format!("classInternalId=\"{}\"", class.integer_id),
    )
    .replace(
        "ascendClassId=\"0\"",
        &format!(
            "ascendClassId=\"{}\" ascendancyInternalId=\"{}\"",
            ascendancy.map_or(0, |asc| asc.class_index),
            ascendancy.map_or("", |asc| asc.internal_id.as_str())
        ),
    )
    .replace(
        "nodes=\"\"",
        &format!(
            "nodes=\"{}\"",
            paid.map_or(String::new(), |node| node.to_string())
        ),
    )
    .replace(
        "<Notes>",
        "<!-- source identity notes -->\n<Notes>café &amp; manual choices. ",
    )
}
fn check_identity(
    result: &EvaluationResult,
    class: &TreeClass,
    asc: Option<&TreeAscendancy>,
    paid: Option<u32>,
) {
    let data = bundled::class_tree().unwrap();
    assert!(result.diagnostic_only);
    assert!(result.coverage.passives.is_none());
    assert_eq!(result.build.class_name, class.name);
    assert_eq!(
        result.build.ascendancy_name,
        asc.map_or("None", |asc| asc.name.as_str())
    );
    let mut nodes = BTreeSet::from([class.start_node_id]);
    nodes.extend(asc.map(|asc| asc.start_node_id));
    nodes.extend(paid);
    let nodes: Vec<_> = nodes.into_iter().collect();
    assert_eq!(result.build.allocated_nodes, nodes);
    let report = projection(result);
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["ascendancy_allocated_count"], 0);
    assert_eq!(report["evidence_kind"], "native_source_resolution");
    assert_eq!(report["point_budget_verified"], false);
    assert_eq!(report["class"]["index"], class.integer_id);
    assert_eq!(report["class"]["internal_id"], class.integer_id);
    assert_eq!(report["class"]["source_index"], class.source_index);
    assert_eq!(
        report["source"]["bundled_content_sha256"],
        bundled::content_sha256()
    );
    assert_eq!(
        report["source"]["upstream_revision"],
        result.backend.rules_revision
    );
    assert_eq!(report["source"]["tree_version"], "0_5");
    assert_eq!(report["allocated_nodes"], serde_json::json!(nodes));
    assert_eq!(
        report["ordinary_allocated_count"],
        u32::from(paid.is_some())
    );
    if let Some(asc) = asc {
        assert_eq!(report["ascendancy"]["internal_id"], asc.internal_id);
        assert_eq!(report["ascendancy"]["start_node_id"], asc.start_node_id);
    } else {
        assert!(report["ascendancy"].is_null());
    }
    let paid_nodes = report["paid_nodes"].as_array().unwrap();
    assert_eq!(paid_nodes.len(), usize::from(paid.is_some()));
    if let Some(id) = paid {
        let expected = data.entrance(class.integer_id, id).unwrap();
        assert_eq!(paid_nodes[0]["physical_node_id"], id);
        assert_eq!(
            paid_nodes[0]["effective_node_id"],
            expected.effective_source_id
        );
        assert_eq!(paid_nodes[0]["name"], expected.name);
        assert_eq!(paid_nodes[0]["stats"], serde_json::json!(expected.stats));
        assert_eq!(
            paid_nodes[0]["override_provenance"],
            serde_json::to_value(&expected.provenance).unwrap()
        );
    }
    result.validate_recorded().unwrap();
}
#[test]
fn all_class_ascendancy_identities_support_both_skills_and_lossless_exports() {
    let data = bundled::class_tree().unwrap();
    assert_eq!(data.classes.len(), 8);
    assert_eq!(data.ascendancies.len(), 23);
    let mut cases = 0;
    for mace in [false, true] {
        for class in data.classes.values() {
            for asc in std::iter::once(None).chain(
                class
                    .ascendancy_ids
                    .iter()
                    .map(|id| Some(&data.ascendancies[id])),
            ) {
                let xml = fixture(class, asc, None, mace);
                let result = evaluate(&xml);
                check_identity(&result, class, asc, None);
                assert_eq!(result.exports[0].content, xml);
                let reimported = evaluate(&result.exports[0].content);
                assert_eq!(measurements(&reimported), measurements(&result));
                assert_eq!(projection(&reimported), projection(&result));
                // Raw storage index is accepted only alongside this class's valid
                // canonical internal ID; it is not a second class identity.
                let legacy = xml.replace(
                    &format!("classId=\"{}\"", class.integer_id),
                    &format!("classId=\"{}\"", class.source_index),
                );
                let legacy_result = evaluate(&legacy);
                assert_eq!(measurements(&legacy_result), measurements(&result));
                assert_eq!(projection(&legacy_result), projection(&result));
                assert_eq!(legacy_result.exports[0].content, legacy);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 62);
}
#[test]
fn all_16_entrances_keep_physical_identity_and_effective_source_stats_on_both_skills() {
    let data = bundled::class_tree().unwrap();
    let mut cases = 0;
    for mace in [false, true] {
        for class in data.classes.values() {
            for &id in data.class_entrances[&class.integer_id].keys() {
                let xml = fixture(class, None, Some(id), mace);
                let result = evaluate(&xml);
                check_identity(&result, class, None, Some(id));
                assert_eq!(result.exports[0].content, xml);
                let explicit_root = xml.replace(
                    &format!("nodes=\"{id}\""),
                    &format!("nodes=\"{id},{}\"", class.start_node_id),
                );
                let explicit_result = evaluate(&explicit_root);
                assert_eq!(measurements(&explicit_result), measurements(&result));
                assert_eq!(projection(&explicit_result), projection(&result));
                assert_eq!(explicit_result.exports[0].content, explicit_root);
                assert_eq!(
                    measurements(&evaluate(&result.exports[0].content)),
                    measurements(&result)
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 32);
}
#[test]
fn ascendancy_and_shared_root_identities_remain_distinct_with_one_paid_entrance() {
    let data = bundled::class_tree().unwrap();
    let class = &data.classes[&1];
    let lich = &data.ascendancies["Witch3"];
    let abyssal = &data.ascendancies["Witch3b"];
    assert_eq!(lich.start_node_id, abyssal.start_node_id);
    let paid = *data.class_entrances[&1].keys().next().unwrap();
    for mace in [false, true] {
        let ordinary = evaluate(&fixture(class, None, Some(paid), mace));
        for asc in [lich, abyssal] {
            let xml = fixture(class, Some(asc), Some(paid), mace);
            let result = evaluate(&xml);
            check_identity(&result, class, Some(asc), Some(paid));
            assert_eq!(
                measurements(&result),
                measurements(&ordinary),
                "Selecting an ascendancy root with no allocated effects must not change the calculation"
            );
            let ids = result
                .build
                .allocated_nodes
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let explicit = xml.replace(&format!("nodes=\"{paid}\""), &format!("nodes=\"{ids}\""));
            let repeated = evaluate(&explicit);
            assert_eq!(measurements(&repeated), measurements(&result));
            assert_eq!(projection(&repeated), projection(&result));
        }
    }
}
#[test]
fn invalid_identities_and_out_of_scope_allocations_fail_without_silent_repair() {
    let data = bundled::class_tree().unwrap();
    let class = &data.classes[&7];
    let entrances: Vec<_> = data.class_entrances[&7].keys().copied().collect();
    let foreign = data
        .class_entrances
        .values()
        .flat_map(|nodes| nodes.keys())
        .find(|id| !entrances.contains(id))
        .copied()
        .unwrap();
    let xml = fixture(class, None, None, false);
    let invalid = [
        (
            "unknown class",
            xml.replace("classInternalId=\"7\"", "classInternalId=\"4294967295\""),
        ),
        (
            "wrong class name",
            xml.replace("className=\"Sorceress\"", "className=\"Warrior\""),
        ),
        (
            "unrelated class storage index",
            xml.replace("classId=\"7\"", "classId=\"10\""),
        ),
        (
            "foreign ascendancy",
            xml.replace("ascendClassName=\"None\"", "ascendClassName=\"Titan\"")
                .replace(
                    "ascendClassId=\"0\" ascendancyInternalId=\"\"",
                    "ascendClassId=\"1\" ascendancyInternalId=\"Warrior1\"",
                ),
        ),
        (
            "unknown ascendancy",
            xml.replace(
                "ascendClassId=\"0\" ascendancyInternalId=\"\"",
                "ascendClassId=\"1\" ascendancyInternalId=\"UnknownAscendancy\"",
            ),
        ),
        (
            "conflicting no-ascendancy",
            xml.replace(
                "ascendancyInternalId=\"\"",
                "ascendancyInternalId=\"Sorceress1\"",
            ),
        ),
        (
            "two paid nodes",
            xml.replace(
                "nodes=\"\"",
                &format!("nodes=\"{},{}\"", entrances[0], entrances[1]),
            ),
        ),
        (
            "repeated paid ID",
            xml.replace(
                "nodes=\"\"",
                &format!("nodes=\"{},{}\"", entrances[0], entrances[0]),
            ),
        ),
        (
            "unknown paid ID",
            xml.replace("nodes=\"\"", "nodes=\"4294967295\""),
        ),
        (
            "foreign entrance",
            xml.replace("nodes=\"\"", &format!("nodes=\"{foreign}\"")),
        ),
        (
            "foreign class root",
            xml.replace("nodes=\"\"", "nodes=\"47175\""),
        ),
        (
            "unselected ascendancy root",
            xml.replace("nodes=\"\"", "nodes=\"23710\""),
        ),
        (
            "ascendancy paid node",
            xml.replace("nodes=\"\"", "nodes=\"58751\""),
        ),
        (
            "weapon-specific passive",
            xml.replace(
                "masteryEffects=\"\"/>",
                "masteryEffects=\"\"><WeaponSet1 nodes=\"4739\"/></Spec>",
            ),
        ),
        (
            "mastery",
            xml.replace("masteryEffects=\"\"", "masteryEffects=\"1,2\""),
        ),
        (
            "secondary ascendancy",
            xml.replace(
                "ascendClassId=\"0\"",
                "ascendClassId=\"0\" secondaryAscendClassId=\"1\"",
            ),
        ),
    ];
    for (label, incompatible) in invalid {
        assert_ne!(incompatible, xml, "unchanged rejection fixture {label}");
        let error = NativeBackend::new()
            .prepare(&request(&incompatible))
            .err()
            .unwrap_or_else(|| panic!("accepted {label}"));
        assert_eq!(
            error.kind,
            EvaluationErrorKind::UnsupportedCapability,
            "{label}: {error}"
        );
    }
    // This is a genuine allocated node owned by the selected ascendancy;
    // ownership alone must not widen admission beyond reviewed passive records.
    for id in ["Witch3", "Witch3b"] {
        let xml = fixture(
            &data.classes[&1],
            Some(&data.ascendancies[id]),
            Some(58751),
            false,
        );
        let error = NativeBackend::new()
            .prepare(&request(&xml))
            .err()
            .expect("ascendancy node effects remain unsupported");
        assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
    }
}

#[test]
fn all_reviewed_ascendancy_passives_compose_on_both_profiles_and_preserve_source_identity() {
    use poe_optimizer_data::class_tree;
    let data = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let selections = class_tree::selections(data.tree()).unwrap();
    let mut cases = 0;
    for selection in selections
        .iter()
        .filter(|tree| tree.ascendancy_node_id.is_some())
    {
        let resolved = selection.resolve(data.tree()).unwrap();
        for mace in [false, true] {
            let xml = fixture(&resolved.class, resolved.ascendancy.as_ref(), None, mace);
            let paid = selection
                .entrance_node_id
                .into_iter()
                .chain(selection.ascendancy_node_id)
                .collect::<BTreeSet<_>>()
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let xml = xml.replace("nodes=\"\"", &format!("nodes=\"{paid}\""));
            let result = evaluate(&xml);
            assert_eq!(result.exports[0].content, xml);
            assert_eq!(
                result
                    .build
                    .allocated_nodes
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>(),
                resolved.allocated_nodes
            );
            let report = projection(&result);
            assert_eq!(report["schema_version"], 2);
            assert_eq!(
                report["ordinary_allocated_count"],
                usize::from(selection.entrance_node_id.is_some())
            );
            assert_eq!(report["ascendancy_allocated_count"], 1);
            let paid_nodes = report["paid_nodes"].as_array().unwrap();
            let asc = paid_nodes.last().unwrap();
            assert_eq!(asc["allocation_kind"], "ascendancy");
            assert_eq!(
                asc["physical_node_id"],
                selection.ascendancy_node_id.unwrap()
            );
            assert_eq!(
                asc["effective_node_id"],
                resolved
                    .ascendancy_node
                    .as_ref()
                    .unwrap()
                    .effective_source_id
            );
            let effects = report["configured_effects"].as_array().unwrap();
            assert_eq!(
                effects.len(),
                1 + usize::from(selection.entrance_node_id.is_some())
            );
            assert!(effects.iter().any(|effect| effect["ascendancy_id"].as_str()
                == selection.ascendancy_id.as_deref()
                && effect["physical_node_id"].as_u64()
                    == selection.ascendancy_node_id.map(u64::from)));
            let reimported = evaluate(&result.exports[0].content);
            assert_eq!(measurements(&result), measurements(&reimported));
            assert_eq!(projection(&result), projection(&reimported));
            let without_owner = fixture(
                &resolved.class,
                None,
                Some(selection.ascendancy_node_id.unwrap()),
                mace,
            );
            assert!(
                NativeBackend::new()
                    .prepare(&request(&without_owner))
                    .is_err()
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 24);
}

#[test]
fn native_realization_rejects_tampered_ascendancy_resolution_and_effect_evidence() {
    use poe_optimizer_data::{class_tree, game_data};
    use poe_optimizer_import::controlled_mace::{
        ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative,
    };
    use std::sync::Arc;
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let selected = class_tree::selections(data.tree())
        .unwrap()
        .into_iter()
        .find(|tree| tree.entrance_node_id.is_some() && tree.ascendancy_node_id.is_some())
        .unwrap();
    let registry = ControlledMaceCatalog::with_tree_choices(
        data.clone(),
        MACE.into(),
        vec![NormalMaceAlternative {
            id: "weapon".into(),
            item_text: format!(
                "Rarity: NORMAL\n{}\nItem Level: 1\nQuality: 0\nImplicits: 0",
                data.package().weapons[0].name
            ),
        }],
        vec![MaceSupportChoice::None],
        vec![selected],
    )
    .unwrap();
    let baseline = evaluate(MACE);
    let scenario = registry
        .bind_native_baseline(&baseline, &poe_optimizer_native::backend_identity())
        .unwrap();
    let candidate = &registry.alternatives()[0].candidate;
    let materialized = registry.materialize(candidate).unwrap();
    let result = evaluate(&materialized.content);
    registry
        .validate_native_realization(candidate, &result, &scenario)
        .unwrap();
    for pointer in [
        "/schema_version",
        "/ordinary_allocated_count",
        "/ascendancy_allocated_count",
        "/paid_nodes/1/physical_node_id",
        "/paid_nodes/1/effective_node_id",
        "/paid_nodes/1/allocation_kind",
        "/configured_effects/1/ascendancy_id",
        "/configured_effects/1/effects/0/value",
        "/data_identity/content_sha256",
        "/point_budget_verified",
    ] {
        let mut altered: EvaluationResult =
            serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
        let attachment = altered
            .attachments
            .iter_mut()
            .find(|a| a.media_type == "application/vnd.poe-optimizer.native-tree+json;version=2")
            .unwrap();
        let mut report: serde_json::Value = serde_json::from_str(&attachment.content).unwrap();
        *report
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing evidence {pointer}")) =
            serde_json::json!("tampered");
        attachment.content = report.to_string();
        assert!(
            registry
                .validate_native_realization(candidate, &altered, &scenario)
                .is_err(),
            "accepted {pointer}"
        );
    }
    let mut old_version = result;
    old_version
        .attachments
        .iter_mut()
        .find(|a| a.media_type == "application/vnd.poe-optimizer.native-tree+json;version=2")
        .unwrap()
        .media_type = "application/vnd.poe-optimizer.native-tree+json;version=1".into();
    assert!(
        registry
            .validate_native_realization(candidate, &old_version, &scenario)
            .is_err()
    );
}

#[test]
fn ascendancy_allocations_require_the_exact_owner_and_separate_category_limits() {
    use poe_optimizer_data::{class_tree, game_data};
    let data = game_data::bundled_snapshot().unwrap();
    let selection = class_tree::selections(data.tree())
        .unwrap()
        .into_iter()
        .find(|tree| tree.entrance_node_id.is_some() && tree.ascendancy_node_id.is_some())
        .unwrap();
    let resolved = selection.resolve(data.tree()).unwrap();
    let source = fixture(&resolved.class, resolved.ascendancy.as_ref(), None, true);
    let selected_asc = selection.ascendancy_node_id.unwrap();
    let foreign_asc = *data
        .tree()
        .ascendancy_nodes
        .keys()
        .find(|id| **id != selected_asc)
        .unwrap();
    let ordinary: Vec<_> = data.tree().class_entrances[&selection.class_id]
        .keys()
        .copied()
        .collect();
    for paid in [
        format!("{foreign_asc}"),
        format!("{selected_asc},{foreign_asc}"),
        format!("{selected_asc},{},{}", ordinary[0], ordinary[1]),
    ] {
        let xml = source.replace("nodes=\"\"", &format!("nodes=\"{paid}\""));
        assert_ne!(xml, source);
        let error = NativeBackend::new()
            .prepare(&request(&xml))
            .err()
            .expect("unsupported allocations admitted");
        assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
    }
}
