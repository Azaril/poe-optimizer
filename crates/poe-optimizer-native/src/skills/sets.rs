//! Borrowed reads of skill-set rows retained at their actual Load publication.
//! Startup constructor/display/Sync state is explicitly not produced here.
use super::*;
use poe_optimizer_import::selected_view::{NumericValue, SelectionDomain, SetOrigin};

pub(super) struct ProducedSkillSet {
    key: f64,
    title: Option<String>,
    origin: SetOrigin,
    groups: Vec<usize>,
}
pub(super) struct SkillSets {
    rows: Vec<ProducedSkillSet>,
    pub(super) keys: NumericSetKeys,
    pub(super) winners: BTreeMap<u64, usize>,
    pub(super) order: Vec<f64>,
    source: SourceOccurrenceId,
    active_key: f64,
    // The source socketGroupList alias survives Load's active-ID/map reset.
    active_row: Option<usize>,
}
impl Machine<'_> {
    pub(super) fn reset_skill_sets(
        &mut self,
        source: SourceOccurrenceId,
    ) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_fields, 5, "skill-set state")?;
        if let Some(sets) = &mut self.sets {
            sets.keys = NumericSetKeys::new(32768);
            sets.winners.clear();
            sets.order.clear();
            sets.source = source;
            sets.active_key = 0.0;
        } else {
            self.sets = Some(SkillSets {
                rows: vec![],
                keys: NumericSetKeys::new(32768),
                winners: BTreeMap::new(),
                order: vec![],
                source,
                active_key: 0.0,
                active_row: None,
            });
        }
        Ok(())
    }
    pub(super) fn create_skill_set(
        &mut self,
        id: f64,
        origin: SetOrigin,
    ) -> Result<usize, EvaluationError> {
        // Existing field budget also bounds new row/map/list metadata. No graph
        // snapshot is imported, and source groups remain in their existing owner.
        Self::spend(&mut self.limits.max_fields, 6, "skill-set metadata")?;
        let sets = self.sets.as_mut().expect("Load entered");
        sets.rows
            .try_reserve(1)
            .map_err(|_| resource("skill-set capacity"))?;
        sets.keys
            .insert(id)
            .map_err(|_| resource("skill set keys"))?;
        let row = sets.rows.len();
        sets.rows.push(ProducedSkillSet {
            key: id,
            title: None,
            origin,
            groups: vec![],
        });
        sets.winners.insert(key(id), row);
        Ok(row)
    }
    pub(super) fn skill_set_title(&mut self, row: usize, title: Option<String>) {
        self.sets.as_mut().expect("Load entered").rows[row].title = title;
    }
    pub(super) fn append_skill_set_order(
        &mut self,
        id: f64,
        ci: usize,
    ) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_fields, 2, "skill-set order")?;
        let sets = self.sets.as_mut().expect("Load entered");
        sets.order
            .try_reserve(1)
            .map_err(|_| resource("skill-set order capacity"))?;
        self.report.containers[ci]
            .order
            .try_reserve(1)
            .map_err(|_| resource("skill-set report capacity"))?;
        sets.order.push(id);
        self.report.containers[ci].order.push(SkillNumber(id));
        Ok(())
    }
    pub(super) fn attach_skill_set_group(
        &mut self,
        row: usize,
        group: usize,
    ) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_fields, 1, "skill-set group reference")?;
        let groups = &mut self.sets.as_mut().expect("Load entered").rows[row].groups;
        groups
            .try_reserve(1)
            .map_err(|_| resource("skill-set group capacity"))?;
        groups.push(group);
        self.report.groups[group].attached = true;
        Ok(())
    }
    pub(super) fn select_skill_set(&mut self, chosen: f64) -> Option<usize> {
        let sets = self.sets.as_mut().expect("Load entered");
        sets.active_key = chosen;
        sets.active_row = sets.winners.get(&key(chosen)).copied();
        sets.active_row
            .and_then(|row| sets.rows[row].groups.first().copied())
    }
    pub(super) fn authored_skill_set_origin(
        &self,
        node: Node<'_, '_>,
    ) -> Result<SetOrigin, EvaluationError> {
        let instance = self.instance(node)?;
        if !matches!(instance, AuthoredInstanceId::SkillSet(_)) {
            return Err(contract(
                "skill-set source is bound to another instance kind",
            ));
        }
        Ok(SetOrigin::Authored {
            instance,
            source: self.source(node)?,
        })
    }
    pub(super) fn default_skill_set_origin(&self) -> SetOrigin {
        SetOrigin::Default {
            domain: SelectionDomain::Skills,
            container: Some(self.sets.as_ref().expect("Load entered").source),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SkillSetReadLimits {
    pub max_steps: u64,
    pub max_text_bytes: usize,
}
impl Default for SkillSetReadLimits {
    fn default() -> Self {
        Self {
            max_steps: 5_000_000,
            max_text_bytes: 262144,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SkillSetReadUsage {
    pub steps: u64,
    pub text_bytes: usize,
}

/// One actual row, including earlier duplicate losers and retained active aliases.
/// Identity is private to the borrowed prepared owner; no serialized runtime ID.
#[derive(Clone, Copy)]
pub struct PreparedSkillSetRow<'a> {
    owner: &'a PreparedSkills,
    row: usize,
}
impl PreparedSkillSetRow<'_> {
    pub fn belongs_to(&self, owner: &PreparedSkills) -> bool {
        std::ptr::eq(self.owner, owner)
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        self.belongs_to(other.owner) && self.row == other.row
    }
}
/// The source always has a constructor/current group-list alias. Until an
/// admitted activation establishes that alias, it is unknown rather than nil.
#[derive(Clone, Copy)]
pub enum SkillSetActivation<'a> {
    Unavailable,
    Selected(PreparedSkillSetRow<'a>),
}
pub struct SkillSetReadView<'a> {
    owner: &'a PreparedSkills,
    sets: &'a SkillSets,
    limits: SkillSetReadLimits,
    usage: SkillSetReadUsage,
}
impl PreparedSkills {
    /// None means no admitted Skills Load has started. The original constructor
    /// and its first SetActiveSkillSet/Sync callbacks are not synthesized.
    /// A present view retains the last actual Load prefix, even on Source failure.
    pub fn skill_set_view(&self, limits: SkillSetReadLimits) -> Option<SkillSetReadView<'_>> {
        self.sets.as_ref().map(|sets| SkillSetReadView {
            owner: self,
            sets,
            limits,
            usage: SkillSetReadUsage::default(),
        })
    }
}
impl<'a> SkillSetReadView<'a> {
    fn charge(&mut self, steps: u64, text: usize) -> Result<(), EvaluationError> {
        let steps = self
            .usage
            .steps
            .checked_add(steps)
            .filter(|n| *n <= self.limits.max_steps)
            .ok_or_else(|| resource("skill-set read work"))?;
        let text_bytes = self
            .usage
            .text_bytes
            .checked_add(text)
            .filter(|n| *n <= self.limits.max_text_bytes)
            .ok_or_else(|| resource("skill-set read text"))?;
        self.usage = SkillSetReadUsage { steps, text_bytes };
        Ok(())
    }
    fn row_data(
        &mut self,
        row: &PreparedSkillSetRow<'_>,
    ) -> Result<&'a ProducedSkillSet, EvaluationError> {
        self.charge(1, 0)?;
        if !row.belongs_to(self.owner) {
            return Err(contract("skill-set row belongs to another prepared owner"));
        }
        self.sets
            .rows
            .get(row.row)
            .ok_or_else(|| contract("skill-set row missing"))
    }
    pub fn usage(&self) -> SkillSetReadUsage {
        self.usage
    }
    pub fn source(&mut self) -> Result<SourceOccurrenceId, EvaluationError> {
        self.charge(1, 0)?;
        Ok(self.sets.source)
    }
    /// This private order is append-only, so its retained length is a proven
    /// dense sequence length, independently of the numeric map's Lua length.
    pub fn dense_order_len(&mut self) -> Result<usize, EvaluationError> {
        self.charge(1, 0)?;
        Ok(self.sets.order.len())
    }
    pub fn is_singleton(&mut self) -> Result<bool, EvaluationError> {
        Ok(self.dense_order_len()? == 1)
    }
    pub fn ordered_key(
        &mut self,
        position: usize,
    ) -> Result<Option<NumericValue>, EvaluationError> {
        self.charge(1, 0)?;
        let index = position
            .checked_sub(1)
            .ok_or_else(|| contract("skill-set order position must be positive"))?;
        Ok(self.sets.order.get(index).map(|n| NumericValue::new(*n)))
    }
    pub fn winner(
        &mut self,
        number: NumericValue,
    ) -> Result<Option<PreparedSkillSetRow<'a>>, EvaluationError> {
        // BTree lookup has logarithmic comparisons; charging the number of
        // retained winners is a conservative bound without assuming node fanout.
        self.charge(self.sets.winners.len() as u64 + 1, 0)?;
        if number.value().is_nan() {
            return Ok(None);
        }
        Ok(self
            .sets
            .winners
            .get(&key(number.value()))
            .map(|&row| PreparedSkillSetRow {
                owner: self.owner,
                row,
            }))
    }
    pub fn active_key(&mut self) -> Result<NumericValue, EvaluationError> {
        self.charge(1, 0)?;
        Ok(NumericValue::new(self.sets.active_key))
    }
    pub fn activation(&mut self) -> Result<SkillSetActivation<'a>, EvaluationError> {
        self.charge(1, 0)?;
        Ok(match self.sets.active_row {
            Some(row) => SkillSetActivation::Selected(PreparedSkillSetRow {
                owner: self.owner,
                row,
            }),
            None => SkillSetActivation::Unavailable,
        })
    }
    pub fn row_key(
        &mut self,
        row: &PreparedSkillSetRow<'_>,
    ) -> Result<NumericValue, EvaluationError> {
        Ok(NumericValue::new(self.row_data(row)?.key))
    }
    pub fn origin(&mut self, row: &PreparedSkillSetRow<'_>) -> Result<SetOrigin, EvaluationError> {
        Ok(self.row_data(row)?.origin)
    }
    pub fn title(
        &mut self,
        row: &PreparedSkillSetRow<'_>,
    ) -> Result<Option<&'a str>, EvaluationError> {
        let title = self.row_data(row)?.title.as_deref();
        if let Some(title) = title {
            self.charge(title.len() as u64, title.len())?;
        }
        Ok(title)
    }
    pub fn group_count(&mut self, row: &PreparedSkillSetRow<'_>) -> Result<usize, EvaluationError> {
        Ok(self.row_data(row)?.groups.len())
    }
    pub fn group(
        &mut self,
        row: &PreparedSkillSetRow<'_>,
        position: usize,
    ) -> Result<Option<&'a PreparedSkillGroup>, EvaluationError> {
        let data = self.row_data(row)?;
        let index = position
            .checked_sub(1)
            .ok_or_else(|| contract("skill group position must be positive"))?;
        data.groups
            .get(index)
            .map(|&index| {
                self.owner
                    .report
                    .groups
                    .get(index)
                    .ok_or_else(|| contract("produced skill group missing"))
            })
            .transpose()
    }
}

#[cfg(test)]
#[path = "sets_tests.rs"]
mod tests;
