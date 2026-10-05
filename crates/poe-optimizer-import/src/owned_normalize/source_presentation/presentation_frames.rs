//! Reviewed saved presentation frames. No calculation input or owner is closed.
use super::super::source_shape::{charge_row, container_text, plain_row, value};
use super::super::*;

fn boolean(value: Option<&str>) -> bool {
    matches!(value, Some("true" | "false"))
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

fn finite(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value.trim_ascii() == value
            && !value.is_empty()
            && value.parse::<f64>().is_ok_and(f64::is_finite)
    })
}

fn tree_view(row: &SourceEvidenceRow<'_>) -> bool {
    // PassiveTreeView.Load:59-74 reads only camera, search highlighting and
    // tooltip display state. Require the complete ordinary saved frame here;
    // partial/malformed saves retain their prior obligation.
    plain_row(
        row,
        &[
            "zoomLevel",
            "zoomX",
            "zoomY",
            "searchStr",
            "showStatDifferences",
        ],
        true,
    ) && value(row, "zoomLevel").is_some_and(|value| {
        !value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && value.parse::<u8>().is_ok_and(|value| value <= 20)
    }) && finite(value(row, "zoomX"))
        && finite(value(row, "zoomY"))
        && value(row, "searchStr").is_some()
        && boolean(value(row, "showStatDifferences"))
}

fn sections(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
) -> Result<Vec<(SourceOccurrenceId, OwnedDefinitionKey)>> {
    let evidence = b.evidence;
    let row = &evidence.rows()[source.ordinal() as usize];
    if !plain_row(row, &[], false) || !container_text(row) {
        return Ok(vec![]);
    }
    let mut result = vec![];
    let mut names = BTreeSet::new();
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        if child.occurrence().parent() != Some(source) {
            return Ok(vec![]);
        }
        match child.occurrence().name() {
            // These settings select the CALCS action and buff mode; neither the
            // Input nor its parent container is a presentation disposition.
            "Input" => {}
            "Section" => {
                // CalcsTab.Load:217-233 changes only subsection.collapsed.
                // CalcSectionControl consumes it for visibility and geometry.
                if !plain_row(child, &["id", "subsection", "collapsed"], true)
                    || !nonempty(value(child, "id"))
                    || !nonempty(value(child, "subsection"))
                    || !boolean(value(child, "collapsed"))
                    || !names.insert((value(child, "id"), value(child, "subsection")))
                {
                    return Ok(vec![]);
                }
                if result.len() >= b.limits.draft.input.max_collection_entries {
                    return Err(NormalizationError::Limit(
                        "source presentation section rows",
                    ));
                }
                result.push((
                    child.occurrence().id(),
                    key("source-calculation-section-layout"),
                ));
            }
            _ => return Ok(vec![]),
        }
    }
    Ok(result)
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    calcs_sections: bool,
    tree_view_enabled: bool,
    empty_notes: bool,
) -> Result<Vec<(SourceOccurrenceId, OwnedDefinitionKey)>> {
    if !calcs_sections && !tree_view_enabled && !empty_notes {
        return Ok(vec![]);
    }
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(vec![]);
    }
    let mut frames: BTreeMap<&str, (SourceOccurrenceId, usize)> = BTreeMap::new();
    for source in root.children() {
        b.charge(1)?;
        let row = &evidence.rows()[source.ordinal() as usize];
        let name = row.occurrence().name();
        if (name == "Calcs" && calcs_sections)
            || (name == "TreeView" && tree_view_enabled)
            || (name == "Notes" && empty_notes)
        {
            let frame = frames.entry(name).or_insert((*source, 0));
            frame.1 = frame.1.saturating_add(1);
        }
    }
    let mut result = vec![];
    for (name, (source, count)) in frames {
        // Duplicate root savers can overwrite state during PoB load. This
        // bounded proof accepts one ordinary saved frame per enabled family.
        if count != 1 {
            continue;
        }
        let row = &evidence.rows()[source.ordinal() as usize];
        charge_row(b, row)?;
        if row.occurrence().parent() != Some(root.occurrence().id()) {
            continue;
        }
        match name {
            "Calcs" => result.extend(sections(b, source)?),
            "TreeView" if tree_view(row) => {
                result.push((source, key("source-tree-view-layout")));
            }
            // NotesTab.Load:75-81 places text in an editor. Nonempty notes may
            // contain useful user intent and remain preserved/unclassified.
            "Notes" if plain_row(row, &[], true) => {
                result.push((source, key("source-empty-notes")));
            }
            _ => {}
        }
        if result.len() > b.limits.draft.input.max_collection_entries {
            return Err(NormalizationError::Limit("source presentation frame rows"));
        }
    }
    Ok(result)
}
