//! Complete constructed modifier-parser definitions; no numerical capability is implied.
use crate::game_data::GameDataError;
use crate::item_loading::{ItemLoadingSource, ItemSourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

mod factories;
pub use factories::*;
pub(crate) mod programs;
pub use programs::*;

pub const MODIFIER_PARSER_SCHEMA_VERSION: u32 = 7;
type Result<T> = std::result::Result<T, GameDataError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParserTableId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParserCallbackId(pub u32);
macro_rules! dictionaries {
    ($($variant:ident => $source:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum ParserDictionary { $($variant),+ }
        impl ParserDictionary {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub fn source_name(self) -> &'static str { match self { $(Self::$variant => $source),+ } }
        }
    }
}
dictionaries! {
    Conqueror => "conquerorList", Form => "formList", ModName => "modNameList",
    ModFlag => "modFlagList", PreFlag => "preFlagList", ModTag => "modTagList",
    GemIdLookup => "gemIdLookup", PreAnchorSpecial => "oldList", Special => "specialModList",
    Unsupported => "unsupportedModList", Suffix => "suffixTypes", Damage => "dmgTypes",
    Penetration => "penTypes", Resource => "resourceTypes", Regeneration => "regenTypes",
    Degeneration => "degenTypes", Cost => "costTypes", BaseCost => "baseCostTypes",
    Flag => "flagTypes", StatusToEffect => "statusToEffectMap", SkillName => "skillNameList",
    PreSkillName => "preSkillNameList", DeprecatedSkillNames => "deprecatedSkillNames",
    Jewel => "jewelFuncList", JewelOther => "jewelOtherFuncs", JewelSelf => "jewelSelfFuncs",
    JewelSelfUnallocated => "jewelSelfUnallocFuncs", JewelThreshold => "jewelThresholdFuncs",
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserNonFinite {
    PositiveInfinity,
    NegativeInfinity,
    Nan,
}
/// One-based graph IDs retain shared/cyclic tables and closure identity. Nil is
/// explicit for captured values; Lua table entries themselves cannot contain nil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ParserValue {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(String),
    NonFinite(ParserNonFinite),
    Table(ParserTableId),
    Callback(ParserCallbackId),
}
impl ParserValue {
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
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserTable {
    #[serde(deserialize_with = "unique_fields")]
    pub fields: BTreeMap<String, ParserValue>,
    #[serde(deserialize_with = "numeric_keys")]
    pub indexed: BTreeMap<i64, ParserValue>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserUpvalue {
    pub name: String,
    pub value: ParserValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserCallbackKind {
    Lua {
        source: ItemSourceSpan,
    },
    /// Named runtime primitive. C implementation upvalues are opaque VM details.
    Builtin {
        symbol: String,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserEnvironment {
    OriginalGlobals,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserCallback {
    pub kind: ParserCallbackKind,
    /// Ordered Lua upvalues preserve distinct closures sharing source lines.
    /// Builtin primitives have no serialized implementation upvalues.
    pub upvalues: Vec<ParserUpvalue>,
    /// Global reads are not serialized execution or an automatically supported callback.
    pub environment: ParserEnvironment,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserDeclaration {
    pub dictionary: ParserDictionary,
    pub key: String,
    pub source: ItemSourceSpan,
    pub source_order: u32,
    pub phase: String,
    pub payload: Option<ParserValue>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserDependencies {
    /// Exact fields read by the original generated-name loops, for every constructed gem.
    pub gem_generator_inputs: ParserTableId,
    /// Fields consumed by grant/support helpers; unrelated effect execution is deferred.
    pub skill_grant_inputs: ParserTableId,
    pub gem_for_base_name: ParserTableId,
    /// Complete unordered assignment alternatives; ambiguous names have no invented winner.
    #[serde(deserialize_with = "unique_map")]
    pub gem_for_base_name_ambiguities: BTreeMap<String, Vec<String>>,
    #[serde(deserialize_with = "unique_map")]
    pub gem_base_name_assignment_candidates: BTreeMap<String, Vec<String>>,
    pub keystones: ParserTableId,
    pub sorted_grant_gem_keys: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserPolicy {
    pub mod_flags: ParserTableId,
    pub keyword_flags: ParserTableId,
    pub skill_types: ParserTableId,
    pub local_hand_tag: ParserTableId,
    pub immune_effect_blacklist: Vec<String>,
    pub cluster_prefix_pattern: String,
    /// Source method-call precheck; syntax is evaluated only at a selected tag callback.
    pub tag_capture_numeric_pattern: String,
    /// Pattern in the authenticated firstToUpper helper's callback replacement.
    pub first_to_upper_pattern: String,
    /// Literal constructor prefix from the authenticated closed flag wrapper.
    pub flag_mod_type: String,
    pub flag_mod_value: bool,
    pub immune_max_single_words: u32,
    pub immune_combined_min_words_exclusive: u32,
    pub immune_max_part_words: u32,
    pub thorns_base_damage: f64,
    pub doubled_more: f64,
    pub doubled_override: f64,
    pub doubled_global_limit: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserCapability {
    DefinitionsOnly,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifierParserData {
    pub schema_version: u32,
    pub source: ItemLoadingSource,
    #[serde(deserialize_with = "unique_map")]
    pub dictionaries: BTreeMap<ParserDictionary, ParserTableId>,
    pub tables: Vec<ParserTable>,
    pub callbacks: Vec<ParserCallback>,
    #[serde(deserialize_with = "unique_map")]
    pub factories: BTreeMap<ParserCallbackId, ParserFactoryDisposition>,
    #[serde(deserialize_with = "unique_map")]
    pub helpers: BTreeMap<String, ParserCallbackId>,
    pub declarations: Vec<ParserDeclaration>,
    pub dynamic_dependencies: ParserDependencies,
    pub policy: ParserPolicy,
    pub capability: ParserCapability,
    /// Complete source programs and explicitly injected execution admissions.
    /// An absent payload is invalid; an empty authored payload grants no permission.
    pub programs: ParserProgramPayload,
}
#[derive(Debug, Clone)]
pub struct ModifierParserCatalog(Arc<ModifierParserData>);
impl ModifierParserCatalog {
    pub fn factory(&self, id: ParserCallbackId) -> Option<&ParserFactoryDisposition> {
        self.0.factories.get(&id)
    }
    pub fn new(data: ModifierParserData) -> Result<Self> {
        data.validate()?;
        Ok(Self(Arc::new(data)))
    }
    pub fn is_same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn data(&self) -> &ModifierParserData {
        &self.0
    }
    pub fn table(&self, id: ParserTableId) -> Option<&ParserTable> {
        id.0.checked_sub(1)
            .and_then(|i| self.0.tables.get(i as usize))
    }
    pub fn callback(&self, id: ParserCallbackId) -> Option<&ParserCallback> {
        id.0.checked_sub(1)
            .and_then(|i| self.0.callbacks.get(i as usize))
    }
    pub fn dictionary(&self, name: ParserDictionary) -> &ParserTable {
        self.table(self.0.dictionaries[&name])
            .expect("validated dictionary reference")
    }
    pub fn exact(&self, dictionary: ParserDictionary, key: &str) -> Option<&ParserValue> {
        self.dictionary(dictionary).fields.get(key)
    }
}
fn unique_fields<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, ParserValue>, D::Error> {
    struct Fields;
    impl<'de> serde::de::Visitor<'de> for Fields {
        type Value = BTreeMap<String, ParserValue>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique Lua string keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((k, v)) = map.next_entry::<String, ParserValue>()? {
                if out.insert(k, v).is_some() {
                    return Err(serde::de::Error::custom("duplicate parser string key"));
                }
            }
            Ok(out)
        }
    }
    d.deserialize_map(Fields)
}
fn numeric_keys<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<i64, ParserValue>, D::Error> {
    struct Keys;
    impl<'de> serde::de::Visitor<'de> for Keys {
        type Value = BTreeMap<i64, ParserValue>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique canonical Lua integer keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((k, v)) = map.next_entry::<String, ParserValue>()? {
                let n = k.parse::<i64>().map_err(serde::de::Error::custom)?;
                if n.to_string() != k || out.insert(n, v).is_some() {
                    return Err(serde::de::Error::custom(
                        "noncanonical or duplicate parser numeric key",
                    ));
                }
            }
            Ok(out)
        }
    }
    d.deserialize_map(Keys)
}
fn catalog_error(message: &str) -> GameDataError {
    GameDataError(format!("modifier parser catalog: {message}"))
}
fn digest(value: &str, n: usize) -> bool {
    value.len() == n
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn text(value: &str, limit: usize) -> bool {
    value.len() <= limit && !value.contains('\0')
}
impl ModifierParserData {
    pub fn validate(&self) -> Result<()> {
        self.validate_definitions()?;
        self.programs
            .validate_owner(self)
            .map_err(|error| GameDataError(error.to_string()))
    }
    fn validate_definitions(&self) -> Result<()> {
        if self.schema_version != MODIFIER_PARSER_SCHEMA_VERSION
            || !digest(&self.source.upstream_revision, 40)
            || self.tables.is_empty()
            || self.tables.len() > 100_000
            || self.callbacks.len() > 20_000
            || self.declarations.len() > 100_000
            || self.helpers.len() > 128
            || self.source.files.is_empty()
            || self.source.files.len() > 512
            || self.source.construction_spans.len() > 512
            || self.source.module_order.len() > 1024
        {
            return Err(catalog_error("invalid version or catalog bounds"));
        }
        factories::validate(self)?;
        if self
            .source
            .files
            .keys()
            .any(|path| !path.starts_with("src/"))
        {
            return Err(catalog_error("invalid source identity"));
        }
        let graph = crate::source_program::graph::GraphValidation::new(
            &self.source,
            &self.tables,
            &self.callbacks,
        )?;
        let charge = |amount| graph.charge(amount);
        let span = |value| graph.span(value);
        let table = |id| graph.table(id);
        let callback = |id| graph.callback(id);
        let value = |value, allow_nil| graph.value(value, allow_nil);
        let mut last_order = 0;
        for d in &self.declarations {
            charge(d.key.len() + d.phase.len())?;
            if d.key.is_empty()
                || !text(&d.key, 4096)
                || d.phase.is_empty()
                || !text(&d.phase, 128)
                || d.source_order <= last_order
            {
                return Err(catalog_error("invalid declaration evidence"));
            }
            last_order = d.source_order;
            span(&d.source)?;
            if let Some(v) = &d.payload {
                value(v, false)?;
            }
        }
        if self.dictionaries.len() != ParserDictionary::ALL.len()
            || ParserDictionary::ALL
                .iter()
                .any(|d| !self.dictionaries.contains_key(d))
        {
            return Err(catalog_error("missing dictionary"));
        }
        for id in self.dictionaries.values() {
            table(*id)?;
            if !self.tables[id.0 as usize - 1].indexed.is_empty() {
                return Err(catalog_error("dictionary contains non-string keys"));
            }
        }
        for (name, id) in &self.helpers {
            charge(name.len())?;
            if name.is_empty() || !text(name, 128) {
                return Err(catalog_error("invalid helper name"));
            }
            callback(*id)?;
        }
        let d = &self.dynamic_dependencies;
        for id in [
            d.gem_generator_inputs,
            d.skill_grant_inputs,
            d.gem_for_base_name,
            d.keystones,
            self.policy.mod_flags,
            self.policy.keyword_flags,
            self.policy.skill_types,
            self.policy.local_hand_tag,
        ] {
            table(id)?;
        }
        if d.sorted_grant_gem_keys.len() > 10_000
            || d.sorted_grant_gem_keys
                .iter()
                .any(|v| v.is_empty() || !text(v, 4096))
            || d.sorted_grant_gem_keys.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(catalog_error("invalid sorted grant gem keys"));
        }
        if d.gem_for_base_name_ambiguities.len() > 10_000 {
            return Err(catalog_error("too many ambiguous gem names"));
        }
        let names = &self.tables[d.gem_for_base_name.0 as usize - 1];
        let gems = &self.tables[d.gem_generator_inputs.0 as usize - 1];
        if d.gem_base_name_assignment_candidates.len() > 20_000
            || d.gem_base_name_assignment_candidates.len()
                != names.fields.len() + d.gem_for_base_name_ambiguities.len()
        {
            return Err(catalog_error("missing gem assignment evidence"));
        }
        for (key, values) in &d.gem_base_name_assignment_candidates {
            if key.is_empty()
                || !text(key, 4096)
                || values.is_empty()
                || values.len() > 10_000
                || values.windows(2).any(|w| w[0] >= w[1])
                || values.iter().any(|v| !gems.fields.contains_key(v))
            {
                return Err(catalog_error("invalid gem assignment candidates"));
            }
            if values.len() == 1 {
                if names.fields.get(key) != Some(&ParserValue::Text(values[0].clone()))
                    || d.gem_for_base_name_ambiguities.contains_key(key)
                {
                    return Err(catalog_error("invalid resolved gem name partition"));
                }
            } else if names.fields.contains_key(key)
                || d.gem_for_base_name_ambiguities.get(key) != Some(values)
            {
                return Err(catalog_error("invalid ambiguous gem name alternatives"));
            }
        }
        if !d.sorted_grant_gem_keys.iter().eq(gems.fields.keys()) {
            return Err(catalog_error(
                "sorted grant keys differ from complete gem input inventory",
            ));
        }
        for text in &d.sorted_grant_gem_keys {
            charge(text.len())?;
        }
        for map in [
            &d.gem_for_base_name_ambiguities,
            &d.gem_base_name_assignment_candidates,
        ] {
            for (key, values) in map {
                charge(key.len())?;
                for value in values {
                    charge(value.len())?;
                }
            }
        }
        let p = &self.policy;
        charge(p.cluster_prefix_pattern.len())?;
        charge(p.tag_capture_numeric_pattern.len())?;
        charge(p.first_to_upper_pattern.len())?;
        charge(p.flag_mod_type.len())?;
        for text in &p.immune_effect_blacklist {
            charge(text.len())?;
        }

        if p.immune_effect_blacklist.len() > 256
            || p.immune_effect_blacklist.iter().any(|v| !text(v, 4096))
            || p.cluster_prefix_pattern.is_empty()
            || !text(&p.cluster_prefix_pattern, 4096)
            || !text(&p.tag_capture_numeric_pattern, 4096)
            || !text(&p.first_to_upper_pattern, 4096)
            || p.flag_mod_type.len() > 4096
            || [
                p.immune_max_single_words,
                p.immune_combined_min_words_exclusive,
                p.immune_max_part_words,
            ]
            .iter()
            .any(|v| *v > 128)
            || [
                p.thorns_base_damage,
                p.doubled_more,
                p.doubled_override,
                p.doubled_global_limit,
            ]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e12)
        {
            return Err(catalog_error("invalid parser policy"));
        }
        Ok(())
    }
}

pub(crate) fn unique_map<'de, D, K, V>(d: D) -> std::result::Result<BTreeMap<K, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    struct Map<K, V>(std::marker::PhantomData<(K, V)>);
    impl<'de, K: Deserialize<'de> + Ord, V: Deserialize<'de>> serde::de::Visitor<'de> for Map<K, V> {
        type Value = BTreeMap<K, V>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique parser map keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((k, v)) = map.next_entry::<K, V>()? {
                if out.insert(k, v).is_some() {
                    return Err(serde::de::Error::custom("duplicate parser map key"));
                }
            }
            Ok(out)
        }
    }
    d.deserialize_map(Map(std::marker::PhantomData))
}
