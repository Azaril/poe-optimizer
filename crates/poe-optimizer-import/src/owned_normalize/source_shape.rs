//! Small source-evidence checks shared by the finite inventory adapters.
use super::*;
use crate::source_xml::{PobContentEntry, SourceContentKind};

pub(super) fn value<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> Option<&'a str> {
    row.attribute(name)?.decoded().ok()
}

/// Inspect original attributes before consulting the cached last-value map.
pub(super) fn plain_row(row: &SourceEvidenceRow<'_>, attributes: &[&str], flat: bool) -> bool {
    if row.occurrence().has_namespace_context() || (flat && !row.children().is_empty()) {
        return false;
    }
    let mut names = BTreeSet::new();
    if !row.attributes().iter().all(|attribute| {
        attribute.origin().namespace.is_none()
            && attributes.contains(&attribute.origin().name.as_str())
            && attribute.decoded().is_ok()
            && names.insert(attribute.origin().name.as_str())
    }) {
        return false;
    }
    let SourceContentEvidence::Available(content) = row.content() else {
        return false;
    };
    content.fragments().iter().all(|fragment| {
        matches!(fragment.kind(), SourceContentKind::Text | SourceContentKind::Element)
    }) && (!flat
        || content.consumed().iter().all(|entry| {
            matches!(entry, PobContentEntry::Text { text, .. } if text.trim_ascii().is_empty())
        }))
}

pub(super) fn container_text(row: &SourceEvidenceRow<'_>) -> bool {
    matches!(row.content(), SourceContentEvidence::Available(content) if content.consumed().iter().all(|entry| {
        matches!(entry, PobContentEntry::Element { .. })
            || matches!(entry, PobContentEntry::Text {text, ..} if text.trim_ascii().is_empty())
    }))
}

/// Charge inspected bytes and children before any grammar scan or indexing.
pub(super) fn charge_row(b: &mut Builder<'_, '_>, row: &SourceEvidenceRow<'_>) -> Result<()> {
    charge_row_parts(b, row, None)
}

/// Structural checks inspect original attribute names and cached decode status,
/// not every value's bytes. Charge the selected values actually scanned by that
/// proof; callers must separately charge any later value inspection. This does
/// not change the full-row accounting used by existing value-consuming proofs.
pub(super) fn charge_frame(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    inspected_values: &[&str],
) -> Result<()> {
    charge_row_parts(b, row, Some(inspected_values))
}

fn charge_row_parts(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    inspected_values: Option<&[&str]>,
) -> Result<()> {
    let mut work = row.children().len().saturating_add(1);
    for attribute in row.attributes() {
        work = work
            .saturating_add(attribute.origin().name.len())
            .saturating_add(1);
        if inspected_values.is_none_or(|names| names.contains(&attribute.origin().name.as_str())) {
            work = work.saturating_add(attribute.raw().len());
        }
    }
    if let SourceContentEvidence::Available(content) = row.content() {
        work = work.saturating_add(content.fragments().len());
        for entry in content.consumed() {
            work = work.saturating_add(match entry {
                PobContentEntry::Text { text, .. } => text.len().saturating_add(1),
                PobContentEntry::Element { .. } => 1,
            });
        }
    }
    b.charge(work)
}

/// Keep the spent issue ID and allocator watermark while retiring its one live
/// origin link. Inventory proof never renumbers records or refunds work.
pub(super) fn retire_membership(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    completion: &mut DraftListCompletion,
    expected_code: &str,
) -> Result<()> {
    let DraftListCompletion::Pending { id, code } = completion else {
        return Ok(());
    };
    if code.as_str() != expected_code {
        return Ok(());
    }
    let retired = *id;
    b.charge(b.origins[source.ordinal() as usize].links.len())?;
    b.origins[source.ordinal() as usize]
        .links
        .retain(|link| !matches!(link, OwnedOriginTarget::Issue(id) if *id == retired));
    *completion = DraftListCompletion::Complete;
    Ok(())
}

/// Canonical positive Lua-exact numeric configuration identities. Reject aliases
/// before using source keys: the source loader indexes them through tonumber.
fn numeric_key(text: &str) -> Option<u64> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value = text.parse::<u64>().ok()?;
    (value < (1_u64 << 53)).then_some(value)
}

/// Validate the complete container shape before admitting any independent set.
/// A malformed sibling cannot silently change which configuration source loaded.
pub(super) fn fresh_config_sets(
    b: &mut Builder<'_, '_>,
) -> Result<Option<Vec<SourceOccurrenceId>>> {
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(None);
    }
    let mut config = None;
    for id in root.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        if row.occurrence().name() == "Config" {
            if config.replace(row).is_some() {
                return Ok(None);
            }
        } else if row.occurrence().name() == "ConfigSet" {
            return Ok(None);
        }
    }
    let Some(config) = config else {
        return Ok(None);
    };
    charge_row(b, config)?;
    if !plain_row(config, &["activeConfigSet"], false) || !container_text(config) {
        return Ok(None);
    }
    let Some(selected) = value(config, "activeConfigSet").and_then(numeric_key) else {
        return Ok(None);
    };
    if config.children().len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("enemy level configuration sets"));
    }
    let mut ids = BTreeSet::new();
    let mut sets = Vec::new();
    for id in config.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        charge_row(b, row)?;
        if row.occurrence().name() != "ConfigSet"
            || row.occurrence().parent() != Some(config.occurrence().id())
            || !plain_row(row, &["id", "title"], false)
            || !container_text(row)
            || !matches!(
                row.authored_instance(),
                Some(AuthoredInstanceId::ConfigSet(_))
            )
        {
            return Ok(None);
        }
        let Some(key) = value(row, "id").and_then(numeric_key) else {
            return Ok(None);
        };
        if !ids.insert(key) {
            return Ok(None);
        }
        let mut names = BTreeSet::new();
        for child in row.children() {
            let child = &evidence.rows()[child.ordinal() as usize];
            charge_row(b, child)?;
            match child.occurrence().name() {
                "Input" | "Placeholder" => {
                    if !plain_row(child, &["name", "number", "string", "boolean"], true)
                        || child.attributes().len() != 2
                    {
                        return Ok(None);
                    }
                    let Some(name) = value(child, "name").filter(|name| !name.is_empty()) else {
                        return Ok(None);
                    };
                    let numeric = value(child, "number").is_some();
                    let string = value(child, "string").is_some();
                    let boolean = value(child, "boolean");
                    if usize::from(numeric) + usize::from(string) + usize::from(boolean.is_some())
                        != 1
                        || boolean.is_some_and(|v| !matches!(v, "true" | "false"))
                        || (child.occurrence().name() == "Placeholder" && boolean.is_some())
                        || !names.insert((child.occurrence().name(), name))
                    {
                        return Ok(None);
                    }
                }
                "CustomModifierBlock" => {
                    if !plain_row(child, &["title", "enabled"], false)
                        || !child.children().is_empty()
                        || value(child, "enabled").is_some_and(|v| !matches!(v, "true" | "false"))
                    {
                        return Ok(None);
                    }
                }
                _ => return Ok(None),
            }
        }
        sets.push(*id);
    }
    Ok(ids.contains(&selected).then_some(sets))
}
