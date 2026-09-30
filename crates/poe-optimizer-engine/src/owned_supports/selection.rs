//! Retain ordered selection independently from any target's type admission.
use super::*;

/// Immutable component snapshot of a single ordered selection. It preserves
/// source scalar inputs and duplicate retained positions, but grants no build
/// coverage or receiver authority. It is deliberately not deserializable.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectedSupports {
    preparation: OwnedContentDigest,
    origins: Vec<ResolvedSupportOrigin>,
    disabled_origins: Vec<usize>,
    selected_origin_indices: Vec<usize>,
}
impl SelectedSupports {
    pub fn preparation(&self) -> OwnedContentDigest {
        self.preparation
    }
    pub fn origins(&self) -> &[ResolvedSupportOrigin] {
        &self.origins
    }
    pub fn disabled_origins(&self) -> &[usize] {
        &self.disabled_origins
    }
    /// Indices into `origins`, in retained-list order. Repeated indices are
    /// meaningful distinct positions and must not be deduplicated.
    pub fn selected_origin_indices(&self) -> &[usize] {
        &self.selected_origin_indices
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SupportSelectionOutcome {
    Known(SelectedSupports),
    Unresolved {
        reason: SupportPreparationGap,
        origin_index: Option<usize>,
    },
}
fn selection_unresolved(
    reason: SupportPreparationGap,
    origin_index: Option<usize>,
) -> SupportSelectionOutcome {
    SupportSelectionOutcome::Unresolved {
        reason,
        origin_index,
    }
}

/// Select once from resolved ordered origins, independently from target
/// eligibility. Reuse the returned snapshot when preparing inherited targets.
pub fn select_supports(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    limits: SupportPreparationLimits,
) -> Result<SupportSelectionOutcome> {
    let mut work = limits.max_work;
    select_supports_with_budget(package, origins, limits, &mut work)
}

pub fn select_supports_with_budget(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    limits: SupportPreparationLimits,
    remaining_work: &mut usize,
) -> Result<SupportSelectionOutcome> {
    with_budget(limits, remaining_work, |budget| {
        validate_origin_count(origins, budget)?;
        validate_origins(package, origins, budget)?;
        select_validated(package, origins, budget)
    })
}

/// Prepare a target from the exact retained selection, without consulting new
/// origin values or rerunning replacement. Summoner facts are explicit inputs;
/// callers supply the parent's post-preparation context for inherited skills.
pub fn prepare_selected_supports(
    package: &OwnedSupportPreparation,
    selected: &SelectedSupports,
    target: &SupportPreparationTarget,
    limits: SupportPreparationLimits,
) -> Result<SupportPreparationOutcome> {
    let mut work = limits.max_work;
    prepare_selected_supports_with_budget(package, selected, target, limits, &mut work)
}

pub fn prepare_selected_supports_with_budget(
    package: &OwnedSupportPreparation,
    selected: &SelectedSupports,
    target: &SupportPreparationTarget,
    limits: SupportPreparationLimits,
    remaining_work: &mut usize,
) -> Result<SupportPreparationOutcome> {
    with_budget(limits, remaining_work, |budget| {
        if selected.preparation != *package.identity() {
            return Err(SupportPreparationError::Invalid(
                "selected supports belong to a different preparation package",
            ));
        }
        validate_origin_count(&selected.origins, budget)?;
        validate_target_address(package, target, budget)?;
        if let Some(outcome) = target_activity(target) {
            return Ok(outcome);
        }
        let types = target_types(package, target, budget)?;
        admit_selected(package, selected, target, types, budget)
    })
}

pub(super) fn select_validated(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    budget: &mut Budget,
) -> Result<SupportSelectionOutcome> {
    let limits = budget.limits;
    match package.input().policy {
        SupportPreparationPolicy::OrderedReplacementRetryFrontierV1 => {}
    }
    // The definition reference vector is private to this call. No mutable
    // eligibility/selection state is retained on shared package definitions.
    budget.charge(origins.len())?;
    let mut definitions = Vec::with_capacity(origins.len());
    let mut disabled = Vec::new();
    for (index, origin) in origins.iter().enumerate() {
        match origin.enabled {
            Some(false) => {
                budget.charge(1)?;
                disabled.push(index);
                definitions.push(None);
                continue;
            }
            None => {
                return Ok(selection_unresolved(
                    SupportPreparationGap::OriginEnabled,
                    Some(index),
                ));
            }
            Some(true) => {}
        }
        budget.charge(package.input().supports.len() + 1)?;
        match package.preparation_for(&origin.gem) {
            Some(SchemaState::Known(definition)) => definitions.push(Some(definition)),
            Some(SchemaState::Unmapped { .. }) => {
                return Ok(selection_unresolved(
                    SupportPreparationGap::UnmappedDefinition,
                    Some(index),
                ));
            }
            None => {
                return Ok(selection_unresolved(
                    SupportPreparationGap::MissingDefinition,
                    Some(index),
                ));
            }
        }
    }
    let mut selected: Vec<usize> = Vec::new();
    for (incoming, definition) in definitions.iter().enumerate() {
        let Some(definition) = definition else {
            continue;
        };
        let mut add = true;
        for previous in &mut selected {
            budget.charge(1)?;
            let other = definitions[*previous].expect("only enabled origins are selected");
            if definition.effect == other.effect {
                add = false;
                let Some(level) = origins[incoming].effective_level else {
                    return Ok(selection_unresolved(
                        SupportPreparationGap::EffectiveLevel,
                        Some(incoming),
                    ));
                };
                let Some(other_level) = origins[*previous].effective_level else {
                    return Ok(selection_unresolved(
                        SupportPreparationGap::EffectiveLevel,
                        Some(*previous),
                    ));
                };
                let replace = if level > other_level {
                    true
                } else if level < other_level {
                    false
                } else {
                    let Some(quality) = &origins[incoming].effective_quality else {
                        return Ok(selection_unresolved(
                            SupportPreparationGap::EffectiveQuality,
                            Some(incoming),
                        ));
                    };
                    let Some(other_quality) = &origins[*previous].effective_quality else {
                        return Ok(selection_unresolved(
                            SupportPreparationGap::EffectiveQuality,
                            Some(*previous),
                        ));
                    };
                    quality.value() > other_quality.value()
                };
                if replace {
                    *previous = incoming;
                }
                break;
            } else if let (Some(families), Some(other_families)) =
                (&definition.families, &other.families)
            {
                budget.charge(families.len())?;
                for family in families {
                    for other_family in other_families {
                        budget.charge(1)?;
                        if family == other_family {
                            add = false;
                            *previous = incoming;
                            break;
                        }
                    }
                }
            } else if definition.plus_version_of.as_ref() == Some(&other.effect) {
                add = false;
                *previous = incoming;
            } else if other.plus_version_of.as_ref() == Some(&definition.effect) {
                add = false;
            }
        }
        if add {
            budget.charge(1)?;
            if selected.len() >= limits.max_origins {
                return Err(SupportPreparationError::Limit("selected positions"));
            }
            selected.push(incoming);
        }
    }
    // Charge before retaining any caller-owned data. Snapshot scalars cannot be
    // changed by subsequent caller mutation or by a child target's inputs.
    budget.charge(origins.len() + selected.len() + disabled.len())?;
    Ok(SupportSelectionOutcome::Known(SelectedSupports {
        preparation: *package.identity(),
        origins: origins.to_vec(),
        disabled_origins: disabled,
        selected_origin_indices: selected,
    }))
}
