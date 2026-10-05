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
    /// Scoped raw inputs and the exact source evidence used to complete them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_inputs: Option<Vec<ItemParameterInputEvidence>>,
}

pub(super) fn normalize_item(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    id: ItemRecordId,
    membership: Option<&item_modifier_membership::CompiledItemModifierMembership<'_>>,
    inputs: Option<&item_parameter_inputs::CompiledItemParameterInputs<'_>>,
    augments: Option<&equipment_membership::CompiledEquipmentMembership<'_>>,
    ordinary_parent: Option<SourceOccurrenceId>,
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
            parameter_inputs: None,
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
    let mut converted = attribution.convert(b.items)?;
    let membership_proof = if let (Some(membership), Some(augments)) = (membership, augments) {
        membership.prove(
            b,
            row,
            &attribution,
            &converted,
            item_modifier_membership::ItemModifierProofContext {
                augments,
                ordinary_parent,
                inputs,
            },
        )?
    } else {
        None
    };
    let physical_inputs = if let Some(inputs) = inputs {
        inputs.prove(b, row, &attribution, &converted, membership_proof.as_ref())?
    } else {
        None
    };
    if physical_inputs.is_some() {
        b.charge(converted.issues.len())?;
        // The scoped proof fills and checks the entire known parameter set.
        // Retire only the converter's now-resolved missing-input diagnostic;
        // static declaration gaps and every other diagnostic remain visible.
        converted.issues.retain(|issue| {
            issue.problem != ItemTextProblem::RequiredParameterMissing || !issue.lines.is_empty()
        });
    }
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
    let mut emitted_modifiers = Vec::new();
    let member_order = if let Some(proof) = &membership_proof {
        let DraftField::Known { value } = &template else {
            return Err(NormalizationError::Policy(
                "item member proof template mismatch",
            ));
        };
        let construction =
            proof
                .construction_for(source, value)
                .ok_or(NormalizationError::Policy(
                    "item member proof identity mismatch",
                ))?;
        let order = proof.ordered_for(source, value);
        match (construction.kind, order) {
            (item_modifier_membership::ModifierConstructionKind::SingletonV1, None) => {}
            (
                item_modifier_membership::ModifierConstructionKind::PairV2
                | item_modifier_membership::ModifierConstructionKind::DeclaredV3,
                Some(values),
            ) if Some(values.len())
                == construction
                    .implicit_count
                    .checked_add(construction.explicit_count) => {}
            _ => {
                return Err(NormalizationError::Policy(
                    "item member proof order mismatch",
                ));
            }
        }
        order
    } else {
        None
    };
    let mut member_ids = if let Some(order) = member_order {
        b.charge(order.len())?;
        vec![None; order.len()]
    } else {
        vec![]
    };
    for modifier in converted.modifiers {
        b.charge(modifier.rolls.len())?;
        let modifier_id = b.id()?;
        if let Some(order) = member_order {
            b.charge(order.len())?;
            let Some(position) = order
                .iter()
                .position(|v| *v == (modifier.line, modifier.emission))
            else {
                return Err(NormalizationError::Policy(
                    "item member proof emission mismatch",
                ));
            };
            if member_ids[position].replace(modifier_id).is_some() {
                return Err(NormalizationError::Policy(
                    "duplicate proved item member emission",
                ));
            }
        }
        b.link(source, OwnedOriginTarget::Modifier(modifier_id))?;
        emitted_modifiers.push(item_range_origins::EmittedModifier {
            line: modifier.line,
            emission: modifier.emission,
            id: modifier_id,
        });
        let position = line_positions[&modifier.line];
        lines[position].modifiers.push(modifier_id);
        // Known assignments do not prove that the declared roll set is closed.
        // V4 item recipes may retain useful inputs under a Partial declaration.
        let rolls = match modifier.rolls_closure {
            SchemaClosure::Complete => modifier.rolls.into(),
            SchemaClosure::Partial { .. } => b.closure(
                source,
                "modifier-roll-schema-partial",
                modifier.rolls.into_iter().map(Into::into).collect(),
            )?,
        };
        modifiers.push(ModifierDraft {
            id: modifier_id,
            definition: modifier.definition.into(),
            rolls,
        });
    }
    let mut parameters: Vec<ParameterDraft> = converted
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
    let parameter_inputs = if let Some(inputs) = physical_inputs {
        b.charge(
            inputs
                .parameters
                .len()
                .saturating_add(inputs.evidence.len()),
        )?;
        parameters.extend(inputs.parameters.into_iter().map(Into::into));
        Some(inputs.evidence)
    } else {
        None
    };
    let parameters = if parameter_inputs.is_some() {
        parameters.into()
    } else {
        b.closure(source, "item-parameters-not-converted", parameters)?
    };
    let (modifiers, modifier_order) = if membership_proof.is_some() {
        let order =
            if member_order.is_some() {
                member_ids.into_iter().collect::<Option<Vec<_>>>().ok_or(
                    NormalizationError::Policy("incomplete proved item member order"),
                )?
            } else {
                // Historical singleton order and allocation behavior are unchanged.
                vec![modifiers[0].id]
            };
        (modifiers.into(), order.into())
    } else {
        (
            b.closure(source, "item-modifiers-not-converted", modifiers)?,
            b.pending(source, "item-modifier-order-not-converted")?,
        )
    };
    let item = ItemDraft {
        id,
        template,
        item_level,
        quality,
        // Only the separate scoped raw-input proof can close this inventory.
        // Static declarations and owner/contributor coverage remain independent.
        parameters,
        modifiers,
        // Physical record identity and source line positions do not determine
        // semantic transform order. Only a reviewed complete conversion can.
        modifier_order,
    };
    let attached =
        item_range_origins::attach(b, row, &attribution, &item, &lines, &emitted_modifiers)?;
    b.item_range_origins_attached |= attached;
    b.item_texts.push(NormalizedItemText {
        source,
        content_entry: Some(content_entry),
        skipped: None,
        lines,
        issues: converted.issues,
        defaults: converted.defaults,
        parameter_inputs,
        attribution: attribution.into_report(),
    });
    Ok(item)
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
