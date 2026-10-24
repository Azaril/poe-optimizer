//! Private group selection proof, separate from admission and native authority.
use super::*;

pub(super) struct GroupSelectors {
    source: SourceOccurrenceId,
    main: bool,
    calcs: bool,
    secondary_unconsumed: bool,
}

impl GroupSelectors {
    pub(super) fn proves(
        &self,
        receipt: &InputReceipt,
        field: &UsageInventoryFieldSelector,
    ) -> bool {
        receipt.source == self.source
            && (matches!(field,
                UsageInventoryFieldSelector::GroupAttribute { name }
                if name == "mainActiveSkill" && self.main
                    || name == "mainActiveSkillCalcs" && self.calcs)
                || matches!(field,
                    UsageInventoryFieldSelector::OccurrenceAttribute { name }
                    if name == "enableGlobal2" && self.secondary_unconsumed))
    }
}

fn exact_neutral(group: &SourceEvidenceRow<'_>, name: &str) -> bool {
    group.attribute(name).is_some_and(|attribute| {
        matches!(attribute.raw(), "nil" | "1") && attribute.decoded().ok() == Some(attribute.raw())
    })
}

/// Generated reconstruction uses the reverse-primary catalog route, not just
/// the saved Gem tuple. Only a complete singleton on that exact unique route
/// makes its secondary switch unread and nil/1 selectors choose the primary or
/// nothing. The live generated receipt separately proves the actual provider,
/// raw inputs and usage outputs; no fact here proves admission or count.
fn generated_singleton(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    preset: &SkillPresetDraft,
    group: &SourceEvidenceRow<'_>,
    receipts: &InventoryReceipts,
) -> Result<Option<GroupSelectors>> {
    b.charge(group.children().len().saturating_add(receipts.inputs.len()))?;
    let [source] = group.children() else {
        return Ok(None);
    };
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
    let InputOutput::Generated(proof) = &receipt.output else {
        return Ok(None);
    };
    if !matching_input_present(b, draft, preset, receipt)? {
        return Ok(None);
    }
    // record_generated_input_disposition creates the immutable source identity
    // from this same checked proof; do not reconstruct it from an origin label.
    let correspondence = proof.correspondence();
    let row = &b.evidence.rows()[source.ordinal() as usize];
    let roles = b.roles;
    let Some(role) =
        skill_input_disposition::charged_source_role(b, roles, row, &correspondence.gem)?
    else {
        return Ok(None);
    };
    if role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
        || matches!(
            role.materialization,
            OwnedGemMaterialization::Unmapped { .. }
        )
        || !skill_input_disposition::source_is_singleton(row, &correspondence.gem, role)
    {
        return Ok(None);
    }
    let OwnedGemSourceConstruction::Observed(facts) = &role.construction else {
        return Ok(None);
    };
    let OwnedReversePrimaryCensus::Observed { routes } = &roles.input().reverse_primary else {
        return Ok(None);
    };
    // The checked role index uses binary lookup. Charge an upper bound on its
    // string comparisons and the exact candidate join before inspecting either.
    b.charge(
        ((usize::BITS - routes.len().saturating_add(1).leading_zeros()) as usize)
            .saturating_mul(correspondence.skill_id.len().saturating_add(1)),
    )?;
    let OwnedReversePrimaryLookup::Observed(route) =
        roles.reverse_primary(&correspondence.skill_id, &correspondence.skill)
    else {
        return Ok(None);
    };
    b.charge(
        route
            .candidates
            .len()
            .saturating_add(facts.catalog_gem_key.len())
            .saturating_add(correspondence.gem.key().as_str().len())
            .saturating_add(1),
    )?;
    let [candidate] = route.candidates.as_slice() else {
        return Ok(None);
    };
    if candidate.gem != correspondence.gem || candidate.catalog_gem_key != facts.catalog_gem_key {
        return Ok(None);
    }
    let secondary_unconsumed = row.attribute("enableGlobal2").is_some_and(|attribute| {
        matches!(attribute.raw(), "nil" | "false" | "true")
            && attribute.decoded().ok() == Some(attribute.raw())
    });
    Ok(Some(GroupSelectors {
        source: *source,
        main: exact_neutral(group, "mainActiveSkill"),
        calcs: exact_neutral(group, "mainActiveSkillCalcs"),
        secondary_unconsumed,
    }))
}

/// Source-reference selection may retain missing additional references without
/// treating the missing mechanic as absent. The authenticated constructor must
/// have produced exactly the mapped primary. Every added reference must remain
/// explicitly Missing: a mapped addition omitted from the final list is a
/// contradiction, even if the role index accepts the individual references.
///
/// This is private SkillUse selector authority only. Support siblings and other
/// consumers retain physical_source_is_singleton. The caller has charged source
/// identity comparisons; charge every additional reference before inspecting it.
/// Catalog reproduction authenticates these observations at publication, while
/// this attempt still checks the actual physical source and current receipts.
fn constructed_primary_selection(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    gem: &GemDefId,
    role: &OwnedGemRoleRow,
) -> Result<bool> {
    if &role.gem != gem
        || role.materialization != OwnedGemMaterialization::Physical
        || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
    {
        return Ok(false);
    }
    let (OwnedPrimarySkill::Known(primary), OwnedGemSourceConstruction::Observed(facts)) =
        (&role.primary, &role.construction)
    else {
        return Ok(false);
    };
    b.charge(
        facts
            .declared_additional_effects
            .len()
            .saturating_add(facts.constructed_additional_effects.len()),
    )?;
    Ok(
        source_shape::value(row, "gemId") == Some(facts.game_id.as_str())
            && source_shape::value(row, "variantId") == Some(facts.variant_id.as_str())
            && source_shape::value(row, "skillId") == Some(facts.primary.effect_id.as_str())
            && matches!(&facts.primary.resolution,
            OwnedSourceEffectResolution::Mapped(resolved) if resolved == primary)
            && facts.resolved_additional_effects.is_empty()
            && facts.resolved_effects.as_slice() == std::slice::from_ref(&facts.primary)
            && facts
                .display_order
                .as_deref()
                .is_none_or(|order| order == [1])
            && facts.declared_additional_effects.iter().all(|reference| {
                reference.effect.resolution == OwnedSourceEffectResolution::Missing
            })
            && facts
                .constructed_additional_effects
                .iter()
                .all(|reference| {
                    reference.effect.resolution == OwnedSourceEffectResolution::Missing
                }),
    )
}

/// A reviewed manual Direct row uses the exact saved catalog tuple, not the
/// generated reverse-catalog route. The full loader clears an initially hidden
/// primary's entire source list. With the retained primary first, literal
/// enabled/global1 true and no late hidden/list writers in the pinned dispatch,
/// nil/1 selects that primary or the empty list; it cannot promote a Command.
/// This is reference-field accounting, not proof that a Skill is admitted.
///
/// All observed additional/missing references remain intact. No singleton or
/// native completeness is inferred, and unavailable observations cannot prove
/// a prefix. Exact source-name/effect comparisons were charged by role lookup.
fn constructed_direct_prefix(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    gem: &GemDefId,
    skill: &SkillDefId,
    role: &OwnedGemRoleRow,
) -> Result<bool> {
    // Charge the extra Direct identity and literal-gate checks before reading
    // them. The immutable role package already validated every list entry;
    // this predicate reads only the first retained constructed/display entry.
    b.charge(
        skill
            .namespace()
            .game()
            .as_str()
            .len()
            .saturating_add(skill.namespace().version().as_str().len())
            .saturating_add(skill.key().as_str().len())
            .saturating_mul(4)
            .saturating_add(row.attributes().len().saturating_mul(24))
            .saturating_add(4),
    )?;
    if &role.gem != gem
        || role.materialization != OwnedGemMaterialization::ProviderOnly
        || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
        || !matches!(&role.primary, OwnedPrimarySkill::Known(primary) if primary == skill)
    {
        return Ok(false);
    }
    let OwnedGemSourceConstruction::Observed(facts) = &role.construction else {
        return Ok(false);
    };
    let exact_true = |name| {
        row.attribute(name)
            .is_some_and(|a| a.raw() == "true" && a.decoded().ok() == Some("true"))
    };
    Ok(
        source_shape::value(row, "gemId") == Some(facts.game_id.as_str())
            && source_shape::value(row, "variantId") == Some(facts.variant_id.as_str())
            && source_shape::value(row, "skillId") == Some(facts.primary.effect_id.as_str())
            && matches!(&facts.primary.resolution,
            OwnedSourceEffectResolution::Mapped(resolved) if resolved == skill)
            && facts.resolved_effects.first() == Some(&facts.primary)
            && facts
                .display_order
                .as_deref()
                .is_none_or(|order| order.first() == Some(&1))
            && exact_true("enabled")
            && exact_true("enableGlobal1"),
    )
}

/// Same-attempt final accounting: every child must have a live checked
/// intrinsic/reference receipt. One physical SkillUse uses the existing
/// constructed-primary proof; one authored Direct uses its reviewed first-effect
/// prefix. All other children must be exact physical support assignments to that
/// same occurrence. Neither path proves admission or execution.
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
    if group.attribute("source").is_some() {
        return generated_singleton(b, draft, preset, group, receipts);
    }
    if !main && !calcs {
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
        let definition = match &receipt.identity {
            UsageInventorySource::Physical { gem } => gem,
            UsageInventorySource::Occurrence { target } => match target.as_ref() {
                OccurrenceUsageTarget::AuthoredDirect { gem, .. } => gem,
                _ => return Ok(None),
            },
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
        match (&receipt.output, &role.role) {
            (
                InputOutput::Authored {
                    skill,
                    gem: Some(gem),
                    ..
                },
                OwnedGemRole::Known(AuthoredGemRole::SkillUse),
            ) => {
                if !matches!(receipt.identity, UsageInventorySource::Physical { .. })
                    || !constructed_primary_selection(b, row, definition, role)?
                    || primary.is_some()
                    || !matches!(&skill.source,
                        DraftAuthoredSkillSource::Gem(DraftField::Known { value }) if *value == gem.id)
                    || gem.definition.to_resolved().as_ref() != Some(definition)
                {
                    return Ok(None);
                }
                primary = Some((*source, skill.id));
            }
            (
                InputOutput::Authored {
                    skill,
                    gem: None,
                    scope: AuthoredInputScope::ReferenceDisposition,
                },
                OwnedGemRole::Known(AuthoredGemRole::SkillUse),
            ) => {
                let UsageInventorySource::Occurrence { target } = &receipt.identity else {
                    return Ok(None);
                };
                let OccurrenceUsageTarget::AuthoredDirect {
                    skill: expected, ..
                } = target.as_ref()
                else {
                    return Ok(None);
                };
                if primary.is_some()
                    || !constructed_direct_prefix(b, row, definition, expected, role)?
                    || !matches!(&skill.source,
                        DraftAuthoredSkillSource::Direct(DraftField::Known { value }) if value == expected.as_ref())
                {
                    return Ok(None);
                }
                primary = Some((*source, skill.id));
            }
            (
                InputOutput::Support { support, gem },
                OwnedGemRole::Known(AuthoredGemRole::SupportAssignment),
            ) => {
                if !matches!(receipt.identity, UsageInventorySource::Physical { .. })
                    || !skill_input_disposition::physical_source_is_singleton(row, definition, role)
                {
                    return Ok(None);
                }
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
        secondary_unconsumed: false,
    }))
}
