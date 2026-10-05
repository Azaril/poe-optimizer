//! Source presentation proofs change provenance only, never an owned obligation.
use super::*;

#[path = "owned_source_presentation_frames.rs"]
mod frame_tests;

const EMPTY_URL: &str = r#"<SocketIdURL nodeId="123" itemPbURL="" name="Caller label"/>"#;

fn enabled_policy(a: &Artifacts) -> NormalizationPolicy {
    let mut p = policy();
    p.source_presentation = Some(SourcePresentationPolicy::PobFreshPresentationV1 {
        mapping_source: *a.mapping.source_identity(),
        empty_socket_urls: true,
        calcs_sections: true,
        tree_view: true,
        empty_notes: true,
    });
    p
}

fn url_policy(a: &Artifacts) -> NormalizationPolicy {
    let mut p = enabled_policy(a);
    let SourcePresentationPolicy::PobFreshPresentationV1 {
        calcs_sections,
        tree_view,
        empty_notes,
        ..
    } = p.source_presentation.as_mut().unwrap();
    *calcs_sections = false;
    *tree_view = false;
    *empty_notes = false;
    p
}

fn normalize(
    xml: &str,
    a: &Artifacts,
    p: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let source = source(xml, 117);
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let result = normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&a.schema),
            item_source: &empty_item_source(&a.schema),
            mappings: &a.mapping,
            registry: &a.registry,
            definitions: &a.schema,
            roles: &a.roles,
            rewards: &a.rewards,
        },
        p,
        &queries(),
        limits,
    )?;
    origin_integrity(&source, &result);
    Ok(result)
}

fn presentation_count(result: &NormalizedImport, code: &str) -> usize {
    result.sidecar().origins.iter().filter(|row| {
        matches!(&row.disposition, SourceDisposition::SourceOnly(found) if found.as_str() == code)
    }).count()
}

fn xml(children: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="73"/><Items activeItemSet="1"><ItemSet id="1" useSecondWeaponSet="false">{children}</ItemSet></Items><Config activeConfigSet="1"><ConfigSet id="1"/></Config></PathOfBuilding2>"#
    )
}

fn assert_only_url_origins_changed(old: &NormalizedImport, next: &NormalizedImport) {
    assert_eq!(old.draft().input(), next.draft().input());
    assert_eq!(old.allocator_after(), next.allocator_after());
    let mut expected = old.sidecar().origins.clone();
    for (old, new) in expected.iter_mut().zip(&next.sidecar().origins) {
        if matches!(&new.disposition, SourceDisposition::SourceOnly(code) if code.as_str() == "empty-socket-trade-url")
        {
            assert!(
                old.links
                    .iter()
                    .all(|link| matches!(link, OwnedOriginTarget::Issue(_)))
            );
            old.links.clear();
            old.disposition = new.disposition.clone();
        }
    }
    assert_eq!(expected, next.sidecar().origins);
}

#[test]
fn reviewed_empty_url_preserves_draft_ids_and_every_unresolved_inventory() {
    let a = artifacts(false);
    let xml = xml(EMPTY_URL);
    let old = normalize(&xml, &a, &policy(), Default::default()).unwrap();
    let next = normalize(&xml, &a, &url_policy(&a), Default::default()).unwrap();
    assert_eq!(presentation_count(&next, "empty-socket-trade-url"), 1);
    assert_only_url_origins_changed(&old, &next);
    assert!(matches!(
        next.draft().input().choice_presets.members[0]
            .choices
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        next.draft().input().scenario_presets.members[0]
            .scenario
            .usage
            .completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn all_five_originals_use_the_same_empty_url_grammar_without_game_definitions() {
    let a = artifacts(false);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    for (case, count) in [3, 18, 3, 6, 6].into_iter().enumerate() {
        let xml = std::fs::read_to_string(root.join(format!("build-{:02}.xml", case + 1))).unwrap();
        let old = normalize(&xml, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&xml, &a, &url_policy(&a), Default::default()).unwrap();
        assert_eq!(
            presentation_count(&next, "empty-socket-trade-url"),
            count,
            "original {}",
            case + 1
        );
        assert_only_url_origins_changed(&old, &next);
        let source = source(&xml, 117);
        for origin in &next.sidecar().origins {
            if matches!(&origin.disposition, SourceDisposition::SourceOnly(code) if code.as_str() == "empty-socket-trade-url")
            {
                assert_eq!(
                    source.occurrence(origin.source).unwrap().name(),
                    "SocketIdURL"
                );
            }
        }
    }
}

#[test]
fn malformed_ambiguous_or_nonempty_urls_keep_their_fallback_obligations() {
    let a = artifacts(false);
    let p = url_policy(&a);
    for row in [
        r#"<SocketIdURL nodeId="123"/>"#.to_owned(),
        r#"<SocketIdURL nodeId="123" itemPbURL="https://example.invalid/trade"/>"#.into(),
        r#"<SocketIdURL nodeId="123" itemPbURL=" "/>"#.into(),
        r#"<SocketIdURL nodeId="123" itemPbURL="" itemId="1"/>"#.into(),
        r#"<SocketIdURL nodeId="123" itemPbURL=""><Nested/></SocketIdURL>"#.into(),
        r#"<SocketIdURL nodeId="123" itemPbURL="">text</SocketIdURL>"#.into(),
        r#"<SocketIdURL xmlns="urn:other" nodeId="123" itemPbURL=""/>"#.into(),
        format!("{EMPTY_URL}{EMPTY_URL}"),
        format!("{EMPTY_URL}<SocketIdURL nodeId=\"0123\" itemPbURL=\"\"/>"),
    ] {
        let input = xml(&row);
        let old = normalize(&input, &a, &policy(), Default::default()).unwrap();
        let next = normalize(&input, &a, &p, Default::default()).unwrap();
        assert_eq!(
            presentation_count(&next, "empty-socket-trade-url"),
            0,
            "{row}"
        );
        assert_eq!(old.sidecar().origins, next.sidecar().origins);
        assert_eq!(old.draft().input(), next.draft().input());
    }
    // Duplicate XML attributes fail at the source decoder, before a conservative
    // normalization draft can exist. They must not be accepted as a last value.
    assert!(
        decode_build(xml(r#"<SocketIdURL nodeId="123" nodeId="124" itemPbURL=""/>"#).as_bytes())
            .is_err()
    );
    for key in ["", "0", "01", "+1", "1.0", "1e0", "-1", "NaN", "4294967296"] {
        let input = xml(&format!(r#"<SocketIdURL nodeId="{key}" itemPbURL=""/>"#));
        assert_eq!(
            presentation_count(
                &normalize(&input, &a, &p, Default::default()).unwrap(),
                "empty-socket-trade-url"
            ),
            0
        );
    }
}

#[test]
fn source_parent_scope_and_independent_sets_are_checked() {
    let a = artifacts(false);
    let p = url_policy(&a);
    let base = xml(EMPTY_URL);
    for input in [
        base.replace("<Items ", "<Items xmlns=\"urn:other\" "),
        base.replace("<ItemSet id=\"1\"", "<ItemSet unexpected=\"x\" id=\"1\""),
        base.replace("<ItemSet id=\"1\"", "<ItemSet id=\"01\""),
        base.replace("</Items>", "<ItemSet id=\"1\"/></Items>"),
        base.replace("</PathOfBuilding2>", "<Items/></PathOfBuilding2>"),
        xml(&format!("<Wrapper>{EMPTY_URL}</Wrapper>")),
        base.replace(EMPTY_URL, "")
            .replace("</Items>", &format!("{EMPTY_URL}</Items>")),
    ] {
        assert_eq!(
            presentation_count(
                &normalize(&input, &a, &p, Default::default()).unwrap(),
                "empty-socket-trade-url"
            ),
            0,
            "{input}"
        );
    }
    let independent = base.replace(
        "</Items>",
        &format!(r#"<ItemSet id="2">{EMPTY_URL}</ItemSet></Items>"#),
    );
    assert_eq!(
        presentation_count(
            &normalize(&independent, &a, &p, Default::default()).unwrap(),
            "empty-socket-trade-url"
        ),
        2
    );
    let mixed = base.replace(
        "</Items>",
        r#"<ItemSet id="2"><SocketIdURL nodeId="123" itemPbURL="nonempty"/></ItemSet></Items>"#,
    );
    assert_eq!(
        presentation_count(
            &normalize(&mixed, &a, &p, Default::default()).unwrap(),
            "empty-socket-trade-url"
        ),
        1
    );
}

#[test]
fn optional_policy_and_disabled_switch_preserve_behavior_and_authenticate_source() {
    let a = artifacts(false);
    let input = xml(EMPTY_URL);
    let wire = serde_json::to_value(policy()).unwrap();
    assert!(wire.get("source_presentation").is_none());
    let mut with_null = wire.clone();
    with_null["source_presentation"] = serde_json::Value::Null;
    let restored: NormalizationPolicy = serde_json::from_value(with_null).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    let old = normalize(&input, &a, &policy(), Default::default()).unwrap();
    let same = normalize(&input, &a, &restored, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_vec(old.sidecar()).unwrap(),
        serde_json::to_vec(same.sidecar()).unwrap()
    );
    let mut p = url_policy(&a);
    let SourcePresentationPolicy::PobFreshPresentationV1 {
        empty_socket_urls, ..
    } = p.source_presentation.as_mut().unwrap();
    *empty_socket_urls = false;
    let disabled = normalize(&input, &a, &p, Default::default()).unwrap();
    assert_eq!(disabled.sidecar().origins, old.sidecar().origins);
    assert_eq!(disabled.draft().input(), old.draft().input());
    let SourcePresentationPolicy::PobFreshPresentationV1 { mapping_source, .. } =
        p.source_presentation.as_mut().unwrap();
    *mapping_source = "f".repeat(64).parse().unwrap();
    assert!(matches!(
        normalize(&input, &a, &p, Default::default()),
        Err(NormalizationError::Binding)
    ));
}

#[test]
fn presentation_work_is_charged_and_failed_runs_do_not_contaminate_replay() {
    let a = artifacts(false);
    let input = xml(EMPTY_URL);
    let p = url_policy(&a);
    let expected = normalize(&input, &a, &p, Default::default()).unwrap();
    // Find the predecessor's exact minimum work budget, then require the new
    // source census to account for its additional work instead of borrowing it.
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: mid,
            ..Default::default()
        };
        match normalize(&input, &a, &policy(), limits) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_)) => low = mid + 1,
            Err(error) => panic!("unexpected baseline error: {error}"),
        }
    }
    assert!(matches!(
        normalize(
            &input,
            &a,
            &p,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let alternate = xml(r#"<SocketIdURL nodeId="123" itemPbURL="nonempty"/>"#);
    assert_eq!(
        presentation_count(
            &normalize(&alternate, &a, &p, Default::default()).unwrap(),
            "empty-socket-trade-url"
        ),
        0
    );
    let replay = normalize(&input, &a, &p, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_vec(expected.sidecar()).unwrap(),
        serde_json::to_vec(replay.sidecar()).unwrap()
    );
}
