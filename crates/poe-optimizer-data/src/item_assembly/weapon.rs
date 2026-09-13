//! Parameters for the fixed, ordered weapon-local producer; never executable expressions.
use super::{
    Budget, ItemAssemblyLocalQuery, ItemAssemblyOverridePolicy, Result, error, required_option,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponAttackSpeed {
    pub output: String,
    pub query: ItemAssemblyLocalQuery,
    pub alternate: ItemAssemblyLocalQuery,
    pub quality_divisor: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponScaledField {
    pub base_field: String,
    pub output: String,
    pub round_places: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponRange {
    pub base_field: String,
    pub output: String,
    pub bonus_output: String,
    pub flat: ItemAssemblyLocalQuery,
    pub metres: ItemAssemblyLocalQuery,
    pub metre_multiplier: f64,
    pub alternate: ItemAssemblyLocalQuery,
    pub quality_divisor: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponReload {
    pub base_field: String,
    pub increased_output: String,
    pub output: String,
    pub query: ItemAssemblyLocalQuery,
    pub round_places: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponCritical {
    pub base_field: String,
    pub output: String,
    pub base: ItemAssemblyLocalQuery,
    pub increased: ItemAssemblyLocalQuery,
    pub alternate: ItemAssemblyLocalQuery,
    pub quality_divisor: f64,
    pub round_places: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyWeaponDamageKind {
    Physical,
    Elemental,
    Unscaled,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponDamageBound {
    pub base_field: String,
    pub output: String,
    pub query: ItemAssemblyLocalQuery,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponDamageChannel {
    /// Original captured-list spelling. Runtime dispatch uses kind, not this label.
    pub name: String,
    pub kind: ItemAssemblyWeaponDamageKind,
    pub minimum: ItemAssemblyWeaponDamageBound,
    pub maximum: ItemAssemblyWeaponDamageBound,
    #[serde(deserialize_with = "required_option")]
    pub increased: Option<ItemAssemblyLocalQuery>,
    pub dps_output: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponDamage {
    /// This same order drives both production and the final post-override sum.
    pub channels: Vec<ItemAssemblyWeaponDamageChannel>,
    pub elemental_increased: ItemAssemblyLocalQuery,
    pub physical_increased: ItemAssemblyLocalQuery,
    pub alternate_quality: ItemAssemblyLocalQuery,
    pub quality_threshold: f64,
    pub suppressed_quality: f64,
    pub base_default: f64,
    pub positive_threshold: f64,
    pub average_divisor: f64,
    pub round_places: u8,
    pub elemental_output: String,
    pub elemental_default: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemAssemblyWeaponFlagComparison {
    Equal,
    NotEqual,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponFlagTest {
    pub operation: ItemAssemblyWeaponFlagComparison,
    pub value: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponNameFlags {
    pub names: Vec<String>,
    pub flags: ItemAssemblyWeaponFlagTest,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponResidualPolicy {
    /// Ordered name/flag disjunction, then keyword disjunction, then absence of tag one.
    pub untagged: Vec<ItemAssemblyWeaponNameFlags>,
    pub keyword_flags: [f64; 2],
    /// This second branch has no keyword guard; absent/sole critical tag is allowed.
    pub critical: ItemAssemblyWeaponNameFlags,
    pub condition_tag_type: String,
    pub critical_condition: String,
    pub primary_slot: u16,
    pub primary_condition: String,
    pub other_condition: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyWeaponPolicy {
    pub base_field: String,
    pub output_field: String,
    pub type_output: String,
    pub type_base_field: String,
    pub name_output: String,
    pub name_item_field: String,
    pub attack_speed: ItemAssemblyWeaponAttackSpeed,
    pub attack_rate: ItemAssemblyWeaponScaledField,
    pub range: ItemAssemblyWeaponRange,
    pub reload: ItemAssemblyWeaponReload,
    pub damage: ItemAssemblyWeaponDamage,
    pub critical: ItemAssemblyWeaponCritical,
    pub overrides: ItemAssemblyOverridePolicy,
    pub residual: ItemAssemblyWeaponResidualPolicy,
    pub percent_divisor: f64,
    pub fraction_base: f64,
    pub total_output: String,
    pub total_initial: f64,
    pub total_default: f64,
}

fn name_flags(b: &mut Budget, rule: &ItemAssemblyWeaponNameFlags) -> Result<()> {
    b.count(rule.names.len(), 64)?;
    if rule.names.is_empty() {
        return Err(error("empty weapon name predicate"));
    }
    for name in &rule.names {
        b.text(name)?;
    }
    b.number(rule.flags.value)
}
pub(super) fn validate(b: &mut Budget, p: &ItemAssemblyWeaponPolicy) -> Result<()> {
    for s in [
        &p.base_field,
        &p.output_field,
        &p.type_output,
        &p.type_base_field,
        &p.name_output,
        &p.name_item_field,
        &p.attack_speed.output,
        &p.attack_rate.base_field,
        &p.attack_rate.output,
        &p.range.base_field,
        &p.range.output,
        &p.range.bonus_output,
        &p.reload.base_field,
        &p.reload.increased_output,
        &p.reload.output,
        &p.critical.base_field,
        &p.critical.output,
        &p.damage.elemental_output,
        &p.overrides.query_name,
        &p.overrides.key_field,
        &p.overrides.value_field,
        &p.residual.condition_tag_type,
        &p.residual.critical_condition,
        &p.residual.primary_condition,
        &p.residual.other_condition,
        &p.total_output,
    ] {
        b.text(s)?;
    }
    for q in [
        &p.attack_speed.query,
        &p.attack_speed.alternate,
        &p.range.flat,
        &p.range.metres,
        &p.range.alternate,
        &p.reload.query,
        &p.critical.base,
        &p.critical.increased,
        &p.critical.alternate,
        &p.damage.elemental_increased,
        &p.damage.physical_increased,
        &p.damage.alternate_quality,
    ] {
        b.query(q)?;
    }
    for n in [
        p.attack_speed.quality_divisor,
        p.range.metre_multiplier,
        p.range.quality_divisor,
        p.critical.quality_divisor,
        p.damage.quality_threshold,
        p.damage.suppressed_quality,
        p.damage.base_default,
        p.damage.positive_threshold,
        p.damage.average_divisor,
        p.damage.elemental_default,
        p.percent_divisor,
        p.fraction_base,
        p.total_initial,
        p.total_default,
        p.residual.keyword_flags[0],
        p.residual.keyword_flags[1],
    ] {
        b.number(n)?;
    }
    for places in [
        p.attack_rate.round_places,
        p.reload.round_places,
        p.critical.round_places,
        p.damage.round_places,
    ] {
        b.count(places.into(), 15)?;
    }
    b.count(p.residual.primary_slot.into(), 256)?;
    b.count(p.damage.channels.len(), 32)?;
    if p.damage.channels.is_empty() {
        return Err(error("empty weapon damage order"));
    }
    let mut names = BTreeSet::new();
    for c in &p.damage.channels {
        b.text(&c.name)?;
        if !names.insert(&c.name) {
            return Err(error("duplicate weapon damage channel"));
        }
        b.text(&c.dps_output)?;
        for bound in [&c.minimum, &c.maximum] {
            b.text(&bound.base_field)?;
            b.text(&bound.output)?;
            b.query(&bound.query)?;
        }
        if (c.kind == ItemAssemblyWeaponDamageKind::Elemental) != c.increased.is_some() {
            return Err(error("weapon damage classification/query mismatch"));
        }
        if let Some(q) = &c.increased {
            b.query(q)?;
        }
    }
    b.count(p.residual.untagged.len(), 64)?;
    if p.residual.untagged.is_empty() {
        return Err(error("empty weapon residual predicate"));
    }
    for rule in &p.residual.untagged {
        name_flags(b, rule)?;
    }
    name_flags(b, &p.residual.critical)
}
