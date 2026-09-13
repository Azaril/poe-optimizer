//! Bounded graph preflight and source-error classification for direct original calls.
//! No hooks, JIT changes, internal-frame inspection or native parity claims.
use super::{factory_source::FactorySource, public_source::Graph};
use mlua::{MultiValue, Value};
use std::collections::BTreeSet;

const MAX_VALUES: usize = 4096;
const MAX_BYTES: usize = 65_536;
fn error(message: &str) -> mlua::Error {
    mlua::Error::RuntimeError(format!("explosion observer: {message}"))
}
/// Bounded raw graph preflight before the existing lossless graph observer.
/// Unknown metatables/callbacks are not silently projected out.
pub fn preflight(values: &[Value]) -> mlua::Result<()> {
    fn visit(
        value: &Value,
        depth: usize,
        seen: &mut BTreeSet<usize>,
        count: &mut usize,
        bytes: &mut usize,
    ) -> mlua::Result<()> {
        *count += 1;
        if depth > 32 || *count > MAX_VALUES {
            return Err(error("graph value/depth bound"));
        }
        match value {
            Value::String(v) => {
                *bytes = bytes
                    .checked_add(v.as_bytes().len())
                    .ok_or_else(|| error("graph byte overflow"))?;
                if *bytes > MAX_BYTES {
                    return Err(error("graph byte bound"));
                }
            }
            Value::Table(t) => {
                if t.metatable().is_some() {
                    return Err(error("unrepresented graph metatable"));
                }
                if seen.insert(t.to_pointer() as usize) {
                    for row in t.pairs::<Value, Value>() {
                        let (k, v) = row?;
                        visit(&k, depth + 1, seen, count, bytes)?;
                        visit(&v, depth + 1, seen, count, bytes)?;
                    }
                }
            }
            Value::Nil | Value::Boolean(_) | Value::Integer(_) | Value::Number(_) => {}
            _ => {
                return Err(error(&format!(
                    "unrepresented graph value type={} depth={depth}",
                    value.type_name()
                )));
            }
        }
        Ok(())
    }
    let (mut seen, mut count, mut bytes) = (BTreeSet::new(), 0, 0);
    for (index, value) in values.iter().enumerate() {
        visit(value, 0, &mut seen, &mut count, &mut bytes).map_err(|failure| {
            error(&format!(
                "root_slot={} root_type={}: {failure}",
                index + 1,
                value.type_name()
            ))
        })?;
    }
    Ok(())
}
pub fn graph(source: &FactorySource, values: MultiValue) -> Graph {
    assert!(
        values.len() <= MAX_VALUES,
        "graph root bound before cloning"
    );
    preflight(values.iter().cloned().collect::<Vec<_>>().as_slice()).unwrap();
    source.public.capture(values).unwrap()
}
/// Expected source failures only. Resource, conversion, external and observer
/// failures never count; the original host deadline is explicitly excluded.
pub fn source_error(error: &mlua::Error, fragment: &str) -> bool {
    source_message(error).is_some_and(|message| message.contains(fragment))
}
pub fn source_error_at(error: &mlua::Error, fragment: &str, line: usize) -> bool {
    source_message(error).is_some_and(|message| {
        message.contains(fragment) && message.contains(&format!("ModParser.lua:{line}:"))
    })
}
fn source_message(error: &mlua::Error) -> Option<&str> {
    let mut current = error;
    for _ in 0..16 {
        match current {
            mlua::Error::CallbackError { cause, .. } => current = cause,
            mlua::Error::RuntimeError(s) => {
                return (s.contains("ModParser.lua:")
                    && !s.contains("explosion observer:")
                    && !s.contains("oracle deadline"))
                .then_some(s.as_str());
            }
            _ => return None,
        }
    }
    None
}
