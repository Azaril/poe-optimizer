//! Injected gem/effect identities. Catalog membership never grants native mechanics.
use crate::game_data::GameDataError;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
const MAX_ROWS: usize = 50_000;
const MAX_REFERENCES: usize = 256;
pub const SKILL_IDENTITY_SCHEMA_VERSION: u32 = 1;
type Result<T> = std::result::Result<T, GameDataError>;
fn error(value: impl std::fmt::Display) -> GameDataError {
    GameDataError(format!("skill identities: {value}"))
}
fn required_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillIdentityCapability {
    IdentityOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySourceSpan {
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillIdentitySource {
    pub upstream_revision: String,
    pub files: BTreeMap<String, String>,
    /// Actual ordered skill-module loads during authenticated construction.
    pub skill_module_order: Vec<String>,
    pub skill_assembly: IdentitySourceSpan,
    pub gem_assembly: IdentitySourceSpan,
    pub load_skill: IdentitySourceSpan,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedIdentityReference {
    pub index: u32,
    pub id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredGemIdentity {
    pub game_id: String,
    pub variant_id: String,
    pub name: String,
    #[serde(deserialize_with = "required_option")]
    pub name_spec: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub base_type_name: Option<String>,
    pub primary_effect_id: String,
    pub additional_effects: Vec<IndexedIdentityReference>,
    /// Stat-set references are not additional granted effects or actions.
    pub additional_stat_sets: Vec<IndexedIdentityReference>,
    #[serde(deserialize_with = "required_option")]
    pub display_order: Option<Vec<i64>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemIdentityDeclaration {
    pub index: u32,
    pub key: String,
    pub source: IdentitySourceSpan,
    pub identity: DeclaredGemIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredSkillIdentity {
    pub name: String,
    #[serde(deserialize_with = "required_option")]
    pub base_type_name: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub support: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub from_tree: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillIdentityDeclaration {
    pub index: u32,
    pub id: String,
    pub source: IdentitySourceSpan,
    pub identity: DeclaredSkillIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemIdentity {
    pub key: String,
    pub game_id: String,
    pub variant_id: String,
    pub name: String,
    #[serde(deserialize_with = "required_option")]
    pub name_spec: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub base_type_name: Option<String>,
    pub primary_effect_id: String,
    pub declared_additional_effects: Vec<IndexedIdentityReference>,
    pub declared_additional_stat_sets: Vec<IndexedIdentityReference>,
    /// Post-setup indexed fields, including source-generated additions.
    pub constructed_additional_effects: Vec<IndexedIdentityReference>,
    /// Actual resolved setupGem list; missing table references are not invented.
    pub additional_effects: Vec<String>,
    /// Actual post-display-order grantedEffectList, distinct from raw field order.
    pub effect_list: Vec<String>,
    #[serde(deserialize_with = "required_option")]
    pub display_order: Option<Vec<i64>>,
    /// One-based declaration occurrence selected by original construction.
    pub winning_declaration: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillIdentity {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "required_option")]
    pub base_type_name: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub support: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub from_tree: Option<bool>,
    pub winning_declaration: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingIdentityReferenceKind {
    PrimaryEffect,
    DeclaredAdditionalEffect,
    ConstructedAdditionalEffect,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissingIdentityReference {
    pub gem_key: String,
    pub kind: MissingIdentityReferenceKind,
    #[serde(deserialize_with = "required_option")]
    pub index: Option<u32>,
    pub effect_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillIdentityData {
    pub schema_version: u32,
    pub capability: SkillIdentityCapability,
    pub source: SkillIdentitySource,
    pub gem_declarations: Vec<GemIdentityDeclaration>,
    pub skill_declarations: Vec<SkillIdentityDeclaration>,
    /// Final constructed rows, sorted only for deterministic data serialization.
    pub gems: Vec<GemIdentity>,
    pub skills: Vec<SkillIdentity>,
    pub missing_references: Vec<MissingIdentityReference>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GemIdentityResolutionStatus {
    Exact,
    SingleFallback,
    Ambiguous,
    Missing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GemIdentityResolutionReason {
    ExactExternalVariant,
    MissingVariantSingleCandidate,
    UnknownVariantSingleCandidate,
    MissingVariantMultipleCandidates,
    UnknownVariantMultipleCandidates,
    CollidingExternalVariant,
    UnknownExternalId,
}
/// References, not cloned catalog rows. Callers should impose report expansion bounds.
#[derive(Debug)]
pub struct GemIdentityResolution<'a> {
    pub status: GemIdentityResolutionStatus,
    pub reason: GemIdentityResolutionReason,
    pub candidates: Vec<&'a GemIdentity>,
}
#[derive(Debug)]
struct Inner {
    data: SkillIdentityData,
    gems: BTreeMap<String, usize>,
    skills: BTreeMap<String, usize>,
    external: BTreeMap<String, Vec<usize>>,
    variants: BTreeMap<(String, String), Vec<usize>>,
    effects: BTreeMap<String, Vec<usize>>,
}
#[derive(Debug, Clone)]
pub struct SkillIdentityCatalog {
    inner: Arc<Inner>,
}
impl SkillIdentityCatalog {
    pub fn new(data: SkillIdentityData) -> Result<Self> {
        data.validate()?;
        let mut inner = Inner {
            data,
            gems: BTreeMap::new(),
            skills: BTreeMap::new(),
            external: BTreeMap::new(),
            variants: BTreeMap::new(),
            effects: BTreeMap::new(),
        };
        for (index, gem) in inner.data.gems.iter().enumerate() {
            inner.gems.insert(gem.key.clone(), index);
            inner
                .external
                .entry(gem.game_id.clone())
                .or_default()
                .push(index);
            inner
                .variants
                .entry((gem.game_id.clone(), gem.variant_id.clone()))
                .or_default()
                .push(index);
            inner
                .effects
                .entry(gem.primary_effect_id.clone())
                .or_default()
                .push(index);
        }
        for (index, skill) in inner.data.skills.iter().enumerate() {
            inner.skills.insert(skill.id.clone(), index);
        }
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
    pub fn data(&self) -> &SkillIdentityData {
        &self.inner.data
    }
    pub fn gem_by_key(&self, key: &str) -> Option<&GemIdentity> {
        self.inner.gems.get(key).map(|i| &self.inner.data.gems[*i])
    }
    pub fn skill_by_id(&self, id: &str) -> Option<&SkillIdentity> {
        self.inner
            .skills
            .get(id)
            .map(|i| &self.inner.data.skills[*i])
    }
    pub fn gems_for_external_id<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a GemIdentity> {
        self.inner
            .external
            .get(id)
            .into_iter()
            .flatten()
            .map(|i| &self.inner.data.gems[*i])
    }
    /// Possible primary gem owners, not an asserted pairs-order gemForSkill winner.
    pub fn gems_for_effect<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a GemIdentity> {
        self.inner
            .effects
            .get(id)
            .into_iter()
            .flatten()
            .map(|i| &self.inner.data.gems[*i])
    }
    pub fn resolve_external<'a>(
        &'a self,
        game_id: &str,
        variant: Option<&str>,
    ) -> GemIdentityResolution<'a> {
        use GemIdentityResolutionReason as R;
        use GemIdentityResolutionStatus as S;
        let rows = if let Some(variant) = variant {
            self.inner.variants.get(&(game_id.into(), variant.into()))
        } else {
            None
        };
        if let Some(rows) = rows {
            return GemIdentityResolution {
                status: if rows.len() == 1 {
                    S::Exact
                } else {
                    S::Ambiguous
                },
                reason: if rows.len() == 1 {
                    R::ExactExternalVariant
                } else {
                    R::CollidingExternalVariant
                },
                candidates: rows.iter().map(|i| &self.inner.data.gems[*i]).collect(),
            };
        }
        let candidates: Vec<_> = self.gems_for_external_id(game_id).collect();
        let (status, reason) = match (candidates.len(), variant.is_some()) {
            (0, _) => (S::Missing, R::UnknownExternalId),
            (1, false) => (S::SingleFallback, R::MissingVariantSingleCandidate),
            (1, true) => (S::SingleFallback, R::UnknownVariantSingleCandidate),
            (_, false) => (S::Ambiguous, R::MissingVariantMultipleCandidates),
            (_, true) => (S::Ambiguous, R::UnknownVariantMultipleCandidates),
        };
        GemIdentityResolution {
            status,
            reason,
            candidates,
        }
    }
}
fn text(value: &str, empty: bool) -> Result<()> {
    if value.len() > 4096 || (!empty && value.is_empty()) || value.contains('\0') {
        Err(error("empty, oversized or NUL-containing string"))
    } else {
        Ok(())
    }
}
fn hash(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn references(values: &[IndexedIdentityReference]) -> Result<()> {
    if values.len() > MAX_REFERENCES {
        return Err(error("too many indexed references"));
    }
    let mut previous = 0;
    for value in values {
        if value.index <= previous || value.index > MAX_REFERENCES as u32 {
            return Err(error(
                "indexed references must be positive increasing and bounded",
            ));
        }
        text(&value.id, false)?;
        previous = value.index;
    }
    Ok(())
}
fn order(values: &Option<Vec<i64>>) -> Result<()> {
    if values.as_ref().is_some_and(|v| {
        v.len() > MAX_REFERENCES || v.iter().any(|n| !(-1_000_000..=1_000_000).contains(n))
    }) {
        Err(error("display order exceeds bounds"))
    } else {
        Ok(())
    }
}
impl SkillIdentityData {
    pub fn expected_missing_references(&self) -> Vec<MissingIdentityReference> {
        let known: BTreeSet<_> = self.skills.iter().map(|s| s.id.as_str()).collect();
        let mut missing = Vec::new();
        for gem in &self.gems {
            if !known.contains(gem.primary_effect_id.as_str()) {
                missing.push(MissingIdentityReference {
                    gem_key: gem.key.clone(),
                    kind: MissingIdentityReferenceKind::PrimaryEffect,
                    index: None,
                    effect_id: gem.primary_effect_id.clone(),
                });
            }
            for (kind, refs) in [
                (
                    MissingIdentityReferenceKind::DeclaredAdditionalEffect,
                    &gem.declared_additional_effects,
                ),
                (
                    MissingIdentityReferenceKind::ConstructedAdditionalEffect,
                    &gem.constructed_additional_effects,
                ),
            ] {
                for r in refs {
                    if !known.contains(r.id.as_str()) {
                        missing.push(MissingIdentityReference {
                            gem_key: gem.key.clone(),
                            kind,
                            index: Some(r.index),
                            effect_id: r.id.clone(),
                        });
                    }
                }
            }
        }
        missing.sort();
        missing
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SKILL_IDENTITY_SCHEMA_VERSION {
            return Err(error("unsupported schema version"));
        }
        for size in [
            self.gems.len(),
            self.skills.len(),
            self.gem_declarations.len(),
            self.skill_declarations.len(),
            self.missing_references.len(),
        ] {
            if size > MAX_ROWS {
                return Err(error("row bound exceeded"));
            }
        }
        if self.gems.is_empty()
            || self.skills.is_empty()
            || self.source.files.is_empty()
            || self.source.files.len() > 128
            || !hash(&self.source.upstream_revision, 40)
        {
            return Err(error("missing catalog/source identity"));
        }
        for (path, digest) in &self.source.files {
            text(path, false)?;
            if path.starts_with('/')
                || path.contains('\\')
                || path.contains(':')
                || path
                    .split('/')
                    .any(|c| c.is_empty() || c == "." || c == "..")
                || !hash(digest, 64)
            {
                return Err(error("invalid relative source path or digest"));
            }
        }
        let span = |s: &IdentitySourceSpan| -> Result<()> {
            if s.line == 0
                || s.end_line < s.line
                || s.end_line > 1_000_000
                || !self.source.files.contains_key(&s.path)
                || !hash(&s.sha256, 64)
            {
                Err(error("invalid source span"))
            } else {
                Ok(())
            }
        };
        for s in [
            &self.source.skill_assembly,
            &self.source.gem_assembly,
            &self.source.load_skill,
        ] {
            span(s)?;
        }
        let mut modules = BTreeSet::new();
        for p in &self.source.skill_module_order {
            if !self.source.files.contains_key(p) || !modules.insert(p) {
                return Err(error("invalid skill module order"));
            }
        }
        if modules.is_empty() || modules.len() > 128 {
            return Err(error("missing skill construction order"));
        }
        let mut gem_last = BTreeMap::new();
        for (i, d) in self.gem_declarations.iter().enumerate() {
            if d.index != i as u32 + 1 {
                return Err(error("gem declaration order/index mismatch"));
            }
            text(&d.key, false)?;
            span(&d.source)?;
            let v = &d.identity;
            for id in [&v.game_id, &v.variant_id, &v.primary_effect_id] {
                text(id, false)?;
            }
            text(&v.name, true)?;
            for t in [&v.name_spec, &v.base_type_name].into_iter().flatten() {
                text(t, true)?;
            }
            references(&v.additional_effects)?;
            references(&v.additional_stat_sets)?;
            order(&v.display_order)?;
            gem_last.insert(d.key.as_str(), d.index);
        }
        let mut skill_last = BTreeMap::new();
        for (i, d) in self.skill_declarations.iter().enumerate() {
            if d.index != i as u32 + 1 {
                return Err(error("skill declaration order/index mismatch"));
            }
            text(&d.id, false)?;
            text(&d.identity.name, true)?;
            if let Some(v) = &d.identity.base_type_name {
                text(v, true)?;
            }
            span(&d.source)?;
            skill_last.insert(d.id.as_str(), d.index);
        }
        let mut keys = BTreeSet::new();
        for g in &self.gems {
            text(&g.key, false)?;
            text(&g.name, true)?;
            for v in [&g.name_spec, &g.base_type_name].into_iter().flatten() {
                text(v, true)?;
            }
            if !keys.insert(&g.key)
                || g.winning_declaration == 0
                || self
                    .gem_declarations
                    .get(g.winning_declaration as usize - 1)
                    .is_none_or(|d| d.key != g.key)
            {
                return Err(error("invalid gem constructed winner"));
            }
            let d = &self.gem_declarations[g.winning_declaration as usize - 1].identity;
            if g.game_id != d.game_id
                || g.variant_id != d.variant_id
                || g.primary_effect_id != d.primary_effect_id
                || g.name_spec != d.name_spec
                || g.base_type_name != d.base_type_name
                || g.declared_additional_effects != d.additional_effects
                || g.declared_additional_stat_sets != d.additional_stat_sets
                || g.display_order != d.display_order
            {
                return Err(error("constructed gem disagrees with declaration identity"));
            }
            references(&g.constructed_additional_effects)?;
            if g.declared_additional_effects
                .iter()
                .any(|reference| !g.constructed_additional_effects.contains(reference))
            {
                return Err(error(
                    "constructed gem dropped a declared additional reference",
                ));
            }
            for list in [&g.additional_effects, &g.effect_list] {
                if list.len() > MAX_REFERENCES {
                    return Err(error("constructed effect list bound"));
                }
                for id in list {
                    text(id, false)?;
                }
            }
        }
        if keys.len() != gem_last.len() {
            return Err(error("gem declaration/constructed partition mismatch"));
        }
        let mut ids = BTreeSet::new();
        for s in &self.skills {
            text(&s.id, false)?;
            text(&s.name, true)?;
            if let Some(v) = &s.base_type_name {
                text(v, true)?;
            }
            if !ids.insert(&s.id)
                || s.winning_declaration == 0
                || self
                    .skill_declarations
                    .get(s.winning_declaration as usize - 1)
                    .is_none_or(|d| d.id != s.id)
            {
                return Err(error("invalid skill constructed winner"));
            }
            let d = &self.skill_declarations[s.winning_declaration as usize - 1].identity;
            if s.base_type_name != d.base_type_name
                || s.support != d.support
                || s.from_tree != d.from_tree
            {
                return Err(error(
                    "constructed skill disagrees with declaration identity",
                ));
            }
        }
        if ids.len() != skill_last.len() {
            return Err(error("skill declaration/constructed partition mismatch"));
        }
        for g in &self.gems {
            if g.additional_effects
                .iter()
                .chain(&g.effect_list)
                .any(|id| !ids.contains(id))
            {
                return Err(error("constructed effect list refers to absent skill"));
            }
        }
        if self.missing_references != self.expected_missing_references() {
            return Err(error("missing-reference evidence mismatch"));
        }
        Ok(())
    }
}
