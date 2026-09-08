//! Immutable support loadouts compiled from injected, typed game data.
//!
//! Compilation runs modifier queries once. Build evaluation uses numeric values
//! and an O(1) dataset binding check; it does not parse gem names or allocate.
use crate::{
    data::GameDataError,
    mace::MaceError,
    modifiers::{
        ModifierDatabase, ModifierInput, ModifierKind, ModifierValue, MorePrecision, NumericKind,
        QueryContext, SumKind,
    },
};
use poe_optimizer_data::game_data::{
    GameDataPackage, SupportDamageType, SupportOperation, SupportScope, SupportStat,
};
use std::{collections::BTreeMap, sync::Arc};

// Typed query semantics from the pinned Global.lua. These are operation masks,
// not configurable game balance values.
const ATTACK: u64 = 0x1;
const HIT: u64 = 0x4;
const MELEE: u64 = 0x100;

/// A validated loadout owned by exactly one compiled dataset. Cloning retains
/// that binding. Fields are private so callers cannot manufacture numeric results.
#[derive(Debug, Clone)]
pub struct PreparedMaceSupports {
    keys: Vec<String>,
    binding: Arc<()>,
    pub(crate) physical_increased: f64,
    pub(crate) physical_more: f64,
    pub(crate) speed_increased: f64,
    pub(crate) speed_more: f64,
    pub(crate) disable_physical: bool,
    pub(crate) disable_fire: bool,
}
impl PreparedMaceSupports {
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

#[derive(Debug)]
pub(crate) struct MaceSupportCatalog {
    loadouts: BTreeMap<Vec<String>, PreparedMaceSupports>,
    binding: Arc<()>,
}
impl MaceSupportCatalog {
    pub(crate) fn compile(package: &GameDataPackage) -> Result<Self, GameDataError> {
        let binding = Arc::new(());
        let mut keys: Vec<_> = package
            .supports
            .iter()
            .map(|support| support.id.clone())
            .collect();
        keys.sort();
        let mut choices = vec![vec![]];
        for (index, key) in keys.iter().enumerate() {
            choices.push(vec![key.clone()]);
            for other in &keys[index + 1..] {
                choices.push(vec![key.clone(), other.clone()]);
            }
        }
        let mut loadouts = BTreeMap::new();
        for keys in choices {
            // Eligibility and family rules may exclude a custom-data combination.
            // Such a loadout is unavailable; no unsupported support is discarded
            // from an otherwise requested loadout during evaluation.
            let Ok(records) = package.validate_mace_support_loadout(&keys) else {
                continue;
            };
            let mut inputs = vec![];
            let mut disable_physical = false;
            let mut disable_fire = false;
            for support in records {
                for damage in &support.disable_damage {
                    match damage {
                        SupportDamageType::Physical => disable_physical = true,
                        SupportDamageType::Fire => disable_fire = true,
                        SupportDamageType::Cold
                        | SupportDamageType::Lightning
                        | SupportDamageType::Chaos => {}
                    }
                }
                for modifier in &support.modifiers {
                    inputs.push(ModifierInput {
                        name: match modifier.stat {
                            SupportStat::PhysicalDamage => "PhysicalDamage",
                            SupportStat::Speed => "Speed",
                        }
                        .into(),
                        kind: ModifierKind::Numeric(match modifier.operation {
                            SupportOperation::Increased => NumericKind::Increased,
                            SupportOperation::More => NumericKind::More,
                        }),
                        value: ModifierValue::Number(modifier.value),
                        flags: match modifier.scope {
                            SupportScope::Any => 0,
                            SupportScope::Melee => MELEE,
                            SupportScope::Attack => ATTACK,
                        },
                        keyword_flags: 0,
                        source: Some(format!("Support:{}", support.id)),
                        tag_kinds: vec![],
                    });
                }
            }
            let error = |error: crate::modifiers::ModifierError| GameDataError(error.to_string());
            let database = ModifierDatabase::try_new(vec![inputs]).map_err(error)?;
            let query = QueryContext {
                flags: ATTACK | HIT | MELEE,
                ..Default::default()
            };
            let precision = MorePrecision::default();
            let prepared = PreparedMaceSupports {
                keys: keys.clone(),
                binding: binding.clone(),
                physical_increased: database
                    .sum(SumKind::Increased, &query, &["PhysicalDamage"])
                    .map_err(error)?,
                physical_more: database
                    .more(&query, &["PhysicalDamage"], &precision)
                    .map_err(error)?,
                speed_increased: database
                    .sum(SumKind::Increased, &query, &["Speed"])
                    .map_err(error)?,
                speed_more: database
                    .more(&query, &["Speed"], &precision)
                    .map_err(error)?,
                disable_physical,
                disable_fire,
            };
            if [
                prepared.physical_increased,
                prepared.physical_more,
                prepared.speed_increased,
                prepared.speed_more,
            ]
            .into_iter()
            .any(|value| !value.is_finite())
            {
                return Err(GameDataError(
                    "support modifier aggregation must be finite".into(),
                ));
            }
            // Individual configurable INC operations may each be valid while
            // their sum produces a negative multiplier. This closed profile does
            // not admit such loadouts, even if a later passive could offset them.
            // Do not invent a clamp absent from the translated source branch.
            if prepared.physical_increased < -100.0 || prepared.speed_increased < -100.0 {
                return Err(GameDataError("combined support INC modifiers must not produce negative physical damage or speed multipliers".into()));
            }
            loadouts.insert(keys, prepared);
        }
        Ok(Self { loadouts, binding })
    }
    pub(crate) fn loadout(&self, keys: &[String]) -> Result<&PreparedMaceSupports, MaceError> {
        self.loadouts.get(keys).ok_or(MaceError(
            "Mace supports must be an eligible canonical loadout of zero to two distinct families",
        ))
    }
    pub(crate) fn legacy(&self, brutality: bool) -> Result<&PreparedMaceSupports, MaceError> {
        if brutality {
            self.loadouts
                .values()
                .find(|loadout| loadout.keys.len() == 1 && loadout.keys[0] == "brutality_i")
                .ok_or(MaceError(
                    "The legacy Brutality support is unavailable in this dataset",
                ))
        } else {
            self.loadout(&[])
        }
    }
    pub(crate) fn contains(&self, supports: &PreparedMaceSupports) -> bool {
        Arc::ptr_eq(&self.binding, &supports.binding)
    }
}
