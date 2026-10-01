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
    let mut work = row.children().len().saturating_add(1);
    for attribute in row.attributes() {
        work = work
            .saturating_add(attribute.raw().len())
            .saturating_add(attribute.origin().name.len())
            .saturating_add(1);
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
