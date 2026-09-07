//! Evaluator-only checks that keep incomplete XML from becoming a fresh default build.
//!
//! This is not a build-legality validator. Optional sections may use PoB defaults;
//! unknown fields remain untouched. Container decoding remains a separate contract.

use std::collections::HashSet;

use roxmltree::{Document, Node, ParsingOptions};
use thiserror::Error;

use crate::import::{MAX_XML_BYTES, MAX_XML_NODES};

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
