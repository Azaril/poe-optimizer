//! Only the retained default state of the source's future jewel-search tool.
//! No equipped item, effective tree, native objective or inventory is inferred.
use super::super::character_reward_inventory::CompiledCharacterRewardInventory;
use super::*;

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    frame: Option<&CompiledCharacterRewardInventory<'_>>,
) -> Result<Vec<(SourceOccurrenceId, OwnedDefinitionKey)>> {
    // The independently checked source policy supplies its injected version.
    // Borrow only structural framing, never the character-reward certificate.
    let Some(frame) = frame else {
        return Ok(vec![]);
    };
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(vec![]);
    }
    let mut build = None;
    for source in root.children() {
        b.charge(1)?;
        let row = &evidence.rows()[source.ordinal() as usize];
        // Count namespaced lookalikes too: another saver must not overwrite a
        // checked prefix. The sole candidate must then be an ordinary frame.
        if row.occurrence().name() == "Build" && build.replace(row).is_some() {
            return Ok(vec![]);
        }
    }
    let Some(build) = build else {
        return Ok(vec![]);
    };
    if build.occurrence().parent() != Some(root.occurrence().id())
        || !frame.accepts_build_frame(b, build)?
    {
        return Ok(vec![]);
    }
    let mut candidate = None;
    for source in build.children() {
        b.charge(1)?;
        let row = &evidence.rows()[source.ordinal() as usize];
        if row.occurrence().name() != "TimelessData" {
            continue;
        }
        charge_row(b, row)?;
        if candidate.is_some()
            || row.occurrence().parent() != Some(build.occurrence().id())
            || row.attributes().len() != 4
            || !plain_row(
                row,
                &[
                    "devotionVariant1",
                    "devotionVariant2",
                    "searchList",
                    "searchListFallback",
                ],
                true,
            )
            || value(row, "devotionVariant1") != Some("1")
            || value(row, "devotionVariant2") != Some("1")
            || value(row, "searchList") != Some("")
            || value(row, "searchListFallback") != Some("")
            || row.attributes().iter().any(|attribute| {
                !attribute
                    .decoded()
                    .is_ok_and(|value| attribute.raw() == value)
            })
            || b.origins
                .get(source.ordinal() as usize)
                .is_none_or(|origin| origin.source != *source)
        {
            return Ok(vec![]);
        }
        candidate = Some(*source);
    }
    // Build.Load/Save retain these values; TreeTab.FindTimelessJewel consumes
    // them only in later search/edit/result actions. Loaded false/zero filter
    // fields need not equal omitted constructor fields, and are not admitted.
    Ok(candidate
        .map(|source| vec![(source, key("source-timeless-search-defaults"))])
        .unwrap_or_default())
}

#[cfg(test)]
mod tests;
