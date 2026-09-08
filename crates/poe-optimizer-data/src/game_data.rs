//! Bounded, portable decoding of complete, explicitly partial game-data packages.
//! The host supplies bytes and trust policy; no runtime I/O or process-global selection.
use crate::bundled::BundledClassTree;
pub use poe_optimizer_core::data::DataIdentity;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 3;
pub const SEMANTICS_VERSION: &str = "poe2-native-profiles-v3";
const PACKAGE_BYTES: &[u8] = include_bytes!("../data/game-data.json");
const SECTIONS: &[&str] = &[
    "tree",
    "character",
    "quests",
    "spark",
    "mace",
    "weapons",
    "defence",
    "monsters",
    "encounters",
    "passive_effects",
];

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid game-data package: {0}")]
pub struct GameDataError(pub String);
type Result<T> = std::result::Result<T, GameDataError>;
fn error(value: impl std::fmt::Display) -> GameDataError {
    GameDataError(value.to_string())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Expected digest is host-supplied, never accepted from the package itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustPolicy {
    Reviewed { expected_sha256: String },
    AllowCustom,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataTrust {
    Reviewed { expected_sha256: String },
    CustomUnreviewed,
}

#[derive(Debug, Clone, Copy)]
pub struct LoadLimits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_values: usize,
    pub max_string_bytes: usize,
    pub max_effects_per_passive: usize,
}
impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            max_bytes: 2 * 1024 * 1024,
            max_depth: 64,
            max_values: 100_000,
            max_string_bytes: 4096,
            max_effects_per_passive: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDataManifest {
    pub game: String,
    pub release: String,
    pub schema_version: u32,
    pub semantics_version: String,
    pub provenance: BTreeMap<String, String>,
    pub section_sha256: BTreeMap<String, String>,
    /// Explicit limitations, not permission to execute unsupported mechanics.
    pub coverage: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterData {
    pub base_evasion: f64,
    pub critical_damage_bonus: f64,
    pub life_per_level: f64,
    pub initial_life: f64,
    pub mana_per_level: f64,
    pub initial_mana: f64,
    pub life_per_strength: f64,
    pub mana_per_intelligence: f64,
    pub accuracy_per_level: f64,
    pub accuracy_per_dexterity: f64,
    pub minimum_life: f64,
    pub minimum_mana: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestData {
    pub flat_life: f64,
    pub life_increased: f64,
    pub mana_increased: f64,
    pub elemental_resistance: f64,
    pub default_enabled: [bool; 6],
    /// Ordered Candlemass, Molten Shrine, Silent Hall, Beira, Garukhan, Blackjaw.
    pub config_keys: [String; 6],
}
/// Exact per-attribute requirements. Individual sources combine by maximum;
/// support socket costs accumulate by color before that maximum is taken.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeRequirements {
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
}
/// Equip/use requirements for the represented base or level-one gem.
/// `level` is character level required for use, never item level or gem level.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementData {
    pub level: u32,
    pub attributes: AttributeRequirements,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportColor {
    Red,
    Green,
    Blue,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SparkData {
    pub requirements: RequirementData,
    pub skill_id: String,
    pub game_id: String,
    pub variant_id: String,
    pub name: String,
    pub default_class_id: u32,
    pub lightning_minimum: f64,
    pub lightning_maximum: f64,
    pub cast_time: f64,
    pub critical_chance: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportData {
    pub requirements: RequirementData,
    pub color: SupportColor,
    pub skill_id: String,
    pub game_id: String,
    pub variant_id: String,
    pub name: String,
    pub physical_more: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaceData {
    pub requirements: RequirementData,
    /// Cost per red/green/blue support socket in its matching attribute.
    pub support_attribute_costs: AttributeRequirements,
    pub skill_id: String,
    pub game_id: String,
    pub variant_id: String,
    pub name: String,
    pub default_class_id: u32,
    pub brutality: SupportData,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaceWeaponData {
    pub requirements: RequirementData,
    pub id: String,
    pub name: String,
    pub physical_minimum: f64,
    pub physical_maximum: f64,
    pub fire_minimum: f64,
    pub fire_maximum: f64,
    pub attack_rate: f64,
    pub critical_chance: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefenceData {
    pub armour_ratio: f64,
    pub deflection_chance_cap: f64,
    pub hit_accuracy_multiplier: f64,
    pub hit_evasion_multiplier: f64,
    pub hit_chance_floor: f64,
    pub hit_chance_cap: f64,
    pub monster_evasion_multiplier: f64,
    pub monster_accuracy_multiplier: f64,
    pub deflection_rating_multiplier: f64,
    pub deflection_chance_multiplier: f64,
    pub deflection_chance_offset: f64,
    pub deflection_rating_floor: f64,
    pub resistance_floor: f64,
    pub player_resistance_cap: f64,
    pub resistance_maximum_cap: f64,
    pub enemy_resistance_cap: f64,
    pub enemy_physical_reduction_cap: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonsterData {
    pub armour: Vec<f64>,
    pub evasion: Vec<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterData {
    pub default_resistance_penalty: f64,
    pub default_boss: String,
    pub normal_elemental_resistance: f64,
    pub standard_elemental_resistance: f64,
    pub pinnacle_elemental_resistance: f64,
    pub normal_level_cap: u32,
    pub pinnacle_level: u32,
}
/// Closed, versioned operation IDs. Display strings never define numeric behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassiveStat {
    ArmourFlat,
    EvasionFlat,
    EnergyShieldFlat,
    SkillSpeedIncreased,
    SpellDamageIncreased,
    AttackDamageIncreased,
    MeleeDamageIncreased,
    ProjectileDamageIncreased,
    MinionDamageIncreased,
    FireResistanceFlat,
    ColdResistanceFlat,
    LightningResistanceFlat,
    ChaosResistanceFlat,
    ElementalResistanceFlat,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveEffect {
    pub stat: PassiveStat,
    pub value: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveEffects {
    pub class_id: u32,
    pub ascendancy_id: Option<String>,
    pub physical_node_id: u32,
    pub effective_node_id: u32,
    /// Source order is preserved even when aggregation currently commutes.
    pub effects: Vec<PassiveEffect>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDataPackage {
    pub manifest: GameDataManifest,
    pub tree: BundledClassTree,
    pub character: CharacterData,
    pub quests: QuestData,
    pub spark: SparkData,
    pub mace: MaceData,
    pub weapons: Vec<MaceWeaponData>,
    pub defence: DefenceData,
    pub monsters: MonsterData,
    pub encounters: EncounterData,
    pub passive_effects: Vec<PassiveEffects>,
}
impl GameDataPackage {
    /// Bounded producer decoding only: duplicate keys and resource excess reject,
    /// but records and stale section digests remain unvalidated mutable data.
    /// Callers must use GameDataLoader before publishing an evaluation snapshot.
    pub fn decode_for_authoring(bytes: &[u8], limits: &LoadLimits) -> Result<Self> {
        check_limits(bytes, limits)?;
        decode_value(bounded_json(bytes, limits)?)
    }
    /// For package producers and explicit custom edits; these hashes prove internal
    /// consistency only. A host review policy still supplies the expected whole hash.
    pub fn refresh_section_digests(&mut self) -> Result<()> {
        let value = serde_json::to_value(&*self).map_err(error)?;
        self.manifest.section_sha256 = section_digests(&value)?;
        Ok(())
    }
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(error)
    }
}
/// Validated state cannot be mutated after its content identity is calculated.
#[derive(Debug, Clone)]
pub struct GameDataSnapshot {
    identity: DataIdentity,
    trust: DataTrust,
    package: GameDataPackage,
}
impl GameDataSnapshot {
    pub fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    pub fn trust(&self) -> &DataTrust {
        &self.trust
    }
    pub fn package(&self) -> &GameDataPackage {
        &self.package
    }
    pub fn passive_effects(
        &self,
        class_id: u32,
        ascendancy_id: Option<&str>,
        physical_node_id: u32,
    ) -> Option<&PassiveEffects> {
        self.package.passive_effects.iter().find(|record| {
            record.class_id == class_id
                && record.ascendancy_id.as_deref() == ascendancy_id
                && record.physical_node_id == physical_node_id
        })
    }
    pub fn tree(&self) -> &BundledClassTree {
        &self.package.tree
    }
}
pub struct GameDataLoader;
impl GameDataLoader {
    pub fn from_bytes(
        bytes: &[u8],
        trust: &TrustPolicy,
        limits: &LoadLimits,
    ) -> Result<GameDataSnapshot> {
        check_limits(bytes, limits)?;
        let digest = hash(bytes);
        let trust = match trust {
            TrustPolicy::Reviewed { expected_sha256 } => {
                if !valid_digest(expected_sha256) || expected_sha256 != &digest {
                    return Err(error(
                        "package does not match host-supplied reviewed SHA-256",
                    ));
                }
                DataTrust::Reviewed {
                    expected_sha256: expected_sha256.clone(),
                }
            }
            TrustPolicy::AllowCustom => DataTrust::CustomUnreviewed,
        };
        let value = bounded_json(bytes, limits)?;
        let package = decode_value(value)?;
        validate(&package, limits)?;
        let identity = DataIdentity {
            game: package.manifest.game.clone(),
            release: package.manifest.release.clone(),
            schema_version: package.manifest.schema_version,
            content_sha256: digest,
            semantics_version: package.manifest.semantics_version.clone(),
        };
        identity.validate().map_err(error)?;
        Ok(GameDataSnapshot {
            identity,
            trust,
            package,
        })
    }
}
pub fn bundled_package_bytes() -> &'static [u8] {
    PACKAGE_BYTES
}
pub fn bundled_package_sha256() -> &'static str {
    include_str!("../data/game-data.sha256").trim()
}
pub fn bundled_snapshot() -> Result<GameDataSnapshot> {
    GameDataLoader::from_bytes(
        PACKAGE_BYTES,
        &TrustPolicy::Reviewed {
            expected_sha256: bundled_package_sha256().into(),
        },
        &LoadLimits::default(),
    )
}
// Some owned upstream record enums predate deny_unknown_fields. Reject fields
// discarded by their deserializer too, including aliases for integer map keys.
fn decode_value(value: serde_json::Value) -> Result<GameDataPackage> {
    let package = GameDataPackage::deserialize(&value).map_err(error)?;
    reject_discarded_fields(
        "package",
        &value,
        &serde_json::to_value(&package).map_err(error)?,
    )?;
    Ok(package)
}
fn reject_discarded_fields(
    path: &str,
    supplied: &serde_json::Value,
    decoded: &serde_json::Value,
) -> Result<()> {
    match (supplied, decoded) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            for (key, value) in left {
                let child = right
                    .get(key)
                    .ok_or_else(|| error(format!("unknown or ambiguous field {path}.{key}")))?;
                reject_discarded_fields(&format!("{path}.{key}"), value, child)?;
            }
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            if left.len() != right.len() {
                return Err(error(format!("ambiguous array at {path}")));
            }
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                reject_discarded_fields(&format!("{path}[{index}]"), left, right)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn check_limits(bytes: &[u8], limits: &LoadLimits) -> Result<()> {
    if bytes.is_empty() || bytes.len() > limits.max_bytes {
        return Err(error("byte limit exceeded or package empty"));
    }
    if limits.max_depth == 0
        || limits.max_values == 0
        || limits.max_string_bytes == 0
        || limits.max_effects_per_passive == 0
    {
        return Err(error("loader limits must be positive"));
    }
    Ok(())
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn section_digests(value: &serde_json::Value) -> Result<BTreeMap<String, String>> {
    SECTIONS
        .iter()
        .map(|name| {
            Ok((
                (*name).into(),
                hash(&serde_json::to_vec(&value[*name]).map_err(error)?),
            ))
        })
        .collect()
}
fn number(name: &str, value: f64, minimum: f64, maximum: f64) -> Result<()> {
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err(error(format!(
            "{name} must be finite and in {minimum}..={maximum}"
        )));
    }
    Ok(())
}
fn validate(package: &GameDataPackage, limits: &LoadLimits) -> Result<()> {
    let m = &package.manifest;
    if m.schema_version != SCHEMA_VERSION {
        return Err(error("unsupported schema_version"));
    }
    if m.game != "poe2" {
        return Err(error(
            "unsupported game namespace; this package schema admits poe2 only",
        ));
    }
    if m.semantics_version != SEMANTICS_VERSION {
        return Err(error("incompatible semantics_version"));
    }
    if m.release.trim().is_empty() || m.provenance.is_empty() || m.coverage.is_empty() {
        return Err(error(
            "release, provenance and explicit coverage are required",
        ));
    }
    if m.section_sha256 != section_digests(&serde_json::to_value(package).map_err(error)?)? {
        return Err(error(
            "section SHA-256 manifest does not match actual records",
        ));
    }
    // Structural tree migration deliberately remains source-pinned. Configurable
    // balance records cannot silently replace topology or its provenance claims.
    crate::bundled::authenticate_bundle(&package.tree.canonical_bytes().map_err(error)?)
        .map_err(error)?;
    let value = serde_json::to_value(package).map_err(error)?;
    for section in [
        "character",
        "quests",
        "spark",
        "mace",
        "weapons",
        "defence",
        "monsters",
        "encounters",
    ] {
        validate_numbers(section, &value[section])?;
    }
    for class in [
        package.spark.default_class_id,
        package.mace.default_class_id,
    ] {
        package.tree.class(class).map_err(error)?;
    }
    for (name, requirement) in [
        ("Spark", &package.spark.requirements),
        ("Mace Strike", &package.mace.requirements),
        ("Brutality", &package.mace.brutality.requirements),
    ] {
        validate_requirement(name, requirement)?;
    }
    // Support attributes are represented by aggregate color costs. A second,
    // nonzero individual attribute requirement would describe unsupported semantics.
    if package.mace.brutality.requirements.attributes != AttributeRequirements::default() {
        return Err(error(
            "support individual attribute requirements must be zero; use support_attribute_costs",
        ));
    }
    let c = &package.character;
    number("minimum_life", c.minimum_life, 1.0, 1e6)?;
    number("minimum_mana", c.minimum_mana, 1.0, 1e6)?;
    let s = &package.spark;
    number("Spark cast_time", s.cast_time, 0.000001, 1e6)?;
    number("Spark critical_chance", s.critical_chance, 0.0, 100.0)?;
    if s.lightning_minimum > s.lightning_maximum {
        return Err(error("Spark lightning damage endpoints are reversed"));
    }
    let identities = [
        (&s.skill_id, &s.game_id, &s.variant_id, &s.name),
        (
            &package.mace.skill_id,
            &package.mace.game_id,
            &package.mace.variant_id,
            &package.mace.name,
        ),
        (
            &package.mace.brutality.skill_id,
            &package.mace.brutality.game_id,
            &package.mace.brutality.variant_id,
            &package.mace.brutality.name,
        ),
    ];
    for field in 0..4 {
        let values: BTreeSet<_> = identities
            .iter()
            .map(|v| match field {
                0 => v.0,
                1 => v.1,
                2 => v.2,
                _ => v.3,
            })
            .collect();
        if values.len() != identities.len()
            || values.iter().any(|s| {
                s.trim().is_empty() || s.trim() != s.as_str() || s.chars().any(char::is_control)
            })
        {
            return Err(error(
                "skill/support selectors must be nonempty and unambiguous",
            ));
        }
    }
    if package.quests.config_keys.iter().any(|v| {
        !v.starts_with("quest") || v.len() <= 5 || v.trim() != v || v.chars().any(char::is_control)
    }) || package
        .quests
        .config_keys
        .iter()
        .collect::<BTreeSet<_>>()
        .len()
        != 6
    {
        return Err(error(
            "quest config keys must be six unique quest-prefixed selectors",
        ));
    }
    if package.weapons.is_empty() || package.weapons.len() > 128 {
        return Err(error("weapon record count must be 1..128"));
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for w in &package.weapons {
        validate_requirement(&w.name, &w.requirements)?;
        if w.id.trim().is_empty()
            || w.name.trim().is_empty()
            || w.id.trim() != w.id
            || w.name.trim() != w.name
            || w.id.chars().any(char::is_control)
            || w.name.chars().any(char::is_control)
            || !ids.insert(&w.id)
            || !names.insert(&w.name)
        {
            return Err(error(
                "weapon IDs and names must be unique nonempty selectors",
            ));
        }
        number("weapon attack_rate", w.attack_rate, 0.000001, 1e6)?;
        number("weapon critical_chance", w.critical_chance, 0.0, 100.0)?;
        if w.physical_minimum > w.physical_maximum || w.fire_minimum > w.fire_maximum {
            return Err(error("weapon damage endpoints are reversed"));
        }
    }
    let d = &package.defence;
    for (name, value) in [
        ("armour_ratio", d.armour_ratio),
        ("hit_accuracy_multiplier", d.hit_accuracy_multiplier),
        ("hit_evasion_multiplier", d.hit_evasion_multiplier),
        ("monster_accuracy_multiplier", d.monster_accuracy_multiplier),
        (
            "deflection_rating_multiplier",
            d.deflection_rating_multiplier,
        ),
    ] {
        number(name, value, 0.000001, 1e6)?;
    }
    for (name, value) in [
        ("deflection_chance_cap", d.deflection_chance_cap),
        ("hit_chance_floor", d.hit_chance_floor),
        ("hit_chance_cap", d.hit_chance_cap),
        ("player_resistance_cap", d.player_resistance_cap),
        ("resistance_maximum_cap", d.resistance_maximum_cap),
        ("enemy_resistance_cap", d.enemy_resistance_cap),
        (
            "enemy_physical_reduction_cap",
            d.enemy_physical_reduction_cap,
        ),
    ] {
        number(name, value, 0.0, 100.0)?;
    }
    if d.hit_chance_floor > d.hit_chance_cap
        || d.resistance_floor > d.player_resistance_cap
        || d.resistance_floor > d.enemy_resistance_cap
        || d.resistance_floor > d.resistance_maximum_cap
    {
        return Err(error("defence floors exceed caps"));
    }
    if package.monsters.armour.len() != 100 || package.monsters.evasion.len() != 100 {
        return Err(error("monster tables must cover each level 1..100 exactly"));
    }
    let e = &package.encounters;
    if !["None", "Boss", "Pinnacle"].contains(&e.default_boss.as_str())
        || !(1..=100).contains(&e.normal_level_cap)
        || !(1..=e.normal_level_cap).contains(&e.pinnacle_level)
    {
        return Err(error("invalid encounter defaults or level bounds"));
    }
    number(
        "default_resistance_penalty",
        e.default_resistance_penalty,
        -60.0,
        0.0,
    )?;
    let mut found = BTreeSet::new();
    for effects in &package.passive_effects {
        if !found.insert((
            effects.class_id,
            effects.ascendancy_id.clone(),
            effects.physical_node_id,
        )) {
            return Err(error("duplicate owned passive effect selector"));
        }
        let node = match effects.ascendancy_id.as_deref() {
            Some(ascendancy_id) => package.tree.ascendancy_passive(
                effects.class_id,
                ascendancy_id,
                effects.physical_node_id,
            ),
            None => package
                .tree
                .entrance(effects.class_id, effects.physical_node_id),
        }
        .map_err(error)?;
        if effects.effective_node_id != node.effective_source_id
            || effects.effects.len() != node.stats.len()
            || effects.effects.is_empty()
            || effects.effects.len() > limits.max_effects_per_passive
        {
            return Err(error(
                "passive effect references, source ordering or effect limits disagree",
            ));
        }
        for effect in &effects.effects {
            let minimum = match effect.stat {
                PassiveStat::FireResistanceFlat
                | PassiveStat::ColdResistanceFlat
                | PassiveStat::LightningResistanceFlat
                | PassiveStat::ChaosResistanceFlat
                | PassiveStat::ElementalResistanceFlat => -1e6,
                _ => 0.0,
            };
            number("passive effect value", effect.value, minimum, 1e6)?;
        }
        let stats: BTreeSet<_> = effects.effects.iter().map(|v| v.stat).collect();
        if stats.len() != effects.effects.len() {
            return Err(error("duplicate passive stat operations"));
        }
    }
    let expected: BTreeSet<_> = package
        .tree
        .class_entrances
        .iter()
        .flat_map(|(class, values)| values.keys().map(|node| (*class, None, *node)))
        .chain(
            package
                .tree
                .ascendancy_passives
                .iter()
                .flat_map(|(ascendancy, nodes)| {
                    let class_id = package.tree.ascendancies[ascendancy].class_id;
                    nodes
                        .keys()
                        .map(move |node| (class_id, Some(ascendancy.clone()), *node))
                }),
        )
        .collect();
    if found != expected {
        return Err(error(
            "typed effects must cover every admitted owned passive exactly once",
        ));
    }
    Ok(())
}
fn validate_requirement(name: &str, requirement: &RequirementData) -> Result<()> {
    number(
        &format!("{name} required character level"),
        requirement.level as f64,
        0.0,
        100.0,
    )
}
fn validate_numbers(path: &str, value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Number(v) => {
            let minimum = if path.ends_with("resistance_floor")
                || path.ends_with("default_resistance_penalty")
            {
                -1e6
            } else {
                0.0
            };
            number(
                path,
                v.as_f64()
                    .ok_or_else(|| error("numeric field is not representable"))?,
                minimum,
                1e6,
            )
        }
        serde_json::Value::Array(v) => {
            for (i, v) in v.iter().enumerate() {
                validate_numbers(&format!("{path}[{i}]"), v)?;
            }
            Ok(())
        }
        serde_json::Value::Object(v) => {
            for (k, v) in v {
                validate_numbers(&format!("{path}.{k}"), v)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
// Deserialize once into a bounded intermediate value. Duplicate keys must fail
// before serde's ordinary map handling could silently keep the last occurrence.
fn bounded_json(bytes: &[u8], limits: &LoadLimits) -> Result<serde_json::Value> {
    use serde::de::{DeserializeSeed, Error as _};
    struct Seed<'a> {
        limits: &'a LoadLimits,
        remaining: &'a mut usize,
        depth: usize,
    }
    impl<'de> DeserializeSeed<'de> for Seed<'_> {
        type Value = serde_json::Value;
        fn deserialize<D: serde::Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> std::result::Result<Self::Value, D::Error> {
            if self.depth > self.limits.max_depth || *self.remaining == 0 {
                return Err(D::Error::custom("JSON depth/value limit exceeded"));
            }
            *self.remaining -= 1;
            deserializer.deserialize_any(self)
        }
    }
    impl<'de> serde::de::Visitor<'de> for Seed<'_> {
        type Value = serde_json::Value;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded JSON without duplicate keys")
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
            serde_json::Number::from_f64(v)
                .map(serde_json::Value::Number)
                .ok_or_else(|| E::custom("nonfinite JSON number"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
            if v.len() > self.limits.max_string_bytes {
                Err(E::custom("JSON string limit exceeded"))
            } else {
                Ok(v.into())
            }
        }
        fn visit_string<E: serde::de::Error>(
            self,
            v: String,
        ) -> std::result::Result<Self::Value, E> {
            if v.len() > self.limits.max_string_bytes {
                Err(E::custom("JSON string limit exceeded"))
            } else {
                Ok(v.into())
            }
        }
        fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }
        fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(v) = seq.next_element_seed(Seed {
                limits: self.limits,
                remaining: self.remaining,
                depth: self.depth + 1,
            })? {
                values.push(v)
            }
            Ok(serde_json::Value::Array(values))
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut values = serde_json::Map::new();
            while let Some(k) = map.next_key::<String>()? {
                if k.len() > self.limits.max_string_bytes {
                    return Err(A::Error::custom("JSON key string limit exceeded"));
                }
                if values.contains_key(&k) {
                    return Err(A::Error::custom(format!("duplicate JSON key: {k}")));
                }
                let v = map.next_value_seed(Seed {
                    limits: self.limits,
                    remaining: self.remaining,
                    depth: self.depth + 1,
                })?;
                values.insert(k, v);
            }
            Ok(serde_json::Value::Object(values))
        }
    }
    let mut remaining = limits.max_values;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Seed {
        limits,
        remaining: &mut remaining,
        depth: 1,
    }
    .deserialize(&mut deserializer)
    .map_err(error)?;
    deserializer.end().map_err(error)?;
    Ok(value)
}
