use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_loading::{ItemLoadingCatalog, ItemMetadataTable, ItemMetadataValue as V},
};
use poe_optimizer_import::item_loading::*;
use std::{collections::BTreeMap, sync::OnceLock};
fn data() -> &'static GameDataSnapshot {
    static D: OnceLock<GameDataSnapshot> = OnceLock::new();
    D.get_or_init(|| bundled_snapshot().unwrap())
}
fn context(c: &ItemLoadingCatalog, v: &str) -> JewelRadiusContext {
    JewelRadiusContext::resolve(c, v, JewelRadiusProvenance::ExplicitCaller).unwrap()
}
fn machine() -> ItemLoadMachine<'static> {
    let mut m = ItemLoadMachine::new(data().item_loading());
    m.set_xml_attributes(&[("id".into(), "1".into())].into());
    m
}
fn index(m: &ItemLoadMachine<'_>) -> Option<f64> {
    match m.state().retained_fields.get("jewelRadiusIndex") {
        Some(ItemScalar::Number(v)) => v.value(),
        _ => None,
    }
}
fn row(label: &str, inner: f64, outer: f64) -> V {
    V::Table(ItemMetadataTable {
        fields: [
            ("label".into(), V::Text(label.into())),
            ("inner".into(), V::Number(inner)),
            ("outer".into(), V::Number(outer)),
        ]
        .into(),
        indexed: BTreeMap::new(),
    })
}
#[test]
fn radius_context_preserves_request_canonical_selection_owner_and_shared_storage() {
    let c = data().item_loading();
    let r = context(c, "release 0_5 preview");
    assert_eq!(r.evidence().requested_tree_version, "release 0_5 preview");
    assert_eq!(r.evidence().resolved_radius_version, "0_1");
    assert!(r.matches_definitions(c));
    assert!(r.shares_storage_with(&r.clone()));
    let independent = ItemLoadingCatalog::new(c.data().clone()).unwrap();
    assert!(!r.matches_definitions(&independent));
    let mut m = ItemLoadMachine::new(&independent);
    assert!(m.set_jewel_radius_context(r).is_err());
    assert!(
        JewelRadiusContext::resolve(c, "0_1", JewelRadiusProvenance::BuildInitialization).is_err()
    );
    JewelRadiusContext::resolve(
        c,
        &c.policy().jewel_radius.latest_tree_version,
        JewelRadiusProvenance::BuildInitialization,
    )
    .unwrap();
}
#[test]
fn numeric_version_fallback_and_source_maximum_comparison_use_injected_data() {
    let mut d = data().item_loading().data().clone();
    d.policy.jewel_radius.distance_multiplier = 2.;
    d.jewel_radii.fields = [
        (
            "0_2".into(),
            V::Array(vec![row("a", 3., 20.), row("b", 4., 30.)]),
        ),
        ("0_10".into(), V::Array(vec![row("c", 0., 99.)])),
    ]
    .into();
    let c = ItemLoadingCatalog::new(d).unwrap();
    let r = context(&c, "0_9");
    assert_eq!(r.evidence().resolved_radius_version, "0_2");
    // The source compares the next unscaled outer radius against the scaled accumulator.
    assert_eq!(r.evidence().maximum_radius, 40.);
    let first = r.radii().indexed[&1].as_table().unwrap();
    assert_eq!(first.fields["outerSquared"].as_f64(), Some(1600.));
    assert_eq!(first.fields["innerSquared"].as_f64(), Some(36.));
    assert_eq!(context(&c, "0_10").evidence().maximum_radius, 198.);
}
#[test]
fn canonical_lookup_failures_malformed_versions_and_nonfinite_results_stay_explicit() {
    let c = data().item_loading();
    assert_eq!(
        JewelRadiusContext::resolve(c, "unknown", JewelRadiusProvenance::ExplicitCaller)
            .unwrap_err()
            .kind,
        JewelRadiusErrorKind::Source
    );
    let mut d = c.data().clone();
    let v = d.jewel_radii.fields.remove("0_1").unwrap();
    d.jewel_radii.fields.insert("00_01".into(), v);
    let c = ItemLoadingCatalog::new(d).unwrap();
    assert!(
        JewelRadiusContext::resolve(&c, "0_5", JewelRadiusProvenance::ExplicitCaller)
            .unwrap_err()
            .message
            .contains("canonical")
    );
    let mut d = data().item_loading().data().clone();
    d.policy.jewel_radius.distance_multiplier = f64::MAX;
    let c = ItemLoadingCatalog::new(d).unwrap();
    assert_eq!(
        JewelRadiusContext::resolve(&c, "0_5", JewelRadiusProvenance::ExplicitCaller)
            .unwrap_err()
            .kind,
        JewelRadiusErrorKind::Unsupported
    );
}
#[test]
fn ambiguous_label_lookup_preserves_label_without_inventing_source_pairs_order() {
    let mut d = data().item_loading().data().clone();
    d.jewel_radii.fields.insert(
        "0_1".into(),
        V::Array(vec![row("Small", 0., 5.), row("Small", 0., 8.)]),
    );
    let c = ItemLoadingCatalog::new(d).unwrap();
    let mut m = ItemLoadMachine::new(&c);
    m.set_jewel_radius_context(context(&c, "0_5")).unwrap();
    m.apply_text(
        "Rarity: NORMAL\nRuby\nRadius: Small",
        &mut UnavailableItemLoadProvider,
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Pending);
    assert!(
        m.pending()
            .unwrap()
            .message
            .contains("multiple source traversal")
    );
    assert!(
        matches!(m.state().retained_fields.get("jewelRadiusLabel"),Some(ItemScalar::Text(v)) if v=="Small")
    );
    assert_eq!(index(&m), None);
}
#[test]
fn missing_context_is_a_reached_dependency_after_the_label_write() {
    let mut m = machine();
    m.apply_text(
        "Rarity: NORMAL\nRuby\nRadius: Small",
        &mut BuiltinItemLoadProvider::new(data()),
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Pending);
    assert!(m.pending().unwrap().message.contains("item-call context"));
    assert!(
        matches!(m.state().retained_fields.get("jewelRadiusLabel"),Some(ItemScalar::Text(v)) if v=="Small")
    );
}
#[test]
fn radius_headers_retain_unmatched_index_and_variable_nil_clears_it() {
    let mut m = machine();
    m.set_jewel_radius_context(context(data().item_loading(), "0_5"))
        .unwrap();
    let mut p = BuiltinItemLoadProvider::new(data());
    m.apply_text("Rarity: NORMAL\nRuby\nRadius: Small", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), Some(1.));
    let old = m.assembled().unwrap().clone();
    m.set_jewel_radius_context(context(data().item_loading(), "0_2"))
        .unwrap();
    assert!(m.assembled().unwrap().shares_storage_with(&old));
    m.apply_text("Rarity: NORMAL\nRuby\nRadius: Unknown", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), Some(1.));
    m.apply_text("Rarity: NORMAL\nRuby\nRadius: Variable", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), None);
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert!(
        m.assembled()
            .unwrap()
            .field(m.assembled().unwrap().root(), "jewelRadiusIndex")
            .is_none()
    );
}
struct Parser;
impl ItemLoadProvider for Parser {
    fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
        let (key, value) = match r.text.as_str() {
            "owned radius" => (
                "radiusIndex",
                V::Table(ItemMetadataTable {
                    fields: [("marker".into(), V::Number(7.))].into(),
                    indexed: BTreeMap::new(),
                }),
            ),
            "zero override" => ("timeLostJewelRadiusOverride", V::Number(0.)),
            _ => {
                return DependencyResult::Available(ParseOutcome {
                    modifiers: None,
                    extra: Some(r.text.clone()),
                });
            }
        };
        let payload = V::Table(ItemMetadataTable {
            fields: [("key".into(), V::Text(key.into())), ("value".into(), value)].into(),
            indexed: BTreeMap::new(),
        });
        DependencyResult::Available(ParseOutcome {
            extra: None,
            modifiers: Some(vec![ItemMetadataTable {
                fields: [
                    ("name".into(), V::Text("JewelData".into())),
                    ("type".into(), V::Text("LIST".into())),
                    ("value".into(), payload),
                    ("flags".into(), V::Number(0.)),
                    ("keywordFlags".into(), V::Number(0.)),
                ]
                .into(),
                indexed: BTreeMap::new(),
            }]),
        })
    }
}
#[test]
fn owned_radius_override_survives_final_assembly_without_stale_scalar_replay() {
    let mut m = machine();
    m.set_jewel_radius_context(context(data().item_loading(), "0_5"))
        .unwrap();
    let mut p = NativeItemLoadProvider::with_native_assembly(data(), Parser);
    m.apply_text("Rarity: NORMAL\nRuby\nRadius: Small", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), Some(1.));
    m.apply_text(
        "Rarity: NORMAL\nRuby\nRadius: Variable\nowned radius",
        &mut p,
    )
    .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert!(!m.state().retained_fields.contains_key("jewelRadiusIndex"));
    let item = m.assembled().unwrap();
    let radius = item
        .field(item.root(), "jewelRadiusIndex")
        .unwrap()
        .as_table()
        .unwrap();
    assert_eq!(item.field(radius, "marker").unwrap().as_number(), Some(7.));
    let current = item
        .field(item.root(), "jewelData")
        .unwrap()
        .as_table()
        .unwrap();
    let current_radius = item
        .field(current, "radiusIndex")
        .unwrap()
        .as_table()
        .unwrap();
    assert_ne!(
        radius, current_radius,
        "final BuildModList does not replay the earlier ParseRaw radius assignment"
    );
    m.apply_text("Rarity: NORMAL\nRuby\nRadius: Medium", &mut p)
        .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), Some(2.));
    m.apply_text(
        "Rarity: NORMAL\nRuby\nRadius: Variable\nowned radius\nzero override",
        &mut p,
    )
    .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(index(&m), Some(0.));
}
#[test]
fn ordinary_cluster_headers_without_cluster_metadata_are_inert_like_source() {
    let mut m = machine();
    let mut p = BuiltinItemLoadProvider::new(data());
    m.apply_text(
        "Rarity: NORMAL\nRuby\nCluster Jewel Skill: missing\nCluster Jewel Node Count: 12",
        &mut p,
    )
    .unwrap();
    m.finish_load(&mut p).unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert!(m.state().cluster_jewel.is_none());
    assert!(
        !m.state()
            .retained_fields
            .contains_key("clusterJewelNodeCount")
    );
}

#[test]
fn oversized_radius_inventory_is_rejected_before_copying_rows() {
    let mut d = data().item_loading().data().clone();
    d.jewel_radii
        .fields
        .insert("0_1".into(), V::Array(vec![row("Small", 0., 5.); 257]));
    let c = ItemLoadingCatalog::new(d).unwrap();
    let e =
        JewelRadiusContext::resolve(&c, "0_5", JewelRadiusProvenance::ExplicitCaller).unwrap_err();
    assert_eq!(e.kind, JewelRadiusErrorKind::Resource);
    assert!(e.message.contains("row inventory"));
}
