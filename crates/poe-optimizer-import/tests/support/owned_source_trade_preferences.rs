use super::*;

const FRAME: &str = "source-trade-search-weights";
const WEIGHT: &str = "source-trade-search-weight";

fn trade_policy(a: &Artifacts) -> NormalizationPolicy {
    let mut p = enabled_policy(a);
    let SourcePresentationPolicy::PobFreshPresentationV1 {
        trade_search_weights,
        ..
    } = p.source_presentation.as_mut().unwrap();
    *trade_search_weights = true;
    p
}

fn trade_xml(body: &str) -> String {
    xml("").replace("</Items>", &format!("{body}</Items>"))
}

fn count(body: &str) -> (usize, usize) {
    let a = artifacts(false);
    let r = normalize(&trade_xml(body), &a, &trade_policy(&a), Default::default()).unwrap();
    (
        presentation_count(&r, FRAME),
        presentation_count(&r, WEIGHT),
    )
}

fn assert_only_trade_origins_changed(before: &NormalizedImport, after: &NormalizedImport) {
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut expected = before.sidecar().origins.clone();
    for (old, new) in expected.iter_mut().zip(&after.sidecar().origins) {
        if matches!(&new.disposition, SourceDisposition::SourceOnly(code) if [FRAME, WEIGHT].contains(&code.as_str()))
        {
            assert_eq!(old.source, new.source);
            assert!(matches!(old.disposition, SourceDisposition::Contributes));
            assert!(!old.links.is_empty());
            assert!(
                old.links
                    .iter()
                    .all(|link| matches!(link, OwnedOriginTarget::Issue(_)))
            );
            assert!(new.links.is_empty());
            old.links.clear();
            old.disposition = new.disposition.clone();
        }
    }
    assert_eq!(expected, after.sidecar().origins);
}

#[test]
fn trade_preferences_preserve_finite_weights_duplicates_and_unknown_stat_names_as_metadata() {
    for weight in ["0", "-0", "-2.50", "0.125", "1e3", "1000000"] {
        let body = format!(
            r#"<TradeSearchWeights><Stat label="Caller label" stat="UnknownFutureMetric" weightMult="{weight}"/><Stat label="Repeated choice" stat="UnknownFutureMetric" weightMult="{weight}"/></TradeSearchWeights>"#
        );
        assert_eq!(count(&body), (1, 2));
        let a = artifacts(false);
        let input = trade_xml(&body);
        let before = normalize(&input, &a, &enabled_policy(&a), Default::default()).unwrap();
        let after = normalize(&input, &a, &trade_policy(&a), Default::default()).unwrap();
        assert_only_trade_origins_changed(&before, &after);
        assert_eq!(
            before.sidecar().source_sha256,
            after.sidecar().source_sha256
        );
        assert_eq!(before.sidecar().source_bytes, after.sidecar().source_bytes);
    }
    assert_eq!(count("<TradeSearchWeights/>"), (1, 0));
}

#[test]
fn malformed_trade_frame_keeps_every_member_unresolved() {
    let good = "<Stat label='Damage' stat='FullDPS' weightMult='1.00'/>";
    for bad in [
        "<Stat label='Damage' stat='FullDPS'/>",
        "<Stat stat='FullDPS' weightMult='1'/>",
        "<Stat label='Damage' weightMult='1'/>",
        "<Stat label='' stat='FullDPS' weightMult='1'/>",
        "<Stat label='Damage' stat='' weightMult='1'/>",
        "<Stat label='Damage' stat='FullDPS' weightMult='NaN'/>",
        "<Stat label='Damage' stat='FullDPS' weightMult='inf'/>",
        "<Stat label='Damage' stat='FullDPS' weightMult='1e9999'/>",
        "<Stat label='Damage' stat='FullDPS' weightMult=' 1'/>",
        "<Stat label='Damage' stat='FullDPS' weightMult='1' future='true'/>",
        "<Stat label='Damage' stat='FullDPS' weightMult='1'><Future/></Stat>",
        "<Stat label='Damage' stat='FullDPS' weightMult='1'>text</Stat>",
        "<Stat xmlns='urn:other' label='Damage' stat='FullDPS' weightMult='1'/>",
        "<Other/>",
        "<!-- unknown -->",
        "<![CDATA[]]>",
        "text",
    ] {
        let body = format!("<TradeSearchWeights>{good}{bad}</TradeSearchWeights>");
        assert_eq!(count(&body), (0, 0), "{body}");
    }
    for body in [
        format!("<TradeSearchWeights future='x'>{good}</TradeSearchWeights>"),
        format!("<TradeSearchWeights xmlns='urn:other'>{good}</TradeSearchWeights>"),
        format!("<TradeSearchWeights>{good}</TradeSearchWeights><TradeSearchWeights/>"),
    ] {
        assert_eq!(count(&body), (0, 0), "{body}");
    }
    assert!(decode_build(trade_xml("<TradeSearchWeights><Stat label='D' stat='FullDPS' weightMult='1' weightMult='2'/></TradeSearchWeights>").as_bytes()).is_err());
}

#[test]
fn trade_preferences_require_the_exact_ordinary_items_parent_and_source_commitment() {
    let a = artifacts(false);
    let base = trade_xml("<TradeSearchWeights/>");
    for input in [
        base.replace("</PathOfBuilding2>", "<Items/></PathOfBuilding2>"),
        base.replace("<Items activeItemSet=", "<Items future='x' activeItemSet="),
        base.replace("<PathOfBuilding2>", "<PathOfBuilding2 future='x'>"),
        base.replace(
            "<Items activeItemSet=",
            "<Items xmlns='urn:other' activeItemSet=",
        ),
        xml("<TradeSearchWeights/>"),
        xml("").replace(
            "</PathOfBuilding2>",
            "<TradeSearchWeights/></PathOfBuilding2>",
        ),
    ] {
        let r = normalize(&input, &a, &trade_policy(&a), Default::default()).unwrap();
        assert_eq!(presentation_count(&r, FRAME), 0, "{input}");
    }
    let mut p = trade_policy(&a);
    let SourcePresentationPolicy::PobFreshPresentationV1 { mapping_source, .. } =
        p.source_presentation.as_mut().unwrap();
    *mapping_source = "f".repeat(64).parse().unwrap();
    assert!(matches!(
        normalize(&base, &a, &p, Default::default()),
        Err(NormalizationError::Binding)
    ));
}

#[test]
fn opting_into_trade_metadata_changes_no_build_query_or_allocator_in_any_original() {
    let a = artifacts(false);
    let p = enabled_policy(&a);
    let wire = serde_json::to_value(p.source_presentation.as_ref().unwrap()).unwrap();
    assert!(wire.get("trade_search_weights").is_none());
    let restored: SourcePresentationPolicy = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), wire);
    for (input, counts) in [
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-03.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-04.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
    ]
    .into_iter()
    .zip([(1, 0), (1, 0), (1, 0), (0, 0), (1, 2)])
    {
        let before = normalize(input, &a, &p, Default::default()).unwrap();
        let after = normalize(input, &a, &trade_policy(&a), Default::default()).unwrap();
        assert_only_trade_origins_changed(&before, &after);
        assert_eq!(presentation_count(&after, FRAME), counts.0);
        assert_eq!(presentation_count(&after, WEIGHT), counts.1);
    }
}

#[test]
fn trade_frame_validation_is_charged_and_failure_does_not_contaminate_replay() {
    let a = artifacts(false);
    let input = trade_xml(
        "<TradeSearchWeights><Stat label='D' stat='Anything' weightMult='-1.25'/></TradeSearchWeights>",
    );
    let prior = enabled_policy(&a);
    let policy = trade_policy(&a);
    let expected = normalize(&input, &a, &policy, Default::default()).unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        match normalize(
            &input,
            &a,
            &prior,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        ) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_)) => low = mid + 1,
            Err(e) => panic!("unexpected baseline error: {e}"),
        }
    }
    assert!(matches!(
        normalize(
            &input,
            &a,
            &policy,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let replay = normalize(&input, &a, &policy, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_vec(expected.sidecar()).unwrap(),
        serde_json::to_vec(replay.sidecar()).unwrap()
    );
}
