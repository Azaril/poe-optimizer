//! Checked source-family absence, without parsing or evaluating modifier text.
//!
//! Normal saved loading can migrate legacy text and clears its old input; the
//! separate headless fallback uses enabled block text. This adapter does not
//! emulate either precedence path. Every serialized text source must be empty,
//! including disabled or overwritten sources, before either path is harmless.
use super::*;

pub(super) struct CheckedCustomModifiers {
    scope: SourceOccurrenceId,
    scenario: ScenarioPresetId,
    origins: Vec<SourceOccurrenceId>,
}

impl CheckedCustomModifiers {
    pub(super) fn scope(&self) -> SourceOccurrenceId {
        self.scope
    }

    pub(super) fn belongs_to(&self, scope: SourceOccurrenceId, scenario: ScenarioPresetId) -> bool {
        self.scope == scope && self.scenario == scenario
    }

    pub(super) fn origins(&self) -> &[SourceOccurrenceId] {
        &self.origins
    }

    pub(super) fn state(&self) -> ConfigurationSourceState {
        if self.origins.is_empty() {
            ConfigurationSourceState::Absent
        } else {
            ConfigurationSourceState::AuthoredNone
        }
    }
}

/// The caller has checked the complete fresh Config container, including every
/// sibling and all original attributes. The family is collected only when the
/// checked census requests this adapter; existing policies do no extra scanning.
pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: ScenarioPresetId,
) -> Result<Option<CheckedCustomModifiers>> {
    let evidence = b.evidence;
    let children = evidence.rows()[scope.ordinal() as usize].children();
    b.charge(children.len())?;
    if children.len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("custom modifier source entries"));
    }
    let mut origins = Vec::new();
    let mut legacy = None;
    for origin in children {
        let row = &evidence.rows()[origin.ordinal() as usize];
        // The shared row charge bounds all inspected names, raw attributes and
        // decoded content. Raw fragment bytes below need their own charge:
        // XML.lua's consumed text has already trimmed ordinary whitespace.
        source_shape::charge_row(b, row)?;
        let custom_block = row.occurrence().name() == "CustomModifierBlock";
        let legacy_input = source_shape::value(row, "name") == Some("customMods");
        if !custom_block && !legacy_input {
            continue;
        }
        if row.occurrence().parent() != Some(scope) {
            return Ok(None);
        }
        if custom_block {
            if !source_shape::plain_row(row, &["title", "enabled"], false)
                || !row.children().is_empty()
                || source_shape::value(row, "enabled")
                    .is_some_and(|value| !matches!(value, "true" | "false"))
            {
                return Ok(None);
            }
            let SourceContentEvidence::Available(content) = row.content() else {
                return Ok(None);
            };
            for fragment in content.fragments() {
                b.charge(fragment.raw().len().saturating_add(1))?;
                if !fragment.raw().is_empty() {
                    return Ok(None);
                }
            }
        } else if row.occurrence().name() != "Input"
            || !source_shape::plain_row(row, &["name", "string"], true)
            || row.attributes().len() != 2
            || source_shape::value(row, "string") != Some("")
            || legacy.replace(*origin).is_some()
        {
            // Placeholder/wrong-lane aliases and duplicate legacy authority
            // cannot be mistaken for constructor absence.
            return Ok(None);
        }
        b.charge(1)?;
        origins.push(*origin);
    }
    Ok(Some(CheckedCustomModifiers {
        scope,
        scenario,
        origins,
    }))
}
