//! Complete source configuration construction. Callbacks remain inert provenance.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_core::options::Scalar;
use poe_optimizer_data::configuration::*;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

type Result<T> = std::result::Result<T, GameDataExtractionError>;
const CONFIG: &str = "src/Modules/ConfigOptions.lua";
const QUESTS: &str = "src/Data/QuestRewards.lua";
const CONFIG_TAB: &str = "src/Classes/ConfigTab.lua";
const FILES: &[&str] = &[
    "src/Modules/Common.lua",
    "src/Data/Global.lua",
    "src/Data/Misc.lua",
    "src/Modules/Data.lua",
    QUESTS,
    "src/Data/Bosses.lua",
    "src/Data/BossSkills.lua",
    CONFIG,
    CONFIG_TAB,
];

fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("missing configuration source {path}")))
}

fn span(
    sources: &BTreeMap<String, String>,
    path: &str,
    first: usize,
    last: usize,
) -> Result<ConfigSourceSpan> {
    let text = source(sources, path)?;
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    if first == 0 || first > last || last > lines.len() {
        return Err(error("invalid configuration source span"));
    }
    Ok(ConfigSourceSpan {
        path: path.into(),
        line: first.try_into().map_err(error)?,
        end_line: last.try_into().map_err(error)?,
        sha256: hash(lines[first - 1..last].concat().as_bytes()),
    })
}
fn anchored_span(
    sources: &BTreeMap<String, String>,
    path: &str,
    begin: &str,
    end: &str,
) -> Result<ConfigSourceSpan> {
    let text = source(sources, path)?;
    let chunk = section(text, begin, end)?;
    let offset = chunk.as_ptr() as usize - text.as_ptr() as usize;
    let first = text[..offset].bytes().filter(|b| *b == b'\n').count() + 1;
    let last = first + chunk.trim_end().bytes().filter(|b| *b == b'\n').count();
    span(sources, path, first, last)
}
// Compile the original method declaration without invoking it. Preserving its
// source line offset lets the host query the exact end, excluding adjacent comments.
fn method_span(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    name: &str,
    next: &str,
) -> Result<ConfigSourceSpan> {
    let text = source(sources, CONFIG_TAB)?;
    let chunk = section(
        text,
        &format!("function ConfigTabClass:{name}("),
        &format!("\nfunction ConfigTabClass:{next}("),
    )?;
    let offset = chunk.as_ptr() as usize - text.as_ptr() as usize;
    let preceding_lines = text[..offset].bytes().filter(|b| *b == b'\n').count();
    let code = format!(
        "{}local ConfigTabClass={{}};{chunk}\nreturn ConfigTabClass.{name}",
        "\n".repeat(preceding_lines)
    );
    callback(
        sources,
        lua.load(code).set_name(format!("@{CONFIG_TAB}")).eval()?,
    )
}

fn callback(sources: &BTreeMap<String, String>, function: Function) -> Result<ConfigSourceSpan> {
    let info = function.info();
    if info.what != "Lua" {
        return Err(error("configuration callback is not source Lua"));
    }
    let path = info
        .source
        .as_deref()
        .and_then(|s| s.strip_prefix('@'))
        .ok_or_else(|| error("configuration callback has no authenticated source path"))?;
    span(
        sources,
        path,
        info.line_defined
            .ok_or_else(|| error("callback start absent"))?,
        info.last_line_defined
            .ok_or_else(|| error("callback end absent"))?,
    )
}

fn scalar(value: Value) -> Result<Scalar> {
    match value {
        Value::Boolean(v) => Ok(Scalar::Boolean(v)),
        Value::Integer(v) => Ok(Scalar::Number(v as f64)),
        Value::Number(v) if v.is_finite() => Ok(Scalar::Number(v)),
        Value::String(v) => Ok(Scalar::Text(v.to_str()?.to_owned())),
        _ => Err(error(
            "configuration scalar has unsupported or nonfinite source type",
        )),
    }
}
fn optional_scalar(table: &Table, key: &str) -> Result<Option<Scalar>> {
    match table.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        value => Ok(Some(scalar(value)?)),
    }
}
fn scalar_kind(value: &Scalar) -> ConfigScalarKind {
    match value {
        Scalar::Boolean(_) => ConfigScalarKind::Boolean,
        Scalar::Number(_) => ConfigScalarKind::Number,
        Scalar::Text(_) => ConfigScalarKind::Text,
    }
}
fn integer(value: Value) -> Result<u32> {
    let n = match value {
        Value::Integer(v) => v as f64,
        Value::Number(v) => v,
        _ => return Err(error("configuration index is not numeric")),
    };
    if n.is_finite() && n >= 1.0 && n <= f64::from(u32::MAX) && n.fract() == 0.0 {
        Ok(n as u32)
    } else {
        Err(error("configuration index is not positive u32"))
    }
}
fn dense(table: &Table) -> Result<usize> {
    if table.metatable().is_some() {
        return Err(error("configuration source table has a metatable"));
    }
    let mut count = 0usize;
    let mut maximum = 0usize;
    for pair in table.clone().pairs::<Value, Value>() {
        let (key, _) = pair?;
        let key = integer(key)? as usize;
        maximum = maximum.max(key);
        count += 1;
        if count > 100_000 {
            return Err(error("configuration source array exceeds bound"));
        }
    }
    if count != maximum {
        return Err(error("configuration source array is sparse"));
    }
    Ok(count)
}
fn metadata(
    value: Value,
    sources: &BTreeMap<String, String>,
    depth: usize,
) -> Result<ConfigMetadataValue> {
    if depth > 16 {
        return Err(error("configuration metadata exceeds nesting bound"));
    }
    Ok(match value {
        Value::Boolean(v) => ConfigMetadataValue::Boolean(v),
        Value::Integer(v) => ConfigMetadataValue::Number(v as f64),
        Value::Number(v) if v.is_finite() => ConfigMetadataValue::Number(v),
        Value::String(v) => ConfigMetadataValue::Text(v.to_str()?.to_owned()),
        Value::Function(f) => ConfigMetadataValue::Callback(callback(sources, f)?),
        Value::Table(t) => {
            if t.metatable().is_some() {
                return Err(error("configuration metadata has a metatable"));
            }
            let mut any_index = false;
            let mut any_name = false;
            for pair in t.clone().pairs::<Value, Value>() {
                match pair?.0 {
                    Value::String(_) => any_name = true,
                    Value::Integer(_) | Value::Number(_) => any_index = true,
                    _ => return Err(error("configuration metadata key unsupported")),
                }
            }
            if any_index && any_name {
                return Err(error(
                    "configuration metadata table mixes index and name keys",
                ));
            }
            if any_index {
                let size = dense(&t)?;
                ConfigMetadataValue::Array(
                    (1..=size)
                        .map(|i| metadata(t.raw_get(i)?, sources, depth + 1))
                        .collect::<Result<_>>()?,
                )
            } else {
                ConfigMetadataValue::Object(object(t, sources, depth + 1, &[])?)
            }
        }
        _ => return Err(error("configuration metadata source value unsupported")),
    })
}
fn object(
    table: Table,
    sources: &BTreeMap<String, String>,
    depth: usize,
    omit: &[&str],
) -> Result<BTreeMap<String, ConfigMetadataValue>> {
    if depth > 16 || table.metatable().is_some() {
        return Err(error(
            "configuration source object exceeds bounds or has metatable",
        ));
    }
    let mut result = BTreeMap::new();
    for pair in table.pairs::<Value, Value>() {
        let (Value::String(key), value) = pair? else {
            return Err(error("configuration object has non-string field"));
        };
        let key = key.to_str()?.to_owned();
        if !omit.contains(&key.as_str()) {
            result.insert(key, metadata(value, sources, depth + 1)?);
        }
        if result.len() > 4096 {
            return Err(error("configuration object exceeds field bound"));
        }
    }
    Ok(result)
}

// This scanner identifies source occurrences only. Source Lua performs every value
// construction; comments, quoted strings and long strings are never searched as code.
#[derive(Debug)]
struct Token<'a> {
    text: &'a str,
    line: usize,
    quoted: bool,
}
fn tokens(text: &str) -> Result<Vec<Token<'_>>> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut line = 1;
    let mut out = Vec::new();
    while i < bytes.len() {
        let start = i;
        let start_line = line;
        if bytes[i].is_ascii_whitespace() {
            if bytes[i] == b'\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        let comment = bytes[i..].starts_with(b"--");
        if comment {
            i += 2;
        }
        let long_start = i;
        let long_open = if bytes.get(i) == Some(&b'[') {
            let mut j = i + 1;
            while bytes.get(j) == Some(&b'=') {
                j += 1;
            }
            (bytes.get(j) == Some(&b'[')).then_some(j)
        } else {
            None
        };
        if let Some(end) = long_open {
            let closer = format!("]{}]", "=".repeat(end - long_start - 1));
            let tail = &text[end + 1..];
            let close = tail
                .find(&closer)
                .ok_or_else(|| error("unterminated Lua long literal"))?;
            i = end + 1 + close + closer.len();
            line += text[start..i].bytes().filter(|b| *b == b'\n').count();
            if !comment {
                out.push(Token {
                    text: &text[start..i],
                    line: start_line,
                    quoted: true,
                });
            }
            continue;
        }
        if comment {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let quoted = bytes[i] == b'\'' || bytes[i] == b'"';
        if quoted {
            let quote = bytes[i];
            i += 1;
            let mut closed = false;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 1;
                    if bytes.get(i) == Some(&b'\n') {
                        line += 1;
                    }
                    i += 1;
                } else if bytes[i] == quote {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    if bytes[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            if !closed {
                return Err(error("unterminated Lua quoted literal"));
            }
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
        } else {
            i += 1;
        }
        // All code outside literals in the reviewed files is ASCII. Reject an
        // unsupported future source spelling instead of slicing a UTF-8 codepoint.
        if !text.is_char_boundary(i) {
            return Err(error(
                "non-ASCII Lua code token requires reviewed scanner support",
            ));
        }
        out.push(Token {
            text: &text[start..i],
            line: start_line,
            quoted,
        });
    }
    Ok(out)
}
fn static_locations(lua: &Lua, text: &str) -> Result<BTreeMap<String, Vec<usize>>> {
    let tokens = tokens(text)?;
    let begin = tokens
        .windows(4)
        .position(|w| {
            w[0].text == "local"
                && w[1].text == "configSettings"
                && w[2].text == "="
                && w[3].text == "{"
        })
        .ok_or_else(|| error("configuration source root table absent"))?
        + 3;
    let mut result: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut depth = 0usize;
    for i in begin..tokens.len() {
        let token = &tokens[i];
        if token.quoted {
            continue;
        }
        if token.text == "{" {
            depth += 1;
        } else if token.text == "}" {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| error("unbalanced configuration source braces"))?;
            if depth == 0 {
                return Ok(result);
            }
        } else if depth == 2
            && token.text == "var"
            && tokens.get(i + 1).is_some_and(|t| t.text == "=")
            && tokens.get(i + 2).is_some_and(|t| t.quoted)
        {
            let key: String = lua.load(format!("return {}", tokens[i + 2].text)).eval()?;
            result.entry(key).or_default().push(token.line);
        }
    }
    Err(error("unbalanced configuration source root table"))
}

fn quest_locations(text: &str) -> Result<Vec<usize>> {
    let mut depth = 0usize;
    let mut result = Vec::new();
    for token in tokens(text)? {
        if token.quoted {
            continue;
        }
        if token.text == "{" {
            depth += 1;
            if depth == 2 {
                result.push(token.line);
            }
        } else if token.text == "}" {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| error("unbalanced quest source braces"))?;
        }
    }
    if depth != 0 {
        return Err(error("unbalanced quest source braces"));
    }
    Ok(result)
}

pub(crate) fn extract(sources: &BTreeMap<String, String>) -> Result<ConfigurationData> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT | StdLib::JIT,
        LuaOptions::default(),
    )?;
    lua.set_memory_limit(64 * 1024 * 1024)?;
    lua.load("jit.off();jit.flush();jit=nil;load=nil;loadstring=nil;loadfile=nil;dofile=nil;collectgarbage=nil;print=nil;package=nil;io=nil;os=nil;debug=nil;ffi=nil;require=nil").exec()?;
    let ticks = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(10_000),
        move |_, _| {
            if ticks.fetch_add(1, Ordering::Relaxed) >= 10_000 {
                return Err(mlua::Error::RuntimeError(
                    "configuration extraction instruction bound".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    let common = source(sources, "src/Modules/Common.lua")?;
    lua.load(section(
        common,
        "function copyTable(tbl, noRecurse)\n",
        "\ndo\n",
    )?)
    .exec()?;
    lua.load(source(sources, "src/Data/Global.lua")?).exec()?;
    let data: Table = lua.load(source(sources, "src/Data/Misc.lua")?).eval()?;
    lua.globals().set("data", data.clone())?;
    let quests: Table = lua
        .load(source(sources, QUESTS)?)
        .set_name(format!("@{QUESTS}"))
        .eval()?;
    data.set("questRewards", quests.clone())?;
    lua.load(section(
        source(sources, "src/Modules/Data.lua")?,
        "data.misc = {",
        "\ndata.skillColorMap = ",
    )?)
    .exec()?;
    let bosses: Table = lua.load(source(sources, "src/Data/Bosses.lua")?).eval()?;
    let skills: Table = lua
        .load(source(sources, "src/Data/BossSkills.lua")?)
        .eval()?;
    lua.globals().set(
        "LoadModule",
        lua.create_function(move |_, name: String| match name.as_str() {
            "Data/Bosses" => Ok(bosses.clone()),
            "Data/BossSkills" => Ok(skills.clone()),
            _ => Err(mlua::Error::RuntimeError(
                "configuration module outside allowlist".into(),
            )),
        })?,
    )?;
    lua.load(format!(
        "local m_floor=math.floor;{}",
        section(
            source(sources, "src/Modules/Data.lua")?,
            "-- Load bosses\n",
            "-- Load skills\n"
        )?
    ))
    .exec()?;
    lua.globals().set("LoadModule", Value::Nil)?;
    let rows: Table = lua
        .load(source(sources, CONFIG)?)
        .set_name(format!("@{CONFIG}"))
        .eval()?;
    let count = dense(&rows)?;
    let mut locations = static_locations(&lua, source(sources, CONFIG)?)?;
    let quest_lines = quest_locations(source(sources, QUESTS)?)?;
    if dense(&quests)? != quest_lines.len() {
        return Err(error("quest source occurrence count mismatch"));
    }
    let generator = anchored_span(
        sources,
        CONFIG,
        "local function addQuestModsRewardsConfigOptions(",
        "\nlocal configSettings = {",
    )?;
    let mut generated = BTreeMap::new();
    for (index, line) in quest_lines.into_iter().enumerate() {
        let quest: Table = quests.raw_get(index + 1)?;
        if quest.raw_get::<Value>("useConfig")? == Value::Boolean(false) {
            continue;
        }
        if matches!(quest.raw_get::<Value>("Stat")?, Value::Nil)
            && matches!(quest.raw_get::<Value>("Options")?, Value::Nil)
        {
            continue;
        }
        let key = format!(
            "quest{}{}{}",
            quest.raw_get::<String>("Description")?,
            quest.raw_get::<String>("Area")?,
            quest.raw_get::<String>("Info")?
        );
        let record = object(quest, sources, 0, &[])?;
        if generated.insert(key, (index + 1, line, record)).is_some() {
            return Err(error("ambiguous generated quest definition identity"));
        }
    }
    let mut definitions = Vec::new();
    for i in 1..=count {
        let row: Table = rows.raw_get(i)?;
        let Some(key) = row.raw_get::<Option<String>>("var")? else {
            object(row, sources, 0, &[])?;
            continue;
        };
        let widget = match row.raw_get::<String>("type")?.as_str() {
            "check" => ConfigWidgetKind::Check,
            "count" => ConfigWidgetKind::Count,
            "countAllowZero" => ConfigWidgetKind::CountAllowZero,
            "integer" => ConfigWidgetKind::Integer,
            "float" => ConfigWidgetKind::Float,
            "list" => ConfigWidgetKind::List,
            "text" => ConfigWidgetKind::Text,
            other => return Err(error(format!("unsupported configuration widget {other}"))),
        };
        let mut options = Vec::new();
        if let Some(list) = row.raw_get::<Option<Table>>("list")? {
            for j in 1..=dense(&list)? {
                let option: Table = list.raw_get(j)?;
                options.push(ConfigOption {
                    index: j.try_into().map_err(error)?,
                    value: scalar(option.raw_get("val")?)?,
                    label: option.raw_get("label")?,
                    metadata: object(option, sources, 0, &["val", "label"])?,
                });
            }
        }
        let mut scalar_kinds = Vec::new();
        if widget == ConfigWidgetKind::List {
            for option in &options {
                let kind = scalar_kind(&option.value);
                if !scalar_kinds.contains(&kind) {
                    scalar_kinds.push(kind);
                }
            }
        } else {
            scalar_kinds.push(match widget {
                ConfigWidgetKind::Check => ConfigScalarKind::Boolean,
                ConfigWidgetKind::Text => ConfigScalarKind::Text,
                _ => ConfigScalarKind::Number,
            });
        }
        let definition_source = if let Some((index, line, record)) = generated.remove(&key) {
            if locations.contains_key(&key) {
                return Err(error("static/generated configuration identity collision"));
            }
            ConfigDefinitionSource {
                location: ConfigSourceLocation {
                    path: QUESTS.into(),
                    line: line.try_into().map_err(error)?,
                },
                quest: Some(ConfigQuestSource {
                    source_index: index.try_into().map_err(error)?,
                    record,
                }),
                generator: Some(generator.clone()),
            }
        } else {
            let lines = locations
                .get_mut(&key)
                .ok_or_else(|| error(format!("unlocated configuration definition {key}")))?;
            if lines.is_empty() {
                return Err(error("configuration source occurrence exhausted"));
            }
            let line = lines.remove(0);
            ConfigDefinitionSource {
                location: ConfigSourceLocation {
                    path: CONFIG.into(),
                    line: line.try_into().map_err(error)?,
                },
                quest: None,
                generator: None,
            }
        };
        definitions.push(ConfigDefinition {
            id: format!("config-row-{i:04}"),
            key,
            source_table_index: i.try_into().map_err(error)?,
            source_variable_order: (definitions.len() + 1).try_into().map_err(error)?,
            widget,
            scalar_kinds,
            label: row.raw_get("label")?,
            options,
            defaults: ConfigDeclaredDefaults {
                input: optional_scalar(&row, "defaultState")?,
                placeholder: optional_scalar(&row, "defaultPlaceholderState")?,
                option_index: match row.raw_get::<Value>("defaultIndex")? {
                    Value::Nil => None,
                    v => Some(integer(v)?),
                },
            },
            source: definition_source,
            metadata: object(
                row,
                sources,
                0,
                &[
                    "var",
                    "type",
                    "label",
                    "list",
                    "defaultState",
                    "defaultPlaceholderState",
                    "defaultIndex",
                ],
            )?,
        });
    }
    if !generated.is_empty() || locations.values().any(|v| !v.is_empty()) {
        return Err(error(format!(
            "unconsumed configuration source occurrences: quests={:?}, static={:?}",
            generated.keys().collect::<Vec<_>>(),
            locations
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .collect::<Vec<_>>()
        )));
    }
    let result = ConfigurationData {
        schema_version: 1,
        source: ConfigSourceIdentity {
            upstream_revision: crate::source::UPSTREAM_REVISION.into(),
            files: FILES
                .iter()
                .map(|p| Ok(((*p).into(), hash(source(sources, p)?.as_bytes()))))
                .collect::<Result<_>>()?,
            create_config_set: method_span(&lua, sources, "CreateConfigSet", "NewConfigSet")?,
            get_default_state: method_span(&lua, sources, "GetDefaultState", "Save")?,
        },
        source_table_rows: count.try_into().map_err(error)?,
        definitions,
        capability: ConfigCapability::MetadataOnly,
    };
    result.validate().map_err(error)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    fn sources() -> BTreeMap<String, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        FILES
            .iter()
            .map(|path| {
                (
                    (*path).into(),
                    crate::source::read_verified_text(&root, path).unwrap(),
                )
            })
            .collect()
    }
    fn visit(
        value: &ConfigMetadataValue,
        callbacks: &mut Vec<ConfigSourceSpan>,
        maximum: &mut usize,
    ) {
        match value {
            ConfigMetadataValue::Text(text) => *maximum = (*maximum).max(text.len()),
            ConfigMetadataValue::Callback(span) => callbacks.push(span.clone()),
            ConfigMetadataValue::Array(values) => {
                for value in values {
                    visit(value, callbacks, maximum)
                }
            }
            ConfigMetadataValue::Object(values) => {
                for value in values.values() {
                    visit(value, callbacks, maximum)
                }
            }
            _ => {}
        }
    }
    #[test]
    fn complete_configuration_construction_preserves_source_rows_values_and_inert_callbacks() {
        let sources = sources();
        let first = extract(&sources).unwrap();
        let second = extract(&sources).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.source_table_rows, 663);
        assert_eq!(first.definitions.len(), 564);
        assert_eq!(first.source.files.len(), 9);
        assert_eq!(first.source.create_config_set.line, 1317);
        assert_eq!(first.source.create_config_set.end_line, 1334);
        assert_eq!(
            first.source.create_config_set,
            span(&sources, CONFIG_TAB, 1317, 1334).unwrap()
        );
        assert_eq!(
            first.source.get_default_state,
            span(&sources, CONFIG_TAB, 980, 998).unwrap()
        );
        assert_eq!(
            first
                .definitions
                .iter()
                .filter(|d| d.source.quest.is_some())
                .count(),
            17
        );
        let repeated: Vec<_> = first
            .definitions
            .iter()
            .filter(|d| d.key == "conditionEnemyExitedPresenceRecently")
            .collect();
        assert_eq!(repeated.len(), 2);
        assert_eq!(repeated[0].source_table_index, 523);
        assert_eq!(repeated[1].source_table_index, 524);
        assert_ne!(repeated[0].id, repeated[1].id);
        assert_ne!(
            repeated[0].metadata.get("apply"),
            repeated[1].metadata.get("apply")
        );
        let penalty = first
            .definitions
            .iter()
            .find(|d| d.key == "resistancePenalty")
            .unwrap();
        assert_eq!(penalty.defaults.option_index, Some(7));
        assert_eq!(penalty.defaults.input, None);
        assert_eq!(penalty.options[6].value, Scalar::Number(-60.0));
        let boss = first
            .definitions
            .iter()
            .find(|d| d.key == "enemyIsBoss")
            .unwrap();
        assert!(
            matches!(boss.metadata.get("tooltip"),Some(ConfigMetadataValue::Text(v)) if v.contains("Pinnacle Boss") && v.contains("monster Armour"))
        );
        let quest = first
            .definitions
            .iter()
            .find(|d| d.key == "questAct 2Valley of the TitansMedallion")
            .unwrap();
        assert_eq!(
            quest.options[1].value,
            Scalar::Text("30% increased Charm Charges Gained\n\t+1 Charm Slot".into())
        );
        let mut callbacks = Vec::new();
        let mut maximum = 0;
        for definition in &first.definitions {
            for value in definition.metadata.values() {
                visit(value, &mut callbacks, &mut maximum);
            }
            for option in &definition.options {
                for value in option.metadata.values() {
                    visit(value, &mut callbacks, &mut maximum);
                }
            }
        }
        assert!(callbacks.len() >= 537);
        for actual in &callbacks {
            assert_eq!(
                actual,
                &span(
                    &sources,
                    &actual.path,
                    actual.line as usize,
                    actual.end_line as usize
                )
                .unwrap()
            );
        }
        eprintln!(
            "configuration rows={} definitions={} callbacks={} maximum_metadata_string_bytes={} json_bytes={}",
            first.source_table_rows,
            first.definitions.len(),
            callbacks.len(),
            maximum,
            serde_json::to_vec(&first).unwrap().len()
        );
    }
    #[test]
    fn source_locations_skip_comments_strings_and_preserve_duplicate_occurrences() {
        let lua = Lua::new();
        let locations=static_locations(&lua,"-- var='fake'\nlocal text=[=[var='hidden']=]\nlocal configSettings = {{var='real'}, {var='real'} --[=[ var='comment' ]=]\n,{ var=\"escaped\\nkey\" }}").unwrap();
        assert_eq!(locations.len(), 2);
        assert_eq!(locations["real"], [3, 3]);
        assert_eq!(locations["escaped\nkey"], [4]);
        assert_eq!(
            quest_locations("return {{ Options={'x','y'} }, -- { }\n{ Info='\\\"{' }}").unwrap(),
            [1, 2]
        );
        assert!(tokens("--[=[unterminated").is_err());
        assert!(tokens("'unterminated").is_err());
    }
    #[test]
    fn extractor_rejects_ambiguous_sparse_nonfinite_or_unlocated_source_metadata() {
        let base = sources();
        for replacement in [
            "configSettings[1].probe = {[1]='a',[3]='c'}",
            "configSettings[1].probe = { [1]='a', name='b' }",
            "configSettings[1].probe = 0/0",
            "configSettings[1].probe = setmetatable({}, {})",
            "configSettings[2].var = 'unlocated-key'",
            "configSettings[2].list[1].val = {}",
            "configSettings[2].defaultIndex = 1.5",
            "configSettings[2].probe = math.abs",
        ] {
            let mut changed = base.clone();
            let text = changed.get_mut(CONFIG).unwrap();
            *text = text.replace(
                "return configSettings",
                &format!("{replacement}\nreturn configSettings"),
            );
            assert!(extract(&changed).is_err(), "accepted {replacement}");
        }
    }
}
