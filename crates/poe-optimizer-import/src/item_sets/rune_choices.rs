//! Finite rune-row construction and traversal-independent name selection.
//! Parser dependencies must return stable per-line values: this is the existing
//! finite metadata ingress, not a reconstruction of arbitrary Lua aliases or
//! parser-cache history. Constructor/parser failure prefixes under the original
//! unrepresented pairs order remain Unsupported. Tied display order is never
//! exposed as a source array-order certificate. The bounded headless proof is
//! documented in docs/rune-headless-selection.md and depends on the locked
//! LuaJIT auxsort implementation, not on a global strict weak order.
use super::{ItemSetLimits, ItemSetUsage};
use crate::item_loading::assembly::{AssemblyError, AssemblyErrorKind};
use crate::item_loading::{DependencyResult, ItemLoadProvider, ParseRequest};
use crate::item_slot_validity::Value;
use poe_optimizer_data::item_assembly::ItemInventoryPolicy;
use poe_optimizer_data::item_loading::{
    ItemLoadingCatalog, ItemMetadataTable as Table, ItemMetadataValue as M, ItemRuneRecord,
};
use std::{
    collections::BTreeMap,
    fmt::{self, Write},
    mem::size_of,
    sync::Arc,
};

type Result<T> = std::result::Result<T, AssemblyError>;
#[derive(Debug)]
struct Owner {
    definitions: ItemLoadingCatalog,
    records: Vec<Table>,
    choices: BTreeMap<String, Vec<usize>>,
    limits: ItemSetLimits,
}
/// A real prepared record retained by its exact private owner, including mods.
#[derive(Debug, Clone)]
pub struct ItemActivationRune {
    owner: Arc<Owner>,
    index: usize,
}
impl ItemActivationRune {
    pub fn name(&self) -> &str {
        self.record().fields["name"]
            .as_str()
            .expect("private rune name")
    }
    pub fn record(&self) -> &Table {
        &self.owner.records[self.index]
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.index == other.index
    }
    pub fn belongs_to(&self, definitions: &ItemLoadingCatalog) -> bool {
        self.owner.definitions.shares_storage_with(definitions)
    }
}
#[derive(Debug, Clone)]
pub struct RuneChoiceCatalog {
    owner: Arc<Owner>,
}
/// Always retain this result, including failed preparation, across resumes.
/// Charges cover this producer's requested logical storage/work. Dependency
/// internals and allocator capacity/RSS are not reported by these counters.
#[derive(Debug)]
pub struct RuneChoicePreparation {
    pub result: DependencyResult<RuneChoiceCatalog>,
    pub usage: ItemSetUsage,
}
impl RuneChoiceCatalog {
    pub fn prepare<P: ItemLoadProvider + ?Sized>(
        definitions: &ItemLoadingCatalog,
        policy: &ItemInventoryPolicy,
        provider: &mut P,
        limits: ItemSetLimits,
    ) -> RuneChoicePreparation {
        let mut budget = Budget {
            limits,
            usage: ItemSetUsage::default(),
        };
        let result = build(definitions, policy, provider, &mut budget);
        RuneChoicePreparation {
            result: dependency(result),
            usage: budget.usage,
        }
    }
    pub fn initial(&self, slot_name: &str) -> DependencyResult<ItemActivationRune> {
        self.initial_with_usage(slot_name).0
    }
    /// Logical byte/identity checks consumed by this query, also on failure.
    /// Callers combine these with their cumulative activation work budget.
    pub fn initial_with_usage(
        &self,
        slot_name: &str,
    ) -> (DependencyResult<ItemActivationRune>, u64) {
        let mut work = SelectionWork::new(self.owner.limits);
        let result = self
            .list(slot_name, &mut work)
            .map(|indices| self.handle(indices[0]));
        (dependency(result), work.steps)
    }
    pub fn select(
        &self,
        slot_name: &str,
        requested: Value<'_>,
        previous: &ItemActivationRune,
    ) -> DependencyResult<ItemActivationRune> {
        self.select_with_usage(slot_name, requested, previous).0
    }
    pub fn select_with_usage(
        &self,
        slot_name: &str,
        requested: Value<'_>,
        previous: &ItemActivationRune,
    ) -> (DependencyResult<ItemActivationRune>, u64) {
        let mut work = SelectionWork::new(self.owner.limits);
        let result = (|| {
            let choices = self.list(slot_name, &mut work)?;
            work.step()?;
            if !Arc::ptr_eq(&self.owner, &previous.owner) {
                return Err(unsupported(
                    "rune selection belongs to another owner or slot",
                ));
            }
            let mut member = false;
            for &index in choices {
                work.step()?;
                if index == previous.index {
                    member = true;
                    break;
                }
            }
            if !member {
                return Err(unsupported(
                    "rune selection belongs to another owner or slot",
                ));
            }
            if let Value::Text(name) = requested {
                work.text(name)?;
                for &index in choices {
                    let candidate = self.owner.records[index].fields["name"]
                        .as_str()
                        .expect("private rune name");
                    if work.equal(candidate, name)? {
                        return Ok(self.handle(index));
                    }
                }
            }
            // SelByValue leaves the exact previous record selected on a miss.
            Ok(previous.clone())
        })();
        (dependency(result), work.steps)
    }
    fn list(&self, slot_name: &str, work: &mut SelectionWork) -> Result<&[usize]> {
        work.text(slot_name)?;
        // Explicit comparisons make query charges independent of BTreeMap's
        // implementation and retain consumed work for unsuccessful lookups.
        for (name, choices) in &self.owner.choices {
            if work.equal(name, slot_name)? {
                return Ok(choices);
            }
        }
        Err(unsupported("unknown injected rune slot"))
    }
    fn handle(&self, index: usize) -> ItemActivationRune {
        ItemActivationRune {
            owner: self.owner.clone(),
            index,
        }
    }
}
struct SelectionWork {
    limits: ItemSetLimits,
    steps: u64,
}
impl SelectionWork {
    fn new(limits: ItemSetLimits) -> Self {
        Self { limits, steps: 0 }
    }
    fn step(&mut self) -> Result<()> {
        if self.steps >= self.limits.max_steps {
            return Err(AssemblyError::resource("rune selection work bound"));
        }
        self.steps += 1;
        Ok(())
    }
    fn text(&mut self, value: &str) -> Result<()> {
        self.step()?;
        if value.len() > self.limits.max_string_bytes {
            return Err(AssemblyError::resource("rune selection input string bound"));
        }
        Ok(())
    }
    fn equal(&mut self, a: &str, b: &str) -> Result<bool> {
        self.step()?; // length comparison
        if a.len() != b.len() {
            return Ok(false);
        }
        for (a, b) in a.bytes().zip(b.bytes()) {
            self.step()?;
            if a != b {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
fn dependency<T>(value: Result<T>) -> DependencyResult<T> {
    match value {
        Ok(value) => DependencyResult::Available(value),
        Err(error) => match error.kind {
            AssemblyErrorKind::Unsupported => DependencyResult::Unavailable(error.message),
            AssemblyErrorKind::Source => DependencyResult::SourceError(error.message),
            AssemblyErrorKind::Resource => DependencyResult::ResourceError(error.message),
        },
    }
}
fn unsupported(message: impl Into<String>) -> AssemblyError {
    AssemblyError::unsupported(message)
}
struct Budget {
    limits: ItemSetLimits,
    usage: ItemSetUsage,
}
impl Budget {
    fn charge(&mut self, bytes: usize, steps: u64) -> Result<()> {
        let b = self
            .usage
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| AssemblyError::resource("rune choice byte overflow"))?;
        let s = self
            .usage
            .steps
            .checked_add(steps)
            .ok_or_else(|| AssemblyError::resource("rune choice work overflow"))?;
        if b > self.limits.max_bytes || s > self.limits.max_steps {
            return Err(AssemblyError::resource("rune choice byte/work bound"));
        }
        self.usage.bytes = b;
        self.usage.steps = s;
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<String> {
        if text.len() > self.limits.max_string_bytes {
            return Err(AssemblyError::resource("rune choice string bound"));
        }
        self.charge(text.len(), 1)?;
        Ok(text.into())
    }
    fn table(&mut self) -> Result<()> {
        if self.usage.tables >= self.limits.max_tables {
            return Err(AssemblyError::resource("rune choice table bound"));
        }
        self.charge(size_of::<Table>(), 1)?;
        self.usage.tables += 1;
        Ok(())
    }
    fn cell(&mut self) -> Result<()> {
        if self.usage.values >= self.limits.max_values {
            return Err(AssemblyError::resource("rune choice value bound"));
        }
        // Logical map/sequence cell cost, independent of allocator capacity.
        self.charge(size_of::<(String, M)>(), 1)?;
        self.usage.values += 1;
        Ok(())
    }
    fn value(&mut self, value: &M, depth: usize) -> Result<()> {
        if depth > 96 {
            return Err(AssemblyError::resource("rune choice metadata depth bound"));
        }
        self.cell()?;
        match value {
            M::Boolean(_) => Ok(()),
            M::Number(n) if n.is_finite() => Ok(()),
            M::Number(_) => Err(unsupported("nonfinite rune choice metadata")),
            M::Text(s) => {
                self.text(s)?;
                Ok(())
            }
            M::Callback(_) => Err(unsupported("rune choice reached callable metadata ingress")),
            M::Array(values) => {
                self.table()?;
                for value in values {
                    self.value(value, depth + 1)?;
                }
                Ok(())
            }
            M::Table(table) => self.metadata(table, depth + 1),
        }
    }
    fn metadata(&mut self, table: &Table, depth: usize) -> Result<()> {
        if depth > 96 {
            return Err(AssemblyError::resource("rune choice metadata depth bound"));
        }
        self.table()?;
        for (key, value) in &table.fields {
            self.text(key)?;
            self.value(value, depth)?;
        }
        for value in table.indexed.values() {
            self.value(value, depth)?;
        }
        Ok(())
    }
    fn put(&mut self, table: &mut Table, key: &str, value: M) -> Result<()> {
        self.cell()?;
        let key = self.text(key)?;
        table.fields.insert(key, value);
        Ok(())
    }
    fn copied(&mut self, table: &mut Table, key: &str, value: Option<&M>) -> Result<()> {
        if let Some(value) = value {
            self.value(value, 0)?;
            self.put(table, key, value.clone())?;
        }
        Ok(())
    }
}
fn truthy(value: Option<&M>) -> bool {
    !matches!(value, None | Some(M::Boolean(false)))
}
fn record(value: &M) -> Result<ItemRuneRecord<'_>> {
    ItemRuneRecord::new(value).ok_or_else(|| unsupported(
        "rune construction reached a non-table; original traversal error prefix is unrepresented"))
}
fn optional_record(value: Option<&M>) -> Result<Option<ItemRuneRecord<'_>>> {
    if truthy(value) {
        value.map(record).transpose()
    } else {
        Ok(None)
    }
}
fn build<P: ItemLoadProvider + ?Sized>(
    definitions: &ItemLoadingCatalog,
    policy: &ItemInventoryPolicy,
    provider: &mut P,
    budget: &mut Budget,
) -> Result<RuneChoiceCatalog> {
    policy.validate().map_err(|e| unsupported(e.to_string()))?;
    budget.charge(size_of::<Owner>() + 2 * size_of::<usize>(), 1)?;
    let p = &policy.activation.rune_choices;
    let raw = definitions
        .runes()
        .ok_or_else(|| unsupported("rune definition family unavailable"))?
        .table();
    if !raw.indexed.is_empty() {
        return Err(unsupported(
            "numeric rune names require a different selected-record interface",
        ));
    }
    let mut records = Vec::new();
    budget.charge(size_of::<Table>(), 1)?;
    budget.table()?;
    let mut empty = Table::default();
    for (key, value) in [
        ("name", &p.empty.name),
        ("label", &p.empty.label),
        ("slot", &p.empty.slot_type),
    ] {
        let value = M::Text(budget.text(value)?);
        budget.put(&mut empty, key, value)?;
    }
    let line = M::Text(budget.text(&p.empty.line)?);
    budget.table()?;
    budget.cell()?;
    budget.put(&mut empty, "lines", M::Array(vec![line]))?;
    budget.table()?;
    budget.put(&mut empty, "mods", M::Array(Vec::new()))?;
    for (key, value) in [
        ("req", p.empty.required_level),
        ("order", p.empty.order),
        ("group", p.empty.group),
    ] {
        budget.put(&mut empty, key, M::Number(value))?;
    }
    budget.put(
        &mut empty,
        "isSocketBound",
        M::Boolean(p.empty.is_socket_bound),
    )?;
    records.push(empty);
    for (name, family) in &raw.fields {
        budget.charge(0, 1)?;
        let family = record(family)?;
        let ItemRuneRecord::Table(family) = family else {
            return Err(unsupported(
                "numeric rune slot keys require an unrepresented slot identity",
            ));
        };
        if !family.indexed.is_empty() {
            return Err(unsupported(
                "numeric rune slot keys require an unrepresented slot identity",
            ));
        }
        for (slot, raw) in &family.fields {
            if records.len() >= budget.limits.max_slots {
                return Err(AssemblyError::resource("rune choice row bound"));
            }
            budget.charge(size_of::<Table>(), 1)?;
            let raw = record(raw)?;
            let mut lines = Vec::new();
            for line in raw.dense_prefix() {
                let M::Text(text) = line else {
                    return Err(unsupported("rune modifier input is not finite text"));
                };
                budget.cell()?;
                lines.push(M::Text(budget.text(text)?));
            }
            let bonded = optional_record(raw.field("bonded"))?;
            if let Some(bonded) = bonded {
                for line in bonded.dense_prefix() {
                    let M::Text(text) = line else {
                        return Err(unsupported(
                            "bonded display concatenation requires a non-text value",
                        ));
                    };
                    let bytes = p
                        .bonded_display_prefix
                        .len()
                        .checked_add(text.len())
                        .ok_or_else(|| AssemblyError::resource("bonded display byte overflow"))?;
                    if bytes > budget.limits.max_string_bytes {
                        return Err(AssemblyError::resource("bonded display string bound"));
                    }
                    budget.cell()?;
                    budget.charge(bytes, 1)?;
                    lines.push(M::Text(format!("{}{text}", p.bonded_display_prefix)));
                }
            }
            let mut mods = Vec::new();
            for (ordinary_index, line) in raw.dense_prefix().enumerate() {
                let M::Text(text) = line else {
                    unreachable!("checked ordinary lines")
                };
                let request = ParseRequest {
                    sequence: usize::try_from(budget.usage.operations)
                        .map_err(|_| AssemblyError::resource("rune parser sequence bound"))?,
                    line_index: None,
                    origin: None,
                    text: budget.text(text)?,
                    combined: false,
                };
                budget.charge(size_of::<ParseRequest>(), 1)?;
                budget.usage.operations = budget
                    .usage
                    .operations
                    .checked_add(1)
                    .ok_or_else(|| AssemblyError::resource("rune parser call overflow"))?;
                let parsed = match provider.parse_modifier(&request) {
                    DependencyResult::Available(value) => value,
                    DependencyResult::Unavailable(message) => {
                        return Err(parser_failure(
                            AssemblyErrorKind::Unsupported,
                            "rune choice parser dependency: ",
                            &message,
                            ParserInput {
                                family: name,
                                slot,
                                ordinary_row: ordinary_index + 1,
                                text: &request.text,
                            },
                            budget,
                        ));
                    }
                    DependencyResult::SourceError(message) => {
                        return Err(parser_failure(
                            AssemblyErrorKind::Unsupported,
                            "rune choice parser Source failure; original initialization prefix unrepresented: ",
                            &message,
                            ParserInput {
                                family: name,
                                slot,
                                ordinary_row: ordinary_index + 1,
                                text: &request.text,
                            },
                            budget,
                        ));
                    }
                    DependencyResult::ResourceError(message) => {
                        return Err(parser_failure(
                            AssemblyErrorKind::Resource,
                            "rune choice parser resource failure: ",
                            &message,
                            ParserInput {
                                family: name,
                                slot,
                                ordinary_row: ordinary_index + 1,
                                text: &request.text,
                            },
                            budget,
                        ));
                    }
                };
                for mut modifier in parsed.modifiers.unwrap_or_default() {
                    budget.metadata(&modifier, 0)?;
                    let len = p
                        .modifier_source_prefix
                        .len()
                        .checked_add(name.len())
                        .ok_or_else(|| AssemblyError::resource("rune source byte overflow"))?;
                    if len > budget.limits.max_string_bytes {
                        return Err(AssemblyError::resource("rune source string bound"));
                    }
                    budget.charge(len, 1)?;
                    let source = format!("{}{name}", p.modifier_source_prefix);
                    set_source(&mut modifier, &source, budget)?;
                    budget.cell()?;
                    mods.push(M::Table(modifier));
                }
            }
            let first_order = optional_record(raw.field("statOrder"))?.and_then(|v| v.indexed(1));
            let fallback = M::Number(p.order_default);
            let order = if truthy(first_order) {
                first_order
            } else {
                let second = bonded
                    .map(|b| optional_record(b.field("statOrder")))
                    .transpose()?
                    .flatten()
                    .and_then(|v| v.indexed(1));
                if truthy(second) { second } else { None }
            }
            .unwrap_or(&fallback);
            budget.value(order, 0)?;
            let order = order.clone();
            budget.table()?;
            let mut row = Table::default();
            for (key, text) in [("name", name), ("slot", slot)] {
                let value = M::Text(budget.text(text)?);
                budget.put(&mut row, key, value)?;
            }
            budget.copied(&mut row, "label", raw.indexed(1))?;
            budget.copied(&mut row, "req", raw.field("levelReq"))?;
            budget.put(&mut row, "order", order)?;
            budget.put(&mut row, "group", M::Number(lines.len() as f64))?;
            budget.table()?;
            budget.put(&mut row, "lines", M::Array(lines))?;
            budget.table()?;
            budget.put(&mut row, "mods", M::Array(mods))?;
            for key in [
                "type",
                "isSocketBound",
                "localMod",
                "limit",
                "canSocketInChakraSlots",
                "canSocketInUniqueItems",
                "canSocketInJewellery",
            ] {
                budget.copied(&mut row, key, raw.field(key))?;
            }
            records.push(row);
        }
    }
    let order = prove_headless_projection(&records, budget, less)?;
    // Only the first row is source-order certified. The remainder is private
    // deterministic storage; unique selected names make that order irrelevant.
    let first = order[0];
    let mut choices = BTreeMap::new();
    for slot in &policy.layout.rune_slots {
        budget.charge(size_of::<(String, Vec<usize>)>() + size_of::<usize>(), 1)?;
        let mut selected = vec![first];
        let mut names = BTreeMap::<&str, usize>::new();
        budget.charge(size_of::<(&str, usize)>(), 1)?;
        names.insert(records[first].fields["name"].as_str().expect("name"), first);
        for &index in &order {
            budget.charge(0, 1)?;
            let row = &records[index];
            if truthy(row.fields.get("canSocketInChakraSlots"))
                && !truthy(row.fields.get("isSocketBound"))
                && row
                    .fields
                    .get("slot")
                    .and_then(M::as_str)
                    .is_some_and(|s| s == slot.slot_type || s == p.broad_slot_type)
            {
                let name = row.fields["name"].as_str().expect("name");
                budget.charge(
                    0,
                    (name.len().saturating_add(1)).saturating_mul(names.len().saturating_add(1))
                        as u64,
                )?;
                if names.insert(name, index).is_some_and(|old| old != index) {
                    return Err(unsupported(
                        "multiple eligible rune records have the same selected name",
                    ));
                }
                budget.charge(size_of::<usize>() + size_of::<(&str, usize)>(), 1)?;
                selected.push(index);
            }
        }
        let key = budget.text(&slot.name)?;
        choices.insert(key, selected);
    }
    Ok(RuneChoiceCatalog {
        owner: Arc::new(Owner {
            definitions: definitions.clone(),
            records,
            choices,
            limits: budget.limits,
        }),
    })
}
// This context names the actual provider request, not a reconstruction of Lua
// pairs order or a claim about an original source initialization failure prefix.
struct ParserInput<'a> {
    family: &'a str,
    slot: &'a str,
    ordinary_row: usize,
    text: &'a str,
}
const PARSER_REASON_BYTES: usize = 4096;
const PARSER_CONTEXT_BYTES: usize = 4096;
struct ParserContext {
    bytes: [u8; PARSER_CONTEXT_BYTES],
    len: usize,
}
impl Write for ParserContext {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(fmt::Error)?;
        let target = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        target.copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}
impl ParserContext {
    fn quoted(&mut self, text: &str, limit: usize) -> std::result::Result<bool, fmt::Error> {
        self.write_char('"')?;
        let mut written = 0;
        let mut truncated = false;
        // At most `limit` escaped bytes are copied, with one character of
        // lookahead. A giant key cannot cause unbounded Debug formatting or
        // allocation before a bound.
        for character in text.chars() {
            let count = character.escape_default().count();
            if count > limit - written {
                truncated = true;
                break;
            }
            for escaped in character.escape_default() {
                self.write_char(escaped)?;
            }
            written += count;
        }
        self.write_char('"')?;
        Ok(truncated)
    }
    fn input(input: ParserInput<'_>) -> std::result::Result<Self, fmt::Error> {
        let mut context = Self {
            bytes: [0; PARSER_CONTEXT_BYTES],
            len: 0,
        };
        context.write_str("family=")?;
        let family_truncated = context.quoted(input.family, 512)?;
        context.write_str("; slot=")?;
        let slot_truncated = context.quoted(input.slot, 512)?;
        write!(context, "; ordinary_row={}; input=", input.ordinary_row)?;
        let input_truncated = context.quoted(input.text, 2048)?;
        write!(
            context,
            "; family_bytes={}; family_truncated={family_truncated}; slot_bytes={}; slot_truncated={slot_truncated}; input_bytes={}; input_truncated={input_truncated}",
            input.family.len(),
            input.slot.len(),
            input.text.len()
        )?;
        Ok(context)
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("only UTF-8 formatter writes")
    }
}
fn parser_failure(
    kind: AssemblyErrorKind,
    prefix: &'static str,
    reason: &str,
    input: ParserInput<'_>,
    budget: &mut Budget,
) -> AssemblyError {
    // The pre-existing bounded error reason remains the diagnostic allowance.
    // Charge every extra retained context byte (including delimiters) and one
    // formatting step before allocating the final message. If that cannot fit,
    // retain the original dependency kind and explicitly omit context within the
    // old reason allowance instead of turning a diagnostic into a new failure.
    let context = (budget.usage.steps < budget.limits.max_steps)
        .then(|| ParserContext::input(input).ok())
        .flatten()
        .filter(|context| budget.charge(context.len + 3, 1).is_ok());
    let message = if let Some(context) = context {
        format!(
            "{prefix}[{}] {}",
            context.as_str(),
            bounded(reason, PARSER_REASON_BYTES)
        )
    } else {
        const OMITTED: &str = "[input context omitted: diagnostic byte/work bound] ";
        format!(
            "{prefix}{OMITTED}{}",
            bounded(reason, PARSER_REASON_BYTES - OMITTED.len())
        )
    };
    AssemblyError { kind, message }
}
fn bounded(message: &str, limit: usize) -> &str {
    let mut end = message.len().min(limit);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    &message[..end]
}

fn set_source(modifier: &mut Table, source: &str, budget: &mut Budget) -> Result<()> {
    let text = M::Text(budget.text(source)?);
    budget.put(modifier, "source", text)?;
    if let Some(M::Table(value)) = modifier.fields.get_mut("value")
        && let Some(nested) = value.fields.get_mut("mod")
        && truthy(Some(nested))
    {
        if let M::Array(values) = nested {
            budget.table()?;
            budget.charge(
                values
                    .len()
                    .checked_mul(size_of::<(i64, M)>())
                    .ok_or_else(|| AssemblyError::resource("nested mod map bound"))?,
                values.len() as u64,
            )?;
            let indexed = std::mem::take(values)
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i as i64 + 1, v))
                .collect();
            *nested = M::Table(Table {
                fields: BTreeMap::new(),
                indexed,
            });
        }
        let M::Table(nested) = nested else {
            return Err(unsupported(
                "setSource reached non-table nested mod; traversal prefix unrepresented",
            ));
        };
        let text = M::Text(budget.text(source)?);
        budget.put(nested, "source", text)?;
    }
    Ok(())
}
fn equal(a: Option<&M>, b: Option<&M>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(M::Number(a)), Some(M::Number(b))) => a == b,
        (Some(M::Text(a)), Some(M::Text(b))) => a == b,
        (Some(M::Boolean(a)), Some(M::Boolean(b))) => a == b,
        _ => false,
    }
}
fn lt(a: Option<&M>, b: Option<&M>) -> Result<bool> {
    match (a, b) {
        (Some(M::Number(a)), Some(M::Number(b))) if a.is_finite() && b.is_finite() => Ok(a < b),
        (Some(M::Text(a)), Some(M::Text(b))) => Ok(a.as_bytes() < b.as_bytes()),
        _ => Err(unsupported(
            "rune comparator reached incomparable values; source sort prefix unrepresented",
        )),
    }
}
fn less(a: &Table, b: &Table, budget: &mut Budget) -> Result<bool> {
    let ao = a.fields.get("order");
    let bo = b.fields.get("order");
    let bytes = ao
        .into_iter()
        .chain(bo)
        .filter_map(M::as_str)
        .map(str::len)
        .sum::<usize>();
    budget.charge(0, bytes as u64 + 1)?;
    if equal(ao, bo) {
        let ar = a.fields.get("req");
        let br = b.fields.get("req");
        budget.charge(
            0,
            ar.into_iter()
                .chain(br)
                .filter_map(M::as_str)
                .map(str::len)
                .sum::<usize>() as u64,
        )?;
        lt(ar, br)
    } else if equal(a.fields.get("group"), b.fields.get("group")) {
        lt(ao, bo)
    } else {
        lt(a.fields.get("group"), b.fields.get("group"))
    }
}
/// Certifies only auxsort completion, its unique first row, and the multiset.
/// The fixed production comparator is less; the private function parameter
/// also lets tests exercise the safety guards independently of that comparator.
fn prove_headless_projection(
    records: &[Table],
    budget: &mut Budget,
    compare: fn(&Table, &Table, &mut Budget) -> Result<bool>,
) -> Result<Vec<usize>> {
    if records.is_empty() {
        return Err(unsupported("rune choices have no initial record"));
    }
    let bytes = records
        .len()
        .checked_mul(size_of::<usize>() + size_of::<bool>())
        .ok_or_else(|| AssemblyError::resource("rune projection allocation overflow"))?;
    budget.charge(bytes, records.len() as u64)?;
    let mut minimum = vec![true; records.len()];
    for i in 0..records.len() {
        // auxsort's <=3-element cases never compare a row with itself. For
        // larger inputs, the unchanged pivot at u-1 is the forward sentinel.
        if records.len() >= 4 && compare(&records[i], &records[i], budget)? {
            return Err(unsupported("rune comparator is not irreflexive"));
        }
        for j in i + 1..records.len() {
            let forward = compare(&records[i], &records[j], budget)?;
            let reverse = compare(&records[j], &records[i], budget)?;
            // Asymmetry makes median-of-three establish the reverse sentinel
            // !(pivot < left). Transitivity is unnecessary for either scan.
            if forward && reverse {
                return Err(unsupported("rune comparator is not asymmetric"));
            }
            minimum[i] &= forward;
            minimum[j] &= reverse;
        }
    }
    budget.charge(0, (records.len() as u64).saturating_mul(2))?;
    let first = minimum
        .iter()
        .position(|&candidate| candidate)
        .ok_or_else(|| unsupported("first rune record has no strict universal minimum"))?;
    // Asymmetry precludes two strict universal minima. Every partition keeps
    // this row left of its pivot, so induction puts this exact row at index1.
    // Remaining order is not an observed or reconstructed source traversal.
    let mut indices = Vec::with_capacity(records.len());
    indices.push(first);
    indices.extend((0..records.len()).filter(|&index| index != first));
    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_loading::ParseOutcome;
    use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
    use std::sync::OnceLock;

    fn snapshot() -> &'static GameDataSnapshot {
        static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
        DATA.get_or_init(|| bundled_snapshot().unwrap())
    }
    fn row(order: f64, req: f64, lines: &[&str]) -> M {
        M::Table(Table {
            fields: [
                ("statOrder".into(), M::Array(vec![M::Number(order)])),
                ("levelReq".into(), M::Number(req)),
                ("canSocketInChakraSlots".into(), M::Boolean(true)),
                ("isSocketBound".into(), M::Boolean(false)),
            ]
            .into(),
            indexed: lines
                .iter()
                .enumerate()
                .map(|(i, s)| (i as i64 + 1, M::Text((*s).into())))
                .collect(),
        })
    }
    fn fixture(rows: Vec<(&str, &str, M)>) -> (ItemLoadingCatalog, ItemInventoryPolicy) {
        let mut data = snapshot().item_loading().data().clone();
        let mut raw = Table::default();
        for (name, slot, value) in rows {
            let family = raw
                .fields
                .entry(name.into())
                .or_insert_with(|| M::Table(Table::default()));
            let M::Table(family) = family else {
                unreachable!()
            };
            family.fields.insert(slot.into(), value);
        }
        data.modifier_tables
            .insert(data.policy.rune_loading.rune_table.clone(), raw);
        let mut policy = snapshot().item_assembly().policy().inventory.clone();
        policy.layout.rune_slots.truncate(1);
        policy.layout.rune_slots[0].name = "Caller control".into();
        policy.layout.rune_slots[0].slot_type = "caller boots".into();
        let p = &mut policy.activation.rune_choices;
        p.broad_slot_type = "caller armour".into();
        p.modifier_source_prefix = "caller source:".into();
        p.bonded_display_prefix = "caller bonded:".into();
        p.empty.name = "Caller empty".into();
        p.empty.group = -1.0;
        p.empty.order = -1.0;
        (ItemLoadingCatalog::new(data).unwrap(), policy)
    }
    #[derive(Default)]
    struct Parser {
        calls: Vec<String>,
        fail: bool,
    }
    impl ItemLoadProvider for Parser {
        fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
            self.calls.push(request.text.clone());
            assert!(!request.combined);
            assert_eq!(request.line_index, None);
            if self.fail {
                return DependencyResult::Unavailable("caller parser".into());
            }
            let nested = Table {
                fields: [("name".into(), M::Text(request.text.clone()))].into(),
                indexed: BTreeMap::new(),
            };
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(vec![Table {
                    fields: [(
                        "value".into(),
                        M::Table(Table {
                            fields: [("mod".into(), M::Table(nested))].into(),
                            indexed: BTreeMap::new(),
                        }),
                    )]
                    .into(),
                    indexed: BTreeMap::new(),
                }]),
                extra: Some("ignored remainder".into()),
            })
        }
    }
    fn available<T>(result: DependencyResult<T>) -> T {
        match result {
            DependencyResult::Available(v) => v,
            _ => panic!("expected available"),
        }
    }
    #[test]
    fn prepares_all_ordinary_inputs_before_filtering_and_retains_real_sourced_modifiers() {
        let mut first = row(1.0, 1.0, &["ordinary"]);
        let M::Table(ref mut first_table) = first else {
            unreachable!()
        };
        first_table
            .fields
            .insert("bonded".into(), M::Array(vec![M::Text("bonded".into())]));
        let (data, policy) = fixture(vec![
            ("First", "caller boots", first),
            ("Other", "unselected type", row(2.0, 1.0, &["other"])),
        ]);
        let mut parser = Parser::default();
        let attempt =
            RuneChoiceCatalog::prepare(&data, &policy, &mut parser, ItemSetLimits::default());
        assert_eq!(parser.calls, ["ordinary", "other"]);
        assert_eq!(attempt.usage.operations, 2);
        let catalog = available(attempt.result);
        let empty = available(catalog.initial("Caller control"));
        let selected = available(catalog.select("Caller control", Value::Text("First"), &empty));
        assert!(selected.belongs_to(&data));
        assert_eq!(
            selected.record().fields["lines"],
            M::Array(vec![
                M::Text("ordinary".into()),
                M::Text("caller bonded:bonded".into()),
            ])
        );
        let M::Array(mods) = &selected.record().fields["mods"] else {
            panic!()
        };
        let M::Table(modifier) = &mods[0] else {
            panic!()
        };
        assert_eq!(
            modifier.fields["source"].as_str(),
            Some("caller source:First")
        );
        let nested = modifier.fields["value"].as_table().unwrap().fields["mod"]
            .as_table()
            .unwrap();
        assert_eq!(
            nested.fields["source"].as_str(),
            Some("caller source:First")
        );
        assert!(
            available(catalog.select("Caller control", Value::Text("unknown"), &selected))
                .same_identity(&selected)
        );
        assert!(
            available(catalog.select("Caller control", Value::Boolean(false), &selected))
                .same_identity(&selected)
        );
        let other = available(
            RuneChoiceCatalog::prepare(
                &data,
                &policy,
                &mut Parser::default(),
                ItemSetLimits::default(),
            )
            .result,
        );
        assert!(matches!(
            other.select("Caller control", Value::Text("First"), &selected),
            DependencyResult::Unavailable(_)
        ));
    }
    #[test]
    fn duplicate_names_and_ambiguous_first_record_are_not_silently_tiebroken() {
        let (data, policy) = fixture(vec![
            ("Same", "caller boots", row(1.0, 1.0, &["a"])),
            ("Same", "caller armour", row(2.0, 1.0, &["b"])),
        ]);
        assert!(matches!(
            RuneChoiceCatalog::prepare(
                &data,
                &policy,
                &mut Parser::default(),
                ItemSetLimits::default()
            )
            .result,
            DependencyResult::Unavailable(_)
        ));
        let (data, mut policy) = fixture(vec![("First", "caller boots", row(-1.0, 1.0, &["a"]))]);
        policy.activation.rune_choices.empty.required_level = 1.0;
        // Equal order compares only req, ignoring different group values.
        assert!(matches!(
            RuneChoiceCatalog::prepare(
                &data,
                &policy,
                &mut Parser::default(),
                ItemSetLimits::default()
            )
            .result,
            DependencyResult::Unavailable(_)
        ));
    }
    fn sort_row(order: f64, req: f64, group: f64) -> Table {
        Table {
            fields: [
                ("order".into(), M::Number(order)),
                ("req".into(), M::Number(req)),
                ("group".into(), M::Number(group)),
            ]
            .into(),
            indexed: BTreeMap::new(),
        }
    }
    fn budget() -> Budget {
        Budget {
            limits: ItemSetLimits::default(),
            usage: ItemSetUsage::default(),
        }
    }
    #[test]
    fn headless_proof_accepts_pinned_cycle_only_with_a_strict_universal_minimum() {
        // Actual injected Adept/armour, Greater Adept/caster, Aldur/armour rows.
        let cycle = [
            sort_row(993.0, 15.0, 3.0),
            sort_row(993.0, 30.0, 1.0),
            sort_row(6239.0, 0.0, 1.0),
        ];
        assert!(less(&cycle[0], &cycle[1], &mut budget()).unwrap());
        assert!(less(&cycle[1], &cycle[2], &mut budget()).unwrap());
        assert!(less(&cycle[2], &cycle[0], &mut budget()).unwrap());
        assert!(prove_headless_projection(&cycle, &mut budget(), less).is_err());
        let rows = [
            cycle[0].clone(),
            sort_row(-1.0, 1.0, -1.0),
            cycle[1].clone(),
            cycle[2].clone(),
        ];
        let indices = prove_headless_projection(&rows, &mut budget(), less).unwrap();
        assert_eq!(indices, [1, 0, 2, 3]); // Private remainder, no source order claim.
        let equal_tail = [
            sort_row(-1.0, 1.0, -1.0),
            sort_row(2.0, 3.0, 1.0),
            sort_row(2.0, 3.0, 1.0),
        ];
        assert_eq!(
            prove_headless_projection(&equal_tail, &mut budget(), less).unwrap()[0],
            0
        );
        let mut b = budget();
        b.limits.max_steps = 1;
        assert_eq!(
            prove_headless_projection(&rows, &mut b, less)
                .unwrap_err()
                .kind,
            AssemblyErrorKind::Resource
        );
    }
    #[test]
    fn headless_proof_checks_pivot_self_reads_and_comparison_safety() {
        let minimum = sort_row(-1.0, 1.0, -1.0);
        let mut no_req = sort_row(1.0, 1.0, 1.0);
        no_req.fields.remove("req");
        // Two-element auxsort compares distinct orders only.
        assert!(
            prove_headless_projection(&[minimum.clone(), no_req.clone()], &mut budget(), less)
                .is_ok()
        );
        // This missing level can be reached by a pivot self-comparison.
        assert!(
            prove_headless_projection(
                &[
                    minimum.clone(),
                    no_req.clone(),
                    sort_row(2.0, 1.0, 1.0),
                    sort_row(3.0, 1.0, 1.0)
                ],
                &mut budget(),
                less
            )
            .is_err()
        );
        // Equal distinct orders also reach the missing level.
        assert!(
            prove_headless_projection(&[no_req, sort_row(1.0, 2.0, 1.0)], &mut budget(), less)
                .is_err()
        );
        fn symmetric(_: &Table, _: &Table, budget: &mut Budget) -> Result<bool> {
            budget.charge(0, 1)?;
            Ok(true)
        }
        assert!(
            prove_headless_projection(
                &[minimum.clone(), sort_row(1.0, 2.0, 1.0)],
                &mut budget(),
                symmetric
            )
            .unwrap_err()
            .message
            .contains("not asymmetric")
        );
        assert!(
            prove_headless_projection(
                &[minimum.clone(), minimum.clone(), minimum.clone(), minimum],
                &mut budget(),
                symmetric
            )
            .unwrap_err()
            .message
            .contains("not irreflexive")
        );
    }
    #[test]
    fn pinned_cycle_subset_preserves_real_rows_and_parses_before_selection() {
        let raw = snapshot().item_loading().runes().unwrap().table();
        let selected = [
            ("Adept Rune", "armour"),
            ("Greater Adept Rune", "caster"),
            ("Aldur's Legacy", "armour"),
        ]
        .into_iter()
        .map(|(name, slot)| {
            (
                name,
                slot,
                raw.fields[name].as_table().unwrap().fields[slot].clone(),
            )
        })
        .collect();
        let (definitions, mut policy) = fixture(selected);
        policy.layout.rune_slots[0].slot_type = "armour".into();
        policy.activation.rune_choices.broad_slot_type = "armour".into();
        let mut parser = Parser::default();
        let catalog = available(
            RuneChoiceCatalog::prepare(
                &definitions,
                &policy,
                &mut parser,
                ItemSetLimits::default(),
            )
            .result,
        );
        assert_eq!(parser.calls.len(), 3); // Includes both ineligible rows.
        let initial = available(catalog.initial("Caller control"));
        let rune = available(catalog.select("Caller control", Value::Text("Adept Rune"), &initial));
        assert_eq!(rune.record().fields["group"], M::Number(3.0));
        let mods = rune.record().fields["mods"].as_array().unwrap();
        assert_eq!(mods.len(), 1); // Bonded descriptions do not invoke the parser.
        assert_eq!(
            mods[0].as_table().unwrap().fields["source"].as_str(),
            Some("caller source:Adept Rune")
        );
        assert!(
            available(catalog.select("Caller control", Value::Text("Greater Adept Rune"), &rune))
                .same_identity(&rune)
        );
        assert!(
            available(catalog.select("Caller control", Value::Text("Aldur's Legacy"), &rune))
                .same_identity(&rune)
        );
    }
    #[test]
    fn failed_parser_request_reports_actual_family_slot_and_ordinary_row() {
        struct FailingParser {
            kind: AssemblyErrorKind,
            calls: Vec<(usize, String)>,
        }
        impl ItemLoadProvider for FailingParser {
            fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
                self.calls.push((request.sequence, request.text.clone()));
                if self.calls.len() == 3 {
                    let reason = "actual caller failure".into();
                    return match self.kind {
                        AssemblyErrorKind::Unsupported => DependencyResult::Unavailable(reason),
                        AssemblyErrorKind::Source => DependencyResult::SourceError(reason),
                        AssemblyErrorKind::Resource => DependencyResult::ResourceError(reason),
                    };
                }
                DependencyResult::Available(ParseOutcome {
                    modifiers: None,
                    extra: None,
                })
            }
        }
        let (data, policy) = fixture(vec![
            ("A earlier", "unselected type", row(1.0, 1.0, &["earlier"])),
            (
                "Z actual",
                "caller boots",
                row(2.0, 1.0, &["prior", "failed input", "not reached"]),
            ),
        ]);
        for kind in [
            AssemblyErrorKind::Unsupported,
            AssemblyErrorKind::Source,
            AssemblyErrorKind::Resource,
        ] {
            let mut parser = FailingParser {
                kind,
                calls: Vec::new(),
            };
            let attempt =
                RuneChoiceCatalog::prepare(&data, &policy, &mut parser, ItemSetLimits::default());
            let message = match (kind, attempt.result) {
                (
                    AssemblyErrorKind::Unsupported | AssemblyErrorKind::Source,
                    DependencyResult::Unavailable(message),
                ) => message,
                (AssemblyErrorKind::Resource, DependencyResult::ResourceError(message)) => message,
                (_, other) => panic!("changed parser failure kind: {other:?}"),
            };
            assert_eq!(
                parser.calls,
                [
                    (0, "earlier".into()),
                    (1, "prior".into()),
                    (2, "failed input".into())
                ]
            );
            assert_eq!(attempt.usage.operations, 3);
            for expected in [
                "family=\"Z actual\"",
                "slot=\"caller boots\"",
                "ordinary_row=2",
                "input=\"failed input\"",
                "input_truncated=false",
                "actual caller failure",
            ] {
                assert!(message.contains(expected), "{message}");
            }
            assert!(!message.contains("not reached"));
            if kind == AssemblyErrorKind::Source {
                assert!(message.contains("original initialization prefix unrepresented"));
            }
        }
    }

    #[test]
    fn parser_context_escapes_and_bounds_before_charging_retained_bytes() {
        let family = format!("quoted\"family\n{}", "é".repeat(2000));
        let slot = "slot\\\t\0";
        let input = format!("actual\ninput\"{}", "雪".repeat(4000));
        let reason = "é".repeat(3000);
        let mut budget = Budget {
            limits: ItemSetLimits::default(),
            usage: ItemSetUsage {
                bytes: 17,
                steps: 9,
                ..ItemSetUsage::default()
            },
        };
        let prefix = "parser failure: ";
        let error = parser_failure(
            AssemblyErrorKind::Unsupported,
            prefix,
            &reason,
            ParserInput {
                family: &family,
                slot,
                ordinary_row: 2,
                text: &input,
            },
            &mut budget,
        );
        assert_eq!(error.kind, AssemblyErrorKind::Unsupported);
        for expected in [
            r#"family="quoted\"family\n"#,
            r#"slot="slot\\\t\u{0}""#,
            r#"input="actual\ninput\""#,
            "family_truncated=true",
            "slot_truncated=false",
            "input_truncated=true",
        ] {
            assert!(
                error.message.contains(expected),
                "{expected}: {}",
                error.message
            );
        }
        assert!(
            error.message.len() <= prefix.len() + PARSER_CONTEXT_BYTES + 3 + PARSER_REASON_BYTES
        );
        let extra =
            error.message.len() - prefix.len() - bounded(&reason, PARSER_REASON_BYTES).len();
        assert_eq!(budget.usage.bytes, 17 + extra);
        assert_eq!(budget.usage.steps, 10);
        assert!(error.message.is_char_boundary(error.message.len()));
    }

    #[test]
    fn diagnostic_budget_exhaustion_preserves_failure_kind_and_usage() {
        for limits in [
            ItemSetLimits {
                max_bytes: 0,
                ..ItemSetLimits::default()
            },
            ItemSetLimits {
                max_steps: 0,
                ..ItemSetLimits::default()
            },
        ] {
            for kind in [
                AssemblyErrorKind::Unsupported,
                AssemblyErrorKind::Source,
                AssemblyErrorKind::Resource,
            ] {
                let mut budget = Budget {
                    limits,
                    usage: ItemSetUsage::default(),
                };
                let reason = "é".repeat(3000);
                let error = parser_failure(
                    kind,
                    "parser failure: ",
                    &reason,
                    ParserInput {
                        family: "Family",
                        slot: "Slot",
                        ordinary_row: 1,
                        text: "actual input",
                    },
                    &mut budget,
                );
                assert_eq!(error.kind, kind);
                assert!(
                    error
                        .message
                        .contains("input context omitted: diagnostic byte/work bound")
                );
                assert!(error.message.len() <= "parser failure: ".len() + PARSER_REASON_BYTES);
                assert_eq!(budget.usage.bytes, 0);
                assert_eq!(budget.usage.steps, 0);
                assert_eq!(budget.usage.operations, 0);
            }
        }
    }

    #[test]
    fn raw_ipairs_holes_and_parser_failure_remain_explicit() {
        let mut raw = row(1.0, 1.0, &["first"]);
        let M::Table(ref mut table) = raw else {
            unreachable!()
        };
        table.indexed.insert(3, M::Text("after hole".into()));
        let (data, policy) = fixture(vec![("One", "caller boots", raw)]);
        let mut parser = Parser::default();
        available(
            RuneChoiceCatalog::prepare(&data, &policy, &mut parser, ItemSetLimits::default())
                .result,
        );
        assert_eq!(parser.calls, ["first"]);
        let mut parser = Parser {
            fail: true,
            ..Parser::default()
        };
        let attempt =
            RuneChoiceCatalog::prepare(&data, &policy, &mut parser, ItemSetLimits::default());
        assert!(matches!(attempt.result, DependencyResult::Unavailable(_)));
        assert_eq!(attempt.usage.operations, 1);
        let limits = ItemSetLimits {
            max_bytes: 0,
            ..ItemSetLimits::default()
        };
        let mut parser = Parser::default();
        assert!(matches!(
            RuneChoiceCatalog::prepare(&data, &policy, &mut parser, limits).result,
            DependencyResult::ResourceError(_)
        ));
        assert!(parser.calls.is_empty());
    }
}
