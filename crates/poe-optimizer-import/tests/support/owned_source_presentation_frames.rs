//! Source-only layout dispositions never imply semantic input completeness.
use super::*;

const SECTION: &str =
    r#"<Section id="caller-section" subsection="caller-subsection" collapsed="true"/>"#;
const TREE_VIEW: &str = r#"<TreeView zoomLevel="9" zoomX="-288.5" zoomY="729.3" searchStr="caller text" showStatDifferences="true"/>"#;
const CONFIG: &str = r#"<Config activeConfigSet="1"><ConfigSet id="1"/></Config>"#;

fn wrap(body: &str) -> String {
    format!("<PathOfBuilding2>{CONFIG}{body}</PathOfBuilding2>")
}

fn count(body: &str, code: &str) -> usize {
    let a = artifacts(false);
    let result = normalize(&wrap(body), &a, &enabled_policy(&a), Default::default()).unwrap();
    presentation_count(&result, code)
}

#[test]
fn presentation_frames_preserve_calculation_inputs_draft_issues_and_allocators() {
    let a = artifacts(false);
    let body = format!(
        r#"<Calcs><Input name="skill_number" number="3"/><Input name="misc_buffMode" string="EFFECTIVE"/>{SECTION}</Calcs>{TREE_VIEW}<Notes>
        </Notes><Party destination="All"/><Import exportParty="false"/><TradeSearchWeights/>"#
    );
    let xml = wrap(&body);
    let policy = enabled_policy(&a);
    let mut old_policy = policy.clone();
    old_policy.source_presentation = None;
    let old = normalize(&xml, &a, &old_policy, Default::default()).unwrap();
    let result = normalize(&xml, &a, &policy, Default::default()).unwrap();
    assert_eq!(result.draft().input(), old.draft().input());
    assert_eq!(result.allocator_after(), old.allocator_after());
    for code in [
        "source-calculation-section-layout",
        "source-tree-view-layout",
        "source-empty-notes",
    ] {
        assert_eq!(presentation_count(&result, code), 1, "{code}");
    }
    let changed: Vec<_> = result
        .sidecar()
        .origins
        .iter()
        .zip(&old.sidecar().origins)
        .filter(|(current, old)| current != old)
        .collect();
    assert_eq!(changed.len(), 3);
    assert!(changed.iter().all(|(current, old)| {
        current.source == old.source
            && current.links.is_empty()
            && !old.links.is_empty()
            && matches!(current.disposition, SourceDisposition::SourceOnly(_))
    }));
}

#[test]
fn section_proof_rejects_unknown_shapes_and_never_admits_nested_layout() {
    let code = "source-calculation-section-layout";
    assert_eq!(count(&format!("<Calcs>{SECTION}</Calcs>"), code), 1);
    assert_eq!(
        count(
            &format!("<Calcs>{}</Calcs>", SECTION.replace("true", "false")),
            code
        ),
        1
    );
    for body in [
        format!("<Calcs future='x'>{SECTION}</Calcs>"),
        format!("<Calcs>{SECTION}<Future/></Calcs>"),
        format!("<Calcs>{SECTION}{SECTION}</Calcs>"),
        format!("<Calcs>{SECTION}</Calcs><Calcs/>"),
        format!("<Party><Calcs>{SECTION}</Calcs></Party>"),
        format!("<Calcs xmlns='urn:future'>{SECTION}</Calcs>"),
        format!("<Calcs>{}</Calcs>", SECTION.replace("/>", " future='x'/>")),
        format!("<Calcs>{}</Calcs>", SECTION.replace("true", "nil")),
        format!("<Calcs>{}</Calcs>", SECTION.replace("caller-section", "")),
        format!(
            "<Calcs>{}</Calcs>",
            SECTION.replace("/>", "><Future/></Section>")
        ),
        format!(
            "<Calcs>{}</Calcs>",
            SECTION.replace("/>", ">authored</Section>")
        ),
    ] {
        assert_eq!(count(&body, code), 0, "{body}");
    }
}

#[test]
fn tree_view_requires_complete_finite_known_leaf_and_empty_notes_stays_narrow() {
    let code = "source-tree-view-layout";
    assert_eq!(count(TREE_VIEW, code), 1);
    for body in [
        "<TreeView/>".to_owned(),
        TREE_VIEW.replace("zoomLevel=\"9\"", "zoomLevel=\"21\""),
        TREE_VIEW.replace("zoomLevel=\"9\"", "zoomLevel=\"1.5\""),
        TREE_VIEW.replace("zoomX=\"-288.5\"", "zoomX=\"NaN\""),
        TREE_VIEW.replace("zoomY=\"729.3\"", "zoomY=\"inf\""),
        TREE_VIEW.replace(
            "showStatDifferences=\"true\"",
            "showStatDifferences=\"nil\"",
        ),
        TREE_VIEW.replace(" searchStr=\"caller text\"", ""),
        TREE_VIEW.replace("/>", " future='x'/>"),
        TREE_VIEW.replace("/>", "><Future/></TreeView>"),
        TREE_VIEW.replace("/>", ">authored</TreeView>"),
        TREE_VIEW.replace("<TreeView ", "<TreeView xmlns='urn:future' "),
        format!("{TREE_VIEW}{TREE_VIEW}"),
        format!("<Items>{TREE_VIEW}</Items>"),
    ] {
        assert_eq!(count(&body, code), 0, "{body}");
    }
    for body in ["<Notes/>", "<Notes> \n\t </Notes>"] {
        assert_eq!(count(body, "source-empty-notes"), 1, "{body}");
    }
    for body in [
        "<Notes>use another skill</Notes>",
        "<Notes future='x'/>",
        "<Notes><Future/></Notes>",
        "<Notes xmlns='urn:future'/>",
        "<Notes/><Notes/>",
        "<Party><Notes/></Party>",
    ] {
        assert_eq!(count(body, "source-empty-notes"), 0, "{body}");
    }
}

#[test]
fn presentation_family_switches_are_independent() {
    let a = artifacts(false);
    let xml = wrap(&format!(
        "<Calcs>{SECTION}</Calcs>{TREE_VIEW}<Notes/><Build><Buffs/></Build>"
    ));
    for enabled in 0..4 {
        let mut policy = enabled_policy(&a);
        let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
            calcs_sections,
            tree_view,
            empty_notes,
            cached_build_buffs,
            ..
        }) = &mut policy.source_presentation
        else {
            panic!("source presentation policy missing")
        };
        *calcs_sections = enabled == 0;
        *tree_view = enabled == 1;
        *empty_notes = enabled == 2;
        *cached_build_buffs = enabled == 3;
        let result = normalize(&xml, &a, &policy, Default::default()).unwrap();
        for (index, code) in [
            "source-calculation-section-layout",
            "source-tree-view-layout",
            "source-empty-notes",
            "source-cached-build-buffs",
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(
                presentation_count(&result, code),
                usize::from(index == enabled)
            );
        }
    }
}

const CACHED_BUFFS: &str = "source-cached-build-buffs";

#[test]
fn cached_buffs_are_only_a_strict_build_child_and_never_a_semantic_container() {
    for leaf in [
        "<Buffs/>",
        r#"<Buffs buffList="" combatList="" curseList="Frost Bomb"/>"#,
        r#"<Buffs buffList="Clarity I, Rage" curseList="a &amp; b"/>"#,
    ] {
        assert_eq!(
            count(&format!("<Build level='73'>{leaf}</Build>"), CACHED_BUFFS),
            1
        );
    }
    for body in [
        "<Buffs/>",
        "<Party><Buffs/></Party>",
        "<Party><Build><Buffs/></Build></Party>",
        "<Build/><Build><Buffs/></Build>",
        "<Build><Buffs/><Buffs/></Build>",
        "<Build><Buffs future='x'/></Build>",
        "<Build future='x'><Buffs/></Build>",
        "<Build><Buffs><Input name='buff' boolean='true'/></Buffs></Build>",
        "<Build><Buffs>authored meaning</Buffs></Build>",
        "<Build><Buffs><!-- ambiguous content --></Buffs></Build>",
        "<Build xmlns='urn:future'><Buffs/></Build>",
        "<Build><Buffs xmlns='urn:future'/></Build>",
    ] {
        assert_eq!(count(body, CACHED_BUFFS), 0, "{body}");
    }
}

fn assert_cached_buffs_delta(xml: &str) {
    let a = artifacts(false);
    let policy = enabled_policy(&a);
    let mut before_policy = policy.clone();
    let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
        cached_build_buffs, ..
    }) = &mut before_policy.source_presentation
    else {
        unreachable!()
    };
    *cached_build_buffs = false;
    let before = normalize(xml, &a, &before_policy, Default::default()).unwrap();
    let after = normalize(xml, &a, &policy, Default::default()).unwrap();
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(presentation_count(&before, CACHED_BUFFS), 0);
    assert_eq!(presentation_count(&after, CACHED_BUFFS), 1);
    let changed: Vec<_> = before
        .sidecar()
        .origins
        .iter()
        .zip(&after.sidecar().origins)
        .filter(|(old, new)| old != new)
        .collect();
    assert_eq!(changed.len(), 1);
    let (old, new) = changed[0];
    assert_eq!(old.source, new.source);
    assert!(matches!(old.disposition, SourceDisposition::Contributes));
    assert!(!old.links.is_empty());
    assert!(
        old.links
            .iter()
            .all(|link| matches!(link, OwnedOriginTarget::Issue(_)))
    );
    assert!(new.links.is_empty());
    assert!(
        matches!(&new.disposition, SourceDisposition::SourceOnly(code) if code.as_str() == CACHED_BUFFS)
    );
}

#[test]
fn cached_buffs_change_only_one_origin_in_each_unchanged_original() {
    for xml in [
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-03.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-04.xml"),
        include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
    ] {
        assert_cached_buffs_delta(xml);
    }
}

#[test]
fn omitted_cached_buffs_switch_authorizes_nothing_and_preserves_policy_wire() {
    let a = artifacts(false);
    let mut policy = enabled_policy(&a);
    let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
        cached_build_buffs, ..
    }) = &mut policy.source_presentation
    else {
        unreachable!()
    };
    *cached_build_buffs = false;
    let wire = serde_json::to_value(policy.source_presentation.as_ref().unwrap()).unwrap();
    assert!(wire.get("cached_build_buffs").is_none());
    let decoded: SourcePresentationPolicy = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), wire);
    assert!(matches!(
        decoded,
        SourcePresentationPolicy::PobFreshPresentationV1 {
            cached_build_buffs: false,
            ..
        }
    ));
    let result = normalize(
        &wrap("<Build><Buffs/></Build>"),
        &a,
        &policy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(presentation_count(&result, CACHED_BUFFS), 0);
}
