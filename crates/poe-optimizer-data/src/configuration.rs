//! Portable source-owned configuration metadata; callbacks never grant native capability.
use crate::game_data::GameDataError;
use poe_optimizer_core::options::Scalar;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const CONFIGURATION_SCHEMA_VERSION: u32 = 1;
const MAX_DEFINITIONS: usize = 8192;
const MAX_TABLE_ROWS: u32 = 100_000;
const MAX_OPTIONS: usize = 4096;
const MAX_TOTAL_OPTIONS: usize = 65_536;
const MAX_METADATA_VALUES: usize = 250_000;
const MAX_METADATA_DEPTH: usize = 24;
const MAX_STRING_BYTES: usize = 4096;
type Result<T> = std::result::Result<T, GameDataError>;
fn error(message: impl std::fmt::Display) -> GameDataError {
    GameDataError(format!("configuration: {message}"))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigCapability {
    MetadataOnly,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigScalarKind {
    Boolean,
    Number,
    Text,
}
impl ConfigScalarKind {
    pub fn of(value: &Scalar) -> Self {
        match value {
            Scalar::Boolean(_) => Self::Boolean,
            Scalar::Number(_) => Self::Number,
            Scalar::Text(_) => Self::Text,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigWidgetKind {
    Check,
    Count,
    CountAllowZero,
    Integer,
    Float,
    List,
    Text,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSourceSpan {
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    /// Digest of the extracted source span; file digest is in source.files.
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSourceIdentity {
    pub upstream_revision: String,
    /// Recorded relative source paths and digests; enclosing host policy
    /// determines trust. Structural validation alone does not verify source bytes.
    pub files: BTreeMap<String, String>,
    pub create_config_set: ConfigSourceSpan,
    pub get_default_state: ConfigSourceSpan,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSourceLocation {
    pub path: String,
    pub line: u32,
}
/// Typed bounded source evidence, never executable effects or native handler IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ConfigMetadataValue {
    Boolean(bool),
    Number(f64),
    Text(String),
    Array(Vec<ConfigMetadataValue>),
    Object(BTreeMap<String, ConfigMetadataValue>),
    Callback(ConfigSourceSpan),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigQuestSource {
    pub source_index: u32,
    pub record: BTreeMap<String, ConfigMetadataValue>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDefinitionSource {
    pub location: ConfigSourceLocation,
    #[serde(deserialize_with = "required_option")]
    pub quest: Option<ConfigQuestSource>,
    #[serde(deserialize_with = "required_option")]
    pub generator: Option<ConfigSourceSpan>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDeclaredDefaults {
    // Declared defaults are source assignments, not an authoring-domain check:
    // CreateConfigSet copies even a scalar outside its widget's usual kind.
    #[serde(deserialize_with = "required_option")]
    pub input: Option<Scalar>,
    #[serde(deserialize_with = "required_option")]
    pub placeholder: Option<Scalar>,
    /// Source defaultIndex, one-based; applied after input, including false.
    #[serde(deserialize_with = "required_option")]
    pub option_index: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigOption {
    pub index: u32,
    pub value: Scalar,
    #[serde(deserialize_with = "required_option")]
    pub label: Option<String>,
    pub metadata: BTreeMap<String, ConfigMetadataValue>,
}
impl ConfigOption {
    /// Typed source equality: -0 == 0, but numbers never match text.
    pub fn matches(&self, value: &Scalar) -> bool {
        self.value == *value
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDefinition {
    /// Occurrence ID scoped by catalog source/content, not an alias for key.
    pub id: String,
    pub key: String,
    pub source_table_index: u32,
    pub source_variable_order: u32,
    pub widget: ConfigWidgetKind,
    pub scalar_kinds: Vec<ConfigScalarKind>,
    #[serde(deserialize_with = "required_option")]
    pub label: Option<String>,
    pub options: Vec<ConfigOption>,
    pub defaults: ConfigDeclaredDefaults,
    pub source: ConfigDefinitionSource,
    /// All other observed row fields, including apply and UI dependencies.
    pub metadata: BTreeMap<String, ConfigMetadataValue>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationData {
    pub schema_version: u32,
    pub source: ConfigSourceIdentity,
    /// Includes source presentation rows omitted from definitions.
    pub source_table_rows: u32,
    pub definitions: Vec<ConfigDefinition>,
    pub capability: ConfigCapability,
}
/// Initial scalar maps before imported overlays or dynamic callbacks. Excludes
/// UI fallback, custom blocks, final scenario and effects.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigInitialState {
    pub inputs: BTreeMap<String, Scalar>,
    pub placeholders: BTreeMap<String, Scalar>,
}
#[derive(Debug)]
struct CatalogInner {
    data: ConfigurationData,
    by_key: BTreeMap<String, Vec<usize>>,
    by_id: BTreeMap<String, usize>,
}
/// Independently usable without an evaluator or Lua. Clone shares storage.
/// Construction validates structure; host package policy establishes trust.
#[derive(Debug, Clone)]
pub struct ConfigDefinitionCatalog(Arc<CatalogInner>);
impl ConfigDefinitionCatalog {
    pub fn new(data: ConfigurationData) -> Result<Self> {
        data.validate()?;
        let mut by_key: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut by_id = BTreeMap::new();
        for (index, definition) in data.definitions.iter().enumerate() {
            by_key
                .entry(definition.key.clone())
                .or_default()
                .push(index);
            by_id.insert(definition.id.clone(), index);
        }
        Ok(Self(Arc::new(CatalogInner {
            data,
            by_key,
            by_id,
        })))
    }
    pub fn data(&self) -> &ConfigurationData {
        &self.0.data
    }
    pub fn definitions(&self) -> &[ConfigDefinition] {
        &self.0.data.definitions
    }
    pub fn definitions_for_key(&self, key: &str) -> impl Iterator<Item = &ConfigDefinition> {
        self.0
            .by_key
            .get(key)
            .into_iter()
            .flatten()
            .map(|index| &self.definitions()[*index])
    }
    pub fn definition(&self, id: &str) -> Option<&ConfigDefinition> {
        self.0
            .by_id
            .get(id)
            .map(|index| &self.definitions()[*index])
    }
    pub fn unique_key_count(&self) -> usize {
        self.0.by_key.len()
    }
    /// Literal CreateConfigSet order: absent defaults remove previous duplicate
    /// values, then defaultIndex overwrites input. No widget fallback is applied.
    pub fn initial_state(&self) -> ConfigInitialState {
        let mut state = ConfigInitialState::default();
        for definition in self.definitions() {
            assign(
                &mut state.inputs,
                &definition.key,
                &definition.defaults.input,
            );
            assign(
                &mut state.placeholders,
                &definition.key,
                &definition.defaults.placeholder,
            );
            if let Some(index) = definition.defaults.option_index {
                state.inputs.insert(
                    definition.key.clone(),
                    definition.options[index as usize - 1].value.clone(),
                );
            }
        }
        state
    }
}
fn assign(map: &mut BTreeMap<String, Scalar>, key: &str, value: &Option<Scalar>) {
    if let Some(value) = value {
        map.insert(key.into(), value.clone());
    } else {
        map.remove(key);
    }
}
impl ConfigurationData {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CONFIGURATION_SCHEMA_VERSION {
            return Err(error("unsupported catalog schema_version"));
        }
        if self.definitions.is_empty()
            || self.definitions.len() > MAX_DEFINITIONS
            || self.source_table_rows < self.definitions.len() as u32
            || self.source_table_rows > MAX_TABLE_ROWS
        {
            return Err(error(
                "definition/table size exceeds bounded catalog domain",
            ));
        }
        self.source.validate()?;
        let mut ids = BTreeSet::new();
        let mut table_index = 0;
        let mut total_options = 0usize;
        let mut metadata_count = 0usize;
        for (index, row) in self.definitions.iter().enumerate() {
            identifier(&row.id, "definition ID", 256)?;
            identifier(&row.key, "configuration key", 1024)?;
            if !ids.insert(&row.id) {
                return Err(error("duplicate definition occurrence ID"));
            }
            if row.source_variable_order as usize != index + 1
                || row.source_table_index <= table_index
                || row.source_table_index > self.source_table_rows
            {
                return Err(error(
                    "definition source positions must retain increasing table order and contiguous one-based variable order",
                ));
            }
            table_index = row.source_table_index;
            validate_location(&row.source.location, &self.source)?;
            if row.source.quest.is_some() != row.source.generator.is_some() {
                return Err(error(
                    "quest evidence requires both quest record and generator span",
                ));
            }
            if let Some(quest) = &row.source.quest {
                if quest.source_index == 0
                    || quest.source_index > MAX_TABLE_ROWS
                    || quest.record.is_empty()
                {
                    return Err(error("invalid quest record evidence"));
                }
                validate_metadata_map(&quest.record, &self.source, 0, &mut metadata_count)?;
            }
            if let Some(span) = &row.source.generator {
                validate_span(span, &self.source)?;
            }
            if let Some(label) = &row.label {
                text(label)?;
            }
            let kinds: BTreeSet<_> = row.scalar_kinds.iter().copied().collect();
            if kinds.is_empty()
                || kinds.len() != row.scalar_kinds.len()
                || row.scalar_kinds.len() > 3
            {
                return Err(error("scalar kinds must be nonempty and unique"));
            }
            total_options = total_options
                .checked_add(row.options.len())
                .ok_or_else(|| error("option count overflow"))?;
            if row.options.len() > MAX_OPTIONS || total_options > MAX_TOTAL_OPTIONS {
                return Err(error("too many configuration options"));
            }
            let expected: BTreeSet<_> = match row.widget {
                ConfigWidgetKind::Check => [ConfigScalarKind::Boolean].into(),
                ConfigWidgetKind::Count
                | ConfigWidgetKind::CountAllowZero
                | ConfigWidgetKind::Integer
                | ConfigWidgetKind::Float => [ConfigScalarKind::Number].into(),
                ConfigWidgetKind::Text => [ConfigScalarKind::Text].into(),
                ConfigWidgetKind::List => row
                    .options
                    .iter()
                    .map(|option| ConfigScalarKind::of(&option.value))
                    .collect(),
            };
            if kinds != expected || (row.widget == ConfigWidgetKind::List) == row.options.is_empty()
            {
                return Err(error("widget, option values and scalar kinds disagree"));
            }
            for (option_index, option) in row.options.iter().enumerate() {
                if option.index as usize != option_index + 1 {
                    return Err(error("option indexes must be one-based in source order"));
                }
                scalar(&option.value)?;
                if let Some(label) = &option.label {
                    text(label)?;
                }
                if option
                    .metadata
                    .keys()
                    .any(|key| matches!(key.as_str(), "val" | "label"))
                {
                    return Err(error("option metadata repeats structural fields"));
                }
                validate_metadata_map(&option.metadata, &self.source, 0, &mut metadata_count)?;
            }
            for value in [&row.defaults.input, &row.defaults.placeholder]
                .into_iter()
                .flatten()
            {
                scalar(value)?;
            }
            if let Some(option_index) = row.defaults.option_index
                && (row.widget != ConfigWidgetKind::List
                    || option_index == 0
                    || option_index as usize > row.options.len())
            {
                return Err(error(
                    "defaultIndex must select a valid one-based list option",
                ));
            }
            if row.metadata.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "var"
                        | "type"
                        | "label"
                        | "list"
                        | "defaultState"
                        | "defaultPlaceholderState"
                        | "defaultIndex"
                )
            }) {
                return Err(error("definition metadata repeats structural fields"));
            }
            validate_metadata_map(&row.metadata, &self.source, 0, &mut metadata_count)?;
        }
        Ok(())
    }
}
impl ConfigSourceIdentity {
    fn validate(&self) -> Result<()> {
        if !hex(&self.upstream_revision, 40) || self.files.is_empty() || self.files.len() > 128 {
            return Err(error("invalid source identity"));
        }
        for (path, digest) in &self.files {
            source_path(path)?;
            if !hex(digest, 64) {
                return Err(error("invalid source file SHA-256"));
            }
        }
        validate_span(&self.create_config_set, self)?;
        validate_span(&self.get_default_state, self)
    }
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn source_path(path: &str) -> Result<()> {
    identifier(path, "source path", 512)?;
    if path.contains(['\\', ':'])
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(error("source paths must be relative normalized paths"));
    }
    Ok(())
}
fn validate_location(location: &ConfigSourceLocation, source: &ConfigSourceIdentity) -> Result<()> {
    source_path(&location.path)?;
    if location.line == 0 || location.line > 1_000_000 || !source.files.contains_key(&location.path)
    {
        return Err(error(
            "source location is outside declared files/line domain",
        ));
    }
    Ok(())
}
fn validate_span(span: &ConfigSourceSpan, source: &ConfigSourceIdentity) -> Result<()> {
    validate_location(
        &ConfigSourceLocation {
            path: span.path.clone(),
            line: span.line,
        },
        source,
    )?;
    if span.end_line < span.line || span.end_line > 1_000_000 || !hex(&span.sha256, 64) {
        return Err(error("invalid callback/function source span"));
    }
    Ok(())
}
fn identifier(value: &str, name: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(error(format!("invalid {name}")));
    }
    Ok(())
}
fn text(value: &str) -> Result<()> {
    if value.len() > MAX_STRING_BYTES || value.contains('\0') {
        return Err(error("text exceeds bounded catalog domain"));
    }
    Ok(())
}
fn scalar(value: &Scalar) -> Result<()> {
    match value {
        Scalar::Number(value) if !value.is_finite() => Err(error("scalar number must be finite")),
        Scalar::Text(value) => text(value),
        _ => Ok(()),
    }
}
fn validate_metadata_map(
    map: &BTreeMap<String, ConfigMetadataValue>,
    source: &ConfigSourceIdentity,
    depth: usize,
    count: &mut usize,
) -> Result<()> {
    for (key, value) in map {
        identifier(key, "metadata key", 256)?;
        validate_metadata(value, source, depth + 1, count)?;
    }
    Ok(())
}
fn validate_metadata(
    value: &ConfigMetadataValue,
    source: &ConfigSourceIdentity,
    depth: usize,
    count: &mut usize,
) -> Result<()> {
    *count += 1;
    if depth > MAX_METADATA_DEPTH || *count > MAX_METADATA_VALUES {
        return Err(error("metadata exceeds bounded catalog domain"));
    }
    match value {
        ConfigMetadataValue::Boolean(_) => Ok(()),
        ConfigMetadataValue::Number(value) => scalar(&Scalar::Number(*value)),
        ConfigMetadataValue::Text(value) => text(value),
        ConfigMetadataValue::Array(values) => {
            for value in values {
                validate_metadata(value, source, depth + 1, count)?;
            }
            Ok(())
        }
        ConfigMetadataValue::Object(map) => validate_metadata_map(map, source, depth, count),
        ConfigMetadataValue::Callback(span) => validate_span(span, source),
    }
}
fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}
