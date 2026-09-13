#[path = "runes.rs"]
mod runes;
use super::affixes::{AffixError, AffixPrograms};
use super::{ItemNumber, LineSelection, VariantState, syntax};
use poe_optimizer_data::item_loading::{
    ItemAffixLookup, ItemAffixSide, ItemLoadingCatalog, ItemMetadataTable, ItemMetadataValue,
};
use poe_optimizer_data::item_scalability::CatalystScalingData;
use poe_optimizer_engine::lua_pattern::{GsubLimits, LuaPattern, MatchBudget, PatternError};
use runes::RunePrograms;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub const MAX_ITEM_LOADING_TEXT: usize = 1024 * 1024;
pub const MAX_ITEM_LOADING_LINES: usize = 8192;
pub const MAX_ITEM_LOADING_CALLS: usize = 32768;
pub const MAX_ITEM_LOADING_DEPENDENCY_MESSAGE: usize = 4096;
pub const MAX_ITEM_LOADING_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
#[derive(Debug, thiserror::Error)]
#[error("native item loading: {0}")]
pub struct ItemLoadError(pub(crate) String);
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ItemScalar {
    Boolean(bool),
    Number(ItemNumber),
    Text(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemLoadStatus {
    Complete,
    NoBase,
    Pending,
    SourceError,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    ModifierParser,
    RangeFormatting,
    AdvancedCopyAffixes,
    RuneReconstruction,
    UniqueDatabase,
    ModifierMagnitudes,
    CraftedAffixes,
    BaseBuffs,
    BaseLookupAmbiguity,
    BaseCompatibility,
    ClusterJewel,
    Assembly,
    NumericIndex,
    UnsupportedHeader,
}
#[derive(Debug, Clone, Serialize)]
pub struct PendingDependency {
    pub kind: DependencyKind,
    pub line_index: Option<usize>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum DependencyResult<T> {
    Available(T),
    Unavailable(String),
    SourceError(String),
    ResourceError(String),
}
#[derive(Debug, Clone, Serialize)]
pub struct ParseRequest {
    pub sequence: usize,
    pub line_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<RuneContribution>,
    pub text: String,
    pub combined: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct ParseOutcome {
    pub modifiers: Option<Vec<ItemMetadataTable>>,
    pub extra: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct FormatRequest {
    pub sequence: usize,
    /// None for generated rows without an authored line index.
    pub line_index: Option<usize>,
    pub text: String,
    pub range: ItemNumber,
    pub scalar: ItemNumber,
    pub corrupted_range: ItemNumber,
}
/// A parser request issued by applyRange before the loader's modifier parse.
#[derive(Debug, Clone, Serialize)]
pub struct FormatParserCall {
    pub request: ParseRequest,
    pub result: DependencyResult<ParseOutcome>,
}
#[derive(Debug, Clone)]
pub struct FormatOutcome {
    pub result: DependencyResult<String>,
    pub precision_parser_calls: Vec<FormatParserCall>,
}
#[derive(Debug, Clone, Serialize)]
pub struct UniqueRequest {
    pub name: String,
    pub title: Option<String>,
    pub base_name: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct UniqueOutcome {
    /// None and ItemNumber::Nil both represent Lua absence; numeric zero is present.
    pub natural_level: Option<ItemNumber>,
    pub level: Option<ItemNumber>,
}
/// Opaque lifetime/attempt authority. Equal serialized item content cannot mint it.
#[derive(Debug, Clone)]
pub struct AssemblyBinding {
    item: Arc<()>,
    attempt: Arc<()>,
    catalog: ItemLoadingCatalog,
}
impl AssemblyBinding {
    pub fn same_item(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.item, &other.item) && self.catalog.shares_storage_with(&other.catalog)
    }
    pub fn matches_attempt(&self, other: &Self) -> bool {
        self.same_item(other) && Arc::ptr_eq(&self.attempt, &other.attempt)
    }
    pub fn matches_definitions(&self, catalog: &ItemLoadingCatalog) -> bool {
        self.catalog.shares_storage_with(catalog)
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct AssemblyRequest {
    #[serde(skip)]
    binding: AssemblyBinding,
    pub final_load: bool,
    /// ParseRaw replaced line/requirement/socket tables since the previous assembly.
    pub reparsed: bool,
    /// Private native history, never reconstructed from serialized diagnostics.
    #[serde(skip)]
    pub previous: Option<super::assembly::AssembledItem>,
    pub state: ItemState,
    /// Actual ParseRaw writes since the last assembly; absence is not a deletion.
    #[serde(skip)]
    armour_header_updates: BTreeMap<String, ItemNumber>,
}
impl AssemblyRequest {
    pub fn binding(&self) -> &AssemblyBinding {
        &self.binding
    }
    /// Only authored defence-header writes, including explicit nil deletion.
    /// Re-entry never reconstructs these from a diagnostic numeric projection.
    pub fn armour_header_updates(&self) -> &BTreeMap<String, ItemNumber> {
        &self.armour_header_updates
    }
}
/// Post-assembly modifier payloads, retaining source row counts and order.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AssemblyModifierPayloads {
    pub buff_mod_lines: Vec<Vec<ItemMetadataTable>>,
    pub enchant_mod_lines: Vec<Vec<ItemMetadataTable>>,
    pub rune_mod_lines: Vec<Vec<ItemMetadataTable>>,
    pub class_requirement_mod_lines: Vec<Vec<ItemMetadataTable>>,
    pub implicit_mod_lines: Vec<Vec<ItemMetadataTable>>,
    pub explicit_mod_lines: Vec<Vec<ItemMetadataTable>>,
}
/// Explicit nested-state update; absence, an empty table and no update differ.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ArmourDataUpdate {
    #[default]
    Preserve,
    Clear,
    Replace(BTreeMap<String, ItemNumber>),
    /// Diagnostic numeric named entries only; a matching owned graph is required.
    /// Non-numeric and indexed values remain in that graph.
    NumericSubset(BTreeMap<String, ItemNumber>),
}
#[derive(Debug, Clone, Serialize)]
pub struct AssemblyOutcome {
    /// An owned producer result, separate from diagnostic evidence. Legacy test
    /// providers may omit it; production registration requires a complete value.
    #[serde(skip)]
    pub assembled: Option<super::assembly::AssembledItem>,
    pub armour_data: ArmourDataUpdate,
    pub modifier_payloads: Option<AssemblyModifierPayloads>,
    /// Exact post-assembly requirement table supplied by the dependency. None
    /// leaves it untouched; Some replaces it, including removal of old keys.
    pub requirements: Option<BTreeMap<String, ItemNumber>>,
    /// Number(Nil) removes a field, matching assignment of nil in Lua.
    pub state_updates: BTreeMap<String, ItemScalar>,
    pub evidence: ItemMetadataTable,
}
/// Assembly failure can follow visible writes. Retain that prefix explicitly
/// instead of making a failing source operation appear to have rolled back.
#[derive(Debug, Clone)]
pub struct AssemblyExecution {
    pub outcome: DependencyResult<AssemblyOutcome>,
    pub prefix: Option<AssemblyOutcome>,
}
/// Explicit dependencies are supplied by a caller; the library has no Lua fallback.
/// Available empty modifier lists mean a successful empty parse, distinct from nil.
pub trait ItemLoadProvider {
    fn parse_modifier(&mut self, _request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        DependencyResult::Unavailable("general modifier parser is unavailable".into())
    }
    fn format_line(&mut self, _request: &FormatRequest) -> DependencyResult<String> {
        DependencyResult::Unavailable("general ItemTools range formatting is unavailable".into())
    }
    fn format_with_trace(&mut self, request: &FormatRequest) -> FormatOutcome {
        FormatOutcome {
            result: self.format_line(request),
            precision_parser_calls: Vec::new(),
        }
    }
    fn catalyst_scaling(&self) -> Option<&CatalystScalingData> {
        None
    }
    fn lookup_unique(
        &mut self,
        _request: &UniqueRequest,
    ) -> DependencyResult<Option<UniqueOutcome>> {
        DependencyResult::Unavailable("parsed unique database is unavailable".into())
    }
    fn assemble(&mut self, _request: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        DependencyResult::Unavailable("complete item modifier assembly is unavailable".into())
    }
    fn assemble_with_trace(&mut self, request: &AssemblyRequest) -> AssemblyExecution {
        AssemblyExecution {
            outcome: self.assemble(request),
            prefix: None,
        }
    }
}
#[derive(Debug, Default)]
pub struct UnavailableItemLoadProvider;
impl ItemLoadProvider for UnavailableItemLoadProvider {}
/// A source definition contribution; generated rows have no authored text index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuneContribution {
    pub socket_index: usize,
    pub slot_key: String,
    pub bonded: bool,
    pub definition_line_index: usize,
    pub combined: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct LoadedModLine {
    pub line: String,
    pub source_line: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rune_origins: Vec<RuneContribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<ItemNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub augment_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rune_count: Option<ItemNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socketed_rune_effect_already_applied: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_value_scalar: Option<ItemNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socketed_augment_type_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socketed_soul_core_type: Option<String>,
    pub selection: LineSelection,
    pub flags: BTreeSet<String>,
    pub mod_tags: Vec<String>,
    pub range: ItemNumber,
    pub corrupted_range: ItemNumber,
    pub value_scalar: ItemNumber,
    pub modifiers: Vec<ItemMetadataTable>,
    pub extra: Option<String>,
}
/// ParseRaw-local suppression sets. Present-empty still prevents regeneration.
#[derive(Default)]
struct BaseBuffLines {
    flask: Option<BTreeSet<String>>,
    charm: Option<BTreeSet<String>>,
}
impl BaseBuffLines {
    fn consume(&mut self, line: &str) -> bool {
        // Original if/elseif order: a shared text can consume one entry from each
        // set on successive authored lines, never both on the first line.
        self.flask.as_mut().is_some_and(|set| set.remove(line))
            || self.charm.as_mut().is_some_and(|set| set.remove(line))
    }
}
/// A present empty independent range is distinct from an absent scalar range.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ItemAffixRange {
    Scalar(ItemNumber),
    Independent(Vec<ItemNumber>),
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemAffix {
    pub mod_id: String,
    pub range: Option<ItemAffixRange>,
    pub fractured: Option<bool>,
}
/// ParseRaw resets both lists, including limits. Rows beyond the active limit
/// remain authored loading state and are not silently truncated.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ItemAffixList {
    pub entries: Vec<ItemAffix>,
    pub limit: Option<ItemNumber>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemState {
    pub raw: String,
    pub raw_lines: Vec<String>,
    pub name: String,
    pub name_prefix: String,
    pub name_suffix: String,
    pub rarity: String,
    pub base_name: Option<String>,
    pub base_present: bool,
    pub item_type: Option<String>,
    pub retained_fields: BTreeMap<String, ItemScalar>,
    /// Numeric named armourData entries, retained across reparses. This is a
    /// diagnostic projection, not assembled defensive calculation input.
    pub armour_data: Option<BTreeMap<String, ItemNumber>>,
    /// False when the owned graph contains values absent from this projection.
    pub armour_data_complete: bool,
    pub variants: VariantState,
    pub requirements: BTreeMap<String, ItemNumber>,
    pub prefixes: ItemAffixList,
    pub suffixes: ItemAffixList,
    pub sockets: Vec<u32>,
    pub runes: Vec<String>,
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub socketed_soul_core_types: BTreeSet<String>,
    pub item_socket_count: usize,
    pub jewel_socket_count: usize,
    pub base_lines: BTreeMap<String, LineSelection>,
    pub buff_mod_lines: Vec<LoadedModLine>,
    pub enchant_mod_lines: Vec<LoadedModLine>,
    pub rune_mod_lines: Vec<LoadedModLine>,
    pub class_requirement_mod_lines: Vec<LoadedModLine>,
    pub implicit_mod_lines: Vec<LoadedModLine>,
    pub explicit_mod_lines: Vec<LoadedModLine>,
    pub parser_calls: Vec<ParseRequest>,
    pub format_calls: Vec<FormatRequest>,
    pub format_parser_calls: Vec<FormatParserCall>,
    pub assembly_calls: usize,
    pub assembly_evidence: Option<ItemMetadataTable>,
}
impl Default for ItemState {
    fn default() -> Self {
        Self {
            raw: String::new(),
            raw_lines: Vec::new(),
            name: "?".into(),
            name_prefix: String::new(),
            name_suffix: String::new(),
            rarity: String::new(),
            base_name: None,
            base_present: false,
            item_type: None,
            retained_fields: BTreeMap::new(),
            armour_data: None,
            armour_data_complete: true,
            variants: VariantState::default(),
            requirements: BTreeMap::new(),
            prefixes: ItemAffixList::default(),
            suffixes: ItemAffixList::default(),
            sockets: Vec::new(),
            runes: Vec::new(),
            socketed_soul_core_types: BTreeSet::new(),
            item_socket_count: 0,
            jewel_socket_count: 0,
            base_lines: BTreeMap::new(),
            buff_mod_lines: Vec::new(),
            enchant_mod_lines: Vec::new(),
            rune_mod_lines: Vec::new(),
            class_requirement_mod_lines: Vec::new(),
            implicit_mod_lines: Vec::new(),
            explicit_mod_lines: Vec::new(),
            parser_calls: Vec::new(),
            format_calls: Vec::new(),
            format_parser_calls: Vec::new(),
            assembly_calls: 0,
            assembly_evidence: None,
        }
    }
}
/// One ordered Item instance. A pending dependency stops this instance. Callers
/// can reconstruct it with a richer provider, keeping source instructions exact.
pub struct ItemLoadMachine<'a> {
    catalog: &'a ItemLoadingCatalog,
    state: ItemState,
    status: ItemLoadStatus,
    pending: Option<PendingDependency>,
    work_bytes: usize,
    affix_programs: Option<AffixPrograms>,
    affix_budget: MatchBudget,
    implicit_budget: MatchBudget,
    rune_programs: Option<RunePrograms>,
    assembly: Option<super::assembly::AssembledItem>,
    assembly_final: bool,
    assembly_reparsed: bool,
    assembly_item: Arc<()>,
    armour_header_updates: BTreeMap<String, ItemNumber>,
}
#[derive(Clone, Copy, PartialEq)]
enum GameStage {
    FindImplicit,
    Implicit,
    FindExplicit,
    Explicit,
    Done,
}
impl<'a> ItemLoadMachine<'a> {
    pub fn new(catalog: &'a ItemLoadingCatalog) -> Self {
        let mut machine = Self {
            catalog,
            state: ItemState::default(),
            status: ItemLoadStatus::NoBase,
            pending: None,
            work_bytes: 0,
            affix_programs: None,
            affix_budget: MatchBudget::default(),
            implicit_budget: MatchBudget::default(),
            rune_programs: None,
            assembly: None,
            assembly_final: false,
            assembly_reparsed: true,
            assembly_item: Arc::new(()),
            armour_header_updates: BTreeMap::new(),
        };
        machine.reset("");
        machine.number(
            "affixLimit",
            ItemNumber::new(catalog.policy().affix_loading.reconcile.initial_limit),
        );
        machine.state.assembly_calls = 1;
        machine
    }
    /// Only a successful final assembly can authorize production registration.
    pub fn assembled(&self) -> Option<&super::assembly::AssembledItem> {
        self.assembly.as_ref().filter(|item| {
            self.assembly_final && self.status == ItemLoadStatus::Complete && item.is_complete()
        })
    }
    /// Retained source-visible assembly state; may be an incomplete failure prefix.
    pub fn assembly_progress(&self) -> Option<&super::assembly::AssembledItem> {
        self.assembly.as_ref()
    }
    pub fn evidence_bytes(&self) -> usize {
        self.work_bytes
    }
    fn charge(&mut self, bytes: usize) -> Result<(), ItemLoadError> {
        self.work_bytes = self
            .work_bytes
            .checked_add(bytes)
            .ok_or_else(|| ItemLoadError("item evidence size overflow".into()))?;
        if self.work_bytes > MAX_ITEM_LOADING_EVIDENCE_BYTES {
            return Err(ItemLoadError("item evidence size bound".into()));
        }
        Ok(())
    }
    pub fn state(&self) -> &ItemState {
        &self.state
    }
    pub fn status(&self) -> ItemLoadStatus {
        self.status
    }
    pub fn pending(&self) -> Option<&PendingDependency> {
        self.pending.as_ref()
    }
    pub fn into_state(self) -> ItemState {
        self.state
    }
    pub fn set_xml_attributes(&mut self, attributes: &BTreeMap<String, String>) {
        self.assembly_final = false;
        for key in ["id", "variant"] {
            let value = attributes
                .get(key)
                .map_or(ItemNumber::Nil, |s| syntax::lua_number(s));
            if key == "variant" {
                self.state.variants.selected = number_option(value);
            } else {
                self.number(key, value);
            }
        }
        for i in 0..5 {
            let key = if i == 0 {
                "variantAlt".into()
            } else {
                format!("variantAlt{}", i + 1)
            };
            if let Some(value) = attributes.get(&key) {
                self.state.variants.has_alternate[i] = true;
                self.state.variants.alternate[i] = number_option(syntax::lua_number(value));
            }
        }
    }
    fn number(&mut self, key: &str, value: ItemNumber) {
        if value == ItemNumber::Nil {
            self.state.retained_fields.remove(key);
        } else {
            self.state
                .retained_fields
                .insert(key.into(), ItemScalar::Number(value));
        }
    }
    fn boolean(&mut self, key: &str, value: bool) {
        self.state
            .retained_fields
            .insert(key.into(), ItemScalar::Boolean(value));
    }
    fn text(&mut self, key: &str, value: &str) {
        self.state
            .retained_fields
            .insert(key.into(), ItemScalar::Text(value.into()));
    }
    fn num(&self, key: &str) -> Option<f64> {
        match self.state.retained_fields.get(key) {
            Some(ItemScalar::Number(n)) => n.value(),
            _ => None,
        }
    }
    fn flag(&self, key: &str) -> bool {
        matches!(
            self.state.retained_fields.get(key),
            Some(ItemScalar::Boolean(true))
        )
    }
    fn txt(&self, key: &str) -> Option<&str> {
        match self.state.retained_fields.get(key) {
            Some(ItemScalar::Text(s)) => Some(s),
            _ => None,
        }
    }
    fn stop(
        &mut self,
        kind: DependencyKind,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Result<(), ItemLoadError> {
        let message = message.into();
        if message.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE {
            return Err(ItemLoadError("dependency message bound".into()));
        }
        self.charge(message.len())?;
        self.status = ItemLoadStatus::Pending;
        self.pending = Some(PendingDependency {
            kind,
            line_index: line,
            message,
        });
        Ok(())
    }
    fn reject_dependency<T>(&mut self, message: String, source: bool) -> Result<T, ItemLoadError> {
        if message.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE {
            return Err(ItemLoadError("dependency error message bound".into()));
        }
        self.charge(message.len())?;
        if source {
            self.status = ItemLoadStatus::SourceError;
        }
        Err(ItemLoadError(message))
    }
    fn charge_parse_outcome(&mut self, value: &ParseOutcome) -> Result<(), ItemLoadError> {
        if value.modifiers.as_ref().is_some_and(|v| v.len() > 4096)
            || value
                .extra
                .as_ref()
                .is_some_and(|v| v.len() > MAX_ITEM_LOADING_TEXT)
        {
            return Err(ItemLoadError("parser result bound".into()));
        }
        self.charge(value.extra.as_ref().map_or(0, String::len))?;
        if let Some(mods) = &value.modifiers {
            self.charge(validate_metadata_tables(mods.iter())?)?;
        }
        Ok(())
    }
    fn reset(&mut self, raw: &str) {
        self.assembly_final = false;
        self.assembly_reparsed = true;
        // A no-base parse can write defence headers without reaching assembly.
        // Keep those pending writes until a later assembly applies them to the graph.
        self.state.raw = raw.into();
        self.state.raw_lines = syntax::raw_lines(raw);
        self.state.name = "?".into();
        self.state.name_prefix.clear();
        self.state.name_suffix.clear();
        self.state.base_present = false;
        // ParseRaw preserves baseName, type and the optional armourData table.
        self.state.rarity = self.role("default").into();
        for key in ["charmLimit", "spiritValue", "runicItem", "quality"] {
            self.state.retained_fields.remove(key);
        }
        self.boolean("checkSection", false);
        self.boolean("advancedCopy", false);
        self.state.sockets.clear();
        self.state.runes.clear();
        self.state.socketed_soul_core_types.clear();
        self.state.item_socket_count = 0;
        self.state.jewel_socket_count = 0;
        self.state.requirements.clear();
        self.state.prefixes = ItemAffixList::default();
        self.state.suffixes = ItemAffixList::default();
        for k in ["runeLevel", "str", "dex", "int"] {
            self.state
                .requirements
                .insert(k.into(), ItemNumber::new(0.0));
        }
        self.state.base_lines.clear();
        self.state.buff_mod_lines.clear();
        self.state.enchant_mod_lines.clear();
        self.state.rune_mod_lines.clear();
        self.state.class_requirement_mod_lines.clear();
        self.state.implicit_mod_lines.clear();
        self.state.explicit_mod_lines.clear();
        self.state
            .retained_fields
            .remove("socketedAugmentTypeOverride");
        self.state.assembly_evidence = None;
    }
    pub fn apply_text(
        &mut self,
        raw: &str,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<(), ItemLoadError> {
        if self.status == ItemLoadStatus::Pending || self.status == ItemLoadStatus::SourceError {
            return Ok(());
        }
        if raw.len() > MAX_ITEM_LOADING_TEXT {
            return Err(ItemLoadError("item text exceeds limit".into()));
        }
        self.charge(raw.len().saturating_mul(2))?;
        self.reset(raw);
        if self.state.raw_lines.len() > MAX_ITEM_LOADING_LINES {
            return Err(ItemLoadError("item line count exceeds limit".into()));
        }
        let lines = self.state.raw_lines.clone();
        let mut index = 0;
        let mut game = false;
        let mut item_class = None;
        if lines.first().is_some_and(|s| s.starts_with("Item Class:")) {
            item_class = lines.first().cloned();
            index += 1;
            if index >= lines.len() {
                self.status = ItemLoadStatus::SourceError;
                return Err(ItemLoadError(
                    "source Rarity lookup indexes an absent line after Item Class".into(),
                ));
            }
        }
        if let Some(line) = lines.get(index).and_then(|s| s.strip_prefix("Rarity: ")) {
            let rarity: String = line.chars().take_while(char::is_ascii_alphabetic).collect();
            if !rarity.is_empty() {
                game = true;
                let rarity = rarity.to_ascii_uppercase();
                if self.catalog.policy().rarities.contains(&rarity) {
                    self.state.rarity = rarity;
                }
                if self.state.rarity == self.role("unique")
                    && lines.iter().any(|l| l.contains("Foil Unique"))
                {
                    self.state.rarity = self.role("relic").into();
                }
                index += 1;
            }
        }
        if lines.get(index).is_some_and(|s| s == "--------") {
            index += 1;
        }
        let mut unidentified = false;
        if let Some(name) = lines.get(index) {
            if self.state.rarity == self.role("unique")
                && self.catalog.base(name).is_some()
                && lines
                    .get(index + 1)
                    .is_none_or(|s| self.catalog.base(s).is_none())
            {
                self.state.name = "Unidentified item".into();
                self.state.base_name = Some(name.clone());
                self.state.base_present = true;
                unidentified = true;
            } else {
                self.state.name = name.clone();
            }
            unidentified |= lines.iter().any(|s| s == "Unidentified");
            if !self.plain_rarity() && !unidentified {
                index += 1;
            }
        }
        let selections = match self.state.variants.scan(&lines) {
            Ok(v) => v,
            Err(e) => {
                self.stop(DependencyKind::NumericIndex, None, e)?;
                return Ok(());
            }
        };
        let mut stage = GameStage::FindImplicit;
        let mut found_explicit = false;
        let mut found_implicit = false;
        let mut implicit_count = 0.0;
        let mut check_section = false;
        let mut imported_level = None;
        let mut base_buffs = BaseBuffLines::default();
        let mut skipped_rune_lines = 0usize;
        while index < lines.len() {
            let original = &lines[index];
            let mut line = original.clone();
            let line_index = index + 1;
            if base_buffs.consume(&line) {
                index += 1;
                continue;
            }
            if line == "--------" {
                check_section = true;
                self.boolean("checkSection", true);
                index += 1;
                continue;
            }
            if line == "Requirements:" {
                index += 1;
                continue;
            }
            if line.starts_with('(') && line.as_bytes().get(1).is_some_and(u8::is_ascii_alphabetic)
            {
                while index < lines.len() && !lines[index].ends_with(')') {
                    index += 1;
                }
                index += 1;
                continue;
            }
            if line.starts_with("{ ") {
                self.stop(DependencyKind::AdvancedCopyAffixes,Some(line_index),"advanced-copy affix matching and spawn weights require represented dependencies")?;
                return Ok(());
            }
            if self.apply_literal(&line) {
                index += 1;
                continue;
            }
            let base_implicit = self.base_implicit(&line, game)?;
            if check_section {
                match stage {
                    GameStage::Implicit => {
                        if found_implicit && !base_implicit {
                            stage = GameStage::Explicit;
                            found_explicit = true;
                        } else {
                            stage = GameStage::FindExplicit;
                        }
                    }
                    GameStage::Explicit => stage = GameStage::Done,
                    GameStage::FindImplicit
                        if self.num("itemLevel").is_some()
                            && !line.contains(" (implicit)")
                            && !line.contains(" (enchant)")
                            && !line.contains("Talisman Tier") =>
                    {
                        stage = GameStage::Explicit;
                        found_explicit = true;
                    }
                    _ => {}
                }
                check_section = false;
                self.boolean("checkSection", false);
            }
            if let Some(n) = line
                .strip_prefix("Requires Level ")
                .or_else(|| line.strip_prefix("Requires: Level "))
                .map(|s| {
                    s.chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                })
                .filter(|s| !s.is_empty())
            {
                self.state
                    .requirements
                    .insert("level".into(), syntax::spec_to_number(&n));
                index += 1;
                continue;
            }
            let mut spec_exists = false;
            if let Some((name, value)) = syntax::parse_spec(&line) {
                spec_exists = true;
                match self.apply_header(name, value, line_index, &mut imported_level)? {
                    Header::Stop => return Ok(()),
                    Header::Skip => {
                        index += 1;
                        continue;
                    }
                    Header::SkipNext => {
                        index += 2;
                        continue;
                    }
                    Header::Implicits(n) => {
                        implicit_count = n;
                        stage = GameStage::Explicit;
                    }
                    Header::Known => {}
                    Header::Unknown => {
                        if !base_implicit {
                            let simple = |s: &str| {
                                !s.bytes().any(|b| {
                                    matches!(
                                        b,
                                        b'^' | b'$'
                                            | b'('
                                            | b')'
                                            | b'%'
                                            | b'.'
                                            | b'['
                                            | b']'
                                            | b'*'
                                            | b'+'
                                            | b'-'
                                            | b'?'
                                    )
                                })
                            };
                            if !simple(name) {
                                self.stop(
                                    DependencyKind::UnsupportedHeader,
                                    Some(line_index),
                                    "custom-name Lua pattern matching is not represented",
                                )?;
                                return Ok(());
                            }
                            let name_matches = self.state.name.contains(name);
                            if name_matches && !simple(value) {
                                self.stop(
                                    DependencyKind::UnsupportedHeader,
                                    Some(line_index),
                                    "custom-name Lua value pattern matching is not represented",
                                )?;
                                return Ok(());
                            }
                            if !name_matches || !self.state.name.contains(value) {
                                found_explicit = true;
                                stage = GameStage::Explicit;
                            }
                        }
                    }
                }
            }
            if line == "Prefixes:" {
                found_explicit = true;
                stage = GameStage::Explicit;
            }
            if !spec_exists || found_explicit || found_implicit || base_implicit {
                let AnnotatedLine {
                    text: clean,
                    mut flags,
                    tags,
                    range,
                    corrupted_range,
                    selection,
                    has_range_tag,
                } = match annotations(&line, self.catalog, &selections[index]) {
                    Ok(value) => value,
                    Err(message) => {
                        self.stop(DependencyKind::NumericIndex, Some(line_index), message)?;
                        return Ok(());
                    }
                };
                line = clean;
                if has_range_tag {
                    self.boolean("advancedCopy", true);
                }
                if flags.contains("rune") {
                    flags.insert("enchant".into());
                }
                if flags.contains("enchant") || base_implicit {
                    flags.insert("implicit".into());
                }
                for key in ["desecrated", "mutated", "fractured"] {
                    if flags.contains(key) {
                        self.boolean(key, true);
                    }
                }
                if self.resolve_base(
                    &line,
                    &selection,
                    item_class.as_deref(),
                    line_index,
                    &mut base_buffs,
                    provider,
                )? {
                    if self.status == ItemLoadStatus::Pending {
                        return Ok(());
                    }
                    index += 1;
                    continue;
                }
                if self.status == ItemLoadStatus::Pending {
                    return Ok(());
                }
                if flags.contains("implicit") {
                    found_implicit = true;
                    stage = GameStage::Implicit;
                }
                if flags.contains("rune") && !flags.contains("disabled") {
                    let Some(skip) = self.skip_bonded_rune_line(&line, line_index)? else {
                        return Ok(());
                    };
                    if skip {
                        skipped_rune_lines += 1;
                        index += 1;
                        continue;
                    }
                }
                if line.ends_with(" - Unscalable Value")
                    || line.ends_with(" \u{2014} Unscalable Value")
                {
                    let stripped = line.strip_suffix(" - Unscalable Value").unwrap_or(&line);
                    line = stripped
                        .strip_suffix(" \u{2014} Unscalable Value")
                        .unwrap_or(stripped)
                        .into();
                    flags.insert("unscalable".into());
                }
                let catalyst_scalar = if let Some(policy) = provider.catalyst_scaling() {
                    poe_optimizer_engine::item_tools::catalyst_scalar(
                        policy,
                        &self.catalog.policy().catalysts,
                        self.num("catalyst"),
                        Some(&tags),
                        &flags,
                        flags.contains("unscalable"),
                        self.num("catalystQuality"),
                    )
                    .map_err(|e| ItemLoadError(e.to_string()))?
                } else if self.num("catalyst").is_some() && !flags.contains("unscalable") {
                    self.stop(
                        DependencyKind::RangeFormatting,
                        Some(line_index),
                        "catalyst tag scalar requires injected ItemTools policy",
                    )?;
                    return Ok(());
                } else {
                    1.0
                };
                // Advanced-copy current(value) and enum preprocessing belongs to
                // Item.ParseRaw. Plain (min-max) ranges are handled by ItemTools.
                if has_advanced_copy_numeric_or_enum(&line) {
                    self.stop(
                        DependencyKind::RangeFormatting,
                        Some(line_index),
                        "advanced-copy value/enum preprocessing is not represented",
                    )?;
                    return Ok(());
                }
                let Some(ranged) = self.format(
                    &line,
                    line_index,
                    corrupted_range,
                    catalyst_scalar,
                    provider,
                )?
                else {
                    return Ok(());
                };
                let Some(mut outcome) = self.parse(&ranged, line_index, false, provider)? else {
                    return Ok(());
                };
                if (outcome.modifiers.is_none() || outcome.extra.is_some())
                    && index + 1 < lines.len()
                {
                    let next = syntax::strip_next(&lines[index + 1]);
                    let combined = format!("{line} {next}");
                    let Some(ranged) = self.format(
                        &combined,
                        line_index,
                        corrupted_range,
                        catalyst_scalar,
                        provider,
                    )?
                    else {
                        return Ok(());
                    };
                    let Some(result) = self.parse(&ranged, line_index, true, provider)? else {
                        return Ok(());
                    };
                    outcome = result;
                    if outcome.modifiers.is_some() && outcome.extra.is_none() {
                        line = format!("{line}\n{next}");
                        index += 1;
                    } else {
                        let Some(result) = self.parse(&ranged, line_index, false, provider)? else {
                            return Ok(());
                        };
                        outcome = result;
                    }
                }
                let lower = if flags.contains("disabled") {
                    String::new()
                } else {
                    line.to_ascii_lowercase()
                };
                if !self.apply_postparse_effects(&lower, line_index)? {
                    return Ok(());
                }
                let Some((augment_override, soul_core_type)) =
                    self.rune_line_hints(&lower, line_index)?
                else {
                    return Ok(());
                };
                if !flags.contains("disabled") && is_magnitude_line(&line) {
                    self.stop(DependencyKind::ModifierMagnitudes,Some(line_index),"modifier magnitude records require ordered reparsing and unique-database dependencies")?;
                    return Ok(());
                }
                if self.state.variants.matches(&selection) {
                    if let Some(value) = &augment_override {
                        self.text("socketedAugmentTypeOverride", value);
                    } else if let Some(value) = &soul_core_type {
                        self.state.socketed_soul_core_types.insert(value.clone());
                    }
                }
                let recognized = outcome.modifiers.is_some();
                let save = recognized
                    || if game {
                        matches!(stage, GameStage::Implicit | GameStage::Explicit)
                            || (stage == GameStage::FindImplicit
                                && self.catalog.base(&line).is_none()
                                && self.state.name != line
                                && !self
                                    .compat("base_aliases")
                                    .and_then(|t| t.fields.get("two_toned_marker"))
                                    .and_then(ItemMetadataValue::as_str)
                                    .is_some_and(|s| line.contains(s))
                                && !self.base_display_line(&line))
                    } else {
                        found_explicit || stage == GameStage::Explicit
                    };
                if save {
                    let modline = LoadedModLine {
                        line: line.clone(),
                        source_line: Some(line_index),
                        rune_origins: Vec::new(),
                        order: None,
                        augment_type: None,
                        rune_count: None,
                        socketed_rune_effect_already_applied: None,
                        display_value_scalar: None,
                        socketed_augment_type_override: augment_override,
                        socketed_soul_core_type: soul_core_type,
                        selection,
                        flags,
                        mod_tags: tags,
                        range: if recognized && range == ItemNumber::Nil {
                            ItemNumber::new(self.catalog.policy().default_affix_quality)
                        } else {
                            range
                        },
                        corrupted_range,
                        value_scalar: if recognized {
                            ItemNumber::new(catalyst_scalar)
                        } else {
                            ItemNumber::Nil
                        },
                        modifiers: outcome.modifiers.unwrap_or_default(),
                        extra: if recognized {
                            outcome.extra
                        } else {
                            Some(line.clone())
                        },
                    };
                    self.push_line(modline, implicit_count, skipped_rune_lines);
                }
                if recognized {
                    if game {
                        match stage {
                            GameStage::FindImplicit => stage = GameStage::Implicit,
                            GameStage::FindExplicit => {
                                found_explicit = true;
                                stage = GameStage::Explicit;
                            }
                            GameStage::Explicit => found_explicit = true,
                            _ => {}
                        }
                    } else {
                        found_explicit = true;
                    }
                } else if game && stage == GameStage::FindExplicit {
                    stage = GameStage::Done;
                }
            }
            index += 1;
        }
        self.finish_parse(imported_level, game, provider)
    }

    fn role(&self, name: &str) -> &str {
        self.compat("rarity_roles")
            .and_then(|t| t.fields.get(name))
            .and_then(ItemMetadataValue::as_str)
            .unwrap_or("")
    }
    fn plain_rarity(&self) -> bool {
        self.state.rarity == self.role("normal") || self.state.rarity == self.role("magic")
    }
    fn apply_flags_policy(&mut self, policy: &str, line: &str) -> bool {
        let Some(fields) = self
            .compat(policy)
            .and_then(|t| t.fields.get(line))
            .and_then(ItemMetadataValue::as_table)
            .map(|t| t.fields.clone())
        else {
            return false;
        };
        for (field, value) in fields {
            if let Some(value) = value.as_bool() {
                self.boolean(&field, value);
            }
        }
        true
    }
    fn apply_literal(&mut self, line: &str) -> bool {
        self.apply_flags_policy("literal_state_flags", line)
    }
    fn compat(&self, key: &str) -> Option<&ItemMetadataTable> {
        self.catalog
            .policy()
            .compatibility
            .get(key)
            .and_then(ItemMetadataValue::as_table)
    }
    fn apply_header(
        &mut self,
        name: &str,
        value: &str,
        line: usize,
        imported: &mut Option<ItemNumber>,
    ) -> Result<Header, ItemLoadError> {
        if self
            .compat("selection_headers")
            .is_some_and(|t| t.fields.contains_key(name))
        {
            return Ok(Header::Known);
        }
        if let Some(key) = self.catalog.defence_header_key(name).map(str::to_owned) {
            // The source branch precedes hidden_specs, even for overlapping names.
            // Only the base reference/name change; ordinary base setup would also
            // reset unrelated requirements, type, affix and modifier state.
            let target = self
                .catalog
                .armour_header_rewrite(name)
                .filter(|rewrite| self.state.base_name.as_deref() == Some(rewrite.from))
                .map(|rewrite| rewrite.to.to_owned());
            self.charge(key.len() + target.as_ref().map_or(0, String::len) + 64)?;
            if let Some(target) = target {
                self.state.base_present = self.catalog.base(&target).is_some();
                self.state.base_name = Some(target);
            }
            let number = syntax::spec_to_number(value);
            if self.armour_header_updates.len() >= 256
                && !self.armour_header_updates.contains_key(&key)
            {
                return Err(ItemLoadError("armour header update count bound".into()));
            }
            self.charge(key.len() + 64)?;
            self.armour_header_updates.insert(key.clone(), number);
            let data = self.state.armour_data.get_or_insert_with(BTreeMap::new);
            if number == ItemNumber::Nil {
                data.remove(&key);
            } else {
                if data.len() >= 256 && !data.contains_key(&key) {
                    return Err(ItemLoadError("armour data entry count bound".into()));
                }
                data.insert(key, number);
            }
            // Existing explicit/implicit state can still cause this line to be
            // parsed as modifier text after the header operation.
            return Ok(Header::Known);
        }
        if let Some(side) = self.catalog.affix_header(name) {
            if !self.prepare_affix_programs(Some(line))? {
                return Ok(Header::Stop);
            }
            let result = self
                .affix_programs
                .as_ref()
                .expect("prepared affix programs")
                .header(
                    value,
                    &self.catalog.policy().affix_loading,
                    self.catalog.policy().default_affix_quality,
                    &mut self.affix_budget,
                );
            let affix = match result {
                Ok(value) => value,
                Err(error) => {
                    self.reject_affix(error, Some(line))?;
                    return Ok(Header::Stop);
                }
            };
            let ranges = match &affix.range {
                Some(ItemAffixRange::Independent(v)) => v.len(),
                _ => 1,
            };
            self.charge(affix.mod_id.len() + ranges * std::mem::size_of::<ItemNumber>() + 128)?;
            let entries = &mut self.affix_list_mut(side).entries;
            if entries.len() >= MAX_ITEM_LOADING_LINES {
                return Err(ItemLoadError("affix row count bound".into()));
            }
            entries.push(affix);
            return Ok(Header::Known);
        }
        if name == self.catalog.policy().rune_loading.socket_header {
            return Ok(if self.rune_sockets(value, line)? {
                Header::Known
            } else {
                Header::Stop
            });
        }
        if name == self.catalog.policy().rune_loading.rune_header {
            if !self.prepare_rune_header(line)? {
                return Ok(Header::Stop);
            }
            self.state.runes.push(value.into());
            return Ok(Header::Known);
        }
        match name {
            "Implicits" => {
                return Ok(Header::Implicits(
                    syntax::spec_to_number(value).value().unwrap_or(0.0),
                ));
            }
            "Has Variants" | "Selected Variants" => return Ok(Header::SkipNext),
            "Level" => {
                *imported = number_option(syntax::spec_to_number(value));
                return Ok(Header::Known);
            }
            "Cluster Jewel Skill" | "Cluster Jewel Node Count" => {
                self.stop(
                    DependencyKind::ClusterJewel,
                    Some(line),
                    "cluster-jewel initialization and selected tree data are unavailable",
                )?;
                return Ok(Header::Stop);
            }
            "Radius" => {
                if self.state.item_type.as_deref() == Some("Jewel") {
                    self.stop(
                        DependencyKind::Assembly,
                        Some(line),
                        "jewel radius requires selected tree version and assembled jewel data",
                    )?;
                    return Ok(Header::Stop);
                }
            }
            "Catalyst" => {
                if let Some(i) = self
                    .catalog
                    .policy()
                    .catalysts
                    .iter()
                    .position(|c| c.name == value)
                {
                    self.number("catalyst", ItemNumber::new((i + 1) as f64));
                }
                return Ok(Header::Known);
            }
            _ => {}
        }
        if name.starts_with("Quality (") && name.ends_with(" Modifiers)") {
            let descriptor = &name[9..name.len() - 11];
            let percent = value
                .find('%')
                .map(|end| {
                    let start = value[..end]
                        .rfind(|c: char| !c.is_ascii_digit())
                        .map_or(0, |i| i + 1);
                    syntax::spec_to_number(&value[start..end])
                })
                .unwrap_or(ItemNumber::Nil);
            self.number("catalystQuality", percent);
            if let Some(i) = self
                .catalog
                .policy()
                .catalysts
                .iter()
                .position(|c| c.descriptor == descriptor)
            {
                self.number("catalyst", ItemNumber::new((i + 1) as f64));
            }
            return Ok(Header::Known);
        }
        if let Some(mapping) = self
            .compat("header_assignments")
            .and_then(|t| t.fields.get(name))
            .and_then(ItemMetadataValue::as_table)
        {
            let field = mapping
                .fields
                .get("field")
                .and_then(ItemMetadataValue::as_str)
                .ok_or_else(|| ItemLoadError("header assignment has no target".into()))?
                .to_owned();
            let kind = mapping
                .fields
                .get("kind")
                .and_then(ItemMetadataValue::as_str)
                .ok_or_else(|| ItemLoadError("header assignment has no kind".into()))?;
            if field.starts_with("armourData.") {
                self.stop(
                    DependencyKind::BaseCompatibility,
                    Some(line),
                    "display defence headers require source base-compatibility rebinding",
                )?;
                return Ok(Header::Stop);
            }
            match kind {
                "number" => {
                    let n = syntax::spec_to_number(value);
                    if let Some(key) = field.strip_prefix("requirements.") {
                        if n == ItemNumber::Nil {
                            self.state.requirements.remove(key);
                        } else {
                            self.state.requirements.insert(key.into(), n);
                        }
                    } else {
                        self.number(&field, n);
                    }
                }
                "text" => self.text(&field, value),
                "presence" => self.boolean(&field, true),
                "boolean" => self.boolean(&field, value == "true"),
                _ => return Err(ItemLoadError("unsupported header assignment kind".into())),
            }
            if let Some(i) = alternate_index(&field, "hasAltVariant") {
                self.state.variants.has_alternate[i] = true;
            }
            if let Some(i) = alternate_index(&field, "variantAlt") {
                self.state.variants.alternate[i] = number_option(syntax::spec_to_number(value));
            }
            if field == "allowDuplicateVariants" {
                self.state.variants.allow_duplicates = value == "true";
            }
            return Ok(if field == "uniqueID" {
                Header::Skip
            } else {
                Header::Known
            });
        }
        if self
            .compat("hidden_specs")
            .is_some_and(|t| t.fields.contains_key(name))
        {
            self.boolean("hidden_specs", true);
            return Ok(Header::Known);
        }
        if self.catalog.policy().header_names.contains(name) {
            self.stop(
                DependencyKind::UnsupportedHeader,
                Some(line),
                format!("source header {name:?} has an unrepresented state operation"),
            )?;
            return Ok(Header::Stop);
        }
        Ok(Header::Unknown)
    }
    fn base_implicit(&mut self, line: &str, game: bool) -> Result<bool, ItemLoadError> {
        if !game || self.flag("crafted") || !self.state.base_present {
            return Ok(false);
        }
        let Some(implicit) = self
            .state
            .base_name
            .as_deref()
            .and_then(|name| self.catalog.base(name))
            .and_then(|base| base.implicit())
        else {
            return Ok(false);
        };
        match base_has_implicit_line(implicit, line, &mut self.implicit_budget) {
            Ok(found) => Ok(found),
            Err(error) => self.reject_dependency(
                format!("base implicit matching: {error}"),
                matches!(error, PatternError::Source(_)),
            ),
        }
    }
    fn base(&self) -> Option<&poe_optimizer_data::item_loading::ItemBaseDefinition> {
        if self.state.base_present {
            self.state
                .base_name
                .as_deref()
                .and_then(|n| self.catalog.base(n))
        } else {
            None
        }
    }
    fn base_display_line(&self, line: &str) -> bool {
        self.base().is_some_and(|b| {
            line == b.item_type
                || b.sub_type()
                    .is_some_and(|s| line == format!("{s} {}", b.item_type))
        })
    }
    fn resolve_base(
        &mut self,
        line: &str,
        selection: &LineSelection,
        item_class: Option<&str>,
        line_index: usize,
        base_buffs: &mut BaseBuffLines,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<bool, ItemLoadError> {
        let mut chosen = None;
        if !self.state.base_present && self.plain_rarity() {
            if item_class.is_some()
                && self
                    .compat("base_aliases")
                    .and_then(|t| t.fields.get("energy_blade_marker"))
                    .and_then(ItemMetadataValue::as_str)
                    .is_some_and(|s| self.state.name.contains(s))
            {
                self.stop(
                    DependencyKind::BaseCompatibility,
                    Some(line_index),
                    "energy-blade class rewrite requires compatibility resolution",
                )?;
                return Ok(false);
            }
            if self.catalog.base(&self.state.name).is_some() {
                chosen = Some(self.state.name.clone());
            } else {
                let mut matches = Vec::new();
                let mut longest = 0;
                for base in self.catalog.bases() {
                    if let Some(start) = self.state.name.find(&base.name) {
                        if base.name.len() > longest {
                            matches.clear();
                            longest = base.name.len();
                        }
                        if base.name.len() == longest {
                            matches.push((base.name.clone(), start));
                        }
                    }
                }
                if matches.len() > 1 {
                    self.stop(
                        DependencyKind::BaseLookupAmbiguity,
                        Some(line_index),
                        "equal-length partial base matches depend on original pairs order",
                    )?;
                    return Ok(false);
                }
                if let Some((name, start)) = matches.pop() {
                    self.state.name_prefix = self.state.name[..start].into();
                    self.state.name_suffix = self.state.name[start + name.len()..].into();
                    chosen = Some(name);
                }
            }
            if chosen.is_none()
                && let Some(marker) = self
                    .compat("base_aliases")
                    .and_then(|t| t.fields.get("two_toned_marker"))
                    .and_then(ItemMetadataValue::as_str)
                && let Some(start) = self.state.name.find(marker)
            {
                let marker = marker.to_owned();
                self.state.name_prefix = self.state.name[..start].into();
                self.state.name_suffix = self.state.name[start + marker.len()..].into();
                chosen = Some(marker);
            }
            self.state.name = syntax::strip_name_parentheses(&self.state.name);
        }
        let mut name = chosen.unwrap_or_else(|| {
            line.strip_prefix(
                self.catalog
                    .policy()
                    .compatibility
                    .get("superior_prefix")
                    .and_then(ItemMetadataValue::as_str)
                    .unwrap_or("\0"),
            )
            .unwrap_or(line)
            .into()
        });
        if let Some(aliases) = self.compat("base_aliases") {
            if aliases
                .fields
                .get("two_toned_marker")
                .and_then(ItemMetadataValue::as_str)
                == Some(name.as_str())
                && let Some(value) = aliases
                    .fields
                    .get("two_toned_default")
                    .and_then(ItemMetadataValue::as_str)
            {
                name = value.into();
            }
            let runic = aliases
                .fields
                .get("runic_markers")
                .and_then(ItemMetadataValue::as_array)
                .is_some_and(|a| {
                    a.iter()
                        .filter_map(ItemMetadataValue::as_str)
                        .any(|s| name.contains(s))
                });
            if runic {
                self.boolean("runicItem", true);
            }
        }
        let Some(base) = self.catalog.base(&name) else {
            return Ok(false);
        };
        self.state
            .base_lines
            .insert(name.clone(), selection.clone());
        let matches = if self.state.variants.uses_versioned_or_grouped() {
            self.state.variants.matches(selection)
        } else {
            self.state.variants.selected.is_none()
                || selection.variants.as_ref().is_none_or(|set| {
                    self.state
                        .variants
                        .selected
                        .and_then(ItemNumber::value)
                        .is_some_and(|n| set.iter().any(|&v| f64::from(v) == n))
                })
        };
        if matches {
            let item_type = base.item_type.clone();
            let charm = base.field("charmLimit").and_then(ItemMetadataValue::as_f64);
            let spirit = base.field("spirit").and_then(ItemMetadataValue::as_f64);
            let reqs = base.requirements().cloned();
            self.state.base_name = Some(name);
            self.state.base_present = true;
            self.state.item_type = Some(item_type.clone());
            if !self.plain_rarity() {
                let title = self.state.name.clone();
                self.text("title", &title);
            }
            self.number("charmLimit", charm.map_or(ItemNumber::Nil, ItemNumber::new));
            self.number(
                "spiritValue",
                spirit.map_or(ItemNumber::Nil, ItemNumber::new),
            );
            let Some(noncorruptible) = self
                .catalog
                .policy()
                .compatibility
                .get("noncorruptible_types")
                .and_then(ItemMetadataValue::as_array)
            else {
                self.stop(
                    DependencyKind::BaseCompatibility,
                    Some(line_index),
                    "injected noncorruptible-type policy is missing",
                )?;
                return Ok(true);
            };
            let corruptible = !noncorruptible
                .iter()
                .filter_map(ItemMetadataValue::as_str)
                .any(|t| t == item_type);
            self.boolean("corruptible", corruptible);
            let fallback = self
                .catalog
                .policy()
                .compatibility
                .get("fallback_modifier_table")
                .and_then(ItemMetadataValue::as_str);
            let table = base
                .sub_type()
                .map(|sub| format!("{}{sub}", base.item_type))
                .filter(|key| self.catalog.modifier_table(key).is_some())
                .or_else(|| {
                    self.catalog
                        .modifier_table(&base.item_type)
                        .map(|_| base.item_type.clone())
                })
                .or_else(|| {
                    fallback
                        .filter(|key| self.catalog.modifier_table(key).is_some())
                        .map(str::to_owned)
                });
            if let Some(table) = table {
                self.text("affixes_table", &table);
            } else {
                self.state.retained_fields.remove("affixes_table");
            }
            for stat in ["str", "dex", "int"] {
                self.state.requirements.insert(
                    stat.into(),
                    ItemNumber::new(
                        reqs.as_ref()
                            .and_then(|r| r.fields.get(stat))
                            .and_then(ItemMetadataValue::as_f64)
                            .unwrap_or(0.0),
                    ),
                );
            }
            self.text("defaultSocketColor", "S");
            self.load_base_buffs(base_buffs, line_index, provider)?;
        }
        Ok(true)
    }
    fn load_base_buffs(
        &mut self,
        local: &mut BaseBuffLines,
        line_index: usize,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<(), ItemLoadError> {
        // Clone the immutable catalog handle, not the definitions or buff arrays.
        let catalog = self.catalog.clone();
        let base = self
            .state
            .base_name
            .as_deref()
            .and_then(|name| catalog.base(name));
        let Some(base) = base else {
            return Ok(());
        };
        for (kind, seen) in [("flask", &mut local.flask), ("charm", &mut local.charm)] {
            let buff = match base.field(kind) {
                None | Some(ItemMetadataValue::Boolean(false)) => continue,
                Some(ItemMetadataValue::Table(table)) => table.fields.get("buff"),
                // A sequence has no string key. Lua strings use the standard
                // string metatable, whose "buff" member is also absent.
                Some(ItemMetadataValue::Array(_) | ItemMetadataValue::Text(_)) => None,
                Some(_) => {
                    return self.reject_dependency(
                        format!("source base {kind}.buff indexes a non-table value"),
                        true,
                    );
                }
            };
            let Some(buff) =
                buff.filter(|value| !matches!(value, ItemMetadataValue::Boolean(false)))
            else {
                continue;
            };
            // The source reads the parent's buff field before testing the local
            // set, but evaluates ipairs only for its first truthy definition.
            if seen.is_some() {
                continue;
            }
            self.charge(64)?;
            *seen = Some(BTreeSet::new());
            if !matches!(
                buff,
                ItemMetadataValue::Array(_) | ItemMetadataValue::Table(_)
            ) {
                return self.reject_dependency(
                    format!("source base {kind} buff ipairs expects a table"),
                    true,
                );
            }
            let mut index = 1_i64;
            loop {
                let value = match buff {
                    ItemMetadataValue::Array(values) => values.get((index - 1) as usize),
                    ItemMetadataValue::Table(table) => table.indexed.get(&index),
                    _ => unreachable!("buff shape checked above"),
                };
                let Some(value) = value else {
                    break;
                };
                let ItemMetadataValue::Text(text) = value else {
                    return self.reject_dependency(
                        format!("source base {kind} buff modifier is not a string"),
                        true,
                    );
                };
                if self.state.buff_mod_lines.len() >= MAX_ITEM_LOADING_LINES {
                    return Err(ItemLoadError("base buff modifier line count bound".into()));
                }
                let set = seen.as_mut().expect("initialized before ipairs");
                if !set.contains(text) {
                    self.charge(text.len() + 64)?;
                    set.insert(text.clone());
                }
                // Base buffs call the parser directly: no formatting, annotation
                // stripping, combined-line retry, or base variant inheritance.
                let Some(outcome) = self.parse(text, line_index, false, provider)? else {
                    return Ok(());
                };
                self.charge(text.len() + 128)?;
                self.state.buff_mod_lines.push(LoadedModLine {
                    line: text.clone(),
                    source_line: Some(line_index),
                    rune_origins: Vec::new(),
                    order: None,
                    augment_type: None,
                    rune_count: None,
                    socketed_rune_effect_already_applied: None,
                    display_value_scalar: None,
                    socketed_augment_type_override: None,
                    socketed_soul_core_type: None,
                    selection: LineSelection::default(),
                    flags: BTreeSet::new(),
                    mod_tags: Vec::new(),
                    range: ItemNumber::Nil,
                    corrupted_range: ItemNumber::Nil,
                    value_scalar: ItemNumber::Nil,
                    modifiers: outcome.modifiers.unwrap_or_default(),
                    extra: outcome.extra,
                });
                index += 1;
            }
        }
        Ok(())
    }
    fn format(
        &mut self,
        text: &str,
        line: usize,
        corrupted: ItemNumber,
        scalar: f64,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<Option<String>, ItemLoadError> {
        if self.state.format_calls.len() >= MAX_ITEM_LOADING_CALLS {
            return Err(ItemLoadError("format request bound".into()));
        }
        let request = FormatRequest {
            sequence: self.state.format_calls.len()
                + self.state.parser_calls.len()
                + self.state.format_parser_calls.len(),
            line_index: Some(line),
            text: text.into(),
            range: ItemNumber::new(1.0),
            scalar: ItemNumber::new(scalar),
            corrupted_range: corrupted,
        };
        self.charge(text.len())?;
        let sequence = request.sequence;
        let outcome = provider.format_with_trace(&request);
        self.state.format_calls.push(request);
        if outcome.precision_parser_calls.len()
            > MAX_ITEM_LOADING_CALLS.saturating_sub(self.state.format_parser_calls.len())
        {
            return Err(ItemLoadError(
                "format precision parser request bound".into(),
            ));
        }
        for (index, call) in outcome.precision_parser_calls.iter().enumerate() {
            if matches!(call.result, DependencyResult::Available(_)) {
                continue;
            }
            let consistent = match (&call.result, &outcome.result) {
                (DependencyResult::Unavailable(a), DependencyResult::Unavailable(b))
                | (DependencyResult::SourceError(a), DependencyResult::SourceError(b))
                | (DependencyResult::ResourceError(a), DependencyResult::ResourceError(b)) => {
                    a == b
                }
                _ => false,
            };
            if index + 1 != outcome.precision_parser_calls.len() || !consistent {
                return Err(ItemLoadError(
                    "inconsistent format precision parser result".into(),
                ));
            }
        }
        let parser_pending = outcome
            .precision_parser_calls
            .iter()
            .any(|call| matches!(call.result, DependencyResult::Unavailable(_)));
        for (index, call) in outcome.precision_parser_calls.into_iter().enumerate() {
            if call.request.sequence != sequence + 1 + index
                || call.request.line_index != Some(line)
                || call.request.origin.is_some()
                || call.request.combined
                || call.request.text.len() > MAX_ITEM_LOADING_TEXT
            {
                return Err(ItemLoadError(
                    "invalid format precision parser request".into(),
                ));
            }
            self.charge(call.request.text.len())?;
            match &call.result {
                DependencyResult::Available(value) => self.charge_parse_outcome(value)?,
                DependencyResult::Unavailable(message)
                | DependencyResult::SourceError(message)
                | DependencyResult::ResourceError(message) => {
                    if message.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE {
                        return Err(ItemLoadError("dependency message bound".into()));
                    }
                    self.charge(message.len())?;
                }
            }
            self.state.format_parser_calls.push(call);
        }
        match outcome.result {
            DependencyResult::SourceError(message) => self.reject_dependency(message, true),
            DependencyResult::ResourceError(message) => self.reject_dependency(message, false),
            DependencyResult::Unavailable(message) => {
                self.stop(
                    if parser_pending {
                        DependencyKind::ModifierParser
                    } else {
                        DependencyKind::RangeFormatting
                    },
                    Some(line),
                    message,
                )?;
                Ok(None)
            }
            DependencyResult::Available(value) => {
                if value.len() > MAX_ITEM_LOADING_TEXT {
                    return Err(ItemLoadError("formatted line bound".into()));
                }
                self.charge(value.len())?;
                Ok(Some(value))
            }
        }
    }
    fn parse(
        &mut self,
        text: &str,
        line: usize,
        combined: bool,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<Option<ParseOutcome>, ItemLoadError> {
        if self.state.parser_calls.len() >= MAX_ITEM_LOADING_CALLS {
            return Err(ItemLoadError("parser request bound".into()));
        }
        self.charge(text.len())?;
        let request = ParseRequest {
            sequence: self.state.format_calls.len()
                + self.state.parser_calls.len()
                + self.state.format_parser_calls.len(),
            line_index: Some(line),
            origin: None,
            text: text.into(),
            combined,
        };
        let result = provider.parse_modifier(&request);
        self.state.parser_calls.push(request);
        match result {
            DependencyResult::SourceError(message) => self.reject_dependency(message, true),
            DependencyResult::ResourceError(message) => self.reject_dependency(message, false),
            DependencyResult::Unavailable(message) => {
                self.stop(DependencyKind::ModifierParser, Some(line), message)?;
                Ok(None)
            }
            DependencyResult::Available(value) => {
                self.charge_parse_outcome(&value)?;
                Ok(Some(value))
            }
        }
    }
    fn push_line(&mut self, line: LoadedModLine, implicit_count: f64, skipped_rune_lines: usize) {
        if line.flags.contains("rune") {
            self.state.rune_mod_lines.push(line);
        } else if line.flags.contains("enchant") {
            self.state.enchant_mod_lines.push(line);
        } else if line.line.contains("Requires Class") {
            self.state.class_requirement_mod_lines.push(line);
        } else if line.flags.contains("implicit")
            || ((self.state.rune_mod_lines.len()
                + skipped_rune_lines
                + self.state.enchant_mod_lines.len()
                + self.state.implicit_mod_lines.len()) as f64)
                < implicit_count
        {
            self.state.implicit_mod_lines.push(line);
        } else {
            self.state.explicit_mod_lines.push(line);
        }
    }
    fn finish_parse(
        &mut self,
        imported: Option<ItemNumber>,
        game: bool,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<(), ItemLoadError> {
        if let (Some(title), Some(base)) = (self.txt("title"), self.state.base_name.as_deref()) {
            self.state.name = format!("{title}, {}", syntax::strip_name_parentheses(base));
        }
        if !self.finish_runes(game, provider)? {
            return Ok(());
        }
        if self.flag("advancedCopy")
            && (self.state.rarity == self.role("unique") || self.state.rarity == self.role("relic"))
            && !self.state.variants.uses_versioned_or_grouped()
        {
            self.stop(
                DependencyKind::AdvancedCopyAffixes,
                None,
                "advanced-copy unique stat reordering requires represented source ordering",
            )?;
            return Ok(());
        }
        if !self.apply_rune_requirements()? {
            return Ok(());
        }
        if self.state.base_present {
            let base_level = self
                .base()
                .and_then(|b| b.requirements())
                .and_then(|r| r.fields.get("level"))
                .and_then(ItemMetadataValue::as_f64)
                .map(ItemNumber::new);
            let mut unique = None;
            if self.state.rarity == self.role("unique") || self.state.rarity == self.role("relic") {
                let request = UniqueRequest {
                    name: self.state.name.clone(),
                    title: self.txt("title").map(str::to_owned),
                    base_name: self.state.base_name.clone(),
                };
                match provider.lookup_unique(&request) {
                    DependencyResult::SourceError(message) => {
                        return self.reject_dependency(message, true);
                    }
                    DependencyResult::ResourceError(message) => {
                        return self.reject_dependency(message, false);
                    }
                    DependencyResult::Unavailable(message) => {
                        self.stop(DependencyKind::UniqueDatabase, None, message)?;
                        return Ok(());
                    }
                    DependencyResult::Available(value) => {
                        if value.as_ref().is_some_and(|v| {
                            v.natural_level.is_some_and(|n| !n.canonical())
                                || v.level.is_some_and(|n| !n.canonical())
                        }) {
                            return Err(ItemLoadError(
                                "unique provider returned noncanonical finite number".into(),
                            ));
                        }
                        unique = value;
                    }
                }
            }
            let natural = if let Some(unique) = unique {
                let n = unique
                    .natural_level
                    .and_then(ItemNumber::value)
                    .or_else(|| unique.level.and_then(ItemNumber::value));
                let Some(n) = n else {
                    self.status = ItemLoadStatus::SourceError;
                    return Err(ItemLoadError(
                        "unique database item has no natural or level requirement".into(),
                    ));
                };
                ItemNumber::new(syntax::lua_max(
                    n,
                    base_level.and_then(ItemNumber::value).unwrap_or(0.0),
                ))
            } else {
                if !self.state.requirements.contains_key("level")
                    && let Some(level) = if self.state.sockets.is_empty() {
                        imported.or(base_level)
                    } else {
                        base_level
                    }
                {
                    self.state.requirements.insert("level".into(), level);
                }
                self.state
                    .requirements
                    .get("level")
                    .copied()
                    .unwrap_or(ItemNumber::new(0.0))
            };
            self.state
                .requirements
                .insert("naturalLevel".into(), natural);
            let natural = natural.value().unwrap_or(0.0);
            let level = self
                .state
                .requirements
                .get("level")
                .copied()
                .and_then(ItemNumber::value)
                .unwrap_or(natural);
            // Source assigns this fallback before m_max can fail on runeLevel.
            self.state
                .requirements
                .insert("level".into(), ItemNumber::new(level));
            let Some(rune_level) = self
                .state
                .requirements
                .get("runeLevel")
                .copied()
                .and_then(ItemNumber::value)
            else {
                self.status = ItemLoadStatus::SourceError;
                return Err(ItemLoadError("item has no rune level requirement".into()));
            };
            let level = syntax::lua_max(syntax::lua_max(level, natural), rune_level);
            self.state
                .requirements
                .insert("level".into(), ItemNumber::new(level));
        }
        if !self.reconcile_affixes()? {
            return Ok(());
        }
        self.state.variants.finish_legacy();
        if self.num("quality").is_none() && self.base().is_some_and(|b| b.quality().is_some()) {
            self.number("quality", ItemNumber::new(0.0));
        }
        self.assemble(false, provider)
    }
    fn affix_list_mut(&mut self, side: ItemAffixSide) -> &mut ItemAffixList {
        match side {
            ItemAffixSide::Prefix => &mut self.state.prefixes,
            ItemAffixSide::Suffix => &mut self.state.suffixes,
        }
    }
    fn reject_affix(
        &mut self,
        error: AffixError,
        line: Option<usize>,
    ) -> Result<(), ItemLoadError> {
        match error {
            AffixError::Unsupported(message) => {
                self.stop(DependencyKind::CraftedAffixes, line, message)
            }
            AffixError::Source(_) | AffixError::Pattern(PatternError::Source(_)) => {
                self.reject_dependency(error.to_string(), true)
            }
            AffixError::Resource(_) | AffixError::Pattern(PatternError::Resource(_)) => {
                self.reject_dependency(error.to_string(), false)
            }
        }
    }
    fn prepare_affix_programs(&mut self, line: Option<usize>) -> Result<bool, ItemLoadError> {
        if self.affix_programs.is_none() {
            match AffixPrograms::compile(&self.catalog.policy().affix_loading) {
                Ok(programs) => {
                    self.charge(programs.compiled_bytes)?;
                    self.affix_programs = Some(programs);
                }
                Err(error) => {
                    self.reject_affix(error, line)?;
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    /// Original if/elseif ordering is observable when injected patterns overlap
    /// an exact effect, or when a disabled line becomes the empty string.
    fn apply_postparse_effects(&mut self, lower: &str, line: usize) -> Result<bool, ItemLoadError> {
        if self
            .catalog
            .policy()
            .affix_loading
            .preceding_line_effects
            .iter()
            .any(|text| text == lower)
        {
            self.apply_flags_policy("postparse_line_effects", lower);
            return Ok(true);
        }
        if !self.prepare_affix_programs(Some(line))? {
            return Ok(false);
        }
        let default = self.catalog.policy().affix_loading.limit_default;
        let result = self
            .affix_programs
            .as_ref()
            .expect("prepared affix programs")
            .limit_effect(lower, default, &mut self.affix_budget);
        match result {
            Ok(Some((side, positive, negative))) => {
                let list = self.affix_list_mut(side);
                list.limit = Some(ItemNumber::new(
                    list.limit.and_then(ItemNumber::value).unwrap_or(default) + positive - negative,
                ));
            }
            Ok(None) => {
                self.apply_flags_policy("postparse_line_effects", lower);
            }
            Err(error) => {
                self.reject_affix(error, Some(line))?;
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn reconcile_affixes(&mut self) -> Result<bool, ItemLoadError> {
        let catalog = self.catalog;
        let policy = &catalog.policy().affix_loading;
        let rules = &policy.reconcile;
        self.number("affixLimit", ItemNumber::new(rules.initial_limit));
        if !self.flag("crafted") {
            return Ok(true);
        }
        let table = self.txt("affixes_table").map(str::to_owned);
        if table
            .as_deref()
            .and_then(|name| catalog.modifier_table(name))
            .is_none()
        {
            self.boolean("crafted", false);
            return Ok(true);
        }
        let has_limits = self.state.prefixes.limit.is_some() || self.state.suffixes.limit.is_some();
        let (mut limit, side_base, side_max) = if self.state.rarity == rules.magic_rarity {
            (
                rules.magic_limit,
                rules.magic_side_base,
                rules.magic_side_max,
            )
        } else if self.state.rarity == rules.rare_rarity {
            let jewel = if self.state.item_type.as_deref() == Some(&rules.jewel_type) {
                let Some(base) = self.base() else {
                    return self.reject_dependency(
                        "attempt to index absent base during crafted jewel reconciliation".into(),
                        true,
                    );
                };
                !(base.sub_type() == Some(&rules.corrupted_jewel_subtype) && self.flag("corrupted"))
            } else {
                false
            };
            let limit = if jewel {
                rules.rare_jewel_limit
            } else {
                rules.rare_limit
            };
            // Source assigns the rare total before applying either side limit.
            self.number("affixLimit", ItemNumber::new(limit));
            (limit, limit / rules.side_divisor, limit)
        } else {
            self.boolean("crafted", false);
            return Ok(true);
        };
        if has_limits {
            for list in [&mut self.state.prefixes, &mut self.state.suffixes] {
                let value = list
                    .limit
                    .and_then(ItemNumber::value)
                    .unwrap_or(policy.limit_default);
                list.limit = Some(ItemNumber::new(syntax::lua_max(
                    syntax::lua_min(value + side_base, side_max),
                    rules.minimum_limit,
                )));
            }
            limit = self
                .state
                .prefixes
                .limit
                .and_then(ItemNumber::value)
                .expect("assigned prefix limit")
                + self
                    .state
                    .suffixes
                    .limit
                    .and_then(ItemNumber::value)
                    .expect("assigned suffix limit");
        }
        self.number("affixLimit", ItemNumber::new(limit));
        // Original source traverses prefixes before suffixes and retains rows
        // beyond the active count. A pending lookup preserves that exact prefix.
        for side in [ItemAffixSide::Prefix, ItemAffixSide::Suffix] {
            let active = self
                .affix_list_mut(side)
                .limit
                .and_then(ItemNumber::value)
                .unwrap_or(limit / rules.side_divisor);
            // Lua numeric for loops take no iterations for NaN or limits < 1.
            if active.is_nan() || active < 1.0 {
                continue;
            }
            if !active.is_finite() || active.floor() > MAX_ITEM_LOADING_LINES as f64 {
                return Err(ItemLoadError("crafted affix active row bound".into()));
            }
            for index in 0..active.floor() as usize {
                if index >= self.affix_list_mut(side).entries.len() {
                    self.charge(policy.none_mod_id.len() + 128)?;
                    self.affix_list_mut(side).entries.push(ItemAffix {
                        mod_id: policy.none_mod_id.clone(),
                        range: None,
                        fractured: None,
                    });
                    continue;
                }
                let id = &self.affix_list_mut(side).entries[index].mod_id;
                if id == &policy.none_mod_id {
                    continue;
                }
                let replacement = match catalog.affix_lookup(table.as_deref(), id) {
                    ItemAffixLookup::Exact { .. } => continue,
                    ItemAffixLookup::Legacy { mod_id, .. } => mod_id,
                    ItemAffixLookup::Missing => &policy.none_mod_id,
                    ItemAffixLookup::Ambiguous => {
                        self.stop(
                            DependencyKind::CraftedAffixes,
                            None,
                            "legacy affix label has multiple source traversal candidates",
                        )?;
                        return Ok(false);
                    }
                    ItemAffixLookup::Unavailable => {
                        self.stop(
                            DependencyKind::CraftedAffixes,
                            None,
                            "affix fallback traversal or identity is not represented",
                        )?;
                        return Ok(false);
                    }
                };
                self.charge(replacement.len())?;
                self.affix_list_mut(side).entries[index].mod_id = replacement.to_owned();
            }
        }
        Ok(true)
    }
    fn assemble(
        &mut self,
        final_load: bool,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<(), ItemLoadError> {
        self.assembly_final = false;
        self.state.assembly_calls += 1;
        if !self.state.base_present {
            self.status = ItemLoadStatus::NoBase;
            return Ok(());
        }
        let request = AssemblyRequest {
            binding: AssemblyBinding {
                item: Arc::clone(&self.assembly_item),
                attempt: Arc::new(()),
                catalog: self.catalog.clone(),
            },
            final_load,
            reparsed: self.assembly_reparsed,
            previous: self.assembly.clone(),
            state: self.state.clone(),
            armour_header_updates: self.armour_header_updates.clone(),
        };
        self.assembly_final = false;
        let attempt = provider.assemble_with_trace(&request);
        if let Some(prefix) = attempt.prefix {
            if prefix.assembled.as_ref().is_some_and(|item| {
                item.binding()
                    .is_none_or(|b| !b.matches_attempt(request.binding()))
            }) {
                return self.reject_dependency(
                    "assembly returned a foreign owned result as failure prefix".into(),
                    false,
                );
            }
            self.apply_assembly_outcome(prefix)?;
        }
        match attempt.outcome {
            DependencyResult::SourceError(message) => return self.reject_dependency(message, true),
            DependencyResult::ResourceError(message) => {
                // A retained graph may be newer than its diagnostic projection.
                // Stop this machine; retry requires a fresh coherent load.
                self.stop(DependencyKind::Assembly, None, message.clone())?;
                return Err(ItemLoadError(message));
            }
            DependencyResult::Unavailable(message) => {
                self.stop(DependencyKind::Assembly, None, message)?
            }
            DependencyResult::Available(result) => {
                if result.assembled.as_ref().is_some_and(|item| {
                    !item.is_complete()
                        || item
                            .binding()
                            .is_none_or(|b| !b.matches_attempt(request.binding()))
                }) {
                    return self.reject_dependency(
                        "assembly returned an incomplete or foreign owned result as success".into(),
                        false,
                    );
                }
                self.apply_assembly_outcome(result)?;
                self.status = ItemLoadStatus::Complete;
                self.assembly_final = final_load;
                self.assembly_reparsed = false;
                self.armour_header_updates.clear();
            }
        }
        Ok(())
    }
    fn apply_assembly_outcome(&mut self, result: AssemblyOutcome) -> Result<(), ItemLoadError> {
        self.charge(validate_metadata(&result.evidence)?)?;
        if result.state_updates.len() > 4096 {
            return Err(ItemLoadError("assembly state update bound".into()));
        }
        if let Some(payloads) = &result.modifier_payloads {
            let lists = [
                (&payloads.buff_mod_lines, &self.state.buff_mod_lines),
                (&payloads.enchant_mod_lines, &self.state.enchant_mod_lines),
                (&payloads.rune_mod_lines, &self.state.rune_mod_lines),
                (
                    &payloads.class_requirement_mod_lines,
                    &self.state.class_requirement_mod_lines,
                ),
                (&payloads.implicit_mod_lines, &self.state.implicit_mod_lines),
                (&payloads.explicit_mod_lines, &self.state.explicit_mod_lines),
            ];
            if lists.iter().any(|(payloads, rows)| {
                payloads.len() != rows.len() || payloads.iter().any(|row| row.len() > 4096)
            }) {
                return Err(ItemLoadError(
                    "assembly modifier payload row count mismatch or bound".into(),
                ));
            }
            self.charge(validate_metadata_tables(lists.iter().flat_map(
                |(payloads, _)| payloads.iter().flat_map(|row| row.iter()),
            ))?)?;
        }
        if let Some(requirements) = &result.requirements {
            if requirements.len() > 256 {
                return Err(ItemLoadError("assembly requirement count bound".into()));
            }
            for (key, value) in requirements {
                if key.is_empty()
                    || key.len() > 128
                    || key.contains('\0')
                    || *value == ItemNumber::Nil
                    || !value.canonical()
                {
                    return Err(ItemLoadError("invalid assembly requirement entry".into()));
                }
                self.charge(key.len() + 32)?;
            }
        }
        if matches!(result.armour_data, ArmourDataUpdate::NumericSubset(_))
            && result.assembled.is_none()
        {
            let message = "partial armour projection requires an owned assembly graph";
            self.stop(DependencyKind::Assembly, None, message)?;
            return Err(ItemLoadError(message.into()));
        }
        if let ArmourDataUpdate::Replace(data) | ArmourDataUpdate::NumericSubset(data) =
            &result.armour_data
        {
            if data.len() > 256 {
                return Err(ItemLoadError("assembly armour data count bound".into()));
            }
            for (key, value) in data {
                if key.is_empty()
                    || key.len() > 4096
                    || key.contains('\0')
                    || *value == ItemNumber::Nil
                    || !value.canonical()
                {
                    return Err(ItemLoadError("invalid assembly armour data entry".into()));
                }
                self.charge(key.len() + 64)?;
            }
            self.charge(64)?;
        }
        for (key, value) in &result.state_updates {
            if key.len() > 4096
                || matches!(value,ItemScalar::Text(t)if t.len()>MAX_ITEM_LOADING_TEXT)
                || matches!(value,ItemScalar::Number(n)if !n.canonical())
            {
                return Err(ItemLoadError("assembly state text bound".into()));
            }
            self.charge(
                key.len()
                    + match value {
                        ItemScalar::Text(t) => t.len(),
                        _ => 32,
                    },
            )?;
        }
        match result.armour_data {
            ArmourDataUpdate::Preserve => {}
            ArmourDataUpdate::Clear => {
                self.state.armour_data = None;
                self.state.armour_data_complete = true;
            }
            ArmourDataUpdate::Replace(data) => {
                self.state.armour_data = Some(data);
                self.state.armour_data_complete = true;
            }
            ArmourDataUpdate::NumericSubset(data) => {
                self.state.armour_data = Some(data);
                self.state.armour_data_complete = false;
            }
        }
        if let Some(requirements) = result.requirements {
            self.state.requirements = requirements;
        }
        for (key, value) in result.state_updates {
            if matches!(value, ItemScalar::Number(ItemNumber::Nil)) {
                self.state.retained_fields.remove(&key);
            } else {
                self.state.retained_fields.insert(key, value);
            }
        }
        if let Some(payloads) = result.modifier_payloads {
            for (payloads, rows) in [
                (payloads.buff_mod_lines, &mut self.state.buff_mod_lines),
                (
                    payloads.enchant_mod_lines,
                    &mut self.state.enchant_mod_lines,
                ),
                (payloads.rune_mod_lines, &mut self.state.rune_mod_lines),
                (
                    payloads.class_requirement_mod_lines,
                    &mut self.state.class_requirement_mod_lines,
                ),
                (
                    payloads.implicit_mod_lines,
                    &mut self.state.implicit_mod_lines,
                ),
                (
                    payloads.explicit_mod_lines,
                    &mut self.state.explicit_mod_lines,
                ),
            ] {
                for (payload, row) in payloads.into_iter().zip(rows) {
                    row.modifiers = payload;
                }
            }
        }
        self.state.assembly_evidence = Some(result.evidence);
        self.assembly = result.assembled;
        Ok(())
    }
    pub fn apply_mod_range(
        &mut self,
        id: Option<&str>,
        range: Option<&str>,
    ) -> Result<(), ItemLoadError> {
        if matches!(
            self.status,
            ItemLoadStatus::Pending | ItemLoadStatus::SourceError
        ) {
            return Ok(());
        }
        self.assembly_final = false;
        let mut id = id
            .map_or(ItemNumber::Nil, syntax::lua_number)
            .value()
            .unwrap_or(0.0);
        let range = range.map_or(ItemNumber::Nil, syntax::lua_number);
        let range = if range == ItemNumber::Nil {
            ItemNumber::new(1.0)
        } else {
            range
        };
        for list in [
            &mut self.state.buff_mod_lines,
            &mut self.state.enchant_mod_lines,
            &mut self.state.implicit_mod_lines,
            &mut self.state.explicit_mod_lines,
        ] {
            if id <= list.len() as f64 {
                if !id.is_finite() || id < 1.0 || id.fract() != 0.0 {
                    self.status = ItemLoadStatus::SourceError;
                    return Err(ItemLoadError(
                        "ModRange indexes an absent source modifier row".into(),
                    ));
                }
                list[id as usize - 1].range = range;
                return Ok(());
            }
            id -= list.len() as f64;
        }
        Ok(())
    }
    pub fn finish_load(
        &mut self,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<(), ItemLoadError> {
        if matches!(
            self.status,
            ItemLoadStatus::Pending | ItemLoadStatus::SourceError
        ) {
            return Ok(());
        }
        self.assembly_final = false;
        if self.state.base_present
            && self.state.jewel_socket_count == 0
            && let Some(n) = self
                .txt("title")
                .and_then(|title| {
                    self.compat("fallback_jewel_socket_counts")
                        .and_then(|t| t.fields.get(title))
                })
                .and_then(ItemMetadataValue::as_f64)
        {
            if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > MAX_ITEM_LOADING_LINES as f64 {
                return Err(ItemLoadError(
                    "fallback jewel socket count exceeds bound".into(),
                ));
            }
            self.state.jewel_socket_count = n as usize;
        }
        if self.state.base_present {
            self.assemble(true, provider)?;
            if self.status == ItemLoadStatus::Complete && self.num("id").is_none_or(f64::is_nan) {
                self.status = ItemLoadStatus::SourceError;
                return Err(ItemLoadError(
                    "source inventory insertion has nil or NaN item id".into(),
                ));
            }
        }
        Ok(())
    }
}
#[derive(Debug)]
enum Header {
    Known,
    Unknown,
    Skip,
    SkipNext,
    Implicits(f64),
    Stop,
}
fn number_option(n: ItemNumber) -> Option<ItemNumber> {
    if n == ItemNumber::Nil { None } else { Some(n) }
}
fn alternate_index(field: &str, prefix: &str) -> Option<usize> {
    let suffix = field.strip_prefix(prefix)?;
    if suffix.is_empty() {
        Some(0)
    } else {
        suffix
            .parse::<usize>()
            .ok()
            .filter(|&v| (2..=5).contains(&v))
            .map(|v| v - 1)
    }
}
fn is_magnitude_line(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    line.contains("modifier magnitudes")
        || line.contains("effect of suffixes")
        || line.contains("effect of prefixes")
}
struct AnnotatedLine {
    text: String,
    flags: BTreeSet<String>,
    tags: Vec<String>,
    range: ItemNumber,
    corrupted_range: ItemNumber,
    selection: LineSelection,
    has_range_tag: bool,
}
fn annotations(
    line: &str,
    catalog: &ItemLoadingCatalog,
    preselected: &LineSelection,
) -> Result<AnnotatedLine, &'static str> {
    fn ids(text: &str, positive: bool) -> Result<BTreeSet<u32>, &'static str> {
        for part in text
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
        {
            if part.parse::<u32>().is_err() {
                return Err("modifier selection tag exceeds native index bound");
            }
        }
        Ok(syntax::ids(text, positive))
    }
    let mut selection = LineSelection::default();
    let mut flags = BTreeSet::new();
    let mut tags = Vec::new();
    let mut range = ItemNumber::Nil;
    let mut has_range_tag = false;
    let mut corrupted = ItemNumber::Nil;
    let mut clean = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('{') {
        clean.push_str(&rest[..start]);
        let Some(end) = rest[start + 1..].find('}').map(|n| start + 1 + n) else {
            clean.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let inside = &rest[start + 1..end];
        let split = inside.bytes().take_while(u8::is_ascii_alphabetic).count();
        let key = &inside[..split];
        let value = inside[split..]
            .strip_prefix(':')
            .unwrap_or(&inside[split..]);
        match key {
            "variant" => {
                selection.variants = Some(if let Some(v) = &preselected.variants {
                    v.clone()
                } else {
                    ids(value, false)?
                })
            }
            "version" => {
                selection.versions = Some(if let Some(v) = &preselected.versions {
                    v.clone()
                } else {
                    ids(value, false)?
                })
            }
            "group" => {
                selection.groups = Some(if let Some(v) = &preselected.groups {
                    v.clone()
                } else {
                    ids(value, true)?
                })
            }
            "tags" => tags.extend(
                value
                    .split(|c: char| !c.is_ascii_alphabetic() && c != '_')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            ),
            "range" => {
                range = syntax::lua_number(value);
                has_range_tag = true;
            }
            "corruptedRange" => corrupted = syntax::lua_number(value),
            _ => {
                if catalog.policy().line_flags.contains(key) {
                    flags.insert(key.into());
                }
            }
        }
        rest = &rest[end + 1..];
    }
    clean.push_str(rest);
    let mut at = 0;
    while let Some(start) = clean[at..].find(" (").map(|n| at + n) {
        let Some(end) = clean[start + 2..].find(')').map(|n| start + 2 + n) else {
            break;
        };
        let flag = &clean[start + 2..end];
        if !flag.is_empty() && flag.bytes().all(|b| b.is_ascii_lowercase()) {
            if catalog.policy().line_flags.contains(flag) {
                flags.insert(flag.into());
            }
            clean.replace_range(start..end + 1, "");
            at = start;
        } else {
            at = end + 1;
        }
    }
    Ok(AnnotatedLine {
        text: clean,
        flags,
        tags,
        range,
        corrupted_range: corrupted,
        selection,
        has_range_tag,
    })
}
fn validate_metadata(table: &ItemMetadataTable) -> Result<usize, ItemLoadError> {
    validate_metadata_tables(std::iter::once(table))
}
fn validate_metadata_tables<'a>(
    tables: impl Iterator<Item = &'a ItemMetadataTable>,
) -> Result<usize, ItemLoadError> {
    struct Budget {
        nodes: usize,
        bytes: usize,
    }
    fn text(s: &str, budget: &mut Budget) -> Result<(), ItemLoadError> {
        budget.bytes = budget
            .bytes
            .checked_add(s.len())
            .ok_or_else(|| ItemLoadError("dependency text overflow".into()))?;
        if s.len() > MAX_ITEM_LOADING_TEXT || budget.bytes > 4 * MAX_ITEM_LOADING_TEXT {
            return Err(ItemLoadError("dependency text bound".into()));
        }
        Ok(())
    }
    fn table(
        t: &ItemMetadataTable,
        depth: usize,
        budget: &mut Budget,
    ) -> Result<(), ItemLoadError> {
        if depth > 24 {
            return Err(ItemLoadError("dependency metadata depth bound".into()));
        }
        // Empty tables still occupy retained storage, including top-level
        // modifier tables returned by parser and assembly dependencies.
        budget.nodes += 1;
        if budget.nodes > 65536 {
            return Err(ItemLoadError("dependency metadata value bound".into()));
        }
        for (k, v) in &t.fields {
            text(k, budget)?;
            visit(v, depth + 1, budget)?;
        }
        for v in t.indexed.values() {
            visit(v, depth + 1, budget)?;
        }
        Ok(())
    }
    fn visit(
        v: &ItemMetadataValue,
        depth: usize,
        budget: &mut Budget,
    ) -> Result<(), ItemLoadError> {
        budget.nodes += 1;
        if depth > 24 || budget.nodes > 65536 {
            return Err(ItemLoadError("dependency metadata value bound".into()));
        }
        match v {
            ItemMetadataValue::Number(n) if !n.is_finite() => {
                return Err(ItemLoadError(
                    "nonfinite dependency modifier metadata".into(),
                ));
            }
            ItemMetadataValue::Text(s) => text(s, budget)?,
            ItemMetadataValue::Array(a) => {
                for v in a {
                    visit(v, depth + 1, budget)?;
                }
            }
            ItemMetadataValue::Table(t) => table(t, depth, budget)?,
            ItemMetadataValue::Callback(function) => {
                let span = &function.callback;
                if span.path.len() > 4096 || span.sha256.len() > 64 {
                    return Err(ItemLoadError("dependency callback descriptor bound".into()));
                }
                text(&span.path, budget)?;
                text(&span.sha256, budget)?;
            }
            _ => {}
        }
        Ok(())
    }
    let mut budget = Budget { nodes: 0, bytes: 0 };
    for t in tables {
        table(t, 0, &mut budget)?;
    }
    Ok(budget.bytes + budget.nodes * 64)
}

fn has_advanced_copy_numeric_or_enum(line: &str) -> bool {
    // Lua's %b() matches balanced pairs. Track all pairs in one pass so nested
    // annotations cannot accidentally pass through to the modifier parser.
    let bytes = line.as_bytes();
    let mut openings = Vec::new();
    let mut digits = 0usize;
    let mut hyphens = 0usize;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte.is_ascii_digit() {
            digits += 1;
        } else if byte == b'-' {
            hyphens += 1;
        } else if byte == b'(' {
            let numeric_prefix =
                index > 0 && (bytes[index - 1].is_ascii_digit() || bytes[index - 1] == b'.');
            openings.push((numeric_prefix, digits, hyphens));
        } else if byte == b')'
            && let Some((numeric_prefix, prior_digits, prior_hyphens)) = openings.pop()
            && ((digits == prior_digits && hyphens != prior_hyphens)
                || (numeric_prefix && digits != prior_digits))
        {
            return true;
        }
    }
    false
}

// Item.lua baseHasImplicitLine: source text outside replaced ranges remains a
// Lua pattern. Exact equality short-circuits even malformed pattern text.
fn base_has_implicit_line(
    implicit: &str,
    line: &str,
    budget: &mut MatchBudget,
) -> Result<bool, PatternError> {
    let mut range_pattern = None;
    // gmatch("[^\n]+") skips empty spans but preserves carriage returns.
    for original in implicit.split('\n') {
        budget.charge(original.len() as u64 + 1)?;
        if !original.starts_with("Grants Skill:") {
            continue;
        }
        if original == line {
            return Ok(true);
        }
        let ranges = match &range_pattern {
            Some(pattern) => pattern,
            None => range_pattern.insert(LuaPattern::compile(b"%(%d+%-%d+%)")?),
        };
        let transformed = ranges.gsub(
            original.as_bytes(),
            b"%%d+",
            None,
            budget,
            GsubLimits {
                max_replacement_bytes: 4,
                max_output_bytes: 64 * 1024 - 2,
            },
        )?;
        // Output and compilation are bounded before allocating their buffers.
        let mut pattern = Vec::with_capacity(transformed.bytes.len() + 2);
        pattern.push(b'^');
        pattern.extend_from_slice(&transformed.bytes);
        pattern.push(b'$');
        let pattern = LuaPattern::compile(&pattern)?;
        budget.charge(pattern.compiled_bytes() as u64)?;
        if pattern
            .match_captures(line.as_bytes(), 1, budget)?
            .is_some()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod base_implicit_tests {
    use super::*;
    #[test]
    fn base_implicit_ranges_and_unescaped_source_patterns() {
        for (definition, line, expected) in [
            (
                "Grants Skill: Level (1-20) A",
                "Grants Skill: Level 18 A",
                true,
            ),
            (
                "Grants Skill: Level (1-20) A",
                "Grants Skill: Level 999 A",
                true,
            ),
            (
                "Grants Skill: Level (1-20) A",
                "Grants Skill: Level -1 A",
                false,
            ),
            (
                "Grants Skill: (1-20) and (3-7)",
                "Grants Skill: 2 and 9",
                true,
            ),
            ("Grants Skill: A.B", "Grants Skill: AxB", true),
            ("Grants Skill: A%.B", "Grants Skill: A.B", true),
            ("Grants Skill: (A)%1", "Grants Skill: AA", true),
            ("Grants Skill: [AB]+", "Grants Skill: ABBA", true),
            ("Grants Skill: A", "prefix Grants Skill: A", false),
            ("Grants Skill: A", "Grants Skill: AB", false),
            ("Other\n\nGrants Skill: (1-2)", "Grants Skill: 7", true),
            ("Grants Skill: A\r\n", "Grants Skill: A", false),
            ("Grants Skill: A\r\n", "Grants Skill: A\r", true),
            ("Grants Skill: A\0ignored", "Grants Skill: A", true),
            ("Grants Skill: [", "Grants Skill: [", true),
            ("Grants Skill: A\nGrants Skill: [", "Grants Skill: A", true),
            ("Grants Skill: [", "Other", false),
            ("Other [", "Other", false),
        ] {
            assert_eq!(
                base_has_implicit_line(definition, line, &mut MatchBudget::default()).unwrap(),
                expected,
                "{definition:?} versus {line:?}"
            );
        }
        assert!(matches!(
            base_has_implicit_line(
                "Grants Skill: [",
                "Grants Skill: A",
                &mut MatchBudget::default()
            ),
            Err(PatternError::Source(_))
        ));
    }
    #[test]
    fn base_implicit_pattern_work_is_cumulative_and_bounded() {
        let limits = poe_optimizer_engine::lua_pattern::MatchLimits {
            max_steps: 256,
            ..Default::default()
        };
        assert!(matches!(
            base_has_implicit_line(&"\n".repeat(300), "unused", &mut MatchBudget::new(limits)),
            Err(PatternError::Resource(_))
        ));
        let mut budget = MatchBudget::new(limits);
        assert!(base_has_implicit_line("Grants Skill: A", "Grants Skill: A", &mut budget).unwrap());
        for _ in 0..100 {
            if let Err(error) =
                base_has_implicit_line("Grants Skill: A", "Grants Skill: A", &mut budget)
            {
                assert!(matches!(error, PatternError::Resource(_)));
                return;
            }
        }
        panic!("repeated matching must consume the same invocation work budget");
    }
}
