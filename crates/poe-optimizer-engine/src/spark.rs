//! Closed native pipeline: level-one quality-zero Spark, explicit class attributes
//! and supported passive effects, no equipment or supports. The host must validate
//! that complete build scope before constructing these explicit inputs.
//!
//! Balance records are supplied by an immutable compiled game-data snapshot.
//! See SOURCE_FILES and the source-executing tests; this is not a general build engine.

use crate::character::CharacterInput;
use crate::data::CompiledGameData;
use crate::defence::round_to_integer;
use std::{error::Error, fmt};

pub const PROFILE_ID: &str = "poe2-spark-body-movement-v6";
pub const TREE_VERSION: &str = "0_5";
pub const CLASS_ID: u32 = 7;
pub const SKILL_ID: &str = "SparkPlayer";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceFile {
    pub path: &'static str,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkData {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
    pub life_per_level: f64,
    pub initial_life: f64,
    pub mana_per_level: f64,
    pub initial_mana: f64,
    pub life_per_strength: f64,
    pub mana_per_intelligence: f64,
    pub lightning_minimum: f64,
    pub lightning_maximum: f64,
    pub cast_time: f64,
    pub critical_chance: f64,
    pub critical_damage_bonus: f64,
    pub quest_flat_life: f64,
    pub quest_life_increased: f64,
    pub quest_mana_increased: f64,
    pub quest_elemental_resistance: f64,
    pub resistance_floor: f64,
    pub player_resistance_cap: f64,
    pub enemy_resistance_cap: f64,
}

/// Reviewed default profile parameters, loaded through the same package compiler.
/// Explicit evaluator instances must use their injected `CompiledGameData` instead.
pub fn data() -> &'static SparkData {
    crate::data::bundled_reference()
        .expect("reviewed game-data package")
        .spark()
}

/// PoB's generated quest config checkboxes default true, independently of level.
/// Quest choice lists default None. Only these six defaults affect this profile's outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkQuestRewards {
    pub candlemass: bool,
    pub molten_shrine: bool,
    pub silent_hall: bool,
    pub beira: bool,
    pub garukhan: bool,
    pub blackjaw: bool,
}
impl Default for SparkQuestRewards {
    fn default() -> Self {
        Self::from_enabled(
            crate::data::bundled_reference()
                .expect("reviewed game-data package")
                .snapshot()
                .package()
                .quests
                .default_enabled,
        )
    }
}

impl SparkQuestRewards {
    /// Preserve the package's ordered quest contract: life, increased life/mana,
    /// then cold/lightning/fire resistance rewards.
    pub fn from_enabled(enabled: [bool; 6]) -> Self {
        let [
            candlemass,
            molten_shrine,
            silent_hall,
            beira,
            garukhan,
            blackjaw,
        ] = enabled;
        Self {
            candlemass,
            molten_shrine,
            silent_hall,
            beira,
            garukhan,
            blackjaw,
        }
    }
}

/// Enemy lightning resistance is resolved by the host from config/encounter defaults.
/// None/standard/pinnacle defaults are 0/30/50 in ConfigOptions. Uber damage reduction,
/// enemyMaxResist overrides, debuffs and other damage-taken modifiers are outside this scope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkInput {
    pub character_level: u32,
    pub resistance_penalty: f64,
    pub enemy_lightning_resistance: f64,
    pub quests: SparkQuestRewards,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkOutput {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
    pub life: f64,
    pub mana: f64,
    pub spirit: f64,
    pub effective_movement_speed_mod: f64,
    pub energy_shield: f64,
    pub armour: f64,
    pub evasion: f64,
    pub fire_resistance: f64,
    pub cold_resistance: f64,
    pub lightning_resistance: f64,
    pub chaos_resistance: f64,
    pub average_hit: f64,
    pub hit_dps: f64,
    pub cast_rate: f64,
    pub crit_chance: f64,
    pub crit_multiplier: f64,
    pub effective_enemy_lightning_resistance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparkError(pub &'static str);
impl fmt::Display for SparkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for SparkError {}

/// Reviewed default class attributes with no passive modifiers.
pub fn default_character() -> CharacterInput {
    crate::data::bundled_reference()
        .expect("reviewed game-data package")
        .default_spark_character()
}

/// Convenience evaluation using the reviewed package through the common compiler.
pub fn evaluate(input: &SparkInput) -> Result<SparkOutput, SparkError> {
    evaluate_with_character(input, &default_character())
}

/// Convenience evaluation using the reviewed package and explicit character inputs.
pub fn evaluate_with_character(
    input: &SparkInput,
    character: &CharacterInput,
) -> Result<SparkOutput, SparkError> {
    let data = crate::data::bundled_reference()
        .map_err(|_| SparkError("The reviewed game-data package failed compilation"))?;
    evaluate_with_data(input, character, data)
}

/// Calculate using only the explicitly supplied immutable dataset and resolved inputs.
/// The caller must validate the full document and allocation scope independently.
pub fn evaluate_with_data(
    input: &SparkInput,
    character: &CharacterInput,
    compiled: &CompiledGameData,
) -> Result<SparkOutput, SparkError> {
    let actor = compiled
        .prepare_actor_resources(
            input.character_level,
            compiled.actor_quest_selection(input.quests),
            character,
            &[],
        )
        .map_err(|error| SparkError(error.0))?;
    evaluate_with_actor(input, character, compiled, &actor)
}

/// Calculate skill output from a prepared shared actor stage. No actor queries
/// or source/configuration parsing run during this successful numerical call.
pub fn evaluate_with_actor(
    input: &SparkInput,
    character: &CharacterInput,
    compiled: &CompiledGameData,
    actor: &crate::actor::PreparedActorResources,
) -> Result<SparkOutput, SparkError> {
    actor
        .validate_profile(compiled, input.character_level, input.quests, character)
        .map_err(|error| SparkError(error.0))?;
    let receiving = actor
        .receiving_for(compiled.receiving_scenario(input.quests, input.resistance_penalty))
        .map_err(|error| SparkError(error.0))?;
    let movement = actor.movement();
    let actor = actor.values();
    let data = compiled.spark();
    let rules = &compiled.snapshot().package().character;
    character.validate().map_err(|error| SparkError(error.0))?;
    let attributes = actor.attributes;
    let modifiers = character.modifiers;
    if !(1..=100).contains(&input.character_level) {
        return Err(SparkError("Spark profile character level must be 1..100"));
    }
    if !input.resistance_penalty.is_finite() || !(-60.0..=0.0).contains(&input.resistance_penalty) {
        return Err(SparkError(
            "Spark profile resistance penalty must be finite and -60..0",
        ));
    }
    if !input.enemy_lightning_resistance.is_finite()
        || !(-200.0..=200.0).contains(&input.enemy_lightning_resistance)
    {
        return Err(SparkError(
            "Spark profile enemy lightning resistance must be finite and -200..200",
        ));
    }
    let life = actor.life;
    let mana = actor.mana;
    let resistance = receiving.map(|value| value.resistances).unwrap_or_else(|| {
        crate::resistance::calculate(
            &modifiers,
            input.resistance_penalty,
            [
                input.quests.blackjaw,
                input.quests.beira,
                input.quests.garukhan,
            ]
            .map(|enabled| {
                if enabled {
                    data.quest_elemental_resistance
                } else {
                    0.0
                }
            }),
            compiled.defence(),
        )
    });
    // For this profile calcResistForType's configurable maximum admits values
    // above75 but caps them at90; no enemyMaxResist override is active.
    let enemy_resistance = input
        .enemy_lightning_resistance
        .clamp(data.resistance_floor, data.enemy_resistance_cap);
    let effective_multiplier = 1.0 - enemy_resistance / 100.0;
    let crit_chance =
        crate::offence::capped_critical_chance(data.critical_chance, rules.critical_chance_cap);
    let crit_multiplier = 1.0 + data.critical_damage_bonus / 100.0;
    // CalcOffence executes separate ordinary/critical damage passes, averages
    // their damage endpoints, applies resistance, then weights by crit chance.
    let damage_increased = modifiers.spell_damage_increased + modifiers.projectile_damage_increased;
    let damage_multiplier = 1.0 + damage_increased / 100.0;
    let lightning_minimum = round_to_integer(data.lightning_minimum * damage_multiplier);
    let lightning_maximum = round_to_integer(data.lightning_maximum * damage_multiplier);
    let hit_average = (lightning_minimum / 2.0 + lightning_maximum / 2.0) * effective_multiplier;
    let crit_average = (lightning_minimum * crit_multiplier / 2.0
        + lightning_maximum * crit_multiplier / 2.0)
        * effective_multiplier;
    let average_hit =
        hit_average * (1.0 - crit_chance / 100.0) + crit_average * crit_chance / 100.0;
    let average_damage = average_hit * 100.0 / 100.0;
    let speed_multiplier =
        round_to_integer((1.0 + modifiers.skill_speed_increased / 100.0) * 100.0) / 100.0;
    let cast_rate = 1.0 / (data.cast_time / speed_multiplier);
    Ok(SparkOutput {
        strength: attributes.strength,
        dexterity: attributes.dexterity,
        intelligence: attributes.intelligence,
        life,
        mana,
        spirit: actor.spirit,
        effective_movement_speed_mod: movement.effective_movement_speed_mod,
        energy_shield: receiving
            .map(|value| value.energy_shield)
            .unwrap_or_else(|| round_to_integer(modifiers.energy_shield_flat).max(0.0)),
        armour: receiving
            .map(|value| value.armour)
            .unwrap_or_else(|| round_to_integer(modifiers.armour_flat).max(0.0)),
        evasion: receiving.map(|value| value.evasion).unwrap_or_else(|| {
            round_to_integer(rules.base_evasion + modifiers.evasion_flat).max(0.0)
        }),
        fire_resistance: resistance.fire,
        cold_resistance: resistance.cold,
        lightning_resistance: resistance.lightning,
        chaos_resistance: resistance.chaos,
        average_hit,
        hit_dps: average_damage * cast_rate,
        cast_rate,
        crit_chance,
        crit_multiplier,
        effective_enemy_lightning_resistance: enemy_resistance,
    })
}

/// SHA-256 over full upstream source files, normalized from CRLF to LF.
pub const SOURCE_FILES: &[SourceFile] = &[
    SourceFile {
        path: "src/Modules/ModParser.lua",
        sha256: "6973c25f296c813187a85024e69737f0e69db43fc3fc8f281e1ac32e4409df95",
    },
    SourceFile {
        path: "src/Data/Misc.lua",
        sha256: "21addc73f772e558143a89c3e45d62e838524f1a968aabe03521254d4ce133c9",
    },
    SourceFile {
        path: "src/Data/QuestRewards.lua",
        sha256: "66429f8ef76747aecf0a449dc898fc749a499f066711b08a4e5cd7a6690b35af",
    },
    SourceFile {
        path: "src/Data/Skills/act_int.lua",
        sha256: "4da9241e4766c5d2f9304f95b73a3b2b9c623c496da17c9d95b10eef5c71174d",
    },
    SourceFile {
        path: "src/Modules/CalcDefence.lua",
        sha256: "b0f498e93dd69ac09deea875dfc186345e26a977419a6b5cec1cf9b04e1f46d2",
    },
    SourceFile {
        path: "src/Modules/CalcOffence.lua",
        sha256: "90924d824cb72feaa9040ad03d840da5623576315d4b2e1a596979002c98af94",
    },
    SourceFile {
        path: "src/Modules/CalcPerform.lua",
        sha256: "d7aee3caffd3e4225066716075e54437466ccf0b8e53a816fa7083e210839cae",
    },
    SourceFile {
        path: "src/Modules/CalcSetup.lua",
        sha256: "5c51f3a8dd93ab99ce67763a015ad4644b4a99b1a74c31359b831e3965404312",
    },
    SourceFile {
        path: "src/Modules/Common.lua",
        sha256: "bae6d0704a92fb04ed56a6033f9229eb2683c53785c14b9c5571b4c0591b0fe8",
    },
    SourceFile {
        path: "src/Modules/ConfigOptions.lua",
        sha256: "0ddf50157dc98a63c11327fc41b989dd7aca713ef7e402037eb79409bffb0467",
    },
    SourceFile {
        path: "src/Modules/Data.lua",
        sha256: "2c7d37cfdeda234a8741847e6dd5019dcf8ca9752aaed596f333f80489b430a4",
    },
    SourceFile {
        path: "src/TreeData/0_5/tree.lua",
        sha256: "e3350cd64e50976911cdc9fd2fc5f4cd650039697affefe33861cf63930cc66c",
    },
];
