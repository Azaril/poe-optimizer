//! Bind the explicit authored order to preparation facts, without source/UI discovery.
use super::*;
use poe_optimizer_core::{
    build_identity::GemInstanceId,
    owned_build::{BuildSpec, LoadoutScope, OwnedInputLimits, SupportOrigin},
    owned_definitions::GameVersionNamespace,
};
use std::collections::BTreeMap;

/// Resolved numerical dependencies for one existing assignment. These values
/// cannot replace its Gem definition, target, enabled state, or selection order.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectiveSupportValues {
    pub assignment: SupportAssignmentId,
    pub effective_level: Option<BoundedInteger>,
    pub effective_quality: Option<FiniteQuantity>,
}

#[derive(Clone, Debug)]
struct IndexedAssignment {
    assignment: SupportAssignmentId,
    gem: usize,
    target: usize,
    enabled: bool,
}
#[derive(Clone, Debug)]
struct IndexedTarget {
    ordered: Vec<usize>,
    authored_active: Option<bool>,
}

/// Immutable support-only snapshot of a validated build. Physical Gem level and
/// quality are never copied. Its compiled owner must bind the exact request
/// identity separately; this component grants no coverage or generated activity.
///
/// Total rows retain BuildSpec's hard entry bound. `max_origins` remains a
/// per-target limit, while the work budget also bounds cold indexing.
#[derive(Clone, Debug)]
pub struct SupportBuildIndex {
    namespace: GameVersionNamespace,
    has_origin_order: bool,
    gems: Vec<GemDefId>,
    assignments: Vec<IndexedAssignment>,
    assignment_lookup: BTreeMap<SupportAssignmentId, usize>,
    targets: Vec<IndexedTarget>,
    target_lookup: BTreeMap<SkillTarget, usize>,
    max_target_depth: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SupportBuildSelectionOutcome {
    Known(SelectedSupports),
    Inactive {
        target: SkillTarget,
    },
    Unresolved {
        reason: SupportPreparationGap,
        origin_index: Option<usize>,
    },
}

enum BoundOrigins {
    Known(Vec<ResolvedSupportOrigin>),
    Inactive,
    MissingOrder,
}

fn depth(target: &SkillTarget) -> usize {
    match target {
        SkillTarget::Authored(_) => 0,
        SkillTarget::Generated(skill) => skill.provider.grant_path.len(),
    }
}

// Conservative comparison allowance for a bounded BTreeMap. Path-key lookups
// additionally charge their maximum indexed/requested depth.
fn lookup_work(len: usize) -> usize {
    16 * (1 + (len + 1).ilog2() as usize)
}

impl SupportBuildIndex {
    /// Driver-only authored origin inventory. Missing order remains distinct from
    /// a present empty sequence; this exposes no computed-value authority.
    pub(crate) fn ordered_assignments(
        &self,
        target: &SkillTarget,
    ) -> Option<impl Iterator<Item = SupportAssignmentId> + '_> {
        if !self.has_origin_order {
            return None;
        }
        let target = self
            .target_lookup
            .get(target)
            .and_then(|index| self.targets.get(*index))?;
        Some(
            target
                .ordered
                .iter()
                .map(|row| self.assignments[*row].assignment),
        )
    }
    pub fn new(build: &BuildSpec, limits: SupportPreparationLimits) -> Result<Self> {
        let mut work = limits.max_work;
        Self::new_with_budget(build, limits, &mut work)
    }

    pub fn new_with_budget(
        build: &BuildSpec,
        limits: SupportPreparationLimits,
        remaining_work: &mut usize,
    ) -> Result<Self> {
        with_budget(limits, remaining_work, |budget| {
            Self::new_inner(build, budget)
        })
    }

    fn new_inner(build: &BuildSpec, budget: &mut Budget) -> Result<Self> {
        let input = build.input();
        let sequences = input.support_origins.as_deref().unwrap_or_default();
        let total = [
            input.gems.len(),
            input.supports.len(),
            input.skills.len(),
            sequences.len(),
        ]
        .into_iter()
        .try_fold(0usize, |total, len| total.checked_add(len))
        .ok_or(SupportPreparationError::Limit("index rows"))?;
        if total > OwnedInputLimits::default().max_entries {
            return Err(SupportPreparationError::Limit("index rows"));
        }
        budget.charge(total)?;
        let mut max_target_depth = 0;
        for target in input
            .supports
            .iter()
            .map(|row| &row.target)
            .chain(sequences.iter().map(|row| &row.target))
        {
            let target_depth = depth(target);
            if target_depth > budget.limits.max_target_depth {
                return Err(SupportPreparationError::Limit("target depth"));
            }
            budget.charge(target_depth + 1)?;
            max_target_depth = max_target_depth.max(target_depth);
        }
        for sequence in sequences {
            if sequence.origins.len() > budget.limits.max_origins {
                return Err(SupportPreparationError::Limit("origins"));
            }
            budget.charge(sequence.origins.len())?;
        }
        // All input collection sizes/depths are checked before secondary indexes
        // allocate or clone source keys. Per-row work precedes each insertion.
        let mut result = Self {
            namespace: input.game_version.clone(),
            has_origin_order: input.support_origins.is_some(),
            gems: Vec::with_capacity(input.gems.len()),
            assignments: Vec::with_capacity(input.supports.len()),
            assignment_lookup: BTreeMap::new(),
            targets: Vec::new(),
            target_lookup: BTreeMap::new(),
            max_target_depth,
        };
        let mut gems: BTreeMap<GemInstanceId, usize> = BTreeMap::new();
        for gem in &input.gems {
            budget.charge(lookup_work(gems.len()))?;
            gems.insert(gem.id, result.gems.len());
            result.gems.push(gem.definition.clone());
        }
        for skill in &input.skills {
            let target = result.insert_target(&SkillTarget::Authored(skill.id), budget)?;
            let in_loadout = match &skill.scope {
                LoadoutScope::Shared => true,
                LoadoutScope::Selected { loadouts } => {
                    budget.charge(loadouts.len())?;
                    loadouts.contains(&input.active_weapon_loadout)
                }
            };
            result.targets[target].authored_active = Some(skill.enabled && in_loadout);
        }
        for assignment in &input.supports {
            let target = result.insert_target(&assignment.target, budget)?;
            budget.charge(lookup_work(gems.len()))?;
            let gem = *gems
                .get(&assignment.support)
                .ok_or(SupportPreparationError::Invalid(
                    "support assignment has no build gem",
                ))?;
            budget.charge(lookup_work(result.assignment_lookup.len()))?;
            result
                .assignment_lookup
                .insert(assignment.id, result.assignments.len());
            result.assignments.push(IndexedAssignment {
                assignment: assignment.id,
                gem,
                target,
                enabled: assignment.enabled,
            });
        }
        for sequence in sequences {
            let target = result.insert_target(&sequence.target, budget)?;
            let mut ordered = Vec::with_capacity(sequence.origins.len());
            for origin in &sequence.origins {
                let SupportOrigin::Assignment(id) = origin;
                budget.charge(lookup_work(result.assignment_lookup.len()))?;
                let row =
                    *result
                        .assignment_lookup
                        .get(id)
                        .ok_or(SupportPreparationError::Invalid(
                            "support order has no build assignment",
                        ))?;
                if result.assignments[row].target != target {
                    return Err(SupportPreparationError::Invalid(
                        "support order crosses its target",
                    ));
                }
                ordered.push(row);
            }
            result.targets[target].ordered = ordered;
        }
        Ok(result)
    }

    fn insert_target(&mut self, target: &SkillTarget, budget: &mut Budget) -> Result<usize> {
        budget.charge(lookup_work(self.target_lookup.len()) * (self.max_target_depth + 1))?;
        if let Some(index) = self.target_lookup.get(target) {
            return Ok(*index);
        }
        budget.charge(depth(target) + 1)?;
        let index = self.targets.len();
        self.target_lookup.insert(target.clone(), index);
        self.targets.push(IndexedTarget {
            ordered: Vec::new(),
            authored_active: None,
        });
        Ok(index)
    }

    /// Authored and explicit support targets, not discovery of generated skills.
    pub fn targets(&self) -> impl Iterator<Item = &SkillTarget> {
        self.target_lookup.keys()
    }

    pub fn has_origin_order(&self) -> bool {
        self.has_origin_order
    }

    /// `None` for absent or generated targets is not evidence of activation.
    pub fn authored_activity(&self, target: &SkillTarget) -> Option<bool> {
        self.target_lookup
            .get(target)
            .and_then(|index| self.targets[*index].authored_active)
    }

    pub fn select_for_target(
        &self,
        package: &OwnedSupportPreparation,
        target: &SkillTarget,
        values: &[EffectiveSupportValues],
        limits: SupportPreparationLimits,
    ) -> Result<SupportBuildSelectionOutcome> {
        let mut work = limits.max_work;
        self.select_for_target_with_budget(package, target, values, limits, &mut work)
    }

    pub fn select_for_target_with_budget(
        &self,
        package: &OwnedSupportPreparation,
        target: &SkillTarget,
        values: &[EffectiveSupportValues],
        limits: SupportPreparationLimits,
        remaining_work: &mut usize,
    ) -> Result<SupportBuildSelectionOutcome> {
        with_budget(limits, remaining_work, |budget| {
            match self.bind_origins(package, target, values, budget)? {
                BoundOrigins::MissingOrder => Ok(SupportBuildSelectionOutcome::Unresolved {
                    reason: SupportPreparationGap::OriginOrder,
                    origin_index: None,
                }),
                BoundOrigins::Inactive => Ok(SupportBuildSelectionOutcome::Inactive {
                    target: target.clone(),
                }),
                BoundOrigins::Known(origins) => {
                    validate_origin_count(&origins, budget)?;
                    validate_origins(package, &origins, budget)?;
                    if let SkillTarget::Generated(skill) = target {
                        budget.charge(skill.provider.grant_path.len() + 1)?;
                        if skill.slot.slot.namespace() != &self.namespace
                            || skill.slot.declaration.namespace() != &self.namespace
                            || skill.provider.grant_path.iter().any(|slot| {
                                slot.slot.namespace() != &self.namespace
                                    || slot.declaration.namespace() != &self.namespace
                            })
                        {
                            return Err(SupportPreparationError::Invalid(
                                "foreign support target namespace",
                            ));
                        }
                    }
                    Ok(
                        match selection::select_validated(package, &origins, budget)? {
                            SupportSelectionOutcome::Known(selected) => {
                                SupportBuildSelectionOutcome::Known(selected)
                            }
                            SupportSelectionOutcome::Unresolved {
                                reason,
                                origin_index,
                            } => SupportBuildSelectionOutcome::Unresolved {
                                reason,
                                origin_index,
                            },
                        },
                    )
                }
            }
        })
    }
}

/// Prepare the support origins declared for this exact target in a validated
/// build. Generated activation and receiving facts remain explicit caller input.
/// Missing effective values are unresolved only when the policy demands them.
pub fn prepare_build_supports(
    package: &OwnedSupportPreparation,
    build: &BuildSpec,
    target: &SupportPreparationTarget,
    values: &[EffectiveSupportValues],
    limits: SupportPreparationLimits,
) -> Result<SupportPreparationOutcome> {
    let mut work = limits.max_work;
    prepare_build_supports_with_budget(package, build, target, values, limits, &mut work)
}

/// Bind and prepare one build target under the caller's shared attempt budget.
/// Binding, ordered selection and type preparation consume the same allowance,
/// including failed or unresolved attempts. Values remain component inputs,
/// not proof of a complete effective-value producer or of build coverage.
pub fn prepare_build_supports_with_budget(
    package: &OwnedSupportPreparation,
    build: &BuildSpec,
    target: &SupportPreparationTarget,
    values: &[EffectiveSupportValues],
    limits: SupportPreparationLimits,
    remaining_work: &mut usize,
) -> Result<SupportPreparationOutcome> {
    with_budget(limits, remaining_work, |budget| {
        prepare_build_supports_inner(package, build, target, values, budget)
    })
}

fn prepare_build_supports_inner(
    package: &OwnedSupportPreparation,
    build: &BuildSpec,
    target: &SupportPreparationTarget,
    values: &[EffectiveSupportValues],
    budget: &mut Budget,
) -> Result<SupportPreparationOutcome> {
    if build.input().game_version != package.input().namespace {
        return Err(SupportPreparationError::Invalid(
            "build and support preparation namespace differ",
        ));
    }
    if depth(&target.target) > budget.limits.max_target_depth {
        return Err(SupportPreparationError::Limit("target depth"));
    }
    // Preserve the existing cheap missing-order outcome, including its budget
    // boundary, before doing optional cold indexing that cannot resolve it.
    if build.input().support_origins.is_none() {
        return Ok(unresolved(SupportPreparationGap::OriginOrder, None));
    }
    let index = SupportBuildIndex::new_inner(build, budget)?;
    match index.bind_origins(package, &target.target, values, budget)? {
        BoundOrigins::MissingOrder => Ok(unresolved(SupportPreparationGap::OriginOrder, None)),
        BoundOrigins::Inactive => Ok(SupportPreparationOutcome::Inactive {
            target: target.target.clone(),
        }),
        BoundOrigins::Known(origins) => prepare_supports_inner(package, &origins, target, budget),
    }
}

impl SupportBuildIndex {
    fn bind_origins(
        &self,
        package: &OwnedSupportPreparation,
        target: &SkillTarget,
        values: &[EffectiveSupportValues],
        budget: &mut Budget,
    ) -> Result<BoundOrigins> {
        if self.namespace != package.input().namespace {
            return Err(SupportPreparationError::Invalid(
                "build and support preparation namespace differ",
            ));
        }
        let target_depth = depth(target);
        if target_depth > budget.limits.max_target_depth {
            return Err(SupportPreparationError::Limit("target depth"));
        }
        if !self.has_origin_order {
            return Ok(BoundOrigins::MissingOrder);
        }
        budget.charge(
            lookup_work(self.target_lookup.len()) * (self.max_target_depth.max(target_depth) + 1),
        )?;
        let target_index = self.target_lookup.get(target).copied();
        let indexed = target_index.map(|index| &self.targets[index]);
        let ordered = indexed.map_or(&[][..], |row| row.ordered.as_slice());
        if ordered.len() > budget.limits.max_origins || values.len() > budget.limits.max_origins {
            return Err(SupportPreparationError::Limit("origins"));
        }
        budget.charge(values.len())?;
        let mut effective = BTreeMap::new();
        for value in values {
            budget.charge(lookup_work(self.assignment_lookup.len()))?;
            let assignment = self
                .assignment_lookup
                .get(&value.assignment)
                .map(|index| &self.assignments[*index])
                .ok_or(SupportPreparationError::Invalid(
                    "effective support input has no build assignment",
                ))?;
            if Some(assignment.target) != target_index {
                return Err(SupportPreparationError::Invalid(
                    "effective support input belongs to another target",
                ));
            }
            budget.charge(lookup_work(effective.len()))?;
            if effective.insert(value.assignment, value).is_some() {
                return Err(SupportPreparationError::Invalid(
                    "duplicate effective support input",
                ));
            }
        }
        budget.charge(ordered.len())?;
        let mut origins = Vec::with_capacity(ordered.len());
        for row in ordered {
            let assignment = &self.assignments[*row];
            budget.charge(lookup_work(effective.len()) + 1)?;
            let scalar = effective.get(&assignment.assignment);
            origins.push(ResolvedSupportOrigin {
                assignment: assignment.assignment,
                gem: self.gems[assignment.gem].clone(),
                enabled: Some(assignment.enabled),
                effective_level: scalar.and_then(|value| value.effective_level),
                effective_quality: scalar.and_then(|value| value.effective_quality.clone()),
            });
        }
        // Check scalar bindings before inactivity, as the original adapter did.
        if matches!(target, SkillTarget::Authored(_)) {
            let active = indexed.and_then(|row| row.authored_active).ok_or(
                SupportPreparationError::Invalid("preparation target has no authored build skill"),
            )?;
            if !active {
                return Ok(BoundOrigins::Inactive);
            }
        }
        Ok(BoundOrigins::Known(origins))
    }
}
