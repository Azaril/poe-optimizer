//! Immutable control-flow plans for validated, injected source programs.
//!
//! Compilation is independent of callback admission and numerical capability. This
//! native executor remains separate from legacy parser/package admission.
mod runtime;
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_data::source_program::SourceProgramCatalog;
pub use runtime::*;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Aggregate instruction limit, including unselected branches and loop control.
pub const MAX_PROGRAM_INSTRUCTIONS: usize = 500_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramCompileError {
    ResourceBound(&'static str),
    InvalidBinding {
        callback: ParserCallbackId,
        binding: u16,
    },
    InvalidControlFlow,
}
impl std::fmt::Display for ProgramCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResourceBound(part) => write!(f, "parser program compilation limit: {part}"),
            Self::InvalidBinding { callback, binding } => write!(
                f,
                "parser program {callback:?} has invalid binding {binding}"
            ),
            Self::InvalidControlFlow => f.write_str("invalid parser program control flow"),
        }
    }
}
impl std::error::Error for ProgramCompileError {}
type Result<T> = std::result::Result<T, ProgramCompileError>;

/// Each binding belongs to this compiled library's retained catalog. Numeric
/// callback/program IDs are never resolved against a separately supplied catalog.
#[derive(Debug, Clone, PartialEq)]
pub enum CompiledProgramBinding {
    Program {
        index: usize,
        callback: ParserCallbackId,
    },
    LegacyFactory {
        callback: ParserCallbackId,
    },
    Intrinsic {
        operation: ParserProgramIntrinsic,
        source: ParserProgramIntrinsicSource,
    },
}

/// A source-mapped instruction. Expression trees preserve their lazy operators,
/// argument/result adjustment and source evaluation order; compilation does not
/// evaluate, coerce or constant-fold source values.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramInstruction {
    pub location: ParserProgramLocation,
    pub operation: ProgramOperation,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ProgramOperation {
    Declare {
        locals: Vec<u16>,
        values: ParserProgramValueList,
    },
    Assign {
        locals: Vec<u16>,
        values: ParserProgramValueList,
    },
    TableSet {
        table: ParserProgramExpr,
        key: ParserProgramExpr,
        value: ParserProgramExpr,
    },
    TableAppend {
        binding: u16,
        table: ParserProgramExpr,
        value: ParserProgramExpr,
    },
    Call {
        call: ParserProgramCall,
    },
    Return {
        values: ParserProgramValueList,
    },
    /// Distinct from returning an explicit Nil expression.
    Fallthrough,
    JumpIfFalse {
        condition: ParserProgramExpr,
        target: usize,
    },
    Jump {
        target: usize,
    },
    NumericInit {
        state: usize,
        local: u16,
        start: ParserProgramExpr,
        limit: ParserProgramExpr,
        step: ParserProgramExpr,
        exit: usize,
    },
    NumericNext {
        state: usize,
        local: u16,
        body: usize,
        exit: usize,
    },
    IteratorInit {
        state: usize,
        locals: Vec<u16>,
        iterator: ParserProgramIterator,
        exit: usize,
    },
    IteratorNext {
        state: usize,
        locals: Vec<u16>,
        body: usize,
        exit: usize,
    },
}

#[derive(Debug)]
pub struct CompiledParserProgram {
    callback: ParserCallbackId,
    parameter_count: u16,
    local_count: u16,
    variadic: bool,
    instructions: Box<[ProgramInstruction]>,
    bindings: Box<[CompiledProgramBinding]>,
    loop_states: usize,
}
impl CompiledParserProgram {
    pub fn callback(&self) -> ParserCallbackId {
        self.callback
    }
    pub fn parameter_count(&self) -> u16 {
        self.parameter_count
    }
    pub fn local_count(&self) -> u16 {
        self.local_count
    }
    pub fn variadic(&self) -> bool {
        self.variadic
    }
    pub fn instructions(&self) -> &[ProgramInstruction] {
        &self.instructions
    }
    pub fn bindings(&self) -> &[CompiledProgramBinding] {
        &self.bindings
    }
    pub fn loop_states(&self) -> usize {
        self.loop_states
    }
}

#[derive(Debug)]
struct Library {
    catalog: SourceProgramCatalog,
    programs: Box<[CompiledParserProgram]>,
    callbacks: BTreeMap<ParserCallbackId, usize>,
    instruction_count: usize,
}
/// Cheaply clone/share prepared immutable code. Future invocation state belongs
/// to each evaluation; this library stores no locals, heap or mutable worker data.
#[derive(Debug, Clone)]
pub struct CompiledSourcePrograms(Arc<Library>);
impl CompiledSourcePrograms {
    pub fn new(catalog: &SourceProgramCatalog) -> Result<Self> {
        let mut callbacks = BTreeMap::new();
        for (index, program) in catalog.data().programs.iter().enumerate() {
            callbacks.insert(program.callback, index);
        }
        let mut remaining = MAX_PROGRAM_INSTRUCTIONS;
        let mut programs = Vec::with_capacity(catalog.data().programs.len());
        for program in &catalog.data().programs {
            let bindings = program
                .bindings
                .iter()
                .enumerate()
                .map(|(index, binding)| {
                    Ok(match binding {
                        ParserProgramBinding::CapturedCallback { callback, .. } => {
                            if let Some(index) = callbacks.get(callback) {
                                CompiledProgramBinding::Program {
                                    index: *index,
                                    callback: *callback,
                                }
                            } else if matches!(
                                catalog.owner().factory(*callback),
                                Some(ParserFactoryDisposition::Pure(_))
                            ) {
                                CompiledProgramBinding::LegacyFactory {
                                    callback: *callback,
                                }
                            } else {
                                return Err(ProgramCompileError::InvalidBinding {
                                    callback: program.callback,
                                    binding: index as u16,
                                });
                            }
                        }
                        ParserProgramBinding::Intrinsic { operation, source } => {
                            CompiledProgramBinding::Intrinsic {
                                operation: *operation,
                                source: source.clone(),
                            }
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let mut lowerer = Lowerer {
                code: Vec::new(),
                loop_states: 0,
                breaks: Vec::new(),
                remaining: &mut remaining,
            };
            lowerer.block(&program.body)?;
            let length = program.provenance.function_end - program.provenance.function_start;
            lowerer.emit(
                ParserProgramLocation {
                    start: length - 1,
                    end: length,
                },
                ProgramOperation::Fallthrough,
            )?;
            lowerer.verify_targets()?;
            programs.push(CompiledParserProgram {
                callback: program.callback,
                parameter_count: program.parameter_count,
                local_count: program.local_count,
                variadic: program.variadic,
                instructions: lowerer.code.into_boxed_slice(),
                bindings: bindings.into_boxed_slice(),
                loop_states: lowerer.loop_states,
            });
        }
        Ok(Self(Arc::new(Library {
            catalog: catalog.clone(),
            programs: programs.into_boxed_slice(),
            callbacks,
            instruction_count: MAX_PROGRAM_INSTRUCTIONS - remaining,
        })))
    }
    pub fn catalog(&self) -> &SourceProgramCatalog {
        &self.0.catalog
    }
    pub fn program(&self, callback: ParserCallbackId) -> Option<&CompiledParserProgram> {
        self.0
            .callbacks
            .get(&callback)
            .map(|&index| &self.0.programs[index])
    }
    pub fn programs(&self) -> &[CompiledParserProgram] {
        &self.0.programs
    }
    pub fn instruction_count(&self) -> usize {
        self.0.instruction_count
    }
}

/// Parser compatibility facade over the shared source compiler and VM.
/// The original parser catalog is retained, with its existing identity contract.
#[derive(Debug, Clone)]
pub struct CompiledParserPrograms {
    catalog: ParserProgramCatalog,
    source: CompiledSourcePrograms,
}
impl CompiledParserPrograms {
    pub fn new(catalog: &ParserProgramCatalog) -> Result<Self> {
        Ok(Self {
            catalog: catalog.clone(),
            source: CompiledSourcePrograms::new(catalog.source_programs())?,
        })
    }
    pub fn catalog(&self) -> &ParserProgramCatalog {
        &self.catalog
    }
    pub fn source_programs(&self) -> &CompiledSourcePrograms {
        &self.source
    }
    pub fn program(&self, callback: ParserCallbackId) -> Option<&CompiledParserProgram> {
        self.source.program(callback)
    }
    pub fn programs(&self) -> &[CompiledParserProgram] {
        self.source.programs()
    }
    pub fn instruction_count(&self) -> usize {
        self.source.instruction_count()
    }
}

struct Lowerer<'a> {
    code: Vec<ProgramInstruction>,
    loop_states: usize,
    breaks: Vec<Vec<usize>>,
    remaining: &'a mut usize,
}
impl Lowerer<'_> {
    fn emit(
        &mut self,
        location: ParserProgramLocation,
        operation: ProgramOperation,
    ) -> Result<usize> {
        *self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(ProgramCompileError::ResourceBound("instructions"))?;
        let index = self.code.len();
        self.code.push(ProgramInstruction {
            location,
            operation,
        });
        Ok(index)
    }
    fn patch_exit(&mut self, index: usize, target: usize) -> Result<()> {
        match &mut self
            .code
            .get_mut(index)
            .ok_or(ProgramCompileError::InvalidControlFlow)?
            .operation
        {
            ProgramOperation::Jump { target: slot }
            | ProgramOperation::JumpIfFalse { target: slot, .. }
            | ProgramOperation::NumericInit { exit: slot, .. }
            | ProgramOperation::NumericNext { exit: slot, .. }
            | ProgramOperation::IteratorInit { exit: slot, .. }
            | ProgramOperation::IteratorNext { exit: slot, .. } => *slot = target,
            _ => return Err(ProgramCompileError::InvalidControlFlow),
        }
        Ok(())
    }
    fn finish_loop(&mut self, initial: usize, next: usize) -> Result<()> {
        let exit = self.code.len();
        self.patch_exit(initial, exit)?;
        self.patch_exit(next, exit)?;
        for instruction in self
            .breaks
            .pop()
            .ok_or(ProgramCompileError::InvalidControlFlow)?
        {
            self.patch_exit(instruction, exit)?;
        }
        Ok(())
    }
    fn block(&mut self, block: &[ParserProgramStatement]) -> Result<()> {
        for statement in block {
            let location = statement.location;
            use ParserProgramStatementKind as S;
            match &statement.operation {
                S::Declare { locals, values } => {
                    self.emit(
                        location,
                        ProgramOperation::Declare {
                            locals: locals.clone(),
                            values: values.clone(),
                        },
                    )?;
                }
                S::Assign { locals, values } => {
                    self.emit(
                        location,
                        ProgramOperation::Assign {
                            locals: locals.clone(),
                            values: values.clone(),
                        },
                    )?;
                }
                S::TableSet { table, key, value } => {
                    self.emit(
                        location,
                        ProgramOperation::TableSet {
                            table: table.clone(),
                            key: key.clone(),
                            value: value.clone(),
                        },
                    )?;
                }
                S::TableAppend {
                    binding,
                    table,
                    value,
                } => {
                    self.emit(
                        location,
                        ProgramOperation::TableAppend {
                            binding: *binding,
                            table: table.clone(),
                            value: value.clone(),
                        },
                    )?;
                }
                S::Call { call } => {
                    self.emit(location, ProgramOperation::Call { call: call.clone() })?;
                }
                S::Return { values } => {
                    self.emit(
                        location,
                        ProgramOperation::Return {
                            values: values.clone(),
                        },
                    )?;
                }
                S::Break => {
                    if self.breaks.is_empty() {
                        return Err(ProgramCompileError::InvalidControlFlow);
                    }
                    let instruction =
                        self.emit(location, ProgramOperation::Jump { target: usize::MAX })?;
                    self.breaks
                        .last_mut()
                        .ok_or(ProgramCompileError::InvalidControlFlow)?
                        .push(instruction);
                }
                S::If {
                    branches,
                    otherwise,
                } => {
                    let mut done = Vec::with_capacity(branches.len());
                    for branch in branches {
                        let condition = self.emit(
                            location,
                            ProgramOperation::JumpIfFalse {
                                condition: branch.condition.clone(),
                                target: usize::MAX,
                            },
                        )?;
                        self.block(&branch.body)?;
                        done.push(
                            self.emit(location, ProgramOperation::Jump { target: usize::MAX })?,
                        );
                        self.patch_exit(condition, self.code.len())?;
                    }
                    self.block(otherwise)?;
                    for jump in done {
                        self.patch_exit(jump, self.code.len())?;
                    }
                }
                S::ForNumeric {
                    local,
                    start,
                    limit,
                    step,
                    body,
                } => {
                    let state = self.loop_states;
                    self.loop_states += 1;
                    let initial = self.emit(
                        location,
                        ProgramOperation::NumericInit {
                            state,
                            local: *local,
                            start: start.clone(),
                            limit: limit.clone(),
                            step: step.clone(),
                            exit: usize::MAX,
                        },
                    )?;
                    self.breaks.push(Vec::new());
                    let body_start = self.code.len();
                    self.block(body)?;
                    let next = self.emit(
                        location,
                        ProgramOperation::NumericNext {
                            state,
                            local: *local,
                            body: body_start,
                            exit: usize::MAX,
                        },
                    )?;
                    self.finish_loop(initial, next)?;
                }
                S::ForEach {
                    locals,
                    iterator,
                    body,
                } => {
                    let state = self.loop_states;
                    self.loop_states += 1;
                    let initial = self.emit(
                        location,
                        ProgramOperation::IteratorInit {
                            state,
                            locals: locals.clone(),
                            iterator: iterator.clone(),
                            exit: usize::MAX,
                        },
                    )?;
                    self.breaks.push(Vec::new());
                    let body_start = self.code.len();
                    self.block(body)?;
                    let next = self.emit(
                        location,
                        ProgramOperation::IteratorNext {
                            state,
                            locals: locals.clone(),
                            body: body_start,
                            exit: usize::MAX,
                        },
                    )?;
                    self.finish_loop(initial, next)?;
                }
            }
        }
        Ok(())
    }
    fn verify_targets(&self) -> Result<()> {
        for instruction in &self.code {
            let targets: &[usize] = match &instruction.operation {
                ProgramOperation::Jump { target }
                | ProgramOperation::JumpIfFalse { target, .. } => std::slice::from_ref(target),
                ProgramOperation::NumericInit { exit, .. }
                | ProgramOperation::IteratorInit { exit, .. } => std::slice::from_ref(exit),
                ProgramOperation::NumericNext { body, exit, .. }
                | ProgramOperation::IteratorNext { body, exit, .. } => {
                    if *body >= self.code.len() {
                        return Err(ProgramCompileError::InvalidControlFlow);
                    }
                    std::slice::from_ref(exit)
                }
                _ => &[],
            };
            if targets.iter().any(|&target| target >= self.code.len()) {
                return Err(ProgramCompileError::InvalidControlFlow);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
