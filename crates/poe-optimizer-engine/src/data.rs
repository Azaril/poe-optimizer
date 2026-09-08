//! Compile one validated configuration snapshot into immutable numeric inputs.
//!
//! Only operation semantics and capability keys live here. Balance values, class
//! attributes, item bases, monster tables and owned passive effects come from the
//! supplied snapshot. Compilation performs no I/O and creates no build results.
use crate::{
    character::{CharacterAttributes, CharacterInput, CharacterModifiers},
    defence::DefenceConstants,
    mace::{MaceData, MaceError, MaceWeapon, MaceWeaponData},
    mace_supports::{MaceSupportCatalog, PreparedMaceSupports},
    spark::SparkData,
};
use poe_optimizer_data::{
    game_data::{self, DataIdentity, DefenceData, GameDataSnapshot, PassiveStat},
    tree_data::TREE_PATH,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::{Arc, OnceLock},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataError(pub String);
impl fmt::Display for GameDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "incompatible native game data: {}", self.0)
    }
}
impl Error for GameDataError {}

/// Shared game data compiled once at host setup. No mutable per-build state.
#[derive(Debug)]
pub struct CompiledGameData {
    snapshot: Arc<GameDataSnapshot>,
    spark: SparkData,
    mace: MaceData,
    weapon_indices: [usize; 2],
    supports: MaceSupportCatalog,
    passives: BTreeMap<(u32, u32), BTreeMap<String, CharacterModifiers>>,
}
impl CompiledGameData {
    pub fn compile(snapshot: Arc<GameDataSnapshot>) -> Result<Self, GameDataError> {
        let package = snapshot.package();
        if package.manifest.game != "poe2"
            || package.manifest.schema_version != game_data::SCHEMA_VERSION
            || package.manifest.semantics_version != game_data::SEMANTICS_VERSION
        {
            return Err(GameDataError(
                "requires the current PoE2 package schema and native profile operation semantics"
                    .into(),
            ));
        }
        // Keep the reviewed source-tree/operation contract during migration.
        // Content hashes differ for custom balance records, but structural source
        // and operation versions cannot silently cross this compatibility seam.
        let source = &snapshot.tree().source;
        let tree_hash = source.source_files_sha256.get(TREE_PATH);
        let matching_tree = [crate::spark::SOURCE_FILES, crate::mace::SOURCE_FILES]
            .into_iter()
            .all(|files| {
                files.iter().any(|file| {
                    file.path == TREE_PATH && tree_hash.map(String::as_str) == Some(file.sha256)
                })
            });
        if source.upstream_revision != crate::UPSTREAM_REVISION
            || source.tree_version != crate::spark::TREE_VERSION
            || source.tree_version != crate::mace::TREE_VERSION
            || !matching_tree
        {
            return Err(GameDataError(
                "data and calculation source revisions, tree versions and tree hashes must agree"
                    .into(),
            ));
        }
        let character = &package.character;
        let quests = &package.quests;
        let defence = &package.defence;
        let default_spark = snapshot
            .tree()
            .class(package.spark.default_class_id)
            .map_err(|error| GameDataError(error.to_string()))?;
        let default_mace = snapshot
            .tree()
            .class(package.mace.default_class_id)
            .map_err(|error| GameDataError(error.to_string()))?;
        let spark = SparkData {
            strength: f64::from(default_spark.base_strength),
            dexterity: f64::from(default_spark.base_dexterity),
            intelligence: f64::from(default_spark.base_intelligence),
            life_per_level: character.life_per_level,
            initial_life: character.initial_life,
            mana_per_level: character.mana_per_level,
            initial_mana: character.initial_mana,
            life_per_strength: character.life_per_strength,
            mana_per_intelligence: character.mana_per_intelligence,
            lightning_minimum: package.spark.lightning_minimum,
            lightning_maximum: package.spark.lightning_maximum,
            cast_time: package.spark.cast_time,
            critical_chance: package.spark.critical_chance,
            critical_damage_bonus: character.critical_damage_bonus,
            quest_flat_life: quests.flat_life,
            quest_life_increased: quests.life_increased,
            quest_mana_increased: quests.mana_increased,
            quest_elemental_resistance: quests.elemental_resistance,
            resistance_floor: defence.resistance_floor,
            player_resistance_cap: defence.player_resistance_cap,
            enemy_resistance_cap: defence.enemy_resistance_cap,
        };
        let mace = MaceData {
            strength: f64::from(default_mace.base_strength),
            dexterity: f64::from(default_mace.base_dexterity),
            intelligence: f64::from(default_mace.base_intelligence),
            accuracy_per_level: character.accuracy_per_level,
            accuracy_per_dexterity: character.accuracy_per_dexterity,
            enemy_physical_reduction_cap: defence.enemy_physical_reduction_cap,
        };
        if package.weapons.len() != 2 {
            return Err(GameDataError(
                "the closed Mace operation requires exactly two weapon capability slots".into(),
            ));
        }
        let find_weapon = |id: &str| {
            package
                .weapons
                .iter()
                .position(|weapon| weapon.id == id)
                .ok_or_else(|| {
                    GameDataError(format!("missing supported weapon capability slot {id}"))
                })
        };
        let weapon_indices = [find_weapon("wooden_club")?, find_weapon("smithing_hammer")?];
        if package.monsters.armour.len() != 100 || package.monsters.evasion.len() != 100 {
            return Err(GameDataError(
                "monster tables must cover every supported level 1..100".into(),
            ));
        }
        let mut passives: BTreeMap<(u32, u32), BTreeMap<String, CharacterModifiers>> =
            BTreeMap::new();
        for passive in &package.passive_effects {
            let mut modifiers = CharacterModifiers::NONE;
            for effect in &passive.effects {
                let field = match effect.stat {
                    PassiveStat::ArmourFlat => &mut modifiers.armour_flat,
                    PassiveStat::EvasionFlat => &mut modifiers.evasion_flat,
                    PassiveStat::EnergyShieldFlat => &mut modifiers.energy_shield_flat,
                    PassiveStat::SkillSpeedIncreased => &mut modifiers.skill_speed_increased,
                    PassiveStat::SpellDamageIncreased => &mut modifiers.spell_damage_increased,
                    PassiveStat::AttackDamageIncreased => &mut modifiers.attack_damage_increased,
                    PassiveStat::MeleeDamageIncreased => &mut modifiers.melee_damage_increased,
                    PassiveStat::ProjectileDamageIncreased => {
                        &mut modifiers.projectile_damage_increased
                    }
                    PassiveStat::MinionDamageIncreased => &mut modifiers.minion_damage_increased,
                    PassiveStat::FireResistanceFlat => &mut modifiers.fire_resistance_flat,
                    PassiveStat::ColdResistanceFlat => &mut modifiers.cold_resistance_flat,
                    PassiveStat::LightningResistanceFlat => {
                        &mut modifiers.lightning_resistance_flat
                    }
                    PassiveStat::ChaosResistanceFlat => &mut modifiers.chaos_resistance_flat,
                    PassiveStat::ElementalResistanceFlat => {
                        &mut modifiers.elemental_resistance_flat
                    }
                };
                *field += effect.value;
            }
            CharacterInput {
                attributes: CharacterAttributes::default(),
                modifiers,
            }
            .validate()
            .map_err(|error| GameDataError(error.to_string()))?;
            if passives
                .entry((passive.class_id, passive.physical_node_id))
                .or_default()
                .insert(passive.ascendancy_id.clone().unwrap_or_default(), modifiers)
                .is_some()
            {
                return Err(GameDataError(
                    "duplicate class/ascendancy/physical passive effect record".into(),
                ));
            }
        }
        let supports = MaceSupportCatalog::compile(package)?;
        Ok(Self {
            supports,
            snapshot,
            spark,
            mace,
            weapon_indices,
            passives,
        })
    }

    /// Convenience composition using the same loader/compiler as external data.
    /// An explicitly supplied dataset never falls back to this package.
    pub fn bundled() -> Result<Arc<Self>, GameDataError> {
        bundled_arc().cloned()
    }
    pub fn snapshot(&self) -> &GameDataSnapshot {
        &self.snapshot
    }
    pub fn identity(&self) -> &DataIdentity {
        self.snapshot.identity()
    }
    pub fn spark(&self) -> &SparkData {
        &self.spark
    }
    pub fn mace(&self) -> &MaceData {
        &self.mace
    }
    /// Resolve a canonical data-key loadout once during build preparation.
    pub fn mace_support_loadout(
        &self,
        keys: &[String],
    ) -> Result<&PreparedMaceSupports, MaceError> {
        self.supports.loadout(keys)
    }
    pub(crate) fn legacy_mace_supports(
        &self,
        brutality: bool,
    ) -> Result<&PreparedMaceSupports, MaceError> {
        self.supports.legacy(brutality)
    }
    pub(crate) fn owns_mace_supports(&self, supports: &PreparedMaceSupports) -> bool {
        self.supports.contains(supports)
    }
    pub fn defence(&self) -> &DefenceData {
        &self.snapshot.package().defence
    }
    pub fn defence_constants(&self) -> DefenceConstants {
        DefenceConstants {
            armour_ratio: self.defence().armour_ratio,
            deflection_chance_cap: self.defence().deflection_chance_cap,
        }
    }
    pub fn default_spark_character(&self) -> CharacterInput {
        CharacterInput {
            attributes: CharacterAttributes {
                strength: self.spark.strength,
                dexterity: self.spark.dexterity,
                intelligence: self.spark.intelligence,
            },
            modifiers: CharacterModifiers::NONE,
        }
    }
    pub fn default_mace_character(&self) -> CharacterInput {
        CharacterInput {
            attributes: CharacterAttributes {
                strength: self.mace.strength,
                dexterity: self.mace.dexterity,
                intelligence: self.mace.intelligence,
            },
            modifiers: CharacterModifiers::NONE,
        }
    }
    pub fn entrance_modifiers(
        &self,
        class_id: u32,
        physical_id: u32,
    ) -> Option<&CharacterModifiers> {
        self.passive_modifiers(class_id, None, physical_id)
    }
    /// Resolve a compiled record with explicit ordinary/ascendancy ownership.
    /// Physical allocation identity is preserved even for class-specific effects.
    pub fn passive_modifiers(
        &self,
        class_id: u32,
        ascendancy_id: Option<&str>,
        physical_id: u32,
    ) -> Option<&CharacterModifiers> {
        // Validated ascendancy identifiers are nonempty, leaving the empty key
        // for ordinary ownership. Borrowed lookup allocates no per-build strings.
        if ascendancy_id == Some("") {
            return None;
        }
        self.passives
            .get(&(class_id, physical_id))?
            .get(ascendancy_id.unwrap_or(""))
    }
    pub fn weapon(&self, weapon: MaceWeapon) -> MaceWeaponData<'_> {
        let slot = match weapon {
            MaceWeapon::WoodenClub => 0,
            MaceWeapon::SmithingHammer => 1,
        };
        let record = &self.snapshot.package().weapons[self.weapon_indices[slot]];
        MaceWeaponData {
            name: &record.name,
            physical_minimum: record.physical_minimum,
            physical_maximum: record.physical_maximum,
            fire_minimum: record.fire_minimum,
            fire_maximum: record.fire_maximum,
            attack_rate: record.attack_rate,
            critical_chance: record.critical_chance,
            required_strength: record.requirements.attributes.strength,
        }
    }
    pub fn monster_evasion(&self, level: u32) -> Result<f64, MaceError> {
        Self::monster(&self.snapshot.package().monsters.evasion, level)
    }
    pub fn monster_armour(&self, level: u32) -> Result<f64, MaceError> {
        Self::monster(&self.snapshot.package().monsters.armour, level)
    }
    fn monster(table: &[f64], level: u32) -> Result<f64, MaceError> {
        level
            .checked_sub(1)
            .and_then(|index| table.get(index as usize))
            .copied()
            .ok_or(MaceError("Monster table level must be 1..100"))
    }
}
fn bundled_arc() -> Result<&'static Arc<CompiledGameData>, GameDataError> {
    static REVIEWED: OnceLock<Result<Arc<CompiledGameData>, GameDataError>> = OnceLock::new();
    REVIEWED
        .get_or_init(|| {
            let snapshot =
                game_data::bundled_snapshot().map_err(|error| GameDataError(error.to_string()))?;
            CompiledGameData::compile(Arc::new(snapshot)).map(Arc::new)
        })
        .as_ref()
        .map_err(Clone::clone)
}
pub(crate) fn bundled_reference() -> Result<&'static CompiledGameData, GameDataError> {
    bundled_arc().map(Arc::as_ref)
}
