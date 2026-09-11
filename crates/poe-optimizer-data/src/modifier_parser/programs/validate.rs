use super::*;
const MAX_DEPTH: usize = 48;
const MAX_LIST: usize = 4096;
const MAX_NODES: usize = 500_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn error(
    kind: ParserProgramErrorKind,
    program: Option<ParserProgramId>,
    location: Option<ParserProgramLocation>,
    message: impl Into<String>,
) -> ParserProgramError {
    ParserProgramError {
        kind,
        program,
        location,
        message: message.into(),
    }
}
struct Budget {
    nodes: usize,
    bytes: usize,
}
impl Budget {
    fn charge(
        &mut self,
        bytes: usize,
        id: ParserProgramId,
        location: Option<ParserProgramLocation>,
    ) -> ParserProgramResult<()> {
        self.nodes = self.nodes.checked_add(1).ok_or_else(|| {
            error(
                ParserProgramErrorKind::ResourceLimit,
                Some(id),
                location,
                "program node overflow",
            )
        })?;
        self.bytes = self.bytes.checked_add(bytes).ok_or_else(|| {
            error(
                ParserProgramErrorKind::ResourceLimit,
                Some(id),
                location,
                "program byte overflow",
            )
        })?;
        if self.nodes > MAX_NODES || self.bytes > MAX_BYTES {
            return Err(error(
                ParserProgramErrorKind::ResourceLimit,
                Some(id),
                location,
                "aggregate program resource bound",
            ));
        }
        Ok(())
    }
    fn text(
        &mut self,
        value: &str,
        id: ParserProgramId,
        location: Option<ParserProgramLocation>,
    ) -> ParserProgramResult<()> {
        if value.len() > 4096 {
            return Err(error(
                ParserProgramErrorKind::ResourceLimit,
                Some(id),
                location,
                "program text length bound",
            ));
        }
        self.charge(value.len(), id, location)
    }
}
struct Check<'a> {
    data: &'a ParserProgramData,
    owner: &'a ModifierParserCatalog,
    program: &'a ParserProgram,
    id: ParserProgramId,
    budget: &'a mut Budget,
    required: &'a mut BTreeSet<ParserProgramCapability>,
    edges: &'a mut BTreeSet<ParserProgramId>,
    declared: Vec<bool>,
}
impl Check<'_> {
    fn fail(
        &self,
        kind: ParserProgramErrorKind,
        location: Option<ParserProgramLocation>,
        message: impl Into<String>,
    ) -> ParserProgramError {
        error(kind, Some(self.id), location, message)
    }
    fn bound(
        &self,
        n: usize,
        max: usize,
        loc: Option<ParserProgramLocation>,
        name: &str,
    ) -> ParserProgramResult<()> {
        if n > max {
            return Err(self.fail(
                ParserProgramErrorKind::ResourceLimit,
                loc,
                format!("program {name} bound"),
            ));
        }
        Ok(())
    }
    fn location(&self, loc: ParserProgramLocation) -> ParserProgramResult<()> {
        let length = self.program.provenance.function_end - self.program.provenance.function_start;
        if loc.start >= loc.end || loc.end > length {
            return Err(self.fail(
                ParserProgramErrorKind::InvalidData,
                Some(loc),
                "source location outside complete function",
            ));
        }
        Ok(())
    }
    fn visible(
        &self,
        scope: &[bool],
        local: u16,
        loc: ParserProgramLocation,
    ) -> ParserProgramResult<()> {
        if scope.get(local as usize) != Some(&true) {
            return Err(self.fail(
                ParserProgramErrorKind::InvalidData,
                Some(loc),
                "local is outside its lexical declaration scope",
            ));
        }
        Ok(())
    }
    fn declare(
        &mut self,
        scope: &mut Arc<Vec<bool>>,
        locals: &[u16],
        loc: ParserProgramLocation,
    ) -> ParserProgramResult<()> {
        self.bound(locals.len(), 128, Some(loc), "declaration count")?;
        if locals.is_empty() {
            return Err(self.fail(
                ParserProgramErrorKind::InvalidData,
                Some(loc),
                "empty local declaration",
            ));
        }
        for local in locals {
            let Some(declared) = self.declared.get_mut(*local as usize) else {
                return Err(self.fail(
                    ParserProgramErrorKind::InvalidData,
                    Some(loc),
                    "local declaration out of range",
                ));
            };
            if *declared {
                return Err(self.fail(
                    ParserProgramErrorKind::InvalidData,
                    Some(loc),
                    "local slot has multiple lexical declarations",
                ));
            }
            *declared = true;
            Arc::make_mut(scope)[*local as usize] = true;
        }
        Ok(())
    }
    fn binding(
        &self,
        index: u16,
        loc: Option<ParserProgramLocation>,
    ) -> ParserProgramResult<&ParserProgramBinding> {
        self.program.bindings.get(index as usize).ok_or_else(|| {
            self.fail(
                ParserProgramErrorKind::Binding,
                loc,
                "binding index out of range",
            )
        })
    }
    fn captured(&self, upvalue: u16, callback: ParserCallbackId) -> ParserProgramResult<()> {
        let original = self
            .owner
            .callback(self.program.callback)
            .expect("validated program owner");
        if original.upvalues.get(upvalue as usize).map(|v| &v.value)
            != Some(&ParserValue::Callback(callback))
        {
            return Err(self.fail(
                ParserProgramErrorKind::Binding,
                None,
                "callee does not match its owner's captured closure",
            ));
        }
        if self.owner.callback(callback).is_none() {
            return Err(self.fail(
                ParserProgramErrorKind::Binding,
                None,
                "dangling captured callback",
            ));
        }
        Ok(())
    }
    fn bindings(&mut self) -> ParserProgramResult<()> {
        self.bound(self.program.bindings.len(), 1024, None, "binding count")?;
        for binding in &self.program.bindings {
            self.budget.charge(0, self.id, None)?;
            match binding {
                ParserProgramBinding::CapturedCallback { upvalue, callback } => {
                    self.captured(*upvalue, *callback)?;
                    if !self.data.callbacks.contains_key(callback)
                        && !matches!(
                            self.owner.factory(*callback),
                            Some(ParserFactoryDisposition::Pure(_))
                        )
                    {
                        return Err(self.fail(
                            ParserProgramErrorKind::UnsupportedCapability,
                            None,
                            "captured helper has no program or legacy pure definition",
                        ));
                    }
                }
                ParserProgramBinding::Intrinsic { operation, source } => {
                    match source {
                        ParserProgramIntrinsicSource::OriginalGlobal => {
                            let Some(path) = operation.global_path() else {
                                return Err(self.fail(
                                    ParserProgramErrorKind::Binding,
                                    None,
                                    "intrinsic requires a captured source function",
                                ));
                            };
                            let owner = self
                                .owner
                                .callback(self.program.callback)
                                .expect("validated owner");
                            if owner.environment != ParserEnvironment::OriginalGlobals
                                || owner.upvalues.iter().any(|u| u.name == path[0])
                            {
                                return Err(self.fail(
                                    ParserProgramErrorKind::Binding,
                                    None,
                                    "intrinsic global root is shadowed or has another environment",
                                ));
                            }
                        }
                        ParserProgramIntrinsicSource::Captured { upvalue, callback } => {
                            self.captured(*upvalue, *callback)?;
                            if *operation != ParserProgramIntrinsic::CreateMod {
                                return Err(self.fail(
                                    ParserProgramErrorKind::UnsupportedCapability,
                                    None,
                                    "captured intrinsic form is not implemented",
                                ));
                            }
                            let target = self.owner.callback(*callback).expect("validated capture");
                            let span = self
                                .owner
                                .data()
                                .source
                                .construction_spans
                                .get("create_mod")
                                .ok_or_else(|| {
                                    self.fail(
                                        ParserProgramErrorKind::Binding,
                                        None,
                                        "missing original constructor source",
                                    )
                                })?;
                            if target.kind
                                != (ParserCallbackKind::Lua {
                                    source: span.clone(),
                                })
                                || target.environment != ParserEnvironment::OriginalGlobals
                                || !target.upvalues.is_empty()
                            {
                                return Err(self.fail(ParserProgramErrorKind::Binding,None,"constructor is not the authenticated opaque original descriptor"));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn intrinsic(
        &self,
        binding: u16,
        loc: ParserProgramLocation,
    ) -> ParserProgramResult<ParserProgramIntrinsic> {
        match self.binding(binding, Some(loc))? {
            ParserProgramBinding::Intrinsic { operation, .. } => Ok(*operation),
            _ => Err(self.fail(
                ParserProgramErrorKind::Binding,
                Some(loc),
                "iterator requires its declared intrinsic binding",
            )),
        }
    }
    fn call_binding(
        &mut self,
        call: &ParserProgramCall,
        loc: ParserProgramLocation,
    ) -> ParserProgramResult<()> {
        let binding = self.binding(call.binding, Some(loc))?.clone();
        match &binding {
            ParserProgramBinding::CapturedCallback { callback, .. } => {
                if call.receiver.is_some() {
                    return Err(self.fail(
                        ParserProgramErrorKind::UnsupportedCapability,
                        Some(loc),
                        "captured dynamic method lookup is not admitted",
                    ));
                }
                if let Some(target) = self.data.callbacks.get(callback) {
                    self.edges.insert(*target);
                } else {
                    self.required
                        .insert(ParserProgramCapability::LegacyPureCalls);
                }
            }
            ParserProgramBinding::Intrinsic { operation, .. } => {
                if call.receiver.is_some() && !operation.is_string_method() {
                    return Err(self.fail(
                        ParserProgramErrorKind::Binding,
                        Some(loc),
                        "intrinsic does not have an admitted string method form",
                    ));
                }
                if *operation == ParserProgramIntrinsic::StringGmatch {
                    self.required.insert(ParserProgramCapability::PatternFor);
                }
                if *operation == ParserProgramIntrinsic::Ipairs {
                    self.required.insert(ParserProgramCapability::DenseFor);
                }
            }
        }
        Ok(())
    }
    fn expression(
        &mut self,
        expr: &ParserProgramExpr,
        scope: &[bool],
        depth: usize,
    ) -> ParserProgramResult<()> {
        self.walk(vec![ValueWork::Expr(expr, depth)], scope)
    }
    fn values(
        &mut self,
        values: &ParserProgramValueList,
        scope: &[bool],
        depth: usize,
        loc: ParserProgramLocation,
    ) -> ParserProgramResult<()> {
        self.walk(vec![ValueWork::Values(values, depth, loc)], scope)
    }
    fn call(
        &mut self,
        call: &ParserProgramCall,
        scope: &[bool],
        depth: usize,
        loc: ParserProgramLocation,
    ) -> ParserProgramResult<()> {
        self.walk(vec![ValueWork::Call(call, depth, loc)], scope)
    }
    fn walk<'b>(
        &mut self,
        mut work: Vec<ValueWork<'b>>,
        scope: &[bool],
    ) -> ParserProgramResult<()> {
        while let Some(task) = work.pop() {
            let (depth, loc) = match &task {
                ValueWork::Expr(e, d) => (*d, e.location),
                ValueWork::Values(_, d, l)
                | ValueWork::Call(_, d, l)
                | ValueWork::Pack(_, d, l) => (*d, *l),
            };
            self.bound(depth, MAX_DEPTH, Some(loc), "structural depth")?;
            self.location(loc)?;
            self.budget.charge(0, self.id, Some(loc))?;
            match task {
                ValueWork::Values(values, _, _) => {
                    self.bound(values.values.len(), MAX_LIST, Some(loc), "value list")?;
                    if let Some(tail) = &values.tail {
                        work.push(ValueWork::Pack(tail, depth + 1, loc));
                    }
                    for e in values.values.iter().rev() {
                        work.push(ValueWork::Expr(e, depth + 1));
                    }
                }
                ValueWork::Pack(pack, _, _) => match pack {
                    ParserProgramPack::Call { call } => {
                        work.push(ValueWork::Call(call, depth + 1, loc))
                    }
                    ParserProgramPack::Varargs => {
                        if !self.program.variadic {
                            return Err(self.fail(
                                ParserProgramErrorKind::InvalidData,
                                Some(loc),
                                "varargs read in a fixed-parameter function",
                            ));
                        }
                        self.required.insert(ParserProgramCapability::Varargs);
                    }
                },
                ValueWork::Call(call, _, _) => {
                    self.call_binding(call, loc)?;
                    work.push(ValueWork::Values(&call.arguments, depth + 1, loc));
                    if let Some(receiver) = &call.receiver {
                        work.push(ValueWork::Expr(receiver, depth + 1));
                    }
                }
                ValueWork::Expr(e, _) => match &e.operation {
                    ParserProgramExprKind::Literal { value } => match value {
                        ParserFactoryLiteral::Number(v) if !v.is_finite() => {
                            return Err(self.fail(
                                ParserProgramErrorKind::InvalidData,
                                Some(loc),
                                "nonfinite literal requires explicit sentinel",
                            ));
                        }
                        ParserFactoryLiteral::Text(v) => self.budget.text(v, self.id, Some(loc))?,
                        _ => {}
                    },
                    ParserProgramExprKind::Bytes { value } => {
                        self.bound(value.len(), 4096, Some(loc), "byte literal")?;
                        self.budget.charge(value.len(), self.id, Some(loc))?;
                    }
                    ParserProgramExprKind::Local { local } => self.visible(scope, *local, loc)?,
                    ParserProgramExprKind::Capture { upvalue } => {
                        if *upvalue as usize
                            >= self
                                .owner
                                .callback(self.program.callback)
                                .expect("validated owner")
                                .upvalues
                                .len()
                        {
                            return Err(self.fail(
                                ParserProgramErrorKind::Binding,
                                Some(loc),
                                "captured value slot out of range",
                            ));
                        }
                    }
                    ParserProgramExprKind::Definition { .. } => {}
                    ParserProgramExprKind::Get { table, key } => {
                        work.push(ValueWork::Expr(key, depth + 1));
                        work.push(ValueWork::Expr(table, depth + 1));
                    }
                    ParserProgramExprKind::Unary { value, .. } => {
                        work.push(ValueWork::Expr(value, depth + 1))
                    }
                    ParserProgramExprKind::Binary { left, right, .. } => {
                        work.push(ValueWork::Expr(right, depth + 1));
                        work.push(ValueWork::Expr(left, depth + 1));
                    }
                    ParserProgramExprKind::Call { call } => {
                        work.push(ValueWork::Call(call, depth + 1, loc))
                    }
                    ParserProgramExprKind::Table { fields } => {
                        self.bound(fields.len(), MAX_LIST, Some(loc), "table literal fields")?;
                        for (index, field) in fields.iter().enumerate().rev() {
                            self.budget.charge(0, self.id, Some(loc))?;
                            match field {
                                ParserProgramField::Named { key, value } => {
                                    self.budget.text(key, self.id, Some(loc))?;
                                    work.push(ValueWork::Expr(value, depth + 1));
                                }
                                ParserProgramField::Keyed { key, value } => {
                                    work.push(ValueWork::Expr(value, depth + 1));
                                    work.push(ValueWork::Expr(key, depth + 1));
                                }
                                ParserProgramField::List { value } => {
                                    work.push(ValueWork::Expr(value, depth + 1))
                                }
                                ParserProgramField::Tail { values } => {
                                    if index + 1 != fields.len() {
                                        return Err(self.fail(
                                            ParserProgramErrorKind::InvalidData,
                                            Some(loc),
                                            "table result-pack expansion must be final",
                                        ));
                                    }
                                    work.push(ValueWork::Pack(values, depth + 1, loc));
                                }
                            }
                        }
                    }
                },
            }
        }
        Ok(())
    }
    fn body(&mut self) -> ParserProgramResult<()> {
        let mut visible = vec![false; self.program.local_count as usize];
        for value in visible
            .iter_mut()
            .take(self.program.parameter_count as usize)
        {
            *value = true;
        }
        self.declared.clone_from(&visible);
        let mut work = vec![Frame {
            body: &self.program.body,
            next: 0,
            visible: Arc::new(visible),
            depth: 0,
            loops: 0,
        }];
        while let Some(mut frame) = work.pop() {
            self.bound(frame.depth, MAX_DEPTH, None, "block depth")?;
            if frame.next == 0 {
                self.bound(frame.body.len(), MAX_LIST, None, "block statement count")?;
                self.budget.charge(0, self.id, None)?;
            }
            let Some(stmt) = frame.body.get(frame.next) else {
                continue;
            };
            frame.next += 1;
            let loc = stmt.location;
            self.location(loc)?;
            self.budget.charge(0, self.id, Some(loc))?;
            let depth = frame.depth + 1;
            let mut children = Vec::new();
            match &stmt.operation {
                ParserProgramStatementKind::Declare { locals, values } => {
                    self.values(values, &frame.visible, depth, loc)?;
                    self.declare(&mut frame.visible, locals, loc)?;
                }
                ParserProgramStatementKind::Assign { locals, values } => {
                    self.bound(locals.len(), 128, Some(loc), "assignment count")?;
                    if locals.is_empty() {
                        return Err(self.fail(
                            ParserProgramErrorKind::InvalidData,
                            Some(loc),
                            "empty assignment",
                        ));
                    }
                    for local in locals {
                        self.visible(&frame.visible, *local, loc)?;
                    }
                    self.values(values, &frame.visible, depth, loc)?;
                }
                ParserProgramStatementKind::If {
                    branches,
                    otherwise,
                } => {
                    self.bound(branches.len(), MAX_LIST, Some(loc), "branch count")?;
                    if branches.is_empty() {
                        return Err(self.fail(
                            ParserProgramErrorKind::InvalidData,
                            Some(loc),
                            "conditional has no branch",
                        ));
                    }
                    for branch in branches {
                        self.expression(&branch.condition, &frame.visible, depth)?;
                        children.push(Frame {
                            body: &branch.body,
                            next: 0,
                            visible: frame.visible.clone(),
                            depth,
                            loops: frame.loops,
                        });
                    }
                    children.push(Frame {
                        body: otherwise,
                        next: 0,
                        visible: frame.visible.clone(),
                        depth,
                        loops: frame.loops,
                    });
                }
                ParserProgramStatementKind::ForNumeric {
                    local,
                    start,
                    limit,
                    step,
                    body,
                } => {
                    self.required.insert(ParserProgramCapability::NumericFor);
                    for expr in [start, limit, step] {
                        self.expression(expr, &frame.visible, depth)?;
                    }
                    let mut scope = frame.visible.clone();
                    self.declare(&mut scope, &[*local], loc)?;
                    children.push(Frame {
                        body,
                        next: 0,
                        visible: scope,
                        depth,
                        loops: frame.loops + 1,
                    });
                }
                ParserProgramStatementKind::ForEach {
                    locals,
                    iterator,
                    body,
                } => {
                    match iterator {
                        ParserProgramIterator::Dense { table, binding } => {
                            if self.intrinsic(*binding, loc)? != ParserProgramIntrinsic::Ipairs {
                                return Err(self.fail(
                                    ParserProgramErrorKind::Binding,
                                    Some(loc),
                                    "dense iterator is not bound to ipairs",
                                ));
                            }
                            self.required.insert(ParserProgramCapability::DenseFor);
                            self.expression(table, &frame.visible, depth)?;
                        }
                        ParserProgramIterator::Pattern { call } => {
                            if self.intrinsic(call.binding, loc)?
                                != ParserProgramIntrinsic::StringGmatch
                            {
                                return Err(self.fail(
                                    ParserProgramErrorKind::Binding,
                                    Some(loc),
                                    "pattern iterator is not bound to string.gmatch",
                                ));
                            }
                            self.required.insert(ParserProgramCapability::PatternFor);
                            self.call(call, &frame.visible, depth, loc)?;
                        }
                    }
                    let mut scope = frame.visible.clone();
                    self.declare(&mut scope, locals, loc)?;
                    children.push(Frame {
                        body,
                        next: 0,
                        visible: scope,
                        depth,
                        loops: frame.loops + 1,
                    });
                }
                ParserProgramStatementKind::TableSet { table, key, value } => {
                    for expr in [table, key, value] {
                        self.expression(expr, &frame.visible, depth)?;
                    }
                }
                ParserProgramStatementKind::TableAppend {
                    binding,
                    table,
                    value,
                } => {
                    if self.intrinsic(*binding, loc)? != ParserProgramIntrinsic::TableInsert {
                        return Err(self.fail(
                            ParserProgramErrorKind::Binding,
                            Some(loc),
                            "append is not bound to table.insert",
                        ));
                    }
                    for expr in [table, value] {
                        self.expression(expr, &frame.visible, depth)?;
                    }
                }
                ParserProgramStatementKind::Call { call } => {
                    self.call(call, &frame.visible, depth, loc)?
                }
                ParserProgramStatementKind::Return { values } => {
                    self.values(values, &frame.visible, depth, loc)?
                }
                ParserProgramStatementKind::Break => {
                    if frame.loops == 0 {
                        return Err(self.fail(
                            ParserProgramErrorKind::InvalidData,
                            Some(loc),
                            "break outside a lexical loop",
                        ));
                    }
                }
            }
            work.push(frame);
            work.extend(children.into_iter().rev());
        }
        Ok(())
    }
}
enum ValueWork<'a> {
    Expr(&'a ParserProgramExpr, usize),
    Values(&'a ParserProgramValueList, usize, ParserProgramLocation),
    Pack(&'a ParserProgramPack, usize, ParserProgramLocation),
    Call(&'a ParserProgramCall, usize, ParserProgramLocation),
}
struct Frame<'a> {
    body: &'a [ParserProgramStatement],
    next: usize,
    visible: Arc<Vec<bool>>,
    depth: usize,
    loops: usize,
}

pub(super) fn validate(
    data: &ParserProgramData,
    owner: &ModifierParserCatalog,
) -> ParserProgramResult<BTreeSet<ParserProgramCapability>> {
    if data.schema_version != PARSER_PROGRAM_SCHEMA_VERSION {
        return Err(error(
            ParserProgramErrorKind::InvalidData,
            None,
            None,
            "unsupported program schema",
        ));
    }
    if data.programs.len() > 20_000 {
        return Err(error(
            ParserProgramErrorKind::ResourceLimit,
            None,
            None,
            "program count bound",
        ));
    }
    if data.callbacks.len() != data.programs.len() {
        return Err(error(
            ParserProgramErrorKind::InvalidData,
            None,
            None,
            "program callback mapping must be complete and one-to-one",
        ));
    }
    let mut budget = Budget { nodes: 0, bytes: 0 };
    for (index, program) in data.programs.iter().enumerate() {
        let id = ParserProgramId(index as u32 + 1);
        if data.callbacks.get(&program.callback) != Some(&id) {
            return Err(error(
                ParserProgramErrorKind::Binding,
                Some(id),
                None,
                "program has no exact owner mapping",
            ));
        }
        if program.local_count > 1024 || program.parameter_count > 128 {
            return Err(error(
                ParserProgramErrorKind::ResourceLimit,
                Some(id),
                None,
                "local/parameter count bound",
            ));
        }
        if program.parameter_count > program.local_count {
            return Err(error(
                ParserProgramErrorKind::InvalidData,
                Some(id),
                None,
                "parameters exceed allocated lexical slots",
            ));
        }
        let Some(callback) = owner.callback(program.callback) else {
            return Err(error(
                ParserProgramErrorKind::Binding,
                Some(id),
                None,
                "missing source callback",
            ));
        };
        if matches!(
            owner.factory(program.callback),
            Some(ParserFactoryDisposition::Pure(_))
        ) {
            return Err(error(
                ParserProgramErrorKind::Binding,
                Some(id),
                None,
                "executable Pure/program overlap",
            ));
        }
        let p = &program.provenance;
        if callback.kind
            != (ParserCallbackKind::Lua {
                source: p.source.clone(),
            })
            || callback.environment != ParserEnvironment::OriginalGlobals
        {
            return Err(error(
                ParserProgramErrorKind::Binding,
                Some(id),
                None,
                "program source differs from its original closure",
            ));
        }
        if p.function_start >= p.function_end
            || p.function_end > 1024 * 1024
            || !digest(&p.function_sha256, 64)
        {
            return Err(error(
                ParserProgramErrorKind::InvalidData,
                Some(id),
                None,
                "invalid complete program function extent/hash",
            ));
        }
        for text in [&p.source.path, &p.source.sha256, &p.function_sha256] {
            budget.text(text, id, None)?;
        }
    }
    let mut required = BTreeSet::new();
    if !data.programs.is_empty() {
        required.insert(ParserProgramCapability::Core);
    }
    let mut graph = vec![BTreeSet::new(); data.programs.len()];
    for (index, program) in data.programs.iter().enumerate() {
        let mut check = Check {
            data,
            owner,
            program,
            id: ParserProgramId(index as u32 + 1),
            budget: &mut budget,
            required: &mut required,
            edges: &mut graph[index],
            declared: vec![false; program.local_count as usize],
        };
        check.bindings()?;
        check.body()?;
    }
    // Detect recursion without recursive validation or eager helper inlining.
    // A compiler/runtime can reject this staged capability or enforce call depth.
    let mut state = vec![0u8; graph.len()];
    for root in 0..graph.len() {
        if state[root] != 0 {
            continue;
        }
        let mut work = vec![(root, false)];
        while let Some((node, exit)) = work.pop() {
            if exit {
                state[node] = 2;
                continue;
            }
            if state[node] == 1 {
                required.insert(ParserProgramCapability::RecursiveCalls);
                continue;
            }
            if state[node] == 2 {
                continue;
            }
            state[node] = 1;
            work.push((node, true));
            for target in graph[node].iter().rev() {
                work.push((target.0 as usize - 1, false));
            }
        }
    }
    Ok(required)
}
