//! Source-bound trade preferences. These do not define native objectives.
use super::super::source_shape::{charge_row, container_text, plain_row, value};
use super::super::*;

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
) -> Result<Vec<(SourceOccurrenceId, OwnedDefinitionKey)>> {
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(vec![]);
    }
    let Some(items) = equipment_membership::ordinary_items(b)? else {
        return Ok(vec![]);
    };
    let parent = &evidence.rows()[items.ordinal() as usize];
    let mut selected = None;
    for child in parent.children() {
        b.charge(1)?;
        let row = &evidence.rows()[child.ordinal() as usize];
        if row.occurrence().name() != "TradeSearchWeights" {
            continue;
        }
        // Repeated saved frames append to the same source-side list. This
        // bounded grammar admits one complete frame, never a partial prefix.
        if selected.replace(*child).is_some() {
            return Ok(vec![]);
        }
    }
    let Some(source) = selected else {
        return Ok(vec![]);
    };
    let row = &evidence.rows()[source.ordinal() as usize];
    charge_row(b, row)?;
    if row.occurrence().parent() != Some(items)
        || !plain_row(row, &[], false)
        || !container_text(row)
    {
        return Ok(vec![]);
    }
    let mut candidates = vec![(source, key("source-trade-search-weights"))];
    if candidates.len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("source trade preference rows"));
    }
    for child in row.children() {
        let stat = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, stat)?;
        if stat.occurrence().parent() != Some(source)
            || stat.occurrence().name() != "Stat"
            || !plain_row(stat, &["label", "stat", "weightMult"], true)
            || !value(stat, "label").is_some_and(|v| !v.is_empty())
            || !value(stat, "stat").is_some_and(|v| !v.is_empty())
            || !value(stat, "weightMult").is_some_and(|v| {
                !v.is_empty() && v.trim_ascii() == v && v.parse::<f64>().is_ok_and(f64::is_finite)
            })
        {
            return Ok(vec![]);
        }
        if candidates.len() >= b.limits.draft.input.max_collection_entries {
            return Err(NormalizationError::Limit("source trade preference rows"));
        }
        // Zero, negative values and repeated stat choices remain distinct raw
        // preferences. No unit, metric mapping, sum or objective is inferred.
        candidates.push((*child, key("source-trade-search-weight")));
    }
    Ok(candidates)
}
