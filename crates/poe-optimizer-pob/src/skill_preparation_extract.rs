//! Authenticated complete skill-loading definitions from original Data construction.
//! Source algorithms are retained by path/span/hash; game constants come from Lua.
use crate::game_data::{GameDataExtractionError, error, hash, section};
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_data::{
    skill_identities::{IdentitySourceSpan, SkillIdentityData},
    skill_preparation::*,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
type Result<T> = std::result::Result<T, GameDataExtractionError>;
const DATA: &str = "src/Modules/Data.lua";
const COMMON: &str = "src/Modules/Common.lua";
const TOOLS: &str = "src/Modules/CalcTools.lua";
const TAB: &str = "src/Classes/SkillsTab.lua";
/// Actual runtime observations belong in evidence, not reproducible game semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPreparationExtractionEvidence {
    pub gem_setup_order: Vec<String>,
    pub gem_search_order: Vec<String>,
    pub table_gem_for_skill: BTreeMap<String, String>,
    pub external_variants: BTreeMap<String, ObservedGemVariants>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedGemVariants {
    pub order: Vec<String>,
    pub winners: BTreeMap<String, String>,
}
impl SkillPreparationExtractionEvidence {
    pub(crate) fn validate(&self, identities: &SkillIdentityData) -> Result<()> {
        let gems: BTreeMap<_, _> = identities
            .gems
            .iter()
            .map(|g| (g.key.as_str(), g))
            .collect();
        let expected: std::collections::BTreeSet<_> = gems.keys().copied().collect();
        for order in [&self.gem_setup_order, &self.gem_search_order] {
            if order.len() != gems.len()
                || order
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>()
                    != expected
            {
                return Err(error(
                    "observed original gem traversal is not a complete permutation",
                ));
            }
        }
        let mut owners = BTreeMap::new();
        let mut variants: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for key in &self.gem_setup_order {
            let gem = gems[key.as_str()];
            owners.insert(gem.primary_effect_id.clone(), key.clone());
            variants
                .entry(gem.game_id.clone())
                .or_default()
                .insert(gem.variant_id.clone(), key.clone());
        }
        if self.table_gem_for_skill != owners || self.external_variants.len() != variants.len() {
            return Err(error(
                "observed construction winners disagree with traversal",
            ));
        }
        for (id, expected) in variants {
            let actual = self
                .external_variants
                .get(&id)
                .ok_or_else(|| error("missing observed external ID"))?;
            if actual.winners != expected
                || actual.order.len() != expected.len()
                || actual
                    .order
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    != expected.keys().collect()
            {
                return Err(error("observed variant traversal/winners disagree"));
            }
        }
        Ok(())
    }
}
fn source<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    sources
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| error(format!("missing skill preparation source {path}")))
}
fn operation(
    sources: &BTreeMap<String, String>,
    path: &str,
    begin: &str,
    end: &str,
) -> Result<IdentitySourceSpan> {
    let text = source(sources, path)?;
    let chunk = section(text, begin, end)?;
    let at = chunk.as_ptr() as usize - text.as_ptr() as usize;
    let line = text[..at].bytes().filter(|b| *b == b'\n').count() + 1;
    let end_line = line + chunk.trim_end().bytes().filter(|b| *b == b'\n').count();
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
fn plain_table(table: &Table, role: &str) -> Result<()> {
    if table.metatable().is_some() {
        return Err(error(format!(
            "skill preparation {role} has an unsupported metatable; raw definitions cannot preserve its lookup semantics"
        )));
    }
    Ok(())
}
fn number(value: Value) -> Result<f64> {
    match value {
        Value::Integer(v) => Ok(v as f64),
        Value::Number(v) if v.is_finite() && v.abs() <= 1e12 => Ok(if v == 0.0 { 0.0 } else { v }),
        _ => Err(error(
            "skill preparation expected bounded finite numeric value",
        )),
    }
}
fn optional_number(table: &Table, key: &str) -> Result<Option<f64>> {
    match table.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        value => number(value).map(Some),
    }
}
fn optional_bool(table: &Table, key: &str) -> Result<Option<bool>> {
    match table.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Boolean(value) => Ok(Some(value)),
        _ => Err(error(format!(
            "skill preparation field {key} expected boolean/nil"
        ))),
    }
}
fn optional_text(table: &Table, key: &str) -> Result<Option<String>> {
    match table.raw_get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::String(value) => Ok(Some(value.to_str()?.to_owned())),
        _ => Err(error(format!(
            "skill preparation field {key} expected string/nil"
        ))),
    }
}
fn required_text(table: &Table, key: &str) -> Result<String> {
    optional_text(table, key)?
        .ok_or_else(|| error(format!("missing skill preparation field {key}")))
}
// Strict known formula shape; changed operators or references require source review.
fn numeric_recipe(text: &str, shape: &str) -> Result<Vec<f64>> {
    let mut rest = text;
    let mut parts = shape.split("{}");
    let first = parts.next().ok_or_else(|| error("empty formula shape"))?;
    rest = rest
        .strip_prefix(first)
        .ok_or_else(|| error("changed gem requirement formula"))?;
    let mut values = Vec::new();
    for part in parts {
        let end = if part.is_empty() {
            rest.len()
        } else {
            rest.find(part)
                .ok_or_else(|| error("changed gem requirement operator"))?
        };
        let value: f64 = rest[..end].parse().map_err(error)?;
        if !value.is_finite() || value.abs() > 1e12 {
            return Err(error("unbounded formula coefficient"));
        }
        values.push(value);
        rest = &rest[end + part.len()..];
    }
    if !rest.is_empty() {
        return Err(error("unexpected formula tail"));
    }
    Ok(values)
}
fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}
fn assigned_line<'a>(text: &'a str, begin: &str) -> Result<&'a str> {
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with(begin))
        .collect();
    if lines.len() != 1 {
        return Err(error(format!(
            "missing or ambiguous source assignment {begin}"
        )));
    }
    Ok(lines[0])
}
fn formula(sources: &BTreeMap<String, String>) -> Result<GemRequirementFormula> {
    let text = section(
        source(sources, TOOLS)?,
        "function calcLib.getGemStatRequirement(level, multi, isSupport)",
        "-- Build table of stats",
    )?;
    let values = numeric_recipe(
        &compact(assigned_line(text, "local req =")?),
        "localreq=round(({}+(level-{})*{})*(multi/{})^{})+{}",
    )?;
    let minimum = numeric_recipe(
        &compact(assigned_line(text, "return req <")?),
        "returnreq<{}and0orreq",
    )?[0];
    let round = section(
        source(sources, COMMON)?,
        "function round(val, dec)",
        "\nend",
    )?;
    let line = round
        .lines()
        .find(|line| line.contains("return m_floor(val +"))
        .ok_or_else(|| error("missing source round bias"))?;
    let round_bias = numeric_recipe(&compact(line), "returnm_floor(val+{})")?[0];
    Ok(GemRequirementFormula {
        base: values[0],
        level_offset: values[1],
        level_multiplier: values[2],
        attribute_divisor: values[3],
        attribute_exponent: values[4],
        round_bias,
        result_offset: values[5],
        minimum_requirement: minimum,
    })
}
fn source_string_assignment(lua: &Lua, text: &str, prefix: &str) -> Result<String> {
    let line = assigned_line(text, prefix)?;
    let (_, value) = line
        .split_once('=')
        .ok_or_else(|| error("missing source assignment"))?;
    Ok(lua.load(format!("return {value}")).eval()?)
}
fn choices(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    let lists = section(
        source(sources, TAB)?,
        "local defaultGemLevelList =",
        "---@class SkillsTab",
    )?;
    let result: Table = lua
        .load(format!(
            "{lists}\nreturn {{defaultGemLevelList,showSupportGemTypeList,sortGemTypeList}}"
        ))
        .eval()?;
    let get = |index: usize, key: &str| -> Result<Vec<String>> {
        let rows: Table = result.raw_get(index)?;
        if rows.raw_len() > 256 {
            return Err(error("skill dropdown limit"));
        }
        rows.sequence_values::<Table>()
            .map(|row| required_text(&row?, key))
            .collect()
    };
    Ok((get(1, "gemLevel")?, get(2, "show")?, get(3, "type")?))
}
pub(crate) fn extract(
    sources: &BTreeMap<String, String>,
    identities: &SkillIdentityData,
) -> Result<(SkillPreparationData, SkillPreparationExtractionEvidence)> {
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
                    "skill preparation instruction bound".into(),
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
    let authenticated = Arc::new(sources.clone());
    lua.globals().set(
        "LoadModule",
        lua.create_function(move |lua, name: String| {
            let path = format!("src/{name}.lua");
            let text = authenticated.get(&path).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("unapproved skill preparation dependency {path}"))
            })?;
            lua.load(text).set_name(format!("@{path}")).eval::<Value>()
        })?,
    )?;
    // Observe original traversal without replacing its iterator, keys or values.
    lua.load(
        r#"
        local originalPairs = pairs
        skillPreparationTraversals = {}
        pairs = function(t)
            if data and data.gems and rawequal(t, data.gems) then
                local trace = {}
                table.insert(skillPreparationTraversals, trace)
                local iterator, state, initial = originalPairs(t)
                return function(state, previous)
                    local key, value = iterator(state, previous)
                    if key ~= nil then trace[#trace + 1] = key end
                    return key, value
                end, state, initial
            end
            return originalPairs(t)
        end
    "#,
    )
    .exec()?;
    let original = source(sources, DATA)?;
    let prefix = section(
        original,
        "LoadModule(\"Data/Global\")",
        "-----------------\n-- Common Data",
    )?;
    let construction = section(original, "-- Load skills\n", "-- Load minions\n")?;
    let data: Table = lua
        .load(format!("{prefix}\n{construction}\nreturn data"))
        .set_name(format!("@{DATA}"))
        .eval()?;
    plain_table(&data, "data")?;
    let traversals: Table = lua.globals().get("skillPreparationTraversals")?;
    let setup: Table = traversals.raw_get(1)?;
    let gem_setup_order = setup
        .sequence_values::<String>()
        .collect::<mlua::Result<Vec<_>>>()?;
    let gem_table: Table = data.get("gems")?;
    plain_table(&gem_table, "gems")?;
    let gem_search_order = gem_table
        .clone()
        .pairs::<String, Value>()
        .map(|p| p.map(|p| p.0))
        .collect::<mlua::Result<Vec<_>>>()?;
    let mut gems = Vec::new();
    for identity in &identities.gems {
        let table: Table = gem_table.raw_get(identity.key.as_str())?;
        plain_table(&table, "gem definition")?;
        gems.push(GemPreparationDefinition {
            key: identity.key.clone(),
            natural_max_level: number(table.raw_get("naturalMaxLevel")?)?,
            req_str: optional_number(&table, "reqStr")?,
            req_dex: optional_number(&table, "reqDex")?,
            req_int: optional_number(&table, "reqInt")?,
        });
    }
    let skills: Table = data.get("skills")?;
    plain_table(&skills, "skills")?;
    let mut effect_pointers = BTreeMap::new();
    let mut row_pointers: BTreeMap<usize, String> = BTreeMap::new();
    let mut effects = Vec::new();
    let mut total_rows = 0usize;
    for identity in &identities.skills {
        let table: Table = skills.raw_get(identity.id.as_str())?;
        plain_table(&table, "effect definition")?;
        if effect_pointers
            .insert(table.to_pointer() as usize, identity.id.clone())
            .is_some()
        {
            return Err(error(
                "aliased skill object IDs need explicit identity model",
            ));
        }
        let levels: Table = table.raw_get("levels")?;
        plain_table(&levels, "effect levels")?;
        let levels_length = levels.raw_len();
        let next_level_key = levels
            .clone()
            .pairs::<Value, Value>()
            .next()
            .transpose()?
            .map(|row| number(row.0))
            .transpose()?;
        let mut original_rows = Vec::new();
        for row in levels.pairs::<Value, Table>() {
            let (key, row) = row?;
            total_rows += 1;
            if total_rows > 500_000 {
                return Err(error("skill preparation total level rows exceed bound"));
            }
            original_rows.push((number(key)?, row));
        }
        original_rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut rows = Vec::new();
        for (key, row) in original_rows {
            plain_table(&row, "level row")?;
            let row_id = row_pointers
                .entry(row.to_pointer() as usize)
                .or_insert_with(|| format!("{}/level/{key}", identity.id))
                .clone();
            let cost = match row.raw_get::<Value>("cost")? {
                Value::Nil => None,
                Value::Table(cost) => {
                    plain_table(&cost, "level cost")?;
                    let mut result = BTreeMap::new();
                    for pair in cost.pairs::<String, Value>() {
                        let (name, value) = pair?;
                        if result.len() >= 256 {
                            return Err(error("skill level cost entry bound"));
                        }
                        result.insert(name, number(value)?);
                    }
                    Some(result)
                }
                _ => return Err(error("skill level cost is not table/nil")),
            };
            rows.push(SkillLevelDefinition {
                key,
                row_id,
                level_requirement: optional_number(&row, "levelRequirement")?,
                cost,
            });
        }
        effects.push(SkillPreparationDefinition {
            id: identity.id.clone(),
            name: required_text(&table, "name")?,
            color: optional_number(&table, "color")?,
            support: optional_bool(&table, "support")?,
            hide_from_sidebar: optional_bool(&table, "hideFromSideBar")?,
            plus_version_of: optional_text(&table, "plusVersionOf")?,
            levels_length,
            next_level_key,
            levels: rows,
        });
    }
    let lookup: Table = data.get("gemForSkill")?;
    plain_table(&lookup, "gemForSkill")?;
    let mut table_gem_for_skill = BTreeMap::new();
    let mut string_gem_for_skill = BTreeMap::new();
    for pair in lookup.pairs::<Value, String>() {
        let (key, value) = pair?;
        match key {
            Value::Table(t) => {
                let id = effect_pointers
                    .get(&(t.to_pointer() as usize))
                    .ok_or_else(|| error("unidentified gemForSkill effect object"))?;
                table_gem_for_skill.insert(id.clone(), value);
            }
            Value::String(t) => {
                string_gem_for_skill.insert(t.to_str()?.to_owned(), value);
            }
            _ => return Err(error("gemForSkill unsupported key type")),
        }
    }
    let external: Table = data.get("gemsByGameId")?;
    plain_table(&external, "gemsByGameId")?;
    let mut observed_external = BTreeMap::new();
    for pair in external.pairs::<String, Table>() {
        let (game_id, rows) = pair?;
        plain_table(&rows, "external variant map")?;
        let mut variants = BTreeMap::new();
        let mut variant_order = Vec::new();
        for row in rows.pairs::<String, Table>() {
            let (key, gem) = row?;
            plain_table(&gem, "external gem definition")?;
            variant_order.push(key.clone());
            variants.insert(key, required_text(&gem, "id")?);
        }
        observed_external.insert(
            game_id,
            ObservedGemVariants {
                order: variant_order,
                winners: variants,
            },
        );
    }
    let evidence = SkillPreparationExtractionEvidence {
        gem_setup_order,
        gem_search_order,
        table_gem_for_skill,
        external_variants: observed_external,
    };
    evidence.validate(identities)?;
    // Hash traversal is runtime-dependent. Prove unique bindings or retain all
    // candidates explicitly; sorted storage must never choose a source winner.
    let canonical_gem_order: Vec<_> = identities
        .gems
        .iter()
        .map(|g| g.key.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let by_key: BTreeMap<_, _> = identities
        .gems
        .iter()
        .map(|g| (g.key.as_str(), g))
        .collect();
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut variant_candidates: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for key in &canonical_gem_order {
        let gem = by_key[key.as_str()];
        owners
            .entry(gem.primary_effect_id.clone())
            .or_default()
            .push(key.clone());
        variant_candidates
            .entry(gem.game_id.clone())
            .or_default()
            .entry(gem.variant_id.clone())
            .or_default()
            .push(key.clone());
    }
    let (table_gem_for_skill, ambiguous_table_gem_for_skill) = split_candidates(owners);
    let external_variants = variant_candidates
        .into_iter()
        .map(|(game_id, candidates)| {
            let canonical_variant_order = candidates.keys().cloned().collect();
            let (variants, ambiguous_variants) = split_candidates(candidates);
            ExternalGemVariants {
                game_id,
                canonical_variant_order,
                variants,
                ambiguous_variants,
            }
        })
        .collect();
    let colors: Table = lua.globals().get("colorCodes")?;
    plain_table(&colors, "colors")?;
    let tab = source(sources, TAB)?;
    let process = section(
        tab,
        "function SkillsTabClass:ProcessSocketGroup(socketGroup)",
        "\nfunction SkillsTabClass:",
    )?;
    let constructor = section(
        tab,
        "function SkillsTabClass:SkillsTab(build)",
        "\nfunction SkillsTabClass:",
    )?;
    let defaults = section(
        constructor,
        "function SkillsTabClass:SkillsTab(build)",
        "-- Set selector",
    )?;
    let (default_gem_level_options, support_type_options, sort_field_options) =
        choices(&lua, sources)?;
    let load = section(
        tab,
        "function SkillsTabClass:Load(xml, fileName)",
        "\nfunction SkillsTabClass:",
    )?;
    let quality = numeric_recipe(
        &compact(assigned_line(load, "self.defaultGemQuality =")?),
        "self.defaultGemQuality=m_max(m_min(tonumber(xml.attrib.defaultGemQuality)or{},{}),{})",
    )?;
    let initial_new_gem_level = numeric_recipe(
        &compact(assigned_line(process, "local prevDefaultLevel =")?),
        "localprevDefaultLevel=gemInstance.gemDataandgemInstance.gemData.naturalMaxLevelor(gemInstance.newand{})",
    )?[0];
    let validate_level = section(
        source(sources, TOOLS)?,
        "function calcLib.validateGemLevel(gemInstance)",
        "\nlocal typeExpressionStack",
    )?;
    let minimum_gem_level = numeric_recipe(
        &compact(assigned_line(validate_level, "gemInstance.level = m_max")?),
        "gemInstance.level=m_max({},gemInstance.level)",
    )?[0];
    let mut operations = BTreeMap::new();
    for (name, path, begin, end) in [
        (
            "data_prefix",
            DATA,
            "LoadModule(\"Data/Global\")",
            "-----------------\n-- Common Data",
        ),
        (
            "skill_and_gem_construction",
            DATA,
            "-- Load skills\n",
            "-- Load minions\n",
        ),
        (
            "skills_load",
            TAB,
            "function SkillsTabClass:Load(xml, fileName)",
            "\nfunction SkillsTabClass:",
        ),
        (
            "load_skill",
            TAB,
            "function SkillsTabClass:LoadSkill(node, skillSetId)",
            "\nfunction SkillsTabClass:",
        ),
        (
            "find_skill_gem",
            TAB,
            "function SkillsTabClass:FindSkillGem(nameSpec)",
            "\nfunction SkillsTabClass:",
        ),
        (
            "process_gem_level",
            TAB,
            "function SkillsTabClass:ProcessGemLevel(gemData)",
            "\nfunction SkillsTabClass:",
        ),
        (
            "process_socket_group",
            TAB,
            "function SkillsTabClass:ProcessSocketGroup(socketGroup)",
            "\nfunction SkillsTabClass:",
        ),
        (
            "validate_gem_level",
            TOOLS,
            "function calcLib.validateGemLevel(gemInstance)",
            "\nlocal typeExpressionStack",
        ),
        (
            "stat_requirement",
            TOOLS,
            "function calcLib.getGemStatRequirement(level, multi, isSupport)",
            "-- Build table of stats",
        ),
        ("round", COMMON, "function round(val, dec)", "\nend"),
        (
            "sanitise",
            COMMON,
            "function sanitiseText(text)",
            "-- Convert int to 4 bytes string",
        ),
        (
            "skill_constructor",
            TAB,
            "function SkillsTabClass:SkillsTab(build)",
            "\nfunction SkillsTabClass:",
        ),
        (
            "skill_options",
            TAB,
            "local defaultGemLevelList =",
            "---@class SkillsTab",
        ),
    ] {
        operations.insert(name.into(), operation(sources, path, begin, end)?);
    }
    let mut files = identities.source.files.clone();
    for path in [TOOLS, TAB, COMMON] {
        files.insert(path.into(), hash(source(sources, path)?.as_bytes()));
    }
    let result = SkillPreparationData {
        schema_version: SKILL_PREPARATION_SCHEMA_VERSION,
        source: SkillPreparationSource {
            upstream_revision: crate::source::UPSTREAM_REVISION.into(),
            files,
            operations,
            canonical_gem_order,
            iteration_policy: "canonical_nonsemantic; ambiguous_source_pairs_winners_unresolved"
                .into(),
        },
        requirement: formula(sources)?,
        colors: SkillPreparationColors {
            unresolved: source_string_assignment(&lua, process, "gemInstance.color = \"")?,
            normal: required_text(&colors, "NORMAL")?,
            strength: required_text(&colors, "STRENGTH")?,
            dexterity: required_text(&colors, "DEXTERITY")?,
            intelligence: required_text(&colors, "INTELLIGENCE")?,
        },
        default_gem_level_options,
        support_type_options,
        sort_field_options,
        default_gem_quality: quality[0],
        maximum_gem_quality: quality[1],
        minimum_gem_quality: quality[2],
        initial_new_gem_level,
        minimum_gem_level,
        default_gem_level: source_string_assignment(&lua, defaults, "self.defaultGemLevel =")?,
        default_support_type: source_string_assignment(
            &lua,
            defaults,
            "self.showSupportGemTypes =",
        )?,
        default_sort_field: source_string_assignment(&lua, defaults, "self.sortGemsByDPSField =")?,
        gems,
        effects,
        table_gem_for_skill,
        ambiguous_table_gem_for_skill,
        string_gem_for_skill,
        external_variants,
    };
    result.validate(identities).map_err(error)?;
    Ok((result, evidence))
}
fn split_candidates(
    candidates: BTreeMap<String, Vec<String>>,
) -> (BTreeMap<String, String>, BTreeMap<String, Vec<String>>) {
    let mut unique = BTreeMap::new();
    let mut ambiguous = BTreeMap::new();
    for (key, values) in candidates {
        if values.len() == 1 {
            unique.insert(key, values[0].clone());
        } else {
            ambiguous.insert(key, values);
        }
    }
    (unique, ambiguous)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (BTreeMap<String, String>, SkillIdentityData) {
        let package: serde_json::Value =
            serde_json::from_slice(poe_optimizer_data::game_data::bundled_package_bytes()).unwrap();
        let identities: SkillIdentityData =
            serde_json::from_value(package["skill_identities"].clone()).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let mut paths: Vec<_> = identities.source.files.keys().cloned().collect();
        paths.extend([TOOLS, TAB, COMMON].map(str::to_owned));
        let sources = paths
            .into_iter()
            .map(|path| {
                let text = std::fs::read_to_string(root.join(&path))
                    .unwrap()
                    .replace("\r\n", "\n");
                (path, text)
            })
            .collect();
        (sources, identities)
    }
    #[test]
    fn source_metatables_require_explicit_semantics_instead_of_silent_raw_lookup() {
        let lua = Lua::new();
        let plain = lua.create_table().unwrap();
        plain_table(&plain, "test").unwrap();
        let metatable = lua.create_table().unwrap();
        let fallback = lua.create_table().unwrap();
        fallback.set("levelRequirement", 99).unwrap();
        metatable.set("__index", fallback).unwrap();
        plain.set_metatable(Some(metatable)).unwrap();
        assert_eq!(plain.get::<u32>("levelRequirement").unwrap(), 99);
        assert!(matches!(
            plain.raw_get::<Value>("levelRequirement").unwrap(),
            Value::Nil
        ));
        assert!(
            plain_table(&plain, "level row")
                .unwrap_err()
                .0
                .contains("unsupported metatable")
        );
    }
    #[test]
    fn complete_original_constructions_preserve_values_and_report_iteration_variation() {
        let (sources, identities) = inputs();
        let (first, first_evidence) = extract(&sources, &identities).unwrap();
        let (second, second_evidence) = extract(&sources, &identities).unwrap();
        let reviewed: serde_json::Value =
            serde_json::from_slice(poe_optimizer_data::game_data::bundled_package_bytes()).unwrap();
        let reviewed: SkillPreparationData =
            serde_json::from_value(reviewed["skill_preparation"].clone()).unwrap();
        assert_eq!(
            first, reviewed,
            "fresh original definitions differ from the reviewed package section"
        );
        assert_eq!(
            first, second,
            "complete canonical package section must reproduce"
        );
        assert_eq!(first.gems, second.gems);
        assert_eq!(first.effects, second.effects);
        assert_eq!(first.requirement, second.requirement);
        eprintln!(
            "skills={} gems={} rows={} setup_order_equal={} search_order_equal={} table_winners_equal={} variants_equal={}",
            first.effects.len(),
            first.gems.len(),
            first.effects.iter().map(|s| s.levels.len()).sum::<usize>(),
            first_evidence.gem_setup_order == second_evidence.gem_setup_order,
            first_evidence.gem_search_order == second_evidence.gem_search_order,
            first.table_gem_for_skill == second.table_gem_for_skill,
            first.external_variants == second.external_variants
        );
        let collision_count = identities
            .gems
            .iter()
            .fold(BTreeMap::<&str, usize>::new(), |mut m, g| {
                *m.entry(&g.primary_effect_id).or_default() += 1;
                m
            })
            .values()
            .filter(|&&count| count > 1)
            .count();
        eprintln!(
            "gemForSkill colliding effect owners={collision_count}; multi_variant_game_ids={}",
            first
                .external_variants
                .iter()
                .filter(|e| e.variants.len() > 1)
                .count()
        );
    }
}
