//! Private group selection proof, separate from admission and native authority.
use super::*;

pub(super) struct GroupSelectors {
    source: SourceOccurrenceId,
    main: bool,
    calcs: bool,
}

impl GroupSelectors {
    pub(super) fn proves(
        &self,
        receipt: &InputReceipt,
        field: &UsageInventoryFieldSelector,
    ) -> bool {
        receipt.source == self.source
            && matches!(field,
                UsageInventoryFieldSelector::GroupAttribute { name }
                if name == "mainActiveSkill" && self.main
                    || name == "mainActiveSkillCalcs" && self.calcs)
    }
}

fn exact_neutral(group: &SourceEvidenceRow<'_>, name: &str) -> bool {
    group.attribute(name).is_some_and(|attribute| {
        matches!(attribute.raw(), "nil" | "1") && attribute.decoded().ok() == Some(attribute.raw())
    })
}

/// Same-attempt final accounting: every physical child must have a live checked
/// intrinsic/reference receipt. One SkillUse and zero or more exact support
/// assignments give the source admission filters at most one candidate. This
/// says nothing about whether that candidate is actually admitted or executed.
///
/// Retain only exact, present, literal nil/1 selector fields on the primary
/// receipt. Missing source is distinct from source="", and an absent selector
/// is not manufactured into a value. All other destinations stay conjunctive.
pub(super) fn prove(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    preset: &SkillPresetDraft,
    group: &SourceEvidenceRow<'_>,
    receipts: &InventoryReceipts,
) -> Result<Option<GroupSelectors>> {
    b.charge(
        group
            .attributes()
            .len()
            .saturating_mul(65)
            .saturating_add(2),
    )?;
    let main = exact_neutral(group, "mainActiveSkill");
    let calcs = exact_neutral(group, "mainActiveSkillCalcs");
    if (!main && !calcs) || group.attribute("source").is_some() {
        return Ok(None);
    }
    let mut primary = None;
    let mut targets = Vec::new();
    b.charge(group.children().len().saturating_add(1))?;
    for source in group.children() {
        b.charge(receipts.inputs.len())?;
        let mut matching = receipts.inputs.iter().filter(|receipt| {
            receipt.source == *source
                && receipt.group == group.occurrence().id()
                && receipt.preset == preset.id
        });
        let Some(receipt) = matching.next() else {
            return Ok(None);
        };
        if matching.next().is_some() {
            return Ok(None);
        }
        let UsageInventorySource::Physical { gem: definition } = &receipt.identity else {
            return Ok(None);
        };
        // input_present relies on this charged outer traversal for authored
        // records and adds its own target/set work for support records.
        b.charge(
            b.origins[receipt.source.ordinal() as usize]
                .links
                .len()
                .saturating_mul(6)
                .saturating_add(
                    b.origins[receipt.group.ordinal() as usize]
                        .links
                        .len()
                        .saturating_mul(2),
                )
                .saturating_add(draft.skills.members.len())
                .saturating_add(draft.gems.members.len())
                .saturating_add(draft.supports.members.len())
                .saturating_add(preset.skills.members.len())
                .saturating_add(preset.supports.members.len())
                .saturating_add(receipt.descendants.len())
                .saturating_add(1),
        )?;
        for descendant in &receipt.descendants {
            b.charge(
                b.origins[descendant.ordinal() as usize]
                    .links
                    .len()
                    .saturating_mul(2),
            )?;
        }
        if !input_present(b, draft, preset, receipt)? {
            return Ok(None);
        }
        let row = &b.evidence.rows()[source.ordinal() as usize];
        if row.occurrence().parent() != Some(group.occurrence().id()) {
            return Ok(None);
        }
        let roles = b.roles;
        let Some(role) = skill_input_disposition::charged_source_role(b, roles, row, definition)?
        else {
            return Ok(None);
        };
        if !skill_input_disposition::physical_source_is_singleton(row, definition, role) {
            return Ok(None);
        }
        match (&receipt.output, &role.role) {
            (
                InputOutput::Authored {
                    skill,
                    gem: Some(gem),
                    ..
                },
                OwnedGemRole::Known(AuthoredGemRole::SkillUse),
            ) => {
                if primary.is_some()
                    || !matches!(&skill.source,
                        DraftAuthoredSkillSource::Gem(DraftField::Known { value }) if *value == gem.id)
                    || gem.definition.to_resolved().as_ref() != Some(definition)
                {
                    return Ok(None);
                }
                primary = Some((*source, skill.id));
            }
            (
                InputOutput::Support { support, gem },
                OwnedGemRole::Known(AuthoredGemRole::SupportAssignment),
            ) => {
                let DraftSkillTarget::Authored(DraftField::Known { value: target }) =
                    &support.target
                else {
                    return Ok(None);
                };
                if support.support.to_resolved() != Some(gem.id)
                    || gem.definition.to_resolved().as_ref() != Some(definition)
                {
                    return Ok(None);
                }
                b.charge(1)?;
                targets.push(*target);
            }
            _ => return Ok(None),
        }
    }
    let Some((source, skill)) = primary else {
        return Ok(None);
    };
    b.charge(targets.len())?;
    if targets.iter().any(|target| *target != skill) {
        return Ok(None);
    }
    Ok(Some(GroupSelectors {
        source,
        main,
        calcs,
    }))
}
