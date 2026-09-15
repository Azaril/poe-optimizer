//! Partial item conversion and exact source-line provenance. This does not replay
//! the source item's mutable ParseRaw/ModRange/variant/socket lifecycle.
use super::*;
use crate::source_xml::PobContentEntry;

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
}

pub(super) fn normalize_item(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    id: ItemRecordId,
) -> Result<ItemDraft> {
    let source = row.occurrence().id();
    let mut texts = Vec::new();
    let mut layout_supported = true;
    if let SourceContentEvidence::Available(content) = row.content() {
        for (index, entry) in content.consumed().iter().enumerate() {
            b.charge(1)?;
            match entry {
                PobContentEntry::Text { text, .. } => texts.push((index, text.as_str())),
                PobContentEntry::Element { .. } => {}
            }
        }
        for child in row.children() {
            let child = &b.evidence.rows()[child.ordinal() as usize];
            // Range overrides only change interpolation. No fraction is supplied
            // below, so any range expression remains pending. Unknown child
            // operations and repeated text resets are not silently ignored.
            if child.occurrence().has_namespace_context() || child.occurrence().name() != "ModRange"
            {
                layout_supported = false;
            }
        }
    }
    let converted = if layout_supported && texts.len() == 1 {
        let (index, text) = texts[0];
        b.charge(text.len())?;
        Some((index, b.items.convert_text(text)?))
    } else {
        None
    };
    let Some((content_entry, converted)) = converted else {
        b.item_texts.push(NormalizedItemText {
            source,
            content_entry: None,
            skipped: Some(key("item-content-lifecycle-not-converted")),
            lines: vec![],
            issues: vec![],
        });
        return Ok(ItemDraft {
            id,
            template: b.pending(source, "item-template-not-converted")?,
            parameters: b.closure(source, "item-parameters-not-converted", vec![])?,
            item_level: b.pending(source, "item-level-not-converted")?,
            quality: b.quality(source)?,
            modifiers: b.closure(source, "item-modifiers-not-converted", vec![])?,
        });
    };
    b.charge(converted.lines.len() + converted.parameters.len() + converted.modifiers.len())?;
    let template = match converted.template {
        ItemField::Known { value, .. } => value.into(),
        _ => b.pending(source, "item-template-not-converted")?,
    };
    let item_level = match converted.item_level {
        ItemField::Known { value, .. } if u16::try_from(value.get()).is_ok() => {
            u16::try_from(value.get())
                .expect("checked item level")
                .into()
        }
        _ => b.pending(source, "item-level-not-converted")?,
    };
    let quality = match converted.quality {
        ItemField::Known { value, .. } => Some(value).into(),
        // Absence of a header does not establish absence of item quality.
        _ => b.quality(source)?,
    };
    let mut lines: Vec<_> = converted
        .lines
        .into_iter()
        .map(|line| NormalizedItemLine {
            index: line.index,
            text: line.text.to_owned(),
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
        .collect();
    b.item_texts.push(NormalizedItemText {
        source,
        content_entry: Some(content_entry),
        skipped: None,
        lines,
        issues: converted.issues,
    });
    Ok(ItemDraft {
        id,
        template,
        item_level,
        quality,
        // Classified text is not whole-item closure. Affix metadata, child range
        // overrides, variants, sockets and implicit lifecycle need their own
        // conversion before these collections can be declared complete.
        parameters: b.closure(source, "item-parameters-not-converted", parameters)?,
        modifiers: b.closure(source, "item-modifiers-not-converted", modifiers)?,
    })
}
