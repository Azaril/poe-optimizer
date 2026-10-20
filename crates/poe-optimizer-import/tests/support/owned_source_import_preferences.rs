//! Import-tab state has its own source-only authority; it closes no inventory.
use super::*;

const CODE: &str = "source-import-preferences";
const CONFIG: &str = r#"<Config activeConfigSet="1"><ConfigSet id="1"/></Config>"#;

fn wrap(body: &str) -> String {
    format!("<PathOfBuilding2>{CONFIG}{body}</PathOfBuilding2>")
}

fn import_policy(a: &Artifacts, enabled: bool) -> NormalizationPolicy {
    let mut p = enabled_policy(a);
    let SourcePresentationPolicy::PobFreshPresentationV1 {
        empty_socket_urls,
        calcs_sections,
        tree_view,
        empty_notes,
        cached_build_buffs,
        empty_party,
        trade_search_weights,
        import_preferences,
        ..
    } = p.source_presentation.as_mut().unwrap();
    *empty_socket_urls = false;
    *calcs_sections = false;
    *tree_view = false;
    *empty_notes = false;
    *cached_build_buffs = false;
    *empty_party = false;
    *trade_search_weights = false;
    *import_preferences = enabled;
    p
}

fn assert_only_import_changed(old: &NormalizedImport, next: &NormalizedImport, count: usize) {
    // Real builds may select a newer profile through another enabled adapter.
    // Toggling this family must preserve whichever profile the input selected.
    let version = old.sidecar().schema_version;
    assert_import_policy_delta(old, next, count, (version, version));
}

fn assert_import_policy_delta(
    old: &NormalizedImport,
    next: &NormalizedImport,
    count: usize,
    schema_versions: (u32, u32),
) {
    assert_eq!(old.draft().input(), next.draft().input());
    assert_eq!(old.allocator_after(), next.allocator_after());
    let mut expected = old.sidecar().origins.clone();
    let mut changed = 0;
    for (before, after) in expected.iter_mut().zip(&next.sidecar().origins) {
        if before == after {
            continue;
        }
        changed += 1;
        assert_eq!(before.source, after.source);
        assert!(matches!(before.disposition, SourceDisposition::Contributes));
        assert!(!before.links.is_empty());
        assert!(
            before
                .links
                .iter()
                .all(|l| matches!(l, OwnedOriginTarget::Issue(_)))
        );
        assert_eq!(after.disposition, SourceDisposition::SourceOnly(key(CODE)));
        assert!(after.links.is_empty());
        before.links.clear();
        before.disposition = after.disposition.clone();
    }
    assert_eq!(changed, count);
    assert_eq!(expected, next.sidecar().origins);
    // Enabling the presentation policy, even with every family disabled, selects
    // its existing sidecar profile. Assert that exact transition rather than
    // ignoring schema changes for all comparisons.
    assert_eq!(old.sidecar().schema_version, schema_versions.0);
    assert_eq!(next.sidecar().schema_version, schema_versions.1);
    // Preserve every other field beyond the profile, policy commitment and
    // exactly the proven source-origin dispositions/links above.
    let before = serde_json::to_value(old.sidecar()).unwrap();
    let mut after = serde_json::to_value(next.sidecar()).unwrap();
    after["policy"] = before["policy"].clone();
    after["origins"] = before["origins"].clone();
    after["schema_version"] = before["schema_version"].clone();
    assert_eq!(before, after);
}

#[test]
fn exact_import_frames_reuse_root_admission_law_and_preserve_mixed_responsibilities() {
    let a = artifacts(false);
    for body in [
        "<Import/>",
        "<Import> \n\t </Import>",
        "<Import exportParty='false'/>",
        "<Import useGeneratedItemText='false'/>",
        "<Import useGeneratedItemText='true' exportParty='false'/>",
        r#"<Import lastRealm="PoE2" lastLeague="A &amp; B" lastAccountHash="saved-hash" lastCharacterHash="saved-character" importLink="https://invalid.example/persist-only" exportParty="false" useGeneratedItemText="true"/>"#,
    ] {
        let source = wrap(body);
        let document = roxmltree::Document::parse(&source).unwrap_or_else(|e| panic!("{e}"));
        // Keep the source document alive for the admission's borrowed projection.
        assert!(poe_optimizer_import::root_admission::validate_main(&document).is_ok());
        let input = wrap(&format!(
            "{body}<Calcs><Input name='skill_number' number='2'/><Input name='misc_buffMode' string='EFFECTIVE'/></Calcs><Unknown retained='true'/>"
        ));
        let old = normalize(&input, &a, &import_policy(&a, false), Default::default()).unwrap();
        let next = normalize(&input, &a, &import_policy(&a, true), Default::default()).unwrap();
        assert_eq!(presentation_count(&next, CODE), 1, "{body}");
        assert_only_import_changed(&old, &next, 1);
    }
}

#[test]
fn effectful_unknown_or_ambiguous_import_frames_keep_every_origin_obligation() {
    let a = artifacts(false);
    for body in [
        "<Import exportParty='true'/>",
        "<Import exportParty='TRUE'/>",
        "<Import exportParty='nil'/>",
        "<Import exportParty=''/>",
        "<Import useGeneratedItemText='1'/>",
        "<Import callback='fetch'/>",
        "<Import exportParty='false' future='x'/>",
        "<Import><Future/></Import>",
        "<Import><Input name='count' number='1'/></Import>",
        "<Import>authored</Import>",
        "<Import><!-- unknown --></Import>",
        "<Import><![CDATA[]]></Import>",
        "<Import xmlns='foreign'/>",
        "<Import xmlns:x='foreign'/>",
        "<Import xmlns:x='foreign' x:exportParty='false'/>",
        "<Import/><Import/>",
        "<Import exportParty='false'/><Import exportParty='true'/>",
        "<Items><Import exportParty='false'/></Items>",
    ] {
        let xml = wrap(body);
        let old = normalize(&xml, &a, &import_policy(&a, false), Default::default()).unwrap();
        let next = normalize(&xml, &a, &import_policy(&a, true), Default::default()).unwrap();
        assert_eq!(presentation_count(&next, CODE), 0, "{body}");
        assert_only_import_changed(&old, &next, 0);
    }
    assert!(
        decode_build(wrap("<Import exportParty='false' exportParty='true'/>").as_bytes()).is_err()
    );
    let input = wrap("<Import/>").replace("<PathOfBuilding2>", "<PathOfBuilding2 future='x'>");
    let old = normalize(&input, &a, &import_policy(&a, false), Default::default()).unwrap();
    let next = normalize(&input, &a, &import_policy(&a, true), Default::default()).unwrap();
    assert_only_import_changed(&old, &next, 0);
}

#[test]
fn import_family_is_independent_and_absent_flag_grants_no_authority() {
    let a = artifacts(false);
    let disabled = import_policy(&a, false);
    let wire = serde_json::to_value(disabled.source_presentation.as_ref().unwrap()).unwrap();
    assert!(wire.get("import_preferences").is_none());
    let restored: SourcePresentationPolicy = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), wire);
    let input = wrap("<Import exportParty='false'/><Party destination='All'/><Notes/>");
    let old = normalize(&input, &a, &policy(), Default::default()).unwrap();
    let all_disabled = normalize(&input, &a, &disabled, Default::default()).unwrap();
    assert_import_policy_delta(&old, &all_disabled, 0, (12, 18));
    let only_import = normalize(&input, &a, &import_policy(&a, true), Default::default()).unwrap();
    assert_only_import_changed(&all_disabled, &only_import, 1);
    assert_eq!(presentation_count(&only_import, "source-empty-party"), 0);
    let mut only_party = disabled.clone();
    let SourcePresentationPolicy::PobFreshPresentationV1 { empty_party, .. } =
        only_party.source_presentation.as_mut().unwrap();
    *empty_party = true;
    let party = normalize(&input, &a, &only_party, Default::default()).unwrap();
    assert_eq!(presentation_count(&party, CODE), 0);
    assert_eq!(presentation_count(&party, "source-empty-party"), 1);
    let mut invalid = disabled;
    let SourcePresentationPolicy::PobFreshPresentationV1 { mapping_source, .. } =
        invalid.source_presentation.as_mut().unwrap();
    *mapping_source = "f".repeat(64).parse().unwrap();
    assert!(matches!(
        normalize(&input, &a, &invalid, Default::default()),
        Err(NormalizationError::Binding)
    ));
}

#[test]
fn all_five_unchanged_sources_only_reclassify_their_exact_import_origin() {
    let a = artifacts(false);
    for xml in [
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-03.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-04.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
    ] {
        let old = normalize(xml, &a, &import_policy(&a, false), Default::default()).unwrap();
        let next = normalize(xml, &a, &import_policy(&a, true), Default::default()).unwrap();
        assert_only_import_changed(&old, &next, 1);
        assert_eq!(presentation_count(&old, CODE), 0);
        assert_eq!(presentation_count(&next, CODE), 1);
        let doc = roxmltree::Document::parse(xml).unwrap();
        let import = doc
            .root_element()
            .children()
            .filter(|n| n.has_tag_name("Import"))
            .collect::<Vec<_>>();
        assert_eq!(import.len(), 1);
        let ordinal = doc
            .descendants()
            .filter(|n| n.is_element())
            .position(|n| n == import[0])
            .unwrap();
        assert_eq!(
            next.sidecar().origins[ordinal].disposition,
            SourceDisposition::SourceOnly(key(CODE))
        );
        let repeated = normalize(xml, &a, &import_policy(&a, true), Default::default()).unwrap();
        assert_eq!(next.draft().input(), repeated.draft().input());
        assert_eq!(
            serde_json::to_value(next.sidecar()).unwrap(),
            serde_json::to_value(repeated.sidecar()).unwrap()
        );
    }
}

#[test]
fn import_inspection_is_charged_and_failure_preserves_fresh_replay() {
    let a = artifacts(false);
    let xml = wrap(&format!(
        "<Import exportParty='false' lastCharacterHash='{}'/>",
        "x".repeat(2000)
    ));
    let baseline = import_policy(&a, false);
    let enabled = import_policy(&a, true);
    let expected = normalize(&xml, &a, &enabled, Default::default()).unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        match normalize(
            &xml,
            &a,
            &baseline,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        ) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_)) => low = mid + 1,
            Err(e) => panic!("unexpected baseline: {e}"),
        }
    }
    assert!(matches!(
        normalize(
            &xml,
            &a,
            &enabled,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let fresh = normalize(&xml, &a, &enabled, Default::default()).unwrap();
    assert_eq!(expected.draft().input(), fresh.draft().input());
    assert_eq!(
        serde_json::to_value(expected.sidecar()).unwrap(),
        serde_json::to_value(fresh.sidecar()).unwrap()
    );
}
