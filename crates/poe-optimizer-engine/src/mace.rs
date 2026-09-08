//! Closed native Mace Strike profile with one normal mace and zero to two
//! configured level-one quality-zero supports. Hosts validate the complete document:
//! explicit class attributes and admitted owned passive effects, no other equipment,
//! supports or external modifiers.
//! Enemy values are resolved by the host; this kernel does not select encounters.

use crate::character::CharacterInput;
use crate::data::CompiledGameData;
use crate::mace_supports::PreparedMaceSupports;
use crate::weapon::PreparedWeaponStats;
use crate::{
    defence::{armour_reduction_percent, hit_chance_with_data, round_to_integer},
    spark::{SourceFile, SparkQuestRewards},
};
use std::{error::Error, fmt};

pub const PROFILE_ID: &str = "poe2-mace-strike-actor-resources-v6";
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
pub struct MaceWeaponData<'a> {
    pub name: &'a str,
    pub physical_minimum: f64,
    pub physical_maximum: f64,
    pub fire_minimum: f64,
    pub fire_maximum: f64,
    pub attack_rate: f64,
    pub critical_chance: f64,
    pub required_strength: u32,
}
impl MaceWeapon {
    /// Convenience lookup in the reviewed package. Instance evaluators use
    /// `CompiledGameData::weapon` so separate datasets remain isolated.
    pub fn data(self) -> MaceWeaponData<'static> {
        crate::data::bundled_reference()
            .expect("reviewed game-data package")
            .weapon(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaceData {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
    pub accuracy_per_level: f64,
    pub accuracy_per_dexterity: f64,
    pub enemy_physical_reduction_cap: f64,
}
/// Reviewed default parameters, loaded through the common package compiler.
pub fn data() -> &'static MaceData {
    crate::data::bundled_reference()
        .expect("reviewed game-data package")
        .mace()
}

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
    /// Legacy convenience selector, consulted only by the old evaluate wrappers.
    /// Explicit prepared evaluation ignores this field and uses its bound loadout.
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
    pub spirit: f64,
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

/// Reviewed default class attributes with no passive modifiers.
pub fn default_character() -> CharacterInput {
    crate::data::bundled_reference()
        .expect("reviewed game-data package")
        .default_mace_character()
}

/// Convenience evaluation using the reviewed package through the common compiler.
pub fn evaluate(input: &MaceInput) -> Result<MaceOutput, MaceError> {
    evaluate_with_character(input, &default_character())
}

/// Convenience evaluation using the reviewed package and explicit character inputs.
pub fn evaluate_with_character(
    input: &MaceInput,
    character: &CharacterInput,
) -> Result<MaceOutput, MaceError> {
    let data = crate::data::bundled_reference()
        .map_err(|_| MaceError("The reviewed game-data package failed compilation"))?;
    evaluate_with_data(input, character, data)
}

/// Calculate using only the explicitly supplied immutable dataset and resolved inputs.
pub fn evaluate_with_data(
    input: &MaceInput,
    character: &CharacterInput,
    compiled: &CompiledGameData,
) -> Result<MaceOutput, MaceError> {
    let supports = compiled.legacy_mace_supports(input.brutality)?;
    evaluate_with_supports(input, character, compiled, supports)
}

/// Calculate with an immutable prepared support loadout. The legacy
/// `input.brutality` selector is deliberately not part of this path.
/// A loadout prepared by another compiled dataset is rejected before calculation.
pub fn evaluate_with_supports(
    input: &MaceInput,
    character: &CharacterInput,
    compiled: &CompiledGameData,
    supports: &PreparedMaceSupports,
) -> Result<MaceOutput, MaceError> {
    let weapon =
        compiled.prepare_mace_weapon(input.weapon, input.quality, input.item_level, &[])?;
    evaluate_with_components(input, character, compiled, &weapon, supports)
}

/// Fresh calculation using explicitly prepared weapon and support components.
/// The legacy brutality selector is ignored; weapon key/quality/item-level must
/// still match the validated input so stale components cannot change a candidate.
pub fn evaluate_with_components(
    input: &MaceInput,
    character: &CharacterInput,
    compiled: &CompiledGameData,
    weapon: &PreparedWeaponStats,
    supports: &PreparedMaceSupports,
) -> Result<MaceOutput, MaceError> {
    let actor = compiled
        .prepare_actor_resources(
            input.character_level,
            compiled.actor_quest_selection(input.quests),
            character,
            &[],
        )
        .map_err(|error| MaceError(error.0))?;
    evaluate_with_actor(input, character, compiled, weapon, supports, &actor)
}

/// Calculate using separately prepared actor, weapon and support components.
/// All components retain exact compiled-data binding and source input identity.
pub fn evaluate_with_actor(
    input: &MaceInput,
    character: &CharacterInput,
    compiled: &CompiledGameData,
    weapon: &PreparedWeaponStats,
    supports: &PreparedMaceSupports,
    actor: &crate::actor::PreparedActorResources,
) -> Result<MaceOutput, MaceError> {
    actor
        .validate_profile(compiled, input.character_level, input.quests, character)
        .map_err(|error| MaceError(error.0))?;
    let actor = actor.values();
    if !compiled.owns_weapon(weapon) {
        return Err(MaceError(
            "Prepared Mace weapon belongs to a different compiled dataset",
        ));
    }
    if weapon.weapon() != input.weapon
        || weapon.quality() != input.quality
        || weapon.item_level() != input.item_level
    {
        return Err(MaceError(
            "Prepared Mace weapon differs from the selected input identity",
        ));
    }
    if !compiled.owns_mace_supports(supports) {
        return Err(MaceError(
            "Prepared Mace supports belong to a different compiled dataset",
        ));
    }
    let data = compiled.mace();
    let rules = &compiled.snapshot().package().character;
    character.validate().map_err(|error| MaceError(error.0))?;
    let attributes = actor.attributes;
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
    let shared = compiled.spark();
    let life = actor.life;
    let mana = actor.mana;
    let resistance = crate::resistance::calculate(
        &modifiers,
        input.resistance_penalty,
        [
            input.quests.blackjaw,
            input.quests.beira,
            input.quests.garukhan,
        ]
        .map(|enabled| {
            if enabled {
                shared.quest_elemental_resistance
            } else {
                0.0
            }
        }),
        compiled.defence(),
    );
    let weapon = weapon.stats();
    // Local flat/INC/quality assembly, local rate and critical rounding were
    // prepared once from the selected item's exact data rules and ordered rolls.
    let weapon_physical_minimum = weapon.physical_minimum;
    let weapon_physical_maximum = weapon.physical_maximum;
    // calcDamage rounds again after damage modifiers, before critical scaling/armour.
    let increased =
        1.0 + (modifiers.attack_damage_increased + modifiers.melee_damage_increased) / 100.0;
    let physical_increased = 1.0
        + (modifiers.attack_damage_increased
            + modifiers.melee_damage_increased
            + supports.physical_increased)
            / 100.0;
    let (physical_minimum, physical_maximum) = if supports.disable_physical {
        (0.0, 0.0)
    } else {
        (
            round_to_integer(weapon_physical_minimum * physical_increased * supports.physical_more),
            round_to_integer(weapon_physical_maximum * physical_increased * supports.physical_more),
        )
    };
    let (fire_minimum, fire_maximum) = if supports.disable_fire {
        (0.0, 0.0)
    } else {
        (
            round_to_integer(weapon.fire_minimum * increased),
            round_to_integer(weapon.fire_maximum * increased),
        )
    };
    let accuracy = actor.accuracy;
    let enemy_evasion = round_to_integer(input.enemy_evasion).max(0.0);
    let hit = hit_chance_with_data(enemy_evasion, accuracy, false, compiled.defence());
    // A critical attack rolls accuracy twice; a failed second check becomes a normal hit.
    let crit_chance =
        crate::offence::capped_critical_chance(weapon.critical_chance, rules.critical_chance_cap)
            * hit
            / 100.0;
    let crit_multiplier = 1.0 + shared.critical_damage_bonus / 100.0;
    let enemy_resistance = input
        .enemy_fire_resistance
        .clamp(shared.resistance_floor, shared.enemy_resistance_cap);
    let fire_effective_multiplier = 1.0 - enemy_resistance / 100.0;
    let damage_pass = |critical_multiplier: f64| {
        let physical_average = physical_minimum * critical_multiplier / 2.0
            + physical_maximum * critical_multiplier / 2.0;
        let physical_reduction = armour_reduction_percent(
            input.enemy_armour,
            physical_average,
            compiled.defence_constants(),
        )
        .clamp(-100.0, data.enemy_physical_reduction_cap);
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
    // ModDB rounds each MORE name first, then CalcOffence combines all INC
    // with that product and rounds the resulting speed multiplier to two places.
    let speed_multiplier = round_to_integer(
        (1.0 + (modifiers.skill_speed_increased + supports.speed_increased) / 100.0)
            * supports.speed_more
            * 100.0,
    ) / 100.0;
    let attack_rate = 1.0 / (base_time / speed_multiplier);
    let hit_dps = average_damage * attack_rate;
    Ok(MaceOutput {
        strength: attributes.strength,
        dexterity: attributes.dexterity,
        intelligence: attributes.intelligence,
        life,
        mana,
        spirit: actor.spirit,
        energy_shield: round_to_integer(modifiers.energy_shield_flat).max(0.0),
        armour: round_to_integer(modifiers.armour_flat).max(0.0),
        evasion: round_to_integer(rules.base_evasion + modifiers.evasion_flat).max(0.0),
        fire_resistance: resistance.fire,
        cold_resistance: resistance.cold,
        lightning_resistance: resistance.lightning,
        chaos_resistance: resistance.chaos,
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

/// Convenience normal-monster lookup in the reviewed package.
/// Injected evaluators use their own `CompiledGameData` lookup methods.
pub fn monster_evasion(level: u32) -> Result<f64, MaceError> {
    crate::data::bundled_reference()
        .map_err(|_| MaceError("The reviewed game-data package failed compilation"))?
        .monster_evasion(level)
}
pub fn monster_armour(level: u32) -> Result<f64, MaceError> {
    crate::data::bundled_reference()
        .map_err(|_| MaceError("The reviewed game-data package failed compilation"))?
        .monster_armour(level)
}

/// Normalized full source hashes for this versioned profile's data and translated branches.
pub const SOURCE_FILES: &[SourceFile] = &[
    SourceFile {
        path: "src/Data/Skills/sup_dex.lua",
        sha256: "de90f36c8134908ff4f86bc9adf20564c9aaa9da2d36294679ca462314ab0eac",
    },
    SourceFile {
        path: "src/Data/Global.lua",
        sha256: "1482a574c9b8a06a4734577e549bd87917e5cd631523708d6f2c2fa62d62db2c",
    },
    SourceFile {
        path: "src/Classes/ModDB.lua",
        sha256: "1417e208c10466395d67a52ec9f3f52719ec16e50760ef06760fb22207a1eab6",
    },
    SourceFile {
        path: "src/Classes/ModStore.lua",
        sha256: "432bcffa24f1f2a232499d0a01b5ba01fe4adc259318f19f16cdddd99afe8c62",
    },
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
