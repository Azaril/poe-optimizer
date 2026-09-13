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
    native_assembly: bool,
}
pub type BuiltinItemLoadProvider<'a> = NativeItemLoadProvider<'a, NativeModifierParserProvider>;
impl<'a> NativeItemLoadProvider<'a, NativeModifierParserProvider> {
    pub fn new(snapshot: &'a GameDataSnapshot) -> Self {
        Self::with_native_assembly(
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
            native_assembly: false,
        }
    }
    /// Resolve unique requirements from this snapshot; parsing and assembly still
    /// use the supplied provider. An unavailable catalog never falls back to it.
    pub fn with_native_unique_lookup(snapshot: &'a GameDataSnapshot, dependencies: P) -> Self {
        Self {
            snapshot,
            dependencies,
            unique_requirements: Some(snapshot.unique_requirements()),
            native_assembly: false,
        }
    }
    /// Native assembly/formatting/unique data, with an explicitly injected parser.
    /// Production uses NativeModifierParserProvider; parity tests can isolate a
    /// dependency without changing the assembler or enabling any runtime fallback.
    pub fn with_native_assembly(snapshot: &'a GameDataSnapshot, dependencies: P) -> Self {
        Self {
            native_assembly: true,
            ..Self::with_native_unique_lookup(snapshot, dependencies)
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
                    origin: None,
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
        if self.native_assembly {
            self.assemble_with_trace(request).outcome
        } else {
            self.dependencies.assemble(request)
        }
    }
    fn assemble_with_trace(&mut self, request: &AssemblyRequest) -> AssemblyExecution {
        if !self.native_assembly {
            return self.dependencies.assemble_with_trace(request);
        }
        let snapshot = self.snapshot;
        let definitions = match snapshot.item_assembly().bind(
            snapshot.item_loading(),
            snapshot.item_scalability(),
            &snapshot.package().actor,
            snapshot.modifier_parser(),
        ) {
            Ok(d) => d,
            Err(error) => {
                return AssemblyExecution {
                    outcome: DependencyResult::Unavailable(error.to_string()),
                    prefix: None,
                };
            }
        };
        let attempt =
            super::assembly::assemble(definitions, request, self, request.previous.as_ref());
        project_attempt(attempt, |item| {
            super::assembly::loading_updates(item, &request.state)
        })
    }
}

/// Projection is a fallible diagnostic adapter. Its refusal cannot destroy the
/// authoritative graph already produced by the source-ordered algorithm.
fn project_attempt(
    attempt: super::assembly::AssemblyAttempt,
    mut project: impl FnMut(
        &super::assembly::AssembledItem,
    ) -> Result<AssemblyOutcome, super::assembly::AssemblyError>,
) -> AssemblyExecution {
    let stage = attempt.stage;
    let prefix = match attempt.partial {
        Some(item) => match project(&item) {
            Ok(value) => Some(value),
            Err(projection_error) => {
                // Preserve the original algorithm error if one was reached;
                // projection failure must not turn a Source error into Resource.
                let error = attempt.result.err().unwrap_or(projection_error);
                return AssemblyExecution {
                    outcome: assembly_error(stage, error),
                    prefix: Some(graph_only(item)),
                };
            }
        },
        None => None,
    };
    match attempt.result {
        Ok(item) => match project(&item) {
            Ok(value) => AssemblyExecution {
                outcome: DependencyResult::Available(value),
                prefix,
            },
            Err(error) => AssemblyExecution {
                outcome: assembly_error(stage, error),
                prefix: Some(graph_only(item)),
            },
        },
        Err(error) => AssemblyExecution {
            outcome: assembly_error(stage, error),
            prefix,
        },
    }
}
fn graph_only(item: super::assembly::AssembledItem) -> AssemblyOutcome {
    AssemblyOutcome {
        assembled: Some(item),
        armour_data: ArmourDataUpdate::Preserve,
        modifier_payloads: None,
        requirements: None,
        state_updates: Default::default(),
        evidence: Default::default(),
    }
}

fn assembly_error(
    stage: &str,
    error: super::assembly::AssemblyError,
) -> DependencyResult<AssemblyOutcome> {
    use super::assembly::AssemblyErrorKind;
    let mut message = format!("{stage}: {}", error.message);
    if message.len() > MAX_ITEM_LOADING_DEPENDENCY_MESSAGE {
        let mut end = MAX_ITEM_LOADING_DEPENDENCY_MESSAGE - 3;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
        message.push_str("...");
    }
    match error.kind {
        AssemblyErrorKind::Unsupported => DependencyResult::Unavailable(message),
        AssemblyErrorKind::Source => DependencyResult::SourceError(message),
        AssemblyErrorKind::Resource => DependencyResult::ResourceError(message),
    }
}

#[cfg(test)]
mod projection_failure_tests {
    use super::*;
    use std::sync::OnceLock;
    fn data() -> &'static GameDataSnapshot {
        static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
        DATA.get_or_init(|| poe_optimizer_data::game_data::bundled_snapshot().unwrap())
    }
    #[derive(Default)]
    struct RefuseProjection {
        last: Option<super::super::assembly::AssembledItem>,
    }
    impl ItemLoadProvider for RefuseProjection {
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(Vec::new()),
                extra: None,
            })
        }
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn assemble_with_trace(&mut self, r: &AssemblyRequest) -> AssemblyExecution {
            let d = data();
            let definitions = d
                .item_assembly()
                .bind(
                    d.item_loading(),
                    d.item_scalability(),
                    &d.package().actor,
                    d.modifier_parser(),
                )
                .unwrap();
            let attempt =
                super::super::assembly::assemble(definitions, r, self, r.previous.as_ref());
            let expected_complete = attempt.result.is_ok();
            self.last = attempt
                .result
                .as_ref()
                .ok()
                .cloned()
                .or_else(|| attempt.partial.clone());
            assert_eq!(self.last.as_ref().unwrap().is_complete(), expected_complete);
            // Exercise the exact production refusal branch independently of
            // allocator size. The artifact itself comes from the real assembler
            // using this machine's private current-attempt authority.
            let result = project_attempt(attempt, |_| {
                Err(super::super::assembly::AssemblyError::resource(
                    "controlled projection refusal",
                ))
            });
            let prefix = result.prefix.as_ref().unwrap();
            assert!(matches!(prefix.armour_data, ArmourDataUpdate::Preserve));
            assert!(prefix.requirements.is_none());
            assert!(prefix.modifier_payloads.is_none());
            assert!(prefix.state_updates.is_empty());
            assert!(prefix.evidence.fields.is_empty() && prefix.evidence.indexed.is_empty());
            assert!(
                prefix
                    .assembled
                    .as_ref()
                    .unwrap()
                    .shares_storage_with(self.last.as_ref().unwrap())
            );
            result
        }
    }
    #[test]
    fn complete_projection_refusal_retains_graph_without_final_admission() {
        let mut machine = ItemLoadMachine::new(data().item_loading());
        let mut provider = RefuseProjection::default();
        let error = machine
            .apply_text("Rarity: NORMAL\nGold Ring", &mut provider)
            .unwrap_err();
        assert!(error.to_string().contains("controlled projection refusal"));
        let graph = machine.assembly_progress().unwrap();
        assert!(graph.is_complete());
        assert_eq!(machine.status(), ItemLoadStatus::Pending);
        assert!(graph.shares_storage_with(provider.last.as_ref().unwrap()));
        assert!(machine.assembled().is_none());
        assert!(
            !machine
                .state()
                .retained_fields
                .contains_key("craftedQuality")
        );
        assert!(!machine.state().requirements.contains_key("strMod"));
    }
    #[test]
    fn partial_projection_refusal_retains_graph_and_original_failure() {
        let mut machine = ItemLoadMachine::new(data().item_loading());
        let mut provider = RefuseProjection::default();
        machine
            .apply_text("Rarity: NORMAL\nRusted Greathelm", &mut provider)
            .unwrap();
        let graph = machine.assembly_progress().unwrap();
        assert!(!graph.is_complete());
        assert!(graph.shares_storage_with(provider.last.as_ref().unwrap()));
        assert!(machine.assembled().is_none());
        assert_eq!(machine.status(), ItemLoadStatus::Pending);
        let message = &machine.pending().unwrap().message;
        assert!(message.contains("item local armour data assembly is unavailable"));
        assert!(!message.contains("controlled projection refusal"));
        assert!(
            !machine
                .state()
                .retained_fields
                .contains_key("craftedQuality")
        );
        assert!(!machine.state().requirements.contains_key("strMod"));
    }
}
