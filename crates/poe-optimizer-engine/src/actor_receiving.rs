//! Shared global receiver stage for admitted Armour/Evasion/ES and resistances.
//! Queries run on the same ordered DB and final attribute conditions as resources.
use super::*;
use crate::{character::CharacterModifiers, resistance::PlayerResistances};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReceivingScenario {
    pub resistance_penalty: f64,
    /// Fire, cold, lightning; chaos receives neither penalty nor these quests.
    pub resistance_quests: [bool; 3],
}
impl ReceivingScenario {
    pub(super) fn validate(self) -> Result<(), ActorError> {
        if !self.resistance_penalty.is_finite()
            || !(-1_000_000.0..=1_000_000.0).contains(&self.resistance_penalty)
        {
            return Err(ActorError(
                "Receiving resistance penalty must be finite and bounded",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReceivingOutput {
    pub armour: f64,
    pub evasion: f64,
    pub energy_shield: f64,
    pub resistances: PlayerResistances,
    /// Truncated total before separately truncated resistance cap/floor.
    pub resistance_totals: PlayerResistances,
    pub resistance_cap: f64,
    pub resistance_floor: f64,
}
pub(super) fn legacy_values(m: &CharacterModifiers) -> [f64; 8] {
    [
        m.armour_flat,
        m.evasion_flat,
        m.energy_shield_flat,
        m.fire_resistance_flat,
        m.cold_resistance_flat,
        m.lightning_resistance_flat,
        m.chaos_resistance_flat,
        m.elemental_resistance_flat,
    ]
}
pub(super) fn validate_source_character(character: &CharacterInput) -> Result<(), ActorError> {
    if legacy_values(&character.modifiers)
        .into_iter()
        .any(|value| value != 0.0)
    {
        return Err(ActorError(
            "Complete receiving preparation requires ordered records instead of legacy defensive scalars",
        ));
    }
    Ok(())
}
// Lua min/max select the second operand on equality, including signed zero.
fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
fn max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}
pub(super) fn calculate(
    queries: &impl ActorQueries,
    compiled: &CompiledGameData,
    armour: ArmourSlots<'_>,
) -> Result<ReceivingOutput, ActorError> {
    let data = compiled.snapshot().package();
    let mut resources = [0.0; 3];
    for resource in &data.receiving_defence.resources {
        let mut names = [""; 3];
        for (name, stat) in names.iter_mut().zip(&resource.query_stats) {
            *name = stat.upstream_name();
        }
        let names = &names[..resource.query_stats.len()];
        // Preserve literal resourceList operations, even with zero conversions.
        // Local slots enter separately below; total additions, MORE, slot-specific
        // query tags and overrides remain excluded. INC is not clamped.
        let global_base = queries.sum(SumKind::Base, names)? + 0.0;
        let global_base = global_base * (100.0 - 0.0) / 100.0;
        let inc = 1.0 + queries.sum(SumKind::Increased, names)? / 100.0;
        // Literal Helmet/Gloves/Boots/Body Armour/Weapon 2/Weapon 3 additions.
        // Each slot is multiplied before addition; flattening would change the
        // floating-point order. No admitted tag changes a slot query's INC.
        let mut value = 0.0;
        for slot in armour.ordered_values(resource.stat) {
            value += slot * inc * 1.0;
        }
        let value = max(
            round_to_integer(finite(value + global_base * inc * 1.0 + 0.0)?),
            0.0,
        );
        let index = match resource.stat {
            ActorStat::Armour => 0,
            ActorStat::Evasion => 1,
            ActorStat::EnergyShield => 2,
            _ => unreachable!("validated receiving resource output"),
        };
        resources[index] = value;
    }
    let cap = min(
        data.defence.resistance_maximum_cap,
        data.defence.player_resistance_cap,
    )
    .trunc();
    let floor = data.defence.resistance_floor.trunc();
    let mut totals = [0.0; 4];
    let mut finals = [0.0; 4];
    for resistance in &data.receiving_defence.resistances {
        let mut names = [""; 2];
        for (name, stat) in names.iter_mut().zip(&resistance.query_stats) {
            *name = stat.upstream_name();
        }
        let names = &names[..resistance.query_stats.len()];
        let base = queries.sum(SumKind::Base, names)?;
        let inc = max(
            (1.0 + queries.sum(SumKind::Increased, names)? / 100.0) * 1.0,
            0.0,
        );
        let total = finite(base * inc)?.trunc();
        let index = match resistance.stat {
            ActorStat::FireResist => 0,
            ActorStat::ColdResist => 1,
            ActorStat::LightningResist => 2,
            ActorStat::ChaosResist => 3,
            _ => unreachable!("validated receiving resistance output"),
        };
        totals[index] = total;
        finals[index] = max(min(total, cap), floor);
    }
    let values = |[fire, cold, lightning, chaos]: [f64; 4]| PlayerResistances {
        fire,
        cold,
        lightning,
        chaos,
    };
    Ok(ReceivingOutput {
        armour: resources[0],
        evasion: resources[1],
        energy_shield: resources[2],
        resistances: values(finals),
        resistance_totals: values(totals),
        resistance_cap: cap,
        resistance_floor: floor,
    })
}
impl CompiledGameData {
    pub fn receiving_scenario(
        &self,
        quests: SparkQuestRewards,
        resistance_penalty: f64,
    ) -> ReceivingScenario {
        ReceivingScenario {
            resistance_penalty,
            resistance_quests: [quests.blackjaw, quests.beira, quests.garukhan],
        }
    }
    pub(super) fn add_receiving_base(&self, base: &mut StackRecords, scenario: ReceivingScenario) {
        let data = self.snapshot().package();
        base.push(
            ActorStat::Evasion,
            ActorNumericOperation::Base,
            data.character.base_evasion,
            "Base",
        );
        for stat in [
            ActorStat::FireResist,
            ActorStat::ColdResist,
            ActorStat::LightningResist,
        ] {
            base.push(
                stat,
                ActorNumericOperation::Base,
                scenario.resistance_penalty,
                "Base",
            );
        }
        base.push(
            ActorStat::ChaosResist,
            ActorNumericOperation::Base,
            0.0,
            "Base",
        );
        for (enabled, stat) in scenario.resistance_quests.into_iter().zip([
            ActorStat::FireResist,
            ActorStat::ColdResist,
            ActorStat::LightningResist,
        ]) {
            if enabled {
                base.push(
                    stat,
                    ActorNumericOperation::Base,
                    data.quests.elemental_resistance,
                    "Config",
                );
            }
        }
    }
}
