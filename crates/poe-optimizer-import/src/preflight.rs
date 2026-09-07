//! Evaluator-only checks that keep incomplete XML from becoming a fresh default build.
//!
//! This is not a build-legality validator. Optional sections may use PoB defaults;
//! unknown fields remain untouched. Container decoding remains a separate contract.

use std::collections::HashSet;

use roxmltree::{Document, Node, ParsingOptions};
use thiserror::Error;

use crate::{MAX_XML_BYTES, MAX_XML_NODES};

/// Pinned src/GameVersions.lua:6; this is the export format, not the game patch.
pub const SUPPORTED_TARGET_VERSION: &str = "0_1";
const TREE_VERSIONS: &[&str] = &["0_1", "0_2", "0_3", "0_4", "0_5"];
// Build.lua:546-557 savers/legacyLoaders, plus the separately loaded Build section.
const SINGLETON_SECTIONS: &[&str] = &[
    "Build", "Config", "Notes", "Party", "Tree", "TreeView", "Items", "Skills", "Calcs", "Import",
    "Spec",
];

#[derive(Debug, Error)]
pub enum PreflightError {
    #[error("build XML exceeds the {MAX_XML_BYTES}-byte limit")]
    TooLarge,
    #[error("invalid build XML: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("evaluation requires an unnamespaced PathOfBuilding2 root")]
    WrongRoot,
    #[error(transparent)]
    IncompatibleSyntax(#[from] crate::xml_compat::XmlCompatibilityError),
    #[error("required {section} section is missing")]
    MissingSection { section: &'static str },
    #[error("duplicate singleton {section} section is ambiguous")]
    DuplicateSection { section: String },
    #[error("namespaced {section} section cannot stand in for a PoB section")]
    NamespacedSection { section: String },
    #[error("{element} requires an explicit {attribute} attribute")]
    MissingAttribute {
        element: &'static str,
        attribute: &'static str,
    },
    #[error("invalid {element}.{attribute} value {value:?}: expected {expected}")]
    InvalidAttribute {
        element: &'static str,
        attribute: &'static str,
        value: String,
        expected: &'static str,
    },
    #[error(
        "unsupported targetVersion {actual:?}; expected {SUPPORTED_TARGET_VERSION}; explicit conversion is required"
    )]
    UnsupportedTargetVersion { actual: String },
    #[error("unsupported treeVersion {actual:?} for the pinned evaluator")]
    UnsupportedTreeVersion { actual: String },
    #[error(
        "Tree must contain at least one explicit Spec; evaluation will not synthesize a default tree"
    )]
    EmptyTree,
    #[error(
        "legacy top-level or URL-only Spec requires conversion to an explicit Tree/Spec with nodes and class identity"
    )]
    LegacyTree,
    #[error("Spec requires a classId or classInternalId")]
    MissingClass,
    #[error(
        "Spec requires an ascendClassId or ascendancyInternalId (empty internal ID means no ascendancy)"
    )]
    MissingAscendancy,
    #[error("Build.mainSkillIndex and Build.mainSocketGroup disagree")]
    ConflictingMainSkill,
}

/// Check the current exported-build structure without changing any imported value.
///
/// Build.LoadDB accepts a missing Build and selects only the first duplicate
/// (Build.lua:2691-2711). Init then calculates a default build. Likewise, TreeTab
/// synthesizes a tree for an empty Tree (TreeTab.lua:515-518), and PassiveSpec's
/// constructor selects a default class (PassiveSpec.lua:41). Require the explicit
/// metadata needed to avoid those cases before calling the Lua loader.
///
/// Empty node lists and omitted optional Skills/Items/Config sections are valid.
/// Class/ascendancy existence, skill resolution, allocation legality and metric
/// coverage still require the pinned game adapter; this function does not certify them.
pub fn validate(xml: &str) -> Result<(), PreflightError> {
    if xml.len() > MAX_XML_BYTES {
        return Err(PreflightError::TooLarge);
    }
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_XML_NODES,
            entity_resolver: None,
        },
    )?;
    let root = document.root_element();
    if root.tag_name().name() != "PathOfBuilding2" || root.tag_name().namespace().is_some() {
        return Err(PreflightError::WrongRoot);
    }
    let mut seen = HashSet::new();
    let mut build = None;
    let mut tree = None;
    let mut skills = None;
    let mut config = None;
    let mut legacy_tree = false;
    for node in root.children().filter(Node::is_element) {
        let name = node.tag_name().name();
        if !SINGLETON_SECTIONS.contains(&name) {
            continue;
        }
        require_unnamespaced(node)?;
        if !seen.insert(name) {
            return Err(PreflightError::DuplicateSection {
                section: name.to_owned(),
            });
        }
        match name {
            "Build" => build = Some(node),
            "Tree" => tree = Some(node),
            "Skills" => skills = Some(node),
            "Config" => config = Some(node),
            "Spec" => legacy_tree = true,
            _ => {}
        }
    }
    let build = build.ok_or(PreflightError::MissingSection { section: "Build" })?;
    validate_build(build)?;
    if legacy_tree {
        return Err(PreflightError::LegacyTree);
    }
    let tree = tree.ok_or(PreflightError::MissingSection { section: "Tree" })?;
    let mut spec_count = 0;
    for node in tree.children().filter(Node::is_element) {
        if node.tag_name().name() == "Spec" {
            require_unnamespaced(node)?;
            validate_spec(node)?;
            spec_count += 1;
        }
    }
    if spec_count == 0 {
        return Err(PreflightError::EmptyTree);
    }
    // The documented default is the first spec. A supplied out-of-range value
    // is different: PoB clamps it, losing the requested selection.
    if let Some(active) = tree.attribute("activeSpec") {
        let selected = decimal(active)
            .ok_or_else(|| invalid("Tree", "activeSpec", active, "a positive spec index"))?;
        if selected == 0 || selected > spec_count {
            return Err(invalid(
                "Tree",
                "activeSpec",
                active,
                "an existing spec index",
            ));
        }
    }
    if let Some(skills) = skills {
        validate_skills(skills)?;
    }
    if let Some(config) = config {
        validate_config(config)?;
    }
    crate::xml_compat::validate(xml)?;
    Ok(())
}

// SkillsTab.Load accepts direct legacy Skill children or SkillSet/Skill children.
// Restrict traversal to those paths so unrelated extension nodes remain untouched.
fn validate_skills(skills: Node<'_, '_>) -> Result<(), PreflightError> {
    for node in skills.children().filter(Node::is_element) {
        if node.tag_name().namespace().is_some() {
            continue;
        }
        match node.tag_name().name() {
            "Skill" => validate_skill_selection(node)?,
            "SkillSet" => {
                for skill in node.children().filter(Node::is_element) {
                    if skill.tag_name().namespace().is_none() && skill.tag_name().name() == "Skill"
                    {
                        validate_skill_selection(skill)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_skill_selection(skill: Node<'_, '_>) -> Result<(), PreflightError> {
    // SkillsTab.Save:480-481 unconditionally applies tostring, so an unset field
    // is exported as literal "nil". Load:315-316 restores that to the same default
    // as an absent attribute. Preserve this documented sentinel, not arbitrary
    // malformed values that happen to make tonumber return nil as well.
    for attribute in ["mainActiveSkill", "mainActiveSkillCalcs"] {
        if let Some(value) = skill.attribute(attribute)
            && value != "nil"
            && decimal(value).is_none_or(|index| index == 0)
        {
            return Err(invalid(
                "Skill",
                attribute,
                value,
                "a positive active-skill index",
            ));
        }
    }
    Ok(())
}

// ConfigTab.Load consumes Input/Placeholder directly or one level inside ConfigSet.
// Lua tonumber accepts overflow as infinity and malformed text as nil; neither is
// an acceptable replacement for an explicitly supplied numeric scenario input.
fn validate_config(config: Node<'_, '_>) -> Result<(), PreflightError> {
    for node in config.children().filter(Node::is_element) {
        if node.tag_name().namespace().is_some() {
            continue;
        }
        if node.tag_name().name() == "ConfigSet" {
            for input in node.children().filter(Node::is_element) {
                if input.tag_name().namespace().is_none() {
                    validate_config_number(input)?;
                }
            }
        } else {
            validate_config_number(node)?;
        }
    }
    Ok(())
}

fn validate_config_number(node: Node<'_, '_>) -> Result<(), PreflightError> {
    let element = match node.tag_name().name() {
        "Input" => "Input",
        "Placeholder" => "Placeholder",
        _ => return Ok(()),
    };
    if let Some(value) = node.attribute("number")
        && !value.trim().parse::<f64>().is_ok_and(f64::is_finite)
    {
        return Err(invalid(element, "number", value, "a finite number"));
    }
    Ok(())
}

fn validate_build(build: Node<'_, '_>) -> Result<(), PreflightError> {
    let version = required(build, "Build", "targetVersion")?;
    if version != SUPPORTED_TARGET_VERSION {
        return Err(PreflightError::UnsupportedTargetVersion {
            actual: version.to_owned(),
        });
    }
    let level = required(build, "Build", "level")?;
    if !decimal(level).is_some_and(|level| (1..=100).contains(&level)) {
        return Err(invalid(
            "Build",
            "level",
            level,
            "an integer from 1 through 100",
        ));
    }
    for attribute in ["mainSocketGroup", "mainSkillIndex"] {
        if let Some(value) = build.attribute(attribute)
            && decimal(value).is_none_or(|index| index == 0)
        {
            return Err(invalid(
                "Build",
                attribute,
                value,
                "a positive skill-group index",
            ));
        }
    }
    if let (Some(old), Some(current)) = (
        build.attribute("mainSkillIndex"),
        build.attribute("mainSocketGroup"),
    ) && decimal(old) != decimal(current)
    {
        return Err(PreflightError::ConflictingMainSkill);
    }
    Ok(())
}

fn validate_spec(spec: Node<'_, '_>) -> Result<(), PreflightError> {
    let version = required(spec, "Spec", "treeVersion")?;
    if !TREE_VERSIONS.contains(&version) {
        return Err(PreflightError::UnsupportedTreeVersion {
            actual: version.to_owned(),
        });
    }
    // PassiveSpec.Load does no identity import without nodes or a legacy URL.
    // A URL-only path can return a decoding error without propagating it; require
    // the explicit node representation emitted by the current PoB exporter.
    let nodes = spec.attribute("nodes").ok_or(PreflightError::LegacyTree)?;
    if !nodes.is_empty() && !nodes.split(',').all(|id| decimal(id.trim()).is_some()) {
        return Err(invalid(
            "Spec",
            "nodes",
            nodes,
            "a comma-separated integer list, or an empty list",
        ));
    }
    let class = spec.attribute("classId");
    let internal_class = spec.attribute("classInternalId");
    if class.is_none() && internal_class.is_none() {
        return Err(PreflightError::MissingClass);
    }
    for (attribute, value) in [
        ("classId", class),
        ("classInternalId", internal_class),
        ("ascendClassId", spec.attribute("ascendClassId")),
    ] {
        if let Some(value) = value
            && decimal(value).is_none()
        {
            return Err(invalid(
                "Spec",
                attribute,
                value,
                "a nonnegative integer identity",
            ));
        }
    }
    if spec.attribute("ascendClassId").is_none() && spec.attribute("ascendancyInternalId").is_none()
    {
        return Err(PreflightError::MissingAscendancy);
    }
    Ok(())
}

fn require_unnamespaced(node: Node<'_, '_>) -> Result<(), PreflightError> {
    if node.tag_name().namespace().is_some() {
        return Err(PreflightError::NamespacedSection {
            section: node.tag_name().name().to_owned(),
        });
    }
    Ok(())
}

fn required<'a>(
    node: Node<'a, '_>,
    element: &'static str,
    attribute: &'static str,
) -> Result<&'a str, PreflightError> {
    node.attribute(attribute)
        .ok_or(PreflightError::MissingAttribute { element, attribute })
}

fn decimal(value: &str) -> Option<u32> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn invalid(
    element: &'static str,
    attribute: &'static str,
    value: &str,
    expected: &'static str,
) -> PreflightError {
    PreflightError::InvalidAttribute {
        element,
        attribute,
        value: value.to_owned(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD: &str = "<Build targetVersion='0_1' level='1'/>";
    const TREE: &str =
        "<Tree><Spec treeVersion='0_5' nodes='' classId='7' ascendClassId='0'/></Tree>";

    fn document(build: &str, tree: &str, other: &str) -> String {
        format!("<PathOfBuilding2>{build}{tree}{other}</PathOfBuilding2>")
    }

    #[test]
    fn accepts_supplied_fixture_and_explicit_empty_build_state() {
        let xml = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/builds/pobarchives-Dfz36mCq.xml"
        ));
        validate(xml).unwrap();
        validate(&document(BUILD, TREE, "")).unwrap();
        validate(&document(
            BUILD,
            TREE,
            "<Skills/><Items/><Config/><Future/><Future/>",
        ))
        .unwrap();
    }

    #[test]
    fn does_not_confuse_nested_or_namespaced_build_sections() {
        assert!(matches!(
            validate("<PathOfBuilding2/>"),
            Err(PreflightError::MissingSection { section: "Build" })
        ));
        assert!(matches!(
            validate(&document(
                "<Future><Build targetVersion='0_1' level='1'/></Future>",
                TREE,
                ""
            )),
            Err(PreflightError::MissingSection { section: "Build" })
        ));
        assert!(matches!(
            validate(&document(
                "<x:Build xmlns:x='urn:extension' targetVersion='0_1' level='1'/>",
                TREE,
                ""
            )),
            Err(PreflightError::NamespacedSection { .. })
        ));
        assert!(matches!(
            validate(&document(
                BUILD,
                TREE,
                "<x:Skills xmlns:x='urn:extension'/>"
            )),
            Err(PreflightError::NamespacedSection { .. })
        ));
    }

    #[test]
    fn rejects_duplicates_of_every_recognized_singleton() {
        for name in SINGLETON_SECTIONS {
            let extra = format!("<{name}/><{name}/>");
            assert!(
                matches!(
                    validate(&document(BUILD, TREE, &extra)),
                    Err(PreflightError::DuplicateSection { .. })
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn requires_explicit_supported_version_and_unclamped_level() {
        for build in [
            "<Build/>",
            "<Build level='1'/>",
            "<Build targetVersion='0_1'/>",
        ] {
            assert!(matches!(
                validate(&document(build, TREE, "")),
                Err(PreflightError::MissingAttribute { .. })
            ));
        }
        for version in ["", "0_0", "0_2", "0_5"] {
            let build = format!("<Build targetVersion='{version}' level='1'/>");
            assert!(matches!(
                validate(&document(&build, TREE, "")),
                Err(PreflightError::UnsupportedTargetVersion { .. })
            ));
        }
        for level in ["", "0", "101", "-1", "1.5", "NaN", "1e2", "4294967296"] {
            let build = format!("<Build targetVersion='0_1' level='{level}'/>");
            assert!(matches!(
                validate(&document(&build, TREE, "")),
                Err(PreflightError::InvalidAttribute {
                    attribute: "level",
                    ..
                })
            ));
        }
        validate(&document(
            "<Build targetVersion='0_1' level='100'/>",
            TREE,
            "",
        ))
        .unwrap();
    }

    #[test]
    fn requires_explicit_tree_and_spec_identity() {
        assert!(matches!(
            validate(&document(BUILD, "", "")),
            Err(PreflightError::MissingSection { section: "Tree" })
        ));
        assert!(matches!(
            validate(&document(BUILD, "<Tree/>", "")),
            Err(PreflightError::EmptyTree)
        ));
        for spec in [
            "<Spec/>",
            "<Spec treeVersion='0_5'/>",
            "<Spec treeVersion='0_5' nodes=''/>",
            "<Spec treeVersion='0_5' nodes='' classId='7'/>",
            "<Spec treeVersion='0_5'><URL>broken</URL></Spec>",
        ] {
            assert!(
                validate(&document(BUILD, &format!("<Tree>{spec}</Tree>"), "")).is_err(),
                "{spec}"
            );
        }
        assert!(matches!(
            validate(&document(
                BUILD,
                "<Spec treeVersion='0_5' nodes='' classId='7' ascendClassId='0'/>",
                ""
            )),
            Err(PreflightError::LegacyTree)
        ));
    }

    #[test]
    fn accepts_internal_ids_and_explicit_no_ascendancy() {
        let tree = "<Tree activeSpec='1'><Spec treeVersion='0_5' nodes='1, 2' classInternalId='7' ascendancyInternalId=''/></Tree>";
        validate(&document(BUILD, tree, "")).unwrap();
        let tree = TREE.replace("0_5", "0_1");
        validate(&document(BUILD, &tree, "")).unwrap();
    }

    #[test]
    fn rejects_namespace_tree_lookalikes_and_malformed_identity_lists() {
        let namespaced = "<Tree><x:Spec xmlns:x='urn:extension' treeVersion='0_5' nodes='' classId='7' ascendClassId='0'/></Tree>";
        assert!(matches!(
            validate(&document(BUILD, namespaced, "")),
            Err(PreflightError::NamespacedSection { .. })
        ));
        for replacement in [
            "nodes='-1'",
            "nodes='1,broken'",
            "nodes='1,'",
            "nodes='nil'",
        ] {
            assert!(matches!(
                validate(&document(BUILD, &TREE.replace("nodes=''", replacement), "")),
                Err(PreflightError::InvalidAttribute {
                    attribute: "nodes",
                    ..
                })
            ));
        }
        for replacement in ["classId='nil'", "classId='-1'", "classId='7.5'"] {
            assert!(
                validate(&document(
                    BUILD,
                    &TREE.replace("classId='7'", replacement),
                    ""
                ))
                .is_err()
            );
        }
        assert!(matches!(
            validate(&document(BUILD, &TREE.replace("0_5", "999"), "")),
            Err(PreflightError::UnsupportedTreeVersion { .. })
        ));
    }

    #[test]
    fn rejects_selection_clamping_and_conflicting_main_skill_aliases() {
        for active in ["0", "2", "-1", "1.5"] {
            let tree = TREE.replace("<Tree>", &format!("<Tree activeSpec='{active}'>"));
            assert!(matches!(
                validate(&document(BUILD, &tree, "")),
                Err(PreflightError::InvalidAttribute {
                    attribute: "activeSpec",
                    ..
                })
            ));
        }
        let build = "<Build targetVersion='0_1' level='1' mainSocketGroup='2' mainSkillIndex='1'/>";
        assert!(matches!(
            validate(&document(build, TREE, "")),
            Err(PreflightError::ConflictingMainSkill)
        ));
        let build = "<Build targetVersion='0_1' level='1' mainSocketGroup='0'/>";
        assert!(matches!(
            validate(&document(build, TREE, "")),
            Err(PreflightError::InvalidAttribute {
                attribute: "mainSocketGroup",
                ..
            })
        ));
    }

    #[test]
    fn rejects_explicit_invalid_active_skill_indexes_in_both_layouts() {
        for attribute in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            for value in [
                "",
                "0",
                "-1",
                "0.5",
                "1.5",
                "NaN",
                "inf",
                "1e999",
                "4294967296",
            ] {
                let skill = format!("<Skill {attribute}='{value}'/>");
                for skills in [
                    format!("<Skills>{skill}</Skills>"),
                    format!("<Skills><SkillSet id='1'>{skill}</SkillSet></Skills>"),
                ] {
                    let error = validate(&document(BUILD, TREE, &skills)).unwrap_err();
                    assert!(
                        matches!(error, PreflightError::InvalidAttribute {
                            element: "Skill", attribute: actual, ..
                        } if actual == attribute),
                        "{attribute}={value:?}: {error}"
                    );
                }
            }
        }
        let error = validate(&document(
            BUILD,
            TREE,
            "<Skills><Skill mainActiveSkill='0'/></Skills>",
        ))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid Skill.mainActiveSkill value \"0\": expected a positive active-skill index"
        );
        for skills in [
            "<Skills><Skill/><Skill mainActiveSkill='1' mainActiveSkillCalcs='2'/></Skills>",
            "<Skills><SkillSet id='1'><Skill mainActiveSkill='2' mainActiveSkillCalcs='1'/></SkillSet></Skills>",
        ] {
            validate(&document(BUILD, TREE, skills)).unwrap();
        }
    }

    #[test]
    fn accepts_upstream_nil_skill_sentinel_without_accepting_zero() {
        let xml = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/builds/pobarchives-Dfz36mCq.xml"
        ));
        assert!(xml.contains("mainActiveSkill=\"nil\""));
        assert!(xml.contains("mainActiveSkillCalcs=\"nil\""));
        validate(xml).unwrap();
        for skills in [
            "<Skills><Skill mainActiveSkill='nil' mainActiveSkillCalcs='nil'/></Skills>",
            "<Skills><SkillSet id='1'><Skill mainActiveSkill='nil' mainActiveSkillCalcs='1'/></SkillSet></Skills>",
            "<Skills><Skill mainActiveSkill='2' mainActiveSkillCalcs='nil'/></Skills>",
        ] {
            validate(&document(BUILD, TREE, skills)).unwrap();
        }
        for skill in [
            "<Skill mainActiveSkill='0' mainActiveSkillCalcs='nil'/>",
            "<Skill mainActiveSkill='nil' mainActiveSkillCalcs='0'/>",
            "<Skill mainActiveSkill='null'/>",
            "<Skill mainActiveSkillCalcs='NIL'/>",
        ] {
            assert!(matches!(
                validate(&document(BUILD, TREE, &format!("<Skills>{skill}</Skills>"))),
                Err(PreflightError::InvalidAttribute {
                    element: "Skill",
                    ..
                })
            ));
        }
    }
    #[test]
    fn rejects_malformed_or_nonfinite_config_numbers_in_both_layouts() {
        for element in ["Input", "Placeholder"] {
            for value in [
                "", "NaN", "nan", "inf", "-inf", "Infinity", "1e999", "-1e999", "broken", "1.2.3",
            ] {
                let input = format!("<{element} name='enemyLevel' number='{value}'/>");
                for config in [
                    format!("<Config>{input}</Config>"),
                    format!("<Config><ConfigSet id='1'>{input}</ConfigSet></Config>"),
                ] {
                    let error = validate(&document(BUILD, TREE, &config)).unwrap_err();
                    assert!(
                        matches!(error, PreflightError::InvalidAttribute {
                            element: actual, attribute: "number", ..
                        } if actual == element),
                        "{element}.number={value:?}: {error}"
                    );
                }
            }
        }
        let error = validate(&document(
            BUILD,
            TREE,
            "<Config><Input name='enemyLevel' number='1e999'/></Config>",
        ))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid Input.number value \"1e999\": expected a finite number"
        );
    }

    #[test]
    fn accepts_finite_config_numbers_and_preserves_literal_strings() {
        for element in ["Input", "Placeholder"] {
            for value in ["0", "-1.5", "+2", "1e2", "1e-999", " 1.5 "] {
                let input = format!("<{element} name='test' number='{value}'/>");
                validate(&document(BUILD, TREE, &format!("<Config>{input}</Config>"))).unwrap();
                validate(&document(
                    BUILD,
                    TREE,
                    &format!("<Config><ConfigSet id='1'>{input}</ConfigSet></Config>"),
                ))
                .unwrap();
            }
            let input = format!("<{element} name='literal' string='inf'/>");
            validate(&document(BUILD, TREE, &format!("<Config>{input}</Config>"))).unwrap();
            validate(&document(
                BUILD,
                TREE,
                &format!("<Config><ConfigSet id='1'>{input}</ConfigSet></Config>"),
            ))
            .unwrap();
        }
        validate(&document(
            BUILD,
            TREE,
            "<Config><Input name='flag' boolean='true'/></Config>",
        ))
        .unwrap();
    }

    #[test]
    fn numeric_checks_do_not_walk_unrelated_extensions() {
        let extensions = r#"
            <Future><Skills><Skill mainActiveSkill='0'/></Skills>
                <Config><Input name='test' number='inf'/></Config></Future>
            <Skills>
                <Future><Skill mainActiveSkill='0'/></Future>
                <SkillSet id='1'><Future><Skill mainActiveSkillCalcs='0'/></Future></SkillSet>
                <x:Skill xmlns:x='urn:extension' mainActiveSkill='0'/>
                <x:SkillSet xmlns:x='urn:extension'><Skill mainActiveSkill='0'/></x:SkillSet>
            </Skills>
            <Config>
                <Future><Input name='test' number='inf'/></Future>
                <ConfigSet id='1'><Future><Placeholder name='test' number='NaN'/></Future></ConfigSet>
                <x:Input xmlns:x='urn:extension' name='test' number='inf'/>
                <x:ConfigSet xmlns:x='urn:extension'><Input name='test' number='inf'/></x:ConfigSet>
            </Config>
        "#;
        // Unknown extension content remains outside numeric scenario validation.
        let plain_extensions = extensions
            .lines()
            .filter(|line| !line.contains("<x:"))
            .collect::<Vec<_>>()
            .join("\n");
        validate(&document(BUILD, TREE, &plain_extensions)).unwrap();
        // The separate evaluator syntax gate rejects namespace declarations that
        // PoB's attribute scanner cannot represent; the lossless importer still accepts them.
        assert!(matches!(
            validate(&document(BUILD, TREE, extensions)),
            Err(PreflightError::IncompatibleSyntax(_))
        ));
    }
    #[test]
    fn evaluator_rejects_parser_disagreement_while_container_import_remains_lossless() {
        for level in ["level = '60'", "level='6&#48;'", "level='6&#x30;'"] {
            let build = format!("<Build targetVersion='0_1' {level}/>");
            let xml = document(&build, TREE, "");
            let parsed = Document::parse(&xml).unwrap();
            assert_eq!(
                parsed
                    .root_element()
                    .first_element_child()
                    .unwrap()
                    .attribute("level"),
                Some("60")
            );
            assert_eq!(crate::decode_build(xml.as_bytes()).unwrap().xml, xml);
            assert!(matches!(
                validate(&xml),
                Err(PreflightError::IncompatibleSyntax(_))
            ));
        }
        let split = "<Item>Rarity: NORMAL\n<![CDATA[Wooden Club\n]]>Item Level: 1\nQuality: 0\nImplicits: 0</Item>";
        let parsed = Document::parse(split).unwrap();
        assert_eq!(
            parsed.root_element().children().count(),
            1,
            "roxmltree merges adjacent ordinary/CDATA strings"
        );
        assert!(crate::xml_compat::validate(split).is_err());
    }
    #[test]
    fn rejects_malformed_and_wrong_root_documents() {
        assert!(matches!(
            validate("<PathOfBuilding2>"),
            Err(PreflightError::Xml(_))
        ));
        assert!(matches!(
            validate("<Other/>"),
            Err(PreflightError::WrongRoot)
        ));
        assert!(matches!(
            validate("<PathOfBuilding2 xmlns='urn:other'/>"),
            Err(PreflightError::WrongRoot)
        ));
    }
}
