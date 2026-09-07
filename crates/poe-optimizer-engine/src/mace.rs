//! Closed native Mace Strike profile with one normal mace and optional
//! level-one quality-zero Brutality I. Hosts validate the complete document:
//! explicit class attributes and admitted entrance effects, no other equipment,
//! allocated ascendancy effects, supports or external modifiers.
//! Enemy values are resolved by the host; this kernel does not select encounters.

use crate::character::{BASE_EVASION, CharacterAttributes, CharacterInput, CharacterModifiers};
use crate::{
    defence::{PINNED_CONSTANTS, armour_reduction_percent, hit_chance, round_to_integer},
    spark::{self, SourceFile, SparkQuestRewards},
};
use std::{error::Error, fmt};

pub const PROFILE_ID: &str = "poe2-mace-strike-class-entrance-v2";
pub const TREE_VERSION: &str = "0_5";
/// Index in the pinned tree classes table; XML classInternalId is a separate id.
pub const CLASS_ID: u32 = 3;
pub const CLASS_INTERNAL_ID: u32 = 6;
pub const SKILL_ID: &str = "Melee1HMacePlayer";
pub const SUPPORT_ID: &str = "SupportBrutalityPlayer";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaceWeapon {
    WoodenClub,
    SmithingHammer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaceWeaponData {
    pub name: &'static str,
    pub physical_minimum: f64,
    pub physical_maximum: f64,
    pub fire_minimum: f64,
    pub fire_maximum: f64,
    pub attack_rate: f64,
    pub critical_chance: f64,
    pub required_strength: u32,
}
impl MaceWeapon {
    pub const fn data(self) -> MaceWeaponData {
        match self {
            Self::WoodenClub => MaceWeaponData {
                name: "Wooden Club",
                physical_minimum: 6.0,
                physical_maximum: 10.0,
                fire_minimum: 0.0,
                fire_maximum: 0.0,
                attack_rate: 1.45,
                critical_chance: 5.0,
                required_strength: 0,
            },
            Self::SmithingHammer => MaceWeaponData {
                name: "Smithing Hammer",
                physical_minimum: 5.0,
                physical_maximum: 9.0,
                fire_minimum: 5.0,
                fire_maximum: 9.0,
                attack_rate: 1.45,
                critical_chance: 5.0,
                required_strength: 11,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaceData {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
    pub accuracy_per_level: f64,
    pub accuracy_per_dexterity: f64,
    pub brutality_physical_more: f64,
    pub enemy_physical_reduction_cap: f64,
}
pub const DATA: MaceData = MaceData {
    strength: 15.0,
    dexterity: 7.0,
    intelligence: 7.0,
    accuracy_per_level: 6.0,
    accuracy_per_dexterity: 6.0,
    brutality_physical_more: 25.0,
    enemy_physical_reduction_cap: 75.0,
};

/// Effective-mode enemy armour/evasion must already include encounter defaults.
/// This profile has no accuracy distance penalty at the default melee distance,
/// enemy block, armour break, enemy physical reduction modifiers or damage-taken
/// modifiers. Fire resistance uses PoB's ordinary configurable maximum, not an
/// enemyMaxResist override. Only hit DPS is claimed, not combined/ailment DPS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaceInput {
    pub character_level: u32,
    pub weapon: MaceWeapon,
    pub quality: u32,
    /// Validated identity/legality input; normal unmodified base stats do not scale with item level.
    pub item_level: u32,
    pub brutality: bool,
    pub resistance_penalty: f64,
    pub quests: SparkQuestRewards,
    pub enemy_armour: f64,
    pub enemy_evasion: f64,
    pub enemy_fire_resistance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaceOutput {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
    pub life: f64,
    pub mana: f64,
    pub energy_shield: f64,
    pub armour: f64,
    pub evasion: f64,
    pub fire_resistance: f64,
    pub cold_resistance: f64,
    pub lightning_resistance: f64,
    pub chaos_resistance: f64,
    pub accuracy: f64,
    pub hit_chance: f64,
    /// PoB's per-hand AverageHit; top-level AverageHit is absent for this attack.
    pub main_hand_average_hit: f64,
    pub average_damage: f64,
    pub hit_dps: f64,
    pub attack_rate: f64,
    pub crit_chance: f64,
    pub crit_multiplier: f64,
    pub weapon_physical_minimum: f64,
    pub weapon_physical_maximum: f64,
    pub weapon_fire_minimum: f64,
    pub weapon_fire_maximum: f64,
    /// Ordinary hit averages after mitigation, before accuracy and crit weighting.
    pub physical_hit_average: f64,
    pub fire_hit_average: f64,
    pub effective_enemy_fire_resistance: f64,
    pub effective_enemy_evasion: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaceError(pub &'static str);
impl fmt::Display for MaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for MaceError {}

/// Legacy profile attributes with no passive modifiers.
pub const DEFAULT_CHARACTER: CharacterInput = CharacterInput {
    attributes: CharacterAttributes {
        strength: DATA.strength,
        dexterity: DATA.dexterity,
        intelligence: DATA.intelligence,
    },
    modifiers: CharacterModifiers::NONE,
};

/// Evaluate the original Warrior/no-passive profile without parsing, Lua or I/O.
pub fn evaluate(input: &MaceInput) -> Result<MaceOutput, MaceError> {
    evaluate_with_character(input, &DEFAULT_CHARACTER)
}

/// Evaluate explicit resolved class attributes and admitted entrance modifiers.
/// The caller must validate the full document and source allocation independently.
pub fn evaluate_with_character(
    input: &MaceInput,
    character: &CharacterInput,
) -> Result<MaceOutput, MaceError> {
    character.validate().map_err(|error| MaceError(error.0))?;
    let attributes = character.attributes;
    let modifiers = character.modifiers;
    if !(1..=100).contains(&input.character_level) {
        return Err(MaceError("Mace profile character level must be 1..100"));
    }
    if !(1..=100).contains(&input.item_level) || input.quality > 20 {
        return Err(MaceError(
            "Mace profile item level must be 1..100 and quality 0..20",
        ));
    }
    if !input.resistance_penalty.is_finite() || !(-60.0..=0.0).contains(&input.resistance_penalty) {
        return Err(MaceError(
            "Mace profile resistance penalty must be finite and -60..0",
        ));
    }
    if !input.enemy_fire_resistance.is_finite()
        || !(-200.0..=200.0).contains(&input.enemy_fire_resistance)
    {
        return Err(MaceError(
            "Mace profile enemy fire resistance must be finite and -200..200",
        ));
    }
    if !input.enemy_armour.is_finite()
        || input.enemy_armour < 0.0
        || !input.enemy_evasion.is_finite()
        || input.enemy_evasion < 0.0
    {
        return Err(MaceError(
            "Mace profile resolved enemy armour and evasion must be finite and nonnegative",
        ));
    }
    let level = f64::from(input.character_level);
    let shared = spark::DATA;
    // Same CalcSetup/CalcPerform/CalcDefence pipeline as Spark, with Warrior attributes.
    let life_base = shared.life_per_level * level
        + shared.initial_life
        + if input.quests.candlemass {
            shared.quest_flat_life
        } else {
            0.0
        }
        + attributes.strength * shared.life_per_strength;
    let mana_base = shared.mana_per_level * level
        + shared.initial_mana
        + attributes.intelligence * shared.mana_per_intelligence;
    let life_increased = if input.quests.molten_shrine {
        shared.quest_life_increased
    } else {
        0.0
    };
    let mana_increased = if input.quests.silent_hall {
        shared.quest_mana_increased
    } else {
        0.0
    };
    let life = round_to_integer(life_base * (1.0 + life_increased / 100.0)).max(1.0);
    let mana = round_to_integer(mana_base * (1.0 + mana_increased / 100.0)).max(1.0);
    let resistance = |quest| {
        (input.resistance_penalty
            + if quest {
                shared.quest_elemental_resistance
            } else {
                0.0
            })
        .trunc()
        .clamp(shared.resistance_floor, shared.player_resistance_cap)
    };
    let weapon = input.weapon.data();
    // Classes/Item: physical quality applies locally, rounding each endpoint.
    // Item level does not enter these ordinary unmodified weapon base calculations.
    let quality_multiplier = 1.0 + f64::from(input.quality) / 100.0;
    let weapon_physical_minimum = round_to_integer(weapon.physical_minimum * quality_multiplier);
    let weapon_physical_maximum = round_to_integer(weapon.physical_maximum * quality_multiplier);
    // calcDamage rounds again after damage modifiers, before critical scaling/armour.
    let more = if input.brutality {
        1.0 + DATA.brutality_physical_more / 100.0
    } else {
        1.0
    };
    let increased =
        1.0 + (modifiers.attack_damage_increased + modifiers.melee_damage_increased) / 100.0;
    let physical_minimum = round_to_integer(weapon_physical_minimum * increased * more);
    let physical_maximum = round_to_integer(weapon_physical_maximum * increased * more);
    let (fire_minimum, fire_maximum) = if input.brutality {
        (0.0, 0.0)
    } else {
        (
            round_to_integer(weapon.fire_minimum * increased),
            round_to_integer(weapon.fire_maximum * increased),
        )
    };
    // CalcSetup's level multiplier carries a negative one-level base adjustment;
    // CalcPerform adds the dexterity bonus before CalcOffence floors accuracy.
    let accuracy = (DATA.accuracy_per_level * level - DATA.accuracy_per_level
        + attributes.dexterity * DATA.accuracy_per_dexterity)
        .floor()
        .max(0.0);
    let enemy_evasion = round_to_integer(input.enemy_evasion).max(0.0);
    let hit = hit_chance(enemy_evasion, accuracy, false);
    // A critical attack rolls accuracy twice; a failed second check becomes a normal hit.
    let crit_chance = round_to_integer(weapon.critical_chance * 100.0) / 100.0 * hit / 100.0;
    let crit_multiplier = 1.0 + shared.critical_damage_bonus / 100.0;
    let enemy_resistance = input
        .enemy_fire_resistance
        .clamp(shared.resistance_floor, shared.enemy_resistance_cap);
    let fire_effective_multiplier = 1.0 - enemy_resistance / 100.0;
    let damage_pass = |critical_multiplier: f64| {
        let physical_average = physical_minimum * critical_multiplier / 2.0
            + physical_maximum * critical_multiplier / 2.0;
        let physical_reduction =
            armour_reduction_percent(input.enemy_armour, physical_average, PINNED_CONSTANTS)
                .clamp(-100.0, DATA.enemy_physical_reduction_cap);
        let physical = physical_average * (1.0 - physical_reduction / 100.0);
        let fire = (fire_minimum * critical_multiplier / 2.0
            + fire_maximum * critical_multiplier / 2.0)
            * fire_effective_multiplier;
        (physical, fire)
    };
    // Crit and ordinary damage must have separate armour reductions: using the
    // ordinary reduction on the combined average incorrectly depresses crit damage.
    let (critical_physical, critical_fire) = damage_pass(crit_multiplier);
    let (physical, fire) = damage_pass(1.0);
    let total_hit_average = physical + fire;
    let total_crit_average = critical_physical + critical_fire;
    let main_hand_average_hit =
        total_hit_average * (1.0 - crit_chance / 100.0) + total_crit_average * crit_chance / 100.0;
    let average_damage = main_hand_average_hit * hit / 100.0;
    let base_time = 1.0 / weapon.attack_rate;
    let speed_multiplier =
        round_to_integer((1.0 + modifiers.skill_speed_increased / 100.0) * 100.0) / 100.0;
    let attack_rate = 1.0 / (base_time / speed_multiplier);
    let hit_dps = average_damage * attack_rate;
    Ok(MaceOutput {
        strength: attributes.strength,
        dexterity: attributes.dexterity,
        intelligence: attributes.intelligence,
        life,
        mana,
        energy_shield: round_to_integer(modifiers.energy_shield_flat).max(0.0),
        armour: round_to_integer(modifiers.armour_flat).max(0.0),
        evasion: round_to_integer(BASE_EVASION + modifiers.evasion_flat).max(0.0),
        fire_resistance: resistance(input.quests.blackjaw),
        cold_resistance: resistance(input.quests.beira),
        lightning_resistance: resistance(input.quests.garukhan),
        chaos_resistance: 0.0,
        accuracy,
        hit_chance: hit,
        main_hand_average_hit,
        average_damage,
        hit_dps,
        attack_rate,
        crit_chance,
        crit_multiplier,
        weapon_physical_minimum,
        weapon_physical_maximum,
        weapon_fire_minimum: weapon.fire_minimum,
        weapon_fire_maximum: weapon.fire_maximum,
        physical_hit_average: physical,
        fire_hit_average: fire,
        effective_enemy_fire_resistance: enemy_resistance,
        effective_enemy_evasion: enemy_evasion,
    })
}

/// Raw normal-monster table values. Host encounter resolution handles PoB's
/// configured level bounds and any boss multipliers before constructing MaceInput.
pub fn monster_evasion(level: u32) -> Result<f64, MaceError> {
    let index = level
        .checked_sub(1)
        .ok_or(MaceError("Monster table level must be 1..100"))?;
    MONSTER_EVASION
        .get(index as usize)
        .copied()
        .ok_or(MaceError("Monster table level must be 1..100"))
}
pub fn monster_armour(level: u32) -> Result<f64, MaceError> {
    let index = level
        .checked_sub(1)
        .ok_or(MaceError("Monster table level must be 1..100"))?;
    MONSTER_ARMOUR
        .get(index as usize)
        .copied()
        .ok_or(MaceError("Monster table level must be 1..100"))
}
const MONSTER_EVASION: [f64; 100] = [
    24.0, 30.0, 36.0, 43.0, 49.0, 56.0, 63.0, 70.0, 77.0, 84.0, 91.0, 98.0, 105.0, 113.0, 120.0,
    128.0, 136.0, 144.0, 152.0, 160.0, 168.0, 176.0, 185.0, 193.0, 202.0, 211.0, 220.0, 229.0,
    238.0, 247.0, 257.0, 266.0, 276.0, 286.0, 296.0, 306.0, 316.0, 326.0, 337.0, 347.0, 358.0,
    369.0, 380.0, 391.0, 403.0, 414.0, 426.0, 438.0, 449.0, 462.0, 474.0, 486.0, 499.0, 511.0,
    524.0, 537.0, 551.0, 564.0, 578.0, 591.0, 605.0, 619.0, 634.0, 648.0, 663.0, 677.0, 692.0,
    708.0, 723.0, 738.0, 754.0, 770.0, 786.0, 803.0, 819.0, 836.0, 853.0, 870.0, 887.0, 905.0,
    923.0, 941.0, 959.0, 977.0, 996.0, 1015.0, 1034.0, 1053.0, 1073.0, 1093.0, 1113.0, 1133.0,
    1154.0, 1174.0, 1195.0, 1217.0, 1238.0, 1260.0, 1282.0, 1304.0,
];

const MONSTER_ARMOUR: [f64; 100] = [
    3.0, 6.0, 8.0, 10.0, 13.0, 16.0, 19.0, 22.0, 26.0, 30.0, 34.0, 39.0, 43.0, 49.0, 54.0, 60.0,
    67.0, 73.0, 81.0, 89.0, 97.0, 106.0, 116.0, 126.0, 137.0, 149.0, 161.0, 174.0, 189.0, 204.0,
    220.0, 237.0, 255.0, 274.0, 295.0, 317.0, 340.0, 364.0, 391.0, 418.0, 448.0, 479.0, 512.0,
    547.0, 585.0, 624.0, 666.0, 711.0, 758.0, 808.0, 861.0, 917.0, 976.0, 1039.0, 1105.0, 1176.0,
    1250.0, 1329.0, 1412.0, 1500.0, 1594.0, 1692.0, 1796.0, 1906.0, 2023.0, 2146.0, 2276.0, 2413.0,
    2558.0, 2712.0, 2874.0, 3044.0, 3225.0, 3416.0, 3617.0, 3829.0, 4053.0, 4290.0, 4540.0, 4803.0,
    5081.0, 5375.0, 5684.0, 6011.0, 6355.0, 6718.0, 7101.0, 7505.0, 7930.0, 8379.0, 8852.0, 9351.0,
    9877.0, 10431.0, 11015.0, 11630.0, 12279.0, 12962.0, 13682.0, 14441.0,
];

/// Normalized full source hashes for this versioned profile's data and translated branches.
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
        path: "src/Data/Bases/mace.lua",
        sha256: "d3bb896069429fcff68d4e04ff517e49ad290cfc70b26fdbe8db3406c8392fec",
    },
    SourceFile {
        path: "src/Data/Skills/other.lua",
        sha256: "fd3170695068ef909101b02f21b50df73ad8af0785229d80f9a1727d570c1962",
    },
    SourceFile {
        path: "src/Data/Skills/sup_str.lua",
        sha256: "70a6903c5d22ca7677dd9cb6284b54f15272dfb8e2036032a83fe0488eece6a3",
    },
    SourceFile {
        path: "src/Data/SkillStatMap.lua",
        sha256: "8ba52caed40a2104b148f1f8d142e2dfce9ba2811d20240e110abb02b1945675",
    },
    SourceFile {
        path: "src/Classes/Item.lua",
        sha256: "97341d95bcc0863280fcf60e68af9459664a5ef06f588aaa0c8db1908f85f534",
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
    SourceFile {
        path: "src/Modules/CalcTools.lua",
        sha256: "83bdbea3790a49ef05fe1050acf1d489cf8cac90865ba31f7bc4a7a5abefcd2c",
    },
];
