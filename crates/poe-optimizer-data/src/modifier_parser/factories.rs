//! Bounded pure factory definitions. Membership grants no call-site capability.
use super::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserFactoryDisposition {
    Pure(Box<ParserPureFactory>),
    Unsupported { reason: String },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserPureFactory {
    pub parameter_count: u16,
    pub body: ParserFactoryExpr,
    pub provenance: ParserFactoryProvenance,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserFactoryProvenance {
    pub source: ItemSourceSpan,
    /// Half-open byte offsets within the LF-normalized source span.
    pub function_start: u32,
    pub function_end: u32,
    pub function_sha256: String,
    pub constructor: Option<ParserCallbackId>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ParserFactoryLiteral {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(String),
    NonFinite(ParserNonFinite),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ParserFactoryExpr {
    Literal(ParserFactoryLiteral),
    /// Zero-based fixed-parameter slot; a missing actual value is nil.
    Argument(u16),
    /// Runtime kind is checked only when selected.
    CapturedScalar {
        upvalue: u16,
    },
    ConstantField {
        table: ParserTableId,
        key: String,
    },
    Negate(Box<ParserFactoryExpr>),
    Table(Vec<ParserFactoryField>),
    /// Includes name/type/value and all variadic nil holes.
    CreateMod {
        args: Vec<ParserFactoryExpr>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ParserFactoryField {
    Named {
        key: String,
        value: ParserFactoryExpr,
    },
    List(ParserFactoryExpr),
}
struct Bounds {
    nodes: usize,
    bytes: usize,
}
impl Bounds {
    fn charge(&mut self, bytes: usize) -> Result<()> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| catalog_error("factory node overflow"))?;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| catalog_error("factory byte overflow"))?;
        if self.nodes > 500_000 || self.bytes > 8 * 1024 * 1024 {
            return Err(catalog_error("factory aggregate resource bound"));
        }
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<()> {
        if value.len() > 4096 {
            return Err(catalog_error("factory string resource bound"));
        }
        self.charge(value.len())
    }
}
pub(super) fn validate(data: &ModifierParserData) -> Result<()> {
    if data.factories.len() != data.callbacks.len() {
        return Err(catalog_error("incomplete factory dispositions"));
    }
    let mut bounds = Bounds { nodes: 0, bytes: 0 };
    for (index, callback) in data.callbacks.iter().enumerate() {
        let disposition = data
            .factories
            .get(&ParserCallbackId(index as u32 + 1))
            .ok_or_else(|| catalog_error("missing factory disposition"))?;
        match disposition {
            ParserFactoryDisposition::Unsupported { reason } => {
                bounds.string(reason)?;
                if reason.is_empty() {
                    return Err(catalog_error("empty factory unsupported reason"));
                }
            }
            ParserFactoryDisposition::Pure(factory) => {
                let ParserCallbackKind::Lua { source } = &callback.kind else {
                    return Err(catalog_error("builtin cannot carry a pure source factory"));
                };
                let p = &factory.provenance;
                if p.source != *source
                    || p.function_start >= p.function_end
                    || p.function_end > 1024 * 1024
                    || !digest(&p.function_sha256, 64)
                    || factory.parameter_count > 128
                {
                    return Err(catalog_error("invalid factory source linkage"));
                }
                bounds.string(&p.source.path)?;
                bounds.string(&p.source.sha256)?;
                bounds.string(&p.function_sha256)?;
                if !matches!(
                    factory.body,
                    ParserFactoryExpr::Table(_)
                        | ParserFactoryExpr::CreateMod { .. }
                        | ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil)
                ) {
                    return Err(catalog_error("factory result must be a table or nil"));
                }
                if let Some(constructor) = p.constructor {
                    let Some(target) = constructor
                        .0
                        .checked_sub(1)
                        .and_then(|i| data.callbacks.get(i as usize))
                    else {
                        return Err(catalog_error("dangling factory constructor"));
                    };
                    if !matches!(target.kind, ParserCallbackKind::Lua { .. })
                        || !callback.upvalues.iter().any(|u| {
                            u.name == "mod" && u.value == ParserValue::Callback(constructor)
                        })
                    {
                        return Err(catalog_error(
                            "factory constructor is not its captured binding",
                        ));
                    }
                }
                let mut uses_constructor = false;
                expression(
                    &factory.body,
                    factory,
                    callback,
                    data,
                    &mut bounds,
                    0,
                    &mut uses_constructor,
                )?;
                if uses_constructor != p.constructor.is_some() {
                    return Err(catalog_error("factory constructor dependency mismatch"));
                }
            }
        }
    }
    Ok(())
}
fn expression(
    expr: &ParserFactoryExpr,
    factory: &ParserPureFactory,
    callback: &ParserCallback,
    data: &ModifierParserData,
    bounds: &mut Bounds,
    depth: usize,
    uses_constructor: &mut bool,
) -> Result<()> {
    if depth > 32 {
        return Err(catalog_error("factory expression depth bound"));
    }
    bounds.charge(0)?;
    match expr {
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Number(n)) if !n.is_finite() => {
            return Err(catalog_error("factory nonfinite number needs sentinel"));
        }
        ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(v)) => bounds.string(v)?,
        ParserFactoryExpr::Literal(_) => {}
        ParserFactoryExpr::Argument(slot) if *slot >= factory.parameter_count => {
            return Err(catalog_error("factory argument outside parameter list"));
        }
        ParserFactoryExpr::Argument(_) => {}
        ParserFactoryExpr::CapturedScalar { upvalue }
            if *upvalue as usize >= callback.upvalues.len() =>
        {
            return Err(catalog_error("factory captured slot out of range"));
        }
        ParserFactoryExpr::CapturedScalar { .. } => {}
        ParserFactoryExpr::ConstantField { table, key } => {
            if ![
                data.policy.mod_flags,
                data.policy.keyword_flags,
                data.policy.skill_types,
            ]
            .contains(table)
            {
                return Err(catalog_error(
                    "factory constant is not an authenticated policy root",
                ));
            }
            bounds.string(key)?;
        }
        ParserFactoryExpr::Negate(value) => expression(
            value,
            factory,
            callback,
            data,
            bounds,
            depth + 1,
            uses_constructor,
        )?,
        ParserFactoryExpr::Table(fields) => {
            if fields.len() > 4096 {
                return Err(catalog_error("factory table field bound"));
            }
            let mut keys = BTreeSet::new();
            for field in fields {
                let value = match field {
                    ParserFactoryField::Named { key, value } => {
                        bounds.string(key)?;
                        if !keys.insert(key.as_str()) {
                            return Err(catalog_error("duplicate factory table key"));
                        }
                        value
                    }
                    ParserFactoryField::List(value) => value,
                };
                expression(
                    value,
                    factory,
                    callback,
                    data,
                    bounds,
                    depth + 1,
                    uses_constructor,
                )?;
            }
        }
        ParserFactoryExpr::CreateMod { args } => {
            *uses_constructor = true;
            if args.len() > 4096 {
                return Err(catalog_error("factory argument count bound"));
            }
            for arg in args {
                expression(
                    arg,
                    factory,
                    callback,
                    data,
                    bounds,
                    depth + 1,
                    uses_constructor,
                )?;
            }
        }
    }
    Ok(())
}
