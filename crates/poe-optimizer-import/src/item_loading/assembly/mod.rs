//! Ordered finite item assembly over injected definitions and private owned values.
//! Unsupported local-data or callback operations retain the reached mutation prefix.
mod collect;
mod hydrate;
mod local;
mod projection;
mod query;
mod slots;
#[cfg(test)]
mod tests;
mod text;
mod value;

use super::{AssemblyRequest, DependencyResult, ItemLoadProvider};
use poe_optimizer_data::item_assembly::ItemAssemblyDefinitions;
pub use projection::loading_updates;
use value::{Arena, Result, TableId, Value};
pub use value::{
    AssembledItem, AssemblyError, AssemblyErrorKind, AssemblyLimits, AssemblyTable,
    AssemblyTableId, AssemblyUsage, AssemblyValue,
};

#[derive(Debug)]
pub struct AssemblyAttempt {
    pub result: Result<AssembledItem>,
    pub partial: Option<AssembledItem>,
    pub stage: &'static str,
}

pub fn implementation_sources() -> &'static [&'static str] {
    &[
        include_str!("mod.rs"),
        include_str!("collect.rs"),
        include_str!("hydrate.rs"),
        include_str!("local.rs"),
        include_str!("projection.rs"),
        include_str!("query.rs"),
        include_str!("slots.rs"),
        include_str!("text.rs"),
        include_str!("value.rs"),
    ]
}

pub fn assemble<P: ItemLoadProvider + ?Sized>(
    definitions: ItemAssemblyDefinitions<'_>,
    request: &AssemblyRequest,
    dependencies: &mut P,
    previous: Option<&AssembledItem>,
) -> AssemblyAttempt {
    assemble_with_limits(
        definitions,
        request,
        dependencies,
        previous,
        AssemblyLimits::default(),
    )
}

pub fn assemble_with_limits<P: ItemLoadProvider + ?Sized>(
    definitions: ItemAssemblyDefinitions<'_>,
    request: &AssemblyRequest,
    dependencies: &mut P,
    previous: Option<&AssembledItem>,
    limits: AssemblyLimits,
) -> AssemblyAttempt {
    let reject = |message: &str| AssemblyAttempt {
        result: Err(AssemblyError::unsupported(message)),
        partial: None,
        stage: "input",
    };
    if !request.binding().matches_definitions(definitions.items()) {
        return reject("item assembly definition owner differs from request");
    }
    if previous.is_some_and(|p| p.binding().is_none_or(|b| !b.same_item(request.binding()))) {
        return reject("previous item assembly belongs to another item or definition owner");
    }
    let initialized = if let Some(previous) = previous {
        Arena::from_item(previous, limits).map(|arena| (arena, previous.root()))
    } else {
        let mut arena = Arena::new(limits);
        arena.new_table().map(|root| (arena, root))
    };
    let (arena, root) = match initialized {
        Ok(v) => v,
        Err(error) => {
            return AssemblyAttempt {
                result: Err(error),
                partial: None,
                stage: "input",
            };
        }
    };
    let mut context = Context {
        arena,
        root,
        definitions,
        request,
        dependencies,
        stage: "input",
        patterns: text::Patterns::new(limits),
        sequence: request.state.parser_calls.len()
            + request.state.format_calls.len()
            + request.state.format_parser_calls.len(),
    };
    let result = context
        .hydrate(previous.is_some())
        .and_then(|()| context.collect());
    let stage = context.stage;
    match result {
        Ok(()) => AssemblyAttempt {
            result: context
                .arena
                .finish_complete(root, request.binding().clone()),
            partial: None,
            stage,
        },
        Err(error) => AssemblyAttempt {
            result: Err(error),
            partial: if stage == "input" {
                None
            } else {
                context
                    .arena
                    .finish_partial(root, request.binding().clone())
                    .ok()
            },
            stage,
        },
    }
}

struct Context<'d, 'r, P: ?Sized> {
    arena: Arena,
    root: TableId,
    definitions: ItemAssemblyDefinitions<'d>,
    request: &'r AssemblyRequest,
    dependencies: &'r mut P,
    stage: &'static str,
    patterns: text::Patterns,
    sequence: usize,
}
impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    fn get(&mut self, key: &str) -> Result<Value> {
        self.arena.get_field(self.root, key)
    }
    fn set(&mut self, key: &str, value: Value) -> Result<()> {
        self.arena.set_field(self.root, key, value)
    }
    fn root_table(&mut self, key: &str) -> Result<TableId> {
        table(self.get(key)?)
    }
    fn fresh_field(&mut self, key: &str) -> Result<TableId> {
        let id = self.arena.new_table()?;
        self.set(key, Value::Table(id))?;
        Ok(id)
    }
    fn mod_list(&mut self) -> Result<TableId> {
        let list = self.arena.new_table()?;
        self.arena
            .set_field(list, "parent", Value::Boolean(false))?;
        for name in ["actor", "multipliers", "conditions"] {
            let inner = self.arena.new_table()?;
            self.arena.set_field(list, name, Value::Table(inner))?;
        }
        Ok(list)
    }
    fn base_field(&mut self, key: &str) -> Result<Value> {
        let base = self.root_table("base")?;
        self.arena.get_field(base, key)
    }
    fn add_new(
        &mut self,
        list: TableId,
        name: &str,
        kind: &str,
        value: Value,
        source: Option<&str>,
    ) -> Result<()> {
        let record = self.arena.new_table()?;
        let n = self.arena.text(name)?;
        self.arena.set_field(record, "name", n)?;
        let t = self.arena.text(kind)?;
        self.arena.set_field(record, "type", t)?;
        self.arena.set_field(record, "value", value)?;
        self.arena.set_field(record, "flags", Value::Number(0.0))?;
        self.arena
            .set_field(record, "keywordFlags", Value::Number(0.0))?;
        if let Some(source) = source {
            let v = self.arena.text(source)?;
            self.arena.set_field(record, "source", v)?;
        }
        self.arena.append(list, Value::Table(record))
    }
}
fn table(value: Value) -> Result<TableId> {
    value
        .as_table()
        .ok_or_else(|| AssemblyError::source("attempt to index a non-table item value"))
}
fn number(value: &Value) -> Result<f64> {
    let n = match value {
        Value::Number(n) => Some(*n),
        Value::Text(s) => poe_optimizer_engine::lua_number::parse_number(s.as_bytes()),
        _ => None,
    };
    match n {
        Some(n) if n.is_finite() => Ok(n),
        Some(_) => Err(AssemblyError::unsupported("nonfinite item arithmetic")),
        None => Err(AssemblyError::source(
            "attempt to perform arithmetic on a non-number item value",
        )),
    }
}
fn finite(value: f64) -> Result<Value> {
    if value.is_finite() {
        Ok(Value::Number(value))
    } else {
        Err(AssemblyError::unsupported(
            "nonfinite item arithmetic result",
        ))
    }
}
fn dependency<T>(result: DependencyResult<T>) -> Result<T> {
    match result {
        DependencyResult::Available(v) => Ok(v),
        DependencyResult::Unavailable(e) => Err(AssemblyError::unsupported(e)),
        DependencyResult::SourceError(e) => Err(AssemblyError::source(e)),
        DependencyResult::ResourceError(e) => Err(AssemblyError::resource(e)),
    }
}
