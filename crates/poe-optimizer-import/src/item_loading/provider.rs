//! Native formatting and unique lookup with explicitly supplied remaining dependencies.
use super::*;
use poe_optimizer_data::game_data::GameDataSnapshot;
use poe_optimizer_data::item_scalability::CatalystScalingData;
use poe_optimizer_data::unique_requirements::{UniqueRequirementCatalog, UniqueRequirementLookup};
use poe_optimizer_engine::item_tools::{
    FormatError, FormatInput, FormatResult, ItemFormatter, ParserFeedback, RangeInput,
};

pub struct NativeItemLoadProvider<'a, P> {
    snapshot: &'a GameDataSnapshot,
    dependencies: P,
    unique_requirements: Option<&'a UniqueRequirementCatalog>,
}
pub type BuiltinItemLoadProvider<'a> = NativeItemLoadProvider<'a, NativeModifierParserProvider>;
impl<'a> NativeItemLoadProvider<'a, NativeModifierParserProvider> {
    pub fn new(snapshot: &'a GameDataSnapshot) -> Self {
        Self::with_native_unique_lookup(
            snapshot,
            NativeModifierParserProvider::new(snapshot.modifier_parser()),
        )
    }
}
impl<'a, P> NativeItemLoadProvider<'a, P> {
    /// Use native formatting, forwarding parsing, unique lookup and assembly to
    /// the explicitly supplied provider. This preserves source-oracle composition.
    pub fn with_dependencies(snapshot: &'a GameDataSnapshot, dependencies: P) -> Self {
        Self {
            snapshot,
            dependencies,
            unique_requirements: None,
        }
    }
    /// Resolve unique requirements from this snapshot; parsing and assembly still
    /// use the supplied provider. An unavailable catalog never falls back to it.
    pub fn with_native_unique_lookup(snapshot: &'a GameDataSnapshot, dependencies: P) -> Self {
        Self {
            snapshot,
            dependencies,
            unique_requirements: Some(snapshot.unique_requirements()),
        }
    }
    pub fn dependencies(&self) -> &P {
        &self.dependencies
    }
    pub fn dependencies_mut(&mut self) -> &mut P {
        &mut self.dependencies
    }
}
fn error(error: FormatError) -> DependencyResult<String> {
    match error {
        FormatError::SourceError(_) => DependencyResult::SourceError(error.to_string()),
        _ => DependencyResult::ResourceError(error.to_string()),
    }
}
impl<P: ItemLoadProvider> ItemLoadProvider for NativeItemLoadProvider<'_, P> {
    fn format_line(&mut self, request: &FormatRequest) -> DependencyResult<String> {
        self.format_with_trace(request).result
    }
    fn format_with_trace(&mut self, request: &FormatRequest) -> FormatOutcome {
        let formatter = ItemFormatter::new(
            self.snapshot.item_scalability(),
            &self.snapshot.package().actor,
        );
        let input = FormatInput {
            line: &request.text,
            range: request
                .range
                .value()
                .map_or(RangeInput::Missing, RangeInput::Scalar),
            value_scalar: request.scalar.value(),
            base_value_scalar: request.corrupted_range.value(),
        };
        let mut calls = Vec::new();
        let result = match formatter.apply_range(input) {
            Ok(FormatResult::Complete(text)) => DependencyResult::Available(text),
            Ok(FormatResult::NeedsParser(fallback)) => {
                let Some(sequence) = request.sequence.checked_add(1) else {
                    return FormatOutcome {
                        result: DependencyResult::ResourceError(
                            "formatter sequence overflow".into(),
                        ),
                        precision_parser_calls: calls,
                    };
                };
                let parser_request = ParseRequest {
                    sequence,
                    line_index: request.line_index,
                    text: fallback.parser_text().into(),
                    combined: false,
                };
                let parsed = self.dependencies.parse_modifier(&parser_request);
                // Reject oversized dependency text before cloning it into both
                // the outer result and the retained nested parser trace.
                if matches!(&parsed,
                    DependencyResult::Unavailable(message)
                    | DependencyResult::SourceError(message)
                    | DependencyResult::ResourceError(message)
                    if message.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE
                ) {
                    return FormatOutcome {
                        result: DependencyResult::ResourceError("dependency message bound".into()),
                        precision_parser_calls: calls,
                    };
                }
                let result = match &parsed {
                    DependencyResult::Available(value) => match formatter.resume(
                        fallback,
                        ParserFeedback {
                            modifiers: value.modifiers.as_deref(),
                            extra: value.extra.as_deref(),
                        },
                    ) {
                        Ok(text) => DependencyResult::Available(text),
                        Err(e) => error(e),
                    },
                    DependencyResult::Unavailable(message) => {
                        DependencyResult::Unavailable(message.clone())
                    }
                    DependencyResult::SourceError(message) => {
                        DependencyResult::SourceError(message.clone())
                    }
                    DependencyResult::ResourceError(message) => {
                        DependencyResult::ResourceError(message.clone())
                    }
                };
                calls.push(FormatParserCall {
                    request: parser_request,
                    result: parsed,
                });
                result
            }
            Err(e) => error(e),
        };
        FormatOutcome {
            result,
            precision_parser_calls: calls,
        }
    }
    fn catalyst_scaling(&self) -> Option<&CatalystScalingData> {
        Some(&self.snapshot.item_scalability().data().catalyst_scaling)
    }
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        self.dependencies.parse_modifier(request)
    }
    fn lookup_unique(
        &mut self,
        request: &UniqueRequest,
    ) -> DependencyResult<Option<UniqueOutcome>> {
        let Some(catalog) = self.unique_requirements else {
            return self.dependencies.lookup_unique(request);
        };
        match catalog.lookup(
            &request.name,
            request.title.as_deref(),
            request.base_name.as_deref(),
        ) {
            UniqueRequirementLookup::Unavailable(reason) => {
                // The catalog validates this bound before any provider is constructed.
                if reason.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE {
                    return DependencyResult::ResourceError("unique catalog message bound".into());
                }
                DependencyResult::Unavailable(reason.into())
            }
            UniqueRequirementLookup::Ready(entry) => {
                DependencyResult::Available(entry.map(|entry| UniqueOutcome {
                    natural_level: entry.natural_level.map(ItemNumber::new),
                    level: entry.level.map(ItemNumber::new),
                }))
            }
        }
    }
    fn assemble(&mut self, request: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        self.dependencies.assemble(request)
    }
}
