use super::super::{CompiledParserPrograms, CompiledProgramBinding, ProgramOperation};
use super::{
    ProgramAllocationUsage, ProgramLimits, ProgramOutput, ProgramRuntimeError as Error,
    ProgramValueGraph, RuntimeResult as Result, intrinsics,
    value::{Heap, V},
};
use crate::lua_pattern::MatchBudget;
use poe_optimizer_data::modifier_parser::*;

// Bounds Rust interpreter recursion independently of caller-configured call limits.
const MAX_EVALUATOR_NESTING: usize = 64;

impl CompiledParserPrograms {
    /// Execute a raw source-bound program. This deliberately does not apply
    /// Special/Prefix/ModTag packing, parser caching or public recursive copying.
    pub fn execute(
        &self,
        callback: ParserCallbackId,
        input: &ProgramValueGraph,
        limits: ProgramLimits,
    ) -> Result<ProgramOutput> {
        let index = *self
            .0
            .callbacks
            .get(&callback)
            .ok_or_else(|| Error::input("callback has no compiled program"))?;
        if input.values.len() > limits.max_results {
            return Err(Error::resource("argument pack size"));
        }
        let (heap, arguments) = Heap::new(self.catalog().owner(), input, &limits)?;
        let mut run = Run {
            library: self,
            heap,
            patterns: MatchBudget::new(limits.pattern),
            limits,
            steps: 0,
            call_depth: 0,
        };
        let values = run.invoke(index, arguments, 0)?;
        let graph = run.heap.freeze(&values)?;
        let used = run.heap.stats();
        Ok(ProgramOutput {
            graph,
            owner: self.catalog().owner().clone(),
            steps: run.steps,
            pattern_steps: run.patterns.steps_used(),
            allocations: ProgramAllocationUsage {
                values: used.values,
                bytes: used.bytes,
                tables: used.tables,
            },
        })
    }
}
struct Run<'a> {
    library: &'a CompiledParserPrograms,
    heap: Heap,
    patterns: MatchBudget,
    limits: ProgramLimits,
    steps: u64,
    call_depth: usize,
}
struct Frame {
    program: usize,
    locals: Vec<V>,
    extra: Vec<V>,
    loops: Vec<Option<Loop>>,
}
enum Loop {
    Numeric { value: f64, limit: f64, step: f64 },
    Dense { table: V, index: u64 },
    Pattern(Box<intrinsics::Gmatch>),
}
impl Run<'_> {
    fn tick(&mut self, depth: usize) -> Result<()> {
        if depth > MAX_EVALUATOR_NESTING {
            return Err(Error::resource("combined expression/call nesting"));
        }
        self.steps = self
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
    fn invoke(&mut self, index: usize, arguments: Vec<V>, depth: usize) -> Result<Vec<V>> {
        self.tick(depth)?;
        if self.call_depth >= self.limits.max_call_depth {
            return Err(Error::resource("program call depth"));
        }
        let plan = &self.library.0.programs[index];
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
                            &mut self.heap,
                            &mut self.patterns,
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
                        let start = number(&start, &mut self.patterns)?
                            .ok_or_else(|| Error::source("for initial value must be a number"))?;
                        let limit = number(&limit, &mut self.patterns)?
                            .ok_or_else(|| Error::source("for limit must be a number"))?;
                        let step = number(&step, &mut self.patterns)?
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
                                    &mut self.heap,
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
                let value = self.heap.get(table, &key)?;
                if matches!(value, V::Nil) {
                    Ok(None)
                } else {
                    self.pack_space(2)?;
                    Ok(Some(vec![key, value]))
                }
            }
            Loop::Pattern(iterator) => iterator.next(&mut self.heap, &mut self.patterns),
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
            let target = intrinsics::precheck_method(*operation, &value, &mut self.heap)?;
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
        let arguments = self.arguments(frame, call, depth + 1)?;
        let binding = &self.library.0.programs[frame.program].bindings[call.binding as usize];
        let values = match binding {
            CompiledProgramBinding::Program { index, .. } => {
                self.invoke(*index, arguments, depth + 1)?
            }
            CompiledProgramBinding::Intrinsic { operation, .. } => intrinsics::call(
                *operation,
                &arguments,
                &mut self.heap,
                &mut self.patterns,
                &self.limits,
            )?,
            CompiledProgramBinding::LegacyFactory { .. } => {
                return Err(Error::unsupported("raw legacy-factory invocation bridge"));
            }
        };
        if values.len() > self.limits.max_results {
            return Err(Error::resource("call result pack"));
        }
        Ok(values)
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
            E::Capture { upvalue } => self
                .heap
                .capture(self.library.0.programs[frame.program].callback, *upvalue),
            E::Definition { root } => {
                let owner = self.library.catalog().owner();
                let id = match root {
                    ParserProgramDefinitionRoot::ModFlags => owner.data().policy.mod_flags,
                    ParserProgramDefinitionRoot::KeywordFlags => owner.data().policy.keyword_flags,
                    ParserProgramDefinitionRoot::SkillTypes => owner.data().policy.skill_types,
                    ParserProgramDefinitionRoot::GemIdLookup => {
                        owner.data().dictionaries[&ParserDictionary::GemIdLookup]
                    }
                };
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
                    ParserProgramUnary::Negate => number(&value, &mut self.patterns)?
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
            Op::Equal | Op::NotEqual | Op::LessThan | Op::LessEqual
        ) && let (V::Bytes(a), V::Bytes(b)) = (&left, &right)
        {
            self.patterns.charge(a.len().min(b.len()) as u64)?;
        }
        match operation {
            Op::And | Op::Or => Ok(right),
            Op::Equal => Ok(V::Boolean(left.lua_equal(&right))),
            Op::NotEqual => Ok(V::Boolean(!left.lua_equal(&right))),
            Op::LessThan | Op::LessEqual => {
                let result = match (&left, &right) {
                    (V::Number(a), V::Number(b)) => {
                        if operation == Op::LessThan {
                            a < b
                        } else {
                            a <= b
                        }
                    }
                    (V::Bytes(a), V::Bytes(b)) => {
                        if operation == Op::LessThan {
                            a < b
                        } else {
                            a <= b
                        }
                    }
                    _ => return Err(Error::source("comparison of incompatible values")),
                };
                Ok(V::Boolean(result))
            }
            Op::Concat => {
                let left = string(&left, &mut self.heap)?;
                let right = string(&right, &mut self.heap)?;
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
                let a = number(&left, &mut self.patterns)?
                    .ok_or_else(|| Error::source("arithmetic on a non-number"))?;
                let b = number(&right, &mut self.patterns)?
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
