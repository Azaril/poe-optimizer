//! Injected source skill/gem loading definitions, separate from identity evidence.
//! Numeric row keys and table alias IDs retain source lookup/mutation semantics.
use crate::{
    game_data::GameDataError,
    skill_identities::{IdentitySourceSpan, SkillIdentityData},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const SKILL_PREPARATION_SCHEMA_VERSION: u32 = 1;
const MAX_ROWS: usize = 50_000;
const MAX_LEVEL_ROWS: usize = 500_000;
type Result<T> = std::result::Result<T, GameDataError>;
fn error(message: impl std::fmt::Display) -> GameDataError {
    GameDataError(format!("skill preparation: {message}"))
}
fn required_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemRequirementFormula {
    pub base: f64,
    pub level_offset: f64,
    pub level_multiplier: f64,
    pub attribute_divisor: f64,
    pub attribute_exponent: f64,
    pub round_bias: f64,
    pub result_offset: f64,
    pub minimum_requirement: f64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPreparationColors {
    pub unresolved: String,
    pub normal: String,
    pub strength: String,
    pub dexterity: String,
    pub intelligence: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemPreparationDefinition {
    pub key: String,
    pub natural_max_level: f64,
    #[serde(deserialize_with = "required_option")]
    pub req_str: Option<f64>,
    #[serde(deserialize_with = "required_option")]
    pub req_dex: Option<f64>,
    #[serde(deserialize_with = "required_option")]
    pub req_int: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillLevelDefinition {
    pub key: f64,
    /// Stable identity of the original row table; shared rows use the same ID.
    pub row_id: String,
    #[serde(deserialize_with = "required_option")]
    pub level_requirement: Option<f64>,
    #[serde(deserialize_with = "required_option")]
    pub cost: Option<BTreeMap<String, f64>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPreparationDefinition {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "required_option")]
    pub color: Option<f64>,
    #[serde(deserialize_with = "required_option")]
    pub support: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub hide_from_sidebar: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub plus_version_of: Option<String>,
    /// Original `#levels` and `next(levels)`; neither inferred from maximum key.
    pub levels_length: usize,
    #[serde(deserialize_with = "required_option")]
    pub next_level_key: Option<f64>,
    pub levels: Vec<SkillLevelDefinition>,
}
impl SkillPreparationDefinition {
    pub fn level(&self, key: f64) -> Option<&SkillLevelDefinition> {
        let key = if key == 0.0 { 0.0 } else { key };
        self.levels
            .binary_search_by(|row| row.key.total_cmp(&key))
            .ok()
            .map(|index| &self.levels[index])
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalGemVariants {
    pub game_id: String,
    /// Canonical iteration only, never an emulation of Lua `pairs`.
    pub canonical_variant_order: Vec<String>,
    /// Source-order-sensitive duplicate variant bindings have no chosen winner.
    pub ambiguous_variants: BTreeMap<String, Vec<String>>,
    pub variants: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPreparationSource {
    pub upstream_revision: String,
    pub files: BTreeMap<String, String>,
    pub operations: BTreeMap<String, IdentitySourceSpan>,
    /// Canonical iteration for reproducible storage/search; never source pairs order.
    pub canonical_gem_order: Vec<String>,
    pub iteration_policy: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPreparationData {
    pub schema_version: u32,
    pub source: SkillPreparationSource,
    pub requirement: GemRequirementFormula,
    pub colors: SkillPreparationColors,
    pub default_gem_level_options: Vec<String>,
    pub support_type_options: Vec<String>,
    pub sort_field_options: Vec<String>,
    pub default_gem_quality: f64,
    pub minimum_gem_quality: f64,
    pub maximum_gem_quality: f64,
    pub initial_new_gem_level: f64,
    pub minimum_gem_level: f64,
    pub default_gem_level: String,
    pub default_support_type: String,
    pub default_sort_field: String,
    pub gems: Vec<GemPreparationDefinition>,
    pub effects: Vec<SkillPreparationDefinition>,
    /// Stable effect-object IDs represent source TABLE keys.
    pub table_gem_for_skill: BTreeMap<String, String>,
    pub ambiguous_table_gem_for_skill: BTreeMap<String, Vec<String>>,
    /// Source STRING-key entries are kept separate; normally empty.
    pub string_gem_for_skill: BTreeMap<String, String>,
    pub external_variants: Vec<ExternalGemVariants>,
}
#[derive(Debug)]
struct Inner {
    data: SkillPreparationData,
    gems: BTreeMap<String, usize>,
    effects: BTreeMap<String, usize>,
    external: BTreeMap<String, usize>,
}
#[derive(Debug, Clone)]
pub struct SkillPreparationCatalog {
    inner: Arc<Inner>,
}
impl SkillPreparationCatalog {
    pub fn new(data: SkillPreparationData, identities: &SkillIdentityData) -> Result<Self> {
        data.validate(identities)?;
        let gems = data
            .gems
            .iter()
            .enumerate()
            .map(|(i, g)| (g.key.clone(), i))
            .collect();
        let effects = data
            .effects
            .iter()
            .enumerate()
            .map(|(i, g)| (g.id.clone(), i))
            .collect();
        let external = data
            .external_variants
            .iter()
            .enumerate()
            .map(|(i, g)| (g.game_id.clone(), i))
            .collect();
        Ok(Self {
            inner: Arc::new(Inner {
                data,
                gems,
                effects,
                external,
            }),
        })
    }
    pub fn data(&self) -> &SkillPreparationData {
        &self.inner.data
    }
    pub fn gem(&self, key: &str) -> Option<&GemPreparationDefinition> {
        self.inner.gems.get(key).map(|&i| &self.inner.data.gems[i])
    }
    pub fn effect(&self, id: &str) -> Option<&SkillPreparationDefinition> {
        self.inner
            .effects
            .get(id)
            .map(|&i| &self.inner.data.effects[i])
    }
    /// Canonical traversal only. Multiple matches never establish a source winner.
    pub fn gems_ordered(&self) -> impl Iterator<Item = &GemPreparationDefinition> {
        self.inner
            .data
            .source
            .canonical_gem_order
            .iter()
            .map(|key| self.gem(key).expect("validated gem traversal"))
    }
    pub fn external_variants(&self, id: &str) -> Option<&ExternalGemVariants> {
        self.inner
            .external
            .get(id)
            .map(|&i| &self.inner.data.external_variants[i])
    }
    pub fn table_gem_for_skill(&self, id: &str) -> Option<&str> {
        self.inner
            .data
            .table_gem_for_skill
            .get(id)
            .map(String::as_str)
    }
    pub fn ambiguous_table_gem_for_skill(&self, id: &str) -> Option<&[String]> {
        self.inner
            .data
            .ambiguous_table_gem_for_skill
            .get(id)
            .map(Vec::as_slice)
    }
    pub fn string_gem_for_skill(&self, id: &str) -> Option<&str> {
        self.inner
            .data
            .string_gem_for_skill
            .get(id)
            .map(String::as_str)
    }
}
impl SkillPreparationData {
    pub fn validate(&self, identities: &SkillIdentityData) -> Result<()> {
        if self.schema_version != SKILL_PREPARATION_SCHEMA_VERSION
            || self.gems.len() > MAX_ROWS
            || self.effects.len() > MAX_ROWS
            || identities.gems.len() > MAX_ROWS
            || identities.skills.len() > MAX_ROWS
        {
            return Err(error("unsupported schema or row limit"));
        }
        let gems: BTreeMap<_, _> = identities
            .gems
            .iter()
            .map(|g| (g.key.as_str(), g))
            .collect();
        let effects: BTreeMap<_, _> = identities
            .skills
            .iter()
            .map(|g| (g.id.as_str(), g))
            .collect();
        if self.gems.len() != gems.len() || self.effects.len() != effects.len() {
            return Err(error("identity/preparation catalog membership differs"));
        }
        let mut seen = BTreeSet::new();
        for gem in &self.gems {
            if !gems.contains_key(gem.key.as_str()) || !seen.insert(gem.key.as_str()) {
                return Err(error("unknown or duplicate gem"));
            }
            finite(gem.natural_max_level)?;
            for value in [gem.req_str, gem.req_dex, gem.req_int]
                .into_iter()
                .flatten()
            {
                finite(value)?;
            }
        }
        let mut seen_effects = BTreeSet::new();
        let mut row_count = 0usize;
        let mut aliases: BTreeMap<&str, &SkillLevelDefinition> = BTreeMap::new();
        for effect in &self.effects {
            let identity = effects
                .get(effect.id.as_str())
                .ok_or_else(|| error("unknown effect"))?;
            if !seen_effects.insert(effect.id.as_str())
                || effect.name != identity.name
                || effect.support != identity.support
            {
                return Err(error("effect identity disagreement or duplicate"));
            }
            if let Some(id) = &effect.plus_version_of {
                label(id)?;
            }
            if let Some(color) = effect.color {
                finite(color)?;
            }
            if effect.levels_length > MAX_LEVEL_ROWS {
                return Err(error("level length exceeds bound"));
            }
            if effect
                .levels
                .windows(2)
                .any(|rows| rows[0].key >= rows[1].key)
            {
                return Err(error(
                    "level rows must have canonical ascending numeric keys",
                ));
            }
            let mut keys = BTreeSet::new();
            for row in &effect.levels {
                row_count += 1;
                if row_count > MAX_LEVEL_ROWS {
                    return Err(error("level rows exceed bound"));
                }
                finite(row.key)?;
                if row.key == 0.0 && row.key.is_sign_negative() {
                    return Err(error("level table zero key must be canonical"));
                }
                if !keys.insert(row.key.to_bits()) {
                    return Err(error("duplicate level key"));
                }
                label(&row.row_id)?;
                if let Some(value) = row.level_requirement {
                    finite(value)?;
                }
                if let Some(cost) = &row.cost {
                    if cost.len() > 256 {
                        return Err(error("cost entries exceed bound"));
                    }
                    for (name, value) in cost {
                        label(name)?;
                        finite(*value)?;
                    }
                }
                if let Some(old) = aliases.insert(&row.row_id, row)
                    && (old.level_requirement != row.level_requirement || old.cost != row.cost)
                {
                    return Err(error("aliased level row values disagree"));
                }
            }
            if effect.levels.is_empty() != effect.next_level_key.is_none()
                || effect
                    .next_level_key
                    .is_some_and(|key| effect.level(key).is_none())
            {
                return Err(error("invalid next-level key"));
            }
            if (effect.levels_length == 0 && effect.level(1.0).is_some())
                || (effect.levels_length > 0
                    && (effect.level(effect.levels_length as f64).is_none()
                        || effect.level(effect.levels_length as f64 + 1.0).is_some()))
            {
                return Err(error("source level length is not a table boundary"));
            }
        }
        let canonical: Vec<_> = gems.keys().map(|s| (*s).to_owned()).collect();
        if self.source.canonical_gem_order != canonical {
            return Err(error("gem iteration must be canonical and complete"));
        }
        let mut owner_candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut variant_candidates: BTreeMap<String, BTreeMap<String, Vec<String>>> =
            BTreeMap::new();
        for key in &self.source.canonical_gem_order {
            let gem = gems[key.as_str()];
            owner_candidates
                .entry(gem.primary_effect_id.clone())
                .or_default()
                .push(key.clone());
            variant_candidates
                .entry(gem.game_id.clone())
                .or_default()
                .entry(gem.variant_id.clone())
                .or_default()
                .push(key.clone());
        }
        let (expected, ambiguous) = split_candidates(owner_candidates);
        if self.table_gem_for_skill != expected || self.ambiguous_table_gem_for_skill != ambiguous {
            return Err(error(
                "table-key gemForSkill unique/ambiguous bindings disagree",
            ));
        }
        for (id, key) in &self.string_gem_for_skill {
            label(id)?;
            if !gems.contains_key(key.as_str()) {
                return Err(error("unknown string-key gem owner"));
            }
        }
        let expected_external: Vec<_> = variant_candidates
            .into_iter()
            .map(|(game_id, candidates)| {
                let canonical_variant_order = candidates.keys().cloned().collect();
                let (variants, ambiguous_variants) = split_candidates(candidates);
                ExternalGemVariants {
                    game_id,
                    canonical_variant_order,
                    variants,
                    ambiguous_variants,
                }
            })
            .collect();
        if self.external_variants != expected_external {
            return Err(error("external variant unique/ambiguous bindings disagree"));
        }
        for value in [
            &self.colors.unresolved,
            &self.colors.normal,
            &self.colors.strength,
            &self.colors.dexterity,
            &self.colors.intelligence,
        ] {
            label(value)?;
        }
        for value in [
            self.default_gem_quality,
            self.minimum_gem_quality,
            self.maximum_gem_quality,
            self.initial_new_gem_level,
            self.minimum_gem_level,
        ] {
            finite(value)?;
        }
        if self.minimum_gem_quality > self.maximum_gem_quality
            || !(self.minimum_gem_quality..=self.maximum_gem_quality)
                .contains(&self.default_gem_quality)
        {
            return Err(error("invalid gem quality policy bounds"));
        }
        let f = &self.requirement;
        for value in [
            f.base,
            f.level_offset,
            f.level_multiplier,
            f.attribute_divisor,
            f.attribute_exponent,
            f.round_bias,
            f.result_offset,
            f.minimum_requirement,
        ] {
            finite(value)?;
        }
        if f.attribute_divisor <= 0.0 {
            return Err(error("attribute divisor must be positive"));
        }
        for (options, selected) in [
            (&self.default_gem_level_options, &self.default_gem_level),
            (&self.support_type_options, &self.default_support_type),
            (&self.sort_field_options, &self.default_sort_field),
        ] {
            if options.is_empty()
                || options.len() > 256
                || !options.contains(selected)
                || options.iter().collect::<BTreeSet<_>>().len() != options.len()
            {
                return Err(error("invalid option choices/default"));
            }
            for option in options {
                label(option)?;
            }
        }
        if self.source.files.len() > 256
            || self.source.operations.len() > 64
            || self.string_gem_for_skill.len() > MAX_ROWS
            || self.external_variants.len() > MAX_ROWS
        {
            return Err(error("source or lookup count exceeds bound"));
        }
        if self.source.upstream_revision != identities.source.upstream_revision {
            return Err(error("source revision disagreement"));
        }
        if self.source.iteration_policy
            != "canonical_nonsemantic; ambiguous_source_pairs_winners_unresolved"
        {
            return Err(error("unknown iteration policy"));
        }
        for (path, digest) in &self.source.files {
            label(path)?;
            if identities
                .source
                .files
                .get(path)
                .is_some_and(|old| old != digest)
            {
                return Err(error("source file identity disagreement"));
            }
            if !valid_digest(digest) {
                return Err(error("invalid source digest"));
            }
        }
        for (name, span) in &self.source.operations {
            label(name)?;
            if !self.source.files.contains_key(&span.path)
                || span.line == 0
                || span.end_line < span.line
                || !valid_digest(&span.sha256)
            {
                return Err(error("invalid operation source span"));
            }
        }
        Ok(())
    }
}
fn finite(value: f64) -> Result<()> {
    if !value.is_finite() || value.abs() > 1e12 {
        Err(error("numeric definition is nonfinite or exceeds bound"))
    } else {
        Ok(())
    }
}
fn label(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        Err(error("invalid bounded label"))
    } else {
        Ok(())
    }
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn split_candidates(
    candidates: BTreeMap<String, Vec<String>>,
) -> (BTreeMap<String, String>, BTreeMap<String, Vec<String>>) {
    let mut unique = BTreeMap::new();
    let mut ambiguous = BTreeMap::new();
    for (key, candidates) in candidates {
        if candidates.len() == 1 {
            unique.insert(key, candidates[0].clone());
        } else {
            ambiguous.insert(key, candidates);
        }
    }
    (unique, ambiguous)
}
