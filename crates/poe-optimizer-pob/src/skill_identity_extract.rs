//! Authenticated identity-only construction of complete original skill/gem catalogs.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_data::skill_identities::*;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
type Result<T> = std::result::Result<T, GameDataExtractionError>;
const DATA: &str = "src/Modules/Data.lua";
const GEMS: &str = "src/Data/Gems.lua";
const SKILLS_TAB: &str = "src/Classes/SkillsTab.lua";
const COMMON: &str = "src/Modules/Common.lua";
fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("missing skill catalog source {path}")))
}
fn span(
    sources: &BTreeMap<String, String>,
    path: &str,
    begin: usize,
    end: usize,
) -> Result<IdentitySourceSpan> {
    let text = source(sources, path)?;
    if begin >= end || end > text.len() {
        return Err(error("invalid identity source byte span"));
    }
    let line = text[..begin].bytes().filter(|b| *b == b'\n').count() + 1;
    let end_line = line
        + text[begin..end]
            .trim_end()
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
    Ok(IdentitySourceSpan {
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
fn chunk_span(
    sources: &BTreeMap<String, String>,
    path: &str,
    begin: &str,
    end: &str,
) -> Result<IdentitySourceSpan> {
    let text = source(sources, path)?;
    let chunk = section(text, begin, end)?;
    let at = chunk.as_ptr() as usize - text.as_ptr() as usize;
    span(sources, path, at, at + chunk.len())
}
fn dense(table: &Table) -> Result<usize> {
    let n = table.raw_len();
    if n > 256 {
        return Err(error("skill identity array exceeds256"));
    }
    let mut count = 0;
    for row in table.pairs::<Value, Value>() {
        let (k, _) = row?;
        let index = match k {
            Value::Integer(i) => i,
            Value::Number(v) if v.is_finite() && v.fract() == 0.0 => v as i64,
            _ => return Err(error("skill identity array key is not integer")),
        };
        if index < 1 || index as usize > n {
            return Err(error("sparse skill identity array"));
        }
        count += 1;
    }
    if count != n {
        return Err(error("sparse skill identity array"));
    }
    Ok(n)
}
fn indexed(table: &Table, prefix: &str) -> Result<Vec<IndexedIdentityReference>> {
    let mut out = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (Value::String(key), value) = pair? else {
            continue;
        };
        let key = key.to_str()?;
        if let Some(index) = key.strip_prefix(prefix) {
            let index = index.parse::<u32>().map_err(error)?;
            let Value::String(id) = value else {
                return Err(error("non-string indexed identity reference"));
            };
            out.push(IndexedIdentityReference {
                index,
                id: id.to_str()?.to_owned(),
            });
        }
    }
    out.sort_by_key(|r| r.index);
    Ok(out)
}
fn display(table: &Table) -> Result<Option<Vec<i64>>> {
    let Some(order) = table.get::<Option<Table>>("grantedEffectDisplayOrder")? else {
        return Ok(None);
    };
    let n = dense(&order)?;
    let mut out = Vec::new();
    for i in 1..=n {
        let v = order.raw_get::<f64>(i)?;
        if !v.is_finite() || v.fract() != 0.0 || !(-1_000_000.0..=1_000_000.0).contains(&v) {
            return Err(error("noninteger/unbounded display order"));
        }
        out.push(v as i64);
    }
    Ok(Some(out))
}
fn optional_text(t: &Table, key: &str) -> Result<Option<String>> {
    match t.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::String(value) => Ok(Some(value.to_str()?.to_owned())),
        _ => Err(error(format!(
            "identity field {key} is not a string or nil"
        ))),
    }
}
fn required_text(t: &Table, key: &str) -> Result<String> {
    optional_text(t, key)?.ok_or_else(|| error(format!("missing identity field {key}")))
}
fn optional_bool(t: &Table, key: &str) -> Result<Option<bool>> {
    match t.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Boolean(value) => Ok(Some(value)),
        _ => Err(error(format!(
            "identity field {key} is not a boolean or nil"
        ))),
    }
}
fn declared_gem(t: &Table) -> Result<DeclaredGemIdentity> {
    Ok(DeclaredGemIdentity {
        game_id: required_text(t, "gameId")?,
        variant_id: required_text(t, "variantId")?,
        name: required_text(t, "name")?,
        name_spec: optional_text(t, "nameSpec")?,
        base_type_name: optional_text(t, "baseTypeName")?,
        primary_effect_id: required_text(t, "grantedEffectId")?,
        additional_effects: indexed(t, "additionalGrantedEffectId")?,
        additional_stat_sets: indexed(t, "additionalStatSet")?,
        display_order: display(t)?,
    })
}
fn declared_skill(t: &Table) -> Result<DeclaredSkillIdentity> {
    Ok(DeclaredSkillIdentity {
        name: required_text(t, "name")?,
        base_type_name: optional_text(t, "baseTypeName")?,
        support: optional_bool(t, "support")?,
        from_tree: optional_bool(t, "fromTree")?,
    })
}
fn effects(t: &Table, key: &str) -> Result<Vec<String>> {
    let list: Table = t.get(key)?;
    let n = dense(&list)?;
    (1..=n)
        .map(|i| {
            let effect: Table = list.raw_get(i)?;
            required_text(&effect, "id")
        })
        .collect()
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

struct Declaration<'a> {
    key: String,
    table: &'a str,
    source: IdentitySourceSpan,
}
fn declarations<'a>(
    lua: &Lua,
    sources: &'a BTreeMap<String, String>,
    path: &str,
    skills: bool,
) -> Result<Vec<Declaration<'a>>> {
    let text = source(sources, path)?;
    let code = tokens(text)?;
    let mut out = Vec::new();
    let mut i = 0;
    let mut depth = 0usize;
    let mut outside = Vec::new();
    while i < code.len() {
        let start = i;
        let at = if skills && code[i].text == "skills" {
            i + 1
        } else {
            i
        };
        let match_start = (!skills || at != i)
            && code.get(at).is_some_and(|t| t.text == "[")
            && code.get(at + 1).is_some_and(|t| t.quoted)
            && code.get(at + 2).is_some_and(|t| t.text == "]")
            && code.get(at + 3).is_some_and(|t| t.text == "=")
            && code.get(at + 4).is_some_and(|t| t.text == "{")
            && (skills || depth == 1);
        if match_start {
            let key: String = lua.load(format!("return {}", code[at + 1].text)).eval()?;
            let open = at + 4;
            let mut end = open + 1;
            let mut balance = 1;
            while end < code.len() && balance > 0 {
                if !code[end].quoted {
                    if code[end].text == "{" {
                        balance += 1;
                    } else if code[end].text == "}" {
                        balance -= 1;
                    }
                }
                end += 1;
            }
            if balance != 0 {
                return Err(error("unbalanced source declaration"));
            }
            let byte_start = code[start].text.as_ptr() as usize - text.as_ptr() as usize;
            let table_start = code[open].text.as_ptr() as usize - text.as_ptr() as usize;
            let byte_end = code[end - 1].text.as_ptr() as usize - text.as_ptr() as usize
                + code[end - 1].text.len();
            let source_span = span(sources, path, byte_start, byte_end)?;
            if source_span.line as usize != code[start].line {
                return Err(error("source declaration line mismatch"));
            }
            out.push(Declaration {
                key,
                table: &text[table_start..byte_end],
                source: source_span,
            });
            if out.len() > 50_000 {
                return Err(error("too many source declarations"));
            }
            i = end;
            continue;
        }
        outside.push(code[i].text);
        if !code[i].quoted {
            if code[i].text == "{" {
                depth += 1;
            } else if code[i].text == "}" {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| error("unbalanced source braces"))?;
            }
        }
        i += 1;
    }
    if depth != 0 {
        return Err(error("unbalanced source table"));
    }
    if skills
        && outside
            != [
                "return", "function", "(", "skills", ",", "mod", ",", "flag", ",", "skill", ")",
                "end",
            ]
    {
        return Err(error(
            "skill declarations are not an unconditional ordered assignment sequence",
        ));
    }
    if !skills
        && (outside.first() != Some(&"return")
            || outside.get(1) != Some(&"{")
            || outside.last() != Some(&"}")
            || outside[2..outside.len() - 1]
                .iter()
                .any(|token| *token != ","))
    {
        return Err(error(
            "gem declarations are not a literal top-level constructor",
        ));
    }
    Ok(out)
}
pub(crate) fn extract(sources: &BTreeMap<String, String>) -> Result<SkillIdentityData> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT | StdLib::JIT,
        LuaOptions::default(),
    )?;
    lua.set_memory_limit(256 * 1024 * 1024)?;
    lua.load("jit.off();jit.flush();jit=nil;load=nil;loadstring=nil;loadfile=nil;dofile=nil;collectgarbage=nil;print=nil;package=nil;io=nil;os=nil;debug=nil;ffi=nil;require=nil;data={}").exec()?;
    let ticks = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(100_000),
        move |_, _| {
            if ticks.fetch_add(1, Ordering::Relaxed) >= 10_000 {
                return Err(mlua::Error::RuntimeError(
                    "skill identity instruction bound".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    let common = source(sources, COMMON)?;
    for (begin, end) in [
        (
            "function sanitiseText(text)",
            "-- Convert int to 4 bytes string",
        ),
        (
            "function copyTable(tbl, noRecurse)",
            "do\n\tlocal subTableMap",
        ),
        (
            "function tableConcat(t1,t2)",
            "--- Simple table value equality",
        ),
    ] {
        lua.load(section(common, begin, end)?).exec()?;
    }
    let modules = Arc::new(Mutex::new(Vec::<String>::new()));
    let loaded = modules.clone();
    let authenticated = Arc::new(sources.clone());
    let module_source = authenticated.clone();
    lua.globals().set(
        "LoadModule",
        lua.create_function(move |lua, name: String| {
            let path = format!("src/{name}.lua");
            let text = module_source.get(&path).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("unapproved skill identity dependency {path}"))
            })?;
            loaded
                .lock()
                .map_err(|_| mlua::Error::RuntimeError("module order poisoned".into()))?
                .push(path.clone());
            lua.load(text).set_name(format!("@{path}")).eval::<Value>()
        })?,
    )?;
    let original = source(sources, DATA)?;
    let prefix = section(
        original,
        "LoadModule(\"Data/Global\")",
        "-----------------\n-- Common Data",
    )?;
    let construction = section(original, "-- Load skills\n", "-- Load minions\n")?;
    let code = format!(
        "{prefix}\n{construction}\nreturn{{data=data,mod=makeSkillMod,flag=makeFlagMod,skill=makeSkillDataMod}}"
    );
    let result: Table = lua.load(code).set_name(format!("@{DATA}")).eval()?;
    let data: Table = result.get("data")?;
    let mod_fn: Function = result.get("mod")?;
    let flag_fn: Function = result.get("flag")?;
    let skill_fn: Function = result.get("skill")?;
    let module_order = modules
        .lock()
        .map_err(|_| error("module order poisoned"))?
        .clone();
    let skill_module_order: Vec<_> = module_order
        .iter()
        .filter(|p| {
            p.starts_with("src/Data/Skills/") && p.as_str() != "src/Data/Skills/SkillAssets.lua"
        })
        .cloned()
        .collect();
    let mut skill_declarations = Vec::new();
    let mut skill_winners = BTreeMap::new();
    for path in &skill_module_order {
        for declaration in declarations(&lua, sources, path, true)? {
            let function: Function = lua
                .load(format!(
                    "return function(mod,flag,skill) return {} end",
                    declaration.table
                ))
                .eval()?;
            let table: Table =
                function.call((mod_fn.clone(), flag_fn.clone(), skill_fn.clone()))?;
            let index = skill_declarations.len() as u32 + 1;
            skill_winners.insert(declaration.key.clone(), index);
            skill_declarations.push(SkillIdentityDeclaration {
                index,
                id: declaration.key,
                source: declaration.source,
                identity: declared_skill(&table)?,
            });
        }
        lua.gc_collect()?;
    }
    let mut gem_declarations = Vec::new();
    for declaration in declarations(&lua, sources, GEMS, false)? {
        let table: Table = lua.load(format!("return {}", declaration.table)).eval()?;
        let index = gem_declarations.len() as u32 + 1;
        gem_declarations.push(GemIdentityDeclaration {
            index,
            key: declaration.key,
            source: declaration.source,
            identity: declared_gem(&table)?,
        });
    }
    let mut skills = Vec::new();
    let skill_table: Table = data.get("skills")?;
    for row in skill_table.pairs::<String, Table>() {
        let (id, table) = row?;
        let winner = *skill_winners
            .get(&id)
            .ok_or_else(|| error(format!("constructed skill lacks declaration {id}")))?;
        let expected = &skill_declarations[winner as usize - 1].identity;
        let raw = declared_skill(&table)?;
        if raw.base_type_name != expected.base_type_name
            || raw.support != expected.support
            || raw.from_tree != expected.from_tree
        {
            return Err(error(format!(
                "constructed skill declaration disagreement {id}"
            )));
        }
        let actual_id = required_text(&table, "id")?;
        if actual_id != id {
            return Err(error("constructed skill ID mismatch"));
        }
        skills.push(SkillIdentity {
            id,
            name: raw.name,
            base_type_name: raw.base_type_name,
            support: raw.support,
            from_tree: raw.from_tree,
            winning_declaration: winner,
        });
    }
    skills.sort_by(|a, b| a.id.cmp(&b.id));
    let mut gems = Vec::new();
    let gem_table: Table = data.get("gems")?;
    for row in gem_table.pairs::<String, Table>() {
        let (key, table) = row?;
        let raw = declared_gem(&table)?;
        let sanitise: Function = lua.globals().get("sanitiseText")?;
        let mut matches = Vec::new();
        for declaration in gem_declarations.iter().filter(|d| d.key == key) {
            let declared = &declaration.identity;
            let name: String = sanitise.call(declared.name.clone())?;
            if raw.game_id == declared.game_id
                && raw.variant_id == declared.variant_id
                && raw.primary_effect_id == declared.primary_effect_id
                && raw.name == name
                && raw.name_spec == declared.name_spec
                && raw.base_type_name == declared.base_type_name
                && raw.additional_stat_sets == declared.additional_stat_sets
                && raw.display_order == declared.display_order
                && declared
                    .additional_effects
                    .iter()
                    .all(|r| raw.additional_effects.contains(r))
            {
                matches.push(declaration.index);
            }
        }
        if matches.len() != 1 {
            return Err(error(format!(
                "constructed gem winner is not uniquely evidenced by declaration identity {key}"
            )));
        }
        let winner = matches[0];
        let declared = &gem_declarations[winner as usize - 1].identity;
        if raw.game_id != declared.game_id
            || raw.variant_id != declared.variant_id
            || raw.primary_effect_id != declared.primary_effect_id
            || raw.base_type_name != declared.base_type_name
            || raw.name_spec != declared.name_spec
            || raw.additional_stat_sets != declared.additional_stat_sets
            || raw.display_order != declared.display_order
        {
            return Err(error(format!(
                "constructed gem declaration disagreement {key}"
            )));
        }
        gems.push(GemIdentity {
            key,
            game_id: raw.game_id,
            variant_id: raw.variant_id,
            name: raw.name,
            name_spec: raw.name_spec,
            base_type_name: raw.base_type_name,
            primary_effect_id: raw.primary_effect_id,
            declared_additional_effects: declared.additional_effects.clone(),
            declared_additional_stat_sets: declared.additional_stat_sets.clone(),
            constructed_additional_effects: raw.additional_effects,
            additional_effects: effects(&table, "additionalGrantedEffects")?,
            effect_list: effects(&table, "grantedEffectList")?,
            display_order: raw.display_order,
            winning_declaration: winner,
        });
    }
    gems.sort_by(|a, b| a.key.cmp(&b.key));
    let mut files = BTreeMap::new();
    for path in module_order
        .iter()
        .map(String::as_str)
        .chain([COMMON, DATA, SKILLS_TAB])
    {
        files.insert(path.into(), hash(source(sources, path)?.as_bytes()));
    }
    let source = SkillIdentitySource {
        upstream_revision: crate::source::UPSTREAM_REVISION.into(),
        files,
        skill_module_order,
        skill_assembly: chunk_span(sources, DATA, "-- Load skills\n", "-- Load gems\n")?,
        gem_assembly: chunk_span(sources, DATA, "-- Load gems\n", "-- Load minions\n")?,
        load_skill: chunk_span(
            sources,
            SKILLS_TAB,
            "function SkillsTabClass:LoadSkill(node, skillSetId)",
            "function SkillsTabClass:Load(xml, fileName)",
        )?,
    };
    let mut out = SkillIdentityData {
        schema_version: SKILL_IDENTITY_SCHEMA_VERSION,
        capability: SkillIdentityCapability::IdentityOnly,
        source,
        gem_declarations,
        skill_declarations,
        gems,
        skills,
        missing_references: Vec::new(),
    };
    out.missing_references = out.expected_missing_references();
    out.validate().map_err(error)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_original_identity_catalog_constructs_without_numeric_stubs() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let mut paths = vec![
            COMMON.to_owned(),
            DATA.into(),
            SKILLS_TAB.into(),
            GEMS.into(),
            "src/Data/Global.lua".into(),
            "src/Data/SkillStatMap.lua".into(),
            "src/Data/Assets.lua".into(),
            "src/Data/Skills/SkillAssets.lua".into(),
        ];
        for name in [
            "act_str", "act_dex", "act_int", "other", "minion", "spectre", "sup_str", "sup_dex",
            "sup_int",
        ] {
            paths.push(format!("src/Data/Skills/{name}.lua"));
        }
        let sources = paths
            .into_iter()
            .map(|p| Ok((p.clone(), crate::source::read_verified_text(&root, &p)?)))
            .collect::<Result<BTreeMap<_, _>>>()
            .unwrap();
        let result = extract(&sources).unwrap();
        eprintln!(
            "identity catalog: {} gem declarations/{} final gems, {} skill declarations/{} skills, {} missing references",
            result.gem_declarations.len(),
            result.gems.len(),
            result.skill_declarations.len(),
            result.skills.len(),
            result.missing_references.len()
        );
        assert!(result.gems.len() > 500);
        assert!(result.skills.len() > 1000);
        assert!(result.gem_declarations.len() > result.gems.len());
        for value in [
            &result.source.skill_assembly,
            &result.source.gem_assembly,
            &result.source.load_skill,
        ]
        .into_iter()
        .chain(result.gem_declarations.iter().map(|d| &d.source))
        .chain(result.skill_declarations.iter().map(|d| &d.source))
        {
            let bytes: String = sources[&value.path]
                .split_inclusive('\n')
                .skip(value.line as usize - 1)
                .take((value.end_line - value.line + 1) as usize)
                .collect();
            assert_eq!(
                hash(bytes.as_bytes()),
                value.sha256,
                "{}:{}",
                value.path,
                value.line
            );
        }
    }
    #[test]
    fn declaration_scanner_preserves_occurrences_and_ignores_literals_and_comments() {
        let lua = Lua::new();
        let text = r#"-- skills["fake"] = {}
return function(skills, mod, flag, skill)
skills["same"] = {name="first", nested={text=[=[skills["fake"] = {}]=]}}
--[==[ skills["fake"] = {} ]==]
skills["same"] = {name="second", text="} [\"noise\"] = {"}
end
"#;
        let sources = BTreeMap::from([("skills.lua".into(), text.into())]);
        let found = declarations(&lua, &sources, "skills.lua", true).unwrap();
        assert_eq!(
            found.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(),
            ["same", "same"]
        );
        assert_eq!(found[0].source.line, 3);
        assert_eq!(found[1].source.line, 5);
        let table: Table = lua
            .load(format!("return {}", found[1].table))
            .eval()
            .unwrap();
        assert_eq!(table.get::<String>("name").unwrap(), "second");
    }
    #[test]
    fn conditional_skill_assignments_and_dynamic_gem_construction_require_review() {
        let lua = Lua::new();
        for text in [
            "return function(skills,mod,flag,skill) if true then skills[\"x\"]={name=\"X\"} end end",
            "return function(skills,mod,flag,skill) skills[\"x\"]={name=\"X\"}; end",
        ] {
            let sources = BTreeMap::from([("skills.lua".into(), text.into())]);
            assert!(declarations(&lua, &sources, "skills.lua", true).is_err());
        }
        for text in [
            "local x = {}; return x",
            "return {generated(), [\"x\"]={name=\"X\"}}",
        ] {
            let sources = BTreeMap::from([("gems.lua".into(), text.into())]);
            assert!(declarations(&lua, &sources, "gems.lua", false).is_err());
        }
        for text in [
            "return {[\"x\"]={name=\"first\"},[\"x\"]={name=\"second\"}}",
            "return {}",
        ] {
            let sources = BTreeMap::from([("gems.lua".into(), text.into())]);
            assert!(declarations(&lua, &sources, "gems.lua", false).is_ok());
        }
    }
    #[test]
    fn span_hashes_complete_inclusive_lines_even_for_midline_declarations() {
        let sources = BTreeMap::from([(
            "test.lua".into(),
            "-- first\n  abc {\n  x=1,\n  }, tail\n\n".into(),
        )]);
        let text: &String = &sources["test.lua"];
        let begin = text.find("abc").unwrap();
        let end = text.find("},").unwrap() + 1;
        let value = span(&sources, "test.lua", begin, end).unwrap();
        assert_eq!((value.line, value.end_line), (2, 4));
        assert_eq!(value.sha256, hash(b"  abc {\n  x=1,\n  }, tail\n"));
    }
    #[test]
    fn identity_metadata_never_coerces_source_numbers_or_booleans_into_strings() {
        let lua = Lua::new();
        for text in [
            "{name=1}",
            "{name=false}",
            "{name='ok', support=1}",
            "{name='ok',baseTypeName=0}",
        ] {
            let table: Table = lua.load(format!("return {text}")).eval().unwrap();
            assert!(declared_skill(&table).is_err());
        }
        let table: Table = lua.load("return {name='ok',support=false}").eval().unwrap();
        assert_eq!(declared_skill(&table).unwrap().support, Some(false));
    }
}
