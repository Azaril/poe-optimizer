//! Loader-local authored configuration prefix, before activation callbacks.
//! This is not ConfigTab.Load completion, root setup, or effective configuration.
use crate::CompiledGameData;
use poe_optimizer_core::{
    data::DataIdentity,
    evaluation::{EvaluationError, EvaluationErrorKind},
    options::Scalar,
};
use poe_optimizer_engine::{lua_number::parse_number, selection_keys::NumericSetKeys};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, SourceOccurrenceId},
    selected_view::{NumericValue, SelectedView, SelectionDomain, SetOrigin},
    source_xml::{self, PobContentEntry},
};
use roxmltree::Node;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Copy)]
pub struct ConfigurationPreparationLimits {
    pub max_sets: usize,
    pub max_records: usize,
    pub max_fields: usize,
    pub max_text_bytes: usize,
    pub max_fragments: usize,
    pub max_pattern_steps: u64,
}
impl Default for ConfigurationPreparationLimits {
    fn default() -> Self {
        Self {
            max_sets: 4096,
            max_records: 32768,
            max_fields: 500000,
            max_text_bytes: 8 * 1024 * 1024,
            max_fragments: 131072,
            max_pattern_steps: 100_000_000,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConfigurationValue {
    Boolean(bool),
    Number(NumericValue),
    Text(String),
}
impl ConfigurationValue {
    pub fn number(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(n.value())
        } else {
            None
        }
    }
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    fn truthy(&self) -> bool {
        !matches!(self, Self::Boolean(false))
    }
    fn empty_text(&self) -> bool {
        matches!(self, Self::Text(s) if s.is_empty())
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigurationBlockText {
    Value {
        value: ConfigurationValue,
    },
    /// Original node[1] is a table, retained until an actual text consumer runs.
    Element {
        source: SourceOccurrenceId,
    },
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigurationBlockOrigin {
    Default,
    Authored { source: SourceOccurrenceId },
    LegacyInput { source: Option<SourceOccurrenceId> },
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedConfigurationBlock {
    pub title: String,
    pub enabled: bool,
    pub text: ConfigurationBlockText,
    pub origin: ConfigurationBlockOrigin,
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedConfigSet {
    pub key: NumericValue,
    pub origin: SetOrigin,
    pub title: Option<String>,
    pub inputs: BTreeMap<String, ConfigurationValue>,
    pub placeholders: BTreeMap<String, ConfigurationValue>,
    pub blocks: Vec<PreparedConfigurationBlock>,
    pub winner: bool,
    pub migration_passes: usize,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationWriteTarget {
    Input,
    Placeholder,
}
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationWrite {
    pub set_index: usize,
    pub target: ConfigurationWriteTarget,
    pub key: String,
    pub value: Option<ConfigurationValue>,
    pub source: Option<SourceOccurrenceId>,
    pub migrated: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationPrefixStatus {
    Prepared,
    SourceFailure,
    Unsupported,
}
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationDiagnostic {
    pub code: &'static str,
    pub source: SourceOccurrenceId,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationFailure {
    pub stage: &'static str,
    pub source: Option<SourceOccurrenceId>,
    pub message: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationContinuationStage {
    UpdateControls,
    InitialBuildModList,
}
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationContinuation {
    pub stage: ConfigurationContinuationStage,
    pub container: Option<SourceOccurrenceId>,
    pub unexecuted_containers: Vec<SourceOccurrenceId>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationPreparationReport {
    pub schema_version: u32,
    pub scope: &'static str,
    pub source_sha256: String,
    pub data: DataIdentity,
    pub view_sha256: String,
    pub status: ConfigurationPrefixStatus,
    pub default_state: BTreeMap<String, ConfigurationValue>,
    pub sets: Vec<PreparedConfigSet>,
    pub order: Vec<Option<NumericValue>>,
    pub active_set: Option<SetOrigin>,
    /// View choice may point to an unexecuted later section; it is not activation.
    pub view_selected_set: Option<SetOrigin>,
    pub writes: Vec<ConfigurationWrite>,
    pub diagnostics: Vec<ConfigurationDiagnostic>,
    pub failure: Option<ConfigurationFailure>,
    pub continuation: Option<ConfigurationContinuation>,
    pub frontiers: Vec<&'static str>,
}
pub struct PreparedConfiguration {
    owner: ImportedBuildInstance,
    data: Arc<CompiledGameData>,
    report: ConfigurationPreparationReport,
}
impl PreparedConfiguration {
    pub fn report(&self) -> &ConfigurationPreparationReport {
        &self.report
    }
    pub fn validate_binding(
        &self,
        build: &ImportedBuildInstance,
        view: &SelectedView<'_>,
        data: &Arc<CompiledGameData>,
    ) -> Result<(), EvaluationError> {
        view.validate_binding(build, data.snapshot())
            .map_err(|e| contract(e.to_string()))?;
        if !self.owner.shares_storage_with(build)
            || !Arc::ptr_eq(&self.data, data)
            || self.report.view_sha256 != view_digest(view)?
        {
            return Err(contract(
                "configuration prefix belongs to another build, view or compiled definition owner",
            ));
        }
        Ok(())
    }
}
fn contract(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::BackendContract, message)
}
fn resource(name: &'static str) -> EvaluationError {
    EvaluationError::new(
        EvaluationErrorKind::InvalidRequest,
        format!("authored configuration preparation exceeds {name} limit"),
    )
}
fn view_digest(view: &SelectedView<'_>) -> Result<String, EvaluationError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(view.report()).map_err(|e| contract(e.to_string()))?)
    ))
}
fn number_key(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
fn scalar(value: Scalar) -> ConfigurationValue {
    match value {
        Scalar::Boolean(v) => ConfigurationValue::Boolean(v),
        Scalar::Number(v) => ConfigurationValue::Number(NumericValue::new(v)),
        Scalar::Text(v) => ConfigurationValue::Text(v),
    }
}

struct Machine<'a> {
    build: &'a ImportedBuildInstance,
    data: &'a Arc<CompiledGameData>,
    report: ConfigurationPreparationReport,
    limits: ConfigurationPreparationLimits,
    sources: BTreeMap<usize, SourceOccurrenceId>,
    instances: BTreeMap<SourceOccurrenceId, AuthoredInstanceId>,
    winners: BTreeMap<u64, usize>,
    keys: NumericSetKeys,
    pattern: poe_optimizer_engine::lua_pattern::MatchBudget,
    input_origins: BTreeMap<(usize, String), SourceOccurrenceId>,
    container: Option<SourceOccurrenceId>,
}
impl Machine<'_> {
    fn spend(left: &mut usize, amount: usize, name: &'static str) -> Result<(), EvaluationError> {
        *left = left.checked_sub(amount).ok_or_else(|| resource(name))?;
        Ok(())
    }
    fn text_cost(&mut self, text: &str) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_text_bytes, text.len(), "text bytes")
    }
    fn value_cost(&mut self, value: &ConfigurationValue) -> Result<(), EvaluationError> {
        if let ConfigurationValue::Text(text) = value {
            self.text_cost(text)?;
        }
        Ok(())
    }
    fn source(&self, node: Node<'_, '_>) -> Result<SourceOccurrenceId, EvaluationError> {
        self.sources
            .get(&node.range().start)
            .copied()
            .ok_or_else(|| contract("unmapped configuration source occurrence"))
    }
    fn attr(&mut self, node: Node<'_, '_>, key: &str) -> Result<Option<String>, EvaluationError> {
        let value = self
            .build
            .attribute(self.source(node)?, key)
            .map_err(|e| contract(e.to_string()))?;
        value
            .map(|v| {
                self.text_cost(v.decoded())?;
                Ok(v.decoded().to_owned())
            })
            .transpose()
    }
    fn children<'a, 'input>(
        &mut self,
        node: Node<'a, 'input>,
    ) -> Result<Vec<Option<Node<'a, 'input>>>, EvaluationError> {
        let content = source_xml::ordered_content(
            node,
            &mut self.limits.max_fragments,
            &mut self.limits.max_text_bytes,
        )
        .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e.to_string()))?;
        let elements: Vec<_> = node.children().filter(Node::is_element).collect();
        Ok(content
            .consumed()
            .iter()
            .map(|entry| match entry {
                PobContentEntry::Element { child_index } => Some(elements[*child_index]),
                PobContentEntry::Text { .. } => None,
            })
            .collect())
    }
    fn fail(
        &mut self,
        status: ConfigurationPrefixStatus,
        stage: &'static str,
        source: Option<SourceOccurrenceId>,
        message: impl Into<String>,
    ) -> Result<(), EvaluationError> {
        let message = message.into();
        self.text_cost(&message)?;
        self.report.status = status;
        self.report.failure = Some(ConfigurationFailure {
            stage,
            source,
            message,
        });
        self.report.continuation = None;
        Ok(())
    }
    fn known(&mut self, node: Node<'_, '_>) -> Result<bool, EvaluationError> {
        let source = self.source(node)?;
        if self
            .build
            .occurrence(source)
            .map_err(|e| contract(e.to_string()))?
            .has_namespace_context()
        {
            self.fail(
                ConfigurationPrefixStatus::Unsupported,
                "source_namespace",
                Some(source),
                "namespaced configuration has no admitted loader interpretation",
            )?;
            Ok(false)
        } else {
            Ok(true)
        }
    }
    fn diagnostic(
        &mut self,
        node: Node<'_, '_>,
        code: &'static str,
        message: &str,
    ) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_records, 1, "diagnostics")?;
        self.text_cost(message)?;
        self.report.diagnostics.push(ConfigurationDiagnostic {
            code,
            source: self.source(node)?,
            message: message.into(),
        });
        Ok(())
    }
    fn default_block(&mut self) -> Result<PreparedConfigurationBlock, EvaluationError> {
        let title = self
            .data
            .snapshot()
            .configuration()
            .data()
            .authored_load
            .default_custom_block_title
            .clone();
        self.text_cost(&title)?;
        Ok(PreparedConfigurationBlock {
            title,
            enabled: true,
            text: ConfigurationBlockText::Value {
                value: ConfigurationValue::Text(String::new()),
            },
            origin: ConfigurationBlockOrigin::Default,
        })
    }
    fn create(
        &mut self,
        key: Option<f64>,
        origin: SetOrigin,
        title: Option<String>,
    ) -> Result<Option<usize>, EvaluationError> {
        Self::spend(&mut self.limits.max_sets, 1, "sets")?;
        let key = key.unwrap_or_else(|| self.keys.sequence_length() as f64 + 1.0);
        if key.is_nan() {
            self.fail(
                ConfigurationPrefixStatus::SourceFailure,
                "create_config_set",
                origin.source(),
                "table index is NaN",
            )?;
            return Ok(None);
        }
        self.keys
            .insert(key)
            .map_err(|_| resource("numeric set keys"))?;
        // Charge every source default assignment before initial_state clones any
        // definition-owned key or value. This includes overwritten assignments
        // and applies cumulatively to every created set, including old winners.
        for definition in self.data.snapshot().configuration().definitions() {
            let option = definition
                .defaults
                .option_index
                .map(|index| &definition.options[index as usize - 1].value);
            for value in [
                definition.defaults.input.as_ref(),
                definition.defaults.placeholder.as_ref(),
                option,
            ]
            .into_iter()
            .flatten()
            {
                Self::spend(&mut self.limits.max_fields, 1, "default fields")?;
                Self::spend(
                    &mut self.limits.max_text_bytes,
                    definition.key.len(),
                    "text bytes",
                )?;
                if let Scalar::Text(text) = value {
                    Self::spend(&mut self.limits.max_text_bytes, text.len(), "text bytes")?;
                }
            }
        }
        let defaults = self.data.snapshot().configuration().initial_state();
        let inputs: BTreeMap<_, _> = defaults
            .inputs
            .into_iter()
            .map(|(k, v)| (k, scalar(v)))
            .collect();
        let placeholders: BTreeMap<_, _> = defaults
            .placeholders
            .into_iter()
            .map(|(k, v)| (k, scalar(v)))
            .collect();
        if let Some(title) = &title {
            self.text_cost(title)?;
        }
        let block = self.default_block()?;
        let index = self.report.sets.len();
        if let Some(old) = self.winners.insert(number_key(key), index) {
            self.report.sets[old].winner = false;
        }
        self.report.sets.push(PreparedConfigSet {
            key: NumericValue::new(key),
            origin,
            title,
            inputs,
            placeholders,
            blocks: vec![block],
            winner: true,
            migration_passes: 0,
        });
        Ok(Some(index))
    }
    fn default_origin(&self) -> SetOrigin {
        SetOrigin::Default {
            domain: SelectionDomain::Configuration,
            container: self.container,
        }
    }
    fn write(
        &mut self,
        set_index: usize,
        target: ConfigurationWriteTarget,
        key: String,
        value: Option<ConfigurationValue>,
        source: Option<SourceOccurrenceId>,
        migrated: bool,
    ) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_records, 1, "writes")?;
        Self::spend(&mut self.limits.max_fields, 1, "written fields")?;
        self.text_cost(&key)?;
        self.text_cost(&key)?;
        self.text_cost(&key)?;
        if let Some(value) = &value {
            self.value_cost(value)?;
            self.value_cost(value)?;
        }
        let map = match target {
            ConfigurationWriteTarget::Input => &mut self.report.sets[set_index].inputs,
            ConfigurationWriteTarget::Placeholder => &mut self.report.sets[set_index].placeholders,
        };
        if let Some(value) = &value {
            map.insert(key.clone(), value.clone());
        } else {
            map.remove(&key);
        }
        if matches!(target, ConfigurationWriteTarget::Input) {
            if let Some(source) = source {
                self.input_origins.insert((set_index, key.clone()), source);
            } else {
                self.input_origins.remove(&(set_index, key.clone()));
            }
        }
        self.report.writes.push(ConfigurationWrite {
            set_index,
            target,
            key,
            value,
            source,
            migrated,
        });
        Ok(())
    }
    fn rewrite(
        &mut self,
        node: Node<'_, '_>,
        key: &str,
        mut value: String,
    ) -> Result<Option<(String, bool)>, EvaluationError> {
        use poe_optimizer_data::configuration::ConfigStringRewrite;
        use poe_optimizer_engine::lua_pattern::{GsubLimits, LuaPattern, PatternError};
        let data = self.data;
        let mut migrated = false;
        for rewrite in &data
            .snapshot()
            .configuration()
            .data()
            .authored_load
            .input_string_rewrites
        {
            if rewrite.key != key {
                continue;
            }
            for operation in &rewrite.operations {
                value = match operation {
                    ConfigStringRewrite::AsciiLower => value.to_ascii_lowercase(),
                    ConfigStringRewrite::AsciiTitleWords => {
                        let mut bytes = value.into_bytes();
                        let mut i = 0;
                        while i < bytes.len() {
                            if bytes[i].is_ascii_lowercase() {
                                bytes[i] = bytes[i].to_ascii_uppercase();
                                i += 1;
                                while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                                    i += 1;
                                }
                            } else {
                                i += 1;
                            }
                        }
                        String::from_utf8(bytes)
                            .map_err(|_| contract("ASCII transform invalidated UTF-8"))?
                    }
                    ConfigStringRewrite::LuaGsub {
                        pattern,
                        replacement,
                    } => {
                        let result = LuaPattern::compile(pattern.as_bytes()).and_then(|pattern| {
                            pattern.gsub(
                                value.as_bytes(),
                                replacement.as_bytes(),
                                None,
                                &mut self.pattern,
                                GsubLimits {
                                    max_output_bytes: self.limits.max_text_bytes,
                                    ..GsubLimits::default()
                                },
                            )
                        });
                        let bytes = match result {
                            Ok(result) => result.bytes,
                            Err(PatternError::Source(error)) => {
                                self.fail(
                                    ConfigurationPrefixStatus::SourceFailure,
                                    "input_string_rewrite",
                                    Some(self.source(node)?),
                                    error.message(),
                                )?;
                                return Ok(None);
                            }
                            Err(_) => return Err(resource("string rewrite work/output")),
                        };
                        match String::from_utf8(bytes) {
                            Ok(value) => value,
                            Err(_) => {
                                self.fail(
                                    ConfigurationPrefixStatus::Unsupported,
                                    "input_string_rewrite",
                                    Some(self.source(node)?),
                                    "rewrite produced non-UTF-8 Lua bytes",
                                )?;
                                return Ok(None);
                            }
                        }
                    }
                };
                self.text_cost(&value)?;
                migrated = true;
            }
        }
        Ok(Some((value, migrated)))
    }
    fn record(&mut self, node: Node<'_, '_>, set_index: usize) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_records, 1, "source records")?;
        if !self.known(node)? {
            return Ok(());
        }
        let tag = node.tag_name().name();
        if tag == "CustomModifierBlock" {
            let title = self.attr(node, "title")?.unwrap_or_else(|| {
                self.data
                    .snapshot()
                    .configuration()
                    .data()
                    .authored_load
                    .default_custom_block_title
                    .clone()
            });
            let enabled = self.attr(node, "enabled")?.is_none_or(|v| v == "true");
            let content = source_xml::ordered_content(
                node,
                &mut self.limits.max_fragments,
                &mut self.limits.max_text_bytes,
            )
            .map_err(|e| {
                EvaluationError::new(EvaluationErrorKind::InvalidRequest, e.to_string())
            })?;
            let text = match content.consumed().first() {
                Some(PobContentEntry::Text { text, .. }) => ConfigurationBlockText::Value {
                    value: ConfigurationValue::Text(text.clone()),
                },
                Some(PobContentEntry::Element { child_index }) => ConfigurationBlockText::Element {
                    source: self.source(
                        node.children()
                            .filter(Node::is_element)
                            .nth(*child_index)
                            .ok_or_else(|| contract("missing block child"))?,
                    )?,
                },
                None => ConfigurationBlockText::Value {
                    value: ConfigurationValue::Text(String::new()),
                },
            };
            self.text_cost(&title)?;
            if let ConfigurationBlockText::Value { value } = &text {
                self.value_cost(value)?;
            }
            Self::spend(&mut self.limits.max_fields, 3, "block fields")?;
            let source = self.source(node)?;
            self.report.sets[set_index]
                .blocks
                .push(PreparedConfigurationBlock {
                    title,
                    enabled,
                    text,
                    origin: ConfigurationBlockOrigin::Authored { source },
                });
            return Ok(());
        }
        if !matches!(tag, "Input" | "Placeholder") {
            return Ok(());
        }
        let Some(key) = self.attr(node, "name")? else {
            return self.diagnostic(node,"missing_name","Input/Placeholder missing name attribute; original Load ignores the helper's error return");
        };
        let mut target = if tag == "Input" {
            ConfigurationWriteTarget::Input
        } else {
            ConfigurationWriteTarget::Placeholder
        };
        let mut migrated = false;
        let value = if let Some(raw) = self.attr(node, "number")? {
            parse_number(raw.as_bytes()).map(|n| ConfigurationValue::Number(NumericValue::new(n)))
        } else if let Some(raw) = self.attr(node, "string")? {
            target = ConfigurationWriteTarget::Input;
            let text = if tag == "Input" {
                let Some((value, changed)) = self.rewrite(node, &key, raw)? else {
                    return Ok(());
                };
                migrated = changed;
                value
            } else {
                raw
            };
            Some(ConfigurationValue::Text(text))
        } else if tag == "Input" {
            if let Some(raw) = self.attr(node, "boolean")? {
                Some(ConfigurationValue::Boolean(raw == "true"))
            } else {
                return self.diagnostic(node,"missing_value","Input missing number, string or boolean attribute; original Load ignores the helper's error return");
            }
        } else {
            return self.diagnostic(node,"missing_value","Placeholder missing number or string attribute; original Load ignores the helper's error return");
        };
        self.write(
            set_index,
            target,
            key,
            value,
            Some(self.source(node)?),
            migrated,
        )
    }
    fn migrate(&mut self) -> Result<(), EvaluationError> {
        let custom_key = self
            .data
            .snapshot()
            .configuration()
            .data()
            .authored_load
            .legacy_custom_mods_key
            .clone();
        let order: Vec<_> = self.report.order.iter().map_while(|key| *key).collect();
        for key in order {
            let Some(&index) = self.winners.get(&number_key(key.value())) else {
                self.fail(
                    ConfigurationPrefixStatus::SourceFailure,
                    "migrate_custom_modifiers",
                    self.container,
                    "migration dereferences a missing ordered config set",
                )?;
                return Ok(());
            };
            self.report.sets[index].migration_passes += 1;
            let set = &self.report.sets[index];
            let legacy = set
                .inputs
                .get(&custom_key)
                .filter(|value| value.truthy())
                .cloned()
                .unwrap_or_else(|| ConfigurationValue::Text(String::new()));
            let empty = set.blocks.is_empty();
            let single_empty = set.blocks.len() == 1
                && matches!(&set.blocks[0].text,ConfigurationBlockText::Value{value}if value.empty_text());
            if !legacy.empty_text() && (empty || single_empty) {
                let mut block = self.default_block()?;
                self.value_cost(&legacy)?;
                block.text = ConfigurationBlockText::Value { value: legacy };
                block.origin = ConfigurationBlockOrigin::LegacyInput {
                    source: self
                        .input_origins
                        .get(&(index, custom_key.clone()))
                        .copied(),
                };
                self.report.sets[index].blocks = vec![block];
            } else if empty {
                self.report.sets[index].blocks = vec![self.default_block()?];
            }
            self.write(
                index,
                ConfigurationWriteTarget::Input,
                custom_key.clone(),
                None,
                None,
                true,
            )?;
        }
        Ok(())
    }
    fn load(&mut self, node: Node<'_, '_>) -> Result<(), EvaluationError> {
        self.container = Some(self.source(node)?);
        if !self.known(node)? {
            return Ok(());
        }
        let children = self.children(node)?;
        if children.is_empty() {
            let title = self
                .data
                .snapshot()
                .configuration()
                .data()
                .authored_load
                .default_set_title
                .clone();
            self.create(Some(1.0), self.default_origin(), Some(title))?;
        }
        for (position, child) in children.into_iter().enumerate() {
            if let Some(child) = child {
                if !self.known(child)? {
                    return Ok(());
                }
                if child.tag_name().name() == "ConfigSet" {
                    let source = self.source(child)?;
                    let instance = *self
                        .instances
                        .get(&source)
                        .ok_or_else(|| contract("missing authored config-set instance"))?;
                    if !matches!(instance, AuthoredInstanceId::ConfigSet(_)) {
                        return Err(contract("wrong config-set instance role"));
                    }
                    let raw = self
                        .attr(child, "id")?
                        .as_deref()
                        .and_then(|value| parse_number(value.as_bytes()));
                    let title = self.attr(child, "title")?.unwrap_or_else(|| {
                        self.data
                            .snapshot()
                            .configuration()
                            .data()
                            .authored_load
                            .default_set_title
                            .clone()
                    });
                    let Some(index) =
                        self.create(raw, SetOrigin::Authored { instance, source }, Some(title))?
                    else {
                        return Ok(());
                    };
                    self.report
                        .order
                        .resize(self.report.order.len().max(position + 1), None);
                    self.report.order[position] = raw.map(NumericValue::new);
                    if raw.is_none() {
                        self.fail(ConfigurationPrefixStatus::SourceFailure,"load_config_set",Some(source),"CreateConfigSet generates a key, then Load dereferences the original nil key")?;
                        return Ok(());
                    }
                    self.report.sets[index].blocks.clear();
                    for entry in self.children(child)?.into_iter().flatten() {
                        self.record(entry, index)?;
                        if self.report.failure.is_some() {
                            return Ok(());
                        }
                    }
                    continue;
                }
            }
            let index = if let Some(&index) = self.winners.get(&number_key(1.0)) {
                index
            } else {
                let title = self
                    .data
                    .snapshot()
                    .configuration()
                    .data()
                    .authored_load
                    .default_set_title
                    .clone();
                self.create(Some(1.0), self.default_origin(), Some(title))?
                    .ok_or_else(|| contract("default config set failed"))?
            };
            if let Some(child) = child {
                self.record(child, index)?;
                if self.report.failure.is_some() {
                    return Ok(());
                }
            }
        }
        self.migrate()?;
        if self.report.failure.is_some() {
            return Ok(());
        }
        let requested = self
            .attr(node, "activeConfigSet")?
            .as_deref()
            .and_then(|value| parse_number(value.as_bytes()))
            .unwrap_or(1.0);
        let chosen = self
            .winners
            .get(&number_key(requested))
            .copied()
            .or_else(|| {
                self.report
                    .order
                    .first()
                    .and_then(|key| *key)
                    .and_then(|key| self.winners.get(&number_key(key.value())).copied())
            });
        let Some(index) = chosen else {
            self.fail(
                ConfigurationPrefixStatus::SourceFailure,
                "set_active_config_set",
                self.container,
                "SetActiveConfigSet dereferences a missing config set",
            )?;
            return Ok(());
        };
        self.report.active_set = Some(self.report.sets[index].origin);
        self.report
            .continuation
            .as_mut()
            .ok_or_else(|| contract("missing configuration continuation"))?
            .stage = ConfigurationContinuationStage::UpdateControls;
        Ok(())
    }
}

pub fn prepare_authored_configuration(
    build: &ImportedBuildInstance,
    view: &SelectedView<'_>,
    data: &Arc<CompiledGameData>,
    limits: ConfigurationPreparationLimits,
) -> Result<PreparedConfiguration, EvaluationError> {
    use poe_optimizer_data::configuration::ConfigWidgetKind;
    use poe_optimizer_engine::lua_pattern::{MatchBudget, MatchLimits};
    view.validate_binding(build, data.snapshot())
        .map_err(|e| contract(e.to_string()))?;
    let document = roxmltree::Document::parse_with_options(
        build.source_xml(),
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: poe_optimizer_import::MAX_XML_NODES,
            ..Default::default()
        },
    )
    .map_err(|e| contract(e.to_string()))?;
    let mut machine = Machine {
        build,
        data,
        limits,
        sources: build
            .occurrences()
            .iter()
            .map(|s| (s.range().start, s.id()))
            .collect(),
        instances: build
            .instances()
            .iter()
            .map(|s| (s.source(), s.instance()))
            .collect(),
        winners: BTreeMap::new(),
        keys: NumericSetKeys::new(limits.max_sets),
        pattern: MatchBudget::new(MatchLimits {
            max_steps: limits.max_pattern_steps,
            ..Default::default()
        }),
        input_origins: BTreeMap::new(),
        container: None,
        report: ConfigurationPreparationReport {
            schema_version: 1,
            scope: "authored_configuration_load_prefix",
            source_sha256: build.source_sha256().into(),
            data: data.identity().clone(),
            view_sha256: view_digest(view)?,
            status: ConfigurationPrefixStatus::Prepared,
            default_state: BTreeMap::new(),
            sets: vec![],
            order: vec![Some(NumericValue::new(1.0))],
            active_set: None,
            view_selected_set: view
                .report()
                .configuration
                .selected
                .as_ref()
                .map(|set| set.origin),
            writes: vec![],
            diagnostics: vec![],
            failure: None,
            continuation: None,
            frontiers: vec![
                "loader-local prefix assumes definition-backed constructor defaults; earlier root initialization callbacks, controls, mod lists and enemyLevel are not reconstructed",
                "UpdateControls, BuildModList/UpdateLevel, mutable apply callbacks, custom modifier parsing, loadout synchronization and effective configuration remain unexecuted",
                "requested view activation is separate from the saved activation prefix and has not been executed",
            ],
        },
    };
    for definition in data.snapshot().configuration().definitions() {
        // UI fallback and CreateConfigSet defaults are distinct source states.
        // Keep this choice borrowed until its budget has been reserved.
        let false_value = Scalar::Boolean(false);
        let zero_value = Scalar::Number(0.0);
        let empty_value = Scalar::Text(String::new());
        let declared = definition
            .defaults
            .input
            .as_ref()
            .filter(|value| !matches!(value, Scalar::Boolean(false)));
        let value = match definition.widget {
            ConfigWidgetKind::List => {
                &definition.options[(definition.defaults.option_index.unwrap_or(1) - 1) as usize]
                    .value
            }
            ConfigWidgetKind::Check => declared.unwrap_or(&false_value),
            ConfigWidgetKind::Count
            | ConfigWidgetKind::CountAllowZero
            | ConfigWidgetKind::Integer
            | ConfigWidgetKind::Float => declared.unwrap_or(&zero_value),
            ConfigWidgetKind::Text => declared.unwrap_or(&empty_value),
        };
        Machine::spend(&mut machine.limits.max_fields, 1, "UI default fields")?;
        machine.text_cost(&definition.key)?;
        if let Scalar::Text(text) = value {
            machine.text_cost(text)?;
        }
        machine
            .report
            .default_state
            .insert(definition.key.clone(), scalar(value.clone()));
    }
    let containers: Vec<_> = machine
        .children(document.root_element())?
        .into_iter()
        .flatten()
        .filter(|node| node.tag_name().name() == "Config")
        .collect();
    let unexecuted_containers = containers
        .iter()
        .skip(1)
        .map(|node| machine.source(*node))
        .collect::<Result<Vec<_>, _>>()?;
    machine.report.continuation = Some(ConfigurationContinuation {
        stage: ConfigurationContinuationStage::InitialBuildModList,
        container: containers
            .first()
            .map(|node| machine.source(*node))
            .transpose()?,
        unexecuted_containers,
    });
    if let Some(node) = containers.first() {
        machine.load(*node)?;
    } else {
        let index = machine
            .create(Some(1.0), machine.default_origin(), None)?
            .ok_or_else(|| contract("constructor config set failed"))?;
        machine.report.active_set = Some(machine.report.sets[index].origin);
    }
    Ok(PreparedConfiguration {
        owner: build.clone(),
        data: Arc::clone(data),
        report: machine.report,
    })
}
