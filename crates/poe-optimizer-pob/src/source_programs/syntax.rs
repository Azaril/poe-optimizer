use super::lowering::*;
type Expr = ParserProgramExpr;
type Statement = ParserProgramStatement;

struct Info {
    value: Expr,
    expansion: Option<ParserProgramPack>,
    target: Option<ParserProgramBinding>,
    height: usize,
}
impl Info {
    fn scalar(self) -> LowerResult<Expr> {
        if matches!(self.expansion, Some(ParserProgramPack::Varargs)) {
            return Err("scalar vararg adjustment is not represented".into());
        }
        Ok(self.value)
    }
}
enum Term<'a> {
    Value(Info),
    Global { path: Vec<&'a str>, start: usize },
}
impl Term<'_> {
    fn value(self) -> LowerResult<Info> {
        match self {
            Self::Value(value) => Ok(value),
            Self::Global { path, .. } => {
                Err(format!("unsupported global value {}", path.join(".")))
            }
        }
    }
}
pub(crate) struct Lowerer<'a, 'b> {
    lua: &'a Lua,
    body: &'a str,
    tokens: Vec<Lex<'a>>,
    at: usize,
    callback_id: ParserCallbackId,
    callback: &'a ParserCallback,
    authorization: &'a LoweringBindings,
    budget: &'b mut Budget,
    scopes: Vec<BTreeMap<&'a str, u16>>,
    bindings: Vec<ParserProgramBinding>,
    slots: u16,
    parameters: u16,
    variadic: bool,
    function_start: usize,
    block_depth: usize,
    loop_depth: usize,
}
impl<'a, 'b> Lowerer<'a, 'b> {
    pub(crate) fn new(
        lua: &'a Lua,
        body: &'a str,
        callback_id: ParserCallbackId,
        callback: &'a ParserCallback,
        authorization: &'a LoweringBindings,
        budget: &'b mut Budget,
    ) -> LowerResult<Self> {
        if callback.environment != ParserEnvironment::OriginalGlobals {
            return Err("program callback does not use original globals".into());
        }
        let tokens = lex(body, budget)?;
        Ok(Self {
            lua,
            body,
            tokens,
            at: 0,
            callback_id,
            callback,
            authorization,
            budget,
            scopes: vec![BTreeMap::new()],
            bindings: vec![],
            slots: 0,
            parameters: 0,
            variadic: false,
            function_start: 0,
            block_depth: 0,
            loop_depth: 0,
        })
    }
    fn peek(&self) -> &str {
        self.tokens.get(self.at).map_or("", |t| t.text)
    }
    fn next_text(&self) -> &str {
        self.tokens.get(self.at + 1).map_or("", |t| t.text)
    }
    fn offset(&self) -> usize {
        self.tokens
            .get(self.at)
            .map_or(self.body.len(), |t| t.start)
    }
    fn end(&self) -> usize {
        self.tokens
            .get(self.at.saturating_sub(1))
            .map_or(self.function_start + 1, |t| t.end)
    }
    fn take(&mut self, expected: &str) -> LowerResult<()> {
        if self.peek() != expected {
            return Err(format!("expected {expected}; found {}", self.peek()));
        }
        self.at += 1;
        Ok(())
    }
    fn name(&mut self) -> LowerResult<&'a str> {
        let token = self.tokens.get(self.at).ok_or("missing identifier")?;
        if token.quoted || !identifier(token.text) {
            return Err(format!("expected identifier; found {}", token.text));
        }
        self.at += 1;
        Ok(token.text)
    }
    fn location(&self, start: usize, end: usize) -> ParserProgramLocation {
        ParserProgramLocation {
            start: (start - self.function_start) as u32,
            end: (end - self.function_start) as u32,
        }
    }
    fn node(
        &mut self,
        operation: ParserProgramExprKind,
        start: usize,
        end: usize,
        height: usize,
    ) -> LowerResult<Info> {
        self.budget.node()?;
        if height + self.block_depth + 4 > 40 {
            return Err("typed expression/block depth bound".into());
        }
        Ok(Info {
            value: Expr {
                location: self.location(start, end),
                operation,
            },
            expansion: None,
            target: None,
            height,
        })
    }
    fn statement(
        &mut self,
        operation: ParserProgramStatementKind,
        start: usize,
    ) -> LowerResult<Statement> {
        self.budget.node()?;
        Ok(Statement {
            location: self.location(start, self.end()),
            operation,
        })
    }
    fn declare(&mut self, name: &'a str) -> LowerResult<u16> {
        if self.slots >= 1024 {
            return Err("program local slot bound".into());
        }
        let slot = self.slots;
        self.slots += 1;
        self.scopes.last_mut().unwrap().insert(name, slot);
        Ok(slot)
    }
    fn local(&self, name: &str) -> Option<u16> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }
    fn bind(&mut self, binding: ParserProgramBinding) -> LowerResult<u16> {
        if let Some(index) = self.bindings.iter().position(|b| *b == binding) {
            return Ok(index as u16);
        }
        if self.bindings.len() >= 1024 {
            return Err("program binding bound".into());
        }
        self.bindings.push(binding);
        Ok((self.bindings.len() - 1) as u16)
    }
    fn global_binding(&mut self, operation: ParserProgramIntrinsic) -> LowerResult<u16> {
        if self.authorization.environment.is_some() {
            return Err("global intrinsic shortcut bypasses observed environment".into());
        }
        let path = operation
            .global_path()
            .ok_or("intrinsic has no global source binding")?;
        if self.callback.upvalues.iter().any(|u| u.name == path[0]) {
            return Err(format!("intrinsic global {} is captured/shadowed", path[0]));
        }
        self.bind(ParserProgramBinding::Intrinsic {
            operation,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        })
    }
    pub(crate) fn program(mut self, span: &ItemSourceSpan) -> LowerResult<ParserProgram> {
        let positions = self
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.quoted && t.text == "function")
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if positions.len() != 1 {
            return Err("source span must contain exactly one complete function".into());
        }
        let index = positions[0];
        if self.authorization.standalone_calls {
            // Lua debug spans cover full lines, so a table-inline callback often
            // ends with `end },`. Isolate only the balanced function, retaining
            // original byte offsets and rejecting ambiguous function occurrences.
            // The parser adapter intentionally retains its reviewed span policy.
            let end = complete_function_token_end(&self.tokens, index)?;
            self.tokens.truncate(end);
        }
        self.function_start = self.tokens[index].start;
        self.at = index + 1;
        if self.peek() != "(" {
            self.name()?;
            while self.peek() == "." {
                self.at += 1;
                self.name()?;
            }
            if self.peek() == ":" {
                if !self.authorization.implicit_self {
                    return Err("method function definitions require implicit-self lowering".into());
                }
                self.at += 1;
                self.name()?;
                self.declare("self")?;
                self.parameters += 1;
            }
        }
        self.take("(")?;
        if self.peek() != ")" {
            loop {
                if self.peek() == "..." {
                    self.variadic = true;
                    self.at += 1;
                    break;
                }
                if self.parameters >= 128 {
                    return Err("program parameter bound".into());
                }
                let name = self.name()?;
                self.declare(name)?;
                self.parameters += 1;
                if self.peek() != "," {
                    break;
                }
                self.at += 1;
            }
        }
        self.take(")")?;
        let body = self.block(&["end"], false)?;
        self.take("end")?;
        let function_end = self.end();
        while matches!(self.peek(), "," | ";") {
            self.at += 1;
        }
        if self.at != self.tokens.len() {
            return Err("unconsumed source after complete function".into());
        }
        Ok(ParserProgram {
            callback: self.callback_id,
            parameter_count: self.parameters,
            variadic: self.variadic,
            local_count: self.slots,
            bindings: self.bindings,
            body,
            provenance: ParserProgramProvenance {
                source: span.clone(),
                function_start: self.function_start as u32,
                function_end: function_end as u32,
                function_sha256: hash(&self.body.as_bytes()[self.function_start..function_end]),
            },
        })
    }
    fn block(&mut self, stops: &[&str], scoped: bool) -> LowerResult<Vec<Statement>> {
        self.block_depth += 1;
        if self.block_depth > 16 {
            return Err("program block depth bound".into());
        }
        if scoped {
            self.scopes.push(BTreeMap::new());
        }
        let mut body = Vec::new();
        while !stops.contains(&self.peek()) {
            if self.peek().is_empty() {
                return Err("unterminated function/control block".into());
            }
            if self.peek() == ";" {
                self.at += 1;
                continue;
            }
            if body.len() >= 4096 {
                return Err("program block length bound".into());
            }
            let terminal = matches!(self.peek(), "return" | "break");
            body.push(self.stmt()?);
            if self.peek() == ";" {
                self.at += 1;
            }
            if terminal && !stops.contains(&self.peek()) {
                return Err("statement after return/break in the same Lua block".into());
            }
        }
        if scoped {
            self.scopes.pop();
        }
        self.block_depth -= 1;
        Ok(body)
    }
    fn stmt(&mut self) -> LowerResult<Statement> {
        let start = self.offset();
        use ParserProgramStatementKind as S;
        let operation = match self.peek() {
            "local" => {
                self.at += 1;
                if self.peek() == "function" {
                    return Err("nested local function is unsupported".into());
                }
                let mut names = vec![self.name()?];
                while self.peek() == "," {
                    self.at += 1;
                    names.push(self.name()?);
                    if names.len() > 128 {
                        return Err("declaration count bound".into());
                    }
                }
                let values = if self.peek() == "=" {
                    self.at += 1;
                    self.values(0)?
                } else {
                    ParserProgramValueList::default()
                };
                let locals = names
                    .into_iter()
                    .map(|n| self.declare(n))
                    .collect::<LowerResult<Vec<_>>>()?;
                S::Declare { locals, values }
            }
            "if" => {
                self.at += 1;
                let mut branches = vec![];
                loop {
                    let condition = self.expr(0, 0)?.scalar()?;
                    self.take("then")?;
                    let body = self.block(&["elseif", "else", "end"], true)?;
                    branches.push(ParserProgramBranch { condition, body });
                    if self.peek() != "elseif" {
                        break;
                    }
                    self.at += 1;
                    if branches.len() >= 4096 {
                        return Err("conditional branch bound".into());
                    }
                }
                let otherwise = if self.peek() == "else" {
                    self.at += 1;
                    self.block(&["end"], true)?
                } else {
                    vec![]
                };
                self.take("end")?;
                S::If {
                    branches,
                    otherwise,
                }
            }
            "for" => {
                self.at += 1;
                let name = self.name()?;
                if self.peek() == "=" {
                    self.at += 1;
                    let initial = self.expr(0, 0)?.scalar()?;
                    self.take(",")?;
                    let limit = self.expr(0, 0)?.scalar()?;
                    let step = if self.peek() == "," {
                        self.at += 1;
                        self.expr(0, 0)?.scalar()?
                    } else {
                        self.node(
                            ParserProgramExprKind::Literal {
                                value: ParserFactoryLiteral::Number(1.0),
                            },
                            start,
                            self.end(),
                            1,
                        )?
                        .value
                    };
                    self.take("do")?;
                    self.scopes.push(BTreeMap::new());
                    let local = self.declare(name)?;
                    self.loop_depth += 1;
                    let body = self.block(&["end"], false)?;
                    self.loop_depth -= 1;
                    self.scopes.pop();
                    self.take("end")?;
                    S::ForNumeric {
                        local,
                        start: initial,
                        limit,
                        step,
                        body,
                    }
                } else {
                    let mut names = vec![name];
                    while self.peek() == "," {
                        self.at += 1;
                        names.push(self.name()?);
                        if names.len() > 128 {
                            return Err("iterator local bound".into());
                        }
                    }
                    self.take("in")?;
                    let iterator = if self.authorization.standalone_calls {
                        self.generic_iterator()?
                    } else {
                        let iterator = self.expr(0, 0)?;
                        let Some(ParserProgramPack::Call { call }) = iterator.expansion else {
                            return Err("generic-for requires a direct bound iterator call".into());
                        };
                        let ParserProgramBinding::Intrinsic { operation, .. } =
                            self.bindings[call.binding as usize]
                        else {
                            return Err("generic-for helper iterator is unsupported".into());
                        };
                        match operation {
                            ParserProgramIntrinsic::StringGmatch => {
                                ParserProgramIterator::Pattern { call }
                            }
                            ParserProgramIntrinsic::Ipairs
                                if call.receiver.is_none()
                                    && call.arguments.tail.is_none()
                                    && call.arguments.values.len() == 1 =>
                            {
                                ParserProgramIterator::Dense {
                                    table: call.arguments.values.into_iter().next().unwrap(),
                                    binding: call.binding,
                                }
                            }
                            _ => {
                                return Err(
                                    "iterator operation or argument adjustment is unsupported"
                                        .into(),
                                );
                            }
                        }
                    };
                    self.take("do")?;
                    self.scopes.push(BTreeMap::new());
                    let locals = names
                        .into_iter()
                        .map(|n| self.declare(n))
                        .collect::<LowerResult<Vec<_>>>()?;
                    self.loop_depth += 1;
                    let body = self.block(&["end"], false)?;
                    self.loop_depth -= 1;
                    self.scopes.pop();
                    self.take("end")?;
                    S::ForEach {
                        locals,
                        iterator,
                        body,
                    }
                }
            }
            "return" => {
                self.at += 1;
                let values = if matches!(self.peek(), "end" | "else" | "elseif" | ";" | "") {
                    ParserProgramValueList::default()
                } else {
                    self.values(0)?
                };
                S::Return { values }
            }
            "break" => {
                self.at += 1;
                if self.loop_depth == 0 {
                    return Err("break outside a loop".into());
                }
                S::Break
            }
            "do" | "while" | "repeat" | "function" => {
                return Err(format!("unsupported statement {}", self.peek()));
            }
            _ => {
                let first = self.postfix(0)?.value()?;
                if matches!(self.peek(), "=" | ",") {
                    let mut targets = vec![first.value];
                    while self.peek() == "," {
                        self.at += 1;
                        targets.push(self.postfix(0)?.value()?.value);
                        if targets.len() > 128 {
                            return Err("assignment target bound".into());
                        }
                    }
                    self.take("=")?;
                    let values = self.values(0)?;
                    if targets
                        .iter()
                        .all(|e| matches!(e.operation, ParserProgramExprKind::Local { .. }))
                    {
                        let locals = targets
                            .into_iter()
                            .map(|e| match e.operation {
                                ParserProgramExprKind::Local { local } => local,
                                _ => unreachable!(),
                            })
                            .collect();
                        S::Assign { locals, values }
                    } else if targets.len() == 1
                        && let ParserProgramExprKind::Capture { upvalue } = targets[0].operation
                    {
                        if !self.authorization.standalone_calls
                            || !matches!(
                                self.callback.upvalues[upvalue as usize].value,
                                ParserValue::LiveCapture {}
                            )
                        {
                            return Err(
                                "capture assignment requires a declared live session cell".into()
                            );
                        }
                        S::CaptureSet { upvalue, values }
                    } else if targets.len() == 1
                        && values.values.len() == 1
                        && values.tail.is_none()
                    {
                        let target = targets.pop().unwrap();
                        let ParserProgramExprKind::Get { table, key } = target.operation else {
                            return Err(
                                "assignment requires visible locals or an indexed table".into()
                            );
                        };
                        S::TableSet {
                            table: *table,
                            key: *key,
                            value: values.values.into_iter().next().unwrap(),
                        }
                    } else if targets.len() == 1 && values.values.is_empty() {
                        let Some(tail) = values.tail else {
                            return Err("missing table assignment value".into());
                        };
                        let ParserProgramPack::Call { call } = *tail else {
                            return Err(
                                "table assignment vararg adjustment requires temporary lowering"
                                    .into(),
                            );
                        };
                        let target = targets.pop().unwrap();
                        let ParserProgramExprKind::Get { table, key } = target.operation else {
                            return Err("assignment target is not an indexed table".into());
                        };
                        let height = call_height(&call) + 1;
                        let value = self
                            .node(
                                ParserProgramExprKind::Call {
                                    call: Box::new(call),
                                },
                                start,
                                self.end(),
                                height,
                            )?
                            .value;
                        S::TableSet {
                            table: *table,
                            key: *key,
                            value,
                        }
                    } else {
                        return Err(
                            "mixed indexed/multiple assignment requires temporary address lowering"
                                .into(),
                        );
                    }
                } else {
                    let ParserProgramExprKind::Call { call } = first.value.operation else {
                        return Err("expression statement is not a function call".into());
                    };
                    S::Call { call: *call }
                }
            }
        };
        self.statement(operation, start)
    }
    fn values(&mut self, depth: usize) -> LowerResult<ParserProgramValueList> {
        let mut parsed = vec![self.expr(0, depth + 1)?];
        while self.peek() == "," {
            self.at += 1;
            parsed.push(self.expr(0, depth + 1)?);
            if parsed.len() > 4096 {
                return Err("result/argument list bound".into());
            }
        }
        let last = parsed.pop().unwrap();
        if parsed
            .iter()
            .any(|e| matches!(e.expansion, Some(ParserProgramPack::Varargs)))
        {
            return Err("nonfinal vararg adjustment is not represented".into());
        }
        let tail = last.expansion.map(Box::new);
        let mut values = parsed.into_iter().map(|e| e.value).collect::<Vec<_>>();
        if tail.is_none() {
            values.push(last.value);
        }
        Ok(ParserProgramValueList { values, tail })
    }
    fn generic_iterator(&mut self) -> LowerResult<ParserProgramIterator> {
        let mut values = self.values(0)?;
        // Retain the existing bound iterator forms only for their exact source
        // initializer shape. Extra expressions and adjusted calls must still run.
        if values.values.is_empty()
            && let Some(tail) = values.tail.take()
        {
            if let ParserProgramPack::Call { call } = *tail {
                match self.bindings[call.binding as usize] {
                    ParserProgramBinding::Intrinsic {
                        operation: ParserProgramIntrinsic::StringGmatch,
                        ..
                    } => {
                        return Ok(ParserProgramIterator::Pattern { call });
                    }
                    ParserProgramBinding::Intrinsic {
                        operation: ParserProgramIntrinsic::Ipairs,
                        ..
                    } if call.receiver.is_none()
                        && call.arguments.tail.is_none()
                        && call.arguments.values.len() == 1 =>
                    {
                        return Ok(ParserProgramIterator::Dense {
                            table: call.arguments.values.into_iter().next().unwrap(),
                            binding: call.binding,
                        });
                    }
                    _ => values.tail = Some(Box::new(ParserProgramPack::Call { call })),
                }
            } else {
                values.tail = Some(tail);
            }
        }
        Ok(ParserProgramIterator::Generic { values })
    }
    fn expr(&mut self, min: u8, depth: usize) -> LowerResult<Info> {
        if depth > 32 {
            return Err("source expression recursion bound".into());
        }
        let start = self.offset();
        let mut left = if matches!(self.peek(), "not" | "-" | "#") {
            let operation = match self.peek() {
                "not" => ParserProgramUnary::Not,
                "-" => ParserProgramUnary::Negate,
                _ => ParserProgramUnary::Length,
            };
            self.at += 1;
            let value = self.expr(7, depth + 1)?;
            if matches!(value.expansion, Some(ParserProgramPack::Varargs)) {
                return Err("scalar vararg adjustment is not represented".into());
            }
            let height = value.height + 1;
            self.node(
                ParserProgramExprKind::Unary {
                    operation,
                    value: Box::new(value.value),
                },
                start,
                self.end(),
                height,
            )?
        } else {
            self.postfix(depth + 1)?.value()?
        };
        while let Some((priority, right_associative, operation)) =
            binary(self.peek(), self.authorization.standalone_calls)
        {
            if priority < min {
                break;
            }
            self.at += 1;
            let right = self.expr(
                if right_associative {
                    priority
                } else {
                    priority + 1
                },
                depth + 1,
            )?;
            if matches!(left.expansion, Some(ParserProgramPack::Varargs))
                || matches!(right.expansion, Some(ParserProgramPack::Varargs))
            {
                return Err("scalar vararg adjustment is not represented".into());
            }
            let height = left.height.max(right.height) + 1;
            left = self.node(
                ParserProgramExprKind::Binary {
                    operation,
                    left: Box::new(left.value),
                    right: Box::new(right.value),
                },
                start,
                self.end(),
                height,
            )?;
        }
        Ok(left)
    }
    fn postfix(&mut self, depth: usize) -> LowerResult<Term<'a>> {
        if depth > 32 {
            return Err("source postfix recursion bound".into());
        }
        let start = self.offset();
        let token = *self.tokens.get(self.at).ok_or("missing expression")?;
        let mut term = if token.quoted || token.numeric {
            self.at += 1;
            Term::Value(self.literal(token)?)
        } else {
            match self.peek() {
                "nil" | "true" | "false" => {
                    let value = match self.peek() {
                        "nil" => ParserFactoryLiteral::Nil,
                        "true" => ParserFactoryLiteral::Boolean(true),
                        _ => ParserFactoryLiteral::Boolean(false),
                    };
                    self.at += 1;
                    Term::Value(self.node(
                        ParserProgramExprKind::Literal { value },
                        start,
                        self.end(),
                        1,
                    )?)
                }
                "..." => {
                    if !self.variadic {
                        return Err("varargs in a fixed parameter function".into());
                    }
                    self.at += 1;
                    // Value context cannot represent first-vararg without a pack;
                    // only final list positions accept this marker.
                    let mut info = self.node(
                        ParserProgramExprKind::Literal {
                            value: ParserFactoryLiteral::Nil,
                        },
                        start,
                        self.end(),
                        1,
                    )?;
                    info.expansion = Some(ParserProgramPack::Varargs);
                    Term::Value(info)
                }
                "(" => {
                    self.at += 1;
                    let mut info = self.expr(0, depth + 1)?;
                    self.take(")")?;
                    if matches!(info.expansion, Some(ParserProgramPack::Varargs)) {
                        return Err(
                            "parenthesized first-vararg requires scalar vararg expression".into(),
                        );
                    }
                    info.expansion = None;
                    info.value.location = self.location(start, self.end());
                    Term::Value(info)
                }
                "{" => Term::Value(self.table(depth + 1)?),
                _ => {
                    let name = self.name()?;
                    if let Some(local) = self.local(name) {
                        Term::Value(self.node(
                            ParserProgramExprKind::Local { local },
                            start,
                            self.end(),
                            1,
                        )?)
                    } else if let Some((slot, capture)) = self
                        .callback
                        .upvalues
                        .iter()
                        .enumerate()
                        .find(|(_, u)| u.name == name)
                    {
                        let captured = capture.value.clone();
                        let mut info = self.node(
                            ParserProgramExprKind::Capture {
                                upvalue: slot as u16,
                            },
                            start,
                            self.end(),
                            1,
                        )?;
                        if let ParserValue::Callback(callback) = captured {
                            info.target = Some(
                                if let Some(operation) =
                                    self.authorization.intrinsics.get(&callback)
                                {
                                    ParserProgramBinding::Intrinsic {
                                        operation: *operation,
                                        source: ParserProgramIntrinsicSource::Captured {
                                            upvalue: slot as u16,
                                            callback,
                                        },
                                    }
                                } else {
                                    ParserProgramBinding::CapturedCallback {
                                        upvalue: slot as u16,
                                        callback,
                                    }
                                },
                            );
                        }
                        Term::Value(info)
                    } else if let Some(root) = self.authorization.environment {
                        self.budget.bytes(name.len())?;
                        let table = self.node(
                            ParserProgramExprKind::NamedDefinition { root },
                            start,
                            self.end(),
                            1,
                        )?;
                        let key = self.node(
                            ParserProgramExprKind::Literal {
                                value: ParserFactoryLiteral::Text(name.into()),
                            },
                            start,
                            self.end(),
                            1,
                        )?;
                        Term::Value(self.node(
                            ParserProgramExprKind::Get {
                                table: Box::new(table.value),
                                key: Box::new(key.value),
                            },
                            start,
                            self.end(),
                            2,
                        )?)
                    } else {
                        let root = self.authorization.roots.get(name).copied();
                        if let Some(root) = root {
                            let operation = match root {
                                SourceProgramDefinitionRoot::Named(root) => {
                                    ParserProgramExprKind::NamedDefinition { root }
                                }
                                legacy => ParserProgramExprKind::Definition {
                                    root: legacy.legacy().expect("legacy root branch"),
                                },
                            };
                            Term::Value(self.node(operation, start, self.end(), 1)?)
                        } else {
                            Term::Global {
                                path: vec![name],
                                start,
                            }
                        }
                    }
                }
            }
        };
        loop {
            if matches!(&term,Term::Value(v) if matches!(v.expansion,Some(ParserProgramPack::Varargs)))
                && matches!(self.peek(), "." | "[" | "(" | ":")
            {
                return Err("scalar vararg adjustment is not represented".into());
            }
            term = match self.peek() {
                "." => {
                    self.at += 1;
                    let name = self.name()?;
                    match term {
                        Term::Global { mut path, start } => {
                            path.push(name);
                            Term::Global { path, start }
                        }
                        Term::Value(table) => {
                            self.budget.bytes(name.len())?;
                            let key = self.node(
                                ParserProgramExprKind::Literal {
                                    value: ParserFactoryLiteral::Text(name.into()),
                                },
                                self.tokens[self.at - 1].start,
                                self.end(),
                                1,
                            )?;
                            Term::Value(self.node(
                                ParserProgramExprKind::Get {
                                    table: Box::new(table.value),
                                    key: Box::new(key.value),
                                },
                                start,
                                self.end(),
                                table.height + 1,
                            )?)
                        }
                    }
                }
                "[" => {
                    self.at += 1;
                    let table = term.value()?;
                    let key = self.expr(0, depth + 1)?;
                    if matches!(key.expansion, Some(ParserProgramPack::Varargs)) {
                        return Err("scalar vararg key is not represented".into());
                    }
                    self.take("]")?;
                    let height = table.height.max(key.height) + 1;
                    Term::Value(self.node(
                        ParserProgramExprKind::Get {
                            table: Box::new(table.value),
                            key: Box::new(key.value),
                        },
                        start,
                        self.end(),
                        height,
                    )?)
                }
                "(" | ":" => {
                    let (binding, receiver, base_height) = if self.peek() == ":" {
                        self.at += 1;
                        let name = self.name()?;
                        let receiver = term.value()?;
                        let binding = if self.authorization.standalone_calls {
                            if name.len() > 256 {
                                return Err("source method name bound".into());
                            }
                            self.budget.bytes(name.len())?;
                            // A colon call saves both the receiver and its looked-up
                            // method before evaluating arguments. Do not lower this
                            // as a captured helper or a function with prepended self:
                            // argument effects may replace the method after lookup.
                            self.bind(ParserProgramBinding::DynamicMethod { key: name.into() })?
                        } else {
                            let operation = match name {
                                "gsub" => ParserProgramIntrinsic::StringGsub,
                                "gmatch" => ParserProgramIntrinsic::StringGmatch,
                                _ => return Err(format!("unsupported method {name}")),
                            };
                            self.global_binding(operation)?
                        };
                        (binding, Some(Box::new(receiver.value)), receiver.height)
                    } else {
                        match term {
                            Term::Value(ref value) => {
                                if let Some(target) = value.target.clone() {
                                    (self.bind(target)?, None, value.height)
                                } else if self.authorization.standalone_calls {
                                    // Save the actual callable value before argument effects.
                                    // This is a dot/value call and never adds implicit self.
                                    (
                                        self.bind(ParserProgramBinding::DynamicCall {})?,
                                        Some(Box::new(value.value.clone())),
                                        value.height,
                                    )
                                } else {
                                    return Err("dynamic/local function call requires callable-value support".into());
                                }
                            }
                            Term::Global { ref path, .. } => {
                                let operation = match path.as_slice() {
                                    ["unpack"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::Unpack
                                    }
                                    ["tonumber"] => ParserProgramIntrinsic::ToNumber,
                                    ["type"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::Type
                                    }
                                    ["select"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::Select
                                    }
                                    ["tostring"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::ToString
                                    }
                                    ["math", "floor"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::MathFloor
                                    }
                                    ["math", "min"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::MathMin
                                    }
                                    ["math", "max"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::MathMax
                                    }
                                    ["string", "lower"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::StringLower
                                    }
                                    ["string", "find"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::StringFind
                                    }
                                    ["string", "sub"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::StringSub
                                    }
                                    ["string", "match"] if self.authorization.standalone_calls => {
                                        ParserProgramIntrinsic::StringMatch
                                    }
                                    ["ipairs"] => ParserProgramIntrinsic::Ipairs,
                                    ["table", "insert"] => ParserProgramIntrinsic::TableInsert,
                                    ["string", "gsub"] => ParserProgramIntrinsic::StringGsub,
                                    ["string", "gmatch"] => ParserProgramIntrinsic::StringGmatch,
                                    _ => {
                                        return Err(format!(
                                            "unsupported global call {}",
                                            path.join(".")
                                        ));
                                    }
                                };
                                (self.global_binding(operation)?, None, 1)
                            }
                        }
                    };
                    self.take("(")?;
                    let arguments = if self.peek() == ")" {
                        ParserProgramValueList::default()
                    } else {
                        self.values(depth + 1)?
                    };
                    self.take(")")?;
                    let height = base_height.max(values_height(&arguments)) + 4;
                    let call = ParserProgramCall {
                        binding,
                        receiver,
                        arguments,
                    };
                    let mut info = self.node(
                        ParserProgramExprKind::Call {
                            call: Box::new(call.clone()),
                        },
                        start,
                        self.end(),
                        height,
                    )?;
                    info.expansion = Some(ParserProgramPack::Call { call });
                    Term::Value(info)
                }
                _ => break,
            };
        }
        Ok(term)
    }
    fn literal(&mut self, token: Lex<'a>) -> LowerResult<Info> {
        let operation = if token.quoted {
            let value = self
                .lua
                .load(format!("return {}", token.text))
                .eval::<mlua::LuaString>()
                .map_err(|e| e.to_string())?;
            let bytes = value.as_bytes();
            self.budget.bytes(bytes.len())?;
            match std::str::from_utf8(&bytes) {
                Ok(text) => ParserProgramExprKind::Literal {
                    value: ParserFactoryLiteral::Text(text.into()),
                },
                Err(_) => ParserProgramExprKind::Bytes {
                    value: bytes.to_vec(),
                },
            }
        } else {
            let value = self
                .lua
                .load(format!("return {}", token.text))
                .eval::<f64>()
                .map_err(|e| e.to_string())?;
            let value = if value.is_finite() {
                ParserFactoryLiteral::Number(value)
            } else {
                ParserFactoryLiteral::NonFinite(if value.is_nan() {
                    ParserNonFinite::Nan
                } else if value.is_sign_negative() {
                    ParserNonFinite::NegativeInfinity
                } else {
                    ParserNonFinite::PositiveInfinity
                })
            };
            ParserProgramExprKind::Literal { value }
        };
        self.node(operation, token.start, token.end, 1)
    }
    fn table(&mut self, depth: usize) -> LowerResult<Info> {
        if depth > 32 {
            return Err("table constructor depth bound".into());
        }
        let start = self.offset();
        self.take("{")?;
        let mut fields = vec![];
        let mut keys = BTreeSet::new();
        let mut height = 1;
        while self.peek() != "}" {
            if fields.len() >= 4096 {
                return Err("table literal field bound".into());
            }
            let key = if identifier(self.peek()) && self.next_text() == "=" {
                let key = self.name()?.as_bytes().to_vec();
                self.budget.bytes(key.len())?;
                self.take("=")?;
                Some(key)
            } else if self.peek() == "[" {
                self.at += 1;
                let token = *self
                    .tokens
                    .get(self.at)
                    .filter(|t| t.quoted)
                    .ok_or("computed/numeric constructor keys require collision/order proof")?;
                self.at += 1;
                let literal = self.literal(token)?;
                let key = match literal.value.operation {
                    ParserProgramExprKind::Literal {
                        value: ParserFactoryLiteral::Text(s),
                    } => s.into_bytes(),
                    ParserProgramExprKind::Bytes { value } => value,
                    _ => unreachable!(),
                };
                self.take("]")?;
                self.take("=")?;
                Some(key)
            } else {
                None
            };
            let value = self.expr(0, depth + 1)?;
            height = height.max(value.height + 3);
            let field = if let Some(key) = key {
                if !keys.insert(key.clone()) {
                    return Err(
                        "duplicate constructor string key requires LuaJIT template-order proof"
                            .into(),
                    );
                }
                if matches!(value.expansion, Some(ParserProgramPack::Varargs)) {
                    return Err("named first-vararg value requires scalar vararg expression".into());
                }
                match String::from_utf8(key) {
                    Ok(key) => ParserProgramField::Named {
                        key,
                        value: value.value,
                    },
                    Err(e) => {
                        let key = self
                            .node(
                                ParserProgramExprKind::Bytes {
                                    value: e.into_bytes(),
                                },
                                start,
                                self.end(),
                                1,
                            )?
                            .value;
                        ParserProgramField::Keyed {
                            key,
                            value: value.value,
                        }
                    }
                }
            } else if let Some(values) = value.expansion {
                let final_field = self.peek() == "}"
                    || (matches!(self.peek(), "," | ";") && self.next_text() == "}");
                if final_field {
                    ParserProgramField::Tail { values }
                } else if matches!(values, ParserProgramPack::Varargs) {
                    return Err("nonfinal vararg literal field requires scalar adjustment".into());
                } else {
                    ParserProgramField::List { value: value.value }
                }
            } else {
                ParserProgramField::List { value: value.value }
            };
            fields.push(field);
            if !matches!(self.peek(), "," | ";") {
                break;
            }
            self.at += 1;
        }
        self.take("}")?;
        self.node(
            ParserProgramExprKind::Table { fields },
            start,
            self.end(),
            height,
        )
    }
}
fn binary(token: &str, standalone: bool) -> Option<(u8, bool, ParserProgramBinary)> {
    use ParserProgramBinary as B;
    Some(match token {
        "or" => (1, false, B::Or),
        "and" => (2, false, B::And),
        "==" => (3, false, B::Equal),
        "~=" => (3, false, B::NotEqual),
        "<" => (3, false, B::LessThan),
        "<=" => (3, false, B::LessEqual),
        ">" if standalone => (3, false, B::GreaterThan),
        ">=" if standalone => (3, false, B::GreaterEqual),
        ".." => (4, true, B::Concat),
        "+" => (5, false, B::Add),
        "-" => (5, false, B::Subtract),
        "*" => (6, false, B::Multiply),
        "/" => (6, false, B::Divide),
        // Lua power binds more tightly than unary operands (priority 7),
        // including a unary exponent, and associates from the right.
        "^" if standalone => (8, true, B::Power),
        _ => return None,
    })
}
// Conservative structural depth (call/list/pack frames count independently),
// independent of current argument values and unreachable branches.
fn values_height(values: &ParserProgramValueList) -> usize {
    values
        .values
        .iter()
        .map(expr_height)
        .chain(values.tail.iter().map(|p| pack_height(p)))
        .max()
        .unwrap_or(0)
}
fn pack_height(pack: &ParserProgramPack) -> usize {
    match pack {
        ParserProgramPack::Varargs => 1,
        ParserProgramPack::Call { call } => call_height(call),
    }
}
fn call_height(call: &ParserProgramCall) -> usize {
    values_height(&call.arguments).max(call.receiver.as_deref().map(expr_height).unwrap_or(0)) + 3
}
fn expr_height(expr: &Expr) -> usize {
    use ParserProgramExprKind as E;
    match &expr.operation {
        E::Get { table, key } => expr_height(table).max(expr_height(key)) + 1,
        E::Unary { value, .. } => expr_height(value) + 1,
        E::Binary { left, right, .. } => expr_height(left).max(expr_height(right)) + 1,
        E::Call { call } => call_height(call) + 1,
        E::Table { fields } => {
            fields
                .iter()
                .map(|f| match f {
                    ParserProgramField::Named { value, .. }
                    | ParserProgramField::List { value } => expr_height(value),
                    ParserProgramField::Keyed { key, value } => {
                        expr_height(key).max(expr_height(value))
                    }
                    ParserProgramField::Tail { values } => pack_height(values),
                })
                .max()
                .unwrap_or(0)
                + 2
        }
        _ => 1,
    }
}
