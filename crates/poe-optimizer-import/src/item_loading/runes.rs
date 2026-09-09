//! Ordered socketed-augment loading. Catalog membership is not socket legality.
//! Ambiguous source traversal remains an explicit dependency, never a tie-break.
use super::*;
use poe_optimizer_data::item_loading::{ItemRuneLoadingPolicy, ItemRuneRecord};
use poe_optimizer_engine::item_runes::{
    self, RuneBudget, RuneError, RuneLineParts, RuneText, VectorPolicy,
};
use poe_optimizer_engine::lua_pattern::{Capture, CompileLimits, LuaPattern};

pub(super) struct RunePrograms {
    text: RuneText,
    skip: LuaPattern,
    override_type: LuaPattern,
    soul_type: LuaPattern,
    strip: LuaPattern,
    socket_char: LuaPattern,
    socket_item: LuaPattern,
    socket_jewel: LuaPattern,
    matching: MatchBudget,
    budget: RuneBudget,
    compiled_bytes: usize,
}
#[derive(Debug)]
enum Failure {
    Source(String),
    Unsupported(String),
    Resource(String),
    Load(ItemLoadError),
    Stopped,
}
type Result<T> = std::result::Result<T, Failure>;
type RuneHints = (Option<String>, Option<String>);
impl From<PatternError> for Failure {
    fn from(error: PatternError) -> Self {
        match error {
            PatternError::Source(_) => Self::Source(error.to_string()),
            _ => Self::Resource(error.to_string()),
        }
    }
}
impl From<RuneError> for Failure {
    fn from(error: RuneError) -> Self {
        match error {
            RuneError::Pattern(value) => value.into(),
            RuneError::Source(_) => Self::Source(error.to_string()),
            RuneError::Resource(_) => Self::Resource(error.to_string()),
        }
    }
}
impl From<ItemLoadError> for Failure {
    fn from(error: ItemLoadError) -> Self {
        Self::Load(error)
    }
}
fn source(message: &str) -> Failure {
    Failure::Source(message.into())
}
fn unsupported(message: &str) -> Failure {
    Failure::Unsupported(message.into())
}
fn truthy(value: Option<&ItemMetadataValue>) -> bool {
    value.is_some_and(|v| !matches!(v, ItemMetadataValue::Boolean(false)))
}
fn string(value: &ItemMetadataValue) -> Result<&str> {
    value
        .as_str()
        .ok_or_else(|| unsupported("rune text has an unrepresented source type"))
}
fn utf8(value: Vec<u8>) -> Result<String> {
    String::from_utf8(value).map_err(|_| unsupported("rune transformation produced non-UTF-8 text"))
}
fn capture(pattern: &LuaPattern, text: &str, budget: &mut MatchBudget) -> Result<Option<String>> {
    let Some(found) = pattern.match_captures(text.as_bytes(), 1, budget)? else {
        return Ok(None);
    };
    match found.captures().first() {
        Some(Capture::Bytes { start, end }) => Ok(Some(
            text.get(*start..*end)
                .ok_or_else(|| unsupported("rune hint capture is not UTF-8"))?
                .into(),
        )),
        Some(Capture::Position(_)) => Err(unsupported(
            "numeric rune context capture is not represented",
        )),
        None => Ok(Some(
            text.get(found.range())
                .ok_or_else(|| unsupported("rune hint match is not UTF-8"))?
                .to_owned(),
        )),
    }
}
/// Empty replacement gsub, including its zero-width and anchored behavior.
fn strip(pattern: &LuaPattern, text: &[u8], budget: &mut MatchBudget) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut copied = 0;
    while let Some(found) = pattern.match_captures(text, (copied + 1) as i32, budget)? {
        let span = found.range();
        out.extend_from_slice(&text[copied..span.start]);
        copied = span.end;
        if span.is_empty() {
            if copied == text.len() {
                break;
            }
            out.push(text[copied]);
            copied += 1;
        }
        if pattern.source().first() == Some(&b'^') {
            break;
        }
    }
    out.extend_from_slice(&text[copied..]);
    Ok(out)
}
impl RunePrograms {
    fn compile(policy: &ItemRuneLoadingPolicy) -> Result<Self> {
        let text = RuneText::compile(policy.numeric_pattern.as_bytes(), CompileLimits::default())?;
        let mut bytes = text.compiled_bytes();
        let mut compile = |text: &str| -> Result<LuaPattern> {
            let pattern = LuaPattern::compile(text.as_bytes())?;
            bytes = bytes
                .checked_add(pattern.compiled_bytes())
                .filter(|n| *n <= 4 * 1024 * 1024)
                .ok_or_else(|| Failure::Resource("rune compiled pattern aggregate bound".into()))?;
            Ok(pattern)
        };
        let reservations = policy
            .other_header_patterns
            .iter()
            .map(|s| compile(s))
            .collect::<Result<Vec<_>>>()?;
        let mut preflight = MatchBudget::default();
        for name in [&policy.rune_header, &policy.socket_header] {
            for pattern in &reservations {
                match pattern.match_captures(name.as_bytes(), 1, &mut preflight) {
                    Ok(None) => {}
                    Ok(Some(_)) => {
                        return Err(unsupported(
                            "rune header overlaps reserved source header pattern",
                        ));
                    }
                    Err(PatternError::Source(_)) => {
                        return Err(unsupported(
                            "reserved rune header pattern cannot be validated",
                        ));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        let skip = compile(&policy.bonded_skip_pattern)?;
        let override_type = compile(&policy.augment_override_pattern)?;
        let soul_type = compile(&policy.soul_core_pattern)?;
        let strip = compile(&policy.combined_parse_strip_pattern)?;
        // gmatch treats a leading caret literally rather than anchoring it.
        let socket_pattern = if policy.socket_character_pattern.starts_with('^') {
            format!("%{}", policy.socket_character_pattern)
        } else {
            policy.socket_character_pattern.clone()
        };
        let socket_char = compile(&socket_pattern)?;
        let socket_item = compile(&policy.item_socket_pattern)?;
        let socket_jewel = compile(&policy.jewel_socket_pattern)?;
        Ok(Self {
            text,
            skip,
            override_type,
            soul_type,
            strip,
            socket_char,
            socket_item,
            socket_jewel,
            matching: MatchBudget::default(),
            budget: RuneBudget::default(),
            compiled_bytes: bytes,
        })
    }
    fn parts(&mut self, policy: &ItemRuneLoadingPolicy, line: &str) -> Result<RuneLineParts> {
        Ok(self.text.line_parts(
            line.as_bytes(),
            policy.stripped_marker.as_bytes(),
            policy.no_number_value,
            &mut self.budget,
        )?)
    }
}
#[derive(Clone)]
struct SlotContext {
    broad: Option<String>,
    specific: String,
}
#[derive(Clone, Debug)]
struct GroupedRune {
    name: String,
    augment_type: String,
    values: Vec<f64>,
    effect_applied: bool,
}
impl ItemLoadMachine<'_> {
    fn rune_failure<T>(
        &mut self,
        result: Result<T>,
        line: Option<usize>,
    ) -> std::result::Result<Option<T>, ItemLoadError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(Failure::Stopped) => Ok(None),
            Err(Failure::Load(error)) => Err(error),
            Err(Failure::Source(message)) => self.reject_dependency(message, true),
            Err(Failure::Resource(message)) => self.reject_dependency(message, false),
            Err(Failure::Unsupported(message)) => {
                self.stop(DependencyKind::RuneReconstruction, line, message)?;
                Ok(None)
            }
        }
    }
    fn prepare_runes(&mut self) -> Result<()> {
        if self.rune_programs.is_none() {
            let programs = RunePrograms::compile(&self.catalog.policy().rune_loading)?;
            self.charge(programs.compiled_bytes)?;
            self.rune_programs = Some(programs);
        }
        Ok(())
    }
    pub(super) fn prepare_rune_header(
        &mut self,
        line: usize,
    ) -> std::result::Result<bool, ItemLoadError> {
        let result = self.prepare_runes();
        Ok(self.rune_failure(result, Some(line))?.is_some())
    }
    pub(super) fn skip_bonded_rune_line(
        &mut self,
        text: &str,
        line: usize,
    ) -> std::result::Result<Option<bool>, ItemLoadError> {
        let result = (|| {
            self.prepare_runes()?;
            let p = self.rune_programs.as_mut().expect("prepared rune programs");
            Ok(p.skip
                .match_captures(text.as_bytes(), 1, &mut p.matching)?
                .is_some())
        })();
        self.rune_failure(result, Some(line))
    }
    pub(super) fn rune_line_hints(
        &mut self,
        text: &str,
        line: usize,
    ) -> std::result::Result<Option<RuneHints>, ItemLoadError> {
        let result = (|| {
            self.prepare_runes()?;
            let p = self.rune_programs.as_mut().expect("prepared rune programs");
            Ok((
                capture(&p.override_type, text, &mut p.matching)?,
                capture(&p.soul_type, text, &mut p.matching)?,
            ))
        })();
        self.rune_failure(result, Some(line))
    }
    pub(super) fn rune_sockets(
        &mut self,
        text: &str,
        line: usize,
    ) -> std::result::Result<bool, ItemLoadError> {
        let result = (|| {
            self.prepare_runes()?;
            let p = self.rune_programs.as_mut().expect("prepared rune programs");
            let mut cursor = 0;
            let mut last_end = None;
            let mut group = 0;
            while cursor <= text.len() {
                let Some(found) = p.socket_char.match_captures(
                    text.as_bytes(),
                    (cursor + 1) as i32,
                    &mut p.matching,
                )?
                else {
                    break;
                };
                let span = found.range();
                if last_end == Some(span.end) {
                    cursor = span.start + 1;
                    continue;
                }
                last_end = Some(span.end);
                cursor = span.end;
                let value = match found.captures().first() {
                    Some(Capture::Bytes { start, end }) => &text.as_bytes()[*start..*end],
                    Some(Capture::Position(_)) => {
                        return Err(source("attempt to index numeric socket capture"));
                    }
                    None => &text.as_bytes()[span],
                };
                if p.socket_item
                    .match_captures(value, 1, &mut p.matching)?
                    .is_some()
                {
                    if self.state.sockets.len() >= MAX_ITEM_LOADING_LINES {
                        return Err(Failure::Resource("socket count bound".into()));
                    }
                    self.state.sockets.push(group);
                    group += 1;
                } else if p
                    .socket_jewel
                    .match_captures(value, 1, &mut p.matching)?
                    .is_some()
                {
                    self.state.jewel_socket_count += 1;
                }
            }
            self.state.item_socket_count = self.state.sockets.len();
            Ok(())
        })();
        Ok(self.rune_failure(result, Some(line))?.is_some())
    }
    fn rune_table(&self) -> Result<&ItemMetadataTable> {
        self.catalog
            .runes()
            .map(|runes| runes.table())
            .ok_or_else(|| unsupported("selected rune definition family is unavailable"))
    }
    fn rune_eligible(&self) -> Result<bool> {
        let Some(base) = self.base() else {
            return Ok(false);
        };
        if truthy(base.field("weapon")) || truthy(base.field("armour")) {
            return Ok(true);
        }
        let tags = base
            .field("tags")
            .and_then(ItemRuneRecord::new)
            .ok_or_else(|| {
                unsupported("socket eligibility base-tag indexing type is not represented")
            })?;
        Ok(self
            .catalog
            .policy()
            .rune_loading
            .caster_tags
            .iter()
            .any(|tag| truthy(tags.field(tag)))
            || self.state.item_socket_count > 0)
    }
    fn rune_context(&self) -> Result<SlotContext> {
        let base = self
            .base()
            .ok_or_else(|| source("socket type lookup indexes absent base"))?;
        let policy = &self.catalog.policy().rune_loading;
        let sub_type = match base.field("subType").filter(|v| truthy(Some(v))) {
            Some(value) => Some(string(value)?.to_ascii_lowercase()),
            None => None,
        };
        let item_type = string(
            base.field("type")
                .ok_or_else(|| source("base type is absent"))?,
        )?
        .to_ascii_lowercase();
        let broad = if truthy(base.field("weapon")) {
            Some(policy.broad_weapon_type.clone())
        } else if truthy(base.field("armour")) {
            Some(policy.broad_armour_type.clone())
        } else {
            let tags = base
                .field("tags")
                .and_then(ItemRuneRecord::new)
                .ok_or_else(|| {
                    unsupported("socket context base-tag indexing type is not represented")
                })?;
            policy
                .caster_tags
                .iter()
                .any(|tag| truthy(tags.field(tag)))
                .then(|| policy.broad_caster_type.clone())
        };
        let specific = policy
            .specific_type_rewrites
            .iter()
            .find(|rule| {
                rule.item_type.as_deref().is_none_or(|t| t == item_type)
                    && sub_type.as_deref() == Some(rule.sub_type.as_str())
            })
            .map_or(item_type, |r| r.to.clone());
        if let Some(value) = self.txt("socketedAugmentTypeOverride") {
            return Ok(SlotContext {
                broad: Some(policy.override_broad_type.clone()),
                specific: value.into(),
            });
        }
        Ok(SlotContext { broad, specific })
    }
    fn rune_parts(&mut self, line: &str) -> Result<RuneLineParts> {
        self.prepare_runes()?;
        self.rune_programs
            .as_mut()
            .expect("prepared rune programs")
            .parts(&self.catalog.policy().rune_loading, line)
    }

    pub(super) fn finish_runes(
        &mut self,
        game: bool,
        provider: &mut impl ItemLoadProvider,
    ) -> std::result::Result<bool, ItemLoadError> {
        let result = self.run_runes(game, provider);
        Ok(self.rune_failure(result, None)?.is_some())
    }
    fn run_runes(&mut self, game: bool, provider: &mut impl ItemLoadProvider) -> Result<()> {
        if !self.state.base_present {
            return Ok(());
        }
        if !self.rune_eligible()? {
            self.state.sockets.clear();
            self.state.item_socket_count = 0;
            self.state.runes.clear();
            return Ok(());
        }
        self.prepare_runes()?;
        let should_fix = self.state.runes.is_empty();
        let table = self.rune_table()?;
        let can_rebuild = !should_fix
            && self.state.runes.iter().all(|name| {
                name == &self.catalog.policy().rune_loading.none_rune_id
                    || truthy(table.fields.get(name))
            });
        if can_rebuild {
            let mut disabled = BTreeMap::<Vec<u8>, usize>::new();
            for index in 0..self.state.rune_mod_lines.len() {
                let row = &self.state.rune_mod_lines[index];
                if row.flags.contains("disabled") {
                    let text = row.line.clone();
                    *disabled
                        .entry(self.rune_parts(&text)?.stripped)
                        .or_default() += 1;
                }
            }
            self.update_runes(provider)?;
            for index in 0..self.state.rune_mod_lines.len() {
                let text = self.state.rune_mod_lines[index].line.clone();
                let parts = self.rune_parts(&text)?;
                if let Some(count) = disabled.get_mut(&parts.stripped)
                    && *count > 0
                {
                    self.state.rune_mod_lines[index]
                        .flags
                        .insert("disabled".into());
                    *count -= 1;
                }
            }
        }
        // GAME re-ranging has its own parser/formatter trace. Stop only when
        // a source modifier actually activates that still-unrepresented step.
        let mode = if game { "GAME" } else { "WIKI" };
        if mode == self.catalog.policy().rune_loading.game_mode && should_fix {
            let policy = &self.catalog.policy().rune_loading;
            let effect_names = [
                policy.effect_global_name.clone(),
                format!(
                    "{}{}{}",
                    policy.effect_name_prefix, policy.rune_augment_type, policy.effect_name_suffix
                ),
                format!(
                    "{}{}{}",
                    policy.effect_name_prefix,
                    policy.extra_slot_augment_type,
                    policy.effect_name_suffix
                ),
            ];
            if [
                &self.state.enchant_mod_lines,
                &self.state.implicit_mod_lines,
                &self.state.explicit_mod_lines,
            ]
            .iter()
            .any(|rows| {
                rows.iter().any(|row| {
                    row.modifiers.iter().any(|m| {
                        m.fields.get("type").and_then(ItemMetadataValue::as_str)
                            == Some(policy.effect_mod_type.as_str())
                            && m.fields
                                .get("name")
                                .and_then(ItemMetadataValue::as_str)
                                .is_some_and(|s| effect_names.iter().any(|n| n == s))
                    })
                })
            }) {
                return Err(unsupported(
                    "rune inference requires ranged augment-effect modifier evaluation",
                ));
            }
        }
        let context = self.rune_context()?;
        let groups = self.rune_groups(&context)?;
        self.annotate_runes(&groups, should_fix)?;
        if should_fix && !self.state.runes.is_empty() {
            self.update_runes(provider)?
        }
        Ok(())
    }
    fn parse_rune(
        &mut self,
        text: &str,
        origin: RuneContribution,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<ParseOutcome> {
        if self.state.parser_calls.len() >= MAX_ITEM_LOADING_CALLS {
            return Err(Failure::Resource("rune parser request bound".into()));
        }
        self.charge(text.len() + origin.slot_key.len())?;
        let request = ParseRequest {
            sequence: self.state.format_calls.len()
                + self.state.parser_calls.len()
                + self.state.format_parser_calls.len(),
            line_index: None,
            origin: Some(origin),
            text: text.into(),
            combined: false,
        };
        let result = provider.parse_modifier(&request);
        self.state.parser_calls.push(request);
        match result {
            DependencyResult::Available(outcome) => {
                self.charge_parse_outcome(&outcome)?;
                Ok(outcome)
            }
            DependencyResult::Unavailable(message) => {
                self.stop(DependencyKind::ModifierParser, None, message)?;
                Err(Failure::Stopped)
            }
            DependencyResult::SourceError(message) => Err(Failure::Source(message)),
            DependencyResult::ResourceError(message) => Err(Failure::Resource(message)),
        }
    }
    fn update_runes(&mut self, provider: &mut impl ItemLoadProvider) -> Result<()> {
        if let Some(level) = self
            .state
            .requirements
            .get("naturalLevel")
            .copied()
            .filter(|v| *v != ItemNumber::Nil)
        {
            self.state.requirements.insert("level".into(), level);
        }
        self.state.rune_mod_lines.clear();
        let mut order_rows = BTreeMap::<Vec<u8>, usize>::new();
        let context = self.rune_context()?;
        let catalog = self.catalog;
        for socket in 0..self.state.item_socket_count {
            let Some(name) = self.state.runes.get(socket) else {
                continue;
            };
            if name == &catalog.policy().rune_loading.none_rune_id {
                continue;
            }
            let table = catalog
                .runes()
                .ok_or_else(|| unsupported("rune definition family is unavailable"))?
                .table();
            let Some(value) = table.fields.get(name) else {
                continue;
            };
            if !truthy(Some(value)) {
                continue;
            }
            let rune = ItemRuneRecord::new(value)
                .ok_or_else(|| unsupported("rune definition indexing type is not represented"))?;
            let policy = &catalog.policy().rune_loading;
            let mut gathered = Vec::new();
            for key in [context.broad.as_deref(), Some(context.specific.as_str())]
                .into_iter()
                .flatten()
            {
                if let Some(value) = rune.field(key).filter(|v| truthy(Some(v))) {
                    gathered.push((key.to_owned(), value));
                }
            }
            let extras: Vec<_> = self
                .state
                .socketed_soul_core_types
                .iter()
                .filter_map(|key| {
                    rune.field(key)
                        .filter(|v| truthy(Some(v)))
                        .map(|value| (key.clone(), value))
                })
                .collect();
            if extras.len() > 1 {
                return Err(unsupported(
                    "extra SoulCore slot contribution order is not represented",
                ));
            }
            for (key, value) in extras {
                let record = ItemRuneRecord::new(value).ok_or_else(|| {
                    unsupported("extra SoulCore slot indexing type is not represented")
                })?;
                if record.field("type").and_then(ItemMetadataValue::as_str)
                    == Some(policy.extra_slot_augment_type.as_str())
                {
                    gathered.push((key, value));
                }
            }
            for (slot, value) in gathered {
                let record = ItemRuneRecord::new(value)
                    .ok_or_else(|| source("rune slot is not iterable"))?;
                self.add_rune_record(record, socket + 1, &slot, &mut order_rows, provider)?;
            }
        }
        Ok(())
    }

    fn add_rune_record(
        &mut self,
        record: ItemRuneRecord<'_>,
        socket: usize,
        slot: &str,
        orders: &mut BTreeMap<Vec<u8>, usize>,
        provider: &mut impl ItemLoadProvider,
    ) -> Result<()> {
        for bonded in [false, true] {
            let rows = if bonded {
                let Some(value) = record.field("bonded").filter(|v| truthy(Some(v))) else {
                    continue;
                };
                ItemRuneRecord::new(value)
                    .ok_or_else(|| source("Bonded rune rows are not iterable"))?
            } else {
                record
            };
            for (index, value) in rows.dense_prefix().enumerate() {
                let line = string(value)?;
                let order = if let Some(value) = rows.field("statOrder").filter(|v| truthy(Some(v)))
                {
                    ItemRuneRecord::new(value)
                        .ok_or_else(|| source("rune stat order is not indexable"))?
                        .indexed((index + 1) as i64)
                } else {
                    None
                };
                let policy = &self.catalog.policy().rune_loading;
                let order = if truthy(order) {
                    order.and_then(ItemMetadataValue::as_f64).ok_or_else(|| {
                        unsupported("nonnumeric rune stat ordering is not represented")
                    })?
                } else {
                    policy.order_default
                };
                let augment = record
                    .field("type")
                    .ok_or_else(|| source("rune type is absent during concatenation"))?;
                let augment = match augment {
                    ItemMetadataValue::Text(text) => text.clone(),
                    ItemMetadataValue::Number(_) => {
                        return Err(unsupported("numeric rune augment state is not represented"));
                    }
                    _ => return Err(source("rune type cannot be concatenated")),
                };
                let prefix = format!(
                    "{}{}{}",
                    augment,
                    policy.order_separator,
                    if bonded {
                        policy.bonded_order_marker.as_str()
                    } else {
                        ""
                    }
                );
                let p = self.rune_programs.as_mut().expect("prepared rune programs");
                let key = item_runes::number_order_key(prefix.as_bytes(), order, &mut p.budget)?;
                let display = if bonded {
                    format!("{}{}", policy.bonded_display_prefix, line)
                } else {
                    line.to_owned()
                };
                self.charge(display.len() + key.len() + slot.len())?;
                let combined = orders.contains_key(&key);
                let origin = RuneContribution {
                    socket_index: socket,
                    slot_key: slot.into(),
                    bonded,
                    definition_line_index: index + 1,
                    combined,
                };
                if let Some(&at) = orders.get(&key) {
                    let p = self.rune_programs.as_mut().expect("prepared rune programs");
                    let text = utf8(p.text.combine(
                        self.state.rune_mod_lines[at].line.as_bytes(),
                        display.as_bytes(),
                        &mut p.budget,
                    )?)?;
                    self.charge(text.len())?;
                    self.state.rune_mod_lines[at].line = text;
                    self.state.rune_mod_lines[at]
                        .rune_origins
                        .push(origin.clone());
                    let p = self.rune_programs.as_mut().expect("prepared rune programs");
                    let parse_line = utf8(strip(
                        &p.strip,
                        self.state.rune_mod_lines[at].line.as_bytes(),
                        &mut p.matching,
                    )?)?;
                    let outcome = self.parse_rune(&parse_line, origin, provider)?;
                    self.state.rune_mod_lines[at].modifiers = outcome.modifiers.unwrap_or_default();
                    self.state.rune_mod_lines[at].extra = outcome.extra;
                } else {
                    let outcome = self.parse_rune(line, origin.clone(), provider)?;
                    let mut flags = BTreeSet::from(["rune".into(), "enchant".into()]);
                    if bonded {
                        flags.insert("bonded".into());
                    }
                    let row = LoadedModLine {
                        line: display,
                        source_line: None,
                        rune_origins: vec![origin],
                        order: Some(ItemNumber::new(order)),
                        augment_type: Some(augment),
                        rune_count: None,
                        socketed_rune_effect_already_applied: None,
                        display_value_scalar: None,
                        socketed_augment_type_override: None,
                        socketed_soul_core_type: None,
                        selection: LineSelection::default(),
                        flags,
                        mod_tags: Vec::new(),
                        range: ItemNumber::Nil,
                        corrupted_range: ItemNumber::Nil,
                        value_scalar: ItemNumber::Nil,
                        modifiers: outcome.modifiers.unwrap_or_default(),
                        extra: outcome.extra,
                    };
                    let at = self
                        .state
                        .rune_mod_lines
                        .iter()
                        .position(|r| {
                            r.order
                                .and_then(ItemNumber::value)
                                .is_some_and(|v| v > order)
                        })
                        .unwrap_or(self.state.rune_mod_lines.len());
                    if self.state.rune_mod_lines.len() >= MAX_ITEM_LOADING_LINES {
                        return Err(Failure::Resource("rune row count bound".into()));
                    }
                    self.state.rune_mod_lines.insert(at, row);
                    for value in orders.values_mut() {
                        if *value >= at {
                            *value += 1;
                        }
                    }
                    orders.insert(key, at);
                }
            }
        }
        Ok(())
    }
    fn rune_groups(
        &mut self,
        context: &SlotContext,
    ) -> Result<BTreeMap<Vec<u8>, Vec<GroupedRune>>> {
        let catalog = self.catalog;
        let policy = &catalog.policy().rune_loading;
        if policy.effect_default != 0.0 {
            return Err(unsupported(
                "nonzero configured rune effect defaults require range scaling",
            ));
        }
        let table = catalog
            .runes()
            .ok_or_else(|| unsupported("selected rune definition family is unavailable"))?
            .table();
        if !table.indexed.is_empty() {
            return Err(unsupported(
                "numeric rune identity traversal is not represented",
            ));
        }
        let vector_policy = VectorPolicy {
            missing_value: policy.vector_default,
            epsilon: policy.vector_tolerance,
        };
        let mut groups = BTreeMap::<Vec<u8>, Vec<GroupedRune>>::new();
        let mut grouped_rows = 0usize;
        for (name, value) in &table.fields {
            let raw = value
                .as_table()
                .ok_or_else(|| unsupported("rune slot identity shape is not represented"))?;
            if !raw.indexed.is_empty() {
                return Err(unsupported(
                    "numeric rune slot traversal is not represented",
                ));
            }
            let mut per_rune = BTreeMap::<Vec<u8>, Vec<GroupedRune>>::new();
            for (slot, value) in &raw.fields {
                let selected_key =
                    context.broad.as_deref() == Some(slot.as_str()) || context.specific == *slot;
                // Lua strings use the string-library index. Its fixed `type`
                // field is absent, so an unrelated string slot is inert here.
                if !selected_key && matches!(value, ItemMetadataValue::Text(_)) {
                    continue;
                }
                let record = ItemRuneRecord::new(value)
                    .ok_or_else(|| source("rune slot record is not indexable"))?;
                let extra = record.field("type").and_then(ItemMetadataValue::as_str)
                    == Some(policy.extra_slot_augment_type.as_str())
                    && self.state.socketed_soul_core_types.contains(slot);
                if !selected_key && !extra {
                    continue;
                }
                let augment = record
                    .field("type")
                    .and_then(ItemMetadataValue::as_str)
                    .ok_or_else(|| unsupported("rune group augment type is not text"))?;
                for bonded in [false, true] {
                    let rows = if bonded {
                        let Some(value) = record.field("bonded").filter(|v| truthy(Some(v))) else {
                            continue;
                        };
                        ItemRuneRecord::new(value)
                            .ok_or_else(|| source("Bonded rune group is not iterable"))?
                    } else {
                        record
                    };
                    for value in rows.dense_prefix() {
                        grouped_rows += 1;
                        if grouped_rows > MAX_ITEM_LOADING_CALLS {
                            return Err(Failure::Resource("rune grouping row count bound".into()));
                        }
                        let line = string(value)?;
                        let line = if bonded {
                            format!("{}{}", policy.bonded_display_prefix, line)
                        } else {
                            line.into()
                        };
                        let parts = self.rune_parts(&line)?;
                        if parts.values.iter().any(|v| !v.is_finite()) {
                            return Err(unsupported(
                                "nonfinite rune group sorting is not order-proven",
                            ));
                        }
                        self.charge(
                            384 + parts.stripped.len()
                                + name.len()
                                + augment.len()
                                + parts.values.len() * 8,
                        )?;
                        per_rune
                            .entry(parts.stripped)
                            .or_default()
                            .push(GroupedRune {
                                name: name.clone(),
                                augment_type: augment.into(),
                                values: parts.values,
                                effect_applied: false,
                            });
                    }
                }
            }
            for (key, values) in per_rune {
                let mut iter = values.into_iter();
                let mut sum = iter.next().expect("nonempty rune group");
                for next in iter {
                    if sum.augment_type != next.augment_type {
                        return Err(unsupported(
                            "rune group type depends on slot traversal order",
                        ));
                    }
                    // Exact integral sums are independent of pairs order.
                    if sum
                        .values
                        .iter()
                        .chain(&next.values)
                        .any(|v| *v < 0.0 || v.fract() != 0.0 || *v > 9_007_199_254_740_992.0)
                        || policy.vector_default != 0.0
                    {
                        return Err(unsupported(
                            "rune group floating sum order is not represented",
                        ));
                    }
                    let p = self.rune_programs.as_mut().expect("prepared rune programs");
                    sum.values = item_runes::add_vectors(
                        &sum.values,
                        &next.values,
                        vector_policy,
                        &mut p.budget,
                    )?;
                    if sum.values.iter().any(|v| *v > 9_007_199_254_740_992.0) {
                        return Err(unsupported("rune group sum exceeds exact integer range"));
                    }
                }
                groups.entry(key).or_default().push(sum);
            }
        }
        // Canonical proof-search order is accepted only when the minimum count
        // vector is unique, not presented as the source's unstable tie order.
        let p = self.rune_programs.as_mut().expect("prepared rune programs");
        for rows in groups.values_mut() {
            // Fallible insertion sorting keeps a consistent comparator and
            // returns immediately at the shared work bound.
            for right in 1..rows.len() {
                let mut at = right;
                while at > 0 {
                    let (a, b) = (&rows[at - 1], &rows[at]);
                    p.matching
                        .charge(a.values.len().max(b.values.len()).max(1) as u64)?;
                    let mut order = std::cmp::Ordering::Equal;
                    for i in 0..a.values.len().max(b.values.len()) {
                        let av = a.values.get(i).copied().unwrap_or(policy.vector_default);
                        let bv = b.values.get(i).copied().unwrap_or(policy.vector_default);
                        if av != bv {
                            order = bv.partial_cmp(&av).expect("finite rune vectors");
                            break;
                        }
                    }
                    if order == std::cmp::Ordering::Equal {
                        order = a.name.cmp(&b.name);
                    }
                    if !order.is_gt() {
                        break;
                    }
                    rows.swap(at - 1, at);
                    at -= 1;
                }
            }
        }
        Ok(groups)
    }

    fn annotate_runes(
        &mut self,
        groups: &BTreeMap<Vec<u8>, Vec<GroupedRune>>,
        should_fix: bool,
    ) -> Result<()> {
        let mut remaining = self.state.item_socket_count;
        let mut inferred = BTreeMap::<String, usize>::new();
        for bonded in [false, true] {
            for index in 0..self.state.rune_mod_lines.len() {
                if self.state.rune_mod_lines[index].flags.contains("bonded") != bonded {
                    continue;
                }
                let text = self.state.rune_mod_lines[index].line.clone();
                let parts = self.rune_parts(&text)?;
                let Some(group) = groups.get(&parts.stripped) else {
                    continue;
                };
                let vectors: Vec<_> = group.iter().map(|r| r.values.as_slice()).collect();
                let caps: Vec<_> = group
                    .iter()
                    .map(|r| inferred.get(&r.name).copied().unwrap_or(0) as f64)
                    .collect();
                let policy = &self.catalog.policy().rune_loading;
                let p = self.rune_programs.as_mut().expect("prepared rune programs");
                let result = item_runes::find_combination(
                    &vectors,
                    &parts.values,
                    self.state.item_socket_count as f64,
                    bonded.then_some(caps.as_slice()),
                    VectorPolicy {
                        missing_value: policy.vector_default,
                        epsilon: policy.vector_tolerance,
                    },
                    &mut p.budget,
                )?;
                let Some(result) = result else { continue };
                if result.ambiguous_minimum {
                    return Err(unsupported(
                        "rune annotation has multiple minimum count vectors under unrepresented Lua traversal order",
                    ));
                }
                let positive: Vec<_> = group
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| {
                        let count = result.counts.get(&(i + 1)).copied().unwrap_or(0);
                        (count > 0).then_some((r, count))
                    })
                    .collect();
                for pair in positive.windows(2) {
                    if (0..pair[0].0.values.len().max(pair[1].0.values.len())).all(|i| {
                        pair[0]
                            .0
                            .values
                            .get(i)
                            .copied()
                            .unwrap_or(policy.vector_default)
                            == pair[1]
                                .0
                                .values
                                .get(i)
                                .copied()
                                .unwrap_or(policy.vector_default)
                    }) && pair[0].0.augment_type != pair[1].0.augment_type
                    {
                        return Err(unsupported(
                            "rune annotation type depends on tied source sort order",
                        ));
                    }
                }
                if !bonded {
                    let added: usize = positive
                        .iter()
                        .map(|(r, n)| n.saturating_sub(inferred.get(&r.name).copied().unwrap_or(0)))
                        .sum();
                    if added > remaining {
                        continue;
                    }
                    remaining -= added;
                    self.state.rune_mod_lines[index].rune_count =
                        Some(ItemNumber::new(result.count as f64));
                    let mut effect = None;
                    for (rune, count) in positive {
                        self.state.rune_mod_lines[index].augment_type =
                            Some(rune.augment_type.clone());
                        effect = Some(effect.unwrap_or(false) || rune.effect_applied);
                        let previous = inferred.get(&rune.name).copied().unwrap_or(0);
                        if should_fix {
                            for _ in previous..count {
                                if self.state.runes.len() >= MAX_ITEM_LOADING_LINES {
                                    return Err(Failure::Resource(
                                        "inferred rune count bound".into(),
                                    ));
                                }
                                self.state.runes.push(rune.name.clone());
                                self.charge(rune.name.len())?;
                            }
                        }
                        inferred.insert(rune.name.clone(), previous.max(count));
                    }
                    self.state.rune_mod_lines[index].socketed_rune_effect_already_applied = effect;
                } else {
                    for (rune, _) in positive {
                        self.state.rune_mod_lines[index].augment_type =
                            Some(rune.augment_type.clone());
                        if rune.effect_applied {
                            self.state.rune_mod_lines[index].socketed_rune_effect_already_applied =
                                Some(true);
                        }
                        // Lua false or nil is nil; preserve a preexisting false.
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn apply_rune_requirements(&mut self) -> std::result::Result<bool, ItemLoadError> {
        let result = self.rune_requirements();
        Ok(self.rune_failure(result, None)?.is_some())
    }
    fn rune_requirements(&mut self) -> Result<()> {
        if self.state.runes.is_empty() {
            return Ok(());
        }
        let catalog = self.catalog;
        let table = catalog
            .runes()
            .ok_or_else(|| unsupported("rune requirement catalog is unavailable"))?
            .table();
        for name in &self.state.runes {
            let Some(value) = table.fields.get(name).filter(|v| truthy(Some(v))) else {
                continue;
            };
            let raw = value
                .as_table()
                .ok_or_else(|| unsupported("rune requirement slot shape is not represented"))?;
            let values: Vec<_> = raw.fields.values().chain(raw.indexed.values()).collect();
            let mut levels = Vec::new();
            for value in &values {
                let level = ItemRuneRecord::new(value)
                    .and_then(|r| r.field("levelReq"))
                    .and_then(ItemMetadataValue::as_f64);
                if let Some(level) = level {
                    levels.push(level)
                } else if values.len() == 1 {
                    return Err(source(
                        "rune requirement maximum has a missing or nonnumeric level",
                    ));
                } else {
                    return Err(unsupported(
                        "rune requirement error prefix depends on slot traversal order",
                    ));
                }
            }
            let mut level = self
                .state
                .requirements
                .get("runeLevel")
                .and_then(|v| v.value())
                .ok_or_else(|| source("rune requirement maximum has an absent current level"))?;
            if levels.iter().any(|n| !n.is_finite())
                || levels.iter().any(|n| *n == 0.0 && n.is_sign_negative())
            {
                return Err(unsupported(
                    "rune requirement maximum order is not represented for nonfinite or signed-zero levels",
                ));
            }
            for value in levels {
                level = syntax::lua_max(level, value);
            }
            self.state
                .requirements
                .insert("runeLevel".into(), ItemNumber::new(level));
        }
        Ok(())
    }
}
