//! Native structural modifier parsing over complete injected source dictionaries.
//!
//! Successful parsing preserves raw records, not numerical mechanic capability.
//! Selected callbacks and source state mutations are explicit pending operations.
//! No Lua state, I/O, hidden fallback, or process-global cache is used.
mod emit;
mod factory;
mod ordinary;
mod programs;
mod strings;
mod value;
use crate::lua_pattern::{LuaPattern, MatchBudget, PatternError};
use crate::modifier_scan::{ScanCapture, ScanError, ScanTable};
use crate::parser_program::{
    CompiledParserPrograms, ProgramLimits, ProgramRequestAccounting, ProgramRuntimeError,
    ProgramValue, ProgramValueGraph,
};
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ParserAdmittedProgramCatalog, ParserCallbackId, ParserDictionary,
    ParserTableId, ParserValue,
};
use std::collections::BTreeMap;
use std::sync::Arc;
pub use value::{ModifierTable, ModifierValue};
use value::{OutputBudget, copy_value, deep_copy, equivalent};

pub const MAX_PARSER_COMPILED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_PARSER_PATTERNS: usize = 65_536;
pub const MAX_PARSER_TEXT_BYTES: usize = 1024 * 1024 - 1;
pub type ParserResult<T> = Result<T, ParserError>;
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    Scan(ScanError),
    Program(ProgramRuntimeError),
    SourceError(String),
    ResourceBound(&'static str),
    InvalidData(String),
    Deferred {
        stage: &'static str,
        callback: Option<ParserCallbackId>,
    },
    Ambiguous {
        dictionary: ParserDictionary,
        patterns: Vec<String>,
    },
}
impl From<ScanError> for ParserError {
    fn from(error: ScanError) -> Self {
        Self::Scan(error)
    }
}
impl From<PatternError> for ParserError {
    fn from(error: PatternError) -> Self {
        Self::Scan(ScanError::Pattern(error))
    }
}
impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scan(e) => e.fmt(f),
            Self::Program(e) => e.fmt(f),
            Self::SourceError(e) => write!(f, "ModParser source error: {e}"),
            Self::ResourceBound(e) => write!(f, "ModParser resource bound: {e}"),
            Self::InvalidData(e) => write!(f, "ModParser invalid data: {e}"),
            Self::Deferred { stage, callback } => {
                write!(f, "ModParser pending {stage}: {callback:?}")
            }
            Self::Ambiguous {
                dictionary,
                patterns,
            } => write!(
                f,
                "ModParser ambiguous {}: {patterns:?}",
                dictionary.source_name()
            ),
        }
    }
}
impl std::error::Error for ParserError {}
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutcome {
    /// None is source nil; an empty table is a distinct, successful source return.
    pub modifiers: Option<ModifierTable>,
    /// Presence includes the source's possibly empty extra string.
    pub extra: Option<Vec<u8>>,
}
#[derive(Debug, Clone)]
struct Dictionary {
    keys: Vec<String>,
    values: Vec<ParserValue>,
    scan: ScanTable,
}
/// Compile once per injected catalog, then share immutable parser definitions.
#[derive(Debug, Clone)]
pub struct CompiledModifierParser {
    catalog: ModifierParserCatalog,
    dictionaries: BTreeMap<ParserDictionary, Dictionary>,
    cluster: LuaPattern,
    tag_capture_numeric: LuaPattern,
    first_to_upper: LuaPattern,
    programs: Option<TypedPrograms>,
}
#[derive(Debug, Clone)]
struct TypedPrograms {
    admission: ParserAdmittedProgramCatalog,
    plans: CompiledParserPrograms,
}
impl CompiledModifierParser {
    pub fn new(catalog: &ModifierParserCatalog) -> ParserResult<Self> {
        let programs = if catalog.data().programs.admissions.is_empty() {
            None
        } else {
            let admission = ParserAdmittedProgramCatalog::new(catalog)
                .map_err(|e| ParserError::InvalidData(e.to_string()))?;
            let plans = CompiledParserPrograms::new(admission.programs())
                .map_err(|e| ParserError::InvalidData(e.to_string()))?;
            Some(TypedPrograms { admission, plans })
        };
        let mut dictionaries = BTreeMap::new();
        let mut rows = 0usize;
        let mut bytes = 0usize;
        for &name in ParserDictionary::ALL {
            let table = catalog.dictionary(name);
            rows = rows
                .checked_add(table.fields.len())
                .filter(|&n| n <= MAX_PARSER_PATTERNS)
                .ok_or(ParserError::ResourceBound("compiled dictionary rows"))?;
            let keys: Vec<_> = table.fields.keys().cloned().collect();
            let values = table.fields.values().cloned().collect();
            let scan = ScanTable::compile(&keys)?;
            bytes = bytes
                .checked_add(scan.compiled_bytes())
                .filter(|&n| n <= MAX_PARSER_COMPILED_BYTES)
                .ok_or(ParserError::ResourceBound("compiled dictionary bytes"))?;
            dictionaries.insert(name, Dictionary { keys, values, scan });
        }
        let cluster = LuaPattern::compile(catalog.data().policy.cluster_prefix_pattern.as_bytes())?;
        let tag_capture_numeric =
            LuaPattern::compile(catalog.data().policy.tag_capture_numeric_pattern.as_bytes())?;
        let first_to_upper =
            LuaPattern::compile(catalog.data().policy.first_to_upper_pattern.as_bytes())?;
        if bytes
            .checked_add(cluster.compiled_bytes())
            .and_then(|bytes| bytes.checked_add(tag_capture_numeric.compiled_bytes()))
            .and_then(|bytes| bytes.checked_add(first_to_upper.compiled_bytes()))
            .is_none_or(|n| n > MAX_PARSER_COMPILED_BYTES)
        {
            return Err(ParserError::ResourceBound("compiled dictionary bytes"));
        }
        Ok(Self {
            catalog: catalog.clone(),
            dictionaries,
            cluster,
            tag_capture_numeric,
            first_to_upper,
            programs,
        })
    }
    pub fn catalog(&self) -> &ModifierParserCatalog {
        &self.catalog
    }
    /// The original public wrapper retries order two iff order one returned both
    /// a modifier table and extra text. The retry replaces the first result.
    pub fn parse(&self, line: &[u8], budget: &mut MatchBudget) -> ParserResult<ParseOutcome> {
        if line.len() > MAX_PARSER_TEXT_BYTES {
            return Err(ParserError::ResourceBound("input bytes"));
        }
        let mut run = Run {
            parser: self,
            budget,
            output: OutputBudget::default(),
            program_accounting: ProgramRequestAccounting::new(ProgramLimits::default()),
            source_tables: BTreeMap::new(),
        };
        let first = run.parse_order(line, 1)?;
        let outcome = if first.modifiers.is_some() && first.extra.is_some() {
            run.parse_order(line, 2)?
        } else {
            first
        };
        // Public parseMod returns a recursive copy of its cached result table,
        // deliberately removing internal tag/table aliases from each return.
        let modifiers = outcome
            .modifiers
            .map(|table| {
                deep_copy(&ModifierValue::Table(Arc::new(table)), &mut run.output)
                    .and_then(|value| value.table().cloned())
            })
            .transpose()?;
        Ok(ParseOutcome {
            modifiers,
            extra: outcome.extra,
        })
    }
}
struct Selected {
    value: ModifierValue,
    captures: Vec<ModifierValue>,
}
struct Run<'a> {
    parser: &'a CompiledModifierParser,
    budget: &'a mut MatchBudget,
    output: OutputBudget,
    program_accounting: ProgramRequestAccounting,
    source_tables: BTreeMap<ParserTableId, Arc<ModifierTable>>,
}
impl Run<'_> {
    fn special_program(
        &mut self,
        callback: ParserCallbackId,
        captures: &[ModifierValue],
    ) -> ParserResult<ParseOutcome> {
        let Some(programs) = self
            .parser
            .programs
            .as_ref()
            .filter(|p| p.admission.is_special(callback))
        else {
            return Err(ParserError::Deferred {
                stage: "special callback",
                callback: Some(callback),
            });
        };
        // Source conversion occurs once at the Special call site, before the
        // program's own raw parameter/result protocol begins.
        if let Some(ModifierValue::Bytes(bytes)) = captures.first() {
            self.budget.charge(bytes.len() as u64)?;
        }
        let first = captures
            .first()
            .and_then(ModifierValue::number)
            .map(ProgramValue::Number)
            .unwrap_or(ProgramValue::Nil);
        self.output.charge(0)?;
        for capture in captures {
            let bytes = capture.as_bytes().map_or(0, |s| s.len());
            self.budget.charge(1 + bytes as u64)?;
            self.output.charge(bytes)?;
        }
        let mut values = Vec::with_capacity(captures.len() + 1);
        values.push(first);
        for capture in captures {
            values.push(match capture {
                ModifierValue::Nil => ProgramValue::Nil,
                ModifierValue::Boolean(value) => ProgramValue::Boolean(*value),
                ModifierValue::Number(value) => ProgramValue::Number(*value),
                ModifierValue::Bytes(value) => ProgramValue::Bytes(value.clone()),
                ModifierValue::Callback(value) => ProgramValue::Callback(*value),
                ModifierValue::Table(_) => {
                    return Err(ParserError::InvalidData(
                        "scanner returned a table capture".into(),
                    ));
                }
            });
        }
        let input = ProgramValueGraph {
            values,
            tables: Vec::new(),
        };
        let output = programs
            .plans
            .execute_shared(callback, &input, &mut self.program_accounting, self.budget)
            .map_err(ParserError::Program)?;
        programs::adapt(&output, &mut self.output, self.budget).map_err(|error| match error {
            ParserError::Deferred {
                stage,
                callback: None,
            } => ParserError::Deferred {
                stage,
                callback: Some(callback),
            },
            other => other,
        })
    }
    fn copy(&mut self, value: &ParserValue) -> ParserResult<ModifierValue> {
        copy_value(
            &self.parser.catalog,
            value,
            &mut self.output,
            &mut self.source_tables,
        )
    }
    fn exact(&mut self, dictionary: ParserDictionary, key: &[u8]) -> ParserResult<ModifierValue> {
        let value = std::str::from_utf8(key)
            .ok()
            .and_then(|key| self.parser.catalog.exact(dictionary, key))
            .cloned()
            .unwrap_or(ParserValue::Nil);
        self.copy(&value)
    }
    fn scan(
        &mut self,
        line: &mut Vec<u8>,
        name: ParserDictionary,
        plain: bool,
    ) -> ParserResult<Selected> {
        let dictionary = &self.parser.dictionaries[&name];
        let Some(found) = dictionary.scan.scan(line, plain, self.budget)? else {
            return Ok(Selected {
                value: ModifierValue::Nil,
                captures: Vec::new(),
            });
        };
        let selected = &dictionary.values[found.row_index];
        let value = copy_value(
            &self.parser.catalog,
            selected,
            &mut self.output,
            &mut self.source_tables,
        )?;
        let lower = (!found.tied_rows.is_empty()).then(|| line.to_ascii_lowercase());
        for &row in &found.tied_rows {
            let lower = lower.as_deref().expect("tie source bytes");
            let pattern = dictionary.scan.pattern(row).expect("validated scan row");
            let tied = pattern
                .find(lower, 1, plain, self.budget)?
                .expect("same scan input");
            let same_captures =
                tied.captures()
                    .iter()
                    .take(5)
                    .zip(&found.captures)
                    .all(|(a, b)| match (a, b) {
                        (
                            crate::lua_pattern::Capture::Bytes { start, end },
                            ScanCapture::Bytes(bytes),
                        ) => &lower[*start..*end] == bytes,
                        (crate::lua_pattern::Capture::Position(a), ScanCapture::Position(b)) => {
                            a == b
                        }
                        _ => false,
                    })
                    && tied.captures().len().min(5) == found.captures.len();
            if value.truthy() && !same_captures {
                let patterns = std::iter::once(found.row_index)
                    .chain(found.tied_rows.iter().copied())
                    .map(|i| dictionary.keys[i].clone())
                    .collect();
                return Err(ParserError::Ambiguous {
                    dictionary: name,
                    patterns,
                });
            }
            let other = copy_value(
                &self.parser.catalog,
                &dictionary.values[row],
                &mut self.output,
                &mut self.source_tables,
            )?;
            if !equivalent(&value, &other, self.budget)? {
                let patterns = std::iter::once(found.row_index)
                    .chain(found.tied_rows.iter().copied())
                    .map(|i| dictionary.keys[i].clone())
                    .collect();
                return Err(ParserError::Ambiguous {
                    dictionary: name,
                    patterns,
                });
            }
        }
        if !value.truthy() {
            return Ok(Selected {
                value: ModifierValue::Nil,
                captures: Vec::new(),
            });
        }
        let remainder = found.remainder();
        let captures = found
            .captures
            .iter()
            .map(|capture| match capture {
                ScanCapture::Bytes(bytes) => ModifierValue::Bytes(bytes.clone()),
                ScanCapture::Position(position) => ModifierValue::Number(*position as f64),
            })
            .collect();
        *line = remainder;
        Ok(Selected { value, captures })
    }
    fn parse_order(&mut self, source: &[u8], order: u8) -> ParserResult<ParseOutcome> {
        use ParserDictionary as D;
        let lower = source.to_ascii_lowercase();
        // Jewel capture rules are first-match iteration, unlike scan. Their
        // selected capture factories remain explicit pending operations.
        let jewels = &self.parser.dictionaries[&D::Jewel];
        for row in 0..jewels.keys.len() {
            let pattern = jewels.scan.pattern(row).expect("compiled jewel row");
            if let Some(found) = pattern.find(&lower, 1, false, self.budget)?
                && !found.captures().is_empty()
            {
                return Err(ParserError::Deferred {
                    stage: "jewel capture factory",
                    callback: None,
                });
            }
        }
        let jewel = self.exact(D::Jewel, &lower)?;
        if jewel.truthy() {
            return Ok(complete(vec![make_mod(
                "JewelFunc",
                "LIST",
                jewel,
                Vec::new(),
            )]));
        }
        if self.exact(D::Unsupported, &lower)?.truthy() {
            return Ok(ParseOutcome {
                modifiers: Some(ModifierTable::default()),
                extra: Some(source.to_vec()),
            });
        }
        let mut special_line = source.to_vec();
        let selected = self.scan(&mut special_line, D::Special, false)?;
        if selected.value.truthy() && special_line.is_empty() {
            if let ModifierValue::Callback(callback) = selected.value {
                return self.special_factory(callback, &selected.captures);
            }
            let value = deep_copy(&selected.value, &mut self.output)?;
            return Ok(ParseOutcome {
                modifiers: Some(value.table()?.clone()),
                extra: None,
            });
        }
        if let Some(found) = self.parser.cluster.match_captures(source, 1, self.budget)? {
            let capture = match found.captures().first() {
                Some(crate::lua_pattern::Capture::Bytes { start, end }) => {
                    ModifierValue::Bytes(source[*start..*end].to_vec())
                }
                Some(crate::lua_pattern::Capture::Position(n)) => ModifierValue::Number(*n as f64),
                None => ModifierValue::Nil,
            };
            return Ok(complete(vec![make_mod(
                "AddToClusterJewelNode",
                "LIST",
                capture,
                Vec::new(),
            )]));
        }
        self.ordinary(source, order)
    }
}
fn complete(mods: Vec<ModifierTable>) -> ParseOutcome {
    ParseOutcome {
        modifiers: Some(ModifierTable::from_values(
            mods.into_iter().map(|t| ModifierValue::Table(Arc::new(t))),
        )),
        extra: None,
    }
}
fn make_mod(
    name: &str,
    kind: &str,
    value: ModifierValue,
    args: Vec<ModifierValue>,
) -> ModifierTable {
    let mut start = 0;
    let mut source = ModifierValue::Nil;
    let mut flags = ModifierValue::Number(0.0);
    let mut keywords = ModifierValue::Number(0.0);
    if let Some(v @ ModifierValue::Bytes(_)) = args.first() {
        source = v.clone();
        start = 1;
    }
    if let Some(v @ ModifierValue::Number(_)) = args.get(1) {
        flags = v.clone();
        start = 2;
    }
    if let Some(v @ ModifierValue::Number(_)) = args.get(2) {
        keywords = v.clone();
        start = 3;
    }
    let mut table = ModifierTable::from_values(args.into_iter().skip(start));
    table.set("name", ModifierValue::text(name));
    table.set("type", ModifierValue::text(kind));
    table.set("value", value);
    table.set("flags", flags);
    table.set("keywordFlags", keywords);
    table.set("source", source);
    table
}

/// Files entering adapter fingerprints; hosts normalize checkout newlines.
pub fn implementation_sources() -> [&'static str; 19] {
    [
        include_str!("modifier_parser.rs"),
        include_str!("parser_program.rs"),
        include_str!("source_program.rs"),
        include_str!("parser_program/runtime/mod.rs"),
        include_str!("parser_program/runtime/execute.rs"),
        include_str!("parser_program/runtime/session.rs"),
        include_str!("parser_program/runtime/value.rs"),
        include_str!("parser_program/runtime/intrinsics.rs"),
        include_str!("modifier_parser/value.rs"),
        include_str!("modifier_parser/ordinary.rs"),
        include_str!("modifier_parser/emit.rs"),
        include_str!("modifier_parser/factory.rs"),
        include_str!("modifier_parser/programs.rs"),
        include_str!("modifier_parser/strings.rs"),
        include_str!("modifier_scan.rs"),
        include_str!("lua_pattern.rs"),
        include_str!("lua_pattern/substitution.rs"),
        include_str!("lua_number.rs"),
        include_str!("lua_bits.rs"),
    ]
}
