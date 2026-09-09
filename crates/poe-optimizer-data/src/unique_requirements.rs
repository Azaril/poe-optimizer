//! Completed unique requirement projections. This capability never grants item assembly.
use crate::{
    bundled::BundledClassTree,
    game_data::GameDataError,
    item_loading::{ItemLoadingData, ItemLoadingSource},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const UNIQUE_REQUIREMENTS_SCHEMA_VERSION: u32 = 1;
const MAX_PROTOTYPES: usize = 50_000;
const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
type Result<T> = std::result::Result<T, GameDataError>;
fn error(message: impl std::fmt::Display) -> GameDataError {
    GameDataError(format!("unique requirements: {message}"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn text(value: &str, maximum: usize, bytes: &mut usize) -> Result<()> {
    *bytes = bytes
        .checked_add(value.len())
        .ok_or_else(|| error("text budget overflow"))?;
    if value.len() > maximum || value.contains('\0') || *bytes > MAX_TEXT_BYTES {
        return Err(error("text bounds"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueRequirementData {
    pub schema_version: u32,
    pub state: UniqueRequirementState,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum UniqueRequirementState {
    Unavailable { reason: String },
    Complete(Box<CompleteUniqueRequirements>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteUniqueRequirements {
    pub source: ItemLoadingSource,
    pub inputs: UniqueRequirementInputs,
    pub policy: UniqueLookupPolicy,
    pub construction: UniqueConstruction,
    /// Complete input ledger, including prototypes skipped by the original loop.
    pub prototypes: Vec<UniquePrototypeOutcome>,
    /// Sorted exact canonical keys. Duplicate construction keys are unsupported.
    pub entries: Vec<UniqueRequirementEntry>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueRequirementInputs {
    pub item_loading_sha256: String,
    pub tree_sha256: String,
    pub tree_version: String,
    pub constructor_rarity: String,
    pub constructor_high_quality: bool,
    pub mod_cache_mode: UniqueModCacheMode,
    pub default_item_quality: f64,
    pub default_affix_quality: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniqueModCacheMode {
    OriginalStoredCache,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueLookupPolicy {
    /// First matching leading prefix with a nonempty suffix is removed once.
    pub base_prefixes: Vec<String>,
    pub title_base_separator: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueConstruction {
    pub source_loop_completed: bool,
    pub loading_cleared: bool,
    pub constructors_finished: u32,
    pub insertions: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniquePrototypeId {
    pub group: String,
    pub index: u32,
    pub raw_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniquePrototypeOutcome {
    pub prototype: UniquePrototypeId,
    pub disposition: UniquePrototypeDisposition,
    /// Sorted distinct potential exact/fallback database keys observed at calls.
    pub lookup_keys: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum UniquePrototypeDisposition {
    Inserted { canonical_key: String },
    SkippedMissingBase,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueRequirementEntry {
    pub canonical_key: String,
    pub prototype: UniquePrototypeId,
    pub base_name: String,
    pub natural_level: Option<f64>,
    pub level: Option<f64>,
}

impl UniqueRequirementData {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            schema_version: UNIQUE_REQUIREMENTS_SCHEMA_VERSION,
            state: UniqueRequirementState::Unavailable {
                reason: reason.into(),
            },
        }
    }
    pub fn complete(&self) -> Option<&CompleteUniqueRequirements> {
        match &self.state {
            UniqueRequirementState::Complete(data) => Some(data),
            _ => None,
        }
    }
    pub fn complete_mut(&mut self) -> Option<&mut CompleteUniqueRequirements> {
        match &mut self.state {
            UniqueRequirementState::Complete(data) => Some(data),
            _ => None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != UNIQUE_REQUIREMENTS_SCHEMA_VERSION {
            return Err(error("schema version"));
        }
        let Some(data) = self.complete() else {
            let UniqueRequirementState::Unavailable { reason } = &self.state else {
                unreachable!()
            };
            if reason.is_empty() {
                return Err(error("empty unavailable reason"));
            }
            return text(reason, 4096, &mut 0);
        };
        data.validate()
    }
    /// Cross-section coherence is separate from reviewed provenance. Custom bytes
    /// cannot prove original execution merely by declaring a complete state.
    pub fn validate_inputs(&self, items: &ItemLoadingData, tree: &BundledClassTree) -> Result<()> {
        self.validate()?;
        let Some(data) = self.complete() else {
            return Ok(());
        };
        if data.inputs.item_loading_sha256 != hash(&serde_json::to_vec(items).map_err(error)?)
            || data.inputs.tree_sha256 != hash(&serde_json::to_vec(tree).map_err(error)?)
            || data.inputs.tree_version != tree.source.tree_version
            || data.source.upstream_revision != items.source.upstream_revision
            || data.inputs.default_item_quality != items.policy.default_item_quality
            || data.inputs.default_affix_quality != items.policy.default_affix_quality
        {
            return Err(error("stale construction input identity"));
        }
        let total = items
            .unique_groups
            .values()
            .try_fold(0usize, |n, rows| n.checked_add(rows.len()))
            .ok_or_else(|| error("prototype count overflow"))?;
        if total > MAX_PROTOTYPES || total != data.prototypes.len() {
            return Err(error("incomplete prototype accounting"));
        }
        let mut outcomes = data.prototypes.iter();
        for (group, rows) in &items.unique_groups {
            for (index, raw) in rows.iter().enumerate() {
                let actual = &outcomes
                    .next()
                    .ok_or_else(|| error("missing prototype"))?
                    .prototype;
                if actual.group != *group
                    || actual.index as usize != index + 1
                    || actual.raw_sha256 != hash(raw.as_bytes())
                {
                    return Err(error("prototype identity/raw digest mismatch"));
                }
            }
        }
        let bases: BTreeSet<_> = items.bases.iter().map(|base| base.name.as_str()).collect();
        if data
            .entries
            .iter()
            .any(|entry| !bases.contains(entry.base_name.as_str()))
        {
            return Err(error("unknown constructed base identity"));
        }
        Ok(())
    }
}
impl CompleteUniqueRequirements {
    fn validate(&self) -> Result<()> {
        if self.prototypes.len() > MAX_PROTOTYPES
            || self.entries.len() > MAX_PROTOTYPES
            || !self.construction.source_loop_completed
            || !self.construction.loading_cleared
            || self.construction.constructors_finished as usize != self.prototypes.len()
            || self.construction.insertions as usize != self.entries.len()
        {
            return Err(error("incomplete or oversized construction"));
        }
        let mut bytes = 0;
        if !digest(&self.inputs.item_loading_sha256, 64)
            || !digest(&self.inputs.tree_sha256, 64)
            || self.inputs.tree_version.is_empty()
            || self.inputs.constructor_rarity.is_empty()
            || !self.inputs.default_item_quality.is_finite()
            || !self.inputs.default_affix_quality.is_finite()
        {
            return Err(error("construction inputs"));
        }
        text(&self.inputs.tree_version, 256, &mut bytes)?;
        text(&self.inputs.constructor_rarity, 256, &mut bytes)?;
        if self.policy.base_prefixes.len() > 32 {
            return Err(error("prefix count bound"));
        }
        let mut prefixes = BTreeSet::new();
        for prefix in &self.policy.base_prefixes {
            text(prefix, 256, &mut bytes)?;
            if prefix.is_empty() || !prefixes.insert(prefix) {
                return Err(error("empty/duplicate prefix"));
            }
        }
        text(&self.policy.title_base_separator, 256, &mut bytes)?;
        let source = &self.source;
        if !digest(&source.upstream_revision, 40)
            || source.files.is_empty()
            || source.files.len() > 512
            || source.construction_spans.is_empty()
            || source.construction_spans.len() > 64
            || source.module_order.len() > 2048
        {
            return Err(error("source inventory bounds"));
        }
        for (path, sha) in &source.files {
            text(path, 4096, &mut bytes)?;
            if !(path.starts_with("src/") || path.starts_with("runtime/lua/"))
                || path.contains('\\')
                || path.split('/').any(|part| matches!(part, "" | "." | ".."))
                || !digest(sha, 64)
            {
                return Err(error("source file identity"));
            }
        }
        for (name, span) in &source.construction_spans {
            text(name, 256, &mut bytes)?;
            text(&span.path, 4096, &mut bytes)?;
            if !source.files.contains_key(&span.path)
                || span.line == 0
                || span.end_line < span.line
                || span.end_line > 1_000_000
                || !digest(&span.sha256, 64)
            {
                return Err(error("source span"));
            }
        }
        for module in &source.module_order {
            text(module, 4096, &mut bytes)?;
            if !source.files.contains_key(module) {
                return Err(error("unknown construction module"));
            }
        }
        for name in [
            "unique_loop",
            "item_constructor",
            "unique_lookup",
            "item_parse",
            "item_assembly",
            "pairs_yield",
            "defaults",
            "stored_cache",
        ] {
            if !source.construction_spans.contains_key(name) {
                return Err(error("missing construction provenance"));
            }
        }
        let prototype = |id: &UniquePrototypeId, bytes: &mut usize| -> Result<()> {
            text(&id.group, 256, bytes)?;
            if id.group.is_empty() || id.index == 0 || !digest(&id.raw_sha256, 64) {
                return Err(error("prototype identity"));
            }
            Ok(())
        };
        let mut previous = None;
        let mut inserted = BTreeMap::new();
        for outcome in &self.prototypes {
            prototype(&outcome.prototype, &mut bytes)?;
            let identity = (&outcome.prototype.group, outcome.prototype.index);
            if previous.is_some_and(|prev| prev >= identity) {
                return Err(error("duplicate/unsorted prototype ledger"));
            }
            previous = Some(identity);
            if outcome.lookup_keys.len() > 32 {
                return Err(error("lookup count bound"));
            }
            if !outcome.lookup_keys.windows(2).all(|pair| pair[0] < pair[1]) {
                return Err(error("duplicate/unsorted lookup keys"));
            }
            for key in &outcome.lookup_keys {
                text(key, 4096, &mut bytes)?;
            }
            if let UniquePrototypeDisposition::Inserted { canonical_key } = &outcome.disposition {
                text(canonical_key, 4096, &mut bytes)?;
                if canonical_key.is_empty()
                    || inserted
                        .insert(canonical_key.as_str(), &outcome.prototype)
                        .is_some()
                {
                    return Err(error("ambiguous constructed canonical key"));
                }
            }
        }
        if inserted.len() != self.entries.len() {
            return Err(error("inserted entry count mismatch"));
        }
        if !self
            .entries
            .windows(2)
            .all(|pair| pair[0].canonical_key < pair[1].canonical_key)
        {
            return Err(error("duplicate/unsorted entries"));
        }
        for entry in &self.entries {
            text(&entry.canonical_key, 4096, &mut bytes)?;
            text(&entry.base_name, 4096, &mut bytes)?;
            prototype(&entry.prototype, &mut bytes)?;
            if entry.base_name.is_empty()
                || inserted.get(entry.canonical_key.as_str()) != Some(&&entry.prototype)
                || (entry.natural_level.is_none() && entry.level.is_none())
                || entry.natural_level.is_some_and(|n| !n.is_finite())
                || entry.level.is_some_and(|n| !n.is_finite())
            {
                return Err(error(format!(
                    "invalid constructed requirement entry {:?}: base={:?} natural={:?} level={:?} ledger_matches={}",
                    entry.canonical_key,
                    entry.base_name,
                    entry.natural_level,
                    entry.level,
                    inserted.get(entry.canonical_key.as_str()) == Some(&&entry.prototype)
                )));
            }
        }
        for outcome in &self.prototypes {
            for key in &outcome.lookup_keys {
                if inserted
                    .get(key.as_str())
                    .is_some_and(|id| **id != outcome.prototype)
                {
                    return Err(error("order-dependent cross-prototype database lookup"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UniqueRequirementCatalog(Arc<UniqueRequirementData>);
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UniqueRequirementLookup<'a> {
    Unavailable(&'a str),
    Ready(Option<&'a UniqueRequirementEntry>),
}
impl UniqueRequirementCatalog {
    pub fn new(data: UniqueRequirementData) -> Result<Self> {
        data.validate()?;
        Ok(Self(Arc::new(data)))
    }
    pub fn data(&self) -> &UniqueRequirementData {
        &self.0
    }
    pub fn lookup(
        &self,
        name: &str,
        title: Option<&str>,
        base_name: Option<&str>,
    ) -> UniqueRequirementLookup<'_> {
        let data = match &self.0.state {
            UniqueRequirementState::Unavailable { reason } => {
                return UniqueRequirementLookup::Unavailable(reason);
            }
            UniqueRequirementState::Complete(data) => data,
        };
        if let Ok(index) = data
            .entries
            .binary_search_by(|entry| entry.canonical_key.as_str().cmp(name))
        {
            return UniqueRequirementLookup::Ready(Some(&data.entries[index]));
        }
        let (Some(title), Some(base_name)) = (title, base_name) else {
            return UniqueRequirementLookup::Ready(None);
        };
        let Some(suffix) = data.policy.base_prefixes.iter().find_map(|prefix| {
            base_name
                .strip_prefix(prefix)
                .filter(|suffix| !suffix.is_empty())
        }) else {
            return UniqueRequirementLookup::Ready(None);
        };
        // Lexicographic comparison against three borrowed byte iterators avoids
        // allocating a temporary canonical key on every search candidate.
        let key = title
            .bytes()
            .chain(data.policy.title_base_separator.bytes())
            .chain(suffix.bytes());
        let index = data
            .entries
            .binary_search_by(|entry| entry.canonical_key.bytes().cmp(key.clone()));
        UniqueRequirementLookup::Ready(index.ok().map(|index| &data.entries[index]))
    }
}
