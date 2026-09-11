use super::super::{
    CompiledParserPrograms, CompiledProgramBinding, CompiledSourcePrograms, ProgramOperation,
};
use super::{
    ProgramAllocationUsage, ProgramLimits, ProgramOutput, ProgramRequestAccounting,
    ProgramRuntimeError as Error, ProgramValueGraph, RuntimeResult as Result, SourceProgramOutput,
    intrinsics,
    value::{ClosureRef, Heap, TableBehavior, V},
};
use crate::lua_pattern::MatchBudget;
use poe_optimizer_data::modifier_parser::*;

// Bounds Rust interpreter recursion independently of caller-configured call limits.
const MAX_EVALUATOR_NESTING: usize = 64;

impl CompiledSourcePrograms {
    /// Execute a raw source-bound program. This deliberately does not apply
    /// Special/Prefix/ModTag packing, parser caching or public recursive copying.
    pub fn execute(
        &self,
        callback: ParserCallbackId,
        input: &ProgramValueGraph,
        limits: ProgramLimits,
    ) -> Result<SourceProgramOutput> {
        let mut accounting = ProgramRequestAccounting::new(limits);
        let mut patterns = MatchBudget::new(limits.pattern);
        self.execute_shared(callback, input, &mut accounting, &mut patterns)
    }
    /// Execute with the parser request's existing accounting and scan budget.
    /// Counters are borrowed directly: an early error never discards prior work.
    pub(crate) fn execute_shared(
        &self,
        callback: ParserCallbackId,
        input: &ProgramValueGraph,
        accounting: &mut ProgramRequestAccounting,
        patterns: &mut MatchBudget,
    ) -> Result<SourceProgramOutput> {
        let limits = accounting.limits;
        let initial_steps = accounting.steps();
        let initial_pattern_steps = patterns.steps_used();
        let initial_allocations = accounting.allocation_usage();
        let index = *self
            .0
            .callbacks
            .get(&callback)
            .ok_or_else(|| Error::input("callback has no compiled program"))?;
        if input.values.len() > limits.max_results {
            return Err(Error::resource("argument pack size"));
        }
        let (mut heap, arguments) = Heap::new_shared(
            self.catalog().owner(),
            input,
            &limits,
            &mut accounting.allocations,
        )?;
        let mut run = Run {
            library: self,
            heap: &mut heap,
            patterns,
            limits,
            steps: &mut accounting.steps,
            call_depth: 0,
        };
        let values = run.invoke(index, arguments, 0)?;
        let graph = run.heap.freeze(&values)?;
        let used = run.heap.stats();
        Ok(SourceProgramOutput {
            graph,
            owner: self.catalog().owner().clone(),
            steps: *run.steps - initial_steps,
            pattern_steps: run.patterns.steps_used() - initial_pattern_steps,
            allocations: ProgramAllocationUsage {
                values: used.values - initial_allocations.values,
                bytes: used.bytes - initial_allocations.bytes,
                tables: used.tables - initial_allocations.tables,
            },
        })
    }
}
impl CompiledParserPrograms {
    pub fn execute(
        &self,
        callback: ParserCallbackId,
        input: &ProgramValueGraph,
        limits: ProgramLimits,
    ) -> Result<ProgramOutput> {
        let source = self.source.execute(callback, input, limits)?;
        Ok(ProgramOutput {
            source,
            owner: self.catalog().owner().clone(),
        })
    }
    pub(crate) fn execute_shared(
        &self,
        callback: ParserCallbackId,
        input: &ProgramValueGraph,
        accounting: &mut ProgramRequestAccounting,
        patterns: &mut MatchBudget,
    ) -> Result<ProgramOutput> {
        let source = self
            .source
            .execute_shared(callback, input, accounting, patterns)?;
        Ok(ProgramOutput {
            source,
            owner: self.catalog().owner().clone(),
        })
    }
}

pub(super) struct Run<'a, 'b, 'c> {
    pub(super) library: &'a CompiledSourcePrograms,
    pub(super) heap: &'b mut Heap<'c>,
    pub(super) patterns: &'b mut MatchBudget,
    pub(super) limits: ProgramLimits,
    pub(super) steps: &'b mut u64,
    pub(super) call_depth: usize,
}
pub(super) enum MethodTarget {
    Value(V),
    StringIntrinsic(ParserProgramIntrinsic),
}
struct Frame {
    program: usize,
    closure: Option<ClosureRef>,
    locals: Vec<V>,
    extra: Vec<V>,
    loops: Vec<Option<Loop>>,
}
enum Loop {
    Numeric { value: f64, limit: f64, step: f64 },
    Dense { table: V, index: u64 },
    Pattern(Box<intrinsics::Gmatch>),
}
impl Run<'_, '_, '_> {
    fn tick(&mut self, depth: usize) -> Result<()> {
        if depth > MAX_EVALUATOR_NESTING {
            return Err(Error::resource("combined expression/call nesting"));
        }
        *self.steps = self
            .steps
            .checked_add(1)
            .filter(|n| *n <= self.limits.max_steps)
            .ok_or_else(|| Error::resource("program instruction/expression budget"))?;
        Ok(())
    }
    fn pack_space(&mut self, count: usize) -> Result<()> {
        if count > self.limits.max_results {
            return Err(Error::resource("result/argument pack size"));
        }
        self.heap.charge_values(count)
    }
    pub(super) fn invoke(
        &mut self,
        index: usize,
        arguments: Vec<V>,
        depth: usize,
    ) -> Result<Vec<V>> {
        self.invoke_frame(index, arguments, depth, None)
    }
    fn invoke_frame(
        &mut self,
        index: usize,
        arguments: Vec<V>,
        depth: usize,
        closure: Option<ClosureRef>,
    ) -> Result<Vec<V>> {
        self.tick(depth)?;
        if self.call_depth >= self.limits.max_call_depth {
            return Err(Error::resource("program call depth"));
        }
        let plan = &self.library.0.programs[index];
        if closure.is_none()
            && self
                .library
                .catalog()
                .owner()
                .closure_prototype_id(plan.callback)
                .is_some()
        {
            return Err(Error::unsupported(
                "closure prototype execution requires a session instance",
            ));
        }
        self.heap
            .charge_values(usize::from(plan.local_count) + plan.loop_states)?;
        let mut locals = vec![V::Nil; usize::from(plan.local_count)];
        for (slot, value) in arguments
            .iter()
            .take(usize::from(plan.parameter_count))
            .enumerate()
        {
            locals[slot] = value.clone();
        }
        let extra = if plan.variadic {
            let count = arguments
                .len()
                .saturating_sub(usize::from(plan.parameter_count));
            self.pack_space(count)?;
            arguments
                .into_iter()
                .skip(usize::from(plan.parameter_count))
                .collect()
        } else {
            Vec::new()
        };
        let mut frame = Frame {
            program: index,
            closure,
            locals,
            extra,
            loops: (0..plan.loop_states).map(|_| None).collect(),
        };
        self.call_depth += 1;
        let result = self.frame(&mut frame, depth + 1);
        self.call_depth -= 1;
        result
    }
    fn frame(&mut self, frame: &mut Frame, depth: usize) -> Result<Vec<V>> {
        // Copy only the library reference. Plans remain immutable and borrowed;
        // source expressions and instruction payloads are never cloned per step.
        let library = self.library;
        let plan = &library.0.programs[frame.program];
        let mut pc = 0;
        loop {
            let instruction = plan
                .instructions
                .get(pc)
                .ok_or_else(|| Error::input("compiled instruction target out of range"))?;
            let location = instruction.location;
            let step = (|| -> Result<Option<Vec<V>>> {
                self.tick(depth)?;
                use ProgramOperation as Op;
                match &instruction.operation {
                    Op::Declare { locals, values } | Op::Assign { locals, values } => {
                        let values = self.values(frame, values, depth + 1)?;
                        // Lua emits the final assignment store first, then
                        // unwinds the earlier LHS stores. Duplicate slots make
                        // that order observable. Declarations use distinct fresh
                        // slots, for which the same order is immaterial.
                        for (index, slot) in locals.iter().enumerate().rev() {
                            frame.locals[*slot as usize] =
                                values.get(index).cloned().unwrap_or(V::Nil);
                        }
                        pc += 1;
                    }
                    Op::CaptureSet { upvalue, values } => {
                        // Lua evaluates the full RHS pack before storing its
                        // adjusted first result into the shared upvalue cell.
                        let values = self.values(frame, values, depth + 1)?;
                        let closure = frame.closure.ok_or_else(|| {
                            Error::unsupported("capture assignment requires a session closure")
                        })?;
                        self.heap.set_closure_capture(
                            closure,
                            *upvalue,
                            values.into_iter().next().unwrap_or(V::Nil),
                        )?;
                        pc += 1;
                    }
                    Op::TableSet { table, key, value } => {
                        let table = self.expr(frame, table, depth + 1)?;
                        let key = self.expr(frame, key, depth + 1)?;
                        let value = self.expr(frame, value, depth + 1)?;
                        self.heap.set(&table, key, value)?;
                        pc += 1;
                    }
                    Op::TableAppend { table, value, .. } => {
                        let table = self.expr(frame, table, depth + 1)?;
                        let value = self.expr(frame, value, depth + 1)?;
                        intrinsics::call(
                            ParserProgramIntrinsic::TableInsert,
                            &[table, value],
                            self.heap,
                            self.patterns,
                            &self.limits,
                        )?;
                        pc += 1;
                    }
                    Op::Call { call } => {
                        self.call(frame, call, depth + 1)?;
                        pc += 1;
                    }
                    Op::Return { values } => {
                        return Ok(Some(self.values(frame, values, depth + 1)?));
                    }
                    Op::Fallthrough => return Ok(Some(Vec::new())),
                    Op::Jump { target } => pc = *target,
                    Op::JumpIfFalse { condition, target } => {
                        pc = if self.expr(frame, condition, depth + 1)?.truthy() {
                            pc + 1
                        } else {
                            *target
                        };
                    }
                    Op::NumericInit {
                        state,
                        local,
                        start,
                        limit,
                        step,
                        exit,
                    } => {
                        // All operands run before the first source coercion.
                        let start = self.expr(frame, start, depth + 1)?;
                        let limit = self.expr(frame, limit, depth + 1)?;
                        let step = self.expr(frame, step, depth + 1)?;
                        let start = number(&start, self.patterns)?
                            .ok_or_else(|| Error::source("for initial value must be a number"))?;
                        let limit = number(&limit, self.patterns)?
                            .ok_or_else(|| Error::source("for limit must be a number"))?;
                        let step = number(&step, self.patterns)?
                            .ok_or_else(|| Error::source("for step must be a number"))?;
                        if numeric_admits(start, limit, step) {
                            frame.locals[*local as usize] = V::Number(start);
                            frame.loops[*state] = Some(Loop::Numeric {
                                value: start,
                                limit,
                                step,
                            });
                            pc += 1;
                        } else {
                            frame.loops[*state] = None;
                            pc = *exit;
                        }
                    }
                    Op::NumericNext {
                        state,
                        local,
                        body,
                        exit,
                    } => {
                        let Some(Loop::Numeric { value, limit, step }) = &mut frame.loops[*state]
                        else {
                            return Err(Error::input("numeric loop state missing"));
                        };
                        *value += *step;
                        if numeric_admits(*value, *limit, *step) {
                            frame.locals[*local as usize] = V::Number(*value);
                            pc = *body;
                        } else {
                            frame.loops[*state] = None;
                            pc = *exit;
                        }
                    }
                    Op::IteratorInit {
                        state,
                        locals,
                        iterator,
                        exit,
                    } => {
                        let mut iterator = match iterator {
                            ParserProgramIterator::Dense { table, .. } => {
                                let table = self.expr(frame, table, depth + 1)?;
                                if !matches!(table, V::Table(_)) {
                                    return Err(Error::source("ipairs expects a table"));
                                }
                                Loop::Dense { table, index: 0 }
                            }
                            ParserProgramIterator::Pattern { call } => {
                                let arguments = self.arguments(frame, call, depth + 1)?;
                                Loop::Pattern(Box::new(intrinsics::Gmatch::new(
                                    &arguments,
                                    self.heap,
                                    &self.limits,
                                )?))
                            }
                        };
                        if let Some(values) = self.next(&mut iterator)? {
                            self.assign_loop(frame, locals, values);
                            frame.loops[*state] = Some(iterator);
                            pc += 1;
                        } else {
                            frame.loops[*state] = None;
                            pc = *exit;
                        }
                    }
                    Op::IteratorNext {
                        state,
                        locals,
                        body,
                        exit,
                    } => {
                        let mut iterator = frame.loops[*state]
                            .take()
                            .ok_or_else(|| Error::input("iterator loop state missing"))?;
                        if let Some(values) = self.next(&mut iterator)? {
                            self.assign_loop(frame, locals, values);
                            frame.loops[*state] = Some(iterator);
                            pc = *body;
                        } else {
                            pc = *exit;
                        }
                    }
                }
                Ok(None)
            })();
            if let Some(result) = step.map_err(|e| e.at(plan.callback, location))? {
                return Ok(result);
            }
        }
    }
    fn next(&mut self, iterator: &mut Loop) -> Result<Option<Vec<V>>> {
        match iterator {
            Loop::Dense { table, index } => {
                *index = index
                    .checked_add(1)
                    .ok_or_else(|| Error::resource("dense iterator index"))?;
                let key = V::Number(*index as f64);
                // LuaJIT ipairs reads the raw array/hash slot, bypassing __index.
                let value = self.heap.raw_get(table, &key)?;
                if matches!(value, V::Nil) {
                    Ok(None)
                } else {
                    self.pack_space(2)?;
                    Ok(Some(vec![key, value]))
                }
            }
            Loop::Pattern(iterator) => iterator.next(self.heap, self.patterns),
            Loop::Numeric { .. } => Err(Error::input("numeric state used as iterator")),
        }
    }
    fn assign_loop(&self, frame: &mut Frame, locals: &[u16], values: Vec<V>) {
        for (index, slot) in locals.iter().enumerate() {
            frame.locals[*slot as usize] = values.get(index).cloned().unwrap_or(V::Nil);
        }
    }
    fn values(
        &mut self,
        frame: &mut Frame,
        list: &ParserProgramValueList,
        depth: usize,
    ) -> Result<Vec<V>> {
        self.tick(depth)?;
        self.pack_space(list.values.len())?;
        let mut values = Vec::with_capacity(list.values.len());
        for value in &list.values {
            values.push(self.expr(frame, value, depth + 1)?);
        }
        if let Some(tail) = &list.tail {
            let more = self.pack(frame, tail, depth + 1)?;
            let length = values
                .len()
                .checked_add(more.len())
                .ok_or_else(|| Error::resource("expanded pack size"))?;
            if length > self.limits.max_results {
                return Err(Error::resource("expanded pack size"));
            }
            self.heap.charge_values(more.len())?;
            values
                .try_reserve_exact(more.len())
                .map_err(|_| Error::resource("expanded pack allocation"))?;
            values.extend(more);
        }
        Ok(values)
    }
    fn pack(
        &mut self,
        frame: &mut Frame,
        pack: &ParserProgramPack,
        depth: usize,
    ) -> Result<Vec<V>> {
        self.tick(depth)?;
        match pack {
            ParserProgramPack::Call { call } => self.call(frame, call, depth + 1),
            ParserProgramPack::Varargs => {
                self.pack_space(frame.extra.len())?;
                Ok(frame.extra.clone())
            }
        }
    }
    fn arguments(
        &mut self,
        frame: &mut Frame,
        call: &ParserProgramCall,
        depth: usize,
    ) -> Result<Vec<V>> {
        let receiver = if let Some(receiver) = &call.receiver {
            let value = self.expr(frame, receiver, depth + 1)?;
            let CompiledProgramBinding::Intrinsic { operation, .. } =
                &self.library.0.programs[frame.program].bindings[call.binding as usize]
            else {
                return Err(Error::unsupported("dynamic method target"));
            };
            let target = intrinsics::precheck_method(*operation, &value, self.heap)?;
            Some((value, target))
        } else {
            None
        };
        let mut values = self.values(frame, &call.arguments, depth + 1)?;
        if let Some((receiver, target)) = receiver {
            intrinsics::finish_method(target)?;
            if values.len() >= self.limits.max_results {
                return Err(Error::resource("method argument pack"));
            }
            self.heap.charge_values(1)?;
            values
                .try_reserve_exact(1)
                .map_err(|_| Error::resource("method argument allocation"))?;
            values.insert(0, receiver);
        }
        Ok(values)
    }
    fn call(
        &mut self,
        frame: &mut Frame,
        call: &ParserProgramCall,
        depth: usize,
    ) -> Result<Vec<V>> {
        self.tick(depth)?;
        let binding = &self.library.0.programs[frame.program].bindings[call.binding as usize];
        if matches!(binding, CompiledProgramBinding::DynamicCall) {
            let callee = call
                .receiver
                .as_ref()
                .ok_or_else(|| Error::input("value call has no callee"))?;
            // Function-value resolution precedes argument expressions, but a
            // non-callable value is rejected only after their effects complete.
            let target = self.expr(frame, callee, depth + 1)?;
            let arguments = self.values(frame, &call.arguments, depth + 1)?;
            return self.invoke_value(target, arguments, depth + 1);
        }
        if let CompiledProgramBinding::DynamicMethod { key } = binding {
            let receiver = call
                .receiver
                .as_ref()
                .ok_or_else(|| Error::input("method has no receiver"))?;
            let receiver = self.expr(frame, receiver, depth + 1)?;
            // Lua's SELF instruction resolves the function before argument effects.
            let target = self.lookup_method(&receiver, key.as_bytes())?;
            let mut arguments = self.values(frame, &call.arguments, depth + 1)?;
            self.prepend(&mut arguments, receiver)?;
            return self.invoke_method_target(target, arguments, depth + 1);
        }
        let arguments = self.arguments(frame, call, depth + 1)?;
        let values = match binding {
            CompiledProgramBinding::Program { index, .. } => {
                self.invoke(*index, arguments, depth + 1)?
            }
            CompiledProgramBinding::Intrinsic { operation, .. } => intrinsics::call(
                *operation,
                &arguments,
                self.heap,
                self.patterns,
                &self.limits,
            )?,
            CompiledProgramBinding::LegacyFactory { .. } => {
                return Err(Error::unsupported("raw legacy-factory invocation bridge"));
            }
            CompiledProgramBinding::DynamicMethod { .. } | CompiledProgramBinding::DynamicCall => {
                unreachable!("handled before arguments")
            }
        };
        if values.len() > self.limits.max_results {
            return Err(Error::resource("call result pack"));
        }
        Ok(values)
    }
    pub(super) fn lookup_method(&mut self, receiver: &V, key: &[u8]) -> Result<MethodTarget> {
        if matches!(receiver, V::Bytes(_)) {
            return match key {
                b"match" => Ok(MethodTarget::StringIntrinsic(
                    ParserProgramIntrinsic::StringMatch,
                )),
                b"gsub" => Ok(MethodTarget::StringIntrinsic(
                    ParserProgramIntrinsic::StringGsub,
                )),
                b"gmatch" => Ok(MethodTarget::StringIntrinsic(
                    ParserProgramIntrinsic::StringGmatch,
                )),
                _ => Err(Error::unsupported("unrepresented string method lookup")),
            };
        }
        let key = self.heap.bytes(key)?;
        Ok(MethodTarget::Value(self.heap.get(receiver, &key)?))
    }
    pub(super) fn invoke_method_target(
        &mut self,
        target: MethodTarget,
        arguments: Vec<V>,
        depth: usize,
    ) -> Result<Vec<V>> {
        match target {
            MethodTarget::Value(value) => self.invoke_value(value, arguments, depth),
            MethodTarget::StringIntrinsic(operation) => intrinsics::call(
                operation,
                &arguments,
                self.heap,
                self.patterns,
                &self.limits,
            ),
        }
    }
    pub(super) fn prepend(&mut self, values: &mut Vec<V>, receiver: V) -> Result<()> {
        if values.len() >= self.limits.max_results {
            return Err(Error::resource("method argument pack"));
        }
        self.heap.charge_values(1)?;
        values
            .try_reserve_exact(1)
            .map_err(|_| Error::resource("method argument allocation"))?;
        values.insert(0, receiver);
        Ok(())
    }
    pub(super) fn invoke_value(
        &mut self,
        target: V,
        arguments: Vec<V>,
        depth: usize,
    ) -> Result<Vec<V>> {
        self.tick(depth)?;
        match target {
            V::Closure(closure) => {
                let callback = self.heap.closure_callback(closure)?;
                let index =
                    *self.library.0.callbacks.get(&callback).ok_or_else(|| {
                        Error::unsupported("session closure has no compiled program")
                    })?;
                self.invoke_frame(index, arguments, depth + 1, Some(closure))
            }
            V::Callback(callback) => {
                if self
                    .library
                    .catalog()
                    .owner()
                    .closure_prototype_id(callback)
                    .is_some()
                {
                    return Err(Error::unsupported(
                        "closure prototype is not an instantiated function",
                    ));
                }
                if let Some(classes) = self.library.catalog().owner().classes() {
                    if callback == classes.source.parent_call_callback {
                        return self.parent_call(arguments, depth + 1);
                    }
                    if callback == classes.source.parent_index_callback {
                        let proxy = arguments.first().cloned().unwrap_or(V::Nil);
                        let key = arguments.get(1).cloned().unwrap_or(V::Nil);
                        let value = self.heap.parent_index(&proxy, &key, depth + 1)?;
                        self.pack_space(1)?;
                        return Ok(vec![value]);
                    }
                    if let Some(index) = classes.classes.iter().position(|class| {
                        class
                            .constructor
                            .as_ref()
                            .and_then(|constructor| constructor.wrapper.as_ref())
                            .is_some_and(|wrapper| wrapper.callback == callback)
                    }) {
                        return self.wrapped_constructor(
                            poe_optimizer_data::source_program::SourceClassId(index as u32 + 1),
                            arguments,
                            depth + 1,
                        );
                    }
                }
                if let Some(operation) = self.library.catalog().owner().intrinsic(callback) {
                    return intrinsics::call(
                        operation,
                        &arguments,
                        self.heap,
                        self.patterns,
                        &self.limits,
                    );
                }
                let index = *self.library.0.callbacks.get(&callback).ok_or_else(|| {
                    Error::unsupported(format!(
                        "method callback {callback:?} has no compiled program"
                    ))
                })?;
                self.invoke(index, arguments, depth + 1)
            }
            V::Table(_)
                if matches!(
                    self.heap.behavior(&target),
                    Some(TableBehavior::ParentProxy)
                ) =>
            {
                let callable = self.heap.raw_field(&target, "__call")?;
                if !matches!(callable, V::Callback(_) | V::Closure(_)) {
                    return Err(Error::source("proxy call metamethod is not a function"));
                }
                let mut arguments = arguments;
                self.prepend(&mut arguments, target)?;
                self.invoke_value(callable, arguments, depth + 1)
            }
            V::Table(_)
                if matches!(
                    self.heap.behavior(&target),
                    Some(TableBehavior::Instance { .. })
                ) =>
            {
                match self.heap.behavior(&target) {
                    Some(TableBehavior::Instance {
                        call_fallback:
                            poe_optimizer_data::source_program::SourceTableCallFallback::NonCallable,
                        ..
                    }) => Err(Error::source("attempt to call a non-function value")),
                    _ => Err(Error::unsupported(
                        "class instance call fallback is unavailable",
                    )),
                }
            }
            V::Table(_) => {
                self.heap.ensure_call_fallback(&target)?;
                Err(Error::source("attempt to call a non-function value"))
            }
            _ => Err(Error::source("attempt to call a non-function value")),
        }
    }
    fn parent_call(&mut self, arguments: Vec<V>, depth: usize) -> Result<Vec<V>> {
        self.tick(depth)?;
        if self.call_depth >= self.limits.max_call_depth {
            return Err(Error::resource("program call depth"));
        }
        self.call_depth += 1;
        let result = (|| {
            let owner = self.library.catalog().owner().clone();
            let policy = &owner.classes().expect("parent callback owner").source;
            let proxy = arguments.first().cloned().unwrap_or(V::Nil);
            let receiver = arguments.get(1).cloned().unwrap_or(V::Nil);
            let parent = self.named_get(&proxy, &policy.proxy_parent)?;
            let object = self.named_get(&proxy, &policy.proxy_object)?;
            let _class_name = self.named_get(&proxy, &policy.proxy_class_name)?;
            let name = self.named_get(&parent, &policy.class_name_field)?;
            let constructor = self.heap.get(&parent, &name)?;
            if !constructor.truthy() {
                return Err(Error::source("parent class has no constructor"));
            }
            let initialized = self.named_get(&object, &policy.parent_init)?;
            if self.heap.get(&initialized, &parent)?.truthy() {
                return Err(Error::source("parent class already initialized"));
            }
            if !receiver.lua_equal(&object) {
                return Err(Error::source(
                    "parent constructor was not provided its object",
                ));
            }
            let input = arguments.into_iter().skip(1).collect::<Vec<_>>();
            self.pack_space(input.len())?;
            self.invoke_value(constructor, input, depth + 1)?;
            // The constructor may have replaced this table. Fetch it again.
            let initialized = self.named_get(&object, &policy.parent_init)?;
            self.heap.set(&initialized, parent, V::Boolean(true))?;
            Ok(Vec::new())
        })();
        self.call_depth -= 1;
        result
    }
    fn wrapped_constructor(
        &mut self,
        class_id: poe_optimizer_data::source_program::SourceClassId,
        arguments: Vec<V>,
        depth: usize,
    ) -> Result<Vec<V>> {
        self.tick(depth)?;
        if self.call_depth >= self.limits.max_call_depth {
            return Err(Error::resource("program call depth"));
        }
        self.call_depth += 1;
        let result = (|| {
            let owner = self.library.catalog().owner().clone();
            let class = owner.class(class_id).expect("wrapper class");
            let original = class.constructor.as_ref().expect("wrapper body").callback;
            let receiver = arguments.first().cloned().unwrap_or(V::Nil);
            let result = self.invoke_value(V::Callback(original), arguments, depth + 1)?;
            let value = result.into_iter().next().unwrap_or(V::Nil);
            let ancestors = self.heap.class_super_parents(class_id)?;
            self.pack_space(ancestors.len())?;
            for parent_id in ancestors {
                self.tick(depth + 1)?;
                let parent = owner.class(parent_id).expect("validated parent");
                if parent.constructor.is_some() {
                    let initialized = self.named_get(
                        &receiver,
                        &owner.classes().expect("wrapper policy").source.parent_init,
                    )?;
                    let key = self.heap.definition(parent.table)?;
                    if !self.heap.get(&initialized, &key)?.truthy() {
                        return Err(Error::source("parent class must be initialized"));
                    }
                }
            }
            if !value.truthy() {
                return Err(Error::source("class constructor did not return a value"));
            }
            self.pack_space(1)?;
            Ok(vec![value])
        })();
        self.call_depth -= 1;
        result
    }
    fn named_get(&mut self, table: &V, name: &str) -> Result<V> {
        let key = self.heap.bytes(name.as_bytes())?;
        self.heap.get(table, &key)
    }
    fn expr(&mut self, frame: &mut Frame, expr: &ParserProgramExpr, depth: usize) -> Result<V> {
        let callback = self.library.0.programs[frame.program].callback;
        self.expr_inner(frame, expr, depth)
            .map_err(|error| error.at(callback, expr.location))
    }
    fn expr_inner(
        &mut self,
        frame: &mut Frame,
        expr: &ParserProgramExpr,
        depth: usize,
    ) -> Result<V> {
        self.tick(depth)?;
        use ParserProgramExprKind as E;
        match &expr.operation {
            E::Literal { value } => self.heap.literal(value),
            E::Bytes { value } => self.heap.bytes(value),
            E::Local { local } => Ok(frame.locals[*local as usize].clone()),
            E::Capture { upvalue } => {
                if let Some(closure) = frame.closure {
                    self.heap.closure_capture(closure, *upvalue)
                } else {
                    self.heap
                        .capture(self.library.0.programs[frame.program].callback, *upvalue)
                }
            }
            E::Definition { root } => {
                let id = self
                    .library
                    .catalog()
                    .owner()
                    .definition_id((*root).into())
                    .ok_or_else(|| Error::input("definition root does not belong to this owner"))?;
                self.heap.definition(id)
            }
            E::NamedDefinition { root } => {
                let id = self
                    .library
                    .catalog()
                    .owner()
                    .definition_id(
                        poe_optimizer_data::source_program::SourceProgramDefinitionRoot::Named(
                            *root,
                        ),
                    )
                    .ok_or_else(|| Error::input("definition root does not belong to this owner"))?;
                self.heap.definition(id)
            }
            E::Get { table, key } => {
                let table = self.expr(frame, table, depth + 1)?;
                let key = self.expr(frame, key, depth + 1)?;
                self.heap.get(&table, &key)
            }
            E::Unary { operation, value } => {
                let value = self.expr(frame, value, depth + 1)?;
                match operation {
                    ParserProgramUnary::Not => Ok(V::Boolean(!value.truthy())),
                    ParserProgramUnary::Negate => number(&value, self.patterns)?
                        .map(|n| V::Number(-n))
                        .ok_or_else(|| Error::source("arithmetic on a non-number")),
                    ParserProgramUnary::Length => match &value {
                        V::Bytes(bytes) => Ok(V::Number(bytes.len() as f64)),
                        V::Table(_) => Ok(V::Number(self.heap.dense_len(&value)? as f64)),
                        _ => Err(Error::source("attempt to get length of unsupported value")),
                    },
                }
            }
            E::Binary {
                operation,
                left,
                right,
            } => {
                let left = self.expr(frame, left, depth + 1)?;
                if *operation == ParserProgramBinary::And && !left.truthy()
                    || *operation == ParserProgramBinary::Or && left.truthy()
                {
                    return Ok(left);
                }
                let right = self.expr(frame, right, depth + 1)?;
                self.binary(*operation, left, right)
            }
            E::Call { call } => Ok(self
                .call(frame, call, depth + 1)?
                .into_iter()
                .next()
                .unwrap_or(V::Nil)),
            E::Table { fields } => self.table(frame, fields, depth + 1),
        }
    }
    fn binary(&mut self, operation: ParserProgramBinary, left: V, right: V) -> Result<V> {
        use ParserProgramBinary as Op;
        if matches!(
            operation,
            Op::Equal
                | Op::NotEqual
                | Op::LessThan
                | Op::LessEqual
                | Op::GreaterThan
                | Op::GreaterEqual
        ) && let (V::Bytes(a), V::Bytes(b)) = (&left, &right)
        {
            self.patterns.charge(a.len().min(b.len()) as u64)?;
        }
        match operation {
            Op::And | Op::Or => Ok(right),
            Op::Equal => Ok(V::Boolean(left.lua_equal(&right))),
            Op::NotEqual => Ok(V::Boolean(!left.lua_equal(&right))),
            Op::LessThan | Op::LessEqual | Op::GreaterThan | Op::GreaterEqual => {
                let result = match (&left, &right) {
                    (V::Number(a), V::Number(b)) => match operation {
                        Op::LessThan => a < b,
                        Op::LessEqual => a <= b,
                        Op::GreaterThan => a > b,
                        Op::GreaterEqual => a >= b,
                        _ => unreachable!(),
                    },
                    (V::Bytes(a), V::Bytes(b)) => match operation {
                        Op::LessThan => a < b,
                        Op::LessEqual => a <= b,
                        Op::GreaterThan => a > b,
                        Op::GreaterEqual => a >= b,
                        _ => unreachable!(),
                    },
                    _ => return Err(Error::source("comparison of incompatible values")),
                };
                Ok(V::Boolean(result))
            }
            Op::Concat => {
                let left = string(&left, self.heap)?;
                let right = string(&right, self.heap)?;
                let length = left
                    .len()
                    .checked_add(right.len())
                    .filter(|n| *n <= self.heap.remaining_bytes())
                    .ok_or_else(|| Error::resource("concatenation bytes"))?;
                self.heap.charge_bytes(length)?;
                let mut bytes = Vec::with_capacity(length);
                bytes.extend_from_slice(&left);
                bytes.extend_from_slice(&right);
                self.heap.bytes(&bytes)
            }
            Op::Add | Op::Subtract | Op::Multiply | Op::Divide => {
                let a = number(&left, self.patterns)?
                    .ok_or_else(|| Error::source("arithmetic on a non-number"))?;
                let b = number(&right, self.patterns)?
                    .ok_or_else(|| Error::source("arithmetic on a non-number"))?;
                Ok(V::Number(match operation {
                    Op::Add => a + b,
                    Op::Subtract => a - b,
                    Op::Multiply => a * b,
                    Op::Divide => a / b,
                    _ => unreachable!(),
                }))
            }
            Op::Modulo | Op::Power => Err(Error::unsupported(
                "modulo/power operation requires source arithmetic proof",
            )),
        }
    }
    fn table(
        &mut self,
        frame: &mut Frame,
        fields: &[ParserProgramField],
        depth: usize,
    ) -> Result<V> {
        let table = self.heap.new_table()?;
        let mut index = 1usize;
        for field in fields {
            self.tick(depth)?;
            match field {
                ParserProgramField::Named { key, value } => {
                    let key = self.heap.bytes(key.as_bytes())?;
                    let value = self.expr(frame, value, depth + 1)?;
                    self.heap.set(&table, key, value)?;
                }
                ParserProgramField::Keyed { key, value } => {
                    let key = self.expr(frame, key, depth + 1)?;
                    let value = self.expr(frame, value, depth + 1)?;
                    self.heap.set(&table, key, value)?;
                }
                ParserProgramField::List { value } => {
                    let value = self.expr(frame, value, depth + 1)?;
                    self.heap.set(&table, V::Number(index as f64), value)?;
                    index += 1;
                }
                ParserProgramField::Tail { values } => {
                    for value in self.pack(frame, values, depth + 1)? {
                        self.heap.set(&table, V::Number(index as f64), value)?;
                        index = index
                            .checked_add(1)
                            .ok_or_else(|| Error::resource("table literal index"))?;
                    }
                }
            }
        }
        Ok(table)
    }
}
fn number(value: &V, work: &mut MatchBudget) -> Result<Option<f64>> {
    match value {
        V::Number(n) => Ok(Some(*n)),
        V::Bytes(bytes) => {
            work.charge(bytes.len() as u64)?;
            Ok(crate::lua_number::parse_number(bytes))
        }
        _ => Ok(None),
    }
}
fn string<'a>(value: &'a V, heap: &mut Heap) -> Result<std::borrow::Cow<'a, [u8]>> {
    if let Some(bytes) = value.as_bytes() {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }
    match value {
        V::Number(n) => {
            heap.charge_bytes(128)?;
            Ok(std::borrow::Cow::Owned(
                crate::item_tools::lua_number_text(*n).into_bytes(),
            ))
        }
        _ => Err(Error::source("concatenation of a non-string value")),
    }
}
fn numeric_admits(value: f64, limit: f64, step: f64) -> bool {
    if value.is_nan() || limit.is_nan() {
        return false;
    }
    if step.is_sign_negative() {
        value >= limit
    } else {
        value <= limit
    }
}
