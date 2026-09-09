//! Complete source item definitions. Membership never grants numerical capability.
use crate::game_data::GameDataError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const ITEM_LOADING_SCHEMA_VERSION: u32 = 2;
type Result<T> = std::result::Result<T, GameDataError>;
fn error(message: impl std::fmt::Display) -> GameDataError {
    GameDataError(format!("item loading catalog: {message}"))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemLoadingCapability {
    DefinitionsOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSourceSpan {
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLoadingSource {
    pub upstream_revision: String,
    pub files: BTreeMap<String, String>,
    pub construction_spans: BTreeMap<String, ItemSourceSpan>,
    pub module_order: Vec<String>,
}
/// A Lua function is retained as source evidence, never as an executable effect.
/// Its captured state and behavior are not represented by this descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemOpaqueFunction {
    pub callback: ItemSourceSpan,
}
/// Scalars retain their source types. Mixed and sparse Lua tables retain numeric
/// keys separately from string keys; integer 1 and string "1" cannot collide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ItemMetadataValue {
    Boolean(bool),
    Number(f64),
    Text(String),
    Array(Vec<Self>),
    Table(ItemMetadataTable),
    Callback(ItemOpaqueFunction),
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemMetadataTable {
    #[serde(deserialize_with = "unique_named_map")]
    pub fields: BTreeMap<String, ItemMetadataValue>,
    #[serde(deserialize_with = "numeric_keys")]
    pub indexed: BTreeMap<i64, ItemMetadataValue>,
}
fn unique_named_map<'de, D, V>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct Keys<V>(std::marker::PhantomData<V>);
    impl<'de, V: Deserialize<'de>> serde::de::Visitor<'de> for Keys<V> {
        type Value = BTreeMap<String, V>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique named item keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, V>()? {
                if out.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate named item key"));
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(Keys(std::marker::PhantomData))
}
fn numeric_keys<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<i64, ItemMetadataValue>, D::Error> {
    struct Keys;
    impl<'de> serde::de::Visitor<'de> for Keys {
        type Value = BTreeMap<i64, ItemMetadataValue>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique canonical numeric Lua keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, ItemMetadataValue>()? {
                let index = key.parse::<i64>().map_err(serde::de::Error::custom)?;
                if index.to_string() != key || out.insert(index, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "noncanonical or duplicate numeric item key",
                    ));
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(Keys)
}
impl ItemMetadataValue {
    pub fn as_str(&self) -> Option<&str> {
        if let Self::Text(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Number(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Boolean(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_table(&self) -> Option<&ItemMetadataTable> {
        if let Self::Table(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&[Self]> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseDefinition {
    pub name: String,
    pub item_type: String,
    pub source_module: String,
    /// Complete constructed source record, including the typed identity's type.
    pub fields: ItemMetadataTable,
}
impl ItemBaseDefinition {
    pub fn field(&self, key: &str) -> Option<&ItemMetadataValue> {
        self.fields.fields.get(key)
    }
    pub fn sub_type(&self) -> Option<&str> {
        self.field("subType").and_then(ItemMetadataValue::as_str)
    }
    pub fn hidden(&self) -> Option<bool> {
        self.field("hidden").and_then(ItemMetadataValue::as_bool)
    }
    pub fn quality(&self) -> Option<f64> {
        self.field("quality").and_then(ItemMetadataValue::as_f64)
    }
    pub fn implicit(&self) -> Option<&str> {
        self.field("implicit").and_then(ItemMetadataValue::as_str)
    }
    pub fn requirements(&self) -> Option<&ItemMetadataTable> {
        self.field("req").and_then(ItemMetadataValue::as_table)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemCatalystDefinition {
    pub name: String,
    pub descriptor: String,
    pub tags: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLoadingPolicy {
    pub default_affix_quality: f64,
    pub default_item_quality: f64,
    pub catalysts: Vec<ItemCatalystDefinition>,
    pub line_flags: BTreeSet<String>,
    /// Literal source color-code lookup keys accepted after Rarity uppercasing.
    pub rarities: BTreeSet<String>,
    pub header_names: BTreeSet<String>,
    /// Complete source defence-header branch: original spelling to armourData key.
    /// Header membership describes a loading operation, not numerical capability.
    #[serde(deserialize_with = "unique_named_map")]
    pub defence_header_keys: BTreeMap<String, String>,
    #[serde(deserialize_with = "unique_named_map")]
    pub compatibility: BTreeMap<String, ItemMetadataValue>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLoadingData {
    pub schema_version: u32,
    pub capability: ItemLoadingCapability,
    pub source: ItemLoadingSource,
    pub policy: ItemLoadingPolicy,
    pub bases: Vec<ItemBaseDefinition>,
    pub modifier_tables: BTreeMap<String, ItemMetadataTable>,
    /// Ordered raw unique prototypes; these are not Main.uniqueDB parsed items.
    pub unique_groups: BTreeMap<String, Vec<String>>,
    /// All source versions, before selecting a tree or executing radius logic.
    pub jewel_radii: ItemMetadataTable,
}
/// Borrowed source identity rewrite. Missing base definitions are legitimate:
/// a complete catalog lookup then corresponds to assigning nil to the Lua base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemBaseRewrite<'a> {
    pub from: &'a str,
    pub to: &'a str,
}
#[derive(Debug)]
struct Inner {
    data: ItemLoadingData,
    by_name: BTreeMap<String, usize>,
}
#[derive(Debug, Clone)]
pub struct ItemLoadingCatalog(Arc<Inner>);
impl ItemLoadingCatalog {
    pub fn new(data: ItemLoadingData) -> Result<Self> {
        data.validate()?;
        let by_name = data
            .bases
            .iter()
            .enumerate()
            .map(|(i, b)| (b.name.clone(), i))
            .collect();
        Ok(Self(Arc::new(Inner { data, by_name })))
    }
    pub fn data(&self) -> &ItemLoadingData {
        &self.0.data
    }
    pub fn bases(&self) -> &[ItemBaseDefinition] {
        &self.0.data.bases
    }
    pub fn base(&self, name: &str) -> Option<&ItemBaseDefinition> {
        self.0.by_name.get(name).map(|i| &self.bases()[*i])
    }
    pub fn policy(&self) -> &ItemLoadingPolicy {
        &self.0.data.policy
    }
    pub fn defence_header_key(&self, header: &str) -> Option<&str> {
        self.policy()
            .defence_header_keys
            .get(header)
            .map(String::as_str)
    }
    pub fn armour_header_rewrite(&self, header: &str) -> Option<ItemBaseRewrite<'_>> {
        let rule = self
            .policy()
            .compatibility
            .get("base_aliases")?
            .as_table()?
            .fields
            .get("armour_header_rewrites")?
            .as_table()?
            .fields
            .get(header)?
            .as_table()?;
        Some(ItemBaseRewrite {
            from: rule.fields.get("from")?.as_str()?,
            to: rule.fields.get("to")?.as_str()?,
        })
    }
    pub fn modifier_table(&self, name: &str) -> Option<&ItemMetadataTable> {
        self.0.data.modifier_tables.get(name)
    }
    pub fn unique_groups(&self) -> &BTreeMap<String, Vec<String>> {
        &self.0.data.unique_groups
    }
}
fn text(value: &str, max: usize) -> Result<()> {
    if value.len() > max || value.contains('\0') {
        Err(error("invalid or oversized text"))
    } else {
        Ok(())
    }
}
fn digest(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn path(s: &str) -> bool {
    s.starts_with("src/")
        && !s.contains('\\')
        && !s.split('/').any(|v| v.is_empty() || v == "." || v == "..")
}
impl ItemLoadingPolicy {
    fn validate_tables(&self) -> Result<()> {
        let get = |key: &str| {
            self.compatibility
                .get(key)
                .ok_or_else(|| error(format!("missing required policy {key}")))
        };
        let named = |key: &str| -> Result<&ItemMetadataTable> {
            let t = get(key)?
                .as_table()
                .ok_or_else(|| error(format!("policy {key} must be a named table")))?;
            if !t.indexed.is_empty() {
                return Err(error(format!("policy {key} has numeric keys")));
            }
            Ok(t)
        };
        let texts = |key: &str| -> Result<()> {
            let v = get(key)?
                .as_array()
                .ok_or_else(|| error(format!("policy {key} must be an array")))?;
            if v.len() > 256
                || v.iter()
                    .any(|v| v.as_str().is_none_or(|s| s.is_empty() || s.len() > 4096))
            {
                return Err(error(format!("policy {key} has invalid text values")));
            }
            Ok(())
        };
        let aliases = named("base_aliases")?;
        let rewrites = aliases
            .fields
            .get("armour_header_rewrites")
            .and_then(ItemMetadataValue::as_table)
            .ok_or_else(|| error("missing named armour header rewrites"))?;
        if !rewrites.indexed.is_empty() || rewrites.fields.len() > 256 {
            return Err(error("invalid armour header rewrite bounds/keys"));
        }
        if self.defence_header_keys.len() > 256 {
            return Err(error("defence header count bound"));
        }
        for (header, key) in &self.defence_header_keys {
            text(header, 256)?;
            text(key, 256)?;
            if header.is_empty() || key.is_empty() || !self.header_names.contains(header) {
                return Err(error("invalid defence header identity or stored key"));
            }
            // Unlike hidden_specs, these tables describe different state operations.
            if named("selection_headers")?.fields.contains_key(header)
                || named("header_assignments")?.fields.contains_key(header)
            {
                return Err(error(
                    "defence header conflicts with another header operation",
                ));
            }
        }
        for (header, value) in &rewrites.fields {
            if !self.defence_header_keys.contains_key(header) {
                return Err(error("armour rewrite has no defence header"));
            }
            let rule = value
                .as_table()
                .ok_or_else(|| error("armour rewrite must be named"))?;
            if !rule.indexed.is_empty() || rule.fields.len() != 2 {
                return Err(error("armour rewrite must have only from/to fields"));
            }
            for key in ["from", "to"] {
                let name = rule
                    .fields
                    .get(key)
                    .and_then(ItemMetadataValue::as_str)
                    .ok_or_else(|| error("armour rewrite from/to must be text"))?;
                text(name, 4096)?;
                if name.is_empty() {
                    return Err(error("empty armour rewrite base identity"));
                }
            }
            // Do not require either string identity to resolve to a base. The
            // original complete PoE2 data retains legacy rules with absent bases.
        }
        for key in ["noncorruptible_types", "mod_magnitude_patterns"] {
            texts(key)?;
        }
        for key in ["hidden_specs", "selection_headers"] {
            if named(key)?
                .fields
                .values()
                .any(|v| v.as_bool() != Some(true))
            {
                return Err(error(format!(
                    "policy {key} must contain true membership values"
                )));
            }
        }
        for key in ["superior_prefix", "fallback_modifier_table"] {
            if get(key)?.as_str().is_none() {
                return Err(error(format!("policy {key} must be text")));
            }
        }
        for v in named("fallback_jewel_socket_counts")?.fields.values() {
            if v.as_f64()
                .is_none_or(|n| !n.is_finite() || !(0.0..=4096.0).contains(&n) || n.fract() != 0.0)
            {
                return Err(error("invalid fallback jewel socket count"));
            }
        }
        for v in named("header_assignments")?.fields.values() {
            let t = v
                .as_table()
                .ok_or_else(|| error("header assignment must be a table"))?;
            if !t.indexed.is_empty()
                || t.fields.len() != 2
                || t.fields
                    .get("kind")
                    .and_then(ItemMetadataValue::as_str)
                    .is_none_or(|s| !["number", "text", "presence", "boolean"].contains(&s))
                || t.fields
                    .get("field")
                    .and_then(ItemMetadataValue::as_str)
                    .is_none_or(|s| {
                        s.is_empty()
                            || s.len() > 128
                            || !s
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
                    })
            {
                return Err(error("invalid header assignment operation"));
            }
        }
        let roles = named("rarity_roles")?;
        for key in ["default", "normal", "magic", "unique", "relic"] {
            if roles
                .fields
                .get(key)
                .and_then(ItemMetadataValue::as_str)
                .is_none_or(|s| !self.rarities.contains(s))
            {
                return Err(error(
                    "rarity role does not reference accepted source token",
                ));
            }
        }
        for key in ["literal_state_flags", "postparse_line_effects"] {
            for v in named(key)?.fields.values() {
                let t = v
                    .as_table()
                    .ok_or_else(|| error("literal line effects must be named tables"))?;
                if !t.indexed.is_empty() || t.fields.values().any(|v| v.as_bool().is_none()) {
                    return Err(error("literal line effects must be boolean"));
                }
            }
        }
        Ok(())
    }
}
impl ItemLoadingData {
    pub fn validate(&self) -> Result<()> {
        self.policy.validate_tables()?;
        if self.schema_version != ITEM_LOADING_SCHEMA_VERSION
            || !digest(&self.source.upstream_revision, 40)
            || self.source.files.is_empty()
            || self.source.files.len() > 512
            || self.bases.is_empty()
            || self.bases.len() > 50_000
        {
            return Err(error("invalid version/source/base bounds"));
        }
        for (p, h) in &self.source.files {
            if !path(p) || !digest(h, 64) {
                return Err(error("invalid source file identity"));
            }
        }
        let validate_span = |s: &ItemSourceSpan| -> Result<()> {
            if !self.source.files.contains_key(&s.path)
                || s.line == 0
                || s.end_line < s.line
                || s.end_line > 1_000_000
                || !digest(&s.sha256, 64)
            {
                Err(error("invalid source span"))
            } else {
                Ok(())
            }
        };
        if self.source.construction_spans.is_empty()
            || self.source.construction_spans.len() > 64
            || self.source.module_order.len() > 512
        {
            return Err(error("invalid source construction bounds"));
        }
        for (k, s) in &self.source.construction_spans {
            text(k, 256)?;
            validate_span(s)?;
        }
        for p in &self.source.module_order {
            if !self.source.files.contains_key(p) {
                return Err(error("unknown construction module"));
            }
        }
        let mut budget = 1_500_000usize;
        let mut strings = self
            .policy
            .defence_header_keys
            .iter()
            .map(|(header, key)| header.len() + key.len())
            .sum::<usize>();
        fn value(
            v: &ItemMetadataValue,
            depth: usize,
            budget: &mut usize,
            strings: &mut usize,
            span: &impl Fn(&ItemSourceSpan) -> Result<()>,
        ) -> Result<()> {
            if depth > 24 || *budget == 0 {
                return Err(error("metadata exceeds depth/value bounds"));
            }
            *budget -= 1;
            match v {
                ItemMetadataValue::Boolean(_) => Ok(()),
                ItemMetadataValue::Number(n) => {
                    if n.is_finite() {
                        Ok(())
                    } else {
                        Err(error("nonfinite metadata"))
                    }
                }
                ItemMetadataValue::Text(s) => {
                    text(s, 4096)?;
                    *strings = strings
                        .checked_add(s.len())
                        .ok_or_else(|| error("text bound overflow"))?;
                    if *strings > 32 * 1024 * 1024 {
                        Err(error("aggregate metadata text bound"))
                    } else {
                        Ok(())
                    }
                }
                ItemMetadataValue::Array(a) => {
                    if a.len() > 100_000 {
                        return Err(error("array bound"));
                    }
                    for v in a {
                        value(v, depth + 1, budget, strings, span)?;
                    }
                    Ok(())
                }
                ItemMetadataValue::Table(t) => table(t, depth + 1, budget, strings, span),
                ItemMetadataValue::Callback(f) => span(&f.callback),
            }
        }
        fn table(
            t: &ItemMetadataTable,
            depth: usize,
            budget: &mut usize,
            strings: &mut usize,
            span: &impl Fn(&ItemSourceSpan) -> Result<()>,
        ) -> Result<()> {
            if t.fields.len() + t.indexed.len() > 100_000 {
                return Err(error("table bound"));
            }
            for (k, v) in &t.fields {
                text(k, 4096)?;
                value(v, depth, budget, strings, span)?;
            }
            for (k, v) in &t.indexed {
                if !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(k) {
                    return Err(error("numeric table key exceeds exact f64 integer"));
                }
                value(v, depth, budget, strings, span)?;
            }
            Ok(())
        }
        let mut names = BTreeSet::new();
        for b in &self.bases {
            text(&b.name, 4096)?;
            text(&b.item_type, 256)?;
            if b.name.is_empty()
                || b.item_type.is_empty()
                || !names.insert(&b.name)
                || !self.source.files.contains_key(&b.source_module)
                || b.field("type").and_then(ItemMetadataValue::as_str) != Some(b.item_type.as_str())
            {
                return Err(error("invalid/duplicate base identity"));
            }
            table(&b.fields, 0, &mut budget, &mut strings, &validate_span)?;
            for (k, expected) in [
                ("subType", 0),
                ("hidden", 1),
                ("quality", 2),
                ("implicit", 0),
                ("req", 3),
            ] {
                if let Some(v) = b.field(k) {
                    let valid = match expected {
                        0 => v.as_str().is_some(),
                        1 => v.as_bool().is_some(),
                        2 => v.as_f64().is_some(),
                        _ => v.as_table().is_some(),
                    };
                    if !valid {
                        return Err(error(format!("base {k} has wrong source type")));
                    }
                }
            }
        }
        if !self.policy.default_affix_quality.is_finite()
            || !(0.0..=1.0).contains(&self.policy.default_affix_quality)
            || !self.policy.default_item_quality.is_finite()
            || !(0.0..=100.0).contains(&self.policy.default_item_quality)
        {
            return Err(error("invalid default quality"));
        }
        if self.policy.catalysts.len() > 256
            || self.policy.rarities.is_empty()
            || self.policy.rarities.len() > 256
            || self.policy.line_flags.is_empty()
            || self.policy.line_flags.len() > 256
            || self.policy.header_names.len() > 256
            || self.policy.compatibility.len() > 256
        {
            return Err(error("policy bounds"));
        }
        let mut catalysts = BTreeSet::new();
        for c in &self.policy.catalysts {
            text(&c.name, 256)?;
            text(&c.descriptor, 256)?;
            if c.name.is_empty() || !catalysts.insert(&c.name) || c.tags.len() > 256 {
                return Err(error("catalyst identity/tags"));
            }
            for t in &c.tags {
                text(t, 256)?;
            }
        }
        for s in self
            .policy
            .rarities
            .iter()
            .chain(&self.policy.line_flags)
            .chain(&self.policy.header_names)
        {
            text(s, 256)?;
            if s.is_empty() {
                return Err(error("empty policy token"));
            }
        }
        for (k, v) in &self.policy.compatibility {
            text(k, 256)?;
            value(v, 0, &mut budget, &mut strings, &validate_span)?;
        }
        if self.modifier_tables.is_empty()
            || self.modifier_tables.len() > 128
            || self.unique_groups.len() > 128
        {
            return Err(error("definition group bounds"));
        }
        for (k, t) in &self.modifier_tables {
            text(k, 256)?;
            table(t, 0, &mut budget, &mut strings, &validate_span)?;
        }
        for (k, a) in &self.unique_groups {
            text(k, 256)?;
            if a.len() > 50_000 {
                return Err(error("unique prototype count"));
            }
            for s in a {
                text(s, 65_536)?;
                strings = strings
                    .checked_add(s.len())
                    .ok_or_else(|| error("text bound overflow"))?;
                if strings > 32 * 1024 * 1024 {
                    return Err(error("aggregate metadata text bound"));
                }
            }
        }
        table(
            &self.jewel_radii,
            0,
            &mut budget,
            &mut strings,
            &validate_span,
        )
    }
}
