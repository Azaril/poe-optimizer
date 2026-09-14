//! Bounded reads of the retained authored configuration prefix.
//! These rows are producer identities, not a final-view or root-time state.
use super::{
    EvaluationError, EvaluationErrorKind, NumericValue, PreparedConfigSet, PreparedConfiguration,
    SetOrigin, contract, number_key, resource,
};

#[derive(Debug, Clone, Copy)]
pub struct ConfigurationReadLimits {
    pub max_steps: u64,
    pub max_text_bytes: usize,
}
impl Default for ConfigurationReadLimits {
    fn default() -> Self {
        Self {
            max_steps: 500_000,
            max_text_bytes: 262_144,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ConfigurationReadUsage {
    pub steps: u64,
    pub text_bytes: usize,
}

/// One exact created row, including a row replaced by a duplicate numeric key.
/// Borrowed identity does not survive movement or mutation of its owner. No raw
/// row index, public fabrication, serialization, or persistent token is exposed.
#[derive(Clone, Copy)]
pub struct ConfigurationSetRow<'a> {
    owner: &'a PreparedConfiguration,
    index: usize,
}
impl ConfigurationSetRow<'_> {
    pub fn belongs_to(&self, owner: &PreparedConfiguration) -> bool {
        std::ptr::eq(self.owner, owner)
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        self.belongs_to(other.owner) && self.index == other.index
    }
}

#[derive(Clone, Copy)]
pub enum ConfigurationActivation<'a> {
    /// The prefix did not reach its active-row assignment. Original constructor
    /// or prior input/placeholder aliases are unrepresented, not known absent.
    Unavailable,
    /// The selected key is retained separately from row.id: Lua numeric key
    /// equality joins both zero signs without changing the selected scalar bits.
    Selected {
        key: NumericValue,
        row: ConfigurationSetRow<'a>,
    },
}

/// Reads charge an independent caller budget and never consume producer limits.
/// This API neither executes nor permits skipping any continuation boundary.
pub struct ConfigurationReadView<'a> {
    state: &'a PreparedConfiguration,
    limits: ConfigurationReadLimits,
    usage: ConfigurationReadUsage,
}
impl PreparedConfiguration {
    pub fn read_view(&self, limits: ConfigurationReadLimits) -> ConfigurationReadView<'_> {
        ConfigurationReadView {
            state: self,
            limits,
            usage: ConfigurationReadUsage::default(),
        }
    }
}
impl<'a> ConfigurationReadView<'a> {
    pub fn usage(&self) -> ConfigurationReadUsage {
        self.usage
    }
    fn charge(&mut self, steps: u64, text_bytes: usize) -> Result<(), EvaluationError> {
        let next_steps = self
            .usage
            .steps
            .checked_add(steps)
            .filter(|n| *n <= self.limits.max_steps)
            .ok_or_else(|| resource("configuration read steps"))?;
        let next_bytes = self
            .usage
            .text_bytes
            .checked_add(text_bytes)
            .filter(|n| *n <= self.limits.max_text_bytes)
            .ok_or_else(|| resource("configuration read text bytes"))?;
        self.usage = ConfigurationReadUsage {
            steps: next_steps,
            text_bytes: next_bytes,
        };
        Ok(())
    }
    fn count_cost(count: usize) -> Result<u64, EvaluationError> {
        u64::try_from(count).map_err(|_| resource("configuration read count"))
    }
    fn scan_cost(count: usize) -> Result<u64, EvaluationError> {
        Self::count_cost(count)?
            .checked_add(1)
            .ok_or_else(|| resource("configuration read count"))
    }
    fn make_row(&self, index: usize) -> Result<ConfigurationSetRow<'a>, EvaluationError> {
        self.state
            .machine
            .report
            .sets
            .get(index)
            .ok_or_else(|| contract("configuration retained row is missing"))?;
        Ok(ConfigurationSetRow {
            owner: self.state,
            index,
        })
    }
    fn set(
        &mut self,
        row: &ConfigurationSetRow<'_>,
    ) -> Result<&'a PreparedConfigSet, EvaluationError> {
        self.charge(1, 0)?;
        if !row.belongs_to(self.state) {
            return Err(contract("configuration row belongs to another producer"));
        }
        self.state
            .machine
            .report
            .sets
            .get(row.index)
            .ok_or_else(|| contract("configuration retained row is missing"))
    }
    pub fn produced_len(&mut self) -> Result<usize, EvaluationError> {
        self.charge(1, 0)?;
        Ok(self.state.machine.report.sets.len())
    }
    /// One-based creation order, independent of saved order and duplicate winners.
    pub fn produced_row(
        &mut self,
        ordinal: usize,
    ) -> Result<Option<ConfigurationSetRow<'a>>, EvaluationError> {
        self.charge(1, 0)?;
        let Some(index) = ordinal.checked_sub(1) else {
            return Err(contract("configuration creation ordinal is not positive"));
        };
        if index >= self.state.machine.report.sets.len() {
            return Ok(None);
        }
        self.make_row(index).map(Some)
    }
    pub fn winner(
        &mut self,
        key: NumericValue,
    ) -> Result<Option<ConfigurationSetRow<'a>>, EvaluationError> {
        // A conservative linear charge bounds the private BTree lookup without
        // exposing or adopting any map iteration order as source semantics.
        self.charge(Self::scan_cost(self.state.machine.winners.len())?, 0)?;
        if key.value().is_nan() {
            return Ok(None);
        }
        self.state
            .machine
            .winners
            .get(&number_key(key.value()))
            .copied()
            .map(|index| self.make_row(index))
            .transpose()
    }
    /// Raw one-based saved-order read. Holes remain absent and later entries stay
    /// readable; this does not infer Lua's length for a sparse table.
    pub fn ordered_key(
        &mut self,
        position: usize,
    ) -> Result<Option<NumericValue>, EvaluationError> {
        self.charge(1, 0)?;
        let Some(index) = position.checked_sub(1) else {
            return Err(contract("configuration order position is not positive"));
        };
        Ok(self
            .state
            .machine
            .report
            .order
            .get(index)
            .copied()
            .flatten())
    }
    /// Only a dense present prefix followed by absent entries has a proven
    /// length. A present value after a hole is an explicit unsupported read.
    pub fn dense_order_len(&mut self) -> Result<usize, EvaluationError> {
        self.charge(Self::scan_cost(self.state.machine.report.order.len())?, 0)?;
        let mut len = 0;
        let mut hole = false;
        for key in &self.state.machine.report.order {
            if key.is_some() {
                if hole {
                    return Err(EvaluationError::new(
                        EvaluationErrorKind::UnsupportedCapability,
                        "configuration order has no proven dense length",
                    ));
                }
                len += 1;
            } else {
                hole = true;
            }
        }
        Ok(len)
    }
    pub fn is_singleton(&mut self) -> Result<bool, EvaluationError> {
        Ok(self.dense_order_len()? == 1)
    }
    pub fn active(&mut self) -> Result<ConfigurationActivation<'a>, EvaluationError> {
        self.charge(1, 0)?;
        match self.state.machine.active {
            None => Ok(ConfigurationActivation::Unavailable),
            Some((key, index)) => Ok(ConfigurationActivation::Selected {
                key,
                row: self.make_row(index)?,
            }),
        }
    }
    pub fn key(&mut self, row: &ConfigurationSetRow<'_>) -> Result<NumericValue, EvaluationError> {
        Ok(self.set(row)?.key)
    }
    pub fn origin(&mut self, row: &ConfigurationSetRow<'_>) -> Result<SetOrigin, EvaluationError> {
        Ok(self.set(row)?.origin)
    }
    pub fn title(
        &mut self,
        row: &ConfigurationSetRow<'_>,
    ) -> Result<Option<&'a str>, EvaluationError> {
        let title = self.set(row)?.title.as_deref();
        if let Some(title) = title {
            self.charge(Self::count_cost(title.len())?, title.len())?;
        }
        Ok(title)
    }
}
