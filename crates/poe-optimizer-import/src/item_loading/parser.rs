//! Explicit conversion from native structural parser output to item metadata.
//! Opaque callbacks and non-finite values stop at this finite item-data seam.
use super::*;
use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
use poe_optimizer_data::modifier_parser::ModifierParserCatalog;
use poe_optimizer_engine::lua_pattern::{MatchBudget, PatternError};
use poe_optimizer_engine::modifier_parser::{
    CompiledModifierParser, ModifierTable, ModifierValue, ParserError,
};
use poe_optimizer_engine::modifier_scan::ScanError;
use std::sync::Arc;

/// Compiled once for an injected catalog. Construction failures are retained and
/// reported on the first parser request; unrelated formatter operations remain usable.
pub struct NativeModifierParserProvider {
    parser: Result<Arc<CompiledModifierParser>, ParserError>,
}
impl NativeModifierParserProvider {
    pub fn new(catalog: &ModifierParserCatalog) -> Self {
        Self {
            parser: CompiledModifierParser::new(catalog).map(Arc::new),
        }
    }
    /// Share immutable compiled rules across imports or parallel preparation workers.
    pub fn from_compiled(parser: Arc<CompiledModifierParser>) -> Self {
        Self { parser: Ok(parser) }
    }
    pub fn compilation_error(&self) -> Option<&ParserError> {
        self.parser.as_ref().err()
    }
}
impl ItemLoadProvider for NativeModifierParserProvider {
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        let parser = match &self.parser {
            Ok(parser) => parser,
            Err(error) => return classify(error),
        };
        // The original combined flag only controls disabled diagnostic logging.
        let output = match parser.parse(request.text.as_bytes(), &mut MatchBudget::default()) {
            Ok(output) => output,
            Err(error) => return classify(&error),
        };
        let mut budget = ConversionBudget::default();
        let modifiers = match output.modifiers {
            Some(root) => {
                if !root.fields.is_empty()
                    || root.indexed.len() > 4096
                    || !root
                        .indexed
                        .keys()
                        .copied()
                        .eq(1..=root.indexed.len() as i64)
                {
                    return DependencyResult::Unavailable(
                        "parser returned a non-list modifier table".into(),
                    );
                }
                let mut result = Vec::new();
                for value in root.indexed.values() {
                    let Some(table) = value.as_table() else {
                        return DependencyResult::Unavailable(
                            "parser returned a non-table modifier".into(),
                        );
                    };
                    match convert_table(table, 0, &mut budget) {
                        Ok(table) => result.push(table),
                        Err(error) => return conversion_error(error),
                    }
                }
                Some(result)
            }
            None => None,
        };
        let extra = match output.extra.map(String::from_utf8).transpose() {
            Ok(extra) => extra,
            Err(_) => {
                return DependencyResult::Unavailable(
                    "parser remainder contains non-UTF-8 bytes".into(),
                );
            }
        };
        DependencyResult::Available(ParseOutcome { modifiers, extra })
    }
}
fn classify<T>(error: &ParserError) -> DependencyResult<T> {
    let message = bounded_message(error.to_string());
    match error {
        ParserError::Program(error) => match error.kind {
            poe_optimizer_engine::parser_program::ProgramRuntimeErrorKind::Source => {
                DependencyResult::SourceError(message)
            }
            poe_optimizer_engine::parser_program::ProgramRuntimeErrorKind::ResourceBound => {
                DependencyResult::ResourceError(message)
            }
            _ => DependencyResult::Unavailable(message),
        },
        ParserError::SourceError(_)
        | ParserError::Scan(ScanError::Pattern(PatternError::Source(_))) => {
            DependencyResult::SourceError(message)
        }
        ParserError::ResourceBound(_) | ParserError::Scan(_) => {
            DependencyResult::ResourceError(message)
        }
        ParserError::InvalidData(_)
        | ParserError::Deferred { .. }
        | ParserError::Ambiguous { .. } => DependencyResult::Unavailable(message),
    }
}
fn bounded_message(mut message: String) -> String {
    if message.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE {
        let mut end = MAX_ITEM_LOADING_DEPENDENCY_MESSAGE - 3;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
        message.push_str("...");
    }
    message
}
enum ConversionError {
    Unsupported(&'static str),
    Resource,
}
fn conversion_error<T>(error: ConversionError) -> DependencyResult<T> {
    match error {
        ConversionError::Unsupported(message) => DependencyResult::Unavailable(message.into()),
        ConversionError::Resource => {
            DependencyResult::ResourceError("parser item-metadata conversion bound".into())
        }
    }
}
#[derive(Default)]
struct ConversionBudget {
    values: usize,
    bytes: usize,
}
impl ConversionBudget {
    fn charge(&mut self, bytes: usize) -> Result<(), ConversionError> {
        self.values = self
            .values
            .checked_add(1)
            .ok_or(ConversionError::Resource)?;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(ConversionError::Resource)?;
        if self.values > 65_536 || self.bytes > 4 * MAX_ITEM_LOADING_TEXT {
            return Err(ConversionError::Resource);
        }
        Ok(())
    }
}
fn convert_table(
    table: &ModifierTable,
    depth: usize,
    budget: &mut ConversionBudget,
) -> Result<ItemMetadataTable, ConversionError> {
    if depth > 64 {
        return Err(ConversionError::Resource);
    }
    budget.charge(0)?;
    let mut out = ItemMetadataTable::default();
    for (key, value) in &table.fields {
        budget.charge(key.len())?;
        out.fields
            .insert(key.clone(), convert(value, depth + 1, budget)?);
    }
    for (key, value) in &table.indexed {
        out.indexed.insert(*key, convert(value, depth + 1, budget)?);
    }
    Ok(out)
}
fn convert(
    value: &ModifierValue,
    depth: usize,
    budget: &mut ConversionBudget,
) -> Result<ItemMetadataValue, ConversionError> {
    budget.charge(0)?;
    Ok(match value {
        ModifierValue::Nil => {
            return Err(ConversionError::Unsupported(
                "parser output contains an explicit nil table field",
            ));
        }
        ModifierValue::Boolean(v) => ItemMetadataValue::Boolean(*v),
        ModifierValue::Number(n) if n.is_finite() => ItemMetadataValue::Number(*n),
        ModifierValue::Number(_) => {
            return Err(ConversionError::Unsupported(
                "non-finite parser output requires a richer item assembly model",
            ));
        }
        ModifierValue::Bytes(bytes) => {
            budget.charge(bytes.len())?;
            let value = std::str::from_utf8(bytes).map_err(|_| {
                ConversionError::Unsupported("parser output contains non-UTF-8 bytes")
            })?;
            ItemMetadataValue::Text(value.into())
        }
        ModifierValue::Table(table)
            if table.fields.is_empty()
                && !table.indexed.is_empty()
                && table
                    .indexed
                    .keys()
                    .copied()
                    .eq(1..=table.indexed.len() as i64) =>
        {
            if depth > 64 {
                return Err(ConversionError::Resource);
            }
            budget.charge(0)?;
            ItemMetadataValue::Array(
                table
                    .indexed
                    .values()
                    .map(|value| convert(value, depth + 1, budget))
                    .collect::<Result<_, _>>()?,
            )
        }
        ModifierValue::Table(table) => {
            ItemMetadataValue::Table(convert_table(table, depth, budget)?)
        }
        ModifierValue::Callback(_) => {
            return Err(ConversionError::Unsupported(
                "parser callback output requires its captured-state item assembly model",
            ));
        }
    })
}

#[cfg(test)]
mod program_error_tests {
    use super::*;
    use poe_optimizer_engine::parser_program::{ProgramRuntimeError, ProgramRuntimeErrorKind};
    #[test]
    fn typed_program_errors_retain_provider_classification_and_source_context() {
        for kind in [
            ProgramRuntimeErrorKind::Source,
            ProgramRuntimeErrorKind::ResourceBound,
            ProgramRuntimeErrorKind::InvalidInput,
            ProgramRuntimeErrorKind::UnsupportedCapability,
        ] {
            let error = ParserError::Program(ProgramRuntimeError {
                kind,
                callback: Some(poe_optimizer_data::modifier_parser::ParserCallbackId(15)),
                location: Some(poe_optimizer_data::modifier_parser::ParserProgramLocation {
                    start: 3,
                    end: 7,
                }),
                message: "reached caller input".into(),
            });
            let result = classify::<()>(&error);
            let message = match (kind, result) {
                (ProgramRuntimeErrorKind::Source, DependencyResult::SourceError(message)) => {
                    message
                }
                (
                    ProgramRuntimeErrorKind::ResourceBound,
                    DependencyResult::ResourceError(message),
                ) => message,
                (
                    ProgramRuntimeErrorKind::InvalidInput
                    | ProgramRuntimeErrorKind::UnsupportedCapability,
                    DependencyResult::Unavailable(message),
                ) => message,
                _ => panic!("wrong provider classification"),
            };
            assert!(
                message.contains("ParserCallbackId(15)")
                    && message.contains("start: 3")
                    && message.contains("reached caller input")
            );
        }
    }
}
