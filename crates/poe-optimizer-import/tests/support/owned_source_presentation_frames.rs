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
    let xml = wrap(&format!("<Calcs>{SECTION}</Calcs>{TREE_VIEW}<Notes/>"));
    for enabled in 0..3 {
        let mut policy = enabled_policy(&a);
        let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
            calcs_sections,
            tree_view,
            empty_notes,
            ..
        }) = &mut policy.source_presentation
        else {
            panic!("source presentation policy missing")
        };
        *calcs_sections = enabled == 0;
        *tree_view = enabled == 1;
        *empty_notes = enabled == 2;
        let result = normalize(&xml, &a, &policy, Default::default()).unwrap();
        for (index, code) in [
            "source-calculation-section-layout",
            "source-tree-view-layout",
            "source-empty-notes",
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
