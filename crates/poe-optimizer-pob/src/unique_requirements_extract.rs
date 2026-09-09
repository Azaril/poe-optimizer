//! Complete original Item/Main unique construction, with observational bookkeeping.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{Function, HookTriggers, Lua, MultiValue, Table, Value, VmState};
use poe_optimizer_data::{
    bundled::BundledClassTree,
    item_loading::{ItemLoadingData, ItemLoadingSource, ItemSourceSpan},
    unique_requirements::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, GameDataExtractionError>;
const ITEM: &str = "src/Classes/Item.lua";
const MAIN: &str = "src/Modules/Main.lua";
const COMMON: &str = "src/Modules/Common.lua";
fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("unprovided unique construction source {path}")))
}
fn span(sources: &BTreeMap<String, String>, path: &str, part: &str) -> Result<ItemSourceSpan> {
    let whole = source(sources, path)?;
    let offset = part.as_ptr() as usize - whole.as_ptr() as usize;
    if offset > whole.len() {
        return Err(error("invalid unique source span"));
    }
    let line = whole[..offset].bytes().filter(|b| *b == b'\n').count() as u32 + 1;
    let end_line = line + part.trim_end().bytes().filter(|b| *b == b'\n').count() as u32;
    let bytes = whole
        .split_inclusive('\n')
        .skip(line as usize - 1)
        .take((end_line - line + 1) as usize)
        .collect::<String>();
    Ok(ItemSourceSpan {
        path: path.into(),
        line,
        end_line,
        sha256: hash(bytes.as_bytes()),
    })
}
fn eval_part(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    path: &str,
    part: &str,
    prefix: &str,
    suffix: &str,
) -> Result<()> {
    let at = span(sources, path, part)?;
    lua.load(format!(
        "{prefix}{}{}\n{suffix}",
        "\n".repeat(at.line as usize - 1),
        part
    ))
    .set_name(format!("@{path}"))
    .exec()?;
    Ok(())
}
fn requirement(value: Value) -> Result<Option<f64>> {
    match value {
        Value::Nil => Ok(None),
        Value::Integer(value) => Ok(Some(value as f64)),
        Value::Number(value) if value.is_finite() => Ok(Some(value)),
        _ => Err(error(
            "unique requirement is not an original finite number or nil",
        )),
    }
}
fn lookup_policy(item: &str) -> Result<UniqueLookupPolicy> {
    let body = section(
        item,
        "function ItemClass:GetUniqueDBItem()",
        "---@class ModLine",
    )?;
    if body.matches("main.uniqueDB.list[").count() != 2 || body.matches("self.name]").count() != 1 {
        return Err(error("changed unique lookup operation"));
    }
    let mut prefixes = Vec::new();
    for tail in body.split("self.baseName:match(\"").skip(1) {
        let pattern = tail
            .split_once("\")")
            .ok_or_else(|| error("unterminated unique prefix"))?
            .0;
        let prefix = pattern
            .strip_prefix('^')
            .and_then(|p| p.strip_suffix("(.+)"))
            .ok_or_else(|| error("unique prefix is not anchored nonempty suffix"))?;
        if prefix.is_empty() || prefix.bytes().any(|b| b"^$().%[]*+-?\\\"".contains(&b)) {
            return Err(error("unique prefix requires extended pattern semantics"));
        }
        prefixes.push(prefix.into());
    }
    let separator = body
        .split_once("self.title .. \"")
        .and_then(|(_, s)| s.split_once("\" .. originalBaseName"))
        .ok_or_else(|| error("changed unique title separator operation"))?
        .0;
    if prefixes.is_empty() || separator.contains('\\') {
        return Err(error("unsupported unique lookup policy"));
    }
    Ok(UniqueLookupPolicy {
        base_prefixes: prefixes,
        title_base_separator: separator.into(),
    })
}
fn load(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    order: &Mutex<Vec<String>>,
    path: &str,
    args: MultiValue,
) -> mlua::Result<MultiValue> {
    let code = source(sources, path).map_err(mlua::Error::external)?;
    let mut trace = order
        .lock()
        .map_err(|_| mlua::Error::RuntimeError("unique module lock".into()))?;
    if trace.len() >= 2048 {
        return Err(mlua::Error::RuntimeError(
            "unique module count bound".into(),
        ));
    }
    trace.push(path.into());
    drop(trace);
    lua.load(code).set_name(format!("@{path}")).call(args)
}
fn host(sources: &BTreeMap<String, String>) -> Result<(Lua, Arc<Mutex<Vec<String>>>)> {
    let lua = Lua::new();
    lua.set_memory_limit(768 * 1024 * 1024)?;
    let started = Instant::now();
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(100_000),
        move |_, _| {
            if started.elapsed() > Duration::from_secs(120) {
                Err(mlua::Error::RuntimeError(
                    "unique construction deadline".into(),
                ))
            } else {
                Ok(VmState::Continue)
            }
        },
    )?;
    poe_optimizer_lua_utf8::register(&lua)?;
    let utf8: Value = lua.load("return require('lua-utf8')").eval()?;
    let bit: Value = lua.globals().get("bit")?;
    lua.load("jit.off();jit.flush();jit=nil;package=nil;io=nil;os=nil;debug=nil;ffi=nil;load=nil;loadstring=nil;loadfile=nil;dofile=nil;collectgarbage=nil;print=nil;launch={devMode=false};main={};ConPrintf=function()end").exec()?;
    lua.globals().set(
        "GetTime",
        lua.create_function(move |_, ()| Ok(started.elapsed().as_secs_f64() * 1000.0))?,
    )?;
    let sources = Arc::new(sources.clone());
    let order = Arc::new(Mutex::new(Vec::new()));
    let provided = sources.clone();
    let observed = order.clone();
    lua.globals().set(
        "LoadModule",
        lua.create_function(move |lua, mut args: MultiValue| {
            let Some(Value::String(name)) = args.pop_front() else {
                return Err(mlua::Error::RuntimeError("missing source module".into()));
            };
            let name = name.to_str()?;
            if name.contains("..") || name.starts_with('/') || name.contains('\\') {
                return Err(mlua::Error::RuntimeError(
                    "invalid source module path".into(),
                ));
            }
            let path = format!("src/{}.lua", name.strip_suffix(".lua").unwrap_or(&name));
            load(lua, &provided, &observed, &path, args)
        })?,
    )?;
    let provided = sources;
    let observed = order.clone();
    let loaded = lua.create_table()?;
    lua.globals().set(
        "require",
        lua.create_function(move |lua, name: String| {
            if name == "lua-utf8" {
                return Ok(utf8.clone());
            }
            if name == "bit" {
                return Ok(bit.clone());
            }
            // Host networking/profiling libraries are optional and never used by Item.
            if name == "lcurl.safe" || name == "lua-profiler" {
                return Ok(Value::Nil);
            }
            if let value @ Value::Table(_) = loaded.get::<Value>(name.as_str())? {
                return Ok(value);
            }
            if name.contains("..") || name.contains('/') || name.contains('\\') {
                return Err(mlua::Error::RuntimeError(
                    "invalid unique library path".into(),
                ));
            }
            let stem = name.replace('.', "/");
            let direct = format!("runtime/lua/{stem}.lua");
            let path = if provided.contains_key(&direct) {
                direct
            } else {
                format!("runtime/lua/{stem}/init.lua")
            };
            let value = load(lua, &provided, &observed, &path, MultiValue::new())?
                .pop_front()
                .unwrap_or(Value::Nil);
            loaded.set(name, value.clone())?;
            Ok(value)
        })?,
    )?;
    for module in [
        "GameVersions",
        "Modules/Common",
        "Modules/CalcFormat",
        "Modules/Data",
        "Modules/ModTools",
        "Modules/ItemTools",
        "Modules/CalcTools",
        "Classes/Item",
    ] {
        lua.globals()
            .get::<Function>("LoadModule")?
            .call::<MultiValue>(module)?;
    }
    Ok((lua, order))
}

// These wrappers record the original iterator outputs and constructor calls;
// they never sort source iteration, substitute constructors, or supply modifiers.
const OBSERVE: &str = r#"
local originalPairs=pairs
local groups={}
for group,rows in originalPairs(data.uniques) do groups[rows]=group end
local current,active
uniqueObservation={records={},reads={},insertions=0,count=0,key_bytes=0}
local observation=uniqueObservation
pairs=function(t)
    local iter,state,key=originalPairs(t)
    local group=groups[t]
    if not group then return iter,state,key end
    return function(s,k)
        local index,raw=iter(s,k)
        if index~=nil then current={group=group,index=index,raw=raw,keys={},key_count=0} end
        return index,raw
    end,state,key
end
local class=common.classes.Item
local originalConstructor=class.Item
class.Item=function(self,raw,rarity,highQuality)
    assert(current and not active and current.raw==raw,'unexpected nested/unattributed original constructor')
    assert(rarity==uniqueConstructorRarity and highQuality==uniqueConstructorHighQuality,'constructor arguments changed')
    assert(observation.count<50000,'unique constructor count bound')
    active=true
    local result=originalConstructor(self,raw,rarity,highQuality)
    active=false
    current.item=self
    observation.count=observation.count+1
    observation.records[observation.count]=current
    return result
end
local backing={}
main.uniqueDB={loading=true,list=setmetatable({}, {
    __index=function(_,key)
        assert(active and current,'unique read outside original constructor')
        assert(type(key)=='string' and #key<=4096,'unique lookup key bound/type')
        if not current.keys[key] then
            assert(current.key_count<32 and observation.key_bytes+#key<=8*1024*1024,'unique lookup aggregate/count bound')
            current.key_count=current.key_count+1
            observation.key_bytes=observation.key_bytes+#key
            current.keys[key]=true
        end
        assert(backing[key]==nil,'original constructor read an earlier unique entry')
        return backing[key]
    end,
    __newindex=function(_,key,item)
        assert(not active and current and current.item==item,'unattributed unique insertion')
        assert(type(key)=='string' and #key<=4096 and backing[key]==nil,'duplicate/invalid unique canonical key')
        backing[key]=item
        current.inserted=key
        observation.insertions=observation.insertions+1
    end
})}
uniqueRestore=function()
    pairs=originalPairs
    class.Item=originalConstructor
end
"#;

pub(crate) fn extract(
    sources: &BTreeMap<String, String>,
    items: &ItemLoadingData,
    tree: &BundledClassTree,
) -> Result<UniqueRequirementData> {
    let (lua, order) = host(sources)?;
    let item_source = source(sources, ITEM)?;
    let main_source = source(sources, MAIN)?;
    let defaults = section(
        main_source,
        "\n\tself.defaultItemAffixQuality = 0.5\n",
        "\tself.showTitlebarName",
    )?;
    eval_part(&lua, sources, MAIN, defaults, "local self=main;", "")?;
    let stored_cache = section(
        main_source,
        "\t\tfor k, v in pairs(LoadModule(\"Data/ModCache\")) do",
        "\n\tend\n",
    )?;
    eval_part(&lua, sources, MAIN, stored_cache, "", "")?;
    let tree_version = &tree.source.tree_version;
    lua.globals()
        .get::<Table>("data")?
        .get::<Function>("setJewelRadiiGlobally")?
        .call::<()>(tree_version.as_str())?;
    let original_loop = section(
        main_source,
        "\t\tfor type, typeList in pairsYield(data.uniques) do",
        "\t\tfor _, raw in pairsYield(data.rares) do",
    )?;
    let arguments = original_loop
        .split_once("new(\"Item\"):Item(raw, ")
        .and_then(|(_, tail)| tail.split_once(')'))
        .ok_or_else(|| error("unrecognized original unique constructor"))?
        .0;
    let args: Table = lua.load(format!("return {{{arguments}}}")).eval()?;
    let rarity: String = args.get(1)?;
    let high_quality: bool = args.get(2)?;
    lua.globals()
        .set("uniqueConstructorRarity", rarity.clone())?;
    lua.globals()
        .set("uniqueConstructorHighQuality", high_quality)?;
    lua.load(OBSERVE)
        .set_name("@unique-construction-observer")
        .exec()?;
    eval_part(
        &lua,
        sources,
        MAIN,
        original_loop,
        "local self=main;local function original_unique_load()",
        "end;local thread=coroutine.create(original_unique_load);repeat local ok,err=coroutine.resume(thread);assert(ok,err) until coroutine.status(thread)=='dead';uniqueRestore()",
    )?;
    let main: Table = lua.globals().get("main")?;
    if !matches!(
        main.get::<Table>("uniqueDB")?.get::<Value>("loading")?,
        Value::Nil
    ) {
        return Err(error("original unique loading not complete"));
    }
    let observed: Table = lua.globals().get("uniqueObservation")?;
    let rows: Table = observed.get("records")?;
    let mut prototypes = Vec::new();
    let mut entries = Vec::new();
    let mut retained = 0usize;
    let mut base_pointers = BTreeMap::new();
    for pair in lua
        .globals()
        .get::<Table>("data")?
        .get::<Table>("itemBases")?
        .pairs::<String, Table>()
    {
        let (name, base) = pair?;
        if base_pointers
            .insert(base.to_pointer() as usize, name)
            .is_some()
        {
            return Err(error("ambiguous original base object identity"));
        }
    }
    for row in rows.sequence_values::<Table>() {
        let row = row?;
        let group: String = row.get("group")?;
        let index: u32 = row.get("index")?;
        let raw: String = row.get("raw")?;
        let expected = items
            .unique_groups
            .get(&group)
            .and_then(|rows| index.checked_sub(1).and_then(|i| rows.get(i as usize)))
            .ok_or_else(|| error("observed prototype outside original inventory"))?;
        if expected != &raw {
            return Err(error(
                "original prototype differs from injected item inventory",
            ));
        }
        let prototype = UniquePrototypeId {
            group,
            index,
            raw_sha256: hash(raw.as_bytes()),
        };
        let lookup_keys: Vec<String> = row
            .get::<Table>("keys")?
            .pairs::<String, bool>()
            .map(|p| p.map(|(key, _)| key))
            .collect::<mlua::Result<BTreeSet<_>>>()?
            .into_iter()
            .collect();
        if lookup_keys.len() > 32 {
            return Err(error("unique lookup count bound"));
        }
        retained = retained
            .checked_add(lookup_keys.iter().map(String::len).sum::<usize>())
            .ok_or_else(|| error("unique retained bytes overflow"))?;
        if retained > 8 * 1024 * 1024 {
            return Err(error("unique retained lookup bytes bound"));
        }
        let item: Table = row.get("item")?;
        let disposition = if let Some(canonical_key) = row.get::<Option<String>>("inserted")? {
            let base: Table = item.get("base")?;
            let base_name = base_pointers
                .get(&(base.to_pointer() as usize))
                .ok_or_else(|| error("unknown constructed base object"))?
                .clone();
            if item.get::<String>("name")? != canonical_key {
                return Err(error("constructed/insertion identity changed"));
            }
            let requirements: Table = item.get("requirements")?;
            entries.push(UniqueRequirementEntry {
                canonical_key: canonical_key.clone(),
                prototype: prototype.clone(),
                base_name,
                natural_level: requirement(requirements.get("naturalLevel")?)?,
                level: requirement(requirements.get("level")?)?,
            });
            UniquePrototypeDisposition::Inserted { canonical_key }
        } else {
            if !matches!(item.get::<Value>("base")?, Value::Nil) {
                return Err(error("constructed base was not inserted"));
            }
            UniquePrototypeDisposition::SkippedMissingBase
        };
        prototypes.push(UniquePrototypeOutcome {
            prototype,
            disposition,
            lookup_keys,
        });
    }
    prototypes.sort_by(|a, b| a.prototype.cmp(&b.prototype));
    entries.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
    let order = order
        .lock()
        .map_err(|_| error("unique module trace lock"))?
        .clone();
    let mut files: BTreeSet<_> = order.iter().cloned().collect();
    files.insert(MAIN.into());
    let mut spans = BTreeMap::new();
    for (name, path, part) in [
        ("unique_loop", MAIN, original_loop),
        ("defaults", MAIN, defaults),
        ("stored_cache", MAIN, stored_cache),
        (
            "item_constructor",
            ITEM,
            section(
                item_source,
                "function ItemClass:Item(",
                "---@enum (key) LineFlags",
            )?,
        ),
        (
            "unique_lookup",
            ITEM,
            section(
                item_source,
                "function ItemClass:GetUniqueDBItem()",
                "---@class ModLine",
            )?,
        ),
        (
            "item_parse",
            ITEM,
            section(
                item_source,
                "function ItemClass:ParseRaw(",
                "function ItemClass:NormaliseQuality(",
            )?,
        ),
        (
            "item_assembly",
            ITEM,
            &item_source[item_source
                .find("function ItemClass:BuildModList()")
                .ok_or_else(|| error("missing original assembly"))?..],
        ),
        (
            "pairs_yield",
            COMMON,
            section(
                source(sources, COMMON)?,
                "function pairsYield(t)",
                "-- Based on https://www.lua.org/pil/19.3.html",
            )?,
        ),
    ] {
        spans.insert(name.into(), span(sources, path, part)?);
        files.insert(path.into());
    }
    let data = UniqueRequirementData {
        schema_version: UNIQUE_REQUIREMENTS_SCHEMA_VERSION,
        state: UniqueRequirementState::Complete(Box::new(CompleteUniqueRequirements {
            source: ItemLoadingSource {
                upstream_revision: crate::source::UPSTREAM_REVISION.into(),
                files: files
                    .into_iter()
                    .map(|path| Ok((path.clone(), hash(source(sources, &path)?.as_bytes()))))
                    .collect::<Result<_>>()?,
                construction_spans: spans,
                module_order: order,
            },
            inputs: UniqueRequirementInputs {
                item_loading_sha256: hash(&serde_json::to_vec(items)?),
                tree_sha256: hash(&serde_json::to_vec(tree)?),
                tree_version: tree_version.clone(),
                constructor_rarity: rarity,
                constructor_high_quality: high_quality,
                mod_cache_mode: UniqueModCacheMode::OriginalStoredCache,
                default_item_quality: main.get("defaultItemQuality")?,
                default_affix_quality: main.get("defaultItemAffixQuality")?,
            },
            policy: lookup_policy(item_source)?,
            construction: UniqueConstruction {
                source_loop_completed: true,
                loading_cleared: true,
                constructors_finished: observed.get("count")?,
                insertions: observed.get("insertions")?,
            },
            prototypes,
            entries,
        })),
    };
    data.validate_inputs(items, tree).map_err(error)?;
    Ok(data)
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
            .map(|path| {
                let text = crate::source::read_verified_text(&root, &path).unwrap();
                (path, text)
            })
            .collect()
    }
    #[test]
    fn requirement_projection_retains_types_without_lua_string_coercion() {
        let lua = Lua::new();
        assert_eq!(requirement(Value::Nil).unwrap(), None);
        assert_eq!(requirement(Value::Integer(0)).unwrap(), Some(0.0));
        assert_eq!(
            requirement(Value::Number(-0.0)).unwrap().unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
        for value in [
            Value::String(lua.create_string("49").unwrap()),
            Value::Boolean(true),
            Value::Number(f64::INFINITY),
            Value::Number(f64::NAN),
            Value::Table(lua.create_table().unwrap()),
        ] {
            assert!(requirement(value).is_err());
        }
    }
    #[test]
    fn original_full_constructor_projection_reproduces_reviewed_catalog() {
        let sources = sources();
        let items = crate::item_loading_extract::extract(&sources).unwrap();
        let tree = poe_optimizer_data::bundled::class_tree().unwrap();
        let actual = extract(&sources, &items, tree).unwrap();
        let bundled = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        assert_eq!(&actual, bundled.unique_requirements().data());
        let complete = actual.complete().unwrap();
        assert_eq!(complete.entries.len(), 443);
        assert_eq!(complete.prototypes.len(), 443);
        assert_eq!(
            complete.inputs.mod_cache_mode,
            UniqueModCacheMode::OriginalStoredCache
        );
        for span in complete.source.construction_spans.values() {
            let bytes = sources[&span.path]
                .split_inclusive('\n')
                .skip(span.line as usize - 1)
                .take((span.end_line - span.line + 1) as usize)
                .collect::<String>();
            assert_eq!(span.sha256, hash(bytes.as_bytes()));
        }
    }
    #[test]
    fn unfinished_original_loading_marker_cannot_publish_ready_catalog() {
        let mut sources = sources();
        let items = crate::item_loading_extract::extract(&sources).unwrap();
        let original = sources.get_mut(MAIN).unwrap();
        assert_eq!(original.matches("self.uniqueDB.loading = nil").count(), 1);
        *original = original.replace(
            "self.uniqueDB.loading = nil",
            "self.uniqueDB.loading = true",
        );
        let tree = poe_optimizer_data::bundled::class_tree().unwrap();
        assert!(
            extract(&sources, &items, tree)
                .unwrap_err()
                .0
                .contains("original unique loading not complete")
        );
    }
    #[test]
    fn lookup_policy_comes_from_original_function_and_rejects_other_pattern_semantics() {
        let sources = sources();
        let item = &sources[ITEM];
        let base = lookup_policy(item).unwrap();
        let changed = item
            .replace("^Runeforged (.+)", "^Caller Prefix (.+)")
            .replace(
                r#"self.title .. ", " .. originalBaseName"#,
                r#"self.title .. " / " .. originalBaseName"#,
            );
        let custom = lookup_policy(&changed).unwrap();
        assert_eq!(custom.base_prefixes[0], "Caller Prefix ");
        assert_eq!(custom.title_base_separator, " / ");
        assert_ne!(custom, base);
        assert!(lookup_policy(&item.replace("^Runeforged (.+)", "^Runeforged (.*)")).is_err());
    }
}
