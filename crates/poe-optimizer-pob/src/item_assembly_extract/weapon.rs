//! Pinned weapon parameter acquisition after complete method/helper authentication.
use super::*;

pub(super) struct Bindings {
    damage_types: Vec<String>,
    mod_flags: Table,
    keyword_flags: Table,
}
fn scalar(value: Value) -> Result<f64> {
    match value {
        Value::Integer(v) => Ok(v as f64),
        Value::Number(v) if v.is_finite() => Ok(v),
        _ => Err(error("weapon flag is not a finite original number")),
    }
}
pub(super) fn authenticate(
    lua: &Lua,
    primitives: &Primitives,
    function: &Function,
    source: &str,
) -> Result<Bindings> {
    let captures = primitives.upvalues(function)?;
    let Some(Value::Table(types)) = captures.get("dmgTypeList") else {
        return Err(error("missing actual weapon damage-list capture"));
    };
    if types.metatable().is_some() {
        return Err(error("weapon damage-list metatable"));
    }
    let mut declarations = source
        .lines()
        .filter_map(|line| line.strip_prefix("local dmgTypeList = "));
    let declaration = declarations
        .next()
        .ok_or_else(|| error("weapon damage-list declaration"))?;
    if declarations.next().is_some() || declaration.len() > 4096 {
        return Err(error(
            "ambiguous or excessive weapon damage-list declaration",
        ));
    }
    let contents = declaration
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(|| error("weapon damage-list declaration shape"))?;
    let call = format!("{contents})");
    let entries = args(&call)?;
    if entries.is_empty() || entries.len() > 32 {
        return Err(error("weapon damage-list count"));
    }
    let mut damage_types = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        if quoted_end(entry)? != entry.len() {
            return Err(error("nonliteral damage-list declaration"));
        }
        let name = string(lua, entry)?;
        let Value::String(actual) = types.raw_get::<Value>(i + 1)? else {
            return Err(error("weapon damage-list capture slot"));
        };
        if actual.as_bytes().as_ref() != name.as_bytes() {
            return Err(error(
                "weapon damage-list capture differs from source declaration",
            ));
        }
        damage_types.push(name);
    }
    let mut count = 0;
    for pair in types.pairs::<Value, Value>() {
        let (key, _) = pair?;
        count += 1;
        if count > damage_types.len() {
            return Err(error("extra weapon damage-list entry"));
        }
        let key = scalar(key)?;
        if key.fract() != 0.0 || key < 1.0 || key > damage_types.len() as f64 {
            return Err(error("weapon damage-list key"));
        }
    }
    if count != damage_types.len() {
        return Err(error("weapon damage-list inventory"));
    }
    let flag_table = |name: &str| -> Result<Table> {
        let table: Table = lua.globals().raw_get(name)?;
        if table.metatable().is_some() {
            return Err(error("weapon flag table metatable"));
        }
        if let Some(value) = captures.get(name)
            && !matches!(value, Value::Table(t) if t.to_pointer() == table.to_pointer())
        {
            return Err(error(
                "weapon flag capture differs from actual global binding",
            ));
        }
        Ok(table)
    };
    Ok(Bindings {
        damage_types,
        mod_flags: flag_table("ModFlag")?,
        keyword_flags: flag_table("KeywordFlag")?,
    })
}
impl Bindings {
    fn flag(&self, text: &str, parser: &ModifierParserData) -> Result<f64> {
        let text = text.trim();
        let (table, key, id) = if let Some(key) = text.strip_prefix("ModFlag.") {
            (&self.mod_flags, key, parser.policy.mod_flags)
        } else if let Some(key) = text.strip_prefix("KeywordFlag.") {
            (&self.keyword_flags, key, parser.policy.keyword_flags)
        } else {
            return text.parse::<f64>().map_err(error).and_then(|v| {
                if v.is_finite() {
                    Ok(v)
                } else {
                    Err(error("nonfinite weapon literal flag"))
                }
            });
        };
        if ident(key) != key || key.is_empty() {
            return Err(error("weapon flag expression"));
        }
        let actual = scalar(table.raw_get(key)?)?;
        let definition =
            id.0.checked_sub(1)
                .and_then(|i| parser.tables.get(i as usize))
                .ok_or_else(|| error("missing injected weapon flag table"))?;
        if !matches!(definition.fields.get(key), Some(ParserValue::Number(v)) if v.to_bits() == actual.to_bits())
        {
            return Err(error(
                "weapon live flag differs from injected parser definition",
            ));
        }
        Ok(actual)
    }
}
fn expr(lua: &Lua, text: &str, damage: Option<&str>) -> Result<String> {
    let mut result = String::new();
    for part in text.trim().split("..") {
        let part = part.trim();
        if part == "dmgType" {
            result.push_str(damage.ok_or_else(|| error("damage name outside channel"))?);
        } else {
            if quoted_end(part)? != part.len() {
                return Err(error("weapon name expression"));
            }
            result.push_str(&string(lua, part)?);
        }
    }
    if result.len() > 4096 {
        return Err(error("weapon name byte bound"));
    }
    Ok(result)
}
fn queries(
    lua: &Lua,
    text: &str,
    damage: Option<&str>,
    b: &Bindings,
    parser: &ModifierParserData,
) -> Result<Vec<ItemAssemblyLocalQuery>> {
    let mut result = Vec::new();
    let mut tail = text;
    while let Some((_, rest)) = tail.split_once("calcLocal(") {
        let a = args(rest)?;
        if a.len() != 4 || a[0] != "modList" || result.len() >= 16 {
            return Err(error("weapon query shape/count"));
        }
        result.push(ItemAssemblyLocalQuery {
            name: expr(lua, a[1], damage)?,
            mod_type: string(lua, a[2])?,
            flags: b.flag(a[3], parser)?,
        });
        // Every weapon calcLocal argument is a literal/name expression; no nested call.
        tail = rest
            .split_once(')')
            .ok_or_else(|| error("weapon query close"))?
            .1;
    }
    Ok(result)
}
fn one_query(
    lua: &Lua,
    text: &str,
    damage: Option<&str>,
    b: &Bindings,
    parser: &ModifierParserData,
) -> Result<ItemAssemblyLocalQuery> {
    let mut q = queries(lua, text, damage, b, parser)?;
    if q.len() != 1 {
        return Err(error("weapon single-query role"));
    }
    Ok(q.remove(0))
}
fn assignment<'a>(text: &'a str, lhs: &str) -> Result<&'a str> {
    let prefix = format!("{lhs} = ");
    let mut rows = text.lines().filter_map(|s| s.trim().strip_prefix(&prefix));
    let row = rows
        .next()
        .ok_or_else(|| error(format!("weapon assignment {lhs}")))?;
    if rows.next().is_some() {
        return Err(error("ambiguous weapon assignment"));
    }
    Ok(row)
}
fn member(text: &str, prefix: &str) -> Result<String> {
    let value = ident(after(text, prefix)?);
    if value.is_empty() {
        Err(error("weapon member"))
    } else {
        Ok(value.into())
    }
}
fn output<'a>(text: &'a str, key: &str) -> Result<(&'a str, &'a str)> {
    text.lines()
        .filter_map(|s| {
            s.trim()
                .strip_prefix("weaponData.")
                .and_then(|s| s.split_once(" = "))
        })
        .find(|(name, _)| *name == key)
        .ok_or_else(|| error("weapon output field"))
}
fn precision_of(rhs: &str) -> Result<u8> {
    let a = args(after(rhs, "round(")?)?;
    if a.len() != 2 {
        return Err(error("weapon rounded field arity"));
    }
    precision(a[1])
}
fn index_expr<'a>(rhs: &'a str, prefix: &str) -> Result<&'a str> {
    after(rhs, prefix)?
        .split_once(']')
        .map(|(s, _)| s)
        .ok_or_else(|| error("weapon field expression"))
}
fn indexed_output<'a>(text: &'a str, rhs: &str) -> Result<&'a str> {
    text.lines()
        .filter_map(|s| {
            let (key, value) = s.trim().strip_prefix("weaponData[")?.split_once("] = ")?;
            (value == rhs).then_some(key)
        })
        .next()
        .ok_or_else(|| error("weapon indexed output"))
}
fn name_groups(
    lua: &Lua,
    text: &str,
    b: &Bindings,
    parser: &ModifierParserData,
) -> Result<Vec<ItemAssemblyWeaponNameFlags>> {
    let mut groups = Vec::new();
    let mut tail = text;
    while let Some((names_text, after_flag)) = tail.split_once("mod.flags") {
        let mut names = Vec::new();
        let mut names_tail = names_text;
        while let Some((_, rest)) = names_tail.split_once("mod.name == ") {
            let end = quoted_end(rest)?;
            names.push(string(lua, &rest[..end])?);
            names_tail = &rest[end..];
        }
        if names.is_empty() || names.len() > 64 || groups.len() >= 64 {
            return Err(error("weapon residual name groups"));
        }
        let after_flag = after_flag.trim_start();
        let (operation, after_op) = if let Some(rest) = after_flag.strip_prefix("==") {
            (ItemAssemblyWeaponFlagComparison::Equal, rest)
        } else if let Some(rest) = after_flag.strip_prefix("~=") {
            (ItemAssemblyWeaponFlagComparison::NotEqual, rest)
        } else {
            return Err(error("weapon residual flag comparison"));
        };
        let after_op = after_op.trim_start();
        let end = after_op
            .find(|c: char| c.is_whitespace() || c == ')')
            .unwrap_or(after_op.len());
        groups.push(ItemAssemblyWeaponNameFlags {
            names,
            flags: ItemAssemblyWeaponFlagTest {
                operation,
                value: b.flag(&after_op[..end], parser)?,
            },
        });
        tail = &after_op[end..];
    }
    Ok(groups)
}
fn tag(lua: &Lua, text: &str) -> Result<(String, u16, String, String)> {
    Ok((
        text_after(lua, text, "type = ")?,
        count(after(text, "slotNum == ")?)?,
        text_after(lua, text, " and ")?,
        text_after(lua, text, " or ")?,
    ))
}
fn residual(
    lua: &Lua,
    text: &str,
    b: &Bindings,
    parser: &ModifierParserData,
) -> Result<ItemAssemblyWeaponResidualPolicy> {
    let predicates = after(text, "for _, mod in ipairs(modList) do")?;
    let (first, keyword) = predicates
        .split_once(") and (mod.keywordFlags")
        .ok_or_else(|| error("weapon residual keyword boundary"))?;
    let untagged = name_groups(lua, first, b, parser)?;
    let keyword = format!("mod.keywordFlags{keyword}");
    let keyword = keyword
        .split_once(") and not mod[1] then")
        .ok_or_else(|| error("weapon keyword guard"))?
        .0;
    let mut flags = Vec::new();
    let mut tail = keyword;
    while let Some((_, rest)) = tail.split_once("mod.keywordFlags == ") {
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ')')
            .unwrap_or(rest.len());
        flags.push(b.flag(&rest[..end], parser)?);
        tail = &rest[end..];
    }
    let keyword_flags = flags
        .try_into()
        .map_err(|_| error("weapon keyword disjunction count"))?;
    let second = after(predicates, "elseif ")?;
    let second = second
        .split_once(" then")
        .ok_or_else(|| error("weapon critical-tag guard"))?
        .0;
    let mut critical = name_groups(lua, second, b, parser)?;
    if critical.len() != 1 {
        return Err(error("weapon critical predicate groups"));
    }
    let condition = text_after(lua, second, "mod[1].type == ")?;
    let critical_condition = text_after(lua, second, "mod[1].var == ")?;
    let first_tag = tag(lua, after(predicates, "mod[1] = ")?)?;
    let insert = args(after(predicates, "t_insert(")?)?;
    if insert.len() != 2
        || insert[0] != "mod"
        || tag(lua, insert[1])? != first_tag
        || condition != first_tag.0
    {
        return Err(error("weapon hand tag constructions disagree"));
    }
    Ok(ItemAssemblyWeaponResidualPolicy {
        untagged,
        keyword_flags,
        critical: critical.remove(0),
        condition_tag_type: first_tag.0,
        critical_condition,
        primary_slot: first_tag.1,
        primary_condition: first_tag.2,
        other_condition: first_tag.3,
    })
}
pub(super) fn extract(
    lua: &Lua,
    slot: &str,
    b: &Bindings,
    parser: &ModifierParserData,
) -> Result<ItemAssemblyWeaponPolicy> {
    let body = after(slot, "if self.base.weapon then")?
        .split_once("elseif self.base.armour then")
        .ok_or_else(|| error("complete weapon branch"))?
        .0;
    let (speed_key, speed) = output(body, "AttackSpeedInc")?;
    let speed_queries = queries(lua, speed, None, b, parser)?;
    let (rate_key, rate) = output(body, "AttackRate")?;
    let (bonus_key, bonus) = output(body, "rangeBonus")?;
    let range_queries = queries(lua, bonus, None, b, parser)?;
    let (range_key, range) = output(body, "range")?;
    let (reload_inc_key, reload_inc) = output(body, "ReloadSpeedInc")?;
    let (reload_key, reload) = output(body, "ReloadTime")?;
    let (crit_key, crit) = output(body, "CritChance")?;
    let crit_queries = queries(lua, crit, None, b, parser)?;
    if speed_queries.len() != 2 || range_queries.len() != 3 || crit_queries.len() != 3 {
        return Err(error("weapon fixed query role inventory"));
    }
    let physical_name = text_after(lua, body, "if dmgType == ")?;
    let elemental = after(body, "elseif dmgType ~= ")?;
    let first_excluded = string(lua, elemental)?;
    let second_excluded = text_after(lua, elemental, "and dmgType ~= ")?;
    let aggregate_guard = after(body, "if dmgType ~= ")?;
    if first_excluded != physical_name
        || string(lua, aggregate_guard)? != physical_name
        || text_after(lua, aggregate_guard, "and dmgType ~= ")? != second_excluded
    {
        return Err(error("weapon damage classifications disagree"));
    }
    let min = assignment(body, "local min")?;
    let max = assignment(body, "local max")?;
    let local_increased = assignment(body, "local localInc")?;
    let mut channels = Vec::new();
    for name in &b.damage_types {
        let kind = if name == &physical_name {
            ItemAssemblyWeaponDamageKind::Physical
        } else if name == &second_excluded {
            ItemAssemblyWeaponDamageKind::Unscaled
        } else {
            ItemAssemblyWeaponDamageKind::Elemental
        };
        let bound = |rhs: &str, output_rhs: &str| -> Result<ItemAssemblyWeaponDamageBound> {
            Ok(ItemAssemblyWeaponDamageBound {
                base_field: expr(lua, index_expr(rhs, "self.base.weapon[")?, Some(name))?,
                output: expr(lua, indexed_output(body, output_rhs)?, Some(name))?,
                query: one_query(lua, rhs, Some(name), b, parser)?,
            })
        };
        channels.push(ItemAssemblyWeaponDamageChannel {
            name: name.clone(),
            kind,
            minimum: bound(min, "min")?,
            maximum: bound(max, "max")?,
            increased: if kind == ItemAssemblyWeaponDamageKind::Elemental {
                Some(one_query(lua, local_increased, Some(name), b, parser)?)
            } else {
                None
            },
            dps_output: expr(lua, indexed_output(body, "dps")?, Some(name))?,
        });
    }
    let base_default = num_after(min, " or ")?;
    let positive_threshold = num_after(body, "if min > ")?;
    if num_after(max, " or ")? != base_default
        || num_after(body, " and max > ")? != positive_threshold
    {
        return Err(error("weapon paired bounds defaults/thresholds disagree"));
    }
    let (elemental_output, elemental_rhs) = output(body, "ElementalDPS")?;
    let quality = body
        .lines()
        .map(str::trim)
        .find(|s| s.starts_with("if calcLocal("))
        .ok_or_else(|| error("weapon alternate quality guard"))?;
    let override_tail = after(body, "for _, value in ipairs(modList:List(nil, ")?;
    let override_store = after(override_tail, "[value.")?;
    let (type_output, type_rhs) = output(body, "type")?;
    let (name_output, name_rhs) = output(body, "name")?;
    let (total_output, total_initial) = output(body, "TotalDPS")?;
    let total_loop = after(body, "weaponData.TotalDPS = weaponData.TotalDPS + ")?;
    for channel in &channels {
        if expr(
            lua,
            index_expr(total_loop, "weaponData[")?,
            Some(&channel.name),
        )? != channel.dps_output
        {
            return Err(error("weapon final DPS field differs from produced field"));
        }
    }
    Ok(ItemAssemblyWeaponPolicy {
        base_field: member(slot, "if self.base.")?,
        output_field: member(body, "self.")?,
        type_output: type_output.into(),
        type_base_field: member(type_rhs, "self.base.")?,
        name_output: name_output.into(),
        name_item_field: member(name_rhs, "self.")?,
        attack_speed: ItemAssemblyWeaponAttackSpeed {
            output: speed_key.into(),
            query: speed_queries[0].clone(),
            alternate: speed_queries[1].clone(),
            quality_divisor: num_after(speed, "self.quality / ")?,
        },
        attack_rate: ItemAssemblyWeaponScaledField {
            base_field: member(rate, "self.base.weapon.")?,
            output: rate_key.into(),
            round_places: precision_of(rate)?,
        },
        range: ItemAssemblyWeaponRange {
            base_field: member(range, "self.base.weapon.")?,
            output: range_key.into(),
            bonus_output: bonus_key.into(),
            flat: range_queries[0].clone(),
            metres: range_queries[1].clone(),
            metre_multiplier: num_after(bonus, " + ")?,
            alternate: range_queries[2].clone(),
            quality_divisor: num_after(bonus, "self.quality / ")?,
        },
        reload: ItemAssemblyWeaponReload {
            base_field: member(reload, "self.base.weapon.")?,
            increased_output: reload_inc_key.into(),
            output: reload_key.into(),
            query: one_query(lua, reload_inc, None, b, parser)?,
            round_places: precision_of(reload)?,
        },
        damage: ItemAssemblyWeaponDamage {
            channels,
            elemental_increased: one_query(
                lua,
                assignment(body, "local LocalIncEle")?,
                None,
                b,
                parser,
            )?,
            physical_increased: one_query(
                lua,
                assignment(body, "local physInc")?,
                None,
                b,
                parser,
            )?,
            alternate_quality: one_query(lua, quality, None, b, parser)?,
            quality_threshold: num_after(quality, " > ")?,
            suppressed_quality: number(assignment(body, "qualityScalar")?)?,
            base_default,
            positive_threshold,
            average_divisor: num_after(assignment(body, "local dps")?, " / ")?,
            // All four damage round calls use the authenticated helper's no-decimal branch.
            round_places: 0,
            elemental_output: elemental_output.into(),
            elemental_default: num_after(elemental_rhs, " or ")?,
        },
        critical: ItemAssemblyWeaponCritical {
            base_field: member(crit, "self.base.weapon.")?,
            output: crit_key.into(),
            base: crit_queries[0].clone(),
            increased: crit_queries[1].clone(),
            alternate: crit_queries[2].clone(),
            quality_divisor: num_after(crit, "self.quality / ")?,
            round_places: precision_of(crit)?,
        },
        overrides: ItemAssemblyOverridePolicy {
            query_name: string(lua, override_tail)?,
            key_field: ident(override_store).into(),
            value_field: member(override_store, " = value.")?,
        },
        residual: residual(lua, body, b, parser)?,
        percent_divisor: num_after(rate, " / ")?,
        fraction_base: num_after(rate, " * (")?,
        total_output: total_output.into(),
        total_initial: number(total_initial)?,
        total_default: num_after(total_loop, " or ")?,
    })
}
