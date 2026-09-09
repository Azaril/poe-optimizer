//! Complete authenticated item-definition construction. No item/effect callbacks run.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_data::item_loading::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
type Result<T> = std::result::Result<T, GameDataExtractionError>;
const DATA: &str = "src/Modules/Data.lua";
const ITEM: &str = "src/Classes/Item.lua";
const COMMON: &str = "src/Modules/Common.lua";
fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("missing item definition source {path}")))
}
fn span(
    sources: &BTreeMap<String, String>,
    path: &str,
    line: u32,
    end_line: u32,
) -> Result<ItemSourceSpan> {
    let text = source(sources, path)?;
    if line == 0 || end_line < line || end_line as usize > text.lines().count() {
        return Err(error("invalid item source span"));
    }
    Ok(ItemSourceSpan {
        path: path.into(),
        line,
        end_line,
        sha256: hash(
            text.split_inclusive('\n')
                .skip(line as usize - 1)
                .take((end_line - line + 1) as usize)
                .collect::<String>()
                .as_bytes(),
        ),
    })
}
fn chunk<'a>(
    sources: &'a BTreeMap<String, String>,
    path: &str,
    begin: &str,
    end: &str,
) -> Result<(&'a str, ItemSourceSpan)> {
    let text = source(sources, path)?;
    let part = section(text, begin, end)?;
    let at = part.as_ptr() as usize - text.as_ptr() as usize;
    let first = text[..at].bytes().filter(|b| *b == b'\n').count() as u32 + 1;
    let last = first + part.trim_end().bytes().filter(|b| *b == b'\n').count() as u32;
    Ok((part, span(sources, path, first, last)?))
}
fn named_eval<T: mlua::FromLuaMulti>(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    path: &str,
    begin: &str,
    end: &str,
    suffix: &str,
) -> Result<T> {
    let (part, s) = chunk(sources, path, begin, end)?;
    Ok(lua
        .load(format!(
            "{}{}\n{suffix}",
            "\n".repeat(s.line as usize - 1),
            part
        ))
        .set_name(format!("@{path}"))
        .eval()?)
}
fn metadata(
    v: Value,
    sources: &BTreeMap<String, String>,
    depth: usize,
    budget: &mut usize,
) -> Result<ItemMetadataValue> {
    if depth > 24 || *budget == 0 {
        return Err(error("item metadata depth/value budget"));
    }
    *budget -= 1;
    Ok(match v {
        Value::Boolean(v) => ItemMetadataValue::Boolean(v),
        Value::Integer(v) => ItemMetadataValue::Number(v as f64),
        Value::Number(v) if v.is_finite() => ItemMetadataValue::Number(v),
        Value::String(v) => ItemMetadataValue::Text(v.to_str()?.to_owned()),
        Value::Table(t) => {
            let t = table(t, sources, depth + 1, budget)?;
            if t.fields.is_empty()
                && !t.indexed.is_empty()
                && t.indexed.keys().copied().eq(1..=t.indexed.len() as i64)
            {
                ItemMetadataValue::Array(t.indexed.into_values().collect())
            } else {
                ItemMetadataValue::Table(t)
            }
        }
        Value::Function(f) => {
            let info = f.info();
            let path = info
                .source
                .as_deref()
                .and_then(|s| s.strip_prefix('@'))
                .ok_or_else(|| error("item callback lacks authenticated Lua source"))?;
            let line =
                info.line_defined
                    .ok_or_else(|| error("item callback lacks start line"))? as u32;
            let end = info
                .last_line_defined
                .ok_or_else(|| error("item callback lacks end line"))? as u32;
            ItemMetadataValue::Callback(ItemOpaqueFunction {
                callback: span(sources, path, line, end)?,
            })
        }
        _ => return Err(error("unsupported/nonfinite item metadata source value")),
    })
}
fn table(
    t: Table,
    sources: &BTreeMap<String, String>,
    depth: usize,
    budget: &mut usize,
) -> Result<ItemMetadataTable> {
    if depth > 24 || t.metatable().is_some() {
        return Err(error("item metadata depth/metatable unsupported"));
    }
    let mut out = ItemMetadataTable::default();
    for pair in t.pairs::<Value, Value>() {
        let (k, v) = pair?;
        let v = metadata(v, sources, depth + 1, budget)?;
        match k {
            Value::String(s) => {
                out.fields.insert(s.to_str()?.to_owned(), v);
            }
            Value::Integer(n) => {
                out.indexed.insert(n, v);
            }
            Value::Number(n)
                if n.is_finite() && n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0 =>
            {
                out.indexed.insert(n as i64, v);
            }
            _ => return Err(error("unsupported item metadata key")),
        }
        if out.fields.len() + out.indexed.len() > 100_000 {
            return Err(error("item metadata table count"));
        }
    }
    Ok(out)
}
fn strings(t: Table) -> Result<Vec<String>> {
    let mut values = BTreeMap::new();
    for p in t.pairs::<usize, String>() {
        let (k, v) = p?;
        values.insert(k, v);
    }
    if !values.keys().copied().eq(1..=values.len()) {
        return Err(error("item source string list is sparse/mixed"));
    }
    Ok(values.into_values().collect())
}
fn quoted_after(text: &str, anchor: &str) -> Result<String> {
    let tail = text
        .split_once(anchor)
        .ok_or_else(|| error(format!("item policy anchor missing {anchor}")))?
        .1;
    let tail = tail.strip_prefix('"').ok_or_else(|| {
        error(format!(
            "item policy expected quoted literal after {anchor}: {}",
            tail.chars().take(70).collect::<String>()
        ))
    })?;
    Ok(tail
        .split_once('"')
        .ok_or_else(|| error("item policy unterminated literal"))?
        .0
        .into())
}
const DEFENCE_HEADER_START: &str = "\t\t\t\telseif specName == \"Armour\" or";
const DEFENCE_HEADER_END: &str = "\t\t\t\telseif specName == \"Level\" then";
/// Observe the original complete header branch on a fresh source-style Item
/// state. Its original numeric helper runs; no calculations or base records are
/// substituted. Rewrites are exported separately from the same source branch.
fn defence_header_keys(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let (branch, _) = chunk(sources, ITEM, DEFENCE_HEADER_START, DEFENCE_HEADER_END)?;
    let predicate = branch
        .lines()
        .next()
        .ok_or_else(|| error("empty defence branch"))?;
    let scan: Function = lua
        .load(
            r#"return function(s)
        local names={}
        for name in s:gmatch('specName == "([^"]+)"') do names[#names+1]=name end
        return names
    end"#,
        )
        .eval()?;
    let headers = strings(scan.call(predicate)?)?;
    if headers.is_empty() || headers.len() > 256 {
        return Err(error("defence header source membership bound"));
    }
    let numeric: Function = named_eval(
        lua,
        sources,
        ITEM,
        "local function specToNumber(s)",
        "local function parseItemSpec(line)",
        "return specToNumber",
    )?;
    let projection: Function = lua.load(format!(
        "return function(specToNumber, specName)\nlocal self={{}}\nlocal specVal='1'\n{}\nend\nreturn self.armourData\nend",
        branch.replacen("elseif", "if", 1),
    )).eval()?;
    let mut keys = BTreeMap::new();
    for header in headers {
        let output: Table = projection.call((numeric.clone(), header.as_str()))?;
        let mut entries = output.pairs::<String, f64>();
        let (key, value) = entries
            .next()
            .ok_or_else(|| error("defence header writes no key"))??;
        if entries.next().is_some() || value != 1.0 || keys.insert(header, key).is_some() {
            return Err(error(
                "defence header projection is not one numeric assignment",
            ));
        }
    }
    Ok(keys)
}
fn policy(lua: &Lua, sources: &BTreeMap<String, String>) -> Result<ItemLoadingPolicy> {
    let constants: Table = named_eval(
        lua,
        sources,
        ITEM,
        "local catalystList =",
        "local function getCatalystScalar",
        "return {names=catalystList,descriptors=catalystDescriptorList,tags=catalystTags}",
    )?;
    let names = strings(constants.get("names")?)?;
    let descriptions = strings(constants.get("descriptors")?)?;
    let tags: Table = constants.get("tags")?;
    if names.len() != descriptions.len() || names.len() != tags.raw_len() {
        return Err(error("catalyst tables are not aligned"));
    }
    let catalysts = names
        .into_iter()
        .zip(descriptions)
        .enumerate()
        .map(|(i, (name, descriptor))| {
            Ok(ItemCatalystDefinition {
                name,
                descriptor,
                tags: strings(tags.get(i + 1)?)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let flags: Table = named_eval(
        lua,
        sources,
        ITEM,
        "local lineFlags =",
        "local function baseHasImplicitLine",
        "return lineFlags",
    )?;
    let mut line_flags = BTreeSet::new();
    for p in flags.pairs::<String, bool>() {
        let (k, v) = p?;
        if !v {
            return Err(error("line flag false"));
        }
        line_flags.insert(k);
    }
    let colors: Table = lua.globals().get("colorCodes")?;
    let rarities = colors
        .pairs::<String, Value>()
        .map(|p| p.map(|p| p.0).map_err(error))
        .collect::<Result<BTreeSet<_>>>()?;
    let item = source(sources, ITEM)?;
    let raw_headers = section(
        item,
        "\t\t\tlocal specName, specVal = parseItemSpec(line)",
        "\t\t\tif line == \"Prefixes:\" then",
    )?;
    let scan:Function=lua.load("return function(s) local t={} for name in s:gmatch('specName == \"([^\"]+)\"') do t[name]=true end return t end").eval()?;
    let headers: Table = scan.call(raw_headers)?;
    let selection: Table = named_eval(
        lua,
        sources,
        ITEM,
        "local variantSelectionSpecNames =",
        "function ItemClass:HasVariantGroups()",
        "return variantSelectionSpecNames",
    )?;
    for p in selection.clone().pairs::<String, bool>() {
        let (k, v) = p?;
        if v {
            headers.set(k, true)?;
        }
    }
    let header_names = headers
        .pairs::<String, bool>()
        .map(|p| p.map(|p| p.0).map_err(error))
        .collect::<Result<_>>()?;
    let defaults = source(sources, "src/Modules/Main.lua")?;
    let number = |anchor: &str| -> Result<f64> {
        let tail = defaults
            .split_once(anchor)
            .ok_or_else(|| error("missing main item default"))?
            .1;
        tail.lines()
            .next()
            .ok_or_else(|| error("empty default"))?
            .trim()
            .parse()
            .map_err(error)
    };
    let mut compatibility = BTreeMap::new();
    let mut budget = 100_000;
    let mut roles = ItemMetadataTable::default();
    roles.fields.insert(
        "default".into(),
        ItemMetadataValue::Text(quoted_after(item, "self.rarity = rarity or ")?),
    );
    let normal_magic = section(
        item,
        r#"if not self.base and (self.rarity == "#,
        "-- Exact match (affix-less magic and normal items)",
    )?;
    roles.fields.insert(
        "normal".into(),
        ItemMetadataValue::Text(quoted_after(normal_magic, "self.rarity == ")?),
    );
    roles.fields.insert(
        "magic".into(),
        ItemMetadataValue::Text(quoted_after(normal_magic, "or self.rarity == ")?),
    );
    let relic = section(item, "-- Hack for relics", "\n\t\t\tl = l + 1")?;
    roles.fields.insert(
        "unique".into(),
        ItemMetadataValue::Text(quoted_after(item, "if self.rarity == ")?),
    );
    roles.fields.insert(
        "relic".into(),
        ItemMetadataValue::Text(quoted_after(relic, "self.rarity = ")?),
    );
    compatibility.insert("rarity_roles".into(), ItemMetadataValue::Table(roles));
    let superior = quoted_after(item, "baseName = line:gsub(")?;
    let superior = superior
        .strip_prefix('^')
        .ok_or_else(|| error("superior prefix is not anchored"))?;
    compatibility.insert(
        "superior_prefix".into(),
        ItemMetadataValue::Text(superior.into()),
    );
    let literal_flags:Function=lua.load(r#"return function(s,variable)
        local out,names,fields={}, {}, {}
        local function finish() if next(fields) then for _,name in ipairs(names) do out[name]=fields end end end
        for line in (s..'\n'):gmatch('(.-)\n') do
            if line:match('^%s*elseif ') or line:match('^%s*if ') then
                finish();names={};fields={}
                for name in line:gmatch(variable..' == "([^"]+)"') do names[#names+1]=name end
            else
                local field,value=line:match('^%s*self%.([%w_]+) = (%a+)%s*$')
                if field and (value=='true' or value=='false') then fields[field]=(value=='true') end
            end
        end
        finish();return out
    end"#).eval()?;
    let state_flags = section(
        item,
        r#"elseif line == "Sanctified" then"#,
        r#"elseif line:match("^%(%a+")"#,
    )?;
    compatibility.insert(
        "literal_state_flags".into(),
        metadata(
            Value::Table(literal_flags.call((state_flags, "line"))?),
            sources,
            0,
            &mut budget,
        )?,
    );
    let post_flags = section(
        item,
        r#"if lineLower == "implicit modifiers cannot be changed" then"#,
        "modLine.socketedAugmentTypeOverride =",
    )?;
    compatibility.insert(
        "postparse_line_effects".into(),
        metadata(
            Value::Table(literal_flags.call((post_flags, "lineLower"))?),
            sources,
            0,
            &mut budget,
        )?,
    );
    let magnitude: Table = named_eval(
        lua,
        sources,
        ITEM,
        "local modMagnitudePattern =",
        r#"if lineLower == "implicit modifiers cannot be changed" then"#,
        "return modMagnitudePattern",
    )?;
    compatibility.insert(
        "mod_magnitude_patterns".into(),
        metadata(Value::Table(magnitude), sources, 0, &mut budget)?,
    );

    compatibility.insert(
        "selection_headers".into(),
        metadata(Value::Table(selection), sources, 0, &mut budget)?,
    );
    let corruption_line = item
        .lines()
        .find(|l| l.contains("self.corruptible = self.base.type"))
        .ok_or_else(|| error("missing corruptible policy"))?;
    let excluded:Function=lua.load(r#"return function(s) local t={} for name in s:gmatch('self.base.type ~= "([^"]+)"') do t[#t+1]=name end return t end"#).eval()?;
    compatibility.insert(
        "noncorruptible_types".into(),
        metadata(
            Value::Table(excluded.call(corruption_line)?),
            sources,
            0,
            &mut budget,
        )?,
    );
    let fallback = section(
        item,
        "self.affixes = (self.base.subType",
        "self.corruptible = ",
    )?;
    let fallback = fallback
        .rsplit_once("or data.itemMods.")
        .ok_or_else(|| error("missing modifier table fallback"))?
        .1
        .trim();
    if !fallback
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(error("nonliteral modifier fallback"));
    }
    compatibility.insert(
        "fallback_modifier_table".into(),
        ItemMetadataValue::Text(fallback.into()),
    );

    let hidden = section(
        raw_headers,
        "elseif specName == \"Critical Hit Range\"",
        "\t\t\t\t\tself.hidden_specs = true",
    )?;
    compatibility.insert(
        "hidden_specs".into(),
        metadata(Value::Table(scan.call(hidden)?), sources, 0, &mut budget)?,
    );
    let mut aliases = ItemMetadataTable::default();
    for (k, anchor) in [
        ("energy_blade_marker", "self.name:match("),
        ("two_toned_marker", "self.name:find("),
    ] {
        let block = if k.starts_with("energy") {
            section(
                item,
                "-- Exact match (affix-less magic and normal items)",
                "-- Partial match (magic items with affixes)",
            )?
        } else {
            section(
                item,
                "if not baseName then\n\t\t\t\t\t\tlocal s, e",
                "self.name = self.name:gsub",
            )?
        };
        aliases.fields.insert(
            k.into(),
            ItemMetadataValue::Text(quoted_after(block, anchor)?),
        );
    }
    let eb = section(
        item,
        "if self.name:match(\"Energy Blade\")",
        "\t\t\t\t\tif data.itemBases[self.name]",
    )?;
    aliases.fields.insert(
        "energy_blade_one_hand_pattern".into(),
        ItemMetadataValue::Text(quoted_after(eb, "itemClass:match(")?),
    );
    aliases.fields.insert(
        "energy_blade_one_hand".into(),
        ItemMetadataValue::Text(quoted_after(
            eb.split_once("self.name = ")
                .ok_or_else(|| error("missing energy blade assignment"))?
                .1,
            ") and ",
        )?),
    );
    aliases.fields.insert(
        "energy_blade_two_hand".into(),
        ItemMetadataValue::Text(quoted_after(eb, " or ")?),
    );
    let tt = section(
        item,
        "if baseName == \"Two-Toned Boots\"",
        "local base = data.itemBases[baseName]",
    )?;
    aliases.fields.insert(
        "two_toned_default".into(),
        ItemMetadataValue::Text(quoted_after(tt, "baseName = ")?),
    );
    let runic = section(
        item,
        "if baseName:find(\"Runeforged\")",
        "self.runicItem = true",
    )?;
    aliases.fields.insert(
        "runic_markers".into(),
        ItemMetadataValue::Array(vec![
            ItemMetadataValue::Text(quoted_after(runic, "baseName:find(")?),
            ItemMetadataValue::Text(quoted_after(runic, "or baseName:find(")?),
        ]),
    );
    let switches = section(
        item,
        r#"if specName == "Evasion Rating" then"#,
        "self.armourData = self.armourData or",
    )?;
    let switches_fn: Function = lua
        .load(
            r#"return function(s)
      local out,header,from={}
      for line in (s..'\n'):gmatch('(.-)\n') do
        header=line:match('specName == "([^"]+)"') or header
        from=line:match('if self.baseName == "([^"]+)"') or from
        local target=line:match('self.baseName = "([^"]+)"')
        if target then assert(header and from);out[header]={from=from,to=target};from=nil end
      end
      return out
    end"#,
        )
        .eval()?;
    aliases.fields.insert(
        "armour_header_rewrites".into(),
        metadata(
            Value::Table(switches_fn.call(switches)?),
            sources,
            0,
            &mut budget,
        )?,
    );
    compatibility.insert("base_aliases".into(), ItemMetadataValue::Table(aliases));
    // Derive direct field assignments from the actual source branch text. Special
    // branches remain named headers plus source spans, never invented effects.
    let assignments:Function=lua.load(r#"return function(s)
        local out={}
        for name, body in s:gmatch('specName == "([^"]+)" then[^\n]*\n([^\n]+)') do
            local field=body:match('^%s*self%.([%w_%.]+) = specToNumber%(specVal%)%s*$')
            local kind='number'
            if not field then field=body:match('^%s*self%.([%w_%.]+) = %(?specVal == "true"%)?%s*$') kind='boolean' end
            if not field then field=body:match('^%s*self%.([%w_%.]+) = specVal%s*$') kind='text' end
            if not field then field=body:match('^%s*self%.([%w_%.]+) = true%s*$') kind='presence' end
            if field then out[name]={field=field,kind=kind} end
        end
        return out
    end"#).eval()?;
    compatibility.insert(
        "header_assignments".into(),
        metadata(
            Value::Table(assignments.call(raw_headers)?),
            sources,
            0,
            &mut budget,
        )?,
    );
    let fallback: Table = named_eval(
        lua,
        sources,
        "src/Classes/ItemsTab.lua",
        "local fallBackJewelSocketCount =",
        "if item.base then",
        "return fallBackJewelSocketCount",
    )?;
    compatibility.insert(
        "fallback_jewel_socket_counts".into(),
        metadata(Value::Table(fallback), sources, 0, &mut budget)?,
    );
    Ok(ItemLoadingPolicy {
        default_affix_quality: number("self.defaultItemAffixQuality = ")?,
        default_item_quality: number("self.defaultItemQuality = ")?,
        catalysts,
        line_flags,
        rarities,
        header_names,
        defence_header_keys: defence_header_keys(lua, sources)?,
        compatibility,
    })
}
pub(crate) fn extract(sources: &BTreeMap<String, String>) -> Result<ItemLoadingData> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT | StdLib::JIT,
        LuaOptions::default(),
    )?;
    lua.set_memory_limit(512 * 1024 * 1024)?;
    lua.load("jit.off();jit.flush();jit=nil;load=nil;loadstring=nil;loadfile=nil;dofile=nil;collectgarbage=nil;print=nil;package=nil;io=nil;os=nil;debug=nil;ffi=nil;require=nil;data={}").exec()?;
    let ticks = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(100_000),
        move |_, _| {
            if ticks.fetch_add(1, Ordering::Relaxed) > 20_000 {
                return Err(mlua::Error::RuntimeError(
                    "item construction instruction budget".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    let mut files = BTreeSet::new();
    files.extend(
        [
            COMMON,
            DATA,
            ITEM,
            "src/Modules/Main.lua",
            "src/Classes/ItemsTab.lua",
        ]
        .map(str::to_owned),
    );
    let mut construction_spans = BTreeMap::new();
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
        named_eval::<()>(&lua, sources, COMMON, begin, end, "")?;
    }
    // Preserve the original helper's complete function span, without executing
    // any UI/runtime initialization from Common.lua.
    let common = source(sources, COMMON)?;
    let helper_start = common
        .find("function isValueInArray(")
        .ok_or_else(|| error("missing isValueInArray"))?;
    let helper_end = common[helper_start..]
        .find("\nend")
        .ok_or_else(|| error("missing helper end"))?
        + helper_start
        + 4;
    lua.load(format!(
        "{}{}",
        "\n".repeat(
            common[..helper_start]
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
        ),
        &common[helper_start..helper_end]
    ))
    .set_name(format!("@{COMMON}"))
    .exec()?;
    let order = Arc::new(Mutex::new(Vec::new()));
    let loaded = order.clone();
    let authenticated = Arc::new(sources.clone());
    lua.globals().set(
        "LoadModule",
        lua.create_function(move |lua, name: String| {
            let path = format!(
                "src/{}",
                if name.ends_with(".lua") {
                    name
                } else {
                    format!("{name}.lua")
                }
            );
            let text = authenticated.get(&path).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("unapproved item dependency {path}"))
            })?;
            loaded
                .lock()
                .map_err(|_| mlua::Error::RuntimeError("module lock".into()))?
                .push(path.clone());
            lua.load(text).set_name(format!("@{path}")).eval::<Value>()
        })?,
    )?;
    let loader: Function = lua.globals().get("LoadModule")?;
    loader.call::<Value>("GameVersions")?;
    // All selected source blocks keep original line positions in one lexical
    // chunk, so makeSkillMod/itemTypes locals and function provenance are exact.
    let ranges = [
        (
            "prefix",
            "LoadModule(\"Data/Global\")",
            "-----------------\n-- Common Data",
        ),
        ("keystones", "data.keystones =", "data.ailmentTypeList ="),
        (
            "jewel_radii",
            "data.jewelRadii =",
            "data.jewelRadius = data.setJewelRadiiGlobally",
        ),
        ("modifiers", "-- Load item modifiers", "data.essences ="),
        ("skills", "-- Load skills\n", "-- Load minions\n"),
        ("bases", "-- Item bases\n", "-- Build lists of item bases"),
        ("uniques", "-- Uniques (loaded", "data.questRewards ="),
    ];
    let original = source(sources, DATA)?;
    let mut code = String::new();
    let mut cursor = 0;
    for (name, begin, end) in ranges {
        let (part, s) = chunk(sources, DATA, begin, end)?;
        let at = part.as_ptr() as usize - original.as_ptr() as usize;
        if at < cursor {
            return Err(error("item construction source blocks reordered"));
        }
        code.push_str(&"\n".repeat(original[cursor..at].bytes().filter(|b| *b == b'\n').count()));
        code.push_str(part);
        cursor = at + part.len();
        construction_spans.insert(name.into(), s);
    }
    lua.load(code).set_name(format!("@{DATA}")).exec()?;
    let module_order = order.lock().map_err(|_| error("module lock"))?.clone();
    files.extend(module_order.iter().cloned());
    let data: Table = lua.globals().get("data")?;
    let base_table: Table = data.get("itemBases")?;
    let mut base_sources = BTreeMap::new();
    // Every original module is independently applied to an empty table solely
    // to retain origin attribution; final values always come from original Data.
    for path in module_order
        .iter()
        .filter(|p| p.starts_with("src/Data/Bases/"))
    {
        let f: Function = lua
            .load(source(sources, path)?)
            .set_name(format!("@{path}"))
            .eval()?;
        let t = lua.create_table()?;
        f.call::<()>(t.clone())?;
        for p in t.pairs::<String, Value>() {
            base_sources.insert(p?.0, path.clone());
        }
    }
    let mut budget = 1_500_000;
    let mut bases = Vec::new();
    for p in base_table.pairs::<String, Table>() {
        let (name, t) = p?;
        bases.push(ItemBaseDefinition {
            item_type: t.get("type")?,
            source_module: base_sources
                .remove(&name)
                .ok_or_else(|| error("base lacks source module"))?,
            name,
            fields: table(t, sources, 0, &mut budget)?,
        });
    }
    bases.sort_by(|a, b| a.name.cmp(&b.name));
    let mut modifier_tables = BTreeMap::new();
    for p in data.get::<Table>("itemMods")?.pairs::<String, Table>() {
        let (k, t) = p?;
        modifier_tables.insert(k, table(t, sources, 0, &mut budget)?);
    }
    let mut unique_groups = BTreeMap::new();
    for p in data.get::<Table>("uniques")?.pairs::<String, Table>() {
        let (k, t) = p?;
        unique_groups.insert(k, strings(t)?);
    }
    let jewel_radii = table(data.get("jewelRadii")?, sources, 0, &mut budget)?;
    let policy = policy(&lua, sources)?;
    for (name, path, begin, end) in [
        (
            "item_parser",
            ITEM,
            "function ItemClass:ParseRaw(",
            "function ItemClass:NormaliseQuality(",
        ),
        (
            "item_loading",
            "src/Classes/ItemsTab.lua",
            "function ItemsTabClass:Load(",
            "function ItemsTabClass:Save(",
        ),
    ] {
        if let Ok((_, s)) = chunk(sources, path, begin, end) {
            construction_spans.insert(name.into(), s);
        } else {
            return Err(error(format!("missing complete {name} source span")));
        }
    }
    let (_, defence_span) = chunk(sources, ITEM, DEFENCE_HEADER_START, DEFENCE_HEADER_END)?;
    construction_spans.insert("defence_headers".into(), defence_span);
    let source = ItemLoadingSource {
        upstream_revision: crate::source::UPSTREAM_REVISION.into(),
        files: files
            .into_iter()
            .map(|p| Ok((p.clone(), hash(source(sources, &p)?.as_bytes()))))
            .collect::<Result<_>>()?,
        construction_spans,
        module_order,
    };
    let result = ItemLoadingData {
        schema_version: ITEM_LOADING_SCHEMA_VERSION,
        capability: ItemLoadingCapability::DefinitionsOnly,
        source,
        policy,
        bases,
        modifier_tables,
        unique_groups,
        jewel_radii,
    };
    result.validate().map_err(error)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_retains_original_callback_spans_without_execution_and_rejects_unmodeled_values() {
        let lua = Lua::new();
        let path = "src/Data/Bases/callback.lua";
        let text = "return function()\n error('must never execute')\nend\n";
        let sources = BTreeMap::from([(path.into(), text.into())]);
        let callback: Function = lua.load(text).set_name(format!("@{path}")).eval().unwrap();
        let out = metadata(Value::Function(callback), &sources, 0, &mut 100).unwrap();
        let ItemMetadataValue::Callback(out) = out else {
            panic!("callback descriptor required")
        };
        assert_eq!(out.callback, span(&sources, path, 1, 3).unwrap());
        let cyclic: Table = lua.load("local t={} t.self=t return t").eval().unwrap();
        assert!(table(cyclic, &sources, 0, &mut 100).is_err());
        let mt: Table = lua.load("return setmetatable({}, {})").eval().unwrap();
        assert!(table(mt, &sources, 0, &mut 100).is_err());
        let key: Table = lua.load("return {[true]=1}").eval().unwrap();
        assert!(table(key, &sources, 0, &mut 100).is_err());
        let unknown: Function = lua.load("return function()end").eval().unwrap();
        assert!(metadata(Value::Function(unknown), &sources, 0, &mut 100).is_err());
    }
    #[test]
    fn defence_keys_are_observed_from_original_branch_and_follow_changed_source_literals() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let original = crate::source::read_verified_text(&root, ITEM).unwrap();
        let changed = original
            .replace("Runic Ward", "Caller Guard")
            .replace("specName = \"Ward\"", "specName = \"CallerValue\"");
        let sources = BTreeMap::from([(ITEM.into(), changed)]);
        let keys = defence_header_keys(&Lua::new(), &sources).unwrap();
        assert_eq!(
            keys.get("Caller Guard").map(String::as_str),
            Some("CallerValue")
        );
        assert!(!keys.contains_key("Runic Ward"));
        assert_eq!(keys.get("Ward").map(String::as_str), Some("Ward"));
    }
    #[test]
    fn complete_package_extension_preserves_existing_sections() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let result = crate::game_data::extract_pinned_game_data_for_review(&root).unwrap();
        let bytes = result.package.canonical_bytes().unwrap();
        let loaded = poe_optimizer_data::game_data::GameDataLoader::from_bytes(
            &bytes,
            &poe_optimizer_data::game_data::TrustPolicy::AllowCustom,
            &poe_optimizer_data::game_data::LoadLimits::default(),
        )
        .unwrap();
        let old: serde_json::Value =
            serde_json::from_slice(poe_optimizer_data::game_data::bundled_package_bytes()).unwrap();
        let new = serde_json::to_value(loaded.package()).unwrap();
        for (name, value) in old.as_object().unwrap() {
            if name != "manifest" && name != "item_loading" {
                assert_eq!(value, &new[name], "existing section changed: {name}");
            }
        }
        let mut old_item = old["item_loading"].clone();
        let mut new_item = new["item_loading"].clone();
        for item in [&mut old_item, &mut new_item] {
            item.as_object_mut().unwrap().remove("schema_version");
            item["policy"]
                .as_object_mut()
                .unwrap()
                .remove("defence_header_keys");
            item["source"]["construction_spans"]
                .as_object_mut()
                .unwrap()
                .remove("defence_headers");
        }
        assert_eq!(
            old_item, new_item,
            "unrelated item-loading definitions changed"
        );
        for (name, digest) in old["manifest"]["section_sha256"].as_object().unwrap() {
            if name != "item_loading" {
                assert_eq!(
                    digest, &new["manifest"]["section_sha256"][name],
                    "section digest changed: {name}"
                );
            }
        }
        if let Some(out) = std::env::var_os("POE_ITEM_PACKAGE_OUTPUT") {
            let out = std::path::PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(out.join("game-data.json"), &bytes).unwrap();
            std::fs::write(
                out.join("evidence.json"),
                serde_json::to_vec_pretty(&result.evidence).unwrap(),
            )
            .unwrap();
            std::fs::write(out.join("game-data.sha256"), format!("{}\n", hash(&bytes))).unwrap();
        }
        eprintln!(
            "full item package bytes={} sha256={} sections={} sources={}",
            bytes.len(),
            hash(&bytes),
            result.package.manifest.section_sha256.len(),
            result.evidence.source_files_sha256.len()
        );
    }
    #[test]
    fn complete_original_item_catalog_constructs() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let mut sources = BTreeMap::new();
        for path in crate::game_data::expected_source_files().unwrap().keys() {
            sources.insert(
                path.clone(),
                crate::source::read_verified_text(&root, path).unwrap(),
            );
        }
        let catalog = extract(&sources).unwrap();
        assert_eq!(
            catalog.policy.defence_header_keys,
            BTreeMap::from([
                ("Armour".into(), "Armour".into()),
                ("Evasion Rating".into(), "Evasion".into()),
                ("Evasion".into(), "Evasion".into()),
                ("Energy Shield".into(), "EnergyShield".into()),
                ("Ward".into(), "Ward".into()),
                ("Runic Ward".into(), "Ward".into()),
            ])
        );
        assert_eq!(
            catalog.source.construction_spans["defence_headers"].line,
            835
        );
        assert_eq!(
            catalog.source.construction_spans["defence_headers"].end_line,
            854
        );
        assert_eq!(catalog.bases.len(), 1756);
        assert!(catalog.bases.iter().any(|b| b.hidden() == Some(true)));
        assert_eq!(catalog.modifier_tables.len(), 9);
        assert_eq!(
            catalog.policy.compatibility["noncorruptible_types"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>(),
            ["Flask", "Charm", "Transcendent Limb"]
        );
        let assignments = catalog.policy.compatibility["header_assignments"]
            .as_table()
            .unwrap();
        assert_eq!(
            assignments.fields["Allow Duplicate Variants"]
                .as_table()
                .unwrap()
                .fields["kind"]
                .as_str(),
            Some("boolean")
        );
        assert!(!assignments.fields.contains_key("Cluster Jewel Skill"));
        let flags = catalog.policy.compatibility["literal_state_flags"]
            .as_table()
            .unwrap();
        assert_eq!(
            flags.fields["Sanctified"].as_table().unwrap().fields["corruptible"].as_bool(),
            Some(false)
        );
        assert_eq!(
            flags.fields["Twice Corrupted"]
                .as_table()
                .unwrap()
                .fields
                .len(),
            2
        );

        let bytes = serde_json::to_vec(&catalog).unwrap();
        if let Some(out) = std::env::var_os("POE_ITEM_CATALOG_OUTPUT") {
            std::fs::write(out, &bytes).unwrap();
        }
        eprintln!(
            "item catalog bytes={} sha256={} bases={} unique_groups={} source_files={}",
            bytes.len(),
            hash(&bytes),
            catalog.bases.len(),
            catalog.unique_groups.len(),
            catalog.source.files.len()
        );
        let decoded: ItemLoadingData = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, catalog);
    }
}
