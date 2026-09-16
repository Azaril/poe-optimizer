//! Bounded offline export of reviewed intrinsic attack data.
//!
//! Only the pinned Misc data literal and isolated unarmed table assignment run.
//! No PoB build, UI, calculator, module loader, or source paths enter the native
//! catalog. Updating the upstream revision requires reviewing this adapter.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_import::{
    owned_intrinsic_attack::{
        IntrinsicAttackCatalog, IntrinsicAttackClass, IntrinsicSourceValue,
        OWNED_INTRINSIC_ATTACK_VERSION,
    },
    owned_mapping::{ExternalSourceSystem, SourcePin},
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::source;

pub const MISC_PATH: &str = "src/Data/Misc.lua";
pub const DATA_PATH: &str = "src/Modules/Data.lua";
const TABLE_START: &str = "\ndata.unarmedWeaponData = {\n";
const TABLE_NEXT: &str = "\ndata.setJewelRadiiGlobally = function(treeVersion)\n";
const HOOK_INTERVAL: usize = 100;

/// Hard ceilings for this small offline data adapter. Callers may lower them.
#[derive(Clone, Copy, Debug)]
pub struct IntrinsicAttackExportLimits {
    pub max_source_bytes: usize,
    pub max_block_bytes: usize,
    pub max_lua_bytes: usize,
    /// Shared across both chunks, checked every 100 VM instructions.
    pub max_instructions: usize,
    pub max_classes: usize,
    pub max_fields: usize,
    pub max_text_bytes: usize,
}
impl Default for IntrinsicAttackExportLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 128 * 1024,
            max_block_bytes: 8 * 1024,
            max_lua_bytes: 8 * 1024 * 1024,
            max_instructions: 100_000,
            max_classes: 64,
            max_fields: 32,
            max_text_bytes: 512,
        }
    }
}
impl IntrinsicAttackExportLimits {
    fn validate(self) -> Result<(), IntrinsicAttackExportError> {
        let ceiling = Self::default();
        for (name, actual, maximum) in [
            (
                "source bytes",
                self.max_source_bytes,
                ceiling.max_source_bytes,
            ),
            ("block bytes", self.max_block_bytes, ceiling.max_block_bytes),
            ("Lua bytes", self.max_lua_bytes, ceiling.max_lua_bytes),
            (
                "instructions",
                self.max_instructions,
                ceiling.max_instructions,
            ),
            ("classes", self.max_classes, ceiling.max_classes),
            ("fields", self.max_fields, ceiling.max_fields),
            ("text bytes", self.max_text_bytes, ceiling.max_text_bytes),
        ] {
            if actual == 0 || actual > maximum {
                return Err(invalid(format!("invalid {name} limit")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum IntrinsicAttackExportError {
    #[error("intrinsic attack export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Source(#[from] source::SourceError),
    #[error("intrinsic attack Lua export: {0}")]
    Lua(#[from] mlua::Error),
}
fn invalid(message: impl Into<String>) -> IntrinsicAttackExportError {
    IntrinsicAttackExportError::Invalid(message.into())
}
type Result<T, E = IntrinsicAttackExportError> = std::result::Result<T, E>;

/// Convert caller-supplied bytes into a portable catalog without reading files.
/// Both full files and the source pin must match the embedded reviewed revision.
/// CRLF is normalized to LF; no other source changes are accepted. All exported
/// rows and fields are retained, including source class 0 and the `type` string.
/// Selection/exclusion and unit conversion belong to the Import policy.
pub fn export_owned_intrinsic_attack(
    misc_bytes: &[u8],
    data_bytes: &[u8],
    source_pin: &SourcePin,
    limits: IntrinsicAttackExportLimits,
) -> Result<IntrinsicAttackCatalog> {
    limits.validate()?;
    if source_pin.system != ExternalSourceSystem::PathOfBuilding2
        || source_pin.revision != source::UPSTREAM_REVISION
        || source_pin.files.len() != 2
    {
        return Err(invalid(
            "source identity must be the reviewed PoB2 revision with exactly two files",
        ));
    }
    let misc = authenticated_text(misc_bytes, MISC_PATH, source_pin, limits)?;
    let data = authenticated_text(data_bytes, DATA_PATH, source_pin, limits)?;
    let block = isolated_table(&data, limits)?;
    let classes = evaluate_tables(&misc, block, limits)?;
    let mut source = source_pin.clone();
    source.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(IntrinsicAttackCatalog {
        schema_version: OWNED_INTRINSIC_ATTACK_VERSION,
        source,
        classes,
    })
}

fn authenticated_text(
    bytes: &[u8],
    path: &str,
    pin: &SourcePin,
    limits: IntrinsicAttackExportLimits,
) -> Result<String> {
    // Bound original bytes before UTF-8 validation or normalization allocates.
    if bytes.len() > limits.max_source_bytes {
        return Err(invalid(format!("source byte limit: {path}")));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| invalid(format!("source is not UTF-8: {path}")))?
        .replace("\r\n", "\n");
    let expected = source::expected_file_sha256(path)?;
    let mut matching = pin.files.iter().filter(|entry| entry.path == path);
    if matching.next().map(|entry| &entry.sha256) != Some(&expected) || matching.next().is_some() {
        return Err(invalid(format!(
            "source pin differs from reviewed manifest: {path}"
        )));
    }
    if format!("{:x}", Sha256::digest(text.as_bytes())) != expected {
        return Err(invalid(format!(
            "source bytes differ from reviewed manifest: {path}"
        )));
    }
    Ok(text)
}

fn isolated_table(data: &str, limits: IntrinsicAttackExportLimits) -> Result<&str> {
    if data.matches(TABLE_START).count() != 1 || data.matches(TABLE_NEXT).count() != 1 {
        return Err(invalid("ambiguous intrinsic attack table boundaries"));
    }
    let begin = data
        .find(TABLE_START)
        .ok_or_else(|| invalid("missing intrinsic attack table"))?
        + 1;
    let end = data
        .find(TABLE_NEXT)
        .ok_or_else(|| invalid("missing intrinsic attack next declaration"))?;
    let block = data
        .get(begin..end)
        .ok_or_else(|| invalid("reversed intrinsic attack table boundaries"))?;
    if block.len() > limits.max_block_bytes || !block.ends_with("}\n") {
        return Err(invalid("intrinsic attack table boundary/byte limit"));
    }
    Ok(block)
}

fn bounded_lua(limits: IntrinsicAttackExportLimits) -> Result<Lua> {
    // Load only JIT control for this trusted initialization, then remove it.
    // Every supplied chunk gets a separate empty environment and no libraries.
    let lua = Lua::new_with(StdLib::JIT, LuaOptions::default())?;
    if lua.used_memory() > limits.max_lua_bytes {
        return Err(invalid("Lua memory limit below initialization"));
    }
    lua.set_memory_limit(limits.max_lua_bytes)?;
    lua.load("jit.off(); jit.flush(); jit=nil").exec()?;
    let count = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(HOOK_INTERVAL as u32),
        move |_, _| {
            if count.fetch_add(HOOK_INTERVAL, Ordering::Relaxed) + HOOK_INTERVAL
                >= limits.max_instructions
            {
                return Err(mlua::Error::RuntimeError(
                    "intrinsic attack instruction limit".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    Ok(lua)
}

fn evaluate_tables(
    misc: &str,
    block: &str,
    limits: IntrinsicAttackExportLimits,
) -> Result<Vec<IntrinsicAttackClass>> {
    let lua = bounded_lua(limits)?;
    let misc_data: Table = lua
        .load(misc)
        .set_name(MISC_PATH)
        .set_mode(mlua::chunk::ChunkMode::Text)
        .set_environment(lua.create_table()?)
        .eval()?;
    plain(&misc_data)?;
    let constants: Table = misc_data.raw_get("characterConstants")?;
    plain(&constants)?;
    let data = lua.create_table()?;
    data.raw_set("characterConstants", constants)?;
    let environment = lua.create_table()?;
    environment.raw_set("data", data.clone())?;
    lua.load(block)
        .set_name("intrinsic-attack-table")
        .set_mode(mlua::chunk::ChunkMode::Text)
        .set_environment(environment)
        .exec()?;
    let classes: Table = data.raw_get("unarmedWeaponData")?;
    plain(&classes)?;
    let mut result = BTreeMap::new();
    for entry in classes.pairs::<Value, Value>() {
        if result.len() >= limits.max_classes {
            return Err(invalid("class count limit"));
        }
        let (key, value) = entry?;
        let class = number(key)?;
        if class.fract() != 0.0 || class < 0.0 || class > u32::MAX as f64 {
            return Err(invalid("class key is not an unsigned integer"));
        }
        let class_key = (class as u32).to_string();
        let Value::Table(fields) = value else {
            return Err(invalid("class row is not a table"));
        };
        plain(&fields)?;
        let mut values = BTreeMap::new();
        for field in fields.pairs::<Value, Value>() {
            if values.len() >= limits.max_fields {
                return Err(invalid("field count limit"));
            }
            let (key, value) = field?;
            let key = text(key, limits)?;
            let value = match value {
                Value::String(_) => IntrinsicSourceValue::Text(text(value, limits)?),
                Value::Integer(_) | Value::Number(_) => {
                    IntrinsicSourceValue::Number(number(value)?)
                }
                _ => return Err(invalid("field must be a finite number or string")),
            };
            values.insert(key, value);
        }
        if values.is_empty() {
            return Err(invalid("class has no fields"));
        }
        result.insert(
            class_key.clone(),
            IntrinsicAttackClass {
                class_key,
                fields: values,
            },
        );
    }
    if result.is_empty() {
        return Err(invalid("catalog has no classes"));
    }
    Ok(result.into_values().collect())
}
fn plain(table: &Table) -> Result<()> {
    if table.metatable().is_some() {
        return Err(invalid("table metatables are not supported"));
    }
    Ok(())
}
fn number(value: Value) -> Result<f64> {
    let number = match value {
        Value::Integer(value) => value as f64,
        Value::Number(value) => value,
        _ => return Err(invalid("expected numeric value")),
    };
    if !number.is_finite() {
        return Err(invalid("non-finite numeric value"));
    }
    Ok(number)
}
fn text(value: Value, limits: IntrinsicAttackExportLimits) -> Result<String> {
    let Value::String(value) = value else {
        return Err(invalid("expected string value"));
    };
    if value.as_bytes().len() > limits.max_text_bytes {
        return Err(invalid("text byte limit"));
    }
    Ok(value.to_str()?.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    const MISC: &str = "return {characterConstants = {crit = 500}}";
    const BLOCK: &str =
        "data.unarmedWeaponData = {[1] = {CritChance = data.characterConstants.crit / 100}}";

    #[test]
    fn isolated_sources_have_no_environment_or_file_access() {
        for name in [
            "_G",
            "getfenv",
            "setfenv",
            "require",
            "package",
            "io",
            "os",
            "ffi",
            "debug",
            "load",
            "loadstring",
            "loadfile",
            "dofile",
            "jit",
        ] {
            let probe_misc =
                format!("return {{characterConstants = {{crit = ({name} == nil and 500 or 0)}}}}");
            assert_eq!(
                evaluate_tables(&probe_misc, BLOCK, Default::default()).unwrap()[0].fields["CritChance"],
                IntrinsicSourceValue::Number(5.0),
                "Misc exposed {name}"
            );
            let probe_table = format!(
                "data.unarmedWeaponData = {{[1] = {{exposed = ({name} == nil and 0 or 1)}}}}"
            );
            assert_eq!(
                evaluate_tables(MISC, &probe_table, Default::default()).unwrap()[0].fields["exposed"],
                IntrinsicSourceValue::Number(0.0),
                "table exposed {name}"
            );
        }
        assert_eq!(
            evaluate_tables(MISC, BLOCK, Default::default()).unwrap()[0].fields["CritChance"],
            IntrinsicSourceValue::Number(5.0)
        );
    }

    #[test]
    fn loops_allocations_and_bytecode_are_rejected() {
        let limits = IntrinsicAttackExportLimits {
            max_instructions: 1_000,
            ..Default::default()
        };
        assert!(
            evaluate_tables("while true do end", BLOCK, limits)
                .unwrap_err()
                .to_string()
                .contains("instruction limit")
        );
        assert!(
            evaluate_tables(MISC, "while true do end", limits)
                .unwrap_err()
                .to_string()
                .contains("instruction limit")
        );
        let limits = IntrinsicAttackExportLimits {
            max_lua_bytes: 128 * 1024,
            ..Default::default()
        };
        let allocation = evaluate_tables(
            "local t = {}; for i=1,100000 do t[i]={i,i,i} end; return {characterConstants=t}",
            BLOCK,
            limits,
        )
        .unwrap_err();
        assert!(
            matches!(
                allocation,
                IntrinsicAttackExportError::Lua(mlua::Error::MemoryError(_))
            ),
            "{allocation}"
        );
        assert!(evaluate_tables("\u{1b}Lua", BLOCK, Default::default()).is_err());
    }

    #[test]
    fn complete_table_extraction_rejects_unsupported_shapes_and_values() {
        for row in [
            "true",
            "{}",
            "{damage = 0/0}",
            "{damage = 1/0}",
            "{damage = true}",
            "{damage = {}}",
            "{[1] = 5}",
        ] {
            let block = format!("data.unarmedWeaponData = {{[1] = {row}}}");
            assert!(
                evaluate_tables(MISC, &block, Default::default()).is_err(),
                "accepted {row}"
            );
        }
        for key in ["1.5", "-1", "4294967296", "'1'"] {
            let block = format!("data.unarmedWeaponData = {{[{key}] = {{damage = 1}}}}");
            assert!(
                evaluate_tables(MISC, &block, Default::default()).is_err(),
                "accepted {key}"
            );
        }
        assert!(evaluate_tables(MISC, "data.unarmedWeaponData = {}", Default::default()).is_err());
    }
}
