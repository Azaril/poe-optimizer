//! Exact ownership of already-attributed item range writes. No input or inventory
//! is completed here, and source ranges never become native evaluator concepts.
use super::source_shape::{charge_row, plain_row};
use super::*;
use crate::owned_item_source::{
    ItemLayoutStatus, ItemRangeAttribution, ItemRangeDecision, ItemRangeOrigin, ItemRangeTarget,
};
use crate::source_xml::PobContentEntry;

/// Captured while the converter's typed emission receives its actual owned ID.
pub(super) struct EmittedModifier {
    pub line: usize,
    pub emission: usize,
    pub id: ModifierInstanceId,
}

fn attribute_matches(row: &SourceEvidenceRow<'_>, origin: &SourceAttributeRef, name: &str) -> bool {
    origin.occurrence == row.occurrence().id()
        && row
            .attributes()
            .get(origin.index as usize)
            .is_some_and(|attribute| {
                attribute.origin().namespace.is_none()
                    && attribute.origin().name == name
                    && attribute.decoded().is_ok()
            })
}

/// Attach only proven output correspondence or a real unresolved responsibility
/// on this exact item. Malformed, overwritten and unsupported writes retain the
/// historical fallback. Existing links are never removed or replaced.
pub(super) fn attach(
    b: &mut Builder<'_, '_>,
    source: &SourceEvidenceRow<'_>,
    attribution: &ItemRangeAttribution,
    item: &ItemDraft,
    lines: &[NormalizedItemLine],
    emitted: &[EmittedModifier],
) -> Result<bool> {
    let report = attribution.report();
    b.charge(report.writes.len())?;
    if !attribution.can_convert_lines()
        || !report
            .writes
            .iter()
            .any(|write| matches!(write.origin, ItemRangeOrigin::Xml { .. }))
    {
        return Ok(false);
    }
    let Some(parent) = equipment_membership::ordinary_items(b)? else {
        return Ok(false);
    };
    charge_row(b, source)?;
    if source.occurrence().parent() != Some(parent)
        || source.occurrence().name() != "Item"
        || report.item != source.occurrence().id()
        || !plain_row(source, &["id"], false)
        || !matches!(
            source.authored_instance(),
            Some(AuthoredInstanceId::ItemRecord(_))
        )
    {
        return Ok(false);
    }
    // The canonical Items census excludes duplicate/aliased Item keys. Check
    // the constructor's exact owned record as well, not merely some parent link.
    let owner_links = b.origins[source.occurrence().id().ordinal() as usize]
        .links
        .len();
    b.charge(owner_links)?;
    let mut owners = b.origins[source.occurrence().id().ordinal() as usize]
        .links
        .iter()
        .filter_map(|link| match link {
            OwnedOriginTarget::Item(id) => Some(*id),
            _ => None,
        });
    if owners.next() != Some(item.id) || owners.next().is_some() {
        return Err(NormalizationError::Policy(
            "item range owned item correspondence",
        ));
    }
    let SourceContentEvidence::Available(content) = source.content() else {
        return Ok(false);
    };
    b.charge(
        lines
            .len()
            .saturating_add(emitted.len())
            .saturating_add(item.modifiers.members.len()),
    )?;
    let by_line: BTreeMap<_, _> = lines.iter().map(|line| (line.index, line)).collect();
    let by_emission: BTreeMap<_, _> = emitted
        .iter()
        .map(|entry| ((entry.line, entry.emission), entry.id))
        .collect();
    let modifiers: BTreeMap<_, _> = item
        .modifiers
        .members
        .iter()
        .map(|modifier| (modifier.id, modifier))
        .collect();
    if by_line.len() != lines.len()
        || by_emission.len() != emitted.len()
        || modifiers.len() != item.modifiers.members.len()
    {
        return Err(NormalizationError::Policy(
            "item range duplicate output correspondence",
        ));
    }
    let mut attachments = Vec::new();
    let evidence = b.evidence;
    for (write_index, write) in report.writes.iter().enumerate() {
        let ItemRangeOrigin::Xml {
            occurrence,
            content_entry,
            id: Some(id),
            range: Some(range),
        } = &write.origin
        else {
            continue;
        };
        let row = &evidence.rows()[occurrence.ordinal() as usize];
        charge_row(b, row)?;
        if row.occurrence().id() != *occurrence
            || row.occurrence().parent() != Some(source.occurrence().id())
            || row.occurrence().name() != "ModRange"
            || !plain_row(row, &["id", "range"], true)
            || row.attributes().len() != 2
            || !attribute_matches(row, id, "id")
            || !attribute_matches(row, range, "range")
            || !write.source_id.is_some_and(|index| index > 0)
            || !write
                .fraction
                .is_some_and(|fraction| fraction.is_finite() && (0.0..=1.0).contains(&fraction))
            || !matches!(content.consumed().get(*content_entry),
                Some(PobContentEntry::Element { child_index })
                    if source.children().get(*child_index) == Some(occurrence))
        {
            continue;
        }
        let targets = match (&report.layout, &write.target) {
            (ItemLayoutStatus::Proven, ItemRangeTarget::Line(line)) => {
                let Some(attributed) = line
                    .checked_sub(1)
                    .and_then(|index| report.lines.get(index))
                else {
                    return Err(NormalizationError::Policy(
                        "item range attributed line correspondence",
                    ));
                };
                if attributed.index != *line
                    || !attributed.member.is_some_and(|member| member.line == *line)
                    || !matches!(attributed.range,
                        ItemRangeDecision::Resolved { fraction, winning_write }
                            if winning_write == write_index && Some(fraction) == write.fraction)
                {
                    continue;
                }
                let Some(output) = by_line.get(line) else {
                    return Err(NormalizationError::Policy(
                        "item range converted line correspondence",
                    ));
                };
                let ItemLineOutcome::Known { emissions, .. } = &output.outcome else {
                    continue;
                };
                b.charge(emissions.len())?;
                if emissions.is_empty()
                    || emissions
                        .iter()
                        .any(|emission| !matches!(emission, ConvertedItemEmission::Modifier { .. }))
                {
                    continue;
                }
                let mut targets = vec![OwnedOriginTarget::Item(item.id)];
                let mut complete = true;
                for (index, emission) in emissions.iter().enumerate() {
                    let ConvertedItemEmission::Modifier {
                        definition,
                        rolls,
                        rolls_closure,
                    } = emission
                    else {
                        unreachable!()
                    };
                    let Some(id) = by_emission.get(&(*line, index)) else {
                        // A recognized line is diagnostic evidence. Aggregation
                        // can withhold its modifier when the template or its
                        // membership is unresolved or rejects that definition.
                        // No allocated output means no correspondence authority.
                        complete = false;
                        break;
                    };
                    let Some(actual) = modifiers.get(id) else {
                        return Err(NormalizationError::Policy(
                            "item range modifier record correspondence",
                        ));
                    };
                    b.charge(rolls.len().saturating_add(actual.rolls.members.len()))?;
                    if !matches!(&actual.definition, DraftField::Known { value } if value == definition)
                        || !matches!(rolls_closure, SchemaClosure::Complete)
                        || actual.rolls.completion != DraftListCompletion::Complete
                        || actual.rolls.members.len() != rolls.len()
                        || !actual
                            .rolls
                            .members
                            .iter()
                            .zip(rolls)
                            .all(|(actual, expected)| {
                                actual.to_resolved().as_ref() == Some(expected)
                            })
                    {
                        complete = false;
                        break;
                    }
                    // Fixed literal rolls still have source ownership. This link
                    // does not assert that the range numerically changed them.
                    targets.push(OwnedOriginTarget::Modifier(*id));
                }
                if !complete {
                    continue;
                }
                targets
            }
            (ItemLayoutStatus::Pending(_), ItemRangeTarget::Pending) => {
                let DraftListCompletion::Pending { id, code } = &item.modifiers.completion else {
                    continue;
                };
                if code.as_str() != "item-modifiers-not-converted" {
                    continue;
                }
                vec![
                    OwnedOriginTarget::Item(item.id),
                    OwnedOriginTarget::Issue(*id),
                ]
            }
            _ => continue,
        };
        if attachments.len() >= b.limits.draft.input.max_collection_entries {
            return Err(NormalizationError::Limit("item range source attachments"));
        }
        b.charge(targets.len())?;
        attachments.push((*occurrence, targets));
    }
    let mut attached = false;
    for (source, targets) in attachments {
        for target in targets {
            b.charge(b.origins[source.ordinal() as usize].links.len())?;
            if !b.origins[source.ordinal() as usize].links.contains(&target) {
                b.link(source, target)?;
                attached = true;
            }
        }
    }
    Ok(attached)
}
