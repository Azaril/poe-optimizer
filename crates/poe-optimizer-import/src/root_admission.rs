//! Conservative MAIN admission for source-proven inert root containers.
//!
//! This is a structural capability check, not complete build validation. Core
//! Build/Config/Tree/Skills/Items/Notes semantics remain with their consumers.
//! Accepted auxiliary bytes are retained; no selector or load migration is applied.
use crate::{
    build_source::{self, RootProjection, RootSection, RootSectionKind, SourceElement},
    source_xml::{SourceXmlError, invalid},
};
use poe_optimizer_core::options::Scalar;
use roxmltree::Document;
use std::collections::BTreeSet;

pub type RootAdmissionError = SourceXmlError;

/// Non-forgeable evidence for the exact document checked by this function.
#[derive(Debug)]
pub struct MainRootAdmission<'input> {
    projection: RootProjection<'input>,
}
impl<'input> MainRootAdmission<'input> {
    pub fn projection(&self) -> &RootProjection<'input> {
        &self.projection
    }
}

/// Admit only the pinned source's inert auxiliary shapes for MAIN calculations.
///
/// Original CalcsTab input belongs to CALCS, independently of Build.mainSocketGroup.
/// The three admitted keys are absent from ConfigTab's legacy migration map.
/// Unknown/legacy keys reject even when a particular Config would disable migration.
/// Bounds and exact scalar syntax are conservative coverage limits, not game legality.
pub fn validate_main<'input>(
    document: &Document<'input>,
) -> Result<MainRootAdmission<'input>, RootAdmissionError> {
    crate::xml_compat::validate_native_with_configuration(document)
        .map_err(|e| invalid(e.byte_offset, e.reason))?;
    let projection = build_source::project(document)?;
    let root = projection.root();
    shape(root, &[], false)?;
    let mut sections = BTreeSet::new();
    for section in projection.sections() {
        let element = section.element();
        if !sections.insert(element.name()) {
            return Err(fail(element, "duplicate root section"));
        }
        if element.has_namespaces() || element.namespace().is_some() {
            return Err(fail(element, "namespaced root section is unsupported"));
        }
        match section.kind() {
            RootSectionKind::Build
            | RootSectionKind::Config
            | RootSectionKind::Items
            | RootSectionKind::Skills
            | RootSectionKind::Tree
            | RootSectionKind::Notes => {}
            RootSectionKind::Import => import(element)?,
            RootSectionKind::TreeView => tree_view(element)?,
            RootSectionKind::Party => party(element)?,
            RootSectionKind::Calcs => calcs(section)?,
            RootSectionKind::LegacySpec | RootSectionKind::Unknown => {
                return Err(fail(element, "unsupported root section"));
            }
        }
    }
    Ok(MainRootAdmission { projection })
}
fn fail(element: &SourceElement<'_>, reason: &str) -> RootAdmissionError {
    invalid(
        element.source_range().start,
        format!("MAIN {}: {reason}", element.name()),
    )
}
fn shape(
    element: &SourceElement<'_>,
    attributes: &[&str],
    leaf: bool,
) -> Result<(), RootAdmissionError> {
    if element.namespace().is_some()
        || element.has_namespaces()
        || element
            .attributes()
            .iter()
            .any(|a| a.namespace().is_some() || !attributes.contains(&a.name()))
        || element.has_non_whitespace_text()
        || (leaf && element.child_element_count() != 0)
    {
        return Err(fail(
            element,
            "unsupported attributes, namespace, or content",
        ));
    }
    Ok(())
}
fn boolean(element: &SourceElement<'_>, name: &str) -> Result<(), RootAdmissionError> {
    if element
        .attribute(name)
        .is_some_and(|v| !matches!(v.decoded(), "true" | "false"))
    {
        return Err(fail(
            element,
            "boolean attribute must be exactly true or false",
        ));
    }
    Ok(())
}
fn import(element: &SourceElement<'_>) -> Result<(), RootAdmissionError> {
    // ImportTab.Load persists these UI fields. SetText/selection do not notify
    // callbacks on load. exportParty=true additionally changes calculation work.
    shape(
        element,
        &[
            "lastRealm",
            "lastLeague",
            "lastAccountHash",
            "lastCharacterHash",
            "importLink",
            "useGeneratedItemText",
            "exportParty",
        ],
        true,
    )?;
    boolean(element, "useGeneratedItemText")?;
    boolean(element, "exportParty")?;
    if element
        .attribute("exportParty")
        .is_some_and(|v| v.decoded() == "true")
    {
        return Err(fail(element, "party buff export is outside MAIN coverage"));
    }
    Ok(())
}
fn tree_view(element: &SourceElement<'_>) -> Result<(), RootAdmissionError> {
    shape(
        element,
        &[
            "searchStr",
            "zoomX",
            "zoomY",
            "zoomLevel",
            "showStatDifferences",
        ],
        true,
    )?;
    boolean(element, "showStatDifferences")?;
    for name in ["zoomX", "zoomY", "zoomLevel"] {
        if let Some(value) = element.attribute(name) {
            value
                .decoded()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| fail(element, "view coordinates must be finite numbers"))?;
        }
    }
    Ok(())
}
fn party(element: &SourceElement<'_>) -> Result<(), RootAdmissionError> {
    // PartyTab.Load's empty shape changes UI controls only. All child payloads
    // stay reference-only, including currently ignored ExportedBuffs records.
    shape(
        element,
        &["destination", "append", "ShowAdvanceTools"],
        true,
    )?;
    boolean(element, "append")?;
    boolean(element, "ShowAdvanceTools")?;
    Ok(())
}
fn calcs(section: &RootSection<'_>) -> Result<(), RootAdmissionError> {
    shape(section.element(), &[], false)?;
    let mut inputs = BTreeSet::new();
    for record in section.records() {
        let element = record.element();
        match element.name() {
            "Section" => {
                shape(element, &["id", "subsection", "collapsed"], true)?;
                if element.attribute("id").is_none() {
                    return Err(fail(element, "calculation layout section requires id"));
                }
                boolean(element, "collapsed")?;
            }
            "Input" => {
                shape(element, &["name", "number", "string", "boolean"], true)?;
                let input = record.scalar_input().ok_or_else(|| {
                    record
                        .scalar_error()
                        .cloned()
                        .unwrap_or_else(|| fail(element, "invalid calculation input"))
                })?;
                if !inputs.insert(input.name()) {
                    return Err(fail(element, "duplicate calculation input"));
                }
                match (input.name(), input.value()) {
                    ("skill_number", Scalar::Number(value))
                        if value.is_finite()
                            && *value >= 1.0
                            && *value <= u32::MAX as f64
                            && value.fract() == 0.0 => {}
                    ("misc_buffMode", Scalar::Text(value))
                        if matches!(
                            value.as_str(),
                            "UNBUFFED" | "BUFFED" | "COMBAT" | "EFFECTIVE"
                        ) => {}
                    ("showMinion", Scalar::Boolean(_)) => {}
                    _ => {
                        return Err(fail(
                            element,
                            "unsupported calculation input name, type, or value",
                        ));
                    }
                }
            }
            _ => return Err(fail(element, "unsupported calculation record")),
        }
    }
    Ok(())
}
