//! Shared bounded validation for immutable captured source graphs.
use crate::{
    game_data::GameDataError,
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::*,
};
use std::cell::Cell;
type Result<T> = std::result::Result<T, GraphError>;
pub(crate) struct GraphError {
    pub kind: ParserProgramErrorKind,
    pub message: String,
}
impl From<GraphError> for GameDataError {
    fn from(error: GraphError) -> Self {
        Self(error.message)
    }
}
fn error(message: &str) -> GraphError {
    GraphError {
        kind: ParserProgramErrorKind::InvalidData,
        message: format!("source graph: {message}"),
    }
}
fn limit(message: &str) -> GraphError {
    GraphError {
        kind: ParserProgramErrorKind::ResourceLimit,
        message: format!("source graph: {message}"),
    }
}
fn text(value: &str, limit: usize) -> bool {
    value.len() <= limit && !value.contains('\0')
}
fn digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub(crate) struct GraphValidation<'a> {
    source: &'a ItemLoadingSource,
    tables: &'a [ParserTable],
    callbacks: &'a [ParserCallback],
    bytes: Cell<usize>,
    values: Cell<usize>,
}
impl<'a> GraphValidation<'a> {
    pub(crate) fn new(
        source: &'a ItemLoadingSource,
        tables: &'a [ParserTable],
        callbacks: &'a [ParserCallback],
    ) -> Result<Self> {
        let graph = Self {
            source,
            tables,
            callbacks,
            bytes: Cell::new(0),
            values: Cell::new(0),
        };
        if tables.len() > 100_000
            || callbacks.len() > 20_000
            || source.files.len() > 512
            || source.construction_spans.len() > 512
            || source.module_order.len() > 1024
        {
            return Err(limit("source or graph count bound"));
        }
        if !digest(&source.upstream_revision, 40) || source.files.is_empty() {
            return Err(error("invalid source identity"));
        }
        graph.charge(source.upstream_revision.len())?;
        for (path, sha) in &source.files {
            graph.charge(path.len() + sha.len())?;
            if !text(path, 512)
                || path.contains(':')
                || path.contains('\\')
                || path
                    .split('/')
                    .any(|v| v.is_empty() || v == "." || v == "..")
                || !digest(sha, 64)
            {
                return Err(error("invalid source identity"));
            }
        }
        for (key, span) in &source.construction_spans {
            graph.charge(key.len())?;
            if key.is_empty() || !text(key, 256) {
                return Err(error("invalid source span label"));
            }
            graph.span(span)?;
        }
        if source
            .module_order
            .iter()
            .any(|p| !source.files.contains_key(p))
        {
            return Err(error("unknown construction module"));
        }
        for path in &source.module_order {
            graph.charge(path.len())?;
        }
        for table in tables {
            if table.fields.len() + table.indexed.len() > 50_000 {
                return Err(limit("table exceeds row bound"));
            }
            for (key, value) in &table.fields {
                graph.charge(key.len())?;
                if !text(key, 4096) {
                    return Err(error("invalid string key"));
                }
                graph.value(value, false)?;
            }
            for (key, value) in &table.indexed {
                if key.unsigned_abs() > 9_007_199_254_740_991 {
                    return Err(error("numeric key outside exact Lua integer range"));
                }
                graph.value(value, false)?;
            }
        }
        for callback in callbacks {
            match &callback.kind {
                ParserCallbackKind::Lua { source } => graph.span(source)?,
                ParserCallbackKind::Builtin { symbol } => {
                    graph.charge(symbol.len())?;
                    if symbol.is_empty() || !text(symbol, 128) || !callback.upvalues.is_empty() {
                        return Err(error("invalid builtin symbol"));
                    }
                }
            }
            if callback.upvalues.len() > 128 {
                return Err(limit("too many captured upvalues"));
            }
            for capture in &callback.upvalues {
                graph.charge(capture.name.len())?;
                if capture.name.is_empty() || !text(&capture.name, 128) {
                    return Err(error("invalid upvalue name"));
                }
                graph.value(&capture.value, true)?;
            }
        }
        Ok(graph)
    }
    pub(crate) fn charge(&self, amount: usize) -> Result<()> {
        let bytes = self
            .bytes
            .get()
            .checked_add(amount)
            .ok_or_else(|| limit("text byte overflow"))?;
        if bytes > 16 * 1024 * 1024 {
            return Err(limit("aggregate text exceeds resource bound"));
        }
        self.bytes.set(bytes);
        Ok(())
    }
    pub(crate) fn span(&self, span: &ItemSourceSpan) -> Result<()> {
        self.charge(span.path.len() + span.sha256.len())?;
        if !self.source.files.contains_key(&span.path)
            || span.line == 0
            || span.end_line < span.line
            || span.end_line > 1_000_000
            || !digest(&span.sha256, 64)
        {
            return Err(error("invalid source span"));
        }
        Ok(())
    }
    pub(crate) fn table(&self, id: ParserTableId) -> Result<()> {
        if id.0 == 0 || id.0 as usize > self.tables.len() {
            Err(error("dangling table reference"))
        } else {
            Ok(())
        }
    }
    pub(crate) fn callback(&self, id: ParserCallbackId) -> Result<()> {
        if id.0 == 0 || id.0 as usize > self.callbacks.len() {
            Err(error("dangling callback reference"))
        } else {
            Ok(())
        }
    }
    pub(crate) fn value(&self, value: &ParserValue, allow_nil: bool) -> Result<()> {
        let count = self
            .values
            .get()
            .checked_add(1)
            .ok_or_else(|| limit("value count overflow"))?;
        if count > 1_000_000 {
            return Err(limit("aggregate value count exceeds resource bound"));
        }
        self.values.set(count);
        match value {
            ParserValue::Nil if !allow_nil => return Err(error("nil Lua table entry")),
            ParserValue::Number(n) if !n.is_finite() => {
                return Err(error("nonfinite number requires explicit sentinel"));
            }
            ParserValue::Text(value) => {
                if !text(value, 4096) {
                    return Err(error("invalid value text"));
                }
                self.charge(value.len())?;
            }
            ParserValue::Table(id) => self.table(*id)?,
            ParserValue::Callback(id) => self.callback(*id)?,
            _ => {}
        }
        Ok(())
    }
}
