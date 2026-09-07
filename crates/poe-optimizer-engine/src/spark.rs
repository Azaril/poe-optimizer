//! Closed native pipeline: Sorceress, level-one quality-zero Spark, no equipment,
//! support gems, allocated passives or other modifiers. The host must validate
//! that complete build scope before constructing these explicit inputs.
//!
//! Constants are transcribed from versioned upstream data, not calibration output.
//! See SOURCE_FILES and the source-executing tests; this is not a general build engine.

use crate::defence::round_to_integer;
use std::{error::Error, fmt};

pub const PROFILE_ID: &str = "poe2-spark-level1-unmodified-v1";
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

/// Sorceress tree class data; Spark level-one statSet; character and quest data;
/// resource initialization and attribute bonuses; resistance caps from Modules/Data.
pub const DATA: SparkData = SparkData {
    strength: 7.0,
    dexterity: 7.0,
    intelligence: 15.0,
    life_per_level: 12.0,
    initial_life: 16.0,
    mana_per_level: 4.0,
    initial_mana: 30.0,
    life_per_strength: 2.0,
    mana_per_intelligence: 2.0,
    lightning_minimum: 1.0,
    lightning_maximum: 10.0,
    cast_time: 0.7,
    critical_chance: 9.0,
    critical_damage_bonus: 100.0,
    quest_flat_life: 20.0,
    quest_life_increased: 5.0,
    quest_mana_increased: 5.0,
    quest_elemental_resistance: 10.0,
    resistance_floor: -200.0,
    player_resistance_cap: 75.0,
    enemy_resistance_cap: 90.0,
};

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
        Self {
            candlemass: true,
            molten_shrine: true,
            silent_hall: true,
            beira: true,
            garukhan: true,
            blackjaw: true,
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
    pub energy_shield: f64,
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

/// Evaluate one validated closed-profile input. No parsing, allocations, Lua,
/// timing, I/O or shared mutable state occur in this native production path.
pub fn evaluate(input: &SparkInput) -> Result<SparkOutput, SparkError> {
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
    let level = f64::from(input.character_level);
    // CalcSetup level multipliers, default quest modifiers, CalcPerform attribute
    // bonuses, then the rounding/minimum of CalcDefence.doActorLifeManaSpirit.
    let life_base = DATA.life_per_level * level
        + DATA.initial_life
        + if input.quests.candlemass {
            DATA.quest_flat_life
        } else {
            0.0
        }
        + DATA.strength * DATA.life_per_strength;
    let life_increased = if input.quests.molten_shrine {
        DATA.quest_life_increased
    } else {
        0.0
    };
    let mana_base = DATA.mana_per_level * level
        + DATA.initial_mana
        + DATA.intelligence * DATA.mana_per_intelligence;
    let mana_increased = if input.quests.silent_hall {
        DATA.quest_mana_increased
    } else {
        0.0
    };
    let life = round_to_integer(life_base * (1.0 + life_increased / 100.0)).max(1.0);
    let mana = round_to_integer(mana_base * (1.0 + mana_increased / 100.0)).max(1.0);
    let resistance = |quest| {
        let total = input.resistance_penalty
            + if quest {
                DATA.quest_elemental_resistance
            } else {
                0.0
            };
        total
            .trunc()
            .clamp(DATA.resistance_floor, DATA.player_resistance_cap)
    };
    // For this profile calcResistForType's configurable maximum admits values
    // above75 but caps them at90; no enemyMaxResist override is active.
    let enemy_resistance = input
        .enemy_lightning_resistance
        .clamp(DATA.resistance_floor, DATA.enemy_resistance_cap);
    let effective_multiplier = 1.0 - enemy_resistance / 100.0;
    let crit_chance = DATA.critical_chance;
    let crit_multiplier = 1.0 + DATA.critical_damage_bonus / 100.0;
    // CalcOffence executes separate ordinary/critical damage passes, averages
    // their damage endpoints, applies resistance, then weights by crit chance.
    let hit_average =
        (DATA.lightning_minimum / 2.0 + DATA.lightning_maximum / 2.0) * effective_multiplier;
    let crit_average = (DATA.lightning_minimum * crit_multiplier / 2.0
        + DATA.lightning_maximum * crit_multiplier / 2.0)
        * effective_multiplier;
    let average_hit =
        hit_average * (1.0 - crit_chance / 100.0) + crit_average * crit_chance / 100.0;
    let average_damage = average_hit * 100.0 / 100.0;
    let cast_rate = 1.0 / DATA.cast_time;
    Ok(SparkOutput {
        strength: DATA.strength,
        dexterity: DATA.dexterity,
        intelligence: DATA.intelligence,
        life,
        mana,
        energy_shield: 0.0,
        fire_resistance: resistance(input.quests.blackjaw),
        cold_resistance: resistance(input.quests.beira),
        lightning_resistance: resistance(input.quests.garukhan),
        chaos_resistance: 0.0,
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
