//! Closed local item algorithm parameters. Roles are operands, never executable expressions.
use super::{Budget, ItemAssemblyLocalQuery, Result, error, required_option, unique};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyArmourRole {
    ArmourBase,
    ArmourEvasionBase,
    EvasionBase,
    EvasionEnergyShieldBase,
    EnergyShieldBase,
    ArmourEnergyShieldBase,
    WardBase,
    EvasionPerLevel,
    EnergyShieldPerLevel,
    WardPerLevel,
    ArmourIncreased,
    ArmourEvasionIncreased,
    EvasionIncreased,
    EvasionEnergyShieldIncreased,
    EnergyShieldIncreased,
    WardIncreased,
    ArmourEnergyShieldIncreased,
    DefencesIncreased,
}
impl ItemAssemblyArmourRole {
    fn has_base(self) -> bool {
        matches!(
            self,
            Self::ArmourBase | Self::EvasionBase | Self::EnergyShieldBase | Self::WardBase
        )
    }
    fn is_base(self) -> bool {
        matches!(
            self,
            Self::ArmourBase
                | Self::ArmourEvasionBase
                | Self::EvasionBase
                | Self::EvasionEnergyShieldBase
                | Self::EnergyShieldBase
                | Self::ArmourEnergyShieldBase
                | Self::WardBase
        )
    }
    fn is_per_level(self) -> bool {
        matches!(
            self,
            Self::EvasionPerLevel | Self::EnergyShieldPerLevel | Self::WardPerLevel
        )
    }
    fn is_increased(self) -> bool {
        !self.is_base() && !self.is_per_level()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyDefenceRole {
    Armour,
    Evasion,
    EnergyShield,
    Ward,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyBaseField {
    pub field: String,
    pub default: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyArmourQuery {
    pub role: ItemAssemblyArmourRole,
    pub query: ItemAssemblyLocalQuery,
    /// Read after the destructive query; `None` means no base-table read.
    #[serde(deserialize_with = "required_option")]
    pub base: Option<ItemAssemblyBaseField>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyDefenceRule {
    pub role: ItemAssemblyDefenceRole,
    pub base: ItemAssemblyBaseField,
    pub base_output: String,
    pub output: String,
    /// Nonempty ordered sums; association follows this order.
    pub base_roles: Vec<ItemAssemblyArmourRole>,
    pub increased_roles: Vec<ItemAssemblyArmourRole>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyPerLevelRule {
    pub base_role: ItemAssemblyArmourRole,
    pub output: String,
    pub increased_roles: Vec<ItemAssemblyArmourRole>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyBlockRule {
    pub base_field: String,
    pub output: String,
    pub base: ItemAssemblyLocalQuery,
    pub increased: ItemAssemblyLocalQuery,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyMovementRule {
    pub base_field: String,
    pub modifier_name: String,
    pub mod_type: String,
    pub multiplier: f64,
    pub tag_type: String,
    pub condition: String,
    pub negated: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyOverridePolicy {
    pub query_name: String,
    pub key_field: String,
    pub value_field: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyArmourPolicy {
    pub base_field: String,
    pub output_field: String,
    /// Complete source order, including each query's immediately following base read.
    pub queries: [ItemAssemblyArmourQuery; 18],
    pub alternate_quality: ItemAssemblyLocalQuery,
    pub quality_threshold: f64,
    pub suppressed_quality: f64,
    /// Base/final writes occur in this order, followed by all per-level writes.
    pub defences: [ItemAssemblyDefenceRule; 4],
    pub per_level: [ItemAssemblyPerLevelRule; 3],
    pub percent_divisor: f64,
    pub fraction_base: f64,
    pub round_places: u8,
    pub block: ItemAssemblyBlockRule,
    pub movement: ItemAssemblyMovementRule,
    pub overrides: ItemAssemblyOverridePolicy,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyQueryOutput {
    pub output: String,
    pub query: ItemAssemblyLocalQuery,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyQueryList {
    Slot,
    Base,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyScopedQueryOutput {
    pub output: String,
    pub query: ItemAssemblyLocalQuery,
    pub list: ItemAssemblyQueryList,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyDurationPolicy {
    pub base_field: String,
    pub output: String,
    pub increased: ItemAssemblyLocalQuery,
    pub more: ItemAssemblyLocalQuery,
    pub round_places: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyRecoveryRole {
    Life,
    Mana,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyRecoveryChannel {
    pub role: ItemAssemblyRecoveryRole,
    pub base_field: String,
    pub base_output: String,
    pub instant_output: String,
    pub gradual_output: String,
    pub total_output: String,
    #[serde(deserialize_with = "required_option")]
    pub additional: Option<ItemAssemblyQueryOutput>,
    pub effect_not_removed: ItemAssemblyScopedQueryOutput,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyRecoveryPolicy {
    pub instant: ItemAssemblyQueryOutput,
    pub increased: ItemAssemblyLocalQuery,
    pub rate: ItemAssemblyLocalQuery,
    /// Each truthy channel completes all its writes/queries before the next channel.
    pub channels: [ItemAssemblyRecoveryChannel; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyChargePolicy {
    pub maximum_base_field: String,
    pub maximum_output: String,
    pub maximum_base: ItemAssemblyLocalQuery,
    pub maximum_increased: ItemAssemblyLocalQuery,
    pub used_base_field: String,
    pub used_output: String,
    pub used_increased: ItemAssemblyLocalQuery,
    pub gain_base: ItemAssemblyQueryOutput,
    pub gain_increased: ItemAssemblyQueryOutput,
    pub gain_multiplier: ItemAssemblyQueryOutput,
    pub effect_output: String,
    pub effect_queries: [ItemAssemblyLocalQuery; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyFlaskPolicy {
    pub base_field: String,
    pub output_field: String,
    pub duration: ItemAssemblyDurationPolicy,
    pub recovery: ItemAssemblyRecoveryPolicy,
    pub charges: ItemAssemblyChargePolicy,
    pub overrides: ItemAssemblyOverridePolicy,
    pub percent_divisor: f64,
    pub fraction_base: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyCharmPolicy {
    pub base_field: String,
    pub output_field: String,
    pub duration: ItemAssemblyDurationPolicy,
    pub charges: ItemAssemblyChargePolicy,
    pub overrides: ItemAssemblyOverridePolicy,
    pub percent_divisor: f64,
    pub fraction_base: f64,
}

fn base(b: &mut Budget, v: &ItemAssemblyBaseField) -> Result<()> {
    b.text(&v.field)?;
    b.number(v.default)
}
fn operands(b: &Budget, v: &[ItemAssemblyArmourRole], increased: bool) -> Result<()> {
    b.count(v.len(), 18)?;
    if v.is_empty()
        || v.iter().any(|r| {
            if increased {
                !r.is_increased()
            } else {
                !r.is_base()
            }
        })
    {
        return Err(error("invalid local defence operand role"));
    }
    unique(v.iter().copied(), v.len())
}
fn output(b: &mut Budget, v: &ItemAssemblyQueryOutput) -> Result<()> {
    b.text(&v.output)?;
    b.query(&v.query)
}
fn overrides(b: &mut Budget, v: &ItemAssemblyOverridePolicy) -> Result<()> {
    for s in [&v.query_name, &v.key_field, &v.value_field] {
        b.text(s)?;
    }
    Ok(())
}
fn duration(b: &mut Budget, v: &ItemAssemblyDurationPolicy) -> Result<()> {
    b.text(&v.base_field)?;
    b.text(&v.output)?;
    b.query(&v.increased)?;
    b.query(&v.more)?;
    b.count(v.round_places.into(), 15)
}
fn charges(b: &mut Budget, v: &ItemAssemblyChargePolicy) -> Result<()> {
    for s in [
        &v.maximum_base_field,
        &v.maximum_output,
        &v.used_base_field,
        &v.used_output,
        &v.effect_output,
    ] {
        b.text(s)?;
    }
    for q in [
        &v.maximum_base,
        &v.maximum_increased,
        &v.used_increased,
        &v.effect_queries[0],
        &v.effect_queries[1],
    ] {
        b.query(q)?;
    }
    for o in [&v.gain_base, &v.gain_increased, &v.gain_multiplier] {
        output(b, o)?;
    }
    Ok(())
}
pub(super) fn validate(
    b: &mut Budget,
    a: &ItemAssemblyArmourPolicy,
    f: &ItemAssemblyFlaskPolicy,
    c: &ItemAssemblyCharmPolicy,
) -> Result<()> {
    for s in [
        &a.base_field,
        &a.output_field,
        &f.base_field,
        &f.output_field,
        &c.base_field,
        &c.output_field,
    ] {
        b.text(s)?;
    }
    unique(a.queries.iter().map(|q| q.role), 18)?;
    for q in &a.queries {
        b.query(&q.query)?;
        if q.base.is_some() != q.role.has_base() {
            return Err(error("local armour base-read role"));
        }
        if let Some(v) = &q.base {
            base(b, v)?;
        }
    }
    unique(a.defences.iter().map(|r| r.role), 4)?;
    for r in &a.defences {
        base(b, &r.base)?;
        b.text(&r.base_output)?;
        b.text(&r.output)?;
        operands(b, &r.base_roles, false)?;
        operands(b, &r.increased_roles, true)?;
    }
    unique(a.per_level.iter().map(|r| r.base_role), 3)?;
    for r in &a.per_level {
        if !r.base_role.is_per_level() {
            return Err(error("local per-level operand role"));
        }
        b.text(&r.output)?;
        operands(b, &r.increased_roles, true)?;
    }
    b.query(&a.alternate_quality)?;
    for n in [
        a.quality_threshold,
        a.suppressed_quality,
        a.percent_divisor,
        a.fraction_base,
        a.movement.multiplier,
        f.percent_divisor,
        f.fraction_base,
        c.percent_divisor,
        c.fraction_base,
    ] {
        b.number(n)?;
    }
    b.count(a.round_places.into(), 15)?;
    for s in [
        &a.block.base_field,
        &a.block.output,
        &a.movement.base_field,
        &a.movement.modifier_name,
        &a.movement.mod_type,
        &a.movement.tag_type,
        &a.movement.condition,
    ] {
        b.text(s)?;
    }
    b.query(&a.block.base)?;
    b.query(&a.block.increased)?;
    overrides(b, &a.overrides)?;
    duration(b, &f.duration)?;
    duration(b, &c.duration)?;
    charges(b, &f.charges)?;
    charges(b, &c.charges)?;
    overrides(b, &f.overrides)?;
    overrides(b, &c.overrides)?;
    output(b, &f.recovery.instant)?;
    b.query(&f.recovery.increased)?;
    b.query(&f.recovery.rate)?;
    unique(f.recovery.channels.iter().map(|r| r.role), 2)?;
    for r in &f.recovery.channels {
        for s in [
            &r.base_field,
            &r.base_output,
            &r.instant_output,
            &r.gradual_output,
            &r.total_output,
            &r.effect_not_removed.output,
        ] {
            b.text(s)?;
        }
        if let Some(v) = &r.additional {
            output(b, v)?;
        }
        b.query(&r.effect_not_removed.query)?;
    }
    Ok(())
}
