//! Partial item conversion and exact source-line provenance. This does not replay
//! the source item's mutable ParseRaw/variant/socket lifecycle.
use super::*;

#[derive(Clone, Debug, Serialize)]
pub struct NormalizedItemLine {
    pub index: usize,
    pub text: String,
    pub outcome: ItemLineOutcome,
    pub modifiers: Vec<ModifierInstanceId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct NormalizedItemText {
    pub source: SourceOccurrenceId,
    /// Index in SourceContent::consumed, not an XML byte offset.
    pub content_entry: Option<usize>,
    pub skipped: Option<OwnedDefinitionKey>,
    pub lines: Vec<NormalizedItemLine>,
    pub issues: Vec<ItemTextIssue>,
    /// Immutable Import provenance; source positions never enter owned build records.
    pub attribution: ItemAttributionReport,
    pub defaults: crate::owned_item_lines::ItemDefaultedInputs,
}

pub(super) fn normalize_item(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    id: ItemRecordId,
) -> Result<ItemDraft> {
    let source = row.occurrence().id();
    let attribution = b.item_source.attribute(b.evidence, source, b.items)?;
    b.charge(attribution.report().lines.len() + attribution.report().writes.len())?;
    if !attribution.can_convert_lines() {
        b.item_texts.push(NormalizedItemText {
            source,
            content_entry: None,
            skipped: Some(key("item-content-lifecycle-not-converted")),
            lines: vec![],
            issues: vec![],
            attribution: attribution.into_report(),
            defaults: Default::default(),
        });
        return Ok(ItemDraft {
            id,
            template: b.pending(source, "item-template-not-converted")?,
            parameters: b.closure(source, "item-parameters-not-converted", vec![])?,
            item_level: b.pending(source, "item-level-not-converted")?,
            quality: b.quality(source)?,
            modifiers: b.closure(source, "item-modifiers-not-converted", vec![])?,
            modifier_order: b.pending(source, "item-modifier-order-not-converted")?,
        });
    }
    let content_entry = attribution
        .report()
        .content_entry
        .expect("admitted item text");
    let raw_lines: BTreeMap<_, _> = attribution
        .report()
        .lines
        .iter()
        .map(|line| (line.index, line.raw.as_str()))
        .collect();
    b.charge(raw_lines.values().map(|text| text.len()).sum())?;
    let converted = attribution.convert(b.items)?;
    b.charge(
        converted.lines.len()
            + converted.parameters.len()
            + converted.modifiers.len()
            + converted.defaults.parameters.len(),
    )?;
    let template = match converted.template {
        ItemField::Known { value, .. } => value.into(),
        _ => b.pending(source, "item-template-not-converted")?,
    };
    let item_level = match converted.item_level {
        ItemField::Known { value, .. } if u16::try_from(value.get()).is_ok() => {
            Some(u16::try_from(value.get()).expect("checked item level")).into()
        }
        ItemField::Absent if converted.defaults.item_level_absent => None.into(),
        // No emitted header is not proof that the source explicitly omits this fact.
        // Only a scoped absence proof or owned authoring may supply Known(None).
        _ => b.pending(source, "item-level-not-converted")?,
    };
    let quality = match converted.quality {
        ItemField::Known { value, .. } => Some(value).into(),
        ItemField::Absent if converted.defaults.quality_absent => None.into(),
        // Absence of a header does not establish absence of item quality.
        _ => b.quality(source)?,
    };
    let mut lines: Vec<_> = converted
        .lines
        .into_iter()
        .map(|line| NormalizedItemLine {
            index: line.index,
            text: raw_lines[&line.index].to_owned(),
            outcome: line.outcome,
            modifiers: vec![],
        })
        .collect();
    let line_positions: BTreeMap<_, _> = lines
        .iter()
        .enumerate()
        .map(|(position, line)| (line.index, position))
        .collect();
    let mut modifiers = Vec::new();
    for modifier in converted.modifiers {
        b.charge(modifier.rolls.len())?;
        let modifier_id = b.id()?;
        b.link(source, OwnedOriginTarget::Modifier(modifier_id))?;
        let position = line_positions[&modifier.line];
        lines[position].modifiers.push(modifier_id);
        modifiers.push(ModifierDraft {
            id: modifier_id,
            definition: modifier.definition.into(),
            rolls: modifier.rolls.into(),
        });
    }
    let parameters = converted
        .parameters
        .into_iter()
        .map(|parameter| parameter.assignment.into())
        .chain(
            converted
                .defaults
                .parameters
                .iter()
                .cloned()
                .map(Into::into),
        )
        .collect();
    b.item_texts.push(NormalizedItemText {
        source,
        content_entry: Some(content_entry),
        skipped: None,
        lines,
        issues: converted.issues,
        defaults: converted.defaults,
        attribution: attribution.into_report(),
    });
    Ok(ItemDraft {
        id,
        template,
        item_level,
        quality,
        // Source range attribution is not whole-item closure. Affix, variant,
        // socket and implicit lifecycle still need their own conversion before
        // these collections can be declared complete.
        parameters: b.closure(source, "item-parameters-not-converted", parameters)?,
        modifiers: b.closure(source, "item-modifiers-not-converted", modifiers)?,
        // Physical record identity and source line positions do not determine
        // semantic transform order. Only a reviewed complete conversion can.
        modifier_order: b.pending(source, "item-modifier-order-not-converted")?,
    })
}

/// XML Item/Slot enumeration does not close semantic membership when socketed
/// augment occurrences can still be missing. Keep all known records and attach
/// the shared collection obligations to every contributing source item.
pub(super) fn defer_unmaterialized_augments(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
) -> Result<()> {
    use crate::owned_item_source::{ItemLayoutStatus, ItemSourceProblem};
    let work = b.item_texts.iter().fold(0usize, |sum, item| {
        item.attribution.lines.iter().fold(sum, |sum, line| {
            sum.saturating_add(line.raw.len())
                .saturating_add(line.blockers.len())
                .saturating_add(1)
        })
    });
    b.charge(work)?;
    let sources: Vec<_> = b
        .item_texts
        .iter()
        .filter(|item| {
            let layout_gap = match &item.attribution.layout {
                ItemLayoutStatus::Pending(problems) | ItemLayoutStatus::Unsupported(problems) => {
                    problems.contains(&ItemSourceProblem::RuneLifecycle)
                }
                ItemLayoutStatus::Proven => false,
            };
            layout_gap
                || item.attribution.lines.iter().any(|line| {
                    line.blockers.contains(&ItemSourceProblem::RuneLifecycle)
                // This is only a possibility witness, never decoded socket data.
                // An absent/unreviewed line recipe must not make a Rune header
                // disappear. A named item's presentation title is not a header.
                || !line.presentation && line.raw.trim_ascii().starts_with("Rune:")
                })
        })
        .map(|item| item.source)
        .collect();
    let Some(first) = sources.first().copied() else {
        return Ok(());
    };
    for (completion, code) in [
        (
            &mut draft.items.completion,
            "socketed-item-membership-not-converted",
        ),
        (
            &mut draft.equipment.completion,
            "socketed-equipment-membership-not-converted",
        ),
    ] {
        let (issue, remaining) = match completion {
            DraftListCompletion::Complete => {
                let issue = b.issue(first)?;
                *completion = DraftListCompletion::Pending {
                    id: issue,
                    code: key(code),
                };
                (issue, &sources[1..])
            }
            DraftListCompletion::Pending { id, .. } => (*id, sources.as_slice()),
        };
        for source in remaining {
            b.link(*source, OwnedOriginTarget::Issue(issue))?;
        }
    }
    Ok(())
}
