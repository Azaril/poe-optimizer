//! Prepared local weapon assembly from the pinned Item.lua rules.
//!
//! Local modifier consumption uses exact flags, zero keyword flags and the
//! literal first-tag predicate. It is intentionally separate from ModDB's
//! subset queries. Preparation retains leftover evidence; supported Mace item
//! preparation refuses any unconsumed modifier instead of treating it globally.
use crate::{
    data::CompiledGameData,
    defence::round_to_integer,
    mace::{MaceError, MaceWeapon, MaceWeaponData},
    modifiers::{ModifierInput, ModifierKind, ModifierValue, NumericKind},
};
use poe_optimizer_data::game_data::{
    ItemCaptureKind, ItemModifierRoll, LocalWeaponOperation, LocalWeaponStat,
};
use std::sync::Arc;

/// Result of one literal numeric Item.calcLocal query. The input list retains
/// its original order except for consumed entries. Later tags do not affect the
/// first-tag predicate; slot-number filtering is a separate upstream stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalConsumption {
    pub value: f64,
    pub consumed: usize,
}

pub fn consume_local_numeric(
    modifiers: &mut Vec<ModifierInput>,
    name: &str,
    kind: NumericKind,
    flags: u64,
) -> Result<LocalConsumption, MaceError> {
    let mut result = LocalConsumption {
        value: if kind == NumericKind::More { 1.0 } else { 0.0 },
        consumed: 0,
    };
    let mut index = 0;
    while let Some(modifier) = modifiers.get(index) {
        if modifier.name == name
            && modifier.kind == ModifierKind::Numeric(kind)
            && modifier.flags == flags
            && modifier.keyword_flags == 0
            && modifier.tag_kinds.first().is_none_or(|tag| tag == "InSlot")
        {
            let ModifierValue::Number(value) = modifier.value else {
                return Err(MaceError("Local numeric modifier must have a number value"));
            };
            if !value.is_finite() {
                return Err(MaceError("Local numeric modifier must be finite"));
            }
            result.value = if kind == NumericKind::More {
                result.value * ((100.0 + value) / 100.0)
            } else {
                result.value + value
            };
            if !result.value.is_finite() {
                return Err(MaceError("Local numeric aggregation must be finite"));
            }
            modifiers.remove(index);
            result.consumed += 1;
        } else {
            index += 1;
        }
    }
    Ok(result)
}

/// Values emitted by Item.lua's local weapon assembly. Suppressed damage pairs
/// use numeric zero and retain explicit presence flags. Local critical chance
/// has not yet received the actor's critical-chance cap or accuracy check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponStats {
    pub physical_minimum: f64,
    pub physical_maximum: f64,
    pub fire_minimum: f64,
    pub fire_maximum: f64,
    pub physical_present: bool,
    pub fire_present: bool,
    pub attack_speed_increased: f64,
    pub attack_rate: f64,
    pub critical_chance: f64,
}

/// Source-local assembly for the admitted physical/fire operations. Modifiers
/// not consumed by these exact queries stay in `modifiers` for caller review.
/// This helper does not certify item legality, slot filtering or global effects.
pub fn assemble_local_weapon(
    base: MaceWeaponData<'_>,
    quality: u32,
    modifiers: &mut Vec<ModifierInput>,
) -> Result<WeaponStats, MaceError> {
    if quality > 20
        || [
            base.physical_minimum,
            base.physical_maximum,
            base.fire_minimum,
            base.fire_maximum,
            base.attack_rate,
            base.critical_chance,
        ]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(MaceError(
            "Local weapon bases must be finite and quality in 0..20",
        ));
    }
    // Query order follows Item.lua: speed, physical endpoints/increase, fire
    // endpoints, then critical chance. Each query removes only local matches.
    let speed = consume_local_numeric(modifiers, "Speed", NumericKind::Increased, 0x1)?.value;
    let attack_rate = round_to_integer(base.attack_rate * (1.0 + speed / 100.0) * 100.0) / 100.0;
    let physical_min = base.physical_minimum
        + consume_local_numeric(modifiers, "PhysicalMin", NumericKind::Base, 0)?.value;
    let physical_max = base.physical_maximum
        + consume_local_numeric(modifiers, "PhysicalMax", NumericKind::Base, 0)?.value;
    let physical_inc =
        consume_local_numeric(modifiers, "PhysicalDamage", NumericKind::Increased, 0)?.value;
    let quality_multiplier = 1.0 + f64::from(quality) / 100.0;
    let physical_min =
        round_to_integer(physical_min * (1.0 + physical_inc / 100.0) * quality_multiplier);
    let physical_max =
        round_to_integer(physical_max * (1.0 + physical_inc / 100.0) * quality_multiplier);
    let fire_min = round_to_integer(
        base.fire_minimum
            + consume_local_numeric(modifiers, "FireMin", NumericKind::Base, 0)?.value,
    );
    let fire_max = round_to_integer(
        base.fire_maximum
            + consume_local_numeric(modifiers, "FireMax", NumericKind::Base, 0)?.value,
    );
    let crit_inc = consume_local_numeric(modifiers, "CritChance", NumericKind::Increased, 0)?.value;
    let critical_chance =
        round_to_integer(base.critical_chance * (1.0 + crit_inc / 100.0) * 100.0) / 100.0;
    if [
        physical_min,
        physical_max,
        fire_min,
        fire_max,
        attack_rate,
        critical_chance,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(MaceError("Local weapon assembly must remain finite"));
    }
    let physical_present = physical_min > 0.0 && physical_max > 0.0;
    let fire_present = fire_min > 0.0 && fire_max > 0.0;
    Ok(WeaponStats {
        physical_minimum: if physical_present { physical_min } else { 0.0 },
        physical_maximum: if physical_present { physical_max } else { 0.0 },
        fire_minimum: if fire_present { fire_min } else { 0.0 },
        fire_maximum: if fire_present { fire_max } else { 0.0 },
        physical_present,
        fire_present,
        attack_speed_increased: speed,
        attack_rate,
        critical_chance,
    })
}

/// Immutable weapon values owned by exactly one compiled dataset. Fields are
/// private; an item parser and host bind these values to the source candidate.
#[derive(Debug, Clone)]
pub struct PreparedWeaponStats {
    binding: Arc<()>,
    weapon: MaceWeapon,
    quality: u32,
    item_level: u32,
    stats: WeaponStats,
    consumed_modifier_count: usize,
    modifier_roll_count: usize,
}
impl PreparedWeaponStats {
    pub fn weapon(&self) -> MaceWeapon {
        self.weapon
    }
    pub fn quality(&self) -> u32 {
        self.quality
    }
    pub fn item_level(&self) -> u32 {
        self.item_level
    }
    pub fn stats(&self) -> WeaponStats {
        self.stats
    }
    pub fn consumed_modifier_count(&self) -> usize {
        self.consumed_modifier_count
    }
    pub fn modifier_roll_count(&self) -> usize {
        self.modifier_roll_count
    }
}
impl CompiledGameData {
    /// Resolve ordered supplied rolls through this dataset's injected rules and
    /// prepare local stats once. The legacy empty-roll path performs no allocation.
    pub fn prepare_mace_weapon(
        &self,
        weapon: MaceWeapon,
        quality: u32,
        item_level: u32,
        rolls: &[ItemModifierRoll],
    ) -> Result<PreparedWeaponStats, MaceError> {
        if quality > 20 || !(1..=100).contains(&item_level) {
            return Err(MaceError(
                "Mace profile item level must be 1..100 and quality 0..20",
            ));
        }
        if rolls.len() > 64 {
            return Err(MaceError(
                "Mace weapon admits at most 64 explicit modifier rolls",
            ));
        }
        let mut modifiers = Vec::new();
        for roll in rolls {
            let rule = self
                .snapshot()
                .package()
                .item_modifier_rule(&roll.rule_id)
                .ok_or(MaceError("Unknown selected item modifier rule"))?;
            if roll.values.len() != rule.captures.len() {
                return Err(MaceError(
                    "Item modifier roll has a different capture count from its selected rule",
                ));
            }
            for (value, kind) in roll.values.iter().zip(&rule.captures) {
                if !value.is_finite()
                    || !(0.0..=1_000_000.0).contains(value)
                    || (*kind == ItemCaptureKind::UnsignedInteger && value.fract() != 0.0)
                {
                    return Err(MaceError(
                        "Item modifier captures must match selected syntax and be finite in 0..1000000",
                    ));
                }
            }
            if roll.values.len() == 2 && roll.values[0] > roll.values[1] {
                return Err(MaceError(
                    "Local added-damage minimum must not exceed maximum",
                ));
            }
            for mapping in &rule.modifiers {
                let name = match mapping.stat {
                    LocalWeaponStat::PhysicalMinimum => "PhysicalMin",
                    LocalWeaponStat::PhysicalMaximum => "PhysicalMax",
                    LocalWeaponStat::FireMinimum => "FireMin",
                    LocalWeaponStat::FireMaximum => "FireMax",
                    LocalWeaponStat::PhysicalDamage => "PhysicalDamage",
                    LocalWeaponStat::Speed => "Speed",
                    LocalWeaponStat::CriticalChance => "CritChance",
                };
                let kind = match mapping.operation {
                    LocalWeaponOperation::Base => NumericKind::Base,
                    LocalWeaponOperation::Increased => NumericKind::Increased,
                };
                modifiers.push(ModifierInput {
                    name: name.into(),
                    kind: ModifierKind::Numeric(kind),
                    value: ModifierValue::Number(
                        *roll
                            .values
                            .get(mapping.capture as usize)
                            .ok_or(MaceError("Invalid item modifier capture mapping"))?,
                    ),
                    flags: mapping.flags,
                    keyword_flags: mapping.keyword_flags,
                    source: Some(roll.rule_id.clone()),
                    tag_kinds: vec![],
                });
            }
        }
        let consumed_modifier_count = modifiers.len();
        let stats = assemble_local_weapon(self.weapon(weapon), quality, &mut modifiers)?;
        if !modifiers.is_empty() {
            return Err(MaceError(
                "Selected item contains unconsumed or unsupported local weapon modifiers",
            ));
        }
        Ok(PreparedWeaponStats {
            binding: self.weapon_binding.clone(),
            weapon,
            quality,
            item_level,
            stats,
            consumed_modifier_count,
            modifier_roll_count: rolls.len(),
        })
    }
    pub(crate) fn owns_weapon(&self, weapon: &PreparedWeaponStats) -> bool {
        Arc::ptr_eq(&self.weapon_binding, &weapon.binding)
    }
}
