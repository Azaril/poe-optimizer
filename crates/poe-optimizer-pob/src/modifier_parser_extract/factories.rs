//! Conservative whole-source lowering; this is not a general Lua interpreter.
use super::*;
use std::collections::BTreeSet;

const CONSTRUCTOR: &str = r#"function modLib.createMod(modName, modType, modVal, ...)
 local flags = 0
 local keywordFlags = 0
 local tagStart = 1
 local source
 if select('#', ...) >= 1 and type(select(1, ...)) == "string" then
  source = select(1, ...)
  tagStart = 2
 end
 if select('#', ...) >= 2 and type(select(2, ...)) == "number" then
  flags = select(2, ...)
  tagStart = 3
 end
 if select('#', ...) >= 3 and type(select(3, ...)) == "number" then
  keywordFlags = select(3, ...)
  tagStart = 4
 end
 return {name=modName,type=modType,value=modVal,flags=flags,
 keywordFlags=keywordFlags,source=source,select(tagStart,...)}
end"#;

pub(super) fn constructor(sources: &BTreeMap<String, String>) -> Result<()> {
    let actual = top_function(sources, TOOLS, "modLib.createMod")?;
    let a = tokens(actual)?;
    let b = tokens(CONSTRUCTOR)?;
    if a.iter().map(|t| t.text).ne(b.iter().map(|t| t.text)) {
        return Err(error("complete createMod constructor mechanism changed"));
    }
    Ok(())
}
pub(super) fn lower(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    data: &ModifierParserData,
    constructor: ParserCallbackId,
) -> Result<BTreeMap<ParserCallbackId, ParserFactoryDisposition>> {
    let mut out = BTreeMap::new();
    for (index, callback) in data.callbacks.iter().enumerate() {
        let disposition = match &callback.kind {
            ParserCallbackKind::Builtin { .. } => unsupported("builtin callback is opaque"),
            ParserCallbackKind::Lua { source: span } => {
                let source = source(sources, &span.path)?;
                let body = source
                    .split_inclusive('\n')
                    .skip(span.line as usize - 1)
                    .take((span.end_line - span.line + 1) as usize)
                    .collect::<String>();
                if hash(body.as_bytes()) != span.sha256 {
                    return Err(error("factory source span mismatch"));
                }
                let ts = tokens(&body)?;
                let result = Lowerer {
                    lua,
                    body: &body,
                    tokens: ts,
                    at: 0,
                    callback,
                    data,
                    constructor,
                    parameters: vec![],
                    used_constructor: false,
                    nodes: 0,
                }
                .factory(span);
                match result {
                    Ok(factory) => ParserFactoryDisposition::Pure(Box::new(factory)),
                    Err(reason) => unsupported(&reason),
                }
            }
        };
        out.insert(ParserCallbackId(index as u32 + 1), disposition);
    }
    Ok(out)
}
fn unsupported(reason: &str) -> ParserFactoryDisposition {
    ParserFactoryDisposition::Unsupported {
        reason: reason.to_owned(),
    }
}
type LowerResult<T> = std::result::Result<T, String>;
struct Lowerer<'a> {
    lua: &'a Lua,
    body: &'a str,
    tokens: Vec<Token<'a>>,
    at: usize,
    callback: &'a ParserCallback,
    data: &'a ModifierParserData,
    constructor: ParserCallbackId,
    parameters: Vec<&'a str>,
    used_constructor: bool,
    nodes: usize,
}
impl<'a> Lowerer<'a> {
    fn peek(&self) -> &str {
        self.tokens.get(self.at).map_or("", |t| t.text)
    }
    fn at_text(&self, offset: usize) -> &str {
        self.tokens.get(self.at + offset).map_or("", |t| t.text)
    }
    fn take(&mut self, value: &str) -> LowerResult<()> {
        if self.peek() != value {
            return Err(format!(
                "expected {value}; unsupported token {}",
                self.peek()
            ));
        }
        self.at += 1;
        Ok(())
    }
    fn name(&mut self) -> LowerResult<&'a str> {
        let Some(t) = self.tokens.get(self.at) else {
            return Err("missing name".into());
        };
        if t.quoted || !identifier(t.text) {
            return Err("unsupported name".into());
        }
        self.at += 1;
        Ok(t.text)
    }
    fn factory(mut self, span: &ItemSourceSpan) -> LowerResult<ParserPureFactory> {
        let mut positions = self
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.quoted && t.text == "function")
            .map(|(i, _)| i);
        let start = positions.next().ok_or("missing function")?;
        if positions.next().is_some() {
            return Err("multiple or nested functions in source span".into());
        }
        self.at = start + 1;
        self.take("(")?; // named helpers are deliberately outside this factory grammar
        if self.peek() != ")" {
            loop {
                if self.parameters.len() >= 128 {
                    return Err("parameter bound".into());
                }
                let name = self.name()?;
                self.parameters.push(name);
                if self.peek() != "," {
                    break;
                }
                self.take(",")?;
            }
        }
        self.take(")")?;
        self.take("return")?;
        let expr = if self.peek() == "end" {
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
        } else {
            self.expr(0)?
        };
        if !matches!(
            expr,
            ParserFactoryExpr::Table(_)
                | ParserFactoryExpr::CreateMod { .. }
                | ParserFactoryExpr::Flag { .. }
                | ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
        ) {
            return Err("top result is not statically table or nil".into());
        }
        let end_token = self.tokens.get(self.at).ok_or("missing function end")?;
        let end =
            end_token.text.as_ptr() as usize + end_token.text.len() - self.body.as_ptr() as usize;
        self.take("end")?;
        while matches!(self.peek(), "," | ";") {
            self.at += 1;
        }
        if self.at != self.tokens.len() {
            return Err("unconsumed source after function".into());
        }
        let start = self.tokens[start].text.as_ptr() as usize - self.body.as_ptr() as usize;
        Ok(ParserPureFactory {
            parameter_count: self.parameters.len() as u16,
            body: expr,
            provenance: ParserFactoryProvenance {
                source: span.clone(),
                function_start: start as u32,
                function_end: end as u32,
                function_sha256: hash(&self.body.as_bytes()[start..end]),
                constructor: self.used_constructor.then_some(self.constructor),
            },
        })
    }
    fn expr(&mut self, depth: usize) -> LowerResult<ParserFactoryExpr> {
        let left = self.unary(depth)?;
        if self.peek() == "."
            && self.at_text(1) == "."
            && self.tokens[self.at].text.as_ptr() as usize + 1
                == self.tokens[self.at + 1].text.as_ptr() as usize
        {
            self.at += 2;
            self.nodes += 1;
            let right = self.expr(depth + 1)?;
            return Ok(ParserFactoryExpr::Concat {
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }
    fn unary(&mut self, depth: usize) -> LowerResult<ParserFactoryExpr> {
        if depth > 32 || self.nodes >= 4096 {
            return Err("factory expression resource bound".into());
        }
        self.nodes += 1;
        if self.peek() == "-" {
            self.at += 1;
            return Ok(ParserFactoryExpr::Negate(Box::new(self.unary(depth + 1)?)));
        }
        if self.peek() == "(" {
            self.at += 1;
            let expr = self.expr(depth + 1)?;
            self.take(")")?;
            return Ok(expr);
        }
        if self.peek() == "{" {
            return self.table(depth + 1);
        }
        let Some(token) = self.tokens.get(self.at) else {
            return Err("missing expression".into());
        };
        if token.quoted {
            self.at += 1;
            let value = self
                .lua
                .load(format!("return {}", token.text))
                .eval::<String>()
                .map_err(|e| e.to_string())?;
            return Ok(ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(
                value,
            )));
        }
        for (name, value) in [
            ("nil", ParserFactoryLiteral::Nil),
            ("true", ParserFactoryLiteral::Boolean(true)),
            ("false", ParserFactoryLiteral::Boolean(false)),
        ] {
            if self.peek() == name {
                self.at += 1;
                return Ok(ParserFactoryExpr::Literal(value));
            }
        }
        let offset = token.text.as_ptr() as usize - self.body.as_ptr() as usize;
        if let Some(length) = numeric_prefix(&self.body[offset..]) {
            let literal = &self.body[offset..offset + length];
            while self.tokens.get(self.at).is_some_and(|t| {
                t.text.as_ptr() as usize - (self.body.as_ptr() as usize) < offset + length
            }) {
                self.at += 1;
            }
            let n = self
                .lua
                .load(format!("return {literal}"))
                .eval::<f64>()
                .map_err(|e| e.to_string())?;
            let value = if n.is_nan() {
                ParserFactoryLiteral::NonFinite(ParserNonFinite::Nan)
            } else if n == f64::INFINITY {
                ParserFactoryLiteral::NonFinite(ParserNonFinite::PositiveInfinity)
            } else if n == f64::NEG_INFINITY {
                ParserFactoryLiteral::NonFinite(ParserNonFinite::NegativeInfinity)
            } else {
                ParserFactoryLiteral::Number(n)
            };
            return Ok(ParserFactoryExpr::Literal(value));
        }
        let name = self.name()?;
        if self.peek() == "(" {
            if name == "tonumber" {
                if self.parameters.contains(&name)
                    || self.callback.upvalues.iter().any(|u| u.name == name)
                {
                    return Err("tonumber is not the unshadowed original global".into());
                }
                self.at += 1;
                if self.peek() == ")" {
                    return Err("tonumber requires exactly one explicit argument".into());
                }
                let value = self.expr(depth + 1)?;
                if self.peek() != ")" {
                    return Err("tonumber requires exactly one explicit argument".into());
                }
                self.take(")")?;
                return Ok(ParserFactoryExpr::ToNumber {
                    value: Box::new(value),
                });
            }
            if name == "firstToUpper" {
                let helper = self
                    .data
                    .helpers
                    .get(name)
                    .copied()
                    .ok_or_else(|| "missing firstToUpper helper".to_owned())?;
                if self.parameters.contains(&name)
                    || !self
                        .callback
                        .upvalues
                        .iter()
                        .any(|u| u.name == name && u.value == ParserValue::Callback(helper))
                {
                    return Err("firstToUpper is not the captured helper".into());
                }
                self.at += 1;
                if self.peek() == ")" {
                    return Err("firstToUpper requires exactly one argument".into());
                }
                let value = self.expr(depth + 1)?;
                if self.peek() != ")" {
                    return Err("firstToUpper requires exactly one argument".into());
                }
                self.take(")")?;
                return Ok(ParserFactoryExpr::FirstToUpper {
                    helper,
                    value: Box::new(value),
                });
            }
            if name == "flag" {
                let helper = self
                    .data
                    .helpers
                    .get(name)
                    .copied()
                    .ok_or_else(|| "missing flag helper".to_owned())?;
                if self.parameters.contains(&name)
                    || !self
                        .callback
                        .upvalues
                        .iter()
                        .any(|u| u.name == name && u.value == ParserValue::Callback(helper))
                {
                    return Err("flag is not the captured helper".into());
                }
                self.at += 1;
                let mut args = vec![];
                if self.peek() != ")" {
                    loop {
                        if args.len() >= 4096 {
                            return Err("flag argument bound".into());
                        }
                        args.push(self.expr(depth + 1)?);
                        if self.peek() != "," {
                            break;
                        }
                        self.at += 1;
                    }
                }
                self.take(")")?;
                return Ok(ParserFactoryExpr::Flag { helper, args });
            }
            if name != "mod"
                || !self
                    .callback
                    .upvalues
                    .iter()
                    .any(|u| u.name == name && u.value == ParserValue::Callback(self.constructor))
            {
                return Err(format!("unsupported call {name}"));
            }
            if self.parameters.contains(&name) {
                return Err("constructor shadowed by parameter".into());
            }
            self.used_constructor = true;
            self.at += 1;
            let mut args = vec![];
            if self.peek() != ")" {
                loop {
                    if args.len() >= 4096 {
                        return Err("constructor argument bound".into());
                    }
                    args.push(self.expr(depth + 1)?);
                    if self.peek() != "," {
                        break;
                    }
                    self.at += 1;
                }
            }
            self.take(")")?;
            return Ok(ParserFactoryExpr::CreateMod { args });
        }
        if let Some(slot) = self.parameters.iter().rposition(|p| *p == name) {
            return Ok(ParserFactoryExpr::Argument(slot as u16));
        }
        if let Some((slot, upvalue)) = self
            .callback
            .upvalues
            .iter()
            .enumerate()
            .find(|(_, u)| u.name == name)
        {
            if matches!(
                upvalue.value,
                ParserValue::Table(_) | ParserValue::Callback(_)
            ) {
                return Err(format!("unsupported non-scalar capture {name}"));
            }
            return Ok(ParserFactoryExpr::CapturedScalar {
                upvalue: slot as u16,
            });
        }
        let table = match name {
            "ModFlag" => self.data.policy.mod_flags,
            "KeywordFlag" => self.data.policy.keyword_flags,
            "SkillType" => self.data.policy.skill_types,
            _ => return Err(format!("unsupported global {name}")),
        };
        self.take(".")?;
        let key = self.name()?.to_owned();
        Ok(ParserFactoryExpr::ConstantField { table, key })
    }
    fn table(&mut self, depth: usize) -> LowerResult<ParserFactoryExpr> {
        self.take("{")?;
        let mut fields = vec![];
        let mut keys = BTreeSet::new();
        while self.peek() != "}" {
            if fields.len() >= 4096 {
                return Err("table field bound".into());
            }
            let key = if self.at_text(1) == "=" && identifier(self.peek()) {
                let key = self.name()?.to_owned();
                self.take("=")?;
                Some(key)
            } else if self.peek() == "[" {
                self.at += 1;
                let Some(token) = self.tokens.get(self.at).filter(|t| t.quoted) else {
                    return Err("computed or numeric table key".into());
                };
                let key = self
                    .lua
                    .load(format!("return {}", token.text))
                    .eval::<String>()
                    .map_err(|e| e.to_string())?;
                self.at += 1;
                self.take("]")?;
                self.take("=")?;
                Some(key)
            } else {
                None
            };
            let value = self.expr(depth)?;
            fields.push(if let Some(key) = key {
                if !keys.insert(key.clone()) {
                    return Err("duplicate named table key".into());
                }
                ParserFactoryField::Named { key, value }
            } else {
                ParserFactoryField::List(value)
            });
            if !matches!(self.peek(), "," | ";") {
                break;
            }
            self.at += 1;
        }
        self.take("}")?;
        Ok(ParserFactoryExpr::Table(fields))
    }
}
fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && ![
            "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in",
            "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
        ]
        .contains(&value)
}
fn numeric_prefix(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    if !b.first().is_some_and(u8::is_ascii_digit)
        && !(b.first() == Some(&b'.') && b.get(1).is_some_and(u8::is_ascii_digit))
    {
        return None;
    }
    let hex = b.starts_with(b"0x") || b.starts_with(b"0X");
    let mut i = if hex { 2 } else { 0 };
    while b.get(i).is_some_and(|v| {
        if hex {
            v.is_ascii_hexdigit()
        } else {
            v.is_ascii_digit()
        }
    }) {
        i += 1;
    }
    if b.get(i) == Some(&b'.') && b.get(i + 1) != Some(&b'.') {
        i += 1;
        while b.get(i).is_some_and(|v| {
            if hex {
                v.is_ascii_hexdigit()
            } else {
                v.is_ascii_digit()
            }
        }) {
            i += 1;
        }
    }
    if b.get(i).is_some_and(|v| {
        if hex {
            matches!(v, b'p' | b'P')
        } else {
            matches!(v, b'e' | b'E')
        }
    }) {
        i += 1;
        if b.get(i).is_some_and(|v| matches!(v, b'+' | b'-')) {
            i += 1;
        }
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
    }
    Some(i)
}

pub(super) fn verify_environment(
    lua: &Lua,
    original_type: &Function,
    original_select: &Function,
    roots: &[(&str, ParserTableId)],
    observed: &BTreeMap<usize, ParserTableId>,
) -> Result<()> {
    for (name, expected) in [("type", original_type), ("select", original_select)] {
        let actual: Function = lua.globals().get(name)?;
        if actual.to_pointer() != expected.to_pointer() || actual.info().what != "C" {
            return Err(error(format!("constructor builtin {name} changed")));
        }
    }
    for (name, expected) in roots {
        let actual: Table = lua.globals().get(*name)?;
        if observed.get(&(actual.to_pointer() as usize)) != Some(expected) {
            return Err(error(format!(
                "factory global {name} is not its observed policy table"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_constructor_authentication_rejects_changed_complete_semantics() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2")
            .join(TOOLS);
        let text = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let mut sources = BTreeMap::from([(TOOLS.to_owned(), text)]);
        constructor(&sources).unwrap();
        let changed = sources[TOOLS].replace("tagStart = 3", "tagStart = 2");
        sources.insert(TOOLS.to_owned(), changed);
        assert!(constructor(&sources).is_err());
    }

    #[test]
    fn source_environment_rejects_rebound_builtin_and_policy_objects() {
        let lua = Lua::new();
        let original_type = lua.globals().get::<Function>("type").unwrap();
        let original_select = lua.globals().get::<Function>("select").unwrap();
        let table = lua.create_table().unwrap();
        lua.globals().set("ModFlag", table.clone()).unwrap();
        let id = ParserTableId(1);
        let observed = BTreeMap::from([(table.to_pointer() as usize, id)]);
        let roots = [("ModFlag", id)];
        verify_environment(&lua, &original_type, &original_select, &roots, &observed).unwrap();
        lua.globals()
            .set("ModFlag", lua.create_table().unwrap())
            .unwrap();
        assert!(
            verify_environment(&lua, &original_type, &original_select, &roots, &observed).is_err()
        );
        lua.globals().set("ModFlag", table).unwrap();
        lua.load("type = function() return 'number' end")
            .exec()
            .unwrap();
        assert!(
            verify_environment(&lua, &original_type, &original_select, &roots, &observed).is_err()
        );
        lua.globals().set("type", original_type.clone()).unwrap();
        lua.globals()
            .set("select", lua.create_function(|_, ()| Ok(1)).unwrap())
            .unwrap();
        assert!(
            verify_environment(&lua, &original_type, &original_select, &roots, &observed).is_err()
        );
    }

    fn one(body: &str, captured: ParserValue) -> LowerResult<ParserPureFactory> {
        one_callback(body, captured, |_| {})
    }
    fn one_callback(
        body: &str,
        captured: ParserValue,
        edit: impl FnOnce(&mut ParserCallback),
    ) -> LowerResult<ParserPureFactory> {
        static DATA: std::sync::OnceLock<ModifierParserCatalog> = std::sync::OnceLock::new();
        let data = DATA
            .get_or_init(|| {
                poe_optimizer_data::game_data::bundled_snapshot()
                    .unwrap()
                    .modifier_parser()
                    .clone()
            })
            .data();
        let constructor = data
            .callbacks
            .iter()
            .flat_map(|c| &c.upvalues)
            .find_map(|u| {
                if u.name == "mod"
                    && let ParserValue::Callback(id) = u.value
                {
                    return Some(id);
                }
                None
            })
            .unwrap();
        let source = ItemSourceSpan {
            path: PARSER.into(),
            line: 1,
            end_line: body.lines().count() as u32,
            sha256: hash(body.as_bytes()),
        };
        let mut callback = ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: source.clone(),
            },
            environment: ParserEnvironment::OriginalGlobals,
            upvalues: vec![
                ParserUpvalue {
                    name: "mod".into(),
                    value: ParserValue::Callback(constructor),
                },
                ParserUpvalue {
                    name: "captured".into(),
                    value: captured,
                },
                ParserUpvalue {
                    name: "flag".into(),
                    value: ParserValue::Callback(data.helpers["flag"]),
                },
                ParserUpvalue {
                    name: "firstToUpper".into(),
                    value: ParserValue::Callback(data.helpers["firstToUpper"]),
                },
            ],
        };
        edit(&mut callback);
        let lua = Lua::new();
        Lowerer {
            lua: &lua,
            body,
            tokens: tokens(body).unwrap(),
            at: 0,
            callback: &callback,
            data,
            constructor,
            parameters: vec![],
            used_constructor: false,
            nodes: 0,
        }
        .factory(&source)
    }

    #[test]
    fn complete_factory_lowering_preserves_parameters_captures_literals_and_nil_holes() {
        let body = r#"["ignored"] = function(_, _, value) return {
            mod(captured, false, -value, nil, 0x10, nil, {["tag"] = "\065", value = 1.25e-2}),
            nil, {empty = "", ModFlag = ModFlag.Attack}
        } end, -- function() is a comment"#;
        let factory = one(body, ParserValue::Text("Injected".into())).unwrap();
        assert_eq!(factory.parameter_count, 3);
        let ParserFactoryExpr::Table(fields) = factory.body else {
            panic!()
        };
        let ParserFactoryField::List(ParserFactoryExpr::CreateMod { args }) = &fields[0] else {
            panic!()
        };
        assert_eq!(args.len(), 7);
        assert_eq!(args[0], ParserFactoryExpr::CapturedScalar { upvalue: 1 });
        assert_eq!(
            args[2],
            ParserFactoryExpr::Negate(Box::new(ParserFactoryExpr::Argument(2)))
        );
        assert_eq!(
            args[3],
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
        );
        assert_eq!(
            args[4],
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Number(16.0))
        );
        assert_eq!(
            args[5],
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
        );
        let duplicate = one("function(a,a) return {a} end", ParserValue::Nil).unwrap();
        assert_eq!(
            duplicate.body,
            ParserFactoryExpr::Table(vec![ParserFactoryField::List(ParserFactoryExpr::Argument(
                1
            ))])
        );
        let nil = one("function() return end", ParserValue::Nil).unwrap();
        assert_eq!(
            nil.body,
            ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
        );
        assert!(nil.provenance.constructor.is_none());
    }

    #[test]
    fn unsupported_factory_syntax_does_not_acquire_partial_recipe() {
        for body in [
            "function(x) return {x + 1} end",
            "function(x) return {x / 2} end",
            "function(x) return {tonumber(x, 16)} end",
            "function(x) return {combineToUpper(x)} end",
            "function(x) local a=x return {a} end",
            "function(x) return {} print(x) end",
            "function(x) return {} end print('tail')",
            "function(x) return {a=1, ['a']=2} end",
            "function(x) return {[1]=x, x} end",
            "function(x) return {[x]=1} end",
            "function(x) return {function() end} end",
            "function(x) return x end",
            "function(x) return {},{} end",
            "function(mod) return {mod('A','BASE',1)} end",
            "function(...) return {} end",
        ] {
            assert!(one(body, ParserValue::Nil).is_err(), "{body}");
        }
        assert!(
            one(
                "function() return {captured} end",
                ParserValue::Table(ParserTableId(1))
            )
            .is_err()
        );
        assert!(
            one(
                "function(ModFlag) return {ModFlag.Attack} end",
                ParserValue::Nil
            )
            .is_err()
        );
    }
    #[test]
    fn string_lowering_retains_unary_precedence_right_association_and_parentheses() {
        fn first(body: &str) -> ParserFactoryExpr {
            let f = one(body, ParserValue::Nil).unwrap();
            let ParserFactoryExpr::Table(mut fields) = f.body else {
                panic!()
            };
            let ParserFactoryField::List(value) = fields.remove(0) else {
                panic!()
            };
            value
        }
        fn arg(n: u16) -> Box<ParserFactoryExpr> {
            Box::new(ParserFactoryExpr::Argument(n))
        }
        assert_eq!(
            first("function(a,b,c) return {-a .. b .. c} end"),
            ParserFactoryExpr::Concat {
                left: Box::new(ParserFactoryExpr::Negate(arg(0))),
                right: Box::new(ParserFactoryExpr::Concat {
                    left: arg(1),
                    right: arg(2)
                })
            }
        );
        assert_eq!(
            first("function(a,b,c) return {(a .. b) .. c} end"),
            ParserFactoryExpr::Concat {
                left: Box::new(ParserFactoryExpr::Concat {
                    left: arg(0),
                    right: arg(1)
                }),
                right: arg(2)
            }
        );
        assert_eq!(
            first("function(a,b) return {-(a .. b)} end"),
            ParserFactoryExpr::Negate(Box::new(ParserFactoryExpr::Concat {
                left: arg(0),
                right: arg(1)
            }))
        );
        let value = first("function(a) return {mod(firstToUpper(a) .. 'Tail', 'BASE', 1)} end");
        assert!(
            matches!(value, ParserFactoryExpr::CreateMod { args } if matches!(&args[0], ParserFactoryExpr::Concat { left, .. } if matches!(**left, ParserFactoryExpr::FirstToUpper { .. })))
        );
    }
    #[test]
    fn string_helper_arity_shadowing_captured_identity_and_complete_shape_are_closed() {
        for body in [
            "function() return {firstToUpper()} end",
            "function(a,b) return {firstToUpper(a,b)} end",
            "function(a) return {firstToUpper(a,firstToUpper(nil))} end",
            "function(firstToUpper) return {firstToUpper('a')} end",
            "function(a) return {a . . 'b'} end",
            "function(a) return {firstToUpper(a):gsub('a','b')} end",
            "function(a) return {string.upper(a)} end",
            "function(a) return firstToUpper(a) end",
        ] {
            assert!(one(body, ParserValue::Nil).is_err(), "{body}");
        }
        for change in 0..3 {
            assert!(
                one_callback(
                    "function(a) return {firstToUpper(a)} end",
                    ParserValue::Nil,
                    |callback| {
                        let u = callback
                            .upvalues
                            .iter_mut()
                            .find(|u| u.name == "firstToUpper")
                            .unwrap();
                        match change {
                            0 => u.name = "otherHelper".into(),
                            1 => u.value = ParserValue::Callback(ParserCallbackId(0)),
                            _ => u.value = ParserValue::Nil,
                        }
                    }
                )
                .is_err()
            );
        }
    }
    #[test]
    fn string_expression_recursion_remains_bounded_before_catalog_installation() {
        let concat = std::iter::repeat_n("a", 100)
            .collect::<Vec<_>>()
            .join(" .. ");
        let nested = format!("{}a{}", "firstToUpper(".repeat(100), ")".repeat(100));
        for expr in [concat, nested] {
            assert!(
                one(
                    &format!("function(a) return {{{expr}}} end"),
                    ParserValue::Nil
                )
                .is_err()
            );
        }
    }
    #[test]
    fn flag_lowering_retains_exact_actual_vector_without_direct_constructor_provenance() {
        for (body, length) in [
            ("function() return flag() end", 0),
            ("function() return flag(nil) end", 1),
            ("function(a) return flag(a, nil, 2, nil, false, nil) end", 6),
        ] {
            let f = one(body, ParserValue::Nil).unwrap();
            assert!(f.provenance.constructor.is_none());
            let ParserFactoryExpr::Flag { args, .. } = f.body else {
                panic!()
            };
            assert_eq!(args.len(), length);
            if length == 6 {
                assert!(matches!(
                    args[1],
                    ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
                ));
                assert!(matches!(
                    args[5],
                    ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
                ));
            }
        }
        let f = one(
            "function() return {flag('A'), mod('B', 'BASE', 1), flag('C')} end",
            ParserValue::Nil,
        )
        .unwrap();
        assert!(f.provenance.constructor.is_some());
    }
    #[test]
    fn flag_lowering_rejects_shadowed_rebound_and_general_variadic_calls() {
        for body in [
            "function(flag) return flag('A') end",
            "function(...) return flag(...) end",
            "function() return flag('A',) end",
            "function() return flag(other()) end",
        ] {
            assert!(one(body, ParserValue::Nil).is_err(), "{body}");
        }
        assert!(
            one_callback("function() return flag('A') end", ParserValue::Nil, |c| c
                .upvalues
                .retain(|u| u.name != "flag"))
            .is_err()
        );
        assert!(
            one_callback("function() return flag('A') end", ParserValue::Nil, |c| c
                .upvalues
                .iter_mut()
                .find(|u| u.name == "flag")
                .unwrap()
                .value =
                ParserValue::Nil)
            .is_err()
        );
    }
    #[test]
    fn number_lowering_preserves_one_value_grouping_and_direct_constructor_semantics() {
        let f = one(
            "function(a) return {mod('N', 'BASE', -tonumber(a)), flag('F', nil, tonumber(a))} end",
            ParserValue::Nil,
        )
        .unwrap();
        assert!(f.provenance.constructor.is_some());
        let ParserFactoryExpr::Table(fields) = f.body else {
            panic!()
        };
        let ParserFactoryField::List(ParserFactoryExpr::CreateMod { args }) = &fields[0] else {
            panic!()
        };
        assert!(
            matches!(&args[2], ParserFactoryExpr::Negate(value) if matches!(**value, ParserFactoryExpr::ToNumber { .. }))
        );
        for child in ["nil", "false", "{}", "'12'", "-0", "tonumber(a)"] {
            let f = one(
                &format!("function(a) return {{tonumber({child})}} end"),
                ParserValue::Nil,
            )
            .unwrap();
            assert!(f.provenance.constructor.is_none());
        }
    }
    #[test]
    fn number_lowering_rejects_omitted_extra_base_shadow_and_scalar_root_forms() {
        for body in [
            "function() return {tonumber()} end",
            "function(a) return {tonumber(a, 10)} end",
            "function(a) return {tonumber(a, nil)} end",
            "function(a) return {tonumber(a, firstToUpper(nil))} end",
            "function(a) return {tonumber(a, nil, firstToUpper(nil))} end",
            "function(tonumber) return {tonumber('1')} end",
            "function(a) return tonumber(a) end",
            "function(a) return {tonumber(a):method()} end",
        ] {
            assert!(one(body, ParserValue::Nil).is_err(), "{body}");
        }
        for captured in [ParserValue::Nil, ParserValue::Number(1.0)] {
            assert!(
                one_callback(
                    "function(a) return {tonumber(a)} end",
                    ParserValue::Nil,
                    |c| c.upvalues.push(ParserUpvalue {
                        name: "tonumber".into(),
                        value: captured
                    })
                )
                .is_err()
            );
        }
    }
    #[test]
    fn number_lowering_bounds_deeply_nested_single_value_calls() {
        let nested = format!("{}a{}", "tonumber(".repeat(100), ")".repeat(100));
        assert!(
            one(
                &format!("function(a) return {{{nested}}} end"),
                ParserValue::Nil
            )
            .is_err()
        );
    }
}
