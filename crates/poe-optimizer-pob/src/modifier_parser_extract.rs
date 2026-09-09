//! Authenticated full ModParser construction. Definitions do not execute native callbacks.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_data::item_loading::{ItemLoadingSource, ItemSourceSpan};
use poe_optimizer_data::modifier_parser::*;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
mod factories;
mod ordinary;

type Result<T> = std::result::Result<T, GameDataExtractionError>;
const PARSER: &str = "src/Modules/ModParser.lua";
const COMMON: &str = "src/Modules/Common.lua";
const TOOLS: &str = "src/Modules/ModTools.lua";
const HELPERS: &[&str] = &[
    "firstToUpper",
    "combineToUpper",
    "getSimpleConv",
    "flag",
    "grantedExtraSkill",
    "triggerExtraSkill",
    "extraSupport",
    "explodeFunc",
    "appendMod",
    "scan",
    "parseMod",
    "capitalizeWordsInString",
    "getPerStat",
    "getThreshold",
];
fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("missing modifier parser source {path}")))
}
fn line_span(
    sources: &BTreeMap<String, String>,
    path: &str,
    line: usize,
    end_line: usize,
) -> Result<ItemSourceSpan> {
    let text = source(sources, path)?;
    if line == 0 || end_line < line || end_line > text.lines().count() {
        return Err(error("invalid parser source span"));
    }
    Ok(ItemSourceSpan {
        path: path.into(),
        line: line.try_into().map_err(error)?,
        end_line: end_line.try_into().map_err(error)?,
        sha256: hash(
            text.split_inclusive('\n')
                .skip(line - 1)
                .take(end_line - line + 1)
                .collect::<String>()
                .as_bytes(),
        ),
    })
}
fn part_span(sources: &BTreeMap<String, String>, path: &str, part: &str) -> Result<ItemSourceSpan> {
    let text = source(sources, path)?;
    let at = part.as_ptr() as usize - text.as_ptr() as usize;
    if at > text.len() {
        return Err(error("invalid parser source part"));
    }
    let line = text[..at].bytes().filter(|b| *b == b'\n').count() + 1;
    line_span(
        sources,
        path,
        line,
        line + part.trim_end().bytes().filter(|b| *b == b'\n').count(),
    )
}
fn top_function<'a>(
    sources: &'a BTreeMap<String, String>,
    path: &str,
    name: &str,
) -> Result<&'a str> {
    section(
        source(sources, path)?,
        &format!("function {name}("),
        "\nend",
    )
    .and_then(|part| {
        let text = source(sources, path)?;
        let at = part.as_ptr() as usize - text.as_ptr() as usize;
        Ok(&text[at..at + part.len() + 4])
    })
}
fn ordered(t: &Table) -> Result<(BTreeMap<String, Value>, BTreeMap<i64, Value>)> {
    if t.metatable().is_some() {
        return Err(error(
            "parser graph metatable requires explicit schema support",
        ));
    }
    let (mut fields, mut indexed) = (BTreeMap::new(), BTreeMap::new());
    for entry in t.clone().pairs::<Value, Value>() {
        let (k, v) = entry?;
        match k {
            Value::String(k) => {
                fields.insert(k.to_str()?.to_owned(), v);
            }
            Value::Integer(k) => {
                indexed.insert(k, v);
            }
            Value::Number(k)
                if k.is_finite() && k.fract() == 0.0 && k.abs() <= 9_007_199_254_740_991.0 =>
            {
                indexed.insert(k as i64, v);
            }
            _ => return Err(error("unsupported parser table key")),
        }
        if fields.len() + indexed.len() > 50_000 {
            return Err(error("parser table row bound"));
        }
    }
    Ok((fields, indexed))
}
struct Graph<'a> {
    global_environment: usize,
    sources: &'a BTreeMap<String, String>,
    get_upvalue: Function,
    builtins: BTreeMap<usize, String>,
    seen_tables: BTreeMap<usize, ParserTableId>,
    seen_callbacks: BTreeMap<usize, ParserCallbackId>,
    tables: Vec<ParserTable>,
    callbacks: Vec<ParserCallback>,
    values: usize,
}
impl Graph<'_> {
    fn value(&mut self, v: Value, depth: usize) -> Result<ParserValue> {
        self.values += 1;
        if self.values > 1_000_000 || depth > 64 {
            return Err(error("parser graph resource bound"));
        }
        Ok(match v {
            Value::Nil => ParserValue::Nil,
            Value::Boolean(v) => ParserValue::Boolean(v),
            Value::Integer(v) => ParserValue::Number(v as f64),
            Value::Number(v) if v.is_nan() => ParserValue::NonFinite(ParserNonFinite::Nan),
            Value::Number(v) if v == f64::INFINITY => {
                ParserValue::NonFinite(ParserNonFinite::PositiveInfinity)
            }
            Value::Number(v) if v == f64::NEG_INFINITY => {
                ParserValue::NonFinite(ParserNonFinite::NegativeInfinity)
            }
            Value::Number(v) => ParserValue::Number(v),
            Value::String(v) => ParserValue::Text(v.to_str()?.to_owned()),
            Value::Table(t) => {
                let pointer = t.to_pointer() as usize;
                if let Some(id) = self.seen_tables.get(&pointer) {
                    return Ok(ParserValue::Table(*id));
                }
                if self.tables.len() >= 100_000 {
                    return Err(error("parser table count bound"));
                }
                let id = ParserTableId(self.tables.len() as u32 + 1);
                self.seen_tables.insert(pointer, id);
                self.tables.push(ParserTable::default());
                let (fields, indexed) = ordered(&t)?;
                let mut out = ParserTable::default();
                // Numeric keys precede string keys, matching the canonical audit traversal.
                for (k, v) in indexed {
                    out.indexed.insert(k, self.value(v, depth + 1)?);
                }
                for (k, v) in fields {
                    out.fields.insert(k, self.value(v, depth + 1)?);
                }
                self.tables[id.0 as usize - 1] = out;
                ParserValue::Table(id)
            }
            Value::Function(f) => {
                let pointer = f.to_pointer() as usize;
                if let Some(id) = self.seen_callbacks.get(&pointer) {
                    return Ok(ParserValue::Callback(*id));
                }
                if self.callbacks.len() >= 20_000 {
                    return Err(error("parser callback count bound"));
                }
                let id = ParserCallbackId(self.callbacks.len() as u32 + 1);
                self.seen_callbacks.insert(pointer, id);
                let info = f.info();
                if info.what != "C"
                    && f.environment()
                        .is_none_or(|env| env.to_pointer() as usize != self.global_environment)
                {
                    return Err(error("parser closure has a non-global environment"));
                }
                let kind = if info.what == "C" {
                    ParserCallbackKind::Builtin {
                        symbol: self
                            .builtins
                            .get(&pointer)
                            .ok_or_else(|| error("unresolved parser builtin"))?
                            .clone(),
                    }
                } else {
                    let path = info
                        .source
                        .as_deref()
                        .and_then(|p| p.strip_prefix('@'))
                        .ok_or_else(|| error("parser callback source is not authenticated"))?;
                    ParserCallbackKind::Lua {
                        source: line_span(
                            self.sources,
                            path,
                            info.line_defined
                                .ok_or_else(|| error("missing callback line"))?,
                            info.last_line_defined
                                .ok_or_else(|| error("missing callback end line"))?,
                        )?,
                    }
                };
                self.callbacks.push(ParserCallback {
                    kind: kind.clone(),
                    upvalues: vec![],
                    environment: ParserEnvironment::OriginalGlobals,
                });
                let mut upvalues = vec![];
                if matches!(kind, ParserCallbackKind::Lua { .. }) {
                    for i in 1..=129 {
                        let (name, value): (Option<String>, Value) =
                            self.get_upvalue.call((f.clone(), i))?;
                        let Some(name) = name else {
                            break;
                        };
                        if i > 128 {
                            return Err(error("parser callback upvalue bound"));
                        }
                        upvalues.push(ParserUpvalue {
                            name,
                            value: self.value(value, depth + 1)?,
                        });
                    }
                }
                self.callbacks[id.0 as usize - 1].upvalues = upvalues;
                ParserValue::Callback(id)
            }
            _ => return Err(error("unsupported parser graph value")),
        })
    }
}
fn number(text: &str, begin: &str, end: &str) -> Result<f64> {
    let part = section(text, begin, end)?;
    let v = part[begin.len()..].trim().parse::<f64>().map_err(error)?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(error("nonfinite parser policy"))
    }
}
fn word_limit(text: &str, begin: &str, end: &str) -> Result<u32> {
    let value = number(text, begin, end)?;
    if !(0.0..=128.0).contains(&value) || value.fract() != 0.0 {
        return Err(error("invalid integral parser word limit"));
    }
    Ok(value as u32)
}
fn pair(lua: &Lua, source: &str) -> Result<[f64; 2]> {
    let table: Table = lua.load(format!("return {source}")).eval()?;
    let (fields, indexed) = ordered(&table)?;
    if !fields.is_empty()
        || indexed.len() != 2
        || !indexed.contains_key(&1)
        || !indexed.contains_key(&2)
    {
        return Err(error("parser policy requires a dense numeric pair"));
    }
    Ok([table.raw_get(1)?, table.raw_get(2)?])
}
fn between<'a>(text: &'a str, begin: &str, end: &str) -> Result<&'a str> {
    Ok(&section(text, begin, end)?[begin.len()..])
}
pub(crate) fn extract(sources: &BTreeMap<String, String>) -> Result<ModifierParserData> {
    // SAFETY: only the trusted host retains getupvalue. DEBUG and all loader/I/O
    // APIs are removed before any authenticated source executes. The worker also
    // applies its startup-inclusive deadline; this VM has memory/instruction caps.
    let lua = unsafe {
        Lua::unsafe_new_with(
            StdLib::TABLE
                | StdLib::STRING
                | StdLib::MATH
                | StdLib::BIT
                | StdLib::JIT
                | StdLib::DEBUG,
            LuaOptions::default(),
        )
    };
    let original_type: Function = lua.globals().get("type")?;
    let original_select: Function = lua.globals().get("select")?;
    lua.set_memory_limit(256 * 1024 * 1024)?;
    let get_upvalue: Function = lua.globals().get::<Table>("debug")?.get("getupvalue")?;
    lua.load("jit.off();jit.flush();jit=nil;debug=nil;io=nil;os=nil;ffi=nil;package=nil;require=nil;load=nil;loadstring=nil;loadfile=nil;dofile=nil;print=nil;collectgarbage=nil").exec()?;
    let ticks = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(100_000),
        move |_, _| {
            if ticks.fetch_add(1, Ordering::Relaxed) > 15_000 {
                Err(mlua::Error::RuntimeError(
                    "parser construction instruction bound".into(),
                ))
            } else {
                Ok(VmState::Continue)
            }
        },
    )?;
    let module_order = Arc::new(Mutex::new(Vec::<String>::new()));
    let observed = module_order.clone();
    let authenticated = Arc::new(sources.clone());
    lua.globals().set(
        "LoadModule",
        lua.create_function(move |lua, (name, args): (String, mlua::Variadic<Value>)| {
            let path = format!("src/{}.lua", name.strip_suffix(".lua").unwrap_or(&name));
            let text = authenticated.get(&path).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("unprovided authenticated module {path}"))
            })?;
            observed
                .lock()
                .map_err(|_| mlua::Error::RuntimeError("module order lock".into()))?
                .push(path.clone());
            lua.load(text)
                .set_name(format!("@{path}"))
                .call::<Value>(args)
        })?,
    )?;
    let mut spans = BTreeMap::new();
    for name in [
        "sanitiseText",
        "copyTable",
        "isValueInArray",
        "tableConcat",
        "triangular",
    ] {
        let part = top_function(sources, COMMON, name)?;
        let sp = part_span(sources, COMMON, part)?;
        lua.load(format!("{}{part}", "\n".repeat(sp.line as usize - 1)))
            .set_name(format!("@{COMMON}"))
            .exec()?;
        spans.insert(name.into(), sp);
    }
    lua.load("LoadModule('GameVersions');LoadModule('Modules/Data')")
        .exec()?;
    lua.globals().set("modLib", lua.create_table()?)?;
    let part = top_function(sources, TOOLS, "modLib.createMod")?;
    let sp = part_span(sources, TOOLS, part)?;
    lua.load(format!("{}{part}", "\n".repeat(sp.line as usize - 1)))
        .set_name(format!("@{TOOLS}"))
        .exec()?;
    spans.insert("create_mod".into(), sp);
    let parser = source(sources, PARSER)?;
    let body = section(
        parser,
        "-- Path of Building",
        "return function(line, isComb)",
    )?;
    let exports = ParserDictionary::ALL
        .iter()
        .map(|d| d.source_name())
        .chain(HELPERS.iter().copied())
        .chain(["gems"]);
    let roots: Table = lua
        .load(format!(
            "{body}\nreturn {{{}}}",
            exports
                .map(|v| format!("{v}={v}"))
                .collect::<Vec<_>>()
                .join(",")
        ))
        .set_name(format!("@{PARSER}"))
        .eval()?;
    spans.insert(
        "constructed_parser".into(),
        part_span(sources, PARSER, body)?,
    );
    roots.set("ModFlag", lua.globals().get::<Table>("ModFlag")?)?;
    roots.set("KeywordFlag", lua.globals().get::<Table>("KeywordFlag")?)?;
    roots.set("SkillType", lua.globals().get::<Table>("SkillType")?)?;
    let data: Table = lua.globals().get("data")?;
    roots.set("keystones", data.get::<Table>("keystones")?)?;
    let (
        gem_names,
        gem_for_base_name_ambiguities,
        gem_base_name_assignment_candidates,
        gem_name_span,
    ) = gem_name_assignments(&lua, sources, &data)?;
    roots.set("gemForBaseName", gem_names)?;
    spans.insert("gem_base_name_assignments".into(), gem_name_span);
    // Capture precisely the complete inputs read by generated-name and grant loops.
    let inputs: Table = lua
        .load(include_str!("modifier_parser_inputs.lua"))
        .eval()?;
    roots.set("gem_generator_inputs", inputs.get::<Table>("gems")?)?;
    roots.set("skill_grant_inputs", inputs.get::<Table>("skills")?)?;
    let grants = section(
        parser,
        r#"elseif modForm == "GRANTS" then"#,
        r#"elseif modForm == "GRANTS_GLOBAL" then"#,
    )?;
    let local_hand = between(grants, "modExtraTags = ", "\n\t\tmodSuffix")?;
    let local_hand: Table = lua.load(format!("return {local_hand}")).eval()?;
    roots.set("local_hand_tag", local_hand)?;
    let blacklist_part = section(parser, "local effectBlacklist = {", "\n\t\t}")?;
    let blacklist: Table = lua
        .load(format!("{blacklist_part}\n}} return effectBlacklist"))
        .eval()?;
    let mut builtins = BTreeMap::new();
    let (fields, _) = ordered(&lua.globals())?;
    for (name, v) in fields {
        match v {
            Value::Function(f) => {
                builtins.insert(f.to_pointer() as usize, name);
            }
            Value::Table(t) if name != "_G" && name != "data" => {
                let (fields, _) = ordered(&t)?;
                for (k, v) in fields {
                    if let Value::Function(f) = v {
                        builtins.insert(f.to_pointer() as usize, format!("{name}.{k}"));
                    }
                }
            }
            _ => {}
        }
    }
    let mut graph = Graph {
        global_environment: lua.globals().to_pointer() as usize,
        sources,
        get_upvalue,
        builtins,
        seen_tables: BTreeMap::new(),
        seen_callbacks: BTreeMap::new(),
        tables: vec![],
        callbacks: vec![],
        values: 0,
    };
    let (root_fields, _) = ordered(&roots)?;
    let mut projected = BTreeMap::new();
    for (name, v) in root_fields {
        projected.insert(name, graph.value(v, 0)?);
    }
    let tid = |name: &str| -> Result<ParserTableId> {
        match projected.get(name) {
            Some(ParserValue::Table(id)) => Ok(*id),
            _ => Err(error(format!("missing parser table root {name}"))),
        }
    };
    let mut dictionaries = BTreeMap::new();
    for name in ParserDictionary::ALL {
        dictionaries.insert(*name, tid(name.source_name())?);
    }
    let mut helpers = BTreeMap::new();
    for name in HELPERS {
        match projected.get(*name) {
            Some(ParserValue::Callback(id)) => {
                helpers.insert((*name).into(), *id);
            }
            _ => return Err(error("missing parser helper")),
        }
    }
    let mut declarations = declarations(&lua, sources)?;
    // Final constructed values are evidence of winners, not a invented pairs priority.
    let full = part_span(sources, PARSER, body)?;
    for (d, id) in &dictionaries {
        for (key, value) in &graph.tables[id.0 as usize - 1].fields {
            declarations.push(ParserDeclaration {
                dictionary: *d,
                key: key.clone(),
                source: full.clone(),
                source_order: declarations.len() as u32 + 1,
                phase: "constructed_final".into(),
                payload: Some(value.clone()),
            });
        }
    }
    let sorted_grant_gem_keys = roots
        .get::<Table>("gems")?
        .sequence_values::<String>()
        .collect::<mlua::Result<Vec<_>>>()?;
    let mut files = BTreeMap::new();
    let order = module_order
        .lock()
        .map_err(|_| error("module order lock"))?
        .clone();
    for path in order
        .iter()
        .map(String::as_str)
        .chain([COMMON, TOOLS, PARSER])
    {
        files.insert(path.into(), hash(source(sources, path)?.as_bytes()));
    }
    let thorns = section(
        parser,
        "elseif modForm == \"DMGTHORNSBASE\" then",
        "elseif modForm == \"DMGBOTH\" then",
    )?;
    let doubled = section(
        parser,
        "elseif modForm == \"DOUBLED\" then",
        "\n\tif not modName then",
    )?;
    let doubled_values = pair(&lua, between(doubled, "modValue = ", "\n")?)?;
    let thorns_values = pair(&lua, between(thorns, "modValue = ", "\n")?)?;
    if thorns_values[0] != thorns_values[1] {
        return Err(error("unequal base thorns pair needs extended policy"));
    }
    let tag_capture_numeric_pattern = ordinary::extract(&lua, sources, &mut spans)?;
    factories::constructor(sources)?;
    factories::verify_environment(
        &lua,
        &original_type,
        &original_select,
        &[
            ("ModFlag", tid("ModFlag")?),
            ("KeywordFlag", tid("KeywordFlag")?),
            ("SkillType", tid("SkillType")?),
        ],
        &graph.seen_tables,
    )?;
    let actual_constructor: Function = lua.globals().get::<Table>("modLib")?.get("createMod")?;
    let constructor = *graph
        .seen_callbacks
        .get(&(actual_constructor.to_pointer() as usize))
        .ok_or_else(|| error("original createMod absent from callback graph"))?;
    let mut out = ModifierParserData {
        schema_version: MODIFIER_PARSER_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: crate::source::UPSTREAM_REVISION.into(),
            files,
            construction_spans: spans,
            module_order: order,
        },
        dictionaries,
        dynamic_dependencies: ParserDependencies {
            gem_generator_inputs: tid("gem_generator_inputs")?,
            skill_grant_inputs: tid("skill_grant_inputs")?,
            gem_for_base_name: tid("gemForBaseName")?,
            gem_for_base_name_ambiguities,
            gem_base_name_assignment_candidates,
            keystones: tid("keystones")?,
            sorted_grant_gem_keys,
        },
        policy: ParserPolicy {
            mod_flags: tid("ModFlag")?,
            keyword_flags: tid("KeywordFlag")?,
            skill_types: tid("SkillType")?,
            local_hand_tag: tid("local_hand_tag")?,
            immune_effect_blacklist: ordered(&blacklist)?.0.into_keys().collect(),
            cluster_prefix_pattern: between(parser, "local addToCluster = line:match(\"", "\")")?
                .to_string(),
            tag_capture_numeric_pattern,
            immune_max_single_words: word_limit(parser, "(numWords > ", ") then")?,
            immune_combined_min_words_exclusive: word_limit(parser, "if numWords > ", " then")?,
            immune_max_part_words: word_limit(parser, "if preWordNum > ", " or")?,
            thorns_base_damage: thorns_values[0],
            doubled_more: doubled_values[0],
            doubled_override: doubled_values[1],
            doubled_global_limit: number(doubled, "globalLimit = ", ",")?,
        },
        tables: graph.tables,
        callbacks: graph.callbacks,
        factories: BTreeMap::new(),
        helpers,
        declarations,
        capability: ParserCapability::DefinitionsOnly,
    };
    out.factories = factories::lower(&lua, sources, &out, constructor)?;
    out.validate().map_err(error)?;
    Ok(out)
}
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

fn declarations(lua: &Lua, sources: &BTreeMap<String, String>) -> Result<Vec<ParserDeclaration>> {
    let text = source(sources, PARSER)?;
    let ts = tokens(text)?;
    let mut out = vec![];
    for i in 2..ts.len() {
        if ts[i].text != "{" || ts[i - 1].text != "=" {
            continue;
        }
        let Some(dictionary) = ParserDictionary::ALL
            .iter()
            .find(|d| d.source_name() == ts[i - 2].text)
            .copied()
        else {
            continue;
        };
        if i >= 3 && ts[i - 3].text != "local" && dictionary != ParserDictionary::Damage {
            continue;
        }
        let mut depth = 1usize;
        let mut blocks = Vec::<(&str, bool)>::new();
        let mut j = i + 1;
        while depth > 0 {
            let t = ts
                .get(j)
                .ok_or_else(|| error("unclosed parser dictionary"))?;
            if !t.quoted {
                match t.text {
                    "{" => depth += 1,
                    "}" => depth -= 1,
                    _ => {}
                }
                if depth == 1 {
                    match t.text {
                        "function" | "if" | "for" | "while" | "repeat" => {
                            blocks.push((t.text, matches!(t.text, "for" | "while")))
                        }
                        "do" => {
                            if let Some((_, pending)) = blocks.last_mut().filter(|(_, p)| *p) {
                                *pending = false;
                            } else {
                                blocks.push(("do", false));
                            }
                        }
                        "end" | "until" => {
                            blocks
                                .pop()
                                .ok_or_else(|| error("unbalanced parser declaration block"))?;
                        }
                        "[" if blocks.is_empty()
                            && ts.get(j + 1).is_some_and(|t| t.quoted)
                            && ts.get(j + 2).is_some_and(|t| t.text == "]")
                            && ts.get(j + 3).is_some_and(|t| t.text == "=") =>
                        {
                            let key: String =
                                lua.load(format!("return {}", ts[j + 1].text)).eval()?;
                            out.push(ParserDeclaration {
                                dictionary,
                                key,
                                source: line_span(sources, PARSER, t.line, t.line)?,
                                source_order: out.len() as u32 + 1,
                                phase: "literal_declaration".into(),
                                payload: None,
                            });
                        }
                        _ => {}
                    }
                }
            }
            j += 1;
        }
    }
    Ok(out)
}

type GemNameAssignments = (
    Table,
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, Vec<String>>,
    ItemSourceSpan,
);

fn gem_name_assignments(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    data: &Table,
) -> Result<GemNameAssignments> {
    let path = "src/Modules/Data.lua";
    let part = section(
        source(sources, path)?,
        "local baseName = gem.name",
        "\n\tgem.additionalGrantedEffects = {}",
    )?;
    let span = part_span(sources, path, part)?;
    // Original assignment statements are executed unchanged against a write-only
    // observation proxy. Original full Data still runs separately above. This
    // captures every candidate, including equal-name overwrites, without selecting
    // an arbitrary winner from Lua pairs order.
    let assignments: Function = lua
        .load(format!(
            "{}return function(data,gem,gemId) {part} end",
            "\n".repeat(span.line as usize - 1)
        ))
        .set_name(format!("@{path}"))
        .eval()?;
    let observed = Arc::new(Mutex::new(BTreeMap::<
        String,
        std::collections::BTreeSet<String>,
    >::new()));
    let record = observed.clone();
    let proxy = lua.create_table()?;
    let meta = lua.create_table()?;
    meta.set(
        "__newindex",
        lua.create_function(move |_, (_, key, value): (Table, String, String)| {
            record
                .lock()
                .map_err(|_| mlua::Error::RuntimeError("gem observation lock".into()))?
                .entry(key)
                .or_default()
                .insert(value);
            Ok(())
        })?,
    )?;
    proxy.set_metatable(Some(meta))?;
    let host = lua.create_table()?;
    host.set("gemForBaseName", proxy)?;
    let gems: Table = lua
        .load(source(sources, "src/Data/Gems.lua")?)
        .set_name("@src/Data/Gems.lua")
        .eval()?;
    let sanitize: Function = lua.globals().get("sanitiseText")?;
    let skills: Table = data.get("skills")?;
    let (fields, _) = ordered(&gems)?;
    for (id, v) in fields {
        let Value::Table(gem) = v else {
            return Err(error("raw gem must be table"));
        };
        let name: String = gem.get("name")?;
        gem.set("name", sanitize.call::<String>(name)?)?;
        let effect_id: String = gem.get("grantedEffectId")?;
        gem.set("grantedEffect", skills.get::<Table>(effect_id)?)?;
        assignments.call::<()>((host.clone(), gem, id))?;
    }
    let original: Table = data.get("gemForBaseName")?;
    let (original, _) = ordered(&original)?;
    let entries = observed.lock().map_err(|_| error("gem observation lock"))?;
    if original.len() != entries.len() {
        return Err(error(format!(
            "gem base-name assignment observation changed key inventory: actual={} observed={} missing={:?} extra={:?}",
            original.len(),
            entries.len(),
            original
                .keys()
                .filter(|k| !entries.contains_key(*k))
                .collect::<Vec<_>>(),
            entries
                .keys()
                .filter(|k| !original.contains_key(*k))
                .collect::<Vec<_>>()
        )));
    }
    let resolved = lua.create_table()?;
    let mut ambiguous = BTreeMap::new();
    for (key, values) in entries.iter() {
        let Some(Value::String(actual)) = original.get(key) else {
            return Err(error("missing original gem base-name winner"));
        };
        if !values.contains(&actual.to_str()?.to_owned()) {
            return Err(error(
                "original gem winner absent from observed alternatives",
            ));
        }
        if values.len() == 1 {
            resolved.set(key.clone(), values.first().unwrap().clone())?;
        } else {
            ambiguous.insert(key.clone(), values.iter().cloned().collect());
        }
    }
    Ok((
        resolved,
        ambiguous,
        entries
            .iter()
            .map(|(key, values)| (key.clone(), values.iter().cloned().collect()))
            .collect(),
        span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        crate::game_data::expected_source_files()
            .unwrap()
            .into_keys()
            .map(|p| {
                let text = crate::source::read_verified_text(&root, &p).unwrap();
                (p, text)
            })
            .collect()
    }
    #[test]
    fn complete_original_dictionary_extraction_matches_reviewed_graph() {
        let sources = sources();
        let actual = extract(&sources).unwrap();
        let bundled = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        assert_eq!(&actual, bundled.modifier_parser().data());
        assert_eq!(actual.dictionaries.len(), 28);
        assert_eq!(
            actual
                .dictionaries
                .values()
                .map(|id| actual.tables[id.0 as usize - 1].fields.len())
                .sum::<usize>(),
            10_027
        );
        assert_eq!(actual.callbacks.len(), 1651);
        assert_eq!(
            actual
                .declarations
                .iter()
                .filter(|d| d.phase == "literal_declaration")
                .count(),
            4390
        );
        assert_eq!(actual.dynamic_dependencies.sorted_grant_gem_keys.len(), 966);
        assert_eq!(
            actual
                .dynamic_dependencies
                .gem_for_base_name_ambiguities
                .values()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![2, 3, 2]
        );
        for callback in &actual.callbacks {
            if let ParserCallbackKind::Lua { source } = &callback.kind {
                assert_eq!(
                    *source,
                    line_span(
                        &sources,
                        &source.path,
                        source.line as usize,
                        source.end_line as usize
                    )
                    .unwrap()
                );
            }
        }
    }
    #[test]
    fn literal_declarations_preserve_duplicate_occurrences_and_ignore_callback_strings() {
        let lua = Lua::new();
        let text = r#"local modNameList = {
            -- ["ignored"] = 4,
            ["duplicate"] = {"First"},
            ["callback"] = function() local nested={ ["nested"] = true }; return "[\"fake\"] = 1" end,
            ["duplicate"] = {"Last"},
        }
"#;
        let sources = BTreeMap::from([(PARSER.into(), text.into())]);
        let rows = declarations(&lua, &sources).unwrap();
        assert_eq!(
            rows.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
            vec!["duplicate", "callback", "duplicate"]
        );
        assert_eq!(
            rows.iter().map(|r| r.source.line).collect::<Vec<_>>(),
            vec![3, 4, 5]
        );
        assert!(rows.iter().all(|r| r.payload.is_none()));
    }
}
