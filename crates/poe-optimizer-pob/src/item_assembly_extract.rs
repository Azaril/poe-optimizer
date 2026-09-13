//! Source-owned common assembly policy, with complete method and live binding proof.
use crate::game_data::{GameDataExtractionError, error, hash};
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_data::{
    game_data::ActorData,
    item_assembly::*,
    item_loading::{ItemLoadingData, ItemLoadingSource, ItemSourceSpan},
    item_scalability::ItemScalabilityData,
    modifier_parser::{ModifierParserData, ParserValue},
};
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, GameDataExtractionError>;
mod inventory;
mod jewel;
mod local;
mod slot_validity;
mod weapon;

const SHAPES: &str = include_str!("item_assembly_extract/source-shapes.json");

struct Primitives {
    functions: BTreeMap<&'static str, Function>,
    get_upvalue: Function,
    tables: BTreeMap<&'static str, Table>,
}
impl Primitives {
    fn capture(lua: &Lua) -> Result<Self> {
        // SAFETY: the host alone retains the observation function. No caller code
        // executes with the debug table present, and source never receives it.
        let debug: Table = unsafe {
            lua.exec_raw((), |state| {
                mlua::ffi::luaopen_debug(state);
            })
        }?;
        let get_upvalue: Function = debug.get("getupvalue")?;
        lua.globals().set("debug", Value::Nil)?;
        let mut functions = BTreeMap::new();
        for (name, group, key) in [
            ("ipairs", "", "ipairs"),
            ("pairs", "", "pairs"),
            ("type", "", "type"),
            ("tonumber", "", "tonumber"),
            ("getmetatable", "", "getmetatable"),
            ("select", "", "select"),
            ("t_insert", "table", "insert"),
            ("t_remove", "table", "remove"),
            ("m_floor", "math", "floor"),
            ("m_min", "math", "min"),
            ("m_max", "math", "max"),
            ("m_modf", "math", "modf"),
            ("m_ceil", "math", "ceil"),
            ("bit_band", "bit", "band"),
            ("bit_bnot", "bit", "bnot"),
            ("gsub", "string", "gsub"),
            ("find", "string", "find"),
            ("match", "string", "match"),
        ] {
            let table = if group.is_empty() {
                lua.globals()
            } else {
                lua.globals().get::<Table>(group)?
            };
            let f: Function = table.raw_get(key)?;
            // Identity is captured before provided source runs. LuaJIT's own
            // table.remove is a Lua builtin (source "remove"), not a C closure.
            // Do not invent a C-only boundary for the standard runtime table.
            functions.insert(name, f);
        }
        let mut tables = BTreeMap::new();
        for name in ["string", "math", "bit", "table"] {
            tables.insert(name, lua.globals().raw_get::<Table>(name)?);
        }
        Ok(Self {
            functions,
            get_upvalue,
            tables,
        })
    }
    fn upvalues(&self, f: &Function) -> Result<BTreeMap<String, Value>> {
        let mut out = BTreeMap::new();
        for index in 1..=256 {
            let (name, value): (Option<String>, Value) =
                self.get_upvalue.call((f.clone(), index))?;
            let Some(name) = name else { return Ok(out) };
            if out.insert(name, value).is_some() {
                return Err(error("duplicate assembly closure upvalue"));
            }
        }
        Err(error("assembly upvalue count bound"))
    }
    fn function_upvalue(&self, f: &Function, name: &str) -> Result<Function> {
        match self.upvalues(f)?.remove(name) {
            Some(Value::Function(f)) => Ok(f),
            _ => Err(error(format!("missing assembly helper {name}"))),
        }
    }
}
fn class_function(lua: &Lua, class: &str, name: &str) -> Result<Function> {
    let classes: Table = lua.globals().get::<Table>("common")?.get("classes")?;
    // These methods are inspected before any Item constructor executes. Load
    // their original class module through the host's authenticated loader, just
    // as Common.getClass does on its first use; no synthetic class is installed.
    if matches!(classes.raw_get::<Value>(class)?, Value::Nil) {
        lua.globals()
            .get::<Function>("LoadModule")?
            .call::<()>(format!("Classes/{class}"))?;
    }
    let definition = classes
        .raw_get::<Table>(class)
        .map_err(|e| error(format!("assembly class {class}: {e}")))?;
    definition
        .get(name)
        .map_err(|e| error(format!("assembly method {class}.{name}: {e}")))
}
fn table_function(lua: &Lua, table: &str, name: &str) -> Result<Function> {
    lua.globals()
        .get::<Table>(table)?
        .get(name)
        .map_err(|e| error(format!("assembly table method {table}.{name}: {e}")))
}
fn same(a: &Function, b: &Function) -> bool {
    a.to_pointer() == b.to_pointer()
}
fn source_body<'a>(
    sources: &'a BTreeMap<String, String>,
    span: &ItemSourceSpan,
) -> Result<&'a str> {
    let whole = sources
        .get(&span.path)
        .ok_or_else(|| error("missing assembly source"))?;
    let start = whole
        .split_inclusive('\n')
        .take(span.line as usize - 1)
        .map(str::len)
        .sum::<usize>();
    let length = whole
        .split_inclusive('\n')
        .skip(span.line as usize - 1)
        .take((span.end_line - span.line + 1) as usize)
        .map(str::len)
        .sum::<usize>();
    let body = whole
        .get(start..start + length)
        .ok_or_else(|| error("invalid assembly source extent"))?;
    if hash(body.as_bytes()) != span.sha256 {
        return Err(error(format!(
            "changed complete assembly source {}:{}",
            span.path, span.line
        )));
    }
    Ok(body)
}
struct Auth<'a> {
    bodies: BTreeMap<String, &'a str>,
    spans: BTreeMap<String, ItemSourceSpan>,
    mask: f64,
    weapon: weapon::Bindings,
}
fn authenticate<'a>(
    lua: &Lua,
    primitives: &Primitives,
    sources: &'a BTreeMap<String, String>,
) -> Result<Auth<'a>> {
    let mut spans: BTreeMap<String, ItemSourceSpan> = serde_json::from_str(SHAPES)?;
    // Source-only role: loading ItemsTab would execute unrelated UI/rune setup.
    // The full original method is exercised independently by the parity host.
    spans.insert("slot_validity".into(), slot_validity::span());
    spans.extend(inventory::spans());
    let mut bodies = BTreeMap::new();
    for (role, span) in &spans {
        bodies.insert(role.clone(), source_body(sources, span)?);
    }
    let mut functions = BTreeMap::new();
    for (role, class, name) in [
        ("build_mod_list", "Item", "BuildModList"),
        ("build_slot", "Item", "BuildModListForSlotNum"),
        ("build_slots", "Item", "BuildModListsForSlots"),
        ("primary_slot", "Item", "GetPrimarySlot"),
        ("variant_count", "Item", "GetModLineVariantCount"),
        ("variant_check", "Item", "CheckModLineVariant"),
        ("variant_groups", "Item", "UsesVersionedOrGroupedVariants"),
        ("independent_variants", "Item", "HasIndependentVariants"),
        ("rune_display", "Item", "ApplySocketedRuneDisplayScalars"),
        ("mod_list_constructor", "ModList", "ModList"),
        ("add_mod", "ModList", "AddMod"),
        ("flag_internal", "ModList", "FlagInternal"),
        ("list_internal", "ModList", "ListInternal"),
        ("mod_store_constructor", "ModStore", "ModStore"),
        ("new_mod", "ModStore", "NewMod"),
        ("flag_query", "ModStore", "Flag"),
        ("list_query", "ModStore", "List"),
        ("scale_add_mod", "ModStore", "ScaleAddMod"),
    ] {
        functions.insert(role, class_function(lua, class, name)?);
    }
    for (role, name) in [
        ("and64", "AND64"),
        ("not64", "NOT64"),
        ("keyword_match", "MatchKeywordFlags"),
        ("copy_table", "copyTable"),
        ("round", "round"),
        ("round_symmetric", "roundSymmetric"),
    ] {
        functions.insert(
            role,
            lua.globals()
                .get(name)
                .map_err(|e| error(format!("assembly global {name}: {e}")))?,
        );
    }
    for (role, table, name) in [
        ("create_mod", "modLib", "createMod"),
        ("set_source", "modLib", "setSource"),
        ("apply_range", "itemLib", "applyRange"),
        ("zero_line", "itemLib", "isZeroValueLine"),
    ] {
        functions.insert(role, table_function(lua, table, name)?);
    }
    functions.insert(
        "ranged_mods",
        primitives.function_upvalue(&functions["build_mod_list"], "getRangedModList")?,
    );
    functions.insert(
        "calc_local",
        primitives.function_upvalue(&functions["build_mod_list"], "calcLocal")?,
    );
    functions.insert(
        "and_pair",
        primitives.function_upvalue(&functions["and64"], "and2")?,
    );
    for (role, f) in &functions {
        let info = f.info();
        let span = &spans[*role];
        if info.what != "Lua"
            || info.source.as_deref() != Some(&format!("@{}", span.path))
            || info.line_defined != Some(span.line as usize)
            || info.last_line_defined != Some(span.end_line as usize)
            || f.environment()
                .is_none_or(|env| env.to_pointer() != lua.globals().to_pointer())
        {
            return Err(error(format!("assembly live source/environment {role}")));
        }
        for (name, value) in primitives.upvalues(f)? {
            if let Some(expected) = primitives.functions.get(name.as_str())
                && !matches!(value,Value::Function(ref actual) if same(actual,expected))
            {
                return Err(error(format!("assembly primitive capture {role}.{name}")));
            }
            if name == "HIGH_MASK_53"
                && !matches!(value,Value::Number(v) if v==((1u64<<21)-1) as f64)
                && !matches!(value,Value::Integer(v) if v==((1i64<<21)-1))
            {
                return Err(error("unrepresented assembly bit width"));
            }
        }
    }
    for (owner, name, target) in [
        ("build_slot", "calcLocal", "calc_local"),
        ("new_mod", "mod_createMod", "create_mod"),
        ("and64", "and2", "and_pair"),
        ("flag_internal", "band", "and64"),
        ("list_internal", "band", "and64"),
        ("keyword_match", "band", "and64"),
    ] {
        if !same(
            &primitives.function_upvalue(&functions[owner], name)?,
            &functions[target],
        ) {
            return Err(error(format!("assembly helper binding {owner}.{name}")));
        }
    }
    for (group, key, primitive) in [
        ("string", "gsub", "gsub"),
        ("string", "find", "find"),
        ("string", "match", "match"),
        ("bit", "band", "bit_band"),
        ("bit", "bnot", "bit_bnot"),
        ("math", "floor", "m_floor"),
        ("math", "min", "m_min"),
        ("math", "max", "m_max"),
        ("math", "modf", "m_modf"),
    ] {
        if !same(
            &table_function(lua, group, key)?,
            &primitives.functions[primitive],
        ) {
            return Err(error("assembly global primitive rebound"));
        }
    }
    for (name, expected) in &primitives.tables {
        let actual: Table = lua.globals().raw_get(*name)?;
        if actual.to_pointer() != expected.to_pointer() {
            return Err(error("assembly primitive table rebound"));
        }
    }
    for name in [
        "type",
        "tonumber",
        "select",
        "pairs",
        "ipairs",
        "getmetatable",
    ] {
        let actual: Function = lua.globals().raw_get(name)?;
        if !same(&actual, &primitives.functions[name]) {
            return Err(error("assembly global primitive rebound"));
        }
    }
    let string_meta: Table = primitives.functions["getmetatable"].call("")?;
    let index: Table = string_meta.raw_get("__index")?;
    if index.to_pointer() != primitives.tables["string"].to_pointer() {
        return Err(error("assembly string method lookup rebound"));
    }
    let mask = match primitives
        .upvalues(&functions["keyword_match"])?
        .remove("MatchAllMask")
    {
        Some(Value::Number(v)) if v.is_finite() => v,
        Some(Value::Integer(v)) => v as f64,
        _ => return Err(error("missing original MatchAllMask capture")),
    };
    let weapon = weapon::authenticate(
        lua,
        primitives,
        &functions["build_slot"],
        sources
            .get("src/Classes/Item.lua")
            .ok_or_else(|| error("missing weapon declaration source"))?,
    )?;
    Ok(Auth {
        weapon,
        bodies,
        spans,
        mask,
    })
}

fn after<'a>(text: &'a str, prefix: &str) -> Result<&'a str> {
    text.split_once(prefix)
        .map(|(_, tail)| tail)
        .ok_or_else(|| error(format!("missing assembly source field {prefix}")))
}
fn quoted_end(text: &str) -> Result<usize> {
    let bytes = text.as_bytes();
    let Some(quote @ (b'\'' | b'"')) = bytes.first().copied() else {
        return Err(error("assembly source is not quoted literal"));
    };
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == quote {
            return Ok(i + 1);
        } else {
            i += 1;
        }
    }
    Err(error("unterminated assembly literal"))
}
fn string(lua: &Lua, text: &str) -> Result<String> {
    let text = text.trim_start();
    let end = quoted_end(text)?;
    Ok(lua.load(format!("return {}", &text[..end])).eval()?)
}
fn text_after(lua: &Lua, text: &str, prefix: &str) -> Result<String> {
    string(lua, after(text, prefix)?)
}
fn number(text: &str) -> Result<f64> {
    let text = text.trim_start();
    let end = text
        .find(|c: char| !c.is_ascii_digit() && !matches!(c, '-' | '+' | '.' | 'e' | 'E'))
        .unwrap_or(text.len());
    let n = text[..end].parse::<f64>().map_err(error)?;
    if !n.is_finite() {
        return Err(error("nonfinite assembly source number"));
    }
    Ok(n)
}
fn num_after(text: &str, prefix: &str) -> Result<f64> {
    number(after(text, prefix)?)
}
fn count(text: &str) -> Result<u16> {
    let v = number(text)?;
    if (0.0..=256.0).contains(&v) && v.fract() == 0.0 {
        Ok(v as u16)
    } else {
        Err(error("assembly source count outside contract"))
    }
}
fn precision(text: &str) -> Result<u8> {
    let value = count(text)?;
    if value > 15 {
        return Err(error("assembly source precision outside contract"));
    }
    u8::try_from(value).map_err(error)
}
fn args(text: &str) -> Result<Vec<&str>> {
    let mut out = vec![];
    let mut stack = vec![];
    let mut start = 0;
    let mut i = 0;
    let b = text.as_bytes();
    while i < b.len() {
        match b[i] {
            b'\'' | b'"' => {
                i += quoted_end(&text[i..])?;
                continue;
            }
            b'(' | b'{' | b'[' => stack.push(b[i]),
            b')' if stack.is_empty() => {
                out.push(text[start..i].trim());
                return Ok(out);
            }
            b')' | b'}' | b']' => {
                stack
                    .pop()
                    .ok_or_else(|| error("assembly source argument grouping"))?;
            }
            b',' if stack.is_empty() => {
                out.push(text[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    Err(error("unclosed assembly source call"))
}
fn query(lua: &Lua, text: &str) -> Result<ItemAssemblyLocalQuery> {
    let a = args(after(text, "calcLocal(")?)?;
    if a.len() != 4 {
        return Err(error("assembly local-query arity"));
    }
    Ok(ItemAssemblyLocalQuery {
        name: string(lua, a[1])?,
        mod_type: string(lua, a[2])?,
        flags: number(a[3])?,
    })
}
fn rewrite(lua: &Lua, text: &str) -> Result<ItemAssemblyTextRewrite> {
    let a = args(after(text, "gsub(")?)?;
    if a.len() != 2 {
        return Err(error("assembly rewrite arity"));
    }
    Ok(ItemAssemblyTextRewrite {
        pattern: string(lua, a[0])?,
        replacement: string(lua, a[1])?,
    })
}

fn ident(text: &str) -> &str {
    let text = text.trim_start();
    let end = text
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(text.len());
    &text[..end]
}
fn attribute(name: &str) -> Result<ItemAssemblyAttribute> {
    match name {
        "str" => Ok(ItemAssemblyAttribute::Strength),
        "dex" => Ok(ItemAssemblyAttribute::Dexterity),
        "int" => Ok(ItemAssemblyAttribute::Intelligence),
        _ => Err(error("unknown structural requirement field")),
    }
}
fn predicate(lua: &Lua, text: &str) -> Result<ItemAssemblySlotPredicate> {
    let text = text.trim();
    if text == "self.base.weapon" {
        return Ok(ItemAssemblySlotPredicate::BaseWeaponTruthy);
    }
    let (field, value) = text
        .split_once(" == ")
        .ok_or_else(|| error("unsupported assembly slot predicate"))?;
    let value = string(lua, value)?;
    Ok(match field {
        "self.base.type" => ItemAssemblySlotPredicate::BaseTypeEquals { value },
        "self.type" => ItemAssemblySlotPredicate::ItemTypeEquals { value },
        "self.base.subType" => ItemAssemblySlotPredicate::BaseSubtypeEquals { value },
        _ => return Err(error("unrepresented slot field")),
    })
}
fn predicates(lua: &Lua, text: &str) -> Result<Vec<ItemAssemblySlotPredicate>> {
    text.split(" or ").map(|p| predicate(lua, p)).collect()
}
fn order(text: &str, pattern: impl Fn(&str) -> String) -> Result<[ItemAssemblyAttribute; 3]> {
    let mut rows = vec![];
    for name in ["str", "dex", "int"] {
        let at = text
            .find(&pattern(name))
            .ok_or_else(|| error("missing requirement order role"))?;
        rows.push((at, attribute(name)?));
    }
    rows.sort_by_key(|r| r.0);
    Ok(rows
        .into_iter()
        .map(|r| r.1)
        .collect::<Vec<_>>()
        .try_into()
        .expect("three attributes"))
}
fn policy(
    lua: &Lua,
    auth: &Auth<'_>,
    items: &ItemLoadingData,
    parser: &ModifierParserData,
    tree: &poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot,
) -> Result<ItemAssemblyPolicy> {
    let body = |name: &str| {
        auth.bodies
            .get(name)
            .copied()
            .ok_or_else(|| error("missing authenticated policy method"))
    };
    let build = body("build_mod_list")?;
    let slot = body("build_slot")?;
    let local = body("calc_local")?;
    let ranged = body("ranged_mods")?;
    let (armour, flask, charm) = local::extract(lua, slot)?;
    let weapon = weapon::extract(lua, slot, &auth.weapon, parser)?;
    let jewel_item_type = text_after(lua, build, "elseif self.type == ")?;
    let jewel = jewel::extract(lua, build, slot, &jewel_item_type)?;
    let slot_validity = slot_validity::extract(lua, body("slot_validity")?)?;
    let inventory = inventory::extract(lua, &auth.bodies, tree)?;
    let quality = query(lua, after(slot, "local craftedQuality = ")?)?;
    let soul = query(lua, after(build, "self.socketedSoulCoreEffectModifier = ")?)?;
    let rune = query(lua, after(build, "self.socketedRuneEffectModifier = ")?)?;
    let global = query(
        lua,
        after(build, "self.socketedAugmentItemEffectModifier = ")?,
    )?;
    let rp = &items.policy.rune_loading;
    for (q, name, field) in [
        (
            &global,
            rp.effect_global_name.clone(),
            "self.socketedAugmentItemEffectModifier = ",
        ),
        (
            &rune,
            format!(
                "{}{}{}",
                rp.effect_name_prefix, rp.rune_augment_type, rp.effect_name_suffix
            ),
            "self.socketedRuneEffectModifier = ",
        ),
        (
            &soul,
            format!(
                "{}{}{}",
                rp.effect_name_prefix, rp.extra_slot_augment_type, rp.effect_name_suffix
            ),
            "self.socketedSoulCoreEffectModifier = ",
        ),
    ] {
        if q.name != name
            || q.mod_type != rp.effect_mod_type
            || q.flags != global.flags
            || num_after(after(build, field)?, " / ")? != rp.effect_divisor
        {
            return Err(error(
                "assembly and rune loading effect definitions disagree",
            ));
        }
    }
    if text_after(lua, build, "if modLine.augmentType == ")? != rp.extra_slot_augment_type
        || text_after(lua, build, "elseif modLine.augmentType == ")? != rp.rune_augment_type
        || num_after(body("rune_display")?, "modLine.displayValueScalar = ")? != rp.scalar_base
    {
        return Err(error("assembly rune display role mismatch"));
    }
    let flag_kind = text_after(lua, local, "if type == ")?;
    if text_after(lua, body("flag_internal")?, "mod.type == ")? != flag_kind {
        return Err(error("inconsistent local/global flag kind"));
    }
    let in_slot = text_after(lua, local, "mod[1].type == ")?;
    if text_after(lua, slot, "or tag.type == ")? != in_slot {
        return Err(error("inconsistent local/slot tag kind"));
    }
    let source_args = after(build, "self.modSource = ")?;
    let class_line = after(build, "self.classRestriction = ")?;
    let class_rewrite = rewrite(lua, class_line)?;
    let mut rows = vec![];
    for line in build.lines().filter(|line| {
        line.contains("for _, modLine in ipairs(self.") && !line.contains("buffModLines")
    }) {
        let key = ident(after(line, "ipairs(self.")?);
        let row = match key {
            "enchantModLines" => ItemAssemblyRows::Enchant,
            "runeModLines" => ItemAssemblyRows::Rune,
            "classRequirementModLines" => ItemAssemblyRows::ClassRequirement,
            "implicitModLines" => ItemAssemblyRows::Implicit,
            "explicitModLines" => ItemAssemblyRows::Explicit,
            _ => return Err(error("unknown assembly row category")),
        };
        if rows.len() < 5 {
            rows.push(row);
        }
    }
    let collection = ItemAssemblyCollectionPolicy {
        rows: rows
            .try_into()
            .map_err(|_| error("incomplete assembly row order"))?,
        source_prefix: string(lua, source_args)?,
        source_separator: text_after(lua, source_args, ")..")?,
        missing_item_id: num_after(source_args, "self.id or ")?,
        duplicate_alternate_count: count(after(body("variant_count")?, "for i = 1, ")?)?,
        class_find_pattern: text_after(lua, build, "modLine.line:find(")?,
        class_variant_rewrite: class_rewrite,
        class_capture_pattern: text_after(lua, class_line, ":match(")?,
    };
    let mut restrictions = vec![];
    for line in build
        .lines()
        .filter(|line| line.contains("self.canSocketJewelBase["))
    {
        restrictions.push(ItemAssemblyJewelRestriction {
            base_name: text_after(lua, line, "self.canSocketJewelBase[")?,
            query: query(lua, line)?,
        });
    }
    let mut named = vec![];
    for tail in build.split("\n\tif self.name == ").skip(1) {
        let (condition, tail) = tail
            .split_once(" then")
            .ok_or_else(|| error("named assembly condition"))?;
        let part = tail
            .split_once("\n\tend")
            .ok_or_else(|| error("named assembly body"))?
            .0;
        let names = condition
            .split(" or self.name == ")
            .map(|name| string(lua, name))
            .collect::<Result<Vec<_>>>()?;
        let a = args(after(part, "baseList:NewMod(")?)?;
        if a.len() != 3 {
            return Err(error("named assembly modifier arity"));
        }
        let modifier = ItemAssemblyKeyValueMod {
            name: string(lua, a[0])?,
            mod_type: string(lua, a[1])?,
            key: text_after(lua, a[2], "key = ")?,
            value: num_after(a[2], "value = ")?,
        };
        let requirement_override = if let Some((_, tail)) = part.split_once("self.requirements.") {
            Some(ItemAssemblyRequirementOverride {
                attribute: attribute(ident(tail))?,
                value: num_after(tail, " = ")?,
            })
        } else {
            None
        };
        named.push(ItemAssemblyNamedRule {
            names,
            modifier,
            requirement_override,
        });
    }
    let converted = after(build, "elseif calcLocal(baseList,")?;
    let converted = converted
        .split_once("\n\telse")
        .ok_or_else(|| error("converted requirement body"))?
        .0;
    let ordinary = after(build, "\n\telse\n\t\tself.requirements.strMod")?;
    let ordinary = format!("self.requirements.strMod{ordinary}");
    let mut attributes = vec![];
    for field in ["str", "dex", "int"] {
        let base_line = after(converted, &format!("self.requirements.{field}Base = "))?
            .lines()
            .next()
            .ok_or_else(|| error("empty converted requirement line"))?;
        let raw_pair = after(base_line, "Conversion * (")?
            .split_once(')')
            .ok_or_else(|| error("raw requirement addends"))?
            .0;
        let raw = raw_pair
            .split(" + ")
            .map(|s| attribute(ident(after(s, "self.requirements.")?)))
            .collect::<Result<Vec<_>>>()?;
        let conv_pair = base_line
            .rsplit_once(" * (")
            .ok_or_else(|| error("conversion subtraction pair"))?
            .1
            .trim_end_matches(')');
        let conv = conv_pair
            .split(" + ")
            .map(|s| {
                attribute(
                    s.trim()
                        .strip_suffix("Conversion")
                        .ok_or_else(|| error("conversion role"))?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        attributes.push(ItemAssemblyAttributeRule {
            attribute: attribute(field)?,
            conversion: query(
                lua,
                after(converted, &format!("local {field}Conversion = "))?,
            )?,
            base: query(lua, base_line)?,
            increased: query(
                lua,
                after(converted, &format!("self.requirements.{field}Mod = "))?,
            )?,
            source_addends: raw
                .try_into()
                .map_err(|_| error("raw requirement pair count"))?,
            conversion_subtrahends: conv
                .try_into()
                .map_err(|_| error("conversion requirement pair count"))?,
        });
    }
    let converted_query = after(build, "elseif calcLocal(")?;
    let req = ItemAssemblyRequirementPolicy {
        no_attributes: query(
            lua,
            after(build, "if calcLocal(")
                .map(|s| format!("calcLocal({s}"))?
                .as_str(),
        )?,
        converted: query(lua, &format!("calcLocal({converted_query}"))?,
        attributes: attributes
            .try_into()
            .map_err(|_| error("attribute role count"))?,
        conversion_read_order: order(converted, |k| format!("local {k}Conversion ="))?,
        converted_write_order: order(converted, |k| format!("self.requirements.{k}Base ="))?,
        ordinary_write_order: order(&ordinary, |k| format!("self.requirements.{k}Mod ="))?,
        percent_divisor: num_after(converted, " / ")?,
    };
    // The same requirement division role occurs in every branch, in original order.
    for field in ["str", "dex", "int"] {
        for source in [
            after(converted, &format!("local {field}Conversion = "))?,
            after(converted, &format!("self.requirements.{field}Mod = "))?,
            after(&ordinary, &format!("self.requirements.{field}Mod = "))?,
        ] {
            if num_after(source, " / ")? != req.percent_divisor {
                return Err(error("requirement divisor mismatch"));
            }
        }
    }
    let primary = body("primary_slot")?;
    let mut primary_rules = vec![];
    let mut condition = None;
    for line in primary.lines() {
        let line = line.trim();
        if let Some(s) = line
            .strip_prefix("if ")
            .or_else(|| line.strip_prefix("elseif "))
        {
            condition = Some(
                s.strip_suffix(" then")
                    .ok_or_else(|| error("primary slot condition"))?,
            );
        } else if line == "else" {
            condition = None;
        } else if let Some(s) = line.strip_prefix("return ")
            && let Some(condition) = condition.take()
        {
            primary_rules.push(ItemAssemblyPrimarySlotRule {
                any: predicates(lua, condition)?,
                slot_name: string(lua, s)?,
            });
        }
    }
    let slots = body("build_slots")?;
    let dispatch = after(slots, "\n\tif ")?
        .split_once(" then")
        .ok_or_else(|| error("slot dispatch predicate"))?
        .0;
    let slot_loop = after(slots, "for i = 1, ")?
        .lines()
        .next()
        .ok_or_else(|| error("slot count expression"))?;
    let tag_body = after(slot, "tag[k] = ")?;
    let first = args(after(tag_body, "gsub(")?)?;
    let second_tail = after(tag_body, ":gsub(")?;
    let second_tail = after(second_tail, ":gsub(")?;
    let second = args(second_tail)?;
    let third = args(after(second_tail, ":gsub(")?)?;
    if first.len() != 2 || second.len() != 2 || third.len() != 2 {
        return Err(error("slot replacement arity"));
    }
    let quality_mod = args(after(slot, "modList:NewMod(")?)?;
    if quality_mod.len() != 4 {
        return Err(error("quality modifier constructor arity"));
    }
    let slots = ItemAssemblySlotPolicy {
        primary: primary_rules,
        multislot_any: predicates(lua, dispatch)?,
        default_multislot_count: count(after(slot_loop, " or ")?)?,
        slot_count_overrides: vec![ItemAssemblySlotCountOverride {
            item_type: text_after(lua, slot_loop, "self.type == ")?,
            count: count(after(slot_loop, " and ")?)?,
        }],
        renamed_slot: count(after(slot, "if slotNum == ")?)?,
        slot_name_rewrite: rewrite(lua, after(slot, "slotName = ")?)?,
        primary_hand_slot: count(after(second[1], "slotNum == ")?)?,
        primary_hand_name: text_after(lua, second[1], " and ")?,
        other_hand_name: text_after(lua, second[1], " or ")?,
        other_number_for_primary: text_after(lua, third[1], " and ")?,
        other_number_for_other: text_after(lua, third[1], " or ")?,
        slot_number_tag_type: text_after(lua, slot, "if tag.type == ")?,
        tag_replacements: [
            ItemAssemblyTagReplacement {
                role: ItemAssemblyTagReplacementRole::SlotName,
                pattern: string(lua, first[0])?,
            },
            ItemAssemblyTagReplacement {
                role: ItemAssemblyTagReplacementRole::Hand,
                pattern: string(lua, second[0])?,
            },
            ItemAssemblyTagReplacement {
                role: ItemAssemblyTagReplacementRole::OtherSlotNumber,
                pattern: string(lua, third[0])?,
            },
        ],
        crafted_quality: quality.clone(),
        quality_name_prefix: string(lua, quality_mod[0])?,
        quality_mod_type: string(lua, quality_mod[1])?,
        quality_source: string(lua, quality_mod[3])?,
        spirit_base: query(lua, after(slot, "local spiritBase = ")?)?,
        spirit_increased: query(lua, after(slot, "local spiritInc = ")?)?,
        spirit_percent_divisor: num_after(slot, "spiritInc / ")?,
        charm_limit: query(lua, after(slot, "self.charmLimit = ")?)?,
        socketed_jewel_effect: query(lua, after(build, "self.socketedJewelEffectModifier = ")?)?,
        socketed_jewel_percent_divisor: num_after(
            after(build, "self.socketedJewelEffectModifier = ")?,
            " / ",
        )?,
    };
    let default_flags = after(body("flag_query")?, "local flags, keywordFlags = ")?
        .lines()
        .next()
        .ok_or_else(|| error("nil query defaults"))?;
    if default_flags
        != after(body("list_query")?, "local flags, keywordFlags = ")?
            .lines()
            .next()
            .ok_or_else(|| error("list query defaults"))?
    {
        return Err(error("nil query defaults disagree"));
    }
    let (flags, keywords) = default_flags
        .split_once(',')
        .ok_or_else(|| error("nil query defaults arity"))?;
    let match_all_field = ident(after(
        body("keyword_match")?,
        "band(modKeywordFlags, KeywordFlag.",
    )?)
    .to_owned();
    let expected = lua
        .globals()
        .get::<Table>("KeywordFlag")?
        .get::<f64>(match_all_field.as_str())?;
    let table = parser
        .policy
        .keyword_flags
        .0
        .checked_sub(1)
        .and_then(|i| parser.tables.get(i as usize))
        .ok_or_else(|| error("missing injected keyword table"))?;
    if !matches!(table.fields.get(&match_all_field),Some(ParserValue::Number(n)) if n.to_bits()==expected.to_bits())
    {
        return Err(error(
            "assembly keyword global differs from injected parser definition",
        ));
    }
    let scale = body("scale_add_mod")?;
    Ok(ItemAssemblyPolicy {
        kinds: ItemAssemblyModKinds {
            base: quality.mod_type,
            increased: global.mod_type,
            more: text_after(lua, local, "elseif type == ")?,
            flag: flag_kind,
            list: text_after(lua, body("list_internal")?, "mod.type == ")?,
        },
        collection,
        range: ItemAssemblyRangePolicy {
            range_find_pattern: text_after(lua, ranged, "modLine.line:find(")?,
            newline_rewrite: rewrite(lua, ranged)?,
        },
        local: ItemAssemblyLocalPolicy {
            keyword_flags: num_after(local, "mod.keywordFlags == ")?,
            in_slot_tag_type: in_slot,
            more_offset: num_after(local, "result * ((")?,
            more_divisor: num_after(local, "mod.value) / ")?,
        },
        nil_queries: ItemAssemblyNilQueryPolicy {
            flags: number(flags)?,
            keyword_flags: number(keywords)?,
            match_all_field,
            captured_match_all_mask: auth.mask,
        },
        rune: ItemAssemblyRunePolicy {
            bonded_unlock: query(
                lua,
                after(build, "self.socketedIdolsUseBondedModifiers = ")?,
            )?,
            effect_flags: global.flags,
        },
        grants: ItemAssemblyGrantPolicy {
            query_name: text_after(lua, build, "baseList:List(nil, ")?,
            unknown_name: text_after(lua, build, "skill.name ~= ")?,
        },
        jewel_restrictions: ItemAssemblyJewelRestrictionPolicy {
            query_name: text_after(lua, build, "baseList:Flag(nil, ")?,
            entries: restrictions,
        },
        named_compatibility: named,
        requirements: req,
        slots,
        slot_validity,
        inventory,
        weapon,
        jewel,
        armour,
        flask,
        charm,
        scale: ItemAssemblyScalePolicy {
            integer_scaled_key: text_after(lua, scale, "scaledMod.value.key == ")?,
            keyed_value_decimal_places: precision(after(
                after(
                    scale,
                    "scaledMod.value[scaledMod.value.keyOfScaledMod] = round(",
                )?,
                " * scale, ",
            )?)?,
            truncation_round_decimal_places: precision(after(
                scale,
                "round(subMod.value * scale, ",
            )?)?,
        },
        jewel_item_type,
    })
}

pub(crate) fn extract(
    sources: &BTreeMap<String, String>,
    items: &ItemLoadingData,
    scalability: &ItemScalabilityData,
    actor: &ActorData,
    parser: &ModifierParserData,
    tree: &poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot,
) -> Result<ItemAssemblyData> {
    let (lua, order, primitives) =
        crate::unique_requirements_extract::host_with_observer(sources, Primitives::capture)?;
    let auth = authenticate(&lua, &primitives, sources)?;
    let policy = policy(&lua, &auth, items, parser, tree)?;
    // Reused precision data is checked against the original complete constructed map.
    let original: Table = lua
        .globals()
        .get::<Table>("data")?
        .get("highPrecisionMods")?;
    let mut count = 0;
    for pair in original.pairs::<String, Table>() {
        let (name, types) = pair?;
        for pair in types.pairs::<String, u8>() {
            let (kind, value) = pair?;
            count += 1;
            if !actor.high_precision_mods.get(&name).is_some_and(|ops| {
                ops.iter()
                    .any(|(op, p)| op.upstream_name() == kind && *p == value)
            }) {
                return Err(error("assembly precision data differs from source"));
            }
        }
    }
    if count
        != actor
            .high_precision_mods
            .values()
            .map(BTreeMap::len)
            .sum::<usize>()
        || lua
            .globals()
            .get::<Table>("data")?
            .get::<u8>("defaultHighPrecision")?
            != scalability.default_high_precision
    {
        return Err(error("assembly precision inventory/fallback mismatch"));
    }
    let module_order = order
        .lock()
        .map_err(|_| error("assembly source module ledger poisoned"))?
        .clone();
    let mut files = BTreeMap::new();
    for path in &module_order {
        files.insert(
            path.clone(),
            hash(
                sources
                    .get(path)
                    .ok_or_else(|| error("unknown assembly loaded module"))?
                    .as_bytes(),
            ),
        );
    }
    for span in auth.spans.values() {
        files.insert(span.path.clone(), hash(sources[&span.path].as_bytes()));
    }
    let data = ItemAssemblyData {
        schema_version: ITEM_ASSEMBLY_SCHEMA_VERSION,
        capability: ItemAssemblyCapability::PolicyOnly,
        source: ItemLoadingSource {
            upstream_revision: crate::source::UPSTREAM_REVISION.into(),
            files,
            construction_spans: auth.spans,
            module_order,
        },
        policy,
    };
    data.validate().map_err(error)?;
    data.validate_dependencies(items, scalability, parser)
        .map_err(error)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    pub(super) fn sources() -> &'static BTreeMap<String, String> {
        static SOURCES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
        SOURCES.get_or_init(|| {
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
        })
    }
    pub(super) fn tree() -> &'static poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot
    {
        static TREE: OnceLock<poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot> =
            OnceLock::new();
        TREE.get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../vendor/path-of-building-poe2");
            let snapshot = crate::tree_data::extract_pinned_tree(&root, "0_5").unwrap();
            let digest = snapshot.sha256().unwrap();
            poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot::from_trusted_extraction(
                snapshot, &digest,
            )
            .unwrap()
        })
    }
    #[test]
    fn complete_original_methods_export_policy_without_assembly_capability() {
        let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        let p = snapshot.package();
        let data = extract(
            sources(),
            &p.item_loading,
            &p.item_scalability,
            &p.actor,
            &p.modifier_parser,
            tree(),
        )
        .unwrap();
        assert_eq!(data.capability, ItemAssemblyCapability::PolicyOnly);
        assert_eq!(
            data.source.construction_spans.len(),
            ITEM_ASSEMBLY_SOURCE_ROLES.len()
        );
        assert_eq!(data.policy.collection.duplicate_alternate_count, 5);
        assert_eq!(data.policy.slots.primary.len(), 6);
        assert_eq!(data.policy.slots.default_multislot_count, 2);
        assert_eq!(data.policy.slots.slot_count_overrides[0].count, 3);
        assert_eq!(data.policy.named_compatibility.len(), 2);
        assert_eq!(data.policy.jewel_restrictions.entries.len(), 4);
        assert_eq!(data.policy.nil_queries.match_all_field, "MatchAll");
        assert_eq!(
            data.policy.collection.class_variant_rewrite.pattern,
            "{variant:([%d,]+)}"
        );
        assert_eq!(data.policy.collection.source_prefix, "Item:");
        assert_eq!(data.policy.range.newline_rewrite.pattern, "\n");
        assert_eq!(
            data.policy.slots.quality_name_prefix,
            "Multiplier:QualityOn"
        );
        let strength = data
            .policy
            .requirements
            .attributes
            .iter()
            .find(|a| a.attribute == ItemAssemblyAttribute::Strength)
            .unwrap();
        assert_eq!(
            strength.source_addends,
            [
                ItemAssemblyAttribute::Intelligence,
                ItemAssemblyAttribute::Dexterity
            ]
        );
        assert_eq!(
            strength.conversion_subtrahends,
            [
                ItemAssemblyAttribute::Dexterity,
                ItemAssemblyAttribute::Intelligence
            ]
        );
        let catalog = ItemAssemblyCatalog::new(data.clone()).unwrap();
        let defs = catalog
            .bind(
                snapshot.item_loading(),
                snapshot.item_scalability(),
                &p.actor,
                snapshot.modifier_parser(),
            )
            .unwrap();
        assert!(
            defs.rune_effect_query(ItemAssemblyRuneEffect::SoulCore)
                .name
                .equals(b"SocketedSoulCoreEffect")
        );
        assert!(std::ptr::eq(
            defs.high_precision_mods(),
            &p.actor.high_precision_mods
        ));
        if let Some(path) = std::env::var_os("POE_OPTIMIZER_ASSEMBLY_POLICY_EXPORT") {
            std::fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
        }
    }
    #[test]
    fn altered_complete_body_and_missing_source_are_rejected() {
        let mut altered = sources().clone();
        let item = altered.get_mut("src/Classes/Item.lua").unwrap();
        *item = item.replace(
            "self.activeBondedState = nil\nend",
            "self.activeBondedState = false\nend",
        );
        let shape: BTreeMap<String, ItemSourceSpan> = serde_json::from_str(SHAPES).unwrap();
        assert!(
            source_body(&altered, &shape["build_mod_list"])
                .unwrap_err()
                .to_string()
                .contains("changed complete")
        );
        altered.remove("src/Classes/ModStore.lua");
        assert!(source_body(&altered, &shape["scale_add_mod"]).is_err());
    }
    #[test]
    fn original_source_alias_and_global_primitive_bindings_are_not_interchangeable() {
        let mut altered = sources().clone();
        let source = altered.get_mut("src/Classes/ModList.lua").unwrap();
        assert!(source.contains("local band = AND64 -- bit.band"));
        *source = source.replace(
            "local band = AND64 -- bit.band",
            "local band = OR64 -- bit.band",
        );
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(&altered, Primitives::capture)
                .unwrap();
        let result = authenticate(&lua, &primitives, &altered);
        assert!(result.err().unwrap().to_string().contains("helper binding"));
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(sources(), Primitives::capture)
                .unwrap();
        let string: Table = lua.globals().get("string").unwrap();
        string
            .set("gsub", string.get::<Function>("find").unwrap())
            .unwrap();
        assert!(
            authenticate(&lua, &primitives, sources())
                .err()
                .unwrap()
                .to_string()
                .contains("primitive")
        );
    }
    #[test]
    fn slot_numeric_lookup_retains_original_tonumber_binding() {
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(sources(), Primitives::capture)
                .unwrap();
        lua.globals()
            .set("tonumber", primitives.functions["type"].clone())
            .unwrap();
        assert!(
            authenticate(&lua, &primitives, sources())
                .err()
                .unwrap()
                .to_string()
                .contains("global primitive rebound")
        );
    }
    #[test]
    fn changed_owner_environment_and_string_method_lookup_are_rejected() {
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(sources(), Primitives::capture)
                .unwrap();
        let f = class_function(&lua, "Item", "BuildModList").unwrap();
        f.set_environment(lua.create_table().unwrap()).unwrap();
        assert!(
            authenticate(&lua, &primitives, sources())
                .err()
                .unwrap()
                .to_string()
                .contains("environment")
        );
        f.set_environment(lua.globals()).unwrap();
        let meta: Table = primitives.functions["getmetatable"].call("").unwrap();
        meta.raw_set("__index", lua.create_table().unwrap())
            .unwrap();
        assert!(
            authenticate(&lua, &primitives, sources())
                .err()
                .unwrap()
                .to_string()
                .contains("method lookup")
        );
    }
    #[test]
    fn local_policies_preserve_source_operand_orders_and_base_list_queries() {
        let shape: BTreeMap<String, ItemSourceSpan> = serde_json::from_str(SHAPES).unwrap();
        let slot = source_body(sources(), &shape["build_slot"]).unwrap();
        let lua = Lua::new();
        let (a, f, c) = local::extract(&lua, slot).unwrap();
        assert_eq!(a.queries.len(), 18);
        assert_eq!(a.queries[0].query.name, "Armour");
        assert_eq!(a.queries[0].base.as_ref().unwrap().field, "Armour");
        assert_eq!(
            a.queries[5].role,
            ItemAssemblyArmourRole::ArmourEnergyShieldBase
        );
        assert_eq!(
            a.defences[2].increased_roles,
            [
                ItemAssemblyArmourRole::EnergyShieldIncreased,
                ItemAssemblyArmourRole::ArmourEnergyShieldIncreased,
                ItemAssemblyArmourRole::EvasionEnergyShieldIncreased,
                ItemAssemblyArmourRole::DefencesIncreased,
            ]
        );
        assert_eq!(
            a.per_level[1].increased_roles,
            a.defences[2].increased_roles
        );
        assert_eq!(a.block.base.name, "BlockChance");
        assert_eq!(a.block.increased.mod_type, "INC");
        assert_eq!(a.movement.multiplier, -1.0);
        assert_eq!(a.overrides.query_name, "ArmourData");
        assert_eq!(f.duration.round_places, 1);
        assert_eq!(f.recovery.channels[0].role, ItemAssemblyRecoveryRole::Life);
        assert_eq!(f.recovery.channels[1].role, ItemAssemblyRecoveryRole::Mana);
        for channel in &f.recovery.channels {
            assert_eq!(channel.effect_not_removed.list, ItemAssemblyQueryList::Base);
        }
        assert_eq!(
            f.recovery.channels[0]
                .additional
                .as_ref()
                .unwrap()
                .query
                .name,
            "FlaskAdditionalLifeRecovery"
        );
        assert_eq!(f.charges.maximum_base.name, "FlaskCharges");
        assert_eq!(f.charges.maximum_increased.mod_type, "INC");
        assert_eq!(f.charges.effect_queries[0].name, "FlaskEffect");
        assert_eq!(c.charges.effect_queries[0].name, "CharmEffect");
        assert_eq!(c.overrides.key_field, "key");
        assert_eq!(c.overrides.value_field, "value");
    }
    #[test]
    fn changed_local_source_is_rejected_before_policy_extraction() {
        let mut altered = sources().clone();
        let item = altered.get_mut("src/Classes/Item.lua").unwrap();
        assert!(item.contains("calcLocal(baseList, \"LifeFlaskEffectNotRemoved\""));
        *item = item.replace(
            "calcLocal(baseList, \"LifeFlaskEffectNotRemoved\"",
            "calcLocal(modList, \"LifeFlaskEffectNotRemoved\"",
        );
        let shape: BTreeMap<String, ItemSourceSpan> = serde_json::from_str(SHAPES).unwrap();
        assert!(
            source_body(&altered, &shape["build_slot"])
                .unwrap_err()
                .to_string()
                .contains("changed complete")
        );
    }
    #[test]
    fn weapon_extraction_uses_actual_capture_order_and_parser_flag_definitions() {
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(sources(), Primitives::capture)
                .unwrap();
        let auth = authenticate(&lua, &primitives, sources()).unwrap();
        let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        let parser = &snapshot.package().modifier_parser;
        let p = weapon::extract(&lua, auth.bodies["build_slot"], &auth.weapon, parser).unwrap();
        assert_eq!(p.base_field, "weapon");
        assert_eq!(p.output_field, "weaponData");
        assert_eq!(
            p.damage
                .channels
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["Physical", "Lightning", "Cold", "Fire", "Chaos"]
        );
        assert_eq!(
            p.damage.channels[0].kind,
            ItemAssemblyWeaponDamageKind::Physical
        );
        assert_eq!(
            p.damage.channels[4].kind,
            ItemAssemblyWeaponDamageKind::Unscaled
        );
        assert_eq!(
            p.damage.channels[1].increased.as_ref().unwrap().name,
            "LocalLightningDamage"
        );
        let flags: Table = lua.globals().raw_get("ModFlag").unwrap();
        let keywords: Table = lua.globals().raw_get("KeywordFlag").unwrap();
        assert_eq!(
            p.attack_speed.query.flags,
            flags.raw_get::<f64>("Attack").unwrap()
        );
        assert_eq!(p.reload.query.flags, p.attack_speed.query.flags);
        assert_eq!(
            p.residual.keyword_flags,
            [0.0, keywords.raw_get("Attack").unwrap()]
        );
        assert_eq!(
            p.residual.critical.flags.value,
            flags.raw_get::<f64>("Spell").unwrap()
        );
        assert_eq!(p.residual.untagged.len(), 5);
        assert_eq!(p.residual.critical.names, ["PoisonChance", "BleedChance"]);
        assert_eq!(p.damage.average_divisor, 2.0);
        assert_eq!(p.attack_speed.quality_divisor, 8.0);
        assert_eq!(p.range.quality_divisor, 10.0);
        assert_eq!(p.critical.quality_divisor, 4.0);
        assert_eq!(p.overrides.query_name, "WeaponData");
        assert_eq!(p.total_output, "TotalDPS");

        let attack: f64 = flags.raw_get("Attack").unwrap();
        flags.raw_set("Attack", attack + 1.0).unwrap();
        assert!(
            weapon::extract(&lua, auth.bodies["build_slot"], &auth.weapon, parser)
                .unwrap_err()
                .to_string()
                .contains("injected parser definition")
        );
    }

    #[test]
    fn weapon_damage_capture_and_declaration_changes_are_not_global_aliases() {
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(sources(), Primitives::capture)
                .unwrap();
        let f = class_function(&lua, "Item", "BuildModListForSlotNum").unwrap();
        let source = &sources()["src/Classes/Item.lua"];
        weapon::authenticate(&lua, &primitives, &f, source).unwrap();
        let captures = primitives.upvalues(&f).unwrap();
        let Value::Table(types) = &captures["dmgTypeList"] else {
            panic!("original damage list table")
        };
        let first: Value = types.raw_get(1).unwrap();
        let second: Value = types.raw_get(2).unwrap();
        types.raw_set(1, second.clone()).unwrap();
        assert!(weapon::authenticate(&lua, &primitives, &f, source).is_err());
        types.raw_set(1, first).unwrap();
        types.raw_set("extra", true).unwrap();
        assert!(weapon::authenticate(&lua, &primitives, &f, source).is_err());
        types.raw_set("extra", Value::Nil).unwrap();
        // A global with the same spelling is unrelated to the actual local capture.
        lua.globals()
            .raw_set("dmgTypeList", lua.create_table().unwrap())
            .unwrap();
        weapon::authenticate(&lua, &primitives, &f, source).unwrap();
        let altered = source.replacen(
            r#"local dmgTypeList = {"Physical", "Lightning", "Cold", "Fire", "Chaos"}"#,
            r#"local dmgTypeList = {"Lightning", "Physical", "Cold", "Fire", "Chaos"}"#,
            1,
        );
        assert_ne!(altered, *source);
        assert!(weapon::authenticate(&lua, &primitives, &f, &altered).is_err());
    }
    #[test]
    fn jewel_extraction_preserves_two_queries_and_value_valued_cluster_formula() {
        let shapes: BTreeMap<String, ItemSourceSpan> = serde_json::from_str(SHAPES).unwrap();
        let build = source_body(sources(), &shapes["build_mod_list"]).unwrap();
        let slot = source_body(sources(), &shapes["build_slot"]).unwrap();
        let lua = Lua::new();
        let p = jewel::extract(&lua, build, slot, "Jewel").unwrap();
        assert_eq!(p.output_field, "jewelData");
        assert_eq!(p.grand_spectrum.name_pattern, "Grand Spectrum");
        assert_eq!(p.grand_spectrum.modifier_name, "Multiplier:GrandSpectrum");
        assert_eq!(p.grand_spectrum.modifier_value, 1.0);
        assert_eq!(p.grand_spectrum.minion_name, "MinionModifier");
        assert_eq!(p.grand_spectrum.nested_mod_field, "mod");
        assert_eq!(p.functions.query_name, "JewelFunc");
        assert_eq!(p.overrides.query_name, "JewelData");
        assert_eq!(p.alternate_class_start.query_name, "AlternateClassStart");
        assert_eq!(p.from_nothing.guard_query_name, "FromNothingKeystones");
        assert_eq!(p.from_nothing.entries.query_name, "FromNothingKeystones");
        let c = &p.cluster;
        assert_eq!(c.notables.output_field, "clusterJewelNotables");
        assert_eq!(c.added_mods.output_field, "clusterJewelAddedMods");
        assert_eq!(c.skill_field, "clusterJewelSkill");
        assert_eq!(c.node_count_field, "clusterJewelNodeCount");
        assert_eq!(c.correction.node_count_below, 4.0);
        assert_eq!(
            c.correction.replacement_skill,
            "affliction_curse_effect_small"
        );
        assert_eq!(c.min_nodes_field, "minNodes");
        assert_eq!(c.max_nodes_field, "maxNodes");
        assert_eq!(c.skills_field, "skills");
        assert_eq!(c.validity.keystone_field, "clusterJewelKeystone");
        assert_eq!(
            c.validity.smalls_are_nothingness_field,
            "clusterJewelSmallsAreNothingness"
        );
        assert_eq!(
            c.validity.socket_count_override_field,
            "clusterJewelSocketCountOverride"
        );
        assert_eq!(
            c.validity.nothingness_count_field,
            "clusterJewelNothingnessCount"
        );
        assert!(jewel::extract(&lua, build, slot, "Other item type").is_err());
    }

    #[test]
    fn jewel_complete_body_guard_rejects_deduplicated_queries_or_changed_validity() {
        let shapes: BTreeMap<String, ItemSourceSpan> = serde_json::from_str(SHAPES).unwrap();
        for (old, new) in [
            (
                "if modList:List(nil, \"FromNothingKeystones\") then",
                "if true then",
            ),
            (
                "or (jewelData.clusterJewelSocketCountOverride and jewelData.clusterJewelNothingnessCount)",
                "or jewelData.clusterJewelSocketCountOverride",
            ),
        ] {
            let mut altered = sources().clone();
            let source = altered.get_mut("src/Classes/Item.lua").unwrap();
            assert!(source.contains(old));
            *source = source.replace(old, new);
            assert!(
                source_body(&altered, &shapes["build_slot"])
                    .unwrap_err()
                    .to_string()
                    .contains("changed complete")
            );
        }
    }

    #[test]
    fn jewel_clamp_authenticates_original_captured_minimum_and_maximum() {
        let mut altered = sources().clone();
        let source = altered.get_mut("src/Classes/Item.lua").unwrap();
        assert!(source.contains("local m_min = math.min"));
        *source = source.replace("local m_min = math.min", "local m_min = math.max");
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(&altered, Primitives::capture)
                .unwrap();
        assert!(
            authenticate(&lua, &primitives, &altered)
                .err()
                .unwrap()
                .to_string()
                .contains("primitive capture")
        );
    }

    #[test]
    fn jewel_new_mod_capture_must_match_direct_create_mod() {
        let mut altered = sources().clone();
        let source = altered.get_mut("src/Classes/ModStore.lua").unwrap();
        assert!(source.contains("local mod_createMod = modLib.createMod"));
        *source = source.replace(
            "local mod_createMod = modLib.createMod",
            "local mod_createMod = modLib.setSource",
        );
        let (lua, _, primitives) =
            crate::unique_requirements_extract::host_with_observer(&altered, Primitives::capture)
                .unwrap();
        assert!(
            authenticate(&lua, &primitives, &altered)
                .err()
                .unwrap()
                .to_string()
                .contains("helper binding new_mod.mod_createMod")
        );
    }
}
